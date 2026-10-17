//! A preview binds row identities as well as the numbers shown in the UI.
mod common;

use common::{evidence, id, inference, memory, owner_seal};
use soul_schema::Subject;
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::forget::{ForgetOps, ForgetUnit};
use soul_store_api::{BlobStore, MemoryStore, ProfileStore};

#[test]
fn equal_counts_do_not_authorize_different_evidence_and_inferences() {
    let dir = tempfile::tempdir().expect("synthetic store");
    let keys = TestKeyProvider::from_seed("forget scope identity");
    let mut store = SqlCipherStore::open(dir.path().join("soul.db"), &keys).expect("open");
    let prose = store
        .seal(owner_seal(
            id("60"),
            id("50"),
            "title_ref",
            "synthetic title",
        ))
        .expect("seal");
    for (evidence_id, inference_id) in [("70", "80"), ("71", "81")] {
        store
            .put_evidence(evidence(id(evidence_id), Subject::Owner))
            .expect("evidence");
        store
            .put_inference(inference(id(inference_id), &[id(evidence_id)]))
            .expect("inference");
    }
    let mut row = memory(id("50"), id("60"), Some(prose.clone()), None, &[id("70")]);
    store.put_memory(row.clone()).expect("memory");
    let unit = ForgetUnit::Memory(row.memory_id);
    let quoted = store.preview_forget_scope(unit).expect("quote");
    assert!(store.forget_scope_is_current(&quoted).expect("unchanged"));

    row.evidence_ids = Some(vec![id("71")]);
    store
        .put_memory(row)
        .expect("replace one evidence with another");
    assert_eq!(
        store.preview_impact(unit).expect("same counts"),
        *quoted.impact()
    );
    assert!(!store
        .forget_scope_is_current(&quoted)
        .expect("changed identity"));
    assert_eq!(
        store.open(&prose).expect("checking destroys nothing"),
        b"synthetic title"
    );
}

#[test]
fn unrelated_memory_does_not_invalidate_a_quote_but_shared_key_growth_does() {
    let dir = tempfile::tempdir().expect("synthetic store");
    let keys = TestKeyProvider::from_seed("forget scope growth");
    let mut store = SqlCipherStore::open(dir.path().join("soul.db"), &keys).expect("open");
    let prose = store
        .seal(owner_seal(id("60"), id("50"), "title_ref", "one"))
        .expect("seal");
    let row = memory(id("50"), id("60"), Some(prose.clone()), None, &[]);
    store.put_memory(row.clone()).expect("memory");
    let quoted = store
        .preview_forget_scope(ForgetUnit::Memory(id("50")))
        .expect("quote");

    let other = store
        .seal(owner_seal(id("61"), id("51"), "title_ref", "two"))
        .expect("seal");
    store
        .put_memory(memory(id("51"), id("61"), Some(other), None, &[]))
        .expect("unrelated");
    assert!(store
        .forget_scope_is_current(&quoted)
        .expect("unrelated write"));

    let mut dependent = row;
    dependent.memory_id = id("52");
    store
        .put_memory(dependent)
        .expect("a second memory shares the key");
    assert!(!store
        .forget_scope_is_current(&quoted)
        .expect("expanded scope"));
    assert_eq!(store.open(&prose).expect("key survived"), b"one");
}
