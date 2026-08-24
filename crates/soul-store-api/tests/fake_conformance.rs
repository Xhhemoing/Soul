//! The in-memory backend must satisfy the whole storage contract.

use soul_store_api::conformance::run_conformance;
use soul_store_api::forget::{ForgetOps, ForgetUnit};
use soul_store_api::{BlobStore, FakeStore, SealRequest, StoreError};

use soul_schema::common::SealedSubject;
use uuid::Uuid;

#[test]
fn fake_store_passes_the_conformance_suite() {
    run_conformance(FakeStore::new);
}

#[test]
fn destroying_a_content_key_removes_its_blobs() {
    let mut store = FakeStore::new();
    let key: Uuid = "0192a1b2-c3d4-7e5f-8a9b-0c1d2e3f4051".parse().expect("uuid");
    let row: Uuid = "0192a1b2-c3d4-7e5f-8a9b-0c1d2e3f4050".parse().expect("uuid");

    let sealed = store
        .seal(SealRequest::new(
            key,
            row,
            "summary_ref",
            SealedSubject::Owner,
            b"prose".to_vec(),
        ))
        .expect("seal");
    assert_eq!(store.blob_count(), 1);
    assert_eq!(store.content_key_count(), 1);

    store
        .execute_forget(ForgetUnit::ContentKey(key))
        .expect("forget by key");

    assert_eq!(store.blob_count(), 0, "the ciphertext goes with the key");
    assert_eq!(store.content_key_count(), 0);
    assert!(matches!(
        store.open(&sealed),
        Err(StoreError::ContentKeyDestroyed(_))
    ));
}

#[test]
fn forgetting_something_that_does_not_exist_is_a_no_op() {
    let mut store = FakeStore::new();
    let unknown: Uuid = "0192a1b2-c3d4-7e5f-8a9b-0c1d2e3f40ff".parse().expect("uuid");

    let impact = store
        .preview_impact(ForgetUnit::Memory(unknown))
        .expect("preview");
    assert_eq!(impact.memories_affected, 0);
    assert!(impact.content_key_ids.is_empty());

    let receipt = store
        .execute_forget(ForgetUnit::Memory(unknown))
        .expect("execute");
    assert_eq!(receipt.impact, impact);
}
