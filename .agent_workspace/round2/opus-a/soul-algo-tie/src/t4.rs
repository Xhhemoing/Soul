//! T4 — T3, then one integer step down for silence.
//!
//! ```text
//! band = T3(tally)                                 # counts, unweighted
//! silence = max(0, as_of - last_contact) / 86400   # whole UTC days
//!
//! if silence >= 360 -> Weak
//! else if silence >= 180 -> one band lower
//! else -> band
//! ```
//!
//! Same hole as T3R fills — a tie that stopped in 2019 is not a current strong
//! tie — and the same two constants, 180 and 360, that T3R uses for its second
//! and fourth buckets. The difference is where the arithmetic happens: T3R
//! re-weights every exchange and asks the user to add quarters; T4 keeps the
//! counts exactly as they are and moves the answer one rung. What the user has
//! to check is "how many days since we last spoke", which is one subtraction
//! against a date already on their screen.
//!
//! The group ceiling is inherited from T3 and applies before demotion, so a
//! group-only tie is never Strong at any point in the computation — demotion
//! can only ever take a band further down.

use crate::constants::{DEMOTE_AFTER_SILENT_DAYS, WEAK_AFTER_SILENT_DAYS};
use crate::t3::T3;
use crate::types::{
    zh_common_weak_reason, zh_counts, Band, Detail, Interaction, Tally, TieAlgorithm, TieScore,
};

pub struct T4;

impl T4 {
    /// The band after silence is applied, plus what T3 had said before it.
    pub fn band_of(tally: &Tally, as_of_unix: i64) -> (Band, Band) {
        let before = T3::band_of(tally, as_of_unix);
        let silence = tally.silent_days(as_of_unix);
        let after = if tally.is_empty() {
            before
        } else if silence >= WEAK_AFTER_SILENT_DAYS {
            Band::Weak
        } else if silence >= DEMOTE_AFTER_SILENT_DAYS {
            before.demoted_once()
        } else {
            before
        };
        (after, before)
    }
}

impl TieAlgorithm for T4 {
    const ID: &'static str = "T4";

    fn score(peer_id: u64, interactions: &[Interaction], as_of_unix: i64) -> TieScore {
        let tally = Tally::of(peer_id, interactions);
        let (band, band_before) = T4::band_of(&tally, as_of_unix);
        tally.to_score(band, Self::ID, as_of_unix, Detail::Demoted { band_before })
    }

    fn explain_zh(score: &TieScore) -> String {
        if let Some(reason) = zh_common_weak_reason(score) {
            return reason;
        }
        let band_before = match score.detail {
            Detail::Demoted { band_before } => band_before,
            _ => score.band,
        };

        // The counts reason is T3's, word for word, because the counts part of
        // the rule *is* T3. Only the recency clause is T4's own.
        let mut before_score = score.clone();
        before_score.band = band_before;
        let counts = zh_counts(score);
        let reason = T3::zh_reason(&before_score);
        let silent = score.silent_days;
        let threshold = if silent >= WEAK_AFTER_SILENT_DAYS {
            WEAK_AFTER_SILENT_DAYS
        } else {
            DEMOTE_AFTER_SILENT_DAYS
        };

        if silent < DEMOTE_AFTER_SILENT_DAYS {
            return format!(
                "{counts}{reason}，最近一次往来距今 {silent} 天，还不到 {DEMOTE_AFTER_SILENT_DAYS} 天，不用往下降，所以算{}。",
                score.band.as_zh(),
            );
        }
        if score.band == band_before {
            // Already at the bottom rung, so silence has nothing left to take.
            return format!(
                "{counts}{reason}；你们最近一次往来距今 {silent} 天，已经不少于 {threshold} 天没有联系，本来就已经是最弱的一档，所以还是{}。",
                score.band.as_zh(),
            );
        }
        format!(
            "{counts}{reason}，本来可以算{}；不过你们最近一次往来距今 {silent} 天，已经不少于 {threshold} 天没有联系，所以往下降到{}。要是你们又聊起来，这一档会自己涨回去。",
            band_before.as_zh(),
            score.band.as_zh(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{direct, group, DAY};

    /// Twelve reciprocal one-to-one exchanges over six days, the newest of
    /// them `silent` days before `as_of`. T3 calls this Strong.
    fn quiet_for(silent: i64) -> (Vec<Interaction>, i64) {
        let as_of = DAY * 10_000 + 12 * 3_600;
        let log = (0..6i64)
            .flat_map(|day| {
                let midnight = DAY * (10_000 - (silent + 5 - day));
                [
                    direct(1, true, midnight + 3_600, 1),
                    direct(1, false, midnight + 7_200, 1),
                ]
            })
            .collect();
        (log, as_of)
    }

    #[test]
    fn the_demotion_boundaries_are_exact_whole_days() {
        for (silent, expected) in [
            (0, Band::Strong),
            (179, Band::Strong),
            (180, Band::Moderate),
            (359, Band::Moderate),
            (360, Band::Weak),
            (5_000, Band::Weak),
        ] {
            let (log, as_of) = quiet_for(silent);
            let score = T4::score(1, &log, as_of);
            assert_eq!(score.silent_days, silent);
            assert_eq!(score.band, expected, "after {silent} silent days");
            assert_eq!(
                T3::score(1, &log, as_of).band,
                Band::Strong,
                "the fixture must be Strong before demotion"
            );
        }
    }

    #[test]
    fn demotion_only_ever_goes_down() {
        for silent in [0, 100, 179, 180, 359, 360, 1_000] {
            let (log, as_of) = quiet_for(silent);
            let score = T4::score(1, &log, as_of);
            let Detail::Demoted { band_before } = score.detail else {
                panic!("expected demotion detail");
            };
            assert!(score.band.rank() <= band_before.rank());
        }
    }

    #[test]
    fn a_moderate_tie_that_went_quiet_becomes_weak() {
        // Twenty group exchanges over ten days, all around 200 days ago: T3
        // caps it at Moderate, silence takes the last rung.
        let as_of = DAY * 10_000;
        let log: Vec<Interaction> = (0..20i64)
            .map(|i| group(1, i % 2 == 0, as_of - DAY * (209 - i / 2), 7))
            .collect();
        assert_eq!(T3::score(1, &log, as_of).band, Band::Moderate);
        assert_eq!(T4::score(1, &log, as_of).band, Band::Weak);
    }

    #[test]
    fn a_group_only_tie_is_never_strong_before_or_after_demotion() {
        let as_of = DAY * 10_000;
        let log: Vec<Interaction> = (0..100i64)
            .map(|i| group(1, i % 2 == 0, as_of - DAY * (100 - i), 7))
            .collect();
        let score = T4::score(1, &log, as_of);
        let Detail::Demoted { band_before } = score.detail else {
            panic!("expected demotion detail");
        };
        assert_ne!(band_before, Band::Strong);
        assert_ne!(score.band, Band::Strong);
    }

    #[test]
    fn silence_is_measured_from_the_newest_exchange_not_the_oldest() {
        // A long-dead history plus one message last week is not a dormant tie.
        let as_of = DAY * 10_000;
        let mut log: Vec<Interaction> = (0..12i64)
            .map(|i| direct(1, i % 2 == 0, as_of - DAY * (2_000 - i), 1))
            .collect();
        assert_eq!(T4::score(1, &log, as_of).band, Band::Weak);
        log.push(direct(1, true, as_of - DAY * 3, 1));
        assert_eq!(T4::score(1, &log, as_of).band, Band::Strong);
    }

    #[test]
    fn nothing_observed_is_weak_and_not_demoted_from_anything() {
        let score = T4::score(1, &[], DAY * 10_000);
        assert_eq!(score.band, Band::Weak);
        assert_eq!(score.silent_days, 0);
    }
}
