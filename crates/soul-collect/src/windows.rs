//! The Windows foreground application.
//!
//! Three calls, and not one of them returns a caption. `GetForegroundWindow`
//! says which window has focus, `GetWindowThreadProcessId` turns that into a
//! process id, and `QueryFullProcessImageNameW` turns that into an executable
//! path, whose directory is then dropped. The window's own text is never read,
//! and
//! `tests/window_titles_are_not_collected.rs` checks that claim against these
//! sources rather than against this comment.
//!
//! A process the collector cannot open — one running elevated, or a protected
//! system process — is reported as nothing in the foreground. Refusing to name
//! it costs the user the duration of that session; guessing at it from the
//! window instead would cost them the promise that only the application is
//! collected.
//!
//! Compiled only on Windows. Linux CI exercises the same collector through
//! [`crate::FakeForegroundSource`].

// The only `unsafe` in the crate, and it is confined to this file: four Win32
// declarations and the calls to them. Everything else is under
// `deny(unsafe_code)` on Windows and `forbid(unsafe_code)` everywhere else, so
// a second platform hook cannot be added without moving it here first.
#![allow(unsafe_code)]

use crate::source::{AppIdentity, ForegroundSource, SourceError};

/// The real desktop.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct WindowsForegroundSource;

impl WindowsForegroundSource {
    pub fn new() -> WindowsForegroundSource {
        WindowsForegroundSource
    }
}

impl ForegroundSource for WindowsForegroundSource {
    fn sample(&mut self) -> Result<Option<AppIdentity>, SourceError> {
        // SAFETY: no arguments, no out-parameters, and a null return is the
        // documented answer when no window has focus.
        let window = unsafe { ffi::GetForegroundWindow() };
        if window.is_null() {
            return Ok(None);
        }

        let mut process_id: u32 = 0;
        // SAFETY: `window` is a handle Windows just returned, and the out
        // parameter is a live `u32` for the duration of the call.
        let thread_id = unsafe { ffi::GetWindowThreadProcessId(window, &mut process_id) };
        if thread_id == 0 || process_id == 0 {
            // The window went away between the two calls.
            return Ok(None);
        }

        match executable_path(process_id) {
            Some(path) => Ok(Some(AppIdentity::from_executable_path(&path)?)),
            None => Ok(None),
        }
    }

    fn describe(&self) -> &'static str {
        "windows.foreground_process"
    }
}

/// Long enough for any real install path; a longer one is reported as
/// unidentifiable rather than truncated into the wrong file name.
const PATH_BUFFER_CHARS: usize = 1_024;

/// Ask for the least the answer needs: the image name of a process, and not the
/// right to read its memory.
const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

fn executable_path(process_id: u32) -> Option<String> {
    // SAFETY: a plain call with scalar arguments. A failure returns null,
    // which is checked before the handle is used.
    let process = unsafe { ffi::OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
    if process.is_null() {
        // Elevated or protected. Not identifiable at this privilege level.
        return None;
    }

    let mut buffer = [0u16; PATH_BUFFER_CHARS];
    let mut length = buffer.len() as u32;
    // SAFETY: `process` is a live handle, the buffer outlives the call, and
    // `length` is its capacity in UTF-16 code units on the way in and the
    // written length on the way out.
    let queried =
        unsafe { ffi::QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length) };
    // SAFETY: closing a handle this function opened and no longer uses.
    unsafe { ffi::CloseHandle(process) };

    if queried == 0 {
        return None;
    }
    let written = (length as usize).min(buffer.len());
    Some(String::from_utf16_lossy(&buffer[..written]))
}

/// The four Win32 entry points this crate uses, declared rather than pulled in
/// as a dependency: the workspace pins every third-party version centrally, and
/// four declarations are cheaper to audit than a binding crate.
#[allow(non_snake_case)]
mod ffi {
    use std::ffi::c_void;

    /// `HWND` and `HANDLE` are both opaque pointer-sized handles.
    pub type Handle = *mut c_void;

    #[link(name = "user32")]
    extern "system" {
        pub fn GetForegroundWindow() -> Handle;
        pub fn GetWindowThreadProcessId(window: Handle, process_id: *mut u32) -> u32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        pub fn OpenProcess(desired_access: u32, inherit_handle: i32, process_id: u32) -> Handle;
        pub fn QueryFullProcessImageNameW(
            process: Handle,
            flags: u32,
            exe_name: *mut u16,
            size: *mut u32,
        ) -> i32;
        pub fn CloseHandle(object: Handle) -> i32;
    }
}
