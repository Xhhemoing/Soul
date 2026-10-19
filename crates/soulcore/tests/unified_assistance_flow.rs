//! U7 synthetic cross-layer trajectories, not a Windows or real-utility trial.
#[path = "../../../apps/desktop/src-tauri/src/assistance_runtime/driver.rs"]
mod assistance_driver;

use assistance_driver::{cycle, CycleOutcome};
use soul_draft::{DraftSource, BODY_SLOT};
use soul_store_api::forget::ForgetOutcomeOps;
use soul_store_api::import::{ImportSourceKind, LegacyImportChoice};
use soul_store_api::ForgetUnit;
use soul_testkit::{fixtures, mock_llm::MockLlm};
use soulcore::commands::memory::ForgetConfirmation;
use soulcore::commands::session::*;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use uuid::Uuid;

fn held(session: &Arc<Mutex<Session>>) -> MutexGuard<'_, Session> {
    session.lock().unwrap()
}
fn request(item: &AssistanceItemRow) -> AssistanceItemRequest {
    AssistanceItemRequest {
        item_id: item.item_id,
        expected_revision: item.revision,
    }
}
fn goal(session: &mut Session, source: Option<AssistanceItemRequest>) -> AssistanceItemRow {
    session
        .create_assistance_goal(CreateAssistanceGoal {
            title: "Synthetic confirmed goal".into(),
            summary: "Private synthetic objective; never a received message".into(),
            deadline_at: Some("2099-10-23T16:00:00+08:00".into()),
            source_observation: source,
        })
        .unwrap()
        .item
}
fn timestamp(ms: i64) -> String {
    format!(
        "{}.{:03}Z",
        soul_policy::clock::rfc3339_utc(ms.div_euclid(1000)).trim_end_matches('Z'),
        ms.rem_euclid(1000)
    )
}
fn memory_id(item: &AssistanceItemRow) -> Uuid {
    match item.source {
        AssistanceSource::ManualGoal { memory_id, .. } => memory_id,
        _ => panic!("manual goal"),
    }
}
fn local_preview(
    session: &Arc<Mutex<Session>>,
    request: PreviewAssistanceLocalTemplate,
) -> Result<AssistanceLocalTemplatePreview, SessionRefusal> {
    let work = held(session).begin_assistance_local_template_preview(request)?;
    let completed = work.execute();
    held(session).complete_assistance_local_template_preview(completed)
}

#[test]
fn imported_goal_preparation_notice_draft_feedback_and_forget_keep_one_identity() {
    let directory = tempfile::tempdir().unwrap();
    let endpoint = MockLlm::start().unwrap();
    endpoint.set_reply("Synthetic reply for explicit review only");
    let mut session = Session::open(directory.path());
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
    session.set_voice("warmth", "warm").unwrap();
    let work = session
        .begin_import_prepare(
            fixtures::read_text("import/soul-import-v1/valid_basic.jsonl").unwrap(),
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    let prepared = session.complete_import_prepare(work.execute()).unwrap();
    let work = session
        .begin_import(
            &prepared.operation.operation_id.to_string(),
            &prepared.operation.preview_id.to_string(),
        )
        .unwrap();
    session.complete_import(work.execute()).unwrap();
    let bundle = session.local_insights().unwrap().remove(0);
    let contact = Uuid::parse_str(&bundle.summaries[0].contact_id).unwrap();
    let observation = session
        .link_assistance_observation(LinkAssistanceObservation {
            bundle_id: bundle.bundle_id,
            contact_id: contact,
        })
        .unwrap()
        .item;
    let item = goal(&mut session, Some(request(&observation)));
    let independent = goal(&mut session, None);
    let id = item.item_id.to_string();
    assert!(!session.assistance_local_preparation(&id).unwrap().enabled);
    assert_eq!(
        session.assistance_reminder(&id).unwrap().state,
        ReminderState::Off
    );
    let enabled = session
        .set_assistance_local_preparation(SetAssistanceLocalPreparation {
            item_id: item.item_id,
            expected_revision: item.revision,
            expected_preference_revision: None,
            enabled: true,
        })
        .unwrap();
    let session = Arc::new(Mutex::new(session));
    let stop = AtomicBool::new(false);
    for _ in 0..20 {
        if cycle(&session, &stop).unwrap() == CycleOutcome::Idle {
            break;
        }
    }
    let metadata = held(&session).assistance_local_preparation(&id).unwrap();
    assert_eq!(metadata.state, AssistanceLocalPreparationState::Ready);
    let preview_request = PreviewAssistanceLocalTemplate {
        item_id: item.item_id,
        expected_revision: item.revision,
        expected_preference_revision: enabled.preference_revision.unwrap(),
        template_id: metadata.template_id.unwrap(),
    };
    // Execute the shipped owned API outside the Session guard.
    let work = held(&session)
        .begin_assistance_local_template_preview(preview_request.clone())
        .unwrap();
    let completion = work.execute();
    let preview = held(&session)
        .complete_assistance_local_template_preview(completion)
        .unwrap();
    assert!(preview.body.contains(BODY_SLOT));
    assert!(!preview.body.contains("Private synthetic objective"));
    assert_eq!(preview.item_id, item.item_id);
    assert_eq!(endpoint.request_count(), 0);

    // The OS observation below is deliberately synthetic, while the claim,
    // source validation, durable receipt and current target are production APIs.
    let base = 1_800_000_000_000;
    held(&session)
        .set_assistance_reminder_at(
            SetAssistanceReminder {
                item_id: item.item_id,
                expected_revision: item.revision,
                expected_schedule_revision: None,
                remind_at: Some(timestamp(base + 1)),
            },
            base,
        )
        .unwrap();
    let mono = Instant::now();
    assert!(held(&session)
        .begin_assistance_reminder_check_at(base, mono)
        .unwrap()
        .is_none());
    let work = held(&session)
        .begin_assistance_reminder_check_at(base + 1, mono + Duration::from_millis(1))
        .unwrap()
        .unwrap();
    let completion = work.execute_at(base + 1);
    let attempt = held(&session)
        .complete_assistance_reminder_check(completion)
        .unwrap()
        .unwrap()
        .attempt_id;
    held(&session)
        .record_assistance_notification_delivery(attempt, NotificationDelivery::SubmittedToOs)
        .unwrap();
    held(&session)
        .record_assistance_notification_event(attempt, NotificationNativeEvent::Opened)
        .unwrap();
    assert_eq!(
        held(&session)
            .resolve_assistance_notification(attempt)
            .unwrap()
            .item_id,
        item.item_id
    );
    assert_eq!(
        held(&session).assistance_item(&id).unwrap().item.handling,
        AssistanceHandling::Open
    );
    assert_eq!(endpoint.request_count(), 0);

    let context = held(&session)
        .prepare_assistance_context(request(&item))
        .unwrap();
    let AssistanceContextView::Insight { insight, .. } = context else {
        panic!("fixed source context")
    };
    let plan = held(&session)
        .prepare_draft_with_insight(
            "Synthetic pasted third-party message",
            &insight.context_id.to_string(),
            false,
            false,
        )
        .unwrap();
    assert_eq!(endpoint.request_count(), 0);
    let approval = plan.prepared.plan.approval();
    let work = held(&session).begin_draft(&approval).unwrap();
    let completed = work.execute();
    let draft = held(&session).complete_draft(completed).unwrap();
    assert_eq!(draft.source, DraftSource::UserEndpoint);
    assert_eq!(endpoint.request_count(), 1);
    assert!(held(&session).begin_draft(&approval).is_err());
    assert_eq!(
        held(&session).assistance_item(&id).unwrap().item.handling,
        AssistanceHandling::Open
    );
    let feedback = held(&session)
        .set_assistance_feedback(SetAssistanceFeedback {
            item_id: item.item_id,
            expected_revision: item.revision,
            expected_feedback_revision: None,
            value: AssistanceFeedbackChange::Helpful,
        })
        .unwrap();
    assert_eq!(feedback.item_revision, item.revision);
    assert_eq!(
        held(&session).assistance_item(&id).unwrap().item.handling,
        AssistanceHandling::Open
    );
    let completed = held(&session)
        .act_assistance_item(AssistanceActionRequest {
            item_id: item.item_id,
            expected_revision: item.revision,
            action: AssistanceAction::CompleteSelfReported,
        })
        .unwrap()
        .item;
    assert_eq!(completed.handling, AssistanceHandling::ResolvedReported);
    let reminder = held(&session).assistance_reminder(&id).unwrap();
    assert_eq!(reminder.state, ReminderState::Invalidated);
    assert!(reminder.latest_attempt.unwrap().latest_withdrawal.is_some());
    assert!(local_preview(&session, preview_request.clone()).is_err());

    // Source forgetting cannot erase an independent owner's goal or turn a
    // completed item back into an open/cancelled task.
    let store = held(&session).store().unwrap();
    store
        .lock()
        .unwrap()
        .execute_forget_outcome(ForgetUnit::Contact(contact))
        .unwrap();
    drop(store);
    assert!(held(&session).memory(&memory_id(&item).to_string()).is_ok());
    assert_eq!(
        held(&session).assistance_item(&id).unwrap().item.handling,
        AssistanceHandling::ResolvedReported
    );
    assert!(local_preview(&session, preview_request).is_err());
    assert!(!held(&session)
        .authorize_assistance_notification_dispatch(attempt)
        .unwrap());
    let memory = memory_id(&item).to_string();
    let forget = held(&session).preview_forget(&memory).unwrap();
    held(&session)
        .forget_memory(&ForgetConfirmation {
            preview_id: forget.preview_id,
            memory_id: memory.clone(),
        })
        .unwrap();
    assert!(held(&session).memory(&memory).is_err());
    assert_eq!(
        held(&session)
            .assistance_item(&independent.item_id.to_string())
            .unwrap()
            .item,
        independent
    );

    // Keep a new, never-executed approval for the unchanged independent goal.
    // Restart refusal therefore cannot be explained by an already-spent token
    // or by forgetting its source.
    let context = held(&session)
        .prepare_assistance_context(request(&independent))
        .unwrap();
    let AssistanceContextView::Memory {
        memory: context, ..
    } = context
    else {
        panic!("independent memory")
    };
    let context_id = context.context_id.unwrap().to_string();
    let pending = held(&session)
        .prepare_draft_with_memory("Another synthetic paste", &context_id, &[], false)
        .unwrap();
    let unused = pending.prepared.plan.approval();
    assert_eq!(endpoint.request_count(), 1);
    drop(session);
    let mut reopened = Session::open(directory.path());
    assert!(reopened.status().store_opened);
    assert_eq!(
        reopened.assistance_item(&id).unwrap().item.validity,
        AssistanceValidity::Forgotten
    );
    assert_eq!(
        reopened
            .assistance_item(&independent.item_id.to_string())
            .unwrap()
            .item,
        independent
    );
    assert_eq!(
        reopened.begin_draft(&unused).unwrap_err().reason_code,
        "PLAN_HASH_MISMATCH"
    );
    assert!(reopened
        .draft_pasted_with_memory("Another synthetic paste", &context_id)
        .is_err());
    assert_eq!(endpoint.request_count(), 1);
}

#[test]
fn hundred_item_backlog_coalesces_preserves_foreground_and_admits_one_due_notice() {
    let directory = tempfile::tempdir().unwrap();
    let endpoint = MockLlm::start().unwrap();
    let mut session = Session::open(directory.path());
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
    let mut items: Vec<_> = (0..100).map(|_| goal(&mut session, None)).collect();
    for number in 0..20 {
        items[0] = session
            .edit_assistance_goal(EditAssistanceGoal {
                item_id: items[0].item_id,
                expected_revision: items[0].revision,
                title: format!("Synthetic revision {number}"),
                summary: "Synthetic goal".into(),
                deadline_at: None,
            })
            .unwrap()
            .item;
    }
    assert_eq!(
        session.assistance_work_status().unwrap().pending_items,
        Some(100)
    );
    let context = session
        .prepare_assistance_context(request(&items[0]))
        .unwrap();
    let AssistanceContextView::Memory { memory, .. } = context else {
        panic!("manual memory")
    };
    let context_id = memory.context_id.unwrap().to_string();
    let session = Arc::new(Mutex::new(session));
    let stop = AtomicBool::new(false);
    assert_eq!(cycle(&session, &stop).unwrap(), CycleOutcome::Idle);
    assert_eq!(
        held(&session)
            .assistance_work_status()
            .unwrap()
            .pending_items,
        Some(100)
    );
    held(&session)
        .draft_pasted_with_memory("Explicit synthetic paste", &context_id)
        .unwrap();
    for _ in 0..100 {
        assert_eq!(cycle(&session, &stop).unwrap(), CycleOutcome::Completed);
    }
    assert_eq!(cycle(&session, &stop).unwrap(), CycleOutcome::Idle);
    let status = held(&session).assistance_work_status().unwrap();
    assert_eq!(status.pending_items, Some(0));
    assert_eq!(status.claimed_items, Some(0));
    for item in &items {
        let work = held(&session)
            .assistance_item_work(&item.item_id.to_string())
            .unwrap();
        assert_eq!(work.state, AssistanceWorkState::Ready);
        assert_eq!(work.result_revision, Some(item.revision));
        assert!(work.pending_revision.is_none() && work.claim_id.is_none());
    }
    let base = 1_800_000_000_000;
    for item in &items {
        held(&session)
            .set_assistance_reminder_at(
                SetAssistanceReminder {
                    item_id: item.item_id,
                    expected_revision: item.revision,
                    expected_schedule_revision: None,
                    remind_at: Some(timestamp(base + 1)),
                },
                base,
            )
            .unwrap();
    }
    let mono = Instant::now();
    assert!(held(&session)
        .begin_assistance_reminder_check_at(base, mono)
        .unwrap()
        .is_none());
    let work = held(&session)
        .begin_assistance_reminder_check_at(base + 1, mono + Duration::from_millis(1))
        .unwrap()
        .unwrap();
    let completed = work.execute_at(base + 1);
    assert!(held(&session)
        .complete_assistance_reminder_check(completed)
        .unwrap()
        .is_some());
    assert!(held(&session)
        .begin_assistance_reminder_check_at(base + 2, mono + Duration::from_millis(2))
        .unwrap()
        .is_none());
    let views: Vec<_> = items
        .iter()
        .map(|item| {
            held(&session)
                .assistance_reminder(&item.item_id.to_string())
                .unwrap()
        })
        .collect();
    assert_eq!(
        views
            .iter()
            .filter(|view| view.latest_attempt.is_some())
            .count(),
        1
    );
    assert!(views
        .iter()
        .all(|view| view.state == ReminderState::ExpiredInInbox));
    assert_eq!(endpoint.request_count(), 0);
}
