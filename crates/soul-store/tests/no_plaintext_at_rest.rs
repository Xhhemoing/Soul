//! AC-04, storage half: what is written must not be legible in the file.
//!
//! Two layers have to hold for that, and this test would notice either one
//! going missing. SQLCipher encrypts the pages, so a plain `Connection::open`
//! cannot read the tables at all. The field seal encrypts the prose again under
//! a content key, so even a reader holding the database key finds ciphertext
//! where the summary should be.
//!
//! The scan covers every file the store leaves behind, not just `soul.db`: the
//! write-ahead log and its shared-memory file are part of the database, and a
//! needle that only ever appeared in the `-wal` would be just as much of a leak.

mod common;

use common::*;

use soul_schema::event::EventKind;
use soul_schema::Subject;
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::{BlobStore, EventStore, MemoryStore, SoulStore};

const SEED: &str = "wp02 plaintext";

/// Needles chosen to be improbable and to cover both scripts: an ASCII run
/// cannot hide in UTF-8 padding, and the CJK run is what a real diary entry
/// looks like.
const NEEDLES: &[&str] = &[
    "SOUL-WP02-PLAINTEXT-NEEDLE-ASCII",
    "站台上的风把围巾吹到了铁轨那边",
    "SOUL-WP02-EVENT-BODY-NEEDLE",
];

#[test]
fn nothing_written_through_the_store_is_legible_in_the_files_on_disk() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("soul.db");
    let keys = TestKeyProvider::from_seed(SEED);

    let sealed_summary = {
        let mut store = SqlCipherStore::open(&path, &keys).expect("open");

        let title = store
            .seal(owner_seal(id("60"), id("50"), "title_ref", NEEDLES[0]))
            .expect("seal title");
        let summary = store
            .seal(owner_seal(id("60"), id("50"), "summary_ref", NEEDLES[1]))
            .expect("seal summary");
        let body = store
            .seal(owner_seal(id("61"), id("52"), "body_ref", NEEDLES[2]))
            .expect("seal event body");

        let mut carrier = event(
            id("52"),
            "2026-08-24T09:00:00Z",
            EventKind::MessageObserved,
            Subject::Owner,
        );
        carrier.body_ref = Some(body);
        store.append_event(carrier).expect("append");
        store
            .put_memory(memory(
                id("50"),
                id("60"),
                Some(title),
                Some(summary.clone()),
                &[],
            ))
            .expect("memory");

        store.flush().expect("fold the write-ahead log back in");
        store.close().expect("close");
        summary
    };

    let files = files_in(dir.path());
    assert!(
        files.iter().any(|(name, _)| name == "soul.db"),
        "the database file should exist; found {:?}",
        files.iter().map(|(name, _)| name).collect::<Vec<_>>(),
    );

    for needle in NEEDLES {
        for (name, bytes) in &files {
            assert!(
                !contains(bytes, needle.as_bytes()),
                "{needle:?} is legible in {name}",
            );
        }
    }

    // The test would also pass if the store had silently written nothing, so
    // check the prose is still recoverable through the front door.
    let store = SqlCipherStore::open(&path, &keys).expect("reopen");
    assert_eq!(
        String::from_utf8(store.open(&sealed_summary).expect("open")).expect("utf-8"),
        NEEDLES[1],
    );
}

#[test]
fn a_reader_without_the_database_key_cannot_even_list_the_tables() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("keyed.db");

    {
        let mut store =
            SqlCipherStore::open(&path, &TestKeyProvider::from_seed(SEED)).expect("open");
        store
            .seal(owner_seal(id("60"), id("50"), "summary_ref", NEEDLES[1]))
            .expect("seal");
        store.flush().expect("flush");
        store.close().expect("close");
    }

    let unkeyed = rusqlite::Connection::open(&path).expect("opening the file itself succeeds");
    let attempt =
        unkeyed.query_row::<i64, _, _>("SELECT count(*) FROM sealed_blobs", [], |row| row.get(0));
    assert!(
        attempt.is_err(),
        "an unkeyed reader must not get past the first read; got {attempt:?}",
    );
}

/// Every regular file in the directory, as `(file name, bytes)`.
fn files_in(dir: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).expect("read the store directory") {
        let entry = entry.expect("directory entry");
        if !entry.file_type().expect("file type").is_file() {
            continue;
        }
        out.push((
            entry.file_name().to_string_lossy().into_owned(),
            std::fs::read(entry.path()).expect("read the file"),
        ));
    }
    out.sort_by(|left, right| left.0.cmp(&right.0));
    out
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    needle.len() <= haystack.len() && haystack.windows(needle.len()).any(|w| w == needle)
}
