//! AC-24 for the store, and the mid-forget half of AC-15.
//!
//! Both use `soul_testkit::crash`, which re-executes this test binary and lets
//! the child abort inside an armed fail point. Nothing here simulates a crash
//! by returning an error: an error unwinds, runs destructors and lets SQLite
//! tidy up, which is exactly the behaviour a power loss does not have. The
//! child really dies, and the parent then reopens whatever the file happens to
//! contain.
//!
//! Two scenarios:
//!
//! * `STORE_EVENT_COMMIT_MID` fires after the third event row is written and
//!   before its transaction commits. Reopening must show the two committed
//!   events, exactly one lost, and a chain that still verifies.
//! * `FORGET_CK_DELETE_MID` fires between destroying one content key and the
//!   next. Reopening must show every key of the unit in the same state — all
//!   present and decryptable, or all destroyed and undecryptable — never a
//!   half-forgotten memory.

mod common;

use common::*;

use soul_schema::audit::AuditAction;
use soul_schema::common::SealedText;
use soul_schema::event::EventKind;
use soul_schema::Subject;
use soul_store::{failpoints, SqlCipherStore, TestKeyProvider};
use soul_store_api::forget::ForgetUnit;
use soul_store_api::types::{EventFilter, StoreError};
use soul_store_api::{
    AuditLog, BlobStore, EventStore, ForgetOps, MemoryStore, ProfileStore, SoulStore,
};
use soul_testkit::crash::{self, run_crashing_subprocess, CrashScenario};

const DB_PATH_ENV: &str = "SOUL_STORE_CRASH_DB";
const SEED: &str = "wp02 crash";

const EVENT_CHILD: &str = "child_appends_three_events_and_may_die_in_the_third";
const FORGET_CHILD: &str = "child_forgets_a_two_key_memory_and_may_die_between_deletions";

const EVENTS_ATTEMPTED: usize = 3;
const MEMORY: &str = "50";
const KEY_TITLE: &str = "60";
const KEY_SUMMARY: &str = "61";
const TITLE: &str = "北京";
const SUMMARY: &str = "第一次去北京，站台上风很大";

fn child_db_path() -> String {
    std::env::var(DB_PATH_ENV).expect("the parent passes the database path")
}

fn open(path: &str) -> SqlCipherStore {
    SqlCipherStore::open(path, &TestKeyProvider::from_seed(SEED)).expect("open the encrypted store")
}

fn event_id(index: usize) -> uuid::Uuid {
    id(&format!("1{index:02}"))
}

// ------------------------------------------------------------- children ---

/// Runs only as the re-executed child. A normal `cargo test` run skips it.
#[test]
fn child_appends_three_events_and_may_die_in_the_third() {
    if !crash::is_child() {
        return;
    }
    crash::abort_if_child();
    let _scenario = fail::FailScenario::setup();

    let mut store = open(&child_db_path());
    for index in 0..EVENTS_ATTEMPTED {
        let id_of_event = event_id(index);
        store
            .append_event(event(
                id_of_event,
                "2026-08-24T09:00:00Z",
                EventKind::AppForeground,
                Subject::Owner,
            ))
            .expect("append");
        store
            .append_audit(audit_entry(
                id(&format!("2{index:02}")),
                AuditAction::CollectStart,
                &[id_of_event],
            ))
            .expect("audit");
    }
    store.flush().expect("flush");
}

#[test]
fn child_forgets_a_two_key_memory_and_may_die_between_deletions() {
    if !crash::is_child() {
        return;
    }
    crash::abort_if_child();
    let _scenario = fail::FailScenario::setup();

    let mut store = open(&child_db_path());
    store
        .execute_forget(ForgetUnit::Memory(id(MEMORY)))
        .expect("execute forget");
    store.flush().expect("flush");
}

// -------------------------------------------------------------- parents ---

#[test]
fn the_fail_point_names_match_the_test_kit() {
    assert_eq!(
        failpoints::STORE_EVENT_COMMIT_MID,
        soul_testkit::crash::failpoints::STORE_EVENT_COMMIT_MID,
    );
    assert_eq!(
        failpoints::FORGET_CK_DELETE_MID,
        soul_testkit::crash::failpoints::FORGET_CK_DELETE_MID,
    );
}

#[test]
fn a_crash_mid_commit_loses_one_event_and_leaves_the_chain_verifiable() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("event-crash.db");
    let path_text = path.to_string_lossy().into_owned();

    // `2*off->panic` lets the first two appends through and kills the third
    // inside its transaction.
    let outcome = run_crashing_subprocess(
        &CrashScenario::new(
            EVENT_CHILD,
            format!("{}=2*off->panic", failpoints::STORE_EVENT_COMMIT_MID),
        )
        .with_env(DB_PATH_ENV, &path_text),
    )
    .expect("re-execute the test binary");
    outcome.assert_died("the event writer child");

    let store = open(&path_text);
    assert_eq!(store.integrity_check().expect("integrity"), "ok");

    let survivors = store
        .list_events(&EventFilter::all())
        .expect("read the events back");
    assert_eq!(
        survivors.len(),
        EVENTS_ATTEMPTED - 1,
        "at most one uncommitted event may be lost",
    );
    assert!(
        survivors.iter().all(|e| e.event_id != event_id(2)),
        "the event that was mid-commit must not be half-present",
    );
    for index in 0..EVENTS_ATTEMPTED - 1 {
        store
            .get_event(event_id(index))
            .unwrap_or_else(|_| panic!("committed event {index} must survive"));
    }

    store
        .verify_audit_chain()
        .expect("the audit chain must verify after the crash");
    assert_eq!(
        store.list_audit().expect("audit").len(),
        EVENTS_ATTEMPTED - 1,
        "one audit entry per committed event",
    );
}

#[test]
fn without_an_armed_fail_point_the_same_child_writes_everything() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("event-control.db");
    let path_text = path.to_string_lossy().into_owned();

    let outcome = run_crashing_subprocess(
        &CrashScenario::new(EVENT_CHILD, "").with_env(DB_PATH_ENV, &path_text),
    )
    .expect("re-execute the test binary");
    assert!(
        !outcome.died(),
        "the control run measures the injection, not a broken child\n--- stderr ---\n{}",
        outcome.stderr,
    );

    let store = open(&path_text);
    assert_eq!(
        store.count_events(&EventFilter::all()).expect("count"),
        EVENTS_ATTEMPTED as u64,
    );
    store.verify_audit_chain().expect("chain");
}

/// Seed a memory sealed under two content keys, so the fail point between the
/// two deletions has something to interrupt.
fn seed_two_key_memory(path: &str) -> (SealedText, SealedText) {
    let mut store = open(path);
    let title = store
        .seal(owner_seal(id(KEY_TITLE), id(MEMORY), "title_ref", TITLE))
        .expect("seal title");
    let summary = store
        .seal(owner_seal(
            id(KEY_SUMMARY),
            id(MEMORY),
            "summary_ref",
            SUMMARY,
        ))
        .expect("seal summary");
    store
        .put_evidence(evidence(id("70"), Subject::Owner))
        .expect("evidence");
    store
        .put_inference(inference(id("80"), &[id("70")]))
        .expect("inference");
    store
        .put_memory(memory(
            id(MEMORY),
            id(KEY_TITLE),
            Some(title.clone()),
            Some(summary.clone()),
            &[id("70")],
        ))
        .expect("memory");
    store
        .append_audit(audit_entry(
            id("90"),
            AuditAction::MemoryWrite,
            &[id(MEMORY)],
        ))
        .expect("audit");
    store.flush().expect("flush");
    store.close().expect("close");
    (title, summary)
}

/// Either every key of the unit is still there and the prose opens, or every
/// key is gone and none of it opens. A memory whose title survived its summary
/// would be a half-kept promise in both directions.
fn assert_all_or_nothing(store: &SqlCipherStore, sealed: &[&SealedText]) -> bool {
    let present: Vec<bool> = sealed
        .iter()
        .map(|s| store.has_content_key(s.content_key_id))
        .collect();
    let all_present = present.iter().all(|p| *p);
    let none_present = present.iter().all(|p| !*p);
    assert!(
        all_present || none_present,
        "the forget must be all-or-nothing across the unit's keys, got {present:?}",
    );

    for (sealed, key_present) in sealed.iter().zip(&present) {
        let opened = store.open(sealed);
        if *key_present {
            assert!(
                opened.is_ok(),
                "a key that survived must still decrypt its blob; got {opened:?}",
            );
        } else {
            assert!(
                matches!(opened, Err(StoreError::ContentKeyDestroyed(_))),
                "a destroyed key must make the prose unrecoverable; got {opened:?}",
            );
        }
    }
    all_present
}

#[test]
fn a_crash_between_content_key_deletions_leaves_no_half_forgotten_memory() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("forget-crash.db");
    let path_text = path.to_string_lossy().into_owned();
    let (title, summary) = seed_two_key_memory(&path_text);

    let outcome = run_crashing_subprocess(
        &CrashScenario::panic_at(FORGET_CHILD, failpoints::FORGET_CK_DELETE_MID)
            .with_env(DB_PATH_ENV, &path_text),
    )
    .expect("re-execute the test binary");
    outcome.assert_died("the forgetting child");

    let store = open(&path_text);
    assert_eq!(store.integrity_check().expect("integrity"), "ok");

    let survived = assert_all_or_nothing(&store, &[&title, &summary]);

    // The forget runs in one transaction, so the crash rolls it back. The
    // assertion above is the contract; this one records which side of it the
    // implementation lands on, and fails if the rollback stops being complete.
    assert!(
        survived,
        "a crash inside the transaction must roll the whole forget back",
    );
    assert_eq!(
        store.get_memory(id(MEMORY)).expect("row").forget_state,
        soul_schema::memory::ForgetState::Active,
        "a rolled-back forget must not leave a tombstone behind",
    );

    store
        .verify_audit_chain()
        .expect("the audit chain must survive a crash mid-forget");
    assert_eq!(store.list_audit().expect("audit").len(), 1);
}

#[test]
fn without_an_armed_fail_point_the_forget_completes_and_the_prose_is_gone() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("forget-control.db");
    let path_text = path.to_string_lossy().into_owned();
    let (title, summary) = seed_two_key_memory(&path_text);

    let outcome = run_crashing_subprocess(
        &CrashScenario::new(FORGET_CHILD, "").with_env(DB_PATH_ENV, &path_text),
    )
    .expect("re-execute the test binary");
    assert!(
        !outcome.died(),
        "the control run must finish\n--- stderr ---\n{}",
        outcome.stderr,
    );

    let store = open(&path_text);
    let survived = assert_all_or_nothing(&store, &[&title, &summary]);
    assert!(
        !survived,
        "an uninterrupted forget must destroy every key of the unit",
    );
    store.verify_audit_chain().expect("chain");
}
