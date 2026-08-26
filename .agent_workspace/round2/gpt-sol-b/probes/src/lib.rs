//! Independent, dependency-free probes for the T3, T3R, and T4 tie rules.
//!
//! The caller supplies `as_of_unix`; none of these functions read a wall clock.

use std::collections::BTreeSet;

pub const MODERATE_MIN_INTERACTIONS: u64 = 3;
pub const STRONG_MIN_INTERACTIONS: u64 = 10;
pub const STRONG_MIN_ACTIVE_DAYS: u64 = 3;
pub const T3R_HALF_LIFE_DAYS: u64 = 90;
pub const T3R_CUTOFF_DAYS: u64 = 360;
pub const T4_DOWNGRADE_AFTER_DAYS: u64 = 180;
pub const T4_WEAK_AT_DAYS: u64 = 360;

const MILLI_PER_INTERACTION: u64 = 1_000;
const SECONDS_PER_DAY: u64 = 86_400;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Outgoing,
    Incoming,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Venue {
    Direct,
    Group,
}

/// Metadata-only input for all three rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interaction {
    pub evidence_id: u64,
    pub conversation_id: u64,
    pub peer_id: u64,
    pub direction: Direction,
    pub venue: Venue,
    pub occurred_at_unix: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    Weak,
    Moderate,
    Strong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    T3,
    T3R,
    T4,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TieScore {
    pub peer_id: u64,
    pub band: Band,
    pub observed_interaction_count: u64,
    pub interaction_count: u64,
    pub weighted_interaction_milli: u64,
    pub outgoing_count: u64,
    pub incoming_count: u64,
    pub conversation_count: u64,
    pub active_day_count: u64,
    pub first_contact_unix: i64,
    pub last_contact_unix: i64,
    pub last_contact_age_days: u64,
    pub any_direct: bool,
    pub evidence_ids: Vec<u64>,
}

#[derive(Debug)]
struct Tally {
    observed_interaction_count: u64,
    interaction_count: u64,
    weighted_interaction_milli: u64,
    outgoing_count: u64,
    incoming_count: u64,
    conversations: BTreeSet<u64>,
    active_days: BTreeSet<i64>,
    first_contact_unix: Option<i64>,
    last_contact_unix: Option<i64>,
    any_direct: bool,
    evidence_ids: BTreeSet<u64>,
}

impl Tally {
    fn new() -> Self {
        Self {
            observed_interaction_count: 0,
            interaction_count: 0,
            weighted_interaction_milli: 0,
            outgoing_count: 0,
            incoming_count: 0,
            conversations: BTreeSet::new(),
            active_days: BTreeSet::new(),
            first_contact_unix: None,
            last_contact_unix: None,
            any_direct: false,
            evidence_ids: BTreeSet::new(),
        }
    }

    fn observe(&mut self, interaction: &Interaction, weight_milli: u64) {
        self.observed_interaction_count = self.observed_interaction_count.saturating_add(1);
        self.evidence_ids.insert(interaction.evidence_id);
        self.first_contact_unix = Some(
            self.first_contact_unix
                .map_or(interaction.occurred_at_unix, |current| {
                    current.min(interaction.occurred_at_unix)
                }),
        );
        self.last_contact_unix = Some(
            self.last_contact_unix
                .map_or(interaction.occurred_at_unix, |current| {
                    current.max(interaction.occurred_at_unix)
                }),
        );

        if weight_milli == 0 {
            return;
        }

        self.interaction_count = self.interaction_count.saturating_add(1);
        self.weighted_interaction_milli =
            self.weighted_interaction_milli.saturating_add(weight_milli);
        match interaction.direction {
            Direction::Outgoing => {
                self.outgoing_count = self.outgoing_count.saturating_add(1);
            }
            Direction::Incoming => {
                self.incoming_count = self.incoming_count.saturating_add(1);
            }
        }
        self.conversations.insert(interaction.conversation_id);
        self.active_days
            .insert(utc_day_floor(interaction.occurred_at_unix));
        self.any_direct |= interaction.venue == Venue::Direct;
    }

    fn reciprocal(&self) -> bool {
        self.outgoing_count > 0 && self.incoming_count > 0
    }

    fn active_day_count(&self) -> u64 {
        u64::try_from(self.active_days.len()).unwrap_or(u64::MAX)
    }

    fn into_result(self, peer_id: u64, band: Band, as_of_unix: i64) -> Option<TieScore> {
        let (Some(first_contact_unix), Some(last_contact_unix)) =
            (self.first_contact_unix, self.last_contact_unix)
        else {
            return None;
        };

        Some(TieScore {
            peer_id,
            band,
            observed_interaction_count: self.observed_interaction_count,
            interaction_count: self.interaction_count,
            weighted_interaction_milli: self.weighted_interaction_milli,
            outgoing_count: self.outgoing_count,
            incoming_count: self.incoming_count,
            conversation_count: u64::try_from(self.conversations.len()).unwrap_or(u64::MAX),
            active_day_count: self.active_day_count(),
            first_contact_unix,
            last_contact_unix,
            last_contact_age_days: elapsed_seconds(as_of_unix, last_contact_unix) / SECONDS_PER_DAY,
            any_direct: self.any_direct,
            evidence_ids: self.evidence_ids.into_iter().collect(),
        })
    }
}

/// T3 preserves the count thresholds while requiring reciprocity, direct
/// contact, and at least three UTC days for Strong. Group-only ties are capped
/// at Moderate.
pub fn score_t3(peer_id: u64, interactions: &[Interaction], as_of_unix: i64) -> Option<TieScore> {
    let tally = tally(peer_id, interactions, as_of_unix, Weighting::Full);
    let band = classify_t3(&tally);
    tally.into_result(peer_id, band, as_of_unix)
}

/// T3R applies T3's latches to an integer-milli, 90-day bucketed count:
/// 1000 before day 90, 500 before day 180, 250 before day 360, then zero.
pub fn score_t3r(peer_id: u64, interactions: &[Interaction], as_of_unix: i64) -> Option<TieScore> {
    let tally = tally(peer_id, interactions, as_of_unix, Weighting::Bucketed);
    let band = classify_t3(&tally);
    tally.into_result(peer_id, band, as_of_unix)
}

/// T4 starts from T3, lowers one band when the latest contact is more than
/// 180 days old, and caps the result at Weak from 360 days onward.
pub fn score_t4(peer_id: u64, interactions: &[Interaction], as_of_unix: i64) -> Option<TieScore> {
    let tally = tally(peer_id, interactions, as_of_unix, Weighting::Full);
    let mut band = classify_t3(&tally);
    let last_contact_unix = tally.last_contact_unix?;
    let age_seconds = elapsed_seconds(as_of_unix, last_contact_unix);

    if age_seconds >= T4_WEAK_AT_DAYS.saturating_mul(SECONDS_PER_DAY) {
        band = Band::Weak;
    } else if age_seconds > T4_DOWNGRADE_AFTER_DAYS.saturating_mul(SECONDS_PER_DAY) {
        band = downgrade_one(band);
    }

    tally.into_result(peer_id, band, as_of_unix)
}

pub fn score(
    algorithm: Algorithm,
    peer_id: u64,
    interactions: &[Interaction],
    as_of_unix: i64,
) -> Option<TieScore> {
    match algorithm {
        Algorithm::T3 => score_t3(peer_id, interactions, as_of_unix),
        Algorithm::T3R => score_t3r(peer_id, interactions, as_of_unix),
        Algorithm::T4 => score_t4(peer_id, interactions, as_of_unix),
    }
}

/// Explain only observable counts, UTC days, directionality, venue, and age.
pub fn explain(algorithm: Algorithm, result: &TieScore) -> String {
    let direction = if result.outgoing_count > 0 && result.incoming_count > 0 {
        "双方有往来"
    } else {
        "仅单向往来"
    };
    let venue = if result.any_direct {
        "含私聊"
    } else {
        "仅群聊"
    };
    let band = match result.band {
        Band::Weak => "弱",
        Band::Moderate => "中",
        Band::Strong => "强",
    };

    match algorithm {
        Algorithm::T3 => format!(
            "编号{}：T3；计入事件{}次；{}；UTC活跃{}天；{}；档位{}。",
            result.peer_id,
            result.interaction_count,
            direction,
            result.active_day_count,
            venue,
            band
        ),
        Algorithm::T3R => format!(
            "编号{}：T3R；计入事件{}次，90天分桶折算{}；{}；UTC活跃{}天；{}；档位{}。",
            result.peer_id,
            result.interaction_count,
            format_milli_count(result.weighted_interaction_milli),
            direction,
            result.active_day_count,
            venue,
            band
        ),
        Algorithm::T4 => format!(
            "编号{}：T4；计入事件{}次；{}；UTC活跃{}天；{}；最近往来距参考时点{}天；档位{}。",
            result.peer_id,
            result.interaction_count,
            direction,
            result.active_day_count,
            venue,
            result.last_contact_age_days,
            band
        ),
    }
}

#[derive(Debug, Clone, Copy)]
enum Weighting {
    Full,
    Bucketed,
}

fn tally(
    peer_id: u64,
    interactions: &[Interaction],
    as_of_unix: i64,
    weighting: Weighting,
) -> Tally {
    let mut tally = Tally::new();
    for interaction in interactions
        .iter()
        .filter(|interaction| interaction.peer_id == peer_id)
    {
        let weight_milli = match weighting {
            Weighting::Full => MILLI_PER_INTERACTION,
            Weighting::Bucketed => {
                t3r_weight_milli(elapsed_seconds(as_of_unix, interaction.occurred_at_unix))
            }
        };
        tally.observe(interaction, weight_milli);
    }
    tally
}

fn classify_t3(tally: &Tally) -> Band {
    let moderate_threshold = MODERATE_MIN_INTERACTIONS.saturating_mul(MILLI_PER_INTERACTION);
    let strong_threshold = STRONG_MIN_INTERACTIONS.saturating_mul(MILLI_PER_INTERACTION);

    if tally.reciprocal()
        && tally.any_direct
        && tally.weighted_interaction_milli >= strong_threshold
        && tally.active_day_count() >= STRONG_MIN_ACTIVE_DAYS
    {
        Band::Strong
    } else if tally.reciprocal() && tally.weighted_interaction_milli >= moderate_threshold {
        Band::Moderate
    } else {
        Band::Weak
    }
}

fn t3r_weight_milli(age_seconds: u64) -> u64 {
    let age_days = age_seconds / SECONDS_PER_DAY;
    if age_days < T3R_HALF_LIFE_DAYS {
        1_000
    } else if age_days < T3R_HALF_LIFE_DAYS.saturating_mul(2) {
        500
    } else if age_days < T3R_CUTOFF_DAYS {
        250
    } else {
        0
    }
}

fn downgrade_one(band: Band) -> Band {
    match band {
        Band::Strong => Band::Moderate,
        Band::Moderate | Band::Weak => Band::Weak,
    }
}

fn elapsed_seconds(as_of_unix: i64, event_unix: i64) -> u64 {
    let difference = i128::from(as_of_unix) - i128::from(event_unix);
    if difference <= 0 {
        0
    } else {
        u64::try_from(difference).unwrap_or(u64::MAX)
    }
}

fn utc_day_floor(occurred_at_unix: i64) -> i64 {
    occurred_at_unix.div_euclid(SECONDS_PER_DAY as i64)
}

fn format_milli_count(value: u64) -> String {
    format!("{}.{:03}次", value / 1_000, value % 1_000)
}
