//! Command mapping and real Session/store regressions; synthetic temporary data only.
use super::*;
use soul_store_api::AuditLog;

#[test]
fn outcome_mapping_never_turns_followup_uncertainty_into_success() {
    let id = Uuid::now_v7();
    let impact = ForgetImpact { content_key_ids: vec![Uuid::now_v7()], ..Default::default() };
    for cleanup in [ForgetCleanup::Complete, ForgetCleanup::Pending { checkpoint: None }] {
        for audit in [ForgetAudit::Recorded, ForgetAudit::Unconfirmed] {
            let receipt = ForgetCommandReceipt {
                unit: ForgetUnit::Memory(id), impact: impact.clone(), cleanup: cleanup.clone(), audit,
            };
            let view = ForgetReceiptView::of(id, &receipt, &impact);
            assert!(view.logical_committed);
            assert!(view.matched_preview);
            assert_eq!(view.cleanup, cleanup);
            assert_eq!(view.audit, audit);
            assert_eq!(view.content_keys_destroyed, 1);
            let encoded = serde_json::to_value(&view).expect("serialize outcome DTO");
            assert_eq!(encoded["logical_committed"], true);
            assert!(encoded.get("title").is_none());
            assert!(encoded.get("summary").is_none());
        }
    }
}

fn write(session: &mut Session) -> MemoryDetail {
    session.write_memory(&NewMemory {
        memory_type: "episodic".into(), title: "synthetic title".into(),
        summary: "synthetic memory; no real personal data".into(),
    }).expect("write temporary memory")
}

#[test]
fn session_uses_outcome_service_and_cleanup_does_not_repeat_audit() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let mut session = Session::open(directory.path());
    let detail = write(&mut session);
    let preview = session.preview_forget(&detail.memory_id).expect("preview");
    let confirmation = ForgetConfirmation { preview_id: preview.preview_id, memory_id: detail.memory_id.clone() };
    let receipt = session.forget_memory(&confirmation).expect("committed outcome");
    assert!(receipt.logical_committed && receipt.matched_preview);
    assert_eq!(receipt.cleanup, ForgetCleanup::Complete);
    assert_eq!(receipt.audit, ForgetAudit::Recorded);
    assert!(session.memory(&detail.memory_id).is_err());
    let shared = session.store().expect("same store");
    let before = shared.lock().expect("lock").list_audit().expect("audit");
    for _ in 0..2 {
        assert_eq!(retry_cleanup_for_session(&session).expect("cleanup").cleanup, ForgetCleanup::Complete);
    }
    let after = shared.lock().expect("lock").list_audit().expect("audit");
    assert_eq!(serde_json::to_value(before).unwrap(), serde_json::to_value(after).unwrap());
    assert!(session.forget_memory(&confirmation).is_err(), "a receipt is not permission to destroy again");
}

#[test]
fn cleanup_after_reopen_does_not_need_a_destructive_confirmation() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let id = {
        let mut session = Session::open(directory.path());
        let detail = write(&mut session);
        let preview = session.preview_forget(&detail.memory_id).expect("preview");
        session.forget_memory(&ForgetConfirmation {
            preview_id: preview.preview_id, memory_id: detail.memory_id.clone(),
        }).expect("forget");
        detail.memory_id
    };
    let session = Session::open(directory.path());
    assert_eq!(retry_cleanup_for_session(&session).expect("cleanup without receipt").cleanup, ForgetCleanup::Complete);
    assert!(session.memory(&id).is_err());
}

#[test]
fn direct_command_refuses_a_second_destruction_without_new_audit() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let mut session = Session::open(directory.path());
    let detail = write(&mut session);
    let shared = session.store().expect("store");
    let mut store = shared.lock().expect("lock");
    let id = detail.memory_id.parse().expect("UUID");
    forget(&mut store, id, 1_725_000_000).expect("first destruction");
    let before = store.list_audit().expect("audit");
    assert!(matches!(forget(&mut store, id, 1_725_000_001), Err(MemoryError::Forgotten(_))));
    assert_eq!(serde_json::to_value(before).unwrap(), serde_json::to_value(store.list_audit().unwrap()).unwrap());
}
