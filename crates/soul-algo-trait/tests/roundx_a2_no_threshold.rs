//! Round X: A2 owns no gate, checked over a count sweep rather than a word list.
//!
//! `a2_defines_no_band_thresholds` and `the_a2_source_still_holds_no_threshold`
//! are the crate's guard against A2 re-growing a band rule, and both work by
//! reading `src/a2.rs` and searching it for six literals: `STRONG_MIN`,
//! `MODERATE_MIN`, `MIN_INTERACTIONS`, `MIN_ACTIVE_DAYS`, `>= 10` and `>=10`.
//! A source scan is the right shape for that claim — it fails on the file that
//! ships rather than on a behaviour that happens to look right — but it is
//! spelled against the names Goal 1 used, so a gate written with any other
//! name or any other comparison walks past it.
//!
//! The behavioural counterpart is `tie_score_matrix`, and that matrix does
//! catch the obvious re-gates: it holds an edge at 240 interactions over 61
//! days and one at 90 direct interactions, and every band is rendered against
//! every fixture, so a rule keyed on any single one of those counts already
//! breaks a test. What it is thin on is counts *at* the boundary. Its fifteen
//! fixtures carry active-day counts of 0, 1, 2, 4, 5, 6, 7, 9, 22, 31, 40 and
//! 61 — never 3, which is what `STRONG_MIN_ACTIVE_DAYS` was — and direct
//! counts of 0, 2, 12 and 90, so the whole 3..10 window a T4D-shaped gate
//! would live in is unrepresented. A gate reintroduced at Goal 1's numbers
//! under a different name would have to be caught by luck.
//!
//! These two tests close that. The first sweeps the counts across those
//! boundaries and asserts the band that comes out is the band that went in.
//! The second asserts the thing the first cannot see: that the *band* is not an
//! input either, because the set of sentences A2 emits has to be a function of
//! the counts alone. A renderer that decided to withhold the recency line on a
//! `Weak` edge would keep every band assertion in the crate passing while
//! having formed exactly the opinion A2 is not allowed to hold.

use soul_algo_trait::a2::{a2_render, TieScore};
use soul_algo_trait::fixtures::{tie_score_matrix, DAY, FIXTURE_AS_OF_UNIX};
use soul_algo_trait::types::Band;

const BANDS: [Band; 4] = [Band::None, Band::Weak, Band::Moderate, Band::Strong];

/// The counts a v0.1 gate could plausibly be spelled against, chosen to sit on
/// both sides of every threshold Goal 1 or T4D ever used: 3
/// (`MODERATE_MIN_INTERACTIONS`, `STRONG_MIN_ACTIVE_DAYS`) and 10
/// (`STRONG_MIN_INTERACTIONS`), plus zero and a value far above both.
const AROUND_THE_GATES: [u32; 8] = [0, 1, 2, 3, 4, 9, 10, 11];

fn keys(score: &TieScore) -> Vec<String> {
    a2_render(score)
        .bullets
        .into_iter()
        .map(|bullet| bullet.statement_key)
        .collect()
}

#[test]
fn no_count_on_either_side_of_a_gate_moves_the_band() {
    let mut checked = 0_u64;

    for interaction_count in AROUND_THE_GATES {
        for active_day_count in AROUND_THE_GATES {
            for direct_count in [None, Some(0), Some(2), Some(3), Some(9), Some(10), Some(11)] {
                for any_direct in [false, true] {
                    for (outgoing, incoming) in [(0, interaction_count), (interaction_count, 0)] {
                        for band in BANDS {
                            let score = TieScore {
                                band,
                                interaction_count,
                                outgoing,
                                incoming,
                                active_day_count,
                                conversation_count: 1,
                                any_direct,
                                direct_count,
                                group_count: direct_count
                                    .map(|direct| interaction_count.saturating_sub(direct)),
                                last_contact_unix: Some(FIXTURE_AS_OF_UNIX - 2 * DAY),
                                as_of_unix: FIXTURE_AS_OF_UNIX,
                                evidence_ids: vec![1, 2],
                                last_contact_evidence_id: Some(2),
                            };

                            let summary = a2_render(&score);
                            assert!(!summary.is_empty());
                            for bullet in &summary.bullets {
                                assert_eq!(
                                    bullet.band, band,
                                    "{} moved the band at {interaction_count}/{active_day_count}/{direct_count:?}",
                                    bullet.statement_key
                                );
                            }
                            checked += 1;
                        }
                    }
                }
            }
        }
    }

    assert_eq!(
        checked,
        (AROUND_THE_GATES.len() * AROUND_THE_GATES.len() * 7 * 2 * 2 * BANDS.len()) as u64
    );
}

#[test]
fn the_band_never_decides_which_sentences_appear() {
    // The other direction of "A2 forms no second opinion". Every existing test
    // reads the band off the bullets that came out; none of them checks that
    // the same bullets come out. Re-rendering each fixture at all four bands
    // and comparing the key lists is the assertion that a band-conditional
    // sentence — added or withheld — cannot survive.
    for (name, mut score) in tie_score_matrix() {
        score.band = Band::None;
        let baseline = keys(&score);

        for band in BANDS {
            score.band = band;
            assert_eq!(
                keys(&score),
                baseline,
                "{name}: the sentences A2 emits changed with the band"
            );
        }
    }
}

#[test]
fn the_band_changes_exactly_one_sentence_and_it_is_the_filing_line() {
    // The complement of the test above, so that "invariant under the band" is
    // not read as "the band is never mentioned". One bullet says which band the
    // graph chose, because the user has to be able to argue with it; every
    // other sentence is a restatement of a count and must read identically at
    // all four bands.
    for (name, mut score) in tie_score_matrix() {
        score.band = Band::None;
        let baseline = a2_render(&score);

        for band in BANDS {
            score.band = band;
            let rendered = a2_render(&score);
            for (left, right) in baseline.bullets.iter().zip(rendered.bullets.iter()) {
                if left.statement_key == "personnel.tie.filed_band" {
                    continue;
                }
                assert_eq!(
                    left.text_zh, right.text_zh,
                    "{name}: {} rewrote itself at {band:?}",
                    left.statement_key
                );
                assert_eq!(left.evidence_ids, right.evidence_ids, "{name}");
            }
        }
    }
}
