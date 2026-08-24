//! The four screens WP09 left empty, driven through the session the desktop
//! shell actually holds.
//!
//! `profile_memory_commands.rs` already checks what `soul-profile` and
//! `soul-memory` do to a store. What is only checkable here is the layer the
//! WebView binds to: that answering the questionnaire through a [`Session`]
//! leaves a profile behind (AC-03 through the shell rather than headless),
//! that the questions the shell draws are the ones the recorder will accept,
//! that a forget cannot happen on a preview nobody read, and that the research
//! and audit reads are the shapes their acceptance criteria describe.

use std::path::PathBuf;

use soulcore::commands::memory::{ForgetConfirmation, MemoryChange, NewMemory};
use soulcore::commands::profile::GivenAnswer;
use soulcore::commands::session::Session;

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

/// One answer per kind of question: an axis, a voice field, and prose.
fn a_partial_questionnaire() -> Vec<GivenAnswer> {
    vec![
        given("q.axis.curiosity", "leans_high"),
        given("q.axis.orderliness", ""),
        given("q.voice.register", "formal"),
        given("q.boundary.topics", "工作以外的事"),
        given("q.value.what_matters", ""),
    ]
}

/// The questions the shell draws are the ones the recorder validates against,
/// and every option on them is answerable.
#[test]
fn the_wizard_is_handed_eleven_answerable_questions() {
    let (keep, directory) = scratch();
    let session = Session::open(&directory);

    let questions = session.questionnaire();
    assert_eq!(questions.len(), 11);
    assert_eq!(
        questions.iter().filter(|question| question.prose).count(),
        3,
        "three of them are text boxes",
    );

    for question in &questions {
        assert!(!question.prompt.is_empty(), "{}", question.question_id);
        if question.prose {
            assert!(question.options.is_empty());
            continue;
        }
        assert_eq!(question.options.len(), 3, "{}", question.question_id);
        for option in &question.options {
            assert_ne!(
                option.reading, option.value,
                "{} offers `{}` with no words for it",
                question.question_id, option.value,
            );
        }
    }

    // The questions can be drawn before anything is open, which is what lets
    // the wizard ask them on a machine whose store is the thing that failed.
    assert!(!session.questionnaire().is_empty());
    drop(keep);
}

/// AC-03 through the shell: no import file, a partly filled questionnaire, and
/// a profile that is not empty afterwards. The axes nobody answered for stay
/// `unknown` rather than being filled in from the ones that were.
#[test]
fn answering_the_questionnaire_leaves_a_profile_the_user_stated() {
    let (keep, directory) = scratch();
    let mut session = Session::open(&directory);

    let before = session.profile().expect("a blank profile is still one");
    assert_eq!(before.axes.len(), 5);
    assert!(before.axes.iter().all(|axis| axis.position == "unknown"));
    assert!(before.stated.is_empty());

    let receipt = session
        .answer_questionnaire(&a_partial_questionnaire())
        .expect("the core records what was answered");
    assert_eq!(receipt.answered, 3, "the two blanks left no row");
    assert_eq!(receipt.axes_known, 1);
    assert_eq!(receipt.axes_unknown, 4);
    assert_eq!(receipt.voice_fields_user_set, 1);
    assert_eq!(receipt.stated_entries, 1);
    assert!(!receipt.profile_is_empty, "AC-03");
    assert_eq!(receipt.evidence_ids.len(), 3);

    let after = session.profile().expect("the profile reads back");
    let curiosity = after
        .axes
        .iter()
        .find(|axis| axis.label == "好奇与开放")
        .expect("the axis the user answered for");
    assert_eq!(curiosity.position, "leans_high");
    assert_eq!(curiosity.evidence_count, 1);
    assert_eq!(
        after
            .axes
            .iter()
            .filter(|axis| axis.position == "unknown")
            .count(),
        4,
        "an axis nobody answered for must not be guessed at",
    );

    // The words the user typed are not on this surface. What comes back is the
    // question they answered and the ids that point at the sealed event.
    assert_eq!(after.stated.len(), 1);
    let stated = &after.stated[0];
    assert_eq!(stated.field, "boundary");
    assert_eq!(stated.question_id, "q.boundary.topics");
    assert!(!stated.prompt.is_empty());
    assert!(!stated.event_id.is_empty());
    assert!(
        !format!("{after:?}").contains("工作以外的事"),
        "the screen carries the user's own words",
    );

    // And it survives a restart, because the profile id is fixed rather than
    // generated per session.
    drop(session);
    let again = Session::open(&directory).profile().expect("read back");
    assert_eq!(again.profile_id, after.profile_id);
    assert_eq!(again.stated.len(), 1);
    drop(keep);
}

/// A questionnaire with nothing in it is refused rather than reported as an
/// intake that wrote no rows.
#[test]
fn a_questionnaire_nobody_answered_is_refused() {
    let (keep, directory) = scratch();
    let mut session = Session::open(&directory);

    let refusal = session
        .answer_questionnaire(&[
            given("q.axis.curiosity", ""),
            given("q.boundary.topics", "  "),
        ])
        .expect_err("there is nothing to write");
    assert!(refusal.explanation.contains("一道题都没有答"));
    assert!(session.profile().expect("still readable").stated.is_empty());
    drop(keep);
}

/// AC-07 through the session: a correction pins the axis, and the screen keeps
/// showing what the machine thinks so the disagreement stays visible.
#[test]
fn correcting_an_axis_locks_it_and_setting_a_voice_field_locks_that() {
    let (keep, directory) = scratch();
    let mut session = Session::open(&directory);
    session
        .answer_questionnaire(&a_partial_questionnaire())
        .expect("record");

    let screen = session.profile().expect("a profile");
    let axis = screen
        .axes
        .iter()
        .find(|axis| axis.label == "好奇与开放")
        .expect("the answered axis")
        .clone();
    assert!(!axis.locked_by_user);
    assert_eq!(axis.choices.len(), 3, "unknown is not offered as a choice");
    assert!(axis
        .choices
        .iter()
        .all(|choice| choice.position != "unknown"));

    let corrected = session
        .correct_axis(&axis.axis_id, "leans_low")
        .expect("the user says it is wrong");
    let axis = corrected
        .axes
        .iter()
        .find(|row| row.axis_id == axis.axis_id)
        .expect("the same axis");
    assert_eq!(axis.position, "leans_low");
    assert!(axis.locked_by_user);

    let refusal = session
        .correct_axis(&axis.axis_id, "unknown")
        .expect_err("`unknown` is what an unanswered axis already says");
    assert!(!refusal.explanation.is_empty());
    session
        .correct_axis("not-an-axis", "mixed")
        .expect_err("that is not one of the five");

    let voiced = session
        .set_voice("warmth", "warm")
        .expect("the user sets it");
    let warmth = voiced
        .voice
        .fields
        .iter()
        .find(|field| field.field == "warmth")
        .expect("the field");
    assert_eq!(warmth.value, "warm");
    assert!(warmth.locked_by_user);
    assert!(
        warmth.question_id.is_none(),
        "the questionnaire does not ask about warmth, and the screen must not \
         point at a question that does not exist",
    );

    session
        .set_voice("warmth", "sparing")
        .expect_err("that is not a value this field takes");
    drop(keep);
}

/// The forget path, both halves. Reading the price destroys nothing, and the
/// act refuses anything but the answer the user was actually shown.
#[test]
fn a_forget_only_runs_on_the_preview_the_user_read() {
    let (keep, directory) = scratch();
    let mut session = Session::open(&directory);

    let written = session
        .write_memory(&NewMemory {
            memory_type: "episodic".to_owned(),
            title: "搬家那天".to_owned(),
            summary: "下午三点交的钥匙。".to_owned(),
        })
        .expect("a memory");
    let memory_id = written.memory_id.clone();

    let edited = session
        .edit_memory(
            &memory_id,
            &MemoryChange {
                title: Some("交钥匙那天".to_owned()),
                ..MemoryChange::default()
            },
        )
        .expect("an edit");
    assert_eq!(edited.title, "交钥匙那天");
    assert_eq!(
        edited.content_key_id, written.content_key_id,
        "an edit reseals under the same key, so the memory stays one forget unit",
    );

    let listed = session.memories().expect("the list");
    assert_eq!(listed.memories.len(), 1);
    assert_eq!(listed.memories[0].forget_state, "active");
    assert!(listed.memory_types.contains(&"commitment".to_owned()));
    assert!(listed.forget_notice.contains("不可撤销"));

    // A forget nobody previewed is refused, and nothing is destroyed by the
    // refusal — the memory is still readable afterwards.
    let refusal = session
        .forget_memory(&ForgetConfirmation {
            preview_id: uuid::Uuid::now_v7().to_string(),
            memory_id: memory_id.clone(),
        })
        .expect_err("no preview was issued");
    assert_eq!(refusal.reason_code, "PLAN_HASH_MISMATCH");
    assert!(session.memory(&memory_id).is_ok());

    let preview = session.preview_forget(&memory_id).expect("the price");
    assert!(!preview.destroys_anything);
    assert_eq!(preview.content_key_count, 1);
    assert!(preview.notice.contains("内容密钥"));
    assert!(
        session.memory(&memory_id).is_ok(),
        "reading the price destroyed something",
    );

    // A second preview replaces the first, so the id from the screen the user
    // stopped reading is no longer the one that would run.
    let second = session.preview_forget(&memory_id).expect("asked again");
    assert_ne!(second.preview_id, preview.preview_id);
    let refusal = session
        .forget_memory(&ForgetConfirmation {
            preview_id: preview.preview_id.clone(),
            memory_id: memory_id.clone(),
        })
        .expect_err("that preview is not the one on screen any more");
    assert_eq!(refusal.reason_code, "PLAN_HASH_MISMATCH");

    let preview = session.preview_forget(&memory_id).expect("the price again");
    let receipt = session
        .forget_memory(&ForgetConfirmation {
            preview_id: preview.preview_id,
            memory_id: memory_id.clone(),
        })
        .expect("the user read it and said yes");
    assert_eq!(receipt.content_keys_destroyed, 1);
    assert!(
        receipt.matched_preview,
        "the receipt charged something else"
    );

    session
        .memory(&memory_id)
        .expect_err("the content key is gone");
    let listed = session.memories().expect("the list still reads");
    assert_eq!(
        listed.memories[0].forget_state, "forgotten",
        "the row stays as a tombstone",
    );
    drop(keep);
}

/// AC-20 as the shell receives it: nothing on disk, and no row about anybody
/// else. The chain underneath it holds the forget that just happened and no
/// prose about it.
#[test]
fn the_research_preview_writes_nothing_and_the_chain_holds_no_prose() {
    let (keep, directory) = scratch();
    let mut session = Session::open(&directory);
    session
        .answer_questionnaire(&a_partial_questionnaire())
        .expect("something to have happened");
    let written = session
        .write_memory(&NewMemory {
            memory_type: "commitment".to_owned(),
            title: "周五之前回信".to_owned(),
            summary: "答应过对方周五之前给个说法。".to_owned(),
        })
        .expect("a memory");

    let before: Vec<PathBuf> = std::fs::read_dir(&directory)
        .expect("read the data directory")
        .map(|entry| entry.expect("an entry").path())
        .collect();

    let research = session.research().expect("a preview");
    assert!(!research.written_to_disk);
    assert_eq!(research.third_party_rows, 0);
    assert_eq!(research.third_party_body, "excluded");
    assert_eq!(research.export_kind, "research_preview");
    assert!(!research.notice.is_empty());
    let rendered = format!("{research:?}");
    for prose in ["周五之前回信", "答应过对方", "工作以外的事"] {
        assert!(!rendered.contains(prose), "the preview carries `{prose}`");
    }

    let after: Vec<PathBuf> = std::fs::read_dir(&directory)
        .expect("read the data directory")
        .map(|entry| entry.expect("an entry").path())
        .collect();
    assert_eq!(before, after, "a preview-only export left a file behind");

    let chain = session.audit().expect("the chain");
    assert!(chain.verified, "{:?}", chain.verification_problem);
    assert!(!chain.entries.is_empty());
    assert!(chain.entries.iter().all(|entry| entry.follows_previous));
    assert!(
        chain
            .entries
            .iter()
            .any(|entry| entry.action == "memory.write"),
        "writing a memory is a thing that happened",
    );

    let played = format!("{chain:?}");
    for prose in ["周五之前回信", "答应过对方", "搬家", "工作以外的事"] {
        assert!(!played.contains(prose), "the chain carries `{prose}`");
    }
    assert!(
        played.contains(&written.memory_id),
        "an entry names what it was about, as a bare identifier",
    );
    drop(keep);
}
