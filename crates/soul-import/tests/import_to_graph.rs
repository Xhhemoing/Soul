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

/// A conversation with two other people in it is a group. What the user says
/// there is observed as an event and attributed to nobody: the export records
/// that the line was typed, not who read it.
#[test]
fn what_the_user_says_to_a_group_is_attributed_to_nobody() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut store, owner) = import(dir.path(), "import/soul-import-v1/three_partners.jsonl");

    // `c-05` is the fixture's only conversation with two peers in it. Both of
    // them spoke; the user's own message there produces no observation.
    let group_observations: Vec<_> = store
        .list_evidence()
        .expect("evidence")
        .iter()
        .flat_map(interaction::interactions_in)
        .filter(|observation| observation.venue == soul_graph::Venue::Group)
        .collect();
    assert!(
        group_observations
            .iter()
            .all(|observation| observation.direction == soul_graph::Direction::Incoming),
        "the user's side of a group is not credited to anybody",
    );
    assert_eq!(
        group_observations.len(),
        2,
        "the two people who spoke in the group each said one thing",
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

/// One line the user typed into a busy group is one line.
///
/// The importer used to write an outgoing observation per person who had ever
/// spoken in the conversation, so a single message in a group of six became
/// six outgoing rows — six people the user apparently reached out to, out of
/// one keystroke. Nothing in the export says who read it, so nothing is
/// credited. What the peers said still is: each of their messages names its
/// own sender, and produces exactly one incoming row for that person.
#[test]
fn an_owner_group_message_does_not_write_one_outgoing_row_per_speaker() {
    const SPEAKERS: usize = 6;

    let mut lines = vec![
        r#"{"type":"header","format":"soul-import-v1","version":1,"exported_at":"2026-08-24T08:00:00Z"}"#
            .to_owned(),
    ];
    for speaker in 1..=SPEAKERS {
        lines.push(format!(
            r#"{{"type":"message","id":"g-{speaker:04}","occurred_at":"2026-08-20T09:{speaker:02}:00Z","sender_scope":"third_party","conversation_id":"g-01","sender_id":"u-p{speaker}","text":"收到"}}"#,
        ));
    }
    lines.push(
        r#"{"type":"message","id":"g-0100","occurred_at":"2026-08-20T10:00:00Z","sender_scope":"self","conversation_id":"g-01","sender_id":"u-self","text":"那就按这个来"}"#
            .to_owned(),
    );

    let staged = soul_import::soul_import_v1::parse(&lines.join("\n")).expect("valid");
    assert!(
        staged.messages.iter().all(|message| message.group),
        "one conversation, {SPEAKERS} peers in it, so every line is a group line",
    );

    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = SqlCipherStore::open(
        dir.path().join("soul.db"),
        &TestKeyProvider::from_seed(SEED),
    )
    .expect("open");
    let receipt = soul_import::commit::commit(&mut store, &staged).expect("commit");
    let owner = receipt.self_contact_id.expect("the file names the user");

    let observations: Vec<_> = store
        .list_evidence()
        .expect("evidence")
        .iter()
        .flat_map(interaction::interactions_in)
        .collect();
    assert_eq!(
        observations
            .iter()
            .filter(|observation| observation.direction == soul_graph::Direction::Outgoing)
            .count(),
        0,
        "one message the user sent, {SPEAKERS} people who had spoken there, and no \
         outgoing row: the export does not say who was listening",
    );
    assert_eq!(
        receipt.evidence_written.len(),
        SPEAKERS,
        "the seven messages produce one row each for the six that name a peer",
    );
    assert_eq!(
        receipt.events_written.len(),
        SPEAKERS + 1,
        "every message is still an event, including the user's own",
    );

    // Each peer is heard from once, in their own right.
    for speaker in 1..=SPEAKERS {
        let peer = contact_for(&store, &format!("u-p{speaker}"));
        let heard: Vec<_> = observations
            .iter()
            .filter(|observation| observation.peer_contact_id == peer)
            .collect();
        assert_eq!(heard.len(), 1, "peer u-p{speaker} sent one message");
        assert_eq!(heard[0].direction, soul_graph::Direction::Incoming);
        assert_eq!(heard[0].venue, soul_graph::Venue::Group);
        assert_eq!(heard[0].self_contact_id, owner);
    }

    // Six group-only ties, and none of them rests on a message the user wrote.
    soul_graph::rebuild(&mut store).expect("rebuild");
    let graph = soul_graph::load(&store).expect("load");
    assert_eq!(graph.edges.len(), SPEAKERS);
    assert!(graph
        .edges
        .iter()
        .all(|edge| edge.types.contains(&soul_graph::TieType::GroupOnly)));
    assert!(graph
        .edges
        .iter()
        .all(|edge| edge.tie_strength.outgoing_count == 0));
}

/// AC-34, the half the fan-out test does not reach: a group message the user
/// sent later than anything the peers said.
///
/// Dropping the outgoing rows is only half of not crediting it. `last_contact`
/// is what the recency clock reads, and it is derived from the observations an
/// edge carries — so an owner group message that had slipped into A's evidence
/// would move A's last contact to the day the user typed, and a tie nobody has
/// heard from since spring would read as current. The peers here speak in
/// August and the user answers three weeks later, which is the gap that makes
/// the difference visible.
#[test]
fn an_owner_group_message_does_not_refresh_when_the_peers_were_last_heard_from() {
    let lines = [
        r#"{"type":"header","format":"soul-import-v1","version":1,"exported_at":"2026-08-24T08:00:00Z"}"#,
        r#"{"type":"message","id":"g-0001","occurred_at":"2026-08-01T09:00:00Z","sender_scope":"third_party","conversation_id":"g-01","sender_id":"u-a","text":"排期我看过了"}"#,
        r#"{"type":"message","id":"g-0002","occurred_at":"2026-08-02T09:00:00Z","sender_scope":"third_party","conversation_id":"g-01","sender_id":"u-b","text":"我这边也跟上"}"#,
        r#"{"type":"message","id":"g-0003","occurred_at":"2026-08-23T09:00:00Z","sender_scope":"self","conversation_id":"g-01","sender_id":"u-self","text":"那就按这个来"}"#,
    ];

    let staged = soul_import::soul_import_v1::parse(&lines.join("\n")).expect("valid");
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = SqlCipherStore::open(
        dir.path().join("soul.db"),
        &TestKeyProvider::from_seed(SEED),
    )
    .expect("open");
    let receipt = soul_import::commit::commit(&mut store, &staged).expect("commit");
    let owner = receipt.self_contact_id.expect("the file names the user");

    assert_eq!(
        receipt.evidence_written.len(),
        2,
        "two peers spoke; the user's own line names nobody",
    );

    soul_graph::rebuild(&mut store).expect("rebuild");
    let graph = soul_graph::load(&store).expect("load");

    // Each peer's last contact is their own message, not the user's answer.
    for (handle, said) in [
        ("u-a", "2026-08-01T09:00:00Z"),
        ("u-b", "2026-08-02T09:00:00Z"),
    ] {
        let peer = contact_for(&store, handle);
        let edges = graph.edges_for(peer);
        assert_eq!(edges.len(), 1, "one edge for {handle}");
        let strength = &edges[0].tie_strength;
        assert_eq!(
            strength.last_contact_utc.as_str(),
            said,
            "{handle} was last heard from when {handle} spoke, not when the user did",
        );
        assert_eq!(strength.outgoing_count, 0);
        assert_eq!(strength.incoming_count, 1);
        assert_eq!(strength.last_direct_contact_utc, None);
    }

    // And nothing was attributed to the user's side of the room at all.
    let observations: Vec<_> = store
        .list_evidence()
        .expect("evidence")
        .iter()
        .flat_map(interaction::interactions_in)
        .collect();
    assert!(observations
        .iter()
        .all(|observation| observation.direction == soul_graph::Direction::Incoming));
    assert!(observations
        .iter()
        .all(|observation| observation.self_contact_id == owner));
    assert!(
        observations
            .iter()
            .all(|observation| observation.occurred_at.as_str() != "2026-08-23T09:00:00Z"),
        "the message the user sent to the group is an event and not an observation",
    );
}

/// A file may write the user's own messages under more than one `sender_id`,
/// and both are still one person.
///
/// The parser used to observe each of them first and only then try to fold the
/// second into the first, which folded nothing: two participants marked as the
/// user, two contacts of class `self`, and a store where `soul_graph::rebuild`
/// fails on `AmbiguousOwner` from then on — permanently, because the rows are
/// written and the user has no way to edit them. One contract-legal file was
/// enough to do it.
#[test]
fn a_file_that_names_the_user_twice_still_leaves_one_owner() {
    let lines = [
        r#"{"type":"header","format":"soul-import-v1","version":1,"exported_at":"2026-08-24T08:00:00Z"}"#,
        r#"{"type":"message","id":"s-0001","occurred_at":"2026-08-01T09:00:00Z","sender_scope":"self","conversation_id":"c-01","sender_id":"u-self","text":"房子的事我来办"}"#,
        r#"{"type":"message","id":"s-0002","occurred_at":"2026-08-01T09:05:00Z","sender_scope":"third_party","conversation_id":"c-01","sender_id":"u-a","text":"辛苦了"}"#,
        r#"{"type":"message","id":"s-0003","occurred_at":"2026-08-02T09:00:00Z","sender_scope":"self","conversation_id":"c-01","sender_id":"u-self-old","text":"我换了个号，还是我"}"#,
        r#"{"type":"message","id":"s-0004","occurred_at":"2026-08-02T09:30:00Z","sender_scope":"third_party","conversation_id":"c-01","sender_id":"u-a","text":"记下了"}"#,
    ];

    let staged = soul_import::soul_import_v1::parse(&lines.join("\n")).expect("valid");
    assert_eq!(
        staged
            .participants
            .iter()
            .filter(|participant| participant.is_owner)
            .count(),
        1,
        "two identifiers for the user are one participant",
    );
    let owner_participant = staged.owner().expect("the file names the user");
    assert_eq!(
        owner_participant.handles.len(),
        2,
        "and that participant keeps both identifiers, so a re-import matches on either",
    );
    assert_eq!(staged.peers().count(), 1);

    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = SqlCipherStore::open(
        dir.path().join("soul.db"),
        &TestKeyProvider::from_seed(SEED),
    )
    .expect("open");
    let receipt = soul_import::commit::commit(&mut store, &staged).expect("commit");
    let owner = receipt.self_contact_id.expect("the file names the user");

    let contacts = store.list_contacts().expect("contacts");
    assert_eq!(
        contacts.len(),
        2,
        "the user and the one person they wrote to"
    );
    let selves: Vec<_> = contacts
        .iter()
        .filter(|contact| contact.contact_class == ContactClass::Owner)
        .collect();
    assert_eq!(selves.len(), 1, "one row of class `self`, not two");
    assert_eq!(selves[0].contact_id, owner);
    assert_eq!(
        selves[0].identifiers.iter().flatten().count(),
        2,
        "both identifiers are digested onto the one row",
    );

    // The point of all of it: the graph still builds, and keeps building.
    soul_graph::rebuild(&mut store).expect("rebuild");
    soul_graph::rebuild(&mut store).expect("a second rebuild is not poisoned either");
    let graph = soul_graph::load(&store).expect("load");
    assert_eq!(graph.self_contact_id, Some(owner));
    assert_eq!(graph.edges.len(), 1, "one peer, one edge");

    let peer = contact_for(&store, "u-a");
    let edges = graph.edges_for(peer);
    assert_eq!(edges.len(), 1);
    assert_eq!(
        edges[0].tie_strength.outgoing_count, 2,
        "both of the user's identifiers wrote to this person, and both count as the user",
    );
    assert_eq!(edges[0].tie_strength.incoming_count, 2);
}

/// The contact the file called `handle`, by the digest the importer stored.
fn contact_for(store: &SqlCipherStore, handle: &str) -> Uuid {
    let wanted = soul_import::ParticipantHandle::platform_uid(handle)
        .value_hash(soul_import::ImportSource::SoulImportV1);
    store
        .list_contacts()
        .expect("contacts")
        .into_iter()
        .find(|contact| {
            contact
                .identifiers
                .iter()
                .flatten()
                .any(|identifier| identifier.value_hash == wanted)
        })
        .map(|contact| contact.contact_id)
        .unwrap_or_else(|| panic!("the file names {handle}"))
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
