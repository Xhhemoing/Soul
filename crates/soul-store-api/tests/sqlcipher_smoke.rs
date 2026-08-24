//! Feasibility smoke test for the encrypted main database.
//!
//! PRODUCT_LOCK assumes SQLCipher and falls back to plain SQLite plus
//! field-level AEAD only if packaging proves impossible. WP01's job is to
//! settle that question with evidence on both CI hosts before WP02 builds on
//! it, so this test must run on ubuntu and on windows-latest alike.
//!
//! What it proves: a keyed database can be created and written; reopening with
//! the key returns the row; reopening without the key fails; and the plaintext
//! is absent from the file on disk.

use rusqlite::Connection;

const KEY: &str = "correct horse battery staple";
const NEEDLE: &str = "SOUL_SQLCIPHER_PLAINTEXT_NEEDLE";

fn open_keyed(path: &std::path::Path) -> Connection {
    let conn = Connection::open(path).expect("open database file");
    conn.pragma_update(None, "key", KEY)
        .expect("PRAGMA key is accepted, which means this is a SQLCipher build");
    conn
}

#[test]
fn sqlcipher_is_available_and_actually_encrypts() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("soul-smoke.db");

    let conn = open_keyed(&path);
    let cipher_version: String = conn
        .query_row("PRAGMA cipher_version", [], |row| row.get(0))
        .expect("a plain SQLite build has no cipher_version, so this must return one");
    assert!(
        !cipher_version.trim().is_empty(),
        "cipher_version must name a SQLCipher release",
    );

    conn.execute("CREATE TABLE sealed_probe (body TEXT NOT NULL)", [])
        .expect("create table");
    conn.execute("INSERT INTO sealed_probe (body) VALUES (?1)", [NEEDLE])
        .expect("insert");
    conn.close().expect("close cleanly so the file is flushed");

    let reopened = open_keyed(&path);
    let body: String = reopened
        .query_row("SELECT body FROM sealed_probe", [], |row| row.get(0))
        .expect("the same key must reopen the database");
    assert_eq!(body, NEEDLE);
    reopened.close().expect("close");

    let unkeyed = Connection::open(&path).expect("opening the file itself always succeeds");
    let attempt = unkeyed.query_row::<String, _, _>("SELECT body FROM sealed_probe", [], |row| {
        row.get(0)
    });
    assert!(
        attempt.is_err(),
        "reading without the key must fail; got {attempt:?}",
    );

    let raw = std::fs::read(&path).expect("read the database file");
    assert!(
        !raw.windows(NEEDLE.len()).any(|w| w == NEEDLE.as_bytes()),
        "the row body must not be legible in the file on disk",
    );
}

/// A wrong key must not silently produce an empty-but-usable database.
#[test]
fn a_wrong_key_cannot_read_the_database() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("soul-wrong-key.db");

    let conn = open_keyed(&path);
    conn.execute("CREATE TABLE sealed_probe (body TEXT NOT NULL)", [])
        .expect("create table");
    conn.execute("INSERT INTO sealed_probe (body) VALUES (?1)", [NEEDLE])
        .expect("insert");
    conn.close().expect("close");

    let wrong = Connection::open(&path).expect("open file");
    wrong
        .pragma_update(None, "key", "not the right key")
        .expect("PRAGMA key itself does not verify anything");
    let attempt =
        wrong.query_row::<String, _, _>("SELECT body FROM sealed_probe", [], |row| row.get(0));
    assert!(attempt.is_err(), "a wrong key must fail the first read");
}
