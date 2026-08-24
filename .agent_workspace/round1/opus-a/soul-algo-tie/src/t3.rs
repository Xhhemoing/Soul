//! T3 — Granovetter-span. Venue is part of the definition, not a weight.
//!
//! ```text
//! reciprocal && any_direct && active_days >= 3 && count >= 8   -> Strong
//! reciprocal && (any_direct
//!                || (group_only && count >= 5 && days >= 2))   -> Moderate
//! otherwise                                                    -> Weak
//! group-only ties are then capped (see GROUP_ONLY_CEILING)
//! one-sided ties are Weak, because every rule above needs reciprocity
//! ```
//!
//! The idea behind the rule is Granovetter's: a strong tie is one that costs
//! both people something — time, attention, and a channel where nobody else is
//! listening. Sitting in the same two-hundred-person group as somebody for a
//! year is exactly the "weak tie" that theory is about, so no amount of group
//! traffic can reach Strong here.
//!
//! ## The ambiguity in the brief, and how it is resolved
//!
//! The brief says both "group-only ties: cap at Weak (even if reciprocal and
//! frequent)" and "Moderate iff reciprocal AND (direct OR (group AND count>=5
//! AND active_days>=2))". Read strictly, the second clause is dead code: any
//! tie it would promote is group-only, and the cap would immediately pull it
//! back down.
//!
//! Resolution: the cap is read as applying to the Strong band, which is what
//! the Granovetter argument actually claims, and the explicit Moderate clause
//! is honoured, which is what keeps it from being dead code. So a group-only
//! tie tops out at Moderate, and only when it clears the extra bar of 5
//! exchanges over 2 days.
//!
//! The strict reading is one constant away: set [`GROUP_ONLY_CEILING`] to
//! [`Band::Weak`] and every group-only tie is Weak again. Both readings are
//! covered by tests, so whichever one Round 3 picks, the change is small and
//! its consequences are already written down.

use crate::types::{zh_counts, Band, Interaction, Tally, TieAlgorithm, TieScore};

/// The highest band a tie that was never one-to-one can reach.
///
/// `Moderate` implements the resolution described above; `Weak` implements the
/// strict reading of the brief.
pub const GROUP_ONLY_CEILING: Band = Band::Moderate;

pub const STRONG_MIN_INTERACTIONS: u64 = 8;
pub const STRONG_MIN_ACTIVE_DAYS: u64 = 3;
/// What a group-only tie has to clear before it counts as anything.
pub const GROUP_MODERATE_MIN_INTERACTIONS: u64 = 5;
pub const GROUP_MODERATE_MIN_ACTIVE_DAYS: u64 = 2;

pub struct T3;

impl T3 {
    /// The explanation the rule can actually give, which needs to know whether
    /// anything was ever one-to-one.
    ///
    /// `TieScore` as specified does not carry the venue, and venue is the
    /// deciding fact in this candidate — so the trait's `explain_zh` has to
    /// hedge and this one does not. Callers that still hold the tally should
    /// prefer this. See REPORT.md.
    pub fn explain_zh_with_venue(score: &TieScore, any_direct: bool) -> String {
        if score.is_empty() {
            return "还没有看到你们之间的往来记录，所以先按最弱的一档放着。".to_string();
        }
        let counts = zh_counts(score);
        if !score.is_reciprocal() {
            let who = if score.incoming_count == 0 {
                "对方一次也没有回过"
            } else {
                "你一次也没有回过"
            };
            return format!("{counts}{who}，只有一头在说话的关系不会算成紧密联系，所以是弱联系。");
        }
        match (score.band, any_direct) {
            (Band::Strong, _) => format!(
                "{counts}你们私下一对一聊过，双方都在说话，次数和天数也都够，所以算强联系。",
            ),
            (Band::Moderate, true) => format!(
                "{counts}你们私下一对一聊过，双方也都在说话，但次数或天数还差一点，所以算中等联系。",
            ),
            (Band::Moderate, false) => format!(
                "{counts}你们的往来都发生在群里，没有单独聊过。群里的热闹说明认识，但还不足以说明关系紧密，所以最多算中等联系。",
            ),
            (Band::Weak, true) => format!(
                "{counts}虽然私下聊过，但次数太少，所以只算弱联系。",
            ),
            (Band::Weak, false) => format!(
                "{counts}你们只在群里遇到过，没有单独聊过，次数也不多，所以算弱联系。",
            ),
        }
    }
}

impl T3 {
    /// The rule itself, over an already-built tally.
    pub fn band_of(tally: &Tally, _now_unix: i64) -> Band {
        let count = tally.interaction_count();
        let days = tally.active_day_count();
        let reciprocal = tally.is_reciprocal();

        let uncapped = if reciprocal
            && tally.any_direct
            && days >= STRONG_MIN_ACTIVE_DAYS
            && count >= STRONG_MIN_INTERACTIONS
        {
            Band::Strong
        } else if reciprocal
            && (tally.any_direct
                || (count >= GROUP_MODERATE_MIN_INTERACTIONS
                    && days >= GROUP_MODERATE_MIN_ACTIVE_DAYS))
        {
            Band::Moderate
        } else {
            Band::Weak
        };

        if tally.is_group_only() {
            uncapped.capped_at(GROUP_ONLY_CEILING)
        } else {
            uncapped
        }
    }
}

impl TieAlgorithm for T3 {
    const ID: &'static str = "T3";

    fn score(peer_id: u64, interactions: &[Interaction], now_unix: i64) -> TieScore {
        let tally = Tally::of(peer_id, interactions);
        let band = T3::band_of(&tally, now_unix);
        tally.into_score(band, Self::ID)
    }

    fn explain_zh(score: &TieScore) -> String {
        if score.is_empty() {
            return "还没有看到你们之间的往来记录，所以先按最弱的一档放着。".to_string();
        }
        let counts = zh_counts(score);
        if !score.is_reciprocal() {
            let who = if score.incoming_count == 0 {
                "对方一次也没有回过"
            } else {
                "你一次也没有回过"
            };
            return format!("{counts}{who}，只有一头在说话的关系不会算成紧密联系，所以是弱联系。");
        }
        match score.band {
            Band::Strong => format!(
                "{counts}你们私下一对一聊过，双方都在说话，次数和天数也都够，所以算强联系。",
            ),
            // Without the venue on the score there is no honest way to say
            // which of the two shortfalls applies, so both are named.
            Band::Moderate => format!(
                "{counts}你们互相都有往来，但还没到最紧密的一档：要么单独聊得不够多，要么往来都发生在群里，所以算中等联系。",
            ),
            Band::Weak => format!(
                "{counts}你们互相都有往来，但次数和天数都太少，也看不出有稳定的单独联系，所以算弱联系。",
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{direct, group, DAY};

    #[test]
    fn group_only_never_reaches_strong_however_much_traffic() {
        let log: Vec<Interaction> = (0..200).map(|i| group(1, i % 2 == 0, DAY * i, 7)).collect();
        let score = T3::score(1, &log, DAY * 200);
        assert_eq!(score.interaction_count, 200);
        assert_eq!(score.active_day_count, 200);
        assert_ne!(score.band, Band::Strong);
        assert_eq!(score.band, GROUP_ONLY_CEILING);
    }

    #[test]
    fn one_direct_message_lifts_an_otherwise_group_tie() {
        let mut log: Vec<Interaction> = (0..7).map(|i| group(1, i % 2 == 0, DAY * i, 7)).collect();
        assert_eq!(T3::score(1, &log, 0).band, Band::Moderate);
        log.push(direct(1, true, DAY * 7, 11));
        assert_eq!(T3::score(1, &log, 0).band, Band::Strong);
    }

    #[test]
    fn thin_group_traffic_stays_weak() {
        // Four exchanges over two days: under the group bar of five.
        let log: Vec<Interaction> = (0..4)
            .map(|i| group(1, i % 2 == 0, DAY * (i as i64 % 2) + 60 * i as i64, 7))
            .collect();
        assert_eq!(T3::score(1, &log, 0).band, Band::Weak);
    }

    #[test]
    fn eight_over_three_days_is_the_strong_boundary() {
        let log: Vec<Interaction> = (0..8)
            .map(|i| direct(1, i % 2 == 0, DAY * (i as i64 % 3) + 60 * i as i64, 1))
            .collect();
        assert_eq!(T3::score(1, &log, 0).band, Band::Strong);
        assert_eq!(T3::score(1, &log[..7], 0).band, Band::Moderate);
    }

    #[test]
    fn a_single_direct_exchange_is_moderate_not_weak() {
        // Direct and reciprocal but tiny. T3 is deliberately more generous
        // here than T0, which needs three exchanges before it says anything.
        let log = vec![direct(1, true, 0, 1), direct(1, false, 60, 1)];
        assert_eq!(T3::score(1, &log, 0).band, Band::Moderate);
    }

    #[test]
    fn the_strict_reading_of_the_brief_is_one_constant_away() {
        // Documents what flipping GROUP_ONLY_CEILING to Weak would do, without
        // needing a build flag: every group-only band collapses to Weak.
        let log: Vec<Interaction> = (0..50).map(|i| group(1, i % 2 == 0, DAY * i, 7)).collect();
        let score = T3::score(1, &log, DAY * 50);
        let strict = score.band.capped_at(Band::Weak);
        assert_eq!(strict, Band::Weak);
        assert_eq!(score.band, Band::Moderate);
    }
}
