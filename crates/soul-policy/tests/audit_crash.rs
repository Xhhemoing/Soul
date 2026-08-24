//! AC-24, audit side: kill the process inside an audit append and reopen.
//!
//! WP02 covered the event table. The audit table is the other half, and it is
//! the harder one, because an audit entry links to the one before it: a
//! half-written entry does not just lose data, it can leave a `prev_hash` that
//! nothing hashes to, and then the chain never verifies again.
//!
//! Two injection sites, one either side of the commit:
//!
//! * `AUDIT_APPEND_PRE_COMMIT` — the row and its subject links are written and
//!   the transaction is open. Reopening must show the entry gone entirely.
//! * `AUDIT_APPEND_POST_WRITE` — the commit went through and the caller was
//!   never told. Reopening must show the entry present, and the chain must
//!   still verify with the caller's bookkeeping one behind.
//!
//! The child is a re-executed copy of this test binary that aborts on panic,
//! so no destructor runs and SQLite gets no chance to tidy up. Each scenario
//! has a control run with no fail point armed, which is what stops a broken
//! child from passing as a successful crash.

use soul_policy::audit::{self, AuditContent, ReasonCode};
use soul_schema::audit::AuditAction;
use soul_store::{failpoints, SqlCipherStore, TestKeyProvider};
use soul_store_api::{AuditLog, SoulStore};
use soul_testkit::crash::{self, run_crashing_subprocess, CrashScenario};

const DB_PATH_ENV: &str = "SOUL_POLICY_AUDIT_CRASH_DB";
const SEED: &str = "wp08 audit crash";
const AT: i64 = 1_787_913_600;

const CHILD: &str = "child_appends_three_audit_entries_and_may_die_in_the_third";
const ENTRIES_ATTEMPTED: usize = 3;

fn open(path: &str) -> SqlCipherStore {
    SqlCipherStore::open(path, &TestKeyProvider::from_seed(SEED)).expect("open the encrypted store")
}

fn child_db_path() -> String {
    std::env::var(DB_PATH_ENV).expect("the parent passes the database path")
}

// -------------------------------------------------------------- the child ---

/// Runs only as the re-executed child. A normal `cargo test` run skips it.
#[test]
fn child_appends_three_audit_entries_and_may_die_in_the_third() {
    if !crash::is_child() {
        return;
    }
    crash::abort_if_child();
    let _scenario = fail::FailScenario::setup();

    let mut store = open(&child_db_path());
    for index in 0..ENTRIES_ATTEMPTED {
        audit::append(
            &mut store,
            AuditContent::allowed(AuditAction::MemoryWrite, ReasonCode::Routine),
            AT + index as i64,
        )
        .expect("append");
    }
    store.flush().expect("flush");
}

// ------------------------------------------------------------- the parents ---

#[test]
fn the_fail_point_names_match_the_test_kit() {
    assert_eq!(
        failpoints::AUDIT_APPEND_PRE_COMMIT,
        soul_testkit::crash::failpoints::AUDIT_APPEND_PRE_COMMIT,
    );
    assert_eq!(
        failpoints::AUDIT_APPEND_POST_WRITE,
        soul_testkit::crash::failpoints::AUDIT_APPEND_POST_WRITE,
    );
}

fn crash_at(failpoint: &str, file: &str) -> (SqlCipherStore, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join(file);
    let path_text = path.to_string_lossy().into_owned();

    // `2*off->panic` lets the first two appends through and kills the third.
    let outcome = run_crashing_subprocess(
        &CrashScenario::new(CHILD, format!("{failpoint}=2*off->panic"))
            .with_env(DB_PATH_ENV, &path_text),
    )
    .expect("re-execute the test binary");
    outcome.assert_died("the audit writer child");

    (open(&path_text), dir)
}

#[test]
fn a_crash_before_the_commit_loses_the_whole_entry_and_leaves_the_chain_verifiable() {
    let (store, _dir) = crash_at(failpoints::AUDIT_APPEND_PRE_COMMIT, "audit-pre-commit.db");

    assert_eq!(store.integrity_check().expect("integrity"), "ok");
    store
        .verify_audit_chain()
        .expect("the chain must verify after a crash before the commit");

    let entries = store.list_audit().expect("read the chain back");
    assert_eq!(
        entries.len(),
        ENTRIES_ATTEMPTED - 1,
        "the uncommitted entry must be gone, and at most one may be lost",
    );

    // Nothing half-written: the surviving links are contiguous from genesis.
    let links = store.audit_links().expect("links");
    assert_eq!(links.iter().map(|l| l.seq).collect::<Vec<_>>(), vec![0, 1]);
    assert_eq!(links[0].prev_hash, soul_store::GENESIS_PREV_HASH);
    assert_eq!(links[1].prev_hash, links[0].entry_hash);
}

#[test]
fn a_crash_after_the_write_keeps_the_entry_and_leaves_the_chain_verifiable() {
    let (store, _dir) = crash_at(failpoints::AUDIT_APPEND_POST_WRITE, "audit-post-write.db");

    assert_eq!(store.integrity_check().expect("integrity"), "ok");
    store
        .verify_audit_chain()
        .expect("the chain must verify after a crash following the commit");

    let entries = store.list_audit().expect("read the chain back");
    assert_eq!(
        entries.len(),
        ENTRIES_ATTEMPTED,
        "the entry was committed before the crash, so it must survive even \
         though the caller never learned its id",
    );

    let links = store.audit_links().expect("links");
    assert_eq!(
        links.iter().map(|l| l.seq).collect::<Vec<_>>(),
        vec![0, 1, 2],
    );
}

/// Appending after either crash must continue the same chain, not start a new
/// one. This is the property a broken `prev_hash` would destroy, and it is
/// invisible to a test that only reopens and verifies.
#[test]
fn the_chain_can_be_extended_after_a_crash() {
    for (failpoint, file, expected_before) in [
        (
            failpoints::AUDIT_APPEND_PRE_COMMIT,
            "audit-extend-pre.db",
            ENTRIES_ATTEMPTED - 1,
        ),
        (
            failpoints::AUDIT_APPEND_POST_WRITE,
            "audit-extend-post.db",
            ENTRIES_ATTEMPTED,
        ),
    ] {
        let (mut store, _dir) = crash_at(failpoint, file);
        assert_eq!(store.list_audit().expect("before").len(), expected_before);

        audit::append(
            &mut store,
            AuditContent::allowed(AuditAction::ForgetExecute, ReasonCode::Routine),
            AT + 100,
        )
        .expect("append after the crash");

        store
            .verify_audit_chain()
            .unwrap_or_else(|error| panic!("{failpoint}: chain broke on extension: {error}"));
        assert_eq!(
            store.list_audit().expect("after").len(),
            expected_before + 1,
        );
    }
}

#[test]
fn without_an_armed_fail_point_the_same_child_writes_everything() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("audit-control.db");
    let path_text = path.to_string_lossy().into_owned();

    let outcome =
        run_crashing_subprocess(&CrashScenario::new(CHILD, "").with_env(DB_PATH_ENV, &path_text))
            .expect("re-execute the test binary");
    assert!(
        !outcome.died(),
        "the control run measures the injection, not a broken child\n--- stderr ---\n{}",
        outcome.stderr,
    );

    let store = open(&path_text);
    assert_eq!(store.list_audit().expect("audit").len(), ENTRIES_ATTEMPTED);
    store.verify_audit_chain().expect("chain");
}
