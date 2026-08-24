//! Independent, dependency-free reproduction of Goal 1's T0 tie scoring.
//!
//! The implementation deliberately does not import any Soul workspace crate.

use std::collections::{BTreeMap, BTreeSet};

pub const MODERATE_MIN_INTERACTIONS: u64 = 3;
pub const STRONG_MIN_INTERACTIONS: u64 = 10;
pub const STRONG_MIN_ACTIVE_DAYS: u64 = 3;
pub const FIXTURE_NOW_UTC: &str = "2026-08-24T14:00:00Z";

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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interaction {
    pub peer_id: String,
    pub direction: Direction,
    pub venue: Venue,
    pub conversation_ref: String,
    pub occurred_at_utc: String,
}

impl Interaction {
    pub fn new(
        peer_id: impl Into<String>,
        direction: Direction,
        venue: Venue,
        conversation_ref: impl Into<String>,
        occurred_at_utc: impl Into<String>,
    ) -> Self {
        Self {
            peer_id: peer_id.into(),
            direction,
            venue,
            conversation_ref: conversation_ref.into(),
            occurred_at_utc: occurred_at_utc.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct T0Score {
    pub band: Band,
    pub interaction_count: u64,
    pub outgoing_count: u64,
    pub incoming_count: u64,
    pub conversation_count: u64,
    pub active_day_count: u64,
    pub first_contact_utc: Option<String>,
    pub last_contact_utc: Option<String>,
    pub has_direct: bool,
}

impl T0Score {
    fn empty() -> Self {
        Self {
            band: Band::Weak,
            interaction_count: 0,
            outgoing_count: 0,
            incoming_count: 0,
            conversation_count: 0,
            active_day_count: 0,
            first_contact_utc: None,
            last_contact_utc: None,
            has_direct: false,
        }
    }
}

#[derive(Debug, Default)]
struct Tally {
    outgoing: u64,
    incoming: u64,
    conversations: BTreeSet<String>,
    active_days: BTreeSet<String>,
    first_contact_utc: Option<String>,
    last_contact_utc: Option<String>,
    has_direct: bool,
}

impl Tally {
    fn absorb(&mut self, interaction: &Interaction) {
        match interaction.direction {
            Direction::Outgoing => self.outgoing += 1,
            Direction::Incoming => self.incoming += 1,
        }
        self.has_direct |= interaction.venue == Venue::Direct;
        self.conversations
            .insert(interaction.conversation_ref.clone());
        self.active_days
            .insert(utc_date(&interaction.occurred_at_utc));

        if self
            .first_contact_utc
            .as_ref()
            .is_none_or(|first| interaction.occurred_at_utc < *first)
        {
            self.first_contact_utc = Some(interaction.occurred_at_utc.clone());
        }
        if self
            .last_contact_utc
            .as_ref()
            .is_none_or(|last| interaction.occurred_at_utc > *last)
        {
            self.last_contact_utc = Some(interaction.occurred_at_utc.clone());
        }
    }

    fn finish(self) -> T0Score {
        let interaction_count = self.outgoing + self.incoming;
        let active_day_count = self.active_days.len() as u64;
        let reciprocal = self.outgoing > 0 && self.incoming > 0;
        let band = if reciprocal
            && interaction_count >= STRONG_MIN_INTERACTIONS
            && active_day_count >= STRONG_MIN_ACTIVE_DAYS
        {
            Band::Strong
        } else if reciprocal && interaction_count >= MODERATE_MIN_INTERACTIONS {
            Band::Moderate
        } else {
            Band::Weak
        };

        T0Score {
            band,
            interaction_count,
            outgoing_count: self.outgoing,
            incoming_count: self.incoming,
            conversation_count: self.conversations.len() as u64,
            active_day_count,
            first_contact_utc: self.first_contact_utc,
            last_contact_utc: self.last_contact_utc,
            has_direct: self.has_direct,
        }
    }
}

fn utc_date(timestamp: &str) -> String {
    timestamp.get(..10).unwrap_or(timestamp).to_owned()
}

/// Score one peer. Interactions for every other peer are ignored.
pub fn score_t0(interactions: &[Interaction], peer_id: &str) -> T0Score {
    let mut tally = Tally::default();
    let mut found = false;
    for interaction in interactions {
        if interaction.peer_id == peer_id {
            tally.absorb(interaction);
            found = true;
        }
    }
    if found {
        tally.finish()
    } else {
        T0Score::empty()
    }
}

/// Score every observed peer in one pass, matching graph_build.rs aggregation.
pub fn score_all_t0(interactions: &[Interaction]) -> BTreeMap<String, T0Score> {
    let mut tallies = BTreeMap::<String, Tally>::new();
    for interaction in interactions {
        tallies
            .entry(interaction.peer_id.clone())
            .or_default()
            .absorb(interaction);
    }
    tallies
        .into_iter()
        .map(|(peer_id, tally)| (peer_id, tally.finish()))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fixture {
    pub id: &'static str,
    pub now_utc: &'static str,
    pub primary_peer: &'static str,
    pub interactions: Vec<Interaction>,
    pub expected_t0_band: Band,
    pub notes: &'static str,
}

fn at(year: u16, month: u8, day: u8, hour: u8, minute: u8) -> String {
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:00Z")
}

pub fn fixture_lilei() -> Fixture {
    let mut interactions = Vec::with_capacity(12);
    for day in 19..=24 {
        let conversation = format!("lilei-{day:02}");
        interactions.push(Interaction::new(
            "lilei",
            Direction::Outgoing,
            Venue::Direct,
            conversation.clone(),
            at(2026, 8, day, 9, 0),
        ));
        interactions.push(Interaction::new(
            "lilei",
            Direction::Incoming,
            Venue::Direct,
            conversation,
            at(2026, 8, day, 9, 5),
        ));
    }
    Fixture {
        id: "F_LILEI",
        now_utc: FIXTURE_NOW_UTC,
        primary_peer: "lilei",
        interactions,
        expected_t0_band: Band::Strong,
        notes: "12 reciprocal direct messages over 6 UTC days; Goal 1 strong case",
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
                "burst-conversation",
                at(2026, 8, 24, 10 + index / 10, (index % 10) * 6),
            )
        })
        .collect();
    Fixture {
        id: "F_BURST",
        now_utc: FIXTURE_NOW_UTC,
        primary_peer: "burst-peer",
        interactions,
        expected_t0_band: Band::Moderate,
        notes: "20 reciprocal direct messages within 2 hours on one UTC day",
    }
}

pub fn fixture_old() -> Fixture {
    let mut interactions = Vec::with_capacity(12);
    for day in 10..=15 {
        let conversation = format!("old-{day:02}");
        interactions.push(Interaction::new(
            "old-peer",
            Direction::Outgoing,
            Venue::Direct,
            conversation.clone(),
            at(2019, 1, day, 8, 0),
        ));
        interactions.push(Interaction::new(
            "old-peer",
            Direction::Incoming,
            Venue::Direct,
            conversation,
            at(2019, 1, day, 8, 10),
        ));
    }
    Fixture {
        id: "F_OLD",
        now_utc: FIXTURE_NOW_UTC,
        primary_peer: "old-peer",
        interactions,
        expected_t0_band: Band::Strong,
        notes: "12 reciprocal direct messages over 6 days in Jan 2019; T0 has no recency",
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
                format!("group-{day:02}"),
                at(2026, 8, day, 12, slot * 5),
            ));
        }
    }
    Fixture {
        id: "F_GROUP",
        now_utc: FIXTURE_NOW_UTC,
        primary_peer: "group-peer",
        interactions,
        expected_t0_band: Band::Strong,
        notes: "50 reciprocal group messages over 10 UTC days; T0 does not downweight groups",
    }
}

pub fn fixture_oneside() -> Fixture {
    let interactions = (0..100)
        .map(|index| {
            Interaction::new(
                "oneside-peer",
                Direction::Outgoing,
                Venue::Direct,
                format!("oneside-{:02}", index % 20 + 1),
                at(2026, 8, (index % 20 + 1) as u8, (index % 24) as u8, 0),
            )
        })
        .collect();
    Fixture {
        id: "F_ONESIDE",
        now_utc: FIXTURE_NOW_UTC,
        primary_peer: "oneside-peer",
        interactions,
        expected_t0_band: Band::Weak,
        notes: "100 outbound direct messages and no inbound message",
    }
}

pub fn fixture_empty() -> Fixture {
    Fixture {
        id: "F_EMPTY",
        now_utc: FIXTURE_NOW_UTC,
        primary_peer: "empty-peer",
        interactions: Vec::new(),
        expected_t0_band: Band::Weak,
        notes: "No interactions",
    }
}

pub fn fixture_scale() -> Fixture {
    let mut interactions = Vec::with_capacity(10_000);
    for peer in 0..200 {
        let peer_id = format!("peer-{peer:03}");
        for index in 0..50 {
            let day = index % 10 + 1;
            interactions.push(Interaction::new(
                peer_id.clone(),
                if index % 2 == 0 {
                    Direction::Outgoing
                } else {
                    Direction::Incoming
                },
                Venue::Direct,
                format!("scale-{peer:03}-{day:02}"),
                at(2026, 8, day as u8, (index / 10) as u8, (peer % 60) as u8),
            ));
        }
    }
    Fixture {
        id: "F_SCALE",
        now_utc: FIXTURE_NOW_UTC,
        primary_peer: "peer-000",
        interactions,
        expected_t0_band: Band::Strong,
        notes: "10000 direct interactions across 200 peers; each peer is reciprocal and strong",
    }
}

pub fn all_fixtures() -> Vec<Fixture> {
    vec![
        fixture_lilei(),
        fixture_burst(),
        fixture_old(),
        fixture_group(),
        fixture_oneside(),
        fixture_empty(),
        fixture_scale(),
    ]
}

fn json_escape(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    escaped
}

pub fn fixture_metadata_jsonl(fixtures: &[Fixture]) -> String {
    let mut jsonl = String::new();
    for fixture in fixtures {
        jsonl.push_str(&format!(
            "{{\"id\":\"{}\",\"n_interactions\":{},\"expected_t0_band\":\"{}\",\"notes\":\"{}\"}}\n",
            json_escape(fixture.id),
            fixture.interactions.len(),
            fixture.expected_t0_band.as_str(),
            json_escape(fixture.notes),
        ));
    }
    jsonl
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interaction(peer_id: &str, direction: Direction, day: u8, index: usize) -> Interaction {
        Interaction::new(
            peer_id,
            direction,
            Venue::Direct,
            format!("{peer_id}-{day:02}"),
            at(2026, 8, day, (index % 24) as u8, (index % 60) as u8),
        )
    }

    #[test]
    fn fixtures_have_expected_t0_bands_and_sizes() {
        let fixtures = all_fixtures();
        let expected_sizes = [12, 20, 12, 50, 100, 0, 10_000];
        assert_eq!(fixtures.len(), expected_sizes.len());
        for (fixture, expected_size) in fixtures.iter().zip(expected_sizes) {
            assert_eq!(fixture.interactions.len(), expected_size, "{}", fixture.id);
            let score = score_t0(&fixture.interactions, fixture.primary_peer);
            assert_eq!(score.band, fixture.expected_t0_band, "{}", fixture.id);
        }

        let scale_scores = score_all_t0(&fixture_scale().interactions);
        assert_eq!(scale_scores.len(), 200);
        assert!(scale_scores
            .values()
            .all(|score| score.band == Band::Strong));

        let lilei = fixture_lilei();
        let lilei_score = score_t0(&lilei.interactions, lilei.primary_peer);
        assert_eq!(
            (lilei_score.outgoing_count, lilei_score.incoming_count),
            (6, 6)
        );
        assert_eq!(lilei_score.active_day_count, 6);
        assert!(lilei_score.has_direct);

        let burst = fixture_burst();
        assert_eq!(
            score_t0(&burst.interactions, burst.primary_peer).active_day_count,
            1
        );

        let group = fixture_group();
        let group_score = score_t0(&group.interactions, group.primary_peer);
        assert_eq!(
            (group_score.outgoing_count, group_score.incoming_count),
            (25, 25)
        );
        assert_eq!(group_score.active_day_count, 10);
        assert!(!group_score.has_direct);

        let oneside = fixture_oneside();
        let oneside_score = score_t0(&oneside.interactions, oneside.primary_peer);
        assert_eq!(
            (oneside_score.outgoing_count, oneside_score.incoming_count),
            (100, 0)
        );
    }

    #[test]
    fn t0_is_deterministic() {
        for fixture in all_fixtures() {
            assert_eq!(
                score_all_t0(&fixture.interactions),
                score_all_t0(&fixture.interactions),
                "{}",
                fixture.id
            );
            assert_eq!(
                score_t0(&fixture.interactions, fixture.primary_peer),
                score_t0(&fixture.interactions, fixture.primary_peer),
                "{}",
                fixture.id
            );
            let mut reversed = fixture.interactions.clone();
            reversed.reverse();
            assert_eq!(
                score_all_t0(&fixture.interactions),
                score_all_t0(&reversed),
                "{} after reversing input",
                fixture.id
            );
        }
    }

    #[test]
    fn t0_never_returns_strong_for_non_reciprocal_input() {
        for direction in [Direction::Outgoing, Direction::Incoming] {
            let mut interactions = Vec::new();
            for count in 0..=128 {
                if count > 0 {
                    interactions.push(interaction(
                        "peer",
                        direction,
                        (count % 31 + 1) as u8,
                        count,
                    ));
                }
                assert_ne!(
                    score_t0(&interactions, "peer").band,
                    Band::Strong,
                    "direction={direction:?}, count={count}"
                );
            }
        }
    }

    #[test]
    fn t0_never_returns_strong_with_fewer_than_three_active_days() {
        for active_days in 1..STRONG_MIN_ACTIVE_DAYS {
            let interactions: Vec<_> = (0..128)
                .map(|index| {
                    interaction(
                        "peer",
                        if index % 2 == 0 {
                            Direction::Outgoing
                        } else {
                            Direction::Incoming
                        },
                        (index as u64 % active_days + 1) as u8,
                        index,
                    )
                })
                .collect();
            let score = score_t0(&interactions, "peer");
            assert_eq!(score.active_day_count, active_days);
            assert_ne!(score.band, Band::Strong);
        }
    }

    #[test]
    fn adding_an_interaction_never_decreases_interaction_count() {
        let mut interactions = Vec::new();
        let additions = [
            interaction("peer-a", Direction::Outgoing, 1, 0),
            interaction("peer-b", Direction::Incoming, 1, 1),
            interaction("peer-a", Direction::Incoming, 2, 2),
            interaction("peer-a", Direction::Outgoing, 3, 3),
        ];

        for addition in additions {
            let before = score_t0(&interactions, "peer-a").interaction_count;
            interactions.push(addition);
            let after = score_t0(&interactions, "peer-a").interaction_count;
            assert!(after >= before, "before={before}, after={after}");
        }
    }

    #[test]
    fn scoring_peer_a_ignores_peer_b_messages() {
        let peer_a = fixture_lilei().interactions;
        let expected = score_t0(&peer_a, "lilei");
        let mut mixed = peer_a;
        mixed.extend(fixture_group().interactions);
        mixed.extend(fixture_oneside().interactions);
        assert_eq!(score_t0(&mixed, "lilei"), expected);
    }
}
