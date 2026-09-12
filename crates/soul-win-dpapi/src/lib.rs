//! Windows DPAPI, and deliberately nothing else.
//!
//! `docs/SECURITY.md` pins the key chain as `DPAPI → KEK → DB DEK → 每单元
//! CK`. The first arrow is a pair of Win32 calls, `CryptProtectData` and
//! `CryptUnprotectData`, and calling them needs `unsafe`. Every crate that
//! holds key material — `soul-store` above all — is `#![forbid(unsafe_code)]`,
//! and that is worth keeping. So the arrow lives here, alone, in a crate whose
//! entire public surface is [`protect`] and [`unprotect`].
//!
//! What that buys, concretely: the reviewable `unsafe` in this workspace's
//! key path is one file, `src/sys.rs`, with two `extern "system"`
//! declarations and one function that calls them. Nothing about Soul's data
//! model, its file formats or its key derivation is in here — this crate does
//! not know what a KEK is.
//!
//! ## What DPAPI actually protects
//!
//! The blob is protected under the **logged-in user's** credentials
//! (`CRYPTPROTECT_LOCAL_MACHINE` is not passed), so:
//!
//! * another account on the same machine cannot unprotect it;
//! * the same account on another machine cannot either, unless the user's
//!   roaming profile and master key travel with them;
//! * the local administrator, a debugger attached to the user's session, and
//!   anything running *as* that user can. `docs/SECURITY.md` already declines
//!   to promise otherwise, and DPAPI does not change that.
//!
//! [`CRYPTPROTECT_UI_FORBIDDEN`] is always set: this runs during store open,
//! where a prompt would be a hang rather than a question. A failure is
//! reported as a failure.
//!
//! ## Off Windows
//!
//! Everything compiles, and both functions return [`DpapiError::Unsupported`].
//! That is what keeps Linux CI building the same call sites that Windows runs,
//! and it is why the caller never needs a `#[cfg]` of its own. It is also why
//! [`SUPPORTED`] exists: a caller that is about to create key material can ask
//! first, rather than finding out after it has written a file.

#![cfg_attr(not(windows), forbid(unsafe_code))]
#![cfg_attr(windows, deny(unsafe_code))]
#![deny(missing_debug_implementations)]

use zeroize::Zeroizing;

#[cfg(windows)]
mod sys;

#[cfg(not(windows))]
mod sys {
    use super::DpapiError;

    pub fn protect(_secret: &[u8], _entropy: &[u8]) -> Result<Vec<u8>, DpapiError> {
        Err(DpapiError::Unsupported)
    }

    pub fn unprotect(_blob: &[u8], _entropy: &[u8]) -> Result<Vec<u8>, DpapiError> {
        Err(DpapiError::Unsupported)
    }
}

/// Whether this build can call DPAPI at all.
///
/// A constant rather than a function of the running machine: DPAPI is present
/// on every Windows that Soul supports, so the only question is which platform
/// was compiled for.
pub const SUPPORTED: bool = cfg!(windows);

/// The flag that turns "ask the user" into "fail".
///
/// Named here rather than buried in `sys.rs` because it is a policy choice,
/// not a binding detail: a key provider called during store open has nobody to
/// prompt.
pub const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x0000_0001;

pub type DpapiResult<T> = Result<T, DpapiError>;

/// Why a DPAPI call did not produce bytes.
///
/// No variant carries plaintext, ciphertext or the entropy — only the Win32
/// status code and a sentence.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DpapiError {
    /// Not Windows. The caller has to find its key material somewhere else, or
    /// say that it cannot.
    #[error("DPAPI is a Windows API and this build is not for Windows")]
    Unsupported,

    #[error("CryptProtectData failed (Win32 error {code})")]
    Protect { code: u32 },

    /// The usual causes, none of which this crate can tell apart: a blob
    /// written by another user or on another machine, a blob that has been
    /// altered, or entropy that does not match.
    #[error(
        "CryptUnprotectData failed (Win32 error {code}); the protected data belongs to another \
         user or another machine, or it has been altered"
    )]
    Unprotect { code: u32 },

    /// DPAPI's length fields are 32-bit. Nothing Soul protects comes close.
    #[error("{0} bytes is more than DPAPI's 32-bit length field can describe")]
    TooLarge(usize),

    /// The call succeeded and handed back an empty buffer. Not a documented
    /// outcome; reported rather than turned into an empty secret.
    #[error("DPAPI reported success and returned no bytes")]
    Empty,
}

/// Protect `secret` under the logged-in user's DPAPI master key.
///
/// `entropy` is DPAPI's optional secondary entropy. It is **not** a secret and
/// adds no strength on its own — the same value has to be supplied to
/// [`unprotect`], so what it buys is that a blob Soul wrote is not something
/// another application on the same account can unprotect by accident. Pass the
/// same bytes to both calls.
///
/// The returned bytes are ciphertext and are safe to write to a file.
pub fn protect(secret: &[u8], entropy: &[u8]) -> DpapiResult<Vec<u8>> {
    sys::protect(secret, entropy)
}

/// Recover what [`protect`] was given, or say why not.
///
/// The result wipes itself when dropped, because on the path this crate exists
/// for it is a key.
pub fn unprotect(blob: &[u8], entropy: &[u8]) -> DpapiResult<Zeroizing<Vec<u8>>> {
    sys::unprotect(blob, entropy).map(Zeroizing::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every variant is a status code and a sentence. None of them has a field
    /// that could hold the bytes that went in, which is the property worth
    /// pinning: a failing key call must not be a way to log a key.
    #[test]
    fn a_failure_reads_as_a_sentence_and_carries_only_a_status_code() {
        assert!(DpapiError::Unprotect { code: 13 }
            .to_string()
            .contains("13"));
        for error in [
            DpapiError::Unsupported,
            DpapiError::Protect { code: 5 },
            DpapiError::Unprotect { code: 13 },
            DpapiError::TooLarge(usize::MAX),
            DpapiError::Empty,
        ] {
            let rendered = error.to_string();
            assert!(rendered.len() > 20, "got {rendered}");
            assert!(!rendered.contains('\n'), "got {rendered}");
        }
    }

    #[test]
    fn support_follows_the_platform_and_nothing_else() {
        assert_eq!(SUPPORTED, cfg!(windows));
    }
}
