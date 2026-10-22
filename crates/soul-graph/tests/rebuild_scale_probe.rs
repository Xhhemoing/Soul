//! Explicit synthetic, in-memory diagnosis of the real graph rebuild path.
//! Includes FakeStore cloning, schema validation and writes; excludes fixture
//! preparation, assertions and reporting. This is not SQLCipher performance.

use std::time::Instant;

use soul_graph::interaction::{conversation_ref, interaction_evidence, InteractionRef};
use soul_graph::{Direction, Venue};
use soul_schema::common::{SchemaVersion, Subject, SupportedBand, Timestamp};
use soul_schema::contact::{ContactClass, SoulContact};
use soul_schema::memory::ForgetState;
use soul_store_api::{FakeStore, GraphStore, ProfileStore};
use uuid::Uuid;

fn id(n: u128) -> Uuid {
    Uuid::from_u128(0x0192_a1b2_c3d4_7e5f_8a9b_0c1d_0000_0000 + n)
}

fn seed(peers: usize) -> FakeStore {
    let mut store = FakeStore::new();
    for n in 0..=peers {
        store
            .put_contact(SoulContact {
                schema_version: SchemaVersion,
                contact_id: id(n as u128 + 1),
                contact_class: if n == 0 {
                    ContactClass::Owner
                } else {
                    ContactClass::ThirdParty
                },
                display_label_ref: None,
                identifiers: None,
                forget_state: ForgetState::Active,
            })
            .expect("synthetic contact");
    }
    for peer in 0..peers {
        let observations = [Direction::Outgoing, Direction::Incoming].map(|direction| {
            let ordinal = match direction {
                Direction::Outgoing => 0,
                Direction::Incoming => 1,
            };
            interaction_evidence(
                id(100_000 + peer as u128),
                Subject::Mixed,
                &InteractionRef::new(
                    id(200_000 + 2 * peer as u128 + ordinal),
                    id(1),
                    id(peer as u128 + 2),
                    conversation_ref("scale-probe", &format!("synthetic-{peer}")),
                    direction,
                    Timestamp::new("2026-08-23T09:00:00Z"),
                    Venue::Direct,
                ),
            )
        });
        let [mut combined, mut incoming] = observations;
        combined.source_refs.append(&mut incoming.source_refs);
        store
            .put_evidence(combined)
            .expect("synthetic observations");
    }
    store
}

#[test]
#[ignore = "explicit synthetic in-memory graph rebuild measurement; no timing threshold"]
fn rebuild_existing_graph_at_100_1000_and_10000_peers() {
    for peers in [100, 1_000, 10_000] {
        for repetition in 1..=3 {
            let mut store = seed(peers);
            let prepared = soul_graph::rebuild(&mut store).expect("prepare old graph rows");
            assert_eq!(prepared.edges_written.len(), peers);
            assert_eq!(prepared.inferences_written.len(), peers);
            // Exercise a real user lock and verdict in the full-row comparison.
            soul_graph::correct_tie(
                &mut store,
                prepared.edges_written[0],
                SupportedBand::Moderate,
                1_787_529_600,
            )
            .expect("synthetic correction");
            let settled = soul_graph::rebuild(&mut store).expect("settle corrected graph");
            let before_edges = store.list_relationships().expect("old relationships");
            let before_inferences = store.list_inferences().expect("old inferences");
            let input_evidence = store.list_evidence().expect("input evidence");
            assert_eq!(before_edges.len(), peers);
            assert_eq!(before_inferences.len(), peers);
            assert_eq!(settled.interactions_read, (2 * peers) as u64);

            let start = Instant::now();
            let rebuilt = soul_graph::rebuild(&mut store).expect("measured rebuild");
            let elapsed_ns = start.elapsed().as_nanos();

            // Whole models compare IDs, user/machine fields, verdict, evidence,
            // display instants and as_of, rather than merely comparing counts.
            assert_eq!(store.list_relationships().unwrap(), before_edges);
            assert_eq!(store.list_inferences().unwrap(), before_inferences);
            assert_eq!(rebuilt.edges_written, settled.edges_written);
            assert_eq!(rebuilt.inferences_written, settled.inferences_written);
            assert_eq!(rebuilt.interactions_read, (2 * peers) as u64);
            assert_eq!(rebuilt.peers_unresolved, 0);
            println!(
                "graph_rebuild_memory peers={peers} repetition={repetition} \
                 evidence={} observations={} old_relationships={} old_inferences={} \
                 rebuilt_relationships={} rebuilt_inferences={} elapsed_ns={elapsed_ns}",
                input_evidence.len(),
                2 * peers,
                before_edges.len(),
                before_inferences.len(),
                rebuilt.edges_written.len(),
                rebuilt.inferences_written.len(),
            );
        }
    }
}
