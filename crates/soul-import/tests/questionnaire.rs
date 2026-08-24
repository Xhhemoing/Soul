//! AC-03 on the import side: with no file to import, the questionnaire leaves
//! something behind, and everything it leaves says the user said it.
//!
//! What a profile *is* belongs to WP03. These tests hold the seam: the answers
//! become events and `user_stated` evidence, and the ids cross to whatever
//! builds the profile through [`UserStatedSink`].

use serde::Deserialize;

use soul_import::questionnaire::{
    self, Answer, CollectingSink, QuestionnaireError, UserStatedSink, QUESTIONNAIRE_REF_KIND,
    QUESTIONS,
};
use soul_schema::common::{Subject, Timestamp};
use soul_schema::event::{EventKind, EventSource};
use soul_schema::evidence::{EvidenceKind, EvidenceMethod};
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::{BlobStore, EventStore, ProfileStore};
use soul_testkit::fixtures;

const SEED: &str = "wp06 questionnaire";

#[derive(Debug, Deserialize)]
struct AnswerFixture {
    answered_at: String,
    answers: Vec<FixtureAnswer>,
}

#[derive(Debug, Deserialize)]
struct FixtureAnswer {
    question_key: String,
    text: String,
}

fn fixture() -> AnswerFixture {
    fixtures::read_json("import/questionnaire/answers_basic.json").expect("fixture")
}

fn answers(fixture: &AnswerFixture) -> Vec<Answer> {
    fixture
        .answers
        .iter()
        .map(|answer| Answer::new(&answer.question_key, &answer.text))
        .collect()
}

fn open(dir: &std::path::Path) -> SqlCipherStore {
    SqlCipherStore::open(dir.join("soul.db"), &TestKeyProvider::from_seed(SEED)).expect("open")
}

#[test]
fn the_questionnaire_is_the_path_when_there_is_no_file_and_when_the_file_is_empty() {
    assert!(questionnaire::fallback_needed(None));

    let staged = soul_import::soul_import_v1::parse(
        r#"{"type":"header","format":"soul-import-v1","version":1,"exported_at":"2026-08-24T08:00:00Z"}"#,
    )
    .expect("a header on its own is a valid file");
    assert!(
        questionnaire::fallback_needed(Some(&staged)),
        "an export with nothing in it leaves as little to work with as no export",
    );

    let text = fixtures::read_text("import/soul-import-v1/valid_basic.jsonl").expect("fixture");
    let real = soul_import::soul_import_v1::parse(&text).expect("valid");
    assert!(!questionnaire::fallback_needed(Some(&real)));
}

/// AC-03. Answers become rows, and every row says where it came from.
#[test]
fn finishing_the_questionnaire_leaves_user_stated_events_and_evidence() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fixture = fixture();
    let mut store = open(dir.path());
    let mut sink = CollectingSink::default();

    let receipt = questionnaire::record(
        &mut store,
        &answers(&fixture),
        &Timestamp::new(&fixture.answered_at),
        &mut sink,
    )
    .expect("record");

    // The fixture leaves one question blank on purpose: a question the user
    // skipped must leave no row at all.
    assert_eq!(receipt.answers.len(), fixture.answers.len() - 1);
    assert_eq!(sink.accepted.len(), receipt.answers.len());
    assert!(receipt.content_key_id.is_some());

    for recorded in &receipt.answers {
        assert_eq!(recorded.method, EvidenceMethod::UserStated);

        let event = store.get_event(recorded.event_id).expect("event");
        assert_eq!(event.source, EventSource::UiQuestionnaire);
        assert_eq!(event.kind, EventKind::QuestionnaireAnswer);
        assert_eq!(event.privacy.subject, Subject::Owner);
        assert_eq!(event.ts.as_str(), fixture.answered_at);
        assert!(
            event.body_ref.is_some(),
            "what the user typed is prose and is sealed like any other",
        );

        let evidence = store.get_evidence(recorded.evidence_id).expect("evidence");
        assert_eq!(evidence.kind, EvidenceKind::Questionnaire);
        assert_eq!(evidence.method, Some(EvidenceMethod::UserStated));
        assert_eq!(evidence.subject, Subject::Owner);
        assert_eq!(evidence.exportable_to_research, Some(false));

        let source_ref = &evidence.source_refs[0];
        assert_eq!(source_ref["ref_kind"], QUESTIONNAIRE_REF_KIND);
        assert_eq!(source_ref["question_key"], recorded.question.key);
        assert_eq!(
            source_ref["event_id"],
            recorded.event_id.to_string(),
            "the evidence points at the event it came from",
        );
    }
}

/// The answers are the user's own words about themselves, so they are sealed,
/// and one key covers the run: "forget what I told the wizard" is one thing to
/// destroy.
#[test]
fn the_answers_are_sealed_under_a_single_key_for_the_run() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fixture = fixture();
    let mut store = open(dir.path());
    let mut sink = CollectingSink::default();

    let receipt = questionnaire::record(
        &mut store,
        &answers(&fixture),
        &Timestamp::new(&fixture.answered_at),
        &mut sink,
    )
    .expect("record");
    let content_key_id = receipt.content_key_id.expect("a key was minted");

    for recorded in &receipt.answers {
        let event = store.get_event(recorded.event_id).expect("event");
        let sealed = event.body_ref.expect("body");
        assert_eq!(sealed.content_key_id, content_key_id);
        assert!(
            sealed.placeholder.is_none(),
            "the user's own words are not third-party data and are not placeheld",
        );
        let opened = String::from_utf8(store.open(&sealed).expect("open")).expect("utf-8");
        assert!(!opened.trim().is_empty());
    }
}

/// Stored evidence points at question keys, so a key this build does not ask
/// would be a dangling reference the moment WP03 read it.
#[test]
fn an_answer_to_a_question_this_build_does_not_ask_is_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path());
    let mut sink = CollectingSink::default();

    let error = questionnaire::record(
        &mut store,
        &[Answer::new("voice.astrology", "水瓶座")],
        &Timestamp::new("2026-08-24T09:30:00Z"),
        &mut sink,
        )
        .expect_err("unknown question");

    assert!(matches!(
        error,
        QuestionnaireError::UnknownQuestion { ref key } if key == "voice.astrology",
    ));
    assert!(sink.accepted.is_empty());
}

/// The seam is driven, not just offered. A sink that refuses an answer stops
/// the run rather than being ignored.
#[test]
fn a_sink_that_refuses_an_answer_stops_the_run() {
    #[derive(Debug, Default)]
    struct RefusingSink {
        seen: usize,
    }

    impl UserStatedSink for RefusingSink {
        fn accept(
            &mut self,
            answer: &soul_import::questionnaire::RecordedAnswer,
        ) -> Result<(), soul_import::questionnaire::SinkError> {
            self.seen += 1;
            Err(soul_import::questionnaire::SinkError {
                question_key: answer.question.key.to_owned(),
                reason: "the profile is locked by the user".into(),
            })
        }
    }

    let dir = tempfile::tempdir().expect("temp dir");
    let fixture = fixture();
    let mut store = open(dir.path());
    let mut sink = RefusingSink::default();

    let error = questionnaire::record(
        &mut store,
        &answers(&fixture),
        &Timestamp::new(&fixture.answered_at),
        &mut sink,
    )
    .expect_err("the sink refused");

    assert!(matches!(error, QuestionnaireError::Sink(_)));
    assert_eq!(sink.seen, 1, "the run stops at the first refusal");
}

/// Soul is not a medical product, so the wizard does not ask medical
/// questions. The denylist is the single source of truth for that vocabulary.
#[test]
fn no_question_reaches_for_diagnostic_vocabulary() {
    let terms = fixtures::denylist_terms().expect("denylist");
    for question in QUESTIONS {
        let lowered = question.prompt.to_lowercase();
        for term in &terms {
            assert!(
                !lowered.contains(&term.to_lowercase()),
                "`{}` asks about `{term}`",
                question.key,
            );
        }
    }
    assert!(QUESTIONS.len() >= 6, "a questionnaire of one is not one");
}
