//! The rejected rules, kept only as executable tombstones.
//!
//! This module is `#[cfg(test)]`: nothing in the product path can reach it,
//! nothing re-exports it, and it takes part in no ablation. It exists because
//! fable-a's acceptance gate G2 asks that a rejection be traceable to the
//! fixture that killed it, and R2-SYNTHESIS asks for a tombstone file at all.
//! A paragraph in a document can rot; a test that fails when the claim stops
//! being true cannot.
//!
//! * **T1 (recency-weighted exponential decay)** — killed in Round 1. No span
//!   gate, so twenty messages in one afternoon reach Strong, and the band is
//!   decided by a float the user cannot reproduce.
//! * **T2 (RFM band)** — killed in Round 1. The axes are compensatory, so a
//!   shortfall on one is bought back on another.
//! * **T3 (Granovetter gate, no recency)** — killed in Round 2. It is exactly
//!   [`T4`]'s counts stage, so the tombstone is the counts stage run without
//!   the silence step: a friendship that ended in 2019 still reads Strong.
//! * **T3R (bucketed integer decay)** — the Round 2 runner-up, and the
//!   designated fallback. R2-SYNTHESIS sets its revival condition: it comes
//!   back if and only if the demotion family's "one greeting revives a
//!   seven-year-old tie" behaviour is judged unacceptable. The price of that
//!   revival is pinned below on `revived_after_gap`.
//!
//! The float in the T1 exhibit is the only one in the crate, and it is here to
//! demonstrate the thing that was rejected.

use crate::testing::{afternoon_20, direct, dormant_2019, group_only_50, revived_after_gap, DAY};
use crate::types::{age_days, epoch_day, Band, Interaction, Tally};
use crate::{TieAlgo, T4, T4D};

use std::collections::BTreeMap;

/// The timestamps of one peer's rows, which is all any tombstone needs.
fn timestamps(peer_id: u64, log: &[Interaction]) -> Vec<i64> {
    log.iter()
        .filter(|row| row.peer_id == peer_id)
        .map(|row| row.occurred_at_unix)
        .collect()
}

// ---------------------------------------------------------------------------
// T1 — exponential decay over a float
// ---------------------------------------------------------------------------

fn t1_mass(peer_id: u64, log: &[Interaction], as_of_unix: i64) -> f64 {
    timestamps(peer_id, log)
        .into_iter()
        .map(|at| {
            let age = age_days(at, as_of_unix) as f64;
            (-f64::ln(2.0) * age / 90.0).exp()
        })
        .sum()
}

fn t1_band(peer_id: u64, log: &[Interaction], as_of_unix: i64) -> Band {
    let tally = Tally::of(peer_id, log);
    let mass = t1_mass(peer_id, log, as_of_unix);
    if tally.is_reciprocal() && mass >= 8.0 {
        Band::Strong
    } else if tally.is_reciprocal() && mass >= 2.0 {
        Band::Moderate
    } else {
        Band::Weak
    }
}

// ---------------------------------------------------------------------------
// T2 — three rungs, summed
// ---------------------------------------------------------------------------

fn t2_sum(tally: &Tally, as_of_unix: i64) -> u8 {
    let recency = match tally.silent_days(as_of_unix) {
        _ if tally.is_empty() => 1,
        d if d <= 7 => 3,
        d if d <= 30 => 2,
        _ => 1,
    };
    let frequency = match tally.interaction_count() {
        c if c >= 10 => 3,
        c if c >= 3 => 2,
        _ => 1,
    };
    let days = match tally.active_day_count() {
        d if d >= 5 => 3,
        d if d >= 2 => 2,
        _ => 1,
    };
    recency + frequency + days
}

fn t2_band(tally: &Tally, as_of_unix: i64) -> Band {
    let sum = t2_sum(tally, as_of_unix);
    if tally.is_reciprocal() && sum >= 8 {
        Band::Strong
    } else if tally.is_reciprocal() && sum >= 5 {
        Band::Moderate
    } else {
        Band::Weak
    }
}

// ---------------------------------------------------------------------------
// T3R — bucketed decay in quarter-interaction units
// ---------------------------------------------------------------------------

/// Weights 1, 1/2, 1/4 and 0, multiplied by four so the whole rule is integer
/// arithmetic. R2-SYNTHESIS voids the milliscale spelling that was 1000× this.
const fn quarter_weight(age_days: i64) -> u64 {
    match age_days {
        d if d >= 360 => 0,
        d if d >= 180 => 1,
        d if d >= 90 => 2,
        _ => 4,
    }
}

/// `(effective interactions, effective active days)`, both in quarter units.
fn t3r_effective(peer_id: u64, log: &[Interaction], as_of_unix: i64) -> (u64, u64) {
    let times = timestamps(peer_id, log);
    let count = times
        .iter()
        .map(|&at| quarter_weight(age_days(at, as_of_unix)))
        .sum();
    // A day is worth the weight of its newest exchange.
    let mut newest_per_day: BTreeMap<i64, i64> = BTreeMap::new();
    for at in times {
        newest_per_day
            .entry(epoch_day(at))
            .and_modify(|latest| *latest = (*latest).max(at))
            .or_insert(at);
    }
    let days = newest_per_day
        .values()
        .map(|&at| quarter_weight(age_days(at, as_of_unix)))
        .sum();
    (count, days)
}

fn t3r_band(peer_id: u64, log: &[Interaction], as_of_unix: i64) -> Band {
    let tally = Tally::of(peer_id, log);
    let (count, days) = t3r_effective(peer_id, log, as_of_unix);
    // 10 interactions and 3 days, in quarter units.
    let uncapped = if tally.is_reciprocal() && tally.any_direct() && count >= 40 && days >= 12 {
        Band::Strong
    } else if tally.is_reciprocal() && count >= 12 {
        Band::Moderate
    } else {
        Band::Weak
    };
    if tally.is_group_only() {
        uncapped.capped_at(Band::Moderate)
    } else {
        uncapped
    }
}

// ---------------------------------------------------------------------------
// The exhibits
// ---------------------------------------------------------------------------

#[test]
fn t1_calls_one_afternoon_a_strong_tie_which_is_why_it_is_not_a_product_path() {
    // Twenty exchanges in one afternoon: mass ≈ 20, no span gate, so T1
    // promotes a single conversation to the top band. Both surviving rules
    // stop at Moderate.
    let f = afternoon_20();
    assert!(t1_mass(f.peer_id, &f.log, f.as_of) > 19.0);
    assert_eq!(t1_band(f.peer_id, &f.log, f.as_of), Band::Strong);
    for algo in TieAlgo::ALL {
        assert_eq!(
            algo.score(f.peer_id, &f.log, f.as_of).band,
            Band::Moderate,
            "{}",
            algo.id()
        );
    }
}

#[test]
fn t1s_group_weight_does_not_stop_a_group_chat_reaching_strong() {
    // Why a weight cannot replace a gate: even at 0.4 per group exchange,
    // fifty recent ones clear a mass of 8.
    let f = group_only_50();
    assert!(t1_mass(f.peer_id, &f.log, f.as_of) * 0.4 > 8.0);
    assert_eq!(t1_band(f.peer_id, &f.log, f.as_of), Band::Strong);
    for algo in TieAlgo::ALL {
        assert_ne!(
            algo.score(f.peer_id, &f.log, f.as_of).band,
            Band::Strong,
            "{}",
            algo.id()
        );
    }
}

#[test]
fn t2_lets_one_axis_buy_back_another() {
    // Ten reciprocal exchanges over two recent days: recency 3 + frequency 3 +
    // days 2 = 8 → Strong, although the span requirement both surviving rules
    // keep says one weekend is not a habit.
    let log: Vec<Interaction> = (0..10)
        .map(|i| direct(1, i % 2 == 0, DAY * (i as i64 % 2) + 60 * i as i64, 1))
        .collect();
    let tally = Tally::of(1, &log);
    let as_of = DAY * 3;
    assert_eq!(t2_sum(&tally, as_of), 8);
    assert_eq!(t2_band(&tally, as_of), Band::Strong);
    assert_eq!(T4::counts_band(&tally), Band::Moderate);
    assert_eq!(T4D::counts_band(&tally), Band::Moderate);
}

#[test]
fn t2_moves_a_band_that_no_new_evidence_touched() {
    let log: Vec<Interaction> = (0..10)
        .map(|i| direct(1, i % 2 == 0, DAY * (i as i64 % 5), 1))
        .collect();
    let tally = Tally::of(1, &log);
    assert_eq!(t2_band(&tally, DAY * 8), Band::Strong);
    assert_eq!(t2_band(&tally, DAY * 40), Band::Moderate);
    // Same evidence, one rung lost to 32 extra rungless days, in a step the
    // user cannot recount.
    assert_eq!(tally.interaction_count(), 10);
}

#[test]
fn t3_cannot_see_that_seven_years_went_by() {
    // T3 is the counts stage with no recency step, so this is literally the
    // counts stage of both survivors, run without their second half.
    let f = dormant_2019();
    let tally = Tally::of(f.peer_id, &f.log);
    assert_eq!(T4::counts_band(&tally), Band::Strong);
    assert_eq!(T4D::counts_band(&tally), Band::Strong);
    // With the recency step, which is the whole of Round 2's argument.
    for algo in TieAlgo::ALL {
        assert_eq!(
            algo.score(f.peer_id, &f.log, f.as_of).band,
            Band::Weak,
            "{}",
            algo.id()
        );
    }
}

#[test]
fn t3rs_revival_condition_costs_the_revived_friendship() {
    // R2-SYNTHESIS: T3R comes back only if «昨天互道一声好即可复活七年前的
    // Strong» is judged unacceptable. This is what buying that costs: thirty
    // exchanges around 200 days old are worth a quarter each, the two from
    // this week are worth one each, so 30 + 8 = 38 quarter-units — 9.5
    // effective exchanges against a bar of 10 — and somebody the user spoke to
    // four days ago comes out Moderate.
    let f = revived_after_gap();
    assert_eq!(
        t3r_effective(f.peer_id, &f.log, f.as_of),
        (38, 18),
        "the arithmetic the fallback rests on"
    );
    assert_eq!(t3r_band(f.peer_id, &f.log, f.as_of), Band::Moderate);
    for algo in TieAlgo::ALL {
        assert_eq!(
            algo.score(f.peer_id, &f.log, f.as_of).band,
            Band::Strong,
            "{}",
            algo.id()
        );
    }
}

#[test]
fn t3r_would_not_have_fixed_the_venue_latch_either() {
    // The reason the fallback is a fallback and not the answer to Round 3:
    // T3R re-weights by age, not by venue, so the group-heavy tie with one
    // private hello each way clears its Strong gate exactly as T4 does.
    let f = crate::testing::group_heavy_plus_one_direct_each_way();
    assert_eq!(t3r_band(f.peer_id, &f.log, f.as_of), Band::Strong);
    assert_eq!(
        TieAlgo::T4.score(f.peer_id, &f.log, f.as_of).band,
        Band::Strong
    );
    assert_eq!(
        TieAlgo::T4D.score(f.peer_id, &f.log, f.as_of).band,
        Band::Weak
    );
}
