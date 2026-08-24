//! T1 — recency-weighted exponential decay.
//!
//! Same reciprocity gate and same active-day gate as T0, but the count is
//! replaced by a weighted mass:
//!
//! ```text
//! w(i)  = venue(i) * 2 ^ (-age_days(i) / 90)
//! mass  = sum w(i)
//! reciprocal && mass >= 8 && active_days >= 3 -> Strong
//! reciprocal && mass >= 2                     -> Moderate
//! otherwise                                   -> Weak
//! ```
//!
//! `exp(-ln2 * age / half_life)` is the half-life spelling of the same curve:
//! an exchange 90 days old counts half, 180 days old a quarter. Group venue is
//! worth 0.4 of a direct exchange, which is the one place this candidate
//! borrows from T3 — being in the same group chat is weaker evidence of a tie
//! than writing to each other.
//!
//! The cost of the weighting is auditability. `TieScore` carries raw,
//! unweighted counts precisely so the user can still check the numbers, but
//! the mass that actually decided the band is a float the user cannot
//! reproduce by counting. See REPORT.md; this is the main strike against T1.

use crate::types::{
    age_days_exact, zh_common_weak_reason, zh_counts, Band, Interaction, Tally, TieAlgorithm,
    TieScore,
};

/// Days after which an exchange counts half as much.
///
/// 90 rather than 30: a monthly half-life demotes a close friend you happen
/// not to have written to since the spring, and the graph is supposed to be a
/// picture of the user's life, not of the last six weeks.
pub const HALF_LIFE_DAYS: f64 = 90.0;
/// One-to-one exchanges carry their full weight.
pub const DIRECT_WEIGHT: f64 = 1.0;
/// Being in the same group is evidence, but weaker evidence.
pub const GROUP_WEIGHT: f64 = 0.4;
pub const STRONG_MIN_MASS: f64 = 8.0;
pub const MODERATE_MIN_MASS: f64 = 2.0;
pub const STRONG_MIN_ACTIVE_DAYS: u64 = 3;

pub struct T1;

impl T1 {
    /// The decayed, venue-weighted weight of one exchange.
    pub fn weight(interaction: &Interaction, now_unix: i64) -> f64 {
        let age_days = age_days_exact(interaction.occurred_at_unix, now_unix);
        let venue = if interaction.venue_direct {
            DIRECT_WEIGHT
        } else {
            GROUP_WEIGHT
        };
        venue * (-f64::ln(2.0) * age_days / HALF_LIFE_DAYS).exp()
    }

    /// The weighted mass behind the band, exposed so a caller can show it in a
    /// developer view and a test can pin it. Never shown to the user.
    pub fn mass(peer_id: u64, interactions: &[Interaction], now_unix: i64) -> f64 {
        interactions
            .iter()
            .filter(|row| row.peer_id == peer_id)
            .map(|row| T1::weight(row, now_unix))
            .sum()
    }

    /// The mass of an already-built tally.
    ///
    /// Summing the tally's own rows rather than the caller's slice means the
    /// mass and the counts can never describe different sets.
    pub fn mass_of(tally: &Tally, now_unix: i64) -> f64 {
        tally
            .matched
            .iter()
            .map(|row| T1::weight(row, now_unix))
            .sum()
    }

    /// The rule itself, over an already-built tally.
    pub fn band_of(tally: &Tally, now_unix: i64) -> Band {
        let mass = T1::mass_of(tally, now_unix);
        if tally.is_reciprocal()
            && mass >= STRONG_MIN_MASS
            && tally.active_day_count() >= STRONG_MIN_ACTIVE_DAYS
        {
            Band::Strong
        } else if tally.is_reciprocal() && mass >= MODERATE_MIN_MASS {
            Band::Moderate
        } else {
            Band::Weak
        }
    }
}

impl TieAlgorithm for T1 {
    const ID: &'static str = "T1";

    fn score(peer_id: u64, interactions: &[Interaction], now_unix: i64) -> TieScore {
        let tally = Tally::of(peer_id, interactions);
        let band = T1::band_of(&tally, now_unix);
        tally.into_score(band, Self::ID)
    }

    fn explain_zh(score: &TieScore) -> String {
        if let Some(reason) = zh_common_weak_reason(score) {
            return reason;
        }
        let counts = zh_counts(score);
        match score.band {
            Band::Strong => format!(
                "{counts}你们互相都发过消息，最近这段时间往来一直没断，天数也够分散，所以算强联系。",
            ),
            // Which of the two gates it missed is readable off the counts, so
            // the sentence names the real one instead of listing both.
            Band::Moderate if score.active_day_count < STRONG_MIN_ACTIVE_DAYS => format!(
                "{counts}你们互相都发过消息，但往来都挤在 {} 天里，还看不出是长期保持的联系，所以算中等联系。",
                score.active_day_count,
            ),
            Band::Moderate => format!(
                "{counts}你们互相都发过消息，但最近这段时间的往来比以前少了，所以算中等联系。",
            ),
            // Reciprocal and not rare, yet still weak: everything on record is
            // old. This is the case T0 cannot express at all.
            Band::Weak if score.interaction_count >= 3 => format!(
                "{counts}你们以前互相都发过消息，但最近很久没有新的往来了，所以现在只算弱联系。要是你们又聊起来，这一档会自己涨回去。",
            ),
            Band::Weak => format!(
                "{counts}你们互相都发过消息，但次数很少，也没有新的往来，所以算弱联系。",
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{direct, group, DAY};

    #[test]
    fn one_half_life_halves_the_weight() {
        let today = direct(1, true, 0, 1);
        assert!((T1::weight(&today, 0) - 1.0).abs() < 1e-12);
        let ninety_days_old = direct(1, true, 0, 1);
        let w = T1::weight(&ninety_days_old, DAY * 90);
        assert!((w - 0.5).abs() < 1e-9, "half-life weight was {w}");
        let w = T1::weight(&ninety_days_old, DAY * 180);
        assert!((w - 0.25).abs() < 1e-9, "two half-lives gave {w}");
    }

    #[test]
    fn group_venue_is_worth_less() {
        let now = 0;
        assert!((T1::weight(&group(1, true, 0, 1), now) - GROUP_WEIGHT).abs() < 1e-12);
        assert!((T1::weight(&direct(1, true, 0, 1), now) - DIRECT_WEIGHT).abs() < 1e-12);
    }

    #[test]
    fn evidence_from_the_future_is_not_worth_more_than_today() {
        // Imported logs sometimes carry clocks that run ahead. Clamping the age
        // at zero keeps the weight at 1.0 instead of letting it grow.
        let tomorrow = direct(1, true, DAY, 1);
        assert!((T1::weight(&tomorrow, 0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn recent_reciprocal_traffic_is_strong_but_the_same_traffic_years_later_is_not() {
        let log: Vec<Interaction> = (0..12).map(|i| direct(1, i % 2 == 0, DAY * i, 1)).collect();
        assert_eq!(T1::score(1, &log, DAY * 12).band, Band::Strong);
        assert_eq!(T1::score(1, &log, DAY * 3650).band, Band::Weak);
    }

    #[test]
    fn raw_counts_stay_unweighted() {
        let log: Vec<Interaction> = (0..12).map(|i| group(1, i % 2 == 0, DAY * i, 1)).collect();
        let score = T1::score(1, &log, DAY * 4000);
        assert_eq!(score.band, Band::Weak);
        // The band decayed; the audit trail did not.
        assert_eq!(score.interaction_count, 12);
        assert_eq!(score.active_day_count, 12);
        assert_eq!(score.outgoing_count, 6);
        assert_eq!(score.incoming_count, 6);
    }
}
