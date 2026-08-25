//! D62: which databases this build may open, and what it does to the ones it
//! refuses.
//!
//! `docs/DECISIONS.md` D62 allows exactly one direction of travel. Forward is
//! additive — every statement in `sql::DDL` is `IF NOT EXISTS`, so a version 1
//! file gains `destroyed_content_keys` and has its stamp moved up. Backward is
//! not travel, it is damage: `SqlCipherStore::open` used to upsert
//! `meta.schema_version = 2` unconditionally, so a database written by a newer
//! Soul was relabelled as one this build had written, and then written to
//! under whatever this build assumes the schema means. The label is the part
//! that does not heal, because every launch after it reads the lie and finds
//! nothing wrong.
//!
//! So the refusal has to happen before the file is touched at all: before the
//! WAL and secure-delete pragmas, before the DDL, before the upsert. These
//! tests plant stamps through a raw SQLCipher connection — the same DEK the
//! store would use, so the plant is a real database and not a fixture — and
//! then check both halves of the answer: that the open fails, and that the
//! bytes it failed on are the bytes that were there before.

use std::path::Path;

use rusqlite::Connection;

use soul_store::sql::STORE_SCHEMA_VERSION;
use soul_store::{KeyProvider, SqlCipherStore, TestKeyProvider};

const SEED: &str = "wp02 schema version";

fn keys() -> TestKeyProvider {
    TestKeyProvider::from_seed(SEED)
}

/// A connection to the store's own file, keyed the way the store keys it.
///
/// Everything the tests do out of band goes through this: planting a stamp a
/// future build would have written, dropping a table a past build did not
/// have, and reading either of those back without asking `open` — which is the
/// only way to see what a refused open left behind.
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

/// Run `statements` against the file and leave nothing in the write-ahead log,
/// so a later read of the bytes on disk sees what was written.
fn plant(path: &Path, statements: &str) {
    let conn = raw(path);
    conn.execute_batch(statements).expect("plant");
    conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
        row.get::<_, i64>(0)
    })
    .expect("fold the log back into the file");
    conn.close().expect("close the planting connection");
}

fn stamp_on_disk(path: &Path) -> Option<String> {
    let conn = raw(path);
    let stamp = conn
        .query_row(
            "SELECT CAST(value AS TEXT) FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok();
    conn.close().expect("close the reading connection");
    stamp
}

fn table_exists(path: &Path, name: &str) -> bool {
    let conn = raw(path);
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_schema WHERE type = 'table' AND name = ?1",
            [name],
            |row| row.get(0),
        )
        .expect("read the table list");
    conn.close().expect("close the reading connection");
    count == 1
}

/// Every name in the directory, sorted. A refusal may not add one either.
fn file_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("read the directory")
        .map(|entry| {
            entry
                .expect("directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

/// (a) The ordinary path: a database this build created carries this build's
/// version, and opening it again is not an event.
#[test]
fn a_database_this_build_created_is_stamped_with_this_build_s_version() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("soul.db");

    SqlCipherStore::open(&path, &keys())
        .expect("first open")
        .close()
        .expect("close");
    assert_eq!(
        stamp_on_disk(&path).as_deref(),
        Some(STORE_SCHEMA_VERSION.to_string().as_str()),
        "a fresh database has to say which schema it holds",
    );

    SqlCipherStore::open(&path, &keys())
        .expect("a database at this version reopens")
        .close()
        .expect("close");
    assert_eq!(
        stamp_on_disk(&path).as_deref(),
        Some(STORE_SCHEMA_VERSION.to_string().as_str()),
    );
}

/// (b) The failure D62 exists for: a database from a build that knew more than
/// this one.
///
/// Refusing to read it is the easy half. The half that used to be wrong is
/// that the refusal must not have written anything first — the stamp above all,
/// because a 99 quietly turned into a 2 is a database that never tells anyone
/// again that it came from somewhere else.
#[test]
fn a_database_from_a_newer_build_is_refused_and_not_written_to() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("soul.db");

    SqlCipherStore::open(&path, &keys())
        .expect("create it with this build")
        .close()
        .expect("close");
    plant(
        &path,
        "UPDATE meta SET value = '99' WHERE key = 'schema_version'",
    );

    let before = std::fs::read(&path).expect("the file as the newer build left it");
    let names_before = file_names(dir.path());

    let refusal = SqlCipherStore::open(&path, &keys())
        .err()
        .expect("this build must not open a version 99 database");

    let said = refusal.to_string();
    let path_text = path.display().to_string();
    assert!(
        said.contains(&path_text),
        "the refusal has to name the database it is about: {said}",
    );
    // The path itself carries digits from the temporary directory, so it is
    // taken out before the two versions are looked for in what is left.
    let versions = said.replace(&path_text, "<the database>");
    assert!(
        versions.contains("99"),
        "the refusal has to say what the file claims to be: {said}",
    );
    assert!(
        versions.contains(&STORE_SCHEMA_VERSION.to_string()),
        "the refusal has to say what this build understands: {said}",
    );

    assert_eq!(
        stamp_on_disk(&path).as_deref(),
        Some("99"),
        "the refused open stamped the newer database down to its own version",
    );
    assert_eq!(
        std::fs::read(&path).expect("the file afterwards"),
        before,
        "the refused open changed the bytes of a database it cannot understand",
    );
    assert_eq!(
        file_names(dir.path()),
        names_before,
        "the refused open left something behind beside the database",
    );
}

/// A stamp that is not a version at all is the same answer. This build cannot
/// place it either, and guessing that it means "old" would put it straight
/// back on the path where a newer file is written to by an older build.
#[test]
fn a_stamp_that_is_not_a_version_is_refused_the_same_way() {
    let dir = tempfile::tempdir().expect("temp dir");

    // The last of these needs a `meta` whose value column is nullable, which
    // this build's DDL is not — but a future build's could be, and a stamp this
    // build cannot read is the case under test either way.
    for (case, planted, expected) in [
        (
            "prose",
            "UPDATE meta SET value = 'tomorrow' WHERE key = 'schema_version'",
            "tomorrow",
        ),
        (
            "a-float",
            "UPDATE meta SET value = 2.5 WHERE key = 'schema_version'",
            "2.5",
        ),
        (
            "nothing",
            "DROP TABLE meta;
             CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT);
             INSERT INTO meta (key, value) VALUES ('schema_version', NULL);",
            "NULL",
        ),
    ] {
        let path = dir.path().join(format!("{case}.db"));
        SqlCipherStore::open(&path, &keys())
            .expect("create it with this build")
            .close()
            .expect("close");
        plant(&path, planted);

        let before = std::fs::read(&path).expect("the planted file");
        let refusal = SqlCipherStore::open(&path, &keys())
            .err()
            .unwrap_or_else(|| panic!("{case}: an unreadable stamp must not be opened past"));
        assert!(
            refusal.to_string().contains(expected),
            "{case}: the refusal has to quote what it found: {refusal}",
        );
        assert_eq!(
            std::fs::read(&path).expect("the file afterwards"),
            before,
            "{case}: the refused open wrote to the file",
        );
    }
}

/// (c) The forward path, which is the one that has to keep working: a version 1
/// database is a version 2 database after this build has opened it, table and
/// stamp both.
#[test]
fn a_version_one_database_gains_the_table_it_was_missing_and_is_stamped_forward() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("soul.db");

    SqlCipherStore::open(&path, &keys())
        .expect("create it with this build")
        .close()
        .expect("close");
    // What a version 1 file is: no ledger of destroyed content keys, and a
    // stamp that says so.
    plant(
        &path,
        "DROP TABLE IF EXISTS destroyed_content_keys;
         UPDATE meta SET value = '1' WHERE key = 'schema_version';",
    );
    assert!(!table_exists(&path, "destroyed_content_keys"));

    SqlCipherStore::open(&path, &keys())
        .expect("a version 1 database still opens")
        .close()
        .expect("close");

    assert!(
        table_exists(&path, "destroyed_content_keys"),
        "the additive DDL has to give an older file the table it lacks",
    );
    assert_eq!(
        stamp_on_disk(&path).as_deref(),
        Some(STORE_SCHEMA_VERSION.to_string().as_str()),
        "an upgraded file has to stop calling itself version 1",
    );
}

/// A file from before the stamp existed — or one whose `meta` table has not
/// been created yet — is not a newer file, and must not be treated as one.
#[test]
fn a_database_with_no_stamp_at_all_is_adopted_rather_than_refused() {
    let dir = tempfile::tempdir().expect("temp dir");

    let no_row = dir.path().join("no-row.db");
    SqlCipherStore::open(&no_row, &keys())
        .expect("create it")
        .close()
        .expect("close");
    plant(&no_row, "DELETE FROM meta WHERE key = 'schema_version'");
    SqlCipherStore::open(&no_row, &keys())
        .expect("a file with no version row is a file this build may write")
        .close()
        .expect("close");
    assert_eq!(
        stamp_on_disk(&no_row).as_deref(),
        Some(STORE_SCHEMA_VERSION.to_string().as_str()),
    );

    let no_table = dir.path().join("no-table.db");
    plant(&no_table, "CREATE TABLE placeholder (id TEXT PRIMARY KEY)");
    SqlCipherStore::open(&no_table, &keys())
        .expect("a database with no meta table is not a newer one")
        .close()
        .expect("close");
    assert_eq!(
        stamp_on_disk(&no_table).as_deref(),
        Some(STORE_SCHEMA_VERSION.to_string().as_str()),
    );
}
