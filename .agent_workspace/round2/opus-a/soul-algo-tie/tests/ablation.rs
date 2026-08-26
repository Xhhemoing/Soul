//! C8 ablation: does the extra machinery in T3R and T4 pay for itself?
//!
//! fable-a's gate (EVAL_MATRIX §C8) is: a non-baseline candidate must give the
//! right band on at least one adversarial fixture where the baseline gives the
//! wrong one, and must not give the wrong band anywhere the baseline was
//! right. This file is that measurement.
//!
//! Three tables, in this order, because the order is the discipline:
//!
//! 1. [`TRUTH`] — what the band *should* be, per fixture. Judged from the
//!    product's own statements (R1-SYNTHESIS, fable-a's EVAL_MATRIX F01–F05)
//!    and from common sense about the tie, never from what a rule happens to
//!    output. Where the round has not agreed on an answer, the entry says
//!    `Unjudged` rather than inventing one.
//! 2. [`OBSERVED`] — what each rule actually says. Pinned exactly, so a change
//!    in any rule shows up as a diff in this file rather than as a silent
//!    shift in a recommendation.
//! 3. [`VIOLATIONS`] — the C8 result: which rule breaks which judged
//!    expectation. This is the table the recommendation in REPORT.md rests on.

use soul_algo_tie::testing::{self, Fixture};
use soul_algo_tie::{score_all, Band, Detail, TieAlgo};

// ---------------------------------------------------------------------------
// 1. What the band should be
// ---------------------------------------------------------------------------

/// The judged expectation for a fixture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Truth {
    /// The round has agreed on one answer.
    Exactly(Band),
    /// The round has agreed only that the top band is wrong. This is how
    /// fable-a states F03 (群聊重度 → ≤Moderate) and F04 (陈年强关系 → 非
    /// Strong): the failure is calling a stranger or a memory a close friend,
    /// and whether the right answer is Moderate or Weak is a taste question
    /// the fixtures cannot settle.
    NotStrong,
    /// No agreed answer. Recorded, not scored.
    Unjudged,
}

impl Truth {
    fn violated_by(self, band: Band) -> bool {
        match self {
            Truth::Exactly(expected) => band != expected,
            Truth::NotStrong => band == Band::Strong,
            Truth::Unjudged => false,
        }
    }
}

const TRUTH: &[(&str, Truth)] = &[
    // Nothing observed, and one unanswered message: reciprocity is a gate.
    ("empty", Truth::Exactly(Band::Weak)),
    ("single_inbound", Truth::Exactly(Band::Weak)),
    // Goal 1's acceptance case. Anything that demotes this is a replacement,
    // not a refinement.
    ("lilei_12", Truth::Exactly(Band::Strong)),
    // EVAL_MATRIX F02: one conversation is not a habit.
    ("afternoon_20", Truth::Exactly(Band::Moderate)),
    // EVAL_MATRIX F03 / R1-SYNTHESIS P0-3.
    ("group_only_50", Truth::NotStrong),
    // EVAL_MATRIX F01: volume cannot buy reciprocity.
    ("one_sided_100", Truth::Exactly(Band::Weak)),
    // A flood, or an import that lost its dates. Same span argument as F02.
    ("flood_1000_in_one_day", Truth::NotStrong),
    // EVAL_MATRIX F05: the steady friendship.
    ("steady_16_over_8_weeks", Truth::Exactly(Band::Strong)),
    // One day short of the demotion threshold. Deliberately unjudged: the
    // round agreed that half a year of silence costs a band, and never agreed
    // what the day before that is worth.
    ("quiet_179_days", Truth::Unjudged),
    // From here the tie has been silent for at least half a year. R1-SYNTHESIS
    // P1-4 says a tie nobody has touched in that long is not a current strong
    // tie, whatever the history says.
    ("quiet_180_days", Truth::NotStrong),
    ("quiet_200_days", Truth::NotStrong),
    // The one contestable cell in this table, flagged as such in ABLATION.md:
    // 32 reciprocal one-to-one exchanges over 12 active days with somebody the
    // user spoke to four days ago. If the graph cannot call that a close tie,
    // the graph is describing an archive rather than a life. Judged Strong.
    ("revived_after_gap", Truth::Exactly(Band::Strong)),
    ("group_only_quiet_200", Truth::NotStrong),
    ("dormant_359_days", Truth::NotStrong),
    ("dormant_360_days", Truth::NotStrong),
    // EVAL_MATRIX F04, the headline case.
    ("dormant_2019", Truth::NotStrong),
];

// ---------------------------------------------------------------------------
// 2. What each rule actually says
// ---------------------------------------------------------------------------

use Band::{Moderate as M, Strong as S, Weak as W};

/// `(fixture, T0, T3, T3R, T4)`, in [`TieAlgo::ALL`] order.
const OBSERVED: &[(&str, Band, Band, Band, Band)] = &[
    ("empty", W, W, W, W),
    ("single_inbound", W, W, W, W),
    ("lilei_12", S, S, S, S),
    ("afternoon_20", M, M, M, M),
    ("group_only_50", S, M, M, M),
    ("one_sided_100", W, W, W, W),
    ("flood_1000_in_one_day", M, M, M, M),
    ("steady_16_over_8_weeks", S, S, S, S),
    ("quiet_179_days", S, S, M, S),
    ("quiet_180_days", S, S, M, M),
    ("quiet_200_days", S, S, M, M),
    ("revived_after_gap", S, S, M, S),
    ("group_only_quiet_200", S, M, M, W),
    ("dormant_359_days", S, S, W, M),
    ("dormant_360_days", S, S, W, W),
    ("dormant_2019", S, S, W, W),
];

// ---------------------------------------------------------------------------
// 3. The C8 result
// ---------------------------------------------------------------------------

/// Every fixture where a rule breaks a judged expectation, per rule.
///
/// T0 and T3 are the baselines being ablated against; T3R and T4 are the
/// candidates. Read this as: T4 breaks nothing, T3R breaks the one cell whose
/// ground truth is a judgment call, and both fix everything T3 gets wrong.
const VIOLATIONS: &[(&str, &[&str])] = &[
    (
        "T0",
        &[
            "group_only_50",
            "quiet_180_days",
            "quiet_200_days",
            "group_only_quiet_200",
            "dormant_359_days",
            "dormant_360_days",
            "dormant_2019",
        ],
    ),
    (
        "T3",
        &[
            "quiet_180_days",
            "quiet_200_days",
            "dormant_359_days",
            "dormant_360_days",
            "dormant_2019",
        ],
    ),
    ("T3R", &["revived_after_gap"]),
    ("T4", &[]),
];

fn fixture(name: &str) -> Fixture {
    testing::all()
        .into_iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("no fixture named {name}"))
}

fn bands(f: &Fixture) -> [Band; 4] {
    score_all(f.peer_id, &f.log, f.as_of).map(|score| score.band)
}

fn band(f: &Fixture, algo: TieAlgo) -> Band {
    algo.score(f.peer_id, &f.log, f.as_of).band
}

fn truth_of(name: &str) -> Truth {
    TRUTH
        .iter()
        .find(|(fixture, _)| *fixture == name)
        .map(|(_, truth)| *truth)
        .unwrap_or_else(|| panic!("no judged truth for {name}"))
}

#[test]
fn every_fixture_appears_in_every_table() {
    let names: Vec<&str> = testing::all().iter().map(|f| f.name).collect();
    assert_eq!(
        names,
        TRUTH.iter().map(|(name, _)| *name).collect::<Vec<_>>()
    );
    assert_eq!(names, OBSERVED.iter().map(|row| row.0).collect::<Vec<_>>());
}

#[test]
fn the_observed_table_is_exactly_what_the_rules_say() {
    // The ablation table in ABLATION.md is this array. If a rule changes, this
    // is the test that makes the document stale loudly rather than quietly.
    for &(name, t0, t3, t3r, t4) in OBSERVED {
        let f = fixture(name);
        assert_eq!(bands(&f), [t0, t3, t3r, t4], "{name}");
    }
}

#[test]
fn the_c8_violation_tables_are_exactly_these() {
    for &(algo_id, expected) in VIOLATIONS {
        let algo = TieAlgo::ALL
            .into_iter()
            .find(|a| a.id() == algo_id)
            .expect("known rule");
        let observed: Vec<&str> = testing::all()
            .iter()
            .filter(|f| truth_of(f.name).violated_by(band(f, algo)))
            .map(|f| f.name)
            .collect();
        assert_eq!(observed, expected.to_vec(), "{algo_id}");
    }
}

#[test]
fn t4_breaks_no_judged_expectation_anywhere() {
    // The headline of the round, stated on its own so it cannot be lost in a
    // table: on every fixture with an agreed answer, the integer demotion rule
    // gives that answer.
    for f in testing::all() {
        let got = band(&f, TieAlgo::T4);
        assert!(
            !truth_of(f.name).violated_by(got),
            "T4 said {got:?} on {}, expected {:?}",
            f.name,
            truth_of(f.name)
        );
    }
}

// ---------------------------------------------------------------------------
// The cases the brief names, one test each
// ---------------------------------------------------------------------------

#[test]
fn f04_dormant_2019_is_the_net_win_for_decay_and_demotion() {
    // Twenty reciprocal one-to-one messages over ten days in 2019, judged on
    // 2026-08-24. T3 has no way to notice that seven years went by and calls a
    // memory a strong tie; both recency-aware rules refuse.
    let f = fixture("dormant_2019");
    assert_eq!(bands(&f), [S, S, W, W]);

    let t4 = TieAlgo::T4.score(f.peer_id, &f.log, f.as_of);
    assert_eq!(t4.silent_days, 2_632);
    assert!(t4.silent_days >= 360, "the cut-off is what fires here");
    let Detail::Demoted { band_before } = t4.detail else {
        panic!("expected demotion detail");
    };
    assert_eq!(band_before, Band::Strong);

    let t3r = TieAlgo::T3R.score(f.peer_id, &f.log, f.as_of);
    let Detail::Decayed(decayed) = t3r.detail else {
        panic!("expected decay detail");
    };
    assert_eq!(
        decayed.eff_count_milli, 0,
        "every weight is zero past 360 days"
    );
    assert_eq!(decayed.bucket_interactions, [0, 0, 0, 20]);

    // The counts survive the demotion: the evidence is still there to be
    // recounted, only the claim about the present is withdrawn.
    for score in score_all(f.peer_id, &f.log, f.as_of) {
        assert_eq!(score.interaction_count, 20);
        assert_eq!(score.active_day_count, 10);
    }
}

#[test]
fn group_only_50_is_never_strong_under_any_candidate() {
    let f = fixture("group_only_50");
    assert_eq!(bands(&f), [S, M, M, M]);
    for algo in TieAlgo::CANDIDATES {
        assert_ne!(band(&f, algo), Band::Strong, "{}", algo.id());
    }
    // And the shipped rule still gets it wrong, which is why T0 is not a
    // fallback: R1-SYNTHESIS «禁止回退到未打补丁的 T0».
    assert_eq!(band(&f, TieAlgo::T0), Band::Strong);
}

#[test]
fn lilei_12_is_strong_under_every_candidate() {
    // Zero-regression anchor: the tie Goal 1 was accepted on.
    let f = fixture("lilei_12");
    assert_eq!(bands(&f), [S, S, S, S]);
    let t3r = TieAlgo::T3R.score(f.peer_id, &f.log, f.as_of);
    let Detail::Decayed(decayed) = t3r.detail else {
        panic!("expected decay detail");
    };
    // Twelve fresh exchanges over six fresh days: 48 and 24 quarter-units,
    // comfortably over the 40 and 12 bars.
    assert_eq!(decayed.eff_count_milli, 48);
    assert_eq!(decayed.eff_days_milli, 24);
}

#[test]
fn afternoon_20_is_moderate_and_never_strong() {
    let f = fixture("afternoon_20");
    assert_eq!(bands(&f), [M, M, M, M]);
    let t3r = TieAlgo::T3R.score(f.peer_id, &f.log, f.as_of);
    let Detail::Decayed(decayed) = t3r.detail else {
        panic!("expected decay detail");
    };
    // The count bar is cleared five times over; the span bar is not. Decay
    // does not change which gate fails.
    assert_eq!(decayed.eff_count_milli, 80);
    assert_eq!(decayed.eff_days_milli, 4);
}

#[test]
fn quiet_200_days_is_the_second_net_win_for_t4_over_t3() {
    // Strong by every count, last heard from 200 days ago. T3 says Strong; the
    // demotion rule takes exactly one rung, which is the whole mechanism.
    let f = fixture("quiet_200_days");
    assert_eq!(bands(&f), [S, S, M, M]);

    let t4 = TieAlgo::T4.score(f.peer_id, &f.log, f.as_of);
    assert_eq!(t4.silent_days, 200);
    let Detail::Demoted { band_before } = t4.detail else {
        panic!("expected demotion detail");
    };
    assert_eq!(band_before, Band::Strong);
    assert_eq!(t4.band, Band::Moderate);
    // T3R lands on the same band by a different route: every exchange is in
    // the quarter bucket, so 12 interactions are worth 3 effective ones.
    let Detail::Decayed(decayed) = TieAlgo::T3R.score(f.peer_id, &f.log, f.as_of).detail else {
        panic!("expected decay detail");
    };
    assert_eq!(decayed.eff_count_milli, 12);
}

#[test]
fn the_two_recency_rules_disagree_on_exactly_these_fixtures() {
    // Recorded as the brief asks. Every disagreement is a case where the two
    // rules measure the same silence differently: T3R re-weights the whole
    // history, T4 looks only at the newest exchange.
    let disagreements: Vec<(&str, Band, Band)> = testing::all()
        .iter()
        .map(|f| (f.name, band(f, TieAlgo::T3R), band(f, TieAlgo::T4)))
        .filter(|(_, t3r, t4)| t3r != t4)
        .collect();
    assert_eq!(
        disagreements,
        vec![
            // Silent for 179 days, so T4 has not fired yet, while T3R has
            // already quartered five of the six active days.
            ("quiet_179_days", M, S),
            // The contestable cell: T4 sees somebody the user spoke to four
            // days ago, T3R sees 9.5 effective exchanges — half a message
            // under its bar.
            ("revived_after_gap", M, S),
            // Group-only and silent: T3R's ceiling holds it at Moderate, T4
            // demotes from that ceiling and reaches Weak.
            ("group_only_quiet_200", M, W),
            // One day inside the cut-off: T3R keeps only the newest day's
            // quarter and lands on Weak, T4 takes one rung and lands on
            // Moderate.
            ("dormant_359_days", W, M),
        ]
    );
}

#[test]
fn the_revived_tie_is_where_t3r_under_calls() {
    // Spelled out because it is the cell the recommendation turns on. Thirty
    // exchanges around 200 days old are worth a quarter each, the two from
    // this week are worth one each: 30 + 8 = 38 quarter-units, i.e. 9.5
    // effective exchanges against a bar of 10. Half a message.
    let f = fixture("revived_after_gap");
    let t3r = TieAlgo::T3R.score(f.peer_id, &f.log, f.as_of);
    let Detail::Decayed(decayed) = t3r.detail else {
        panic!("expected decay detail");
    };
    assert_eq!(t3r.interaction_count, 32);
    assert_eq!(t3r.active_day_count, 12);
    assert_eq!(decayed.bucket_interactions, [2, 0, 30, 0]);
    assert_eq!(decayed.eff_count_milli, 38);
    assert_eq!(decayed.eff_days_milli, 18);
    assert_eq!(t3r.band, Band::Moderate);

    let t4 = TieAlgo::T4.score(f.peer_id, &f.log, f.as_of);
    assert_eq!(t4.silent_days, 4);
    assert_eq!(t4.band, Band::Strong);
}

#[test]
fn the_demotion_boundaries_land_where_the_constants_say() {
    // 179 / 180 and 359 / 360, as fixtures rather than as unit tests, so the
    // ablation table shows the step rather than implying it.
    assert_eq!(band(&fixture("quiet_179_days"), TieAlgo::T4), Band::Strong);
    assert_eq!(
        band(&fixture("quiet_180_days"), TieAlgo::T4),
        Band::Moderate
    );
    assert_eq!(
        band(&fixture("dormant_359_days"), TieAlgo::T4),
        Band::Moderate
    );
    assert_eq!(band(&fixture("dormant_360_days"), TieAlgo::T4), Band::Weak);
}

// ---------------------------------------------------------------------------
// Properties that must hold whichever rule wins
// ---------------------------------------------------------------------------

#[test]
fn every_rule_agrees_about_the_counts_whatever_it_decides_about_the_band() {
    // The audit trail belongs to the evidence, not to the rule. If two rules
    // disagreed about how many messages there were, the user could not check
    // either of them.
    for f in testing::all() {
        let scores = score_all(f.peer_id, &f.log, f.as_of);
        for score in &scores[1..] {
            assert_eq!(
                score.interaction_count, scores[0].interaction_count,
                "{}",
                f.name
            );
            assert_eq!(score.outgoing_count, scores[0].outgoing_count, "{}", f.name);
            assert_eq!(score.incoming_count, scores[0].incoming_count, "{}", f.name);
            assert_eq!(
                score.active_day_count, scores[0].active_day_count,
                "{}",
                f.name
            );
            assert_eq!(
                score.conversation_count, scores[0].conversation_count,
                "{}",
                f.name
            );
            assert_eq!(
                score.first_contact_unix, scores[0].first_contact_unix,
                "{}",
                f.name
            );
            assert_eq!(
                score.last_contact_unix, scores[0].last_contact_unix,
                "{}",
                f.name
            );
            assert_eq!(score.any_direct, scores[0].any_direct, "{}", f.name);
            assert_eq!(score.silent_days, scores[0].silent_days, "{}", f.name);
        }
    }
}

#[test]
fn one_sided_ties_are_weak_under_every_rule() {
    for f in testing::all() {
        let scores = score_all(f.peer_id, &f.log, f.as_of);
        if scores[0].is_reciprocal() {
            continue;
        }
        for score in scores {
            assert_eq!(
                score.band,
                Band::Weak,
                "{} / {}",
                f.name,
                score.algorithm_id
            );
        }
    }
}

#[test]
fn group_only_ties_are_never_strong_under_any_candidate() {
    for f in testing::all() {
        let scores = score_all(f.peer_id, &f.log, f.as_of);
        if !scores[0].is_group_only() {
            continue;
        }
        for algo in TieAlgo::CANDIDATES {
            assert_ne!(band(&f, algo), Band::Strong, "{} / {}", f.name, algo.id());
        }
    }
}

#[test]
fn forgetting_evidence_can_only_lower_a_band() {
    // The orphan protocol: drop rows and re-run, never migrate state. This has
    // to hold for the weighted rule too, which is not obvious — removing rows
    // changes the buckets as well as the totals.
    for f in testing::all() {
        for algo in TieAlgo::ALL {
            let before = algo.score(f.peer_id, &f.log, f.as_of).band;
            for keep in [0, 1, f.log.len() / 3, f.log.len() / 2].map(|n| n.min(f.log.len())) {
                let after = algo.score(f.peer_id, &f.log[..keep], f.as_of).band;
                assert!(
                    after.rank() <= before.rank(),
                    "{} / {} rose from {before:?} to {after:?} after forgetting",
                    f.name,
                    algo.id()
                );
            }
        }
    }
}

#[test]
fn input_order_does_not_change_any_score() {
    for f in testing::all() {
        if f.log.len() < 2 {
            continue;
        }
        let shuffled: Vec<_> = (0..f.log.len())
            .map(|i| f.log[(i * 7 + 5) % f.log.len()].clone())
            .collect();
        assert_eq!(
            score_all(f.peer_id, &shuffled, f.as_of),
            score_all(f.peer_id, &f.log, f.as_of),
            "{}",
            f.name
        );
    }
}

#[test]
fn the_product_path_contains_no_floating_point() {
    // C2 in the shared brief asks for no float instability. The strongest form
    // of that promise is that there is no float at all, so this reads the
    // sources rather than trusting the claim. `tombstones.rs` is exempt and
    // absent from the list: it is `#[cfg(test)]` and its float is the exhibit.
    for (name, source) in [
        ("lib.rs", include_str!("../src/lib.rs")),
        ("types.rs", include_str!("../src/types.rs")),
        ("constants.rs", include_str!("../src/constants.rs")),
        ("gate.rs", include_str!("../src/gate.rs")),
        ("t0.rs", include_str!("../src/t0.rs")),
        ("t3.rs", include_str!("../src/t3.rs")),
        ("t3r.rs", include_str!("../src/t3r.rs")),
        ("t4.rs", include_str!("../src/t4.rs")),
        ("testing.rs", include_str!("../src/testing.rs")),
    ] {
        for banned in ["f64", "f32"] {
            assert!(
                !source.contains(banned),
                "{name} mentions {banned}: the product path must be integer-only"
            );
        }
    }
}

#[test]
fn a_large_ego_network_is_scored_in_one_pass() {
    // C6: linear in evidence, not in evidence times peers. 100 000 rows across
    // 1 000 peers, scored by every rule. No timing assertion — a wall-clock
    // bound in a test is a flake — but a quadratic implementation would not
    // finish this in debug mode.
    let mut log = Vec::with_capacity(100_000);
    for row in 0..100_000i64 {
        let peer = (row % 1_000) as u64;
        log.push(soul_algo_tie::Interaction {
            peer_id: peer,
            outgoing: row % 2 == 0,
            occurred_at_unix: testing::at(row % 500, row % 14),
            venue_direct: row % 7 != 0,
            conversation_id: peer,
        });
    }
    for algo in TieAlgo::ALL {
        let scored = algo.score_ego_network(&log, testing::AS_OF_2026_08_24);
        assert_eq!(scored.len(), 1_000);
        assert_eq!(
            scored.iter().map(|(_, s)| s.interaction_count).sum::<u64>(),
            100_000
        );
    }
}
