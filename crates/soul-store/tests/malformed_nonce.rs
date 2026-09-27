//! Corrupt persisted nonces must produce store errors, never a process panic.

mod common;

use soul_store::{KeyProvider, SqlCipherStore, TestKeyProvider};
use soul_store_api::{BlobStore, StoreError};

#[test]
fn malformed_wrapping_nonce_returns_a_store_error() {
    check_malformed_nonce(
        "SELECT wrap_nonce FROM content_keys",
        "UPDATE content_keys SET wrap_nonce = ?1",
        true,
    );
}

#[test]
fn malformed_blob_nonce_returns_a_store_error() {
    check_malformed_nonce(
        "SELECT nonce FROM sealed_blobs",
        "UPDATE sealed_blobs SET nonce = ?1",
        false,
    );
}

fn check_malformed_nonce(select: &str, update: &str, wrapping_key: bool) {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("corrupt-nonce.db");
    let keys = TestKeyProvider::from_seed("malformed nonce regression");
    let mut store = SqlCipherStore::open(&path, &keys).expect("open store");
    let sealed = store
        .seal(common::owner_seal(
            common::id("60"),
            common::id("50"),
            "summary_ref",
            "recoverable after repairing only the nonce",
        ))
        .expect("seal");

    let conn = rusqlite::Connection::open(&path).expect("open repair connection");
    conn.execute_batch(&format!(
        "PRAGMA key = \"x'{}'\";",
        keys.database_key().expect("test database key").to_hex()
    ))
    .expect("unlock test database");
    let original: Vec<u8> = conn
        .query_row(select, [], |row| row.get(0))
        .expect("read original nonce");

    for len in [0, 23, 25] {
        conn.execute(update, [vec![0u8; len]])
            .expect("persist malformed nonce");
        let error = store
            .open(&sealed)
            .expect_err("corruption must be reported");
        if wrapping_key {
            assert!(matches!(error, StoreError::Backend(_)), "{error:?}");
        } else {
            assert!(
                matches!(error, StoreError::SealBroken(id) if id == sealed.blob_id),
                "{error:?}",
            );
        }
    }

    conn.execute(update, [original]).expect("restore nonce");
    assert_eq!(
        store
            .open(&sealed)
            .expect("the failed reads changed no data"),
        b"recoverable after repairing only the nonce"
    );
}
