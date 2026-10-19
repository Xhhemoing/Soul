#![allow(unsafe_code)]
//! Win32 ABI and all unsafe notification code stay in this module.
use super::{
    engine::{Os, OsSubmission, Shared},
    EventKind, Failure, NOTIFICATION_BODY, NOTIFICATION_TITLE,
};
use std::collections::BTreeSet;
use std::ffi::c_void;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};

type Handle = *mut c_void;
type WindowProc = unsafe extern "system" fn(Handle, u32, usize, isize) -> isize;
const WM_NCCREATE: u32 = 0x81;
const WM_NCDESTROY: u32 = 0x82;
const WM_QUIT: u32 = 0x12;
const WM_POWERBROADCAST: u32 = 0x218;
const CALLBACK: u32 = 0x8000 + 0x33;
const GWLP_USERDATA: i32 = -21;
const NIM_ADD: u32 = 0;
const NIM_MODIFY: u32 = 1;
const NIM_DELETE: u32 = 2;
const NIM_SETVERSION: u32 = 4;
static CLASS_SEQUENCE: AtomicU64 = AtomicU64::new(1);

struct Context {
    shared: Arc<Shared>,
    taskbar_message: u32,
    reclaimed: Arc<AtomicBool>,
}
pub(super) struct WindowsOs {
    window: Handle,
    instance: Handle,
    class: Vec<u16>,
    icon: Handle,
    live: BTreeSet<u16>,
    shared: Arc<Shared>,
}
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
fn copy_wide<const N: usize>(destination: &mut [u16; N], value: &str) {
    for (slot, unit) in destination.iter_mut().take(N - 1).zip(value.encode_utf16()) {
        *slot = unit;
    }
}

impl WindowsOs {
    pub fn new(shared: Arc<Shared>) -> Result<Self, Failure> {
        // SAFETY: null requests the current executable's borrowed module handle.
        let instance = unsafe { ffi::GetModuleHandleW(std::ptr::null()) };
        if instance.is_null() {
            return Err(Failure::Initialize);
        }
        let class = wide(&format!(
            "Soul.NotificationTransport.{}.{}",
            std::process::id(),
            CLASS_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let definition = WindowClass {
            style: 0,
            procedure: Some(window_proc),
            class_extra: 0,
            window_extra: 0,
            instance,
            icon: std::ptr::null_mut(),
            cursor: std::ptr::null_mut(),
            background: std::ptr::null_mut(),
            menu: std::ptr::null(),
            class_name: class.as_ptr(),
        };
        // SAFETY: the ABI structure and its UTF-16 buffers outlive this call.
        if unsafe { ffi::RegisterClassW(&definition) } == 0 {
            return Err(Failure::Initialize);
        }
        let taskbar = wide("TaskbarCreated");
        // SAFETY: a valid nul-terminated static message name.
        let taskbar_message = unsafe { ffi::RegisterWindowMessageW(taskbar.as_ptr()) };
        if taskbar_message == 0 {
            unsafe { ffi::UnregisterClassW(class.as_ptr(), instance) };
            return Err(Failure::Initialize);
        }
        let reclaimed = Arc::new(AtomicBool::new(false));
        let context = Box::into_raw(Box::new(Context {
            shared: Arc::clone(&shared),
            taskbar_message,
            reclaimed: Arc::clone(&reclaimed),
        }));
        let title = wide("Soul notification transport");
        // SAFETY: a hidden top-level window, created and destroyed on this same
        // worker thread. WM_NCCREATE installs the owned Context; WM_NCDESTROY
        // releases it. It is never a borrowed pointer into the caller's stack.
        let window = unsafe {
            ffi::CreateWindowExW(
                0,
                class.as_ptr(),
                title.as_ptr(),
                0,
                0,
                0,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                context.cast(),
            )
        };
        if window.is_null() {
            if !reclaimed.load(Ordering::Acquire) {
                unsafe { drop(Box::from_raw(context)) };
            }
            unsafe { ffi::UnregisterClassW(class.as_ptr(), instance) };
            return Err(Failure::Initialize);
        }
        // SAFETY: IDI_APPLICATION is a shared system icon, not an owned HICON.
        let icon = unsafe { ffi::LoadIconW(std::ptr::null_mut(), 32512usize as *const u16) };
        if icon.is_null() {
            unsafe {
                ffi::DestroyWindow(window);
                ffi::UnregisterClassW(class.as_ptr(), instance);
            }
            return Err(Failure::Initialize);
        }
        Ok(Self {
            window,
            instance,
            class,
            icon,
            live: BTreeSet::new(),
            shared,
        })
    }
    fn data(&self, id: u16) -> NotifyIconData {
        NotifyIconData {
            size: std::mem::size_of::<NotifyIconData>() as u32,
            window: self.window,
            id: id as u32,
            flags: 0,
            callback: CALLBACK,
            icon: self.icon,
            tip: [0; 128],
            state: 0,
            state_mask: 0,
            info: [0; 256],
            timeout_or_version: 0,
            title: [0; 64],
            info_flags: 0,
            guid: [0; 16],
            balloon_icon: std::ptr::null_mut(),
        }
    }
}
impl Os for WindowsOs {
    fn submit(&mut self, id: u16) -> OsSubmission {
        let mut data = self.data(id);
        data.flags = 1 | 2 | 4;
        copy_wide(&mut data.tip, NOTIFICATION_TITLE);
        // SAFETY: this thread owns the HWND; the complete ABI value outlives
        // each synchronous Shell call. No Rust state lock is held across it.
        if unsafe { ffi::Shell_NotifyIconW(NIM_ADD, &mut data) } == 0 {
            return OsSubmission {
                accepted: false,
                icon_live: false,
            };
        }
        self.live.insert(id);
        data.flags = 0;
        data.timeout_or_version = 4;
        if unsafe { ffi::Shell_NotifyIconW(NIM_SETVERSION, &mut data) } == 0 {
            let deleted = self.delete(id);
            return OsSubmission {
                accepted: false,
                icon_live: !deleted,
            };
        }
        // NIF_REALTIME avoids an obsolete balloon being queued by the Shell.
        // NIIF_NOSOUND and RESPECT_QUIET_TIME do not imply delivery or display.
        data.flags = 0x10 | 0x40;
        data.info_flags = 1 | 0x10 | 0x80;
        copy_wide(&mut data.title, NOTIFICATION_TITLE);
        copy_wide(&mut data.info, NOTIFICATION_BODY);
        let accepted = unsafe { ffi::Shell_NotifyIconW(NIM_MODIFY, &mut data) } != 0;
        OsSubmission {
            accepted,
            icon_live: true,
        }
    }
    fn delete(&mut self, id: u16) -> bool {
        let mut data = self.data(id);
        let deleted = unsafe { ffi::Shell_NotifyIconW(NIM_DELETE, &mut data) } != 0;
        if deleted {
            self.live.remove(&id);
        }
        deleted
    }
    fn pump(&mut self) {
        let mut message = Message {
            window: std::ptr::null_mut(),
            message: 0,
            w_param: 0,
            l_param: 0,
            time: 0,
            point: [0, 0],
            private: 0,
        };
        // SAFETY: this worker owns the message queue. The shared registry is
        // never locked while DispatchMessage may synchronously reenter us.
        // A sustained message producer must not starve stop/withdraw/expiry.
        // Messages beyond this batch stay in the native queue for the next pass.
        for _ in 0..64 {
            if unsafe { ffi::PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, 1) } == 0 {
                break;
            }
            if message.message == WM_QUIT {
                self.shared.stop();
                break;
            }
            unsafe {
                ffi::TranslateMessage(&message);
                ffi::DispatchMessageW(&message);
            }
        }
    }
}
impl Drop for WindowsOs {
    fn drop(&mut self) {
        for id in self.live.iter().copied().collect::<Vec<_>>() {
            let _ = self.delete(id);
        }
        // SAFETY: destruction occurs on the creation thread. The Context is
        // owned by WM_NCDESTROY, including failure paths or thread teardown;
        // it is not freed while an HWND could still call its procedure.
        unsafe {
            ffi::DestroyWindow(self.window);
            ffi::UnregisterClassW(self.class.as_ptr(), self.instance);
        }
    }
}

unsafe extern "system" fn window_proc(
    window: Handle,
    message: u32,
    w_param: usize,
    l_param: isize,
) -> isize {
    // An unwind may never cross the Win32 callback ABI.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
        if message == WM_NCCREATE {
            let create = &*(l_param as *const CreateStructure);
            ffi::SetWindowLongPtrW(window, GWLP_USERDATA, create.parameter as isize);
            return 1;
        }
        let pointer = ffi::GetWindowLongPtrW(window, GWLP_USERDATA) as *mut Context;
        if !pointer.is_null() {
            if message == WM_NCDESTROY {
                ffi::SetWindowLongPtrW(window, GWLP_USERDATA, 0);
                let context = Box::from_raw(pointer);
                context.reclaimed.store(true, Ordering::Release);
                drop(context);
            } else {
                let context = &*pointer;
                if message == CALLBACK {
                    let event = (l_param as usize & 0xffff) as u32;
                    let id = ((l_param as usize >> 16) & 0xffff) as u16;
                    let kind = match event {
                        0x402 => Some(EventKind::Shown),
                        0x403 => Some(EventKind::Hidden),
                        0x404 => Some(EventKind::TimedOut),
                        0x405 => Some(EventKind::Opened),
                        _ => None,
                    };
                    if let Some(kind) = kind {
                        context.shared.event(Some(id), kind);
                    }
                    return 0;
                }
                if message == context.taskbar_message {
                    context.shared.event(None, EventKind::ExplorerLost);
                    return 0;
                }
                if message == WM_POWERBROADCAST && matches!(w_param, 0x7 | 0x12) {
                    context.shared.event(None, EventKind::Resume);
                    return 1;
                }
            }
        }
        ffi::DefWindowProcW(window, message, w_param, l_param)
    }));
    result.unwrap_or(0)
}

#[repr(C)]
struct WindowClass {
    style: u32,
    procedure: Option<WindowProc>,
    class_extra: i32,
    window_extra: i32,
    instance: Handle,
    icon: Handle,
    cursor: Handle,
    background: Handle,
    menu: *const u16,
    class_name: *const u16,
}
#[repr(C)]
struct CreateStructure {
    parameter: *mut c_void,
    instance: Handle,
    menu: Handle,
    parent: Handle,
    height: i32,
    width: i32,
    y: i32,
    x: i32,
    style: i32,
    name: *const u16,
    class: *const u16,
    extended_style: u32,
}
#[repr(C)]
struct Message {
    window: Handle,
    message: u32,
    w_param: usize,
    l_param: isize,
    time: u32,
    point: [i32; 2],
    private: u32,
}
#[repr(C)]
struct NotifyIconData {
    size: u32,
    window: Handle,
    id: u32,
    flags: u32,
    callback: u32,
    icon: Handle,
    tip: [u16; 128],
    state: u32,
    state_mask: u32,
    info: [u16; 256],
    timeout_or_version: u32,
    title: [u16; 64],
    info_flags: u32,
    guid: [u8; 16],
    balloon_icon: Handle,
}

#[allow(non_snake_case)]
mod ffi {
    use super::*;
    #[link(name = "kernel32")]
    extern "system" {
        pub fn GetModuleHandleW(name: *const u16) -> Handle;
    }
    #[link(name = "user32")]
    extern "system" {
        pub fn RegisterClassW(class: *const WindowClass) -> u16;
        pub fn UnregisterClassW(class: *const u16, instance: Handle) -> i32;
        pub fn RegisterWindowMessageW(name: *const u16) -> u32;
        pub fn CreateWindowExW(
            extended: u32,
            class: *const u16,
            title: *const u16,
            style: u32,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            parent: Handle,
            menu: Handle,
            instance: Handle,
            parameter: *mut c_void,
        ) -> Handle;
        pub fn DestroyWindow(window: Handle) -> i32;
        pub fn LoadIconW(instance: Handle, name: *const u16) -> Handle;
        pub fn SetWindowLongPtrW(window: Handle, index: i32, value: isize) -> isize;
        pub fn GetWindowLongPtrW(window: Handle, index: i32) -> isize;
        pub fn DefWindowProcW(
            window: Handle,
            message: u32,
            w_param: usize,
            l_param: isize,
        ) -> isize;
        pub fn PeekMessageW(
            message: *mut Message,
            window: Handle,
            minimum: u32,
            maximum: u32,
            remove: u32,
        ) -> i32;
        pub fn TranslateMessage(message: *const Message) -> i32;
        pub fn DispatchMessageW(message: *const Message) -> isize;
    }
    #[link(name = "shell32")]
    extern "system" {
        pub fn Shell_NotifyIconW(message: u32, data: *mut NotifyIconData) -> i32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_x64_abi_matches_the_sdk_layout() {
        assert_eq!(std::mem::size_of::<NotifyIconData>(), 976);
        assert_eq!(std::mem::size_of::<WindowClass>(), 72);
        assert_eq!(std::mem::size_of::<CreateStructure>(), 80);
        assert_eq!(std::mem::size_of::<Message>(), 48);
    }
}
