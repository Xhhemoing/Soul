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
//!
//! The last two tests are about what the delete leaves on disk rather than what
//! the API answers afterwards: a freed page whose old bytes are still legible,
//! or a write-ahead log still holding the page as it was, would satisfy every
//! assertion above and still keep the wrapped content key inside a file the
//! database key opens.

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
        store
            .get_memory(id(MEMORY_TRIP))
            .expect("row remains")
            .forget_state,
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

/// Destroying the wrapped key only means something if the bytes go with it.
///
/// Two things have to be true for that, and they are true for different
/// reasons. The freed page content has to be zeroed rather than left on the
/// free list: SQLCipher already does that when its codec attaches, and the
/// store now asks for it by name and refuses to open without it, so the first
/// assertion below is what stops that turning back into an accident. The
/// write-ahead log has to be folded back in: nothing was doing that, and the
/// frames written before the delete carry the page exactly as it was while the
/// wrapped key sat on it. Both files open under the DEK, so either one left
/// alone hands the wrapped key back to a reader the threat model already
/// assumes has the database key.
#[test]
fn a_forget_zeroes_the_freed_bytes_and_leaves_no_write_ahead_log_behind() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("forget-secure-delete.db");
    let wal = dir.path().join("forget-secure-delete.db-wal");
    let keys = TestKeyProvider::from_seed(SEED);

    let mut store = SqlCipherStore::open(&path, &keys).expect("open");
    assert_eq!(
        store.secure_delete().expect("read the pragma back"),
        1,
        "the connection has to zero freed page content, not add it to the free list",
    );

    seed(&mut store);
    assert!(
        wal.metadata().map(|meta| meta.len()).unwrap_or(0) > 0,
        "the fixture must leave frames in the write-ahead log, or the assertion below \
         would pass against a store that never wrote one",
    );

    store
        .execute_forget(ForgetUnit::Memory(id(MEMORY_TRIP)))
        .expect("execute forget");

    // No `flush()` here on purpose: the forget itself has to do this, because
    // between the commit and whenever the application next happens to flush,
    // the log is exactly where the wrapped key still is.
    assert_eq!(
        wal.metadata()
            .expect("the log file survives a truncation")
            .len(),
        0,
        "execute_forget must truncate the write-ahead log, which is what discards the \
         frames still carrying the page the wrapped key sat on",
    );

    // The pragma is a property of the connection, not of the file, so a
    // restart has to set it again before the next forget.
    store.close().expect("close, as if the application exited");
    let reopened = SqlCipherStore::open(&path, &keys).expect("reopen");
    assert_eq!(
        reopened.secure_delete().expect("read the pragma back"),
        1,
        "a reopened store must not be the weaker one",
    );
}

/// Why the test above stops at the pragma rather than reading the pages.
///
/// The assertion one would rather make is direct: after the forget and the
/// checkpoint, walk every page of the database as SQLCipher decrypts it — free
/// pages included — and show the wrapped key appears in none of them. SQLite
/// has one interface for that, the `sqlite_dbpage` virtual table, and the
/// build this crate links does not carry it. `libsqlite3-sys`'s bundled build
/// defines `SQLITE_ENABLE_DBSTAT_VTAB` and not `SQLITE_ENABLE_DBPAGE_VTAB`,
/// and turning the latter on is not free: `sqlite_dbpage` is writable, so a
/// build that carries it hands anything holding the DEK a way to rewrite raw
/// pages under the schema's back. Without it the pages are reachable only
/// through the file, and the file is ciphertext, so a scan there cannot tell a
/// zeroed free page from a full one.
///
/// What can still be shown, against the very library the store links, is what
/// the setting is worth: the same delete is run twice over a plain unencrypted
/// file, once with the pragma off and once with it on, and only the second
/// file has forgotten the row. The store's own connection is asserted to be on
/// that second side by the test above, and by `SqlCipherStore::open`, which
/// refuses to hand back a store whose connection answers anything else.
#[test]
fn this_build_has_no_page_reader_and_secure_delete_is_what_removes_the_bytes() {
    let dir = tempfile::tempdir().expect("temp dir");

    let probe = rusqlite::Connection::open_in_memory().expect("in-memory sqlite");
    assert!(
        probe.prepare("SELECT data FROM sqlite_dbpage").is_err(),
        "this build now carries sqlite_dbpage: replace the reasoning above with the scan \
         it makes possible — every decrypted page after a forget, and the wrapped key in \
         none of them",
    );

    let left_behind = bytes_after_a_delete(&dir.path().join("insecure.db"), "OFF");
    assert!(
        contains(&left_behind, WRAPPED_KEY_NEEDLE),
        "without the pragma this build leaves the deleted bytes in the file; if it stopped \
         doing that, this test no longer shows what the store's pragma is buying",
    );

    let zeroed = bytes_after_a_delete(&dir.path().join("secure.db"), "ON");
    assert!(
        !contains(&zeroed, WRAPPED_KEY_NEEDLE),
        "with the pragma on, the deleted bytes must be gone from the free page as well as \
         from the table",
    );
}

/// Stand-in for a wrapped content key: long enough that finding it in the file
/// cannot be a coincidence.
const WRAPPED_KEY_NEEDLE: &[u8] = b"SOUL-WP02-WRAPPED-CONTENT-KEY-NEEDLE-9f3c";

/// Write one row holding [`WRAPPED_KEY_NEEDLE`], delete it under the given
/// `secure_delete` setting, fold the log back in, and hand back the file.
///
/// Plain SQLite, no key: the point is what the page looks like underneath, and
/// under SQLCipher that is exactly what cannot be read from outside.
fn bytes_after_a_delete(path: &std::path::Path, secure_delete: &str) -> Vec<u8> {
    let conn = rusqlite::Connection::open(path).expect("open a plain sqlite file");
    conn.pragma_update(None, "journal_mode", "WAL")
        .expect("write-ahead log, as the store uses");
    conn.pragma_update(None, "secure_delete", secure_delete)
        .expect("set secure_delete");
    conn.execute_batch("CREATE TABLE content_keys (content_key_id TEXT PRIMARY KEY, wrapped BLOB)")
        .expect("create the table");
    conn.execute(
        "INSERT INTO content_keys (content_key_id, wrapped) VALUES ('ck', ?1)",
        [WRAPPED_KEY_NEEDLE],
    )
    .expect("write the wrapped key");
    checkpoint(&conn);

    conn.execute("DELETE FROM content_keys WHERE content_key_id = 'ck'", [])
        .expect("destroy it");
    checkpoint(&conn);
    conn.close().expect("close");

    std::fs::read(path).expect("read the file back")
}

fn checkpoint(conn: &rusqlite::Connection) {
    conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
        row.get::<_, i64>(0)
    })
    .expect("fold the write-ahead log back in");
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    needle.len() <= haystack.len()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
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
            .seal(third_party_seal(
                id("63"),
                id("40"),
                "display_label_ref",
                "李雷",
            ))
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

/// A contact the export never named still owns the key their message bodies
/// were sealed under.
///
/// Nothing in `contacts` can hold that key: the column there is the display
/// label's, and there is no label. The only record is the blob the importer
/// anchors against the contact's own row, so the forget has to reach it —
/// otherwise the preview shows zeros, the receipt says the person was
/// forgotten, and every sealed body they wrote still opens.
#[test]
fn forgetting_a_contact_with_no_label_destroys_the_bodies_sealed_under_their_key() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("forget-unlabelled-contact.db");
    let keys = TestKeyProvider::from_seed(SEED);

    const NAMELESS: &str = "43";
    const NAMELESS_KEY: &str = "64";
    const OTHER: &str = "44";
    const OTHER_KEY: &str = "65";
    const THEIR_MESSAGE: &str = "明天上午十点在公司门口见";
    const SOMEBODY_ELSE_MESSAGE: &str = "这周先把方案定下来";

    let (theirs, somebody_elses) = {
        let mut store = SqlCipherStore::open(&path, &keys).expect("open");

        // What an import of a file with no display names writes: a contact row
        // carrying no label, one blob anchoring the key to that row, and the
        // bodies under the same key.
        store
            .put_contact(contact(id(NAMELESS), ContactClass::ThirdParty))
            .expect("their contact row");
        store
            .seal(third_party_seal(
                id(NAMELESS_KEY),
                id(NAMELESS),
                "content_key_anchor",
                &id(NAMELESS).to_string(),
            ))
            .expect("anchor their key to their row");
        let theirs = store
            .seal(third_party_seal(
                id(NAMELESS_KEY),
                id("53"),
                "body_ref",
                THEIR_MESSAGE,
            ))
            .expect("seal what they wrote");

        // A second nameless contact, so a forget that reaches by row rather
        // than by key is visible.
        store
            .put_contact(contact(id(OTHER), ContactClass::ThirdParty))
            .expect("another contact row");
        store
            .seal(third_party_seal(
                id(OTHER_KEY),
                id(OTHER),
                "content_key_anchor",
                &id(OTHER).to_string(),
            ))
            .expect("anchor the other key");
        let somebody_elses = store
            .seal(third_party_seal(
                id(OTHER_KEY),
                id("54"),
                "body_ref",
                SOMEBODY_ELSE_MESSAGE,
            ))
            .expect("seal what somebody else wrote");

        // An edge between the two, and something inferred from it. SECURITY.md
        // states the whole testable semantics of a forget as three things —
        // the key is destroyed, the prose will not open, and what rested on
        // the evidence is demoted — and the third only happens if the contact
        // was reached at all.
        store
            .put_evidence(evidence(id("73"), Subject::ThirdParty))
            .expect("edge evidence");
        store
            .put_relationship(relationship(id("45"), id(OTHER), id(NAMELESS), &[id("73")]))
            .expect("edge");
        store
            .put_inference(inference(id("83"), &[id("73")]))
            .expect("inference resting on the edge");

        let impact = store
            .preview_impact(ForgetUnit::Contact(id(NAMELESS)))
            .expect("preview");
        assert_eq!(
            impact.content_key_ids,
            vec![id(NAMELESS_KEY)],
            "the key their bodies are under is theirs, label or no label",
        );
        assert_eq!(impact.contacts_affected, 1);
        assert_eq!(
            impact.sealed_blobs_destroyed, 2,
            "the anchor and the one body sealed under the same key",
        );
        assert_eq!(impact.inferences_orphaned, 1);

        let receipt = store
            .execute_forget(ForgetUnit::Contact(id(NAMELESS)))
            .expect("forget them");
        assert_eq!(receipt.impact, impact, "the receipt charges what it quoted");
        store.flush().expect("flush");
        store.close().expect("close");
        (theirs, somebody_elses)
    };

    let store = SqlCipherStore::open(&path, &keys).expect("reopen");
    assert!(
        matches!(store.open(&theirs), Err(StoreError::ContentKeyDestroyed(_))),
        "a body sealed under a forgotten contact's key must not open again",
    );
    assert!(!store.has_content_key(id(NAMELESS_KEY)));
    assert_eq!(
        store.get_contact(id(NAMELESS)).expect("row").forget_state,
        ForgetState::Forgotten,
        "the row stays as a tombstone, and it has to say it was forgotten",
    );

    assert_eq!(
        String::from_utf8(store.open(&somebody_elses).expect("still readable")).expect("utf-8"),
        SOMEBODY_ELSE_MESSAGE,
        "forgetting one nameless contact must not touch another",
    );
    assert_eq!(
        store.get_contact(id(OTHER)).expect("row").forget_state,
        ForgetState::Active,
    );
    assert_eq!(
        store.inference_state(id("83")).expect("state"),
        InferenceState::Orphaned,
        "what rested on the edge to a forgotten person cannot go on standing",
    );
}
