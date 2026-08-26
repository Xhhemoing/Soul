//! AC-20 through the session, on a store that has the owner's own prose in it.
//!
//! `session_screens.rs` already shows the preview writes no file. What it
//! cannot show is which rows reach the screen, because the only events in that
//! store are the ones the questionnaire wrote and nothing there separates "the
//! owner's" from "research may count this". Those are two different questions,
//! and until the rollup read `privacy.egress.research_export` it only asked the
//! first: an imported message and a questionnaire answer are both the owner's,
//! both stored `research_export: deny`, and both published.
//!
//! So this file imports a real corpus, answers a real questionnaire, and then
//! reads the preview the desktop shell would draw. The rows that survive are
//! the trait axes; the hours of somebody's imported chat history are not among
//! them, and neither is the fact that a question was answered at a particular
//! minute.

use std::path::PathBuf;

use soul_testkit::fixtures;
use soulcore::commands::profile::GivenAnswer;
use soulcore::commands::session::Session;

/// The corpus the flow imports. Three partners, and messages in both
/// directions, so the store ends up holding owner rows and third-party rows.
const IMPORT: &str = "import/soul-import-v1/three_partners.jsonl";

fn scratch() -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let canonical = std::fs::canonicalize(directory.path()).expect("canonical temp dir");
    (directory, canonical)
}

fn given(question_id: &str, answer: &str) -> GivenAnswer {
    GivenAnswer {
        question_id: question_id.to_owned(),
        given: answer.to_owned(),
    }
}

#[test]
fn neither_an_imported_message_nor_a_questionnaire_answer_reaches_the_research_preview() {
    let (keep, directory) = scratch();
    let mut session = Session::open(&directory);

    let text = fixtures::read_text(IMPORT).expect("the fixture is on disk");
    let receipt = session
        .commit_soul_import_v1(&text)
        .expect("a valid export commits");
    assert!(
        receipt.events_written > 0,
        "nothing was imported, so there is nothing for the preview to have withheld",
    );

    let intake = session
        .answer_questionnaire(&[
            given("q.axis.curiosity", "leans_high"),
            given("q.voice.register", "formal"),
            given("q.boundary.topics", "工作以外的事"),
        ])
        .expect("the core records what was answered");
    assert_eq!(intake.answered, 3);

    let research = session.research().expect("a preview");

    // Both kinds are the owner's own. A filter reading `privacy_subject` and
    // nothing else lets every one of them through.
    for withheld in ["import.item", "questionnaire.answer"] {
        assert!(
            research
                .rows
                .iter()
                .all(|row| row.event_kind.as_deref() != Some(withheld)),
            "`{withheld}` is stored `research_export: deny` and reached the preview: {:?}",
            research.rows,
        );
    }

    // And the preview is not empty as a side effect of that: the axis the
    // questionnaire produced is a band, which is what research may see.
    assert!(
        research
            .rows
            .iter()
            .any(|row| row.self_trait_axis.is_some()),
        "the answered axis is the row this preview is supposed to have: {:?}",
        research.rows,
    );

    assert!(
        research.third_party_rows_excluded > 0,
        "the corpus has other people's messages in it, so an exclusion ran or nothing was \
         looked at",
    );
    assert!(
        research.deny_rows_excluded > 0,
        "the import and the questionnaire both wrote denied owner rows; a zero here means \
         the disposition was never read",
    );
    assert_eq!(research.third_party_rows, 0);
    assert!(!research.written_to_disk);
    drop(keep);
}
