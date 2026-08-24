//! Dependency-free adversarial model of Soul's T0 and A0 rules.
//!
//! This is intentionally independent from the production crates. It models
//! only metadata that the two rules need, so a scoring caller cannot pass
//! message prose through [`Interaction`].

use std::collections::BTreeSet;

pub const MODERATE_MIN_INTERACTIONS: u64 = 3;
pub const STRONG_MIN_INTERACTIONS: u64 = 10;
pub const STRONG_MIN_ACTIVE_DAYS: u64 = 3;
const SECONDS_PER_DAY: i64 = 86_400;

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

/// Metadata-only input. There is deliberately no prose payload field.
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TieResult {
    pub peer_id: u64,
    pub band: Band,
    pub interaction_count: u64,
    pub outgoing_count: u64,
    pub incoming_count: u64,
    pub conversation_count: u64,
    pub active_day_count: u64,
    pub first_contact_unix: i64,
    pub last_contact_unix: i64,
    pub has_direct: bool,
    pub evidence_ids: Vec<u64>,
}

/// Reimplement T0: reciprocity + count + distinct UTC-day thresholds.
///
/// `now_unix` is accepted only to probe clock-skew behavior. T0 has no recency
/// term and therefore must not read it.
pub fn score_t0(
    peer_id: u64,
    interactions: &[Interaction],
    forgotten_evidence_ids: &BTreeSet<u64>,
    _now_unix: i64,
) -> Option<TieResult> {
    let mut outgoing = 0u64;
    let mut incoming = 0u64;
    let mut conversations = BTreeSet::new();
    let mut active_days = BTreeSet::new();
    let mut evidence_ids = BTreeSet::new();
    let mut first_contact = None;
    let mut last_contact = None;
    let mut has_direct = false;

    for interaction in interactions.iter().filter(|interaction| {
        interaction.peer_id == peer_id && !forgotten_evidence_ids.contains(&interaction.evidence_id)
    }) {
        match interaction.direction {
            Direction::Outgoing => outgoing = outgoing.saturating_add(1),
            Direction::Incoming => incoming = incoming.saturating_add(1),
        }
        conversations.insert(interaction.conversation_id);
        active_days.insert(utc_day_floor(interaction.occurred_at_unix));
        evidence_ids.insert(interaction.evidence_id);
        has_direct |= interaction.venue == Venue::Direct;
        first_contact = Some(
            first_contact.map_or(interaction.occurred_at_unix, |current: i64| {
                current.min(interaction.occurred_at_unix)
            }),
        );
        last_contact = Some(
            last_contact.map_or(interaction.occurred_at_unix, |current: i64| {
                current.max(interaction.occurred_at_unix)
            }),
        );
    }

    let (Some(first_contact_unix), Some(last_contact_unix)) = (first_contact, last_contact) else {
        return None;
    };
    let interaction_count = outgoing.saturating_add(incoming);
    let active_day_count = active_days.len() as u64;
    let reciprocal = outgoing > 0 && incoming > 0;
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

    Some(TieResult {
        peer_id,
        band,
        interaction_count,
        outgoing_count: outgoing,
        incoming_count: incoming,
        conversation_count: conversations.len() as u64,
        active_day_count,
        first_contact_unix,
        last_contact_unix,
        has_direct,
        evidence_ids: evidence_ids.into_iter().collect(),
    })
}

/// Explain only observable metadata; no contact display label is accepted.
pub fn explain_t0(result: &TieResult) -> String {
    let band = match result.band {
        Band::Weak => "弱",
        Band::Moderate => "中",
        Band::Strong => "强",
    };
    let venue = if result.has_direct {
        "含私聊"
    } else {
        "仅群聊"
    };
    format!(
        "编号{}：往来档为{}；双方事件{}次；UTC活跃日{}天；{}。",
        result.peer_id, band, result.interaction_count, result.active_day_count, venue
    )
}

/// Mathematical UTC day floor, including instants before the Unix epoch.
fn utc_day_floor(occurred_at_unix: i64) -> i64 {
    occurred_at_unix.div_euclid(SECONDS_PER_DAY)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisPosition {
    LeansLow,
    Mixed,
    LeansHigh,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxisState {
    pub position: AxisPosition,
    pub band: Band,
    pub evidence_ids: Vec<u64>,
    pub locked_by_user: bool,
}

impl Default for AxisState {
    fn default() -> Self {
        Self {
            position: AxisPosition::Unknown,
            band: Band::Weak,
            evidence_ids: Vec::new(),
            locked_by_user: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisUpdate {
    Applied,
    RefusedAxisLocked,
    RefusedNoLiveEvidence,
}

/// A0 user correction: replace support, set Strong, and lock the axis.
pub fn correct_axis(state: &mut AxisState, position: AxisPosition, evidence_id: u64) {
    state.position = position;
    state.band = Band::Strong;
    state.evidence_ids = vec![evidence_id];
    state.locked_by_user = true;
}

/// A0 inference: forgotten support is unusable and a correction always wins.
pub fn apply_axis_inference(
    state: &mut AxisState,
    position: AxisPosition,
    band: Band,
    evidence_ids: &[u64],
    forgotten_evidence_ids: &BTreeSet<u64>,
) -> AxisUpdate {
    let live_evidence: BTreeSet<u64> = evidence_ids
        .iter()
        .copied()
        .filter(|id| !forgotten_evidence_ids.contains(id))
        .collect();
    if live_evidence.is_empty() {
        return AxisUpdate::RefusedNoLiveEvidence;
    }
    if state.locked_by_user {
        return AxisUpdate::RefusedAxisLocked;
    }

    state.position = position;
    state.band = band;
    state.evidence_ids = live_evidence.into_iter().collect();
    AxisUpdate::Applied
}
