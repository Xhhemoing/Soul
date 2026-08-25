//! Product-level gates for the frozen T4D tie-strength rule.
//!
//! These tests deliberately enter through the SQLCipher store and
//! [`soul_graph::rebuild`]. They therefore stay red while rebuild still uses
//! the retired total-traffic rule, and turn green only when the product path
//! actually uses T4D with one store-wide `as_of`.

use uuid::Uuid;

use soul_graph::interaction::{conversation_ref, interaction_evidence, InteractionRef};
use soul_graph::{Direction, Venue};
use soul_schema::common::{SchemaVersion, Subject, SupportedBand, Timestamp};
use soul_schema::contact::{ContactClass, SoulContact};
use soul_schema::memory::ForgetState;
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::ProfileStore;

const SEED: &str = "t4d product gate";

fn owner() -> Uuid {
    Uuid::from_u128(0x0192_a1b2_c3d4_7e5f_8a9b_0c1d_2e3f_4001)
}

fn peer(tail: u128) -> Uuid {
    Uuid::from_u128(0x0192_a1b2_c3d4_7e5f_8a9b_0c1d_2e3f_4000 + tail)
}

fn contact(contact_id: Uuid, contact_class: ContactClass) -> SoulContact {
    SoulContact {
        schema_version: SchemaVersion,
        contact_id,
        contact_class,
        display_label_ref: None,
        identifiers: None,
        forget_state: ForgetState::Active,
    }
}

fn open(dir: &std::path::Path, peers: &[Uuid]) -> SqlCipherStore {
    let mut store =
        SqlCipherStore::open(dir.join("soul.db"), &TestKeyProvider::from_seed(SEED)).expect("open");
    store
        .put_contact(contact(owner(), ContactClass::Owner))
        .expect("owner");
    for peer_id in peers {
        store
            .put_contact(contact(*peer_id, ContactClass::ThirdParty))
            .expect("peer");
    }
    store
}

fn observe(
    store: &mut SqlCipherStore,
    peer_id: Uuid,
    conversation: &str,
    direction: Direction,
    occurred_at: &str,
    venue: Venue,
) {
    let interaction = InteractionRef::new(
        Uuid::now_v7(),
        owner(),
        peer_id,
        conversation_ref("t4d-product", conversation),
        direction,
        Timestamp::new(occurred_at),
        venue,
    );
    let subject = match direction {
        Direction::Outgoing => Subject::Mixed,
        Direction::Incoming => Subject::ThirdParty,
    };
    store
        .put_evidence(interaction_evidence(Uuid::now_v7(), subject, &interaction))
        .expect("evidence");
}

fn add_reciprocal_direct_history(store: &mut SqlCipherStore, peer_id: Uuid, year: u32, month: u32) {
    for day in 1..=6 {
        observe(
            store,
            peer_id,
            "direct",
            Direction::Outgoing,
            &format!("{year:04}-{month:02}-{day:02}T09:00:00Z"),
            Venue::Direct,
        );
        observe(
            store,
            peer_id,
            "direct",
            Direction::Incoming,
            &format!("{year:04}-{month:02}-{day:02}T09:05:00Z"),
            Venue::Direct,
        );
    }
}

fn add_group_traffic(store: &mut SqlCipherStore, peer_id: Uuid, count: u32) {
    for index in 0..count {
        let day = 1 + index % 5;
        let direction = if index % 2 == 0 {
            Direction::Outgoing
        } else {
            Direction::Incoming
        };
        observe(
            store,
            peer_id,
            "group",
            direction,
            &format!("2026-08-{day:02}T12:00:00Z"),
            Venue::Group,
        );
    }
}

fn rebuilt_band(store: &mut SqlCipherStore, peer_id: Uuid) -> SupportedBand {
    soul_graph::rebuild(store).expect("rebuild");
    let graph = soul_graph::load(store).expect("load");
    let edges = graph.edges_for(peer_id);
    assert_eq!(edges.len(), 1, "expected one edge for peer {peer_id}");
    edges[0].tie_strength.band
}

#[test]
fn twelve_reciprocal_direct_exchanges_over_six_days_are_strong() {
    let dir = tempfile::tempdir().expect("temp dir");
    let peer_id = peer(2);
    let mut store = open(dir.path(), &[peer_id]);
    add_reciprocal_direct_history(&mut store, peer_id, 2026, 8);

    assert_eq!(rebuilt_band(&mut store, peer_id), SupportedBand::Strong);
}

#[test]
fn one_direct_exchange_each_way_does_not_let_group_volume_create_strong() {
    let dir = tempfile::tempdir().expect("temp dir");
    let peer_id = peer(3);
    let mut store = open(dir.path(), &[peer_id]);
    add_group_traffic(&mut store, peer_id, 100);
    observe(
        &mut store,
        peer_id,
        "direct",
        Direction::Outgoing,
        "2026-08-06T09:00:00Z",
        Venue::Direct,
    );
    observe(
        &mut store,
        peer_id,
        "direct",
        Direction::Incoming,
        "2026-08-06T09:05:00Z",
        Venue::Direct,
    );

    assert_eq!(rebuilt_band(&mut store, peer_id), SupportedBand::Weak);
}

#[test]
fn group_only_traffic_is_weak_at_any_volume() {
    let dir = tempfile::tempdir().expect("temp dir");
    let peer_id = peer(4);
    let mut store = open(dir.path(), &[peer_id]);
    add_group_traffic(&mut store, peer_id, 100);

    assert_eq!(rebuilt_band(&mut store, peer_id), SupportedBand::Weak);
}

#[test]
fn dormant_peer_is_scored_against_the_store_wide_as_of() {
    let dir = tempfile::tempdir().expect("temp dir");
    let dormant = peer(5);
    let active = peer(6);
    let mut store = open(dir.path(), &[dormant, active]);
    add_reciprocal_direct_history(&mut store, dormant, 2020, 1);
    observe(
        &mut store,
        active,
        "recent",
        Direction::Incoming,
        "2026-08-24T12:00:00Z",
        Venue::Direct,
    );

    assert_eq!(rebuilt_band(&mut store, dormant), SupportedBand::Weak);
}
