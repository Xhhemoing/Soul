//! What DPAPI does, checked on the platform that has it.
//!
//! The Windows half runs on `windows-latest` through `cargo test --workspace
//! --all-targets`; there is no way to run it anywhere else, and a fake would
//! only test the fake. The Linux half checks the other promise this crate
//! makes — that off Windows it refuses in as many words rather than returning
//! bytes from somewhere.

use soul_win_dpapi::{protect, unprotect, DpapiError};

/// The bytes the store's key provider actually protects: a 32-byte KEK.
const KEK: &[u8] = b"0123456789abcdef0123456789abcdef";

/// Soul's secondary entropy. Not a secret — see the crate docs.
const ENTROPY: &[u8] = b"soul/v1/test-entropy";

#[cfg(windows)]
mod on_windows {
    use super::*;

    #[test]
    fn a_protected_secret_comes_back_byte_for_byte() {
        let blob = protect(KEK, ENTROPY).expect("CryptProtectData");
        let recovered = unprotect(&blob, ENTROPY).expect("CryptUnprotectData");
        assert_eq!(&recovered[..], KEK);
    }

    /// The point of the exercise. If the key were sitting in the file, the
    /// Windows install path would be no better than the developer seed file it
    /// replaces.
    #[test]
    fn the_blob_does_not_contain_the_secret() {
        let blob = protect(KEK, ENTROPY).expect("CryptProtectData");
        assert!(blob.len() > KEK.len(), "a blob of {} bytes", blob.len());
        assert!(
            !blob.windows(KEK.len()).any(|window| window == KEK),
            "the protected blob repeats the secret it was given",
        );
    }

    /// DPAPI salts each call, so two protections of one secret differ. Worth
    /// pinning because a deterministic blob would make "the file changed"
    /// mean something it does not.
    #[test]
    fn protecting_the_same_secret_twice_gives_two_different_blobs() {
        let first = protect(KEK, ENTROPY).expect("first");
        let second = protect(KEK, ENTROPY).expect("second");
        assert_ne!(first, second);
        assert_eq!(&unprotect(&first, ENTROPY).expect("first")[..], KEK);
        assert_eq!(&unprotect(&second, ENTROPY).expect("second")[..], KEK);
    }

    #[test]
    fn the_wrong_entropy_does_not_unprotect() {
        let blob = protect(KEK, ENTROPY).expect("CryptProtectData");
        let refused = unprotect(&blob, b"soul/v1/some-other-entropy");
        assert!(
            matches!(refused, Err(DpapiError::Unprotect { .. })),
            "{refused:?}",
        );
        assert!(matches!(
            unprotect(&blob, b""),
            Err(DpapiError::Unprotect { .. })
        ));
    }

    /// DPAPI authenticates its own blob. A file somebody edited must not
    /// unprotect into a key that is nearly right.
    #[test]
    fn an_altered_blob_is_refused_rather_than_half_recovered() {
        let mut blob = protect(KEK, ENTROPY).expect("CryptProtectData");
        let last = blob.len() - 1;
        blob[last] ^= 0xff;
        assert!(
            matches!(unprotect(&blob, ENTROPY), Err(DpapiError::Unprotect { .. })),
            "an altered blob unprotected",
        );
    }

    #[test]
    fn nonsense_is_refused_with_a_status_code_and_no_panic() {
        for garbage in [&b""[..], &b"not a DPAPI blob"[..], &[0u8; 512][..]] {
            match unprotect(garbage, ENTROPY) {
                Err(DpapiError::Unprotect { code }) => assert_ne!(code, 0),
                Err(DpapiError::Empty) => {}
                other => panic!("{other:?}"),
            }
        }
    }
}

#[cfg(not(windows))]
mod off_windows {
    use super::*;

    /// The refusal is the contract on this side. A stub that returned bytes —
    /// derived, zeroed, anything — would make the Linux test suite green
    /// against key material no Windows machine would ever produce.
    #[test]
    fn both_directions_refuse_instead_of_producing_key_material() {
        assert_eq!(protect(KEK, ENTROPY).unwrap_err(), DpapiError::Unsupported);
        assert_eq!(
            unprotect(b"whatever", ENTROPY).unwrap_err(),
            DpapiError::Unsupported,
        );
    }
}
