//! A refused session edit must not leave the data it failed to audit.
//!
//! The trigger fails the last write in each real SQLCipher command, after its
//! evidence, profile or sealed memory has been written. Removing the session's
//! transaction must make these tests fail even though each individual store
//! write still uses its own savepoint.

use rusqlite::types::Value;
use rusqlite::Connection;
use soul_store::{DpapiKeyProvider, KeyProvider, TestKeyProvider};
use soulcore::commands::memory::{MemoryChange, NewMemory};
use soulcore::commands::session::{Session, SessionRefusal, KEY_BLOB_FILE_NAME};
use soulcore::commands::store::database_path;

type TableRows = Vec<Vec<Value>>;

fn scratch() -> (tempfile::TempDir, Session, Connection) {
    let directory = tempfile::tempdir().expect("a temporary data directory");
    let session = Session::open(directory.path());
    assert!(
        session.status().store_opened,
        "{}",
        session.status().store_notice
    );

    // This extra connection belongs only to the test: it plants the fault and
    // observes durable rows, while the command uses the session's own store.
    let keys: Box<dyn KeyProvider> = if cfg!(windows) {
        Box::new(DpapiKeyProvider::new(
            directory.path().join(KEY_BLOB_FILE_NAME),
        ))
    } else {
        Box::new(TestKeyProvider::in_dir(directory.path()))
    };
    let conn = Connection::open(database_path(directory.path())).expect("fault connection");
    conn.execute_batch(&format!(
        "PRAGMA key = \"x'{}'\";",
        keys.database_key()
            .expect("the session's database key")
            .to_hex()
    ))
    .expect("key the fault connection");
    (directory, session, conn)
}

fn rows(conn: &Connection) -> Vec<(&'static str, TableRows)> {
    [
        "content_keys",
        "sealed_blobs",
        "events",
        "profiles",
        "evidence",
        "memories",
        "memory_content_keys",
        "memory_evidence",
        "audit",
        "audit_subjects",
    ]
    .into_iter()
    .map(|table| {
        let mut query = conn
            .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
            .expect("read affected table");
        let values = query
            .query_map([], |row| {
                (0..row.as_ref().column_count())
                    .map(|column| row.get(column))
                    .collect::<rusqlite::Result<Vec<Value>>>()
            })
            .expect("read table rows")
            .collect::<rusqlite::Result<TableRows>>()
            .expect("read row values");
        (table, values)
    })
    .collect()
}

fn fail_audit(conn: &Connection) {
    conn.execute_batch(
        "CREATE TRIGGER reject_session_audit BEFORE INSERT ON audit
         BEGIN SELECT RAISE(ABORT, 'forced audit failure'); END;",
    )
    .expect("install the audit failure");
}

fn assert_rolled_back(conn: &Connection, before: &[(&str, TableRows)], refusal: SessionRefusal) {
    assert!(
        refusal.explanation.contains("forced audit failure"),
        "the command failed somewhere other than its audit write: {refusal:?}",
    );
    for ((table, expected), (actual_table, actual)) in before.iter().zip(rows(conn)) {
        assert_eq!(*table, actual_table);
        assert_eq!(actual, *expected, "a refused edit changed {table}");
    }
    conn.execute_batch("DROP TRIGGER reject_session_audit")
        .expect("allow the retry's audit write");
}

fn assert_audited(session: &Session, expected_entries: usize, action: &str) {
    let audit = session.audit().expect("read the audit chain");
    assert!(audit.verified, "{:?}", audit.verification_problem);
    assert_eq!(audit.entries.len(), expected_entries);
    assert_eq!(
        audit.entries.last().expect("the committed edit").action,
        action
    );
}

fn new_memory() -> NewMemory {
    NewMemory {
        memory_type: "episodic".to_owned(),
        title: "Moving day".to_owned(),
        summary: "We carried the books upstairs.".to_owned(),
    }
}

#[test]
fn an_axis_correction_rolls_back_its_evidence_and_profile_when_audit_fails() {
    let (_directory, mut session, conn) = scratch();
    let profile = session.profile().expect("the original profile");
    let axis_id = &profile.axes[0].axis_id;
    let before = rows(&conn);
    fail_audit(&conn);

    let refusal = session
        .correct_axis(axis_id, "leans_high")
        .expect_err("the audit write refuses the correction");

    assert_rolled_back(&conn, &before, refusal);
    assert_eq!(session.profile().expect("unchanged profile"), profile);
    let corrected = session
        .correct_axis(axis_id, "leans_high")
        .expect("retry correction");
    let axis = corrected
        .axes
        .iter()
        .find(|axis| axis.axis_id == *axis_id)
        .expect("corrected axis");
    assert_eq!(axis.position, "leans_high");
    assert!(axis.locked_by_user);
    assert_eq!(axis.evidence_count, 1);
    assert_eq!(session.profile().expect("committed profile"), corrected);
    assert_audited(&session, 1, "profile.correct");
}

#[test]
fn a_voice_change_preserves_the_existing_profile_when_audit_fails() {
    let (_directory, mut session, conn) = scratch();
    let profile = session
        .set_voice("register", "formal")
        .expect("original voice");
    let before = rows(&conn);
    fail_audit(&conn);

    let refusal = session
        .set_voice("register", "casual")
        .expect_err("the audit write refuses the voice change");

    assert_rolled_back(&conn, &before, refusal);
    assert_eq!(session.profile().expect("unchanged profile"), profile);
    let updated = session
        .set_voice("register", "casual")
        .expect("retry voice change");
    let register = updated
        .voice
        .fields
        .iter()
        .find(|field| field.field == "register")
        .expect("register field");
    assert_eq!(register.value, "casual");
    assert!(register.locked_by_user);
    assert_eq!(session.profile().expect("committed profile"), updated);
    assert_audited(&session, 2, "profile.correct");
}

#[test]
fn a_new_memory_leaves_no_key_blob_or_row_when_audit_fails() {
    let (_directory, mut session, conn) = scratch();
    let before = rows(&conn);
    fail_audit(&conn);

    let refusal = session
        .write_memory(&new_memory())
        .expect_err("the audit write refuses the new memory");

    assert_rolled_back(&conn, &before, refusal);
    assert!(session
        .memories()
        .expect("unchanged memories")
        .memories
        .is_empty());
    let written = session
        .write_memory(&new_memory())
        .expect("retry memory creation");
    assert_eq!(written.title, "Moving day");
    assert_eq!(written.summary, "We carried the books upstairs.");
    assert_eq!(
        session
            .memory(&written.memory_id)
            .expect("committed memory"),
        written
    );
    assert_eq!(
        session
            .memories()
            .expect("one committed memory")
            .memories
            .len(),
        1
    );
    assert_audited(&session, 1, "memory.write");
}

#[test]
fn a_memory_edit_preserves_existing_content_and_blobs_when_audit_fails() {
    let (_directory, mut session, conn) = scratch();
    let original = session
        .write_memory(&new_memory())
        .expect("original memory");
    let change = MemoryChange {
        memory_type: Some("semantic".to_owned()),
        title: Some("Book shelves".to_owned()),
        summary: Some("The books belong upstairs.".to_owned()),
    };
    let before = rows(&conn);
    fail_audit(&conn);

    let refusal = session
        .edit_memory(&original.memory_id, &change)
        .expect_err("the audit write refuses the edit");

    assert_rolled_back(&conn, &before, refusal);
    assert_eq!(
        session
            .memory(&original.memory_id)
            .expect("unchanged memory"),
        original
    );
    let updated = session
        .edit_memory(&original.memory_id, &change)
        .expect("retry edit");
    assert_eq!(updated.memory_id, original.memory_id);
    assert_eq!(updated.content_key_id, original.content_key_id);
    assert_eq!(updated.memory_type, "semantic");
    assert_eq!(updated.title, "Book shelves");
    assert_eq!(updated.summary, "The books belong upstairs.");
    assert_eq!(
        session
            .memory(&updated.memory_id)
            .expect("committed memory"),
        updated
    );
    assert_eq!(
        session
            .memories()
            .expect("one edited memory")
            .memories
            .len(),
        1
    );
    assert_audited(&session, 2, "memory.write");
}
