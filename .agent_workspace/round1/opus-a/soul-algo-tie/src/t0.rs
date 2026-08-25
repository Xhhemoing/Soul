//! T0 — Count + Reciprocity + Span. The rule Goal 1 already ships.
//!
//! Ported from `soul-graph::build::Tally::band`, thresholds included:
//!
//! ```text
//! reciprocal && count >= 10 && active_days >= 3  -> Strong
//! reciprocal && count >= 3                       -> Moderate
//! otherwise                                      -> Weak
//! ```
//!
//! Three properties of the original are kept on purpose, because Round 1 is
//! measuring the baseline, not improving it:
//!
//! * No recency. A tie that went quiet in 2019 scores exactly as a tie that
//!   was busy this morning. `now_unix` is accepted and unused.
//! * No venue weighting. Fifty messages in a group of two hundred people count
//!   the same as fifty one-to-one messages.
//! * Active days are UTC dates. Goal 1 gets them by taking the first ten
//!   characters of an RFC 3339 timestamp normalised to `Z`; this port floors
//!   the Unix second to an epoch day. Same set, no string parsing, and neither
//!   spelling reads the local timezone.

use crate::types::{
    zh_common_weak_reason, zh_counts, Band, Interaction, Tally, TieAlgorithm, TieScore,
};

/// Reciprocal contact from here on is more than an exchanged greeting.
pub const MODERATE_MIN_INTERACTIONS: u64 = 3;
/// A strong tie has to be both frequent and spread over several days: twenty
/// messages in one afternoon is one conversation, not a habit.
pub const STRONG_MIN_INTERACTIONS: u64 = 10;
pub const STRONG_MIN_ACTIVE_DAYS: u64 = 3;

pub struct T0;

impl T0 {
    /// The rule itself, over an already-built tally.
    ///
    /// Scoring a whole ego network goes through here so the log is read once
    /// rather than once per peer.
    pub fn band_of(tally: &Tally, _now_unix: i64) -> Band {
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

    fn score(peer_id: u64, interactions: &[Interaction], now_unix: i64) -> TieScore {
        let tally = Tally::of(peer_id, interactions);
        let band = T0::band_of(&tally, now_unix);
        tally.into_score(band, Self::ID)
    }

    fn explain_zh(score: &TieScore) -> String {
        if let Some(reason) = zh_common_weak_reason(score) {
            return reason;
        }
        let counts = zh_counts(score);
        match score.band {
            Band::Strong => format!(
                "{counts}你们互相都发过消息，往来次数达到 {STRONG_MIN_INTERACTIONS} 次以上，而且分散在 {STRONG_MIN_ACTIVE_DAYS} 天以上，不是一次聊完就没了，所以算强联系。",
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
    use crate::testing::{direct, DAY};

    #[test]
    fn thresholds_match_goal_one_constants() {
        assert_eq!(MODERATE_MIN_INTERACTIONS, 3);
        assert_eq!(STRONG_MIN_INTERACTIONS, 10);
        assert_eq!(STRONG_MIN_ACTIVE_DAYS, 3);
    }

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
    fn now_is_ignored() {
        let log = vec![
            direct(1, true, 0, 1),
            direct(1, false, DAY, 1),
            direct(1, true, DAY * 2, 1),
        ];
        let at_the_time = T0::score(1, &log, DAY * 3);
        let a_decade_later = T0::score(1, &log, DAY * 3650);
        assert_eq!(at_the_time, a_decade_later);
        assert_eq!(at_the_time.band, Band::Moderate);
    }

    #[test]
    fn venue_is_ignored() {
        let group: Vec<Interaction> = (0..12)
            .map(|i| Interaction {
                peer_id: 1,
                outgoing: i % 2 == 0,
                occurred_at_unix: DAY * i,
                venue_direct: false,
                conversation_id: 9,
            })
            .collect();
        assert_eq!(T0::score(1, &group, 0).band, Band::Strong);
    }
}
