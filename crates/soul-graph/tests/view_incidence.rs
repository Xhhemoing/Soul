//! Stored-graph read semantics, before and after indexing incident edges.

use serde_json::json;
use soul_graph::GraphError;
use soul_schema::common::SchemaVersion;
use soul_schema::contact::{ContactClass, SoulContact};
use soul_schema::memory::ForgetState;
use soul_schema::relationship::{EgressScope, SoulRelationship};
use soul_store_api::{FakeStore, GraphStore};
use uuid::Uuid;

fn id(n: u128) -> Uuid {
    Uuid::from_u128(0x0192_a1b2_c3d4_7e5f_8a9b_0c1d_0000_0000 + n)
}

fn contact(n: u128, class: ContactClass, state: ForgetState) -> SoulContact {
    SoulContact {
        schema_version: SchemaVersion,
        contact_id: id(n),
        contact_class: class,
        display_label_ref: None,
        identifiers: None,
        forget_state: state,
    }
}

fn edge(n: u128, from: u128, to: u128, count: u64, last: &str) -> SoulRelationship {
    SoulRelationship {
        schema_version: SchemaVersion,
        relationship_id: id(n),
        from_contact_id: id(from),
        to_contact_id: id(to),
        types: Some(vec![json!("direct"), json!("one_sided")]),
        tie_strength: Some(json!({
            "band": "weak", "interaction_count": count,
            "outgoing_count": count, "incoming_count": 0,
            "conversation_count": 1, "active_day_count": 1,
            "first_contact_utc": "2026-08-23T00:00:00Z",
            "last_contact_utc": last
        })),
        evidence_ids: vec![id(900)],
        egress_scope: Some(EgressScope::LocalOnly),
    }
}

#[test]
fn incidence_keeps_both_directions_self_loops_missing_endpoints_and_list_order() {
    let mut store = FakeStore::new();
    for (n, class, state) in [
        (4, ContactClass::ThirdParty, ForgetState::Active),
        (2, ContactClass::ThirdParty, ForgetState::Active),
        (1, ContactClass::Owner, ForgetState::Active),
        (3, ContactClass::ThirdParty, ForgetState::Forgotten),
    ] {
        store.put_contact(contact(n, class, state)).unwrap();
    }
    // Chronological order differs from the original-string maximum. Loading
    // a persistent graph has always retained the latter, without normalization.
    let offset = "2026-08-23T23:00:00+10:00";
    let utc = "2026-08-23T14:00:00Z";
    for row in [
        edge(104, 3, 99, 11, utc),
        edge(102, 2, 1, 3, utc),
        edge(105, 98, 99, 13, utc),
        edge(101, 1, 2, 2, offset),
        edge(103, 2, 2, 5, utc),
    ] {
        store.put_relationship(row).unwrap();
    }

    let graph = soul_graph::load(&store).unwrap();
    assert_eq!(graph.self_contact_id, Some(id(1)));
    assert_eq!(
        graph
            .edges
            .iter()
            .map(|edge| edge.relationship_id)
            .collect::<Vec<_>>(),
        [id(101), id(102), id(103), id(104), id(105)]
    );
    let expected = [
        (1, 5, vec![id(101), id(102)], Some(offset)),
        (2, 10, vec![id(101), id(102), id(103)], Some(offset)),
        (3, 11, vec![id(104)], Some(utc)),
        (4, 0, vec![], None),
    ];
    assert_eq!(graph.nodes.len(), expected.len());
    for (node, (n, count, edges, last)) in graph.nodes.iter().zip(expected) {
        assert_eq!(node.contact_id, id(n));
        assert_eq!(node.interaction_count, count);
        assert_eq!(node.edge_ids, edges);
        assert_eq!(node.last_contact_utc.as_ref().map(|at| at.as_str()), last);
        assert_eq!(node.forget_state == ForgetState::Forgotten, n == 3);
    }
    assert_eq!(graph.edges[2].from_contact_id, graph.edges[2].to_contact_id);
    assert!(graph.third_party_data_is_local_only());
}

#[test]
fn no_owner_or_no_contacts_still_returns_all_readable_edges() {
    for with_contact in [false, true] {
        let mut store = FakeStore::new();
        if with_contact {
            store
                .put_contact(contact(2, ContactClass::ThirdParty, ForgetState::Active))
                .unwrap();
        }
        let stored = edge(101, 2, 99, 7, "2026-08-23T00:00:00Z");
        store.put_relationship(stored).unwrap();
        let graph = soul_graph::load(&store).unwrap();
        assert_eq!(graph.self_contact_id, None);
        assert_eq!(graph.nodes.len(), usize::from(with_contact));
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].relationship_id, id(101));
        if with_contact {
            assert_eq!(graph.nodes[0].interaction_count, 7);
            assert_eq!(graph.nodes[0].edge_ids, [id(101)]);
        }
    }
}

#[test]
fn unreadable_unrelated_edges_are_refused_in_store_order_even_without_contacts() {
    for with_owner in [false, true] {
        for field in ["types", "tie_strength"] {
            let mut store = FakeStore::new();
            if with_owner {
                store
                    .put_contact(contact(1, ContactClass::Owner, ForgetState::Active))
                    .unwrap();
            }
            let mut first = edge(101, 98, 99, 1, "2026-08-23T00:00:00Z");
            if field == "types" {
                first.types = Some(vec![json!("unrecognized")]);
            } else {
                first.tie_strength = Some(json!({}));
            }
            let mut later = edge(102, 1, 2, 1, "2026-08-23T00:00:00Z");
            later.types = None;
            store.put_relationship(later).unwrap();
            store.put_relationship(first).unwrap();
            assert!(matches!(
                soul_graph::load(&store),
                Err(GraphError::UnreadableEdge { relationship_id, field: found })
                    if relationship_id == id(101) && found == field
            ));
        }
    }
}

#[test]
fn ambiguous_owner_precedes_edge_validation() {
    let mut store = FakeStore::new();
    for n in [1, 2] {
        store
            .put_contact(contact(n, ContactClass::Owner, ForgetState::Active))
            .unwrap();
    }
    let mut broken = edge(101, 98, 99, 1, "2026-08-23T00:00:00Z");
    broken.types = None;
    store.put_relationship(broken).unwrap();
    assert!(matches!(
        soul_graph::load(&store),
        Err(GraphError::AmbiguousOwner { count: 2 })
    ));
}
