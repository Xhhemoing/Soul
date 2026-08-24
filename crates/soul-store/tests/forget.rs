//! AC-15: preview the impact, forget, restart, and find the prose gone.
//!
//! The restart is the part that matters. A store that keeps the content key in
//! a field and clears it would pass every in-process assertion and still hand
//! the plaintext back after the application is restarted, so every check below
//! runs against a store that was closed and reopened from the file.
//!
//! The preview numbers are checked against a fixture built so that no plausible
//! constant would satisfy it: forgetting the first memory destroys three sealed
//! blobs across two content keys, and forgetting the second destroys one.

mod common;

use common::*;

use soul_schema::audit::AuditAction;
use soul_schema::contact::ContactClass;
use soul_schema::event::EventKind;
use soul_schema::memory::ForgetState;
use soul_schema::Subject;
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::forget::ForgetUnit;
use soul_store_api::types::{InferenceState, StoreError};
use soul_store_api::{
    AuditLog, BlobStore, EventStore, ForgetOps, GraphStore, MemoryStore, ProfileStore, SoulStore,
};

const SEED: &str = "wp02 forget";
const TRIP_SUMMARY: &str = "第一次去北京，站台上风很大";
const TRIP_TITLE: &str = "北京";
const NOTE_SUMMARY: &str = "第二段记忆，和第一段无关";

// One memory over two content keys, one memory over a third.
const MEMORY_TRIP: &str = "50";
const MEMORY_NOTE: &str = "51";
const KEY_TITLE: &str = "60";
const KEY_SUMMARY: &str = "61";
const KEY_NOTE: &str = "62";
const EVIDENCE_TRIP: &str = "70";
const EVIDENCE_NOTE: &str = "71";
const INFERENCE_TRIP: &str = "80";
const INFERENCE_NOTE: &str = "81";

struct Seeded {
    trip_title: soul_schema::common::SealedText,
    trip_summary: soul_schema::common::SealedText,
    note_summary: soul_schema::common::SealedText,
}

/// Two independent memories, so a forget that over-reaches is visible.
fn seed(store: &mut SqlCipherStore) -> Seeded {
    let trip_title = store
        .seal(owner_seal(
            id(KEY_TITLE),
            id(MEMORY_TRIP),
            "title_ref",
            TRIP_TITLE,
        ))
        .expect("seal title");
    let trip_summary = store
        .seal(owner_seal(
            id(KEY_SUMMARY),
            id(MEMORY_TRIP),
            "summary_ref",
            TRIP_SUMMARY,
        ))
        .expect("seal summary");
    // A third blob under a key the trip memory already owns: the preview has to
    // count blobs, not memories.
    let event_body = store
        .seal(owner_seal(
            id(KEY_TITLE),
            id("52"),
            "body_ref",
            "北京南站的广播声",
        ))
        .expect("seal event body");
    let note_summary = store
        .seal(owner_seal(
            id(KEY_NOTE),
            id(MEMORY_NOTE),
            "summary_ref",
            NOTE_SUMMARY,
        ))
        .expect("seal note");

    let mut carrier = event(
        id("52"),
        "2026-08-24T09:00:00Z",
        EventKind::MessageObserved,
        Subject::Owner,
    );
    carrier.body_ref = Some(event_body);
    store.append_event(carrier).expect("append event");

    store
        .put_evidence(evidence(id(EVIDENCE_TRIP), Subject::Owner))
        .expect("trip evidence");
    store
        .put_evidence(evidence(id(EVIDENCE_NOTE), Subject::Owner))
        .expect("note evidence");
    store
        .put_inference(inference(id(INFERENCE_TRIP), &[id(EVIDENCE_TRIP)]))
        .expect("trip inference");
    store
        .put_inference(inference(id(INFERENCE_NOTE), &[id(EVIDENCE_NOTE)]))
        .expect("note inference");

    store
        .put_memory(memory(
            id(MEMORY_TRIP),
            id(KEY_TITLE),
            Some(trip_title.clone()),
            Some(trip_summary.clone()),
            &[id(EVIDENCE_TRIP)],
        ))
        .expect("trip memory");
    store
        .put_memory(memory(
            id(MEMORY_NOTE),
            id(KEY_NOTE),
            None,
            Some(note_summary.clone()),
            &[id(EVIDENCE_NOTE)],
        ))
        .expect("note memory");

    store
        .append_audit(audit_entry(
            id("90"),
            AuditAction::MemoryWrite,
            &[id(MEMORY_TRIP)],
        ))
        .expect("audit for the trip");
    store
        .append_audit(audit_entry(
            id("91"),
            AuditAction::MemoryWrite,
            &[id(MEMORY_NOTE)],
        ))
        .expect("audit for the note");
    store
        .append_audit(audit_entry(id("92"), AuditAction::ConsentGrant, &[]))
        .expect("audit for something else");

    Seeded {
        trip_title,
        trip_summary,
        note_summary,
    }
}

#[test]
fn the_preview_counts_what_is_there_and_the_receipt_matches_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let keys = TestKeyProvider::from_seed(SEED);
    let mut store = SqlCipherStore::open(dir.path().join("forget.db"), &keys).expect("open");
    seed(&mut store);

    let trip = store
        .preview_impact(ForgetUnit::Memory(id(MEMORY_TRIP)))
        .expect("preview the trip");
    assert_eq!(
        trip.content_key_ids,
        vec![id(KEY_TITLE), id(KEY_SUMMARY)],
        "a memory whose title and summary sit under different keys owns both",
    );
    assert_eq!(trip.memories_affected, 1);
    assert_eq!(trip.contacts_affected, 0);
    assert_eq!(
        trip.sealed_blobs_destroyed, 3,
        "title, summary, and the event body sealed under the same key",
    );
    assert_eq!(trip.inferences_orphaned, 1);
    assert_eq!(trip.audit_entries_retained, 1);

    let note = store
        .preview_impact(ForgetUnit::Memory(id(MEMORY_NOTE)))
        .expect("preview the note");
    assert_eq!(
        note.sealed_blobs_destroyed, 1,
        "the second memory must produce its own number, not the first one's",
    );
    assert_eq!(note.content_key_ids, vec![id(KEY_NOTE)]);

    let unknown = store
        .preview_impact(ForgetUnit::Memory(id("ff")))
        .expect("preview something that does not exist");
    assert_eq!(unknown.sealed_blobs_destroyed, 0);
    assert_eq!(unknown.content_key_ids, Vec::new());

    // The preview is read-only, and running it twice cannot change the answer.
    assert_eq!(
        store
            .preview_impact(ForgetUnit::Memory(id(MEMORY_TRIP)))
            .expect("preview again"),
        trip,
    );
    assert_eq!(
        store
            .get_memory(id(MEMORY_TRIP))
            .expect("still there")
            .forget_state,
        ForgetState::Active,
    );

    let receipt = store
        .execute_forget(ForgetUnit::Memory(id(MEMORY_TRIP)))
        .expect("execute");
    assert_eq!(
        receipt.impact, trip,
        "the user must not be shown one number and charged another",
    );
}

#[test]
fn after_a_forget_and_a_restart_the_prose_is_unreadable_and_the_audit_chain_still_verifies() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("forget-restart.db");
    let keys = TestKeyProvider::from_seed(SEED);

    let sealed = {
        let mut store = SqlCipherStore::open(&path, &keys).expect("open");
        let sealed = seed(&mut store);

        assert_eq!(
            String::from_utf8(store.open(&sealed.trip_summary).expect("readable before"))
                .expect("utf-8"),
            TRIP_SUMMARY,
        );

        store
            .execute_forget(ForgetUnit::Memory(id(MEMORY_TRIP)))
            .expect("execute forget");
        store.flush().expect("flush");
        store.close().expect("close, as if the application exited");
        sealed
    };

    // Everything below runs against the file, reopened from scratch.
    let store = SqlCipherStore::open(&path, &keys).expect("reopen after a restart");

    assert!(
        !store.has_content_key(id(KEY_TITLE)) && !store.has_content_key(id(KEY_SUMMARY)),
        "both content keys the memory owned must be gone",
    );
    assert!(
        store.has_content_key(id(KEY_NOTE)),
        "the untouched memory must keep its key",
    );
    assert_eq!(
        store.content_key_count().expect("count keys"),
        1,
        "only the note's key survives",
    );

    for gone in [&sealed.trip_title, &sealed.trip_summary] {
        assert!(
            matches!(store.open(gone), Err(StoreError::ContentKeyDestroyed(_))),
            "the prose must be unreadable once its key is destroyed",
        );
    }
    assert_eq!(
        String::from_utf8(store.open(&sealed.note_summary).expect("still readable"))
            .expect("utf-8"),
        NOTE_SUMMARY,
        "forgetting one memory must not touch another",
    );

    assert_eq!(
        store.get_memory(id(MEMORY_TRIP)).expect("row remains").forget_state,
        ForgetState::Forgotten,
        "the row stays as a tombstone so the user can see the memory existed",
    );
    assert_eq!(
        store.get_memory(id(MEMORY_NOTE)).expect("row").forget_state,
        ForgetState::Active,
    );

    assert_eq!(
        store
            .inference_state(id(INFERENCE_TRIP))
            .expect("trip inference state"),
        InferenceState::Orphaned,
        "an inference whose evidence was forgotten must be demoted",
    );
    assert_eq!(
        store
            .inference_state(id(INFERENCE_NOTE))
            .expect("note inference state"),
        InferenceState::Live,
    );

    let audit = store.list_audit().expect("audit survives");
    assert_eq!(audit.len(), 3, "forgetting must not delete audit entries");
    store
        .verify_audit_chain()
        .expect("the chain must still verify after a forget and a restart");

    let encoded = serde_json::to_string(&audit).expect("audit serializes");
    for prose in [TRIP_SUMMARY, TRIP_TITLE, NOTE_SUMMARY] {
        assert!(
            !encoded.contains(prose),
            "the audit chain must never have carried the prose in the first place",
        );
    }
    assert!(
        audit
            .iter()
            .any(|entry| entry.subject_refs.as_deref().unwrap_or_default()
                == [id(MEMORY_TRIP)].as_slice()),
        "the bare UUID of a forgotten memory may, and should, outlive the row",
    );
}

/// Forgetting a contact reaches the evidence carried by the graph edges it
/// takes part in, not just the label sealed on the row.
#[test]
fn forgetting_a_contact_orphans_what_the_edge_supported() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("forget-contact.db");
    let keys = TestKeyProvider::from_seed(SEED);

    let label = {
        let mut store = SqlCipherStore::open(&path, &keys).expect("open");

        let label = store
            .seal(third_party_seal(id("63"), id("40"), "display_label_ref", "李雷"))
            .expect("seal the label");
        let mut them = contact(id("40"), ContactClass::ThirdParty);
        them.display_label_ref = Some(label.clone());
        store.put_contact(them).expect("their contact row");
        store
            .put_contact(contact(id("41"), ContactClass::Owner))
            .expect("my contact row");

        store
            .put_evidence(evidence(id("72"), Subject::Mixed))
            .expect("edge evidence");
        store
            .put_relationship(relationship(id("42"), id("41"), id("40"), &[id("72")]))
            .expect("edge");
        store
            .put_inference(inference(id("82"), &[id("72")]))
            .expect("inference resting on the edge");

        let impact = store
            .preview_impact(ForgetUnit::Contact(id("40")))
            .expect("preview");
        assert_eq!(impact.contacts_affected, 1);
        assert_eq!(impact.inferences_orphaned, 1);
        assert_eq!(impact.sealed_blobs_destroyed, 1);

        store
            .execute_forget(ForgetUnit::Contact(id("40")))
            .expect("forget them");
        store.flush().expect("flush");
        store.close().expect("close");
        label
    };

    let store = SqlCipherStore::open(&path, &keys).expect("reopen");
    assert!(matches!(
        store.open(&label),
        Err(StoreError::ContentKeyDestroyed(_))
    ));
    assert_eq!(
        store.get_contact(id("40")).expect("row").forget_state,
        ForgetState::Forgotten,
    );
    assert_eq!(
        store.inference_state(id("82")).expect("state"),
        InferenceState::Orphaned,
    );
}
