//! The four candidates on the same evidence.
//!
//! Every case in the shared brief, asserted for all four rules at once, so the
//! places where the candidates disagree are written down rather than argued
//! about. Where a disagreement is the point of a candidate, the assertion says
//! so in a comment.

use soul_algo_tie::testing::{
    direct, dormant_since_2019, empty, group, group_only_50, lilei_12_over_6_days,
    one_sided_100_outbound, single_inbound, twenty_in_one_afternoon, DAY, NOW_2026_08_24,
};
use soul_algo_tie::{score_all, Band, Interaction, TieAlgo};

/// The band each rule gives, in `TieAlgo::ALL` order.
fn bands(peer_id: u64, log: &[Interaction], now: i64) -> [Band; 4] {
    score_all(peer_id, log, now).map(|score| score.band)
}

#[test]
fn nothing_observed_is_weak_everywhere() {
    let (log, now) = empty();
    assert_eq!(bands(1, &log, now), [Band::Weak; 4]);
    for score in score_all(1, &log, now) {
        assert_eq!(score.interaction_count, 0);
        assert_eq!(score.outgoing_count, 0);
        assert_eq!(score.incoming_count, 0);
        assert_eq!(score.conversation_count, 0);
        assert_eq!(score.active_day_count, 0);
        assert_eq!(score.first_contact_unix, 0);
        assert_eq!(score.last_contact_unix, 0);
    }
}

#[test]
fn one_unanswered_message_is_weak_everywhere() {
    let (log, now) = single_inbound();
    assert_eq!(bands(1, &log, now), [Band::Weak; 4]);
    let score = &score_all(1, &log, now)[0];
    assert_eq!(score.incoming_count, 1);
    assert_eq!(score.outgoing_count, 0);
    assert_eq!(score.active_day_count, 1);
}

#[test]
fn the_goal_one_fixture_is_strong_under_every_candidate() {
    // 12 reciprocal one-to-one messages over 6 days, ending 3 days ago. The
    // shipped rule calls this Strong and none of the alternatives may quietly
    // demote the case the product was accepted on.
    let (log, now) = lilei_12_over_6_days();
    assert_eq!(bands(1, &log, now), [Band::Strong; 4]);
    for score in score_all(1, &log, now) {
        assert_eq!(score.interaction_count, 12);
        assert_eq!(score.outgoing_count, 6);
        assert_eq!(score.incoming_count, 6);
        assert_eq!(score.active_day_count, 6);
        assert_eq!(score.conversation_count, 1);
    }
}

#[test]
fn twenty_messages_in_one_afternoon_are_never_strong() {
    // The case Goal 1 wrote a comment about: one conversation is not a habit.
    // Every candidate has a span requirement of some kind, and all four stop
    // at Moderate here.
    let (log, now) = twenty_in_one_afternoon();
    let observed = bands(2, &log, now);
    assert!(!observed.contains(&Band::Strong), "{observed:?}");
    assert_eq!(observed, [Band::Moderate; 4]);
    assert_eq!(score_all(2, &log, now)[0].active_day_count, 1);
}

#[test]
fn fifty_group_messages_separate_the_venue_aware_rule_from_the_rest() {
    // The headline disagreement of the round. Fifty reciprocal messages spread
    // over fifty days, every one of them in a shared group chat: T0, T1 and T2
    // all call it Strong, and only T3 refuses.
    let (log, now) = group_only_50();
    assert_eq!(
        bands(3, &log, now),
        [Band::Strong, Band::Strong, Band::Strong, Band::Moderate]
    );
    // T1's group weight of 0.4 slows the climb but does not stop it: fifty
    // recent exchanges still clear a mass of 8.
    assert!(soul_algo_tie::T1::mass(3, &log, now) > 8.0);
}

#[test]
fn a_tie_that_stopped_in_2019_separates_the_recency_aware_rules() {
    // Twenty reciprocal one-to-one messages over ten days, seven years ago.
    // T0 and T3 have no notion of time passing and still call it Strong; T1
    // decays it to nothing; T2 drops one rung and lands on Moderate.
    let (log, now) = dormant_since_2019();
    assert_eq!(
        bands(4, &log, now),
        [Band::Strong, Band::Weak, Band::Moderate, Band::Strong]
    );
    assert!(soul_algo_tie::T1::mass(4, &log, now) < 0.001);
}

#[test]
fn a_hundred_unanswered_messages_stay_weak_everywhere() {
    // Reciprocity is a gate in all four rules, so volume cannot buy a band.
    let (log, now) = one_sided_100_outbound();
    assert_eq!(bands(5, &log, now), [Band::Weak; 4]);
    let score = &score_all(5, &log, now)[0];
    assert_eq!(score.interaction_count, 100);
    assert_eq!(score.active_day_count, 100);
    assert_eq!(score.incoming_count, 0);
}

#[test]
fn input_order_does_not_change_any_score() {
    // The evidence store makes no ordering promise, so the rules must not
    // depend on one. Shuffled with a fixed permutation, not a random one.
    let (log, now) = lilei_12_over_6_days();
    let shuffled: Vec<Interaction> = (0..log.len())
        .map(|i| log[(i * 7 + 5) % log.len()].clone())
        .collect();
    assert_ne!(shuffled, log);
    assert_eq!(score_all(1, &shuffled, now), score_all(1, &log, now));
}

#[test]
fn other_peers_in_the_log_are_ignored() {
    let (mut log, now) = lilei_12_over_6_days();
    let expected = score_all(1, &log, now);
    for peer in 2..=50u64 {
        for i in 0..20i64 {
            log.push(group(peer, i % 2 == 0, NOW_2026_08_24 - DAY * i, peer));
        }
    }
    assert_eq!(score_all(1, &log, now), expected);
}

#[test]
fn a_thousand_messages_in_one_day_do_not_buy_a_strong_band() {
    // Flooding, and the shape of an imported chat log that lost its dates. A
    // thousand messages is 100x the strong threshold and buys nothing, because
    // every candidate needs at least two active days before it will move.
    let log: Vec<Interaction> = (0..1000i64)
        .map(|i| direct(9, i % 2 == 0, NOW_2026_08_24 - DAY + i * 60, 9))
        .collect();
    let observed = bands(9, &log, NOW_2026_08_24);
    assert_eq!(
        observed,
        [
            Band::Moderate,
            Band::Moderate,
            Band::Moderate,
            Band::Moderate
        ]
    );
    assert_eq!(score_all(9, &log, NOW_2026_08_24)[0].active_day_count, 1);
}

#[test]
fn active_days_are_utc_and_do_not_follow_the_machine_clock() {
    // Six exchanges at 01:00 in a UTC+8 timezone, on six consecutive local
    // days. In UTC they are 17:00 on the six preceding days, so the count of
    // distinct days is the same either way — which is the property that keeps
    // a tie scored on the user's laptop equal to the same tie scored in CI.
    let local_midnight_utc8 = NOW_2026_08_24 - DAY * 10 - 8 * 3_600;
    let log: Vec<Interaction> = (0..6i64)
        .flat_map(|day| {
            [
                direct(7, true, local_midnight_utc8 + DAY * day + 3_600, 7),
                direct(7, false, local_midnight_utc8 + DAY * day + 2 * 3_600, 7),
            ]
        })
        .collect();
    let score = &score_all(7, &log, NOW_2026_08_24)[0];
    assert_eq!(score.active_day_count, 6);
    assert_eq!(score.band, Band::Strong);
}

#[test]
fn evidence_from_before_1970_does_not_collapse_into_one_day() {
    // Negative Unix seconds appear in imported archives with broken clocks.
    // Floor division keeps each of them on its own day; truncation would fold
    // the last day of 1969 into the first of 1970.
    let log: Vec<Interaction> = (1..=6i64)
        .map(|i| direct(8, i % 2 == 0, -DAY * i, 8))
        .collect();
    let score = &score_all(8, &log, 0)[0];
    assert_eq!(score.active_day_count, 6);
    assert_eq!(score.first_contact_unix, -DAY * 6);
    assert_eq!(score.last_contact_unix, -DAY);
}

#[test]
fn every_rule_agrees_about_the_counts_whatever_it_decides_about_the_band() {
    // The audit trail belongs to the evidence, not to the rule. If two
    // candidates ever disagreed about how many messages there were, the user
    // could not check either of them.
    for (name, peer_id, log, now) in soul_algo_tie::testing::all() {
        let scores = score_all(peer_id, &log, now);
        for score in &scores[1..] {
            assert_eq!(
                score.interaction_count, scores[0].interaction_count,
                "{name}"
            );
            assert_eq!(score.outgoing_count, scores[0].outgoing_count, "{name}");
            assert_eq!(score.incoming_count, scores[0].incoming_count, "{name}");
            assert_eq!(
                score.conversation_count, scores[0].conversation_count,
                "{name}"
            );
            assert_eq!(score.active_day_count, scores[0].active_day_count, "{name}");
            assert_eq!(
                score.first_contact_unix, scores[0].first_contact_unix,
                "{name}"
            );
            assert_eq!(
                score.last_contact_unix, scores[0].last_contact_unix,
                "{name}"
            );
        }
    }
}

#[test]
fn forgetting_the_evidence_can_only_lower_a_band() {
    // Any new rule has to survive the forgetting protocol: drop rows and
    // re-run, never migrate state. Removing the second half of a tie must not
    // make it look stronger under any candidate.
    let (log, now) = lilei_12_over_6_days();
    for algo in TieAlgo::ALL {
        let before = algo.score(1, &log, now).band;
        let after = algo.score(1, &log[..4], now).band;
        assert!(
            after.rank() <= before.rank(),
            "{} rose from {before:?} to {after:?} after forgetting",
            algo.id()
        );
    }
}
