//! A0: the questionnaire answers, the user corrects, and the machine stops
//! arguing.

use soul_algo_trait::a0::{a0_all_axes, a0_axis_state, A0_ALGORITHM_ID};
use soul_algo_trait::denylist::numeric_rating_hit;
use soul_algo_trait::{ApplyResult, AxisId, Band, EvidenceRef, Position};

const AXIS: AxisId = AxisId::SocialEnergy;

#[test]
fn empty_evidence_leaves_the_axis_unknown() {
    let outcome = a0_axis_state(AXIS, &[]);

    assert_eq!(outcome.state.position, Position::Unknown);
    assert_eq!(outcome.state.band, Band::None);
    assert!(outcome.state.evidence_ids.is_empty());
    assert!(!outcome.state.locked_by_user);
    assert_eq!(outcome.state.algorithm_id, A0_ALGORITHM_ID);
    assert!(outcome.shadow.is_empty());
}

#[test]
fn a_questionnaire_answer_is_moderate_and_does_not_lock() {
    let evidence = vec![EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh)];

    let outcome = a0_axis_state(AXIS, &evidence);

    assert_eq!(outcome.state.position, Position::LeansHigh);
    assert_eq!(outcome.state.band, Band::Moderate);
    assert_eq!(outcome.state.evidence_ids, vec![1]);
    assert!(!outcome.state.locked_by_user);
}

#[test]
fn a_correction_is_strong_and_locks() {
    let evidence = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh),
        EvidenceRef::correction(2, AXIS, Position::LeansLow),
    ];

    let outcome = a0_axis_state(AXIS, &evidence);

    assert_eq!(outcome.state.position, Position::LeansLow);
    assert_eq!(outcome.state.band, Band::Strong);
    assert!(outcome.state.locked_by_user);
    assert_eq!(outcome.state.evidence_ids, vec![2]);
}

/// The headline rule: after a correction, inference is recorded and ignored.
#[test]
fn inference_after_a_lock_does_not_move_the_axis() {
    let evidence = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh),
        EvidenceRef::correction(2, AXIS, Position::LeansLow),
        EvidenceRef::inference(3, AXIS, Position::LeansHigh, Band::Strong),
        EvidenceRef::inference(4, AXIS, Position::Mixed, Band::Moderate),
    ];

    let outcome = a0_axis_state(AXIS, &evidence);

    assert_eq!(outcome.state.position, Position::LeansLow);
    assert_eq!(outcome.state.band, Band::Strong);
    assert!(outcome.state.locked_by_user);
    assert_eq!(outcome.state.evidence_ids, vec![2]);

    assert_eq!(outcome.refused_ids(), vec![3, 4]);
    let refused: Vec<ApplyResult> = outcome.refused().map(|entry| entry.result).collect();
    assert_eq!(
        refused,
        vec![ApplyResult::RefusedLocked, ApplyResult::RefusedLocked]
    );
}

/// Refused is not the same as dropped: the shadow still says what the machine
/// thought, so the user can see the disagreement.
#[test]
fn a_refused_inference_is_still_returned_in_the_shadow() {
    let evidence = vec![
        EvidenceRef::correction(1, AXIS, Position::LeansLow),
        EvidenceRef::inference(2, AXIS, Position::LeansHigh, Band::Moderate),
    ];

    let outcome = a0_axis_state(AXIS, &evidence);
    let shadow = outcome
        .shadow
        .iter()
        .find(|entry| entry.evidence_id == 2)
        .expect("the refused inference is kept");

    assert_eq!(shadow.position, Position::LeansHigh);
    assert_eq!(shadow.band, Band::Moderate);
    assert_eq!(shadow.result, ApplyResult::RefusedLocked);
    assert!(!shadow.cited);
}

/// A lock stops the machine, not the user.
#[test]
fn a_later_correction_still_applies() {
    let evidence = vec![
        EvidenceRef::correction(1, AXIS, Position::LeansLow),
        EvidenceRef::inference(2, AXIS, Position::LeansHigh, Band::Strong),
        EvidenceRef::correction(3, AXIS, Position::LeansHigh),
    ];

    let outcome = a0_axis_state(AXIS, &evidence);

    assert_eq!(outcome.state.position, Position::LeansHigh);
    assert_eq!(outcome.state.evidence_ids, vec![3]);
    assert!(outcome.state.locked_by_user);
    assert_eq!(outcome.refused_ids(), vec![2]);
}

/// "Stop guessing" is a legitimate correction. It locks, and it claims nothing.
#[test]
fn a_correction_to_unknown_locks_at_band_none() {
    let evidence = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh),
        EvidenceRef::correction(2, AXIS, Position::Unknown),
        EvidenceRef::inference(3, AXIS, Position::LeansHigh, Band::Moderate),
    ];

    let outcome = a0_axis_state(AXIS, &evidence);

    assert_eq!(outcome.state.position, Position::Unknown);
    assert_eq!(outcome.state.band, Band::None);
    assert!(outcome.state.locked_by_user);
    assert_eq!(outcome.refused_ids(), vec![3]);
}

#[test]
fn forgetting_the_latest_row_makes_the_axis_fall_back() {
    let questionnaire = EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh);
    let inference = EvidenceRef::inference(2, AXIS, Position::LeansLow, Band::Weak);

    let before = a0_axis_state(AXIS, &[questionnaire.clone(), inference.clone()]);
    assert_eq!(before.state.position, Position::LeansLow);
    assert_eq!(before.state.band, Band::Weak);

    let after = a0_axis_state(AXIS, &[questionnaire, inference.into_forgotten()]);

    assert_eq!(after.state.position, Position::LeansHigh);
    assert_eq!(after.state.band, Band::Moderate);
    assert_eq!(after.state.evidence_ids, vec![1]);
    assert!(
        after.shadow.iter().all(|entry| entry.evidence_id != 2),
        "a forgotten row is dropped before aggregation, shadow included"
    );
}

#[test]
fn forgetting_the_correction_unlocks_the_axis() {
    let questionnaire = EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh);
    let correction = EvidenceRef::correction(2, AXIS, Position::LeansLow);
    let later = EvidenceRef::inference(3, AXIS, Position::Mixed, Band::Weak);

    let after = a0_axis_state(AXIS, &[questionnaire, correction.into_forgotten(), later]);

    assert!(!after.state.locked_by_user);
    assert_eq!(after.state.position, Position::Mixed);
    assert_eq!(after.state.band, Band::Weak);
    assert_eq!(after.state.evidence_ids, vec![3]);
}

#[test]
fn forgetting_everything_returns_the_axis_to_unknown() {
    let evidence = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh).into_forgotten(),
        EvidenceRef::correction(2, AXIS, Position::LeansLow).into_forgotten(),
    ];

    let outcome = a0_axis_state(AXIS, &evidence);

    assert_eq!(outcome.state.position, Position::Unknown);
    assert_eq!(outcome.state.band, Band::None);
    assert!(!outcome.state.locked_by_user);
    assert!(outcome.state.evidence_ids.is_empty());
}

#[test]
fn evidence_for_another_axis_is_ignored() {
    let evidence = vec![
        EvidenceRef::questionnaire(1, AxisId::Curiosity, Position::LeansHigh),
        EvidenceRef::questionnaire(2, AXIS, Position::LeansLow),
    ];

    let outcome = a0_axis_state(AXIS, &evidence);

    assert_eq!(outcome.state.evidence_ids, vec![2]);
    assert_eq!(outcome.state.position, Position::LeansLow);
}

#[test]
fn all_five_axes_come_back_in_questionnaire_order() {
    let outcomes = a0_all_axes(&[EvidenceRef::questionnaire(
        1,
        AxisId::Accommodation,
        Position::LeansLow,
    )]);

    let axes: Vec<AxisId> = outcomes.iter().map(|outcome| outcome.state.axis).collect();
    assert_eq!(axes, AxisId::ALL.to_vec());

    let touched: Vec<&soul_algo_trait::AxisState> = outcomes
        .iter()
        .map(|outcome| &outcome.state)
        .filter(|state| state.band != Band::None)
        .collect();
    assert_eq!(touched.len(), 1);
    assert_eq!(touched[0].axis, AxisId::Accommodation);
}

/// D22: an axis carries a direction and a band, and nothing that reads as a
/// rating — including in the debug rendering a developer or a log would see.
#[test]
fn no_axis_state_carries_anything_numeric() {
    let evidence = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh),
        EvidenceRef::correction(2, AXIS, Position::LeansLow),
        EvidenceRef::inference(3, AXIS, Position::Mixed, Band::Weak),
    ];

    for outcome in a0_all_axes(&evidence) {
        let rendered = format!("{:?}", outcome.state);
        assert_eq!(
            numeric_rating_hit(&rendered),
            None,
            "state must not read as a rating: {rendered}"
        );
    }
}
