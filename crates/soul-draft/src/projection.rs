//! The demotion clock, read forwards: which day this band changes if nothing
//! else happens.
//!
//! AD-9 settles what "simple prediction" means in v0.1, and AD-13 settles how
//! it lands: not a learned model and not a new work package, but the clock the
//! tie rule already runs, said out loud one step ahead. The tie rule demotes a
//! band once the silence reaches [`DEMOTE_ONE_BAND_DAYS`] and puts a floor
//! under it at [`FORCE_WEAK_DAYS`]; both thresholds are closed intervals, so
//! the day they are reached is a date this module can name from the last
//! exchange alone.
//!
//! Three properties make that a projection rather than a second opinion:
//!
//! * **No threshold of its own.** The two constants are imported from
//!   `soul_algo_tie::constants`, which is where the rule reads them. D52 bans a
//!   third copy and `tests/day_constants_agree.rs` enforces the ban on this
//!   crate's sources.
//! * **No clock.** `as_of` arrives on [`ClockReading`], as it does everywhere
//!   else in this crate: the same edge projects the same dates in Shanghai, in
//!   CI, and tomorrow.
//! * **No band decided here.** The band is read off the edge and the sentence
//!   says what the *rule* will do to it. Nothing in this module can move a
//!   band, and a band the user set is not projected at all.
//!
//! ## When there is nothing to say
//!
//! Five silences, each because the sentence would otherwise be false or
//! useless (`COPY_ZH.md` §6):
//!
//! * a Weak tie has no next band down;
//! * an edge the user has corrected is filed under their verdict, and the
//!   machine's clock no longer decides it — the same argument GC-9a makes for
//!   dropping the filing sentence;
//! * no citable evidence means no point can be built at all (AC-16);
//! * silence already past the floor means everything the clock had to do is
//!   done;
//! * an instant that cannot be read is not a date to forecast from, and the
//!   caller passes `None` rather than a guess.
//!
//! ## The group case
//!
//! The demotion clock reads the newest exchange in **any** venue, while the
//! band is decided on the one-to-one rows alone (T4D). So a tie can sit still
//! at its band with no private word exchanged for half a year, held there by a
//! group both people are in. That is a deliberate pricing decision rather than
//! a defect, and it is the one thing about the clock a user cannot work out
//! from the counts on the screen — so it gets a sentence of its own.

use soul_algo_tie::constants::{DEMOTE_ONE_BAND_DAYS, FORCE_WEAK_DAYS};
use soul_algo_tie::{zh_date, SECONDS_PER_DAY};
use soul_schema::common::SupportedBand;
use uuid::Uuid;

/// The closer every projected sentence carries (`COPY_ZH.md` §0.6).
pub const WORKING_HYPOTHESIS_CLOSER: &str = "这是工作假设，你可以直接改。";

pub const STRONG_TO_MODERATE_KEY: &str = "personnel.projection.strong_to_moderate";
pub const MODERATE_TO_WEAK_KEY: &str = "personnel.projection.moderate_to_weak";
pub const FORCED_WEAK_KEY: &str = "personnel.projection.forced_weak";
pub const GROUP_MAINTAINED_KEY: &str = "personnel.projection.group_maintained";

/// Every key [`project`] can emit, in output order. The screening tests walk
/// this list, so a template added without a test is a failing build.
pub const PROJECTION_STATEMENT_KEYS: [&str; 4] = [
    STRONG_TO_MODERATE_KEY,
    MODERATE_TO_WEAK_KEY,
    FORCED_WEAK_KEY,
    GROUP_MAINTAINED_KEY,
];

/// What the clock needs, and nothing else.
///
/// Instants are whole Unix seconds because that is what the silence is counted
/// in upstream; `None` is how a caller says an instant was absent or
/// unreadable, which is a suppression rather than a guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockReading {
    /// The band in force on the edge, whoever set it.
    pub band: SupportedBand,
    /// The one store-wide instant the rebuild scored against. Never a wall
    /// clock.
    pub as_of_unix: i64,
    /// The newest exchange in any venue: what the demotion clock reads.
    pub last_contact_unix: i64,
    /// The newest one-to-one exchange, when there has been one. What the band
    /// is decided on.
    pub last_direct_contact_unix: Option<i64>,
    /// Whether the user has overruled the band here.
    pub locked_by_user: bool,
}

impl ClockReading {
    /// Whole days of silence in any venue, counted from `as_of`.
    ///
    /// Clamped at zero for the same reason the renderer clamps its own gap: an
    /// exchange dated after `as_of` is a disagreement upstream, not contact in
    /// the future.
    pub fn silent_days(&self) -> i64 {
        whole_days(self.as_of_unix - self.last_contact_unix)
    }

    /// Whole days since the newest one-to-one exchange, when there is one.
    pub fn direct_silent_days(&self) -> Option<i64> {
        self.last_direct_contact_unix
            .map(|last| whole_days(self.as_of_unix - last))
    }
}

/// One projected sentence and the rows it rests on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedBullet {
    /// One of [`PROJECTION_STATEMENT_KEYS`].
    pub statement_key: &'static str,
    /// The sentence, ending in [`WORKING_HYPOTHESIS_CLOSER`].
    pub text_zh: String,
    /// Never empty: [`project`] returns nothing when there is nothing to cite.
    pub evidence_ids: Vec<Uuid>,
}

/// Project one edge's clock.
///
/// `evidence` is what the sentences may cite — the row holding the last
/// exchange when the caller could identify one, since that date is the whole
/// of what the projection rests on.
///
/// Which sentences come back:
///
/// | Band on the edge | Silence | Sentences |
/// |---|---|---|
/// | Strong | under the demotion day | one band down, then the floor |
/// | Moderate | under the demotion day | one band down (which is the floor) |
/// | Strong or Moderate | at or past the demotion day | the floor |
/// | anything | at or past the floor day | none |
/// | Weak | any | none |
///
/// The group sentence is orthogonal to the table and joins whatever it finds
/// there.
pub fn project(reading: &ClockReading, evidence: &[Uuid]) -> Vec<ProjectedBullet> {
    if evidence.is_empty()
        || reading.locked_by_user
        || reading.band == SupportedBand::Weak
        || reading.silent_days() >= FORCE_WEAK_DAYS
    {
        return Vec::new();
    }

    let demotion_day = zh_date(reading.last_contact_unix + DEMOTE_ONE_BAND_DAYS * SECONDS_PER_DAY);
    let floor_day = zh_date(reading.last_contact_unix + FORCE_WEAK_DAYS * SECONDS_PER_DAY);
    let before_the_demotion_day = reading.silent_days() < DEMOTE_ONE_BAND_DAYS;

    let mut texts: Vec<(&'static str, String)> = Vec::new();
    match (reading.band, before_the_demotion_day) {
        (SupportedBand::Strong, true) => {
            texts.push((STRONG_TO_MODERATE_KEY, strong_to_moderate(&demotion_day)));
            texts.push((FORCED_WEAK_KEY, forced_weak(&floor_day)));
        }
        (SupportedBand::Moderate, true) => {
            texts.push((MODERATE_TO_WEAK_KEY, moderate_to_weak(&demotion_day)));
        }
        // Past the demotion day the step this band would have taken has been
        // taken, so the only thing left on the clock is the floor. A Weak band
        // never reaches here; it returned above.
        _ => texts.push((FORCED_WEAK_KEY, forced_weak(&floor_day))),
    }

    if let Some(days) = group_maintained_days(reading) {
        texts.push((GROUP_MAINTAINED_KEY, group_maintained(days)));
    }

    texts
        .into_iter()
        .map(|(statement_key, text_zh)| ProjectedBullet {
            statement_key,
            text_zh,
            evidence_ids: evidence.to_vec(),
        })
        .collect()
}

// ------------------------------------------------------------- internals ---

/// The templates, one function each and the band words written out.
///
/// No shared template with the band as a parameter: `COPY_ZH.md` §6.4 asks for
/// the words in the copy rather than computed beside it, so that reading the
/// frozen file tells you what a user will see without following a lookup.
fn strong_to_moderate(demotion_day: &str) -> String {
    format!(
        "如果你们一直没有新的往来，到 {demotion_day} 就满 {DEMOTE_ONE_BAND_DAYS} 天没有联系，\
         从那天起这一档会从「强」降到「中等」。在那之前只要再聊起来，这一档就不会降。\
         {WORKING_HYPOTHESIS_CLOSER}"
    )
}

fn moderate_to_weak(demotion_day: &str) -> String {
    format!(
        "如果你们一直没有新的往来，到 {demotion_day} 就满 {DEMOTE_ONE_BAND_DAYS} 天没有联系，\
         从那天起这一档会从「中等」降到「弱」。在那之前只要再聊起来，这一档就不会降。\
         {WORKING_HYPOTHESIS_CLOSER}"
    )
}

fn forced_weak(floor_day: &str) -> String {
    format!(
        "如果你们一直没有新的往来，到 {floor_day} 就满 {FORCE_WEAK_DAYS} 天没有联系，\
         从那天起这一档会算「弱」。过去的往来仍会保留展示，重新来往就会回升。\
         {WORKING_HYPOTHESIS_CLOSER}"
    )
}

fn group_maintained(direct_silent_days: i64) -> String {
    format!(
        "你们一对一已经 {direct_silent_days} 天没有单独说过话；降档看的是任一场地的最近一次往来，\
         所以这一档现在是靠群里的往来维持的。等群里也安静下来，这一档才会跟着往下降。\
         {WORKING_HYPOTHESIS_CLOSER}"
    )
}

/// The one-to-one silence, when it is long enough to have cost a band and the
/// group traffic is what stopped it doing so.
fn group_maintained_days(reading: &ClockReading) -> Option<i64> {
    let direct = reading.direct_silent_days()?;
    (direct >= DEMOTE_ONE_BAND_DAYS && reading.silent_days() < DEMOTE_ONE_BAND_DAYS)
        .then_some(direct)
}

fn whole_days(seconds: i64) -> i64 {
    seconds.max(0) / SECONDS_PER_DAY
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-08-24T00:00:00Z, the instant the frozen fixtures score against.
    const AS_OF: i64 = 1_787_529_600;

    fn reading(band: SupportedBand, silent_days: i64) -> ClockReading {
        ClockReading {
            band,
            as_of_unix: AS_OF,
            last_contact_unix: AS_OF - silent_days * SECONDS_PER_DAY,
            last_direct_contact_unix: Some(AS_OF - silent_days * SECONDS_PER_DAY),
            locked_by_user: false,
        }
    }

    fn keys(bullets: &[ProjectedBullet]) -> Vec<&str> {
        bullets.iter().map(|b| b.statement_key).collect()
    }

    fn rows() -> Vec<Uuid> {
        vec![Uuid::from_u128(1)]
    }

    #[test]
    fn every_template_ends_in_the_closer() {
        let cases = [
            strong_to_moderate("2027 年 1 月 1 日"),
            moderate_to_weak("2027 年 1 月 1 日"),
            forced_weak("2027 年 1 月 1 日"),
            group_maintained(200),
        ];
        for text in cases {
            assert!(text.ends_with(WORKING_HYPOTHESIS_CLOSER), "{text}");
        }
    }

    #[test]
    fn the_projected_day_is_the_day_the_count_reaches_the_threshold() {
        let clock = reading(SupportedBand::Strong, 0);
        let bullets = project(&clock, &rows());
        // The last exchange is on 2026-08-24, so the demotion lands
        // DEMOTE_ONE_BAND_DAYS later and the floor WEAK_AFTER_SILENT_DAYS after that.
        assert!(
            bullets[0].text_zh.contains("2027 年 2 月 20 日"),
            "{:?}",
            bullets[0]
        );
        assert!(
            bullets[1].text_zh.contains("2027 年 8 月 19 日"),
            "{:?}",
            bullets[1]
        );
    }

    #[test]
    fn a_weak_tie_has_nothing_below_it_to_project() {
        assert!(project(&reading(SupportedBand::Weak, 3), &rows()).is_empty());
    }

    #[test]
    fn a_band_the_user_set_is_not_on_the_machine_s_clock() {
        let mut clock = reading(SupportedBand::Strong, 3);
        clock.locked_by_user = true;
        assert!(project(&clock, &rows()).is_empty());
    }

    #[test]
    fn nothing_to_cite_is_nothing_to_say() {
        assert!(project(&reading(SupportedBand::Strong, 3), &[]).is_empty());
    }

    #[test]
    fn a_strong_tie_is_told_about_both_steps() {
        assert_eq!(
            keys(&project(&reading(SupportedBand::Strong, 3), &rows())),
            vec![STRONG_TO_MODERATE_KEY, FORCED_WEAK_KEY],
        );
    }

    #[test]
    fn a_moderate_tie_that_has_already_come_down_is_told_only_about_the_floor() {
        let clock = reading(SupportedBand::Moderate, DEMOTE_ONE_BAND_DAYS);
        assert_eq!(keys(&project(&clock, &rows())), vec![FORCED_WEAK_KEY]);
    }

    #[test]
    fn the_group_sentence_needs_a_quiet_private_channel_and_a_live_group_one() {
        let mut clock = reading(SupportedBand::Moderate, 2);
        assert_eq!(keys(&project(&clock, &rows())), vec![MODERATE_TO_WEAK_KEY]);

        clock.last_direct_contact_unix = Some(AS_OF - DEMOTE_ONE_BAND_DAYS * SECONDS_PER_DAY);
        assert_eq!(
            keys(&project(&clock, &rows())),
            vec![MODERATE_TO_WEAK_KEY, GROUP_MAINTAINED_KEY],
        );

        // A tie with no one-to-one row at all says nothing about one.
        clock.last_direct_contact_unix = None;
        assert_eq!(keys(&project(&clock, &rows())), vec![MODERATE_TO_WEAK_KEY]);
    }

    #[test]
    fn contact_dated_after_as_of_reads_as_today_rather_than_as_future_silence() {
        let mut clock = reading(SupportedBand::Strong, 0);
        clock.last_contact_unix = AS_OF + SECONDS_PER_DAY;
        assert_eq!(clock.silent_days(), 0);
    }
}
