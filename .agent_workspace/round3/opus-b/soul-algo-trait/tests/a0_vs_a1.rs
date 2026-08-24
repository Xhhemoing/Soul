//! A0 against A1 on the same evidence: where they agree, what A1 would buy, and
//! what neither of them will do.
//!
//! This is the ablation table `REPORT.md` quotes. It runs rather than being
//! transcribed, so the report cannot drift from the code.
//!
//! Round 3 reads it as an argument for **not** retaining A1: on the v0.1 data
//! plane the retained default (A0 last-write-wins) and A1's default land on the
//! same band for every fixture a v0.1 install can produce. A candidate that
//! cannot be distinguished from the baseline on the data that exists is not
//! carrying its own weight yet. `a1_is_indistinguishable_from_a0_on_v01_data`
//! is that claim, run.

use soul_algo_trait::a0::{a0_axis_state_with, WriteMode, A0_DEFAULT_WRITE_MODE};
use soul_algo_trait::a1::{
    a1_axis_state, a1_axis_state_with, A1Independence, A1_DEFAULT_INDEPENDENCE,
};
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
fn on_the_retained_path_only_a_correction_reaches_strong() {
    // A0 is keeper #2 and A0 is not wired to A1, so this is the v0.1 meaning of
    // `Strong` in one assertion: the user looked at the claim and said so.
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
        "A1 would, on evidence v0.1 cannot produce: the reason it is written \
         down and the reason it is not retained"
    );
}

#[test]
fn a1_is_indistinguishable_from_a0_on_v01_data() {
    // The retention argument, run. Restrict every fixture to the evidence kinds
    // a v0.1 install can produce against an axis, and A1's default agrees with
    // the frozen A0 default on all five axes of all of them. A1 has nothing to
    // add until there is a second instrument, which is exactly why it ships
    // inert rather than as keeper #3.
    for (name, log) in axis_evidence_matrix() {
        let v01: Vec<EvidenceRef> = log
            .into_iter()
            .filter(|row| row.kind.reachable_for_axis_in_v01())
            .collect();

        for axis in AxisId::ALL {
            let a0 = a0_axis_state_with(axis, &v01, A0_DEFAULT_WRITE_MODE).state;
            let a1 = a1_axis_state(axis, &v01).state;
            assert_eq!(a0.band, a1.band, "{name}/{}", axis.key());
            assert_eq!(a0.position, a1.position, "{name}/{}", axis.key());
            assert_eq!(
                a0.locked_by_user,
                a1.locked_by_user,
                "{name}/{}",
                axis.key()
            );
        }
    }
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
    //
    // Columns 1 and 4 are the shipped defaults (`A0_DEFAULT_WRITE_MODE`,
    // `A1_DEFAULT_INDEPENDENCE`); columns 2 and 3 are the named alternatives.
    // `the_default_columns_are_the_shipped_defaults` keeps that mapping honest.
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
            "three_message_days_same_kind",
            Band::Weak,
            Band::Weak,
            Band::Strong,
            Band::Weak,
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
fn the_default_columns_are_the_shipped_defaults() {
    // The table above names its variants explicitly, which is what makes it a
    // readable ablation and also what would let it keep passing after a default
    // moved underneath it. This is the assertion that would not.
    assert_eq!(A0_DEFAULT_WRITE_MODE, WriteMode::LastWriteWins);
    assert_eq!(A1_DEFAULT_INDEPENDENCE, A1Independence::TwoKindsAcrossDays);

    for (name, log) in axis_evidence_matrix() {
        assert_eq!(
            a1_axis_state(AXIS, &log).state,
            a1_axis_state_with(AXIS, &log, A1_DEFAULT_INDEPENDENCE).state,
            "{name}"
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
