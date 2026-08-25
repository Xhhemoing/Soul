//! AC-03: after the questionnaire the profile is non-empty, and every field in
//! it came from the user.
//!
//! "Non-empty" is the easy half. The half worth testing is provenance: each
//! axis the questionnaire moved has to cite a `SoulEvidence` row that resolves,
//! is `kind: questionnaire`, and is `method: user_stated`. An axis that was
//! merely set would pass a shape check and fail the product's promise that the
//! user can ask why.

use serde_json::Value;
use uuid::Uuid;

use soul_profile::questionnaire::QuestionnaireResponse;
use soul_profile::{
    axes, intake, read_profile, read_voice, EmojiUse, ProfileError, VoiceDirectness, VoiceField,
    VoiceRegister,
};
use soul_schema::common::{EvidenceBand, Subject};
use soul_schema::evidence::{EvidenceKind, EvidenceMethod};
use soul_schema::profile::AxisPosition;
use soul_store_api::types::StoreError;
use soul_store_api::{AuditLog, FakeStore, ProfileStore};
use soul_testkit::fixtures;

/// 2026-08-24T00:00:00Z. Passed in rather than read, so the audit entries are
/// the same on every run.
const NOW: i64 = 1_787_529_600;

fn answers(name: &str) -> QuestionnaireResponse {
    fixtures::read_json(&format!("profile/{name}.json")).expect("load the questionnaire fixture")
}

#[test]
fn a_completed_questionnaire_produces_a_profile_every_field_of_which_the_user_said() {
    let mut store = FakeStore::new();
    let profile_id = Uuid::now_v7();

    let outcome = intake(
        &mut store,
        profile_id,
        &answers("questionnaire_answers_basic"),
        NOW,
    )
    .expect("intake");

    assert_eq!(
        outcome.profile.trait_axes.len(),
        5,
        "the profile carries all five default axes",
    );
    assert_eq!(
        outcome.evidence_ids.len(),
        7,
        "one evidence row per answer, five axes plus two voice fields",
    );

    for axis in &outcome.profile.trait_axes {
        assert_ne!(
            axis.position,
            AxisPosition::Unknown,
            "every axis in this fixture was answered",
        );
        assert_eq!(
            axis.evidence_band,
            EvidenceBand::Moderate,
            "a questionnaire answer is moderate support, not strong",
        );
        assert_eq!(
            axis.locked_by_user,
            Some(false),
            "answering is not correcting; the axis stays open to later evidence",
        );

        let cited = axis
            .evidence_ids
            .clone()
            .unwrap_or_else(|| panic!("axis {} cites nothing", axis.axis_id));
        assert!(!cited.is_empty());
        for evidence_id in cited {
            let evidence = store
                .get_evidence(evidence_id)
                .expect("every cited evidence id resolves");
            assert_eq!(evidence.kind, EvidenceKind::Questionnaire);
            assert_eq!(evidence.method, Some(EvidenceMethod::UserStated));
            assert_eq!(evidence.subject, Subject::Owner);
            assert!(
                !evidence.source_refs.is_empty(),
                "the evidence points back at the question that produced it",
            );
        }
    }

    let curiosity = outcome
        .profile
        .trait_axes
        .iter()
        .find(|axis| axis.axis_id == axes::CURIOSITY.axis_id)
        .expect("the curiosity axis");
    assert_eq!(
        curiosity.position,
        AxisPosition::LeansHigh,
        "the fixture answered this one leans_high",
    );

    assert_eq!(
        read_profile(&store, profile_id).expect("read back"),
        outcome.profile,
        "what was written is what comes back",
    );
}

#[test]
fn the_voice_the_user_answered_is_pinned_and_the_rest_is_left_open() {
    let mut store = FakeStore::new();
    let profile_id = Uuid::now_v7();
    intake(
        &mut store,
        profile_id,
        &answers("questionnaire_answers_basic"),
        NOW,
    )
    .expect("intake");

    let voice = read_voice(&store, profile_id).expect("voice");
    assert_eq!(voice.directness, VoiceDirectness::Direct);
    assert_eq!(voice.emoji_use, EmojiUse::Never);
    assert!(voice.is_locked(VoiceField::Directness));
    assert!(voice.is_locked(VoiceField::EmojiUse));

    assert_eq!(
        voice.register,
        VoiceRegister::Plain,
        "an unasked field keeps the neutral default",
    );
    assert!(
        !voice.is_locked(VoiceField::Register),
        "and stays open to inference, because the user never spoke about it",
    );
}

#[test]
fn an_unanswered_axis_stays_unknown_instead_of_being_guessed() {
    let mut store = FakeStore::new();
    let profile_id = Uuid::now_v7();

    let outcome = intake(
        &mut store,
        profile_id,
        &answers("questionnaire_answers_partial"),
        NOW,
    )
    .expect("intake");

    let placed = outcome
        .profile
        .trait_axes
        .iter()
        .filter(|axis| axis.position != AxisPosition::Unknown)
        .count();
    assert_eq!(placed, 2, "the fixture answers two of the five");

    for axis in outcome
        .profile
        .trait_axes
        .iter()
        .filter(|axis| axis.position == AxisPosition::Unknown)
    {
        assert_eq!(axis.evidence_band, EvidenceBand::None);
        assert!(
            axis.evidence_ids.is_none(),
            "an unanswered axis cites nothing, rather than citing something weakly",
        );
    }
}

#[test]
fn the_answers_that_must_not_land_do_not_land() {
    let cases: Value =
        fixtures::read_json("profile/questionnaire_answers_rejected.json").expect("load");

    for case in cases["cases"].as_array().expect("cases") {
        let id = case["id"].as_str().expect("case id");
        let parsed = serde_json::from_value::<QuestionnaireResponse>(case["response"].clone());

        if id == "numeric_position" {
            assert!(
                parsed.is_err(),
                "a numeric answer must not even deserialize: a direction axis has no scale",
            );
            continue;
        }

        let response = parsed.unwrap_or_else(|e| panic!("case {id} should parse: {e}"));
        let mut store = FakeStore::new();
        let profile_id = Uuid::now_v7();
        let error = intake(&mut store, profile_id, &response, NOW)
            .err()
            .unwrap_or_else(|| panic!("case {id} was accepted"));

        match (id, &error) {
            ("unknown_question", ProfileError::UnknownQuestion(_)) => {}
            ("target_mismatch", ProfileError::AnswerTargetMismatch { .. }) => {}
            ("empty", ProfileError::EmptyQuestionnaire) => {}
            _ => panic!("case {id} produced the wrong error: {error}"),
        }

        assert!(
            matches!(
                store.get_profile(profile_id),
                Err(StoreError::NotFound { .. })
            ),
            "case {id} must leave no half-written profile behind",
        );
        assert!(
            store.list_audit().expect("audit").is_empty(),
            "case {id} must not record a completed intake",
        );
    }
}
