//! The oracle against an independent transcription of the shipped rule.
//!
//! R1-SYNTHESIS retired T0 as a specification but kept it as the oracle, and
//! this is the job it does. Round 3 split the shared [`Tally`] by venue — four
//! counts and two day sets where there used to be two counts and one — and a
//! restructure of the thing every rule counts with is exactly the kind of
//! change that could silently move the shipped band.
//!
//! The oracle below is `soul-graph::build::Tally` rewritten from the snapshot
//! in `.agent_workspace/context/impl/graph_build.rs`, keeping the two things
//! the port changed: RFC 3339 strings instead of integers, and "first ten
//! characters of the timestamp" instead of an epoch day. If the port is
//! faithful, the two agree on every input — including the ones where the
//! string and integer spellings of a UTC date could come apart.

use soul_algo_tie::testing::oracle::T0;
use soul_algo_tie::testing::DAY;
use soul_algo_tie::types::civil_from_epoch_day;
use soul_algo_tie::{epoch_day, Band, Interaction, Tally, TieAlgorithm};

use std::collections::BTreeSet;

const MODERATE_MIN_INTERACTIONS: u64 = 3;
const STRONG_MIN_INTERACTIONS: u64 = 10;
const STRONG_MIN_ACTIVE_DAYS: u64 = 3;

/// An instant as Goal 1 stores it: RFC 3339, normalised to `Z`.
fn rfc3339_utc(unix: i64) -> String {
    let (year, month, day) = civil_from_epoch_day(epoch_day(unix));
    let seconds_into_day = unix.rem_euclid(86_400);
    let (h, m, s) = (
        seconds_into_day / 3_600,
        (seconds_into_day % 3_600) / 60,
        seconds_into_day % 60,
    );
    format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}Z")
}

/// `graph_build::utc_date`, verbatim.
fn utc_date(timestamp: &str) -> String {
    timestamp.chars().take(10).collect()
}

/// `graph_build::Tally::band`, verbatim, over string timestamps.
fn goal1_band(peer_id: u64, interactions: &[Interaction]) -> Band {
    let mut outgoing = 0u64;
    let mut incoming = 0u64;
    let mut active_days: BTreeSet<String> = BTreeSet::new();
    for row in interactions.iter().filter(|row| row.peer_id == peer_id) {
        match row.outgoing {
            true => outgoing += 1,
            false => incoming += 1,
        }
        active_days.insert(utc_date(&rfc3339_utc(row.occurred_at_unix)));
    }
    let count = outgoing + incoming;
    let days = active_days.len() as u64;
    let reciprocal = outgoing > 0 && incoming > 0;
    if reciprocal && count >= STRONG_MIN_INTERACTIONS && days >= STRONG_MIN_ACTIVE_DAYS {
        Band::Strong
    } else if reciprocal && count >= MODERATE_MIN_INTERACTIONS {
        Band::Moderate
    } else {
        Band::Weak
    }
}

/// The active-day set as Goal 1 spells it.
fn goal1_active_days(peer_id: u64, interactions: &[Interaction]) -> BTreeSet<String> {
    interactions
        .iter()
        .filter(|row| row.peer_id == peer_id)
        .map(|row| utc_date(&rfc3339_utc(row.occurred_at_unix)))
        .collect()
}

/// A fixed-seed generator, so a failure reproduces exactly.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }
}

#[test]
fn the_two_spellings_of_a_utc_date_agree() {
    // Midnight, one second before midnight, and one second after, on both
    // sides of the epoch. This is where "floor the integer" and "take the
    // first ten characters" would diverge if either were wrong.
    for anchor in [-DAY * 400, -DAY, 0, DAY, 1_787_529_600, 1_787_529_600 + DAY] {
        for offset in [-1, 0, 1, 86_399, 86_400] {
            let unix = anchor + offset;
            let from_string = utc_date(&rfc3339_utc(unix));
            let (y, m, d) = civil_from_epoch_day(epoch_day(unix));
            assert_eq!(from_string, format!("{y:04}-{m:02}-{d:02}"), "at {unix}");
        }
    }
}

#[test]
fn the_oracle_matches_the_shipped_rule_on_every_generated_tie() {
    let mut rng = Lcg(0x0500_10A1_u64);
    for case in 0..2_000 {
        let rows = rng.below(25);
        let log: Vec<Interaction> = (0..rows)
            .map(|_| Interaction {
                peer_id: rng.below(3),
                outgoing: rng.below(2) == 0,
                // A five-day window at second resolution, so day boundaries
                // are crossed often and sometimes by one second.
                occurred_at_unix: 1_787_529_600 + rng.below(DAY as u64 * 5) as i64 - DAY * 2,
                venue_direct: rng.below(2) == 0,
                conversation_id: rng.below(4),
            })
            .collect();
        for peer_id in 0..3u64 {
            assert_eq!(
                T0::score(peer_id, &log, 1_787_529_600).band,
                goal1_band(peer_id, &log),
                "case {case}, peer {peer_id}, log {log:?}"
            );
        }
    }
}

#[test]
fn the_oracle_matches_the_shipped_rule_across_the_epoch() {
    // Imported archives with broken clocks land before 1970. The shipped rule
    // would store them as RFC 3339 too, so the port has to agree there as well.
    let mut rng = Lcg(99);
    for _ in 0..500 {
        let log: Vec<Interaction> = (0..rng.below(20))
            .map(|_| Interaction {
                peer_id: 1,
                outgoing: rng.below(2) == 0,
                occurred_at_unix: rng.below(DAY as u64 * 8) as i64 - DAY * 4,
                venue_direct: true,
                conversation_id: 1,
            })
            .collect();
        assert_eq!(T0::score(1, &log, 0).band, goal1_band(1, &log), "{log:?}");
    }
}

#[test]
fn the_split_tally_still_holds_goal_ones_active_day_set_and_totals() {
    // Round 3 replaced the single day set with two and the two counts with
    // four. The union of the venues must still be exactly what Goal 1 counted.
    let mut rng = Lcg(0xC0FFEE);
    for _ in 0..1_000 {
        let log: Vec<Interaction> = (0..rng.below(30))
            .map(|_| Interaction {
                peer_id: 1,
                outgoing: rng.below(2) == 0,
                occurred_at_unix: 1_787_529_600 + rng.below(DAY as u64 * 6) as i64 - DAY * 3,
                venue_direct: rng.below(2) == 0,
                conversation_id: 1,
            })
            .collect();
        let tally = Tally::of(1, &log);
        let ours: BTreeSet<String> = tally
            .days
            .iter()
            .map(|&day| {
                let (y, m, d) = civil_from_epoch_day(day);
                format!("{y:04}-{m:02}-{d:02}")
            })
            .collect();
        assert_eq!(ours, goal1_active_days(1, &log), "{log:?}");
        assert!(tally.direct_days.is_subset(&tally.days), "{log:?}");

        // The Goal 1 totals, recovered from the split.
        let goal1_out = log.iter().filter(|row| row.outgoing).count() as u64;
        let goal1_in = log.iter().filter(|row| !row.outgoing).count() as u64;
        assert_eq!(tally.outgoing(), goal1_out, "{log:?}");
        assert_eq!(tally.incoming(), goal1_in, "{log:?}");
        assert_eq!(tally.interaction_count(), log.len() as u64, "{log:?}");
    }
}

#[test]
fn the_counts_the_oracle_reports_are_the_counts_goal_one_reports() {
    // `TieStrength` in `graph_model.rs` carries exactly these tallies, and the
    // UI shows them next to the band. Same evidence, same numbers — plus the
    // Round 3 split, which adds columns without changing any of them.
    let log: Vec<Interaction> = (0..9i64)
        .map(|i| Interaction {
            peer_id: 1,
            outgoing: i % 3 == 0,
            occurred_at_unix: DAY * (i / 2) + 3_600 * i,
            venue_direct: i % 2 == 0,
            conversation_id: (i % 3) as u64,
        })
        .collect();
    let score = T0::score(1, &log, 0);
    assert_eq!(score.interaction_count, 9);
    assert_eq!(score.outgoing_count, 3);
    assert_eq!(score.incoming_count, 6);
    assert_eq!(score.conversation_count, 3);
    assert_eq!(score.active_day_count, 5);
    assert_eq!(score.first_contact_unix, 0);
    assert_eq!(score.last_contact_unix, DAY * 4 + 3_600 * 8);
    assert_eq!(score.algorithm_id, "T0");
    assert_eq!(score.band, Band::Moderate);

    assert_eq!(score.direct_out_count, 2);
    assert_eq!(score.direct_in_count, 3);
    assert_eq!(score.group_out_count, 1);
    assert_eq!(score.group_in_count, 3);
    assert_eq!(score.direct_count() + score.group_count(), 9);
    assert_eq!(score.direct_active_day_count, 5);
}
