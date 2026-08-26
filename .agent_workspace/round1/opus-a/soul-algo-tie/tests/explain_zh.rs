//! What the user is allowed to be told.
//!
//! PRODUCT_LOCK requires that a band can be explained in Chinese, that no
//! score or percentile is ever shown, and that the explanation is checkable
//! against counts the user can recount. These tests hold the four candidates
//! to that, as properties over every fixture rather than as spot checks on one
//! string.

use soul_algo_tie::testing::{
    dormant_since_2019, group_only_50, lilei_12_over_6_days, one_sided_100_outbound,
};
use soul_algo_tie::{score_all, Band, TieAlgo, T3};

/// Every explanation of every fixture under every rule.
fn all_explanations() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (name, peer_id, log, now) in soul_algo_tie::testing::all() {
        for algo in TieAlgo::ALL {
            let score = algo.score(peer_id, &log, now);
            out.push((format!("{name}/{}", algo.id()), algo.explain_zh(&score)));
        }
    }
    out
}

#[test]
fn every_explanation_is_written_in_chinese() {
    for (case, text) in all_explanations() {
        assert!(
            text.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)),
            "{case} had no Chinese in it: {text}"
        );
    }
}

#[test]
fn no_explanation_leaks_english_jargon() {
    // The blanket rule: an explanation contains digits, punctuation and
    // Chinese, and no Latin letters at all. That covers "score", "percentile"
    // and "RFM" without having to keep a list of banned words up to date, and
    // it also keeps the internal algorithm id off the user's screen.
    for (case, text) in all_explanations() {
        assert!(
            !text.chars().any(|c| c.is_ascii_alphabetic()),
            "{case} contained Latin letters: {text}"
        );
        for banned in ["score", "percentile", "RFM", "T0", "T1", "T2", "T3"] {
            assert!(
                !text.to_lowercase().contains(&banned.to_lowercase()),
                "{case} contained {banned}: {text}"
            );
        }
    }
}

#[test]
fn no_explanation_makes_a_clinical_or_quantitative_claim() {
    // Bands are evidence, not measurement. Nothing may look like a rating.
    for (case, text) in all_explanations() {
        for banned in ["分数", "评分", "百分", "得分", "指数", "打分", "%", "％"] {
            assert!(!text.contains(banned), "{case} contained {banned}: {text}");
        }
    }
}

#[test]
fn every_explanation_repeats_the_counts_the_user_can_check() {
    // The audit path: whatever the rule did internally, the sentence the user
    // reads names the number of exchanges, the number of days, and a date.
    for (name, peer_id, log, now) in soul_algo_tie::testing::all() {
        for algo in TieAlgo::ALL {
            let score = algo.score(peer_id, &log, now);
            if score.interaction_count == 0 {
                continue;
            }
            let text = algo.explain_zh(&score);
            let case = format!("{name}/{}", algo.id());
            assert!(
                text.contains(&score.interaction_count.to_string()),
                "{case} did not name the count: {text}"
            );
            assert!(text.contains("天"), "{case} did not mention days: {text}");
            assert!(text.contains("年"), "{case} did not name a date: {text}");
        }
    }
}

#[test]
fn the_strong_fixture_reads_as_a_reason_not_as_a_verdict() {
    let (log, now) = lilei_12_over_6_days();
    for algo in TieAlgo::ALL {
        let score = algo.score(1, &log, now);
        assert_eq!(score.band, Band::Strong);
        let text = algo.explain_zh(&score);
        assert!(text.contains("强联系"), "{}: {text}", algo.id());
        assert!(text.contains("12"), "{}: {text}", algo.id());
        assert!(text.contains("所以"), "{}: {text}", algo.id());
    }
}

#[test]
fn a_one_sided_tie_is_told_which_side_is_missing() {
    let (log, now) = one_sided_100_outbound();
    for algo in TieAlgo::ALL {
        let text = algo.explain_zh(&algo.score(5, &log, now));
        assert!(text.contains("对方"), "{}: {text}", algo.id());
        assert!(text.contains("弱联系"), "{}: {text}", algo.id());
    }
}

#[test]
fn the_dormant_tie_is_explained_differently_by_the_rules_that_disagree() {
    // T0 says strong, T1 says the traffic is old. The two explanations must
    // not read the same, or the user cannot tell which rule they are looking
    // at without being shown an identifier.
    let (log, now) = dormant_since_2019();
    let t0 = TieAlgo::T0.explain_zh(&TieAlgo::T0.score(4, &log, now));
    let t1 = TieAlgo::T1.explain_zh(&TieAlgo::T1.score(4, &log, now));
    assert_ne!(t0, t1);
    assert!(t0.contains("强联系"), "{t0}");
    assert!(t1.contains("弱联系"), "{t1}");
    assert!(t1.contains("很久"), "{t1}");
    // Both name the same date, because both are describing the same evidence.
    assert!(t0.contains("2019"), "{t0}");
    assert!(t1.contains("2019"), "{t1}");
}

#[test]
fn t3_can_name_the_venue_when_it_is_given_the_venue() {
    // The trait's explain_zh only sees a TieScore, which carries no venue, so
    // it has to hedge. Given the fact, T3 can say the thing that actually
    // decided the band. See REPORT.md: the fix is a field on the score.
    let (log, now) = group_only_50();
    let score = TieAlgo::T3.score(3, &log, now);
    let hedged = TieAlgo::T3.explain_zh(&score);
    let precise = T3::explain_zh_with_venue(&score, false);
    assert_ne!(hedged, precise);
    assert!(precise.contains("群里"), "{precise}");
    assert!(precise.contains("没有单独聊过"), "{precise}");
    assert!(
        !precise.chars().any(|c| c.is_ascii_alphabetic()),
        "{precise}"
    );
}

#[test]
fn an_empty_tie_says_so_plainly() {
    let (log, now) = soul_algo_tie::testing::empty();
    for score in score_all(1, &log, now) {
        for algo in TieAlgo::ALL {
            let text = algo.explain_zh(&score);
            assert!(text.contains("还没有"), "{text}");
            assert!(!text.contains("0 次"), "{text}");
        }
    }
}
