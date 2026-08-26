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

use std::path::{Path, PathBuf};

use soulcore::commands::memory::{ForgetConfirmation, MemoryChange, NewMemory};
use soulcore::commands::profile::GivenAnswer;
use soulcore::commands::session::{
    Session, FORGET_IMPACT_CHANGED_NOTICE, FORGET_NOT_PREVIEWED_NOTICE,
};

/// Something a person pasted, for the drafting tests below.
const PASTED: &str = "周五那个方案你还改吗？我这边可以等到下午三点。";

/// What the local template says when the user has pinned 温度 to 热络, and to
/// 克制. Both are `soul-draft`'s `template::stance`, spelled out here because a
/// test that asked the template what it says would pass whatever it answered.
const WARM_STANCE: &str = "先谢谢你专门说一声。";
const COOL_STANCE: &str = "直接说重点。";
const EVEN_STANCE: &str = "我这边的想法是这样：";

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

    // AC-23 for the no-file branch of intake. The questionnaire is the other
    // way a profile gets made, so it records under the same action a file
    // import does — `soul-import`'s recorder builds the entry and
    // `soul-profile`'s intake appends it, and neither of those is the layer
    // the 审计 page reads. Only the session is, and an intake the chain never
    // heard about would leave three sealed answers and no record that anybody
    // was ever asked.
    let chain = session.audit().expect("the store opened");
    assert!(chain.verified, "{:?}", chain.verification_problem);
    let recorded = chain
        .entries
        .iter()
        .find(|entry| entry.action == "import.commit")
        .unwrap_or_else(|| {
            panic!(
                "the questionnaire was answered and the chain never heard about it: {:?}",
                chain.entries,
            )
        });
    assert_eq!(recorded.decision, "allowed");
    assert_eq!(
        recorded.items,
        Some(receipt.answered as u64),
        "the entry counts something other than what the screen was told",
    );
    assert!(recorded.follows_previous);

    // A count and an identifier. Not the sentence the user typed into the
    // boundary question, which is the one answer here that is prose.
    let played = serde_json::to_string(&chain).expect("serialize the chain");
    assert!(
        !played.contains("工作以外的事"),
        "the chain carries what the user typed: {played}",
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

    // AC-23 for the profile page: the disagreement between the machine and the
    // person is a thing that happened, so the chain holds it. `soul-profile`'s
    // own tests prove the entry is built; what is only checkable here is that
    // the session carries it to the store the 审计 page reads back — a
    // correction the chain never heard about would make the pinned axis
    // unaccountable, which is the whole point of pinning it.
    let chain = session.audit().expect("the chain");
    assert!(chain.verified, "{:?}", chain.verification_problem);
    let corrected = chain
        .entries
        .iter()
        .find(|entry| entry.action == "profile.correct")
        .unwrap_or_else(|| {
            panic!(
                "the user overruled the machine and the chain never heard about it: {:?}",
                chain.entries,
            )
        });
    assert_eq!(corrected.decision, "allowed");
    assert!(corrected.follows_previous);
    assert!(
        corrected
            .subject_refs
            .iter()
            .any(|held| *held == axis.axis_id),
        "the entry does not say which axis was overruled: {corrected:?}",
    );
    assert_eq!(
        chain
            .entries
            .iter()
            .filter(|entry| entry.action == "profile.correct")
            .count(),
        2,
        "one axis and one voice field were pinned, and each is its own entry: {:?}",
        chain.entries,
    );

    // And the questionnaire that got the profile here typed a boundary in
    // prose; the chain is entitled to the axis identifier and to none of that.
    let played = serde_json::to_string(&chain).expect("serialize the chain");
    assert!(
        !played.contains("工作以外的事"),
        "the chain carries what the user typed: {played}",
    );
    drop(keep);
}

/// AC-07 on the page it is about: the voice the user pinned is the voice the
/// draft is written in.
///
/// `soul-draft`'s `voice_and_template.rs` already pins each fragment to a voice
/// value, and the test above already proves `set_voice` locks the field in the
/// store. Neither of them can see the path between the two, and until now there
/// was none: `draft_pasted` built its request from `ProfileBrief::neutral()`, so
/// every draft an installed Soul produced read identically no matter what the
/// profile page said. The assertions are on the text the session hands back,
/// because that is the string the user reads.
#[test]
fn the_voice_the_user_pinned_is_the_voice_a_local_draft_is_written_in() {
    let (keep, directory) = scratch();
    let mut session = Session::open(&directory);

    let neutral = session.draft_pasted(PASTED).expect("a draft").text;
    assert!(
        neutral.contains(EVEN_STANCE),
        "a profile nobody has touched drafts in the neutral voice: {neutral}",
    );

    session.set_voice("warmth", "warm").expect("热络");
    let warm = session.draft_pasted(PASTED).expect("a draft").text;
    assert_ne!(
        warm, neutral,
        "pinning a voice field changed nothing about what Soul writes",
    );
    assert!(warm.contains(WARM_STANCE), "{warm}");
    assert!(!warm.contains(EVEN_STANCE));

    session.set_voice("warmth", "cool").expect("克制");
    let cool = session.draft_pasted(PASTED).expect("a draft").text;
    assert!(cool.contains(COOL_STANCE), "{cool}");
    assert_ne!(cool, warm, "两个取值写出同一份草稿");

    // The voice survives a restart, because it lives in the profile rather
    // than in the session that drafted with it.
    drop(session);
    let mut next_launch = Session::open(&directory);
    let after = next_launch.draft_pasted(PASTED).expect("a draft").text;
    assert_eq!(after, cool);
    drop(keep);
}

/// AC-23 for drafting: a draft on the local path is a thing the chain heard
/// about.
///
/// `draft_commands.rs` proves the entry is built and carries no prose. What is
/// only checkable here is that somebody writes it: `Session::draft_pasted`
/// returned `Drafted.draft` and dropped `Drafted.audit`, so `/audit` on an
/// installed Soul never showed 起草 at all.
#[test]
fn a_local_draft_lands_in_the_audit_chain_as_counts_and_a_code() {
    let (keep, directory) = scratch();
    let mut session = Session::open(&directory);

    let before = session.audit().expect("the chain").entries.len();
    session.draft_pasted(PASTED).expect("a draft");

    let chain = session.audit().expect("the chain");
    assert!(chain.verified, "{:?}", chain.verification_problem);
    assert_eq!(chain.entries.len(), before + 1, "one draft, one entry");
    assert!(chain.entries.iter().all(|entry| entry.follows_previous));
    assert!(
        chain
            .entries
            .iter()
            .any(|entry| entry.action == "draft.create"),
        "drafting is a thing that happened: {:?}",
        chain.entries,
    );

    // And it is still counts and codes: the paste is not in the chain, and
    // neither is the draft that was written from it.
    let played = format!("{chain:?}");
    for prose in ["周五那个方案", "下午三点", WARM_STANCE, EVEN_STANCE] {
        assert!(!played.contains(prose), "the chain carries `{prose}`");
    }
    drop(keep);
}

/// AC-14's Given is three memories, and until now every test on this path
/// wrote one.
///
/// One memory cannot show the thing the cell is about: that a store holding
/// several hands each of them back as itself. A list has to name three rows
/// rather than one, opening each has to return the prose that memory was
/// written with rather than the last one written, and an edit has to land on
/// the memory it named and leave the other two exactly as they were. The audit
/// half is the same requirement asked of a chain that has three memories'
/// worth of writes on it: 读写一致，审计无内容.
///
/// Forgetting is deliberately not part of this: `a_forget_only_runs_on_the_
/// preview_the_user_read` below owns that path, and a memory destroyed here
/// would be one this test could no longer read back.
#[test]
fn three_memories_read_back_as_written_and_the_chain_holds_none_of_their_prose() {
    let (keep, directory) = scratch();
    let mut session = Session::open(&directory);

    // 三条记忆，类型、标题、正文都不一样——都一样的话，「读回来的是自己那条」
    // 就无从谈起。
    let typed = [
        NewMemory {
            memory_type: "episodic".to_owned(),
            title: "搬家那天".to_owned(),
            summary: "下午三点交的钥匙。".to_owned(),
        },
        NewMemory {
            memory_type: "commitment".to_owned(),
            title: "周五之前回信".to_owned(),
            summary: "答应过对方周五之前给个说法。".to_owned(),
        },
        NewMemory {
            memory_type: "preference".to_owned(),
            title: "早上不接电话".to_owned(),
            summary: "十点以前只看文字消息，电话都不接。".to_owned(),
        },
    ];

    let written: Vec<_> = typed
        .iter()
        .map(|new| session.write_memory(new).expect("a memory"))
        .collect();
    for (new, detail) in typed.iter().zip(&written) {
        assert_eq!(detail.memory_type, new.memory_type);
        assert_eq!(detail.title, new.title);
        assert_eq!(detail.summary, new.summary);
    }
    let keys: std::collections::BTreeSet<&str> = written
        .iter()
        .map(|detail| detail.content_key_id.as_str())
        .collect();
    assert_eq!(
        keys.len(),
        3,
        "两条记忆共用一把内容密钥，忘掉一条就会带走另一条",
    );

    // The list is three rows and no prose, and each row's counts belong to the
    // memory it names rather than to whichever was written last.
    let listed = session.memories().expect("the list");
    assert_eq!(listed.memories.len(), 3);
    for (new, detail) in typed.iter().zip(&written) {
        let row = listed
            .memories
            .iter()
            .find(|row| row.memory_id == detail.memory_id)
            .unwrap_or_else(|| panic!("`{}` is not in the list", new.title));
        assert_eq!(row.forget_state, "active");
        assert_eq!(row.memory_type, new.memory_type);
        assert_eq!(row.title_chars, new.title.chars().count() as u64);
        assert_eq!(row.summary_chars, new.summary.chars().count() as u64);
    }
    let rows = serde_json::to_string(&listed).expect("serialize the list");
    for new in &typed {
        assert!(
            !rows.contains(&new.title),
            "the list carries `{}`",
            new.title
        );
        assert!(!rows.contains(&new.summary), "the list carries a summary");
    }

    // Opening each one returns what that one was written with.
    for (new, detail) in typed.iter().zip(&written) {
        let read = session.memory(&detail.memory_id).expect("the user asked");
        assert_eq!(read, *detail, "`{}` did not read back as itself", new.title);
        assert_eq!(read.title, new.title);
        assert_eq!(read.summary, new.summary);
    }

    // An edit lands on the memory it named. The other two are untouched, and
    // the edited one is resealed under the key it already had.
    const RETITLED: &str = "交钥匙那天";
    const RESUMMARIZED: &str = "钥匙是下午三点交的，房东没上来。";
    let edited = session
        .edit_memory(
            &written[0].memory_id,
            &MemoryChange {
                title: Some(RETITLED.to_owned()),
                summary: Some(RESUMMARIZED.to_owned()),
                ..MemoryChange::default()
            },
        )
        .expect("an edit");
    assert_eq!(edited.title, RETITLED);
    assert_eq!(edited.summary, RESUMMARIZED);
    assert_eq!(
        edited.memory_type, typed[0].memory_type,
        "改的是正文，不是类型"
    );
    assert_eq!(
        edited.content_key_id, written[0].content_key_id,
        "an edit reseals under the same key, so the memory stays one forget unit",
    );
    for (new, detail) in typed.iter().zip(&written).skip(1) {
        let read = session.memory(&detail.memory_id).expect("still readable");
        assert_eq!(read, *detail, "editing one memory changed `{}`", new.title,);
    }
    let listed = session.memories().expect("the list");
    assert_eq!(listed.memories.len(), 3, "an edit is not a fourth memory");
    assert!(listed
        .memories
        .iter()
        .all(|row| row.forget_state == "active"));

    // AC-23 across all of it: four writes, verified, and not one word of what
    // any of the three said. `bytes` stays empty on every entry so the length
    // of a summary is not a side channel either.
    let chain = session.audit().expect("the chain");
    assert!(chain.verified, "{:?}", chain.verification_problem);
    assert!(chain.entries.iter().all(|entry| entry.follows_previous));
    assert_eq!(
        chain
            .entries
            .iter()
            .filter(|entry| entry.action == "memory.write")
            .count(),
        4,
        "three writes and one edit: {:?}",
        chain.entries,
    );
    assert!(chain.entries.iter().all(|entry| entry.bytes.is_none()));

    let played = serde_json::to_string(&chain).expect("serialize the chain");
    let debugged = format!("{chain:?}");
    for prose in typed
        .iter()
        .flat_map(|new| [new.title.as_str(), new.summary.as_str()])
        .chain([RETITLED, RESUMMARIZED])
    {
        assert!(!played.contains(prose), "the chain carries `{prose}`");
        assert!(!debugged.contains(prose), "the chain carries `{prose}`");
    }
    for detail in &written {
        assert!(
            played.contains(&detail.memory_id),
            "an entry names what it was about, as a bare identifier",
        );
    }
    drop(keep);
}

/// The forget path, both halves. Reading the price destroys nothing, and the
/// act refuses anything but the answer the user was actually shown.
///
/// Shown, and still true: the ids on a confirmation say which screen it came
/// from and nothing about whether the store has moved since. The edit in the
/// middle is the case that made the difference visible — same content key,
/// same forget unit, both ids matching, and a cost the user never read.
#[test]
fn a_forget_only_runs_on_the_preview_the_user_read() {
    /// The edit that lands between the preview and the confirmation below.
    const REWRITTEN: &str = "钥匙是下午三点交的，房东没上来。";

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
    // D15's other half: the notice may not let "forgotten" read as "the bits
    // are gone", so it names the disk it does not scrub. The preview carries
    // the same constant, so saying it once here covers both screens.
    assert!(
        listed.forget_notice.contains("磁盘块") && listed.forget_notice.contains("SSD"),
        "the notice does not say that the disk itself is not wiped: {}",
        listed.forget_notice,
    );
    // The limit next to it, and a different one: a forget writes. The
    // transaction below deletes key and blob rows, tombstones the memory and
    // appends an audit record, so the notice may scope what it touches and may
    // not claim that it writes nothing.
    assert!(
        !listed.forget_notice.contains("不写任何文件"),
        "the notice claims a forget writes no file, which its own transaction \
         contradicts: {}",
        listed.forget_notice,
    );
    assert!(
        listed.forget_notice.contains("Soul 自己的加密库要写")
            && listed.forget_notice.contains("数据目录以外的"),
        "the notice does not say which storage a forget writes: {}",
        listed.forget_notice,
    );

    // A forget nobody previewed is refused, and nothing is destroyed by the
    // refusal — the memory is still readable afterwards.
    let before = session.audit().expect("the chain").entries.len();
    let refusal = session
        .forget_memory(&ForgetConfirmation {
            preview_id: uuid::Uuid::now_v7().to_string(),
            memory_id: memory_id.clone(),
        })
        .expect_err("no preview was issued");
    assert_eq!(refusal.reason_code, "PLAN_HASH_MISMATCH");
    assert!(session.memory(&memory_id).is_ok());

    // AC-23: the refusal is a thing that happened to the user, so the chain
    // holds it — under the action the contract already has for a confirmation
    // the guard turned away, and with no word of which memory was named.
    let chain = session.audit().expect("the chain");
    assert!(chain.verified, "{:?}", chain.verification_problem);
    assert_eq!(chain.entries.len(), before + 1);
    let entry = chain.entries.last().expect("the denial just written");
    assert_eq!(entry.action, "hitl.deny");
    assert_eq!(entry.decision, "denied");
    assert_eq!(entry.reason_code.as_deref(), Some("PLAN_HASH_MISMATCH"));
    assert!(entry.follows_previous);

    let played = serde_json::to_string(&chain).expect("serialize the chain");
    let debugged = format!("{chain:?}");
    for prose in ["搬家那天", "交钥匙那天", "下午三点交的钥匙。"] {
        assert!(!played.contains(prose), "the chain carries `{prose}`");
        assert!(!debugged.contains(prose), "the chain carries `{prose}`");
    }

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
    let before = session.audit().expect("the chain").entries.len();
    let refusal = session
        .forget_memory(&ForgetConfirmation {
            preview_id: preview.preview_id.clone(),
            memory_id: memory_id.clone(),
        })
        .expect_err("that preview is not the one on screen any more");
    assert_eq!(refusal.reason_code, "PLAN_HASH_MISMATCH");

    // A stale preview id is the same refusal, and it is recorded too.
    let chain = session.audit().expect("the chain");
    assert!(chain.verified, "{:?}", chain.verification_problem);
    assert_eq!(chain.entries.len(), before + 1);
    let entry = chain.entries.last().expect("the second denial");
    assert_eq!(entry.action, "hitl.deny");
    assert_eq!(entry.decision, "denied");
    assert_eq!(entry.reason_code.as_deref(), Some("PLAN_HASH_MISMATCH"));
    assert!(entry.follows_previous);

    // Matching the two ids says the confirmation came from a screen this
    // session issued. It does not say the screen is still true. An edit
    // reseals under the same content key, so the memory stays one forget unit
    // and both halves of the confirmation still match — while the impact
    // underneath moves, and the forget would run at a number nobody read.
    let stale = session.preview_forget(&memory_id).expect("the price");
    session
        .edit_memory(
            &memory_id,
            &MemoryChange {
                summary: Some(REWRITTEN.to_owned()),
                ..MemoryChange::default()
            },
        )
        .expect("an edit between reading the price and paying it");

    let before = session.audit().expect("the chain").entries.len();
    let refusal = session
        .forget_memory(&ForgetConfirmation {
            preview_id: stale.preview_id.clone(),
            memory_id: memory_id.clone(),
        })
        .expect_err("the screen that was read is not the price any more");
    assert_eq!(refusal.reason_code, "PLAN_HASH_MISMATCH");
    assert_eq!(
        refusal.explanation, FORGET_IMPACT_CHANGED_NOTICE,
        "the id matched and the memory moved, and telling the user their \
         confirmation missed the preview sends them looking for a stale \
         window that is not there",
    );

    // Nothing was destroyed by the refusal: the memory still opens, and it
    // opens on the edit that caused it.
    let opened = session
        .memory(&memory_id)
        .expect("a refused forget destroyed something");
    assert_eq!(opened.summary, REWRITTEN);
    let chain = session.audit().expect("the chain");
    assert!(chain.verified, "{:?}", chain.verification_problem);
    assert_eq!(chain.entries.len(), before + 1);
    let entry = chain.entries.last().expect("the third denial");
    assert_eq!(entry.action, "hitl.deny");
    assert_eq!(entry.decision, "denied");
    assert_eq!(entry.reason_code.as_deref(), Some("PLAN_HASH_MISMATCH"));
    assert!(entry.follows_previous);

    // And this refusal did take the preview, unlike the ones above: the
    // numbers on it are no longer true of anything, so pressing the same
    // button again is a confirmation with nothing behind it rather than a
    // second chance to spend a stale price.
    let again = session
        .forget_memory(&ForgetConfirmation {
            preview_id: stale.preview_id.clone(),
            memory_id: memory_id.clone(),
        })
        .expect_err("a stale price is not spendable twice either");
    assert_eq!(again.explanation, FORGET_NOT_PREVIEWED_NOTICE);
    assert!(session.memory(&memory_id).is_ok());

    let preview = session.preview_forget(&memory_id).expect("the price again");
    assert_ne!(
        (
            preview.sealed_blobs_destroyed,
            preview.audit_entries_retained,
        ),
        (stale.sealed_blobs_destroyed, stale.audit_entries_retained),
        "the edit left the price where it was, so the refusal above proved \
         nothing about the case it is named for",
    );
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

    // AC-23's hardest case: the record of a destruction has to outlive the
    // thing destroyed, and it has to do that without keeping a copy. The two
    // denials above are already in the chain; this is the one that went
    // through, and after it there is nothing left to read the title from — so
    // if the title were in the chain, the chain would be the last copy of it.
    let chain = session.audit().expect("the chain");
    assert!(chain.verified, "{:?}", chain.verification_problem);
    let executed = chain
        .entries
        .iter()
        .find(|entry| entry.action == "forget.execute")
        .unwrap_or_else(|| {
            panic!(
                "a memory was destroyed and the chain never heard about it: {:?}",
                chain.entries,
            )
        });
    assert_eq!(executed.decision, "allowed");
    assert!(executed.follows_previous);
    assert!(
        executed.subject_refs.iter().any(|held| *held == memory_id),
        "the entry does not say which memory went: {executed:?}",
    );

    let played = serde_json::to_string(&chain).expect("serialize the chain");
    let debugged = format!("{chain:?}");
    for prose in ["搬家那天", "交钥匙那天", "下午三点交的钥匙。", REWRITTEN] {
        assert!(!played.contains(prose), "the chain carries `{prose}`");
        assert!(!debugged.contains(prose), "the chain carries `{prose}`");
    }
    drop(keep);
}

/// A confirmation that named the wrong thing costs the click, not the preview.
///
/// The refusal above is the contract: a forget runs on the preview the user
/// read and on nothing else. What this pins is the other side of it — the
/// preview survives being refused. Match first, take on success: a WebView
/// that echoed a stale id, or a second window that answered for the wrong
/// memory, leaves the held preview exactly where it was, and the correct
/// confirmation right afterwards still goes through.
///
/// The alternative would make one wrong id the reason a user has to walk the
/// irreversible screen again, which is the sort of retry that gets clicked
/// through rather than read.
#[test]
fn a_forget_refused_for_the_wrong_id_leaves_the_preview_the_user_read_standing() {
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
    let other = session
        .write_memory(&NewMemory {
            memory_type: "commitment".to_owned(),
            title: "周五之前回信".to_owned(),
            summary: "答应过对方周五之前给个说法。".to_owned(),
        })
        .expect("a second memory");

    let preview = session.preview_forget(&memory_id).expect("the price");

    // Wrong preview id, right memory. The screen the user is looking at is
    // still the one this session holds.
    let refusal = session
        .forget_memory(&ForgetConfirmation {
            preview_id: uuid::Uuid::now_v7().to_string(),
            memory_id: memory_id.clone(),
        })
        .expect_err("that is not the preview that was issued");
    assert_eq!(refusal.reason_code, "PLAN_HASH_MISMATCH");
    assert_eq!(refusal.explanation, FORGET_NOT_PREVIEWED_NOTICE);

    // Right preview id, wrong memory. Both halves are matched, and a
    // confirmation that gets one of them wrong is not a licence to spend the
    // other.
    let refusal = session
        .forget_memory(&ForgetConfirmation {
            preview_id: preview.preview_id.clone(),
            memory_id: other.memory_id.clone(),
        })
        .expect_err("the preview was read for another memory");
    assert_eq!(refusal.reason_code, "PLAN_HASH_MISMATCH");

    // Nothing was destroyed by either refusal.
    assert!(session.memory(&memory_id).is_ok());
    assert!(session.memory(&other.memory_id).is_ok());

    // And the preview the user actually read is still the one on the session,
    // so the confirmation they meant to send works without a second walk
    // through the irreversible screen.
    let receipt = session
        .forget_memory(&ForgetConfirmation {
            preview_id: preview.preview_id.clone(),
            memory_id: memory_id.clone(),
        })
        .expect("the preview two refusals did not consume");
    assert!(
        receipt.matched_preview,
        "the receipt charged something other than the preview"
    );
    assert_eq!(receipt.content_keys_destroyed, 1);
    session
        .memory(&memory_id)
        .expect_err("the content key is gone");
    assert!(
        session.memory(&other.memory_id).is_ok(),
        "the memory a refused confirmation named was forgotten too",
    );

    // Spent, though: the same confirmation a second time has no preview behind
    // it, so a replay buys nothing.
    let refusal = session
        .forget_memory(&ForgetConfirmation {
            preview_id: preview.preview_id,
            memory_id: memory_id.clone(),
        })
        .expect_err("the forget that ran took the preview with it");
    assert_eq!(refusal.reason_code, "PLAN_HASH_MISMATCH");

    // AC-23: three denials and one execution, and not a word of any title.
    let chain = session.audit().expect("the chain");
    assert!(chain.verified, "{:?}", chain.verification_problem);
    assert_eq!(
        chain
            .entries
            .iter()
            .filter(|entry| entry.action == "hitl.deny")
            .count(),
        3,
        "the refusals are not all in the chain: {:?}",
        chain.entries,
    );
    assert_eq!(
        chain
            .entries
            .iter()
            .filter(|entry| entry.action == "forget.execute")
            .count(),
        1,
        "one preview, one destruction: {:?}",
        chain.entries,
    );
    assert!(chain.entries.iter().all(|entry| entry.follows_previous));
    let played = serde_json::to_string(&chain).expect("serialize the chain");
    for prose in ["搬家那天", "下午三点交的钥匙。", "周五之前回信"] {
        assert!(!played.contains(prose), "the chain carries `{prose}`");
    }
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

    let before = footprint(&directory);
    assert!(
        !before.is_empty(),
        "the data directory is empty, so the comparison below would hold for a \
         product that wrote nothing because there was nothing there",
    );

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

    // Not a list of names. `soul-store`'s `a_preview_leaves_no_new_file_behind`
    // compares names and lengths one layer down; what a product wrapper can
    // still do without either of them noticing is append an audit row, grow
    // the write-ahead log, or rewrite a file in place — so the bytes are what
    // is compared here.
    let after = footprint(&directory);
    assert_eq!(
        before, after,
        "a preview-only export changed what is on disk",
    );

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

/// ----------------------------------------------------------- footprint ---
///
/// Everything under a directory, as sorted `(relative path, length, digest)`.
///
/// AC-20 says a research preview writes nothing, and a list of paths cannot
/// tell that apart from a preview that appended an audit row, grew the
/// write-ahead log, or rewrote a file in place — all of which leave exactly
/// the same names behind. The digest can.
///
/// SQLite's `-shm` is left out on purpose. It is the write-ahead index, it is
/// rebuilt from the log, it holds none of Soul's data, and it is stamped by
/// the act of taking a read lock — including it would make every read look
/// like a write. The `-wal` itself is included, which is where an appended
/// row would land.
fn footprint(root: &Path) -> Vec<(String, u64, String)> {
    fn walk(root: &Path, at: &Path, into: &mut Vec<(String, u64, String)>) {
        let listing =
            std::fs::read_dir(at).unwrap_or_else(|error| panic!("read {}: {error}", at.display()));
        for entry in listing {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                walk(root, &path, into);
                continue;
            }
            let relative = path
                .strip_prefix(root)
                .expect("every entry is under the root")
                .to_string_lossy()
                .replace('\\', "/");
            if relative.ends_with("-shm") {
                continue;
            }
            let bytes = std::fs::read(&path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
            into.push((relative, bytes.len() as u64, sha256_hex(&bytes)));
        }
    }

    let mut found = Vec::new();
    walk(root, root, &mut found);
    found.sort();
    found
}

/// FIPS 180-4 SHA-256, spelled out.
///
/// `soulcore` has no digest of its own and a comparison in one test is not
/// worth an edge on the dependency graph `xtask e0-audit` and `deny.toml`
/// walk. `the_digest_agrees_with_the_published_vectors` is what keeps it
/// honest — a hash that answered a constant would make the footprint
/// comparison above pass for a product that rewrote every file it has.
fn sha256_hex(bytes: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let mut padded = bytes.to_vec();
    let bits = (bytes.len() as u64) * 8;
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bits.to_be_bytes());

    for block in padded.chunks_exact(64) {
        let mut schedule = [0u32; 64];
        for (slot, word) in schedule.iter_mut().zip(block.chunks_exact(4)) {
            *slot = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for index in 16..64 {
            let fifteen = schedule[index - 15];
            let two = schedule[index - 2];
            let s0 = fifteen.rotate_right(7) ^ fifteen.rotate_right(18) ^ (fifteen >> 3);
            let s1 = two.rotate_right(17) ^ two.rotate_right(19) ^ (two >> 10);
            schedule[index] = schedule[index - 16]
                .wrapping_add(s0)
                .wrapping_add(schedule[index - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ ((!e) & g);
            let first = h
                .wrapping_add(s1)
                .wrapping_add(choose)
                .wrapping_add(K[index])
                .wrapping_add(schedule[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let second = s0.wrapping_add(majority);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(first);
            d = c;
            c = b;
            b = a;
            a = first.wrapping_add(second);
        }

        for (slot, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }

    let mut hex = String::with_capacity(64);
    for byte in state.iter().flat_map(|word| word.to_be_bytes()) {
        hex.push(char::from_digit(u32::from(byte >> 4), 16).expect("a nibble is a hex digit"));
        hex.push(char::from_digit(u32::from(byte & 0x0f), 16).expect("a nibble is a hex digit"));
    }
    hex
}

/// The two vectors FIPS 180-4 publishes, plus one that needs a second block.
#[test]
fn the_digest_agrees_with_the_published_vectors() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    );
    assert_eq!(
        sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
    );
}
