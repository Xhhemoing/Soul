//! The WP02 command surface, exercised end to end on a Linux host.
//!
//! This is the shape WP09 will bind the desktop shell to, so it has to work
//! without a platform key store and without a UI. Everything it proves is
//! already proved inside `soul-store`; what is checked here is that the thin
//! wrappers reach it, and that opening a store in a directory really produces
//! `soul.db` in that directory rather than somewhere else.

use soul_schema::common::{
    ActorSubject, Derivation, EgressPolicy, Privacy, Purpose, Retention, SchemaVersion,
    SealedSubject, Subject, Timestamp,
};
use soul_schema::event::{EventKind, EventSource, SoulEvent};
use soul_schema::memory::{ForgetState, MemoryType, SoulMemory};
use soul_store_api::forget::ForgetUnit;
use soul_store_api::research::ResearchPreviewRequest;
use soul_store_api::types::{SealRequest, StoreError};
use soul_store_api::{BlobStore, EventStore, MemoryStore, SoulStore};
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
            egress: EgressPolicy::default(),
        },
        body_ref: None,
    }
}
