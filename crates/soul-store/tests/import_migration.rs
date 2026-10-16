mod common;
use rusqlite::Connection;
use soul_schema::common::Subject;
use soul_schema::event::EventKind;
use soul_store::{KeyProvider, SqlCipherStore, TestKeyProvider};
use soul_store_api::import::{ImportIndexStore, ImportSourceKind};
use soul_store_api::EventStore;
use std::path::Path;

fn keys() -> TestKeyProvider {
    TestKeyProvider::from_seed("import migration")
}
fn raw(path: &Path) -> Connection {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(&format!(
        "PRAGMA key = \"x'{}'\";",
        keys().database_key().unwrap().to_hex()
    ))
    .unwrap();
    conn
}
fn downgrade(path: &Path) {
    let conn = raw(path);
    conn.execute_batch("DROP TABLE import_operations; DROP TABLE import_namespaces; DROP TABLE import_participants;
        DROP TABLE import_message_keys; DROP TABLE import_indexed_events;
        DELETE FROM meta WHERE key LIKE 'import.legacy.%'; UPDATE meta SET value='2' WHERE key='schema_version';").unwrap();
}

#[test]
fn version_two_gains_tables_without_inventing_message_ids_for_its_history() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("soul.db");
    let mut store = SqlCipherStore::open(&db, &keys()).unwrap();
    store
        .append_event(common::event(
            common::id("401"),
            "2026-08-24T09:00:00Z",
            EventKind::ImportItem,
            Subject::Owner,
        ))
        .unwrap();
    drop(store);
    downgrade(&db);
    let store = SqlCipherStore::open(&db, &keys()).unwrap();
    assert_eq!(
        store
            .import_legacy_events(ImportSourceKind::SoulImportV1)
            .unwrap(),
        1
    );
    assert!(store.import_namespaces().unwrap().is_empty());
    assert!(store.recent_import_operations(100).unwrap().is_empty());
    drop(store);
    let store = SqlCipherStore::open(&db, &keys()).unwrap();
    assert_eq!(
        store
            .import_legacy_events(ImportSourceKind::SoulImportV1)
            .unwrap(),
        1
    );
}

#[test]
fn a_failed_additive_migration_rolls_back_ddl_and_leaves_the_old_stamp() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("soul.db");
    drop(SqlCipherStore::open(&db, &keys()).unwrap());
    downgrade(&db);
    let conn = raw(&db);
    conn.execute_batch("CREATE TABLE import_participants(identity_id TEXT PRIMARY KEY);")
        .unwrap();
    drop(conn);
    assert!(SqlCipherStore::open(&db, &keys()).is_err());
    let conn = raw(&db);
    assert_eq!(
        conn.query_row(
            "SELECT value FROM meta WHERE key='schema_version'",
            [],
            |row| row.get::<_, String>(0)
        )
        .unwrap(),
        "2"
    );
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM sqlite_schema WHERE type='table' AND name='import_operations'",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM meta WHERE key LIKE 'import.legacy.%'",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}
