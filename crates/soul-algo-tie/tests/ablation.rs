//! Does the venue latch in T4D pay for itself?
//!
//! fable-a's gate (EVAL_MATRIX §C8) is: a candidate must give the right band
//! on at least one adversarial fixture where the incumbent gives the wrong
//! one, and must not give the wrong band anywhere the incumbent was right.
//! Round 3's incumbent is T4, not T0. This file is that measurement.
//!
//! Three tables, in this order, because the order is the discipline:
//!
//! 1. [`TRUTH`] — what the band *should* be, per fixture. Judged from the
//!    product's own statements (R1-SYNTHESIS, R2-SYNTHESIS, fable-a's
//!    EVAL_MATRIX F01–F05) and from common sense about the tie, never from
//!    what a rule happens to output. Where the round has not agreed on an
//!    answer, the entry says `Unjudged` rather than inventing one.
//! 2. [`OBSERVED`] — what each rule actually says. Pinned exactly, so a change
//!    in any rule shows up as a diff in this file rather than as a silent
//!    shift in a recommendation.
//! 3. [`VIOLATIONS`] — the C8 result: which rule breaks which judged
//!    expectation. This is the table the recommendation in REPORT.md rests on.

use soul_algo_tie::testing::oracle::T0;
use soul_algo_tie::testing::{self, Fixture};
use soul_algo_tie::{score_both, Band, Detail, TieAlgo, TieAlgorithm};

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
    /// Strong), and how the Round 3 brief states the group-heavy cases: the
    /// failure is calling a colleague or a memory a close friend, and whether
    /// the right answer is Moderate or Weak is a taste question the fixtures
    /// cannot settle.
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
    // Goal 1's acceptance case, and the Round 3 brief's no-regression anchor.
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
    // P1-4: a tie nobody has touched in that long is not a current strong tie,
    // whatever the history says.
    ("quiet_180_days", Truth::NotStrong),
    ("quiet_200_days", Truth::NotStrong),
    // The contestable cell Round 2 flagged: 32 reciprocal one-to-one exchanges
    // over 12 active days with somebody the user spoke to four days ago. If
    // the graph cannot call that a close tie, it is describing an archive
    // rather than a life. Judged Strong, which is what killed T3R.
    ("revived_after_gap", Truth::Exactly(Band::Strong)),
    ("group_only_quiet_200", Truth::NotStrong),
    ("dormant_359_days", Truth::NotStrong),
    ("dormant_360_days", Truth::NotStrong),
    // EVAL_MATRIX F04, the headline case.
    ("dormant_2019", Truth::NotStrong),
    // R2-SYNTHESIS §潜在边界风险 2, the fixture the Round 3 brief requires:
    // a group fan-out plus one private hello each way is a colleague.
    ("group_heavy_plus_one_direct_each_way", Truth::NotStrong),
    ("group_heavy_plus_three_directs", Truth::NotStrong),
    ("group_heavy_plus_directs_one_way", Truth::NotStrong),
    // Deliberately unjudged, and the honest place to record the disagreement:
    // the Round 3 brief instructs that a group message yesterday prevents the
    // dormancy demotion, so Strong is the specified output — but no round has
    // agreed that a friendship last spoken to privately 300 days ago *is* a
    // strong tie. REPORT.md §任一场地的近因 argues both sides.
    ("dormant_direct_group_ping_yesterday", Truth::Unjudged),
    // Its control has no such excuse: 300 days of complete silence.
    ("dormant_direct_no_ping", Truth::NotStrong),
];

// ---------------------------------------------------------------------------
// 2. What each rule actually says
// ---------------------------------------------------------------------------

use Band::{Moderate as M, Strong as S, Weak as W};

/// `(fixture, T0, T4, T4D)`. T0 is the shipped baseline, kept as an oracle and
/// shown here for scale; the decision is between the last two columns.
const OBSERVED: &[(&str, Band, Band, Band)] = &[
    ("empty", W, W, W),
    ("single_inbound", W, W, W),
    ("lilei_12", S, S, S),
    ("afternoon_20", M, M, M),
    ("group_only_50", S, M, W),
    ("one_sided_100", W, W, W),
    ("flood_1000_in_one_day", M, M, M),
    ("steady_16_over_8_weeks", S, S, S),
    ("quiet_179_days", S, S, S),
    ("quiet_180_days", S, M, M),
    ("quiet_200_days", S, M, M),
    ("revived_after_gap", S, S, S),
    ("group_only_quiet_200", S, W, W),
    ("dormant_359_days", S, M, M),
    ("dormant_360_days", S, W, W),
    ("dormant_2019", S, W, W),
    ("group_heavy_plus_one_direct_each_way", S, S, W),
    ("group_heavy_plus_three_directs", S, S, M),
    ("group_heavy_plus_directs_one_way", S, S, W),
    ("dormant_direct_group_ping_yesterday", S, S, S),
    ("dormant_direct_no_ping", S, M, M),
];

// ---------------------------------------------------------------------------
// 3. The C8 result
// ---------------------------------------------------------------------------

/// Every fixture where a rule breaks a judged expectation.
///
/// Read this as: T4D breaks nothing, T4 breaks exactly the three group-heavy
/// cases R2-SYNTHESIS opened against it, and the shipped rule breaks eleven.
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
            "group_heavy_plus_one_direct_each_way",
            "group_heavy_plus_three_directs",
            "group_heavy_plus_directs_one_way",
            "dormant_direct_no_ping",
        ],
    ),
    (
        "T4",
        &[
            "group_heavy_plus_one_direct_each_way",
            "group_heavy_plus_three_directs",
            "group_heavy_plus_directs_one_way",
        ],
    ),
    ("T4D", &[]),
];

fn fixture(name: &str) -> Fixture {
    testing::all()
        .into_iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("no fixture named {name}"))
}

/// `[T0, T4, T4D]`, in the order [`OBSERVED`] lists them.
fn bands(f: &Fixture) -> [Band; 3] {
    let [t4, t4d] = score_both(f.peer_id, &f.log, f.as_of);
    [
        T0::score(f.peer_id, &f.log, f.as_of).band,
        t4.band,
        t4d.band,
    ]
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
    // The ablation table in REPORT.md is this array. If a rule changes, this is
    // the test that makes the document stale loudly rather than quietly.
    for &(name, t0, t4, t4d) in OBSERVED {
        let f = fixture(name);
        assert_eq!(bands(&f), [t0, t4, t4d], "{name}");
    }
}

#[test]
fn the_c8_violation_tables_are_exactly_these() {
    for &(algo_id, expected) in VIOLATIONS {
        let observed: Vec<&str> = testing::all()
            .iter()
            .filter(|f| {
                let got = match algo_id {
                    "T0" => T0::score(f.peer_id, &f.log, f.as_of).band,
                    _ => band(f, TieAlgo::from_id(algo_id).expect("known rule")),
                };
                truth_of(f.name).violated_by(got)
            })
            .map(|f| f.name)
            .collect();
        assert_eq!(observed, expected.to_vec(), "{algo_id}");
    }
}

#[test]
fn t4d_breaks_no_judged_expectation_anywhere() {
    // The headline of the round, stated on its own so it cannot be lost in a
    // table: on every fixture with an agreed answer, the direct-count rule
    // gives that answer.
    for f in testing::all() {
        let got = band(&f, TieAlgo::T4D);
        assert!(
            !truth_of(f.name).violated_by(got),
            "T4D said {got:?} on {}, expected {:?}",
            f.name,
            truth_of(f.name)
        );
    }
}

// ---------------------------------------------------------------------------
// The cases the brief names, one test each
// ---------------------------------------------------------------------------

#[test]
fn group_heavy_plus_one_direct_each_way_must_not_be_strong_and_is_weak() {
    // The fixture the Round 3 brief requires. Thirty reciprocal group
    // exchanges over ten days plus exactly one private message in each
    // direction. Two private exchanges is under the Moderate bar of three, so
    // the answer is Weak — and the requirement is the weaker one, that it is
    // not Strong.
    let f = fixture("group_heavy_plus_one_direct_each_way");
    let score = TieAlgo::T4D.score(f.peer_id, &f.log, f.as_of);

    assert_eq!(score.interaction_count, 32);
    assert_eq!(score.direct_count(), 2);
    assert_eq!(score.direct_out_count, 1);
    assert_eq!(score.direct_in_count, 1);
    assert_eq!(score.group_count(), 30);
    assert_eq!(score.active_day_count, 10);
    assert_eq!(score.direct_active_day_count, 1);
    assert!(
        score.is_direct_reciprocal(),
        "both sides did write privately — the gate that stops this is the count, not reciprocity"
    );
    assert_ne!(score.band, Band::Strong);
    assert_eq!(score.band, Band::Weak);

    // Nothing was demoted: the counts stage never got above Weak, and the tie
    // is two days old.
    assert_eq!(score.silent_days, 2);
    assert_eq!(
        score.detail,
        Detail::Demoted {
            band_before: Band::Weak
        }
    );

    // The rule it replaces, on the same evidence.
    assert_eq!(band(&f, TieAlgo::T4), Band::Strong);
}

#[test]
fn lilei_12_is_strong_under_both_rules() {
    // The zero-regression anchor the brief names: the tie Goal 1 was accepted
    // on is still a close tie under the default rule.
    let f = fixture("lilei_12");
    assert_eq!(bands(&f), [S, S, S]);
    let score = TieAlgo::T4D.score(f.peer_id, &f.log, f.as_of);
    assert_eq!(score.direct_count(), 12);
    assert_eq!(score.direct_active_day_count, 6);
    assert_eq!(score.group_count(), 0);
    assert_eq!(score.band, Band::Strong);
}

#[test]
fn three_private_exchanges_are_what_separates_the_two_group_heavy_fixtures() {
    // The pair that shows T4D moved the bar rather than banning group-heavy
    // ties: same fan-out, one more private message each, one band up.
    let two = fixture("group_heavy_plus_one_direct_each_way");
    let three = fixture("group_heavy_plus_three_directs");
    assert_eq!(band(&two, TieAlgo::T4D), Band::Weak);
    assert_eq!(band(&three, TieAlgo::T4D), Band::Moderate);
    assert_eq!(band(&two, TieAlgo::T4), band(&three, TieAlgo::T4));
}

#[test]
fn private_messages_that_only_go_one_way_stay_weak_under_the_default_rule() {
    let f = fixture("group_heavy_plus_directs_one_way");
    let score = TieAlgo::T4D.score(f.peer_id, &f.log, f.as_of);
    assert_eq!(score.direct_out_count, 8);
    assert_eq!(score.direct_in_count, 0);
    assert!(score.is_reciprocal(), "the group traffic goes both ways");
    assert!(!score.is_direct_reciprocal());
    assert_eq!(score.band, Band::Weak);
    assert_eq!(band(&f, TieAlgo::T4), Band::Strong);
}

#[test]
fn a_group_message_yesterday_is_what_separates_the_two_dormant_fixtures() {
    // The brief's other instruction, measured on a pair that differs by one
    // row. Both rules behave identically here, because the recency clock is
    // shared.
    let pinged = fixture("dormant_direct_group_ping_yesterday");
    let quiet = fixture("dormant_direct_no_ping");
    for algo in TieAlgo::ALL {
        assert_eq!(band(&pinged, algo), Band::Strong, "{}", algo.id());
        assert_eq!(band(&quiet, algo), Band::Moderate, "{}", algo.id());
    }
    let score = TieAlgo::T4D.score(pinged.peer_id, &pinged.log, pinged.as_of);
    assert_eq!(score.silent_days, 1);
    assert_eq!(score.direct_count(), 12);
    assert_eq!(score.group_count(), 1);
    // The private conversation itself is 300 days old, and the score says so.
    assert_eq!(
        soul_algo_tie::age_days(score.last_direct_contact_unix, score.as_of_unix),
        300
    );
}

#[test]
fn f04_dormant_2019_is_still_refused_by_both_rules() {
    let f = fixture("dormant_2019");
    assert_eq!(bands(&f), [S, W, W]);
    for algo in TieAlgo::ALL {
        let score = algo.score(f.peer_id, &f.log, f.as_of);
        assert_eq!(score.silent_days, 2_632);
        let Detail::Demoted { band_before } = score.detail else {
            panic!("expected demotion detail");
        };
        assert_eq!(band_before, Band::Strong, "{}", algo.id());
        assert_eq!(score.band, Band::Weak, "{}", algo.id());
        // The counts survive the demotion: the evidence is still there to be
        // recounted, only the claim about the present is withdrawn.
        assert_eq!(score.interaction_count, 20);
        assert_eq!(score.active_day_count, 10);
    }
}

#[test]
fn the_demotion_boundaries_land_where_the_constants_say_under_both_rules() {
    for (name, expected) in [
        ("quiet_179_days", Band::Strong),
        ("quiet_180_days", Band::Moderate),
        ("dormant_359_days", Band::Moderate),
        ("dormant_360_days", Band::Weak),
    ] {
        let f = fixture(name);
        for algo in TieAlgo::ALL {
            assert_eq!(band(&f, algo), expected, "{name} / {}", algo.id());
        }
    }
}

#[test]
fn the_two_rules_disagree_on_exactly_these_fixtures() {
    let disagreements: Vec<(&str, Band, Band)> = testing::all()
        .iter()
        .map(|f| (f.name, band(f, TieAlgo::T4), band(f, TieAlgo::T4D)))
        .filter(|(_, t4, t4d)| t4 != t4d)
        .collect();
    assert_eq!(
        disagreements,
        vec![
            // The cost of the change: a project-channel colleague reads Weak
            // rather than Moderate, because group traffic no longer counts
            // towards a band at all.
            ("group_only_50", M, W),
            // The three the change was made for.
            ("group_heavy_plus_one_direct_each_way", S, W),
            ("group_heavy_plus_three_directs", S, M),
            ("group_heavy_plus_directs_one_way", S, W),
        ]
    );
}

#[test]
fn the_two_rules_agree_on_every_fixture_with_no_group_rows() {
    // The Round 3 acceptance question, as a test rather than as a claim: T4D
    // may only differ from T4 where a group message is involved.
    for f in testing::private_only() {
        assert_eq!(
            band(&f, TieAlgo::T4),
            band(&f, TieAlgo::T4D),
            "{} regressed",
            f.name
        );
    }
}

// ---------------------------------------------------------------------------
// Properties that must hold whichever rule wins
// ---------------------------------------------------------------------------

#[test]
fn both_rules_agree_about_the_counts_whatever_they_decide_about_the_band() {
    // The audit trail belongs to the evidence, not to the rule. If two rules
    // disagreed about how many messages there were, the user could not check
    // either of them.
    for f in testing::all() {
        let [t4, t4d] = score_both(f.peer_id, &f.log, f.as_of);
        assert_eq!(t4.interaction_count, t4d.interaction_count, "{}", f.name);
        assert_eq!(t4.direct_out_count, t4d.direct_out_count, "{}", f.name);
        assert_eq!(t4.direct_in_count, t4d.direct_in_count, "{}", f.name);
        assert_eq!(t4.group_out_count, t4d.group_out_count, "{}", f.name);
        assert_eq!(t4.group_in_count, t4d.group_in_count, "{}", f.name);
        assert_eq!(t4.active_day_count, t4d.active_day_count, "{}", f.name);
        assert_eq!(
            t4.direct_active_day_count, t4d.direct_active_day_count,
            "{}",
            f.name
        );
        assert_eq!(t4.conversation_count, t4d.conversation_count, "{}", f.name);
        assert_eq!(t4.first_contact_unix, t4d.first_contact_unix, "{}", f.name);
        assert_eq!(t4.last_contact_unix, t4d.last_contact_unix, "{}", f.name);
        assert_eq!(
            t4.last_direct_contact_unix, t4d.last_direct_contact_unix,
            "{}",
            f.name
        );
        assert_eq!(t4.silent_days, t4d.silent_days, "{}", f.name);
    }
}

#[test]
fn one_sided_ties_are_weak_under_both_rules() {
    for f in testing::all() {
        let scores = score_both(f.peer_id, &f.log, f.as_of);
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
fn group_only_ties_are_never_strong_under_either_rule() {
    for f in testing::all() {
        let scores = score_both(f.peer_id, &f.log, f.as_of);
        if !scores[0].is_group_only() {
            continue;
        }
        for score in scores {
            assert_ne!(
                score.band,
                Band::Strong,
                "{} / {}",
                f.name,
                score.algorithm_id
            );
        }
    }
}

#[test]
fn forgetting_evidence_can_only_lower_a_band() {
    // The orphan protocol: drop rows and re-run, never migrate state.
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
            score_both(f.peer_id, &shuffled, f.as_of),
            score_both(f.peer_id, &f.log, f.as_of),
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
        ("recency.rs", include_str!("../src/recency.rs")),
        ("t4.rs", include_str!("../src/t4.rs")),
        ("t4d.rs", include_str!("../src/t4d.rs")),
        ("testing/mod.rs", include_str!("../src/testing/mod.rs")),
        (
            "testing/oracle.rs",
            include_str!("../src/testing/oracle.rs"),
        ),
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
    // 1 000 peers, scored by both rules. No timing assertion — a wall-clock
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
