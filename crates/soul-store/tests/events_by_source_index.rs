//! What the collect screen's counter costs.
//!
//! `soulcore::commands::collect::events_collected` asks the store how many
//! `collector.foreground_app` rows there are, and the Collect screen asks it
//! once a second with the store lock held. The query is
//! `SELECT count(*) FROM events WHERE source = ?`, so its price is set by the
//! whole `events` table rather than by the handful of rows a collect run has
//! written: a Telegram import's messages are in there, and so are the rows a
//! forget turned into tombstones, since forgetting destroys a content key and
//! leaves the row.
//!
//! `events_by_source` is what keeps that poll off the table. Nothing else
//! would notice if it disappeared — the count would stay correct — so these
//! tests read the query plan rather than the answer, through a raw SQLCipher
//! connection keyed the way the store keys it.

mod common;

use std::path::Path;

use rusqlite::Connection;

use soul_schema::common::Subject;
use soul_schema::event::{EventKind, EventSource};
use soul_store::sql::STORE_SCHEMA_VERSION;
use soul_store::{KeyProvider, SqlCipherStore, TestKeyProvider};
use soul_store_api::types::EventFilter;
use soul_store_api::EventStore;

const SEED: &str = "wp02 events by source";

/// The query `events_collected` ends up running, verbatim.
const COUNT_BY_SOURCE: &str = "SELECT count(*) FROM events WHERE source = ?";

/// Rows from an import, standing in for everything in the table that the
/// foreground count is not about.
const IMPORTED_ROWS: usize = 400;

fn keys() -> TestKeyProvider {
    TestKeyProvider::from_seed(SEED)
}

fn raw(path: &Path) -> Connection {
    let conn = Connection::open(path).expect("open the database file directly");
    conn.execute_batch(&format!(
        "PRAGMA key = \"x'{}'\";",
        keys()
            .database_key()
            .expect("the same DEK the store opens under")
            .to_hex()
    ))
    .expect("key the connection");
    conn
}

/// One foreground event, and an import around it.
///
/// The import's rows are copied from a real one the store wrote, id rewritten
/// in the document as well as the column, so what the planner sees is the
/// table a collect run on a used database would poll.
fn a_database_with_an_import_in_it(path: &Path) {
    let mut store = SqlCipherStore::open(path, &keys()).expect("open");
    store
        .append_event(common::event(
            common::id("1"),
            "2026-08-24T09:00:00Z",
            EventKind::AppForeground,
            Subject::Owner,
        ))
        .expect("the one row the count is about");
    store
        .append_event(common::event(
            common::id("2"),
            "2026-08-24T10:00:00Z",
            EventKind::ImportItem,
            Subject::Owner,
        ))
        .expect("the first row it is not about");
    store.close().expect("close");

    let conn = raw(path);
    let imported = common::id("2").to_string();
    conn.execute_batch("BEGIN").expect("begin");
    for index in 0..IMPORTED_ROWS {
        conn.execute(
            "INSERT INTO events
                (event_id, ts, source, kind, actor_subject, privacy_subject, body_blob_id, doc)
             SELECT ?1, ts, source, kind, actor_subject, privacy_subject, body_blob_id,
                    replace(doc, event_id, ?1)
             FROM events WHERE event_id = ?2",
            rusqlite::params![
                common::id(&format!("{:03}", 100 + index)).to_string(),
                imported
            ],
        )
        .expect("widen the import");
    }
    conn.execute_batch("COMMIT").expect("commit");
    conn.close().expect("close the filling connection");
}

/// How SQLite says it would answer `sql`, as one string.
fn plan_for(conn: &Connection, sql: &str, source: &str) -> String {
    let mut statement = conn
        .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
        .expect("prepare the plan");
    let steps: Vec<String> = statement
        .query_map([source], |row| row.get::<_, String>(3))
        .expect("read the plan")
        .map(|step| step.expect("plan step"))
        .collect();
    steps.join("; ")
}

fn index_exists(conn: &Connection, name: &str) -> bool {
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_schema WHERE type = 'index' AND name = ?1",
            [name],
            |row| row.get(0),
        )
        .expect("read the index list");
    count == 1
}

fn stamp_on_disk(conn: &Connection) -> Option<String> {
    conn.query_row(
        "SELECT CAST(value AS TEXT) FROM meta WHERE key = 'schema_version'",
        [],
        |row| row.get::<_, String>(0),
    )
    .ok()
}

/// The foreground text the store writes, read out of a row it wrote rather
/// than spelled out here, so the test binds what `events_collected` binds.
fn foreground_source(conn: &Connection) -> String {
    conn.query_row(
        "SELECT source FROM events WHERE event_id = ?1",
        [common::id("1").to_string()],
        |row| row.get(0),
    )
    .expect("the foreground row")
}

/// The plan names the index, and the same plan without the index is a scan.
/// The second half is the part worth having: it says the first half is about
/// `events_by_source` and not about some other way SQLite found to be quick.
#[test]
fn the_foreground_count_is_answered_from_the_index_and_not_from_the_table() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("collect-poll.db");
    a_database_with_an_import_in_it(&path);

    let conn = raw(&path);
    let source = foreground_source(&conn);

    let with_index = plan_for(&conn, COUNT_BY_SOURCE, &source);
    assert!(
        with_index.contains("events_by_source"),
        "the once-a-second count has to be answered from the index: {with_index}",
    );

    conn.execute_batch("DROP INDEX events_by_source")
        .expect("take the index away");
    let without_index = plan_for(&conn, COUNT_BY_SOURCE, &source);
    assert!(
        !without_index.contains("events_by_source"),
        "the index was dropped and the plan still claims it: {without_index}",
    );
    assert!(
        without_index.contains("SCAN"),
        "without the index this poll reads every event in the database, \
         which is the cost the index exists to remove: {without_index}",
    );
    conn.close().expect("close");
}

/// D62's forward path, for an index rather than a table: a database written
/// before `events_by_source` existed gains it the next time Soul opens it,
/// and its version stamp does not move, because nothing about what the file
/// *means* changed.
#[test]
fn a_database_written_before_the_index_gains_it_on_the_next_open() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("older-file.db");
    a_database_with_an_import_in_it(&path);

    let conn = raw(&path);
    conn.execute_batch("DROP INDEX events_by_source")
        .expect("what a file from before the index looks like");
    let stamp_before = stamp_on_disk(&conn);
    assert!(!index_exists(&conn, "events_by_source"));
    conn.close().expect("close");

    let store = SqlCipherStore::open(&path, &keys()).expect("an older file still opens");
    assert_eq!(
        store
            .count_events(&EventFilter::with_source(
                EventSource::CollectorForegroundApp
            ))
            .expect("count"),
        1,
        "the index may not change the answer, only what reaching it costs",
    );
    store.close().expect("close");

    let conn = raw(&path);
    assert!(
        index_exists(&conn, "events_by_source"),
        "the additive DDL has to give an older file the index it lacks",
    );
    assert_eq!(
        stamp_on_disk(&conn),
        stamp_before,
        "an index is not a schema version: adding one may not restamp the file",
    );
    assert_eq!(
        stamp_on_disk(&conn).as_deref(),
        Some(STORE_SCHEMA_VERSION.to_string().as_str()),
    );
    conn.close().expect("close");
}
