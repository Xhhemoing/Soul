//! Every string this crate can put in front of a person, screened.
//!
//! The scan walks generated output rather than a hand-kept list, so a new
//! bullet or a reworded pole is covered the moment it exists.

use soul_algo_trait::a0::{a0_all_axes, A0_ALGORITHM_ID};
use soul_algo_trait::a1::{a1_all_axes, A1_ALGORITHM_ID};
use soul_algo_trait::a2::{a2_personnel_summary, A2_ALGORITHM_ID};
use soul_algo_trait::a3::{A3_ALGORITHM_ID, A3_REFUSAL_REASON};
use soul_algo_trait::denylist::{
    assert_publishable, diagnostic_hit, numeric_rating_hit, peer_claim_hit, Screen,
    DIAGNOSTIC_DENYLIST,
};
use soul_algo_trait::fixtures::{axis_evidence_matrix, peer_stats_matrix, FIXTURE_NOW_UNIX};
use soul_algo_trait::{axis_vocabulary, AxisId, Band, Position};

/// Everything a user could read, plus the machine keys that end up in an
/// export or an audit row.
fn everything_this_crate_can_say() -> Vec<String> {
    let mut strings = axis_vocabulary();

    strings.push(A0_ALGORITHM_ID.to_owned());
    strings.push(A1_ALGORITHM_ID.to_owned());
    strings.push(A2_ALGORITHM_ID.to_owned());
    strings.push(A3_ALGORITHM_ID.to_owned());
    strings.push(A3_REFUSAL_REASON.to_owned());

    for band in [Band::None, Band::Weak, Band::Moderate, Band::Strong] {
        strings.push(band.as_str().to_owned());
    }
    for position in [
        Position::LeansLow,
        Position::Mixed,
        Position::LeansHigh,
        Position::Unknown,
    ] {
        strings.push(position.as_str().to_owned());
    }

    for (_, evidence) in axis_evidence_matrix() {
        for outcome in a0_all_axes(&evidence)
            .into_iter()
            .chain(a1_all_axes(&evidence))
        {
            strings.push(outcome.state.describe());
            strings.push(outcome.state.statement_key());
        }
    }

    for (_, stats) in peer_stats_matrix() {
        for offset in [0, 86_400, 400 * 86_400] {
            for bullet in a2_personnel_summary(&stats, FIXTURE_NOW_UNIX + offset).bullets {
                strings.push(bullet.statement_key);
                strings.push(bullet.text_zh);
            }
        }
    }

    strings
}

/// PRODUCT_LOCK「不是临床医学产品」and D5.
#[test]
fn nothing_this_crate_says_contains_a_clinical_word() {
    for text in everything_this_crate_can_say() {
        assert_eq!(
            diagnostic_hit(&text),
            None,
            "clinical word in user-facing string: {text}"
        );
    }
}

/// D22: direction and band, never a score, a percentage or a scale.
#[test]
fn nothing_this_crate_says_reads_as_a_rating() {
    for text in everything_this_crate_can_say() {
        assert_eq!(
            numeric_rating_hit(&text),
            None,
            "rating marker in user-facing string: {text}"
        );
    }
}

#[test]
fn the_combined_screen_agrees_with_the_individual_ones() {
    for text in everything_this_crate_can_say() {
        assert_eq!(assert_publishable(&text), Ok(()), "rejected: {text}");
    }
}

/// The personnel summary is the only output about a third party, and it is the
/// only place the peer-claim screen applies: an axis pole describes the owner,
/// who asked for it.
#[test]
fn no_personnel_bullet_describes_the_peer_themselves() {
    for (name, stats) in peer_stats_matrix() {
        for bullet in a2_personnel_summary(&stats, FIXTURE_NOW_UNIX).bullets {
            assert_eq!(
                peer_claim_hit(&bullet.text_zh),
                None,
                "{name}/{} described the peer: {}",
                bullet.statement_key,
                bullet.text_zh
            );
        }
    }
}

/// A screen that cannot catch anything is worse than no screen, because it
/// looks like one.
#[test]
fn the_screens_catch_what_they_are_for() {
    for word in DIAGNOSTIC_DENYLIST {
        let sentence = format!("这是一段包含 {word} 的文字。");
        let hit = diagnostic_hit(&sentence).expect("the denylist must catch its own words");
        assert_eq!(hit.screen, Screen::Diagnostic);
    }

    assert!(diagnostic_hit("this has a SCORE in it").is_some());
    assert!(diagnostic_hit("Percentile ranking").is_some());
    assert!(numeric_rating_hit("好奇与开放 82%").is_some());
    assert!(numeric_rating_hit("这条轴的评分是四").is_some());
    assert!(peer_claim_hit("你们大概是朋友").is_some());

    assert_eq!(diagnostic_hit("往来是双向的"), None);
    assert_eq!(numeric_rating_hit("有记录的往来 240 次"), None);
}

/// Counts are not ratings. If the rating screen ever starts rejecting plain
/// numbers, A2 loses the only thing it is allowed to say.
#[test]
fn plain_counts_are_still_allowed() {
    assert_eq!(
        numeric_rating_hit("有记录的往来 12 次，出现在 5 个不同的日子。"),
        None
    );
    assert_eq!(numeric_rating_hit("最近一次往来距今 400 天。"), None);
}

#[test]
fn every_axis_reads_as_a_direction_rather_than_a_level() {
    for axis in AxisId::ALL {
        for position in [
            Position::LeansLow,
            Position::Mixed,
            Position::LeansHigh,
            Position::Unknown,
        ] {
            let sentence = axis.describe(position);
            assert!(sentence.starts_with(axis.label()));
            assert_eq!(assert_publishable(&sentence), Ok(()));
        }
        assert_eq!(
            axis.statement_key(Position::LeansHigh),
            format!("trait_axis.{}.leans_high", axis.key())
        );
    }
}
