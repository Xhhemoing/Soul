//! The WP02 command surface, exercised end to end on a Linux host.
//!
//! This is the shape WP09 will bind the desktop shell to, so it has to work
//! without a platform key store and without a UI. Everything it proves is
//! already proved inside `soul-store`; what is checked here is that the thin
//! wrappers reach it, and that opening a store in a directory really produces
//! `soul.db` in that directory rather than somewhere else.

use soul_schema::common::{
    ActorSubject, Derivation, E0Deny, E1Disposition, EgressPolicy, Privacy, Purpose,
    ResearchDisposition, Retention, SchemaVersion, SealedSubject, Subject, Timestamp,
};
use soul_schema::event::{EventKind, EventSource, SoulEvent};
use soul_schema::memory::{ForgetState, MemoryType, SoulMemory};
use soul_store::{DpapiKeyProvider, KeyProvider, TestKeyProvider};
use soul_store_api::forget::ForgetUnit;
use soul_store_api::research::ResearchPreviewRequest;
use soul_store_api::types::{SealRequest, StoreError};
use soul_store_api::{BlobStore, EventStore, MemoryStore, SoulStore};
use soulcore::commands::session::{Session, KEY_BLOB_FILE_NAME};
use soulcore::commands::store as store_commands;
use uuid::Uuid;

const SEED: &str = "soulcore store commands";
const SUMMARY: &str = "在城郊的旧书店里待了一下午";

#[test]
fn open_preview_forget_and_preview_research_all_work_headless() {
    let dir = tempfile::tempdir().expect("temp dir");

    let memory_id = Uuid::now_v7();
    let content_key_id = Uuid::now_v7();

    let sealed = {
        let mut store = store_commands::open_test_store(dir.path(), SEED).expect("open");
        assert_eq!(
            store.path(),
            store_commands::database_path(dir.path()),
            "the database belongs in the directory the caller named",
        );

        let sealed = store
            .seal(SealRequest::new(
                content_key_id,
                memory_id,
                "summary_ref",
                SealedSubject::Owner,
                SUMMARY.as_bytes().to_vec(),
            ))
            .expect("seal");
        store
            .put_memory(SoulMemory {
                schema_version: SchemaVersion,
                memory_id,
                memory_type: MemoryType::Episodic,
                title_ref: None,
                summary_ref: Some(sealed.clone()),
                source_event_ids: None,
                evidence_ids: None,
                content_key_id,
                forget_state: ForgetState::Active,
                third_party_content_present: Some(false),
            })
            .expect("memory");
        store
            .append_event(owner_event("2026-08-24T15:00:00Z"))
            .expect("event");

        let impact =
            store_commands::preview_forget(&store, ForgetUnit::Memory(memory_id)).expect("preview");
        assert_eq!(impact.memories_affected, 1);
        assert_eq!(impact.sealed_blobs_destroyed, 1);

        let report = store_commands::research_preview(
            &store,
            &ResearchPreviewRequest::default().with_max_rows(10),
        )
        .expect("research preview");
        assert!(!report.manifest.rows.is_empty());
        assert!(!report.manifest.written_to_disk);

        let receipt = store_commands::execute_forget(&mut store, ForgetUnit::Memory(memory_id))
            .expect("execute");
        assert_eq!(receipt.impact, impact);

        store.flush().expect("flush");
        store.close().expect("close");
        sealed
    };

    let store = store_commands::open_test_store(dir.path(), SEED).expect("reopen");
    assert!(
        matches!(store.open(&sealed), Err(StoreError::ContentKeyDestroyed(_))),
        "the forget must still hold after the process that did it is gone",
    );
    store.verify_audit_chain().expect("chain");
}

/// D62 seen from the product side: a database written by a newer Soul is a
/// session with no store, not a session with a store it has quietly downgraded.
///
/// `soul-store` proves the refusal and proves that the file is untouched. What
/// is left to show is that the refusal survives the trip to
/// [`Session::status`], where the shell reads it — no new field, no new command:
/// `store_opened` is already false whenever the open failed, and the reason it
/// failed is already inside `store_notice`. The only thing this pins is that the
/// reason still names the version, because "the store did not open" without it
/// is indistinguishable from a lost key file.
#[test]
fn a_database_from_a_newer_build_leaves_the_session_without_a_store_and_says_why() {
    let dir = tempfile::tempdir().expect("temp dir");
    let directory = std::fs::canonicalize(dir.path()).expect("canonical temp dir");

    let first = Session::open(&directory);
    assert!(
        first.status().store_opened,
        "{}",
        first.status().store_notice
    );
    drop(first);

    // The stamp a Soul several schemas ahead would have left behind, written
    // under the key material this machine's session uses. Session picks DPAPI
    // on Windows and the developer key file elsewhere — planting with the
    // other provider opens an undecryptable file (NotADatabase), not a stamp.
    let dek = if cfg!(windows) {
        DpapiKeyProvider::new(directory.join(KEY_BLOB_FILE_NAME))
            .database_key()
            .expect("the DPAPI key blob the session just created")
            .to_hex()
    } else {
        TestKeyProvider::in_dir(&directory)
            .database_key()
            .expect("the developer key file the session just created")
            .to_hex()
    };
    let conn = rusqlite::Connection::open(store_commands::database_path(&directory))
        .expect("open the database file directly");
    conn.execute_batch(&format!("PRAGMA key = \"x'{dek}'\";"))
        .expect("key the connection");
    conn.execute(
        "UPDATE meta SET value = '99' WHERE key = 'schema_version'",
        [],
    )
    .expect("plant a newer schema version");
    conn.close().expect("close");

    let status = Session::open(&directory).status();
    assert!(
        !status.store_opened,
        "the session opened a database written by a build that knew more than it does",
    );
    assert!(
        status.store_notice.contains("schema_version") && status.store_notice.contains("99"),
        "the notice has to say the database is from another version: {}",
        status.store_notice,
    );
}

/// A collected hour, written the way `soul-collect` writes one.
///
/// `research_export: bucket` rather than the default `deny`, because the
/// rollup reads that field: naming `Purpose::Research` and then storing the
/// row as denied is exactly the combination that produced an always-empty
/// preview.
fn owner_event(ts: &str) -> SoulEvent {
    SoulEvent {
        schema_version: SchemaVersion,
        event_id: Uuid::now_v7(),
        ts: Timestamp::new(ts),
        source: EventSource::CollectorForegroundApp,
        kind: EventKind::AppForeground,
        actor_subject: ActorSubject::Owner,
        consent_id: None,
        privacy: Privacy {
            subject: Subject::Owner,
            derivation: Derivation::Raw,
            purposes: vec![Purpose::Research],
            retention: Retention::until_forgotten(),
            egress: EgressPolicy {
                e0: E0Deny,
                e1: E1Disposition::Deny,
                research_export: ResearchDisposition::Bucket,
            },
        },
        body_ref: None,
    }
}
