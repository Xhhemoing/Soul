//! T4 — the Round 2 winner, kept verbatim as the rollback rule.
//!
//! ```text
//! # counts stage: every venue counts
//! Strong    if reciprocal ∧ any_direct ∧ count >= 10 ∧ active_days >= 3
//! Moderate  if reciprocal ∧ count >= 3
//! Weak      otherwise
//! then: a tie that was never one to one is capped at Moderate
//!
//! # recency stage
//! silence >= 360 -> Weak;  silence >= 180 -> one band lower
//! ```
//!
//! Nothing here is new. R2-SYNTHESIS picked this rule, and Round 3 keeps it
//! byte-compatible in behaviour so that adopting [`crate::T4D`] stays a
//! one-line revert rather than an archaeology exercise. The two Round 3
//! changes are structural and are asserted not to move any band:
//!
//! * the ladder moved into [`crate::gate`] and the silence step into
//!   [`crate::recency`], both shared with T4D;
//! * the counts come out of the split tally as sums, so `count` here is
//!   `direct + group` and `days` is the union of both venues' days.
//!
//! The known hole is the one fable-b found and R2-SYNTHESIS §潜在边界风险 2
//! recorded: `any_direct` is a boolean, so a group fan-out plus a single
//! private hello in each direction satisfies it. `tests/ablation.rs` pins that
//! failure on the `group_heavy_plus_one_direct_each_way` fixture instead of
//! describing it, and T4D is the answer to it.

use crate::constants::{
    GROUP_ONLY_CEILING, MODERATE_MIN_INTERACTIONS, STRONG_MIN_ACTIVE_DAYS, STRONG_MIN_INTERACTIONS,
};
use crate::gate::{self, Observed, Thresholds};
use crate::recency::{self, crossed_threshold};
use crate::types::{
    zh_common_weak_reason, zh_counts_total, zh_venue, Band, Detail, Interaction, Tally,
    TieAlgorithm, TieScore,
};

/// The bars, in whole interactions and whole days.
pub const THRESHOLDS: Thresholds = Thresholds {
    strong_min_count: STRONG_MIN_INTERACTIONS,
    strong_min_days: STRONG_MIN_ACTIVE_DAYS,
    moderate_min_count: MODERATE_MIN_INTERACTIONS,
};

pub struct T4;

impl T4 {
    /// What the ladder sees: every venue's rows, added together.
    pub fn observed(tally: &Tally) -> Observed {
        Observed {
            reciprocal: tally.is_reciprocal(),
            count: tally.interaction_count(),
            days: tally.active_day_count(),
        }
    }

    /// The counts stage, before silence.
    ///
    /// The ceiling is applied here rather than inside the ladder so that the
    /// product invariant — a tie that was never one to one is not a strong tie
    /// — is one visible line instead of an emergent property of two gates.
    pub fn counts_band(tally: &Tally) -> Band {
        let ladder = gate::band(T4::observed(tally), THRESHOLDS);
        if tally.any_direct() {
            ladder
        } else {
            ladder.capped_at(GROUP_ONLY_CEILING)
        }
    }

    /// The band after silence is applied, plus what the counts stage said
    /// before it.
    pub fn band_of(tally: &Tally, as_of_unix: i64) -> (Band, Band) {
        let before = T4::counts_band(tally);
        let after = recency::demote(before, tally.silent_days(as_of_unix));
        (after, before)
    }

    /// The counts clause, without a conclusion, so the recency clause can be
    /// bolted on after it.
    pub(crate) fn zh_reason(score: &TieScore) -> String {
        let venue = zh_venue(score.any_direct());
        match score.band {
            Band::Strong => format!(
                "{venue}，双方都发过消息，往来 {} 次不少于 {STRONG_MIN_INTERACTIONS} 次，分布的 {} 天也不少于 {STRONG_MIN_ACTIVE_DAYS} 天",
                score.interaction_count, score.active_day_count,
            ),
            Band::Moderate if !score.any_direct() => format!(
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

impl TieAlgorithm for T4 {
    const ID: &'static str = "T4";

    fn score(peer_id: u64, interactions: &[Interaction], as_of_unix: i64) -> TieScore {
        let tally = Tally::of(peer_id, interactions);
        let (band, band_before) = T4::band_of(&tally, as_of_unix);
        tally.to_score(band, Self::ID, as_of_unix, Detail::Demoted { band_before })
    }

    fn explain_zh(score: &TieScore) -> String {
        let counts = zh_counts_total(score);
        if let Some(reason) = zh_common_weak_reason(score, &counts) {
            return reason;
        }
        let band_before = match score.detail {
            Detail::Demoted { band_before } => band_before,
            Detail::RawCounts => score.band,
        };

        // The counts clause describes the band the counts produced, not the
        // one silence left behind.
        let mut before_score = score.clone();
        before_score.band = band_before;
        let reason = T4::zh_reason(&before_score);
        let silent = score.silent_days;

        let Some(threshold) = crossed_threshold(silent) else {
            return format!(
                "{counts}{reason}，最近一次往来距今 {silent} 天，还不到 {} 天，不用往下降，所以算{}。",
                crate::constants::DEMOTE_AFTER_SILENT_DAYS,
                score.band.as_zh(),
            );
        };
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
    /// them `silent` days before `as_of`. The counts stage calls this Strong.
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
                T4::counts_band(&Tally::of(1, &log)),
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
    fn ten_over_three_days_is_the_strong_boundary() {
        let log: Vec<Interaction> = (0..10)
            .map(|i| direct(1, i % 2 == 0, DAY * (i as i64 % 3) + 60 * i as i64, 1))
            .collect();
        assert_eq!(T4::score(1, &log, 0).band, Band::Strong);
        assert_eq!(T4::score(1, &log[..9], 0).band, Band::Moderate);
    }

    #[test]
    fn a_group_only_tie_is_capped_at_moderate_however_much_traffic() {
        let as_of = DAY * 10_000;
        let log: Vec<Interaction> = (0..100i64)
            .map(|i| group(1, i % 2 == 0, as_of - DAY * (100 - i), 7))
            .collect();
        let score = T4::score(1, &log, as_of);
        let Detail::Demoted { band_before } = score.detail else {
            panic!("expected demotion detail");
        };
        assert_eq!(band_before, GROUP_ONLY_CEILING);
        assert_ne!(score.band, Band::Strong);
    }

    #[test]
    fn one_private_hello_each_way_is_enough_to_lift_a_group_tie_which_is_the_defect() {
        // fable-b's finding, pinned as a unit test so the rollback rule's known
        // weakness is executable. Twenty group exchanges over ten days plus one
        // private message in each direction: T4 says Strong.
        let as_of = DAY * 10_000;
        let mut log: Vec<Interaction> = (0..20i64)
            .map(|i| group(1, i % 2 == 0, as_of - DAY * (10 - i / 2), 7))
            .collect();
        assert_eq!(T4::score(1, &log, as_of).band, Band::Moderate);
        log.push(direct(1, true, as_of - DAY * 2, 8));
        log.push(direct(1, false, as_of - DAY * 2 + 60, 8));
        assert_eq!(T4::score(1, &log, as_of).band, Band::Strong);
    }

    #[test]
    fn silence_is_measured_from_the_newest_exchange_not_the_oldest() {
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
        assert_eq!(
            score.detail,
            Detail::Demoted {
                band_before: Band::Weak
            }
        );
    }
}
