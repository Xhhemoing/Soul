//! Real SQLCipher reads preserve citation slots, errors and request freshness.

mod people_view_support;

use std::collections::BTreeSet;

use people_view_support::{
    evidence, evidence_id, expected_view, id, peer, relationship, seed, Shape, SEED, SLOTS_PER_EDGE,
};
use serde_json::json;
use soul_graph::GraphError;
use soul_schema::common::{SealedSubject, SupportedBand};
use soul_schema::evidence::EvidenceMethod;
use soul_store::{KeyProvider, SqlCipherStore, TestKeyProvider};
use soul_store_api::forget::{ForgetOps, ForgetUnit};
use soul_store_api::types::{SealRequest, StoreError};
use soul_store_api::{BlobStore, GraphStore, ProfileStore};
use soulcore::commands::{graph, store as store_commands};

fn fixture(shape: Shape) -> (tempfile::TempDir, SqlCipherStore) {
    let directory = tempfile::tempdir().unwrap();
    let mut store = store_commands::open_test_store(directory.path(), SEED).unwrap();
    seed(&mut store, 3, shape);
    (directory, store)
}

fn raw(store: &SqlCipherStore) -> rusqlite::Connection {
    let connection = rusqlite::Connection::open(store.path()).unwrap();
    connection
        .execute_batch(&format!(
            "PRAGMA key = \"x'{}'\";",
            TestKeyProvider::from_seed(SEED)
                .database_key()
                .unwrap()
                .to_hex()
        ))
        .unwrap();
    connection
}

#[test]
fn unique_and_shared_citations_keep_complete_views_and_public_resolution_order() {
    for shape in [Shape::Unique, Shape::Shared] {
        let (_directory, store) = fixture(shape);
        let view = graph::people_view(&store).unwrap();
        assert_eq!(view, expected_view(3, shape), "{}", shape.name());
        let slots: Vec<_> = view.ties.iter().flat_map(|tie| &tie.evidence).collect();
        assert_eq!(slots.len(), 3 * SLOTS_PER_EDGE);
        assert_eq!(
            slots
                .iter()
                .map(|row| &row.evidence_id)
                .collect::<BTreeSet<_>>()
                .len(),
            shape.distinct(3)
        );
        let loaded = graph::load(&store).unwrap();
        for edge in &loaded.edges {
            let expected: Vec<_> = edge
                .evidence_ids
                .iter()
                .map(|id| store.get_evidence(*id).unwrap())
                .collect();
            assert_eq!(graph::edge_evidence(&store, edge).unwrap(), expected);
            assert_eq!(
                graph::evidence_for(&store, edge.relationship_id).unwrap(),
                expected
            );
        }
    }
}

#[test]
fn duplicates_before_missing_rows_do_not_change_the_first_missing_id() {
    let (_directory, mut store) = fixture(Shape::Shared);
    let first_missing = id(800_001);
    let later_missing = id(800_000);
    let mut first = store.get_relationship(relationship(0)).unwrap();
    first.evidence_ids = vec![evidence_id(0), evidence_id(0), first_missing];
    store.put_relationship(first).unwrap();
    let mut later = store.get_relationship(relationship(1)).unwrap();
    later.evidence_ids = vec![later_missing];
    store.put_relationship(later).unwrap();
    assert!(matches!(
        graph::people_view(&store),
        Err(GraphError::Store(StoreError::NotFound { kind: "evidence", id })) if id == first_missing
    ));
    assert!(matches!(
        graph::evidence_for(&store, relationship(0)),
        Err(GraphError::Store(StoreError::NotFound { kind: "evidence", id })) if id == first_missing
    ));
}

#[test]
fn unreferenced_bad_documents_are_not_read_and_first_referenced_failure_wins() {
    let (_directory, mut store) = fixture(Shape::Shared);
    let connection = raw(&store);
    // This stored row exists in both shapes, but the shared edges never cite it.
    connection
        .execute(
            "UPDATE evidence SET doc = ?1 WHERE evidence_id = ?2",
            rusqlite::params!["not-json", evidence_id(3).to_string()],
        )
        .unwrap();
    assert!(store.get_evidence(evidence_id(3)).is_err());
    assert_eq!(
        graph::people_view(&store).unwrap(),
        expected_view(3, Shape::Shared)
    );

    // A damaged document can disagree with its lookup key. The old reader
    // returned that document as-is; caching must still use the requested UUID
    // so it cannot shadow the different row requested in the next slot.
    let mut mismatched = evidence(0);
    mismatched.evidence_id = evidence_id(1);
    connection
        .execute(
            "UPDATE evidence SET doc = ?1 WHERE evidence_id = ?2",
            rusqlite::params![
                serde_json::to_string(&mismatched).unwrap(),
                evidence_id(0).to_string()
            ],
        )
        .unwrap();
    let mut expected_mismatch = expected_view(3, Shape::Shared);
    for tie in &mut expected_mismatch.ties {
        for slot in [0, 2] {
            tie.evidence[slot].evidence_id = evidence_id(1).to_string();
        }
    }
    assert_eq!(graph::people_view(&store).unwrap(), expected_mismatch);
    store.put_evidence(evidence(0)).unwrap();

    let mut first = store.get_relationship(relationship(0)).unwrap();
    // Traversal must resolve id 1 first, even though id 0 sorts before it.
    first.evidence_ids = vec![evidence_id(1), evidence_id(1), evidence_id(0)];
    store.put_relationship(first).unwrap();
    for (index, field) in [(0, "kind"), (1, "method")] {
        let mut document = serde_json::to_value(evidence(index)).unwrap();
        document[field] = json!("unrecognized");
        connection
            .execute(
                "UPDATE evidence SET doc = ?1 WHERE evidence_id = ?2",
                rusqlite::params![document.to_string(), evidence_id(index).to_string()],
            )
            .unwrap();
    }
    let expected_error = store.get_evidence(evidence_id(1)).unwrap_err();
    assert_ne!(
        expected_error,
        store.get_evidence(evidence_id(0)).unwrap_err()
    );
    match graph::people_view(&store) {
        Err(GraphError::Store(error)) => assert_eq!(error, expected_error),
        other => panic!("expected first referenced bad document, got {other:?}"),
    }
}

#[test]
fn later_requests_observe_changed_metadata_and_real_forget_after_warm_reads() {
    let (directory, mut store) = fixture(Shape::Shared);
    // Give this synthetic contact a real forget unit; no platform keys or
    // imported private material participate in the key-destruction path.
    let content_key = id(700_000);
    let label = store
        .seal(SealRequest::new(
            content_key,
            peer(0),
            "display_label_ref",
            SealedSubject::ThirdParty,
            b"Synthetic contact".to_vec(),
        ))
        .unwrap();
    let mut contact = store.get_contact(peer(0)).unwrap();
    contact.display_label_ref = Some(label);
    store.put_contact(contact).unwrap();
    assert_eq!(
        graph::people_view(&store).unwrap(),
        expected_view(3, Shape::Shared)
    );

    let mut changed = store.get_evidence(evidence_id(0)).unwrap();
    changed.strength = SupportedBand::Strong;
    changed.method = Some(EvidenceMethod::Statistical);
    changed.source_refs = vec![json!({"synthetic_revision": 2})];
    store.put_evidence(changed).unwrap();
    let mut expected = expected_view(3, Shape::Shared);
    for tie in &mut expected.ties {
        for slot in [0, 2] {
            tie.evidence[slot].strength = "strong".into();
            tie.evidence[slot].method = "statistical".into();
        }
    }
    assert_eq!(graph::people_view(&store).unwrap(), expected);

    let unit = ForgetUnit::Contact(peer(0));
    let impact = store.preview_impact(unit).unwrap();
    assert_eq!(impact.content_key_ids, [content_key]);
    assert_eq!(impact.contacts_affected, 1);
    assert_eq!(store.execute_forget(unit).unwrap().impact, impact);
    assert!(store.content_key_destroyed(content_key).unwrap());
    expected.people[1].forgotten = true;
    assert_eq!(graph::people_view(&store).unwrap(), expected);
    store.close().unwrap();
    let reopened = store_commands::open_test_store(directory.path(), SEED).unwrap();
    assert_eq!(graph::people_view(&reopened).unwrap(), expected);
}
