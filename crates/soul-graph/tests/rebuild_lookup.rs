//! Equivalence protection for the old-row lookup in graph rebuild.
//! These cases must pass both before and after the lookup optimization.

use serde_json::json;
use soul_graph::interaction::{conversation_ref, interaction_evidence, InteractionRef};
use soul_graph::{Direction, GraphError, TieStrength, Venue};
use soul_schema::common::{SchemaVersion, Subject, SupportedBand, Timestamp};
use soul_schema::contact::{ContactClass, SoulContact};
use soul_schema::inference::UserVerdict;
use soul_schema::memory::ForgetState;
use soul_store_api::{FakeStore, GraphStore, ProfileStore};
use uuid::Uuid;

fn id(n: u128) -> Uuid {
    Uuid::from_u128(0x0192_a1b2_c3d4_7e5f_8a9b_0c1d_0000_0000 + n)
}

fn seeded() -> FakeStore {
    let mut store = FakeStore::new();
    for (n, contact_class) in [(1, ContactClass::Owner), (2, ContactClass::ThirdParty)] {
        store
            .put_contact(SoulContact {
                schema_version: SchemaVersion,
                contact_id: id(n),
                contact_class,
                display_label_ref: None,
                identifiers: None,
                forget_state: ForgetState::Active,
            })
            .expect("synthetic contact");
    }
    for (n, direction) in [(0, Direction::Outgoing), (1, Direction::Incoming)] {
        store
            .put_evidence(interaction_evidence(
                id(100 + n),
                Subject::Mixed,
                &InteractionRef::new(
                    id(200 + n),
                    id(1),
                    id(2),
                    conversation_ref("lookup-test", "synthetic"),
                    direction,
                    Timestamp::new("2026-08-23T09:00:00Z"),
                    Venue::Direct,
                ),
            ))
            .expect("synthetic observation");
    }
    soul_graph::rebuild(&mut store).expect("initial rebuild");
    store
}

#[test]
fn duplicate_edges_and_tie_inferences_keep_the_first_row_in_either_direction() {
    for first_is_reversed in [false, true] {
        let mut store = seeded();
        let mut second = store.list_relationships().unwrap().remove(0);
        let mut first = second.clone();
        first.relationship_id = id(500);
        if first_is_reversed {
            std::mem::swap(&mut first.from_contact_id, &mut first.to_contact_id);
        } else {
            std::mem::swap(&mut second.from_contact_id, &mut second.to_contact_id);
        }
        for (edge, band) in [
            (&mut first, SupportedBand::Moderate),
            (&mut second, SupportedBand::Strong),
        ] {
            let mut strength: TieStrength =
                serde_json::from_value(edge.tie_strength.clone().unwrap()).unwrap();
            strength.band = band;
            strength.user_band = Some(band);
            strength.locked_by_user = Some(true);
            edge.tie_strength = Some(serde_json::to_value(strength).unwrap());
            store.put_relationship(edge.clone()).unwrap();
        }
        let original = store.list_inferences().unwrap().remove(0);
        let mut chosen = original.clone();
        chosen.inference_id = id(600);
        chosen.target["relationship_id"] = json!(first.relationship_id.to_string());
        chosen.user_verdict = Some(UserVerdict::Corrected);
        let mut later = chosen.clone();
        later.inference_id = id(601);
        later.user_verdict = Some(UserVerdict::Rejected);
        // Insert in the opposite order: first means the store's list order.
        store.put_inference(later.clone()).unwrap();
        store.put_inference(chosen.clone()).unwrap();

        let built = soul_graph::rebuild(&mut store).expect("duplicate lookup rebuild");
        assert_eq!(built.edges_written, [first.relationship_id]);
        assert_eq!(built.inferences_written, [chosen.inference_id]);
        let rebuilt = store.get_relationship(first.relationship_id).unwrap();
        assert_eq!(
            (rebuilt.from_contact_id, rebuilt.to_contact_id),
            (id(1), id(2))
        );
        let strength: TieStrength = serde_json::from_value(rebuilt.tie_strength.unwrap()).unwrap();
        assert_eq!(strength.band, SupportedBand::Moderate);
        assert_eq!(strength.user_band, Some(SupportedBand::Moderate));
        assert!(strength.is_locked_by_user());
        let inference = store.get_inference(chosen.inference_id).unwrap();
        assert_eq!(inference.user_verdict, Some(UserVerdict::Corrected));
        assert_eq!(strength.machine_band, Some(inference.evidence_band));
        assert_eq!(
            store.get_relationship(second.relationship_id).unwrap(),
            second
        );
        assert_eq!(store.get_inference(later.inference_id).unwrap(), later);
    }
}

#[test]
fn inference_target_matching_is_exact_string_matching_with_the_tie_prefix() {
    let mut store = seeded();
    let mut edge = store.list_relationships().unwrap().remove(0);
    edge.relationship_id = id(0xabcd);
    store.put_relationship(edge.clone()).unwrap();
    let prototype = store.list_inferences().unwrap().remove(0);
    let exact = edge.relationship_id.to_string();
    let candidates = [
        (json!(exact.to_uppercase()), "graph.tie.weak"),
        (
            json!(edge.relationship_id.simple().to_string()),
            "graph.tie.weak",
        ),
        (json!(123), "graph.tie.weak"),
        (json!(null), "graph.tie.weak"),
        (json!(exact), "other.graph.tie.weak"),
    ];
    let mut untouched = Vec::new();
    for (n, (target, statement)) in candidates.into_iter().enumerate() {
        let mut inference = prototype.clone();
        inference.inference_id = id(700 + n as u128);
        inference.target["relationship_id"] = target;
        inference.statement_key = statement.to_owned();
        inference.user_verdict = Some(UserVerdict::Accepted);
        store.put_inference(inference.clone()).unwrap();
        untouched.push(inference);
    }
    let mut chosen = prototype;
    chosen.inference_id = id(800);
    chosen.target["relationship_id"] = json!(exact);
    // The predicate recognizes the prefix, not a closed set of suffixes.
    chosen.statement_key = "graph.tie.legacy-suffix".to_owned();
    chosen.user_verdict = Some(UserVerdict::Rejected);
    store.put_inference(chosen.clone()).unwrap();

    let built = soul_graph::rebuild(&mut store).unwrap();
    assert_eq!(built.edges_written, [edge.relationship_id]);
    assert_eq!(built.inferences_written, [chosen.inference_id]);
    assert_eq!(
        store
            .get_inference(chosen.inference_id)
            .unwrap()
            .user_verdict,
        Some(UserVerdict::Rejected)
    );
    for inference in untouched {
        assert_eq!(
            store.get_inference(inference.inference_id).unwrap(),
            inference
        );
    }
}

#[test]
fn unrelated_or_later_duplicate_unreadable_strengths_are_not_read() {
    let mut store = seeded();
    let selected = store.list_relationships().unwrap().remove(0);
    for (relationship_id, from, to) in [
        (id(10), id(1), id(3)),
        (id(11), id(2), id(3)),
        (id(12), id(1), id(1)),
        (
            "ffffffff-ffff-7fff-bfff-ffffffffffff".parse().unwrap(),
            id(2),
            id(1),
        ),
    ] {
        let mut unrelated = selected.clone();
        unrelated.relationship_id = relationship_id;
        unrelated.from_contact_id = from;
        unrelated.to_contact_id = to;
        // Legacy schema admits {}, but TieStrength cannot deserialize it.
        unrelated.tie_strength = Some(json!({}));
        store.put_relationship(unrelated).unwrap();
    }
    let built = soul_graph::rebuild(&mut store).expect("only selected strength is read");
    assert_eq!(built.edges_written, [selected.relationship_id]);
}

#[test]
fn the_selected_unreadable_strength_still_refuses_the_rebuild() {
    let mut store = seeded();
    let mut edge = store.list_relationships().unwrap().remove(0);
    edge.tie_strength = Some(json!({}));
    store.put_relationship(edge.clone()).unwrap();
    assert!(matches!(
        soul_graph::rebuild(&mut store),
        Err(GraphError::UnreadableEdge { relationship_id, field: "tie_strength" })
            if relationship_id == edge.relationship_id
    ));
    assert_eq!(store.get_relationship(edge.relationship_id).unwrap(), edge);
}
