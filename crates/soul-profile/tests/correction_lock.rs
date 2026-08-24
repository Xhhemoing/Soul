//! User corrections win, and keep winning.
//!
//! PRODUCT_LOCK's fourth slice item is "用户纠正锁定，后续推断不覆盖；起草语气立即改变",
//! and AC-07 is the drafting half of it. Both halves are here: an axis the user
//! corrected is not moved by a later, stronger inference, and a voice field the
//! user set is what the profile read returns — which is what WP10 will draft
//! from.
//!
//! The inference that gets refused is still stored. That is deliberate: the
//! user is entitled to see that the machine still disagrees, and dropping it
//! would make the audit entry a lie.

use uuid::Uuid;

use soul_profile::service::AxisUpdate;
use soul_profile::{
    axes, axis_is_locked, correct_axis, intake, profile_view, read_profile, read_voice,
    record_axis_inference, render, set_voice, suggest_voice, AxisProposal, VoiceDirectness,
    VoiceRegister, VoiceSetting,
};
use soul_schema::common::{EvidenceBand, Privacy, Purpose, SchemaVersion, Subject, SupportedBand};
use soul_schema::evidence::{EvidenceKind, EvidenceMethod, SoulEvidence};
use soul_schema::inference::InferenceMethod;
use soul_schema::profile::{AxisPosition, SoulProfile};
use soul_store_api::{FakeStore, ProfileStore};

const NOW: i64 = 1_787_529_600;

fn observed_evidence(store: &mut FakeStore) -> Uuid {
    let evidence_id = Uuid::now_v7();
    store
        .put_evidence(SoulEvidence {
            schema_version: SchemaVersion,
            evidence_id,
            kind: EvidenceKind::Message,
            subject: Subject::Owner,
            source_refs: vec![serde_json::json!({ "origin": "wp03-test", "count": 3 })],
            strength: SupportedBand::Strong,
            method: Some(EvidenceMethod::Heuristic),
            exportable_to_research: Some(false),
            privacy: Some(Privacy::local_only(
                Subject::Owner,
                vec![Purpose::SoulProfile],
            )),
        })
        .expect("evidence");
    evidence_id
}

fn position(profile: &SoulProfile, axis_id: Uuid) -> AxisPosition {
    profile
        .trait_axes
        .iter()
        .find(|axis| axis.axis_id == axis_id)
        .expect("axis is present")
        .position
}

fn seeded() -> (FakeStore, Uuid) {
    let mut store = FakeStore::new();
    let profile_id = Uuid::now_v7();
    let response =
        soul_profile::questionnaire::every_axis(AxisPosition::Mixed, "2026-08-24T09:00:00Z");
    intake(&mut store, profile_id, &response, NOW).expect("intake");
    (store, profile_id)
}

#[test]
fn an_unlocked_axis_moves_when_evidence_says_so() {
    let (mut store, profile_id) = seeded();
    let evidence_id = observed_evidence(&mut store);

    let outcome = record_axis_inference(
        &mut store,
        profile_id,
        AxisProposal::new(
            axes::CURIOSITY.axis_id,
            AxisPosition::LeansHigh,
            SupportedBand::Moderate,
            vec![evidence_id],
        )
        .by(InferenceMethod::Rule),
        NOW,
    )
    .expect("inference");

    assert_eq!(outcome.update, AxisUpdate::Applied);
    let profile = read_profile(&store, profile_id).expect("profile");
    assert_eq!(
        position(&profile, axes::CURIOSITY.axis_id),
        AxisPosition::LeansHigh
    );
    assert!(!axis_is_locked(&profile, axes::CURIOSITY.axis_id));
}

#[test]
fn a_corrected_axis_is_not_overwritten_by_a_later_stronger_inference() {
    let (mut store, profile_id) = seeded();

    correct_axis(
        &mut store,
        profile_id,
        axes::CURIOSITY.axis_id,
        AxisPosition::LeansLow,
        NOW,
    )
    .expect("correction");

    let profile = read_profile(&store, profile_id).expect("profile");
    assert!(axis_is_locked(&profile, axes::CURIOSITY.axis_id));
    let corrected_axis = profile
        .trait_axes
        .iter()
        .find(|axis| axis.axis_id == axes::CURIOSITY.axis_id)
        .expect("axis");
    assert_eq!(corrected_axis.evidence_band, EvidenceBand::Strong);
    let correction_evidence = store
        .get_evidence(corrected_axis.evidence_ids.as_ref().expect("cites")[0])
        .expect("the correction is itself evidence, and it resolves");
    assert_eq!(correction_evidence.kind, EvidenceKind::UserCorrection);
    assert_eq!(correction_evidence.method, Some(EvidenceMethod::UserStated));

    let evidence_id = observed_evidence(&mut store);
    let outcome = record_axis_inference(
        &mut store,
        profile_id,
        AxisProposal::new(
            axes::CURIOSITY.axis_id,
            AxisPosition::LeansHigh,
            SupportedBand::Strong,
            vec![evidence_id],
        ),
        NOW,
    )
    .expect("the inference is stored even though it will not be applied");
    assert_eq!(outcome.update, AxisUpdate::RefusedAxisLocked);

    let profile = read_profile(&store, profile_id).expect("profile");
    assert_eq!(
        position(&profile, axes::CURIOSITY.axis_id),
        AxisPosition::LeansLow,
        "the correction stands",
    );
    assert!(axis_is_locked(&profile, axes::CURIOSITY.axis_id));
    assert_eq!(
        store
            .get_evidence(
                profile
                    .trait_axes
                    .iter()
                    .find(|axis| axis.axis_id == axes::CURIOSITY.axis_id)
                    .expect("axis")
                    .evidence_ids
                    .as_ref()
                    .expect("cites")[0]
            )
            .expect("resolves")
            .kind,
        EvidenceKind::UserCorrection,
        "the axis still cites the correction, not the inference that lost",
    );

    let stored = store
        .get_inference(outcome.inference_id)
        .expect("the refused inference is kept, so the user can see the disagreement");
    assert_eq!(stored.evidence_ids, vec![evidence_id]);
}

#[test]
fn locking_one_axis_does_not_lock_the_others() {
    let (mut store, profile_id) = seeded();
    correct_axis(
        &mut store,
        profile_id,
        axes::CURIOSITY.axis_id,
        AxisPosition::LeansLow,
        NOW,
    )
    .expect("correction");

    let evidence_id = observed_evidence(&mut store);
    let outcome = record_axis_inference(
        &mut store,
        profile_id,
        AxisProposal::new(
            axes::ORDERLINESS.axis_id,
            AxisPosition::LeansHigh,
            SupportedBand::Weak,
            vec![evidence_id],
        ),
        NOW,
    )
    .expect("inference");

    assert_eq!(outcome.update, AxisUpdate::Applied);
    let profile = read_profile(&store, profile_id).expect("profile");
    assert_eq!(
        position(&profile, axes::ORDERLINESS.axis_id),
        AxisPosition::LeansHigh,
    );
    assert_eq!(
        position(&profile, axes::CURIOSITY.axis_id),
        AxisPosition::LeansLow,
    );
}

/// AC-07, profile side: the read a draft is built from returns the user's
/// value, and nothing inferred can take it back.
#[test]
fn a_voice_field_the_user_set_is_what_the_profile_read_returns() {
    let (mut store, profile_id) = seeded();

    set_voice(
        &mut store,
        profile_id,
        VoiceSetting::Directness(VoiceDirectness::Reserved),
        NOW,
    )
    .expect("the user sets the tone");
    assert_eq!(
        read_voice(&store, profile_id).expect("voice").directness,
        VoiceDirectness::Reserved,
    );

    let applied = suggest_voice(
        &mut store,
        profile_id,
        VoiceSetting::Directness(VoiceDirectness::Direct),
        NOW,
    )
    .expect("suggestion is not an error, just a refusal");
    assert!(!applied, "inference must not move a field the user set");
    assert_eq!(
        read_voice(&store, profile_id).expect("voice").directness,
        VoiceDirectness::Reserved,
    );

    let applied = suggest_voice(
        &mut store,
        profile_id,
        VoiceSetting::Register(VoiceRegister::Formal),
        NOW,
    )
    .expect("suggestion");
    assert!(applied, "a field the user never touched is still open");
    assert_eq!(
        read_voice(&store, profile_id).expect("voice").register,
        VoiceRegister::Formal,
    );

    let view = profile_view(&store, profile_id).expect("view");
    assert_eq!(view.voice.directness, VoiceDirectness::Reserved);
    let rendered = render(&view).expect("render");
    assert!(
        rendered.contains("含蓄"),
        "the reading a draft is built from shows the user's value: {rendered}",
    );
}
