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
    record_axis_inference, render, set_voice, suggest_voice, AxisProposal, IntakeSkip,
    VoiceDirectness, VoiceRegister, VoiceSetting,
};
use soul_schema::common::{EvidenceBand, Privacy, Purpose, SchemaVersion, Subject, SupportedBand};
use soul_schema::evidence::{EvidenceKind, EvidenceMethod, SoulEvidence};
use soul_schema::inference::InferenceMethod;
use soul_schema::profile::{AxisPosition, SoulProfile};
use soul_store_api::{EventStore, FakeStore, ProfileStore};

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

/// The other half of the lock, and the one the questionnaire used to walk
/// through: answering the wizard again is not a correction, so it does not get
/// to move an axis the user corrected. The answer is still recorded — the user
/// answered the question, and that is a fact about the user whatever the axis
/// does with it — and the run says which answers it refused and why.
#[test]
fn re_answering_the_questionnaire_does_not_move_a_corrected_axis() {
    let (mut store, profile_id) = seeded();
    correct_axis(
        &mut store,
        profile_id,
        axes::CURIOSITY.axis_id,
        AxisPosition::LeansLow,
        NOW,
    )
    .expect("correction");
    let corrected_evidence = read_profile(&store, profile_id)
        .expect("profile")
        .trait_axes
        .iter()
        .find(|axis| axis.axis_id == axes::CURIOSITY.axis_id)
        .and_then(|axis| axis.evidence_ids.clone())
        .expect("the correction is cited");

    let response =
        soul_profile::questionnaire::every_axis(AxisPosition::LeansHigh, "2026-08-25T09:00:00Z");
    let outcome = intake(&mut store, profile_id, &response, NOW).expect("intake");

    assert_eq!(
        outcome.evidence_ids.len(),
        5,
        "all five answers were recorded, including the one that lost",
    );

    let ignored = match outcome.ignored.as_slice() {
        [only] => *only,
        other => panic!("exactly one answer hit a locked axis, got {other:?}"),
    };
    assert_eq!(ignored.axis_id, axes::CURIOSITY.axis_id);
    assert_eq!(ignored.question_id, axes::CURIOSITY.question_id);
    assert_eq!(ignored.position, AxisPosition::LeansHigh);
    assert_eq!(ignored.reason, IntakeSkip::AxisLockedByUser);
    assert_eq!(ignored.reason.as_str(), "axis_locked_by_user");
    assert!(outcome.ignored_any());
    assert_eq!(outcome.ignored_ids(), vec![ignored.evidence_id]);
    assert!(
        outcome.evidence_ids.contains(&ignored.evidence_id),
        "the refused answer is one of the run's rows, not a row that went missing",
    );

    let evidence = store
        .get_evidence(ignored.evidence_id)
        .expect("the refused answer's evidence row exists and resolves");
    assert_eq!(evidence.kind, EvidenceKind::Questionnaire);
    assert_eq!(evidence.method, Some(EvidenceMethod::UserStated));
    assert!(
        store.get_event(ignored.event_id).is_ok(),
        "and so does the event its words are sealed in",
    );

    let profile = read_profile(&store, profile_id).expect("profile");
    let curiosity = profile
        .trait_axes
        .iter()
        .find(|axis| axis.axis_id == axes::CURIOSITY.axis_id)
        .expect("axis");
    assert_eq!(
        curiosity.position,
        AxisPosition::LeansLow,
        "the correction stands",
    );
    assert_eq!(curiosity.locked_by_user, Some(true), "and so does the lock");
    assert_eq!(
        curiosity.evidence_band,
        EvidenceBand::Strong,
        "the axis was not quietly downgraded to what a questionnaire is worth",
    );
    assert_eq!(
        curiosity.evidence_ids.as_ref(),
        Some(&corrected_evidence),
        "it still cites the correction, not the answer that lost",
    );

    let orderliness = profile
        .trait_axes
        .iter()
        .find(|axis| axis.axis_id == axes::ORDERLINESS.axis_id)
        .expect("axis");
    assert_eq!(
        orderliness.position,
        AxisPosition::LeansHigh,
        "an axis the user never corrected still moves; the lock is per axis",
    );
    assert_eq!(orderliness.evidence_band, EvidenceBand::Moderate);
    assert_eq!(
        outcome.profile, profile,
        "what the run returned is what was written",
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
