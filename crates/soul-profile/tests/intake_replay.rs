//! Cross-crate lockstep test for the product intake path and frozen A0 replay.

use uuid::Uuid;

use soul_algo_trait::a0::{
    a0_all_axes_with, apply_intake, IntakeAnswer, IntakeSkip as ReplayIntakeSkip, WriteMode,
};
use soul_algo_trait::types::{AxisId, AxisState, Band, EvidenceRef, Position};
use soul_profile::{axes, correct_axis, intake, IntakeSkip as ProductIntakeSkip, DEFAULT_AXES};
use soul_schema::common::EvidenceBand;
use soul_schema::profile::{AxisPosition, SoulProfile, TraitAxis};
use soul_store_api::FakeStore;

const INITIAL_AT: i64 = 1_787_529_600;
const CORRECTION_AT: i64 = INITIAL_AT + 60;
const REFILL_AT: i64 = INITIAL_AT + 120;

fn dense_axis_id(axis_id: Uuid) -> u64 {
    DEFAULT_AXES
        .iter()
        .position(|axis| axis.axis_id == axis_id)
        .map(|index| index as u64 + 1)
        .expect("product axis belongs to the canonical questionnaire")
}

fn replay_axis_id(axis_id: Uuid) -> AxisId {
    AxisId::ALL[(dense_axis_id(axis_id) - 1) as usize]
}

fn dense_evidence_id(canonical_order: &[Uuid], evidence_id: Uuid) -> u64 {
    canonical_order
        .iter()
        .position(|candidate| *candidate == evidence_id)
        .map(|index| index as u64 + 1)
        .expect("product evidence was interned before replay")
}

fn replay_position(position: AxisPosition) -> Position {
    match position {
        AxisPosition::LeansLow => Position::LeansLow,
        AxisPosition::Mixed => Position::Mixed,
        AxisPosition::LeansHigh => Position::LeansHigh,
        AxisPosition::Unknown => Position::Unknown,
    }
}

fn replay_band(band: EvidenceBand) -> Band {
    match band {
        EvidenceBand::None => Band::None,
        EvidenceBand::Weak => Band::Weak,
        EvidenceBand::Moderate => Band::Moderate,
        EvidenceBand::Strong => Band::Strong,
    }
}

fn product_axis(profile: &SoulProfile, axis_id: Uuid) -> &TraitAxis {
    profile
        .trait_axes
        .iter()
        .find(|axis| axis.axis_id == axis_id)
        .expect("canonical axis is present")
}

fn assert_same_state(product: &TraitAxis, replay: &AxisState) {
    assert_eq!(replay.axis, replay_axis_id(product.axis_id));
    assert_eq!(replay.position, replay_position(product.position));
    assert_eq!(replay.locked_by_user, product.locked_by_user == Some(true));
    assert_eq!(replay.band, replay_band(product.evidence_band));
}

#[test]
fn product_intake_matches_frozen_a0_after_a_user_correction() {
    let mut store = FakeStore::new();
    let profile_id = Uuid::now_v7();

    let initial_response =
        soul_profile::questionnaire::every_axis(AxisPosition::Mixed, "2026-08-24T09:00:00Z");
    let initial =
        intake(&mut store, profile_id, &initial_response, INITIAL_AT).expect("initial intake");

    // Keep the correction inside the questionnaire-reachable positions. The
    // lower-level APIs diverge for `Unknown`: product `correct_axis` stores a
    // Strong correction while A0 normalizes Unknown to Band::None. Soulcore
    // filters that input, so this product-path equality test must not "fix" or
    // accidentally freeze the unreachable API difference.
    let corrected = correct_axis(
        &mut store,
        profile_id,
        axes::CURIOSITY.axis_id,
        AxisPosition::LeansLow,
        CORRECTION_AT,
    )
    .expect("user correction");
    let correction_evidence_id = product_axis(&corrected, axes::CURIOSITY.axis_id)
        .evidence_ids
        .as_ref()
        .and_then(|ids| ids.first())
        .copied()
        .expect("correction cites its evidence");

    let refill_response =
        soul_profile::questionnaire::every_axis(AxisPosition::LeansHigh, "2026-08-25T09:00:00Z");
    let refill = intake(&mut store, profile_id, &refill_response, REFILL_AT).expect("refill");

    assert_eq!(initial.evidence_ids.len(), DEFAULT_AXES.len());
    assert_eq!(refill.evidence_ids.len(), DEFAULT_AXES.len());

    // UUID values are intentionally random. Intern chronologically, and use
    // canonical questionnaire order within each intake batch, to get stable
    // dense ids without sorting by random UUID bytes.
    let mut evidence_order = initial.evidence_ids.clone();
    evidence_order.push(correction_evidence_id);
    evidence_order.extend(refill.evidence_ids.iter().copied());

    let mut prior_log: Vec<EvidenceRef> = DEFAULT_AXES
        .iter()
        .zip(&initial.evidence_ids)
        .map(|(axis, evidence_id)| {
            EvidenceRef::questionnaire(
                dense_evidence_id(&evidence_order, *evidence_id),
                replay_axis_id(axis.axis_id),
                Position::Mixed,
                INITIAL_AT,
            )
        })
        .collect();
    prior_log.push(EvidenceRef::correction(
        dense_evidence_id(&evidence_order, correction_evidence_id),
        replay_axis_id(axes::CURIOSITY.axis_id),
        Position::LeansLow,
        CORRECTION_AT,
    ));

    let replay_answers: Vec<IntakeAnswer> = DEFAULT_AXES
        .iter()
        .zip(&refill.evidence_ids)
        .map(|(axis, evidence_id)| {
            IntakeAnswer::new(
                dense_evidence_id(&evidence_order, *evidence_id),
                replay_axis_id(axis.axis_id),
                Position::LeansHigh,
                REFILL_AT,
            )
        })
        .collect();
    let replay_report = apply_intake(&prior_log, &replay_answers, WriteMode::LastWriteWins);

    let replayed_states: Vec<AxisState> =
        a0_all_axes_with(&replay_report.log_after, WriteMode::LastWriteWins)
            .into_iter()
            .map(|outcome| outcome.state)
            .collect();
    assert_eq!(
        replay_report.axes, replayed_states,
        "imperative A0 intake and full replay must agree"
    );

    for axis in DEFAULT_AXES {
        let product = product_axis(&refill.profile, axis.axis_id);
        assert_same_state(product, replay_report.axis(replay_axis_id(axis.axis_id)));
        let replayed = replayed_states
            .iter()
            .find(|state| state.axis == replay_axis_id(axis.axis_id))
            .expect("replay returns every canonical axis");
        assert_same_state(product, replayed);
    }

    assert!(refill.ignored.iter().any(|ignored| {
        ignored.axis_id == axes::CURIOSITY.axis_id
            && ignored.reason == ProductIntakeSkip::AxisLockedByUser
    }));
    assert!(replay_report.ignored.iter().any(|ignored| {
        ignored.axis == replay_axis_id(axes::CURIOSITY.axis_id)
            && ignored.reason == ReplayIntakeSkip::AxisLockedByUser
    }));

    let product_ignored: Vec<_> = refill
        .ignored
        .iter()
        .map(|ignored| {
            (
                dense_evidence_id(&evidence_order, ignored.evidence_id),
                replay_axis_id(ignored.axis_id),
                replay_position(ignored.position),
                ignored.reason.as_str(),
            )
        })
        .collect();
    let replay_ignored: Vec<_> = replay_report
        .ignored
        .iter()
        .map(|ignored| {
            (
                ignored.evidence_id,
                ignored.axis,
                ignored.position,
                ignored.reason.as_str(),
            )
        })
        .collect();
    assert_eq!(product_ignored, replay_ignored);

    let product_applied: Vec<u64> = refill
        .evidence_ids
        .iter()
        .filter(|evidence_id| {
            !refill
                .ignored
                .iter()
                .any(|ignored| ignored.evidence_id == **evidence_id)
        })
        .map(|evidence_id| dense_evidence_id(&evidence_order, *evidence_id))
        .collect();
    assert_eq!(product_applied, replay_report.applied);
}
