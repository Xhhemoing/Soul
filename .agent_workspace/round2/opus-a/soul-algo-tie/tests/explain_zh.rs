//! What the user is allowed to be told.
//!
//! PRODUCT_LOCK requires that a band can be explained in Chinese, that no
//! score or percentile is ever shown, and that the explanation is checkable
//! against counts the user can recount. R1-SYNTHESIS §下轮攻坚重点 4 freezes
//! the vocabulary: 次数、天数、是否互惠、是否私聊、距今多少天, and nothing else.
//!
//! These are properties over every fixture and every rule rather than spot
//! checks on one string, because the failure mode is a rule quietly acquiring
//! a sentence nobody reviewed.

use soul_algo_tie::testing;
use soul_algo_tie::{Band, TieAlgo};

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
fn the_integer_rules_never_print_a_fraction() {
    // T3 and T4 are the "you can do this arithmetic in your head" rules. The
    // promise is stronger than "no floats internally": no decimal point ever
    // reaches the user, so every number on screen is something countable.
    for f in testing::all() {
        for algo in [TieAlgo::T0, TieAlgo::T3, TieAlgo::T4] {
            let text = algo.explain_zh(&algo.score(f.peer_id, &f.log, f.as_of));
            assert!(
                !text.contains('.'),
                "{}/{} printed a decimal: {text}",
                f.name,
                algo.id()
            );
            assert!(!text.contains('/'), "{}/{}: {text}", f.name, algo.id());
            for banned in ["折算", "半次", "四分之一"] {
                assert!(
                    !text.contains(banned),
                    "{}/{} borrowed the decay vocabulary: {text}",
                    f.name,
                    algo.id()
                );
            }
        }
    }
}

#[test]
fn every_explanation_names_the_numbers_the_user_can_recount() {
    // 次数、天数、距今多少天 and a date, in every non-empty case, under every
    // rule. The audit path: whatever the rule did internally, the sentence
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
fn the_candidates_say_whether_it_was_private_and_whether_both_sides_spoke() {
    // The two booleans that decide the band in this family have to be in the
    // sentence, or the user cannot tell why a group tie stopped at Moderate.
    for f in testing::all() {
        for algo in TieAlgo::CANDIDATES {
            let score = algo.score(f.peer_id, &f.log, f.as_of);
            if score.is_empty() {
                continue;
            }
            let text = algo.explain_zh(&score);
            let case = format!("{}/{}", f.name, algo.id());
            if score.is_reciprocal() {
                assert!(
                    text.contains("一对一") || text.contains("群里"),
                    "{case} did not say where it happened: {text}"
                );
                assert!(
                    text.contains("双方"),
                    "{case} did not say both sides spoke: {text}"
                );
            } else {
                assert!(
                    text.contains("一次也没有回过"),
                    "{case} did not say who never answered: {text}"
                );
            }
        }
    }
}

#[test]
fn the_group_only_tie_is_told_it_is_group_only() {
    let f = testing::group_only_50();
    for algo in TieAlgo::CANDIDATES {
        let text = algo.explain_zh(&algo.score(f.peer_id, &f.log, f.as_of));
        assert!(text.contains("没有单独聊过"), "{}: {text}", algo.id());
        assert!(text.contains("中等联系"), "{}: {text}", algo.id());
    }
    // The shipped rule cannot say any of this, because it never looked.
    let t0 = TieAlgo::T0.explain_zh(&TieAlgo::T0.score(f.peer_id, &f.log, f.as_of));
    assert!(!t0.contains("群里"), "{t0}");
    assert!(t0.contains("强联系"), "{t0}");
}

#[test]
fn t3r_shows_its_arithmetic_as_exact_quarters() {
    // The decay rule's bargain: it may use fractions, but only ones that
    // terminate, and it must show the bucket counts they came from so the user
    // can redo the sum.
    let f = testing::revived_after_gap();
    let text = TieAlgo::T3R.explain_zh(&TieAlgo::T3R.score(f.peer_id, &f.log, f.as_of));
    assert!(text.contains("按 1 次计"), "{text}");
    assert!(text.contains("按半次计"), "{text}");
    assert!(text.contains("按四分之一计"), "{text}");
    // 2 whole + 30 quarters = 9.5 effective exchanges, against a bar of 10.
    assert!(text.contains("2 次按 1 次计"), "{text}");
    assert!(text.contains("30 次按四分之一计"), "{text}");
    assert!(text.contains("相当于 9.5 次"), "{text}");
    assert!(text.contains("中等联系"), "{text}");
    // Two decimal places at most, and only ever on a quarter boundary.
    for fragment in text.split('.').skip(1) {
        let decimals: String = fragment
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        assert!(
            ["25", "5", "75"].contains(&decimals.as_str()),
            "{text} printed .{decimals}"
        );
    }
}

#[test]
fn t4_says_how_long_the_silence_was_and_what_it_cost() {
    let f = testing::quiet_200_days();
    let score = TieAlgo::T4.score(f.peer_id, &f.log, f.as_of);
    let text = TieAlgo::T4.explain_zh(&score);
    assert_eq!(score.band, Band::Moderate);
    assert!(text.contains("距今 200 天"), "{text}");
    assert!(text.contains("不少于 180 天没有联系"), "{text}");
    assert!(text.contains("本来可以算强联系"), "{text}");
    assert!(text.contains("往下降到中等联系"), "{text}");
    assert!(text.contains("又聊起来"), "{text}");
}

#[test]
fn the_dormant_tie_reads_differently_under_the_rule_that_cannot_see_time() {
    // T3 says strong, T4 says the traffic is seven years old. The two
    // explanations must not read the same, or the user cannot tell which rule
    // they are looking at without being shown an identifier.
    let f = testing::dormant_2019();
    let t3 = TieAlgo::T3.explain_zh(&TieAlgo::T3.score(f.peer_id, &f.log, f.as_of));
    let t4 = TieAlgo::T4.explain_zh(&TieAlgo::T4.score(f.peer_id, &f.log, f.as_of));
    let t3r = TieAlgo::T3R.explain_zh(&TieAlgo::T3R.score(f.peer_id, &f.log, f.as_of));
    assert_ne!(t3, t4);
    assert!(t3.contains("强联系"), "{t3}");
    assert!(t4.contains("弱联系"), "{t4}");
    assert!(t4.contains("2632 天"), "{t4}");
    assert!(t3r.contains("弱联系"), "{t3r}");
    assert!(t3r.contains("相当于 0 次"), "{t3r}");
    // All three name the same date, because all three describe the same
    // evidence.
    for text in [&t3, &t3r, &t4] {
        assert!(text.contains("2019"), "{text}");
    }
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
fn t3r_warns_that_the_tie_has_gone_quiet() {
    // fable-a's CANDIDATE_SPEC 展示补充: past 180 days the edge carries a
    // sentence saying so. T4 does not need it — its whole explanation is that
    // sentence.
    let quiet = testing::quiet_200_days();
    let fresh = testing::lilei_12();
    let quiet_text =
        TieAlgo::T3R.explain_zh(&TieAlgo::T3R.score(quiet.peer_id, &quiet.log, quiet.as_of));
    let fresh_text =
        TieAlgo::T3R.explain_zh(&TieAlgo::T3R.score(fresh.peer_id, &fresh.log, fresh.as_of));
    assert!(quiet_text.contains("最近 200 天没有往来"), "{quiet_text}");
    assert!(!fresh_text.contains("没有往来"), "{fresh_text}");
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
