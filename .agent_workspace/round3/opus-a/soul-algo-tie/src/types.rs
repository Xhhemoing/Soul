//! What both rules read and return.
//!
//! One shape of input (an observed [`Interaction`]), one shape of output (a
//! [`TieScore`]), and one pass over the evidence ([`Tally`]) that T4 and T4D
//! share. Keeping the tally in one place means the two rules differ only in
//! *which of its counts they band on*, which is the only thing Round 3 is
//! trying to decide.
//!
//! ## Split counts (Round 3)
//!
//! Round 2 carried one pair of counts (outgoing / incoming) and one boolean
//! (`any_direct`). fable-b showed that the boolean is not a strong enough
//! latch — a group fan-out plus one private hello on each side still cleared
//! every Strong gate — so the tally now keeps four counts,
//! `direct_out / direct_in / group_out / group_in`, and two day sets, one for
//! every venue and one for the one-to-one rows only. The combined totals every
//! rule used to report are derived sums, so nothing a user could previously
//! recount has changed.
//!
//! ## `as_of`, not "now"
//!
//! Every entry point takes `as_of_unix` and nothing in this crate reads a
//! clock. R2-SYNTHESIS §冻结边界: one value per rebuild, for the whole store,
//! supplied by the caller. Ages are whole UTC days, floored, clamped at zero:
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
    /// The whole recency mechanism: no float, no re-derivation, just a step
    /// down a three-valued ladder the user can see.
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

/// The working the rule showed, beyond the counts every rule reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Detail {
    /// The band came straight from the counts, with no recency step. Only the
    /// test-only oracle in [`crate::testing::oracle`] produces this.
    RawCounts,
    /// The band came from the counts, and then silence pushed it down or left
    /// it alone. Both product rules produce this.
    Demoted {
        /// What the counts stage said before recency was applied.
        band_before: Band,
    },
}

/// What one rule concluded about one peer, plus everything the user needs to
/// check the conclusion by hand.
///
/// Every count here is a plain tally of raw observations, unweighted. A user
/// who disagrees with the band can still count messages and get the same
/// numbers.
///
/// When `interaction_count` is 0 there was nothing to observe and every
/// timestamp is 0; read them as "no contact", not as 1970.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TieScore {
    pub band: Band,

    // -- combined totals, the numbers Round 2 already reported ---------------
    pub interaction_count: u64,
    pub outgoing_count: u64,
    pub incoming_count: u64,

    // -- the Round 3 split ---------------------------------------------------
    /// One-to-one messages the user wrote.
    pub direct_out_count: u64,
    /// One-to-one messages the peer wrote.
    pub direct_in_count: u64,
    /// Messages the user wrote with other people in the room.
    pub group_out_count: u64,
    /// Messages the peer wrote with other people in the room.
    pub group_in_count: u64,

    pub conversation_count: u64,
    /// Distinct UTC days with any exchange at all.
    pub active_day_count: u64,
    /// Distinct UTC days with a one-to-one exchange. Never larger than
    /// `active_day_count`.
    pub direct_active_day_count: u64,

    pub first_contact_unix: i64,
    /// The newest exchange in **any** venue. This is what the recency step
    /// measures against, in both rules.
    pub last_contact_unix: i64,
    /// The newest one-to-one exchange, 0 when there was none. Reported, never
    /// banded on: R2-SYNTHESIS keeps one recency clock, not two.
    pub last_direct_contact_unix: i64,

    /// Whole UTC days between `last_contact_unix` and `as_of`. 0 when nothing
    /// was observed.
    pub silent_days: i64,
    /// The instant the rule was asked about. Never a clock reading.
    pub as_of_unix: i64,
    pub detail: Detail,
    pub algorithm_id: &'static str,
}

impl TieScore {
    /// One-to-one messages, both directions.
    pub const fn direct_count(&self) -> u64 {
        self.direct_out_count + self.direct_in_count
    }

    /// Group messages, both directions.
    pub const fn group_count(&self) -> u64 {
        self.group_out_count + self.group_in_count
    }

    /// Both sides have written at least once, in any venue. T4's gate.
    pub const fn is_reciprocal(&self) -> bool {
        self.outgoing_count > 0 && self.incoming_count > 0
    }

    /// Both sides have written at least once **one to one**. T4D's gate.
    ///
    /// This is the latch fable-b asked for: being talked at in a group and
    /// answering in the same group says two people are in a room together, not
    /// that either of them chose the other.
    pub const fn is_direct_reciprocal(&self) -> bool {
        self.direct_out_count > 0 && self.direct_in_count > 0
    }

    /// At least one exchange happened one to one.
    pub const fn any_direct(&self) -> bool {
        self.direct_count() > 0
    }

    /// Nothing was observed about this peer.
    pub const fn is_empty(&self) -> bool {
        self.interaction_count == 0
    }

    /// Observed, but never one to one.
    pub const fn is_group_only(&self) -> bool {
        !self.is_empty() && !self.any_direct()
    }
}

/// A rule for turning observations into a band.
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
    /// nothing else. Callers can rely on the returned string containing no
    /// ASCII letters at all.
    fn explain_zh(score: &TieScore) -> String;
}

/// The UTC day a timestamp falls in, as a count of days since 1970-01-01.
///
/// Floor division, not truncation: `div_euclid` sends -1 (one second before
/// the epoch) to day -1, where `/` would send it to day 0 and quietly merge
/// the last second of 1969 into the first day of 1970. Nothing here reads the
/// machine's timezone, so a tie scored in Shanghai and the same tie scored in
/// CI agree.
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
/// Clamping means a clock that is slightly ahead of `as_of` — imported
/// evidence often is — reads as "today" rather than as a negative age.
pub const fn age_days(occurred_at_unix: i64, as_of_unix: i64) -> i64 {
    let delta = as_of_unix.saturating_sub(occurred_at_unix);
    if delta <= 0 {
        0
    } else {
        delta / SECONDS_PER_DAY
    }
}

/// The newest timestamp in a log, one legal way for a caller to pick `as_of`.
///
/// `None` for an empty store: there is no evidence, so there is no "as of".
/// The rules never call this themselves, because the store-level maximum is
/// not knowable from one peer's rows. Scoring a dormant peer against that
/// peer's own last message would make every dormant tie look current — see
/// `tests/as_of_discipline.rs`.
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

/// Everything one peer's observations add up to, in one pass.
///
/// This mirrors `Tally` in Goal 1's `graph_build.rs`. The differences are
/// mechanical — timestamps are integers instead of RFC 3339 strings, active
/// days are epoch day numbers instead of the first ten characters of the
/// string, and the counts are split by venue — and the combined totals are
/// unchanged. Pinned in `tests/goal1_fidelity.rs`.
#[derive(Clone, Debug, Default)]
pub struct Tally {
    /// One-to-one, written by the user.
    pub direct_out: u64,
    /// One-to-one, written by the peer.
    pub direct_in: u64,
    /// In a group, written by the user.
    pub group_out: u64,
    /// In a group, written by the peer.
    pub group_in: u64,
    pub conversations: BTreeSet<u64>,
    /// Distinct UTC days with any exchange. Goal 1's active-day set.
    pub days: BTreeSet<i64>,
    /// Distinct UTC days with a one-to-one exchange. A subset of `days`.
    pub direct_days: BTreeSet<i64>,
    pub first_contact: i64,
    pub last_contact: i64,
    /// Newest one-to-one exchange, 0 when there was none.
    pub last_direct_contact: i64,
}

impl Tally {
    /// Read the log once, keeping only the rows about `peer_id`.
    ///
    /// O(n) in the length of the log plus O(k log k) for the ordered
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
        if self.is_empty() {
            self.first_contact = at;
            self.last_contact = at;
        }
        if at < self.first_contact {
            self.first_contact = at;
        }
        if at > self.last_contact {
            self.last_contact = at;
        }

        match (interaction.venue_direct, interaction.outgoing) {
            (true, true) => self.direct_out += 1,
            (true, false) => self.direct_in += 1,
            (false, true) => self.group_out += 1,
            (false, false) => self.group_in += 1,
        }
        if interaction.venue_direct {
            self.direct_days.insert(epoch_day(at));
            if self.last_direct_contact == 0 || at > self.last_direct_contact {
                self.last_direct_contact = at;
            }
        }
        self.conversations.insert(interaction.conversation_id);
        self.days.insert(epoch_day(at));
    }

    pub const fn direct_count(&self) -> u64 {
        self.direct_out + self.direct_in
    }

    pub const fn group_count(&self) -> u64 {
        self.group_out + self.group_in
    }

    pub const fn outgoing(&self) -> u64 {
        self.direct_out + self.group_out
    }

    pub const fn incoming(&self) -> u64 {
        self.direct_in + self.group_in
    }

    pub const fn interaction_count(&self) -> u64 {
        self.direct_count() + self.group_count()
    }

    pub fn active_day_count(&self) -> u64 {
        self.days.len() as u64
    }

    pub fn direct_active_day_count(&self) -> u64 {
        self.direct_days.len() as u64
    }

    pub fn conversation_count(&self) -> u64 {
        self.conversations.len() as u64
    }

    /// Both sides wrote something, anywhere. T4's gate.
    pub const fn is_reciprocal(&self) -> bool {
        self.outgoing() > 0 && self.incoming() > 0
    }

    /// Both sides wrote something one to one. T4D's gate.
    pub const fn is_direct_reciprocal(&self) -> bool {
        self.direct_out > 0 && self.direct_in > 0
    }

    pub const fn any_direct(&self) -> bool {
        self.direct_count() > 0
    }

    pub const fn any_group(&self) -> bool {
        self.group_count() > 0
    }

    pub const fn is_empty(&self) -> bool {
        self.interaction_count() == 0
    }

    /// Only ever seen with other people in the conversation.
    pub const fn is_group_only(&self) -> bool {
        !self.is_empty() && !self.any_direct()
    }

    /// Whole UTC days since the last exchange **in any venue**, 0 when there
    /// was none.
    ///
    /// Any venue on purpose: R3 brief. A group message yesterday means the two
    /// of you are still in each other's lives, so the dormancy step does not
    /// fire, even under the rule that will not band on group traffic.
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
            outgoing_count: self.outgoing(),
            incoming_count: self.incoming(),
            direct_out_count: self.direct_out,
            direct_in_count: self.direct_in,
            group_out_count: self.group_out,
            group_in_count: self.group_in,
            conversation_count: self.conversation_count(),
            active_day_count: self.active_day_count(),
            direct_active_day_count: self.direct_active_day_count(),
            first_contact_unix: self.first_contact,
            last_contact_unix: self.last_contact,
            last_direct_contact_unix: self.last_direct_contact,
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

/// T4's opening line: how much was said in total, over how long, how long ago.
///
/// Every number in it is one the user can recount: exchanges, days,
/// conversations, a date, and days since.
pub(crate) fn zh_counts_total(score: &TieScore) -> String {
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

/// T4D's opening line: the same totals, with the one-to-one exchanges counted
/// apart from the ones that happened in a group.
///
/// The split is not decoration. T4D bands on the one-to-one numbers alone, so
/// if the sentence only gave a total the user could not tell why a busy group
/// tie came out Weak. Both numbers are always named, including when one of
/// them is zero.
pub(crate) fn zh_counts_split(score: &TieScore) -> String {
    format!(
        "你们一共有 {} 次往来：一对一 {} 次（你发出 {} 次，对方发来 {} 次），群里 {} 次（你发出 {} 次，对方发来 {} 次）；这些往来分布在 {} 天、{} 个会话里，最近一次是 {}，距今 {} 天。",
        score.interaction_count,
        score.direct_count(),
        score.direct_out_count,
        score.direct_in_count,
        score.group_count(),
        score.group_out_count,
        score.group_in_count,
        score.active_day_count,
        score.conversation_count,
        zh_date(score.last_contact_unix),
        score.silent_days,
    )
}

/// The nothing-observed and nobody-answered cases, which read the same
/// whichever rule asked, because both rules gate on reciprocity first.
///
/// `counts` is the caller's own rendering of the numbers, so T4 keeps its
/// total and T4D keeps its split.
pub(crate) fn zh_common_weak_reason(score: &TieScore, counts: &str) -> Option<String> {
    if score.is_empty() {
        return Some("还没有看到你们之间的往来记录，所以先按最弱的一档放着。".to_string());
    }
    if score.incoming_count == 0 {
        return Some(format!(
            "{counts}对方一次也没有回过，只有一头在说话的关系不算紧密，所以是弱联系。"
        ));
    }
    if score.outgoing_count == 0 {
        return Some(format!(
            "{counts}你一次也没有回过，只有一头在说话的关系不算紧密，所以是弱联系。"
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
    use crate::testing::{direct, group};

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
    fn the_four_counts_add_up_to_the_two_totals() {
        let log = vec![
            direct(1, true, 0, 1),
            direct(1, false, 3_600, 1),
            direct(1, true, 7_200, 1),
            group(1, false, 10_800, 2),
            group(1, false, 14_400, 2),
        ];
        let tally = Tally::of(1, &log);
        assert_eq!((tally.direct_out, tally.direct_in), (2, 1));
        assert_eq!((tally.group_out, tally.group_in), (0, 2));
        assert_eq!(tally.direct_count(), 3);
        assert_eq!(tally.group_count(), 2);
        assert_eq!(tally.outgoing(), 2);
        assert_eq!(tally.incoming(), 3);
        assert_eq!(tally.interaction_count(), 5);
        // Reciprocal in both senses here, and the two senses are different
        // questions — see `is_direct_reciprocal`.
        assert!(tally.is_reciprocal());
        assert!(tally.is_direct_reciprocal());
    }

    #[test]
    fn group_traffic_alone_is_reciprocal_but_never_directly_reciprocal() {
        let log = vec![group(1, true, 0, 1), group(1, false, 3_600, 1)];
        let tally = Tally::of(1, &log);
        assert!(tally.is_reciprocal());
        assert!(!tally.is_direct_reciprocal());
        assert!(tally.is_group_only());
        assert_eq!(tally.last_direct_contact, 0);
        assert_eq!(tally.direct_active_day_count(), 0);
    }

    #[test]
    fn direct_days_are_a_subset_of_all_days() {
        let log = vec![
            group(1, true, 0, 1),
            direct(1, false, SECONDS_PER_DAY, 1),
            group(1, true, SECONDS_PER_DAY * 2, 1),
            direct(1, true, SECONDS_PER_DAY * 2 + 60, 1),
        ];
        let tally = Tally::of(1, &log);
        assert_eq!(tally.active_day_count(), 3);
        assert_eq!(tally.direct_active_day_count(), 2);
        assert!(tally.direct_days.is_subset(&tally.days));
        assert_eq!(tally.last_direct_contact, SECONDS_PER_DAY * 2 + 60);
        assert_eq!(tally.last_contact, SECONDS_PER_DAY * 2 + 60);
    }

    #[test]
    fn several_exchanges_in_a_day_are_one_active_day() {
        let log = vec![
            direct(1, true, 3_600, 1),
            direct(1, false, 20 * 3_600, 1),
            direct(1, true, 10 * 3_600, 1),
        ];
        let tally = Tally::of(1, &log);
        assert_eq!(tally.active_day_count(), 1);
        assert_eq!(tally.last_contact, 20 * 3_600);
    }

    #[test]
    fn tally_ignores_other_peers() {
        let log = vec![direct(1, true, 0, 7), direct(2, false, 0, 8)];
        let tally = Tally::of(1, &log);
        assert_eq!(tally.interaction_count(), 1);
        assert_eq!(tally.conversation_count(), 1);
    }

    #[test]
    fn as_of_max_is_the_newest_row_or_nothing() {
        assert_eq!(as_of_max(&[]), None);
        let log = vec![direct(1, true, 10, 1), direct(2, true, 99, 1)];
        assert_eq!(as_of_max(&log), Some(99));
    }
}
