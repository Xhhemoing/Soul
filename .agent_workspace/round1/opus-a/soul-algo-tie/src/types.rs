//! What every tie-strength candidate reads and returns.
//!
//! One shape of input (an observed [`Interaction`]), one shape of output (a
//! [`TieScore`]), and one pass over the evidence ([`Tally`]) that all four
//! candidates share. Keeping the tally in one place means the candidates
//! differ only in the rule that turns counts into a band, which is the only
//! thing Round 1 is trying to compare.

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
    /// Weak < Moderate < Strong, for capping rules that need to say "no
    /// higher than this".
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

    /// The band as the storage layer spells it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Band::Weak => "weak",
            Band::Moderate => "moderate",
            Band::Strong => "strong",
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

/// What one candidate concluded about one peer, plus everything the user needs
/// to check the conclusion by hand.
///
/// Every field except `band` and `algorithm_id` is a plain tally of raw
/// observations, unweighted, in every candidate — including the ones that
/// decide the band from weighted quantities. A user who disagrees with the
/// band can still count messages and get the same numbers.
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
}

/// A candidate rule for turning observations into a band.
pub trait TieAlgorithm {
    const ID: &'static str;

    /// Score one peer. `interactions` may contain anybody; only rows whose
    /// `peer_id` matches are read.
    fn score(peer_id: u64, interactions: &[Interaction], now_unix: i64) -> TieScore;

    /// Why the band came out that way, in the language the user reads.
    ///
    /// No English, no jargon, no internal identifiers: counts, days and dates
    /// only. Callers can rely on the returned string containing no ASCII
    /// letters at all.
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

/// Whole UTC days between two instants, floored at zero.
///
/// Clamping means a clock that is slightly ahead of `now` — imported evidence
/// often is — reads as "today" rather than as negative age.
pub const fn age_days_floor(occurred_at_unix: i64, now_unix: i64) -> i64 {
    let delta = now_unix.saturating_sub(occurred_at_unix);
    if delta <= 0 {
        0
    } else {
        delta / SECONDS_PER_DAY
    }
}

/// Fractional days between two instants, floored at zero.
pub fn age_days_exact(occurred_at_unix: i64, now_unix: i64) -> f64 {
    let delta = now_unix.saturating_sub(occurred_at_unix).max(0);
    delta as f64 / SECONDS_PER_DAY as f64
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
/// mechanical: timestamps are integers instead of RFC 3339 strings, and active
/// days are epoch day numbers instead of the first ten characters of the
/// string. Both spellings compute the same set of UTC dates.
#[derive(Clone, Debug, Default)]
pub struct Tally {
    pub outgoing: u64,
    pub incoming: u64,
    pub conversations: BTreeSet<u64>,
    pub active_days: BTreeSet<i64>,
    pub first_contact: i64,
    pub last_contact: i64,
    /// At least one exchange happened one to one.
    pub any_direct: bool,
    /// At least one exchange happened with other people present.
    pub any_group: bool,
    /// Every matching interaction, so a weighted rule can make a second pass
    /// without re-filtering the whole log.
    pub matched: Vec<Interaction>,
}

impl Tally {
    /// Read the log once, keeping only the rows about `peer_id`.
    ///
    /// O(n) in the length of the log plus O(k log k) for the two distinct-value
    /// sets, where k is the number of rows about this peer. No pairwise work,
    /// so scoring a whole ego network stays linear in evidence.
    pub fn of(peer_id: u64, interactions: &[Interaction]) -> Tally {
        let mut tally = Tally::default();
        for interaction in interactions.iter().filter(|row| row.peer_id == peer_id) {
            tally.absorb(interaction);
        }
        tally
    }

    fn absorb(&mut self, interaction: &Interaction) {
        let at = interaction.occurred_at_unix;
        if self.matched.is_empty() {
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
        self.active_days.insert(epoch_day(at));
        self.matched.push(interaction.clone());
    }

    pub fn interaction_count(&self) -> u64 {
        self.outgoing + self.incoming
    }

    pub fn active_day_count(&self) -> u64 {
        self.active_days.len() as u64
    }

    pub fn conversation_count(&self) -> u64 {
        self.conversations.len() as u64
    }

    pub fn is_reciprocal(&self) -> bool {
        self.outgoing > 0 && self.incoming > 0
    }

    /// Only ever seen with other people in the conversation.
    pub fn is_group_only(&self) -> bool {
        !self.matched.is_empty() && !self.any_direct
    }

    /// The tally as a score, with the band the caller decided.
    pub fn into_score(self, band: Band, algorithm_id: &'static str) -> TieScore {
        TieScore {
            band,
            interaction_count: self.interaction_count(),
            outgoing_count: self.outgoing,
            incoming_count: self.incoming,
            conversation_count: self.conversation_count(),
            active_day_count: self.active_day_count(),
            first_contact_unix: self.first_contact,
            last_contact_unix: self.last_contact,
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

/// The opening line every explanation shares: who said how much, over how long.
///
/// Kept in one place so the four candidates cannot drift into describing the
/// same counts in four different ways.
pub(crate) fn zh_counts(score: &TieScore) -> String {
    format!(
        "你们一共有 {} 次往来（你发出 {} 次，对方发来 {} 次），分布在 {} 天、{} 个会话里，最近一次是 {}。",
        score.interaction_count,
        score.outgoing_count,
        score.incoming_count,
        score.active_day_count,
        score.conversation_count,
        zh_date(score.last_contact_unix),
    )
}

/// The one-sided and nothing-observed cases, which read the same whichever
/// rule asked.
pub(crate) fn zh_common_weak_reason(score: &TieScore) -> Option<String> {
    if score.is_empty() {
        return Some("还没有看到你们之间的往来记录，所以先按最弱的一档放着。".to_string());
    }
    if score.incoming_count == 0 {
        return Some(format!(
            "{}对方一次也没有回过，所以这条关系只算弱联系。",
            zh_counts(score)
        ));
    }
    if score.outgoing_count == 0 {
        return Some(format!(
            "{}你一次也没有回过，所以这条关系只算弱联系。",
            zh_counts(score)
        ));
    }
    None
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
    fn ages_never_go_negative() {
        assert_eq!(age_days_floor(100, 0), 0);
        assert_eq!(age_days_exact(100, 0), 0.0);
        assert_eq!(age_days_floor(0, SECONDS_PER_DAY * 7 + 1), 7);
        assert_eq!(age_days_exact(0, SECONDS_PER_DAY / 2), 0.5);
    }

    #[test]
    fn bands_cap_downwards_only() {
        assert_eq!(Band::Strong.capped_at(Band::Weak), Band::Weak);
        assert_eq!(Band::Weak.capped_at(Band::Strong), Band::Weak);
        assert_eq!(Band::Moderate.capped_at(Band::Moderate), Band::Moderate);
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
}
