//! A1: counting independent evidence, and the things counting is not allowed
//! to do.

use soul_algo_trait::a1::{
    a1_all_axes, a1_axis_state, a1_axis_state_with, DisagreementPolicy, A1_ALGORITHM_ID,
    A1_INDEPENDENT_FOR_STRONG,
};
use soul_algo_trait::{ApplyResult, AxisId, Band, EvidenceRef, Position};

const AXIS: AxisId = AxisId::Curiosity;

#[test]
fn empty_evidence_leaves_the_axis_unknown() {
    let outcome = a1_axis_state(AXIS, &[]);

    assert_eq!(outcome.state.position, Position::Unknown);
    assert_eq!(outcome.state.band, Band::None);
    assert!(outcome.state.evidence_ids.is_empty());
    assert_eq!(outcome.state.algorithm_id, A1_ALGORITHM_ID);
}

#[test]
fn one_questionnaire_answer_stays_moderate() {
    let outcome = a1_axis_state(
        AXIS,
        &[EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh)],
    );

    assert_eq!(outcome.state.position, Position::LeansHigh);
    assert_eq!(outcome.state.band, Band::Moderate);
    assert_eq!(outcome.state.evidence_ids, vec![1]);
}

#[test]
fn two_agreeing_answers_are_still_only_moderate() {
    let evidence = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh),
        EvidenceRef::questionnaire(2, AXIS, Position::LeansHigh),
    ];

    let outcome = a1_axis_state(AXIS, &evidence);

    assert_eq!(outcome.state.band, Band::Moderate);
    assert_eq!(outcome.state.evidence_ids, vec![1, 2]);
}

/// The upgrade. Three independent rows, each worth `Moderate` on its own,
/// agreeing on one direction.
#[test]
fn three_agreeing_independent_answers_reach_strong() {
    assert_eq!(A1_INDEPENDENT_FOR_STRONG, 3);
    let evidence = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh),
        EvidenceRef::questionnaire(2, AXIS, Position::LeansHigh),
        EvidenceRef::questionnaire(3, AXIS, Position::LeansHigh),
    ];

    let outcome = a1_axis_state(AXIS, &evidence);

    assert_eq!(outcome.state.position, Position::LeansHigh);
    assert_eq!(outcome.state.band, Band::Strong);
    assert_eq!(outcome.state.evidence_ids, vec![1, 2, 3]);
    assert!(
        !outcome.state.locked_by_user,
        "counting never locks an axis"
    );
}

/// Independence is by `evidence_id`. Citing one row three times is one row.
#[test]
fn the_same_row_cited_three_times_does_not_upgrade() {
    let row = EvidenceRef::questionnaire(7, AXIS, Position::LeansHigh);
    let evidence = vec![row.clone(), row.clone(), row];

    let outcome = a1_axis_state(AXIS, &evidence);

    assert_eq!(outcome.state.band, Band::Moderate);
    assert_eq!(outcome.state.evidence_ids, vec![7]);
    assert_eq!(
        outcome.shadow.len(),
        1,
        "duplicates are collapsed, not counted"
    );
}

/// Rows that are thin on their own do not add up to a strong claim.
#[test]
fn three_agreeing_weak_rows_stay_weak() {
    let evidence = vec![
        EvidenceRef::inference(1, AXIS, Position::LeansLow, Band::Weak),
        EvidenceRef::inference(2, AXIS, Position::LeansLow, Band::Weak),
        EvidenceRef::inference(3, AXIS, Position::LeansLow, Band::Weak),
    ];

    let outcome = a1_axis_state(AXIS, &evidence);

    assert_eq!(outcome.state.position, Position::LeansLow);
    assert_eq!(outcome.state.band, Band::Weak);
}

/// A machine inference cannot promote itself by labelling its own row.
#[test]
fn a_single_self_labelled_strong_inference_is_capped_at_moderate() {
    let outcome = a1_axis_state(
        AXIS,
        &[EvidenceRef::inference(
            1,
            AXIS,
            Position::LeansHigh,
            Band::Strong,
        )],
    );

    assert_eq!(outcome.state.band, Band::Moderate);
}

/// The required disagreement rule.
#[test]
fn disagreement_yields_mixed_and_weak() {
    let evidence = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansLow),
        EvidenceRef::questionnaire(2, AXIS, Position::LeansHigh),
    ];

    let outcome = a1_axis_state(AXIS, &evidence);

    assert_eq!(outcome.state.position, Position::Mixed);
    assert_eq!(outcome.state.band, Band::Weak);
    assert_eq!(outcome.state.evidence_ids, vec![1, 2]);
    assert!(!outcome.state.locked_by_user);
}

/// Strict is the default: three against one is still a disagreement, and the
/// three do not get to overrule the one by tally.
#[test]
fn strict_policy_does_not_let_a_tally_settle_a_disagreement() {
    let evidence = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh),
        EvidenceRef::questionnaire(2, AXIS, Position::LeansHigh),
        EvidenceRef::questionnaire(3, AXIS, Position::LeansHigh),
        EvidenceRef::questionnaire(4, AXIS, Position::LeansLow),
    ];

    let outcome = a1_axis_state(AXIS, &evidence);

    assert_eq!(outcome.state.position, Position::Mixed);
    assert_eq!(outcome.state.band, Band::Weak);
}

/// The opt-in alternative reading, kept comparable rather than argued about.
#[test]
fn majority_policy_picks_a_side_but_never_calls_it_strong() {
    let evidence = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh),
        EvidenceRef::questionnaire(2, AXIS, Position::LeansHigh),
        EvidenceRef::questionnaire(3, AXIS, Position::LeansHigh),
        EvidenceRef::questionnaire(4, AXIS, Position::LeansLow),
    ];

    let outcome = a1_axis_state_with(AXIS, &evidence, DisagreementPolicy::Majority);

    assert_eq!(outcome.state.position, Position::LeansHigh);
    assert_eq!(outcome.state.band, Band::Moderate);
    assert_eq!(outcome.state.evidence_ids, vec![1, 2, 3]);

    let outvoted = outcome
        .shadow
        .iter()
        .find(|entry| entry.evidence_id == 4)
        .expect("the outvoted row is kept");
    assert_eq!(
        outvoted.result,
        ApplyResult::Applied,
        "no lock was involved, so nothing was refused"
    );
    assert!(!outvoted.cited, "but it does not support the position");
}

#[test]
fn majority_policy_falls_back_to_mixed_on_a_tie() {
    let evidence = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh),
        EvidenceRef::questionnaire(2, AXIS, Position::LeansLow),
    ];

    let outcome = a1_axis_state_with(AXIS, &evidence, DisagreementPolicy::Majority);

    assert_eq!(outcome.state.position, Position::Mixed);
    assert_eq!(outcome.state.band, Band::Weak);
}

/// An explicit `mixed` claim is a statement, not a vote to be outnumbered.
#[test]
fn an_explicit_mixed_row_forces_mixed_under_every_policy() {
    let evidence = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh),
        EvidenceRef::questionnaire(2, AXIS, Position::LeansHigh),
        EvidenceRef::questionnaire(3, AXIS, Position::Mixed),
    ];

    for policy in [
        DisagreementPolicy::StrictMixed,
        DisagreementPolicy::Majority,
        DisagreementPolicy::BandFloored,
    ] {
        let outcome = a1_axis_state_with(AXIS, &evidence, policy);
        assert_eq!(outcome.state.position, Position::Mixed, "{policy:?}");
        assert_eq!(outcome.state.band, Band::Weak, "{policy:?}");
    }
}

/// The case the band-floored policy exists for: one weak machine guess against
/// what the user actually answered.
#[test]
fn band_floored_policy_does_not_let_a_weak_guess_unseat_an_answer() {
    let evidence = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh),
        EvidenceRef::inference(2, AXIS, Position::LeansLow, Band::Weak),
    ];

    let strict = a1_axis_state_with(AXIS, &evidence, DisagreementPolicy::StrictMixed);
    assert_eq!(strict.state.position, Position::Mixed);

    let floored = a1_axis_state_with(AXIS, &evidence, DisagreementPolicy::BandFloored);
    assert_eq!(floored.state.position, Position::LeansHigh);
    assert_eq!(floored.state.band, Band::Moderate);
    assert_eq!(floored.state.evidence_ids, vec![1]);
}

/// Two rows of the same worth pointing opposite ways is a real disagreement,
/// and the band-floored policy does not resolve it by counting either.
#[test]
fn band_floored_policy_still_reports_an_even_disagreement_as_mixed() {
    let evidence = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh),
        EvidenceRef::questionnaire(2, AXIS, Position::LeansLow),
        EvidenceRef::questionnaire(3, AXIS, Position::LeansLow),
    ];

    let outcome = a1_axis_state_with(AXIS, &evidence, DisagreementPolicy::BandFloored);

    assert_eq!(outcome.state.position, Position::Mixed);
    assert_eq!(outcome.state.band, Band::Weak);
}

/// The lock survives the upgrade rule: counting never beats the user.
#[test]
fn a_correction_beats_any_number_of_agreeing_inferences() {
    let evidence = vec![
        EvidenceRef::correction(1, AXIS, Position::LeansLow),
        EvidenceRef::inference(2, AXIS, Position::LeansHigh, Band::Moderate),
        EvidenceRef::inference(3, AXIS, Position::LeansHigh, Band::Moderate),
        EvidenceRef::inference(4, AXIS, Position::LeansHigh, Band::Moderate),
        EvidenceRef::inference(5, AXIS, Position::LeansHigh, Band::Moderate),
    ];

    let outcome = a1_axis_state(AXIS, &evidence);

    assert_eq!(outcome.state.position, Position::LeansLow);
    assert_eq!(outcome.state.band, Band::Strong);
    assert!(outcome.state.locked_by_user);
    assert_eq!(outcome.state.evidence_ids, vec![1]);
    assert_eq!(outcome.refused_ids(), vec![2, 3, 4, 5]);
}

/// Order does not matter to the lock: an inference recorded before the
/// correction is refused too, because A1 re-derives from the whole set.
#[test]
fn inferences_recorded_before_the_correction_are_refused_as_well() {
    let evidence = vec![
        EvidenceRef::inference(1, AXIS, Position::LeansHigh, Band::Moderate),
        EvidenceRef::correction(2, AXIS, Position::LeansLow),
    ];

    let outcome = a1_axis_state(AXIS, &evidence);

    assert_eq!(outcome.state.position, Position::LeansLow);
    assert_eq!(outcome.refused_ids(), vec![1]);
}

#[test]
fn agreeing_corrections_are_all_cited_and_a_superseded_one_is_not() {
    let evidence = vec![
        EvidenceRef::correction(1, AXIS, Position::LeansHigh),
        EvidenceRef::correction(2, AXIS, Position::LeansLow),
        EvidenceRef::correction(3, AXIS, Position::LeansLow),
    ];

    let outcome = a1_axis_state(AXIS, &evidence);

    assert_eq!(outcome.state.position, Position::LeansLow);
    assert_eq!(outcome.state.evidence_ids, vec![2, 3]);
    assert_eq!(outcome.refused_ids(), vec![1]);
}

#[test]
fn forgetting_one_of_three_drops_the_axis_back_to_moderate() {
    let first = EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh);
    let second = EvidenceRef::questionnaire(2, AXIS, Position::LeansHigh);
    let third = EvidenceRef::questionnaire(3, AXIS, Position::LeansHigh);

    let before = a1_axis_state(AXIS, &[first.clone(), second.clone(), third.clone()]);
    assert_eq!(before.state.band, Band::Strong);

    let after = a1_axis_state(AXIS, &[first, second, third.into_forgotten()]);

    assert_eq!(after.state.position, Position::LeansHigh);
    assert_eq!(after.state.band, Band::Moderate);
    assert_eq!(after.state.evidence_ids, vec![1, 2]);
}

/// A row that names no direction supports nothing, so it neither counts nor
/// blocks the upgrade.
#[test]
fn rows_claiming_unknown_are_dropped_before_aggregation() {
    let evidence = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh),
        EvidenceRef::inference(2, AXIS, Position::Unknown, Band::Strong),
        EvidenceRef::questionnaire(3, AXIS, Position::LeansHigh),
        EvidenceRef::questionnaire(4, AXIS, Position::LeansHigh),
    ];

    let outcome = a1_axis_state(AXIS, &evidence);

    assert_eq!(outcome.state.band, Band::Strong);
    assert_eq!(outcome.state.evidence_ids, vec![1, 3, 4]);
}

#[test]
fn a_correction_to_unknown_locks_at_band_none() {
    let evidence = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh),
        EvidenceRef::questionnaire(2, AXIS, Position::LeansHigh),
        EvidenceRef::questionnaire(3, AXIS, Position::LeansHigh),
        EvidenceRef::correction(4, AXIS, Position::Unknown),
    ];

    let outcome = a1_axis_state(AXIS, &evidence);

    assert_eq!(outcome.state.position, Position::Unknown);
    assert_eq!(outcome.state.band, Band::None);
    assert!(outcome.state.locked_by_user);
}

#[test]
fn evidence_for_another_axis_never_counts_toward_an_upgrade() {
    let evidence = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh),
        EvidenceRef::questionnaire(2, AxisId::Orderliness, Position::LeansHigh),
        EvidenceRef::questionnaire(3, AxisId::SocialEnergy, Position::LeansHigh),
    ];

    let outcome = a1_axis_state(AXIS, &evidence);

    assert_eq!(outcome.state.band, Band::Moderate);
    assert_eq!(outcome.state.evidence_ids, vec![1]);
}

#[test]
fn untouched_axes_stay_unknown() {
    let outcomes = a1_all_axes(&[EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh)]);

    for outcome in outcomes {
        if outcome.state.axis == AXIS {
            assert_eq!(outcome.state.band, Band::Moderate);
        } else {
            assert_eq!(outcome.state.position, Position::Unknown);
            assert_eq!(outcome.state.band, Band::None);
            assert!(outcome.state.evidence_ids.is_empty());
        }
    }
}
