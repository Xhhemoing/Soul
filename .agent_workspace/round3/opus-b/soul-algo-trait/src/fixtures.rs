//! Deterministic fixtures, shipped in the library rather than in one test file
//! so that every screen, every later round and any benchmark walks the same
//! cases.
//!
//! Nothing here reads a clock or a file. [`FIXTURE_AS_OF_UNIX`] is the `as_of`
//! every recency case is written against, so a fixture cannot rot into a flaky
//! test the week after it was added.

use crate::a2::TieScore;
use crate::types::{AxisId, Band, EvidenceKind, EvidenceRef, Position};

/// The fixed `as_of` the peer fixtures are written against:
/// 2025-10-09T07:33:20Z. It stands for「数据内最大时间戳」, never a wall clock.
pub const FIXTURE_AS_OF_UNIX: i64 = 1_760_000_000;

/// The fixed day the axis fixtures start on: 2023-11-14T00:00:00Z, an exact
/// UTC midnight, so that `DAY_ZERO + n * DAY` is unambiguously the start of the
/// `n`th day. A fixture base that landed mid-afternoon would make "same day"
/// depend on how many seconds a test happened to add.
pub const FIXTURE_DAY_ZERO_UNIX: i64 = 1_699_920_000;

/// Seconds in a day, so a fixture can say "eleven days later" and mean it.
pub const DAY: i64 = 86_400;

/// Edge scores covering the shapes A2 has to survive: nothing at all, counts
/// with no citable evidence, one-way traffic, a group-only peer, a thick
/// reciprocal edge, a single-day burst, a long-dormant edge, an unbanded edge,
/// a timestamp from a disagreeing clock, and — added in Round 3 — the four
/// venue-split shapes a T4D-style scorer can produce.
///
/// The bands are the ones a tie algorithm would have assigned. A2 does not get
/// a vote, so the fixtures deliberately include combinations A2 would never
/// have chosen for itself — a `Weak` band on a thick edge, for instance — to
/// prove that it renders what it is given.
pub fn tie_score_matrix() -> Vec<(&'static str, TieScore)> {
    vec![
        ("empty", TieScore::default()),
        (
            "counts_without_citable_evidence",
            TieScore {
                band: Band::Moderate,
                interaction_count: 9,
                outgoing: 5,
                incoming: 4,
                active_day_count: 4,
                conversation_count: 2,
                any_direct: true,
                direct_count: None,
                group_count: None,
                last_contact_unix: Some(FIXTURE_AS_OF_UNIX - DAY),
                as_of_unix: FIXTURE_AS_OF_UNIX,
                evidence_ids: Vec::new(),
                last_contact_evidence_id: None,
            },
        ),
        (
            "single_outgoing_message",
            TieScore {
                band: Band::Weak,
                interaction_count: 1,
                outgoing: 1,
                incoming: 0,
                active_day_count: 1,
                conversation_count: 1,
                any_direct: true,
                direct_count: None,
                group_count: None,
                last_contact_unix: Some(FIXTURE_AS_OF_UNIX - 3 * DAY),
                as_of_unix: FIXTURE_AS_OF_UNIX,
                evidence_ids: vec![101],
                last_contact_evidence_id: Some(101),
            },
        ),
        (
            "one_way_incoming_only",
            TieScore {
                band: Band::Weak,
                interaction_count: 12,
                outgoing: 0,
                incoming: 12,
                active_day_count: 5,
                conversation_count: 1,
                any_direct: true,
                direct_count: None,
                group_count: None,
                last_contact_unix: Some(FIXTURE_AS_OF_UNIX - 11 * DAY),
                as_of_unix: FIXTURE_AS_OF_UNIX,
                evidence_ids: vec![201, 202, 203],
                last_contact_evidence_id: Some(203),
            },
        ),
        (
            "group_only_reciprocal",
            TieScore {
                band: Band::Moderate,
                interaction_count: 14,
                outgoing: 6,
                incoming: 8,
                active_day_count: 4,
                conversation_count: 1,
                any_direct: false,
                direct_count: None,
                group_count: None,
                last_contact_unix: Some(FIXTURE_AS_OF_UNIX - 2 * DAY),
                as_of_unix: FIXTURE_AS_OF_UNIX,
                evidence_ids: vec![301, 302, 303, 304],
                last_contact_evidence_id: Some(304),
            },
        ),
        (
            "direct_reciprocal_thick",
            TieScore {
                band: Band::Strong,
                interaction_count: 240,
                outgoing: 130,
                incoming: 110,
                active_day_count: 61,
                conversation_count: 9,
                any_direct: true,
                direct_count: None,
                group_count: None,
                last_contact_unix: Some(FIXTURE_AS_OF_UNIX),
                as_of_unix: FIXTURE_AS_OF_UNIX,
                evidence_ids: vec![401, 402, 403, 404, 405],
                last_contact_evidence_id: Some(405),
            },
        ),
        (
            "single_day_burst",
            TieScore {
                band: Band::Moderate,
                interaction_count: 1_000,
                outgoing: 500,
                incoming: 500,
                active_day_count: 1,
                conversation_count: 1,
                any_direct: true,
                direct_count: None,
                group_count: None,
                last_contact_unix: Some(FIXTURE_AS_OF_UNIX - DAY),
                as_of_unix: FIXTURE_AS_OF_UNIX,
                evidence_ids: vec![501, 502],
                last_contact_evidence_id: Some(502),
            },
        ),
        (
            "dormant_but_strong",
            TieScore {
                band: Band::Strong,
                interaction_count: 180,
                outgoing: 90,
                incoming: 90,
                active_day_count: 40,
                conversation_count: 3,
                any_direct: true,
                direct_count: None,
                group_count: None,
                last_contact_unix: Some(FIXTURE_AS_OF_UNIX - 400 * DAY),
                as_of_unix: FIXTURE_AS_OF_UNIX,
                evidence_ids: vec![601, 602, 603],
                last_contact_evidence_id: Some(603),
            },
        ),
        (
            "quiet_for_exactly_the_threshold",
            TieScore {
                band: Band::Moderate,
                interaction_count: 18,
                outgoing: 9,
                incoming: 9,
                active_day_count: 7,
                conversation_count: 2,
                any_direct: true,
                direct_count: None,
                group_count: None,
                last_contact_unix: Some(FIXTURE_AS_OF_UNIX - 180 * DAY),
                as_of_unix: FIXTURE_AS_OF_UNIX,
                evidence_ids: vec![701, 702],
                last_contact_evidence_id: Some(702),
            },
        ),
        (
            "no_band_assigned_yet",
            TieScore {
                band: Band::None,
                interaction_count: 2,
                outgoing: 1,
                incoming: 1,
                active_day_count: 2,
                conversation_count: 1,
                any_direct: true,
                direct_count: None,
                group_count: None,
                last_contact_unix: None,
                as_of_unix: FIXTURE_AS_OF_UNIX,
                evidence_ids: vec![801],
                last_contact_evidence_id: None,
            },
        ),
        (
            "clock_ahead_of_last_contact",
            TieScore {
                band: Band::Weak,
                interaction_count: 3,
                outgoing: 2,
                incoming: 1,
                active_day_count: 2,
                conversation_count: 1,
                any_direct: false,
                direct_count: None,
                group_count: None,
                last_contact_unix: Some(FIXTURE_AS_OF_UNIX + 5 * DAY),
                as_of_unix: FIXTURE_AS_OF_UNIX,
                evidence_ids: vec![901],
                last_contact_evidence_id: Some(901),
            },
        ),
        // `R2-SYNTHESIS.md` 边界风险 2, as a rendering case. A busy shared group
        // plus one private hello in each direction: 137 interactions, of which
        // two were one to one. Whether that is allowed to be Strong is T4D's
        // question and not A2's, so the band here is the one the *current* T4
        // would assign, and the job of the split sentence is to make the shape
        // of the edge visible to the user either way.
        (
            "group_heavy_plus_one_direct_each_way",
            TieScore {
                band: Band::Strong,
                interaction_count: 137,
                outgoing: 70,
                incoming: 67,
                active_day_count: 22,
                conversation_count: 2,
                any_direct: true,
                direct_count: Some(2),
                group_count: Some(135),
                last_contact_unix: Some(FIXTURE_AS_OF_UNIX - 4 * DAY),
                as_of_unix: FIXTURE_AS_OF_UNIX,
                evidence_ids: vec![1001, 1002, 1003],
                last_contact_evidence_id: Some(1003),
            },
        ),
        (
            "direct_heavy_with_split",
            TieScore {
                band: Band::Strong,
                interaction_count: 96,
                outgoing: 50,
                incoming: 46,
                active_day_count: 31,
                conversation_count: 4,
                any_direct: true,
                direct_count: Some(90),
                group_count: Some(6),
                last_contact_unix: Some(FIXTURE_AS_OF_UNIX - DAY),
                as_of_unix: FIXTURE_AS_OF_UNIX,
                evidence_ids: vec![1101, 1102],
                last_contact_evidence_id: Some(1102),
            },
        ),
        // A tie algorithm that splits its tally but has never seen this peer
        // outside a group. Both the split sentence and the group-only sentence
        // apply, and they are separate sentences because they rest on separate
        // fields.
        (
            "group_only_with_split",
            TieScore {
                band: Band::Moderate,
                interaction_count: 41,
                outgoing: 20,
                incoming: 21,
                active_day_count: 9,
                conversation_count: 1,
                any_direct: false,
                direct_count: Some(0),
                group_count: Some(41),
                last_contact_unix: Some(FIXTURE_AS_OF_UNIX - 6 * DAY),
                as_of_unix: FIXTURE_AS_OF_UNIX,
                evidence_ids: vec![1201],
                last_contact_evidence_id: Some(1201),
            },
        ),
        // Half a split is not a split: a tie algorithm that supplied one field
        // and not the other gets no sentence at all, rather than an invitation
        // to subtract.
        (
            "half_a_split_is_not_a_split",
            TieScore {
                band: Band::Moderate,
                interaction_count: 20,
                outgoing: 11,
                incoming: 9,
                active_day_count: 6,
                conversation_count: 2,
                any_direct: true,
                direct_count: Some(12),
                group_count: None,
                last_contact_unix: Some(FIXTURE_AS_OF_UNIX - 2 * DAY),
                as_of_unix: FIXTURE_AS_OF_UNIX,
                evidence_ids: vec![1301],
                last_contact_evidence_id: Some(1301),
            },
        ),
    ]
}

/// Evidence logs covering what A0 and A1 have to survive: nothing, a plain
/// questionnaire, a correction, intake after a lock, a same-day re-fill (F16),
/// genuinely independent agreement (F16b), same-kind agreement across days
/// (the case the Round 3 default refuses to upgrade), a contradiction, a
/// forget, and duplicate citations of one row.
pub fn axis_evidence_matrix() -> Vec<(&'static str, Vec<EvidenceRef>)> {
    let axis = AxisId::SocialEnergy;
    let day = |n: i64| FIXTURE_DAY_ZERO_UNIX + n * DAY;

    vec![
        ("empty", Vec::new()),
        (
            "questionnaire_only",
            vec![EvidenceRef::questionnaire(
                1,
                axis,
                Position::LeansHigh,
                day(0),
            )],
        ),
        (
            "questionnaire_then_correction",
            vec![
                EvidenceRef::questionnaire(1, axis, Position::LeansHigh, day(0)),
                EvidenceRef::correction(2, axis, Position::LeansLow, day(1)),
            ],
        ),
        (
            "questionnaire_refilled_after_correction",
            vec![
                EvidenceRef::questionnaire(1, axis, Position::LeansHigh, day(0)),
                EvidenceRef::correction(2, axis, Position::LeansLow, day(1)),
                EvidenceRef::questionnaire(3, axis, Position::LeansHigh, day(9)),
            ],
        ),
        (
            "inference_after_lock",
            vec![
                EvidenceRef::correction(1, axis, Position::LeansLow, day(0)),
                EvidenceRef::inference(
                    2,
                    axis,
                    Position::LeansHigh,
                    Band::Strong,
                    EvidenceKind::Message,
                    day(1),
                ),
            ],
        ),
        (
            "contradiction",
            vec![
                EvidenceRef::questionnaire(1, axis, Position::LeansLow, day(0)),
                EvidenceRef::inference(
                    2,
                    axis,
                    Position::LeansHigh,
                    Band::Moderate,
                    EvidenceKind::Message,
                    day(5),
                ),
            ],
        ),
        (
            "f16_five_refills_same_day",
            (1..=5)
                .map(|n: i64| {
                    EvidenceRef::questionnaire(
                        n as u64,
                        axis,
                        Position::LeansHigh,
                        day(0) + n * 600,
                    )
                })
                .collect(),
        ),
        (
            "f16b_questionnaire_plus_three_independent_days",
            vec![
                EvidenceRef::questionnaire(1, axis, Position::LeansHigh, day(1)),
                EvidenceRef::inference(
                    2,
                    axis,
                    Position::LeansHigh,
                    Band::Weak,
                    EvidenceKind::Message,
                    day(5),
                ),
                EvidenceRef::inference(
                    3,
                    axis,
                    Position::LeansHigh,
                    Band::Weak,
                    EvidenceKind::Message,
                    day(20),
                ),
                EvidenceRef::inference(
                    4,
                    axis,
                    Position::LeansHigh,
                    Band::Weak,
                    EvidenceKind::Message,
                    day(40),
                ),
            ],
        ),
        (
            "three_questionnaire_refills_on_three_days",
            vec![
                EvidenceRef::questionnaire(1, axis, Position::LeansHigh, day(0)),
                EvidenceRef::questionnaire(2, axis, Position::LeansHigh, day(30)),
                EvidenceRef::questionnaire(3, axis, Position::LeansHigh, day(90)),
            ],
        ),
        // The same shape one kind along: three behavioural rows on three days,
        // no questionnaire. Still one kind, so the Round 3 default still does
        // not upgrade — the rule is "two instruments", not "not the
        // questionnaire".
        (
            "three_message_days_same_kind",
            vec![
                EvidenceRef::inference(
                    1,
                    axis,
                    Position::LeansHigh,
                    Band::Weak,
                    EvidenceKind::Message,
                    day(2),
                ),
                EvidenceRef::inference(
                    2,
                    axis,
                    Position::LeansHigh,
                    Band::Weak,
                    EvidenceKind::Message,
                    day(15),
                ),
                EvidenceRef::inference(
                    3,
                    axis,
                    Position::LeansHigh,
                    Band::Weak,
                    EvidenceKind::Message,
                    day(31),
                ),
            ],
        ),
        (
            "three_agreeing_one_forgotten",
            vec![
                EvidenceRef::questionnaire(1, axis, Position::LeansHigh, day(0)),
                EvidenceRef::questionnaire(2, axis, Position::LeansHigh, day(30)),
                EvidenceRef::questionnaire(3, axis, Position::LeansHigh, day(90)).into_forgotten(),
            ],
        ),
        (
            "one_row_cited_three_times",
            vec![
                EvidenceRef::questionnaire(9, axis, Position::LeansHigh, day(0)),
                EvidenceRef::questionnaire(9, axis, Position::LeansHigh, day(30)),
                EvidenceRef::questionnaire(9, axis, Position::LeansHigh, day(90)),
            ],
        ),
        (
            "correction_to_unknown",
            vec![
                EvidenceRef::questionnaire(1, axis, Position::LeansHigh, day(0)),
                EvidenceRef::correction(2, axis, Position::Unknown, day(1)),
            ],
        ),
        (
            "weak_inference_over_moderate_questionnaire",
            vec![
                EvidenceRef::questionnaire(1, axis, Position::LeansHigh, day(0)),
                EvidenceRef::inference(
                    2,
                    axis,
                    Position::LeansLow,
                    Band::Weak,
                    EvidenceKind::Message,
                    day(3),
                ),
            ],
        ),
        (
            "everything_forgotten",
            vec![
                EvidenceRef::questionnaire(1, axis, Position::LeansHigh, day(0)).into_forgotten(),
                EvidenceRef::correction(2, axis, Position::LeansLow, day(1)).into_forgotten(),
            ],
        ),
    ]
}
