//! T0 — Count + Reciprocity + Span. The rule Goal 1 ships.
//!
//! **Not a candidate any more.** R1-SYNTHESIS retired T0 as a specification
//! («T0 原样：不保留为规范»). It stays in the crate for one job: it is the
//! regression oracle. `tests/goal1_fidelity.rs` holds it against an
//! independent transcription of `soul-graph::build::Tally::band`, so if a
//! refactor of the shared [`Tally`] ever changes what the shipped rule would
//! have said, a test goes red before any survivor is compared against it.
//!
//! ```text
//! reciprocal && count >= 10 && active_days >= 3  -> Strong
//! reciprocal && count >= 3                       -> Moderate
//! otherwise                                      -> Weak
//! ```
//!
//! Three properties of the original are kept on purpose:
//!
//! * No recency. A tie that went quiet in 2019 scores exactly as a tie that
//!   was busy this morning. `as_of_unix` is accepted and unused for banding.
//! * No venue. Fifty messages in a group of two hundred people count the same
//!   as fifty one-to-one messages. This is defect P0-3 in R1-SYNTHESIS.
//! * Active days are UTC dates. Goal 1 gets them by taking the first ten
//!   characters of an RFC 3339 timestamp normalised to `Z`; this port floors
//!   the Unix second to an epoch day. Same set, no string parsing, and neither
//!   spelling reads the local timezone.

use crate::constants::{
    MODERATE_MIN_INTERACTIONS, STRONG_MIN_ACTIVE_DAYS, STRONG_MIN_INTERACTIONS,
};
use crate::types::{
    zh_common_weak_reason, zh_counts, Band, Detail, Interaction, Tally, TieAlgorithm, TieScore,
};

pub struct T0;

impl T0 {
    /// The rule itself, over an already-built tally.
    pub fn band_of(tally: &Tally, _as_of_unix: i64) -> Band {
        let count = tally.interaction_count();
        let days = tally.active_day_count();
        if tally.is_reciprocal()
            && count >= STRONG_MIN_INTERACTIONS
            && days >= STRONG_MIN_ACTIVE_DAYS
        {
            Band::Strong
        } else if tally.is_reciprocal() && count >= MODERATE_MIN_INTERACTIONS {
            Band::Moderate
        } else {
            Band::Weak
        }
    }
}

impl TieAlgorithm for T0 {
    const ID: &'static str = "T0";

    fn score(peer_id: u64, interactions: &[Interaction], as_of_unix: i64) -> TieScore {
        let tally = Tally::of(peer_id, interactions);
        let band = T0::band_of(&tally, as_of_unix);
        tally.to_score(band, Self::ID, as_of_unix, Detail::RawCounts)
    }

    fn explain_zh(score: &TieScore) -> String {
        if let Some(reason) = zh_common_weak_reason(score) {
            return reason;
        }
        let counts = zh_counts(score);
        match score.band {
            Band::Strong => format!(
                "{counts}你们互相都发过消息，往来次数不少于 {STRONG_MIN_INTERACTIONS} 次，而且分散在不少于 {STRONG_MIN_ACTIVE_DAYS} 天里，不是一次聊完就没了，所以算强联系。",
            ),
            Band::Moderate => {
                let shortfall = if score.interaction_count >= STRONG_MIN_INTERACTIONS {
                    format!(
                        "次数已经够多，但都挤在 {} 天里，还看不出是长期习惯",
                        score.active_day_count,
                    )
                } else {
                    format!("总次数还不到 {STRONG_MIN_INTERACTIONS} 次")
                };
                format!("{counts}你们互相都发过消息，{shortfall}，所以算中等联系。")
            }
            Band::Weak => format!(
                "{counts}你们互相都发过消息，但总共不到 {MODERATE_MIN_INTERACTIONS} 次，还只是打过招呼，所以算弱联系。",
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{direct, group, DAY};

    #[test]
    fn ten_over_three_days_is_the_exact_strong_boundary() {
        // 10 interactions, 3 active days, both directions: the smallest tie the
        // shipped rule calls strong.
        let mut log = Vec::new();
        for i in 0..10 {
            log.push(direct(
                1,
                i % 2 == 0,
                DAY * (i as i64 % 3) + 3_600 * i as i64,
                1,
            ));
        }
        let score = T0::score(1, &log, 0);
        assert_eq!(score.interaction_count, 10);
        assert_eq!(score.active_day_count, 3);
        assert_eq!(score.band, Band::Strong);

        // One message fewer, same spread: not strong.
        let score = T0::score(1, &log[..9], 0);
        assert_eq!(score.interaction_count, 9);
        assert_eq!(score.band, Band::Moderate);
    }

    #[test]
    fn as_of_does_not_change_the_shipped_band() {
        // The defect, pinned rather than fixed: this is the baseline, and the
        // survivors are judged by how they differ from it.
        let log = vec![
            direct(1, true, 0, 1),
            direct(1, false, DAY, 1),
            direct(1, true, DAY * 2, 1),
        ];
        assert_eq!(
            T0::score(1, &log, DAY * 3).band,
            T0::score(1, &log, DAY * 3650).band
        );
    }

    #[test]
    fn venue_is_ignored_which_is_the_group_defect() {
        // R1-SYNTHESIS P0-3. Twelve group messages over twelve days and the
        // shipped rule calls a project-channel colleague a strong tie.
        let group_log: Vec<Interaction> =
            (0..12).map(|i| group(1, i % 2 == 0, DAY * i, 9)).collect();
        assert_eq!(T0::score(1, &group_log, DAY * 12).band, Band::Strong);
    }
}
