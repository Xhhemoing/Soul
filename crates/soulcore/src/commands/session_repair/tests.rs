use super::*;
use rusqlite::{types::Value, Connection};
use soul_schema::memory::SoulMemory;
use soul_store_api::MemoryStore;

fn scratch() -> (tempfile::TempDir, Session, Connection) {
    let directory = tempfile::tempdir().unwrap();
    let session = Session::open(directory.path());
    assert!(session.status().store_opened);
    // Fault injection uses only this temporary Session's newly created keys.
    let (keys, _) = key_provider(directory.path());
    let conn = Connection::open(store_commands::database_path(directory.path())).unwrap();
    conn.execute_batch(&format!(
        "PRAGMA key = \"x'{}'\";",
        keys.database_key().unwrap().to_hex()
    ))
    .unwrap();
    (directory, session, conn)
}

fn memory(session: &mut Session) -> (Uuid, MemoryDetail) {
    let detail = session
        .write_memory(&NewMemory {
            memory_type: "semantic".into(),
            title: "synthetic title".into(),
            summary: "synthetic summary".into(),
        })
        .unwrap();
    (Uuid::parse_str(&detail.memory_id).unwrap(), detail)
}

fn stored(session: &Session, id: Uuid) -> SoulMemory {
    hold(&session.opened_store().unwrap())
        .get_memory(id)
        .unwrap()
}

fn corrupt(conn: &Connection, memory: &SoulMemory) {
    let id = memory.summary_ref.as_ref().unwrap().blob_id.to_string();
    let mut bytes: Vec<u8> = conn
        .query_row(
            "SELECT ciphertext FROM sealed_blobs WHERE blob_id=?1",
            [&id],
            |row| row.get(0),
        )
        .unwrap();
    bytes[0] ^= 1;
    assert_eq!(
        conn.execute(
            "UPDATE sealed_blobs SET ciphertext=?1 WHERE blob_id=?2",
            rusqlite::params![bytes, id],
        )
        .unwrap(),
        1
    );
}

// All persisted tables, including complete audit and key rows; comparisons
// intentionally avoid Debug output containing ciphertext or wrapped keys.
fn rows(conn: &Connection) -> Vec<Vec<Vec<Value>>> {
    let tables = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    tables
        .iter()
        .map(|table| {
            let quoted = table.replace('"', "\"\"");
            let mut statement = conn
                .prepare(&format!("SELECT * FROM \"{quoted}\" ORDER BY rowid"))
                .unwrap();
            let columns = statement.column_count();
            let rows = statement
                .query_map([], |row| (0..columns).map(|index| row.get(index)).collect())
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            rows
        })
        .collect()
}

#[test]
fn n2_restores_summary_through_real_session_and_read() {
    let (_dir, mut session, conn) = scratch();
    let (id, detail) = memory(&mut session);
    assert_eq!(session.memory(&detail.memory_id).unwrap(), detail);
    let request = session.prepare_repair(id).expect("prepare normal snapshot");
    let before = rows(&conn);
    let original = stored(&session, id);
    corrupt(&conn, &original);
    assert!(session.memory(&detail.memory_id).is_err());
    assert_eq!(session.confirm_repair(request, id).unwrap(), 1);
    assert_eq!(session.memory(&detail.memory_id).unwrap(), detail);
    assert!(stored(&session, id) == original);
    assert!(rows(&conn) == before);
    assert!(session.confirm_repair(request, id).is_err());
    assert!(rows(&conn) == before);
}

#[test]
fn n2_cancel_before_confirm_refuses_and_after_commit_does_not_undo() {
    for cancel_first in [true, false] {
        let (_dir, mut session, conn) = scratch();
        let (id, detail) = memory(&mut session);
        let request = session.prepare_repair(id).unwrap();
        corrupt(&conn, &stored(&session, id));
        if cancel_first {
            let broken = rows(&conn);
            session.cancel_repair(request, id);
            assert!(session.confirm_repair(request, id).is_err());
            assert!(session.memory(&detail.memory_id).is_err());
            assert!(rows(&conn) == broken);
        } else {
            assert_eq!(session.confirm_repair(request, id).unwrap(), 1);
            let committed = rows(&conn);
            session.cancel_repair(request, id);
            assert!(session.confirm_repair(request, id).is_err());
            assert_eq!(session.memory(&detail.memory_id).unwrap(), detail);
            assert!(rows(&conn) == committed);
        }
    }
}

#[test]
fn n2_replacement_and_failed_prepare_destroy_old_identity() {
    let (_dir, mut session, conn) = scratch();
    let (a, _) = memory(&mut session);
    let (b, _) = memory(&mut session);
    let old = session.prepare_repair(a).unwrap();
    let current = session.prepare_repair(b).unwrap();
    assert!(session.confirm_repair(old, a).is_err());
    assert!(session.confirm_repair(current, a).is_err());
    session.cancel_repair(old, b);
    corrupt(&conn, &stored(&session, b));
    assert_eq!(session.confirm_repair(current, b).unwrap(), 1);
    let old = session.prepare_repair(a).unwrap();
    assert!(session.prepare_repair(Uuid::now_v7()).is_err());
    assert!(session.confirm_repair(old, a).is_err());
    let first = session.prepare_repair(a).unwrap();
    let next = session.prepare_repair(a).unwrap();
    assert_ne!(first, next);
    assert!(session.confirm_repair(first, a).is_err());
    assert_eq!(session.confirm_repair(next, a).unwrap(), 0);
}

#[test]
fn n2_related_edit_even_unchanged_or_failed_consumes_old_request() {
    let changes = [
        MemoryChange::default(),
        MemoryChange {
            memory_type: Some("semantic".into()),
            ..Default::default()
        },
        MemoryChange {
            summary: Some("synthetic summary".into()),
            ..Default::default()
        },
        MemoryChange {
            summary: Some("changed synthetic summary".into()),
            ..Default::default()
        },
        MemoryChange {
            memory_type: Some("invalid type".into()),
            ..Default::default()
        },
    ];
    for (index, change) in changes.iter().enumerate() {
        let (_dir, mut session, conn) = scratch();
        let (id, detail) = memory(&mut session);
        let original = stored(&session, id);
        let request = session.prepare_repair(id).unwrap();
        let edited = session.edit_memory(&detail.memory_id, change);
        assert_eq!(edited.is_err(), index == 4);
        if index == 3 {
            session
                .edit_memory(
                    &detail.memory_id,
                    &MemoryChange {
                        summary: Some(detail.summary.clone()),
                        ..Default::default()
                    },
                )
                .unwrap();
        }
        if index == 0 || index == 1 || index == 4 {
            assert!(stored(&session, id) == original);
        }
        corrupt(&conn, &stored(&session, id));
        let before = rows(&conn);
        assert!(session.confirm_repair(request, id).is_err());
        assert!(rows(&conn) == before);
    }
}

#[test]
fn n2_forget_rejects_old_request_and_keeps_independent_memory() {
    let (_dir, mut session, conn) = scratch();
    let (id, detail) = memory(&mut session);
    let (_, independent) = memory(&mut session);
    let original = stored(&session, id);
    let request = session.prepare_repair(id).unwrap();
    let preview = session.preview_forget(&detail.memory_id).unwrap();
    session
        .forget_memory(&ForgetConfirmation {
            preview_id: preview.preview_id,
            memory_id: detail.memory_id.clone(),
        })
        .unwrap();
    let forgotten = rows(&conn);
    assert!(session.confirm_repair(request, id).is_err());
    assert!(session.memory(&detail.memory_id).is_err());
    assert_eq!(session.memory(&independent.memory_id).unwrap(), independent);
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM content_keys WHERE content_key_id=?1",
            [original.content_key_id.to_string()],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM sealed_blobs WHERE content_key_id=?1",
            [original.content_key_id.to_string()],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert!(rows(&conn) == forgotten);
}

#[test]
fn n2_unrelated_memory_and_profile_writes_preserve_request() {
    let (_dir, mut session, conn) = scratch();
    let (id, detail) = memory(&mut session);
    let (_, other) = memory(&mut session);
    let request = session.prepare_repair(id).unwrap();
    let (_, new) = memory(&mut session);
    let edited = session
        .edit_memory(
            &other.memory_id,
            &MemoryChange {
                summary: Some("independent edited summary".into()),
                ..Default::default()
            },
        )
        .unwrap();
    let profile = session.set_voice("warmth", "warm").unwrap();
    let before = rows(&conn);
    corrupt(&conn, &stored(&session, id));
    assert_eq!(session.confirm_repair(request, id).unwrap(), 1);
    assert_eq!(session.memory(&detail.memory_id).unwrap(), detail);
    assert_eq!(session.memory(&new.memory_id).unwrap(), new);
    assert_eq!(session.memory(&other.memory_id).unwrap(), edited);
    assert_eq!(session.profile().unwrap(), profile);
    assert!(rows(&conn) == before);
}

#[test]
fn n2_linked_sources_refused_at_capture_and_after_prepare() {
    for evidence in [false, true] {
        for after_prepare in [false, true] {
            let (_dir, mut session, conn) = scratch();
            let (id, detail) = memory(&mut session);
            let request = after_prepare.then(|| session.prepare_repair(id).unwrap());
            let mut linked = stored(&session, id);
            if evidence {
                linked.evidence_ids = Some(vec![Uuid::now_v7()]);
            } else {
                linked.source_event_ids = Some(vec![Uuid::now_v7()]);
            }
            hold(&session.opened_store().unwrap())
                .put_memory(linked.clone())
                .unwrap();
            // The ordinary read succeeds even for unsupported linked sources.
            assert_eq!(session.memory(&detail.memory_id).unwrap(), detail);
            if let Some(request) = request {
                corrupt(&conn, &linked);
                let before = rows(&conn);
                assert!(session.confirm_repair(request, id).is_err());
                assert!(session.confirm_repair(request, id).is_err());
                assert!(rows(&conn) == before);
            } else {
                let before = rows(&conn);
                assert!(session.prepare_repair(id).is_err());
                assert!(rows(&conn) == before);
            }
        }
    }
}

#[test]
fn n2_transaction_failure_consumes_confirmation_without_writes() {
    let (_dir, mut session, conn) = scratch();
    let (id, detail) = memory(&mut session);
    let request = session.prepare_repair(id).unwrap();
    corrupt(&conn, &stored(&session, id));
    conn.execute_batch("CREATE TRIGGER n2_fail BEFORE UPDATE OF ciphertext ON sealed_blobs BEGIN SELECT RAISE(ABORT, 'synthetic failure'); END;").unwrap();
    let before = rows(&conn);
    assert!(session.confirm_repair(request, id).is_err());
    assert!(rows(&conn) == before);
    conn.execute_batch("DROP TRIGGER n2_fail").unwrap();
    assert!(session.confirm_repair(request, id).is_err());
    assert!(session.memory(&detail.memory_id).is_err());
    assert!(rows(&conn) == before);
}

#[test]
fn n2_drop_open_cannot_reuse_original_request() {
    let (dir, mut session, conn) = scratch();
    let (id, detail) = memory(&mut session);
    let request = session.prepare_repair(id).unwrap();
    corrupt(&conn, &stored(&session, id));
    drop(conn);
    drop(session);
    let mut reopened = Session::open(dir.path());
    assert!(reopened.status().store_opened);
    assert!(reopened.confirm_repair(request, id).is_err());
    assert!(reopened.prepare_repair(id).is_err());
    assert!(reopened.memory(&detail.memory_id).is_err());
}

fn context(session: &mut Session, id: Uuid) -> String {
    session
        .prepare_memory_context(MemoryContextRequest {
            candidate_ids: vec![id],
            query: None,
            memory_type: None,
            time_specific: false,
        })
        .unwrap()
        .context_id
        .unwrap()
        .to_string()
}

#[test]
fn n2_confirmation_success_or_failure_invalidates_old_held_context() {
    for fail in [false, true] {
        let (_dir, mut session, conn) = scratch();
        let (id, _) = memory(&mut session);
        let request = session.prepare_repair(id).unwrap();
        let context_id = context(&mut session, id);
        corrupt(&conn, &stored(&session, id));
        if fail {
            conn.execute_batch("CREATE TRIGGER n2_fail BEFORE UPDATE OF ciphertext ON sealed_blobs BEGIN SELECT RAISE(ABORT, 'synthetic failure'); END;").unwrap();
        }
        assert_eq!(session.confirm_repair(request, id).is_err(), fail);
        assert_eq!(
            session
                .draft_pasted_with_memory("synthetic paste", &context_id)
                .unwrap_err()
                .reason_code,
            "MEMORY_CONTEXT_CHANGED"
        );
    }
}

#[test]
fn n2_old_endpoint_approval_never_revives_after_success_or_failure() {
    for fail in [false, true] {
        let endpoint = soul_testkit::mock_llm::MockLlm::start().unwrap();
        let (_dir, mut session, conn) = scratch();
        let (id, _) = memory(&mut session);
        session.set_user_endpoint(&endpoint.base_url()).unwrap();
        let request = session.prepare_repair(id).unwrap();
        let approval = session
            .prepare_draft("synthetic paste", None)
            .unwrap()
            .approval();
        corrupt(&conn, &stored(&session, id));
        if fail {
            conn.execute_batch("CREATE TRIGGER n2_fail BEFORE UPDATE OF ciphertext ON sealed_blobs BEGIN SELECT RAISE(ABORT, 'synthetic failure'); END;").unwrap();
        }
        assert_eq!(session.confirm_repair(request, id).is_err(), fail);
        assert!(session.generate_draft(&approval).is_err());
        assert!(session.generate_draft(&approval).is_err());
        assert_eq!(endpoint.request_count(), 0);
    }
}

#[test]
fn n2_busy_physical_worker_retained_and_late_completion_refused() {
    let endpoint = soul_testkit::mock_llm::MockLlm::start().unwrap();
    let (_dir, mut session, conn) = scratch();
    let (id, detail) = memory(&mut session);
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
    let request = session.prepare_repair(id).unwrap();
    let approval = session
        .prepare_draft("synthetic paste", None)
        .unwrap()
        .approval();
    let work = session.begin_draft(&approval).unwrap();
    let worker_id = session.active_draft.as_ref().unwrap().id;
    corrupt(&conn, &stored(&session, id));
    assert!(session.confirm_repair(request, id).is_err());
    assert_eq!(session.active_draft.as_ref().unwrap().id, worker_id);
    assert!(!session
        .active_draft
        .as_ref()
        .unwrap()
        .finished
        .load(Ordering::Acquire));
    assert!(session
        .prepare_draft("another synthetic paste", None)
        .is_err());
    let completion = work.execute();
    assert_eq!(endpoint.request_count(), 1);
    assert_eq!(session.active_draft.as_ref().unwrap().id, worker_id);
    assert!(session.confirm_repair(request, id).is_err());
    assert!(session.complete_draft(completion).is_err());
    assert!(session.active_draft.is_none());
    assert!(session.confirm_repair(request, id).is_err());
    assert!(session.memory(&detail.memory_id).is_err());
    let audit = session.audit().unwrap();
    assert!(audit.verified);
    assert!(!audit.entries.iter().any(|e| e.action == "draft.create"));
    assert_eq!(
        audit
            .entries
            .iter()
            .filter(|e| e.action == "egress.request" && e.decision == "allowed")
            .count(),
        1
    );
}
