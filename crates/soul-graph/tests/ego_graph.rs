//! AC-06 and AC-08 on the graph side, against the real store.
//!
//! The evidence here is written by hand rather than imported, so that what is
//! being tested is the derivation and not the parser. `soul-import`'s own
//! end-to-end test walks the same ground starting from a fixture file.

use uuid::Uuid;

use soul_graph::interaction::{conversation_ref, interaction_evidence, InteractionRef};
use soul_graph::{Direction, Venue};
use soul_schema::common::{SchemaVersion, Subject, SupportedBand, Timestamp};
use soul_schema::contact::{ContactClass, SoulContact};
use soul_schema::memory::ForgetState;
use soul_schema::relationship::EgressScope;
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::{GraphStore, ProfileStore};

const SEED: &str = "wp05 ego graph";

/// A UUIDv7-shaped literal, so a failure names the same person every run.
fn id(tail: &str) -> Uuid {
    format!("0192a1b2-c3d4-7e5f-8a9b-0c1d2e3f4{tail:0>3}")
        .parse()
        .expect("hand-written UUIDv7 literal")
}

fn owner() -> Uuid {
    id("001")
}

fn open(dir: &std::path::Path) -> SqlCipherStore {
    SqlCipherStore::open(dir.join("soul.db"), &TestKeyProvider::from_seed(SEED)).expect("open")
}

fn contact(contact_id: Uuid, class: ContactClass) -> SoulContact {
    SoulContact {
        schema_version: SchemaVersion,
        contact_id,
        contact_class: class,
        display_label_ref: None,
        identifiers: None,
        forget_state: ForgetState::Active,
    }
}

/// One observation, written the way an importer writes it.
fn observe(
    store: &mut SqlCipherStore,
    evidence_id: Uuid,
    peer: Uuid,
    conversation: &str,
    direction: Direction,
    at: &str,
    venue: Venue,
) {
    let observation = InteractionRef::new(
        Uuid::now_v7(),
        owner(),
        peer,
        conversation_ref("test", conversation),
        direction,
        Timestamp::new(at),
        venue,
    );
    let subject = match direction {
        Direction::Outgoing => Subject::Mixed,
        Direction::Incoming => Subject::ThirdParty,
    };
    store
        .put_evidence(interaction_evidence(evidence_id, subject, &observation))
        .expect("evidence");
}

/// Three conversation partners: one talked to often over several days, one
/// seen a handful of times, one only ever in a group.
fn seed_three_partners(store: &mut SqlCipherStore) -> (Uuid, Uuid, Uuid) {
    let (lilei, wangxiao, zhaoqi) = (id("002"), id("003"), id("004"));
    store
        .put_contact(contact(owner(), ContactClass::Owner))
        .expect("owner");
    for peer in [lilei, wangxiao, zhaoqi] {
        store
            .put_contact(contact(peer, ContactClass::ThirdParty))
            .expect("peer");
    }

    let mut next = 100u32;
    let mut evidence_id = || {
        next += 1;
        id(&next.to_string())
    };

    for day in 18..24 {
        observe(
            store,
            evidence_id(),
            lilei,
            "c-01",
            Direction::Outgoing,
            &format!("2026-08-{day}T09:00:00Z"),
            Venue::Direct,
        );
        observe(
            store,
            evidence_id(),
            lilei,
            "c-01",
            Direction::Incoming,
            &format!("2026-08-{day}T09:05:00Z"),
            Venue::Direct,
        );
    }

    observe(
        store,
        evidence_id(),
        wangxiao,
        "c-02",
        Direction::Outgoing,
        "2026-08-20T19:30:00Z",
        Venue::Direct,
    );
    observe(
        store,
        evidence_id(),
        wangxiao,
        "c-02",
        Direction::Incoming,
        "2026-08-20T19:34:00Z",
        Venue::Direct,
    );
    observe(
        store,
        evidence_id(),
        wangxiao,
        "c-02",
        Direction::Incoming,
        "2026-08-21T10:00:00Z",
        Venue::Direct,
    );

    observe(
        store,
        evidence_id(),
        zhaoqi,
        "c-05",
        Direction::Outgoing,
        "2026-08-23T20:11:00Z",
        Venue::Group,
    );
    observe(
        store,
        evidence_id(),
        zhaoqi,
        "c-05",
        Direction::Incoming,
        "2026-08-23T20:15:00Z",
        Venue::Group,
    );

    (lilei, wangxiao, zhaoqi)
}

/// AC-08: three conversation partners, three nodes, and every edge backed by
/// evidence that resolves.
#[test]
fn three_conversation_partners_produce_three_evidence_backed_edges() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path());
    let (lilei, wangxiao, zhaoqi) = seed_three_partners(&mut store);

    let build = soul_graph::rebuild(&mut store).expect("rebuild");
    assert_eq!(build.edges_written.len(), 3);
    assert_eq!(build.peers_unresolved, 0);

    let graph = soul_graph::load(&store).expect("load");
    assert_eq!(graph.self_contact_id, Some(owner()));
    assert!(
        graph.third_party_nodes().len() >= 3,
        "three partners must be three nodes, found {}",
        graph.third_party_nodes().len(),
    );

    for peer in [lilei, wangxiao, zhaoqi] {
        let edges = graph.edges_for(peer);
        assert_eq!(edges.len(), 1, "one edge per partner");
        let edge = edges[0];
        assert!(
            !edge.evidence_ids.is_empty(),
            "edge to {peer} cites no evidence",
        );
        let resolved = soul_graph::resolve_evidence(&store, edge).expect("resolve");
        assert_eq!(resolved.len(), edge.evidence_ids.len());
        assert_eq!(edge.other_end(peer), Some(owner()));
    }
}

/// The four things PRODUCT_LOCK asks an edge to carry, read back off the store
/// rather than off the value the build returned.
#[test]
fn an_edge_carries_strength_type_last_contact_and_evidence() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path());
    let (lilei, _, zhaoqi) = seed_three_partners(&mut store);
    soul_graph::rebuild(&mut store).expect("rebuild");
    let graph = soul_graph::load(&store).expect("load");

    let close = graph.edges_for(lilei)[0];
    assert_eq!(close.tie_strength.band, SupportedBand::Strong);
    assert_eq!(close.tie_strength.interaction_count, 12);
    assert_eq!(close.tie_strength.outgoing_count, 6);
    assert_eq!(close.tie_strength.incoming_count, 6);
    assert_eq!(close.tie_strength.active_day_count, 6);
    assert_eq!(
        close.tie_strength.last_contact_utc.as_str(),
        "2026-08-23T09:05:00Z",
    );
    assert!(close.types.contains(&soul_graph::TieType::Direct));
    assert!(close.types.contains(&soul_graph::TieType::Reciprocal));

    // Only ever seen in a group, so the tie says so rather than pretending the
    // two of them have spoken one to one.
    let distant = graph.edges_for(zhaoqi)[0];
    assert!(distant.types.contains(&soul_graph::TieType::GroupOnly));
    assert_eq!(distant.tie_strength.band, SupportedBand::Weak);
}

/// AC-06 on the graph side: the inference behind an edge names evidence, and
/// every id it names resolves to a row.
#[test]
fn a_tie_inference_dereferences_to_the_observations_behind_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path());
    seed_three_partners(&mut store);
    let build = soul_graph::rebuild(&mut store).expect("rebuild");

    assert_eq!(build.inferences_written.len(), 3);
    for inference_id in &build.inferences_written {
        let inference = store.get_inference(*inference_id).expect("inference");
        assert!(
            !inference.evidence_ids.is_empty(),
            "an inference with no evidence must never reach the store",
        );
        for evidence_id in &inference.evidence_ids {
            store.get_evidence(*evidence_id).expect("evidence resolves");
        }
        assert!(inference.statement_key.starts_with("graph.tie."));
        assert!(
            inference.falsifier.is_some(),
            "an inference has to say what would overturn it",
        );

        let relationship_id: Uuid = inference.target["relationship_id"]
            .as_str()
            .expect("the target names its edge")
            .parse()
            .expect("uuid");
        let edge = store
            .get_relationship(relationship_id)
            .expect("the edge resolves");
        assert_eq!(edge.evidence_ids, inference.evidence_ids);
    }
}

/// Third-party data does not leave the machine, and the graph says so on every
/// row rather than leaving it to the caller.
#[test]
fn every_third_party_node_and_edge_is_local_only() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path());
    seed_three_partners(&mut store);
    soul_graph::rebuild(&mut store).expect("rebuild");

    let graph = soul_graph::load(&store).expect("load");
    assert!(graph.third_party_data_is_local_only());
    for stored in store.list_relationships().expect("edges") {
        assert_eq!(stored.egress_scope, Some(EgressScope::LocalOnly));
    }
    for evidence in store.list_evidence().expect("evidence") {
        assert_eq!(
            evidence.exportable_to_research,
            Some(false),
            "an interaction is somebody else's data and never goes to research",
        );
    }
}

/// Importing a second overlapping export must update the ties, not grow a
/// second graph on top of the first.
#[test]
fn rebuilding_updates_the_same_edges_instead_of_duplicating_them() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path());
    let (lilei, _, _) = seed_three_partners(&mut store);

    let first = soul_graph::rebuild(&mut store).expect("first");
    observe(
        &mut store,
        id("900"),
        lilei,
        "c-01",
        Direction::Incoming,
        "2026-08-25T11:00:00Z",
        Venue::Direct,
    );
    let second = soul_graph::rebuild(&mut store).expect("second");

    assert_eq!(first.edges_written, second.edges_written);
    assert_eq!(first.inferences_written, second.inferences_written);
    assert_eq!(store.list_relationships().expect("edges").len(), 3);
    assert_eq!(store.list_inferences().expect("inferences").len(), 3);

    let graph = soul_graph::load(&store).expect("load");
    let edge = graph.edges_for(lilei)[0];
    assert_eq!(edge.tie_strength.interaction_count, 13);
    assert_eq!(
        edge.tie_strength.last_contact_utc.as_str(),
        "2026-08-25T11:00:00Z",
        "the new observation is the most recent contact",
    );
}

/// Evidence about somebody whose contact row has gone is reported, not
/// silently folded into the graph as an edge to nobody.
#[test]
fn evidence_naming_a_contact_that_is_gone_is_counted_rather_than_dropped() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path());
    seed_three_partners(&mut store);
    observe(
        &mut store,
        id("901"),
        id("777"),
        "c-09",
        Direction::Incoming,
        "2026-08-24T08:00:00Z",
        Venue::Direct,
    );

    let build = soul_graph::rebuild(&mut store).expect("rebuild");
    assert_eq!(build.peers_unresolved, 1);
    assert_eq!(build.edges_written.len(), 3, "no edge to a missing person");
}

/// Nothing has been imported, so there is nobody to be at the centre.
#[test]
fn an_empty_store_produces_an_empty_graph_rather_than_an_error() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path());

    let build = soul_graph::rebuild(&mut store).expect("rebuild");
    assert!(build.edges_written.is_empty());
    assert!(build.audit.is_empty());

    let graph = soul_graph::load(&store).expect("load");
    assert_eq!(graph.self_contact_id, None);
    assert!(graph.nodes.is_empty());
}
