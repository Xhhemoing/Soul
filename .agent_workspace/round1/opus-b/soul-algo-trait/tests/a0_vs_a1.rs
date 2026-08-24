//! Where A0 and A1 differ, pinned so the difference is a finding rather than a
//! surprise.
//!
//! A1 is meant to be a strict improvement on the baseline, so the cases where
//! it agrees matter as much as the cases where it does not: the agreements are
//! what makes it safe to swap in, and the disagreements are what a Round 2
//! arbitration is actually choosing between.

use soul_algo_trait::a0::a0_axis_state;
use soul_algo_trait::a1::a1_axis_state;
use soul_algo_trait::fixtures::axis_evidence_matrix;
use soul_algo_trait::{AxisId, Band, EvidenceRef, Position};

const AXIS: AxisId = AxisId::SocialEnergy;

/// Both algorithms lock, both refuse, and both cite the correction. Nothing
/// about the promise「用户纠正锁定，后续推断不覆盖」changes when A1 is swapped
/// in.
#[test]
fn the_lock_behaves_identically_under_both() {
    for (name, evidence) in axis_evidence_matrix() {
        let baseline = a0_axis_state(AXIS, &evidence);
        let counted = a1_axis_state(AXIS, &evidence);

        assert_eq!(
            baseline.state.locked_by_user, counted.state.locked_by_user,
            "{name}: the two algorithms disagree about whether the axis is locked"
        );

        if baseline.state.locked_by_user {
            assert_eq!(
                baseline.state.position, counted.state.position,
                "{name}: a locked axis must sit where the user put it"
            );
            assert_eq!(baseline.state.band, counted.state.band, "{name}");
        }
    }
}

/// Neither algorithm ever claims a direction without citing a row, and neither
/// ever cites a row that was forgotten.
#[test]
fn neither_algorithm_claims_anything_uncited() {
    for (name, evidence) in axis_evidence_matrix() {
        let forgotten: Vec<u64> = evidence
            .iter()
            .filter(|row| row.forgotten)
            .map(|row| row.evidence_id)
            .collect();

        for (label, outcome) in [
            ("A0", a0_axis_state(AXIS, &evidence)),
            ("A1", a1_axis_state(AXIS, &evidence)),
        ] {
            let state = outcome.state;
            if state.position == Position::Unknown && state.band == Band::None {
                continue;
            }
            assert!(
                !state.evidence_ids.is_empty(),
                "{name}/{label}: a claim with nothing behind it"
            );
            for id in &state.evidence_ids {
                assert!(
                    !forgotten.contains(id),
                    "{name}/{label}: cited forgotten row {id}"
                );
            }
        }
    }
}

/// **The finding.** Under the baseline, one thin machine inference silently
/// replaces the answer the user gave: `place_axis` in Goal 1's `service.rs`
/// overwrites position, band and citation in one go, so a `Weak` guess
/// downgrades a `Moderate` answer and takes its place in the evidence list.
/// A1 keeps both rows in view and reports the disagreement instead.
#[test]
fn a_weak_inference_overwrites_a_questionnaire_answer_under_a0_but_not_under_a1() {
    let evidence = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh),
        EvidenceRef::inference(2, AXIS, Position::LeansLow, Band::Weak),
    ];

    let baseline = a0_axis_state(AXIS, &evidence);
    assert_eq!(baseline.state.position, Position::LeansLow);
    assert_eq!(baseline.state.band, Band::Weak);
    assert_eq!(
        baseline.state.evidence_ids,
        vec![2],
        "the user's answer is no longer cited at all"
    );

    let counted = a1_axis_state(AXIS, &evidence);
    assert_eq!(counted.state.position, Position::Mixed);
    assert_eq!(counted.state.band, Band::Weak);
    assert_eq!(
        counted.state.evidence_ids,
        vec![1, 2],
        "both rows stay visible"
    );
}

/// The band upgrade is the other difference, and it only goes up when several
/// independent rows agree.
#[test]
fn only_a1_reaches_strong_without_a_correction() {
    let evidence = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh),
        EvidenceRef::questionnaire(2, AXIS, Position::LeansHigh),
        EvidenceRef::questionnaire(3, AXIS, Position::LeansHigh),
    ];

    assert_eq!(a0_axis_state(AXIS, &evidence).state.band, Band::Moderate);
    assert_eq!(a1_axis_state(AXIS, &evidence).state.band, Band::Strong);

    let single = vec![EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh)];
    assert_eq!(
        a1_axis_state(AXIS, &single).state.band,
        Band::Moderate,
        "one questionnaire answer is never strong"
    );
}

/// Replay order matters to A0 and not to A1, which is worth knowing before
/// either is put behind an importer that does not guarantee ordering.
#[test]
fn a1_is_order_independent_and_a0_is_not() {
    let first = EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh);
    let second = EvidenceRef::inference(2, AXIS, Position::LeansLow, Band::Moderate);

    let forward = vec![first.clone(), second.clone()];
    let reversed = vec![second, first];

    assert_ne!(
        a0_axis_state(AXIS, &forward).state.position,
        a0_axis_state(AXIS, &reversed).state.position
    );
    assert_eq!(
        a1_axis_state(AXIS, &forward).state.position,
        a1_axis_state(AXIS, &reversed).state.position
    );
}
