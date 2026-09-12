//! Product-level gates for the frozen T4D tie-strength rule.
//!
//! These tests deliberately enter through the SQLCipher store and
//! [`soul_graph::rebuild`]. They therefore stay red while rebuild uses any rule
//! of its own, and turn green only when the product path actually runs T4D with
//! one store-wide `as_of`.
//!
//! Not one threshold is written down here. The counts ladder, the demotion day
//! and the floor all come from `soul_algo_tie::constants`, and the instants come
//! from that crate's own fixture helper, so a test that says "one day short of
//! the demotion" means it whatever the frozen number becomes.

use uuid::Uuid;

use soul_algo_tie::constants::{
    DEMOTE_ONE_BAND_DAYS, MODERATE_MIN_INTERACTIONS, STRONG_MIN_ACTIVE_DAYS,
    STRONG_MIN_INTERACTIONS, WEAK_AFTER_SILENT_DAYS,
};
use soul_algo_tie::testing::at;
use soul_graph::interaction::{conversation_ref, interaction_evidence, InteractionRef};
use soul_graph::{Direction, Venue};
use soul_policy::clock::rfc3339_utc;
use soul_schema::common::{SchemaVersion, Subject, SupportedBand, Timestamp};
use soul_schema::contact::{ContactClass, SoulContact};
use soul_schema::memory::ForgetState;
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::{GraphStore, ProfileStore};

const SEED: &str = "t4d product gate";

/// The reply to a message, five minutes later. Small enough that both land on
/// the same UTC day.
const REPLY_AFTER: i64 = 300;

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
    occurred_at_unix: i64,
    venue: Venue,
) {
    let interaction = InteractionRef::new(
        Uuid::now_v7(),
        owner(),
        peer_id,
        conversation_ref("t4d-product", conversation),
        direction,
        Timestamp::new(rfc3339_utc(occurred_at_unix)),
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

/// Two one-to-one exchanges a day on each of `STRONG_MIN_ACTIVE_DAYS` doubled
/// days, ending `newest_days_ago` days before the fixture anchor.
///
/// Both Strong gates are cleared by construction and neither number is spelled
/// here: this is the `lilei_12` shape, built from the frozen counts.
fn add_reciprocal_direct_history(
    store: &mut SqlCipherStore,
    peer_id: Uuid,
    newest_days_ago: i64,
) -> i64 {
    let days = STRONG_MIN_ACTIVE_DAYS as i64 * 2;
    assert!(days * 2 >= STRONG_MIN_INTERACTIONS as i64);
    for day in 0..days {
        let sent = at(newest_days_ago + day, 9);
        observe(
            store,
            peer_id,
            "direct",
            Direction::Outgoing,
            sent,
            Venue::Direct,
        );
        observe(
            store,
            peer_id,
            "direct",
            Direction::Incoming,
            sent + REPLY_AFTER,
            Venue::Direct,
        );
    }
    at(newest_days_ago, 9) + REPLY_AFTER
}

/// One row about a second person, which is what puts the store's `as_of` where
/// the test wants it: `silent_days` whole days after this peer went quiet.
fn add_silence(store: &mut SqlCipherStore, elsewhere: Uuid, last_contact_unix: i64, days: i64) {
    observe(
        store,
        elsewhere,
        "recent",
        Direction::Incoming,
        last_contact_unix + days * 86_400,
        Venue::Direct,
    );
}

fn add_group_traffic(store: &mut SqlCipherStore, peer_id: Uuid, count: i64) {
    for index in 0..count {
        let direction = match index % 2 {
            0 => Direction::Outgoing,
            _ => Direction::Incoming,
        };
        observe(
            store,
            peer_id,
            "group",
            direction,
            at(index % 5, 12),
            Venue::Group,
        );
    }
}

fn rebuilt(store: &mut SqlCipherStore, peer_id: Uuid) -> soul_graph::TieStrength {
    soul_graph::rebuild(store).expect("rebuild");
    let graph = soul_graph::load(store).expect("load");
    let edges = graph.edges_for(peer_id);
    assert_eq!(edges.len(), 1, "expected one edge for peer {peer_id}");
    edges[0].tie_strength.clone()
}

fn rebuilt_band(store: &mut SqlCipherStore, peer_id: Uuid) -> SupportedBand {
    rebuilt(store, peer_id).band
}

/// The band a private history that has been silent for `days` comes out at,
/// and the silence the edge records for it.
fn after_silence(dormant: Uuid, elsewhere: Uuid, days: i64) -> (SupportedBand, i64) {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path(), &[dormant, elsewhere]);
    let went_quiet = add_reciprocal_direct_history(&mut store, dormant, days);
    add_silence(&mut store, elsewhere, went_quiet, days);
    let strength = rebuilt(&mut store, dormant);
    (strength.band, strength.silent_days)
}

#[test]
fn a_reciprocal_direct_history_over_enough_days_is_strong() {
    let dir = tempfile::tempdir().expect("temp dir");
    let peer_id = peer(2);
    let mut store = open(dir.path(), &[peer_id]);
    add_reciprocal_direct_history(&mut store, peer_id, 0);

    assert_eq!(rebuilt_band(&mut store, peer_id), SupportedBand::Strong);
}

#[test]
fn one_direct_exchange_each_way_does_not_let_group_volume_create_strong() {
    let dir = tempfile::tempdir().expect("temp dir");
    let peer_id = peer(3);
    let mut store = open(dir.path(), &[peer_id]);
    add_group_traffic(&mut store, peer_id, STRONG_MIN_INTERACTIONS as i64 * 10);
    let hello = at(0, 9);
    observe(
        &mut store,
        peer_id,
        "direct",
        Direction::Outgoing,
        hello,
        Venue::Direct,
    );
    observe(
        &mut store,
        peer_id,
        "direct",
        Direction::Incoming,
        hello + REPLY_AFTER,
        Venue::Direct,
    );

    assert_eq!(rebuilt_band(&mut store, peer_id), SupportedBand::Weak);
}

#[test]
fn group_only_traffic_is_weak_at_any_volume() {
    let dir = tempfile::tempdir().expect("temp dir");
    let peer_id = peer(4);
    let mut store = open(dir.path(), &[peer_id]);
    add_group_traffic(&mut store, peer_id, STRONG_MIN_INTERACTIONS as i64 * 10);

    assert_eq!(rebuilt_band(&mut store, peer_id), SupportedBand::Weak);
}

#[test]
fn dormant_peer_is_scored_against_the_store_wide_as_of() {
    let (band, silent_days) = after_silence(peer(5), peer(6), WEAK_AFTER_SILENT_DAYS + 1);
    assert_eq!(band, SupportedBand::Weak);
    assert_eq!(silent_days, WEAK_AFTER_SILENT_DAYS + 1);
}

#[test]
fn one_exchange_short_of_the_moderate_bar_is_still_weak() {
    let dir = tempfile::tempdir().expect("temp dir");
    let peer_id = peer(7);
    let mut store = open(dir.path(), &[peer_id]);
    for index in 0..(MODERATE_MIN_INTERACTIONS as i64 - 1) {
        let direction = match index % 2 {
            0 => Direction::Outgoing,
            _ => Direction::Incoming,
        };
        observe(
            &mut store,
            peer_id,
            "direct",
            direction,
            at(index, 9),
            Venue::Direct,
        );
    }

    assert_eq!(rebuilt_band(&mut store, peer_id), SupportedBand::Weak);
}

#[test]
fn the_moderate_bar_exactly_met_is_moderate() {
    let dir = tempfile::tempdir().expect("temp dir");
    let peer_id = peer(8);
    let mut store = open(dir.path(), &[peer_id]);
    for index in 0..(MODERATE_MIN_INTERACTIONS as i64) {
        let direction = match index % 2 {
            0 => Direction::Outgoing,
            _ => Direction::Incoming,
        };
        observe(
            &mut store,
            peer_id,
            "direct",
            direction,
            at(index, 9),
            Venue::Direct,
        );
    }

    assert_eq!(rebuilt_band(&mut store, peer_id), SupportedBand::Moderate);
}

/// The count gate on its own is not the Strong gate: the same traffic squeezed
/// into one day short of the span never gets there, however much of it there is.
#[test]
fn enough_exchanges_over_too_few_days_are_not_strong() {
    let dir = tempfile::tempdir().expect("temp dir");
    let peer_id = peer(9);
    let mut store = open(dir.path(), &[peer_id]);
    let days = STRONG_MIN_ACTIVE_DAYS as i64 - 1;
    for index in 0..(STRONG_MIN_INTERACTIONS as i64 + 2) {
        let direction = match index % 2 {
            0 => Direction::Outgoing,
            _ => Direction::Incoming,
        };
        observe(
            &mut store,
            peer_id,
            "direct",
            direction,
            at(index % days, 9) + index * 60,
            Venue::Direct,
        );
    }

    let strength = rebuilt(&mut store, peer_id);
    assert_eq!(strength.band, SupportedBand::Moderate);
    assert_eq!(strength.direct_active_day_count, days as u64);
}

/// Both Strong gates met exactly, and no more.
#[test]
fn the_strong_gates_exactly_met_are_strong() {
    let dir = tempfile::tempdir().expect("temp dir");
    let peer_id = peer(10);
    let mut store = open(dir.path(), &[peer_id]);
    for index in 0..(STRONG_MIN_INTERACTIONS as i64) {
        let direction = match index % 2 {
            0 => Direction::Outgoing,
            _ => Direction::Incoming,
        };
        observe(
            &mut store,
            peer_id,
            "direct",
            direction,
            at(index % STRONG_MIN_ACTIVE_DAYS as i64, 9) + index * 60,
            Venue::Direct,
        );
    }

    let strength = rebuilt(&mut store, peer_id);
    assert_eq!(strength.band, SupportedBand::Strong);
    assert_eq!(strength.direct_active_day_count, STRONG_MIN_ACTIVE_DAYS);
    assert_eq!(strength.interaction_count, STRONG_MIN_INTERACTIONS);
}

/// The demotion is a closed interval: one day short of it changes nothing.
#[test]
fn one_day_short_of_the_demotion_keeps_a_strong_tie_strong() {
    let (band, silent_days) = after_silence(peer(11), peer(12), DEMOTE_ONE_BAND_DAYS - 1);
    assert_eq!(band, SupportedBand::Strong);
    assert_eq!(silent_days, DEMOTE_ONE_BAND_DAYS - 1);
}

#[test]
fn the_demotion_day_itself_costs_a_strong_tie_one_band() {
    let (band, silent_days) = after_silence(peer(13), peer(14), DEMOTE_ONE_BAND_DAYS);
    assert_eq!(band, SupportedBand::Moderate);
    assert_eq!(silent_days, DEMOTE_ONE_BAND_DAYS);
}

/// One rung, not two: silence up to the day before the floor has cost exactly
/// one band and no more.
#[test]
fn one_day_short_of_the_floor_has_still_cost_only_one_band() {
    let (band, silent_days) = after_silence(peer(15), peer(16), WEAK_AFTER_SILENT_DAYS - 1);
    assert_eq!(band, SupportedBand::Moderate);
    assert_eq!(silent_days, WEAK_AFTER_SILENT_DAYS - 1);
}

#[test]
fn the_floor_makes_a_strong_history_weak() {
    let (band, silent_days) = after_silence(peer(17), peer(18), WEAK_AFTER_SILENT_DAYS);
    assert_eq!(band, SupportedBand::Weak);
    assert_eq!(silent_days, WEAK_AFTER_SILENT_DAYS);
}
