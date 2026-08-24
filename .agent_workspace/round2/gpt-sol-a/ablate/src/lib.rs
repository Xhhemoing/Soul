//! Dependency-free Round 2 ablation of T3, T3R, and T4.
//!
//! This crate is an independent implementation of the frozen rules. It does
//! not import code from any Round 1 candidate.

use std::collections::{BTreeMap, BTreeSet};

pub const SECONDS_PER_DAY: i64 = 86_400;
pub const MODERATE_MIN: u64 = 3;
pub const STRONG_MIN: u64 = 10;
pub const STRONG_DAYS: u64 = 3;
pub const FULL_WEIGHT_MILLI: u64 = 4;
pub const MODERATE_MILLI: u64 = MODERATE_MIN * FULL_WEIGHT_MILLI;
pub const STRONG_MILLI: u64 = STRONG_MIN * FULL_WEIGHT_MILLI;
pub const STRONG_DAY_MILLI: u64 = STRONG_DAYS * FULL_WEIGHT_MILLI;

/// Convert a valid proleptic-Gregorian civil time to Unix seconds.
///
/// The fixture constant below is computed through this function rather than
/// being copied from a precomputed timestamp.
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
    let day_of_era =
        year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let unix_day = era * 146_097 + day_of_era - 719_468;
    unix_day * SECONDS_PER_DAY
        + hour as i64 * 3_600
        + minute as i64 * 60
        + second as i64
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

/// T3R fixed-point weight in quarter-count units.
pub const fn weight_milli(age: i64) -> u64 {
    match age {
        0..=89 => 4,
        90..=179 => 2,
        180..=359 => 1,
        _ => 0,
    }
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
            Self::Weak => "weak",
            Self::Moderate => "moderate",
            Self::Strong => "strong",
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
    pub last_contact_unix: Option<i64>,
    /// Present only for T3R; 4 units equal one full interaction.
    pub event_milli: Option<u64>,
    /// Present only for T3R; each UTC day contributes at most one weight.
    pub day_milli: Option<u64>,
}

impl TieScore {
    pub fn empty() -> Self {
        Self {
            band: Band::Weak,
            interaction_count: 0,
            outgoing_count: 0,
            incoming_count: 0,
            active_day_count: 0,
            any_direct: false,
            last_contact_unix: None,
            event_milli: None,
            day_milli: None,
        }
    }
}

#[derive(Debug, Default)]
struct RawTally {
    outgoing: u64,
    incoming: u64,
    active_days: BTreeSet<i64>,
    any_direct: bool,
    last_contact_unix: Option<i64>,
}

impl RawTally {
    fn absorb(&mut self, interaction: &Interaction) {
        match interaction.direction {
            Direction::Outgoing => self.outgoing += 1,
            Direction::Incoming => self.incoming += 1,
        }
        self.active_days.insert(utc_day(interaction.occurred_at_unix));
        self.any_direct |= interaction.venue == Venue::Direct;
        if self
            .last_contact_unix
            .is_none_or(|last| interaction.occurred_at_unix > last)
        {
            self.last_contact_unix = Some(interaction.occurred_at_unix);
        }
    }

    fn reciprocal(&self) -> bool {
        self.outgoing >= 1 && self.incoming >= 1
    }

    fn interaction_count(&self) -> u64 {
        self.outgoing + self.incoming
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

    fn t3_band(&self) -> Band {
        if self.reciprocal()
            && self.any_direct
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

    fn finish(self, band: Band) -> TieScore {
        TieScore {
            band,
            interaction_count: self.interaction_count(),
            outgoing_count: self.outgoing,
            incoming_count: self.incoming,
            active_day_count: self.active_days.len() as u64,
            any_direct: self.any_direct,
            last_contact_unix: self.last_contact_unix,
            event_milli: None,
            day_milli: None,
        }
    }
}

#[derive(Debug, Default)]
struct WeightedTally {
    outgoing: u64,
    incoming: u64,
    active_day_weights: BTreeMap<i64, u64>,
    event_milli: u64,
    any_direct: bool,
    last_contact_unix: Option<i64>,
}

impl WeightedTally {
    fn absorb(&mut self, interaction: &Interaction, as_of: i64) {
        match interaction.direction {
            Direction::Outgoing => self.outgoing += 1,
            Direction::Incoming => self.incoming += 1,
        }
        self.any_direct |= interaction.venue == Venue::Direct;
        if self
            .last_contact_unix
            .is_none_or(|last| interaction.occurred_at_unix > last)
        {
            self.last_contact_unix = Some(interaction.occurred_at_unix);
        }

        let day = utc_day(interaction.occurred_at_unix);
        let weight = weight_milli(age_days(as_of, interaction.occurred_at_unix));
        self.event_milli += weight;
        self.active_day_weights
            .entry(day)
            .and_modify(|day_weight| *day_weight = (*day_weight).max(weight))
            .or_insert(weight);
    }

    fn finish(self) -> TieScore {
        let reciprocal = self.outgoing >= 1 && self.incoming >= 1;
        let day_milli = self.active_day_weights.values().sum();
        let band = if reciprocal
            && self.any_direct
            && self.event_milli >= STRONG_MILLI
            && day_milli >= STRONG_DAY_MILLI
        {
            Band::Strong
        } else if reciprocal && self.event_milli >= MODERATE_MILLI {
            Band::Moderate
        } else {
            Band::Weak
        };

        TieScore {
            band,
            interaction_count: self.outgoing + self.incoming,
            outgoing_count: self.outgoing,
            incoming_count: self.incoming,
            active_day_count: self.active_day_weights.len() as u64,
            any_direct: self.any_direct,
            last_contact_unix: self.last_contact_unix,
            event_milli: Some(self.event_milli),
            day_milli: Some(day_milli),
        }
    }
}

fn aggregate_raw(interactions: &[Interaction]) -> BTreeMap<String, RawTally> {
    let mut tallies = BTreeMap::<String, RawTally>::new();
    for interaction in interactions {
        tallies
            .entry(interaction.peer_id.clone())
            .or_default()
            .absorb(interaction);
    }
    tallies
}

/// T0 is included only as a performance baseline.
pub fn score_all_t0(interactions: &[Interaction], _as_of: i64) -> BTreeMap<String, TieScore> {
    aggregate_raw(interactions)
        .into_iter()
        .map(|(peer_id, tally)| {
            let band = tally.t0_band();
            (peer_id, tally.finish(band))
        })
        .collect()
}

pub fn score_all_t3(interactions: &[Interaction], _as_of: i64) -> BTreeMap<String, TieScore> {
    aggregate_raw(interactions)
        .into_iter()
        .map(|(peer_id, tally)| {
            let band = tally.t3_band();
            (peer_id, tally.finish(band))
        })
        .collect()
}

pub fn score_all_t3r(interactions: &[Interaction], as_of: i64) -> BTreeMap<String, TieScore> {
    let mut tallies = BTreeMap::<String, WeightedTally>::new();
    for interaction in interactions {
        tallies
            .entry(interaction.peer_id.clone())
            .or_default()
            .absorb(interaction, as_of);
    }
    tallies
        .into_iter()
        .map(|(peer_id, tally)| (peer_id, tally.finish()))
        .collect()
}

pub fn score_all_t4(interactions: &[Interaction], as_of: i64) -> BTreeMap<String, TieScore> {
    aggregate_raw(interactions)
        .into_iter()
        .map(|(peer_id, tally)| {
            let mut band = tally.t3_band();
            if let Some(last_contact) = tally.last_contact_unix {
                let age = age_days(as_of, last_contact);
                if age >= 360 {
                    band = Band::Weak;
                } else if age >= 180 {
                    band = band.demote_one();
                }
            }
            (peer_id, tally.finish(band))
        })
        .collect()
}

fn score_peer(
    all_scores: BTreeMap<String, TieScore>,
    peer_id: &str,
    weighted: bool,
) -> TieScore {
    all_scores.get(peer_id).cloned().unwrap_or_else(|| {
        let mut empty = TieScore::empty();
        if weighted {
            empty.event_milli = Some(0);
            empty.day_milli = Some(0);
        }
        empty
    })
}

pub fn score_t0(interactions: &[Interaction], peer_id: &str, as_of: i64) -> TieScore {
    score_peer(score_all_t0(interactions, as_of), peer_id, false)
}

pub fn score_t3(interactions: &[Interaction], peer_id: &str, as_of: i64) -> TieScore {
    score_peer(score_all_t3(interactions, as_of), peer_id, false)
}

pub fn score_t3r(interactions: &[Interaction], peer_id: &str, as_of: i64) -> TieScore {
    score_peer(score_all_t3r(interactions, as_of), peer_id, true)
}

pub fn score_t4(interactions: &[Interaction], peer_id: &str, as_of: i64) -> TieScore {
    score_peer(score_all_t4(interactions, as_of), peer_id, false)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fixture {
    pub id: &'static str,
    pub primary_peer: &'static str,
    pub as_of: i64,
    pub interactions: Vec<Interaction>,
}

fn at(year: i64, month: u32, day: u32, hour: u32, minute: u32) -> i64 {
    civil_to_unix(year, month, day, hour, minute, 0)
}

pub fn fixture_lilei() -> Fixture {
    let mut interactions = Vec::with_capacity(12);
    for day in 19..=24 {
        interactions.push(Interaction::new(
            "lilei",
            Direction::Outgoing,
            Venue::Direct,
            at(2026, 8, day, 9, 0),
        ));
        interactions.push(Interaction::new(
            "lilei",
            Direction::Incoming,
            Venue::Direct,
            at(2026, 8, day, 9, 5),
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
                at(
                    2026,
                    8,
                    24,
                    10 + index as u32 / 10,
                    (index as u32 % 10) * 6,
                ),
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

pub fn fixture_old() -> Fixture {
    let mut interactions = Vec::with_capacity(12);
    for day in 10..=15 {
        interactions.push(Interaction::new(
            "old-peer",
            Direction::Outgoing,
            Venue::Direct,
            at(2019, 1, day, 8, 0),
        ));
        interactions.push(Interaction::new(
            "old-peer",
            Direction::Incoming,
            Venue::Direct,
            at(2019, 1, day, 8, 10),
        ));
    }
    Fixture {
        id: "F_OLD",
        primary_peer: "old-peer",
        as_of: FIXTURE_AS_OF_UNIX,
        interactions,
    }
}

pub fn fixture_group() -> Fixture {
    let mut interactions = Vec::with_capacity(50);
    for day in 1..=10 {
        for slot in 0..5 {
            let index = (day - 1) * 5 + slot;
            interactions.push(Interaction::new(
                "group-peer",
                if index % 2 == 0 {
                    Direction::Outgoing
                } else {
                    Direction::Incoming
                },
                Venue::Group,
                at(2026, 8, day, 12, slot as u32 * 5),
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

pub fn fixture_oneside() -> Fixture {
    let interactions = (0..100)
        .map(|index| {
            Interaction::new(
                "oneside-peer",
                Direction::Outgoing,
                Venue::Direct,
                at(
                    2026,
                    8,
                    (index % 20 + 1) as u32,
                    (index % 24) as u32,
                    0,
                ),
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

pub fn fixture_empty() -> Fixture {
    Fixture {
        id: "F_EMPTY",
        primary_peer: "empty-peer",
        as_of: FIXTURE_AS_OF_UNIX,
        interactions: Vec::new(),
    }
}

pub fn fixture_dormant_200d() -> Fixture {
    let mut interactions = Vec::with_capacity(12);
    let last_day = utc_day(FIXTURE_AS_OF_UNIX) - 200;
    for offset in 0..6 {
        let day = last_day - (5 - offset);
        interactions.push(Interaction::new(
            "dormant-peer",
            Direction::Outgoing,
            Venue::Direct,
            day * SECONDS_PER_DAY + 9 * 3_600,
        ));
        interactions.push(Interaction::new(
            "dormant-peer",
            Direction::Incoming,
            Venue::Direct,
            day * SECONDS_PER_DAY + 9 * 3_600 + 5 * 60,
        ));
    }
    Fixture {
        id: "F_DORMANT_200d",
        primary_peer: "dormant-peer",
        as_of: FIXTURE_AS_OF_UNIX,
        interactions,
    }
}

pub fn ablation_fixtures() -> Vec<Fixture> {
    vec![
        fixture_lilei(),
        fixture_burst(),
        fixture_old(),
        fixture_group(),
        fixture_oneside(),
        fixture_empty(),
        fixture_dormant_200d(),
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
                at(
                    2026,
                    8,
                    (index % 10 + 1) as u32,
                    (index / 10) as u32,
                    (peer % 60) as u32,
                ),
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

    fn bands(fixture: &Fixture) -> (Band, Band, Band) {
        (
            score_t3(
                &fixture.interactions,
                fixture.primary_peer,
                fixture.as_of,
            )
            .band,
            score_t3r(
                &fixture.interactions,
                fixture.primary_peer,
                fixture.as_of,
            )
            .band,
            score_t4(
                &fixture.interactions,
                fixture.primary_peer,
                fixture.as_of,
            )
            .band,
        )
    }

    fn strong_at_last_age(age: i64) -> Vec<Interaction> {
        let last_day = utc_day(FIXTURE_AS_OF_UNIX) - age;
        let mut interactions = Vec::with_capacity(12);
        for offset in 0..6 {
            let day = last_day - (5 - offset);
            interactions.push(Interaction::new(
                "boundary",
                Direction::Outgoing,
                Venue::Direct,
                day * SECONDS_PER_DAY + 9 * 3_600,
            ));
            interactions.push(Interaction::new(
                "boundary",
                Direction::Incoming,
                Venue::Direct,
                day * SECONDS_PER_DAY + 9 * 3_600 + 60,
            ));
        }
        interactions
    }

    #[test]
    fn computes_and_verifies_fixture_as_of() {
        assert_eq!(civil_to_unix(1970, 1, 1, 0, 0, 0), 0);
        assert_eq!(civil_to_unix(2000, 2, 29, 0, 0, 0) + SECONDS_PER_DAY,
            civil_to_unix(2000, 3, 1, 0, 0, 0));
        assert_eq!(FIXTURE_AS_OF_UNIX, 1_787_580_000);
    }

    #[test]
    fn utc_day_uses_euclidean_division() {
        assert_eq!(utc_day(0), 0);
        assert_eq!(utc_day(86_399), 0);
        assert_eq!(utc_day(-1), -1);
        assert_eq!(utc_day(-86_400), -1);
        assert_eq!(utc_day(-86_401), -2);
    }

    #[test]
    fn fixture_ablation_matrix_matches_spec() {
        let expected = [
            ("F_LILEI", (Band::Strong, Band::Strong, Band::Strong)),
            (
                "F_BURST",
                (Band::Moderate, Band::Moderate, Band::Moderate),
            ),
            ("F_OLD", (Band::Strong, Band::Weak, Band::Weak)),
            (
                "F_GROUP",
                (Band::Moderate, Band::Moderate, Band::Moderate),
            ),
            ("F_ONESIDE", (Band::Weak, Band::Weak, Band::Weak)),
            ("F_EMPTY", (Band::Weak, Band::Weak, Band::Weak)),
            (
                "F_DORMANT_200d",
                (Band::Strong, Band::Moderate, Band::Moderate),
            ),
        ];
        let fixtures = ablation_fixtures();
        assert_eq!(fixtures.len(), expected.len());
        for (fixture, (expected_id, expected_bands)) in fixtures.iter().zip(expected) {
            assert_eq!(fixture.id, expected_id);
            assert_eq!(bands(fixture), expected_bands, "{}", fixture.id);
        }
    }

    #[test]
    fn reciprocal_requires_at_least_one_event_in_each_direction() {
        let at = FIXTURE_AS_OF_UNIX;
        let mut interactions = vec![
            Interaction::new("peer", Direction::Outgoing, Venue::Direct, at),
            Interaction::new("peer", Direction::Outgoing, Venue::Direct, at),
        ];
        assert_eq!(score_t3(&interactions, "peer", at).band, Band::Weak);
        interactions.push(Interaction::new(
            "peer",
            Direction::Incoming,
            Venue::Direct,
            at,
        ));
        assert_eq!(score_t3(&interactions, "peer", at).band, Band::Moderate);
    }

    #[test]
    fn t3_group_only_is_never_strong() {
        let fixture = fixture_group();
        let score = score_t3(
            &fixture.interactions,
            fixture.primary_peer,
            fixture.as_of,
        );
        assert_eq!(score.interaction_count, 50);
        assert_eq!(score.active_day_count, 10);
        assert!(!score.any_direct);
        assert_eq!(score.band, Band::Moderate);
    }

    #[test]
    fn t3r_weight_buckets_have_exact_boundaries() {
        assert_eq!(weight_milli(-1), 0);
        assert_eq!(weight_milli(0), 4);
        assert_eq!(weight_milli(89), 4);
        assert_eq!(weight_milli(90), 2);
        assert_eq!(weight_milli(179), 2);
        assert_eq!(weight_milli(180), 1);
        assert_eq!(weight_milli(359), 1);
        assert_eq!(weight_milli(360), 0);
        assert_eq!(weight_milli(10_000), 0);
    }

    #[test]
    fn t3r_weights_each_distinct_day_once() {
        let as_of_day = utc_day(FIXTURE_AS_OF_UNIX);
        let ages = [0, 0, 90, 90, 180, 180, 360, 360];
        let interactions: Vec<_> = ages
            .into_iter()
            .enumerate()
            .map(|(index, age)| {
                Interaction::new(
                    "weighted",
                    if index % 2 == 0 {
                        Direction::Outgoing
                    } else {
                        Direction::Incoming
                    },
                    Venue::Direct,
                    (as_of_day - age) * SECONDS_PER_DAY + index as i64,
                )
            })
            .collect();
        let score = score_t3r(&interactions, "weighted", FIXTURE_AS_OF_UNIX);
        assert_eq!(score.event_milli, Some(14));
        assert_eq!(score.day_milli, Some(7));
        assert_eq!(score.active_day_count, 4);
        assert_eq!(score.band, Band::Moderate);
    }

    #[test]
    fn t3r_single_day_burst_fails_strong_day_gate() {
        let fixture = fixture_burst();
        let score = score_t3r(
            &fixture.interactions,
            fixture.primary_peer,
            fixture.as_of,
        );
        assert_eq!(score.event_milli, Some(80));
        assert_eq!(score.day_milli, Some(4));
        assert_eq!(score.band, Band::Moderate);
    }

    #[test]
    fn t3r_dormant_all_zero_is_weak() {
        let fixture = fixture_old();
        let score = score_t3r(
            &fixture.interactions,
            fixture.primary_peer,
            fixture.as_of,
        );
        assert_eq!(score.event_milli, Some(0));
        assert_eq!(score.day_milli, Some(0));
        assert_eq!(score.band, Band::Weak);
    }

    #[test]
    fn t3r_group_only_is_not_strong() {
        let fixture = fixture_group();
        let score = score_t3r(
            &fixture.interactions,
            fixture.primary_peer,
            fixture.as_of,
        );
        assert_eq!(score.event_milli, Some(200));
        assert_eq!(score.day_milli, Some(40));
        assert!(!score.any_direct);
        assert_eq!(score.band, Band::Moderate);
    }

    #[test]
    fn t4_age_boundaries_are_inclusive() {
        for (age, expected) in [
            (179, Band::Strong),
            (180, Band::Moderate),
            (359, Band::Moderate),
            (360, Band::Weak),
        ] {
            let interactions = strong_at_last_age(age);
            assert_eq!(
                score_t4(&interactions, "boundary", FIXTURE_AS_OF_UNIX).band,
                expected,
                "age={age}"
            );
        }
    }

    #[test]
    fn score_all_keeps_peers_isolated_and_scale_is_complete() {
        let scale = fixture_scale();
        let scores = score_all_t3r(&scale.interactions, scale.as_of);
        assert_eq!(scale.interactions.len(), 10_000);
        assert_eq!(scores.len(), 200);
        assert!(scores.values().all(|score| score.band == Band::Strong));

        let empty = score_t3r(&scale.interactions, "missing-peer", scale.as_of);
        assert_eq!(empty.band, Band::Weak);
        assert_eq!(empty.event_milli, Some(0));
        assert_eq!(empty.day_milli, Some(0));
    }
}
