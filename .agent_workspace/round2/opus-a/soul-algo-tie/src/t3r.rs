//! T3R — T3's gate over bucketed, decayed counts. Integer arithmetic only.
//!
//! fable-a's frozen synthesis of T3 (Granovetter gate) and T1 (recency
//! weighting), with T1's continuous `exp(-λΔt)` replaced by a four-step
//! table so the user can reproduce the arithmetic on paper:
//!
//! ```text
//! age_days(i) = max(0, as_of - occurred_at(i)) / 86400
//!
//! w(i) = 1    if   0 ≤ age < 90     # 4 quarter-units
//!      = 1/2  if  90 ≤ age < 180    # 2
//!      = 1/4  if 180 ≤ age < 360    # 1
//!      = 0    if 360 ≤ age          # 0
//!
//! eff_count_milli = Σ w(i)                    in quarter-units
//! eff_days_milli  = Σ w(latest event of day)  in quarter-units, over active days
//!
//! Strong    if reciprocal ∧ any_direct ∧ eff_count_milli ≥ 40 ∧ eff_days_milli ≥ 12
//! Moderate  if reciprocal ∧ eff_count_milli ≥ 12
//! Weak      otherwise
//! ```
//!
//! ## Why quarter-units
//!
//! The weights are 1, ½, ¼ and 0, so multiplying by four makes every quantity
//! an integer and the thresholds become the frozen 10 / 3 / 3 multiplied by
//! four: 40, 12, 12. No float enters the decision, which is what makes the
//! comparison at the boundary exact and the test suite non-flaky. The
//! user-facing number is `eff_count_milli / 4`, printed as a decimal that
//! terminates after two places.
//!
//! ## The choice this spec left open: how a *day* is weighted
//!
//! fable-a's spec says each active day contributes "that day's weight" and
//! leaves two readings. This implementation takes **the weight of the latest
//! exchange on that day**. Reasons:
//!
//! * It is identical to "the largest weight of any exchange that day", because
//!   weight is non-increasing in age — so the two obvious readings of "the
//!   day's weight" coincide, and the CANDIDATE_SPEC phrasing 「每个自然日取该日
//!   内最大 w」 is satisfied verbatim.
//! * It is a fact about evidence that exists (a real exchange), not about a
//!   midnight that may be a full day away from anything that happened.
//! * It never weights a day higher than the newest thing in it.
//!
//! The alternative — bucketing the day's date against `as_of` — differs only
//! when a single UTC day straddles a bucket edge, and only ever downwards.
//! `day_weight_comes_from_the_latest_exchange_of_that_day` pins the difference
//! rather than leaving it to be discovered.

use crate::constants::{
    bucket_of, bucket_weight_milli, BUCKET_LOWER_EDGES_DAYS, DORMANT_NOTICE_DAYS,
    MILLI_PER_INTERACTION, MODERATE_MIN_INTERACTIONS, STRONG_MIN_ACTIVE_DAYS,
    STRONG_MIN_INTERACTIONS,
};
use crate::gate::{self, Observed, Thresholds};
use crate::types::{
    age_days, zh_common_weak_reason, zh_counts, zh_quarters, zh_venue, Band, Decayed, Detail,
    Interaction, Tally, TieAlgorithm, TieScore,
};

/// T3's bars, restated in quarter-units. Derived, never retyped: if the
/// shipped constants move, these move with them.
pub const THRESHOLDS: Thresholds = Thresholds {
    strong_min_count: STRONG_MIN_INTERACTIONS * MILLI_PER_INTERACTION,
    strong_min_days: STRONG_MIN_ACTIVE_DAYS * MILLI_PER_INTERACTION,
    moderate_min_count: MODERATE_MIN_INTERACTIONS * MILLI_PER_INTERACTION,
};

pub struct T3R;

impl T3R {
    /// The decayed measure of a tally, in quarter-units.
    ///
    /// One pass over the peer's timestamps and one over its active days, so
    /// this stays O(k) in the rows about this peer.
    pub fn measure(tally: &Tally, as_of_unix: i64) -> Decayed {
        let mut decayed = Decayed {
            eff_count_milli: 0,
            eff_days_milli: 0,
            bucket_interactions: [0; 4],
            bucket_days: [0; 4],
        };

        for &at in &tally.timestamps {
            let age = age_days(at, as_of_unix);
            decayed.bucket_interactions[bucket_of(age)] += 1;
            decayed.eff_count_milli += bucket_weight_milli(age);
        }

        // A day is worth what its newest exchange is worth. See the module
        // docs for why this reading was chosen over the day's own date.
        for &latest in tally.days.values() {
            let age = age_days(latest, as_of_unix);
            decayed.bucket_days[bucket_of(age)] += 1;
            decayed.eff_days_milli += bucket_weight_milli(age);
        }

        decayed
    }

    /// The rule itself, over an already-built tally.
    pub fn band_of(tally: &Tally, as_of_unix: i64) -> (Band, Decayed) {
        let decayed = T3R::measure(tally, as_of_unix);
        let observed = Observed {
            reciprocal: tally.is_reciprocal(),
            any_direct: tally.any_direct,
            group_only: tally.is_group_only(),
            count: decayed.eff_count_milli,
            days: decayed.eff_days_milli,
        };
        (gate::band(observed, THRESHOLDS), decayed)
    }

    /// The bucket rule, said once, in the words the user is shown.
    pub fn zh_rule() -> String {
        format!(
            "怎么算的：只数次数和日子，不看聊天内容。距今不到 {} 天的往来按 1 次计，满 {} 天不满 {} 天的按半次计，满 {} 天不满 {} 天的按四分之一计，再早的不计。",
            BUCKET_LOWER_EDGES_DAYS[1],
            BUCKET_LOWER_EDGES_DAYS[1],
            BUCKET_LOWER_EDGES_DAYS[2],
            BUCKET_LOWER_EDGES_DAYS[2],
            BUCKET_LOWER_EDGES_DAYS[3],
        )
    }

    /// The working: how many exchanges fell in each bucket, and what they add
    /// up to. Every number here is countable off the evidence list.
    fn zh_working(score: &TieScore, decayed: &Decayed) -> String {
        format!(
            "你们的 {} 次往来里，{} 次按 1 次计、{} 次按半次计、{} 次按四分之一计、{} 次不计，折算下来相当于 {} 次；有往来的 {} 天里，{} 天按 1 天计、{} 天按半天计、{} 天按四分之一天计、{} 天不计，折算下来相当于 {} 天。",
            score.interaction_count,
            decayed.bucket_interactions[0],
            decayed.bucket_interactions[1],
            decayed.bucket_interactions[2],
            decayed.bucket_interactions[3],
            zh_quarters(decayed.eff_count_milli),
            score.active_day_count,
            decayed.bucket_days[0],
            decayed.bucket_days[1],
            decayed.bucket_days[2],
            decayed.bucket_days[3],
            zh_quarters(decayed.eff_days_milli),
        )
    }

    fn zh_reason(score: &TieScore, decayed: &Decayed) -> String {
        let venue = zh_venue(score.any_direct);
        let eff_count = zh_quarters(decayed.eff_count_milli);
        let eff_days = zh_quarters(decayed.eff_days_milli);
        match score.band {
            Band::Strong => format!(
                "{venue}，双方都发过消息，折算后的 {eff_count} 次不少于 {STRONG_MIN_INTERACTIONS} 次、{eff_days} 天不少于 {STRONG_MIN_ACTIVE_DAYS} 天，所以算强联系。"
            ),
            Band::Moderate if !score.any_direct => format!(
                "{venue}；双方虽然都发过消息，折算后是 {eff_count} 次、{eff_days} 天，但群里再热闹也只说明认识，不足以说明关系紧密，所以最多算中等联系。"
            ),
            Band::Moderate if decayed.eff_count_milli < THRESHOLDS.strong_min_count => format!(
                "{venue}，双方也都发过消息，但折算后只有 {eff_count} 次，不到 {STRONG_MIN_INTERACTIONS} 次，所以算中等联系。"
            ),
            Band::Moderate => format!(
                "{venue}，双方也都发过消息，折算后的次数够了，但折算后只有 {eff_days} 天，不到 {STRONG_MIN_ACTIVE_DAYS} 天，所以算中等联系。"
            ),
            Band::Weak => format!(
                "{venue}，双方虽然都发过消息，但折算后只剩 {eff_count} 次，不到 {MODERATE_MIN_INTERACTIONS} 次，所以算弱联系。"
            ),
        }
    }
}

impl TieAlgorithm for T3R {
    const ID: &'static str = "T3R";

    fn score(peer_id: u64, interactions: &[Interaction], as_of_unix: i64) -> TieScore {
        let tally = Tally::of(peer_id, interactions);
        let (band, decayed) = T3R::band_of(&tally, as_of_unix);
        tally.to_score(band, Self::ID, as_of_unix, Detail::Decayed(decayed))
    }

    fn explain_zh(score: &TieScore) -> String {
        if let Some(reason) = zh_common_weak_reason(score) {
            return reason;
        }
        let Detail::Decayed(decayed) = score.detail else {
            // Only reachable if a caller hand-built a score with the wrong
            // working. Fall back to the counts rather than inventing numbers.
            return format!("{}这一档是按折算后的次数和天数定的。", zh_counts(score));
        };
        let mut text = format!(
            "{}{}{}{}",
            zh_counts(score),
            T3R::zh_rule(),
            T3R::zh_working(score, &decayed),
            T3R::zh_reason(score, &decayed),
        );
        if score.silent_days >= DORMANT_NOTICE_DAYS {
            text.push_str(&format!(
                "另外提醒一句：你们最近 {} 天没有往来。",
                score.silent_days
            ));
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{direct, group, DAY};

    fn measure(log: &[Interaction], as_of: i64) -> Decayed {
        T3R::measure(&Tally::of(1, log), as_of)
    }

    #[test]
    fn thresholds_are_the_frozen_bars_times_four() {
        assert_eq!(THRESHOLDS.strong_min_count, 40);
        assert_eq!(THRESHOLDS.strong_min_days, 12);
        assert_eq!(THRESHOLDS.moderate_min_count, 12);
    }

    #[test]
    fn each_bucket_contributes_its_documented_weight() {
        let as_of = DAY * 1_000;
        let log = vec![
            direct(1, true, as_of, 1),                // age 0   -> 4
            direct(1, false, as_of - DAY * 89, 1),    // age 89  -> 4
            direct(1, true, as_of - DAY * 90, 1),     // age 90  -> 2
            direct(1, false, as_of - DAY * 179, 1),   // age 179 -> 2
            direct(1, true, as_of - DAY * 180, 1),    // age 180 -> 1
            direct(1, false, as_of - DAY * 359, 1),   // age 359 -> 1
            direct(1, true, as_of - DAY * 360, 1),    // age 360 -> 0
            direct(1, false, as_of - DAY * 3_600, 1), // ancient -> 0
        ];
        let decayed = measure(&log, as_of);
        assert_eq!(decayed.bucket_interactions, [2, 2, 2, 2]);
        assert_eq!(decayed.eff_count_milli, 4 + 4 + 2 + 2 + 1 + 1);
        // Eight distinct days, each holding one exchange.
        assert_eq!(decayed.bucket_days, [2, 2, 2, 2]);
        assert_eq!(decayed.eff_days_milli, decayed.eff_count_milli);
    }

    #[test]
    fn day_weight_comes_from_the_latest_exchange_of_that_day() {
        // One UTC day straddling the 90-day edge: the 00:30 message is 90 days
        // old, the 23:30 one is 89. The day is worth the newer of the two, so
        // 4 quarter-units and not 2 — and the interaction weights still differ
        // from each other, which is what makes the two readings distinguishable.
        let as_of = DAY * 1_000 + 12 * 3_600;
        let day_start = DAY * (1_000 - 90);
        let log = vec![
            direct(1, true, day_start + 1_800, 1),
            direct(1, false, day_start + 23 * 3_600 + 1_800, 1),
        ];
        let decayed = measure(&log, as_of);
        assert_eq!(decayed.bucket_interactions, [1, 1, 0, 0]);
        assert_eq!(decayed.eff_count_milli, 4 + 2);
        assert_eq!(decayed.bucket_days, [1, 0, 0, 0]);
        assert_eq!(decayed.eff_days_milli, 4);
    }

    #[test]
    fn everything_older_than_the_cutoff_is_worth_nothing() {
        let as_of = DAY * 2_000;
        let log: Vec<Interaction> = (0..40)
            .map(|i| direct(1, i % 2 == 0, as_of - DAY * (400 + i), 1))
            .collect();
        let score = T3R::score(1, &log, as_of);
        assert_eq!(score.band, Band::Weak);
        // The band went to nothing; the audit trail did not.
        assert_eq!(score.interaction_count, 40);
        assert_eq!(score.active_day_count, 40);
        let Detail::Decayed(decayed) = score.detail else {
            panic!("expected decayed detail");
        };
        assert_eq!(decayed.eff_count_milli, 0);
        assert_eq!(decayed.bucket_interactions[3], 40);
    }

    #[test]
    fn the_strong_corner_is_exact_in_quarter_units() {
        // Ten fresh exchanges over three fresh days: 40 and 12 exactly.
        let as_of = DAY * 500 + 20 * 3_600;
        let log: Vec<Interaction> = (0..10)
            .map(|i| {
                let day = 500 - (i as i64 % 3);
                direct(1, i % 2 == 0, DAY * day + 3_600 * (i as i64 + 1), 1)
            })
            .collect();
        let (band, decayed) = T3R::band_of(&Tally::of(1, &log), as_of);
        assert_eq!(decayed.eff_count_milli, 40);
        assert_eq!(decayed.eff_days_milli, 12);
        assert_eq!(band, Band::Strong);

        // One of the ten pushed past the half-life: two quarter-units instead
        // of four, and the gate is a hard bar rather than a rounding.
        let mut aged = log.clone();
        aged[0] = direct(1, true, DAY * (500 - 95) + 20 * 3_600, 1);
        let (band, decayed) = T3R::band_of(&Tally::of(1, &aged), as_of);
        assert_eq!(decayed.eff_count_milli, 38);
        assert_eq!(band, Band::Moderate);
    }

    #[test]
    fn a_group_only_tie_cannot_be_strong_however_fresh() {
        let as_of = DAY * 500;
        let log: Vec<Interaction> = (0..60)
            .map(|i| group(1, i % 2 == 0, as_of - DAY * i, 7))
            .collect();
        let score = T3R::score(1, &log, as_of);
        assert_eq!(score.band, Band::Moderate);
    }

    #[test]
    fn evidence_from_the_future_is_not_worth_more_than_today() {
        // Imported logs sometimes carry clocks that run ahead. Ages clamp at
        // zero, so the newest bucket is the most anything can be worth.
        let log = vec![direct(1, true, DAY * 10, 1), direct(1, false, DAY * 10, 1)];
        let decayed = measure(&log, 0);
        assert_eq!(decayed.eff_count_milli, 8);
    }

    #[test]
    fn the_measure_does_not_depend_on_input_order() {
        let as_of = DAY * 700;
        let log: Vec<Interaction> = (0..30)
            .map(|i| direct(1, i % 2 == 0, as_of - DAY * (i as i64 * 13 % 400), 1))
            .collect();
        let shuffled: Vec<Interaction> = (0..log.len())
            .map(|i| log[(i * 7 + 3) % log.len()].clone())
            .collect();
        assert_eq!(measure(&log, as_of), measure(&shuffled, as_of));
    }
}
