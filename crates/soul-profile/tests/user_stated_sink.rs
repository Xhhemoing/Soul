//! The seam itself: `soul-import` records, and this crate is what it hands the
//! answers to.
//!
//! `tests/questionnaire_intake.rs` proves the result. This file proves the
//! route, by driving `soul_import::questionnaire::record` directly — the same
//! call the import fallback makes — with `ProfileSink` in the sink slot, and
//! asserting that what comes out the other side is already profile vocabulary:
//! an axis and a position, a voice field and a setting.

use uuid::Uuid;

use soul_import::questionnaire::{self as recorder, Answer, QuestionnaireError, UserStatedSink};
use soul_profile::sink::{ProfileSink, StagedValue};
use soul_profile::voice::{VoiceField, VoiceRegister, VoiceSetting};
use soul_profile::{axes, ProfileSink as ReExported, QuestionTarget, StatedField};
use soul_schema::common::Timestamp;
use soul_schema::evidence::EvidenceMethod;
use soul_schema::profile::AxisPosition;
use soul_store_api::FakeStore;

const ANSWERED_AT: &str = "2026-08-24T09:30:00Z";

fn record(answers: &[Answer], sink: &mut dyn UserStatedSink) -> recorder::QuestionnaireReceipt {
    let mut store = FakeStore::new();
    recorder::record(&mut store, answers, &Timestamp::new(ANSWERED_AT), sink).expect("record")
}

#[test]
fn the_recorder_hands_answers_to_the_profile_in_the_profile_s_own_terms() {
    let mut sink = ProfileSink::new();
    let receipt = record(
        &[
            Answer::new("q.axis.curiosity", "leans_high"),
            Answer::new("q.voice.register", "formal"),
            Answer::new("q.boundary.topics", "家里人的事，别替我起草。"),
        ],
        &mut sink,
    );

    assert_eq!(receipt.answers.len(), 3);
    assert_eq!(sink.accepted().len(), 3);
    for recorded in &receipt.answers {
        assert_eq!(recorded.method, EvidenceMethod::UserStated);
    }

    let curiosity = sink.accepted()[0];
    assert_eq!(
        curiosity.target,
        QuestionTarget::Axis(axes::CURIOSITY),
        "the sink is what turns a question key into an axis",
    );
    assert_eq!(
        curiosity.value,
        StagedValue::Position(AxisPosition::LeansHigh),
    );
    assert_eq!(curiosity.evidence_id, receipt.answers[0].evidence_id);
    assert_eq!(curiosity.event_id, receipt.answers[0].event_id);

    assert_eq!(
        sink.accepted()[1].value,
        StagedValue::Voice(VoiceSetting::Register(VoiceRegister::Formal)),
    );
    assert_eq!(
        sink.accepted()[1].target,
        QuestionTarget::Voice(VoiceField::Register),
    );

    assert_eq!(
        sink.accepted()[2].target,
        QuestionTarget::Stated(StatedField::Boundary),
    );
    assert_eq!(
        sink.accepted()[2].value,
        StagedValue::Stated,
        "prose stays sealed in the event; the sink carries the pointer and no words",
    );

    // The same type, reachable from the crate root, because a caller wiring
    // the import fallback should not have to know which module it lives in.
    let _: ReExported = ProfileSink::new();
}

/// An option nobody offered never reaches the sink: the recorder refuses it
/// first, and says so without repeating what came in.
#[test]
fn an_option_the_question_does_not_offer_is_refused_before_the_profile_sees_it() {
    let mut store = FakeStore::new();
    let mut sink = ProfileSink::new();

    let error = recorder::record(
        &mut store,
        &[Answer::new("q.axis.curiosity", "五分里的四分")],
        &Timestamp::new(ANSWERED_AT),
        &mut sink,
    )
    .expect_err("not one of the three directions");

    assert!(matches!(error, QuestionnaireError::UnknownOption { .. }));
    assert!(
        !error.to_string().contains("五分里的四分"),
        "a refusal does not echo the answer: {error}",
    );
    assert!(sink.is_empty());
}

/// The refusal WP06 built the sink to allow, exercised on the case that would
/// actually happen: a question added to the recorder that the profile has no
/// field for. The seam is a refusal, not a shrug — an answer the profile
/// cannot place stops the run rather than being recorded and forgotten.
#[test]
fn a_question_the_profile_has_no_field_for_is_refused() {
    static ADDED_LATER: recorder::Question = recorder::Question {
        key: "q.axis.made_up",
        prompt: "一道这套档案还不认得的题",
        shape: recorder::AnswerShape::Choice(&["leans_high"]),
    };

    assert!(
        soul_profile::target_of(ADDED_LATER.key).is_none(),
        "the fixture would have caught it if this were a real question",
    );

    let refused = ProfileSink::new()
        .accept(&recorder::RecordedAnswer {
            event_id: Uuid::now_v7(),
            evidence_id: Uuid::now_v7(),
            method: EvidenceMethod::UserStated,
            question: &ADDED_LATER,
            choice: Some("leans_high"),
        })
        .expect_err("no field to put it in");
    assert_eq!(refused.question_key, ADDED_LATER.key);

    // And the shape has to match too: an option key for a text box is a
    // wizard and a profile disagreeing about what was asked.
    let mismatched = ProfileSink::new()
        .accept(&recorder::RecordedAnswer {
            event_id: Uuid::now_v7(),
            evidence_id: Uuid::now_v7(),
            method: EvidenceMethod::UserStated,
            question: recorder::question("q.boundary.topics").expect("a canonical question"),
            choice: Some("leans_high"),
        })
        .expect_err("a boundary is not chosen from a list");
    assert_eq!(mismatched.question_key, "q.boundary.topics");
}
