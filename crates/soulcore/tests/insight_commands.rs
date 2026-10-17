//! W5 feedback uses the existing local graph/A2 path and revalidates its facts.
use soul_import::indexed::{commit_indexed, new_namespace, prepare_indexed};
use soul_schema::common::SupportedBand;
use soul_store::import_ops::ImportRuntimeLease;
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::import::*;
use soul_store_api::insight::{InsightValidity, LocalInsightBundle};
use soul_store_api::{AuditLog, EventStore, ForgetOps, ForgetUnit, GraphStore, ProfileStore};
use soul_testkit::fixtures;
use soulcore::commands::insights;
use soulcore::commands::session::SessionRefusal;
use uuid::Uuid;

const AT: i64 = 1_800_000_000;

fn open(directory: &std::path::Path) -> SqlCipherStore {
    SqlCipherStore::open(
        directory.join("soul.db"),
        &TestKeyProvider::from_seed("W5 local feedback"),
    )
    .unwrap()
}

fn fixture() -> (tempfile::TempDir, SqlCipherStore, LocalInsightBundle) {
    let directory = tempfile::tempdir().unwrap();
    let lease = ImportRuntimeLease::claim(directory.path().join("soul.db")).unwrap();
    let mut store = open(directory.path());
    store.recover_import_operations(&lease).unwrap();
    let input = soul_import::soul_import_v1::parse(
        &fixtures::read_text("import/soul-import-v1/valid_basic.jsonl").unwrap(),
    )
    .unwrap();
    let namespace = new_namespace(input.source, ImportCoverage::FullFromCreation);
    let plan = prepare_indexed(
        &store,
        &input,
        namespace.clone(),
        LegacyImportChoice::Reject,
    )
    .unwrap();
    let operation = ImportOperation {
        operation_id: Uuid::now_v7(),
        preview_id: Uuid::now_v7(),
        source: namespace.source,
        namespace_id: namespace.namespace_id,
        legacy_choice: LegacyImportChoice::Reject,
        state: ImportOperationState::Reserved,
        preview: ImportCounts::default(),
        receipt: None,
        error_code: None,
        audit_state: ImportAuditState::NotApplicable,
        created_at: AT,
    };
    store.reserve_import_operation(&lease, &operation).unwrap();
    store
        .start_import_operation(&lease, operation.operation_id, operation.preview_id)
        .unwrap();
    let result: Result<_, SessionRefusal> = store.transact(|store| {
        let receipt = commit_indexed(store, &input, &plan).unwrap();
        for entry in receipt.audit.clone() {
            soul_policy::audit::append_or_store_error(store, entry, AT)?;
        }
        let graph = soul_graph::rebuild(store)?;
        for entry in graph.audit {
            soul_policy::audit::append_or_store_error(store, entry, AT)?;
        }
        let counts = ImportCounts {
            participants: input.participants.len() as u64,
            messages: input.messages.len() as u64,
            events_written: receipt.events_written.len() as u64,
            evidence_written: receipt.evidence_written.len() as u64,
            contacts_created: receipt.contacts_created.len() as u64,
            contacts_matched: receipt.contacts_matched.len() as u64,
            ties_rebuilt: graph.edges_written.len() as u64,
            ..Default::default()
        };
        let committed = store.commit_import_operation(&lease, operation.operation_id, counts)?;
        insights::create_for_import(store, &committed, &receipt.contacts_touched(), AT)
    });
    (directory, store, result.unwrap())
}

#[test]
fn local_feedback_rebuilds_a2_after_restart_without_endpoint_or_new_audit() {
    let (directory, store, bundle) = fixture();
    let audit_count = store.list_audit().unwrap().len();
    drop(store);
    let mut store = open(directory.path());
    let views = insights::views(&mut store).unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].bundle_id, bundle.bundle_id);
    assert_eq!(views[0].counts, bundle.counts);
    assert_eq!(views[0].validity, InsightValidity::Active);
    assert!(!views[0].summaries.is_empty());
    for summary in &views[0].summaries {
        assert_eq!(summary.source, "counts");
        assert!(!summary.clinical_claim);
        assert!(summary
            .points
            .iter()
            .all(|point| !point.evidence_ids.is_empty()));
        let id = Uuid::parse_str(&summary.contact_id).unwrap();
        assert_ne!(
            store.get_contact(id).unwrap().contact_class,
            soul_schema::contact::ContactClass::Owner
        );
    }
    assert_eq!(store.list_audit().unwrap().len(), audit_count);
    let persisted = serde_json::to_string(&store.local_insight(bundle.bundle_id).unwrap()).unwrap();
    assert!(!persisted.contains("今晚"));
    assert!(!persisted.contains("conversation_ref"));
    assert!(!persisted.contains("source_refs"));
    assert!(!persisted.contains(&views[0].summaries[0].text));
}

#[test]
fn automatic_feedback_requires_a_transaction_and_rolls_back_with_its_domain() {
    let (_directory, mut store, bundle) = fixture();
    let operation = store.import_operation(bundle.operation_id).unwrap();
    assert!(insights::create_for_import(&mut store, &operation, &[], AT + 1).is_err());
    let mut attempted = None;
    let result: Result<(), SessionRefusal> = store.transact(|store| {
        attempted = Some(insights::create_for_import(store, &operation, &[], AT + 1)?.bundle_id);
        Err(SessionRefusal {
            reason_code: "TEST_ROLLBACK".into(),
            explanation: "synthetic failure".into(),
        })
    });
    assert!(result.is_err());
    assert!(store.local_insight(attempted.unwrap()).is_err());
    assert_eq!(store.recent_local_insights(100).unwrap().len(), 1);
}

#[test]
fn correction_invalidates_exact_snapshot_and_recompute_preserves_old_ignore() {
    let (_directory, mut store, bundle) = fixture();
    let before = insights::views(&mut store).unwrap().remove(0);
    let ignored = insights::ignore(&mut store, bundle.bundle_id).unwrap();
    assert!(ignored.ignored);
    assert!(ignored.summaries.is_empty());
    assert_eq!(ignored.validity, InsightValidity::Active);
    let contact = Uuid::parse_str(&before.summaries[0].contact_id).unwrap();
    let relationship = store.relationships_for(contact).unwrap().remove(0);
    soul_graph::correct_tie(
        &mut store,
        relationship.relationship_id,
        SupportedBand::Strong,
        AT + 1,
    )
    .unwrap();
    let invalid = insights::views(&mut store).unwrap().remove(0);
    assert_eq!(invalid.validity, InsightValidity::Invalidated);
    assert!(invalid.ignored);
    assert!(invalid.summaries.is_empty());
    assert_eq!(invalid.counts, before.counts);
    let new = insights::recompute(&mut store, bundle.bundle_id, AT + 2).unwrap();
    assert_ne!(new.bundle_id, bundle.bundle_id);
    assert_eq!(new.operation_id, bundle.operation_id);
    assert_eq!(new.validity, InsightValidity::Active);
    assert!(!new.ignored);
    assert!(!new.summaries.is_empty());
    assert!(store.local_insight(bundle.bundle_id).unwrap().ignored);
}

#[test]
fn same_count_replaced_evidence_invalidates_without_session_revision() {
    let (_directory, mut store, bundle) = fixture();
    let mut edge = store.list_relationships().unwrap().remove(0);
    let mut replacement = store.get_evidence(edge.evidence_ids[0]).unwrap();
    replacement.evidence_id = Uuid::now_v7();
    store.put_evidence(replacement.clone()).unwrap();
    edge.evidence_ids[0] = replacement.evidence_id;
    store.put_relationship(edge).unwrap();
    let view = insights::views(&mut store).unwrap().remove(0);
    assert_eq!(view.bundle_id, bundle.bundle_id);
    assert_eq!(view.validity, InsightValidity::Invalidated);
    assert!(view.summaries.is_empty());
}

#[test]
fn body_pointer_replay_is_not_a_readable_source_and_cannot_be_recomputed() {
    let (_directory, mut store, bundle) = fixture();
    let edge = store.list_relationships().unwrap().remove(0);
    let evidence = store.get_evidence(edge.evidence_ids[0]).unwrap();
    let observation = soul_graph::interaction::interactions_in(&evidence).remove(0);
    let mut event = store.get_event(observation.event_id).unwrap();
    // append-only events cannot be overwritten through the store. Instead,
    // point the evidence at a real event with another row's canonical seal.
    let original = event.event_id;
    event.event_id = Uuid::now_v7();
    store.append_event(event.clone()).unwrap();
    let mut changed = evidence.clone();
    let mut reference = soul_graph::interaction::interactions_in(&changed).remove(0);
    assert_ne!(reference.event_id, event.event_id);
    assert_eq!(reference.event_id, original);
    reference.event_id = event.event_id;
    changed.source_refs = vec![reference.to_source_ref()];
    store.put_evidence(changed).unwrap();
    let view = insights::views(&mut store).unwrap().remove(0);
    assert_eq!(view.validity, InsightValidity::Invalidated);
    assert!(view.summaries.is_empty());
    let before = store.recent_local_insights(100).unwrap().len();
    assert!(insights::recompute(&mut store, bundle.bundle_id, AT + 1).is_err());
    assert_eq!(store.recent_local_insights(100).unwrap().len(), before);
}

#[test]
fn forget_revokes_feedback_but_keeps_only_historical_counts_after_restart() {
    let (directory, mut store, bundle) = fixture();
    let current = insights::views(&mut store).unwrap().remove(0);
    let contact = Uuid::parse_str(&current.summaries[0].contact_id).unwrap();
    store.execute_forget(ForgetUnit::Contact(contact)).unwrap();
    drop(store);
    let mut store = open(directory.path());
    let historical = insights::views(&mut store).unwrap().remove(0);
    assert_eq!(historical.validity, InsightValidity::Invalidated);
    assert!(historical.summaries.is_empty());
    assert_eq!(historical.counts, bundle.counts);
    assert!(insights::recompute(&mut store, bundle.bundle_id, AT + 1).is_err());
}

#[test]
fn owner_only_candidates_yield_a_receipt_and_no_fabricated_summary() {
    let (_directory, mut store, bundle) = fixture();
    let operation = store.import_operation(bundle.operation_id).unwrap();
    let owner = store
        .list_contacts()
        .unwrap()
        .into_iter()
        .find(|c| c.contact_class == soul_schema::contact::ContactClass::Owner)
        .unwrap()
        .contact_id;
    let receipt_only = store
        .transact(|store| insights::create_for_import(store, &operation, &[owner], AT + 1))
        .unwrap();
    let view = insights::views(&mut store)
        .unwrap()
        .into_iter()
        .find(|v| v.bundle_id == receipt_only.bundle_id)
        .unwrap();
    assert!(view.summaries.is_empty());
    assert_eq!(view.counts, bundle.counts);
    assert!(view.notice.contains("历史操作计数"));
}
