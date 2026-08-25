//! The two Win32 calls, and the whole of this workspace's unsafe key path.
//!
//! One function does both directions, because the two API entry points take
//! the same arguments in the same order and differ only in which one is
//! called: a second copy of the buffer handling would be a second thing to
//! review. Below it are the four `extern "system"` declarations, and that is
//! the end of the crate.
//!
//! Reviewer's checklist for the block below, in the order the code runs:
//!
//! 1. the two input `DATA_BLOB`s borrow Rust slices that outlive the call, and
//!    DPAPI does not write through either of them;
//! 2. the output `DATA_BLOB` starts zeroed, so a failing call leaves a null
//!    pointer and the length is never read;
//! 3. on success Windows allocated the output with `LocalAlloc`; it is copied
//!    once, wiped, and freed with `LocalFree`, exactly once, on every path out;
//! 4. no pointer outlives the function.

// The only `unsafe` in the crate, and it is confined to this file. The crate
// root is `deny(unsafe_code)` on Windows and `forbid(unsafe_code)` everywhere
// else, so a second platform call cannot be added without landing here first.
#![allow(unsafe_code)]

use std::ptr;
use std::slice;

use zeroize::Zeroize;

use crate::{DpapiError, CRYPTPROTECT_UI_FORBIDDEN};

pub fn protect(secret: &[u8], entropy: &[u8]) -> Result<Vec<u8>, DpapiError> {
    call(Direction::Protect, secret, entropy)
}

pub fn unprotect(blob: &[u8], entropy: &[u8]) -> Result<Vec<u8>, DpapiError> {
    call(Direction::Unprotect, blob, entropy)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Protect,
    Unprotect,
}

impl Direction {
    fn failed(self, code: u32) -> DpapiError {
        match self {
            Direction::Protect => DpapiError::Protect { code },
            Direction::Unprotect => DpapiError::Unprotect { code },
        }
    }
}

fn call(direction: Direction, input: &[u8], entropy: &[u8]) -> Result<Vec<u8>, DpapiError> {
    let input_blob = borrowed_blob(input)?;
    let entropy_blob = borrowed_blob(entropy)?;
    // DPAPI takes a null pointer to mean "no secondary entropy". An empty
    // slice would otherwise arrive as a dangling pointer with a zero length.
    let entropy_argument: *const ffi::DataBlob = if entropy.is_empty() {
        ptr::null()
    } else {
        &entropy_blob
    };
    let mut out = ffi::DataBlob {
        cb_data: 0,
        pb_data: ptr::null_mut(),
    };

    // SAFETY: `input_blob` and `entropy_blob` describe slices that live until
    // the end of this function and that DPAPI only reads. `out` is a live,
    // zeroed `DATA_BLOB` for the duration of the call. Both entry points are
    // declared with the signatures documented for wincrypt.h, and the two
    // pointer arguments this code does not use (`pvReserved`,
    // `pPromptStruct`) are documented as ignorable when null — which, with
    // `CRYPTPROTECT_UI_FORBIDDEN`, is also why no prompt can be raised.
    let succeeded = unsafe {
        match direction {
            Direction::Protect => ffi::CryptProtectData(
                &input_blob,
                ptr::null(),
                entropy_argument,
                ptr::null_mut(),
                ptr::null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            ),
            Direction::Unprotect => ffi::CryptUnprotectData(
                &input_blob,
                ptr::null_mut(),
                entropy_argument,
                ptr::null_mut(),
                ptr::null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            ),
        }
    };

    if succeeded == 0 {
        // SAFETY: reads this thread's last-error value. It is meaningful here
        // because nothing has run between the failing call and this one.
        let code = unsafe { ffi::GetLastError() };
        return Err(direction.failed(code));
    }

    if out.pb_data.is_null() || out.cb_data == 0 {
        // Not a documented outcome. Reported rather than returned as an empty
        // secret, which would unprotect into a key of no bytes.
        if !out.pb_data.is_null() {
            // SAFETY: a non-null buffer Windows allocated and this function
            // is done with.
            unsafe { ffi::LocalFree(out.pb_data.cast()) };
        }
        return Err(DpapiError::Empty);
    }

    // SAFETY: the call succeeded, so `out.pb_data` points at `out.cb_data`
    // initialised bytes allocated by Windows with `LocalAlloc`. The slice is
    // used only inside this block; it is copied, wiped so the freed allocation
    // does not keep a key around, and freed once. Nothing derived from it
    // escapes.
    let copied = unsafe {
        let produced = slice::from_raw_parts_mut(out.pb_data, out.cb_data as usize);
        let copied = produced.to_vec();
        produced.zeroize();
        ffi::LocalFree(out.pb_data.cast());
        copied
    };
    Ok(copied)
}

/// A `DATA_BLOB` pointing at bytes Rust owns.
///
/// `pb_data` is `*mut` because that is the C layout; neither entry point
/// writes through the input blobs.
fn borrowed_blob(bytes: &[u8]) -> Result<ffi::DataBlob, DpapiError> {
    let length = u32::try_from(bytes.len()).map_err(|_| DpapiError::TooLarge(bytes.len()))?;
    Ok(ffi::DataBlob {
        cb_data: length,
        pb_data: bytes.as_ptr().cast_mut(),
    })
}

/// The four Win32 entry points this crate uses, declared rather than pulled in
/// as a dependency, the same way `soul-collect` declares its foreground-window
/// calls: the workspace pins every third-party version centrally, and four
/// declarations are cheaper to audit than a binding crate.
#[allow(non_snake_case)]
mod ffi {
    use std::ffi::c_void;

    /// `DATA_BLOB` from wincrypt.h: a length and a pointer, in that order.
    #[derive(Debug)]
    #[repr(C)]
    pub struct DataBlob {
        pub cb_data: u32,
        pub pb_data: *mut u8,
    }

    #[link(name = "crypt32")]
    extern "system" {
        pub fn CryptProtectData(
            data_in: *const DataBlob,
            data_description: *const u16,
            optional_entropy: *const DataBlob,
            reserved: *mut c_void,
            prompt_struct: *mut c_void,
            flags: u32,
            data_out: *mut DataBlob,
        ) -> i32;

        // `data_description` is an out-parameter for the description
        // `CryptProtectData` was given. Always null at the call site: this
        // crate never sets one, so there is nothing to read back and nothing
        // to free.
        pub fn CryptUnprotectData(
            data_in: *const DataBlob,
            data_description: *mut *mut u16,
            optional_entropy: *const DataBlob,
            reserved: *mut c_void,
            prompt_struct: *mut c_void,
            flags: u32,
            data_out: *mut DataBlob,
        ) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        pub fn LocalFree(memory: *mut c_void) -> *mut c_void;
        pub fn GetLastError() -> u32;
    }
}
