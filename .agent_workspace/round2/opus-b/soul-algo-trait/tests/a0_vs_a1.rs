//! A0 against A1 on the same evidence: where they agree, where A1 earns its
//! keep, and what neither of them will do.
//!
//! This is the ablation table `REPORT.md` quotes. It runs rather than being
//! transcribed, so the report cannot drift from the code.

use soul_algo_trait::a0::{a0_axis_state_with, WriteMode};
use soul_algo_trait::a1::{a1_axis_state, a1_axis_state_with, A1Independence};
use soul_algo_trait::fixtures::axis_evidence_matrix;
use soul_algo_trait::types::{AxisId, Band, EvidenceRef, Position};

const AXIS: AxisId = AxisId::SocialEnergy;

fn fixture(name: &str) -> Vec<EvidenceRef> {
    axis_evidence_matrix()
        .into_iter()
        .find(|(key, _)| *key == name)
        .unwrap_or_else(|| panic!("no fixture named {name}"))
        .1
}

#[test]
fn neither_algorithm_ever_moves_a_locked_axis() {
    for (name, log) in axis_evidence_matrix() {
        let locked = log.iter().any(|row| row.locked_by_user && !row.forgotten);
        if !locked {
            continue;
        }

        let last = log
            .iter()
            .filter(|row| row.locked_by_user && !row.forgotten && row.axis == AXIS)
            .next_back();
        let Some(last) = last else { continue };

        for mode in [WriteMode::LastWriteWins, WriteMode::NoDowngrade] {
            let state = a0_axis_state_with(AXIS, &log, mode).state;
            assert_eq!(state.position, last.position, "A0 {name} {mode:?}");
            assert!(state.locked_by_user, "A0 {name} {mode:?}");
        }

        let state = a1_axis_state(AXIS, &log).state;
        assert_eq!(state.position, last.position, "A1 {name}");
        assert!(state.locked_by_user, "A1 {name}");
    }
}

#[test]
fn neither_algorithm_ever_claims_without_citing() {
    for (name, log) in axis_evidence_matrix() {
        for mode in [WriteMode::LastWriteWins, WriteMode::NoDowngrade] {
            let state = a0_axis_state_with(AXIS, &log, mode).state;
            if state.position != Position::Unknown {
                assert!(!state.evidence_ids.is_empty(), "A0 {name} {mode:?}");
            }
        }

        let state = a1_axis_state(AXIS, &log).state;
        if state.position != Position::Unknown {
            assert!(!state.evidence_ids.is_empty(), "A1 {name}");
        }
    }
}

#[test]
fn only_a1_reaches_strong_without_a_correction() {
    for (name, log) in axis_evidence_matrix() {
        let corrected = log
            .iter()
            .any(|row| row.locked_by_user && !row.forgotten && row.axis == AXIS);
        if corrected {
            continue;
        }

        for mode in [WriteMode::LastWriteWins, WriteMode::NoDowngrade] {
            let state = a0_axis_state_with(AXIS, &log, mode).state;
            assert!(
                state.band != Band::Strong || state.locked_by_user,
                "A0 {name} {mode:?} reached Strong on its own"
            );
        }
    }

    assert_eq!(
        a1_axis_state(
            AXIS,
            &fixture("f16b_questionnaire_plus_three_independent_days")
        )
        .state
        .band,
        Band::Strong,
        "and A1 does, which is the only reason to keep it"
    );
}

#[test]
fn a1_and_a0_agree_when_there_is_only_one_thing_to_say() {
    for name in [
        "empty",
        "questionnaire_only",
        "questionnaire_then_correction",
        "inference_after_lock",
        "correction_to_unknown",
        "everything_forgotten",
    ] {
        let log = fixture(name);
        let a0 = a0_axis_state_with(AXIS, &log, WriteMode::LastWriteWins).state;
        let a1 = a1_axis_state(AXIS, &log).state;
        assert_eq!(a0.position, a1.position, "{name}");
        assert_eq!(a0.band, a1.band, "{name}");
        assert_eq!(a0.locked_by_user, a1.locked_by_user, "{name}");
    }
}

#[test]
fn the_ablation_table_is_what_the_report_says_it_is() {
    // (fixture, A0 LWW, A0 NoDowngrade, A1 KindAndDay, A1 TwoKindsAcrossDays)
    let expected = [
        (
            "questionnaire_only",
            Band::Moderate,
            Band::Moderate,
            Band::Moderate,
            Band::Moderate,
        ),
        (
            "questionnaire_then_correction",
            Band::Strong,
            Band::Strong,
            Band::Strong,
            Band::Strong,
        ),
        (
            "questionnaire_refilled_after_correction",
            Band::Strong,
            Band::Strong,
            Band::Strong,
            Band::Strong,
        ),
        (
            "f16_five_refills_same_day",
            Band::Moderate,
            Band::Moderate,
            Band::Moderate,
            Band::Moderate,
        ),
        (
            "f16b_questionnaire_plus_three_independent_days",
            Band::Weak,
            Band::Moderate,
            Band::Strong,
            Band::Strong,
        ),
        (
            "three_questionnaire_refills_on_three_days",
            Band::Moderate,
            Band::Moderate,
            Band::Strong,
            Band::Moderate,
        ),
        (
            "weak_inference_over_moderate_questionnaire",
            Band::Weak,
            Band::Moderate,
            Band::Weak,
            Band::Weak,
        ),
        (
            "contradiction",
            Band::Moderate,
            Band::Moderate,
            Band::Weak,
            Band::Weak,
        ),
    ];

    for (name, lww, no_downgrade, kind_and_day, two_kinds) in expected {
        let log = fixture(name);
        assert_eq!(
            a0_axis_state_with(AXIS, &log, WriteMode::LastWriteWins)
                .state
                .band,
            lww,
            "{name} under A0 LastWriteWins"
        );
        assert_eq!(
            a0_axis_state_with(AXIS, &log, WriteMode::NoDowngrade)
                .state
                .band,
            no_downgrade,
            "{name} under A0 NoDowngrade"
        );
        assert_eq!(
            a1_axis_state_with(AXIS, &log, A1Independence::KindAndDay)
                .state
                .band,
            kind_and_day,
            "{name} under A1 KindAndDay"
        );
        assert_eq!(
            a1_axis_state_with(AXIS, &log, A1Independence::TwoKindsAcrossDays)
                .state
                .band,
            two_kinds,
            "{name} under A1 TwoKindsAcrossDays"
        );
    }
}

#[test]
fn a1_is_the_path_that_does_not_clobber() {
    // The Round 1 defect from the other side: under A0's contract one Weak
    // guess erases the questionnaire's citation. A1 never had the problem,
    // because it re-derives the axis from the whole surviving set every time.
    let log = fixture("weak_inference_over_moderate_questionnaire");

    let a0 = a0_axis_state_with(AXIS, &log, WriteMode::LastWriteWins).state;
    assert_eq!(a0.evidence_ids, vec![2]);

    let a1 = a1_axis_state(AXIS, &log).state;
    assert_eq!(a1.position, Position::Mixed);
    assert_eq!(a1.evidence_ids, vec![1, 2], "both are still on the record");
}
