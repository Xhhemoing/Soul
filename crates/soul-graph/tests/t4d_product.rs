//! Product-level gates for the frozen T4D tie-strength rule.
//!
//! These tests deliberately enter through the SQLCipher store and
//! [`soul_graph::rebuild`]. They therefore stay red while rebuild still uses
//! the retired total-traffic rule, and turn green only when the product path
//! actually uses T4D with one store-wide `as_of`.
//!
//! The AC-28 / AC-29 / AC-30 gates at the bottom of the file are held to the
//! named fixtures in `soul_algo_tie::testing` rather than to evidence typed
//! out here. Goal 1's obligation is that the product path and the frozen crate
//! reach the same verdict on the same evidence, so the fixtures are imported
//! whole and the expected reading is asked of the frozen crate; nothing here
//! restates a threshold.

use std::collections::BTreeSet;

use uuid::Uuid;

use soul_algo_tie::testing::{
    dormant_2019, group_heavy_plus_one_direct_each_way, group_heavy_plus_three_directs, lilei_12,
    Fixture, AS_OF_2026_08_24,
};
use soul_algo_tie::{
    as_of_max, civil_from_epoch_day, epoch_day, Band, TieAlgo, TieScore, SECONDS_PER_DAY,
};
use soul_graph::interaction::{conversation_ref, interaction_evidence, InteractionRef};
use soul_graph::{Direction, TieStrength, Venue};
use soul_schema::common::{SchemaVersion, Subject, SupportedBand, Timestamp};
use soul_schema::contact::{ContactClass, SoulContact};
use soul_schema::memory::ForgetState;
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::{GraphStore, ProfileStore};

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

#[test]
fn two_reciprocal_direct_events_are_still_weak() {
    let dir = tempfile::tempdir().expect("temp dir");
    let peer_id = peer(7);
    let mut store = open(dir.path(), &[peer_id]);
    observe(
        &mut store,
        peer_id,
        "direct",
        Direction::Outgoing,
        "2026-08-01T09:00:00Z",
        Venue::Direct,
    );
    observe(
        &mut store,
        peer_id,
        "direct",
        Direction::Incoming,
        "2026-08-02T09:00:00Z",
        Venue::Direct,
    );

    assert_eq!(rebuilt_band(&mut store, peer_id), SupportedBand::Weak);
}

#[test]
fn three_reciprocal_direct_events_over_three_days_are_moderate() {
    let dir = tempfile::tempdir().expect("temp dir");
    let peer_id = peer(8);
    let mut store = open(dir.path(), &[peer_id]);
    for (day, direction) in [
        (1, Direction::Outgoing),
        (2, Direction::Incoming),
        (3, Direction::Outgoing),
    ] {
        observe(
            &mut store,
            peer_id,
            "direct",
            direction,
            &format!("2026-08-{day:02}T09:00:00Z"),
            Venue::Direct,
        );
    }

    assert_eq!(rebuilt_band(&mut store, peer_id), SupportedBand::Moderate);
}

#[test]
fn nine_events_and_even_a_tenth_over_only_two_utc_days_are_not_strong() {
    let dir = tempfile::tempdir().expect("temp dir");
    let peer_id = peer(9);
    let mut store = open(dir.path(), &[peer_id]);
    for index in 0..9 {
        let day = 1 + index % 2;
        let direction = if index % 2 == 0 {
            Direction::Outgoing
        } else {
            Direction::Incoming
        };
        observe(
            &mut store,
            peer_id,
            "direct",
            direction,
            &format!("2026-08-{day:02}T{index:02}:00:00Z"),
            Venue::Direct,
        );
    }

    assert_eq!(rebuilt_band(&mut store, peer_id), SupportedBand::Moderate);

    observe(
        &mut store,
        peer_id,
        "direct",
        Direction::Incoming,
        "2026-08-02T10:00:00Z",
        Venue::Direct,
    );
    assert_eq!(rebuilt_band(&mut store, peer_id), SupportedBand::Moderate);
}

#[test]
fn ten_reciprocal_direct_events_over_three_utc_days_are_strong() {
    let dir = tempfile::tempdir().expect("temp dir");
    let peer_id = peer(10);
    let mut store = open(dir.path(), &[peer_id]);
    for index in 0..10 {
        let day = 1 + index % 3;
        let direction = if index % 2 == 0 {
            Direction::Outgoing
        } else {
            Direction::Incoming
        };
        observe(
            &mut store,
            peer_id,
            "direct",
            direction,
            &format!("2026-08-{day:02}T{index:02}:00:00Z"),
            Venue::Direct,
        );
    }

    assert_eq!(rebuilt_band(&mut store, peer_id), SupportedBand::Strong);
}

#[test]
fn one_hundred_seventy_nine_silent_days_keep_a_strong_tie_strong() {
    let dir = tempfile::tempdir().expect("temp dir");
    let dormant = peer(11);
    let active = peer(12);
    let mut store = open(dir.path(), &[dormant, active]);
    add_reciprocal_direct_history(&mut store, dormant, 2025, 1);
    observe(
        &mut store,
        active,
        "recent",
        Direction::Incoming,
        "2025-07-04T09:05:00Z",
        Venue::Direct,
    );

    assert_eq!(rebuilt_band(&mut store, dormant), SupportedBand::Strong);
}

#[test]
fn one_hundred_eighty_silent_days_demote_strong_to_moderate() {
    let dir = tempfile::tempdir().expect("temp dir");
    let dormant = peer(13);
    let active = peer(14);
    let mut store = open(dir.path(), &[dormant, active]);
    add_reciprocal_direct_history(&mut store, dormant, 2025, 1);
    observe(
        &mut store,
        active,
        "recent",
        Direction::Incoming,
        "2025-07-05T09:05:00Z",
        Venue::Direct,
    );

    assert_eq!(rebuilt_band(&mut store, dormant), SupportedBand::Moderate);
}

#[test]
fn three_hundred_fifty_nine_silent_days_demote_only_one_band() {
    let dir = tempfile::tempdir().expect("temp dir");
    let dormant = peer(15);
    let active = peer(16);
    let mut store = open(dir.path(), &[dormant, active]);
    add_reciprocal_direct_history(&mut store, dormant, 2025, 1);
    observe(
        &mut store,
        active,
        "recent",
        Direction::Incoming,
        "2025-12-31T09:05:00Z",
        Venue::Direct,
    );

    assert_eq!(rebuilt_band(&mut store, dormant), SupportedBand::Moderate);
}

#[test]
fn three_hundred_sixty_silent_days_make_a_strong_history_weak() {
    let dir = tempfile::tempdir().expect("temp dir");
    let dormant = peer(17);
    let active = peer(18);
    let mut store = open(dir.path(), &[dormant, active]);
    add_reciprocal_direct_history(&mut store, dormant, 2025, 1);
    observe(
        &mut store,
        active,
        "recent",
        Direction::Incoming,
        "2026-01-01T09:05:00Z",
        Venue::Direct,
    );

    assert_eq!(rebuilt_band(&mut store, dormant), SupportedBand::Weak);
}

// ---------------------------------------------------------------------------
// AC-28 / AC-29 / AC-30, held to the frozen crate's own named fixtures
// ---------------------------------------------------------------------------

/// The instant every fixture in `soul_algo_tie::testing` is judged against,
/// written the way the store writes timestamps.
///
/// Pinned against the frozen crate's own constant by
/// [`the_fixture_loader_preserves_the_instant_and_the_venue`], so this string
/// is a spelling of `AS_OF_2026_08_24` rather than a second opinion about it.
const FIXTURE_AS_OF_UTC: &str = "2026-08-24T14:00:00Z";

/// A whole Unix second as the RFC 3339 UTC string the store holds.
///
/// The calendar comes from the frozen crate, which is where the graph's own
/// adapter reads it back out again.
fn rfc3339_utc(occurred_at_unix: i64) -> String {
    let (year, month, day) = civil_from_epoch_day(epoch_day(occurred_at_unix));
    let second_of_day = occurred_at_unix.rem_euclid(SECONDS_PER_DAY);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        second_of_day / 3_600,
        second_of_day / 60 % 60,
        second_of_day % 60,
    )
}

/// Import one named fixture as stored evidence about `peer_id`.
///
/// One evidence row per interaction, carrying the fixture's own direction,
/// venue and instant. Conversations are named after the fixture so two
/// fixtures in one store cannot collide on a conversation id.
fn load_fixture(store: &mut SqlCipherStore, peer_id: Uuid, fixture: &Fixture) {
    for row in &fixture.log {
        assert_eq!(
            row.peer_id, fixture.peer_id,
            "{} carries a row about somebody else",
            fixture.name,
        );
        let direction = match row.outgoing {
            true => Direction::Outgoing,
            false => Direction::Incoming,
        };
        let venue = match row.venue_direct {
            true => Venue::Direct,
            false => Venue::Group,
        };
        observe(
            store,
            peer_id,
            &format!("{}-{}", fixture.name, row.conversation_id),
            direction,
            &rfc3339_utc(row.occurred_at_unix),
            venue,
        );
    }
}

/// A single row on a peer of its own, at the instant the fixtures are judged.
///
/// A rebuild scores the whole store against its newest observation, so this is
/// how a fixture gets read at its own `as_of` instead of at whatever its last
/// message happens to be.
fn pin_store_as_of(store: &mut SqlCipherStore, anchor: Uuid) {
    observe(
        store,
        anchor,
        "as-of-anchor",
        Direction::Incoming,
        FIXTURE_AS_OF_UTC,
        Venue::Direct,
    );
}

fn strength_of(store: &SqlCipherStore, peer_id: Uuid) -> TieStrength {
    let graph = soul_graph::load(store).expect("load");
    let edges = graph.edges_for(peer_id);
    assert_eq!(edges.len(), 1, "expected one edge for peer {peer_id}");
    edges[0].tie_strength.clone()
}

fn supported(band: Band) -> SupportedBand {
    match band {
        Band::Weak => SupportedBand::Weak,
        Band::Moderate => SupportedBand::Moderate,
        Band::Strong => SupportedBand::Strong,
    }
}

/// What the frozen rule makes of this fixture as of one instant.
fn frozen_reading(fixture: &Fixture, as_of_unix: i64) -> TieScore {
    soul_algo_tie::score(fixture.peer_id, &fixture.log, as_of_unix)
}

/// The stored edge says exactly what the frozen crate says about the same
/// evidence: the band, every count behind it, and every instant.
fn assert_edge_matches_frozen(strength: &TieStrength, fixture: &Fixture, as_of_unix: i64) {
    let reading = frozen_reading(fixture, as_of_unix);
    let name = fixture.name;
    assert_eq!(strength.band, supported(reading.band), "{name}: band");
    assert_eq!(strength.algorithm_id, reading.algorithm_id, "{name}: rule");
    assert_eq!(
        (
            strength.direct_out_count,
            strength.direct_in_count,
            strength.group_out_count,
            strength.group_in_count,
        ),
        (
            reading.direct_out_count,
            reading.direct_in_count,
            reading.group_out_count,
            reading.group_in_count,
        ),
        "{name}: split counts",
    );
    assert_eq!(
        (
            strength.interaction_count,
            strength.outgoing_count,
            strength.incoming_count,
            strength.conversation_count,
        ),
        (
            reading.interaction_count,
            reading.outgoing_count,
            reading.incoming_count,
            reading.conversation_count,
        ),
        "{name}: totals",
    );
    assert_eq!(
        (strength.active_day_count, strength.direct_active_day_count),
        (reading.active_day_count, reading.direct_active_day_count),
        "{name}: active days",
    );
    assert_eq!(strength.silent_days, reading.silent_days, "{name}: silence");
    assert_eq!(
        strength.first_contact_utc.as_str(),
        rfc3339_utc(reading.first_contact_unix),
        "{name}: first contact",
    );
    assert_eq!(
        strength.last_contact_utc.as_str(),
        rfc3339_utc(reading.last_contact_unix),
        "{name}: last contact",
    );
    assert_eq!(
        strength
            .last_direct_contact_utc
            .as_ref()
            .map(Timestamp::as_str)
            .map(str::to_owned),
        (reading.last_direct_contact_unix != 0)
            .then(|| rfc3339_utc(reading.last_direct_contact_unix)),
        "{name}: last private contact",
    );
    assert_eq!(
        strength.as_of_utc.as_ref().map(Timestamp::as_str),
        Some(rfc3339_utc(reading.as_of_unix).as_str()),
        "{name}: as_of",
    );
}

/// The loader is a spelling change and nothing else: the instants, venues and
/// directions the fixture holds are the ones the store gets back.
#[test]
fn the_fixture_loader_preserves_the_instant_and_the_venue() {
    assert_eq!(rfc3339_utc(AS_OF_2026_08_24), FIXTURE_AS_OF_UTC);

    let dir = tempfile::tempdir().expect("temp dir");
    let anchored = peer(19);
    let mut store = open(dir.path(), &[anchored]);
    let fixture = lilei_12();
    load_fixture(&mut store, anchored, &fixture);

    // No anchor row, so the store's own newest observation is this fixture's.
    let expected_as_of = as_of_max(&fixture.log).expect("the fixture has rows");
    soul_graph::rebuild(&mut store).expect("rebuild");
    assert_edge_matches_frozen(&strength_of(&store, anchored), &fixture, expected_as_of);
}

/// AC-28. The decisive fixture: a project channel plus exactly one private
/// message each way. Weak at the store, and the edge carries the number the
/// verdict turns on — how many days these two ever spoke privately.
#[test]
fn the_decisive_group_flood_edge_is_weak_and_carries_its_direct_active_day_count() {
    let dir = tempfile::tempdir().expect("temp dir");
    let colleague = peer(20);
    let anchor = peer(21);
    let mut store = open(dir.path(), &[colleague, anchor]);
    let fixture = group_heavy_plus_one_direct_each_way();
    load_fixture(&mut store, colleague, &fixture);
    pin_store_as_of(&mut store, anchor);

    soul_graph::rebuild(&mut store).expect("rebuild");
    let strength = strength_of(&store, colleague);

    assert_eq!(strength.band, SupportedBand::Weak);
    // Thirty group exchanges across ten days, and one afternoon of private
    // hellos. The private day count is the decisive one, and it is on the edge
    // rather than left for a reader to recount.
    assert_eq!(strength.direct_active_day_count, 1);
    assert_eq!(strength.active_day_count, 10);
    assert!(strength.direct_active_day_count < strength.active_day_count);
    assert_eq!(
        (strength.direct_out_count, strength.direct_in_count),
        (1, 1)
    );
    assert_eq!(
        (strength.group_out_count, strength.group_in_count),
        (15, 15),
    );
    assert_eq!(
        strength.as_of_utc.as_ref().map(Timestamp::as_str),
        Some(FIXTURE_AS_OF_UTC),
    );
    assert_edge_matches_frozen(&strength, &fixture, AS_OF_2026_08_24);

    // The falsifier the row exists for: counting any venue calls this same
    // evidence a close tie, so reverting the venue latch turns this test red
    // rather than quietly restoring the defect.
    assert_eq!(
        TieAlgo::T4
            .score(fixture.peer_id, &fixture.log, fixture.as_of)
            .band,
        Band::Strong,
        "the rollback rule calls the decisive fixture Strong; that is the hole T4D closes",
    );
}

/// AC-29. The anchor and the self-healing path in one store and one rebuild:
/// absorbing T4D must not re-judge `lilei_12`, and three private messages must
/// be enough to lift the same group-heavy shape back to Moderate.
#[test]
fn the_anchor_stays_strong_while_three_directs_reach_moderate_in_one_rebuild() {
    let dir = tempfile::tempdir().expect("temp dir");
    let friend = peer(22);
    let colleague = peer(23);
    let anchor = peer(24);
    let mut store = open(dir.path(), &[friend, colleague, anchor]);
    let anchor_case = lilei_12();
    let healed = group_heavy_plus_three_directs();
    load_fixture(&mut store, friend, &anchor_case);
    load_fixture(&mut store, colleague, &healed);
    pin_store_as_of(&mut store, anchor);

    soul_graph::rebuild(&mut store).expect("rebuild");
    let friend_strength = strength_of(&store, friend);
    let colleague_strength = strength_of(&store, colleague);

    assert_eq!(friend_strength.band, SupportedBand::Strong);
    assert_eq!(colleague_strength.band, SupportedBand::Moderate);
    assert_edge_matches_frozen(&friend_strength, &anchor_case, AS_OF_2026_08_24);
    assert_edge_matches_frozen(&colleague_strength, &healed, AS_OF_2026_08_24);

    // What separates this colleague from the Weak one above is the private
    // traffic and nothing else: the group side of the two fixtures is the same
    // fan-out.
    let decisive = group_heavy_plus_one_direct_each_way();
    let weak_reading = frozen_reading(&decisive, AS_OF_2026_08_24);
    assert_eq!(
        (
            colleague_strength.group_out_count,
            colleague_strength.group_in_count,
        ),
        (weak_reading.group_out_count, weak_reading.group_in_count),
    );
    assert_eq!(
        colleague_strength.direct_out_count + colleague_strength.direct_in_count,
        3,
    );
    assert_eq!(colleague_strength.direct_active_day_count, 2);
}

/// AC-30. One store holding an active tie and a friendship that stopped in
/// 2019: one rebuild, one `as_of`, and it is on both edges.
#[test]
fn both_edges_carry_the_same_store_wide_as_of_and_only_the_dormant_one_is_weak() {
    let dir = tempfile::tempdir().expect("temp dir");
    let active = peer(25);
    let dormant = peer(26);
    let mut store = open(dir.path(), &[active, dormant]);
    let active_case = lilei_12();
    let dormant_case = dormant_2019();
    load_fixture(&mut store, active, &active_case);
    load_fixture(&mut store, dormant, &dormant_case);

    // The documented default: the newest observation in the whole store,
    // recomputed here from the imported evidence rather than typed out.
    let whole_store: Vec<_> = active_case
        .log
        .iter()
        .chain(&dormant_case.log)
        .cloned()
        .collect();
    let store_as_of = as_of_max(&whole_store).expect("the store has rows");
    assert_eq!(store_as_of, as_of_max(&active_case.log).expect("rows"));

    soul_graph::rebuild(&mut store).expect("rebuild");
    let active_strength = strength_of(&store, active);
    let dormant_strength = strength_of(&store, dormant);

    let graph = soul_graph::load(&store).expect("load");
    assert_eq!(graph.edges.len(), 2);
    let stamps: BTreeSet<&str> = graph
        .edges
        .iter()
        .map(|edge| {
            edge.tie_strength
                .as_of_utc
                .as_ref()
                .map(Timestamp::as_str)
                .expect("every rebuilt edge records the instant it was scored against")
        })
        .collect();
    assert_eq!(
        stamps,
        BTreeSet::from([rfc3339_utc(store_as_of).as_str()]),
        "one rebuild, one as_of, written onto every edge it touched",
    );

    assert_eq!(dormant_strength.band, SupportedBand::Weak);
    assert_eq!(active_strength.band, SupportedBand::Strong);
    assert_edge_matches_frozen(&active_strength, &active_case, store_as_of);
    assert_edge_matches_frozen(&dormant_strength, &dormant_case, store_as_of);

    // The silence is on the edge next to the instant it was measured from, so
    // a reader can check the subtraction without re-running anything.
    assert_eq!(active_strength.silent_days, 0);
    assert!(dormant_strength.silent_days > 2_500);

    // The falsifier: giving each peer its own newest timestamp would make this
    // 2019 friendship look current, and the band would come back Strong.
    let per_peer_as_of = as_of_max(&dormant_case.log).expect("rows");
    assert!(per_peer_as_of < store_as_of);
    assert_eq!(
        frozen_reading(&dormant_case, per_peer_as_of).band,
        Band::Strong,
        "a per-peer as_of is what this row exists to forbid",
    );
}
