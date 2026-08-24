//! `as_of` comes from the caller, and it comes from the whole store.
//!
//! Two promises, both easy to break by accident:
//!
//! 1. **No clock.** Nothing in the crate reads `SystemTime`, so a rebuild is
//!    reproducible and a test cannot rot into a failure a year from now. The
//!    only way in is the `as_of_unix` parameter.
//! 2. **Store-level, not peer-level.** R2-SYNTHESIS §冻结边界: one value per
//!    rebuild, for the whole store, supplied by the caller. Taking it per peer
//!    would make every dormant tie look current, because a dormant peer's own
//!    newest message is, by definition, the last thing that happened to that
//!    peer.

use soul_algo_tie::testing::{self, at, AS_OF_2026_08_24, DAY, TODAY_MIDNIGHT};
use soul_algo_tie::{
    age_days, as_of_max, civil_from_epoch_day, epoch_day, score_both, Band, TieAlgo,
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
    // exactly `d` whole days old as of the anchor, so an expected band can be
    // read off the fixture definition without arithmetic.
    for days_ago in [0, 1, 89, 90, 179, 180, 300, 359, 360, 2_632] {
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
    // Two runs, byte-identical results, with no way for a clock to get in.
    // Running the same call twice is a weak test on its own — what makes it
    // meaningful is that `as_of` is the only temporal input, which the type
    // signature already guarantees.
    for f in testing::all() {
        assert_eq!(
            score_both(f.peer_id, &f.log, f.as_of),
            score_both(f.peer_id, &f.log, f.as_of),
            "{}",
            f.name
        );
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
fn the_store_maximum_is_the_other_legal_as_of_and_the_shift_it_causes_is_recorded() {
    // `as_of = max(occurred_at)` over the whole store. The newest row in the
    // merged store is a message from the flood fixture, roughly ten hours
    // before the anchor, so every age drops by exactly one day.
    let store = testing::store();
    let as_of = as_of_max(&store).expect("the store is not empty");
    assert!(as_of < AS_OF_2026_08_24);
    assert_eq!(as_of, at(1, 0) + 999 * 50);

    // Every fixture keeps its band except the ones that were sitting exactly on
    // a threshold — which is not a defect, it is the thresholds being sharp.
    // Recording them here is cheaper than discovering them later.
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
            ("quiet_180_days", "T4D", Band::Moderate, Band::Strong),
            // One day younger, so the cut-off has not fired yet and only the
            // single-rung demotion applies.
            ("dormant_360_days", "T4", Band::Weak, Band::Moderate),
            ("dormant_360_days", "T4D", Band::Weak, Band::Moderate),
        ]
    );
}

#[test]
fn a_peer_local_as_of_would_hide_every_dormant_tie() {
    // The trap, written down. Scoring the 2019 fixture against its own newest
    // message makes both rules call it Strong, because relative to itself
    // nothing is old. This is why `as_of` is a parameter of the rebuild and not
    // something a rule may derive from the rows it was handed.
    let f = testing::dormant_2019();
    let peer_local = as_of_max(&f.log).expect("the fixture is not empty");
    for algo in TieAlgo::ALL {
        assert_eq!(
            algo.score(f.peer_id, &f.log, peer_local).band,
            Band::Strong,
            "{}",
            algo.id()
        );
        assert_eq!(
            algo.score(f.peer_id, &f.log, f.as_of).band,
            Band::Weak,
            "{}",
            algo.id()
        );
    }
}

#[test]
fn scoring_the_whole_store_at_once_matches_scoring_each_peer() {
    let store = testing::store();
    let as_of = as_of_max(&store).expect("the store is not empty");
    for algo in TieAlgo::ALL {
        let whole = algo.score_ego_network(&store, as_of);
        // Every fixture with evidence appears exactly once.
        assert_eq!(whole.len(), testing::all().len() - 1);
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
    for score in score_both(8, &log, 0) {
        assert_eq!(score.active_day_count, 6);
        assert_eq!(score.direct_active_day_count, 6);
        assert_eq!(score.first_contact_unix, -DAY * 6);
        assert_eq!(score.last_contact_unix, -DAY);
        assert_eq!(score.last_direct_contact_unix, -DAY);
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
    for score in score_both(7, &log, AS_OF_2026_08_24) {
        assert_eq!(score.active_day_count, 6);
        assert_eq!(score.direct_active_day_count, 6);
        assert_eq!(score.band, Band::Strong);
    }
}

#[test]
fn a_last_direct_contact_before_the_epoch_is_still_reported() {
    // `last_direct_contact_unix` uses 0 to mean "never", so a real exchange at
    // exactly the epoch second, or before it, must not read as "never".
    let log = vec![
        testing::group(1, true, -DAY * 10, 1),
        testing::direct(1, false, -DAY * 20, 1),
    ];
    let score = TieAlgo::T4D.score(1, &log, 0);
    assert_eq!(score.last_direct_contact_unix, -DAY * 20);
    assert_eq!(score.last_contact_unix, -DAY * 10);
}
