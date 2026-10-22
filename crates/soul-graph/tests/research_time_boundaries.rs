//! Synthetic time experiments through SQLCipher and the real graph builder.
//! Explicit graph tags with missing or invalid required metadata are refused.

use soul_graph::interaction::{conversation_ref, interaction_evidence, InteractionRef};
use soul_graph::{Direction, GraphError, Venue};
use soul_schema::common::{SchemaVersion, Subject, Timestamp};
use soul_schema::contact::{ContactClass, SoulContact};
use soul_schema::memory::ForgetState;
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::{GraphStore, ProfileStore};
use uuid::Uuid;

const UTC: [&str; 3] = [
    "2024-02-29T23:30:00Z",
    "2024-03-01T00:00:00Z",
    "2024-03-01T00:15:00Z",
];
const OFFSET: [&str; 3] = [
    "2024-03-01T00:30:00+01:00",
    "2024-03-01T00:00:00Z",
    "2024-03-01T00:15:00Z",
];

fn id(n: u128) -> Uuid {
    Uuid::from_u128(0x0192_a1b2_c3d4_7e5f_8a9b_0c1d_2e3f_4000 + n)
}

fn setup() -> (tempfile::TempDir, SqlCipherStore) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = SqlCipherStore::open(
        dir.path().join("time.db"),
        &TestKeyProvider::from_seed("synthetic research time"),
    )
    .unwrap();
    for (n, class) in [
        (1, ContactClass::Owner),
        (2, ContactClass::ThirdParty),
        (3, ContactClass::ThirdParty),
    ] {
        store
            .put_contact(SoulContact {
                schema_version: SchemaVersion,
                contact_id: id(n),
                contact_class: class,
                display_label_ref: None,
                identifiers: None,
                forget_state: ForgetState::Active,
            })
            .unwrap();
    }
    (dir, store)
}

fn evidence(n: usize, peer: u128, time: &str) -> soul_schema::evidence::SoulEvidence {
    interaction_evidence(
        id(100 + n as u128),
        Subject::ThirdParty,
        &InteractionRef::new(
            id(200 + n as u128),
            id(1),
            id(peer),
            conversation_ref("research", "synthetic"),
            if n == 0 {
                Direction::Outgoing
            } else {
                Direction::Incoming
            },
            Timestamp::new(time),
            Venue::Direct,
        ),
    )
}

fn rebuilt(times: [&str; 3], reverse: bool) -> soul_graph::SoulGraph {
    let (_dir, mut store) = setup();
    // Reverse references inside one evidence row as well as insert order, so
    // SQL's stable evidence ordering cannot make this a vacuous permutation.
    let mut combined = evidence(0, 2, times[0]);
    combined
        .source_refs
        .extend((1..3).map(|n| evidence(n, 2, times[n]).source_refs.remove(0)));
    if reverse {
        combined.source_refs.reverse();
    }
    let anchor = evidence(3, 3, "2024-03-01T12:15:00Z");
    let rows = if reverse {
        vec![anchor, combined]
    } else {
        vec![combined, anchor]
    };
    for row in rows {
        store.put_evidence(row).unwrap();
    }
    assert_eq!(
        soul_graph::rebuild(&mut store).unwrap().interactions_read,
        4
    );
    let graph = soul_graph::load(&store).unwrap();
    for edge in &graph.edges {
        assert_eq!(
            edge.tie_strength.as_of_utc.as_ref().unwrap().as_str(),
            "2024-03-01T12:15:00Z"
        );
        let resolved = soul_graph::resolve_evidence(&store, edge).unwrap();
        assert_eq!(resolved.len(), edge.evidence_ids.len());
        assert!(resolved
            .iter()
            .all(|row| edge.evidence_ids.contains(&row.evidence_id)));
    }
    graph
}

#[test]
fn utc_cross_day_counts_and_display_are_invariant_under_observation_order() {
    let forward = rebuilt(UTC, false);
    let reverse = rebuilt(UTC, true);
    for graph in [&forward, &reverse] {
        let edge = graph
            .edges
            .iter()
            .find(|edge| edge.tie_strength.interaction_count == 3)
            .unwrap();
        assert_eq!(edge.tie_strength.direct_active_day_count, 2);
        assert_eq!(edge.tie_strength.first_contact_utc.as_str(), UTC[0]);
        assert_eq!(edge.tie_strength.last_contact_utc.as_str(), UTC[2]);
    }
    let left = forward
        .edges
        .iter()
        .find(|edge| edge.tie_strength.interaction_count == 3)
        .unwrap();
    let right = reverse
        .edges
        .iter()
        .find(|edge| edge.tie_strength.interaction_count == 3)
        .unwrap();
    assert_eq!(left.tie_strength, right.tie_strength);
    assert_eq!(left.evidence_ids, right.evidence_ids);
}

#[test]
fn mixed_offsets_keep_display_extrema_consistent_with_actual_instants() {
    for reverse in [false, true] {
        let canonical = rebuilt(UTC, reverse);
        let mixed = rebuilt(OFFSET, reverse);
        let baseline = canonical
            .edges
            .iter()
            .find(|edge| edge.tie_strength.interaction_count == 3)
            .unwrap();
        let edge = mixed
            .edges
            .iter()
            .find(|edge| edge.tie_strength.interaction_count == 3)
            .unwrap();
        assert_eq!(edge.tie_strength.band, baseline.tie_strength.band);
        assert_eq!(edge.tie_strength.direct_active_day_count, 2);
        assert_eq!(
            edge.tie_strength.silent_days,
            baseline.tie_strength.silent_days
        );
        // Compare instants rather than requiring normalization of stored text.
        for (actual, expected) in [
            (&edge.tie_strength.first_contact_utc, UTC[0]),
            (&edge.tie_strength.last_contact_utc, UTC[2]),
            (
                edge.tie_strength.last_direct_contact_utc.as_ref().unwrap(),
                UTC[2],
            ),
        ] {
            assert_eq!(
                soul_graph::t4d_adapt::rfc3339_to_unix(actual.as_str()).unwrap(),
                soul_graph::t4d_adapt::rfc3339_to_unix(expected).unwrap(),
                "display extrema must represent the scorer's actual chronology"
            );
        }
    }
}

#[test]
fn fractional_display_extrema_distinguish_zero_from_nonzero_within_one_second() {
    assert_fractional_extrema(
        [
            "2024-03-01T00:00:00Z",
            "2024-03-01T00:00:00.1Z",
            "2024-03-01T00:00:00.12Z",
        ],
        "2024-03-01T00:00:00Z",
        "2024-03-01T00:00:00.12Z",
    );
}

#[test]
fn fractional_display_extrema_compare_decimal_values_of_different_lengths() {
    assert_fractional_extrema(
        [
            "2024-03-01T00:00:00.9Z",
            "2024-03-01T00:00:00.91Z",
            "2024-03-01T00:00:00.899Z",
        ],
        "2024-03-01T00:00:00.899Z",
        "2024-03-01T00:00:00.91Z",
    );
}

fn assert_fractional_extrema(times: [&str; 3], first: &str, last: &str) {
    for reverse in [false, true] {
        let graph = rebuilt(times, reverse);
        let edge = graph
            .edges
            .iter()
            .find(|edge| edge.tie_strength.interaction_count == 3)
            .unwrap();
        let strength = &edge.tie_strength;
        // The display retains subsecond ordering; the scorer still sees three
        // observations at the same whole second and the existing anchor as_of.
        assert_eq!(strength.direct_active_day_count, 1);
        assert_eq!(strength.silent_days, 0);
        assert_eq!(strength.first_contact_utc.as_str(), first);
        assert_eq!(strength.last_contact_utc.as_str(), last);
        assert_eq!(
            strength.last_direct_contact_utc.as_ref().unwrap().as_str(),
            last
        );
    }
}

#[test]
fn equivalent_fractional_offset_expressions_select_stable_original_display_text() {
    let times = [
        "2024-03-01T00:00:00.1Z",
        "2024-03-01T00:00:00.10Z",
        "2024-03-01T01:00:00.100+01:00",
    ];
    let forward = rebuilt(times, false);
    let reverse = rebuilt(times, true);
    let strength = |graph: &soul_graph::SoulGraph| {
        graph
            .edges
            .iter()
            .find(|edge| edge.tie_strength.interaction_count == 3)
            .unwrap()
            .tie_strength
            .clone()
    };
    let left = strength(&forward);
    let right = strength(&reverse);
    assert_eq!(
        left, right,
        "equivalent instants must not depend on observation order"
    );
    assert_eq!(left.direct_active_day_count, 1);
    assert_eq!(left.silent_days, 0);
    for displayed in [
        left.first_contact_utc.as_str(),
        left.last_contact_utc.as_str(),
        left.last_direct_contact_utc.as_ref().unwrap().as_str(),
    ] {
        assert!(
            times.contains(&displayed),
            "retain an original timestamp expression"
        );
    }
}

#[test]
fn missing_and_invalid_present_time_both_refuse_rebuild() {
    for missing in [true, false] {
        let (_dir, mut store) = setup();
        for (n, time) in UTC.iter().enumerate() {
            let mut row = evidence(n, 2, time);
            if n == 0 {
                if missing {
                    row.source_refs[0]
                        .as_object_mut()
                        .unwrap()
                        .remove("occurred_at");
                } else {
                    row.source_refs[0]["occurred_at"] = "last tuesday".into();
                }
            }
            store.put_evidence(row).unwrap();
        }
        assert!(
            matches!(soul_graph::rebuild(&mut store), Err(GraphError::UnreadableInteraction { evidence_id }) if evidence_id == id(100))
        );
        assert!(store.list_relationships().unwrap().is_empty());
        assert!(store.list_inferences().unwrap().is_empty());
    }
}

#[test]
fn tagged_damage_refuses_without_rewriting_existing_graph() {
    for (field, value) in [
        ("occurred_at", None),
        ("occurred_at", Some(serde_json::Value::Null)),
        ("occurred_at", Some(serde_json::json!(123))),
        (
            "occurred_at",
            Some(serde_json::json!("SECRET_INVALID_TIME")),
        ),
        ("event_id", Some(serde_json::json!("SECRET_INVALID_UUID"))),
        ("venue", None),
        ("direction", Some(serde_json::json!("SECRET_DIRECTION"))),
    ] {
        let (_dir, mut store) = setup();
        store.put_evidence(evidence(0, 2, UTC[0])).unwrap();
        soul_graph::rebuild(&mut store).unwrap();
        let relationships = store.list_relationships().unwrap();
        let inferences = store.list_inferences().unwrap();
        let mut damaged = evidence(1, 2, UTC[1]);
        match value {
            Some(value) => {
                damaged.source_refs[0][field] = value;
            }
            None => {
                damaged.source_refs[0]
                    .as_object_mut()
                    .unwrap()
                    .remove(field);
            }
        }
        // A valid sibling must not hide an invalid tagged reference.
        damaged
            .source_refs
            .push(evidence(2, 2, UTC[2]).source_refs.remove(0));
        store.put_evidence(damaged).unwrap();
        let error = soul_graph::rebuild(&mut store).expect_err(field);
        assert!(
            matches!(error, GraphError::UnreadableInteraction { evidence_id } if evidence_id == id(101))
        );
        assert!(!error.to_string().contains("SECRET"));
        assert_eq!(store.list_relationships().unwrap(), relationships);
        assert_eq!(store.list_inferences().unwrap(), inferences);
    }
}

#[test]
fn no_owner_still_refuses_explicitly_tagged_damage_but_ignores_other_refs() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = SqlCipherStore::open(
        dir.path().join("empty.db"),
        &TestKeyProvider::from_seed("synthetic no owner"),
    )
    .unwrap();
    let mut row = evidence(0, 2, UTC[0]);
    row.source_refs = vec![serde_json::json!({"ref_kind":"other_module", "occurred_at":null})];
    store.put_evidence(row.clone()).unwrap();
    assert_eq!(
        soul_graph::rebuild(&mut store).unwrap().interactions_read,
        0
    );
    row.source_refs
        .push(evidence(0, 2, UTC[0]).source_refs.remove(0));
    store.put_evidence(row.clone()).unwrap();
    assert!(soul_graph::rebuild(&mut store)
        .unwrap()
        .edges_written
        .is_empty());
    row.source_refs[1]
        .as_object_mut()
        .unwrap()
        .remove("occurred_at");
    assert!(
        soul_graph::interaction::is_interaction(&row),
        "tag classification is not validation"
    );
    store.put_evidence(row).unwrap();
    assert!(
        matches!(soul_graph::rebuild(&mut store), Err(GraphError::UnreadableInteraction { evidence_id }) if evidence_id == id(100))
    );
}
