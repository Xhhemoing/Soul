//! A2: it renders the score it is given, and forms no opinion of its own.

use soul_algo_trait::a2::{
    a2_render, PersonnelSummary, TieScore, A2_STATEMENT_KEYS, DORMANT_AFTER_DAYS,
};
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

// ----------------------------------------------------------- venue split ---

#[test]
fn the_venue_split_is_rendered_when_the_scorer_supplies_it() {
    // `R2-SYNTHESIS.md` 边界风险 2: 137 interactions of which two were one to
    // one. Whether that may be Strong is T4D's question; A2's job is to put the
    // two numbers where the user can see them.
    let summary = a2_render(&score("group_heavy_plus_one_direct_each_way"));

    assert!(summary.has("personnel.venue.direct_and_group_counts"));
    let text = summary
        .bullets
        .iter()
        .find(|bullet| bullet.statement_key == "personnel.venue.direct_and_group_counts")
        .map(|bullet| bullet.text_zh.clone())
        .expect("a split bullet");
    assert!(text.contains('2') && text.contains("135"), "{text}");
    assert!(
        text.contains("一对一") && text.contains("群里同场"),
        "{text}"
    );
}

#[test]
fn the_split_never_touches_the_band() {
    // The whole risk of adding a field to a renderer: that the renderer starts
    // having an opinion about it. Two interaction counts out of 137 is exactly
    // the shape somebody would be tempted to demote, so the band that comes out
    // is the band that went in, at all four values.
    for band in [Band::None, Band::Weak, Band::Moderate, Band::Strong] {
        let scored = TieScore {
            band,
            ..score("group_heavy_plus_one_direct_each_way")
        };
        let summary = a2_render(&scored);
        assert!(summary.has("personnel.venue.direct_and_group_counts"));
        assert!(summary.bullets.iter().all(|bullet| bullet.band == band));
    }
}

#[test]
fn no_split_no_sentence() {
    // Absent on either side, and the sentence is gone. A2 does not derive the
    // missing half by subtraction, which would make it the author of a number
    // nobody handed it.
    for name in ["direct_reciprocal_thick", "half_a_split_is_not_a_split"] {
        let summary = a2_render(&score(name));
        assert!(
            !summary.has("personnel.venue.direct_and_group_counts"),
            "{name}"
        );
        for text in summary.texts() {
            assert!(!text.contains("一对一往来"), "{name}: {text}");
        }
    }

    assert_eq!(score("half_a_split_is_not_a_split").venue_split(), None);
    assert_eq!(
        score("group_heavy_plus_one_direct_each_way").venue_split(),
        Some((2, 135))
    );
}

#[test]
fn the_split_and_the_group_only_flag_are_separate_sentences() {
    // They rest on separate fields — `direct_count`/`group_count` and
    // `any_direct` — so a scorer that supplies both gets both, and A2 does not
    // reconcile them.
    let summary = a2_render(&score("group_only_with_split"));
    assert!(summary.has("personnel.venue.direct_and_group_counts"));
    assert!(summary.has("personnel.venue.group_only"));
}

#[test]
fn every_statement_key_is_reachable_and_listed() {
    // `A2_STATEMENT_KEYS` is A2's whole output surface. It is only worth having
    // if it is complete in both directions: nothing emitted that is not listed,
    // and nothing listed that no fixture can produce.
    let mut seen: Vec<String> = Vec::new();
    for (name, base) in tie_score_matrix() {
        for bullet in a2_render(&base).bullets {
            assert!(
                A2_STATEMENT_KEYS.contains(&bullet.statement_key.as_str()),
                "{name} emitted the unlisted key {}",
                bullet.statement_key
            );
            if !seen.contains(&bullet.statement_key) {
                seen.push(bullet.statement_key);
            }
        }
    }

    for key in A2_STATEMENT_KEYS {
        assert!(
            seen.iter().any(|emitted| emitted == key),
            "{key} is listed but no fixture produces it"
        );
    }
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
fn the_dormancy_threshold_is_closed_at_the_constant() {
    assert_eq!(DORMANT_AFTER_DAYS, 180);

    let at_threshold = score("quiet_for_exactly_the_threshold");
    assert_eq!(at_threshold.days_since_last_contact(), Some(180));
    assert!(at_threshold.is_dormant());
    assert!(a2_render(&at_threshold).has("personnel.recency.dormant"));

    let one_day_earlier = TieScore {
        last_contact_unix: Some(FIXTURE_AS_OF_UNIX - 179 * DAY),
        ..at_threshold
    };
    assert!(!one_day_earlier.is_dormant());
    assert!(!a2_render(&one_day_earlier).has("personnel.recency.dormant"));
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
