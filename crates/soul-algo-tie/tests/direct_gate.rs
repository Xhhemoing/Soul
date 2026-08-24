//! The properties T4D is supposed to have, over generated evidence.
//!
//! The fixtures in `ablation.rs` say what the rule does on twenty-one named
//! ties. These say what it does on ties nobody wrote down, which is where a
//! venue latch would be expected to leak: the whole point of the change is
//! that no amount of group traffic can substitute for a private one, and "no
//! amount" is a claim about all inputs, not about the ones we thought of.

use soul_algo_tie::testing::{self, DAY};
use soul_algo_tie::{Band, Interaction, Tally, TieAlgo, T4, T4D};

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

/// Evidence about one peer: up to 40 rows spread over up to 800 days, with
/// venue and direction chosen independently, so group-heavy, private-heavy and
/// one-sided ties all appear.
fn generated_log(rng: &mut Lcg) -> Vec<Interaction> {
    let rows = rng.below(40);
    (0..rows)
        .map(|_| Interaction {
            peer_id: 1,
            outgoing: rng.below(2) == 0,
            occurred_at_unix: testing::at(rng.below(800) as i64, rng.below(15) as i64),
            venue_direct: rng.below(3) == 0,
            conversation_id: rng.below(3),
        })
        .collect()
}

fn direct_rows(log: &[Interaction]) -> Vec<Interaction> {
    log.iter().filter(|row| row.venue_direct).cloned().collect()
}

#[test]
fn t4d_never_says_more_than_t4() {
    // T4D counts a subset of T4's rows over a subset of its days, direct
    // reciprocity implies reciprocity, and both rules share one recency step,
    // so the default rule can only ever be the more cautious of the two. This
    // is what makes "adopt T4D" a strictly conservative move on any evidence,
    // not just on the fixtures.
    let mut rng = Lcg(0x7413_D000);
    for case in 0..4_000 {
        let log = generated_log(&mut rng);
        let as_of = testing::AS_OF_2026_08_24;
        let t4 = TieAlgo::T4.score(1, &log, as_of);
        let t4d = TieAlgo::T4D.score(1, &log, as_of);
        assert!(
            t4d.band.rank() <= t4.band.rank(),
            "case {case}: T4D said {:?} where T4 said {:?}, log {log:?}",
            t4d.band,
            t4.band
        );
    }
}

#[test]
fn the_two_rules_are_the_same_rule_on_evidence_with_no_group_rows() {
    // The no-regression claim in REPORT.md, as a property. On a private-only
    // log the two rules read literally the same numbers, so they must agree
    // before the recency step as well as after it.
    let mut rng = Lcg(0x9001_C0DE);
    for case in 0..4_000 {
        let log = direct_rows(&generated_log(&mut rng));
        let tally = Tally::of(1, &log);
        assert_eq!(
            T4::counts_band(&tally),
            T4D::counts_band(&tally),
            "case {case}: counts stage differs on {log:?}"
        );
        for as_of in [testing::AS_OF_2026_08_24, testing::at(0, 0) + DAY * 900] {
            assert_eq!(
                TieAlgo::T4.score(1, &log, as_of).band,
                TieAlgo::T4D.score(1, &log, as_of).band,
                "case {case} at {as_of}: {log:?}"
            );
        }
    }
}

#[test]
fn group_rows_cannot_move_the_counts_stage_of_the_default_rule() {
    // The latch itself. Deleting every group row leaves T4D's counts band
    // untouched, however much of the traffic it was — which is the precise
    // sense in which group activity "does not count".
    let mut rng = Lcg(0x006C_0AD5);
    for case in 0..4_000 {
        let log = generated_log(&mut rng);
        let with_group = Tally::of(1, &log);
        let without = Tally::of(1, &direct_rows(&log));
        assert_eq!(
            T4D::counts_band(&with_group),
            T4D::counts_band(&without),
            "case {case}: {log:?}"
        );
    }
}

#[test]
fn group_rows_can_still_move_the_final_band_through_the_recency_clock() {
    // The other half of the same coin, and the thing the Round 3 brief asked
    // for: group traffic is not evidence of closeness, but it is evidence that
    // the person is still around. This is the only channel through which a
    // group message changes a T4D band, and it only ever runs upwards.
    let mut rng = Lcg(0xC10C_C10C);
    let as_of = testing::AS_OF_2026_08_24;
    let mut moved = 0;
    for _ in 0..4_000 {
        let log = generated_log(&mut rng);
        let quiet = direct_rows(&log);
        let with_group = TieAlgo::T4D.score(1, &log, as_of).band;
        let without = TieAlgo::T4D.score(1, &quiet, as_of).band;
        assert!(
            with_group.rank() >= without.rank(),
            "group rows lowered a band: {log:?}"
        );
        if with_group != without {
            moved += 1;
        }
    }
    assert!(
        moved > 0,
        "the generator never produced a tie where a group message saved a band"
    );
}

#[test]
fn every_strong_band_from_the_default_rule_is_backed_by_private_traffic() {
    // The invariant the product cares about, stated over all inputs: if the
    // graph calls somebody a close tie, there are at least ten one-to-one
    // exchanges in both directions across at least three separate days behind
    // it.
    let mut rng = Lcg(0x5730_4E67);
    for _ in 0..4_000 {
        let log = generated_log(&mut rng);
        let score = TieAlgo::T4D.score(1, &log, testing::AS_OF_2026_08_24);
        if score.band != Band::Strong {
            continue;
        }
        assert!(score.direct_out_count >= 1, "{log:?}");
        assert!(score.direct_in_count >= 1, "{log:?}");
        assert!(score.direct_count() >= 10, "{log:?}");
        assert!(score.direct_active_day_count >= 3, "{log:?}");
        assert!(score.silent_days < 180, "{log:?}");
    }
}

#[test]
fn a_tie_only_ever_seen_in_a_group_is_weak_under_the_default_rule() {
    let mut rng = Lcg(0xBEEF_0F17);
    for _ in 0..2_000 {
        let log: Vec<Interaction> = generated_log(&mut rng)
            .into_iter()
            .map(|row| Interaction {
                venue_direct: false,
                ..row
            })
            .collect();
        let score = TieAlgo::T4D.score(1, &log, testing::AS_OF_2026_08_24);
        assert_eq!(score.band, Band::Weak, "{log:?}");
        // And the rule it replaces stops one rung higher, which is the one
        // place T4D is harsher.
        let t4 = TieAlgo::T4.score(1, &log, testing::AS_OF_2026_08_24);
        assert_ne!(t4.band, Band::Strong, "{log:?}");
    }
}

#[test]
fn no_amount_of_group_traffic_reaches_strong_without_ten_private_exchanges() {
    // The adversarial construction, done deliberately rather than by chance:
    // a thousand group messages a day for a year, plus nine private exchanges
    // spread over nine days. One private message short, and the answer is
    // still not Strong.
    let as_of = testing::AS_OF_2026_08_24;
    let mut log: Vec<Interaction> = Vec::new();
    for day in 0..365i64 {
        for i in 0..3i64 {
            log.push(testing::group(
                1,
                (day + i) % 2 == 0,
                testing::at(day, 9),
                1,
            ));
        }
    }
    for i in 0..9i64 {
        log.push(testing::direct(1, i % 2 == 0, testing::at(i, 11), 2));
    }
    let score = TieAlgo::T4D.score(1, &log, as_of);
    assert_eq!(score.interaction_count, 1_104);
    assert_eq!(score.direct_count(), 9);
    assert_eq!(score.band, Band::Moderate);
    assert_eq!(TieAlgo::T4.score(1, &log, as_of).band, Band::Strong);

    // The tenth private exchange is the whole difference.
    log.push(testing::direct(1, true, testing::at(9, 11), 2));
    assert_eq!(TieAlgo::T4D.score(1, &log, as_of).band, Band::Strong);
}

#[test]
fn time_passing_never_raises_a_band() {
    // The recency step is a one-way ratchet for fixed evidence, under both
    // rules. Without this, "your tie got weaker because you stopped talking"
    // could reverse itself while nothing happened.
    let mut rng = Lcg(0x7143_5EED);
    for _ in 0..2_000 {
        let log = generated_log(&mut rng);
        for algo in TieAlgo::ALL {
            let mut previous = Band::Strong;
            for extra_days in [0i64, 90, 179, 180, 359, 360, 900] {
                let band = algo
                    .score(1, &log, testing::AS_OF_2026_08_24 + extra_days * DAY)
                    .band;
                assert!(
                    band.rank() <= previous.rank(),
                    "{} rose from {previous:?} to {band:?} after {extra_days} more days: {log:?}",
                    algo.id()
                );
                previous = band;
            }
        }
    }
}
