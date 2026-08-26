//! T1 and T2, kept only as executable tombstones.
//!
//! This module is `#[cfg(test)]`: nothing in the product path can reach it,
//! nothing re-exports it, and it takes part in no ablation. It exists because
//! R1-SYNTHESIS retired both rules and fable-a's acceptance gate G2 asks that
//! a rejection be traceable to the fixture that killed it. A paragraph in a
//! document can rot; a test that fails when the claim stops being true cannot.
//!
//! * **T1 (recency-weighted exponential decay)** — killed as a product path.
//!   It has no span gate, so twenty messages in one afternoon reach Strong,
//!   and its band is decided by a float the user cannot reproduce. Its
//!   *measure* survives inside T3R, in integer form.
//! * **T2 (RFM band)** — killed. The axes are compensatory, so a shortfall on
//!   one can be bought back on another, and the quantile reading of it is
//!   population-relative, which is a percentile in all but name.
//!
//! The float below is the only one in the crate, and it is here to demonstrate
//! the thing that was rejected.

use crate::testing::{afternoon_20, direct, group_only_50, DAY};
use crate::types::{age_days, Band, Interaction, Tally};
use crate::{TieAlgo, T3};

/// T1's weighted mass, exactly as Round 1 opus-a shipped it.
fn t1_mass(tally: &Tally, as_of_unix: i64) -> f64 {
    tally
        .timestamps
        .iter()
        .map(|&at| {
            let age = age_days(at, as_of_unix) as f64;
            (-f64::ln(2.0) * age / 90.0).exp()
        })
        .sum()
}

/// T1's band. Reciprocity and active days gate on raw counts; only the mass is
/// weighted. Venue is not a gate, which is the other half of why it lost.
fn t1_band(tally: &Tally, as_of_unix: i64) -> Band {
    let mass = t1_mass(tally, as_of_unix);
    if tally.is_reciprocal() && mass >= 8.0 {
        Band::Strong
    } else if tally.is_reciprocal() && mass >= 2.0 {
        Band::Moderate
    } else {
        Band::Weak
    }
}

/// T2's three rungs, summed. Recency ≤7d → 3, ≤30d → 2, else 1; frequency
/// ≥10 → 3, ≥3 → 2, else 1; active days ≥5 → 3, ≥2 → 2, else 1.
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

#[test]
fn t1_calls_one_afternoon_a_strong_tie_which_is_why_it_is_not_a_product_path() {
    // EVAL_MATRIX F02. Twenty exchanges in one afternoon: mass ≈ 20, no span
    // gate, so T1 promotes a single conversation to the top band. Every
    // surviving rule stops at Moderate.
    let f = afternoon_20();
    let tally = Tally::of(f.peer_id, &f.log);
    assert!(t1_mass(&tally, f.as_of) > 19.0);
    assert_eq!(t1_band(&tally, f.as_of), Band::Strong);
    for algo in TieAlgo::CANDIDATES {
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
    // R1-SYNTHESIS P0-3, and the reason a weight cannot replace a gate: even
    // at 0.4 per group exchange, fifty recent ones clear a mass of 8.
    let f = group_only_50();
    let tally = Tally::of(f.peer_id, &f.log);
    assert!(t1_mass(&tally, f.as_of) * 0.4 > 8.0);
    assert_eq!(t1_band(&tally, f.as_of), Band::Strong);
    assert_ne!(T3::band_of(&tally, f.as_of), Band::Strong);
}

#[test]
fn t2_lets_one_axis_buy_back_another() {
    // Ten reciprocal exchanges over two recent days: recency 3 + frequency 3 +
    // days 2 = 8 → Strong, although the span requirement every surviving rule
    // keeps says one weekend is not a habit.
    let log: Vec<Interaction> = (0..10)
        .map(|i| direct(1, i % 2 == 0, DAY * (i as i64 % 2) + 60 * i as i64, 1))
        .collect();
    let tally = Tally::of(1, &log);
    let as_of = DAY * 3;
    assert_eq!(t2_sum(&tally, as_of), 8);
    assert_eq!(t2_band(&tally, as_of), Band::Strong);
    assert_eq!(T3::band_of(&tally, as_of), Band::Moderate);
}

#[test]
fn t2_moves_a_band_that_no_new_evidence_touched() {
    // The structural failure (EVAL_MATRIX F12) in its absolute-threshold form:
    // the rungs are cut-offs on quantities that a *quantile* reading would
    // make population-relative. The tombstone pins the mechanism rather than
    // the quantile arithmetic: the same tie changes band with nothing but the
    // passage of `as_of`, in steps the user cannot recount, and the
    // explanation has to be comparative.
    let log: Vec<Interaction> = (0..10)
        .map(|i| direct(1, i % 2 == 0, DAY * (i as i64 % 5), 1))
        .collect();
    let tally = Tally::of(1, &log);
    assert_eq!(t2_band(&tally, DAY * 8), Band::Strong);
    assert_eq!(t2_band(&tally, DAY * 40), Band::Moderate);
    // Same evidence, one rung lost to eight extra rungless days: nothing in
    // the counts the user can see changed.
    assert_eq!(tally.interaction_count(), 10);
}
