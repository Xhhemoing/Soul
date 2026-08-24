//! Independent, dependency-free T4 versus T4D ablation.
//!
//! Both policies aggregate the same event log at one caller-supplied `as_of`.
//! T4 uses all-venue counts plus an `any_direct` Strong latch. T4D instead
//! applies every relationship gate to direct-event counts only. Both policies
//! use the latest event in any venue for dormancy demotion.

use std::collections::{BTreeMap, BTreeSet};

pub const SECONDS_PER_DAY: i64 = 86_400;
pub const MODERATE_MIN: u64 = 3;
pub const STRONG_MIN: u64 = 10;
pub const STRONG_DAYS: u64 = 3;
pub const DEMOTE_DAYS: i64 = 180;
pub const WEAK_DAYS: i64 = 360;
pub const MODEL_SLUG: &str = "gpt-5.6-sol-xhigh-fast";

/// Convert a valid proleptic-Gregorian civil time to Unix seconds.
pub const fn civil_to_unix(
    year: i64,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
) -> i64 {
    let adjusted_year = year - if month <= 2 { 1 } else { 0 };
    let era = if adjusted_year >= 0 {
        adjusted_year / 400
    } else {
        (adjusted_year - 399) / 400
    };
    let year_of_era = adjusted_year - era * 400;
    let shifted_month = if month > 2 {
        month as i64 - 3
    } else {
        month as i64 + 9
    };
    let day_of_year = (153 * shifted_month + 2) / 5 + day as i64 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let unix_day = era * 146_097 + day_of_era - 719_468;
    unix_day * SECONDS_PER_DAY + hour as i64 * 3_600 + minute as i64 * 60 + second as i64
}

pub const FIXTURE_AS_OF_UNIX: i64 = civil_to_unix(2026, 8, 24, 14, 0, 0);
pub const FIXTURE_AS_OF_RFC3339: &str = "2026-08-24T14:00:00Z";

/// Return the UTC civil-day number containing `unix`.
pub const fn utc_day(unix: i64) -> i64 {
    unix.div_euclid(SECONDS_PER_DAY)
}

/// Whole UTC-date boundaries between an event and `as_of`.
pub const fn age_days(as_of: i64, occurred_at: i64) -> i64 {
    utc_day(as_of) - utc_day(occurred_at)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    Outgoing,
    Incoming,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Venue {
    Direct,
    Group,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Band {
    Weak,
    Moderate,
    Strong,
}

impl Band {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Weak => "Weak",
            Self::Moderate => "Moderate",
            Self::Strong => "Strong",
        }
    }

    pub const fn demote_one(self) -> Self {
        match self {
            Self::Strong => Self::Moderate,
            Self::Moderate | Self::Weak => Self::Weak,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interaction {
    pub peer_id: String,
    pub direction: Direction,
    pub venue: Venue,
    pub occurred_at_unix: i64,
}

impl Interaction {
    pub fn new(
        peer_id: impl Into<String>,
        direction: Direction,
        venue: Venue,
        occurred_at_unix: i64,
    ) -> Self {
        Self {
            peer_id: peer_id.into(),
            direction,
            venue,
            occurred_at_unix,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TieScore {
    pub band: Band,
    pub interaction_count: u64,
    pub outgoing_count: u64,
    pub incoming_count: u64,
    pub active_day_count: u64,
    pub any_direct: bool,
    pub direct_count: u64,
    pub direct_outgoing_count: u64,
    pub direct_incoming_count: u64,
    pub direct_day_count: u64,
    pub last_contact_unix: Option<i64>,
    pub last_contact_age_days: Option<i64>,
}

impl TieScore {
    fn empty() -> Self {
        Self {
            band: Band::Weak,
            interaction_count: 0,
            outgoing_count: 0,
            incoming_count: 0,
            active_day_count: 0,
            any_direct: false,
            direct_count: 0,
            direct_outgoing_count: 0,
            direct_incoming_count: 0,
            direct_day_count: 0,
            last_contact_unix: None,
            last_contact_age_days: None,
        }
    }
}

#[derive(Debug, Default)]
struct Tally {
    outgoing: u64,
    incoming: u64,
    active_days: BTreeSet<i64>,
    direct_outgoing: u64,
    direct_incoming: u64,
    direct_days: BTreeSet<i64>,
    last_contact_unix: Option<i64>,
}

impl Tally {
    fn absorb(&mut self, interaction: &Interaction) {
        match interaction.direction {
            Direction::Outgoing => self.outgoing += 1,
            Direction::Incoming => self.incoming += 1,
        }

        let day = utc_day(interaction.occurred_at_unix);
        self.active_days.insert(day);
        if interaction.venue == Venue::Direct {
            match interaction.direction {
                Direction::Outgoing => self.direct_outgoing += 1,
                Direction::Incoming => self.direct_incoming += 1,
            }
            self.direct_days.insert(day);
        }

        if self
            .last_contact_unix
            .is_none_or(|last| interaction.occurred_at_unix > last)
        {
            self.last_contact_unix = Some(interaction.occurred_at_unix);
        }
    }

    const fn interaction_count(&self) -> u64 {
        self.outgoing + self.incoming
    }

    const fn direct_count(&self) -> u64 {
        self.direct_outgoing + self.direct_incoming
    }

    const fn reciprocal(&self) -> bool {
        self.outgoing >= 1 && self.incoming >= 1
    }

    const fn direct_reciprocal(&self) -> bool {
        self.direct_outgoing >= 1 && self.direct_incoming >= 1
    }

    fn t0_band(&self) -> Band {
        if self.reciprocal()
            && self.interaction_count() >= STRONG_MIN
            && self.active_days.len() as u64 >= STRONG_DAYS
        {
            Band::Strong
        } else if self.reciprocal() && self.interaction_count() >= MODERATE_MIN {
            Band::Moderate
        } else {
            Band::Weak
        }
    }

    /// Round 2 T4 base: all-venue gates and one direct-event Strong latch.
    fn t4_base_band(&self) -> Band {
        if self.reciprocal()
            && self.direct_count() >= 1
            && self.interaction_count() >= STRONG_MIN
            && self.active_days.len() as u64 >= STRONG_DAYS
        {
            Band::Strong
        } else if self.reciprocal() && self.interaction_count() >= MODERATE_MIN {
            Band::Moderate
        } else {
            Band::Weak
        }
    }

    /// T4D base: reciprocity, count, and day gates all use direct events.
    fn t4d_base_band(&self) -> Band {
        if self.direct_reciprocal()
            && self.direct_count() >= STRONG_MIN
            && self.direct_days.len() as u64 >= STRONG_DAYS
        {
            Band::Strong
        } else if self.direct_reciprocal() && self.direct_count() >= MODERATE_MIN {
            Band::Moderate
        } else {
            Band::Weak
        }
    }

    fn finish(self, base_band: Band, as_of: i64, demote: bool) -> TieScore {
        let last_contact_age_days = self.last_contact_unix.map(|last| age_days(as_of, last));
        let band = if demote {
            apply_dormancy(base_band, last_contact_age_days)
        } else {
            base_band
        };
        TieScore {
            band,
            interaction_count: self.interaction_count(),
            outgoing_count: self.outgoing,
            incoming_count: self.incoming,
            active_day_count: self.active_days.len() as u64,
            any_direct: self.direct_count() >= 1,
            direct_count: self.direct_count(),
            direct_outgoing_count: self.direct_outgoing,
            direct_incoming_count: self.direct_incoming,
            direct_day_count: self.direct_days.len() as u64,
            last_contact_unix: self.last_contact_unix,
            last_contact_age_days,
        }
    }
}

const fn apply_dormancy(base_band: Band, last_contact_age_days: Option<i64>) -> Band {
    match last_contact_age_days {
        Some(age) if age >= WEAK_DAYS => Band::Weak,
        Some(age) if age >= DEMOTE_DAYS => base_band.demote_one(),
        _ => base_band,
    }
}

fn aggregate(interactions: &[Interaction]) -> BTreeMap<String, Tally> {
    let mut tallies = BTreeMap::<String, Tally>::new();
    for interaction in interactions {
        tallies
            .entry(interaction.peer_id.clone())
            .or_default()
            .absorb(interaction);
    }
    tallies
}

/// T0 is retained only as the benchmark baseline.
pub fn score_all_t0(interactions: &[Interaction], as_of: i64) -> BTreeMap<String, TieScore> {
    aggregate(interactions)
        .into_iter()
        .map(|(peer_id, tally)| {
            let base_band = tally.t0_band();
            (peer_id, tally.finish(base_band, as_of, false))
        })
        .collect()
}

pub fn score_all_t4(interactions: &[Interaction], as_of: i64) -> BTreeMap<String, TieScore> {
    aggregate(interactions)
        .into_iter()
        .map(|(peer_id, tally)| {
            let base_band = tally.t4_base_band();
            (peer_id, tally.finish(base_band, as_of, true))
        })
        .collect()
}

pub fn score_all_t4d(interactions: &[Interaction], as_of: i64) -> BTreeMap<String, TieScore> {
    aggregate(interactions)
        .into_iter()
        .map(|(peer_id, tally)| {
            let base_band = tally.t4d_base_band();
            (peer_id, tally.finish(base_band, as_of, true))
        })
        .collect()
}

fn score_peer(all_scores: BTreeMap<String, TieScore>, peer_id: &str) -> TieScore {
    all_scores
        .get(peer_id)
        .cloned()
        .unwrap_or_else(TieScore::empty)
}

pub fn score_t0(interactions: &[Interaction], peer_id: &str, as_of: i64) -> TieScore {
    score_peer(score_all_t0(interactions, as_of), peer_id)
}

pub fn score_t4(interactions: &[Interaction], peer_id: &str, as_of: i64) -> TieScore {
    score_peer(score_all_t4(interactions, as_of), peer_id)
}

pub fn score_t4d(interactions: &[Interaction], peer_id: &str, as_of: i64) -> TieScore {
    score_peer(score_all_t4d(interactions, as_of), peer_id)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fixture {
    pub id: &'static str,
    pub primary_peer: &'static str,
    pub as_of: i64,
    pub interactions: Vec<Interaction>,
}

fn at_age(age: i64, seconds_into_day: i64) -> i64 {
    (utc_day(FIXTURE_AS_OF_UNIX) - age) * SECONDS_PER_DAY + seconds_into_day
}

pub fn fixture_lilei() -> Fixture {
    let mut interactions = Vec::with_capacity(12);
    for age in 0..6 {
        interactions.push(Interaction::new(
            "lilei",
            Direction::Outgoing,
            Venue::Direct,
            at_age(age, 9 * 3_600),
        ));
        interactions.push(Interaction::new(
            "lilei",
            Direction::Incoming,
            Venue::Direct,
            at_age(age, 9 * 3_600 + 5 * 60),
        ));
    }
    Fixture {
        id: "F_LILEI",
        primary_peer: "lilei",
        as_of: FIXTURE_AS_OF_UNIX,
        interactions,
    }
}

pub fn fixture_burst() -> Fixture {
    let interactions = (0..20)
        .map(|index| {
            Interaction::new(
                "burst-peer",
                if index % 2 == 0 {
                    Direction::Outgoing
                } else {
                    Direction::Incoming
                },
                Venue::Direct,
                at_age(0, 10 * 3_600 + index * 60),
            )
        })
        .collect();
    Fixture {
        id: "F_BURST",
        primary_peer: "burst-peer",
        as_of: FIXTURE_AS_OF_UNIX,
        interactions,
    }
}

pub fn fixture_group() -> Fixture {
    let mut interactions = Vec::with_capacity(50);
    for age in 0..10 {
        for slot in 0..5 {
            let index = age * 5 + slot;
            interactions.push(Interaction::new(
                "group-peer",
                if index % 2 == 0 {
                    Direction::Outgoing
                } else {
                    Direction::Incoming
                },
                Venue::Group,
                at_age(age, 10 * 3_600 + slot * 60),
            ));
        }
    }
    Fixture {
        id: "F_GROUP",
        primary_peer: "group-peer",
        as_of: FIXTURE_AS_OF_UNIX,
        interactions,
    }
}

pub fn fixture_old() -> Fixture {
    let mut interactions = Vec::with_capacity(12);
    for offset in 0..6 {
        let age = 2_800 + offset;
        interactions.push(Interaction::new(
            "old-peer",
            Direction::Outgoing,
            Venue::Direct,
            at_age(age, 8 * 3_600),
        ));
        interactions.push(Interaction::new(
            "old-peer",
            Direction::Incoming,
            Venue::Direct,
            at_age(age, 8 * 3_600 + 5 * 60),
        ));
    }
    Fixture {
        id: "F_OLD",
        primary_peer: "old-peer",
        as_of: FIXTURE_AS_OF_UNIX,
        interactions,
    }
}

pub fn fixture_oneside() -> Fixture {
    let interactions = (0..100)
        .map(|index| {
            Interaction::new(
                "oneside-peer",
                Direction::Outgoing,
                Venue::Direct,
                at_age(index % 20, 9 * 3_600 + (index / 20) * 60),
            )
        })
        .collect();
    Fixture {
        id: "F_ONESIDE",
        primary_peer: "oneside-peer",
        as_of: FIXTURE_AS_OF_UNIX,
        interactions,
    }
}

pub fn fixture_dormant_200d() -> Fixture {
    let mut interactions = Vec::with_capacity(12);
    for offset in 0..6 {
        let age = 200 + offset;
        interactions.push(Interaction::new(
            "dormant-peer",
            Direction::Outgoing,
            Venue::Direct,
            at_age(age, 9 * 3_600),
        ));
        interactions.push(Interaction::new(
            "dormant-peer",
            Direction::Incoming,
            Venue::Direct,
            at_age(age, 9 * 3_600 + 5 * 60),
        ));
    }
    Fixture {
        id: "F_DORMANT_200d",
        primary_peer: "dormant-peer",
        as_of: FIXTURE_AS_OF_UNIX,
        interactions,
    }
}

pub fn fixture_heavy_group_plus_two_directs() -> Fixture {
    let mut interactions = Vec::with_capacity(38);
    for age in 0..3 {
        for slot in 0..12 {
            interactions.push(Interaction::new(
                "heavy-group-peer",
                if slot % 2 == 0 {
                    Direction::Outgoing
                } else {
                    Direction::Incoming
                },
                Venue::Group,
                at_age(age, 8 * 3_600 + slot * 60),
            ));
        }
    }
    interactions.push(Interaction::new(
        "heavy-group-peer",
        Direction::Outgoing,
        Venue::Direct,
        at_age(2, 11 * 3_600),
    ));
    interactions.push(Interaction::new(
        "heavy-group-peer",
        Direction::Incoming,
        Venue::Direct,
        at_age(0, 11 * 3_600 + 5 * 60),
    ));
    Fixture {
        id: "F_HEAVY_GROUP_PLUS_TWO_DIRECTS",
        primary_peer: "heavy-group-peer",
        as_of: FIXTURE_AS_OF_UNIX,
        interactions,
    }
}

pub fn ablation_fixtures() -> Vec<Fixture> {
    vec![
        fixture_lilei(),
        fixture_burst(),
        fixture_group(),
        fixture_old(),
        fixture_oneside(),
        fixture_dormant_200d(),
        fixture_heavy_group_plus_two_directs(),
    ]
}

pub fn fixture_scale() -> Fixture {
    let mut interactions = Vec::with_capacity(10_000);
    for peer in 0..200 {
        let peer_id = format!("peer-{peer:03}");
        for index in 0..50 {
            interactions.push(Interaction::new(
                peer_id.clone(),
                if index % 2 == 0 {
                    Direction::Outgoing
                } else {
                    Direction::Incoming
                },
                Venue::Direct,
                at_age(index % 10, 8 * 3_600 + (index / 10) * 60),
            ));
        }
    }
    Fixture {
        id: "F_SCALE",
        primary_peer: "peer-000",
        as_of: FIXTURE_AS_OF_UNIX,
        interactions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn score_fixture(fixture: &Fixture) -> (TieScore, TieScore) {
        (
            score_t4(&fixture.interactions, fixture.primary_peer, fixture.as_of),
            score_t4d(&fixture.interactions, fixture.primary_peer, fixture.as_of),
        )
    }

    fn direct_fixture(
        id: &'static str,
        peer: &'static str,
        count: usize,
        day_count: i64,
        last_age: i64,
    ) -> Fixture {
        let interactions = (0..count)
            .map(|index| {
                Interaction::new(
                    peer,
                    if index % 2 == 0 {
                        Direction::Outgoing
                    } else {
                        Direction::Incoming
                    },
                    Venue::Direct,
                    at_age(
                        last_age + index as i64 % day_count,
                        9 * 3_600 + index as i64,
                    ),
                )
            })
            .collect();
        Fixture {
            id,
            primary_peer: peer,
            as_of: FIXTURE_AS_OF_UNIX,
            interactions,
        }
    }

    #[test]
    fn computes_fixture_as_of_and_euclidean_utc_days() {
        assert_eq!(civil_to_unix(1970, 1, 1, 0, 0, 0), 0);
        assert_eq!(FIXTURE_AS_OF_UNIX, 1_787_580_000);
        assert_eq!(utc_day(86_399), 0);
        assert_eq!(utc_day(-1), -1);
        assert_eq!(utc_day(-86_401), -2);
    }

    #[test]
    fn required_fixture_matrix_matches_spec() {
        let expected = [
            ("F_LILEI", Band::Strong, Band::Strong),
            ("F_BURST", Band::Moderate, Band::Moderate),
            ("F_GROUP", Band::Moderate, Band::Weak),
            ("F_OLD", Band::Weak, Band::Weak),
            ("F_ONESIDE", Band::Weak, Band::Weak),
            ("F_DORMANT_200d", Band::Moderate, Band::Moderate),
            ("F_HEAVY_GROUP_PLUS_TWO_DIRECTS", Band::Strong, Band::Weak),
        ];
        let fixtures = ablation_fixtures();
        assert_eq!(fixtures.len(), expected.len());
        for (fixture, (id, t4, t4d)) in fixtures.iter().zip(expected) {
            let scores = score_fixture(fixture);
            assert_eq!(fixture.id, id);
            assert_eq!(scores.0.band, t4, "{id} T4");
            assert_eq!(scores.1.band, t4d, "{id} T4D");
        }
    }

    #[test]
    fn t4_dormancy_boundaries_are_closed() {
        for (age, expected) in [
            (179, Band::Strong),
            (180, Band::Moderate),
            (359, Band::Moderate),
            (360, Band::Weak),
        ] {
            let fixture = direct_fixture("boundary", "boundary", 12, 3, age);
            assert_eq!(score_fixture(&fixture).0.band, expected, "T4 age={age}");
            assert_eq!(score_fixture(&fixture).1.band, expected, "T4D age={age}");
        }
    }

    #[test]
    fn group_only_is_never_t4_strong_and_has_no_t4d_gate_credit() {
        let fixture = fixture_group();
        let (t4, t4d) = score_fixture(&fixture);
        assert_eq!(t4.interaction_count, 50);
        assert_eq!(t4.active_day_count, 10);
        assert_eq!(t4.direct_count, 0);
        assert_eq!(t4.band, Band::Moderate);
        assert_eq!(t4d.band, Band::Weak);
    }

    #[test]
    fn heavy_group_fixture_exposes_t4_any_direct_latch() {
        let fixture = fixture_heavy_group_plus_two_directs();
        let (t4, t4d) = score_fixture(&fixture);
        assert_eq!(t4.interaction_count, 38);
        assert_eq!(t4.active_day_count, 3);
        assert_eq!(t4.direct_count, 2);
        assert_eq!(t4.direct_outgoing_count, 1);
        assert_eq!(t4.direct_incoming_count, 1);
        assert_eq!(t4.direct_day_count, 2);
        assert_eq!(t4.band, Band::Strong);
        assert_eq!(t4d.band, Band::Weak);
    }

    #[test]
    fn t4d_thresholds_use_direct_count_and_direct_days() {
        let two = direct_fixture("two", "two", 2, 1, 0);
        let three = direct_fixture("three", "three", 3, 1, 0);
        let ten_two_days = direct_fixture("ten2", "ten2", 10, 2, 0);
        let ten_three_days = direct_fixture("ten3", "ten3", 10, 3, 0);
        assert_eq!(score_fixture(&two).1.band, Band::Weak);
        assert_eq!(score_fixture(&three).1.band, Band::Moderate);
        assert_eq!(score_fixture(&ten_two_days).1.band, Band::Moderate);
        assert_eq!(score_fixture(&ten_three_days).1.band, Band::Strong);
    }

    #[test]
    fn t4d_reciprocity_is_direct_not_all_venue() {
        let mut interactions: Vec<_> = (0..10)
            .map(|index| {
                Interaction::new(
                    "peer",
                    Direction::Outgoing,
                    Venue::Direct,
                    at_age(index % 3, 9 * 3_600 + index),
                )
            })
            .collect();
        interactions.push(Interaction::new(
            "peer",
            Direction::Incoming,
            Venue::Group,
            at_age(0, 10 * 3_600),
        ));
        assert_eq!(
            score_t4(&interactions, "peer", FIXTURE_AS_OF_UNIX).band,
            Band::Strong
        );
        assert_eq!(
            score_t4d(&interactions, "peer", FIXTURE_AS_OF_UNIX).band,
            Band::Weak
        );
    }

    #[test]
    fn t4d_demotion_uses_latest_event_from_any_venue() {
        let mut fixture = direct_fixture("any-venue", "any-venue", 12, 3, 400);
        fixture.interactions.push(Interaction::new(
            "any-venue",
            Direction::Incoming,
            Venue::Group,
            at_age(0, 12 * 3_600),
        ));
        let score = score_fixture(&fixture).1;
        assert_eq!(score.direct_count, 12);
        assert_eq!(score.last_contact_age_days, Some(0));
        assert_eq!(score.band, Band::Strong);
    }

    #[test]
    fn one_sided_direct_volume_remains_weak() {
        let fixture = fixture_oneside();
        let (t4, t4d) = score_fixture(&fixture);
        assert_eq!(t4.direct_count, 100);
        assert_eq!(t4.direct_incoming_count, 0);
        assert_eq!(t4.band, Band::Weak);
        assert_eq!(t4d.band, Band::Weak);
    }

    #[test]
    fn as_of_is_one_caller_supplied_value() {
        let fixture = fixture_lilei();
        assert_eq!(score_fixture(&fixture).1.band, Band::Strong);
        let later_as_of = fixture.as_of + WEAK_DAYS * SECONDS_PER_DAY;
        assert_eq!(
            score_t4d(&fixture.interactions, fixture.primary_peer, later_as_of).band,
            Band::Weak
        );
    }

    #[test]
    fn scale_has_exactly_ten_thousand_inputs_and_is_peer_isolated() {
        let fixture = fixture_scale();
        let t0 = score_all_t0(&fixture.interactions, fixture.as_of);
        let t4d = score_all_t4d(&fixture.interactions, fixture.as_of);
        assert_eq!(fixture.interactions.len(), 10_000);
        assert_eq!(t0.len(), 200);
        assert_eq!(t4d.len(), 200);
        assert!(t4d.values().all(|score| score.band == Band::Strong));
        assert_eq!(
            score_t4d(&fixture.interactions, "missing", fixture.as_of).band,
            Band::Weak
        );
    }
}
