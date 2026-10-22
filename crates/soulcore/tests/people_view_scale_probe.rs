//! Bounded warm-read diagnostic for graph loading and complete people metadata.
//! No latency threshold, Session/IPC claim, SQL trace or real input is involved.

mod people_view_support;

use std::collections::BTreeSet;
use std::time::Instant;

use people_view_support::{expected_view, seed, Shape, SEED, SLOTS_PER_EDGE};
use serde_json::json;
use soul_store_api::{GraphStore, ProfileStore};
use soulcore::commands::{graph, store};

fn timed<T>(operation: impl FnOnce() -> T) -> (T, u64) {
    let start = Instant::now();
    let result = operation();
    (result, start.elapsed().as_nanos().try_into().unwrap())
}

#[test]
#[ignore = "explicit finite synthetic SQLCipher diagnostic; no performance gate"]
fn people_view_unique_and_shared_metadata_scale() {
    for peers in [100, 1000] {
        for shape in [Shape::Unique, Shape::Shared] {
            for trial in 1..=3 {
                let directory = tempfile::tempdir().unwrap();
                let mut database = store::open_test_store(directory.path(), SEED).unwrap();
                seed(&mut database, peers, shape);

                // All preparation, structural accounting, full reference
                // reads and expected-view construction are outside timers.
                let contacts = database.list_contacts().unwrap().len();
                let relationships = database.list_relationships().unwrap();
                let stored_evidence = database.list_evidence().unwrap().len();
                let slots: usize = relationships
                    .iter()
                    .map(|edge| edge.evidence_ids.len())
                    .sum();
                let distinct = relationships
                    .iter()
                    .flat_map(|edge| &edge.evidence_ids)
                    .collect::<BTreeSet<_>>()
                    .len();
                assert_eq!(contacts, peers + 1);
                assert_eq!(relationships.len(), peers);
                assert_eq!(slots, peers * SLOTS_PER_EDGE);
                assert_eq!(stored_evidence, slots);
                assert_eq!(distinct, shape.distinct(peers));
                let expected = expected_view(peers, shape);
                let reference_graph = graph::load(&database).unwrap();
                let reference_view = graph::people_view(&database).unwrap();
                assert_eq!(reference_view, expected);

                // Same order for every sample: warmed graph load, then one
                // full real people_view call (which itself loads the graph).
                // These timings are separate observations and are not added.
                let (loaded, graph_load_ns) = timed(|| graph::load(&database));
                let (view, people_view_ns) = timed(|| graph::people_view(&database));
                let loaded = loaded.unwrap();
                let view = view.unwrap();
                assert_eq!(loaded, reference_graph);
                assert_eq!(view, reference_view);
                assert_eq!(view, expected);
                assert_eq!(graph::load(&database).unwrap(), reference_graph);
                assert_eq!(
                    view.ties
                        .iter()
                        .map(|tie| tie.evidence.len())
                        .sum::<usize>(),
                    slots
                );

                // K/U describe citation structure, not measured SQL counts.
                println!(
                    "SOUL_PEOPLE_VIEW_SCALE {}",
                    json!({
                        "record": "people_view_scale", "shape": shape.name(), "trial": trial,
                        "peers": peers, "contacts_c": contacts, "edges_e": relationships.len(),
                        "evidence_slots_k": slots, "distinct_evidence_u": distinct,
                        "duplicate_slots": slots - distinct, "stored_evidence": stored_evidence,
                        "graph_load_ns": graph_load_ns, "people_view_ns": people_view_ns,
                        "timed_surface": "graph_commands::load_then_people_view",
                        "warm_reference_reads": true, "session_lock_included": false,
                        "formal_q2": false
                    })
                );
            }
        }
    }
}
