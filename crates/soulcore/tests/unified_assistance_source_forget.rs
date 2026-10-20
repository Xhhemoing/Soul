//! Deterministic core boundaries, not a platform or real-user utility trial.
use soul_draft::DraftSource;
use soul_store_api::forget::ForgetOutcomeOps;
use soul_store_api::import::{ImportSourceKind, LegacyImportChoice};
use soul_store_api::ForgetUnit;
use soul_testkit::{fixtures, mock_llm::MockLlm};
use soulcore::commands::draft::Approval;
use soulcore::commands::memory::ForgetConfirmation;
use soulcore::commands::session::*;
use uuid::Uuid;

const GOAL: &str = "SYNTHETIC_SOURCED_GOAL_PRIVATE";
const INDEPENDENT: &str = "SYNTHETIC_INDEPENDENT_GOAL_PRIVATE";
const PASTE: &str = "SYNTHETIC_THIRD_PARTY_PASTE_20261019";
const REPLY: &str = "Synthetic endpoint reply for review only";

fn pointer(item: &AssistanceItemRow) -> AssistanceItemRequest {
    AssistanceItemRequest {
        item_id: item.item_id,
        expected_revision: item.revision,
    }
}

fn goal(session: &mut Session, source: Option<AssistanceItemRequest>) -> AssistanceItemRow {
    session
        .create_assistance_goal(CreateAssistanceGoal {
            title: "Synthetic confirmed task".into(),
            summary: if source.is_some() { GOAL } else { INDEPENDENT }.into(),
            deadline_at: None,
            source_observation: source,
        })
        .unwrap()
        .item
}

fn memory_id(item: &AssistanceItemRow) -> Uuid {
    match item.source {
        AssistanceSource::ManualGoal { memory_id, .. } => memory_id,
        _ => panic!("a handwritten goal owns a memory"),
    }
}

fn audit_count(session: &Session, action: &str) -> usize {
    let chain = serde_json::to_value(session.audit().unwrap()).unwrap();
    chain["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["action"] == action)
        .count()
}

fn prepare(session: &mut Session, item: &AssistanceItemRow) -> Approval {
    let context = session.prepare_assistance_context(pointer(item)).unwrap();
    let AssistanceContextView::Insight { insight, .. } = context else {
        panic!("the goal uses its explicitly bound import observation")
    };
    session
        .prepare_draft_with_insight(PASTE, &insight.context_id.to_string(), false, false)
        .unwrap()
        .prepared
        .plan
        .approval()
}

fn source_forget_trajectory(after_execution: bool) {
    let directory = tempfile::tempdir().unwrap();
    let endpoint = MockLlm::start().unwrap();
    endpoint.set_reply(REPLY);
    let mut session = Session::open(directory.path());
    assert!(session.status().store_opened);
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
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
    let contact_id = Uuid::parse_str(&bundle.summaries[0].contact_id).unwrap();
    let observation = session
        .link_assistance_observation(LinkAssistanceObservation {
            bundle_id: bundle.bundle_id,
            contact_id,
        })
        .unwrap()
        .item;
    let sourced = goal(&mut session, Some(pointer(&observation)));
    let independent = goal(&mut session, None);

    // Positive control: this source and endpoint work before destruction.
    let approved = prepare(&mut session, &sourced);
    assert_eq!(endpoint.request_count(), 0);
    let work = session.begin_draft(&approved).unwrap();
    let completed = work.execute();
    let draft = session.complete_draft(completed).unwrap();
    assert_eq!(draft.source, DraftSource::UserEndpoint);
    assert_eq!(endpoint.request_count(), 1);
    session
        .set_assistance_feedback(SetAssistanceFeedback {
            item_id: sourced.item_id,
            expected_revision: sourced.revision,
            expected_feedback_revision: None,
            value: AssistanceFeedbackChange::Helpful,
        })
        .unwrap();
    assert_eq!(
        session
            .assistance_item(&sourced.item_id.to_string())
            .unwrap()
            .item,
        sourced,
        "drafting and helpful feedback do not complete or revise the goal"
    );

    let unused = prepare(&mut session, &sourced);
    let completed = if after_execution {
        let work = session.begin_draft(&unused).unwrap();
        Some(work.execute())
    } else {
        None
    };
    let expected_requests = if after_execution { 2 } else { 1 };
    assert_eq!(endpoint.request_count(), expected_requests);
    let accepted_before = audit_count(&session, "draft.create");
    assert_eq!(accepted_before, 1);

    // Another domain writer changes the SAME store. Do not call Session's
    // invalidate_drafts: acceptance must independently revalidate the source.
    // Contact forgetting is a core API, not a newly invented desktop command.
    let shared = session.store().unwrap();
    let forgotten = shared
        .lock()
        .unwrap()
        .execute_forget_outcome(ForgetUnit::Contact(contact_id))
        .unwrap();
    assert!(!forgotten.receipt.impact.content_key_ids.is_empty());
    drop(shared);
    let refusal = match completed {
        Some(completed) => session.complete_draft(completed).unwrap_err(),
        None => session.begin_draft(&unused).unwrap_err(),
    };
    assert_eq!(refusal.reason_code, "PLAN_HASH_MISMATCH");
    assert_eq!(endpoint.request_count(), expected_requests);
    assert_eq!(
        audit_count(&session, "draft.create"),
        accepted_before,
        "a refused result must not be recorded as an accepted draft"
    );
    let current = session
        .assistance_item(&sourced.item_id.to_string())
        .unwrap();
    assert_eq!(current.item.validity, AssistanceValidity::Stale);
    assert_eq!(current.item.handling, AssistanceHandling::Open);
    assert_eq!(current.goal.unwrap().summary, GOAL);
    assert!(current.observation.is_none());
    assert_eq!(
        session
            .memory(&memory_id(&sourced).to_string())
            .unwrap()
            .summary,
        GOAL
    );
    assert_eq!(
        session
            .memory(&memory_id(&independent).to_string())
            .unwrap()
            .summary,
        INDEPENDENT
    );
    assert_eq!(
        session
            .prepare_assistance_context(pointer(&current.item))
            .unwrap_err()
            .reason_code,
        "ASSISTANCE_CHANGED"
    );

    // A never-dispatched approval for an unchanged independent goal isolates
    // restart refusal from forgetting, spent permits and missing endpoints.
    let context = session
        .prepare_assistance_context(pointer(&independent))
        .unwrap();
    let AssistanceContextView::Memory { memory, .. } = context else {
        panic!("the independent goal uses its own memory")
    };
    let independent_context = memory.context_id.unwrap().to_string();
    let independent_approval = session
        .prepare_draft_with_memory(PASTE, &independent_context, &[], false)
        .unwrap()
        .prepared
        .plan
        .approval();
    drop(session);
    let mut reopened = Session::open(directory.path());
    assert!(reopened.status().store_opened);
    reopened.set_user_endpoint(&endpoint.base_url()).unwrap();
    assert_eq!(
        reopened
            .begin_draft(&independent_approval)
            .unwrap_err()
            .reason_code,
        "PLAN_HASH_MISMATCH"
    );
    assert_eq!(
        reopened
            .assistance_item(&sourced.item_id.to_string())
            .unwrap()
            .item,
        current.item
    );
    assert_eq!(
        reopened
            .assistance_item(&independent.item_id.to_string())
            .unwrap()
            .item,
        independent
    );
    assert!(reopened
        .draft_pasted_with_memory(PASTE, &independent_context)
        .is_err());

    let context = reopened
        .prepare_assistance_context(pointer(&independent))
        .unwrap();
    let AssistanceContextView::Memory { memory, .. } = context else {
        panic!("fresh explicit selection of the surviving goal still works")
    };
    assert_eq!(
        reopened
            .draft_pasted_with_memory(PASTE, &memory.context_id.unwrap().to_string())
            .unwrap()
            .draft
            .source,
        DraftSource::ToneTemplate
    );
    // Retained content remains usable after an explicit source decision;
    // source forgetting must not permanently strand the handwritten goal.
    let rebound = reopened
        .bind_assistance_goal_source(BindAssistanceGoalSource {
            item_id: current.item.item_id,
            expected_revision: current.item.revision,
            source_observation: None,
        })
        .unwrap()
        .item;
    assert_eq!(rebound.item_id, sourced.item_id);
    assert_eq!(rebound.validity, AssistanceValidity::Current);
    assert_eq!(rebound.handling, AssistanceHandling::Open);
    let context = reopened
        .prepare_assistance_context(pointer(&rebound))
        .unwrap();
    let AssistanceContextView::Memory { memory, .. } = context else {
        panic!("explicitly clearing the revoked source selects only goal memory")
    };
    assert_eq!(
        reopened
            .draft_pasted_with_memory(PASTE, &memory.context_id.unwrap().to_string())
            .unwrap()
            .draft
            .source,
        DraftSource::ToneTemplate
    );
    let id = memory_id(&sourced).to_string();
    let preview = reopened.preview_forget(&id).unwrap();
    reopened
        .forget_memory(&ForgetConfirmation {
            preview_id: preview.preview_id,
            memory_id: id.clone(),
        })
        .unwrap();
    assert!(reopened.memory(&id).is_err());
    assert_eq!(
        reopened
            .assistance_item(&sourced.item_id.to_string())
            .unwrap()
            .item
            .validity,
        AssistanceValidity::Forgotten
    );
    assert!(reopened
        .memory(&memory_id(&independent).to_string())
        .is_ok());
    assert_eq!(endpoint.request_count(), expected_requests);

    let chain = serde_json::to_value(reopened.audit().unwrap()).unwrap();
    assert_eq!(chain["verified"], true);
    let entries = chain["entries"].as_array().unwrap();
    assert_eq!(
        entries
            .iter()
            .filter(|entry| entry["action"] == "egress.request")
            .count(),
        expected_requests,
        "already-executed requests remain audited even when acceptance refuses"
    );
    for prose in [GOAL, INDEPENDENT, PASTE, REPLY] {
        assert!(!chain.to_string().contains(prose));
    }
}

#[test]
fn source_forget_before_dispatch_rejects_approved_material_without_another_request() {
    source_forget_trajectory(false);
}

#[test]
fn source_forget_after_execution_rejects_result_without_erasing_actual_egress() {
    source_forget_trajectory(true);
}
