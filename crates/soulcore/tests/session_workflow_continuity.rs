//! A single real Session trajectory for the W1–W5 local workflow additions.
//!
//! This complements the original AC-21 headless flow. Contact forgetting below
//! deliberately calls the existing store domain API: no contact-forget IPC or
//! automatic conversion of imported statistics into a memory is implied.

use soul_draft::DraftSource;
use soul_policy::redactor::THIRD_PARTY_PLACEHOLDER;
use soul_store_api::import::{ImportOperationState, ImportSourceKind, LegacyImportChoice};
use soul_store_api::insight::InsightValidity;
use soul_store_api::types::EventFilter;
use soul_store_api::{EventStore, ForgetOps, ForgetUnit};
use soul_testkit::{fixtures, mock_llm::MockLlm};
use soulcore::commands::memory::{ForgetConfirmation, NewMemory};
use soulcore::commands::session::{
    ImportOperationView, MemoryContextRequest, PreparedImportView, Session,
};
use soulcore::netwatch::{Support, Watch};
use uuid::Uuid;

const TITLE: &str = "合成偏好：先核对时间再作承诺";
const NOTE: &str = "合成私人记录：我希望约定时间经过确认再安排后续。";
const PASTE: &str = "合成第三人原文：请在周五下午三点以前确认这次安排。";
const REPLY: &str = "我会先核对时间，确认后再安排后续。";

fn prepare(session: &mut Session, text: &str, namespace: Option<Uuid>) -> PreparedImportView {
    let work = session
        .begin_import_prepare(
            text.to_owned(),
            ImportSourceKind::SoulImportV1,
            namespace,
            LegacyImportChoice::Reject,
        )
        .expect("prepare the immutable fixture");
    session
        .complete_import_prepare(work.execute())
        .expect("receive this preparation")
}

fn confirm(session: &mut Session, prepared: &PreparedImportView) -> ImportOperationView {
    let work = session
        .begin_import(
            &prepared.operation.operation_id.to_string(),
            &prepared.operation.preview_id.to_string(),
        )
        .expect("confirm only the two immutable ids");
    session
        .complete_import(work.execute())
        .expect("receive the durable operation")
}

fn selection(memory_id: Uuid) -> MemoryContextRequest {
    MemoryContextRequest {
        candidate_ids: vec![memory_id],
        query: None,
        memory_type: None,
        time_specific: false,
    }
}

fn domain_counts(session: &Session) -> (u64, usize, usize) {
    let shared = session.store().expect("same production store");
    let events = shared
        .lock()
        .unwrap()
        .count_events(&EventFilter::all())
        .unwrap();
    let graph = session.people().unwrap();
    (
        events,
        graph.people.len(),
        graph.ties.iter().map(|tie| tie.evidence.len()).sum(),
    )
}

#[test]
fn imported_feedback_and_explicit_drafting_survive_forget_and_restart_boundaries() {
    let watch = Watch::start();
    let dir = tempfile::tempdir().unwrap();
    let text = fixtures::read_text("import/soul-import-v1/three_partners.jsonl").unwrap();
    let endpoint = MockLlm::start().unwrap();
    endpoint.set_reply(REPLY);
    let mut session = Session::open(dir.path());
    assert!(session.status().store_opened);
    assert!(session.snapshot().fully_closed);

    // The same confirmation is a receipt lookup; a new explicit intent on
    // the same namespace is deduplicated against the sealed natural keys.
    let prepared = prepare(&mut session, &text, None);
    assert_eq!(prepared.operation.preview.events_written, 16);
    assert_eq!(prepared.operation.preview.evidence_written, 15);
    let committed = confirm(&mut session, &prepared);
    let counts = committed.operation.receipt.clone().unwrap();
    assert_eq!(committed.operation.state, ImportOperationState::Committed);
    assert_eq!(counts.events_written, 16);
    assert_eq!(counts.evidence_written, 15);
    let initial = domain_counts(&session);
    // Despite its historical filename, this fixture names four peers plus self.
    assert_eq!(initial, (16, 5, 15));
    let first_bundle = session
        .local_insights()
        .unwrap()
        .into_iter()
        .find(|view| view.operation_id == committed.operation.operation_id)
        .expect("the import transaction generated its local candidate");
    assert_eq!(first_bundle.validity, InsightValidity::Active);
    assert!(!first_bundle.summaries.is_empty());
    assert!(first_bundle.summaries.iter().all(|summary| {
        summary.source == "counts"
            && !summary.points.is_empty()
            && summary
                .points
                .iter()
                .all(|point| !point.evidence_ids.is_empty())
    }));
    let bundles_before = session.local_insights().unwrap();
    let audit_before = session.audit().unwrap().entries;
    assert_eq!(confirm(&mut session, &prepared), committed);
    assert_eq!(domain_counts(&session), initial);
    assert_eq!(session.local_insights().unwrap(), bundles_before);
    assert_eq!(session.audit().unwrap().entries, audit_before);

    let duplicate = prepare(&mut session, &text, Some(prepared.operation.namespace_id));
    assert_eq!(duplicate.operation.preview.events_written, 0);
    let duplicate_receipt = confirm(&mut session, &duplicate).operation.receipt.unwrap();
    assert_eq!(duplicate_receipt.events_written, 0);
    assert_eq!(duplicate_receipt.evidence_written, 0);
    assert_eq!(duplicate_receipt.duplicates, 16);
    assert_eq!(domain_counts(&session), initial);
    assert_eq!(
        endpoint.request_count(),
        0,
        "imports and local A2 never use an endpoint"
    );

    // Ignore is instance-specific. A correction changes the existing user
    // lock, invalidates the old dependencies, and recomputation gets a new id.
    let contact = first_bundle.summaries[0].contact_id.clone();
    let graph = session.people().unwrap();
    let tie = graph
        .ties
        .iter()
        .find(|tie| tie.from_contact_id == contact || tie.to_contact_id == contact)
        .unwrap();
    let relationship_id = tie.relationship_id.clone();
    let desired = if tie.band == "strong" {
        "weak"
    } else {
        "strong"
    };
    let ignored = session
        .ignore_local_insight(&first_bundle.bundle_id.to_string())
        .unwrap();
    assert!(ignored.ignored);
    assert_eq!(ignored.validity, InsightValidity::Active);
    assert_eq!(ignored.counts, counts);
    assert!(ignored.summaries.is_empty());
    let corrected = session.correct_tie(&relationship_id, desired).unwrap();
    let locked = corrected
        .ties
        .iter()
        .find(|tie| tie.relationship_id == relationship_id)
        .unwrap();
    assert!(locked.locked_by_user);
    assert_eq!(locked.band, desired);
    let old = session
        .local_insights()
        .unwrap()
        .into_iter()
        .find(|view| view.bundle_id == first_bundle.bundle_id)
        .unwrap();
    assert!(old.ignored);
    assert_eq!(old.validity, InsightValidity::Invalidated);
    assert!(old.summaries.is_empty());
    let recomputed = session
        .recompute_local_insight(&old.bundle_id.to_string())
        .unwrap();
    assert_ne!(recomputed.bundle_id, old.bundle_id);
    assert_eq!(recomputed.operation_id, old.operation_id);
    assert_eq!(recomputed.validity, InsightValidity::Active);
    assert!(!recomputed.ignored);
    assert!(!recomputed.summaries.is_empty());
    assert_eq!(endpoint.request_count(), 0);

    // This is a hand-written memory, not an imported statistic masquerading
    // as a memory. A local use consumes its grant and leaves the template as
    // a deterministic skeleton with an independently displayed reference.
    let memory = session
        .write_memory(&NewMemory {
            memory_type: "preference".into(),
            title: TITLE.into(),
            summary: NOTE.into(),
        })
        .unwrap();
    let memory_id = Uuid::parse_str(&memory.memory_id).unwrap();
    let local_id = session
        .prepare_memory_context(selection(memory_id))
        .unwrap()
        .context_id
        .unwrap()
        .to_string();
    let plain = session.draft_pasted(PASTE).unwrap();
    let local = session.draft_pasted_with_memory(PASTE, &local_id).unwrap();
    assert_eq!(local.draft.source, DraftSource::ToneTemplate);
    assert_eq!(local.draft.text, plain.text);
    assert_eq!(local.context.selected[0].citation.memory_id, memory_id);
    assert_eq!(local.context.selected[0].summary, NOTE);
    assert!(session.draft_pasted_with_memory(PASTE, &local_id).is_err());

    // A new explicit selection is needed before E1. Only this Owner excerpt
    // is opted in; the unrelated third-party paste remains placeheld.
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
    let remote_id = session
        .prepare_memory_context(selection(memory_id))
        .unwrap()
        .context_id
        .unwrap()
        .to_string();
    let plan = session
        .prepare_draft_with_memory(PASTE, &remote_id, &[memory_id], false)
        .unwrap();
    assert_eq!(plan.prepared.endpoint_owner_ids, vec![memory_id]);
    let approval = plan.prepared.plan.approval();
    let completed = session.begin_draft(&approval).unwrap().execute();
    let drafted = session.complete_draft(completed).unwrap();
    assert_eq!(drafted.source, DraftSource::UserEndpoint);
    assert_eq!(drafted.text, REPLY);
    assert!(session.begin_draft(&approval).is_err());
    assert!(session.draft_pasted_with_memory(PASTE, &remote_id).is_err());
    assert_eq!(endpoint.request_count(), 1);
    let wire = &endpoint.requests()[0].body;
    assert!(wire.contains(NOTE));
    assert!(wire.contains(THIRD_PARTY_PLACEHOLDER));
    assert!(!wire.contains(PASTE));
    assert!(!wire.contains(&memory.memory_id));

    // Keep a prepared approval across forgetting. Neither the context nor
    // that approval may make a second request, now or after reopening.
    let stale_context = session
        .prepare_memory_context(selection(memory_id))
        .unwrap()
        .context_id
        .unwrap()
        .to_string();
    let stale_plan = session
        .prepare_draft_with_memory(PASTE, &stale_context, &[memory_id], false)
        .unwrap();
    let stale_approval = stale_plan.prepared.plan.approval();
    let forget = session.preview_forget(&memory.memory_id).unwrap();
    let forgotten = session
        .forget_memory(&ForgetConfirmation {
            preview_id: forget.preview_id,
            memory_id: memory.memory_id.clone(),
        })
        .unwrap();
    assert!(forgotten.logical_committed);
    assert!(session.memory(&memory.memory_id).is_err());
    assert!(session
        .draft_pasted_with_memory(PASTE, &stale_context)
        .is_err());
    assert!(session.begin_draft(&stale_approval).is_err());
    assert_eq!(endpoint.request_count(), 1);

    // Leave an unconfirmed import intent for startup recovery. Forgetting a
    // peer here is an existing store-domain operation, not a new UI command.
    let reserved = prepare(&mut session, &text, Some(prepared.operation.namespace_id));
    let shared = session.store().unwrap();
    shared
        .lock()
        .unwrap()
        .execute_forget(ForgetUnit::Contact(Uuid::parse_str(&contact).unwrap()))
        .unwrap();
    assert!(session
        .recompute_local_insight(&recomputed.bundle_id.to_string())
        .is_err());
    drop(session);
    let blocked = Session::open(dir.path());
    assert!(
        !blocked.status().store_opened,
        "the raw store Arc still owns the runtime lease"
    );
    drop(blocked);
    drop(shared);

    let mut reopened = Session::open(dir.path());
    assert!(reopened.status().store_opened);
    assert!(!reopened.snapshot().llm_endpoint_configured);
    let historical = reopened
        .import_operation(&committed.operation.operation_id.to_string())
        .unwrap();
    assert_eq!(historical.operation.receipt, Some(counts.clone()));
    assert!(historical.namespace_revoked);
    assert!(!historical.prepared_here);
    let interrupted = reopened
        .import_operation(&reserved.operation.operation_id.to_string())
        .unwrap();
    assert_eq!(
        interrupted.operation.state,
        ImportOperationState::Interrupted
    );
    assert!(interrupted.operation.receipt.is_none());
    assert!(reopened
        .begin_import(
            &reserved.operation.operation_id.to_string(),
            &reserved.operation.preview_id.to_string()
        )
        .is_err());
    let retry = reopened
        .begin_import_prepare(
            text.clone(),
            ImportSourceKind::SoulImportV1,
            Some(prepared.operation.namespace_id),
            LegacyImportChoice::Reject,
        )
        .unwrap()
        .execute();
    assert_eq!(
        reopened
            .complete_import_prepare(retry)
            .unwrap_err()
            .reason_code,
        "IMPORT_NAMESPACE_REVOKED"
    );
    assert!(
        reopened
            .import_recovery_state()
            .unwrap()
            .can_start_new_intent
    );
    assert!(reopened
        .draft_pasted_with_memory(PASTE, &stale_context)
        .is_err());
    assert!(reopened.begin_draft(&stale_approval).is_err());
    assert!(reopened
        .prepare_memory_context(selection(memory_id))
        .unwrap()
        .context_id
        .is_none());
    let surviving_lock = reopened
        .people()
        .unwrap()
        .ties
        .into_iter()
        .find(|tie| tie.relationship_id == relationship_id)
        .unwrap();
    assert!(surviving_lock.locked_by_user);
    assert_eq!(surviving_lock.band, desired);
    let after = reopened.local_insights().unwrap();
    let ignored_history = after
        .iter()
        .find(|view| view.bundle_id == first_bundle.bundle_id)
        .unwrap();
    assert!(ignored_history.ignored);
    let historical_bundle = after
        .iter()
        .find(|view| view.bundle_id == recomputed.bundle_id)
        .unwrap();
    assert_eq!(historical_bundle.validity, InsightValidity::Invalidated);
    assert_eq!(historical_bundle.counts, counts);
    assert!(historical_bundle.summaries.is_empty());
    assert!(reopened
        .recompute_local_insight(&recomputed.bundle_id.to_string())
        .is_err());
    assert_eq!(endpoint.request_count(), 1);

    let audit = reopened.audit().unwrap();
    assert!(audit.verified);
    let serialized = serde_json::to_string(&audit).unwrap();
    for private in [TITLE, NOTE, PASTE, REPLY] {
        assert!(
            !serialized.contains(private),
            "audit repeated synthetic prose"
        );
    }
    let parsed = soul_import::soul_import_v1::parse(&text).unwrap();
    for message in parsed
        .messages
        .iter()
        .filter(|message| message.body.as_str().chars().count() >= 8)
    {
        assert!(
            !serialized.contains(message.body.as_str()),
            "audit repeated an imported body"
        );
    }
    assert!(audit
        .entries
        .iter()
        .any(|entry| entry.action == "egress.request"));
    assert_eq!(
        audit
            .entries
            .iter()
            .filter(|entry| entry.action == "draft.create")
            .count(),
        3
    );
    let observed = watch.stop();
    assert!(observed.is_clean(), "{observed}");
    if cfg!(target_os = "linux") {
        assert_eq!(observed.support, Support::Observed);
    }
    assert!(observed.samples > 0);
}
