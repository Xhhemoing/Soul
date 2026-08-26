//! T3 — Granovetter-span. Venue is part of the definition, not a weight.
//!
//! ```text
//! Strong    if reciprocal ∧ any_direct ∧ count ≥ 10 ∧ active_days ≥ 3
//! Moderate  if reciprocal ∧ count ≥ 3      # a group-only tie stops here
//! Weak      otherwise
//! ```
//!
//! The idea is Granovetter's: a strong tie is one that costs both people
//! something — time, attention, and a channel where nobody else is listening.
//! Sitting in the same two-hundred-person group as somebody for a year is
//! exactly the weak tie the theory is about, so no amount of group traffic
//! reaches Strong here.
//!
//! Two changes from Round 1's T3, both required by this round's brief and both
//! in the direction of "align with what is already frozen":
//!
//! * The Strong count gate is 10, not 8 — Goal 1's `STRONG_MIN_INTERACTIONS`
//!   and fable-a's frozen `CANDIDATE_SPEC`. Round 1's 8 was an unforced
//!   difference that would have polluted the ablation.
//! * Moderate is the single clause `reciprocal ∧ count ≥ 3` for everybody.
//!   Round 1 had a separate, laxer group ladder (5 exchanges over 2 days) and
//!   promoted any reciprocal direct pair, however small. Both are gone: one
//!   Moderate bar, the shipped one.
//!
//! What T3 still does not have is any notion of time passing. A tie that
//! stopped in 2019 scores exactly as one that stopped this morning. That is
//! the hole T3R and T4 are here to fill, and `tests/ablation.rs` measures it.

use crate::constants::{
    MODERATE_MIN_INTERACTIONS, STRONG_MIN_ACTIVE_DAYS, STRONG_MIN_INTERACTIONS,
};
use crate::gate::{self, Observed, Thresholds};
use crate::types::{
    zh_common_weak_reason, zh_counts, zh_venue, Band, Detail, Interaction, Tally, TieAlgorithm,
    TieScore,
};

/// T3's bars, in whole interactions and whole days.
pub const THRESHOLDS: Thresholds = Thresholds {
    strong_min_count: STRONG_MIN_INTERACTIONS,
    strong_min_days: STRONG_MIN_ACTIVE_DAYS,
    moderate_min_count: MODERATE_MIN_INTERACTIONS,
};

pub struct T3;

impl T3 {
    /// What the gate sees: raw counts, unweighted.
    pub fn observed(tally: &Tally) -> Observed {
        Observed {
            reciprocal: tally.is_reciprocal(),
            any_direct: tally.any_direct,
            group_only: tally.is_group_only(),
            count: tally.interaction_count(),
            days: tally.active_day_count(),
        }
    }

    /// The rule itself, over an already-built tally.
    pub fn band_of(tally: &Tally, _as_of_unix: i64) -> Band {
        gate::band(T3::observed(tally), THRESHOLDS)
    }

    /// The reason clause, without a conclusion, so that T4 can reuse it
    /// verbatim and then add its own.
    ///
    /// Only the counts, the days, the two booleans and the date are named, so
    /// every number in it is one the user can recount by hand. No decimals,
    /// no weights, nothing derived.
    pub(crate) fn zh_reason(score: &TieScore) -> String {
        let venue = zh_venue(score.any_direct);
        match score.band {
            Band::Strong => format!(
                "{venue}，双方都发过消息，往来 {} 次不少于 {STRONG_MIN_INTERACTIONS} 次，分布的 {} 天也不少于 {STRONG_MIN_ACTIVE_DAYS} 天",
                score.interaction_count, score.active_day_count,
            ),
            Band::Moderate if !score.any_direct => format!(
                "{venue}；双方虽然都发过消息、在群里来往了 {} 次，但群里再热闹也只说明认识，不足以说明关系紧密",
                score.interaction_count,
            ),
            Band::Moderate if score.interaction_count < STRONG_MIN_INTERACTIONS => format!(
                "{venue}，双方也都发过消息，但一共 {} 次，还不到 {STRONG_MIN_INTERACTIONS} 次",
                score.interaction_count,
            ),
            Band::Moderate => format!(
                "{venue}，双方也都发过消息，往来 {} 次够多，但只分布在 {} 天里，不到 {STRONG_MIN_ACTIVE_DAYS} 天，看不出是长期习惯",
                score.interaction_count, score.active_day_count,
            ),
            Band::Weak => format!(
                "{venue}，双方虽然都发过消息，但一共只有 {} 次，不到 {MODERATE_MIN_INTERACTIONS} 次，还只是打过招呼",
                score.interaction_count,
            ),
        }
    }
}

impl TieAlgorithm for T3 {
    const ID: &'static str = "T3";

    fn score(peer_id: u64, interactions: &[Interaction], as_of_unix: i64) -> TieScore {
        let tally = Tally::of(peer_id, interactions);
        let band = T3::band_of(&tally, as_of_unix);
        tally.to_score(band, Self::ID, as_of_unix, Detail::RawCounts)
    }

    fn explain_zh(score: &TieScore) -> String {
        if let Some(reason) = zh_common_weak_reason(score) {
            return reason;
        }
        format!(
            "{}{}，所以算{}。",
            zh_counts(score),
            T3::zh_reason(score),
            score.band.as_zh(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::GROUP_ONLY_CEILING;
    use crate::testing::{direct, group, DAY};

    #[test]
    fn group_only_never_reaches_strong_however_much_traffic() {
        let log: Vec<Interaction> = (0..200).map(|i| group(1, i % 2 == 0, DAY * i, 7)).collect();
        let score = T3::score(1, &log, DAY * 200);
        assert_eq!(score.interaction_count, 200);
        assert_eq!(score.active_day_count, 200);
        assert_ne!(score.band, Band::Strong);
        assert_eq!(score.band, GROUP_ONLY_CEILING);
        assert!(!score.any_direct);
    }

    #[test]
    fn one_direct_message_lifts_an_otherwise_group_tie() {
        // The venue gate is about whether a private channel ever existed, not
        // about how much of the traffic went through it. Nine group exchanges
        // plus one private one clears the Strong bar; without the private one
        // it is capped.
        let mut log: Vec<Interaction> = (0..9).map(|i| group(1, i % 2 == 0, DAY * i, 7)).collect();
        assert_eq!(T3::score(1, &log, 0).band, Band::Moderate);
        log.push(direct(1, true, DAY * 9, 11));
        assert_eq!(T3::score(1, &log, 0).band, Band::Strong);
    }

    #[test]
    fn ten_over_three_days_is_the_strong_boundary() {
        let log: Vec<Interaction> = (0..10)
            .map(|i| direct(1, i % 2 == 0, DAY * (i as i64 % 3) + 60 * i as i64, 1))
            .collect();
        assert_eq!(T3::score(1, &log, 0).band, Band::Strong);
        // Nine is Moderate: the Round 1 gate of 8 would have said Strong here.
        assert_eq!(T3::score(1, &log[..9], 0).band, Band::Moderate);
        assert_eq!(T3::score(1, &log[..8], 0).band, Band::Moderate);
    }

    #[test]
    fn a_single_direct_exchange_is_weak_like_the_shipped_rule() {
        // Round 1's T3 called this Moderate. Aligning the Moderate bar on
        // Goal 1's 3 takes that generosity away, on purpose.
        let log = vec![direct(1, true, 0, 1), direct(1, false, 60, 1)];
        assert_eq!(T3::score(1, &log, 0).band, Band::Weak);
    }

    #[test]
    fn thin_group_traffic_reaches_the_same_moderate_bar_as_everybody_else() {
        let log: Vec<Interaction> = (0..3)
            .map(|i| group(1, i % 2 == 0, DAY * (i as i64 % 2) + 60 * i as i64, 7))
            .collect();
        assert_eq!(T3::score(1, &log, 0).band, Band::Moderate);
        assert_eq!(T3::score(1, &log[..2], 0).band, Band::Weak);
    }

    #[test]
    fn the_strict_reading_of_the_group_ceiling_is_one_constant_away() {
        // Documents what flipping GROUP_ONLY_CEILING to Weak would do, without
        // needing a build flag: every group-only band collapses to Weak.
        let log: Vec<Interaction> = (0..50).map(|i| group(1, i % 2 == 0, DAY * i, 7)).collect();
        let score = T3::score(1, &log, DAY * 50);
        assert_eq!(score.band, Band::Moderate);
        assert_eq!(score.band.capped_at(Band::Weak), Band::Weak);
    }

    #[test]
    fn recency_is_not_part_of_this_rule() {
        let log: Vec<Interaction> = (0..12).map(|i| direct(1, i % 2 == 0, DAY * i, 1)).collect();
        assert_eq!(T3::score(1, &log, DAY * 12).band, Band::Strong);
        assert_eq!(T3::score(1, &log, DAY * 12 + DAY * 3650).band, Band::Strong);
    }
}
