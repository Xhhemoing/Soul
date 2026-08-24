//! Round 3 / fable-a — independent T4 vs T4D verification for ALGO_FROZEN.
//!
//! This crate exists so the docs-freeze slot does not freeze on hearsay: the
//! freeze condition in R2-SYNTHESIS ("T4D adopted iff `lilei_12` stays Strong
//! and `group_heavy_plus_one_direct_each_way` is not Strong") is decided here
//! by running both rules over pinned fixtures, independently of the other
//! Round 3 slots.
//!
//! ## Frozen definitions (docs/algorithms/DECISION.md is the prose form)
//!
//! ```text
//! Shared: as_of is ONE caller-supplied value per rebuild (store-level).
//!         silent_days = max(0, as_of - last_contact_in_ANY_venue) / 86400.
//!         Recency step (both rules): >= 360 -> Weak; >= 180 -> one band
//!         lower; else unchanged. Empty observation -> Weak, no time math.
//!
//! T4  base: reciprocal(all) ∧ any_direct ∧ count(all) >= 10 ∧ days(all) >= 3 -> Strong
//!           reciprocal(all) ∧ count(all) >= 3                                -> Moderate
//!           otherwise                                                        -> Weak
//!
//! T4D base: reciprocal(direct) ∧ count(direct) >= 10 ∧ days(direct) >= 3    -> Strong
//!           reciprocal(direct) ∧ count(direct) >= 3                          -> Moderate
//!           otherwise                                                        -> Weak
//! ```
//!
//! T4D moves the reciprocity, count and day gates onto one-to-one rows only
//! (R2-SYNTHESIS 边界风险 2); the dormancy clock deliberately stays on the
//! newest exchange in any venue, same as T4, so the A2 dormant-notice sentence
//! and the band can never contradict each other on one screen.
//!
//! No dependencies, no wall clock, no floats, no unsafe.

#![forbid(unsafe_code)]

pub const SECONDS_PER_DAY: i64 = 86_400;

pub const MODERATE_MIN_INTERACTIONS: u64 = 3;
pub const STRONG_MIN_INTERACTIONS: u64 = 10;
pub const STRONG_MIN_ACTIVE_DAYS: u64 = 3;
pub const DEMOTE_AFTER_SILENT_DAYS: i64 = 180;
pub const WEAK_AFTER_SILENT_DAYS: i64 = 360;

/// 2026-08-24T14:00:00Z — the round's pinned `as_of` (same value as Round 2
/// opus-a `AS_OF_2026_08_24`, derived not looked up).
pub const AS_OF_2026_08_24: i64 = 1_787_580_000;
/// 2026-08-24T00:00:00Z.
pub const TODAY_MIDNIGHT: i64 = 1_787_529_600;
/// 2019-06-01T00:00:00Z, for the dormant fixture.
pub const T_2019: i64 = 1_559_347_200;

/// `hour` o'clock UTC, `days_ago` whole days before the anchor's midnight.
/// For `hour <= 14` the instant is exactly `days_ago` whole days old at
/// [`AS_OF_2026_08_24`].
pub const fn at(days_ago: i64, hour: i64) -> i64 {
    TODAY_MIDNIGHT - days_ago * SECONDS_PER_DAY + hour * 3_600
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Band {
    Weak,
    Moderate,
    Strong,
}

impl Band {
    pub const fn rank(self) -> u8 {
        match self {
            Band::Weak => 0,
            Band::Moderate => 1,
            Band::Strong => 2,
        }
    }

    pub const fn demoted_once(self) -> Band {
        match self {
            Band::Strong => Band::Moderate,
            Band::Moderate | Band::Weak => Band::Weak,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Band::Weak => "Weak",
            Band::Moderate => "Moderate",
            Band::Strong => "Strong",
        }
    }
}

/// One observed exchange. Metadata only: no content field exists to read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Interaction {
    pub outgoing: bool,
    pub occurred_at_unix: i64,
    /// False means other people were in the room.
    pub venue_direct: bool,
}

pub const fn direct(outgoing: bool, occurred_at_unix: i64) -> Interaction {
    Interaction {
        outgoing,
        occurred_at_unix,
        venue_direct: true,
    }
}

pub const fn group(outgoing: bool, occurred_at_unix: i64) -> Interaction {
    Interaction {
        outgoing,
        occurred_at_unix,
        venue_direct: false,
    }
}

/// Raw tallies over one venue selection, plus the all-venue last contact.
#[derive(Clone, Debug, Default)]
struct Tally {
    outgoing: u64,
    incoming: u64,
    days: std::collections::BTreeSet<i64>,
}

impl Tally {
    fn absorb(&mut self, row: &Interaction) {
        match row.outgoing {
            true => self.outgoing += 1,
            false => self.incoming += 1,
        }
        self.days
            .insert(row.occurred_at_unix.div_euclid(SECONDS_PER_DAY));
    }

    fn count(&self) -> u64 {
        self.outgoing + self.incoming
    }

    fn day_count(&self) -> u64 {
        self.days.len() as u64
    }

    fn reciprocal(&self) -> bool {
        self.outgoing > 0 && self.incoming > 0
    }
}

/// Everything the verdicts and the report table need.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Score {
    pub band: Band,
    pub band_before_recency: Band,
    pub interaction_count: u64,
    pub active_day_count: u64,
    pub direct_count: u64,
    pub direct_day_count: u64,
    pub group_count: u64,
    pub silent_days: i64,
}

fn observe(log: &[Interaction]) -> (Tally, Tally, Option<i64>) {
    let mut all = Tally::default();
    let mut only_direct = Tally::default();
    let mut last: Option<i64> = None;
    for row in log {
        all.absorb(row);
        if row.venue_direct {
            only_direct.absorb(row);
        }
        last = Some(match last {
            Some(seen) if seen >= row.occurred_at_unix => seen,
            _ => row.occurred_at_unix,
        });
    }
    (all, only_direct, last)
}

const fn silent_days(last_contact: i64, as_of: i64) -> i64 {
    let delta = as_of - last_contact;
    if delta <= 0 {
        0
    } else {
        delta / SECONDS_PER_DAY
    }
}

const fn apply_recency(base: Band, silent: i64) -> Band {
    if silent >= WEAK_AFTER_SILENT_DAYS {
        Band::Weak
    } else if silent >= DEMOTE_AFTER_SILENT_DAYS {
        base.demoted_once()
    } else {
        base
    }
}

fn finish(base: Band, all: Tally, only_direct: Tally, last: Option<i64>, as_of: i64) -> Score {
    let silent = match last {
        Some(last) => silent_days(last, as_of),
        None => 0,
    };
    let band = match last {
        Some(_) => apply_recency(base, silent),
        None => base,
    };
    Score {
        band,
        band_before_recency: base,
        interaction_count: all.count(),
        active_day_count: all.day_count(),
        direct_count: only_direct.count(),
        direct_day_count: only_direct.day_count(),
        group_count: all.count() - only_direct.count(),
        silent_days: silent,
    }
}

/// T4 — Round 2 winner: all-venue gates, `any_direct` latch, any-venue clock.
pub fn score_t4(log: &[Interaction], as_of: i64) -> Score {
    let (all, only_direct, last) = observe(log);
    let base = if all.reciprocal()
        && only_direct.count() > 0
        && all.count() >= STRONG_MIN_INTERACTIONS
        && all.day_count() >= STRONG_MIN_ACTIVE_DAYS
    {
        Band::Strong
    } else if all.reciprocal() && all.count() >= MODERATE_MIN_INTERACTIONS {
        Band::Moderate
    } else {
        Band::Weak
    };
    finish(base, all, only_direct, last, as_of)
}

/// T4D — the same rule with reciprocity, count and day gates on one-to-one
/// rows only. The dormancy clock stays on the newest exchange in any venue.
pub fn score_t4d(log: &[Interaction], as_of: i64) -> Score {
    let (all, only_direct, last) = observe(log);
    let base = if only_direct.reciprocal()
        && only_direct.count() >= STRONG_MIN_INTERACTIONS
        && only_direct.day_count() >= STRONG_MIN_ACTIVE_DAYS
    {
        Band::Strong
    } else if only_direct.reciprocal() && only_direct.count() >= MODERATE_MIN_INTERACTIONS {
        Band::Moderate
    } else {
        Band::Weak
    };
    finish(base, all, only_direct, last, as_of)
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

pub struct Fixture {
    pub name: &'static str,
    pub log: Vec<Interaction>,
    pub as_of: i64,
}

fn fixture(name: &'static str, log: Vec<Interaction>) -> Fixture {
    Fixture {
        name,
        log,
        as_of: AS_OF_2026_08_24,
    }
}

/// `per_day` exchanges a day, alternating direction, on `days` consecutive
/// days ending `newest_days_ago` days before the anchor. Same shape as Round 2
/// opus-a `testing::run`.
fn run(newest_days_ago: i64, days: i64, per_day: i64, venue_direct: bool) -> Vec<Interaction> {
    let mut log = Vec::new();
    for day in 0..days {
        let days_ago = newest_days_ago + (days - 1 - day);
        for slot in 0..per_day {
            log.push(Interaction {
                outgoing: (day * per_day + slot) % 2 == 0,
                occurred_at_unix: at(days_ago, 9 + 2 * slot),
                venue_direct,
            });
        }
    }
    log
}

pub fn empty() -> Fixture {
    fixture("empty", Vec::new())
}

pub fn single_inbound() -> Fixture {
    fixture("single_inbound", vec![direct(false, at(1, 9))])
}

/// The Goal 1 anchor: 12 reciprocal one-to-one messages over 6 days, ending
/// three days before `as_of`. Must stay Strong under any adopted rule.
pub fn lilei_12() -> Fixture {
    fixture("lilei_12", run(3, 6, 2, true))
}

pub fn afternoon_20() -> Fixture {
    let log = (0..20i64)
        .map(|i| direct(i % 2 == 0, at(1, 13) + i * 120))
        .collect();
    fixture("afternoon_20", log)
}

pub fn group_only_50() -> Fixture {
    let log = (0..50i64)
        .map(|i| group(i % 2 == 0, at(50 - i, 10)))
        .collect();
    fixture("group_only_50", log)
}

pub fn one_sided_100() -> Fixture {
    let log = (0..100i64).map(|i| direct(true, at(100 - i, 8))).collect();
    fixture("one_sided_100", log)
}

pub fn steady_16_over_8_weeks() -> Fixture {
    let mut log = Vec::new();
    for week in 0..8i64 {
        let days_ago = 56 - week * 7;
        log.push(direct(true, at(days_ago, 9)));
        log.push(direct(false, at(days_ago, 13)));
    }
    fixture("steady_16_over_8_weeks", log)
}

/// Twenty reciprocal one-to-one messages over ten days, all in 2019 (F04).
pub fn dormant_2019() -> Fixture {
    let log = (0..20i64)
        .map(|i| {
            direct(
                i % 2 == 0,
                T_2019 + SECONDS_PER_DAY * (i / 2) + (i % 2) * 3_600,
            )
        })
        .collect();
    fixture("dormant_2019", log)
}

/// F04b — 44 reciprocal one-to-one messages, every age in [190, 211] days.
pub fn f04b_semi_dormant_44() -> Fixture {
    let mut log = Vec::new();
    for i in 0..22i64 {
        log.push(direct(true, at(190 + i, 9)));
        log.push(direct(false, at(190 + i, 13)));
    }
    fixture("f04b_semi_dormant_44", log)
}

/// F04c — the 2019 history plus one greeting each way yesterday. The known,
/// documented T4/T4D limitation; the T3R rollback trigger.
pub fn f04c_revived_one_ping() -> Fixture {
    let mut log = dormant_2019().log;
    log.push(direct(true, at(1, 9)));
    log.push(direct(false, at(1, 13)));
    fixture("f04c_revived_one_ping", log)
}

/// The Round 3 decisive fixture (fable-b PIPELINE_DEBT §1.3): 100 group
/// messages fanned out over 50 days, plus exactly one one-to-one greeting in
/// each direction. Pinned expectation: NOT Strong.
pub fn group_heavy_plus_one_direct_each_way() -> Fixture {
    let mut log = run(1, 50, 2, false);
    log.push(direct(true, at(2, 10)));
    log.push(direct(false, at(1, 10)));
    fixture("group_heavy_plus_one_direct_each_way", log)
}

/// Strong one-to-one history by count, newest exchange `n` days ago.
pub fn quiet_for(n: i64) -> Fixture {
    Fixture {
        name: "quiet_for",
        log: run(n, 6, 2, true),
        as_of: AS_OF_2026_08_24,
    }
}

/// One-to-one history that went quiet 200 days ago, but the group thread is
/// alive: pins the any-venue dormancy clock.
pub fn direct_quiet_200_group_yesterday() -> Fixture {
    let mut log = run(200, 6, 2, true);
    log.push(group(true, at(1, 9)));
    log.push(group(false, at(1, 10)));
    fixture("direct_quiet_200_group_yesterday", log)
}

/// The same one-to-one history with no group thread: the clock has nothing
/// fresher than 200 days, so both rules demote.
pub fn direct_quiet_200_alone() -> Fixture {
    fixture("direct_quiet_200_alone", run(200, 6, 2, true))
}

/// The self-heal path out of the heavy-group Weak: three greetings each way
/// over three days clears the Moderate bar on one-to-one counts.
pub fn group_heavy_plus_three_direct_each_way() -> Fixture {
    let mut log = run(1, 50, 2, false);
    for day in 1..=3i64 {
        log.push(direct(true, at(day, 9)));
        log.push(direct(false, at(day, 13)));
    }
    fixture("group_heavy_plus_three_direct_each_way", log)
}

/// Every named fixture, in report-table order.
pub fn all() -> Vec<Fixture> {
    vec![
        empty(),
        single_inbound(),
        lilei_12(),
        afternoon_20(),
        group_only_50(),
        one_sided_100(),
        steady_16_over_8_weeks(),
        dormant_2019(),
        f04b_semi_dormant_44(),
        f04c_revived_one_ping(),
        group_heavy_plus_one_direct_each_way(),
        group_heavy_plus_three_direct_each_way(),
        direct_quiet_200_group_yesterday(),
        direct_quiet_200_alone(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The freeze condition, verbatim from the Round 3 brief: T4D is adopted
    /// iff `lilei_12` stays Strong AND `group_heavy_plus_one_direct_each_way`
    /// is not Strong. Both halves, plus the proof that T4 has the hole.
    #[test]
    fn freeze_condition_is_satisfied() {
        let lilei = lilei_12();
        assert_eq!(score_t4d(&lilei.log, lilei.as_of).band, Band::Strong);
        assert_eq!(score_t4(&lilei.log, lilei.as_of).band, Band::Strong);

        let heavy = group_heavy_plus_one_direct_each_way();
        // The hole T4D exists to close: two greetings unlock 100 group rows.
        assert_eq!(score_t4(&heavy.log, heavy.as_of).band, Band::Strong);
        assert_ne!(score_t4d(&heavy.log, heavy.as_of).band, Band::Strong);
        assert_eq!(score_t4d(&heavy.log, heavy.as_of).band, Band::Weak);
        assert_eq!(score_t4d(&heavy.log, heavy.as_of).direct_count, 2);
    }

    /// The full expected matrix. Any diff against this table is a spec change
    /// and must go through the parent's DECISIONS file.
    #[test]
    fn the_matrix_is_pinned() {
        let expected: &[(&str, Band, Band)] = &[
            ("empty", Band::Weak, Band::Weak),
            ("single_inbound", Band::Weak, Band::Weak),
            ("lilei_12", Band::Strong, Band::Strong),
            ("afternoon_20", Band::Moderate, Band::Moderate),
            ("group_only_50", Band::Moderate, Band::Weak),
            ("one_sided_100", Band::Weak, Band::Weak),
            ("steady_16_over_8_weeks", Band::Strong, Band::Strong),
            ("dormant_2019", Band::Weak, Band::Weak),
            ("f04b_semi_dormant_44", Band::Moderate, Band::Moderate),
            ("f04c_revived_one_ping", Band::Strong, Band::Strong),
            (
                "group_heavy_plus_one_direct_each_way",
                Band::Strong,
                Band::Weak,
            ),
            (
                "group_heavy_plus_three_direct_each_way",
                Band::Strong,
                Band::Moderate,
            ),
            (
                "direct_quiet_200_group_yesterday",
                Band::Strong,
                Band::Strong,
            ),
            ("direct_quiet_200_alone", Band::Moderate, Band::Moderate),
        ];
        let fixtures = all();
        assert_eq!(fixtures.len(), expected.len());
        for (f, (name, t4, t4d)) in fixtures.iter().zip(expected) {
            assert_eq!(f.name, *name);
            assert_eq!(score_t4(&f.log, f.as_of).band, *t4, "{name} / T4");
            assert_eq!(score_t4d(&f.log, f.as_of).band, *t4d, "{name} / T4D");
        }
    }

    #[test]
    fn demotion_boundaries_are_closed_whole_days_for_both_rules() {
        for (silent, expected) in [
            (0, Band::Strong),
            (179, Band::Strong),
            (180, Band::Moderate),
            (359, Band::Moderate),
            (360, Band::Weak),
            (5_000, Band::Weak),
        ] {
            let f = quiet_for(silent);
            let t4 = score_t4(&f.log, f.as_of);
            let t4d = score_t4d(&f.log, f.as_of);
            assert_eq!(t4.silent_days, silent);
            assert_eq!(t4.band, expected, "T4 at {silent}");
            assert_eq!(t4d.band, expected, "T4D at {silent}");
        }
    }

    #[test]
    fn one_sided_never_leaves_weak() {
        for n in [1i64, 3, 10, 100] {
            let log: Vec<Interaction> = (0..n).map(|i| direct(true, at(n - i, 9))).collect();
            assert_eq!(score_t4(&log, AS_OF_2026_08_24).band, Band::Weak);
            assert_eq!(score_t4d(&log, AS_OF_2026_08_24).band, Band::Weak);
        }
    }

    #[test]
    fn group_only_never_reaches_strong_and_t4d_declines_to_band_on_it() {
        for n in [3i64, 50, 1_000] {
            let log: Vec<Interaction> = (0..n)
                .map(|i| group(i % 2 == 0, at(1 + i % 60, 9)))
                .collect();
            let t4 = score_t4(&log, AS_OF_2026_08_24);
            let t4d = score_t4d(&log, AS_OF_2026_08_24);
            assert_ne!(t4.band, Band::Strong, "T4 group-only n={n}");
            assert!(t4.band.rank() <= Band::Moderate.rank());
            assert_eq!(t4d.band, Band::Weak, "T4D group-only n={n}");
        }
    }

    #[test]
    fn silence_never_promotes() {
        for f in all() {
            for rule in [score_t4 as fn(&[Interaction], i64) -> Score, score_t4d] {
                let s = rule(&f.log, f.as_of);
                assert!(
                    s.band.rank() <= s.band_before_recency.rank(),
                    "{} promoted by recency",
                    f.name
                );
            }
        }
    }

    /// T4D's structural invariants: Strong implies at least ten reciprocal
    /// one-to-one exchanges over at least three days, and contact within the
    /// last 180 days.
    #[test]
    fn t4d_strong_is_earned_on_direct_counts_and_freshness() {
        for f in all() {
            let s = score_t4d(&f.log, f.as_of);
            if s.band == Band::Strong {
                assert!(s.direct_count >= STRONG_MIN_INTERACTIONS, "{}", f.name);
                assert!(s.direct_day_count >= STRONG_MIN_ACTIVE_DAYS, "{}", f.name);
                assert!(s.silent_days < DEMOTE_AFTER_SILENT_DAYS, "{}", f.name);
            }
        }
    }

    #[test]
    fn permutation_of_the_log_changes_nothing() {
        for f in all() {
            let mut reversed = f.log.clone();
            reversed.reverse();
            assert_eq!(
                score_t4(&f.log, f.as_of),
                score_t4(&reversed, f.as_of),
                "{}",
                f.name
            );
            assert_eq!(
                score_t4d(&f.log, f.as_of),
                score_t4d(&reversed, f.as_of),
                "{}",
                f.name
            );
        }
    }

    #[test]
    fn shifting_evidence_and_as_of_together_changes_nothing() {
        let shift = 37 * SECONDS_PER_DAY;
        for f in all() {
            let shifted: Vec<Interaction> = f
                .log
                .iter()
                .map(|row| Interaction {
                    occurred_at_unix: row.occurred_at_unix + shift,
                    ..*row
                })
                .collect();
            assert_eq!(
                score_t4d(&f.log, f.as_of).band,
                score_t4d(&shifted, f.as_of + shift).band,
                "{}",
                f.name
            );
        }
    }

    /// The as_of trap from ABLATION_PROTOCOL §1.4: score the 2019 fixture
    /// against its own newest row and dormancy silently vanishes. The rebuild
    /// must use one store-level as_of.
    #[test]
    fn peer_local_as_of_wrongly_revives_the_dormant_tie() {
        let f = dormant_2019();
        let wrong_as_of = f.log.iter().map(|r| r.occurred_at_unix).max().unwrap();
        assert_eq!(score_t4d(&f.log, wrong_as_of).band, Band::Strong);
        assert_eq!(score_t4d(&f.log, f.as_of).band, Band::Weak);
        assert_eq!(score_t4d(&f.log, f.as_of).silent_days, 2632);
    }

    /// The pinned clock: a fresh group message keeps a direct-banded tie from
    /// demoting (deliberate asymmetry, priced in DECISION.md), while the same
    /// tie without the group thread demotes at 200 silent days.
    #[test]
    fn the_dormancy_clock_reads_any_venue_in_both_rules() {
        let alive = direct_quiet_200_group_yesterday();
        let s = score_t4d(&alive.log, alive.as_of);
        assert_eq!(s.silent_days, 1);
        assert_eq!(s.band, Band::Strong);

        let alone = direct_quiet_200_alone();
        let s = score_t4d(&alone.log, alone.as_of);
        assert_eq!(s.silent_days, 200);
        assert_eq!(s.band, Band::Moderate);
    }

    /// F04c stays a documented limitation under T4D — the rollback trigger,
    /// not a regression: T4 behaves identically.
    #[test]
    fn f04c_is_the_same_known_limitation_in_both_rules() {
        let f = f04c_revived_one_ping();
        assert_eq!(score_t4(&f.log, f.as_of).band, Band::Strong);
        assert_eq!(score_t4d(&f.log, f.as_of).band, Band::Strong);
    }

    #[test]
    fn no_rule_reads_a_clock_the_same_input_scores_the_same_twice() {
        for f in all() {
            assert_eq!(score_t4d(&f.log, f.as_of), score_t4d(&f.log, f.as_of));
            assert_eq!(score_t4(&f.log, f.as_of), score_t4(&f.log, f.as_of));
        }
    }
}
