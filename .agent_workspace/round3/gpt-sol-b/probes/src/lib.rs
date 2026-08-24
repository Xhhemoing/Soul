//! Independent Round 3 implementation of T4 and its direct-only T4D variant.
//!
//! The scorer consumes only event metadata. It does not inspect or retain
//! message content, and it never reads the wall clock.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

pub const DAY_SECONDS: i64 = 86_400;
pub const MODERATE_MIN_INTERACTIONS: usize = 3;
pub const STRONG_MIN_INTERACTIONS: usize = 10;
pub const STRONG_MIN_ACTIVE_DAYS: usize = 3;
pub const DEMOTE_ONE_BAND_DAYS: u64 = 180;
pub const FORCE_WEAK_DAYS: u64 = 360;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Incoming,
    Outgoing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Venue {
    Direct,
    Group,
}

/// The complete input record: time, direction, and venue metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Interaction {
    pub occurred_at: i64,
    pub direction: Direction,
    pub venue: Venue,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
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

    const fn demote_one(self) -> Self {
        match self {
            Self::Strong => Self::Moderate,
            Self::Moderate | Self::Weak => Self::Weak,
        }
    }
}

/// Auditable counters returned with the classification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Score {
    pub band: Band,
    pub total_count: usize,
    pub total_active_days: usize,
    pub direct_count: usize,
    pub direct_active_days: usize,
    pub age_days: Option<u64>,
    pub resolved_as_of: Option<i64>,
}

#[derive(Clone, Copy)]
enum Candidate {
    T4,
    T4D,
}

#[derive(Default)]
struct Tally {
    count: usize,
    active_days: BTreeSet<i64>,
    has_incoming: bool,
    has_outgoing: bool,
    latest: Option<i64>,
}

impl Tally {
    fn observe(&mut self, interaction: Interaction) {
        self.count += 1;
        self.active_days
            .insert(interaction.occurred_at.div_euclid(DAY_SECONDS));
        match interaction.direction {
            Direction::Incoming => self.has_incoming = true,
            Direction::Outgoing => self.has_outgoing = true,
        }
        self.latest = Some(self.latest.map_or(interaction.occurred_at, |old| {
            old.max(interaction.occurred_at)
        }));
    }

    fn is_reciprocal(&self) -> bool {
        self.has_incoming && self.has_outgoing
    }

    fn active_day_count(&self) -> usize {
        self.active_days.len()
    }
}

/// Score T4. `as_of` must be the one value selected for the whole rebuild.
///
/// `None` resolves to the maximum event time in the supplied slice. That is
/// safe only when the slice represents the whole store; it is deliberately
/// exposed so the peer-local fallback trap can be regression-tested.
pub fn t4(interactions: &[Interaction], as_of: Option<i64>) -> Score {
    score(Candidate::T4, interactions, as_of)
}

/// Score T4D. Counts, reciprocity, active days, and recency are all computed
/// from direct events; group events remain visible in the total counters.
pub fn t4d(interactions: &[Interaction], as_of: Option<i64>) -> Score {
    score(Candidate::T4D, interactions, as_of)
}

fn score(candidate: Candidate, interactions: &[Interaction], as_of: Option<i64>) -> Score {
    let mut total = Tally::default();
    let mut direct = Tally::default();

    for &interaction in interactions {
        total.observe(interaction);
        if interaction.venue == Venue::Direct {
            direct.observe(interaction);
        }
    }

    let resolved_as_of = as_of.or(total.latest);
    let (base_band, latest) = match candidate {
        Candidate::T4 => (t4_base(&total, direct.count > 0), total.latest),
        Candidate::T4D => (t4d_base(&direct), direct.latest),
    };
    let age_days = resolved_as_of.zip(latest).map(|(now, last)| {
        let elapsed = i128::from(now) - i128::from(last);
        elapsed.max(0).div_euclid(i128::from(DAY_SECONDS)) as u64
    });
    let band = apply_recency(base_band, age_days);

    Score {
        band,
        total_count: total.count,
        total_active_days: total.active_day_count(),
        direct_count: direct.count,
        direct_active_days: direct.active_day_count(),
        age_days,
        resolved_as_of,
    }
}

fn t4_base(total: &Tally, any_direct: bool) -> Band {
    if total.is_reciprocal()
        && any_direct
        && total.count >= STRONG_MIN_INTERACTIONS
        && total.active_day_count() >= STRONG_MIN_ACTIVE_DAYS
    {
        Band::Strong
    } else if total.is_reciprocal() && total.count >= MODERATE_MIN_INTERACTIONS {
        Band::Moderate
    } else {
        Band::Weak
    }
}

fn t4d_base(direct: &Tally) -> Band {
    if direct.is_reciprocal()
        && direct.count >= STRONG_MIN_INTERACTIONS
        && direct.active_day_count() >= STRONG_MIN_ACTIVE_DAYS
    {
        Band::Strong
    } else if direct.is_reciprocal() && direct.count >= MODERATE_MIN_INTERACTIONS {
        Band::Moderate
    } else {
        Band::Weak
    }
}

fn apply_recency(base: Band, age_days: Option<u64>) -> Band {
    match age_days {
        Some(age) if age >= FORCE_WEAK_DAYS => Band::Weak,
        Some(age) if age >= DEMOTE_ONE_BAND_DAYS => base.demote_one(),
        _ => base,
    }
}
