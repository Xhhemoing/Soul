//! A2: it renders the score it is given, and forms no opinion of its own.

use soul_algo_trait::a2::{a2_render, PersonnelSummary, TieScore, DORMANT_AFTER_DAYS};
use soul_algo_trait::denylist::{assert_publishable_about_peer, peer_claim_hit};
use soul_algo_trait::fixtures::{tie_score_matrix, DAY, FIXTURE_AS_OF_UNIX};
use soul_algo_trait::types::Band;

fn score(name: &str) -> TieScore {
    tie_score_matrix()
        .into_iter()
        .find(|(key, _)| *key == name)
        .unwrap_or_else(|| panic!("no fixture named {name}"))
        .1
}

// ------------------------------------------------------- pure renderer ---

/// The load-bearing test of this whole module.
///
/// A2 may not carry a second copy of the tie thresholds. `include_str!` reads
/// the source at compile time, so this is a fact about the file that ships, not
/// about a behaviour that happens to look right today.
#[test]
fn a2_defines_no_band_thresholds() {
    let source = include_str!("../src/a2.rs");

    for forbidden in [
        "STRONG_MIN",
        "MODERATE_MIN",
        "MIN_INTERACTIONS",
        "MIN_ACTIVE_DAYS",
        ">= 10",
        ">=10",
    ] {
        assert!(
            !source.contains(forbidden),
            "src/a2.rs contains {forbidden:?}: A2 has started deciding bands again"
        );
    }
}

#[test]
fn the_band_on_every_bullet_is_the_band_it_was_handed() {
    for (name, mut base) in tie_score_matrix() {
        if base.evidence_ids.is_empty() {
            continue;
        }
        for band in [Band::None, Band::Weak, Band::Moderate, Band::Strong] {
            base.band = band;
            let summary = a2_render(&base);
            for bullet in &summary.bullets {
                assert_eq!(bullet.band, band, "{name} at {band:?}");
            }
        }
    }
}

#[test]
fn the_counts_alone_never_change_the_band() {
    // The 3 / 10 / 3 shaped edge: thick, reciprocal, spread over many days,
    // and filed Weak by whatever scored it. A2 renders Weak, because A2 does
    // not get a vote.
    let thick_but_weak = TieScore {
        band: Band::Weak,
        ..score("direct_reciprocal_thick")
    };

    let summary = a2_render(&thick_but_weak);
    assert!(summary.bullets.iter().all(|b| b.band == Band::Weak));
    assert!(summary.texts().iter().any(|text| text.contains("240")));
}

#[test]
fn a_weak_band_never_reads_as_a_strong_relationship() {
    let thick_but_weak = TieScore {
        band: Band::Weak,
        ..score("direct_reciprocal_thick")
    };

    for text in a2_render(&thick_but_weak).texts() {
        assert!(!text.contains('强'), "a Weak edge said 强: {text}");
        assert!(!text.contains("强关系"), "{text}");
    }
}

#[test]
fn no_band_ever_claims_a_relationship() {
    // 「强」 is allowed as the name of a band and 「强关系」 is not, at any band.
    // The distinction is the whole reason A2 exists rather than a template.
    for (name, mut base) in tie_score_matrix() {
        if base.evidence_ids.is_empty() {
            continue;
        }
        for band in [Band::None, Band::Weak, Band::Moderate, Band::Strong] {
            base.band = band;
            for text in a2_render(&base).texts() {
                assert_eq!(peer_claim_hit(text), None, "{name} at {band:?}: {text}");
            }
        }
    }
}

// --------------------------------------------------------- what it says ---

#[test]
fn every_bullet_cites_something() {
    for (name, base) in tie_score_matrix() {
        for bullet in &a2_render(&base).bullets {
            assert!(
                !bullet.evidence_ids.is_empty(),
                "{name}: {} cites nothing",
                bullet.statement_key
            );
        }
    }
}

#[test]
fn empty_evidence_means_no_bullets_at_all() {
    // The documented decision: an empty summary is not a summary with a
    // placeholder in it. The sentence a caller shows instead is a constant, not
    // a bullet, because a bullet promises evidence.
    for name in ["empty", "counts_without_citable_evidence"] {
        let summary = a2_render(&score(name));
        assert!(summary.is_empty(), "{name}");
        assert!(summary.bullets.is_empty(), "{name}");
    }

    assert!(!PersonnelSummary::nothing_to_say_zh().is_empty());
    assert_publishable_about_peer(PersonnelSummary::nothing_to_say_zh()).unwrap();
}

#[test]
fn counts_with_no_band_yet_still_render() {
    let summary = a2_render(&score("no_band_assigned_yet"));
    assert!(!summary.is_empty());
    assert!(summary.has("personnel.tie.filed_band"));
    assert!(summary
        .texts()
        .iter()
        .any(|text| text.contains("还看不出该归在哪一档")));
}

#[test]
fn a_one_way_edge_says_so_in_both_directions() {
    let outgoing = a2_render(&score("single_outgoing_message"));
    assert!(outgoing.has("personnel.tie.one_way_outgoing"));
    assert!(!outgoing.has("personnel.tie.two_way"));

    let incoming = a2_render(&score("one_way_incoming_only"));
    assert!(incoming.has("personnel.tie.one_way_incoming"));
    assert!(incoming.texts().iter().any(|text| text.contains("12")));
}

#[test]
fn a_group_only_edge_is_flagged_and_a_direct_one_is_not() {
    assert!(a2_render(&score("group_only_reciprocal")).has("personnel.venue.group_only"));
    assert!(!a2_render(&score("direct_reciprocal_thick")).has("personnel.venue.group_only"));
}

#[test]
fn the_recency_line_cites_the_one_row_it_rests_on() {
    let summary = a2_render(&score("one_way_incoming_only"));
    let recency = summary
        .bullets
        .iter()
        .find(|bullet| bullet.statement_key == "personnel.recency.days_since_last")
        .expect("a recency bullet");
    assert_eq!(recency.evidence_ids, vec![203]);
    assert!(recency.text_zh.contains("11"));
}

#[test]
fn an_edge_with_no_last_contact_says_nothing_about_recency() {
    let summary = a2_render(&score("no_band_assigned_yet"));
    assert!(!summary.has("personnel.recency.days_since_last"));
    assert!(!summary.has("personnel.recency.dormant"));
}

// ------------------------------------------------------------- dormancy ---

#[test]
fn dormancy_is_a_sentence_not_a_demotion() {
    let dormant = score("dormant_but_strong");
    assert!(dormant.is_dormant());

    let summary = a2_render(&dormant);
    assert!(summary.has("personnel.recency.dormant"));
    assert!(
        summary.bullets.iter().all(|b| b.band == Band::Strong),
        "the gap is described; the band is the graph's to change, not A2's"
    );
    assert!(summary
        .texts()
        .iter()
        .any(|text| text.contains("400") && text.contains("不是现在的联系频率")));
}

#[test]
fn the_dormancy_threshold_is_strictly_more_than_the_constant() {
    assert_eq!(DORMANT_AFTER_DAYS, 180);

    let at_threshold = score("quiet_for_exactly_the_threshold");
    assert_eq!(at_threshold.days_since_last_contact(), Some(180));
    assert!(!at_threshold.is_dormant());
    assert!(!a2_render(&at_threshold).has("personnel.recency.dormant"));

    let one_day_later = TieScore {
        last_contact_unix: Some(FIXTURE_AS_OF_UNIX - 181 * DAY),
        ..at_threshold
    };
    assert!(one_day_later.is_dormant());
    assert!(a2_render(&one_day_later).has("personnel.recency.dormant"));
}

#[test]
fn a_last_contact_ahead_of_as_of_reads_as_today() {
    let ahead = score("clock_ahead_of_last_contact");
    assert_eq!(ahead.days_since_last_contact(), Some(0));
    assert!(!ahead.is_dormant());
    assert!(a2_render(&ahead)
        .texts()
        .iter()
        .any(|text| text.contains("最新的记录当天")));
}

// ---------------------------------------------------------- determinism ---

#[test]
fn the_same_score_always_renders_the_same_summary() {
    for (name, base) in tie_score_matrix() {
        assert_eq!(a2_render(&base), a2_render(&base), "{name}");
    }
}

#[test]
fn shifting_as_of_and_last_contact_together_changes_nothing() {
    // Nothing here reads a clock, so an export replayed a year later says the
    // same thing.
    for (name, base) in tie_score_matrix() {
        let shifted = TieScore {
            as_of_unix: base.as_of_unix + 365 * DAY,
            last_contact_unix: base.last_contact_unix.map(|at| at + 365 * DAY),
            ..base.clone()
        };
        assert_eq!(a2_render(&base), a2_render(&shifted), "{name}");
    }
}
