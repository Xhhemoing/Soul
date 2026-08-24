//! `as_of` comes from the data, and it comes from the whole store.
//!
//! Two separate promises, both easy to break by accident:
//!
//! 1. **No clock.** Nothing in the crate reads `SystemTime`, so a rebuild is
//!    reproducible and a test cannot rot into a failure a year from now. The
//!    only way in is the `as_of_unix` parameter.
//! 2. **Store-level, not peer-level.** `as_of = max(occurred_at)` over the
//!    whole store. Taking it per peer would make every dormant tie look
//!    current, because a dormant peer's own newest message is, by definition,
//!    the last thing that happened to that peer.

use soul_algo_tie::testing::{self, at, AS_OF_2026_08_24, DAY, TODAY_MIDNIGHT};
use soul_algo_tie::{
    age_days, as_of_max, civil_from_epoch_day, epoch_day, score_all, Band, TieAlgo,
};

#[test]
fn the_anchor_is_the_instant_the_brief_names() {
    // 2026-08-24T14:00:00Z, computed rather than looked up.
    assert_eq!(
        civil_from_epoch_day(epoch_day(AS_OF_2026_08_24)),
        (2026, 8, 24)
    );
    assert_eq!(AS_OF_2026_08_24.rem_euclid(DAY), 14 * 3_600);
    assert_eq!(AS_OF_2026_08_24, TODAY_MIDNIGHT + 14 * 3_600);
    assert_eq!(AS_OF_2026_08_24, 1_787_580_000);
    // 2026-01-01T00:00:00Z plus the 235 days to 24 August plus 14 hours.
    assert_eq!(AS_OF_2026_08_24, 1_767_225_600 + 235 * DAY + 14 * 3_600);
}

#[test]
fn a_fixture_events_age_is_exactly_the_days_ago_it_was_placed_at() {
    // The property the fixtures rely on: `at(d, h)` for any hour up to 14:00 is
    // exactly `d` whole days old as of the anchor, so a bucket can be read off
    // the fixture definition without arithmetic.
    for days_ago in [0, 1, 89, 90, 179, 180, 359, 360, 2_632] {
        for hour in 0..=14 {
            assert_eq!(
                age_days(at(days_ago, hour), AS_OF_2026_08_24),
                days_ago,
                "{days_ago} days ago at {hour} o'clock"
            );
        }
    }
}

#[test]
fn no_fixture_event_lies_in_the_future() {
    for f in testing::all() {
        for row in &f.log {
            assert!(
                row.occurred_at_unix <= f.as_of,
                "{} has evidence after its as_of",
                f.name
            );
        }
    }
}

#[test]
fn scoring_is_a_pure_function_of_the_evidence_and_the_as_of() {
    // The F15 red line, as a property: two runs, byte-identical results, with
    // no way for a clock to get in. Running the same call twice is a weak test
    // on its own — what makes it meaningful is that `as_of` is the only
    // temporal input, which the type signature already guarantees.
    for f in testing::all() {
        let first = score_all(f.peer_id, &f.log, f.as_of);
        let second = score_all(f.peer_id, &f.log, f.as_of);
        assert_eq!(first, second, "{}", f.name);
        for algo in TieAlgo::ALL {
            assert_eq!(
                algo.explain_zh(&algo.score(f.peer_id, &f.log, f.as_of)),
                algo.explain_zh(&algo.score(f.peer_id, &f.log, f.as_of)),
                "{}",
                f.name
            );
        }
    }
}

#[test]
fn the_store_maximum_is_the_other_legal_as_of_and_it_agrees() {
    // `as_of = max(occurred_at)` over the whole store, which is what fable-a's
    // CANDIDATE_SPEC freezes. The newest row in the merged store is a message
    // from the flood fixture, roughly ten hours before the anchor, so every
    // age drops by exactly one day.
    let store = testing::store();
    let as_of = as_of_max(&store).expect("the store is not empty");
    assert!(as_of < AS_OF_2026_08_24);
    assert_eq!(as_of, at(1, 0) + 999 * 50);

    // Every fixture keeps its band except the two that were sitting exactly on
    // a threshold — which is not a defect, it is the thresholds being sharp.
    // Recording them here is cheaper than discovering them in Round 3.
    let shifted: Vec<(&str, &str, Band, Band)> = testing::all()
        .iter()
        .flat_map(|f| {
            TieAlgo::ALL.into_iter().filter_map(move |algo| {
                let anchored = algo.score(f.peer_id, &f.log, f.as_of).band;
                let store_wide = algo.score(f.peer_id, &f.log, as_of).band;
                (anchored != store_wide).then_some((f.name, algo.id(), anchored, store_wide))
            })
        })
        .collect();
    assert_eq!(
        shifted,
        vec![
            // One day younger, so the demotion has not fired yet.
            ("quiet_180_days", "T4", Band::Moderate, Band::Strong),
            // One day younger, so the cut-off has not fired yet and only the
            // single-rung demotion applies.
            ("dormant_360_days", "T4", Band::Weak, Band::Moderate),
        ]
    );
}

#[test]
fn a_peer_local_as_of_would_hide_every_dormant_tie() {
    // The trap, written down. Scoring the 2019 fixture against its own newest
    // message makes both recency-aware rules call it Strong, because relative
    // to itself nothing is old. This is why `as_of` is a parameter of the
    // rebuild and not something a rule may derive from the rows it was handed.
    let f = testing::dormant_2019();
    let peer_local = as_of_max(&f.log).expect("the fixture is not empty");

    assert_eq!(
        [
            TieAlgo::T3R.score(f.peer_id, &f.log, peer_local).band,
            TieAlgo::T4.score(f.peer_id, &f.log, peer_local).band,
        ],
        [Band::Strong, Band::Strong]
    );
    assert_eq!(
        [
            TieAlgo::T3R.score(f.peer_id, &f.log, f.as_of).band,
            TieAlgo::T4.score(f.peer_id, &f.log, f.as_of).band,
        ],
        [Band::Weak, Band::Weak]
    );
}

#[test]
fn scoring_the_whole_store_at_once_matches_scoring_each_peer() {
    let store = testing::store();
    let as_of = as_of_max(&store).expect("the store is not empty");
    for algo in TieAlgo::ALL {
        let whole = algo.score_ego_network(&store, as_of);
        // Every fixture with evidence appears exactly once.
        assert_eq!(whole.len(), 15);
        for (peer_id, score) in whole {
            assert_eq!(score, algo.score(peer_id, &store, as_of));
        }
    }
}

#[test]
fn evidence_from_before_1970_does_not_collapse_into_one_day() {
    // Negative Unix seconds appear in imported archives with broken clocks.
    // Floor division keeps each of them on its own day; truncation would fold
    // the last day of 1969 into the first of 1970.
    let log: Vec<_> = (1..=6i64)
        .map(|i| testing::direct(8, i % 2 == 0, -DAY * i, 8))
        .collect();
    for score in score_all(8, &log, 0) {
        assert_eq!(score.active_day_count, 6);
        assert_eq!(score.first_contact_unix, -DAY * 6);
        assert_eq!(score.last_contact_unix, -DAY);
    }
}

#[test]
fn active_days_are_utc_and_do_not_follow_the_machine_clock() {
    // Six exchanges at 01:00 local time in UTC+8, on six consecutive local
    // days. In UTC they are 17:00 on the six preceding days, so the number of
    // distinct days is the same either way — the property that keeps a tie
    // scored on the user's laptop equal to the same tie scored in CI.
    let local_midnight_utc8 = at(20, 0) - 8 * 3_600;
    let log: Vec<_> = (0..6i64)
        .flat_map(|day| {
            [
                testing::direct(7, true, local_midnight_utc8 + DAY * day + 3_600, 7),
                testing::direct(7, false, local_midnight_utc8 + DAY * day + 2 * 3_600, 7),
            ]
        })
        .collect();
    for score in score_all(7, &log, AS_OF_2026_08_24) {
        assert_eq!(score.active_day_count, 6);
        assert_eq!(score.band, Band::Strong);
    }
}
