//! Exact typed A2 selection, single-use grants, and actual loopback request material.
use std::collections::BTreeSet;

use soul_draft::brief::ProfileBrief;
use soul_draft::draft::{DraftRequest, Drafter};
use soul_policy::e1::{encoded_chat_body, E1Purpose};
use soul_policy::redactor::{ExemptionRequest, KnownIdentifiers, Redactor};
use soul_store_api::import::{ImportSourceKind, LegacyImportChoice};
use soul_store_api::insight::InsightValidity;
use soul_store_api::{ForgetOps, ForgetUnit, GraphStore, ProfileStore};
use soul_testkit::{fixtures, mock_llm::MockLlm};
use soulcore::commands::draft::{MAX_MEMORY_REQUEST_BYTES, UNNAMED_MODEL};
use soulcore::commands::insights::{self, LocalInsightView};
use soulcore::commands::memory::NewMemory;
use soulcore::commands::session::{
    InsightContextRequest, InsightContextView, MemoryContextRequest, Session,
};
use uuid::Uuid;

fn fixture() -> (tempfile::TempDir, Session, LocalInsightView) {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    assert!(session.status().store_opened);
    let text = fixtures::read_text("import/soul-import-v1/valid_basic.jsonl").unwrap();
    let prepare = session
        .begin_import_prepare(
            text,
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    let prepared = session.complete_import_prepare(prepare.execute()).unwrap();
    let work = session
        .begin_import(
            &prepared.operation.operation_id.to_string(),
            &prepared.operation.preview_id.to_string(),
        )
        .unwrap();
    session.complete_import(work.execute()).unwrap();
    let bundle = session.local_insights().unwrap().remove(0);
    assert!(
        bundle.summaries.len() >= 2,
        "fixture must exercise exclusion of another contact"
    );
    (dir, session, bundle)
}

fn request(bundle: &LocalInsightView) -> InsightContextRequest {
    InsightContextRequest {
        bundle_id: bundle.bundle_id,
        contact_id: Uuid::parse_str(&bundle.summaries[0].contact_id).unwrap(),
    }
}

fn select(session: &mut Session, bundle: &LocalInsightView) -> InsightContextView {
    session.prepare_insight_context(request(bundle)).unwrap()
}

fn no_draft_create(session: &Session) {
    assert!(!session
        .audit()
        .unwrap()
        .entries
        .iter()
        .any(|entry| entry.action == "draft.create"));
}

#[test]
fn local_reference_has_exact_citations_and_preserves_the_template_and_single_use() {
    let (_dir, mut session, bundle) = fixture();
    let endpoint = MockLlm::start().unwrap();
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
    let plain = session.draft_pasted("synthetic paste").unwrap();
    let context = select(&mut session, &bundle);
    let reference = &context.reference;
    let cited: BTreeSet<_> = reference
        .summary
        .points
        .iter()
        .flat_map(|point| point.evidence_ids.iter().cloned())
        .collect();
    let actual: BTreeSet<_> = reference
        .sources
        .iter()
        .map(|source| source.evidence_id.to_string())
        .collect();
    assert_eq!(actual, cited);
    assert_eq!(actual.len(), reference.sources.len());
    assert_eq!(reference.summary.source, "counts");
    assert!(reference.notice.contains("当前整库"));
    assert!(reference.notice.contains("不代表生成草稿逐句"));
    let shared = session.store().unwrap();
    let store = shared.lock().unwrap();
    for source in &reference.sources {
        let row = store.get_evidence(source.evidence_id).unwrap();
        let expected: BTreeSet<_> = soul_graph::interaction::interactions_in(&row)
            .expect("valid fixture")
            .into_iter()
            .map(|reference| reference.event_id)
            .collect();
        assert_eq!(
            source.event_ids.iter().copied().collect::<BTreeSet<_>>(),
            expected
        );
        assert_eq!(source.kind, row.kind);
    }
    let other_ids: BTreeSet<_> = bundle.summaries[1]
        .points
        .iter()
        .flat_map(|point| point.evidence_ids.iter().cloned())
        .collect();
    assert!(actual.is_disjoint(&other_ids));
    drop(store);
    let id = context.context_id.to_string();
    let local = session
        .local_draft_with_insight("synthetic paste", &id)
        .unwrap();
    assert_eq!(local.draft, plain);
    assert_eq!(local.reference, context.reference);
    assert!(session.local_draft_with_insight("next", &id).is_err());
    assert_eq!(endpoint.request_count(), 0);
}

#[test]
fn exact_bundle_lookup_is_not_limited_to_recent_feedback_and_rejects_untrusted_input() {
    let (_dir, mut session, bundle) = fixture();
    let shared = session.store().unwrap();
    {
        let mut store = shared.lock().unwrap();
        let operation = store.import_operation(bundle.operation_id).unwrap();
        for at in 0..101 {
            store
                .transact(|store| {
                    insights::create_for_import(store, &operation, &[], 2_000_000_000 + at)
                })
                .unwrap();
        }
    }
    assert!(!session
        .local_insights()
        .unwrap()
        .iter()
        .any(|row| row.bundle_id == bundle.bundle_id));
    assert!(session.prepare_insight_context(request(&bundle)).is_ok());
    let mut unknown = request(&bundle);
    unknown.bundle_id = Uuid::now_v7();
    assert!(session.prepare_insight_context(unknown).is_err());
    let mut unknown = request(&bundle);
    unknown.contact_id = Uuid::now_v7();
    assert!(session.prepare_insight_context(unknown).is_err());
    let wire = serde_json::json!({"bundle_id": bundle.bundle_id, "contact_id": bundle.summaries[0].contact_id, "text": "untrusted narrative"});
    assert!(serde_json::from_value::<InsightContextRequest>(wire).is_err());
    shared
        .lock()
        .unwrap()
        .set_local_insight_ignored(bundle.bundle_id, true)
        .unwrap();
    assert!(session.prepare_insight_context(request(&bundle)).is_err());
}

#[test]
fn wire_defaults_to_local_marker_and_only_explicit_counts_use_the_trusted_aggregate() {
    let (_dir, mut session, bundle) = fixture();
    let endpoint = MockLlm::start().unwrap();
    endpoint.set_reply("收到，我们稍后再聊。");
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
    for include_counts in [false, true] {
        let context = select(&mut session, &bundle);
        let prepared = session
            .prepare_draft_with_insight(
                "PASTE_SECRET_271",
                &context.context_id.to_string(),
                include_counts,
                false,
            )
            .unwrap();
        assert_eq!(prepared.reference, context.reference);
        assert_eq!(prepared.prepared.includes_counts, include_counts);
        assert!(!prepared.prepared.body_preview.contains("PASTE_SECRET_271"));
        assert_eq!(prepared.prepared.plan.third_party_turns, 1);
        assert_eq!(prepared.prepared.plan.placeheld_turns, 1);
        for point in &context.reference.summary.points {
            assert_eq!(
                prepared.prepared.body_preview.contains(&point.statement),
                include_counts
            );
        }
        let approval = prepared.prepared.plan.approval();
        session.generate_draft(&approval).unwrap();
        assert!(session.begin_draft(&approval).is_err());
        assert!(session
            .local_draft_with_insight("next", &context.context_id.to_string())
            .is_err());
        let recorded = endpoint.requests().pop().unwrap();
        assert_eq!(recorded.body.len(), prepared.prepared.encoded_bytes);
        let json: serde_json::Value = serde_json::from_str(&recorded.body).unwrap();
        assert_eq!(
            json["messages"][1]["content"],
            format!(
                "{}\n{}\n{}",
                soul_policy::e1::QUOTE_OPEN,
                prepared.prepared.body_preview,
                soul_policy::e1::QUOTE_CLOSE
            )
        );
        for id in [
            bundle.bundle_id,
            bundle.operation_id,
            bundle.namespace_id,
            context.reference.contact_id,
            context.reference.relationship_id,
        ] {
            assert!(!recorded.body.contains(&id.to_string()));
        }
        for source in &context.reference.sources {
            assert!(!recorded.body.contains(&source.evidence_id.to_string()));
            for id in &source.event_ids {
                assert!(!recorded.body.contains(&id.to_string()));
            }
        }
        for raw in [
            "u-lilei",
            "u-wangxiao",
            "好的没问题",
            "今晚在 café",
            "👩‍💻",
            "conversation_ref",
        ] {
            assert!(!recorded.body.contains(raw));
        }
    }
    assert_eq!(endpoint.request_count(), 2);
    let context = select(&mut session, &bundle);
    let included = session
        .prepare_draft_with_insight(
            "PASTE_EXPLICIT_849",
            &context.context_id.to_string(),
            true,
            true,
        )
        .unwrap();
    assert!(included
        .prepared
        .body_preview
        .contains("PASTE_EXPLICIT_849"));
    assert!(included.prepared.plan.carries_exempted_original);
    assert_eq!(
        included.prepared.plan.third_party_turns, 1,
        "aggregate is not another pasted or memory turn"
    );
    assert_eq!(
        endpoint.request_count(),
        2,
        "preparation itself has no HTTP"
    );
}

#[test]
fn an_exact_dependency_change_with_the_same_evidence_ids_refuses_before_dispatch() {
    let (_dir, mut session, bundle) = fixture();
    let endpoint = MockLlm::start().unwrap();
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
    let selected = select(&mut session, &bundle);
    let plan = session
        .prepare_draft_with_insight("synthetic", &selected.context_id.to_string(), true, false)
        .unwrap();
    let shared = session.store().unwrap();
    let mut store = shared.lock().unwrap();
    let mut edge = store
        .get_relationship(selected.reference.relationship_id)
        .unwrap();
    let before_ids = edge.evidence_ids.clone();
    let strength = edge.tie_strength.as_mut().unwrap();
    let old_count = strength["interaction_count"].as_u64().unwrap();
    strength["interaction_count"] = serde_json::json!(old_count + 1);
    store.put_relationship(edge).unwrap();
    assert_eq!(
        store
            .get_relationship(selected.reference.relationship_id)
            .unwrap()
            .evidence_ids,
        before_ids
    );
    drop(store);
    assert!(session.begin_draft(&plan.prepared.plan.approval()).is_err());
    assert_eq!(endpoint.request_count(), 0);
    no_draft_create(&session);
}

#[test]
fn direct_store_forget_after_http_is_audited_then_rejected_without_draft_create() {
    let (_dir, mut session, bundle) = fixture();
    let endpoint = MockLlm::start().unwrap();
    endpoint.set_reply("收到，我们稍后再聊。");
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
    let selected = select(&mut session, &bundle);
    let plan = session
        .prepare_draft_with_insight("synthetic", &selected.context_id.to_string(), true, false)
        .unwrap();
    let completion = session
        .begin_draft(&plan.prepared.plan.approval())
        .unwrap()
        .execute();
    session
        .store()
        .unwrap()
        .lock()
        .unwrap()
        .execute_forget(ForgetUnit::Contact(selected.reference.contact_id))
        .unwrap();
    assert!(session.complete_draft(completion).is_err());
    assert_eq!(endpoint.request_count(), 1);
    let audit = session.audit().unwrap();
    assert!(audit
        .entries
        .iter()
        .any(|entry| entry.action == "egress.request"));
    no_draft_create(&session);
    assert!(session.prepare_insight_context(request(&bundle)).is_err());
}

#[test]
fn another_contacts_changed_snapshot_invalidates_the_whole_selected_bundle() {
    let (_dir, mut session, bundle) = fixture();
    let selected = select(&mut session, &bundle);
    let other = Uuid::parse_str(&bundle.summaries[1].contact_id).unwrap();
    let shared = session.store().unwrap();
    let mut store = shared.lock().unwrap();
    let mut edge = store.relationships_for(other).unwrap().remove(0);
    assert_ne!(edge.relationship_id, selected.reference.relationship_id);
    let strength = edge.tie_strength.as_mut().unwrap();
    let old_count = strength["interaction_count"].as_u64().unwrap();
    strength["interaction_count"] = serde_json::json!(old_count + 1);
    store.put_relationship(edge).unwrap();
    drop(store);
    assert!(session
        .local_draft_with_insight("synthetic", &selected.context_id.to_string())
        .is_err());
}

#[test]
fn replacement_paths_cannot_reuse_the_old_context_or_approve_a_new_one() {
    let (_dir, mut session, bundle) = fixture();
    let endpoint = MockLlm::start().unwrap();
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
    let first = select(&mut session, &bundle);
    let old = session
        .prepare_draft_with_insight("synthetic", &first.context_id.to_string(), false, false)
        .unwrap();
    let second = select(&mut session, &bundle);
    assert!(session.begin_draft(&old.prepared.plan.approval()).is_err());
    assert!(session
        .local_draft_with_insight("synthetic", &second.context_id.to_string())
        .is_ok());
    let selected = select(&mut session, &bundle);
    let pending = session
        .prepare_draft_with_insight("synthetic", &selected.context_id.to_string(), true, false)
        .unwrap();
    session.prepare_draft("plain replacement", None).unwrap();
    assert!(session
        .begin_draft(&pending.prepared.plan.approval())
        .is_err());
    assert!(session
        .local_draft_with_insight("synthetic", &selected.context_id.to_string())
        .is_err());
    let memory = session
        .write_memory(&NewMemory {
            memory_type: "preference".into(),
            title: "synthetic preference".into(),
            summary: "synthetic note".into(),
        })
        .unwrap();
    let selected = select(&mut session, &bundle);
    let pending = session
        .prepare_draft_with_insight("synthetic", &selected.context_id.to_string(), true, false)
        .unwrap();
    let memory_context = session
        .prepare_memory_context(MemoryContextRequest {
            candidate_ids: vec![Uuid::parse_str(&memory.memory_id).unwrap()],
            query: None,
            memory_type: None,
            time_specific: false,
        })
        .unwrap()
        .context_id
        .unwrap();
    let memory_plan = session
        .prepare_draft_with_memory("synthetic", &memory_context.to_string(), &[], false)
        .unwrap();
    assert!(session
        .begin_draft(&pending.prepared.plan.approval())
        .is_err());
    let selected = select(&mut session, &bundle);
    assert!(session
        .begin_draft(&memory_plan.prepared.plan.approval())
        .is_err());
    assert!(session
        .draft_pasted_with_memory("synthetic", &memory_context.to_string())
        .is_err());
    session.draft_pasted("plain local replacement").unwrap();
    assert!(session
        .local_draft_with_insight("synthetic", &selected.context_id.to_string())
        .is_err());
    assert_eq!(endpoint.request_count(), 0);
}

#[test]
fn session_recompute_and_ignore_revoke_grants_but_direct_store_new_bundle_does_not() {
    let (_dir, mut session, bundle) = fixture();
    let selected = select(&mut session, &bundle);
    let shared = session.store().unwrap();
    let recomputed =
        insights::recompute(&mut shared.lock().unwrap(), bundle.bundle_id, 2_000_000_000).unwrap();
    assert_ne!(recomputed.bundle_id, bundle.bundle_id);
    assert_eq!(
        shared
            .lock()
            .unwrap()
            .local_insight(bundle.bundle_id)
            .unwrap()
            .validity,
        InsightValidity::Active
    );
    assert!(session
        .local_draft_with_insight("synthetic", &selected.context_id.to_string())
        .is_ok());
    let selected = select(&mut session, &bundle);
    session
        .recompute_local_insight(&bundle.bundle_id.to_string())
        .unwrap();
    assert!(session
        .local_draft_with_insight("synthetic", &selected.context_id.to_string())
        .is_err());
    let selected = select(&mut session, &bundle);
    session
        .ignore_local_insight(&bundle.bundle_id.to_string())
        .unwrap();
    assert!(session
        .local_draft_with_insight("synthetic", &selected.context_id.to_string())
        .is_err());
    assert!(session.prepare_insight_context(request(&bundle)).is_err());
}

#[test]
fn whole_statistics_overflow_refuses_when_the_same_paste_base_fits_and_clears_old_pending() {
    let (_dir, mut session, bundle) = fixture();
    let endpoint = MockLlm::start().unwrap();
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
    let selected = select(&mut session, &bundle);
    let id = selected.context_id.to_string();
    let empty = session
        .prepare_draft_with_insight("", &id, true, true)
        .unwrap();
    let pasted = "q".repeat(MAX_MEMORY_REQUEST_BYTES - empty.prepared.encoded_bytes + 1);
    // Independent base construction uses the production request and serializer.
    // It has no fake statistical points and no widened input limit.
    let request = DraftRequest::from_paste(ProfileBrief::neutral(), &pasted);
    let exemption = ExemptionRequest::for_turn(request.third_party_turn_ids()[0]).confirm(true);
    let drafter = Drafter::new(Redactor::new(KnownIdentifiers::new()), UNNAMED_MODEL);
    let base = drafter.redact(&request, exemption).unwrap();
    assert!(
        encoded_chat_body(E1Purpose::Draft, UNNAMED_MODEL, &base).len() <= MAX_MEMORY_REQUEST_BYTES
    );
    let local_only = session
        .prepare_draft_with_insight(&pasted, &id, false, true)
        .unwrap();
    assert!(local_only.prepared.encoded_bytes <= MAX_MEMORY_REQUEST_BYTES);
    assert!(session
        .prepare_draft_with_insight(&pasted, &id, true, true)
        .is_err());
    assert!(session
        .begin_draft(&local_only.prepared.plan.approval())
        .is_err());
    assert_eq!(endpoint.request_count(), 0);
    no_draft_create(&session);
}

#[test]
fn restart_keeps_bundles_but_cannot_restore_a_context_or_approval() {
    let (dir, mut session, bundle) = fixture();
    let endpoint = MockLlm::start().unwrap();
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
    let selected = select(&mut session, &bundle);
    let plan = session
        .prepare_draft_with_insight("synthetic", &selected.context_id.to_string(), true, false)
        .unwrap();
    drop(session);
    let mut restarted = Session::open(dir.path());
    assert!(restarted.status().store_opened);
    assert!(restarted
        .local_draft_with_insight("synthetic", &selected.context_id.to_string())
        .is_err());
    assert!(restarted
        .begin_draft(&plan.prepared.plan.approval())
        .is_err());
    assert!(restarted.prepare_insight_context(request(&bundle)).is_ok());
    assert_eq!(endpoint.request_count(), 0);
}
