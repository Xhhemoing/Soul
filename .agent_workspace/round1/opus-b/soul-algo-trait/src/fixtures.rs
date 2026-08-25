//! Deterministic fixtures, shipped in the library rather than in one test
//! file so that every screen, every later round and any benchmark walks the
//! same cases.
//!
//! Nothing here reads a clock or a file. [`FIXTURE_NOW_UNIX`] is the "now"
//! every recency case is written against, so a fixture cannot rot into a
//! flaky test the week after it was added.

use crate::a2::PeerStats;
use crate::types::{AxisId, Band, EvidenceRef, Position};

/// The fixed "now" the peer fixtures are written against: 2025-10-09T07:33:20Z.
pub const FIXTURE_NOW_UNIX: i64 = 1_760_000_000;

/// Seconds in a day, so a fixture can say "eleven days ago" and mean it.
const DAY: i64 = 86_400;

/// Peer records covering the shapes A2 has to survive: nothing at all, one-way
/// traffic, a group-only peer, a thick reciprocal edge, a single-day burst, a
/// long-cold edge, and a timestamp from a disagreeing clock.
pub fn peer_stats_matrix() -> Vec<(&'static str, PeerStats)> {
    vec![
        ("empty", PeerStats::default()),
        (
            "counts_without_citable_evidence",
            PeerStats {
                interaction_count: 9,
                outgoing: 5,
                incoming: 4,
                active_days: 4,
                last_contact_unix: Some(FIXTURE_NOW_UNIX - DAY),
                venue_direct_seen: true,
                reciprocal: true,
                evidence_ids: Vec::new(),
                last_contact_evidence_id: None,
            },
        ),
        (
            "single_outgoing_message",
            PeerStats {
                interaction_count: 1,
                outgoing: 1,
                incoming: 0,
                active_days: 1,
                last_contact_unix: Some(FIXTURE_NOW_UNIX - 3 * DAY),
                venue_direct_seen: true,
                reciprocal: false,
                evidence_ids: vec![101],
                last_contact_evidence_id: Some(101),
            },
        ),
        (
            "one_way_incoming_only",
            PeerStats {
                interaction_count: 12,
                outgoing: 0,
                incoming: 12,
                active_days: 5,
                last_contact_unix: Some(FIXTURE_NOW_UNIX - 11 * DAY),
                venue_direct_seen: true,
                reciprocal: false,
                evidence_ids: vec![201, 202, 203],
                last_contact_evidence_id: Some(203),
            },
        ),
        (
            "group_only_reciprocal",
            PeerStats {
                interaction_count: 14,
                outgoing: 6,
                incoming: 8,
                active_days: 4,
                last_contact_unix: Some(FIXTURE_NOW_UNIX - 2 * DAY),
                venue_direct_seen: false,
                reciprocal: true,
                evidence_ids: vec![301, 302, 303, 304],
                last_contact_evidence_id: Some(304),
            },
        ),
        (
            "direct_reciprocal_thick",
            PeerStats {
                interaction_count: 240,
                outgoing: 130,
                incoming: 110,
                active_days: 61,
                last_contact_unix: Some(FIXTURE_NOW_UNIX),
                venue_direct_seen: true,
                reciprocal: true,
                evidence_ids: vec![401, 402, 403, 404, 405],
                last_contact_evidence_id: Some(405),
            },
        ),
        (
            "single_day_burst",
            PeerStats {
                interaction_count: 1_000,
                outgoing: 500,
                incoming: 500,
                active_days: 1,
                last_contact_unix: Some(FIXTURE_NOW_UNIX - DAY),
                venue_direct_seen: true,
                reciprocal: true,
                evidence_ids: vec![501, 502],
                last_contact_evidence_id: Some(502),
            },
        ),
        (
            "long_cold_edge",
            PeerStats {
                interaction_count: 18,
                outgoing: 9,
                incoming: 9,
                active_days: 7,
                last_contact_unix: Some(FIXTURE_NOW_UNIX - 400 * DAY),
                venue_direct_seen: true,
                reciprocal: true,
                evidence_ids: vec![601, 602, 603],
                last_contact_evidence_id: Some(603),
            },
        ),
        (
            "two_way_but_not_confirmed_reciprocal",
            PeerStats {
                interaction_count: 4,
                outgoing: 2,
                incoming: 2,
                active_days: 2,
                last_contact_unix: None,
                venue_direct_seen: true,
                reciprocal: false,
                evidence_ids: vec![701],
                last_contact_evidence_id: None,
            },
        ),
        (
            "clock_ahead_of_last_contact",
            PeerStats {
                interaction_count: 3,
                outgoing: 2,
                incoming: 1,
                active_days: 2,
                last_contact_unix: Some(FIXTURE_NOW_UNIX + 5 * DAY),
                venue_direct_seen: false,
                reciprocal: true,
                evidence_ids: vec![801],
                last_contact_evidence_id: Some(801),
            },
        ),
    ]
}

/// Evidence logs covering what A0 and A1 have to survive: nothing, a plain
/// questionnaire, a correction, inference after a lock, a contradiction, a
/// forget, and duplicate citations of one row.
pub fn axis_evidence_matrix() -> Vec<(&'static str, Vec<EvidenceRef>)> {
    let axis = AxisId::SocialEnergy;
    vec![
        ("empty", Vec::new()),
        (
            "questionnaire_only",
            vec![EvidenceRef::questionnaire(1, axis, Position::LeansHigh)],
        ),
        (
            "questionnaire_then_correction",
            vec![
                EvidenceRef::questionnaire(1, axis, Position::LeansHigh),
                EvidenceRef::correction(2, axis, Position::LeansLow),
            ],
        ),
        (
            "inference_after_lock",
            vec![
                EvidenceRef::correction(1, axis, Position::LeansLow),
                EvidenceRef::inference(2, axis, Position::LeansHigh, Band::Strong),
            ],
        ),
        (
            "contradiction",
            vec![
                EvidenceRef::questionnaire(1, axis, Position::LeansLow),
                EvidenceRef::questionnaire(2, axis, Position::LeansHigh),
            ],
        ),
        (
            "three_agreeing",
            vec![
                EvidenceRef::questionnaire(1, axis, Position::LeansHigh),
                EvidenceRef::questionnaire(2, axis, Position::LeansHigh),
                EvidenceRef::questionnaire(3, axis, Position::LeansHigh),
            ],
        ),
        (
            "three_agreeing_one_forgotten",
            vec![
                EvidenceRef::questionnaire(1, axis, Position::LeansHigh),
                EvidenceRef::questionnaire(2, axis, Position::LeansHigh),
                EvidenceRef::questionnaire(3, axis, Position::LeansHigh).into_forgotten(),
            ],
        ),
        (
            "one_row_cited_three_times",
            vec![
                EvidenceRef::questionnaire(9, axis, Position::LeansHigh),
                EvidenceRef::questionnaire(9, axis, Position::LeansHigh),
                EvidenceRef::questionnaire(9, axis, Position::LeansHigh),
            ],
        ),
        (
            "correction_to_unknown",
            vec![
                EvidenceRef::questionnaire(1, axis, Position::LeansHigh),
                EvidenceRef::correction(2, axis, Position::Unknown),
            ],
        ),
        (
            "everything_forgotten",
            vec![
                EvidenceRef::questionnaire(1, axis, Position::LeansHigh).into_forgotten(),
                EvidenceRef::correction(2, axis, Position::LeansLow).into_forgotten(),
            ],
        ),
    ]
}
