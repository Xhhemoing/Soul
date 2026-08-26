//! What every surviving candidate reads and returns.
//!
//! One shape of input (an observed [`Interaction`]), one shape of output (a
//! [`TieScore`]), and one pass over the evidence ([`Tally`]) that all the
//! candidates share. Keeping the tally in one place means the candidates
//! differ only in the rule that turns observations into a band, which is the
//! only thing the ablation is trying to measure.
//!
//! ## `as_of`, not "now"
//!
//! Every entry point takes `as_of_unix` and nothing in this crate reads a
//! clock. `as_of` is a property of the evidence store — fable-a's frozen spec
//! says `as_of = max(occurred_at)` over the **whole store** — so two runs of
//! the same rebuild over the same rows produce byte-identical output whatever
//! day it is. Ages are whole UTC days, floored, clamped at zero:
//! `age_days = max(0, as_of - occurred_at) / 86400`.

use std::collections::{BTreeMap, BTreeSet};

/// Seconds in a UTC day. No leap seconds: Unix time does not have them, so a
/// day boundary is always a multiple of this number.
pub const SECONDS_PER_DAY: i64 = 86_400;

/// How much contact the evidence supports.
///
/// The same three words the rest of Soul uses for evidence bands. Not a score,
/// not a percentile: a band a user can be shown alongside the counts that
/// produced it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Band {
    Weak,
    Moderate,
    Strong,
}

impl Band {
    /// Weak < Moderate < Strong, for capping rules that need to say "no higher
    /// than this".
    ///
    /// Deliberately a method rather than a derived `Ord`: the ordering is a
    /// property of the capping rules, not a claim that bands are numbers.
    pub const fn rank(self) -> u8 {
        match self {
            Band::Weak => 0,
            Band::Moderate => 1,
            Band::Strong => 2,
        }
    }

    /// The lower of two bands.
    pub const fn capped_at(self, ceiling: Band) -> Band {
        if self.rank() <= ceiling.rank() {
            self
        } else {
            ceiling
        }
    }

    /// One rung down. Weak has nowhere to go.
    ///
    /// T4's whole mechanism: no float, no re-derivation, just a step down a
    /// three-valued ladder the user can see.
    pub const fn demoted_once(self) -> Band {
        match self {
            Band::Strong => Band::Moderate,
            Band::Moderate | Band::Weak => Band::Weak,
        }
    }

    /// The band as the storage layer spells it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Band::Weak => "weak",
            Band::Moderate => "moderate",
            Band::Strong => "strong",
        }
    }

    /// The band as the user reads it.
    pub const fn as_zh(self) -> &'static str {
        match self {
            Band::Weak => "弱联系",
            Band::Moderate => "中等联系",
            Band::Strong => "强联系",
        }
    }
}

/// One observed exchange with one person.
///
/// No body, no name, no window title. Everything a tie-strength rule is
/// allowed to see is here, and none of it is content — which is what lets the
/// whole family stay inside the third-party rule in PRODUCT_LOCK.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Interaction {
    pub peer_id: u64,
    /// True when the user wrote it.
    pub outgoing: bool,
    /// UTC seconds. Negative values (before 1970) are handled, see
    /// [`epoch_day`].
    pub occurred_at_unix: i64,
    /// False means the exchange happened with other people in the room.
    pub venue_direct: bool,
    pub conversation_id: u64,
}

/// The working the rule showed, beyond the raw counts every rule reports.
///
/// Round 1 opus-a §3.5 recorded the hole this closes: `TieScore` carried the
/// counts but not the quantity that actually decided the band, so
/// `explain_zh` had to hedge. The variants are small and contain no content,
/// no names and no free text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Detail {
    /// The band came from the raw counts (T0, T3).
    RawCounts,
    /// The band came from decayed counts (T3R).
    Decayed(Decayed),
    /// The band came from a raw-count band, then silence pushed it down (T4).
    Demoted {
        /// What the underlying rule said before recency was applied.
        band_before: Band,
    },
}

/// T3R's working, in quarter-interaction units.
///
/// `eff_count_milli / 4` is the effective number of interactions, and it is
/// exact: every weight is a multiple of a quarter, so the decimal the user is
/// shown terminates after two places.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decayed {
    /// Sum of bucket weights over interactions.
    pub eff_count_milli: u64,
    /// Sum of bucket weights over distinct active days.
    pub eff_days_milli: u64,
    /// How many interactions landed in each bucket, newest bucket first.
    pub bucket_interactions: [u64; 4],
    /// How many active days landed in each bucket, newest bucket first.
    pub bucket_days: [u64; 4],
}

/// What one candidate concluded about one peer, plus everything the user needs
/// to check the conclusion by hand.
///
/// Every count here is a plain tally of raw observations, unweighted, in every
/// candidate — including the one that decides the band from weighted
/// quantities. A user who disagrees with the band can still count messages and
/// get the same numbers.
///
/// When `interaction_count` is 0 there was nothing to observe and both
/// timestamps are 0; read them as "no contact", not as 1970.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TieScore {
    pub band: Band,
    pub interaction_count: u64,
    pub outgoing_count: u64,
    pub incoming_count: u64,
    pub conversation_count: u64,
    pub active_day_count: u64,
    pub first_contact_unix: i64,
    pub last_contact_unix: i64,
    /// At least one exchange happened one to one.
    ///
    /// Round 2 requirement: venue is the deciding fact in T3/T3R/T4, so it has
    /// to travel with the verdict or the explanation layer has to go back to
    /// the evidence store to say why.
    pub any_direct: bool,
    /// Whole UTC days between the last exchange and `as_of`. 0 when nothing
    /// was observed.
    pub silent_days: i64,
    /// The instant the rule was asked about. Never a clock reading.
    pub as_of_unix: i64,
    pub detail: Detail,
    pub algorithm_id: &'static str,
}

impl TieScore {
    /// Both sides have written at least once.
    ///
    /// The gate every candidate in this family keeps: a mailing list is not a
    /// friendship, however long it is.
    pub const fn is_reciprocal(&self) -> bool {
        self.outgoing_count > 0 && self.incoming_count > 0
    }

    /// Nothing was observed about this peer.
    pub const fn is_empty(&self) -> bool {
        self.interaction_count == 0
    }

    /// Observed, but never one to one.
    pub const fn is_group_only(&self) -> bool {
        !self.is_empty() && !self.any_direct
    }
}

/// A candidate rule for turning observations into a band.
pub trait TieAlgorithm {
    const ID: &'static str;

    /// Score one peer as of `as_of_unix`.
    ///
    /// `interactions` may contain anybody; only rows whose `peer_id` matches
    /// are read. `as_of_unix` is store-level and comes from the caller — see
    /// the module docs on why it is never a clock reading.
    fn score(peer_id: u64, interactions: &[Interaction], as_of_unix: i64) -> TieScore;

    /// Why the band came out that way, in the language the user reads.
    ///
    /// No English, no jargon, no internal identifiers: counts, days, dates and
    /// — for T3R only — quarter fractions. Callers can rely on the returned
    /// string containing no ASCII letters at all.
    fn explain_zh(score: &TieScore) -> String;
}

/// The UTC day a timestamp falls in, as a count of days since 1970-01-01.
///
/// Floor division, not truncation: `div_euclid` sends -1 (one second before
/// the epoch) to day -1, where `/` would send it to day 0 and quietly merge
/// the last second of 1969 into the first day of 1970. Nothing here reads the
/// machine's timezone, so a tie scored in Shanghai and the same tie scored in
/// CI agree, which is the whole reason active days are counted in UTC rather
/// than in the user's local calendar.
///
/// The trade-off is stated rather than hidden: for a user at UTC+8, an
/// exchange at 01:00 local counts as the previous day. Active days is a
/// spread-over-time signal, so an off-by-one on the boundary costs at most one
/// day at the edges, and it costs the same day for every observer.
pub const fn epoch_day(occurred_at_unix: i64) -> i64 {
    occurred_at_unix.div_euclid(SECONDS_PER_DAY)
}

/// Whole UTC days between an instant and `as_of`, floored, never negative.
///
/// `age_days = max(0, as_of - occurred_at) / 86400`. Clamping means a clock
/// that is slightly ahead of `as_of` — imported evidence often is — reads as
/// "today" rather than as negative age, so no bucket weight can exceed the
/// newest bucket's.
pub const fn age_days(occurred_at_unix: i64, as_of_unix: i64) -> i64 {
    let delta = as_of_unix.saturating_sub(occurred_at_unix);
    if delta <= 0 {
        0
    } else {
        delta / SECONDS_PER_DAY
    }
}

/// The newest timestamp in a log, which is what `as_of` is defined to be.
///
/// `None` for an empty store: there is no evidence, so there is no "as of".
/// Callers hand this to [`TieAlgorithm::score`]; the rules never call it
/// themselves, because the store-level maximum is not knowable from one peer's
/// rows. Scoring a dormant peer against that peer's own last message would
/// make every dormant tie look current — see `tests/as_of_discipline.rs`.
pub fn as_of_max(interactions: &[Interaction]) -> Option<i64> {
    interactions.iter().map(|row| row.occurred_at_unix).max()
}

/// The civil UTC date of a day number, as (year, month, day).
///
/// Hinnant's `civil_from_days`, which is exact for the whole proleptic
/// Gregorian range and needs no dependency and no timezone database. It exists
/// so an explanation can name a date instead of a Unix integer.
pub const fn civil_from_epoch_day(day: i64) -> (i64, u32, u32) {
    let z = day + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// A UTC instant written the way the user reads dates.
pub fn zh_date(occurred_at_unix: i64) -> String {
    let (y, m, d) = civil_from_epoch_day(epoch_day(occurred_at_unix));
    format!("{y} 年 {m} 月 {d} 日")
}

/// A count of quarter-units written as a decimal, without touching a float.
///
/// `38 -> "9.5"`, `40 -> "10"`, `13 -> "3.25"`. At most two decimal places,
/// always exact, because every weight is a multiple of a quarter.
pub fn zh_quarters(milli: u64) -> String {
    let whole = milli / 4;
    match milli % 4 {
        0 => format!("{whole}"),
        1 => format!("{whole}.25"),
        2 => format!("{whole}.5"),
        _ => format!("{whole}.75"),
    }
}

/// Everything one peer's observations add up to, in one pass.
///
/// This mirrors `Tally` in Goal 1's `graph_build.rs`. The differences are
/// mechanical: timestamps are integers instead of RFC 3339 strings, and active
/// days are epoch day numbers instead of the first ten characters of the
/// string. Both spellings compute the same set of UTC dates — pinned in
/// `tests/goal1_fidelity.rs`.
#[derive(Clone, Debug, Default)]
pub struct Tally {
    pub outgoing: u64,
    pub incoming: u64,
    pub conversations: BTreeSet<u64>,
    /// Epoch day number -> the latest timestamp observed on that day.
    ///
    /// The keys are Goal 1's active-day set. The values are what T3R needs:
    /// a day's decay weight is the weight of its newest exchange.
    pub days: BTreeMap<i64, i64>,
    pub first_contact: i64,
    pub last_contact: i64,
    /// At least one exchange happened one to one.
    pub any_direct: bool,
    /// At least one exchange happened with other people present.
    pub any_group: bool,
    /// When each matching exchange happened, so a weighted rule can make a
    /// second pass without re-filtering the whole log. Timestamps only: no
    /// direction, no venue, no content.
    pub timestamps: Vec<i64>,
}

impl Tally {
    /// Read the log once, keeping only the rows about `peer_id`.
    ///
    /// O(n) in the length of the log plus O(k log k) for the two ordered
    /// collections, where k is the number of rows about this peer. No pairwise
    /// work, so scoring a whole ego network stays linear in evidence.
    pub fn of(peer_id: u64, interactions: &[Interaction]) -> Tally {
        let mut tally = Tally::default();
        for interaction in interactions.iter().filter(|row| row.peer_id == peer_id) {
            tally.absorb(interaction);
        }
        tally
    }

    fn absorb(&mut self, interaction: &Interaction) {
        let at = interaction.occurred_at_unix;
        if self.timestamps.is_empty() {
            self.first_contact = at;
            self.last_contact = at;
        }
        if at < self.first_contact {
            self.first_contact = at;
        }
        if at > self.last_contact {
            self.last_contact = at;
        }

        match interaction.outgoing {
            true => self.outgoing += 1,
            false => self.incoming += 1,
        }
        self.any_direct |= interaction.venue_direct;
        self.any_group |= !interaction.venue_direct;
        self.conversations.insert(interaction.conversation_id);
        self.days
            .entry(epoch_day(at))
            .and_modify(|latest| {
                if at > *latest {
                    *latest = at;
                }
            })
            .or_insert(at);
        self.timestamps.push(at);
    }

    pub fn interaction_count(&self) -> u64 {
        self.outgoing + self.incoming
    }

    pub fn active_day_count(&self) -> u64 {
        self.days.len() as u64
    }

    pub fn conversation_count(&self) -> u64 {
        self.conversations.len() as u64
    }

    pub fn is_reciprocal(&self) -> bool {
        self.outgoing > 0 && self.incoming > 0
    }

    pub fn is_empty(&self) -> bool {
        self.timestamps.is_empty()
    }

    /// Only ever seen with other people in the conversation.
    pub fn is_group_only(&self) -> bool {
        !self.is_empty() && !self.any_direct
    }

    /// Whole UTC days since the last exchange, 0 when there was none.
    pub fn silent_days(&self, as_of_unix: i64) -> i64 {
        if self.is_empty() {
            0
        } else {
            age_days(self.last_contact, as_of_unix)
        }
    }

    /// The tally as a score, with the band and the working the caller decided.
    pub fn to_score(
        &self,
        band: Band,
        algorithm_id: &'static str,
        as_of_unix: i64,
        detail: Detail,
    ) -> TieScore {
        TieScore {
            band,
            interaction_count: self.interaction_count(),
            outgoing_count: self.outgoing,
            incoming_count: self.incoming,
            conversation_count: self.conversation_count(),
            active_day_count: self.active_day_count(),
            first_contact_unix: self.first_contact,
            last_contact_unix: self.last_contact,
            any_direct: self.any_direct,
            silent_days: self.silent_days(as_of_unix),
            as_of_unix,
            detail,
            algorithm_id,
        }
    }
}

/// Tally every peer in the log in a single pass.
///
/// The ego-network entry point. Calling [`Tally::of`] once per peer would read
/// the whole log once per peer; this reads it once in total, which is what
/// keeps a rebuild linear in evidence rather than in evidence times people.
pub fn tally_ego_network(interactions: &[Interaction]) -> BTreeMap<u64, Tally> {
    let mut by_peer: BTreeMap<u64, Tally> = BTreeMap::new();
    for interaction in interactions {
        by_peer
            .entry(interaction.peer_id)
            .or_default()
            .absorb(interaction);
    }
    by_peer
}

/// The opening line every explanation shares: who said how much, over how
/// long, and how long ago.
///
/// Kept in one place so the candidates cannot drift into describing the same
/// counts in different ways. Every number in it is one the user can recount:
/// exchanges, days, conversations, a date, and days since.
pub(crate) fn zh_counts(score: &TieScore) -> String {
    format!(
        "你们一共有 {} 次往来（你发出 {} 次，对方发来 {} 次），分布在 {} 天、{} 个会话里，最近一次是 {}，距今 {} 天。",
        score.interaction_count,
        score.outgoing_count,
        score.incoming_count,
        score.active_day_count,
        score.conversation_count,
        zh_date(score.last_contact_unix),
        score.silent_days,
    )
}

/// The nothing-observed and one-sided cases, which read the same whichever
/// rule asked, because every rule in the family gates on reciprocity first.
pub(crate) fn zh_common_weak_reason(score: &TieScore) -> Option<String> {
    if score.is_empty() {
        return Some("还没有看到你们之间的往来记录，所以先按最弱的一档放着。".to_string());
    }
    if score.incoming_count == 0 {
        return Some(format!(
            "{}对方一次也没有回过，只有一头在说话的关系不算紧密，所以是弱联系。",
            zh_counts(score)
        ));
    }
    if score.outgoing_count == 0 {
        return Some(format!(
            "{}你一次也没有回过，只有一头在说话的关系不算紧密，所以是弱联系。",
            zh_counts(score)
        ));
    }
    None
}

/// Whether the pair ever spoke one to one, as the user reads it.
pub(crate) fn zh_venue(any_direct: bool) -> &'static str {
    if any_direct {
        "你们私下一对一聊过"
    } else {
        "你们的往来都发生在群里，没有单独聊过"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_day_floors_below_the_epoch() {
        assert_eq!(epoch_day(0), 0);
        assert_eq!(epoch_day(SECONDS_PER_DAY - 1), 0);
        assert_eq!(epoch_day(SECONDS_PER_DAY), 1);
        // Truncating division would call this day 0 and merge 1969-12-31 into
        // 1970-01-01.
        assert_eq!(epoch_day(-1), -1);
        assert_eq!(epoch_day(-SECONDS_PER_DAY), -1);
        assert_eq!(epoch_day(-SECONDS_PER_DAY - 1), -2);
    }

    #[test]
    fn civil_dates_round_trip_known_instants() {
        assert_eq!(civil_from_epoch_day(0), (1970, 1, 1));
        assert_eq!(civil_from_epoch_day(-1), (1969, 12, 31));
        // 2026-08-24T00:00:00Z
        assert_eq!(
            civil_from_epoch_day(epoch_day(1_787_529_600)),
            (2026, 8, 24)
        );
        // A leap day, which off-by-one date maths tends to miss.
        assert_eq!(
            civil_from_epoch_day(epoch_day(1_709_164_800)),
            (2024, 2, 29)
        );
    }

    #[test]
    fn ages_are_whole_days_and_never_negative() {
        assert_eq!(age_days(100, 0), 0);
        assert_eq!(age_days(0, SECONDS_PER_DAY * 7 + 1), 7);
        assert_eq!(age_days(0, SECONDS_PER_DAY * 7 - 1), 6);
        assert_eq!(age_days(0, 0), 0);
    }

    #[test]
    fn bands_cap_downwards_and_demote_one_rung() {
        assert_eq!(Band::Strong.capped_at(Band::Weak), Band::Weak);
        assert_eq!(Band::Weak.capped_at(Band::Strong), Band::Weak);
        assert_eq!(Band::Moderate.capped_at(Band::Moderate), Band::Moderate);
        assert_eq!(Band::Strong.demoted_once(), Band::Moderate);
        assert_eq!(Band::Moderate.demoted_once(), Band::Weak);
        assert_eq!(Band::Weak.demoted_once(), Band::Weak);
    }

    #[test]
    fn quarters_read_back_as_exact_decimals() {
        assert_eq!(zh_quarters(0), "0");
        assert_eq!(zh_quarters(1), "0.25");
        assert_eq!(zh_quarters(2), "0.5");
        assert_eq!(zh_quarters(3), "0.75");
        assert_eq!(zh_quarters(40), "10");
        assert_eq!(zh_quarters(38), "9.5");
        assert_eq!(zh_quarters(13), "3.25");
    }

    #[test]
    fn a_day_remembers_its_latest_exchange() {
        let log = vec![
            Interaction {
                peer_id: 1,
                outgoing: true,
                occurred_at_unix: 3_600,
                venue_direct: true,
                conversation_id: 1,
            },
            Interaction {
                peer_id: 1,
                outgoing: false,
                occurred_at_unix: 20 * 3_600,
                venue_direct: true,
                conversation_id: 1,
            },
            Interaction {
                peer_id: 1,
                outgoing: true,
                occurred_at_unix: 10 * 3_600,
                venue_direct: true,
                conversation_id: 1,
            },
        ];
        let tally = Tally::of(1, &log);
        assert_eq!(tally.active_day_count(), 1);
        assert_eq!(tally.days.get(&0), Some(&(20 * 3_600)));
    }

    #[test]
    fn tally_ignores_other_peers() {
        let log = vec![
            Interaction {
                peer_id: 1,
                outgoing: true,
                occurred_at_unix: 0,
                venue_direct: true,
                conversation_id: 7,
            },
            Interaction {
                peer_id: 2,
                outgoing: false,
                occurred_at_unix: 0,
                venue_direct: true,
                conversation_id: 8,
            },
        ];
        let tally = Tally::of(1, &log);
        assert_eq!(tally.interaction_count(), 1);
        assert_eq!(tally.conversation_count(), 1);
    }

    #[test]
    fn as_of_max_is_the_newest_row_or_nothing() {
        assert_eq!(as_of_max(&[]), None);
        let log = vec![
            Interaction {
                peer_id: 1,
                outgoing: true,
                occurred_at_unix: 10,
                venue_direct: true,
                conversation_id: 1,
            },
            Interaction {
                peer_id: 2,
                outgoing: true,
                occurred_at_unix: 99,
                venue_direct: true,
                conversation_id: 1,
            },
        ];
        assert_eq!(as_of_max(&log), Some(99));
    }
}
