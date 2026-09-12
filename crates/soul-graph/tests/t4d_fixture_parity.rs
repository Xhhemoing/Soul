//! The frozen rule's own fixtures, judged through the product.
//!
//! `crates/soul-algo-tie/src/testing` holds the cases the tie rule was chosen
//! against, and the algorithm crate already scores them. What that cannot show
//! is that the path a user actually travels — evidence rows in the encrypted
//! store, one [`soul_graph::rebuild`], the band read back off the edge —
//! reaches the same verdict. A rebuild that dropped a venue flag, rounded a
//! day, or scored each peer against its own newest row would leave every test
//! in the algorithm crate green.
//!
//! So the fixtures are *imported* rather than re-derived: the expected band on
//! every edge is whatever [`soul_algo_tie::score`] says about that peer under
//! the same store-wide `as_of`, and no threshold is written down here. A
//! second copy of 3 / 10 / 3 / 180 / 360 in this file would be the very thing
//! the ban on a second threshold literal exists to prevent.
//!
//! All the fixtures share one store, which is also what makes the `as_of`
//! discipline checkable: they were written against one anchor instant, and one
//! row placed at that instant is what makes the whole-store `max(occurred_at)`
//! land on it.

use uuid::Uuid;

use soul_algo_tie::testing::{self, AS_OF_2026_08_24};
use soul_algo_tie::{Band, Interaction};
use soul_graph::interaction::{conversation_ref, interaction_evidence, InteractionRef};
use soul_graph::{Direction, Venue};
use soul_schema::common::{SchemaVersion, Subject, SupportedBand, Timestamp};
use soul_schema::contact::{ContactClass, SoulContact};
use soul_schema::memory::ForgetState;
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::{GraphStore, ProfileStore};

const SEED: &str = "t4d fixture parity";

/// The peer whose single row sits on the anchor instant.
///
/// Outside the fixtures' own range of peer ids, and inbound so it is somebody
/// the user heard from rather than a message the user is credited with.
const ANCHOR_PEER: u64 = 1_000;

fn owner() -> Uuid {
    Uuid::from_u128(0x0192_a1b2_c3d4_7e5f_8a9b_0c1d_2e3f_5000)
}

/// A stable contact id per fixture peer. The number is scratch on both sides —
/// the rule interns its own — so the mapping only has to be one to one.
fn contact_id(peer_id: u64) -> Uuid {
    Uuid::from_u128(0x0192_a1b2_c3d4_7e5f_8a9b_0c1d_2e3f_5000 + u128::from(peer_id) + 1)
}

fn contact(id: Uuid, contact_class: ContactClass) -> SoulContact {
    SoulContact {
        schema_version: SchemaVersion,
        contact_id: id,
        contact_class,
        display_label_ref: None,
        identifiers: None,
        forget_state: ForgetState::Active,
    }
}

/// Every fixture's log, plus the anchor row, in one list.
fn corpus() -> Vec<Interaction> {
    let mut log: Vec<Interaction> = testing::all()
        .into_iter()
        .flat_map(|fixture| fixture.log)
        .collect();
    log.push(testing::direct(ANCHOR_PEER, false, AS_OF_2026_08_24, 9_999));
    log
}

/// Write the whole corpus into a real store, one evidence row per observation.
fn store_with_corpus(dir: &std::path::Path) -> SqlCipherStore {
    let mut store =
        SqlCipherStore::open(dir.join("soul.db"), &TestKeyProvider::from_seed(SEED)).expect("open");
    store
        .put_contact(contact(owner(), ContactClass::Owner))
        .expect("owner");

    let log = corpus();
    let mut seen: Vec<u64> = log.iter().map(|row| row.peer_id).collect();
    seen.sort_unstable();
    seen.dedup();
    for peer_id in seen {
        store
            .put_contact(contact(contact_id(peer_id), ContactClass::ThirdParty))
            .expect("peer");
    }

    for row in &log {
        let observation = InteractionRef::new(
            Uuid::now_v7(),
            owner(),
            contact_id(row.peer_id),
            conversation_ref("t4d-fixture", &row.conversation_id.to_string()),
            match row.outgoing {
                true => Direction::Outgoing,
                false => Direction::Incoming,
            },
            Timestamp::new(soul_policy::clock::rfc3339_utc(row.occurred_at_unix)),
            match row.venue_direct {
                true => Venue::Direct,
                false => Venue::Group,
            },
        );
        let subject = match row.outgoing {
            true => Subject::Mixed,
            false => Subject::ThirdParty,
        };
        store
            .put_evidence(interaction_evidence(Uuid::now_v7(), subject, &observation))
            .expect("evidence");
    }
    store
}

fn band_of(band: Band) -> SupportedBand {
    match band {
        Band::Weak => SupportedBand::Weak,
        Band::Moderate => SupportedBand::Moderate,
        Band::Strong => SupportedBand::Strong,
    }
}

/// Every fixture the tie rule was chosen against, scored twice: once by the
/// frozen crate over the fixture's own log, once by the product over rows in
/// the encrypted store. The two have to agree on the band and on every count
/// the band was decided from.
#[test]
fn the_product_path_reaches_the_same_verdict_as_the_frozen_rule() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = store_with_corpus(dir.path());
    soul_graph::rebuild(&mut store).expect("rebuild");
    let graph = soul_graph::load(&store).expect("load");

    let log = corpus();
    let mut checked = 0;
    let mut bands = std::collections::BTreeSet::new();
    for fixture in testing::all() {
        let expected = soul_algo_tie::score(fixture.peer_id, &log, AS_OF_2026_08_24);
        let edges = graph.edges_for(contact_id(fixture.peer_id));
        if expected.interaction_count == 0 {
            assert!(
                edges.is_empty(),
                "{} has nothing behind it and must not produce an edge",
                fixture.name,
            );
            continue;
        }
        assert_eq!(edges.len(), 1, "one edge for {}", fixture.name);
        let held = &edges[0].tie_strength;

        assert_eq!(
            held.band,
            band_of(expected.band),
            "{} ({}) is banded differently by the store than by the rule",
            fixture.name,
            fixture.summary,
        );
        assert_eq!(held.algorithm_id, expected.algorithm_id, "{}", fixture.name);
        assert_eq!(
            held.interaction_count, expected.interaction_count,
            "{}",
            fixture.name
        );
        assert_eq!(
            held.direct_out_count, expected.direct_out_count,
            "{}",
            fixture.name
        );
        assert_eq!(
            held.direct_in_count, expected.direct_in_count,
            "{}",
            fixture.name
        );
        assert_eq!(
            held.group_out_count, expected.group_out_count,
            "{}",
            fixture.name
        );
        assert_eq!(
            held.group_in_count, expected.group_in_count,
            "{}",
            fixture.name
        );
        assert_eq!(
            held.active_day_count, expected.active_day_count,
            "{}",
            fixture.name
        );
        assert_eq!(
            held.direct_active_day_count, expected.direct_active_day_count,
            "{}",
            fixture.name
        );
        assert_eq!(held.silent_days, expected.silent_days, "{}", fixture.name);
        assert_eq!(
            held.conversation_count, expected.conversation_count,
            "{}",
            fixture.name
        );
        bands.insert(format!("{:?}", held.band));
        checked += 1;
    }
    assert!(
        checked >= 20,
        "only {checked} fixtures reached the store; the corpus went missing",
    );
    // Agreement is only worth something if the two sides disagree somewhere in
    // principle. A corpus that came back all one band would pass every
    // assertion above while proving nothing about the gates.
    assert_eq!(
        bands.len(),
        3,
        "the corpus stopped exercising all three bands: {bands:?}",
    );
}

/// One `as_of` for the whole library, and it is the newest row in it.
///
/// Read off the edges rather than out of the rebuild: this is the value a user
/// can see beside the band, and it is the same string on the 2019 tie as on
/// the one from yesterday. Per-peer recency would give every dormant edge its
/// own anchor and quietly make all of them look current.
#[test]
fn every_edge_in_the_store_is_scored_against_the_same_instant() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = store_with_corpus(dir.path());
    soul_graph::rebuild(&mut store).expect("rebuild");
    let graph = soul_graph::load(&store).expect("load");

    let anchor = Timestamp::new(soul_policy::clock::rfc3339_utc(AS_OF_2026_08_24));
    assert!(graph.edges.len() >= 20);
    for edge in &graph.edges {
        assert_eq!(
            edge.tie_strength.as_of_utc.as_ref(),
            Some(&anchor),
            "an edge was scored against its own newest row: {:?}",
            edge.tie_strength.as_of_utc,
        );
    }
}

/// AC-29: absorbing T4D must not re-band the anchor case, and the tie the
/// venue latch demotes has to be able to climb back out.
///
/// `lilei_12` is the Goal 1 acceptance fixture — twelve reciprocal one-to-one
/// exchanges over six days — and a rule that stopped calling it strong would
/// be a replacement rather than a refinement. `group_heavy_plus_three_directs`
/// is the self-heal path beside it: the same loud group channel that
/// `group_heavy_plus_one_direct_each_way` is refused for, plus the third
/// one-to-one message. One private exchange is what moves it, which is the
/// whole claim that the latch sits on the direct count rather than on a
/// dislike of group chats. Both are asserted here, in one test, on rows that
/// went through the encrypted store.
#[test]
fn the_anchor_tie_stays_strong_and_the_latched_tie_heals_at_three_directs() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = store_with_corpus(dir.path());
    soul_graph::rebuild(&mut store).expect("rebuild");
    let graph = soul_graph::load(&store).expect("load");

    let band = |fixture: testing::Fixture| {
        let edges = graph.edges_for(contact_id(fixture.peer_id));
        assert_eq!(edges.len(), 1, "one edge for {}", fixture.name);
        edges[0].tie_strength.band
    };

    assert_eq!(band(testing::lilei_12()), SupportedBand::Strong);
    assert_eq!(
        band(testing::group_heavy_plus_three_directs()),
        SupportedBand::Moderate,
        "the third one-to-one message is what brings the tie back",
    );
    // The pair it is only meaningful against: two private messages is still
    // under the bar, however loud the group is.
    assert_eq!(
        band(testing::group_heavy_plus_one_direct_each_way()),
        SupportedBand::Weak,
    );
}
