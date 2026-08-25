//! AC-24 and the mid-forget half of AC-15, through the object the desktop
//! shell actually holds.
//!
//! `soul-store`'s `crash_recovery.rs` and `soul-policy`'s `audit_crash.rs`
//! already kill a process inside a write and reopen what it left behind. Both
//! of them hold a bare [`SqlCipherStore`](soul_store::SqlCipherStore): they
//! open the database themselves, write through the store traits, and reopen
//! the same file. That proves the store survives a power loss, and it says
//! nothing about the thing a user has — a [`Session`], which opens a key
//! provider, a configuration file and a database together, and which is the
//! only object in the product that ever opens any of them.
//!
//! So this file crashes the product path. The child opens a `Session`, writes
//! through the commands the shell binds to, and dies inside an armed fail
//! point; the parent opens a `Session` on the same directory and reads it
//! back through those same commands. Nothing here simulates a crash by
//! returning an error: an error unwinds, runs destructors and lets SQLite
//! tidy up, which is exactly what a power loss does not do.
//!
//! Two scenarios:
//!
//! * `STORE_EVENT_COMMIT_MID` fires inside the third event an import commit
//!   writes. `Session::commit_import` runs the whole file — the events, the
//!   people, the graph rebuild and the audit entries all three owe — inside one
//!   `SqlCipherStore::transact`, so what a power loss there costs is the import
//!   and not one message of it. Reopening must show none of it, a chain the
//!   审计 page can still verify, and the same file importable again with no
//!   duplicate events, which is the thing a half-written import could not
//!   offer.
//! * `FORGET_CK_DELETE_MID` fires between destroying one content key and the
//!   next. Reopening must show a memory that is all of one thing — readable
//!   and active, or forgotten and unopenable — and never half of each.
//!
//! The fail points themselves live in `soul-store` and compile to nothing
//! unless the `failpoints` feature is on. `soulcore` turns it on in
//! dev-dependencies only, so a shipped build carries no injection sites; the
//! feature reaches `soul-store` through the same unification `soul-policy`
//! relies on.
//!
//! Off Windows the session opens its database with the developer seed file
//! beside it, which is the same thing `session_commands.rs` already runs on.

use std::path::Path;

use soul_store_api::types::EventFilter;
use soul_store_api::EventStore;
use soul_testkit::crash::{self, run_crashing_subprocess, CrashScenario};
use soulcore::commands::memory::{ForgetConfirmation, NewMemory};
use soulcore::commands::session::Session;

/// How the parent tells the child which directory to be a Soul in.
const DATA_DIR_ENV: &str = "SOUL_SESSION_CRASH_DIR";

const IMPORT_CHILD: &str = "child_commits_an_import_through_a_session_and_may_die_mid_event";
const FORGET_CHILD: &str = "child_forgets_a_memory_through_a_session_and_may_die_mid_destruction";

/// Three messages, so a fail point armed to let two through has a third to
/// interrupt. Written out here rather than read from `fixtures/` because the
/// count is the assertion: a fixture that grew a line would quietly change
/// what "one uncommitted write" means.
const EXPORT: &str = concat!(
    r#"{"type":"header","format":"soul-import-v1","version":1,"exported_at":"2026-08-24T08:00:00Z"}"#,
    "\n",
    r#"{"type":"message","id":"m-01","occurred_at":"2026-08-20T09:12:00Z","sender_scope":"self","conversation_id":"c-01","sender_id":"u-self","text":"钥匙我下午三点交给房东"}"#,
    "\n",
    r#"{"type":"message","id":"m-02","occurred_at":"2026-08-20T09:13:05Z","sender_scope":"third_party","conversation_id":"c-01","sender_id":"u-lin","text":"那我在楼下等你"}"#,
    "\n",
    r#"{"type":"message","id":"m-03","occurred_at":"2026-08-20T09:15:40Z","sender_scope":"third_party","conversation_id":"c-01","sender_id":"u-lin","text":"顺便把水电单也拿上"}"#,
    "\n",
);

const MESSAGES: usize = 3;

/// What the export says. None of it may be legible in the chain the parent
/// reads back, crash or no crash.
const SPOKEN: &[&str] = &[
    "钥匙我下午三点交给房东",
    "那我在楼下等你",
    "顺便把水电单也拿上",
];

const MEMORY_TITLE: &str = "退租那天";
const MEMORY_SUMMARY: &str = "下午三点把钥匙交给房东。";

fn child_directory() -> String {
    std::env::var(DATA_DIR_ENV).expect("the parent passes the data directory")
}

/// The events this session's store holds.
///
/// Reached through `Session::store`, which is the one handle the product
/// opens: a second `SqlCipherStore::open` here would be counting rows in a
/// connection no running Soul has.
fn events_in(session: &Session) -> usize {
    let store = session
        .store()
        .expect("the session opened the database the child left behind");
    let store = store.lock().expect("nobody panicked holding the store");
    store
        .list_events(&EventFilter::all())
        .expect("the events read back")
        .len()
}

fn scratch(name: &str) -> (tempfile::TempDir, String) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = std::fs::canonicalize(directory.path())
        .expect("canonical temp dir")
        .join(name);
    std::fs::create_dir_all(&path).expect("the data directory");
    let text = path.to_string_lossy().into_owned();
    (directory, text)
}

/// The chain, checked and searched for anything anybody wrote.
fn assert_chain_holds_no_prose(session: &Session, prose: &[&str]) {
    let chain = session
        .audit()
        .expect("the store opened, so the chain reads back");
    assert!(
        chain.verified,
        "the chain did not survive the crash: {:?}",
        chain.verification_problem,
    );
    assert!(
        chain.entries.iter().all(|entry| entry.follows_previous),
        "a crash left an entry that does not follow the one before it: {:?}",
        chain.entries,
    );
    let played = serde_json::to_string(&chain).expect("serialize the chain");
    for said in prose {
        assert!(!played.contains(said), "the chain carries `{said}`");
    }
}

// ------------------------------------------------------------- children ---

/// Runs only as the re-executed child. A normal `cargo test` run skips it.
#[test]
fn child_commits_an_import_through_a_session_and_may_die_mid_event() {
    if !crash::is_child() {
        return;
    }
    crash::abort_if_child();
    let _scenario = fail::FailScenario::setup();

    let mut session = Session::open(child_directory());
    session
        .commit_soul_import_v1(EXPORT)
        .expect("the export commits");
}

#[test]
fn child_forgets_a_memory_through_a_session_and_may_die_mid_destruction() {
    if !crash::is_child() {
        return;
    }
    crash::abort_if_child();
    let _scenario = fail::FailScenario::setup();

    let mut session = Session::open(child_directory());
    let written = session
        .write_memory(&NewMemory {
            memory_type: "episodic".to_owned(),
            title: MEMORY_TITLE.to_owned(),
            summary: MEMORY_SUMMARY.to_owned(),
        })
        .expect("a memory");
    let preview = session
        .preview_forget(&written.memory_id)
        .expect("the price");
    session
        .forget_memory(&ForgetConfirmation {
            preview_id: preview.preview_id,
            memory_id: written.memory_id,
        })
        .expect("the user read it and said yes");
}

// -------------------------------------------------------------- parents ---

/// AC-24 through the product, re-pinned: a launch that died mid-commit reopens
/// as a launch, with the interrupted import gone whole and everything that was
/// already there where it was.
///
/// `2*off->panic` lets the first two events through and kills the third inside
/// its savepoint, which is the state a power loss leaves behind. What the two
/// that got through cost has changed, and deliberately: they were written
/// inside the transaction `Session::commit_import` opens, and nothing committed
/// it, so reopening finds none of the three rather than two of them.
///
/// That is a stronger reading of AC-24's "at most one uncommitted write may be
/// lost", not a weaker one. The unit that was being written is the import — the
/// events, the people, the graph derived from them and the audit entries the
/// commit owes are one write, because none of them means anything without the
/// others — and exactly one of those is lost. The store-level promise is
/// unchanged and `soul-store`'s own `crash_recovery.rs` still holds it: a bare
/// `append_event` with nobody wrapping it commits on its own, and a crash there
/// costs that one row.
///
/// The half-written outcome this replaces was not merely untidy. Imported
/// events carry no external id, so a user who re-imported the file to finish
/// the job got a second copy of everything that had survived; the only way to
/// recover was not to. The last third of this test is that: the same file, the
/// same directory, and the message count of one import.
#[test]
fn a_session_that_died_mid_import_reopens_with_the_whole_import_rolled_back() {
    let (keep, directory) = scratch("import-crash");

    let outcome = run_crashing_subprocess(
        &CrashScenario::new(
            IMPORT_CHILD,
            format!(
                "{}=2*off->panic",
                soul_testkit::crash::failpoints::STORE_EVENT_COMMIT_MID,
            ),
        )
        .with_env(DATA_DIR_ENV, &directory),
    )
    .expect("re-execute the test binary");
    outcome.assert_died("the importing child");

    // The whole of the product's opening sequence, on a database nobody
    // closed: key provider, configuration file, store, identifier sync.
    let session = Session::open(&directory);
    let status = session.status();
    assert!(
        status.store_opened,
        "a crash left a database the next launch could not open: {}",
        status.store_notice,
    );

    assert_eq!(
        events_in(&session),
        0,
        "an import is one write: a crash inside it must leave none of it, \
         and two committed events would be a partial import no retry can fix",
    );
    assert!(
        session
            .people()
            .expect("the graph reads back")
            .people
            .is_empty(),
        "the contacts the interrupted commit wrote outlived the events it wrote",
    );
    assert_chain_holds_no_prose(&session, SPOKEN);

    // The chain can be extended: a launch after a crash is a launch, not a
    // museum. This is the property a broken `prev_hash` would destroy, and
    // reopening alone cannot see it.
    let mut session = session;
    session
        .draft_pasted("周五那个方案你还改吗？")
        .expect("a draft");
    let chain = session.audit().expect("the chain");
    assert!(
        chain.verified,
        "the chain broke on the first write after the crash: {:?}",
        chain.verification_problem,
    );
    assert!(chain
        .entries
        .iter()
        .any(|entry| entry.action == "draft.create"));

    // And the retry the rollback is for. Nothing was left behind to collide
    // with, so the same export goes in once and counts once.
    let receipt = session
        .commit_soul_import_v1(EXPORT)
        .expect("the file the crash interrupted imports again");
    assert_eq!(receipt.events_written, MESSAGES);
    assert_eq!(
        events_in(&session),
        MESSAGES,
        "re-importing after a crash wrote the surviving events a second time",
    );
    assert_eq!(
        receipt.contacts_matched, 0,
        "the crash left contacts behind for the retry to recognize",
    );
    assert_chain_holds_no_prose(&session, SPOKEN);
    drop(keep);
}

/// The control: with nothing armed, the same child commits the whole export.
///
/// Without it a child that was simply broken would pass as a successful
/// crash, and the test above would be measuring nothing.
#[test]
fn without_an_armed_fail_point_the_same_session_commits_the_whole_import() {
    let (keep, directory) = scratch("import-control");

    let outcome = run_crashing_subprocess(
        &CrashScenario::new(IMPORT_CHILD, "").with_env(DATA_DIR_ENV, &directory),
    )
    .expect("re-execute the test binary");
    assert!(
        !outcome.died(),
        "the control run measures the injection, not a broken child\n--- stderr ---\n{}",
        outcome.stderr,
    );

    let session = Session::open(&directory);
    assert_eq!(events_in(&session), MESSAGES);
    let chain = session.audit().expect("the chain");
    assert!(chain.verified, "{:?}", chain.verification_problem);
    assert!(
        chain
            .entries
            .iter()
            .any(|entry| entry.action == "import.commit"),
        "an uninterrupted commit records itself: {:?}",
        chain.entries,
    );
    drop(keep);
}

/// Whether the memory the child wrote is still readable, having first checked
/// that it is one thing or the other.
///
/// A memory whose row said `active` and whose prose would not open, or whose
/// row said `forgotten` and whose prose opened anyway, is a half-forget in
/// one direction or the other. Both are worse than either outcome.
fn assert_all_or_nothing(session: &Session, directory: &Path) -> bool {
    let listed = session.memories().expect("the list reads back");
    assert_eq!(
        listed.memories.len(),
        1,
        "the child wrote one memory into {}: {:?}",
        directory.display(),
        listed.memories,
    );
    let row = &listed.memories[0];
    let opened = session.memory(&row.memory_id);

    match row.forget_state.as_str() {
        "active" => {
            let content = opened.unwrap_or_else(|refusal| {
                panic!("an active memory whose prose will not open: {refusal}")
            });
            assert_eq!(content.title, MEMORY_TITLE);
            assert_eq!(content.summary, MEMORY_SUMMARY);
            true
        }
        "forgotten" => {
            assert!(
                opened.is_err(),
                "a forgotten memory opened anyway: {opened:?}",
            );
            false
        }
        other => panic!("a forget left the row saying `{other}`"),
    }
}

/// AC-15 through the product: a crash between one content key and the next
/// leaves no half-forgotten memory.
#[test]
fn a_session_that_died_mid_forget_reopens_with_the_memory_whole_or_gone() {
    let (keep, directory) = scratch("forget-crash");

    let outcome = run_crashing_subprocess(
        &CrashScenario::panic_at(
            FORGET_CHILD,
            soul_testkit::crash::failpoints::FORGET_CK_DELETE_MID,
        )
        .with_env(DATA_DIR_ENV, &directory),
    )
    .expect("re-execute the test binary");
    outcome.assert_died("the forgetting child");

    let session = Session::open(&directory);
    assert!(
        session.status().store_opened,
        "a crash mid-forget left a database the next launch could not open: {}",
        session.status().store_notice,
    );

    let survived = assert_all_or_nothing(&session, Path::new(&directory));

    // The forget runs in one transaction, so the crash rolls it back. The
    // assertion above is the contract; this one records which side of it the
    // implementation lands on, and fails if the rollback stops being whole.
    assert!(
        survived,
        "a crash inside the transaction must roll the whole forget back",
    );

    assert_chain_holds_no_prose(&session, &[MEMORY_TITLE, MEMORY_SUMMARY]);
    drop(keep);
}

/// The control: with nothing armed, the same child destroys the key, and the
/// launch after it cannot read the memory either.
#[test]
fn without_an_armed_fail_point_the_forget_completes_and_the_next_launch_cannot_read_it() {
    let (keep, directory) = scratch("forget-control");

    let outcome = run_crashing_subprocess(
        &CrashScenario::new(FORGET_CHILD, "").with_env(DATA_DIR_ENV, &directory),
    )
    .expect("re-execute the test binary");
    assert!(
        !outcome.died(),
        "the control run must finish\n--- stderr ---\n{}",
        outcome.stderr,
    );

    let session = Session::open(&directory);
    let survived = assert_all_or_nothing(&session, Path::new(&directory));
    assert!(
        !survived,
        "an uninterrupted forget must destroy the memory's content key",
    );

    let chain = session.audit().expect("the chain");
    assert!(chain.verified, "{:?}", chain.verification_problem);
    assert!(
        chain
            .entries
            .iter()
            .any(|entry| entry.action == "forget.execute"),
        "a completed forget records itself: {:?}",
        chain.entries,
    );
    assert_chain_holds_no_prose(&session, &[MEMORY_TITLE, MEMORY_SUMMARY]);
    drop(keep);
}
