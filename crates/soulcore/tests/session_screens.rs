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
    // D15's other half: the notice may not let "forgotten" read as "the bits
    // are gone", so it names the disk it does not scrub. The preview carries
    // the same constant, so saying it once here covers both screens.
    assert!(
        listed.forget_notice.contains("磁盘块") && listed.forget_notice.contains("SSD"),
        "the notice does not say that the disk itself is not wiped: {}",
        listed.forget_notice,
    );
    // And it may not claim the opposite of what a forget does. Dropping the
    // wrapped key, writing the tombstone and appending the denial-or-receipt
    // all land in `soul.db`; what v0.1 does not do is execute a file plan, so
    // that is the promise the sentence is allowed to make.
    assert!(
        !listed.forget_notice.contains("不写任何文件"),
        "the notice claims a forget writes nothing, and it writes the store: {}",
        listed.forget_notice,
    );
    assert!(
        listed.forget_notice.contains("不是文件整理的执行"),
        "the notice does not say which kind of write this is not: {}",
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
    for prose in ["搬家那天", "交钥匙那天", "下午三点交的钥匙。"] {
        assert!(!played.contains(prose), "the chain carries `{prose}`");
        assert!(!debugged.contains(prose), "the chain carries `{prose}`");
    }
    drop(keep);
}

/// A confirmation that does not match costs the user nothing — not even the
/// preview they are looking at.
///
/// The refusal above was written first and took the held preview on its way
/// out, which is a second, quieter destruction: nothing was forgotten, and the
/// screen still showing 「这一次会销毁 1 个内容密钥」 had been silently voided.
/// The next press of the button the user had already read would be refused as
/// well, with a sentence telling them to go and read a preview again — the one
/// in front of them.
///
/// So the order here is the product's: preview once, confirm wrongly, and then
/// confirm with the id that was on screen the whole time. The last call has to
/// destroy the memory.
#[test]
fn a_forget_confirmation_that_misses_leaves_the_preview_on_screen_spendable() {
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

    let preview = session.preview_forget(&memory_id).expect("the price");
    let before = session.audit().expect("the chain").entries.len();

    // The shape a stale confirmation arrives in: the right memory, an id from
    // a screen that is not this one.
    let refusal = session
        .forget_memory(&ForgetConfirmation {
            preview_id: uuid::Uuid::now_v7().to_string(),
            memory_id: memory_id.clone(),
        })
        .expect_err("that is not the preview on screen");
    assert_eq!(refusal.reason_code, "PLAN_HASH_MISMATCH");
    assert!(
        session.memory(&memory_id).is_ok(),
        "a refused confirmation destroyed the memory",
    );

    // The denial is recorded, and it is the only thing that happened.
    let chain = session.audit().expect("the chain");
    assert!(chain.verified, "{:?}", chain.verification_problem);
    assert_eq!(chain.entries.len(), before + 1);
    assert_eq!(
        chain.entries.last().expect("the denial").action,
        "hitl.deny",
    );

    // And now the button the user has been looking at all along works. Without
    // match-before-take this is a second refusal on a screen nothing is wrong
    // with, and the memory outlives a forget the user asked for twice.
    let receipt = session
        .forget_memory(&ForgetConfirmation {
            preview_id: preview.preview_id.clone(),
            memory_id: memory_id.clone(),
        })
        .expect("the preview the user read is still the one being held");
    assert_eq!(receipt.content_keys_destroyed, 1);
    assert!(
        receipt.matched_preview,
        "the receipt charged something else"
    );
    session
        .memory(&memory_id)
        .expect_err("the content key is gone");

    // One preview buys one forget: the same confirmation again finds nothing
    // held, and there is no second key to destroy.
    session
        .forget_memory(&ForgetConfirmation {
            preview_id: preview.preview_id,
            memory_id: memory_id.clone(),
        })
        .expect_err("the preview was spent by the forget that ran");
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
