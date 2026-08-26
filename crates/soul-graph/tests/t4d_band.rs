//! The frozen tie-strength rule, proved through the store rather than in the
//! algorithm crate.
//!
//! `soul-algo-tie` already holds the rule to its own fixtures. What is checked
//! here is the wiring around it: that a rebuild adapts stored evidence without
//! losing a venue or an instant, that every peer is scored against one
//! store-wide `as_of`, and that the numbers the rule reports survive the round
//! trip through the relationship row.
//!
//! No threshold is spelled out in this file. Where a test needs the demotion
//! day or the floor it reads them from `soul_algo_tie::constants`, and the
//! whole fixture table is replayed from `soul_algo_tie::testing` rather than
//! restated here, so a case cannot be copied slightly wrong.

use serde_json::json;
use uuid::Uuid;

use soul_algo_tie::constants::{DEMOTE_ONE_BAND_DAYS, WEAK_AFTER_SILENT_DAYS};
use soul_algo_tie::testing::DAY;
use soul_graph::interaction::{conversation_ref, interaction_evidence, InteractionRef};
use soul_graph::{Direction, GraphError, TieType, Venue};
use soul_policy::clock::rfc3339_utc;
use soul_schema::common::{SchemaVersion, Subject, SupportedBand, Timestamp};
use soul_schema::contact::{ContactClass, SoulContact};
use soul_schema::memory::ForgetState;
use soul_schema::relationship::{EgressScope, SoulRelationship};
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::{GraphStore, ProfileStore};

const SEED: &str = "t4d band at the store";

/// 2025-01-06T09:05:00Z, the last private word in the histories below.
///
/// Held as a Unix second so a test can say "this many days later" without
/// doing calendar arithmetic by hand; every silence in this file is measured
/// off it and rendered back into the RFC 3339 string the store keeps.
/// `the_history_anchor_is_the_date_it_claims_to_be` holds the two spellings
/// together.
const HISTORY_END: i64 = 1_736_154_300;

fn owner() -> Uuid {
    Uuid::from_u128(0x0192_a1b2_c3d4_7e5f_8a9b_0c1d_2e3f_4001)
}

fn peer(tail: u128) -> Uuid {
    Uuid::from_u128(0x0192_a1b2_c3d4_7e5f_8a9b_0c1d_2e3f_4100 + tail)
}

/// A contact id for one of the frozen crate's fixture peers.
///
/// The fixture ids are small dense integers of the kind the rule works in, and
/// the store works in UUIDs; this is the only place the two meet, and it is a
/// test-local mapping rather than anything the product writes down.
fn fixture_peer(fixture_peer_id: u64) -> Uuid {
    Uuid::from_u128(0x0192_a1b2_c3d4_7e5f_8a9b_0c1d_2e3f_4200 + u128::from(fixture_peer_id))
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
        conversation_ref("t4d-band", conversation),
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

/// Twelve one-to-one exchanges over six consecutive days, both directions.
/// The lilei shape, which the frozen rule calls Strong when it is not stale.
fn reciprocal_direct_history(store: &mut SqlCipherStore, peer_id: Uuid, year: u32, month: u32) {
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

/// A store holding every row of the named fixtures, judged at the instant they
/// were written for.
///
/// The fixtures come with their own `as_of`, and a rebuild takes the newest row
/// in the store. Those agree because of the last row written here: one message
/// from a peer nobody is asking about, at exactly the fixture anchor. Without
/// it a fixture would be judged against its own newest message, which is the
/// per-peer clock the whole design refuses.
fn staged(dir: &std::path::Path, fixtures: &[soul_algo_tie::testing::Fixture]) -> SqlCipherStore {
    let clock = fixture_peer(0);
    let mut contacts: Vec<Uuid> = fixtures
        .iter()
        .map(|fixture| fixture_peer(fixture.peer_id))
        .collect();
    contacts.push(clock);

    let mut store = open(dir, &contacts);
    for fixture in fixtures {
        assert_eq!(
            fixture.as_of,
            soul_algo_tie::testing::AS_OF_2026_08_24,
            "{}: this replay puts one instant in the store",
            fixture.name,
        );
        for row in &fixture.log {
            observe(
                &mut store,
                fixture_peer(row.peer_id),
                &format!("conversation-{}", row.conversation_id),
                match row.outgoing {
                    true => Direction::Outgoing,
                    false => Direction::Incoming,
                },
                &rfc3339_utc(row.occurred_at_unix),
                match row.venue_direct {
                    true => Venue::Direct,
                    false => Venue::Group,
                },
            );
        }
    }
    observe(
        &mut store,
        clock,
        "the-clock",
        Direction::Incoming,
        &rfc3339_utc(soul_algo_tie::testing::AS_OF_2026_08_24),
        Venue::Direct,
    );
    store
}

/// The edge one peer has, read back off the store.
fn edge_strength(store: &SqlCipherStore, peer_id: Uuid) -> soul_graph::TieStrength {
    let graph = soul_graph::load(store).expect("load");
    let edges = graph.edges_for(peer_id);
    assert_eq!(edges.len(), 1, "expected one edge for peer {peer_id}");
    edges[0].tie_strength.clone()
}

fn edge_types(store: &SqlCipherStore, peer_id: Uuid) -> Vec<TieType> {
    let graph = soul_graph::load(store).expect("load");
    graph.edges_for(peer_id)[0].types.clone()
}

/// The decisive fixture behind the rule, run through the product path: a busy
/// project channel plus one private hello each way is not a close tie.
#[test]
fn a_group_flood_with_one_direct_hello_each_way_is_weak_at_the_store() {
    let dir = tempfile::tempdir().expect("temp dir");
    let colleague = peer(1);
    let mut store = open(dir.path(), &[colleague]);

    for index in 0..30 {
        let direction = match index % 2 {
            0 => Direction::Outgoing,
            _ => Direction::Incoming,
        };
        observe(
            &mut store,
            colleague,
            "project-channel",
            direction,
            &format!("2026-08-{:02}T12:{:02}:00Z", 1 + index / 3, index % 3 * 10),
            Venue::Group,
        );
    }
    observe(
        &mut store,
        colleague,
        "hello",
        Direction::Outgoing,
        "2026-08-10T20:00:00Z",
        Venue::Direct,
    );
    observe(
        &mut store,
        colleague,
        "hello",
        Direction::Incoming,
        "2026-08-10T20:04:00Z",
        Venue::Direct,
    );

    soul_graph::rebuild(&mut store).expect("rebuild");
    let strength = edge_strength(&store, colleague);

    assert_eq!(strength.band, SupportedBand::Weak);
    assert_eq!(strength.direct_out_count, 1);
    assert_eq!(strength.direct_in_count, 1);
    assert_eq!(strength.group_out_count, 15);
    assert_eq!(strength.group_in_count, 15);
    assert_eq!(strength.interaction_count, 32);
    assert_eq!(strength.active_day_count, 10);

    // The observed shapes are what was seen, not what it was worth: these two
    // did speak privately, and both of them have written.
    let types = edge_types(&store, colleague);
    assert!(types.contains(&TieType::Direct));
    assert!(types.contains(&TieType::Reciprocal));
}

/// Fifty group exchanges over fifty days, never once one to one. Weak, with no
/// ceiling to soften it — the priced cost of banding on private traffic alone.
#[test]
fn group_only_fifty_is_weak_not_moderate_at_the_store() {
    let dir = tempfile::tempdir().expect("temp dir");
    let acquaintance = peer(2);
    let mut store = open(dir.path(), &[acquaintance]);

    for index in 0..50u32 {
        let direction = match index % 2 {
            0 => Direction::Outgoing,
            _ => Direction::Incoming,
        };
        observe(
            &mut store,
            acquaintance,
            "big-room",
            direction,
            &format!("2026-{:02}-{:02}T08:00:00Z", 6 + index / 28, 1 + index % 28),
            Venue::Group,
        );
    }

    soul_graph::rebuild(&mut store).expect("rebuild");
    let strength = edge_strength(&store, acquaintance);

    assert_eq!(strength.band, SupportedBand::Weak);
    assert_eq!(strength.interaction_count, 50);
    assert_eq!(strength.active_day_count, 50);
    assert_eq!(strength.direct_out_count + strength.direct_in_count, 0);
    assert_eq!(strength.last_direct_contact_utc, None);
    assert!(edge_types(&store, acquaintance).contains(&TieType::GroupOnly));
}

/// One `as_of` for the whole store, so a tie nobody has touched in years is
/// demoted by evidence that has nothing to do with it.
#[test]
fn one_store_wide_as_of_demotes_the_dormant_not_the_active() {
    let dir = tempfile::tempdir().expect("temp dir");
    let dormant = peer(3);
    let active = peer(4);
    let mut store = open(dir.path(), &[dormant, active]);
    reciprocal_direct_history(&mut store, dormant, 2019, 3);
    observe(
        &mut store,
        active,
        "recent",
        Direction::Incoming,
        "2026-08-24T12:00:00Z",
        Venue::Direct,
    );

    soul_graph::rebuild(&mut store).expect("rebuild");
    let stale = edge_strength(&store, dormant);
    assert_eq!(stale.band, SupportedBand::Weak);
    assert_eq!(
        stale.as_of_utc.as_ref().map(Timestamp::as_str),
        Some("2026-08-24T12:00:00Z"),
        "every peer is scored against the newest row in the store",
    );
    assert!(
        stale.silent_days > WEAK_AFTER_SILENT_DAYS,
        "silence is measured against the store, not against this peer's own last message",
    );

    // The same history in a store that holds nothing newer is the documented
    // default: `as_of` is that peer's own newest row, so nothing is dormant.
    let alone_dir = tempfile::tempdir().expect("temp dir");
    let mut alone = open(alone_dir.path(), &[dormant]);
    reciprocal_direct_history(&mut alone, dormant, 2019, 3);
    soul_graph::rebuild(&mut alone).expect("rebuild");
    let held = edge_strength(&alone, dormant);
    assert_eq!(held.band, SupportedBand::Strong);
    assert_eq!(held.silent_days, 0);
}

/// The recency clock reads every venue: a group message the day before the
/// store's newest row means this person has not gone quiet.
#[test]
fn a_group_message_yesterday_stops_the_dormancy_step_at_the_store() {
    let dir = tempfile::tempdir().expect("temp dir");
    let friend = peer(5);
    let elsewhere = peer(6);
    let mut store = open(dir.path(), &[friend, elsewhere]);
    reciprocal_direct_history(&mut store, friend, 2025, 1);
    // Long enough after the last private word that the private history alone
    // would be past the floor, and one day before the newest row in the store.
    let group_ping = HISTORY_END + WEAK_AFTER_SILENT_DAYS * DAY;
    observe(
        &mut store,
        friend,
        "big-room",
        Direction::Incoming,
        &rfc3339_utc(group_ping),
        Venue::Group,
    );
    observe(
        &mut store,
        elsewhere,
        "recent",
        Direction::Outgoing,
        &rfc3339_utc(group_ping + DAY),
        Venue::Direct,
    );

    soul_graph::rebuild(&mut store).expect("rebuild");
    let strength = edge_strength(&store, friend);

    assert_eq!(strength.band, SupportedBand::Strong);
    assert_eq!(strength.silent_days, 1);
    assert_eq!(
        strength
            .last_direct_contact_utc
            .as_ref()
            .map(Timestamp::as_str),
        Some("2025-01-06T09:05:00Z"),
        "the private conversation is still where it was, and the edge says so",
    );
}

/// The anchor the silences below are measured from is the date it says it is,
/// so a reader can check the arithmetic without running it.
#[test]
fn the_history_anchor_is_the_date_it_claims_to_be() {
    assert_eq!(rfc3339_utc(HISTORY_END), "2025-01-06T09:05:00Z");
}

/// The demotion edges are closed intervals, and they survive the trip from
/// stored RFC 3339 strings through Unix seconds into whole days.
///
/// Both silences cross a month boundary and the longer one crosses a year, so
/// this is calendar arithmetic and not just seconds divided by 86 400.
#[test]
fn demotion_edges_are_closed_at_the_store() {
    // A private history ending at `HISTORY_END`, and a second peer whose
    // single row puts the store's `as_of` an exact number of days later, at
    // the same time of day.
    let band_after = |silent_days: i64| {
        let dir = tempfile::tempdir().expect("temp dir");
        let held = peer(7);
        let elsewhere = peer(8);
        let mut store = open(dir.path(), &[held, elsewhere]);
        reciprocal_direct_history(&mut store, held, 2025, 1);
        observe(
            &mut store,
            elsewhere,
            "recent",
            Direction::Outgoing,
            &rfc3339_utc(HISTORY_END + silent_days * DAY),
            Venue::Direct,
        );
        soul_graph::rebuild(&mut store).expect("rebuild");
        let strength = edge_strength(&store, held);
        (strength.band, strength.silent_days)
    };

    assert_eq!(
        band_after(DEMOTE_ONE_BAND_DAYS - 1),
        (SupportedBand::Strong, DEMOTE_ONE_BAND_DAYS - 1),
    );
    assert_eq!(
        band_after(DEMOTE_ONE_BAND_DAYS),
        (SupportedBand::Moderate, DEMOTE_ONE_BAND_DAYS),
    );
    assert_eq!(
        band_after(WEAK_AFTER_SILENT_DAYS - 1),
        (SupportedBand::Moderate, WEAK_AFTER_SILENT_DAYS - 1),
    );
    assert_eq!(
        band_after(WEAK_AFTER_SILENT_DAYS),
        (SupportedBand::Weak, WEAK_AFTER_SILENT_DAYS),
    );
}

/// The split counts are persisted rather than left for a reader to re-derive,
/// and they still add up after the round trip through the store.
#[test]
fn split_counts_round_trip_through_the_store() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mixed = peer(9);
    let mut store = open(dir.path(), &[mixed]);
    reciprocal_direct_history(&mut store, mixed, 2026, 8);
    observe(
        &mut store,
        mixed,
        "big-room",
        Direction::Outgoing,
        "2026-08-07T18:00:00Z",
        Venue::Group,
    );
    observe(
        &mut store,
        mixed,
        "big-room",
        Direction::Incoming,
        "2026-08-08T18:00:00Z",
        Venue::Group,
    );
    observe(
        &mut store,
        mixed,
        "big-room",
        Direction::Incoming,
        "2026-08-09T18:00:00Z",
        Venue::Group,
    );

    soul_graph::rebuild(&mut store).expect("rebuild");
    let strength = edge_strength(&store, mixed);

    assert_eq!(
        strength.direct_out_count
            + strength.direct_in_count
            + strength.group_out_count
            + strength.group_in_count,
        strength.interaction_count,
    );
    assert_eq!(
        strength.direct_out_count + strength.group_out_count,
        strength.outgoing_count,
    );
    assert_eq!(
        strength.direct_in_count + strength.group_in_count,
        strength.incoming_count,
    );
    assert_eq!(
        (strength.direct_out_count, strength.direct_in_count),
        (6, 6)
    );
    assert_eq!((strength.group_out_count, strength.group_in_count), (1, 2));
    assert!(strength.direct_active_day_count <= strength.active_day_count);
    assert_eq!(
        (strength.direct_active_day_count, strength.active_day_count),
        (6, 9)
    );
    assert_eq!(
        strength
            .last_direct_contact_utc
            .as_ref()
            .map(Timestamp::as_str),
        Some("2026-08-06T09:05:00Z"),
    );
    assert_eq!(
        strength.last_contact_utc.as_str(),
        "2026-08-09T18:00:00Z",
        "the display instant is the stored string, not a re-rendered one",
    );
    assert_eq!(strength.algorithm_id, "T4D");
    assert_eq!(
        strength.as_of_utc.as_ref().map(Timestamp::as_str),
        Some("2026-08-09T18:00:00Z"),
    );
    assert_eq!(strength.silent_days, 0);
    assert_eq!(strength.machine_band, Some(strength.band));
    assert_eq!(strength.locked_by_user, None);
    assert_eq!(strength.user_band, None);
}

/// An edge written before the rule landed still loads, and the next rebuild
/// replaces it with one that carries the split.
#[test]
fn a_pre_wiring_edge_still_loads_and_the_next_rebuild_upgrades_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let old_friend = peer(10);
    let mut store = open(dir.path(), &[old_friend]);
    let relationship_id = Uuid::now_v7();
    let evidence_id = Uuid::now_v7();
    let interaction = InteractionRef::new(
        Uuid::now_v7(),
        owner(),
        old_friend,
        conversation_ref("t4d-band", "legacy"),
        Direction::Outgoing,
        Timestamp::new("2026-08-01T09:00:00Z"),
        Venue::Direct,
    );
    store
        .put_evidence(interaction_evidence(
            evidence_id,
            Subject::Mixed,
            &interaction,
        ))
        .expect("evidence");
    store
        .put_relationship(SoulRelationship {
            schema_version: SchemaVersion,
            relationship_id,
            from_contact_id: owner(),
            to_contact_id: old_friend,
            types: Some(vec![json!("direct"), json!("one_sided")]),
            // Exactly the object the pre-wiring build wrote: no split counts,
            // no `as_of`, no algorithm id.
            tie_strength: Some(json!({
                "band": "moderate",
                "interaction_count": 4,
                "outgoing_count": 4,
                "incoming_count": 0,
                "conversation_count": 1,
                "active_day_count": 2,
                "first_contact_utc": "2026-07-30T09:00:00Z",
                "last_contact_utc": "2026-07-31T09:00:00Z",
            })),
            evidence_ids: vec![evidence_id],
            egress_scope: Some(EgressScope::LocalOnly),
        })
        .expect("legacy edge");

    let legacy = edge_strength(&store, old_friend);
    assert_eq!(legacy.band, SupportedBand::Moderate);
    assert_eq!(legacy.algorithm_id, "");
    assert_eq!(legacy.as_of_utc, None);
    assert_eq!(legacy.direct_out_count, 0);

    soul_graph::rebuild(&mut store).expect("rebuild");
    let upgraded = edge_strength(&store, old_friend);
    assert_eq!(upgraded.algorithm_id, "T4D");
    assert_eq!(upgraded.direct_out_count, 1);
    assert_eq!(upgraded.interaction_count, 1);
    assert_eq!(upgraded.band, SupportedBand::Weak);
    assert_eq!(
        store.list_relationships().expect("edges").len(),
        1,
        "the rebuild updates the edge it found rather than growing a second one",
    );
}

/// Evidence whose instant cannot be read is damaged, and damaged evidence
/// fails the rebuild instead of being folded in under a guess.
#[test]
fn a_broken_timestamp_fails_the_rebuild_and_names_the_row() {
    let dir = tempfile::tempdir().expect("temp dir");
    let unlucky = peer(11);
    let mut store = open(dir.path(), &[unlucky]);
    let broken_id = Uuid::now_v7();
    let interaction = InteractionRef::new(
        Uuid::now_v7(),
        owner(),
        unlucky,
        conversation_ref("t4d-band", "broken"),
        Direction::Incoming,
        Timestamp::new("last tuesday"),
        Venue::Direct,
    );
    store
        .put_evidence(interaction_evidence(
            broken_id,
            Subject::ThirdParty,
            &interaction,
        ))
        .expect("evidence");

    match soul_graph::rebuild(&mut store) {
        Err(GraphError::UnreadableInteraction { evidence_id }) => {
            assert_eq!(evidence_id, broken_id)
        }
        other => panic!("expected the rebuild to name the unreadable row, got {other:?}"),
    }
    assert!(
        store.list_relationships().expect("edges").is_empty(),
        "nothing is written when the evidence cannot be read",
    );
}

/// A rebuild updates what the machine thinks; it does not un-review an
/// inference the user has already ruled on.
#[test]
fn a_rebuild_keeps_the_verdict_the_user_gave() {
    use soul_schema::inference::UserVerdict;

    let dir = tempfile::tempdir().expect("temp dir");
    let reviewed = peer(12);
    let mut store = open(dir.path(), &[reviewed]);
    reciprocal_direct_history(&mut store, reviewed, 2026, 8);
    let build = soul_graph::rebuild(&mut store).expect("rebuild");
    let inference_id = build.inferences_written[0];

    for verdict in [
        UserVerdict::Corrected,
        UserVerdict::Accepted,
        UserVerdict::Rejected,
    ] {
        let mut inference = store.get_inference(inference_id).expect("inference");
        inference.user_verdict = Some(verdict);
        store.put_inference(inference).expect("verdict");

        observe(
            &mut store,
            reviewed,
            "direct",
            Direction::Incoming,
            "2026-08-07T09:05:00Z",
            Venue::Direct,
        );
        soul_graph::rebuild(&mut store).expect("rebuild");

        assert_eq!(
            store
                .get_inference(inference_id)
                .expect("inference")
                .user_verdict,
            Some(verdict),
        );
    }
}

/// A band the user chose survives the rebuild, and the machine's own reading
/// is written down beside it rather than over it.
#[test]
fn a_locked_edge_keeps_the_users_band_and_records_the_machines() {
    let dir = tempfile::tempdir().expect("temp dir");
    let corrected = peer(13);
    let mut store = open(dir.path(), &[corrected]);
    reciprocal_direct_history(&mut store, corrected, 2026, 8);
    let build = soul_graph::rebuild(&mut store).expect("rebuild");
    let relationship_id = build.edges_written[0];
    assert_eq!(edge_strength(&store, corrected).band, SupportedBand::Strong);

    let mut edge = store.get_relationship(relationship_id).expect("edge");
    let mut strength = edge_strength(&store, corrected);
    strength.band = SupportedBand::Moderate;
    strength.locked_by_user = Some(true);
    strength.user_band = Some(SupportedBand::Moderate);
    edge.tie_strength = Some(serde_json::to_value(&strength).expect("strength"));
    store.put_relationship(edge).expect("locked edge");

    observe(
        &mut store,
        corrected,
        "direct",
        Direction::Outgoing,
        "2026-08-07T09:00:00Z",
        Venue::Direct,
    );
    soul_graph::rebuild(&mut store).expect("rebuild");

    let after = edge_strength(&store, corrected);
    assert_eq!(after.band, SupportedBand::Moderate, "the user's band holds");
    assert_eq!(after.user_band, Some(SupportedBand::Moderate));
    assert_eq!(after.locked_by_user, Some(true));
    assert_eq!(
        after.machine_band,
        Some(SupportedBand::Strong),
        "the machine keeps saying what it thinks, next to the band in force",
    );
    assert_eq!(
        after.interaction_count, 13,
        "the lock is on the band, not on the counts",
    );
}

/// Every case the frozen rule holds itself to, replayed through the store.
///
/// `soul-algo-tie` runs these logs in memory against its own `as_of`. This
/// writes the same rows into SQLCipher, rebuilds, reads the relationship back,
/// and asks for the same answer — so an adapter that dropped a venue, rounded
/// an instant or let `as_of` go per-peer is caught by the whole table at once
/// rather than by whichever case somebody thought to copy by hand.
#[test]
fn every_frozen_fixture_bands_the_same_through_the_store() {
    use soul_algo_tie::testing::{all, Fixture, AS_OF_2026_08_24};

    // `empty` has nothing to write, so there is no edge to read back.
    let fixtures: Vec<Fixture> = all()
        .into_iter()
        .filter(|fixture| !fixture.log.is_empty())
        .collect();
    assert!(
        fixtures.len() > 10,
        "the frozen fixture table has all but disappeared: {} case(s)",
        fixtures.len(),
    );

    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = staged(dir.path(), &fixtures);
    soul_graph::rebuild(&mut store).expect("rebuild");

    for fixture in &fixtures {
        let name = fixture.name;
        let expected = soul_algo_tie::score(fixture.peer_id, &fixture.log, fixture.as_of);
        let stored = edge_strength(&store, fixture_peer(fixture.peer_id));

        assert_eq!(
            stored.band,
            match expected.band {
                soul_algo_tie::Band::Weak => SupportedBand::Weak,
                soul_algo_tie::Band::Moderate => SupportedBand::Moderate,
                soul_algo_tie::Band::Strong => SupportedBand::Strong,
            },
            "{name}: the store banded this differently from the rule",
        );
        assert_eq!(stored.algorithm_id, expected.algorithm_id, "{name}: rule");
        assert_eq!(
            (
                stored.direct_out_count,
                stored.direct_in_count,
                stored.group_out_count,
                stored.group_in_count,
            ),
            (
                expected.direct_out_count,
                expected.direct_in_count,
                expected.group_out_count,
                expected.group_in_count,
            ),
            "{name}: the split the band was decided on",
        );
        assert_eq!(
            (stored.active_day_count, stored.direct_active_day_count),
            (expected.active_day_count, expected.direct_active_day_count),
            "{name}: days",
        );
        assert_eq!(
            stored.interaction_count, expected.interaction_count,
            "{name}: rows",
        );
        assert_eq!(stored.silent_days, expected.silent_days, "{name}: silence");
        assert_eq!(
            stored.as_of_utc.as_ref().map(Timestamp::as_str),
            Some(rfc3339_utc(AS_OF_2026_08_24).as_str()),
            "{name}: one store-wide as_of",
        );
    }
}

/// AC-28. The fixture the round was called over, through the product path.
///
/// A project channel the two of them are both loud in, plus exactly one
/// private message each way. The old rule counted every venue and called that
/// a close tie; the point of absorbing T4D is that the store no longer does.
/// The split the band was decided on is on the edge, so a reader can recount
/// it, and the assertion is against the *other* rule as well: the same rows
/// under `T4` are Strong, which is the defect this row exists to keep dead.
#[test]
fn the_decisive_fixture_is_not_a_close_tie_at_the_store() {
    use soul_algo_tie::testing::group_heavy_plus_one_direct_each_way;
    use soul_algo_tie::TieAlgo;

    let fixture = group_heavy_plus_one_direct_each_way();
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = staged(dir.path(), std::slice::from_ref(&fixture));
    soul_graph::rebuild(&mut store).expect("rebuild");

    let strength = edge_strength(&store, fixture_peer(fixture.peer_id));
    assert_ne!(strength.band, SupportedBand::Strong);
    assert_eq!(strength.band, SupportedBand::Weak);

    // Every number the band rests on, written down rather than left to be
    // re-derived from the evidence.
    assert_eq!(
        (strength.direct_out_count, strength.direct_in_count),
        (1, 1)
    );
    assert_eq!(strength.group_out_count + strength.group_in_count, 30);
    assert_eq!(strength.direct_active_day_count, 1);
    assert_eq!(strength.algorithm_id, "T4D");

    // And the reason this is a gate rather than a preference: counting every
    // venue, which is what the rule this replaced did, makes the same rows a
    // close tie. Reverting the decision has to turn this line red.
    assert_eq!(
        TieAlgo::T4
            .score(fixture.peer_id, &fixture.log, fixture.as_of)
            .band,
        soul_algo_tie::Band::Strong,
        "the venue-blind rule still says Strong here; that is the defect",
    );
}

/// AC-29. Absorbing the new rule did not cost the case the old one got right.
///
/// `lilei_12` is the tie Goal 1 shipped as Strong, and it still is. The second
/// half is the way back up: the same group-heavy shape as AC-28 with one more
/// private exchange than the Moderate bar asks for is Moderate, so the rule
/// moved the bar onto the one-to-one count rather than refusing group-heavy
/// ties outright.
#[test]
fn the_anchor_fixture_is_untouched_and_the_way_back_up_is_open() {
    use soul_algo_tie::testing::{group_heavy_plus_three_directs, lilei_12};

    let anchor = lilei_12();
    let self_healed = group_heavy_plus_three_directs();
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = staged(dir.path(), &[anchor.clone(), self_healed.clone()]);
    soul_graph::rebuild(&mut store).expect("rebuild");

    assert_eq!(
        edge_strength(&store, fixture_peer(anchor.peer_id)).band,
        SupportedBand::Strong,
        "the tie Goal 1 shipped as close is still close",
    );
    assert_eq!(
        edge_strength(&store, fixture_peer(self_healed.peer_id)).band,
        SupportedBand::Moderate,
        "one private exchange over the bar, and the group traffic counts for something",
    );
}

/// AC-30. One store, one `as_of`, and a friendship that ended in 2019.
///
/// The dormant tie is Strong on its counts and Weak on the calendar, and what
/// makes the calendar bite is a row that has nothing to do with it. Both the
/// instant everything was judged against and the silence it produced are on
/// the edge, so the demotion can be checked by hand. Giving each peer its own
/// newest row as `as_of` would make this line red twice over: the dormant tie
/// would come back Strong, and its silence would read zero.
#[test]
fn a_store_holding_an_active_tie_and_a_2019_one_uses_a_single_as_of() {
    use soul_algo_tie::testing::{dormant_2019, lilei_12, AS_OF_2026_08_24};

    let dormant = dormant_2019();
    let active = lilei_12();
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = staged(dir.path(), &[dormant.clone(), active.clone()]);
    soul_graph::rebuild(&mut store).expect("rebuild");

    let anchor = rfc3339_utc(AS_OF_2026_08_24);
    let stale = edge_strength(&store, fixture_peer(dormant.peer_id));
    let current = edge_strength(&store, fixture_peer(active.peer_id));

    assert_eq!(stale.band, SupportedBand::Weak);
    assert_eq!(current.band, SupportedBand::Strong);
    for (name, strength) in [("dormant", &stale), ("active", &current)] {
        assert_eq!(
            strength.as_of_utc.as_ref().map(Timestamp::as_str),
            Some(anchor.as_str()),
            "{name}: both edges were judged at the same instant, and say so",
        );
    }
    assert!(
        stale.silent_days > WEAK_AFTER_SILENT_DAYS,
        "a tie last touched in 2019 records the years of silence, not zero",
    );
    assert!(
        current.silent_days < DEMOTE_ONE_BAND_DAYS,
        "and the recent one records days",
    );
}

/// The band is opaque to this crate now: no threshold, no comparison, no
/// second definition of a number the algorithm crate owns.
///
/// The numbers to look for are read from the frozen crate rather than written
/// down, so this test cannot become the third copy of the thing it forbids.
#[test]
fn the_graph_source_holds_no_second_threshold() {
    use soul_algo_tie::constants::{
        MODERATE_MIN_INTERACTIONS, STRONG_MIN_ACTIVE_DAYS, STRONG_MIN_INTERACTIONS,
    };

    const SOURCES: [(&str, &str); 3] = [
        ("build.rs", include_str!("../src/build.rs")),
        ("model.rs", include_str!("../src/model.rs")),
        ("view.rs", include_str!("../src/view.rs")),
    ];

    let mut forbidden: Vec<String> = [
        "MODERATE_MIN",
        "STRONG_MIN",
        "MIN_INTERACTIONS",
        "MIN_ACTIVE_DAYS",
    ]
    .iter()
    .map(|name| (*name).to_owned())
    .collect();
    for value in [
        MODERATE_MIN_INTERACTIONS as i64,
        STRONG_MIN_INTERACTIONS as i64,
        STRONG_MIN_ACTIVE_DAYS as i64,
        DEMOTE_ONE_BAND_DAYS,
        WEAK_AFTER_SILENT_DAYS,
    ] {
        forbidden.push(format!(">= {value}"));
        forbidden.push(format!(">={value}"));
    }

    for (name, source) in SOURCES {
        for needle in &forbidden {
            assert!(
                !source.contains(needle.as_str()),
                "{name} holds `{needle}`; the band belongs to the frozen rule alone",
            );
        }
    }
}
