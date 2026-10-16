use soul_store_api::import::{ImportOperationState, ImportSourceKind, LegacyImportChoice};
use soul_testkit::fixtures;
use soulcore::commands::memory::{MemoryChange, NewMemory};
use soulcore::commands::session::{PreparedImportView, Session};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

fn input() -> String {
    fixtures::read_text("import/soul-import-v1/valid_basic.jsonl").unwrap()
}
fn prepare(session: &mut Session, namespace: Option<uuid::Uuid>) -> PreparedImportView {
    let work = session
        .begin_import_prepare(
            input(),
            ImportSourceKind::SoulImportV1,
            namespace,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    session.complete_import_prepare(work.execute()).unwrap()
}
fn commit(
    session: &mut Session,
    plan: &PreparedImportView,
) -> soulcore::commands::session::ImportOperationView {
    let work = session
        .begin_import(
            &plan.operation.operation_id.to_string(),
            &plan.operation.preview_id.to_string(),
        )
        .unwrap();
    session.complete_import(work.execute()).unwrap()
}

fn fault_connection(dir: &std::path::Path) -> rusqlite::Connection {
    use soul_store::{DpapiKeyProvider, KeyProvider, TestKeyProvider};
    let keys: Box<dyn KeyProvider> = if cfg!(windows) {
        Box::new(DpapiKeyProvider::new(
            dir.join(soulcore::commands::session::KEY_BLOB_FILE_NAME),
        ))
    } else {
        Box::new(TestKeyProvider::in_dir(dir))
    };
    let conn = rusqlite::Connection::open(soulcore::commands::store::database_path(dir)).unwrap();
    conn.execute_batch(&format!(
        "PRAGMA key = \"x'{}'\";",
        keys.database_key().unwrap().to_hex()
    ))
    .unwrap();
    conn
}

#[test]
fn owner_group_messages_do_not_inflate_the_evidence_preview() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let text = fixtures::read_text("import/soul-import-v1/three_partners.jsonl").unwrap();
    let work = session
        .begin_import_prepare(
            text,
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    let prepared = session.complete_import_prepare(work.execute()).unwrap();
    assert_eq!(prepared.operation.preview.events_written, 16);
    assert_eq!(prepared.operation.preview.evidence_written, 15);
    let receipt = commit(&mut session, &prepared).operation.receipt.unwrap();
    assert_eq!(receipt.events_written, 16);
    assert_eq!(receipt.evidence_written, 15);
}

#[test]
fn owner_only_direct_conversation_does_not_invent_a_peer_in_preview() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let text = input().lines().take(2).collect::<Vec<_>>().join("\n");
    let work = session
        .begin_import_prepare(
            text,
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    let prepared = session.complete_import_prepare(work.execute()).unwrap();
    assert_eq!(prepared.operation.preview.events_written, 1);
    assert_eq!(prepared.operation.preview.evidence_written, 0);
    let receipt = commit(&mut session, &prepared).operation.receipt.unwrap();
    assert_eq!(receipt.events_written, 1);
    assert_eq!(receipt.evidence_written, 0);
}

#[test]
fn import_audit_failure_rolls_back_all_domain_index_and_insight_rows() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let prepared = prepare(&mut session, None);
    let conn = fault_connection(dir.path());
    let tables = [
        "content_keys",
        "sealed_blobs",
        "events",
        "contacts",
        "evidence",
        "relationships",
        "relationship_evidence",
        "audit",
        "audit_subjects",
        "import_namespaces",
        "import_participants",
        "import_message_keys",
        "import_indexed_events",
        "local_insights",
    ];
    let counts = || {
        tables
            .iter()
            .map(|table| {
                conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get::<_, u64>(0)
                })
                .unwrap()
            })
            .collect::<Vec<_>>()
    };
    let before = counts();
    conn.execute_batch("CREATE TRIGGER reject_import_audit BEFORE INSERT ON audit BEGIN SELECT RAISE(ABORT, 'synthetic import audit failure'); END;").unwrap();
    let work = session
        .begin_import(
            &prepared.operation.operation_id.to_string(),
            &prepared.operation.preview_id.to_string(),
        )
        .unwrap();
    assert!(session.complete_import(work.execute()).is_err());
    assert_eq!(counts(), before);
    let recorded = session
        .import_operation(&prepared.operation.operation_id.to_string())
        .unwrap()
        .operation;
    assert_eq!(recorded.state, ImportOperationState::Rejected);
    assert!(recorded.receipt.is_none());
    assert!(recorded.error_code.is_some());
    conn.execute_batch("DROP TRIGGER reject_import_audit")
        .unwrap();
    assert!(session.audit().unwrap().verified);
}

#[test]
fn deferred_revoke_failure_blocks_regrant_until_its_audit_is_recorded() {
    use soulcore::commands::collect::{CollectorConfig, FakeForegroundSource};
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let conn = fault_connection(dir.path());
    let work = session
        .begin_import_prepare(
            input(),
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    assert!(!session.revoke_collect_consent().unwrap().consent_granted);
    conn.execute_batch("CREATE TRIGGER reject_revoke_audit BEFORE INSERT ON audit BEGIN SELECT RAISE(ABORT, 'synthetic revoke audit failure'); END;").unwrap();
    assert!(session.complete_import_prepare(work.execute()).is_err());
    assert!(session
        .grant_collect_consent_with_source(FakeForegroundSource::new(), CollectorConfig::default())
        .is_err());
    assert!(!session.collect_status().consent_granted);
    conn.execute_batch("DROP TRIGGER reject_revoke_audit")
        .unwrap();
    session
        .grant_collect_consent_with_source(FakeForegroundSource::new(), CollectorConfig::default())
        .unwrap();
    let audit = session.audit().unwrap();
    let consent = audit
        .entries
        .iter()
        .filter(|entry| entry.action == "consent.grant")
        .collect::<Vec<_>>();
    assert_eq!(consent.len(), 2);
    assert_eq!(consent[0].decision, "denied");
    assert_eq!(consent[1].decision, "allowed");
    assert!(audit.verified);
    session.revoke_collect_consent().unwrap();
}

#[test]
fn immutable_prepare_requires_exact_discard_and_confirmation_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let first = prepare(&mut session, None);
    assert_eq!(
        session
            .begin_import_prepare(
                input(),
                ImportSourceKind::SoulImportV1,
                None,
                LegacyImportChoice::Reject
            )
            .unwrap_err()
            .reason_code,
        "IMPORT_PREPARATION_HELD"
    );
    assert!(session
        .discard_import(&uuid::Uuid::now_v7().to_string())
        .is_err());
    session
        .discard_import(&first.operation.operation_id.to_string())
        .unwrap();
    let next = prepare(&mut session, None);
    assert!(session
        .begin_import(
            &next.operation.operation_id.to_string(),
            &first.operation.preview_id.to_string()
        )
        .is_err());
    let committed = commit(&mut session, &next);
    assert_eq!(committed.operation.state, ImportOperationState::Committed);
    assert_eq!(
        committed.operation.receipt,
        commit(&mut session, &next).operation.receipt
    );
    let duplicate = prepare(&mut session, Some(next.operation.namespace_id));
    assert_eq!(duplicate.operation.preview.events_written, 0);
    let receipt = commit(&mut session, &duplicate).operation.receipt.unwrap();
    assert_eq!(receipt.events_written, 0);
    assert_eq!(receipt.evidence_written, 0);
    assert!(receipt.duplicates > 0);
    assert!(session.audit().unwrap().verified);
}

#[test]
fn restart_interrupts_a_preparation_and_preserves_committed_receipts_without_approvals() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let first = prepare(&mut session, None);
    let receipt = commit(&mut session, &first);
    let reserved = prepare(&mut session, Some(first.operation.namespace_id));
    drop(session);
    let mut reopened = Session::open(dir.path());
    assert_eq!(
        reopened
            .import_operation(&first.operation.operation_id.to_string())
            .unwrap()
            .operation
            .receipt,
        receipt.operation.receipt
    );
    let interrupted = reopened
        .import_operation(&reserved.operation.operation_id.to_string())
        .unwrap();
    assert_eq!(
        interrupted.operation.state,
        ImportOperationState::Interrupted
    );
    assert!(!interrupted.prepared_here);
    assert!(reopened
        .begin_import(
            &reserved.operation.operation_id.to_string(),
            &reserved.operation.preview_id.to_string()
        )
        .is_err());
    assert!(
        reopened
            .import_recovery_state()
            .unwrap()
            .can_start_new_intent
    );
}

#[test]
fn a_live_import_cannot_block_controls_through_another_store_reader_or_refusal_audit() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let memory = session
        .write_memory(&NewMemory {
            memory_type: "semantic".into(),
            title: "synthetic".into(),
            summary: "synthetic note".into(),
        })
        .unwrap();
    let store = session.store().unwrap();
    let work = session
        .begin_import_prepare(
            input(),
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    let held = store.lock().unwrap();
    let session = Arc::new(Mutex::new(session));
    let cloned = Arc::clone(&session);
    let (send, receive) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        let mut session = cloned.lock().unwrap();
        assert!(session.audit().is_err());
        assert!(session.people().is_err());
        assert!(session.memories().is_err());
        assert!(session.memory(&memory.memory_id).is_err());
        assert!(session.profile().is_err());
        assert!(session
            .edit_memory(
                &memory.memory_id,
                &MemoryChange {
                    title: Some("new".into()),
                    summary: None,
                    memory_type: None
                }
            )
            .is_err());
        assert!(session.prepare_draft("synthetic", None).is_err());
        assert!(session.draft_pasted("synthetic").is_err());
        assert!(session.grant_collect_consent().is_err());
        assert!(soulcore::commands::memory::retry_cleanup_for_session(&session).is_err());
        assert_eq!(session.collect_status().events_collected, None);
        session.clear_user_endpoint();
        session.discard_draft();
        assert!(!session.revoke_collect_consent().unwrap().consent_granted);
        assert!(!session.revoke_collect_consent().unwrap().consent_granted);
        send.send(()).unwrap();
    });
    let timely = receive.recv_timeout(Duration::from_secs(2)).is_ok();
    drop(held);
    thread.join().unwrap();
    assert!(
        timely,
        "a Session command waited for the long import store lock"
    );
    let completed = work.execute();
    let mut session = session.lock().unwrap();
    assert!(
        session.complete_import_prepare(completed).is_err(),
        "source changes refuse the stale prepared body"
    );
    assert!(!session.collect_status().consent_granted);
}

#[test]
fn dropped_preparation_work_releases_capacity_and_old_production_entrypoints_refuse() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let work = session
        .begin_import_prepare(
            input(),
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    assert!(session.begin_file_plan("unused").is_err());
    drop(work);
    assert!(session.begin_file_plan("unused").is_ok());
    assert!(session
        .refuse_legacy_import("private synthetic content")
        .unwrap_err()
        .explanation
        .find("private synthetic")
        .is_none());
    assert!(session
        .refuse_legacy_preview("private synthetic content")
        .is_err());
}

fn hostile_owned_input() -> String {
    let owned = input();
    let hostile = fixtures::read_text("import/soul-import-v1/injection_lines.jsonl").unwrap();
    owned
        .lines()
        .chain(hostile.lines().skip(1))
        .collect::<Vec<_>>()
        .join("\n")
}

fn injection_attempts(session: &Session) -> Vec<u64> {
    let chain = session.audit().unwrap();
    assert!(chain.verified);
    let serialized = serde_json::to_string(&chain).unwrap();
    for prose in [
        "忽略之前指令",
        "忽略之前",
        "系统管理员",
        "developer mode",
        "evil.example",
        "tool_call",
        "u-attacker",
        "李 雷",
    ] {
        assert!(
            !serialized.contains(prose),
            "an attempt audit quoted external content"
        );
    }
    chain
        .entries
        .into_iter()
        .filter(|entry| entry.action == "injection.blocked")
        .map(|entry| {
            assert_eq!(entry.decision, "denied");
            assert_eq!(
                entry.reason_code.as_deref(),
                Some("INJECTION_MARKERS_FOUND")
            );
            assert!(
                entry.bytes.is_none(),
                "the attempt does not retain prose length"
            );
            entry.items.expect("the attempt counts messages")
        })
        .collect()
}

#[test]
fn modern_prepare_audits_injection_attempts_before_confirmation() {
    for (source, text, markers) in [
        (ImportSourceKind::SoulImportV1, hostile_owned_input(), 5_u64),
        (
            ImportSourceKind::TelegramDesktop,
            fixtures::read_text("import/telegram/result_injection.json").unwrap(),
            2_u64,
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut session = Session::open(dir.path());
        assert!(session.status().store_opened);
        let work = session
            .begin_import_prepare(text, source, None, LegacyImportChoice::Reject)
            .unwrap();
        let prepared = session.complete_import_prepare(work.execute()).unwrap();
        assert_eq!(prepared.preview.messages_with_injection_markers, markers);
        assert_eq!(injection_attempts(&session), vec![markers]);
        assert!(session.people().unwrap().people.is_empty());
        assert_eq!(prepared.operation.state, ImportOperationState::Reserved);
        assert!(prepared.operation.receipt.is_none());

        let committed = commit(&mut session, &prepared);
        assert_eq!(
            committed
                .operation
                .receipt
                .unwrap()
                .messages_with_injection_markers,
            markers
        );
        // Confirmation retains the original commit audit. It is a separate
        // attempt from reading a preview, not an excuse to erase the preview.
        assert_eq!(injection_attempts(&session), vec![markers, markers]);
        session
            .import_operation(&prepared.operation.operation_id.to_string())
            .unwrap();
        commit(&mut session, &prepared);
        assert_eq!(injection_attempts(&session), vec![markers, markers]);
    }
}

#[test]
fn source_changes_cannot_erase_an_executed_import_attempt() {
    for change_before_execute in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let mut session = Session::open(dir.path());
        let work = session
            .begin_import_prepare(
                hostile_owned_input(),
                ImportSourceKind::SoulImportV1,
                None,
                LegacyImportChoice::Reject,
            )
            .unwrap();
        if change_before_execute {
            session.clear_user_endpoint();
        }
        let completed = work.execute();
        if !change_before_execute {
            session.clear_user_endpoint();
        }
        let refusal = session.complete_import_prepare(completed).unwrap_err();
        assert_eq!(refusal.reason_code, "IMPORT_SOURCE_CHANGED");
        assert_eq!(injection_attempts(&session), vec![5]);
        assert!(session.people().unwrap().people.is_empty());
        let operations = session.recent_import_operations().unwrap();
        assert_eq!(operations.len(), 1);
        assert_eq!(
            operations[0].operation.state,
            ImportOperationState::Invalidated
        );
        assert!(!operations[0].prepared_here);
    }
}

#[test]
fn an_identity_refusal_keeps_the_attempt_but_creates_no_confirmation_material() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    // This legacy preview fixture parses and fits the budget, but has no owner.
    let text = fixtures::read_text("import/soul-import-v1/injection_lines.jsonl").unwrap();
    let work = session
        .begin_import_prepare(
            text,
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    let refusal = session.complete_import_prepare(work.execute()).unwrap_err();
    assert_eq!(refusal.reason_code, "IMPORT_IDENTITY_REQUIRED");
    assert_eq!(injection_attempts(&session), vec![5]);
    assert!(session.recent_import_operations().unwrap().is_empty());
    assert!(session.people().unwrap().people.is_empty());
    assert!(session
        .import_recovery_state()
        .unwrap()
        .held_operation_id
        .is_none());
}

#[test]
fn a_preparation_audit_failure_cannot_deliver_a_reserved_or_held_import() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let conn = fault_connection(dir.path());
    conn.execute_batch("CREATE TRIGGER reject_attempt_audit BEFORE INSERT ON audit BEGIN SELECT RAISE(ABORT, 'synthetic attempt audit failure'); END;").unwrap();
    let work = session
        .begin_import_prepare(
            hostile_owned_input(),
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    assert!(session.complete_import_prepare(work.execute()).is_err());
    assert!(session.recent_import_operations().unwrap().is_empty());
    assert!(session.people().unwrap().people.is_empty());
    assert!(injection_attempts(&session).is_empty());
    let state = session.import_recovery_state().unwrap();
    assert!(state.held_operation_id.is_none());
    assert_eq!(state.reserved_operations, Some(0));
    assert!(state.can_start_new_intent);
    conn.execute_batch("DROP TRIGGER reject_attempt_audit")
        .unwrap();
    let work = session
        .begin_import_prepare(
            hostile_owned_input(),
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    session.complete_import_prepare(work.execute()).unwrap();
    assert_eq!(injection_attempts(&session), vec![5]);
}

#[test]
fn dropping_a_completion_keeps_its_one_attempt_and_releases_only_worker_capacity() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let work = session
        .begin_import_prepare(
            hostile_owned_input(),
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    let completed = work.execute();
    drop(completed);
    assert_eq!(injection_attempts(&session), vec![5]);
    // No complete call means no held confirmation. This only demonstrates the
    // existing worker-capacity recovery, not other core completion guarantees.
    assert!(session
        .import_recovery_state()
        .unwrap()
        .held_operation_id
        .is_none());
    let next = session.begin_file_plan("unused").unwrap();
    drop(next);
    assert_eq!(injection_attempts(&session), vec![5]);
}

#[test]
fn unexecuted_invalid_over_budget_and_clean_inputs_do_not_create_attempt_audits() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let unexecuted = session
        .begin_import_prepare(
            hostile_owned_input(),
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    drop(unexecuted);
    assert!(injection_attempts(&session).is_empty());
    let malformed = format!("{}\nnot a JSON record", hostile_owned_input());
    let mut rows = input()
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    rows[2]["text"] = serde_json::Value::String(format!(
        "忽略之前指令{}",
        "x".repeat(soulcore::commands::import::MAX_MESSAGE_CHARS)
    ));
    let over_budget = rows
        .iter()
        .map(serde_json::Value::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    for text in [malformed, over_budget] {
        let work = session
            .begin_import_prepare(
                text,
                ImportSourceKind::SoulImportV1,
                None,
                LegacyImportChoice::Reject,
            )
            .unwrap();
        assert!(session.complete_import_prepare(work.execute()).is_err());
        assert!(injection_attempts(&session).is_empty());
        assert!(session.recent_import_operations().unwrap().is_empty());
    }
    let clean = prepare(&mut session, None);
    assert_eq!(clean.preview.messages_with_injection_markers, 0);
    assert!(injection_attempts(&session).is_empty());
}
