//! A1: the independence key, F16, F16b, and the conservative mixed rule.
//!
//! F16 and F16b are the death line and the survival line `EVAL_MATRIX.md` set
//! for this candidate. F16: five re-fills of the same questionnaire on one day
//! must not upgrade. F16b: a questionnaire answer plus three behavioural rows
//! on three separate days must.

use soul_algo_trait::a1::{
    a1_axis_state, a1_axis_state_with, A1Independence, A1Reason, A1_GROUPS_FOR_STRONG,
};
use soul_algo_trait::fixtures::{axis_evidence_matrix, DAY, FIXTURE_DAY_ZERO_UNIX};
use soul_algo_trait::types::{
    ApplyResult, AxisId, Band, EvidenceKind, EvidenceRef, Position, SECONDS_PER_DAY,
};

const AXIS: AxisId = AxisId::SocialEnergy;

fn day(n: i64) -> i64 {
    FIXTURE_DAY_ZERO_UNIX + n * DAY
}

fn fixture(name: &str) -> Vec<EvidenceRef> {
    axis_evidence_matrix()
        .into_iter()
        .find(|(key, _)| *key == name)
        .unwrap_or_else(|| panic!("no fixture named {name}"))
        .1
}

fn behavioural(id: u64, position: Position, at: i64) -> EvidenceRef {
    EvidenceRef::inference(id, AXIS, position, Band::Weak, EvidenceKind::Message, at)
}

// ------------------------------------------------------------------- F16 ---

#[test]
fn f16_five_refills_in_one_afternoon_do_not_upgrade() {
    let outcome = a1_axis_state(AXIS, &fixture("f16_five_refills_same_day"));

    assert_eq!(outcome.state.position, Position::LeansHigh);
    assert_eq!(
        outcome.state.band,
        Band::Moderate,
        "five rows, one questionnaire, one day: one group"
    );
    assert_eq!(outcome.groups.len(), 1);
    assert_eq!(outcome.groups[0].evidence_ids, vec![1, 2, 3, 4, 5]);
    assert_eq!(outcome.supporting_group_count(), 1);
    assert!(matches!(
        outcome.reason,
        A1Reason::NotEnoughIndependentGroups { groups: 1 }
    ));
    assert_eq!(
        outcome.state.evidence_ids,
        vec![1, 2, 3, 4, 5],
        "all five are still cited; they are just not five reasons"
    );
}

#[test]
fn f16_holds_however_many_times_the_questionnaire_is_refilled_in_a_day() {
    let rows: Vec<EvidenceRef> = (1..=50)
        .map(|n| EvidenceRef::questionnaire(n as u64, AXIS, Position::LeansHigh, day(0) + n * 900))
        .collect();

    let outcome = a1_axis_state(AXIS, &rows);
    assert_eq!(outcome.groups.len(), 1);
    assert_eq!(outcome.state.band, Band::Moderate);
}

#[test]
fn a_day_boundary_is_a_utc_day_boundary() {
    // One second either side of midnight is two groups; 23 hours inside one day
    // is still one. The key has to be a calendar day or "same day" is a matter
    // of opinion.
    let midnight = day(10);
    let same_day = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh, midnight),
        EvidenceRef::questionnaire(2, AXIS, Position::LeansHigh, midnight + SECONDS_PER_DAY - 1),
    ];
    assert_eq!(a1_axis_state(AXIS, &same_day).groups.len(), 1);

    let across_midnight = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh, midnight - 1),
        EvidenceRef::questionnaire(2, AXIS, Position::LeansHigh, midnight),
    ];
    assert_eq!(a1_axis_state(AXIS, &across_midnight).groups.len(), 2);
}

#[test]
fn the_day_key_works_either_side_of_the_epoch() {
    let before = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh, -1),
        EvidenceRef::questionnaire(2, AXIS, Position::LeansHigh, -SECONDS_PER_DAY + 1),
    ];
    assert_eq!(
        a1_axis_state(AXIS, &before).groups.len(),
        1,
        "1969-12-31 is one day, not two half-days rounded toward the epoch"
    );

    let straddling = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh, -1),
        EvidenceRef::questionnaire(2, AXIS, Position::LeansHigh, 0),
    ];
    assert_eq!(a1_axis_state(AXIS, &straddling).groups.len(), 2);
}

// ------------------------------------------------------------------ F16b ---

#[test]
fn f16b_three_independent_agreeing_groups_reach_strong() {
    let outcome = a1_axis_state(
        AXIS,
        &fixture("f16b_questionnaire_plus_three_independent_days"),
    );

    assert_eq!(outcome.state.position, Position::LeansHigh);
    assert_eq!(outcome.state.band, Band::Strong);
    assert_eq!(outcome.groups.len(), 4);
    assert_eq!(outcome.supporting_group_count(), 4);
    assert_eq!(outcome.state.evidence_ids, vec![1, 2, 3, 4]);
    assert!(matches!(
        outcome.reason,
        A1Reason::UpgradedByIndependentGroups { groups: 4 }
    ));
}

#[test]
fn exactly_three_groups_is_the_upgrade_boundary() {
    let three = vec![
        behavioural(1, Position::LeansHigh, day(1)),
        behavioural(2, Position::LeansHigh, day(2)),
        behavioural(3, Position::LeansHigh, day(3)),
    ];
    assert_eq!(a1_axis_state(AXIS, &three).state.band, Band::Strong);
    assert_eq!(A1_GROUPS_FOR_STRONG, 3);

    let two = &three[..2];
    let outcome = a1_axis_state(AXIS, two);
    assert_eq!(outcome.state.band, Band::Weak, "strongest single row");
    assert!(matches!(
        outcome.reason,
        A1Reason::NotEnoughIndependentGroups { groups: 2 }
    ));
}

#[test]
fn three_different_sources_on_one_day_are_three_groups() {
    // The key is (kind, utc_day), not the day alone: a message observation and
    // an app-usage observation recorded the same afternoon are two things that
    // happened, not one thing counted twice.
    let rows = vec![
        EvidenceRef::inference(
            1,
            AXIS,
            Position::LeansHigh,
            Band::Weak,
            EvidenceKind::Message,
            day(1),
        ),
        EvidenceRef::inference(
            2,
            AXIS,
            Position::LeansHigh,
            Band::Weak,
            EvidenceKind::AppUsage,
            day(1) + 60,
        ),
        EvidenceRef::inference(
            3,
            AXIS,
            Position::LeansHigh,
            Band::Weak,
            EvidenceKind::Aggregate,
            day(1) + 120,
        ),
    ];

    let outcome = a1_axis_state(AXIS, &rows);
    assert_eq!(outcome.groups.len(), 3);
    assert_eq!(outcome.state.band, Band::Strong);
    assert_eq!(outcome.distinct_supporting_days(), 1);
    assert_eq!(outcome.distinct_supporting_kinds(), 3);
}

#[test]
fn a_forgotten_row_takes_its_group_with_it() {
    let outcome = a1_axis_state(AXIS, &fixture("three_agreeing_one_forgotten"));
    assert_eq!(outcome.groups.len(), 2);
    assert_eq!(outcome.state.band, Band::Moderate);
    assert!(!outcome.state.evidence_ids.contains(&3));
}

#[test]
fn one_row_cited_three_times_is_one_row() {
    let outcome = a1_axis_state(AXIS, &fixture("one_row_cited_three_times"));
    assert_eq!(outcome.groups.len(), 1);
    assert_eq!(outcome.state.evidence_ids, vec![9]);
    assert_eq!(outcome.state.band, Band::Moderate);
}

// ------------------------------------------------------- disagreement ---

#[test]
fn one_group_each_way_is_already_mixed() {
    let rows = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansLow, day(0)),
        behavioural(2, Position::LeansHigh, day(5)),
    ];

    let outcome = a1_axis_state(AXIS, &rows);
    assert_eq!(outcome.state.position, Position::Mixed);
    assert_eq!(outcome.state.band, Band::Weak);
    assert_eq!(outcome.reason, A1Reason::Contradicted);
    assert_eq!(
        outcome.state.evidence_ids,
        vec![1, 2],
        "the disagreement is the finding, so both sides are cited"
    );
}

#[test]
fn a_majority_never_settles_a_disagreement() {
    let rows = vec![
        behavioural(1, Position::LeansHigh, day(1)),
        behavioural(2, Position::LeansHigh, day(2)),
        behavioural(3, Position::LeansHigh, day(3)),
        behavioural(4, Position::LeansHigh, day(4)),
        EvidenceRef::questionnaire(5, AXIS, Position::LeansLow, day(9)),
    ];

    let outcome = a1_axis_state(AXIS, &rows);
    assert_eq!(
        outcome.state.position,
        Position::Mixed,
        "four against one is still not a vote"
    );
    assert_eq!(outcome.state.band, Band::Weak);
}

#[test]
fn a_source_that_contradicts_itself_in_a_day_supports_neither_side() {
    let rows = vec![
        behavioural(1, Position::LeansHigh, day(1)),
        behavioural(2, Position::LeansLow, day(1) + 3_600),
    ];

    let outcome = a1_axis_state(AXIS, &rows);
    assert_eq!(outcome.groups.len(), 1);
    assert_eq!(outcome.groups[0].position, Position::Mixed);
    assert_eq!(outcome.state.position, Position::Mixed);
    assert_eq!(outcome.state.band, Band::Weak);
}

#[test]
fn an_explicit_mixed_answer_is_a_statement_not_a_tie() {
    let rows = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::Mixed, day(0)),
        behavioural(2, Position::LeansHigh, day(5)),
        behavioural(3, Position::LeansHigh, day(9)),
        behavioural(4, Position::LeansHigh, day(14)),
    ];

    let outcome = a1_axis_state(AXIS, &rows);
    assert_eq!(outcome.state.position, Position::Mixed);
    assert_eq!(outcome.state.band, Band::Weak);
}

// -------------------------------------------------------------- the lock ---

#[test]
fn a_correction_settles_the_axis_before_any_counting_happens() {
    let rows = vec![
        behavioural(1, Position::LeansHigh, day(1)),
        behavioural(2, Position::LeansHigh, day(2)),
        behavioural(3, Position::LeansHigh, day(3)),
        EvidenceRef::correction(4, AXIS, Position::LeansLow, day(4)),
    ];

    let outcome = a1_axis_state(AXIS, &rows);
    assert_eq!(outcome.state.position, Position::LeansLow);
    assert_eq!(outcome.state.band, Band::Strong);
    assert!(outcome.state.locked_by_user);
    assert_eq!(outcome.state.evidence_ids, vec![4]);
    assert_eq!(outcome.reason, A1Reason::UserLocked);
    assert!(
        outcome.groups.is_empty(),
        "no grouping runs once the user has spoken"
    );
    assert_eq!(outcome.refused_ids(), vec![1, 2, 3]);
    assert!(outcome
        .shadow
        .iter()
        .take(3)
        .all(|entry| entry.result == ApplyResult::RefusedLocked));
}

#[test]
fn a_correction_to_unknown_locks_the_axis_at_unknown() {
    let outcome = a1_axis_state(AXIS, &fixture("correction_to_unknown"));
    assert_eq!(outcome.state.position, Position::Unknown);
    assert_eq!(outcome.state.band, Band::None);
    assert!(outcome.state.locked_by_user);
}

#[test]
fn nothing_at_all_is_unknown() {
    let outcome = a1_axis_state(AXIS, &[]);
    assert_eq!(outcome.state.position, Position::Unknown);
    assert_eq!(outcome.state.band, Band::None);
    assert_eq!(outcome.reason, A1Reason::NoEvidence);

    let all_gone = a1_axis_state(AXIS, &fixture("everything_forgotten"));
    assert_eq!(all_gone.state.position, Position::Unknown);
    assert_eq!(all_gone.reason, A1Reason::NoEvidence);
}

// ------------------------------------------------- EMPTY_ON_V01 evidence ---

#[test]
fn a1_does_fire_on_v01_data_when_the_questionnaire_is_refilled_across_days() {
    // The scenario `REPORT.md` records as `EMPTY_ON_V01=false`: three
    // questionnaire re-fills, same axis, same direction, three different UTC
    // days, no correction. Nothing but the questionnaire is involved, so a v0.1
    // install can reach it.
    let outcome = a1_axis_state(AXIS, &fixture("three_questionnaire_refills_on_three_days"));

    assert_eq!(outcome.groups.len(), 3);
    assert_eq!(outcome.distinct_supporting_kinds(), 1);
    assert_eq!(outcome.state.band, Band::Strong);
    assert!(outcome
        .state
        .evidence_ids
        .iter()
        .all(|id| [1, 2, 3].contains(id)));
}

#[test]
fn the_strict_variant_cannot_fire_on_v01_data() {
    // Same rows, `TwoKindsAcrossDays`: the upgrade also needs the axis to have
    // been heard about from more than one kind of source. v0.1 can only produce
    // `questionnaire` and `user_correction` rows against an axis, and a
    // correction locks before counting, so one kind is the ceiling.
    let outcome = a1_axis_state_with(
        AXIS,
        &fixture("three_questionnaire_refills_on_three_days"),
        A1Independence::TwoKindsAcrossDays,
    );

    assert_eq!(outcome.state.band, Band::Moderate);
    assert!(matches!(
        outcome.reason,
        A1Reason::NotEnoughIndependentGroups { groups: 3 }
    ));
}

#[test]
fn the_strict_variant_still_passes_f16b() {
    let outcome = a1_axis_state_with(
        AXIS,
        &fixture("f16b_questionnaire_plus_three_independent_days"),
        A1Independence::TwoKindsAcrossDays,
    );
    assert_eq!(outcome.state.band, Band::Strong);
    assert_eq!(outcome.distinct_supporting_kinds(), 2);
}

#[test]
fn only_two_evidence_kinds_can_reach_an_axis_in_v01() {
    // The fact the whole EMPTY_ON_V01 argument rests on, kept as a test so that
    // adding a third producer breaks something visible.
    let reachable: Vec<&str> = EvidenceKind::ALL
        .iter()
        .filter(|kind| kind.reachable_for_axis_in_v01())
        .map(|kind| kind.as_str())
        .collect();
    assert_eq!(reachable, vec!["questionnaire", "user_correction"]);
}

// ------------------------------------------------------------ explanation ---

#[test]
fn every_explanation_is_countable_and_publishable() {
    for (name, log) in axis_evidence_matrix() {
        for independence in [
            A1Independence::KindAndDay,
            A1Independence::TwoKindsAcrossDays,
        ] {
            let text = a1_axis_state_with(AXIS, &log, independence).explain_zh();
            assert!(!text.is_empty(), "{name}");
            soul_algo_trait::denylist::assert_publishable(&text)
                .unwrap_or_else(|hit| panic!("{name}: {hit:?} in {text}"));
        }
    }
}

#[test]
fn the_upgrade_explanation_names_the_groups_days_and_sources() {
    let outcome = a1_axis_state(
        AXIS,
        &fixture("f16b_questionnaire_plus_three_independent_days"),
    );
    let text = outcome.explain_zh();
    assert!(text.contains('4'), "four groups: {text}");
    assert!(text.contains('2'), "two kinds of source: {text}");
}
