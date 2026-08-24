//! T4D — T4 with the counts gates moved onto the one-to-one rows.
//!
//! ```text
//! # counts stage: only one-to-one rows are counted
//! direct_reciprocal = direct_out >= 1 ∧ direct_in >= 1
//!
//! Strong    if direct_reciprocal ∧ direct_count >= 10 ∧ direct_active_days >= 3
//! Moderate  if direct_reciprocal ∧ direct_count >= 3
//! Weak      otherwise
//!
//! # recency stage: the clock is the newest exchange in ANY venue
//! silence >= 360 -> Weak;  silence >= 180 -> one band lower
//! ```
//!
//! ## What changed and why
//!
//! T4 asks "did these two ever speak privately?" and then bands on how much
//! they said *anywhere*. fable-b showed what that buys: fan out a message to a
//! two-hundred-person group every day for a month, exchange one private hello
//! each way, and the boolean is satisfied while the counts come almost
//! entirely from a room neither person chose. R2-SYNTHESIS §潜在边界风险 2
//! asked Round 3 to move the count and day gates onto the direct rows. That is
//! the whole of T4D.
//!
//! Three consequences, all deliberate:
//!
//! * A tie only ever seen in a group has `direct_out = direct_in = 0`, so it
//!   fails the reciprocity gate and lands on **Weak**. T4's Moderate ceiling
//!   for group-only ties has no equivalent here — it cannot have one, because
//!   the quantity the ceiling applied to is not counted any more. This is the
//!   one place T4D is harsher than T4, and it is priced in `REPORT.md`.
//! * The group rows still reach the user: they are reported in the split
//!   counts and named in the explanation. Not counted is not the same as not
//!   shown.
//! * The **recency clock still reads every venue**. Somebody who answered you
//!   in a group yesterday has not gone quiet, so the dormancy step does not
//!   fire on them, even though that group message can never lift their band.
//!   Instructed by the Round 3 brief; the risk is written up in `REPORT.md`
//!   §任一场地的近因.
//!
//! ## The bound on how much this can cost
//!
//! T4D counts a subset of T4's rows over a subset of T4's days, and direct
//! reciprocity implies overall reciprocity, so — the ladder being monotone —
//! T4D's counts band never exceeds T4's, and neither does its final band. On
//! evidence with no group rows at all the two inputs are literally the same
//! numbers, so the two rules agree exactly. Both statements are properties,
//! not observations: see `tests/direct_gate.rs`.

use crate::constants::{
    MODERATE_MIN_INTERACTIONS, STRONG_MIN_ACTIVE_DAYS, STRONG_MIN_INTERACTIONS,
};
use crate::gate::{self, Observed, Thresholds};
use crate::recency::{self, crossed_threshold};
use crate::types::{
    zh_common_weak_reason, zh_counts_split, Band, Detail, Interaction, Tally, TieAlgorithm,
    TieScore,
};

/// The bars. The same three numbers as T4 — Goal 1's 3 / 10 / 3 — read in a
/// different unit: one-to-one exchanges rather than exchanges.
pub const THRESHOLDS: Thresholds = Thresholds {
    strong_min_count: STRONG_MIN_INTERACTIONS,
    strong_min_days: STRONG_MIN_ACTIVE_DAYS,
    moderate_min_count: MODERATE_MIN_INTERACTIONS,
};

pub struct T4D;

impl T4D {
    /// What the ladder sees: the one-to-one rows, and nothing else.
    pub fn observed(tally: &Tally) -> Observed {
        Observed {
            reciprocal: tally.is_direct_reciprocal(),
            count: tally.direct_count(),
            days: tally.direct_active_day_count(),
        }
    }

    /// The counts stage, before silence.
    ///
    /// No venue ceiling and no `any_direct` boolean: a tie with no one-to-one
    /// rows has a direct count of zero and fails the first gate.
    pub fn counts_band(tally: &Tally) -> Band {
        gate::band(T4D::observed(tally), THRESHOLDS)
    }

    /// The band after silence is applied, plus what the counts stage said
    /// before it.
    pub fn band_of(tally: &Tally, as_of_unix: i64) -> (Band, Band) {
        let before = T4D::counts_band(tally);
        let after = recency::demote(before, tally.silent_days(as_of_unix));
        (after, before)
    }

    /// The counts clause, without a conclusion.
    ///
    /// Every branch names the one-to-one count, because that is the number the
    /// band was decided on, and every branch that has group traffic to explain
    /// says what happened to it.
    pub(crate) fn zh_reason(score: &TieScore) -> String {
        let direct = score.direct_count();
        let head = match score.band {
            Band::Strong => format!(
                "你们一对一聊过 {direct} 次，双方都发过，不少于 {STRONG_MIN_INTERACTIONS} 次，而且分布在 {} 天里，也不少于 {STRONG_MIN_ACTIVE_DAYS} 天",
                score.direct_active_day_count,
            ),
            Band::Moderate if direct >= STRONG_MIN_INTERACTIONS => format!(
                "你们一对一聊过 {direct} 次，双方都发过，次数够多，但只分布在 {} 天里，不到 {STRONG_MIN_ACTIVE_DAYS} 天，看不出是长期习惯",
                score.direct_active_day_count,
            ),
            Band::Moderate => format!(
                "你们一对一聊过 {direct} 次，双方都发过，但还不到 {STRONG_MIN_INTERACTIONS} 次",
            ),
            Band::Weak if direct == 0 => {
                "你们从来没有单独聊过，一对一 0 次，看不出你们私下有来往".to_string()
            }
            Band::Weak if score.direct_in_count == 0 => format!(
                "一对一的 {direct} 次全是你发出的，对方一次也没有单独回过你",
            ),
            Band::Weak if score.direct_out_count == 0 => format!(
                "一对一的 {direct} 次全是对方发来的，你一次也没有单独回过",
            ),
            Band::Weak => format!(
                "你们一对一聊过 {direct} 次，双方都发过，但不到 {MODERATE_MIN_INTERACTIONS} 次，还只是打过招呼",
            ),
        };
        format!("{head}{}", T4D::zh_group_note(score))
    }

    /// What the group traffic was allowed to do, said out loud.
    ///
    /// Silent when there is none, so a purely private tie does not get a
    /// sentence about a room it was never in.
    fn zh_group_note(score: &TieScore) -> String {
        if score.group_count() == 0 {
            return String::new();
        }
        format!(
            "；你们在群里还有 {} 次往来，那只说明你们常在同一个场合，不算进这一档",
            score.group_count(),
        )
    }
}

impl TieAlgorithm for T4D {
    const ID: &'static str = "T4D";

    fn score(peer_id: u64, interactions: &[Interaction], as_of_unix: i64) -> TieScore {
        let tally = Tally::of(peer_id, interactions);
        let (band, band_before) = T4D::band_of(&tally, as_of_unix);
        tally.to_score(band, Self::ID, as_of_unix, Detail::Demoted { band_before })
    }

    fn explain_zh(score: &TieScore) -> String {
        let counts = zh_counts_split(score);
        if let Some(reason) = zh_common_weak_reason(score, &counts) {
            return reason;
        }
        let band_before = match score.detail {
            Detail::Demoted { band_before } => band_before,
            Detail::RawCounts => score.band,
        };

        let mut before_score = score.clone();
        before_score.band = band_before;
        let reason = T4D::zh_reason(&before_score);
        let silent = score.silent_days;

        let Some(threshold) = crossed_threshold(silent) else {
            return format!(
                "{counts}{reason}，最近一次往来距今 {silent} 天，还不到 {} 天，不用往下降，所以算{}。",
                crate::constants::DEMOTE_AFTER_SILENT_DAYS,
                score.band.as_zh(),
            );
        };
        if score.band == band_before {
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
    use crate::T4;

    /// `directs` one-to-one exchanges alternating direction over `days`
    /// consecutive days, plus `group_per_day` group exchanges on each of the
    /// same days. Everything ends `newest_days_ago` days before `as_of`.
    fn mixed(
        days: i64,
        directs_per_day: i64,
        group_per_day: i64,
        newest_days_ago: i64,
    ) -> (Vec<Interaction>, i64) {
        let as_of = DAY * 10_000;
        let mut log = Vec::new();
        for day in 0..days {
            let midnight = as_of - DAY * (newest_days_ago + days - 1 - day);
            for slot in 0..directs_per_day {
                log.push(direct(1, (day + slot) % 2 == 0, midnight + slot * 600, 1));
            }
            for slot in 0..group_per_day {
                log.push(group(
                    1,
                    (day + slot) % 2 == 0,
                    midnight + 3_600 + slot * 600,
                    2,
                ));
            }
        }
        (log, as_of)
    }

    #[test]
    fn twelve_private_exchanges_over_six_days_are_strong() {
        let (log, as_of) = mixed(6, 2, 0, 3);
        let score = T4D::score(1, &log, as_of);
        assert_eq!(score.direct_count(), 12);
        assert_eq!(score.direct_active_day_count, 6);
        assert_eq!(score.band, Band::Strong);
    }

    #[test]
    fn a_group_flood_with_one_private_hello_each_way_is_not_strong() {
        // The fixture the Round 3 brief names, as a unit test. Thirty group
        // exchanges over ten days, plus exactly one private message in each
        // direction: two direct exchanges, which is under the Moderate bar of
        // three, so Weak.
        let (mut log, as_of) = mixed(10, 0, 3, 2);
        log.push(direct(1, true, as_of - DAY * 2 + 100, 3));
        log.push(direct(1, false, as_of - DAY * 2 + 200, 3));
        let score = T4D::score(1, &log, as_of);
        assert_eq!(score.interaction_count, 32);
        assert_eq!(score.direct_count(), 2);
        assert!(score.is_direct_reciprocal());
        assert_eq!(score.band, Band::Weak);
        // And the rule it replaces calls the same evidence a close friendship.
        assert_eq!(T4::score(1, &log, as_of).band, Band::Strong);
    }

    #[test]
    fn three_private_exchanges_reach_moderate_and_no_further() {
        let (mut log, as_of) = mixed(10, 0, 3, 2);
        log.push(direct(1, true, as_of - DAY * 3 + 100, 3));
        log.push(direct(1, false, as_of - DAY * 3 + 200, 3));
        log.push(direct(1, true, as_of - DAY * 2 + 300, 3));
        let score = T4D::score(1, &log, as_of);
        assert_eq!(score.direct_count(), 3);
        assert_eq!(score.band, Band::Moderate);
    }

    #[test]
    fn private_messages_that_only_go_one_way_are_weak_however_busy_the_group_is() {
        let (mut log, as_of) = mixed(10, 0, 4, 1);
        for i in 0..8 {
            log.push(direct(1, true, as_of - DAY * 2 + i * 100, 3));
        }
        let score = T4D::score(1, &log, as_of);
        assert_eq!(score.direct_out_count, 8);
        assert_eq!(score.direct_in_count, 0);
        assert!(score.is_reciprocal(), "the group traffic goes both ways");
        assert!(!score.is_direct_reciprocal());
        assert_eq!(score.band, Band::Weak);
    }

    #[test]
    fn a_group_only_tie_is_weak_rather_than_capped_at_moderate() {
        let (log, as_of) = mixed(50, 0, 2, 1);
        let score = T4D::score(1, &log, as_of);
        assert_eq!(score.interaction_count, 100);
        assert_eq!(score.direct_count(), 0);
        assert_eq!(score.band, Band::Weak);
        // The difference from the rollback rule, stated where it happens.
        assert_eq!(T4::score(1, &log, as_of).band, Band::Moderate);
    }

    #[test]
    fn a_group_message_yesterday_stops_the_dormancy_step() {
        // The instruction in the Round 3 brief: the recency clock reads every
        // venue. A private history that would be demoted for silence is held
        // where it is by one group message the day before `as_of`.
        let (mut log, as_of) = mixed(6, 2, 0, 300);
        assert_eq!(T4D::score(1, &log, as_of).band, Band::Moderate);
        log.push(group(1, false, as_of - DAY, 9));
        let score = T4D::score(1, &log, as_of);
        assert_eq!(score.silent_days, 1);
        assert_eq!(score.direct_count(), 12);
        assert_eq!(score.band, Band::Strong);
        // The group message did not add anything to the band's own evidence.
        let Detail::Demoted { band_before } = score.detail else {
            panic!("expected demotion detail");
        };
        assert_eq!(band_before, Band::Strong);
        // The private conversation itself is still where it was: the newest
        // one-to-one row sits ten minutes into the day 300 days back, so its
        // floored age is 299 whole days.
        assert_eq!(crate::age_days(score.last_direct_contact_unix, as_of), 299);
    }

    #[test]
    fn the_boundaries_are_on_the_direct_count_not_the_total() {
        // Nine private exchanges over three days plus a hundred group ones is
        // Moderate; the tenth private exchange is what makes it Strong.
        let (mut log, as_of) = mixed(3, 3, 30, 1);
        assert_eq!(T4D::score(1, &log, as_of).direct_count(), 9);
        assert_eq!(T4D::score(1, &log, as_of).band, Band::Moderate);
        log.push(direct(1, false, as_of - DAY + 5_000, 1));
        assert_eq!(T4D::score(1, &log, as_of).band, Band::Strong);
    }

    #[test]
    fn private_exchanges_crammed_into_two_days_do_not_reach_strong() {
        let (log, as_of) = mixed(2, 10, 0, 1);
        let score = T4D::score(1, &log, as_of);
        assert_eq!(score.direct_count(), 20);
        assert_eq!(score.direct_active_day_count, 2);
        assert_eq!(score.band, Band::Moderate);
    }

    #[test]
    fn nothing_observed_is_weak_and_not_demoted_from_anything() {
        let score = T4D::score(1, &[], DAY * 10_000);
        assert_eq!(score.band, Band::Weak);
        assert_eq!(score.silent_days, 0);
        assert_eq!(score.last_direct_contact_unix, 0);
    }
}
