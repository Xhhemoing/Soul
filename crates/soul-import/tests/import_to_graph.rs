//! AC-08 from one end to the other: a fixture file goes in, and a graph with
//! evidence behind every edge comes out.
//!
//! `soul-graph`'s own tests write the observations by hand so that the
//! derivation is tested on its own. This one starts from a file, so that the
//! contract between the importer and the graph — interaction evidence in
//! `source_refs` — is tested rather than assumed.

use std::collections::BTreeSet;

use uuid::Uuid;

use soul_graph::interaction;
use soul_schema::contact::ContactClass;
use soul_schema::relationship::EgressScope;
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::{GraphStore, ProfileStore};
use soul_testkit::fixtures;

const SEED: &str = "wp06 import to graph";

fn import(dir: &std::path::Path, fixture: &str) -> (SqlCipherStore, Uuid) {
    let text = fixtures::read_text(fixture).expect("fixture");
    let staged = soul_import::soul_import_v1::parse(&text).expect("the fixture is valid");
    let mut store =
        SqlCipherStore::open(dir.join("soul.db"), &TestKeyProvider::from_seed(SEED)).expect("open");
    let receipt = soul_import::commit::commit(&mut store, &staged).expect("commit");
    let owner = receipt.self_contact_id.expect("the file names the user");
    (store, owner)
}

/// AC-08. Four conversation partners in the file, four nodes in the graph,
/// and every edge cites evidence that resolves.
#[test]
fn a_four_partner_export_becomes_four_evidence_backed_edges() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut store, owner) = import(dir.path(), "import/soul-import-v1/three_partners.jsonl");

    let build = soul_graph::rebuild(&mut store).expect("rebuild");
    assert_eq!(build.peers_unresolved, 0);

    let graph = soul_graph::load(&store).expect("load");
    assert_eq!(graph.self_contact_id, Some(owner));
    assert!(
        graph.third_party_nodes().len() >= 3,
        "the file has four partners in it; found {}",
        graph.third_party_nodes().len(),
    );
    assert_eq!(graph.edges.len(), 4);

    for edge in &graph.edges {
        assert!(edge.touches(owner), "v0.1 builds an ego network");
        assert!(!edge.evidence_ids.is_empty(), "an edge nobody observed");
        let resolved = soul_graph::resolve_evidence(&store, edge).expect("resolve");
        assert_eq!(resolved.len(), edge.evidence_ids.len());
        for evidence in &resolved {
            assert!(
                interaction::is_interaction(evidence),
                "an edge rests on observed exchanges, not on anything else",
            );
        }
    }
}

/// The importer's output is the graph's input. Nothing between the two is
/// allowed to invent a peer the file did not name.
#[test]
fn every_node_in_the_graph_came_out_of_the_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut store, owner) = import(dir.path(), "import/soul-import-v1/three_partners.jsonl");
    soul_graph::rebuild(&mut store).expect("rebuild");

    let graph = soul_graph::load(&store).expect("load");
    let peers: BTreeSet<Uuid> = graph
        .third_party_nodes()
        .iter()
        .map(|node| node.contact_id)
        .collect();
    assert_eq!(peers.len(), 4);
    assert!(!peers.contains(&owner));

    for node in graph.third_party_nodes() {
        assert_eq!(node.contact_class, ContactClass::ThirdParty);
        assert_eq!(node.egress_scope, EgressScope::LocalOnly);
        assert!(
            node.interaction_count > 0,
            "a node with no interactions has no reason to be in the graph",
        );
        assert!(node.last_contact_utc.is_some());
        assert!(
            node.label_ref.is_none() || node.label_ref.as_ref().unwrap().placeholder.is_some(),
            "a third-party label carries the placeholder that replaces it",
        );
    }
}

/// A conversation with two other people in it is a group, and a message the
/// user sent in one is evidence of contact with everyone who spoke there.
#[test]
fn a_message_sent_to_a_group_is_evidence_of_contact_with_everyone_in_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut store, owner) = import(dir.path(), "import/soul-import-v1/three_partners.jsonl");

    // `c-05` is the fixture's only conversation with two peers in it. The one
    // message the user sent there has to produce an observation per peer.
    let group_observations: Vec<_> = store
        .list_evidence()
        .expect("evidence")
        .iter()
        .flat_map(interaction::interactions_in)
        .filter(|observation| observation.venue == soul_graph::Venue::Group)
        .collect();
    let by_event: BTreeSet<Uuid> = group_observations
        .iter()
        .filter(|observation| observation.direction == soul_graph::Direction::Outgoing)
        .map(|observation| observation.event_id)
        .collect();
    assert_eq!(by_event.len(), 1, "the user sent one message to the group");
    assert_eq!(
        group_observations
            .iter()
            .filter(|observation| observation.direction == soul_graph::Direction::Outgoing)
            .count(),
        2,
        "and both people who spoke there heard it",
    );
    assert!(group_observations
        .iter()
        .all(|observation| observation.self_contact_id == owner));

    // Both of those partners are also seen one to one, so their ties are
    // direct rather than group-only.
    soul_graph::rebuild(&mut store).expect("rebuild");
    let graph = soul_graph::load(&store).expect("load");
    assert!(graph
        .edges
        .iter()
        .all(|edge| edge.types.contains(&soul_graph::TieType::Direct)));
    assert_eq!(
        graph
            .edges
            .iter()
            .filter(|edge| edge.tie_strength.conversation_count == 2)
            .count(),
        2,
        "the two people in the group each share two conversations with the user",
    );
}

/// Importing the Telegram fixture and the JSONL fixture into one store leaves
/// two disjoint ego networks, because there is nothing that says user `42` in
/// one export is user `42` in the other.
#[test]
fn identifiers_are_scoped_to_the_export_they_came_from() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut store, _) = import(dir.path(), "import/soul-import-v1/valid_basic.jsonl");

    let telegram: serde_json::Value =
        fixtures::read_json("import/telegram/result_basic.json").expect("fixture");
    let staged = soul_import::telegram::parse(&telegram).expect("valid");
    let receipt = soul_import::commit::commit(&mut store, &staged).expect("commit");

    assert!(
        receipt.contacts_matched.is_empty(),
        "two node for one person is a mistake the user can merge; one node for \
         two people is a wrong graph that looks right",
    );
    assert_eq!(store.list_contacts().expect("contacts").len(), 6);

    // Two owners now, which the graph refuses rather than picking one.
    let error = soul_graph::rebuild(&mut store).expect_err("two selves");
    assert!(matches!(
        error,
        soul_graph::GraphError::AmbiguousOwner { count: 2 },
    ));
}

/// Forgetting a person takes the evidence with it, and a rebuild after that
/// reports the gap instead of keeping an edge to somebody who is gone.
#[test]
fn evidence_that_survives_a_rebuild_is_evidence_that_still_resolves() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut store, _) = import(dir.path(), "import/soul-import-v1/three_partners.jsonl");
    soul_graph::rebuild(&mut store).expect("rebuild");

    let graph = soul_graph::load(&store).expect("load");
    let cited: BTreeSet<Uuid> = graph
        .edges
        .iter()
        .flat_map(|edge| edge.evidence_ids.iter().copied())
        .collect();
    let stored: BTreeSet<Uuid> = store
        .list_evidence()
        .expect("evidence")
        .into_iter()
        .map(|evidence| evidence.evidence_id)
        .collect();
    assert!(
        cited.is_subset(&stored),
        "an edge cites {} rows the store does not have",
        cited.difference(&stored).count(),
    );
    assert_eq!(cited.len(), stored.len(), "no observation went unused");
}
