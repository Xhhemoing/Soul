//! What the user is allowed to be told.
//!
//! PRODUCT_LOCK requires that a band can be explained in Chinese, that no
//! score or percentile is ever shown, and that the explanation is checkable
//! against counts the user can recount. R1-SYNTHESIS §下轮攻坚重点 4 freezes
//! the vocabulary: 次数、天数、是否互惠、是否私聊、距今多少天, and nothing else.
//!
//! Round 3 adds one requirement on top, and it is the reason the split counts
//! exist at all: the default rule bands on the one-to-one exchanges, so its
//! sentence must say how many of those there were **separately** from how many
//! happened in a group. A user told only "32 exchanges" cannot check a Weak
//! band; a user told "2 one to one, 30 in the group" can.
//!
//! These are properties over every fixture rather than spot checks on one
//! string, because the failure mode is a rule quietly acquiring a sentence
//! nobody reviewed.

use soul_algo_tie::testing;
use soul_algo_tie::{explain_zh, Band, TieAlgo};

/// Every explanation of every fixture under every rule.
fn all_explanations() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for f in testing::all() {
        for algo in TieAlgo::ALL {
            let score = algo.score(f.peer_id, &f.log, f.as_of);
            out.push((format!("{}/{}", f.name, algo.id()), algo.explain_zh(&score)));
        }
    }
    out
}

fn t4d_text(f: &testing::Fixture) -> String {
    explain_zh(&TieAlgo::T4D.score(f.peer_id, &f.log, f.as_of))
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
    // and "Granovetter" without having to keep a list of banned words up to
    // date, and it keeps the internal rule identifier off the user's screen.
    for (case, text) in all_explanations() {
        assert!(
            !text.chars().any(|c| c.is_ascii_alphabetic()),
            "{case} contained Latin letters: {text}"
        );
    }
}

#[test]
fn no_explanation_makes_a_clinical_or_quantitative_claim() {
    // Bands are evidence, not measurement. Nothing may look like a rating, and
    // nothing may look like a diagnosis.
    for (case, text) in all_explanations() {
        for banned in [
            "分数",
            "评分",
            "百分",
            "得分",
            "指数",
            "打分",
            "权重",
            "%",
            "％",
            "抑郁",
            "焦虑",
            "人格",
            "障碍",
            "倾向性",
        ] {
            assert!(!text.contains(banned), "{case} contained {banned}: {text}");
        }
    }
}

#[test]
fn neither_rule_ever_prints_a_fraction() {
    // Both survivors are "you can do this arithmetic in your head" rules. The
    // promise is stronger than "no floats internally": no decimal point ever
    // reaches the user, so every number on screen is something countable. The
    // decay vocabulary that would have needed one is in the tombstones.
    for (case, text) in all_explanations() {
        assert!(!text.contains('.'), "{case} printed a decimal: {text}");
        assert!(!text.contains('/'), "{case}: {text}");
        for banned in ["折算", "半次", "四分之一"] {
            assert!(
                !text.contains(banned),
                "{case} borrowed the decay vocabulary: {text}"
            );
        }
    }
}

#[test]
fn every_explanation_names_the_numbers_the_user_can_recount() {
    // 次数、天数、距今多少天 and a date, in every non-empty case, under both
    // rules. The audit path: whatever the rule did internally, the sentence
    // repeats the counts.
    for f in testing::all() {
        for algo in TieAlgo::ALL {
            let score = algo.score(f.peer_id, &f.log, f.as_of);
            if score.is_empty() {
                continue;
            }
            let text = algo.explain_zh(&score);
            let case = format!("{}/{}", f.name, algo.id());
            assert!(
                text.contains(&score.interaction_count.to_string()),
                "{case} did not name the count: {text}"
            );
            assert!(
                text.contains(&format!("分布在 {} 天", score.active_day_count)),
                "{case} did not name the active days: {text}"
            );
            assert!(
                text.contains(&format!("距今 {} 天", score.silent_days)),
                "{case} did not say how long ago: {text}"
            );
            assert!(text.contains("年"), "{case} did not name a date: {text}");
        }
    }
}

#[test]
fn the_default_rule_names_the_one_to_one_count_apart_from_the_group_count() {
    // The Round 3 requirement, as a property over every fixture: both numbers,
    // always, including when one of them is zero. Without this the user cannot
    // check the number the band was decided on.
    for f in testing::all() {
        let score = TieAlgo::T4D.score(f.peer_id, &f.log, f.as_of);
        if score.is_empty() {
            continue;
        }
        let text = t4d_text(&f);
        assert!(
            text.contains(&format!("一对一 {} 次", score.direct_count())),
            "{} did not name the one-to-one count: {text}",
            f.name
        );
        assert!(
            text.contains(&format!("群里 {} 次", score.group_count())),
            "{} did not name the group count: {text}",
            f.name
        );
        assert!(
            text.contains(&format!("你发出 {} 次", score.direct_out_count)),
            "{} did not split the one-to-one direction: {text}",
            f.name
        );
    }
}

#[test]
fn the_rollback_rule_keeps_reporting_one_total() {
    // T4 is unchanged, including its sentence: it bands on the total, so it
    // says the total. The two rules must not read the same, or a reviewer
    // comparing screenshots cannot tell which one produced the band.
    for f in testing::all() {
        let t4 = TieAlgo::T4.score(f.peer_id, &f.log, f.as_of);
        let text = TieAlgo::T4.explain_zh(&t4);
        if t4.is_empty() {
            continue;
        }
        assert!(
            text.contains(&format!(
                "你们一共有 {} 次往来（你发出 {} 次，对方发来 {} 次）",
                t4.interaction_count, t4.outgoing_count, t4.incoming_count
            )),
            "{}: {text}",
            f.name
        );
        assert_ne!(text, t4d_text(&f), "{}", f.name);
    }
}

#[test]
fn the_group_heavy_tie_is_told_exactly_why_it_is_weak() {
    // The headline fixture's sentence, which is the part of the change the
    // user actually sees.
    let f = testing::group_heavy_plus_one_direct_each_way();
    let text = t4d_text(&f);
    assert!(text.contains("你们一共有 32 次往来"), "{text}");
    assert!(text.contains("一对一 2 次"), "{text}");
    assert!(text.contains("群里 30 次"), "{text}");
    assert!(text.contains("不到 3 次"), "{text}");
    assert!(text.contains("不算进这一档"), "{text}");
    assert!(text.contains("弱联系"), "{text}");

    // And the rule it replaces tells the same user they are close friends.
    let t4 = TieAlgo::T4.explain_zh(&TieAlgo::T4.score(f.peer_id, &f.log, f.as_of));
    assert!(t4.contains("强联系"), "{t4}");
}

#[test]
fn the_group_only_tie_is_told_it_was_never_a_private_conversation() {
    let f = testing::group_only_50();
    let text = t4d_text(&f);
    assert!(text.contains("一对一 0 次"), "{text}");
    assert!(text.contains("从来没有单独聊过"), "{text}");
    assert!(text.contains("弱联系"), "{text}");
    // The rollback rule stops one rung higher and says so in its own words.
    let t4 = TieAlgo::T4.explain_zh(&TieAlgo::T4.score(f.peer_id, &f.log, f.as_of));
    assert!(t4.contains("没有单独聊过"), "{t4}");
    assert!(t4.contains("中等联系"), "{t4}");
}

#[test]
fn a_private_only_tie_is_not_told_about_a_group_it_was_never_in() {
    // The split names the group count in the tally line — that number is 0 and
    // the user can check it — but no purely private tie gets the sentence about
    // group traffic not counting.
    for f in testing::private_only() {
        let text = t4d_text(&f);
        assert!(!text.contains("不算进这一档"), "{}: {text}", f.name);
    }
}

#[test]
fn the_default_rule_says_how_long_the_silence_was_and_what_it_cost() {
    let f = testing::quiet_200_days();
    let score = TieAlgo::T4D.score(f.peer_id, &f.log, f.as_of);
    let text = t4d_text(&f);
    assert_eq!(score.band, Band::Moderate);
    assert!(text.contains("距今 200 天"), "{text}");
    assert!(text.contains("不少于 180 天没有联系"), "{text}");
    assert!(text.contains("本来可以算强联系"), "{text}");
    assert!(text.contains("往下降到中等联系"), "{text}");
    assert!(text.contains("又聊起来"), "{text}");
}

#[test]
fn the_tie_kept_alive_by_a_group_message_reads_as_a_live_tie() {
    // The honest reading of the any-venue recency clock: the sentence says the
    // last exchange was yesterday, and it says only one of the 13 exchanges
    // was in the group. A user who thinks that is the wrong band has
    // everything needed to see why it happened.
    let f = testing::dormant_direct_group_ping_yesterday();
    let text = t4d_text(&f);
    assert!(text.contains("距今 1 天"), "{text}");
    assert!(text.contains("一对一 12 次"), "{text}");
    assert!(text.contains("群里 1 次"), "{text}");
    assert!(text.contains("强联系"), "{text}");
    assert!(text.contains("不用往下降"), "{text}");

    // Its control, one group message poorer, is told what the silence cost.
    let quiet = t4d_text(&testing::dormant_direct_no_ping());
    assert!(quiet.contains("距今 300 天"), "{quiet}");
    assert!(quiet.contains("往下降到中等联系"), "{quiet}");
}

#[test]
fn the_dormant_tie_reads_differently_from_the_live_one() {
    let dormant = t4d_text(&testing::dormant_2019());
    let live = t4d_text(&testing::lilei_12());
    assert_ne!(dormant, live);
    assert!(dormant.contains("弱联系"), "{dormant}");
    assert!(dormant.contains("2632 天"), "{dormant}");
    assert!(dormant.contains("2019"), "{dormant}");
    assert!(live.contains("强联系"), "{live}");
}

#[test]
fn an_empty_tie_says_so_plainly() {
    let f = testing::empty();
    for algo in TieAlgo::ALL {
        let text = algo.explain_zh(&algo.score(f.peer_id, &f.log, f.as_of));
        assert!(text.contains("还没有"), "{text}");
        assert!(!text.contains("0 次"), "{text}");
        assert!(!text.contains("1970"), "{text}");
    }
}

#[test]
fn a_one_sided_tie_is_told_which_side_is_missing() {
    let f = testing::one_sided_100();
    for algo in TieAlgo::ALL {
        let text = algo.explain_zh(&algo.score(f.peer_id, &f.log, f.as_of));
        assert!(text.contains("对方一次也没有回过"), "{}: {text}", algo.id());
        assert!(text.contains("弱联系"), "{}: {text}", algo.id());
    }
}

#[test]
fn a_tie_that_is_private_but_one_sided_is_told_that_specifically() {
    // Distinct from the case above: the peer *does* answer, but only in the
    // group. The default rule has to say that, or a Weak band next to a
    // reciprocal-looking count is unexplainable.
    let f = testing::group_heavy_plus_directs_one_way();
    let text = t4d_text(&f);
    assert!(text.contains("全是你发出的"), "{text}");
    assert!(text.contains("对方一次也没有单独回过你"), "{text}");
    assert!(text.contains("弱联系"), "{text}");
}

#[test]
fn every_explanation_ends_in_a_band_the_user_recognises() {
    for f in testing::all() {
        for algo in TieAlgo::ALL {
            let score = algo.score(f.peer_id, &f.log, f.as_of);
            let text = algo.explain_zh(&score);
            if score.is_empty() {
                continue;
            }
            assert!(
                text.contains(score.band.as_zh()),
                "{}/{} never named its band: {text}",
                f.name,
                algo.id()
            );
        }
    }
}
