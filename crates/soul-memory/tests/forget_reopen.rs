//! AC-15, memory side: forget, close the database, reopen it, and find the
//! prose unrecoverable, the inference orphaned, and the audit entry still
//! there.
//!
//! The restart is what makes the test worth writing. A forget implemented as
//! "clear the key from the struct" passes every in-process assertion and hands
//! the plaintext straight back on the next launch, so nothing below is checked
//! until the store has been closed and opened from the file again.
//!
//! The preview numbers get the same treatment. They must come from the store's
//! own query, which is only demonstrable by making a constant wrong: the second
//! memory here owns fewer sealed blobs than the first, and the first grows a
//! third blob when it is edited, because the superseded ciphertext stays under
//! the same key.

mod common;

use common::*;

use uuid::Uuid;

use soul_memory::{create, forget, list, preview_forget, read, update, MemoryError};
use soul_schema::audit::AuditAction;
use soul_schema::memory::ForgetState;
use soul_store_api::forget::ForgetUnit;
use soul_store_api::types::{InferenceState, StoreError};
use soul_store_api::{AuditLog, BlobStore, MemoryStore, ProfileStore, SoulStore};

#[test]
fn the_preview_counts_what_is_really_there_and_the_receipt_charges_the_same() {
    let fixture = MemoryFixture::load();
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path().join("preview.db"));

    let trip_evidence = Uuid::now_v7();
    let note_evidence = Uuid::now_v7();
    let trip_inference = Uuid::now_v7();
    let note_inference = Uuid::now_v7();
    let axis = Uuid::now_v7();
    seed_derivation(&mut store, trip_evidence, trip_inference, axis);
    seed_derivation(&mut store, note_evidence, note_inference, axis);

    let trip = create(
        &mut store,
        &fixture
            .get("first_train_north")
            .draft()
            .citing(&[trip_evidence]),
        NOW,
    )
    .expect("create the trip");
    let note = create(
        &mut store,
        &fixture
            .get("keeps_promises_to_self")
            .draft()
            .citing(&[note_evidence]),
        NOW,
    )
    .expect("create the note");

    let impact = preview_forget(&store, trip.memory_id).expect("preview the trip");
    assert_eq!(
        impact.content_key_ids,
        vec![trip.content_key_id],
        "a memory owns exactly the one key its title and summary were sealed under",
    );
    assert_eq!(impact.memories_affected, 1);
    assert_eq!(impact.contacts_affected, 0);
    assert_eq!(
        impact.sealed_blobs_destroyed, 2,
        "the title and the summary",
    );
    assert_eq!(impact.inferences_orphaned, 1);
    assert_eq!(
        impact.audit_entries_retained, 1,
        "the create is the only entry naming this memory so far",
    );

    // A different memory has to produce a different answer, or the numbers
    // could be constants.
    let note_impact = preview_forget(&store, note.memory_id).expect("preview the note");
    assert_eq!(note_impact.content_key_ids, vec![note.content_key_id]);
    assert_eq!(note_impact.inferences_orphaned, 1);
    assert_ne!(
        note_impact.content_key_ids, impact.content_key_ids,
        "two memories must not share a forget unit",
    );

    // An edit reseals under the same key, so the superseded ciphertext is one
    // more blob the forget will take with it, and one more audit entry.
    update(
        &mut store,
        trip.memory_id,
        &soul_memory::MemoryEdit::summary("重写过的摘要"),
        NOW,
    )
    .expect("edit the trip");
    let after_edit = preview_forget(&store, trip.memory_id).expect("preview after the edit");
    assert_eq!(
        after_edit.sealed_blobs_destroyed, 3,
        "the replaced summary is still ciphertext under the same key, and the \
         preview has to count it or the user is told less will be destroyed \
         than actually is",
    );
    assert_eq!(after_edit.audit_entries_retained, 2);

    // The preview is read-only: running it must not have moved anything.
    assert_eq!(
        preview_forget(&store, trip.memory_id).expect("preview again"),
        after_edit,
    );
    assert_eq!(
        store
            .get_memory(trip.memory_id)
            .expect("still there")
            .forget_state,
        ForgetState::Active,
    );

    let receipt = forget(&mut store, trip.memory_id, NOW).expect("forget");
    assert_eq!(
        receipt.impact, after_edit,
        "the user must not be shown one number and charged another",
    );
    assert_eq!(receipt.unit, ForgetUnit::Memory(trip.memory_id));
}

#[test]
fn after_a_forget_and_a_restart_the_words_are_gone_the_inference_is_orphaned_and_the_audit_remains()
{
    let fixture = MemoryFixture::load();
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("forget-restart.db");

    let forgotten_evidence = Uuid::now_v7();
    let kept_evidence = Uuid::now_v7();
    let forgotten_inference = Uuid::now_v7();
    let kept_inference = Uuid::now_v7();
    let axis = Uuid::now_v7();

    let (forgotten, kept, sealed) = {
        let mut store = open(&path);
        seed_derivation(&mut store, forgotten_evidence, forgotten_inference, axis);
        seed_derivation(&mut store, kept_evidence, kept_inference, axis);

        let forgotten = create(
            &mut store,
            &fixture
                .get("kitchen_argument")
                .draft()
                .citing(&[forgotten_evidence]),
            NOW,
        )
        .expect("create the memory that will be forgotten");
        let kept = create(
            &mut store,
            &fixture
                .get("first_train_north")
                .draft()
                .citing(&[kept_evidence]),
            NOW,
        )
        .expect("create the memory that will not be");

        // Readable before, so the assertion after the forget means something.
        assert_eq!(
            read(&store, forgotten.memory_id)
                .expect("read before")
                .title,
            fixture.get("kitchen_argument").title,
        );

        let sealed = (
            forgotten.title_ref.clone().expect("a sealed title"),
            forgotten.summary_ref.clone().expect("a sealed summary"),
        );

        forget(&mut store, forgotten.memory_id, NOW).expect("forget");
        store.flush().expect("flush");
        store.close().expect("close, as if the application exited");
        (forgotten, kept, sealed)
    };

    // Everything below runs against the file, reopened from scratch.
    let store = open(&path);

    assert!(
        !store.has_content_key(forgotten.content_key_id),
        "the content key must be gone from the file, not just from memory",
    );
    assert!(
        store.has_content_key(kept.content_key_id),
        "forgetting one memory must not reach another",
    );

    for gone in [&sealed.0, &sealed.1] {
        assert!(
            matches!(store.open(gone), Err(StoreError::ContentKeyDestroyed(_))),
            "the ciphertext is still on disk and must stay unopenable; a store \
             that returns plaintext here has not forgotten anything",
        );
    }

    assert!(
        matches!(
            read(&store, forgotten.memory_id),
            Err(MemoryError::Forgotten(id)) if id == forgotten.memory_id,
        ),
        "reading a forgotten memory says it was forgotten, which is not the \
         same as saying it never existed",
    );
    let still_here = read(&store, kept.memory_id).expect("the other memory is untouched");
    assert_eq!(still_here.title, fixture.get("first_train_north").title);
    assert_eq!(still_here.summary, fixture.get("first_train_north").summary);

    // The row survives as a tombstone: the user is allowed to see that they
    // forgot something.
    let digests = list(&store).expect("list");
    assert_eq!(digests.len(), 2);
    let tombstone = digests
        .iter()
        .find(|digest| digest.memory_id == forgotten.memory_id)
        .expect("the forgotten memory is still listed");
    assert_eq!(tombstone.forget_state, ForgetState::Forgotten);

    assert_eq!(
        store
            .inference_state(forgotten_inference)
            .expect("state of the inference that rested on the forgotten evidence"),
        InferenceState::Orphaned,
        "an inference whose evidence can no longer be produced must be demoted, \
         not quietly kept as if it were still supported",
    );
    assert_eq!(
        store
            .inference_state(kept_inference)
            .expect("state of the other inference"),
        InferenceState::Live,
    );

    let audit = store.list_audit().expect("audit survives the forget");
    store
        .verify_audit_chain()
        .expect("the chain must still verify after a forget and a restart");
    assert_eq!(
        audit.len(),
        3,
        "two creates and the forget; forgetting deletes no audit entry",
    );
    let forget_entry = audit
        .iter()
        .find(|entry| entry.action == AuditAction::ForgetExecute)
        .expect("the forget is on the record");
    assert_eq!(
        forget_entry.subject_refs.as_deref(),
        Some([forgotten.memory_id].as_slice()),
        "the bare uuid of a forgotten memory may, and should, outlive its words",
    );
    assert_eq!(
        forget_entry.counts.as_ref().and_then(|counts| counts.items),
        Some(1),
        "one content key destroyed",
    );

    let encoded = serde_json::to_string(&audit).expect("audit serializes");
    for prose in fixture.prose() {
        assert!(
            !encoded.contains(prose),
            "the audit chain must never have carried the prose in the first \
             place, which is why forgetting does not have to reach it",
        );
    }
}

#[test]
fn a_forgotten_memory_cannot_be_read_edited_or_forgotten_a_second_time() {
    let fixture = MemoryFixture::load();
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path().join("twice.db"));

    let memory = create(&mut store, &fixture.get("kitchen_argument").draft(), NOW).expect("create");
    let first = forget(&mut store, memory.memory_id, NOW).expect("forget");
    assert!(!store.has_content_key(memory.content_key_id));

    assert!(matches!(
        read(&store, memory.memory_id),
        Err(MemoryError::Forgotten(_)),
    ));
    assert!(
        matches!(
            update(
                &mut store,
                memory.memory_id,
                &soul_memory::MemoryEdit::title("改个标题"),
                NOW,
            ),
            Err(MemoryError::Forgotten(_)),
        ),
        "an edit after a forget would seal new text under a destroyed key and \
         quietly fail, or worse, mint a new one",
    );

    // Forgetting again is not an error — retrying after a crash has to be
    // safe. The receipt still names the key, because the key-to-memory mapping
    // is part of the tombstone, but nothing is destroyed the second time:
    // that is what the blob count dropping to zero says.
    assert_eq!(first.impact.sealed_blobs_destroyed, 2);
    let again = forget(&mut store, memory.memory_id, NOW).expect("forget again");
    assert_eq!(again.impact.content_key_ids, first.impact.content_key_ids);
    assert_eq!(
        again.impact.sealed_blobs_destroyed, 0,
        "the ciphertext went with the first forget; a second one has nothing \
         left to take",
    );
    assert!(!store.has_content_key(memory.content_key_id));
    assert!(matches!(
        read(&store, memory.memory_id),
        Err(MemoryError::Forgotten(_)),
    ));

    assert!(
        matches!(
            forget(&mut store, Uuid::now_v7(), NOW),
            Err(MemoryError::Store(StoreError::NotFound { .. })),
        ),
        "forgetting a memory that never existed is a not-found, not a receipt \
         for imaginary destruction",
    );
}
