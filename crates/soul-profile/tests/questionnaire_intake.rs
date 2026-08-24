//! AC-03: after the questionnaire the profile is non-empty, and every field in
//! it came from the user.
//!
//! "Non-empty" is the easy half. The half worth testing is provenance: each
//! axis the questionnaire moved has to cite a `SoulEvidence` row that resolves,
//! is `kind: questionnaire`, and is `method: user_stated`. An axis that was
//! merely set would pass a shape check and fail the product's promise that the
//! user can ask why.
//!
//! Since WP13 the evidence is written by `soul-import`'s recorder — one
//! questionnaire, one set of rows — and the axes cite those rows. These tests
//! therefore also stand behind the merge: if the profile ever went back to
//! minting evidence of its own, the events asserted here would have nothing
//! pointing at them.

use serde_json::Value;
use uuid::Uuid;

use soul_profile::questionnaire::QuestionnaireResponse;
use soul_profile::{
    axes, intake, read_profile, read_voice, EmojiUse, ProfileError, VoiceDirectness, VoiceField,
    VoiceRegister, VoiceWarmth,
};
use soul_schema::common::{EvidenceBand, Subject};
use soul_schema::event::{EventKind, EventSource};
use soul_schema::evidence::{EvidenceKind, EvidenceMethod};
use soul_schema::profile::AxisPosition;
use soul_store_api::types::StoreError;
use soul_store_api::{AuditLog, BlobStore, EventStore, FakeStore, ProfileStore};
use soul_testkit::fixtures;

/// 2026-08-24T00:00:00Z. Passed in rather than read, so the audit entries are
/// the same on every run.
const NOW: i64 = 1_787_529_600;

fn answers(name: &str) -> QuestionnaireResponse {
    fixtures::read_json(&format!("questionnaire/{name}.json")).expect("load the fixture")
}

#[test]
fn a_completed_questionnaire_produces_a_profile_every_field_of_which_the_user_said() {
    let mut store = FakeStore::new();
    let profile_id = Uuid::now_v7();

    let outcome = intake(&mut store, profile_id, &answers("answers_basic"), NOW).expect("intake");

    assert_eq!(
        outcome.profile.trait_axes.len(),
        5,
        "the profile carries all five default axes",
    );
    assert_eq!(
        outcome.evidence_ids.len(),
        10,
        "one row per answered question: five axes, three voice fields, two of \
         the three text boxes; the blank one leaves nothing",
    );
    assert_eq!(outcome.event_ids.len(), outcome.evidence_ids.len());

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
            assert_eq!(evidence.exportable_to_research, Some(false));

            let source = &evidence.source_refs[0];
            assert_eq!(source["ref_kind"], "questionnaire_answer");
            assert!(
                source["question_key"].as_str().is_some(),
                "the evidence names the question that produced it: {source}",
            );
            assert!(
                source["option_key"].as_str().is_some(),
                "and, for a question the user chose from, which option: {source}",
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

/// Every answer is also an event, which is what the import side of the same
/// questionnaire always produced. One questionnaire, one set of rows.
#[test]
fn every_answer_is_an_event_whose_body_is_sealed() {
    let mut store = FakeStore::new();
    let response = answers("answers_basic");
    let outcome = intake(&mut store, Uuid::now_v7(), &response, NOW).expect("intake");

    for event_id in &outcome.event_ids {
        let event = store.get_event(*event_id).expect("event");
        assert_eq!(event.source, EventSource::UiQuestionnaire);
        assert_eq!(event.kind, EventKind::QuestionnaireAnswer);
        assert_eq!(event.ts.as_str(), response.answered_at);
        let sealed = event.body_ref.expect("the answer is sealed, not stored");
        assert!(
            sealed.placeholder.is_none(),
            "the user's own words are not third-party data and are not placeheld",
        );
        assert!(!store.open(&sealed).expect("open").is_empty());
    }

    let keys: Vec<Uuid> = outcome
        .event_ids
        .iter()
        .map(|id| {
            store
                .get_event(*id)
                .expect("event")
                .body_ref
                .expect("body")
                .content_key_id
        })
        .collect();
    assert!(
        keys.windows(2).all(|pair| pair[0] == pair[1]),
        "one key for the run, so \"forget what I told the wizard\" is one thing to destroy",
    );
}

/// The prose answers reach the profile as pointers. The words stay in the
/// sealed event: `docs/SECURITY.md` reserves prose for `sealedText`, and the
/// profiles table is not that.
#[test]
fn what_the_user_stated_lands_in_the_profile_without_landing_in_the_profile() {
    let mut store = FakeStore::new();
    let response = answers("answers_basic");
    let outcome = intake(&mut store, Uuid::now_v7(), &response, NOW).expect("intake");

    let boundaries = outcome.profile.boundaries.clone().expect("boundaries");
    assert_eq!(
        boundaries.len(),
        1,
        "two boundary questions, one of them left blank",
    );
    assert_eq!(boundaries[0]["question_id"], "q.boundary.topics");

    let values = outcome.profile.values.clone().expect("values");
    assert_eq!(values.len(), 1);
    assert_eq!(values[0]["question_id"], "q.value.what_matters");

    let stated: Vec<&str> = response
        .answers
        .iter()
        .filter_map(|answer| match answer {
            soul_profile::Answer::Prose { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .filter(|text| !text.trim().is_empty())
        .collect();
    assert_eq!(stated.len(), 2, "the fixture states two things");

    let encoded = serde_json::to_string(&outcome.profile).expect("serialize");
    for prose in stated {
        assert!(
            !encoded.contains(prose),
            "the profile repeated what the user typed",
        );
    }

    for entry in boundaries.iter().chain(values.iter()) {
        let evidence_id: Uuid = entry["evidence_id"]
            .as_str()
            .expect("an evidence id")
            .parse()
            .expect("a uuid");
        assert_eq!(
            store.get_evidence(evidence_id).expect("resolves").kind,
            EvidenceKind::Questionnaire,
        );
        let event_id: Uuid = entry["event_id"]
            .as_str()
            .expect("an event id")
            .parse()
            .expect("a uuid");
        assert!(store.get_event(event_id).is_ok(), "the pointer resolves");
    }
}

#[test]
fn the_voice_the_user_answered_is_pinned_and_the_rest_is_left_open() {
    let mut store = FakeStore::new();
    let profile_id = Uuid::now_v7();
    intake(&mut store, profile_id, &answers("answers_basic"), NOW).expect("intake");

    let voice = read_voice(&store, profile_id).expect("voice");
    assert_eq!(voice.register, VoiceRegister::Formal);
    assert_eq!(voice.directness, VoiceDirectness::Direct);
    assert_eq!(voice.emoji_use, EmojiUse::Never);
    for field in [
        VoiceField::Register,
        VoiceField::Directness,
        VoiceField::EmojiUse,
    ] {
        assert!(voice.is_locked(field), "{field:?} was answered");
    }

    assert_eq!(
        voice.warmth,
        VoiceWarmth::Even,
        "an unasked field keeps the neutral default",
    );
    assert!(
        !voice.is_locked(VoiceField::Warmth),
        "and stays open to inference, because the user never spoke about it",
    );
}

#[test]
fn an_unanswered_axis_stays_unknown_instead_of_being_guessed() {
    let mut store = FakeStore::new();
    let profile_id = Uuid::now_v7();

    let outcome = intake(&mut store, profile_id, &answers("answers_partial"), NOW).expect("intake");

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
    assert!(
        outcome.profile.boundaries.is_none() && outcome.profile.values.is_none(),
        "an unanswered text box leaves no placeholder either",
    );
}

#[test]
fn the_answers_that_must_not_land_do_not_land() {
    let cases: Value = fixtures::read_json("questionnaire/answers_rejected.json").expect("load");

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
            ("prose_for_a_choice", ProfileError::AnswerTargetMismatch { .. }) => {}
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
        assert!(
            store.list_evidence().expect("evidence").is_empty(),
            "case {id} must not leave evidence behind either",
        );
    }
}

/// A questionnaire whose every answer was left blank is an unanswered
/// questionnaire, not a profile of five `unknown` axes with an audit entry
/// saying something happened.
#[test]
fn a_questionnaire_answered_entirely_in_blanks_is_refused() {
    let mut store = FakeStore::new();
    let profile_id = Uuid::now_v7();
    let response = QuestionnaireResponse::new(
        "2026-08-24T09:30:00Z",
        vec![
            soul_profile::Answer::Prose {
                question_id: "q.boundary.topics".into(),
                text: "  ".into(),
            },
            soul_profile::Answer::Prose {
                question_id: "q.value.what_matters".into(),
                text: String::new(),
            },
        ],
    );

    assert!(matches!(
        intake(&mut store, profile_id, &response, NOW),
        Err(ProfileError::EmptyQuestionnaire),
    ));
    assert!(store.list_audit().expect("audit").is_empty());
    assert!(matches!(
        store.get_profile(profile_id),
        Err(StoreError::NotFound { .. })
    ));
}
