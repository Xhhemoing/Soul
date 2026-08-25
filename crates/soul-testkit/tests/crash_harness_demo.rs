//! Proves the crash harness kills a writer for real and that the survivor is
//! inspectable afterwards.
//!
//! The subject is a toy SQLite writer, not a Soul store: WP01 has no store to
//! crash yet. What is being demonstrated is the harness itself, so that AC-24
//! can be written against the real audit chain in WP08 without first having to
//! establish that the child actually dies.
//!
//! Shape of the run:
//!
//! * the parent creates a database path and re-executes this binary;
//! * the child commits one row, then aborts inside a second, uncommitted
//!   transaction at the `STORE_EVENT_COMMIT_MID` fail point;
//! * the parent reopens the file and checks the committed row survived, the
//!   uncommitted one did not, and the database is not corrupt.

use rusqlite::Connection;
use soul_testkit::crash::{self, failpoints, run_crashing_subprocess, CrashOutcome, CrashScenario};

const DB_PATH_ENV: &str = "SOUL_CRASH_DEMO_DB";
const CHILD_TEST: &str = "child_writer_aborts_mid_transaction";
const COMMITTED: &str = "committed-before-the-crash";
const UNCOMMITTED: &str = "never-committed";

fn open(path: &str) -> Connection {
    let conn = Connection::open(path).expect("open toy database");
    conn.pragma_update(None, "journal_mode", "WAL")
        .expect("WAL keeps the committed row recoverable after an abort");
    conn.pragma_update(None, "synchronous", "FULL")
        .expect("do not let the write sit in a buffer we are about to lose");
    conn
}

/// Runs as the re-executed child. A normal `cargo test` run skips it.
#[test]
fn child_writer_aborts_mid_transaction() {
    if !crash::is_child() {
        return;
    }
    crash::abort_if_child();
    let _scenario = fail::FailScenario::setup();

    let path = std::env::var(DB_PATH_ENV).expect("the parent passes the database path");
    let conn = open(&path);
    conn.execute("CREATE TABLE IF NOT EXISTS log (body TEXT NOT NULL)", [])
        .expect("create table");

    conn.execute("INSERT INTO log (body) VALUES (?1)", [COMMITTED])
        .expect("first row commits on its own");

    conn.execute_batch("BEGIN").expect("begin transaction");
    conn.execute("INSERT INTO log (body) VALUES (?1)", [UNCOMMITTED])
        .expect("second row is written inside the transaction");

    fail::fail_point!(failpoints::STORE_EVENT_COMMIT_MID);

    // Only reached when the fail point is disarmed, which is the control run.
    conn.execute_batch("COMMIT").expect("commit");
}

#[test]
fn a_crash_mid_transaction_loses_only_the_uncommitted_row() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("crash-demo.db");
    let path_text = path.to_string_lossy().into_owned();

    let outcome: CrashOutcome = run_crashing_subprocess(
        &CrashScenario::panic_at(CHILD_TEST, failpoints::STORE_EVENT_COMMIT_MID)
            .with_env(DB_PATH_ENV, &path_text),
    )
    .expect("re-execute the test binary");

    outcome.assert_died("the writer child");
    assert!(
        path.exists(),
        "the child should have created the database before dying",
    );

    let conn = Connection::open(&path).expect("reopen after the crash");
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .expect("integrity check runs");
    assert_eq!(integrity, "ok", "the file survived the abort intact");

    let bodies: Vec<String> = conn
        .prepare("SELECT body FROM log ORDER BY rowid")
        .expect("prepare")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows");

    assert_eq!(
        bodies,
        vec![COMMITTED.to_owned()],
        "the committed row must survive and the uncommitted one must not",
    );
}

/// Without an armed fail point the same child finishes normally, so the
/// previous test is measuring the injection rather than a broken child.
#[test]
fn the_child_only_dies_when_a_fail_point_is_armed() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("no-crash.db");
    let path_text = path.to_string_lossy().into_owned();

    let outcome = run_crashing_subprocess(
        &CrashScenario::new(CHILD_TEST, "").with_env(DB_PATH_ENV, &path_text),
    )
    .expect("re-execute the test binary");

    assert!(
        !outcome.died(),
        "with no fail point armed the child should reach COMMIT and exit 0\n--- stderr ---\n{}",
        outcome.stderr,
    );

    let conn = Connection::open(&path).expect("reopen");
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM log", [], |row| row.get(0))
        .expect("count");
    assert_eq!(count, 2, "both rows land when nothing interrupts the child");
}

#[test]
fn the_fail_point_names_are_stable() {
    assert_eq!(failpoints::ALL.len(), 4);
    for name in failpoints::ALL {
        assert!(
            name.starts_with("soul::"),
            "{name} should be namespaced so an unrelated FAILPOINTS value cannot arm it",
        );
    }
}
