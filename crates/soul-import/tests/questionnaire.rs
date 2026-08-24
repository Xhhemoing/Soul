//! AC-03 on the import side: with no file to import, the questionnaire leaves
//! something behind, and everything it leaves says the user said it.
//!
//! What a profile *is* belongs to WP03. These tests hold the seam: the answers
//! become events and `user_stated` evidence, and the ids cross to whatever
//! builds the profile through [`UserStatedSink`]. They also hold the half of
//! the merge this crate is responsible for — [`QUESTIONS`] is the one list, and
//! `fixtures/questionnaire/v0_1.json` says so from outside the source.

use serde_json::Value;

use soul_import::questionnaire::{
    self, Answer, AnswerShape, CollectingSink, QuestionnaireError, UserStatedSink,
    QUESTIONNAIRE_REF_KIND, QUESTIONS,
};
use soul_schema::common::{Subject, Timestamp};
use soul_schema::event::{EventKind, EventSource};
use soul_schema::evidence::{EvidenceKind, EvidenceMethod};
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::{BlobStore, EventStore, ProfileStore};
use soul_testkit::fixtures;

const SEED: &str = "wp06 questionnaire";

/// The answers fixture is written in the profile's typed form, because there
/// is one questionnaire and one set of answers to it. Flattening it here is
/// what this crate would receive from a wizard: a question key and either an
/// option token or what the user typed.
fn fixture() -> (String, Vec<Answer>) {
    let response: Value = fixtures::read_json("questionnaire/answers_basic.json").expect("fixture");
    let answered_at = response["answered_at"].as_str().expect("a time").to_owned();

    let answers = response["answers"]
        .as_array()
        .expect("answers")
        .iter()
        .map(|answer| {
            let key = answer["question_id"].as_str().expect("a question id");
            let given = match answer["kind"].as_str().expect("a kind") {
                "axis" => answer["position"].as_str().expect("a position"),
                "voice" => answer["setting"]["value"].as_str().expect("a setting"),
                "prose" => answer["text"].as_str().expect("text"),
                other => panic!("the fixture has a `{other}` answer this crate cannot flatten"),
            };
            Answer::new(key, given)
        })
        .collect();

    (answered_at, answers)
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

/// The merge, from this side. One list, and the fixture both crates read is
/// what says which one it is.
#[test]
fn the_questions_this_build_asks_are_the_ones_the_fixture_pins() {
    let defined: Value = fixtures::read_json("questionnaire/v0_1.json").expect("the question set");
    let defined = defined["questions"].as_array().expect("questions");

    assert_eq!(QUESTIONS.len(), defined.len());
    for (question, pinned) in QUESTIONS.iter().zip(defined) {
        assert_eq!(question.key, pinned["id"].as_str().expect("an id"));

        let (shape, options) = match question.shape {
            AnswerShape::Choice(options) => ("choice", options),
            AnswerShape::Prose => ("prose", &[] as &[&str]),
        };
        assert_eq!(shape, pinned["shape"].as_str().expect("a shape"));

        let pinned_options: Vec<&str> = pinned["options"]
            .as_array()
            .map(|list| {
                list.iter()
                    .map(|o| o.as_str().expect("an option"))
                    .collect()
            })
            .unwrap_or_default();
        assert_eq!(options, pinned_options.as_slice(), "{}", question.key);
    }

    let unique: std::collections::BTreeSet<&str> = QUESTIONS.iter().map(|q| q.key).collect();
    assert_eq!(unique.len(), QUESTIONS.len(), "a key is asked once");
}

/// AC-03. Answers become rows, and every row says where it came from.
#[test]
fn finishing_the_questionnaire_leaves_user_stated_events_and_evidence() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (answered_at, answers) = fixture();
    let mut store = open(dir.path());
    let mut sink = CollectingSink::default();

    let receipt = questionnaire::record(
        &mut store,
        &answers,
        &Timestamp::new(&answered_at),
        &mut sink,
    )
    .expect("record");

    // The fixture leaves one question blank on purpose: a question the user
    // skipped must leave no row at all.
    assert_eq!(receipt.answers.len(), answers.len() - 1);
    assert_eq!(sink.accepted.len(), receipt.answers.len());
    assert!(receipt.content_key_id.is_some());

    for recorded in &receipt.answers {
        assert_eq!(recorded.method, EvidenceMethod::UserStated);

        let event = store.get_event(recorded.event_id).expect("event");
        assert_eq!(event.source, EventSource::UiQuestionnaire);
        assert_eq!(event.kind, EventKind::QuestionnaireAnswer);
        assert_eq!(event.privacy.subject, Subject::Owner);
        assert_eq!(event.ts.as_str(), answered_at);
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

        match recorded.choice {
            Some(option) => {
                assert!(recorded.question.options().contains(&option));
                assert_eq!(
                    source_ref["option_key"], option,
                    "a chosen answer is explainable without opening the sealed body",
                );
            }
            None => {
                assert!(recorded.question.is_prose());
                assert!(source_ref.get("option_key").is_none());
            }
        }
    }
}

/// The answers are the user's own words about themselves, so they are sealed,
/// and one key covers the run: "forget what I told the wizard" is one thing to
/// destroy.
#[test]
fn the_answers_are_sealed_under_a_single_key_for_the_run() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (answered_at, answers) = fixture();
    let mut store = open(dir.path());
    let mut sink = CollectingSink::default();

    let receipt = questionnaire::record(
        &mut store,
        &answers,
        &Timestamp::new(&answered_at),
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

/// A choice question takes one of its options and nothing else. The old
/// question set was eight text boxes, so this is the mistake a wizard written
/// against it would make, and the refusal must not carry the mistake back.
#[test]
fn an_answer_that_is_not_one_of_the_offered_options_is_refused_without_being_quoted() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path());
    let mut sink = CollectingSink::default();
    let typed = "看情况吧，跟熟的人就直说";

    let error = questionnaire::record(
        &mut store,
        &[Answer::new("q.voice.directness", typed)],
        &Timestamp::new("2026-08-24T09:30:00Z"),
        &mut sink,
    )
    .expect_err("prose is not one of the three options");

    assert!(matches!(
        error,
        QuestionnaireError::UnknownOption { ref key, offered: 3 } if key == "q.voice.directness",
    ));
    assert!(
        !error.to_string().contains(typed),
        "a refusal does not repeat what the user wrote: {error}",
    );
    assert!(sink.accepted.is_empty());
    assert!(
        store.list_evidence().expect("evidence").is_empty(),
        "and nothing lands",
    );
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
    let (answered_at, answers) = fixture();
    let mut store = open(dir.path());
    let mut sink = RefusingSink::default();

    let error = questionnaire::record(
        &mut store,
        &answers,
        &Timestamp::new(&answered_at),
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
    assert!(
        QUESTIONS.len() <= 12,
        "a wizard nobody finishes fills in no profile at all",
    );
}
