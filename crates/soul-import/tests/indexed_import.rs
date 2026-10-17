use soul_import::indexed::{commit_indexed, new_namespace, prepare_indexed};
use soul_import::{ImportError, ImportSource, StagedImport};
use soul_policy::injection::UntrustedText;
use soul_schema::memory::ForgetState;
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::import::*;
use soul_store_api::types::EventFilter;
use soul_store_api::{BlobStore, EventStore, ForgetOps, ForgetUnit, GraphStore};
use soul_testkit::fixtures;

fn staged() -> StagedImport {
    soul_import::soul_import_v1::parse(
        &fixtures::read_text("import/soul-import-v1/valid_basic.jsonl").unwrap(),
    )
    .unwrap()
}
fn store(dir: &std::path::Path) -> SqlCipherStore {
    SqlCipherStore::open(
        dir.join("soul.db"),
        &TestKeyProvider::from_seed("indexed imports"),
    )
    .unwrap()
}
fn apply(
    store: &mut SqlCipherStore,
    input: &StagedImport,
    ns: ImportNamespace,
    choice: LegacyImportChoice,
) -> Result<soul_import::ImportReceipt, ImportError> {
    let plan = prepare_indexed(store, input, ns, choice)?;
    store.transact(|store| {
        let receipt = commit_indexed(store, input, &plan)?;
        for audit in receipt.audit.clone() {
            soul_policy::audit::append_or_store_error(store, audit, 1_800_000_000)?;
        }
        soul_graph::rebuild(store).map_err(|_| ImportError::IdentityUnavailable)?;
        Ok(receipt)
    })
}

#[test]
fn repeated_and_overlapping_exports_do_not_grow_events_evidence_or_ties() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(dir.path());
    let input = staged();
    let ns = new_namespace(input.source, ImportCoverage::FullFromCreation);
    let id = ns.namespace_id;
    let first = apply(&mut store, &input, ns, LegacyImportChoice::Reject).unwrap();
    let before = store.list_relationships().unwrap();
    let ns = store.import_namespace(id).unwrap();
    let plan = prepare_indexed(&store, &input, ns.clone(), LegacyImportChoice::Reject).unwrap();
    assert_eq!(plan.new_messages(), 0);
    assert_eq!(plan.duplicate_messages(), input.messages.len() as u64);
    let again = apply(&mut store, &input, ns.clone(), LegacyImportChoice::Reject).unwrap();
    assert!(again.events_written.is_empty());
    assert!(again.evidence_written.is_empty());
    assert_eq!(store.list_relationships().unwrap(), before);
    assert_eq!(
        store.count_events(&EventFilter::all()).unwrap(),
        first.events_written.len() as u64
    );
    let mut overlap = input.clone();
    overlap.messages[0].external_id = "new-stable-id".into();
    assert_eq!(
        apply(&mut store, &overlap, ns, LegacyImportChoice::Reject)
            .unwrap()
            .events_written
            .len(),
        1
    );
}

#[test]
fn same_id_changed_body_or_metadata_is_a_conflict_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(dir.path());
    let input = staged();
    let ns = new_namespace(input.source, ImportCoverage::FullFromCreation);
    let id = ns.namespace_id;
    apply(&mut store, &input, ns, LegacyImportChoice::Reject).unwrap();
    let before = store.count_events(&EventFilter::all()).unwrap();
    let mut changed = input.clone();
    changed.messages[0].body = UntrustedText::new("synthetic changed prose");
    assert!(matches!(
        prepare_indexed(
            &store,
            &changed,
            store.import_namespace(id).unwrap(),
            LegacyImportChoice::Reject
        ),
        Err(ImportError::IdentityConflict)
    ));
    assert_eq!(store.count_events(&EventFilter::all()).unwrap(), before);
}

#[test]
fn duplicate_ids_inside_one_file_collapse_and_conflicting_duplicates_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(dir.path());
    let mut input = staged();
    input.messages.push(input.messages[0].clone());
    let ns = new_namespace(input.source, ImportCoverage::FullFromCreation);
    let plan = prepare_indexed(&store, &input, ns.clone(), LegacyImportChoice::Reject).unwrap();
    assert_eq!(plan.duplicate_messages(), 1);
    input.messages.last_mut().unwrap().body = UntrustedText::new("synthetic conflict");
    assert!(matches!(
        prepare_indexed(&store, &input, ns, LegacyImportChoice::Reject),
        Err(ImportError::IdentityConflict)
    ));
}

#[test]
fn a_preview_dependency_change_requires_a_new_preparation() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(dir.path());
    let input = staged();
    let ns = new_namespace(input.source, ImportCoverage::FullFromCreation);
    let plan = prepare_indexed(&store, &input, ns.clone(), LegacyImportChoice::Reject).unwrap();
    apply(&mut store, &input, ns, LegacyImportChoice::Reject).unwrap();
    let result = store.transact(|store| commit_indexed(store, &input, &plan));
    assert!(result.is_err());
    assert_eq!(
        store.count_events(&EventFilter::all()).unwrap(),
        input.messages.len() as u64
    );
}

#[test]
fn legacy_rows_require_an_explicit_forward_only_choice_and_remain_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(dir.path());
    let input = staged();
    soul_import::commit(&mut store, &input).unwrap();
    let ns = new_namespace(input.source, ImportCoverage::ForwardOnly);
    let id = ns.namespace_id;
    assert!(matches!(
        prepare_indexed(&store, &input, ns.clone(), LegacyImportChoice::Reject),
        Err(ImportError::LegacyChoiceRequired)
    ));
    let old = store.list_events(&EventFilter::all()).unwrap();
    apply(&mut store, &input, ns, LegacyImportChoice::ForwardOnly).unwrap();
    for event in old {
        assert_eq!(store.get_event(event.event_id).unwrap(), event);
    }
    assert_eq!(
        store.count_events(&EventFilter::all()).unwrap(),
        (input.messages.len() * 2) as u64
    );
    assert_eq!(
        store.import_namespace(id).unwrap().coverage,
        ImportCoverage::ForwardOnly
    );
}

#[test]
fn forget_revokes_its_namespace_destroys_sealed_ids_and_does_not_create_legacy_rows() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(dir.path());
    let input = staged();
    let ns = new_namespace(input.source, ImportCoverage::FullFromCreation);
    let id = ns.namespace_id;
    apply(&mut store, &input, ns, LegacyImportChoice::Reject).unwrap();
    let peer = store
        .list_contacts()
        .unwrap()
        .into_iter()
        .find(|c| c.contact_class == soul_schema::contact::ContactClass::ThirdParty)
        .unwrap();
    let identity = store
        .import_participant_batch(id, None, 256)
        .unwrap()
        .into_iter()
        .find(|i| i.contact_id == peer.contact_id)
        .unwrap();
    let preview = store
        .preview_impact(ForgetUnit::Contact(peer.contact_id))
        .unwrap();
    assert_eq!(preview.import_namespaces_revoked, 1);
    store
        .execute_forget(ForgetUnit::Contact(peer.contact_id))
        .unwrap();
    assert!(store.open(&identity.identity_ref).is_err());
    assert_eq!(
        store.import_namespace(id).unwrap().state,
        ImportNamespaceState::Revoked
    );
    assert_eq!(
        store
            .import_legacy_events(ImportSourceKind::SoulImportV1)
            .unwrap(),
        0
    );
    drop(store);
    let store = crate::store(dir.path());
    assert_eq!(
        store.get_contact(peer.contact_id).unwrap().forget_state,
        ForgetState::Forgotten
    );
    assert!(prepare_indexed(
        &store,
        &input,
        store.import_namespace(id).unwrap(),
        LegacyImportChoice::Reject
    )
    .is_err());
    assert!(prepare_indexed(
        &store,
        &input,
        new_namespace(ImportSource::SoulImportV1, ImportCoverage::FullFromCreation),
        LegacyImportChoice::Reject
    )
    .is_err());
}

#[test]
fn an_unreadable_or_replayed_identity_is_refused_before_it_can_be_written() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(dir.path());
    let input = staged();
    let ns = new_namespace(input.source, ImportCoverage::FullFromCreation);
    let id = ns.namespace_id;
    apply(&mut store, &input, ns, LegacyImportChoice::Reject).unwrap();
    let mut corrupt = store.import_message_batch(id, None, 256).unwrap().remove(0);
    corrupt.identity_id = uuid::Uuid::now_v7();
    corrupt.event_id = uuid::Uuid::now_v7();
    corrupt.identity_ref.blob_id = uuid::Uuid::now_v7();
    assert!(store.put_import_message(&corrupt).is_err());
    assert_eq!(
        prepare_indexed(
            &store,
            &input,
            store.import_namespace(id).unwrap(),
            LegacyImportChoice::Reject
        )
        .unwrap()
        .new_messages(),
        0
    );
    assert_eq!(
        store.count_events(&EventFilter::all()).unwrap(),
        input.messages.len() as u64
    );
}

#[test]
fn forward_only_jsonl_namespaces_do_not_merge_third_party_local_ids() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(dir.path());
    let input = staged();
    soul_import::commit(&mut store, &input).unwrap();
    let old_peers: std::collections::BTreeSet<_> = store
        .list_contacts()
        .unwrap()
        .into_iter()
        .filter(|contact| contact.contact_class == soul_schema::contact::ContactClass::ThirdParty)
        .map(|contact| contact.contact_id)
        .collect();
    let first = new_namespace(input.source, ImportCoverage::ForwardOnly);
    let first_id = first.namespace_id;
    let receipt = apply(&mut store, &input, first, LegacyImportChoice::ForwardOnly).unwrap();
    assert!(receipt
        .contacts_created
        .iter()
        .all(|id| !old_peers.contains(id)));
    let first_peers: std::collections::BTreeSet<_> = store
        .import_participant_batch(first_id, None, 256)
        .unwrap()
        .into_iter()
        .map(|identity| identity.contact_id)
        .filter(|id| {
            !store
                .get_contact(*id)
                .unwrap()
                .contact_class
                .eq(&soul_schema::contact::ContactClass::Owner)
        })
        .collect();
    let same_namespace = store.import_namespace(first_id).unwrap();
    let same = apply(
        &mut store,
        &input,
        same_namespace,
        LegacyImportChoice::ForwardOnly,
    )
    .unwrap();
    assert!(same.contacts_created.is_empty());
    assert!(same.events_written.is_empty());
    let second = new_namespace(input.source, ImportCoverage::ForwardOnly);
    let second_id = second.namespace_id;
    apply(&mut store, &input, second, LegacyImportChoice::ForwardOnly).unwrap();
    let second_peers: std::collections::BTreeSet<_> = store
        .import_participant_batch(second_id, None, 256)
        .unwrap()
        .into_iter()
        .map(|identity| identity.contact_id)
        .filter(|id| {
            store.get_contact(*id).unwrap().contact_class
                == soul_schema::contact::ContactClass::ThirdParty
        })
        .collect();
    assert!(!first_peers.is_empty());
    assert!(!second_peers.is_empty());
    assert!(first_peers.is_disjoint(&second_peers));
    assert!(second_peers.is_disjoint(&old_peers));
}
