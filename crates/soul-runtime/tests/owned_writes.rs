//! Real synthetic Session mutations: actor binding preserves the core's one-use slots.
use std::time::Duration;

use soul_runtime::{Caller, Runtime, RuntimeContext};
use soul_types::{BuiltinFixtureId, OperationOutcome, OperationPhase, SyntheticImportRequest};
use soulcore::commands::memory::{ForgetConfirmation, MemoryChange, NewMemory};
use soulcore::commands::session::{PreferenceOccasion, Session, VoiceRegister};
use uuid::Uuid;

fn contexts() -> (RuntimeContext, RuntimeContext) {
    (
        RuntimeContext::new(Caller::http(), None),
        RuntimeContext::new(Caller::desktop("main"), Some(Uuid::now_v7())),
    )
}

fn fixture() -> (tempfile::TempDir, Runtime) {
    let directory = tempfile::tempdir().unwrap();
    let mut session = Session::open(directory.path());
    assert!(session.status().store_opened);
    session.set_voice("register", "plain").unwrap();
    (directory, Runtime::new_synthetic(session))
}

fn imported(runtime: &Runtime, ctx: &RuntimeContext) -> String {
    let prepared = runtime
        .prepare_synthetic_import(
            ctx,
            SyntheticImportRequest {
                fixture_id: BuiltinFixtureId::Basic,
            },
        )
        .unwrap();
    runtime
        .confirm_import(
            ctx,
            &prepared.operation.operation_id.to_string(),
            &prepared.operation.preview_id.to_string(),
        )
        .unwrap();
    runtime.held_session().scoped_register_recipients().unwrap()[0]
        .person_id
        .to_string()
}

#[test]
fn foreign_import_confirm_cancel_and_prepare_cannot_spend_the_global_preparation() {
    let (directory, runtime) = fixture();
    let (http, desktop) = contexts();
    let prepared = runtime
        .prepare_synthetic_import(
            &http,
            SyntheticImportRequest {
                fixture_id: BuiltinFixtureId::Basic,
            },
        )
        .unwrap();
    let operation = prepared.operation.operation_id.to_string();
    let preview = prepared.operation.preview_id.to_string();
    assert_eq!(
        runtime
            .confirm_import(&desktop, &operation, &preview)
            .unwrap_err()
            .reason_code,
        "RUNTIME_CALLER_MISMATCH"
    );
    assert_eq!(
        runtime
            .discard_import(&desktop, &operation)
            .unwrap_err()
            .reason_code,
        "RUNTIME_CALLER_MISMATCH"
    );
    assert!(runtime
        .prepare_synthetic_import(
            &desktop,
            SyntheticImportRequest {
                fixture_id: BuiltinFixtureId::Basic
            }
        )
        .is_err());
    let committed = runtime.confirm_import(&http, &operation, &preview).unwrap();
    assert!(committed.operation.receipt.is_some());
    let again = runtime.confirm_import(&http, &operation, &preview).unwrap();
    assert_eq!(again.operation, committed.operation);
    assert_eq!(
        runtime.import_operation(&operation).unwrap().operation,
        committed.operation
    );
    drop(runtime);
    let reopened = Runtime::new_synthetic(Session::open(directory.path()));
    assert!(reopened
        .confirm_import(&http, &operation, &preview)
        .is_err());
    assert_eq!(
        reopened.import_operation(&operation).unwrap().operation,
        committed.operation
    );
}

#[test]
fn scoped_foreign_submit_never_consumes_and_same_owner_uuid_rules_stay_intact() {
    let (_directory, runtime) = fixture();
    let (http, desktop) = contexts();
    let person = imported(&runtime, &http);
    let drafted = runtime
        .draft_scoped_reply(
            &http,
            "SYNTHETIC_PRIVATE_PASTE",
            &person,
            PreferenceOccasion::Personal,
        )
        .unwrap();
    let proposal = runtime
        .propose_scoped_register(
            &http,
            &drafted.result_id.to_string(),
            1,
            VoiceRegister::Formal,
        )
        .unwrap();
    assert_eq!(
        runtime
            .confirm_scoped_register(
                &desktop,
                &proposal.proposal_id.to_string(),
                proposal.expected_revision
            )
            .unwrap_err()
            .reason_code,
        "RUNTIME_CALLER_MISMATCH"
    );
    assert_eq!(
        runtime
            .discard_scoped_register_proposal(&desktop, &proposal.proposal_id.to_string())
            .unwrap_err()
            .reason_code,
        "RUNTIME_CALLER_MISMATCH"
    );
    assert!(runtime
        .confirm_scoped_register(&http, "not-a-uuid", proposal.expected_revision)
        .is_err());
    assert_eq!(
        runtime
            .held_session()
            .runtime_pending_slots()
            .scoped_proposal,
        Some(proposal.proposal_id)
    );
    assert!(runtime
        .confirm_scoped_register(
            &http,
            &Uuid::now_v7().to_string(),
            proposal.expected_revision
        )
        .is_err());
    assert!(runtime
        .held_session()
        .runtime_pending_slots()
        .scoped_proposal
        .is_none());
    assert!(runtime
        .confirm_scoped_register(
            &http,
            &proposal.proposal_id.to_string(),
            proposal.expected_revision
        )
        .is_err());
    let next = runtime
        .propose_scoped_register(
            &http,
            &drafted.result_id.to_string(),
            2,
            VoiceRegister::Plain,
        )
        .unwrap();
    let receipt = runtime
        .confirm_scoped_register(&http, &next.proposal_id.to_string(), next.expected_revision)
        .unwrap();
    assert!(runtime
        .confirm_scoped_register(&http, &next.proposal_id.to_string(), next.expected_revision)
        .is_err());
    runtime
        .edit_memory(
            &desktop,
            &receipt.memory_id.to_string(),
            &MemoryChange {
                title: None,
                summary: Some("Synthetic ordinary edit".into()),
                memory_type: None,
            },
        )
        .unwrap();
    let status = runtime
        .scoped_register_status(&person, PreferenceOccasion::Personal)
        .unwrap();
    assert_eq!(
        status.state,
        soulcore::commands::session::ScopedRegisterStatusState::Inactive
    );
    let replacement = runtime
        .draft_scoped_reply(
            &desktop,
            "Synthetic replacement",
            &person,
            PreferenceOccasion::Personal,
        )
        .unwrap();
    assert!(runtime
        .propose_scoped_register(
            &http,
            &replacement.result_id.to_string(),
            3,
            VoiceRegister::Formal
        )
        .is_err());
    assert!(runtime
        .propose_scoped_register(
            &desktop,
            &replacement.result_id.to_string(),
            3,
            VoiceRegister::Formal
        )
        .is_ok());
}

#[test]
fn explicit_forget_replacement_transfers_owner_and_wrong_echo_retains_quote() {
    let (_directory, runtime) = fixture();
    let (http, desktop) = contexts();
    let memory = runtime
        .held_session()
        .write_memory(&NewMemory {
            memory_type: "episodic".into(),
            title: "Synthetic forget".into(),
            summary: "SYNTHETIC_PRIVATE_BODY".into(),
        })
        .unwrap();
    let first = runtime.preview_forget(&http, &memory.memory_id).unwrap();
    let old = ForgetConfirmation {
        preview_id: first.preview_id,
        memory_id: memory.memory_id.clone(),
    };
    let before_denial = runtime.held_session().audit().unwrap().entries.len();
    assert_eq!(
        runtime
            .forget_memory(&desktop, &old)
            .unwrap_err()
            .reason_code,
        "RUNTIME_CALLER_MISMATCH"
    );
    let audit = runtime.held_session().audit().unwrap();
    assert_eq!(audit.entries.len(), before_denial + 1);
    let denial = audit.entries.last().unwrap();
    assert_eq!(denial.action, "capability.reject");
    assert_eq!(denial.reason_code.as_deref(), Some("TOKEN_SCOPE_MISMATCH"));
    assert_eq!(denial.subject_refs.len(), 1);
    assert!(!serde_json::to_string(&audit)
        .unwrap()
        .contains("SYNTHETIC_PRIVATE_BODY"));
    let replacement = runtime.preview_forget(&desktop, &memory.memory_id).unwrap();
    assert_eq!(
        runtime.forget_memory(&http, &old).unwrap_err().reason_code,
        "RUNTIME_CALLER_MISMATCH"
    );
    assert!(runtime.forget_memory(&desktop, &old).is_err());
    let receipt = runtime
        .forget_memory(
            &desktop,
            &ForgetConfirmation {
                preview_id: replacement.preview_id,
                memory_id: memory.memory_id.clone(),
            },
        )
        .unwrap();
    assert!(receipt.logical_committed);
    assert!(receipt.matched_preview);
    assert!(runtime.held_session().memory(&memory.memory_id).is_err());
    assert_eq!(
        runtime.held_session().memories().unwrap().memories[0].forget_state,
        "forgotten"
    );
}

#[test]
fn explicit_new_draft_replaces_global_owner_and_core_invalidation_releases_stale_metadata() {
    let (_directory, runtime) = fixture();
    let (http, desktop) = contexts();
    let person = imported(&runtime, &http);
    let first = runtime
        .draft_scoped_reply(&http, "Synthetic first", &person, PreferenceOccasion::Work)
        .unwrap();
    let old = runtime
        .propose_scoped_register(&http, &first.result_id.to_string(), 1, VoiceRegister::Plain)
        .unwrap();
    let next = runtime
        .draft_scoped_reply(
            &desktop,
            "Synthetic next",
            &person,
            PreferenceOccasion::Work,
        )
        .unwrap();
    let current = runtime
        .propose_scoped_register(
            &desktop,
            &next.result_id.to_string(),
            2,
            VoiceRegister::Formal,
        )
        .unwrap();
    assert_eq!(
        runtime
            .confirm_scoped_register(&http, &old.proposal_id.to_string(), old.expected_revision)
            .unwrap_err()
            .reason_code,
        "RUNTIME_CALLER_MISMATCH"
    );
    assert_eq!(
        runtime
            .held_session()
            .runtime_pending_slots()
            .scoped_proposal,
        Some(current.proposal_id)
    );
    runtime
        .held_session()
        .set_voice("register", "casual")
        .unwrap();
    assert!(runtime
        .held_session()
        .runtime_pending_slots()
        .scoped_result
        .is_none());
    let recovered = runtime
        .draft_scoped_reply(
            &http,
            "Synthetic recovered",
            &person,
            PreferenceOccasion::Work,
        )
        .unwrap();
    assert!(runtime
        .propose_scoped_register(
            &http,
            &recovered.result_id.to_string(),
            3,
            VoiceRegister::Plain
        )
        .is_ok());
}

#[test]
fn persisted_import_prepare_and_explicit_discard_each_have_body_free_audit() {
    let (_directory, runtime) = fixture();
    let (http, _) = contexts();
    let prepared = runtime
        .prepare_synthetic_import(
            &http,
            SyntheticImportRequest {
                fixture_id: BuiltinFixtureId::Basic,
            },
        )
        .unwrap();
    let id = prepared.operation.operation_id.to_string();
    let audit = runtime.held_session().audit().unwrap();
    let preparation = audit
        .entries
        .iter()
        .find(|entry| entry.action == "import.prepare")
        .unwrap();
    assert_eq!(preparation.subject_refs, vec![id.clone()]);
    runtime.discard_import(&http, &id).unwrap();
    let audit = runtime.held_session().audit().unwrap();
    let cancellation = audit
        .entries
        .iter()
        .find(|entry| entry.action == "import.discard")
        .unwrap();
    assert_eq!(cancellation.subject_refs, vec![id]);
    let encoded = serde_json::to_string(&audit).unwrap();
    assert!(!encoded.contains("u-lilei"));
    assert!(!encoded.contains("明天上午"));
}

#[test]
fn started_precedes_session_lock_and_refusal_settles_without_body_or_path() {
    let (_directory, runtime) = fixture();
    let (_, desktop) = contexts();
    let receiver = runtime.subscribe_events(8);
    let session = runtime.shared_session();
    let guard = session.lock().unwrap();
    let worker_runtime = runtime.clone();
    let worker = std::thread::spawn(move || {
        worker_runtime.edit_memory(
            &desktop,
            "SYNTHETIC_PRIVATE_PATH",
            &MemoryChange {
                title: None,
                summary: Some("SYNTHETIC_PRIVATE_BODY".into()),
                memory_type: None,
            },
        )
    });
    let started = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(started.phase, OperationPhase::Started);
    assert_eq!(runtime.event_snapshot().active_operations.len(), 1);
    assert!(started.correlation_id.is_some());
    drop(guard);
    assert!(worker.join().unwrap().is_err());
    let settled = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(settled.phase, OperationPhase::Settled);
    assert_eq!(settled.outcome, Some(OperationOutcome::Refused));
    assert_eq!(settled.operation_id, started.operation_id);
    assert_eq!(settled.sequence, started.sequence + 1);
    assert!(runtime.event_snapshot().active_operations.is_empty());
    let encoded = serde_json::to_string(&(started, settled)).unwrap();
    assert!(!encoded.contains("SYNTHETIC_PRIVATE"));
    assert!(session.try_lock().is_ok());
}

#[test]
fn slow_subscriber_is_bounded_and_snapshot_recovers_the_sequence_gap() {
    let (_directory, runtime) = fixture();
    let (http, _) = contexts();
    let receiver = runtime.subscribe_events(1);
    for _ in 0..3 {
        assert!(runtime
            .edit_memory(
                &http,
                "not-a-uuid",
                &MemoryChange {
                    title: None,
                    summary: None,
                    memory_type: None
                }
            )
            .is_err());
    }
    let first = receiver.try_recv().unwrap();
    assert!(receiver.try_recv().is_err());
    let snapshot = runtime.event_snapshot();
    assert_eq!(snapshot.epoch, first.epoch);
    assert_eq!(snapshot.sequence, 6);
    assert!(snapshot.sequence > first.sequence);
    assert!(snapshot.active_operations.is_empty());
}
