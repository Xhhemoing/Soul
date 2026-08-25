//! Field-level AEAD, independent of any store implementation.
//!
//! SECURITY.md commits to XChaCha20-Poly1305 over each prose field with
//! `row_id|field_name` as additional authenticated data. This pins that down:
//! the right AAD opens the box, a wrong one does not.

use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng, Payload};
use chacha20poly1305::XChaCha20Poly1305;
use soul_schema::common::field_aad;
use uuid::Uuid;

const PROSE: &str = "第一次去北京，站台上风很大 café 🙂";

#[test]
fn xchacha20poly1305_binds_row_and_field() {
    let row: Uuid = "0192a1b2-c3d4-7e5f-8a9b-0c1d2e3f4050"
        .parse()
        .expect("uuid");
    let other_row: Uuid = "0192a1b2-c3d4-7e5f-8a9b-0c1d2e3f4052"
        .parse()
        .expect("uuid");

    let key = XChaCha20Poly1305::generate_key(&mut OsRng);
    let cipher = XChaCha20Poly1305::new(&key);
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);

    let aad = field_aad(&row, "summary_ref");
    assert_eq!(aad, format!("{row}|summary_ref"));

    let ciphertext = cipher
        .encrypt(
            &nonce,
            Payload {
                msg: PROSE.as_bytes(),
                aad: aad.as_bytes(),
            },
        )
        .expect("seal");

    assert!(
        !ciphertext
            .windows(PROSE.len())
            .any(|w| w == PROSE.as_bytes()),
        "the plaintext must not survive in the ciphertext",
    );

    let opened = cipher
        .decrypt(
            &nonce,
            Payload {
                msg: &ciphertext,
                aad: aad.as_bytes(),
            },
        )
        .expect("correct AAD opens the box");
    assert_eq!(String::from_utf8(opened).expect("utf-8"), PROSE);

    for wrong in [
        field_aad(&row, "title_ref"),
        field_aad(&other_row, "summary_ref"),
        String::new(),
    ] {
        assert!(
            cipher
                .decrypt(
                    &nonce,
                    Payload {
                        msg: &ciphertext,
                        aad: wrong.as_bytes(),
                    },
                )
                .is_err(),
            "AAD `{wrong}` must not authenticate this blob",
        );
    }
}
