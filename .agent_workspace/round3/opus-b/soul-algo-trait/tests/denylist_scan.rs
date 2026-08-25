//! Every user-facing string this crate can produce, screened.
//!
//! The point of walking the fixtures rather than a hand-written list is that a
//! new sentence is screened the moment it is added, instead of the next time
//! somebody remembers to look.

use soul_algo_trait::a1::{a1_axis_state_with, A1Independence};
use soul_algo_trait::a2::{a2_render, PersonnelSummary, A2_STATEMENT_KEYS};
use soul_algo_trait::denylist::{
    assert_publishable, assert_publishable_about_peer, diagnostic_hit, numeric_rating_hit,
    peer_claim_hit, Screen, DIAGNOSTIC_DENYLIST,
};
use soul_algo_trait::fixtures::{axis_evidence_matrix, tie_score_matrix};
use soul_algo_trait::types::{AxisId, Band, Position};
use soul_algo_trait::{axis_vocabulary, A2_ALGORITHM_ID, A3_ALGORITHM_ID};

#[test]
fn the_brief_s_denylist_is_the_denylist() {
    // The exact ten words the Round 2 brief froze, in order.
    assert_eq!(
        DIAGNOSTIC_DENYLIST,
        [
            "抑郁",
            "焦虑",
            "障碍",
            "诊断",
            "人格障碍",
            "病",
            "score",
            "percentile",
            "百分位",
            "量表",
        ]
    );
}

#[test]
fn every_axis_string_is_publishable() {
    for text in axis_vocabulary() {
        assert_publishable(&text).unwrap_or_else(|hit| panic!("{hit:?} in {text}"));
    }
}

#[test]
fn every_a2_sentence_is_publishable_about_a_third_party() {
    for (name, mut base) in tie_score_matrix() {
        for band in [Band::None, Band::Weak, Band::Moderate, Band::Strong] {
            base.band = band;
            for text in a2_render(&base).texts() {
                assert_publishable_about_peer(text)
                    .unwrap_or_else(|hit| panic!("{name} at {band:?}: {hit:?} in {text}"));
            }
        }
    }
    assert_publishable_about_peer(PersonnelSummary::nothing_to_say_zh()).unwrap();
}

#[test]
fn every_a1_explanation_is_publishable() {
    for (name, log) in axis_evidence_matrix() {
        for axis in AxisId::ALL {
            for independence in A1Independence::ALL {
                let text = a1_axis_state_with(axis, &log, independence).explain_zh();
                assert_publishable(&text).unwrap_or_else(|hit| panic!("{name}: {hit:?} in {text}"));
            }
        }
    }
}

// -------------------------------------- the Chinese templates, by name ---

#[test]
fn every_chinese_template_this_crate_ships_is_screened() {
    // The scans above walk fixtures, which is what catches a sentence somebody
    // adds. This one walks the *output surface* — every A2 statement key and
    // every A1 reason — and fails if a template exists that no fixture reaches,
    // so a Chinese string cannot be shipped unscreened by the simple expedient
    // of not having a fixture for it.
    let mut rendered: Vec<String> = Vec::new();
    for (_, mut base) in tie_score_matrix() {
        for band in [Band::None, Band::Weak, Band::Moderate, Band::Strong] {
            base.band = band;
            for bullet in a2_render(&base).bullets {
                rendered.push(bullet.statement_key);
            }
        }
    }

    for key in A2_STATEMENT_KEYS {
        assert!(
            rendered.iter().any(|emitted| emitted == key),
            "{key} ships a Chinese template that no fixture renders, so nothing screens it"
        );
    }

    // Every A1 reason, likewise: each one has its own sentence.
    let mut reasons: Vec<String> = Vec::new();
    for (_, log) in axis_evidence_matrix() {
        for independence in A1Independence::ALL {
            let outcome = a1_axis_state_with(AxisId::SocialEnergy, &log, independence);
            let reason = format!("{:?}", outcome.reason);
            let name = reason.split_whitespace().next().unwrap_or("").to_owned();
            if !reasons.contains(&name) {
                reasons.push(name);
            }
        }
    }
    for expected in [
        "NoEvidence",
        "UserLocked",
        "Contradicted",
        "UpgradedByIndependentGroups",
        "NotEnoughIndependentGroups",
        "NeedsASecondSource",
    ] {
        assert!(
            reasons.iter().any(|seen| seen == expected),
            "no fixture produces A1Reason::{expected}, so its sentence is unscreened"
        );
    }
}

#[test]
fn the_chinese_denylist_catches_a_diagnostic_word_inside_a_real_template() {
    // A word list is only worth having if it fires on the shape of sentence
    // this crate actually emits, rather than on a bare word. These are the A2
    // and A1 templates with one clinical word substituted in.
    for sentence in [
        "按上面的计数，这条往来归在「强」一档，看得出焦虑倾向。",
        "这个方向有 3 组互相独立的依据，可以诊断为外向。",
        "已经 400 天没有新的往来了；这属于回避型人格障碍。",
        "有记录的往来 12 次，量表得分偏高。",
    ] {
        let hit = diagnostic_hit(sentence)
            .unwrap_or_else(|| panic!("a clinical word survived the screen: {sentence}"));
        assert_eq!(hit.screen, Screen::Diagnostic);
        assert!(assert_publishable(sentence).is_err(), "{sentence}");
    }
}

#[test]
fn the_venue_split_sentence_is_publishable_about_a_third_party() {
    // The one template Round 3 added. It names two counts about somebody who
    // never consented to being described, so it goes through the peer screen as
    // well as the other two — and it must not drift into 「一对一关系」 or any
    // other claim about what the numbers mean.
    for name in [
        "group_heavy_plus_one_direct_each_way",
        "direct_heavy_with_split",
        "group_only_with_split",
    ] {
        let summary = a2_render(
            &tie_score_matrix()
                .into_iter()
                .find(|(key, _)| *key == name)
                .expect("fixture")
                .1,
        );

        let text = summary
            .bullets
            .iter()
            .find(|bullet| bullet.statement_key == "personnel.venue.direct_and_group_counts")
            .map(|bullet| bullet.text_zh.as_str())
            .unwrap_or_else(|| panic!("{name} has no split bullet"));

        assert_publishable_about_peer(text).unwrap_or_else(|hit| panic!("{hit:?} in {text}"));
        assert_eq!(numeric_rating_hit(text), None, "counts are not a rating");
        assert_eq!(peer_claim_hit(text), None, "{text}");
    }
}

#[test]
fn every_band_and_position_word_is_publishable() {
    for band in [Band::None, Band::Weak, Band::Moderate, Band::Strong] {
        assert_publishable(band.label_zh()).unwrap();
        assert_publishable(band.as_str()).unwrap();
    }
    for position in [
        Position::LeansLow,
        Position::Mixed,
        Position::LeansHigh,
        Position::Unknown,
    ] {
        assert_publishable(position.as_str()).unwrap();
    }
}

#[test]
fn algorithm_ids_carry_no_forbidden_word() {
    for id in [
        soul_algo_trait::A0_ALGORITHM_ID,
        soul_algo_trait::A1_ALGORITHM_ID,
        A2_ALGORITHM_ID,
        A3_ALGORITHM_ID,
    ] {
        assert_publishable(id).unwrap_or_else(|hit| panic!("{hit:?} in {id}"));
    }
}

// --------------------------------------------------- the screens work ---

#[test]
fn the_diagnostic_screen_catches_what_it_is_for() {
    for word in DIAGNOSTIC_DENYLIST {
        let sentence = format!("这个人有{word}的倾向。");
        assert!(
            diagnostic_hit(&sentence).is_some(),
            "{word} slipped through"
        );
        assert!(assert_publishable(&sentence).is_err());
    }
}

#[test]
fn ascii_denylist_words_are_case_folded() {
    for spelling in ["SCORE", "Score", "sCoRe", "PERCENTILE"] {
        let hit = diagnostic_hit(spelling).expect("case-folded match");
        assert_eq!(hit.screen, Screen::Diagnostic);
    }
}

#[test]
fn a_plain_count_is_not_a_rating() {
    // The whole difference between "we counted this" and "we graded you".
    for sentence in [
        "有记录的往来 12 次，出现在 5 个不同的日子。",
        "最近一次往来距最新的记录 11 天。",
        "这个方向有 3 组互相独立的依据。",
    ] {
        assert_eq!(numeric_rating_hit(sentence), None, "{sentence}");
        assert_publishable(sentence).unwrap();
    }
}

#[test]
fn a_rating_is_caught_even_when_it_is_dressed_as_a_count() {
    for sentence in ["社交能量 78%", "这个人的量表得分偏高", "打分：4 / 5"] {
        assert!(numeric_rating_hit(sentence).is_some(), "{sentence}");
    }
}

#[test]
fn the_peer_screen_only_applies_to_peer_copy() {
    // 「内向」 is a forbidden claim about a third party and an ordinary word in
    // the owner's own axis vocabulary, which is why there are two entry points
    // rather than one list.
    let claim = "这个人很内向。";
    assert!(peer_claim_hit(claim).is_some());
    assert!(assert_publishable(claim).is_ok());
    assert!(assert_publishable_about_peer(claim).is_err());
}
