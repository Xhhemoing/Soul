//! T2 — the RFM band, borrowed from customer analytics.
//!
//! Three ordinal features, each mapped to 1..=3, then summed:
//!
//! ```text
//! recency   (days since last contact):  <=7 -> 3,  <=30 -> 2,  else 1
//! frequency (interaction count):        >=10 -> 3, >=3  -> 2,  else 1
//! "monetary" (active days):             >=5 -> 3,  >=2  -> 2,  else 1
//!
//! reciprocal && r+f+m >= 8 -> Strong
//! reciprocal && r+f+m >= 5 -> Moderate
//! otherwise                -> Weak
//! ```
//!
//! Nothing is spent in this product, so the monetary axis is active days: how
//! much of the user's calendar the person occupies. The sum is compensatory,
//! which is the interesting difference from T0 and T3 — a shortfall on one
//! axis can be bought back on another, so a tie can reach Strong on ten
//! messages over two recent days, which the two gated rules refuse. Whether
//! that is sensitivity or noise is the question Round 1 has to answer; it is
//! measured in the fixtures rather than asserted here.
//!
//! Recency is floored to whole UTC days so the boundaries are exact: at
//! 7 days and 0 seconds the tie is still on the top rung, at 8 days it is not.

use crate::types::{
    age_days_floor, zh_common_weak_reason, zh_counts, Band, Interaction, Tally, TieAlgorithm,
    TieScore,
};

pub const RECENCY_TOP_MAX_DAYS: i64 = 7;
pub const RECENCY_MID_MAX_DAYS: i64 = 30;
pub const FREQUENCY_TOP_MIN: u64 = 10;
pub const FREQUENCY_MID_MIN: u64 = 3;
pub const ACTIVE_DAYS_TOP_MIN: u64 = 5;
pub const ACTIVE_DAYS_MID_MIN: u64 = 2;
pub const STRONG_MIN_SUM: u8 = 8;
pub const MODERATE_MIN_SUM: u8 = 5;

/// The three rungs, before they are summed. Exposed for tests and for a
/// developer view; never shown to the user.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rungs {
    pub recency: u8,
    pub frequency: u8,
    pub active_days: u8,
}

impl Rungs {
    pub const fn sum(self) -> u8 {
        self.recency + self.frequency + self.active_days
    }
}

pub struct T2;

impl T2 {
    pub fn rungs(peer_id: u64, interactions: &[Interaction], now_unix: i64) -> Rungs {
        let tally = Tally::of(peer_id, interactions);
        T2::rungs_of(&tally, now_unix)
    }

    /// The rule itself, over an already-built tally.
    pub fn band_of(tally: &Tally, now_unix: i64) -> Band {
        let sum = T2::rungs_of(tally, now_unix).sum();
        if tally.is_reciprocal() && sum >= STRONG_MIN_SUM {
            Band::Strong
        } else if tally.is_reciprocal() && sum >= MODERATE_MIN_SUM {
            Band::Moderate
        } else {
            Band::Weak
        }
    }

    pub fn rungs_of(tally: &Tally, now_unix: i64) -> Rungs {
        let recency_days = age_days_floor(tally.last_contact, now_unix);
        let recency = if tally.matched.is_empty() {
            1
        } else if recency_days <= RECENCY_TOP_MAX_DAYS {
            3
        } else if recency_days <= RECENCY_MID_MAX_DAYS {
            2
        } else {
            1
        };
        let count = tally.interaction_count();
        let frequency = if count >= FREQUENCY_TOP_MIN {
            3
        } else if count >= FREQUENCY_MID_MIN {
            2
        } else {
            1
        };
        let days = tally.active_day_count();
        let active_days = if days >= ACTIVE_DAYS_TOP_MIN {
            3
        } else if days >= ACTIVE_DAYS_MID_MIN {
            2
        } else {
            1
        };
        Rungs {
            recency,
            frequency,
            active_days,
        }
    }
}

impl TieAlgorithm for T2 {
    const ID: &'static str = "T2";

    fn score(peer_id: u64, interactions: &[Interaction], now_unix: i64) -> TieScore {
        let tally = Tally::of(peer_id, interactions);
        let band = T2::band_of(&tally, now_unix);
        tally.into_score(band, Self::ID)
    }

    fn explain_zh(score: &TieScore) -> String {
        if let Some(reason) = zh_common_weak_reason(score) {
            return reason;
        }
        let counts = zh_counts(score);
        match score.band {
            Band::Strong => format!(
                "{counts}最近往来、往来次数、活跃天数这三样都不差，而且双方都在说话，所以算强联系。",
            ),
            Band::Moderate => format!(
                "{counts}这三样里——最近有没有往来、一共来往过多少次、一共有多少天在联系——你们只占了一部分，所以算中等联系。",
            ),
            Band::Weak => format!(
                "{counts}最近没有新的往来，次数和天数也都偏少，三样都不占，所以算弱联系。",
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{direct, DAY};

    #[test]
    fn recency_boundaries_are_whole_days() {
        let log = vec![direct(1, true, 0, 1), direct(1, false, 0, 1)];
        // Exactly seven days and 23 hours later is still "within 7 days" by
        // floored day counting; eight days is not.
        assert_eq!(T2::rungs(1, &log, DAY * 7 + 3_600 * 23).recency, 3);
        assert_eq!(T2::rungs(1, &log, DAY * 8).recency, 2);
        assert_eq!(T2::rungs(1, &log, DAY * 30).recency, 2);
        assert_eq!(T2::rungs(1, &log, DAY * 31).recency, 1);
    }

    #[test]
    fn axes_compensate_for_each_other() {
        // Ten reciprocal messages over two recent days: recency 3, frequency 3,
        // active days 2 -> 8 -> Strong, although the span gate in T0 and T3
        // would refuse it. Recorded deliberately: this is T2's signature.
        let mut log = Vec::new();
        for i in 0..10 {
            log.push(direct(
                1,
                i % 2 == 0,
                DAY * (i as i64 % 2) + 60 * i as i64,
                1,
            ));
        }
        let now = DAY * 3;
        assert_eq!(
            T2::rungs(1, &log, now),
            Rungs {
                recency: 3,
                frequency: 3,
                active_days: 2
            }
        );
        assert_eq!(T2::score(1, &log, now).band, Band::Strong);
    }

    #[test]
    fn two_messages_today_already_reach_moderate() {
        // Floor of the reciprocal case: 3 + 1 + 1 = 5. T0 calls the same
        // evidence Weak. Noted as a sensitivity difference, not as a bug.
        let log = vec![direct(1, true, 0, 1), direct(1, false, 60, 1)];
        assert_eq!(T2::score(1, &log, 3_600).band, Band::Moderate);
    }

    #[test]
    fn one_sided_never_leaves_weak_however_high_the_rungs() {
        let log: Vec<Interaction> = (0..40).map(|i| direct(1, true, DAY * i, 1)).collect();
        let score = T2::score(1, &log, DAY * 40);
        assert_eq!(T2::rungs(1, &log, DAY * 40).sum(), 9);
        assert_eq!(score.band, Band::Weak);
    }
}
