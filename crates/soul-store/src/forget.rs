//! Forgetting, implemented as destruction of wrapped content keys.
//!
//! Nothing here overwrites disk blocks, and PRODUCT_LOCK does not claim it
//! does. What it claims, and what this module makes true, is narrower and
//! testable: the wrapped content key is deleted, so the ciphertext that was
//! sealed under it can no longer be opened; the rows that depended on it become
//! tombstones the user can still see; inferences that rested on the evidence
//! become `orphaned`; and the audit chain is left exactly as it was, because it
//! holds no prose and must never be able to block a forget.
//!
//! Every number in the preview comes from a query. A preview that guessed would
//! be worse than none: the user is being asked to authorise something
//! irreversible on the strength of it, and `run_conformance` checks that the
//! receipt matches the preview it was shown.

use std::collections::BTreeSet;

use rusqlite::{OptionalExtension, Transaction};
use serde_json::Value;
use uuid::Uuid;

use soul_schema::memory::ForgetState;
use soul_store_api::forget::{
    ForgetCleanup, ForgetImpact, ForgetOps, ForgetOutcome, ForgetOutcomeOps, ForgetReceipt,
    ForgetUnit,
};
use soul_store_api::types::{InferenceState, StoreError, StoreResult};

use crate::store::{as_text, backend, enum_text, placeholders, SqlCipherStore};

/// Everything one forget would touch, resolved against the database.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ForgetPlan {
    pub(crate) content_keys: Vec<Uuid>,
    pub(crate) memories: Vec<Uuid>,
    pub(crate) contacts: Vec<Uuid>,
    pub(crate) evidence: Vec<Uuid>,
    pub(crate) inferences: Vec<Uuid>,
}

impl SqlCipherStore {
    /// Read one column of UUIDs.
    fn uuid_column(&self, sql: &str, bound: &[String]) -> StoreResult<Vec<Uuid>> {
        let mut statement = self.conn.prepare(sql).map_err(backend)?;
        let rows = statement
            .query_map(rusqlite::params_from_iter(bound.iter()), |row| {
                row.get::<_, String>(0)
            })
            .map_err(backend)?;
        let mut out = Vec::new();
        for row in rows {
            let text = row.map_err(backend)?;
            out.push(
                text.parse::<Uuid>()
                    .map_err(|error| StoreError::Backend(format!("{text}: {error}")))?,
            );
        }
        Ok(out)
    }

    fn count(&self, sql: &str, bound: &[String]) -> StoreResult<u64> {
        self.conn
            .query_row(sql, rusqlite::params_from_iter(bound.iter()), |row| {
                row.get::<_, i64>(0)
            })
            .map(|count| count as u64)
            .map_err(backend)
    }

    /// The content keys one forget unit owns.
    fn content_keys_of(&self, unit: ForgetUnit) -> StoreResult<Vec<Uuid>> {
        match unit {
            ForgetUnit::ContentKey(id) => Ok(vec![id]),
            ForgetUnit::Memory(id) => self.uuid_column(
                "SELECT content_key_id FROM memory_content_keys
                 WHERE memory_id = ? ORDER BY content_key_id",
                &[id.to_string()],
            ),
            // A contact owns more than the key behind their display label. An
            // import seals their message bodies under a content key of their
            // own and anchors it against their row; when the export gave no
            // display name — no `soul-import-v1` file does — that anchor is
            // the only record the key was ever theirs. Reading the label
            // column alone is how forgetting such a person became a no-op that
            // still issued a receipt.
            ForgetUnit::Contact(id) => self.uuid_column(
                "SELECT DISTINCT content_key_id FROM (
                     SELECT display_label_key_id AS content_key_id FROM contacts
                      WHERE contact_id = ? AND display_label_key_id IS NOT NULL
                     UNION ALL
                     SELECT content_key_id FROM sealed_blobs WHERE row_id = ?
                 ) ORDER BY content_key_id",
                &[id.to_string(), id.to_string()],
            ),
        }
    }

    pub(crate) fn forget_plan(&self, unit: ForgetUnit) -> StoreResult<ForgetPlan> {
        let content_keys = self.content_keys_of(unit)?;
        if content_keys.is_empty() {
            return Ok(ForgetPlan::default());
        }
        let key_text = as_text(&content_keys);
        let key_slots = placeholders(key_text.len());

        let memories = self.uuid_column(
            &format!(
                "SELECT DISTINCT memory_id FROM memory_content_keys
                 WHERE content_key_id IN ({key_slots}) ORDER BY memory_id"
            ),
            &key_text,
        )?;
        // The same two paths, read the other way round: a label-less contact
        // is reachable from their key only through the blob anchored to their
        // row, and without that the row would never become a tombstone.
        let mut keys_twice = key_text.clone();
        keys_twice.extend(key_text.clone());
        let contacts = self.uuid_column(
            &format!(
                "SELECT contact_id FROM contacts
                 WHERE display_label_key_id IN ({key_slots})
                    OR contact_id IN (SELECT row_id FROM sealed_blobs
                                      WHERE content_key_id IN ({key_slots}))
                 ORDER BY contact_id"
            ),
            &keys_twice,
        )?;

        // Evidence a memory cites directly, plus evidence carried by the graph
        // edges a forgotten contact takes part in.
        let mut evidence: BTreeSet<Uuid> = BTreeSet::new();
        if !memories.is_empty() {
            let memory_text = as_text(&memories);
            let slots = placeholders(memory_text.len());
            evidence.extend(self.uuid_column(
                &format!(
                    "SELECT DISTINCT evidence_id FROM memory_evidence
                     WHERE memory_id IN ({slots})"
                ),
                &memory_text,
            )?);
        }
        if !contacts.is_empty() {
            let contact_text = as_text(&contacts);
            let slots = placeholders(contact_text.len());
            let mut bound = contact_text.clone();
            bound.extend(contact_text);
            evidence.extend(self.uuid_column(
                &format!(
                    "SELECT DISTINCT link.evidence_id
                     FROM relationship_evidence link
                     JOIN relationships edge ON edge.relationship_id = link.relationship_id
                     WHERE edge.from_contact_id IN ({slots})
                        OR edge.to_contact_id   IN ({slots})"
                ),
                &bound,
            )?);
        }
        let evidence: Vec<Uuid> = evidence.into_iter().collect();

        let mut inferences = Vec::new();
        if !evidence.is_empty() {
            let evidence_text = as_text(&evidence);
            let slots = placeholders(evidence_text.len());
            let mut bound = evidence_text;
            bound.push("live".into());
            inferences = self.uuid_column(
                &format!(
                    "SELECT DISTINCT link.inference_id
                     FROM inference_evidence link
                     JOIN inferences node ON node.inference_id = link.inference_id
                     WHERE link.evidence_id IN ({slots}) AND node.state = ?
                     ORDER BY link.inference_id"
                ),
                &bound,
            )?;
        }

        Ok(ForgetPlan {
            content_keys,
            memories,
            contacts,
            evidence,
            inferences,
        })
    }

    pub(crate) fn impact_of(&self, unit: ForgetUnit) -> StoreResult<ForgetImpact> {
        self.impact_from_plan(unit, &self.forget_plan(unit)?)
    }

    fn impact_from_plan(&self, unit: ForgetUnit, plan: &ForgetPlan) -> StoreResult<ForgetImpact> {
        if plan.content_keys.is_empty() {
            return Ok(ForgetImpact::default());
        }

        let key_text = as_text(&plan.content_keys);
        let sealed_blobs_destroyed = self.count(
            &format!(
                "SELECT count(*) FROM sealed_blobs WHERE content_key_id IN ({})",
                placeholders(key_text.len())
            ),
            &key_text,
        )?;

        // The unit itself is named as well, so an audit entry that refers to
        // the memory by id counts as retained even before the row is a
        // tombstone.
        let touched: BTreeSet<Uuid> = plan
            .memories
            .iter()
            .chain(plan.contacts.iter())
            .copied()
            .chain(std::iter::once(unit.id()))
            .collect();
        let touched_text = as_text(&touched.into_iter().collect::<Vec<_>>());
        let audit_entries_retained = self.count(
            &format!(
                "SELECT count(DISTINCT seq) FROM audit_subjects WHERE subject_id IN ({})",
                placeholders(touched_text.len())
            ),
            &touched_text,
        )?;

        Ok(ForgetImpact {
            content_key_ids: plan.content_keys.clone(),
            memories_affected: plan.memories.len() as u64,
            contacts_affected: plan.contacts.len() as u64,
            sealed_blobs_destroyed,
            inferences_orphaned: plan.inferences.len() as u64,
            audit_entries_retained,
        })
    }
}

/// Rewrite a row's `forget_state`, in the indexed column and in the stored
/// document, so a reader cannot see one value through one path and another
/// through the other.
fn mark_forgotten(tx: &Transaction<'_>, table: &str, id_column: &str, id: Uuid) -> StoreResult<()> {
    let doc: Option<String> = tx
        .query_row(
            &format!("SELECT doc FROM {table} WHERE {id_column} = ?1"),
            [id.to_string()],
            |row| row.get(0),
        )
        .optional()
        .map_err(backend)?;
    let Some(doc) = doc else {
        return Ok(());
    };

    let mut value: Value = serde_json::from_str(&doc).map_err(backend)?;
    let forgotten_text = enum_text(&ForgetState::Forgotten)?;
    if let Some(object) = value.as_object_mut() {
        object.insert("forget_state".into(), Value::String(forgotten_text.clone()));
    }
    tx.execute(
        &format!("UPDATE {table} SET forget_state = ?2, doc = ?3 WHERE {id_column} = ?1"),
        rusqlite::params![
            id.to_string(),
            forgotten_text,
            serde_json::to_string(&value).map_err(backend)?
        ],
    )
    .map_err(backend)?;
    Ok(())
}

impl ForgetOps for SqlCipherStore {
    fn preview_impact(&self, unit: ForgetUnit) -> StoreResult<ForgetImpact> {
        self.impact_of(unit)
    }

    fn execute_forget(&mut self, unit: ForgetUnit) -> StoreResult<ForgetReceipt> {
        let outcome = self.execute_forget_outcome(unit)?;
        match outcome.cleanup {
            ForgetCleanup::Complete => Ok(outcome.receipt),
            ForgetCleanup::Pending { .. } => Err(StoreError::Backend(
                "content-key destruction committed, but WAL cleanup is unconfirmed; \
                 old WAL frames may still hold wrapped key bytes. Do not execute forget \
                 again: use retry_forget_cleanup to retry only cleanup"
                    .into(),
            )),
        }
    }
}

impl ForgetOutcomeOps for SqlCipherStore {
    fn execute_forget_outcome(&mut self, unit: ForgetUnit) -> StoreResult<ForgetOutcome> {
        // One resolution, used both for the receipt and for the deletions, so
        // the user cannot be charged for something the receipt did not name.
        let plan = self.forget_plan(unit)?;
        let impact = self.impact_from_plan(unit, &plan)?;
        let orphaned = enum_text(&InferenceState::Orphaned)?;

        let tx = self.conn.transaction().map_err(backend)?;

        for (index, content_key_id) in plan.content_keys.iter().enumerate() {
            tx.execute(
                "DELETE FROM content_keys WHERE content_key_id = ?1",
                [content_key_id.to_string()],
            )
            .map_err(backend)?;

            // Deleting the row frees the id. `ensure_content_key` mints a key
            // for any id it does not already hold, so without this the next
            // seal naming a forgotten id would quietly give the tombstones
            // that name it a live key again. Same transaction as the delete,
            // so a crash cannot leave one without the other.
            tx.execute(
                "INSERT OR IGNORE INTO destroyed_content_keys (content_key_id) VALUES (?1)",
                [content_key_id.to_string()],
            )
            .map_err(backend)?;

            // AC-15 injects between one destruction and the next. The whole
            // forget runs in one transaction, so a crash here leaves every key
            // intact rather than half the unit readable and half not.
            if index == 0 {
                fail::fail_point!(crate::failpoints::FORGET_CK_DELETE_MID);
            }
        }

        for content_key_id in &plan.content_keys {
            tx.execute(
                "DELETE FROM sealed_blobs WHERE content_key_id = ?1",
                [content_key_id.to_string()],
            )
            .map_err(backend)?;
        }

        for memory_id in &plan.memories {
            mark_forgotten(&tx, "memories", "memory_id", *memory_id)?;
        }
        for contact_id in &plan.contacts {
            mark_forgotten(&tx, "contacts", "contact_id", *contact_id)?;
        }

        for inference_id in &plan.inferences {
            tx.execute(
                "UPDATE inferences SET state = ?2 WHERE inference_id = ?1",
                rusqlite::params![inference_id.to_string(), orphaned],
            )
            .map_err(backend)?;
        }

        // The audit tables are deliberately untouched.
        tx.commit().map_err(backend)?;

        // Zeroing the freed page, which `PRAGMA secure_delete` does, only
        // settles the main database file. The write-ahead log still holds the
        // frames written before the delete, and those carry the page as it was
        // when the wrapped key was on it. Truncating the log is what discards
        // them; until then the key sits next to a database the DEK opens.
        // A query can succeed while checkpointing reports SQLITE_BUSY in
        // column zero. Preserve the committed receipt even on cleanup failure.
        let cleanup = self.retry_forget_cleanup();
        Ok(ForgetOutcome {
            receipt: ForgetReceipt { unit, impact },
            cleanup,
        })
    }

    fn retry_forget_cleanup(&mut self) -> ForgetCleanup {
        // Only this pragma: no key deletion, no new impact resolution, and no
        // audit append. No raw database errors or paths leave this API.
        match self.checkpoint_status() {
            Ok(checkpoint) => ForgetCleanup::from_checkpoint(checkpoint),
            Err(_) => ForgetCleanup::Pending { checkpoint: None },
        }
    }
}

#[cfg(test)]
mod outcome_tests {
    use std::time::Duration;

    use soul_schema::common::{SealedSubject, SealedText};
    use soul_store_api::forget::WalCheckpoint;
    use soul_store_api::types::SealRequest;
    use soul_store_api::{BlobStore, SoulStore};

    use super::*;
    use crate::TestKeyProvider;

    fn seeded() -> (tempfile::TempDir, SqlCipherStore, TestKeyProvider, Uuid, SealedText) {
        let directory = tempfile::tempdir().expect("temporary synthetic store");
        let keys = TestKeyProvider::from_seed("forget outcome regression");
        let mut store = SqlCipherStore::open(directory.path().join("soul.db"), &keys)
            .expect("open SQLCipher");
        store.conn.busy_timeout(Duration::ZERO).expect("no busy wait");
        store.conn.pragma_update(None, "wal_autocheckpoint", 0).expect("disable auto checkpoint");
        let key = Uuid::now_v7();
        let sealed = store.seal(SealRequest::new(
            key,
            Uuid::now_v7(),
            "body_ref",
            SealedSubject::Owner,
            b"synthetic prose only".to_vec(),
        )).expect("seal fixture");
        (directory, store, keys, key, sealed)
    }

    fn pin_reader(store: &SqlCipherStore, keys: &TestKeyProvider) -> SqlCipherStore {
        let reader = SqlCipherStore::open(store.path(), keys).expect("test-only second connection");
        reader.conn.execute_batch("BEGIN").expect("begin snapshot");
        let count: i64 = reader.conn.query_row(
            "SELECT count(*) FROM content_keys", [], |row| row.get(0),
        ).expect("pin pre-forget snapshot");
        assert_eq!(count, 1);
        reader
    }

    fn changes(store: &SqlCipherStore) -> i64 {
        store.conn.query_row("SELECT total_changes()", [], |row| row.get(0))
            .expect("read change count")
    }

    #[test]
    fn busy_cleanup_keeps_the_committed_receipt() {
        let (_directory, mut store, keys, key, sealed) = seeded();
        let reader = pin_reader(&store, &keys);
        let expected = store.preview_impact(ForgetUnit::ContentKey(key)).expect("preview");
        let outcome = store.execute_forget_outcome(ForgetUnit::ContentKey(key))
            .expect("destruction committed despite busy cleanup");
        assert_eq!(outcome.receipt.impact, expected);
        assert!(matches!(outcome.cleanup, ForgetCleanup::Pending {
            checkpoint: Some(WalCheckpoint { busy: 1, .. }),
        }));
        assert!(store.open(&sealed).is_err(), "live store must no longer open the prose");
        assert!(reader.open(&sealed).is_ok(), "pinned old snapshot still exposes the risk");
        reader.conn.execute_batch("ROLLBACK").expect("release reader");
        assert_eq!(store.retry_forget_cleanup(), ForgetCleanup::Complete);
    }

    #[test]
    fn copied_frames_with_a_reader_are_not_a_completed_truncate() {
        let (_directory, mut store, keys, _key, _sealed) = seeded();
        let reader = pin_reader(&store, &keys);
        let ForgetCleanup::Pending { checkpoint: Some(observed) } = store.retry_forget_cleanup()
        else { panic!("reader must prevent truncation") };
        assert_eq!(observed.busy, 1);
        assert!(observed.log_frames > 0);
        assert_eq!(observed.log_frames, observed.checkpointed_frames);
        reader.conn.execute_batch("ROLLBACK").expect("release reader");
    }

    #[test]
    fn cleanup_retries_do_not_repeat_destruction_or_write_audit() {
        let (directory, mut store, keys, key, sealed) = seeded();
        let reader = pin_reader(&store, &keys);
        let outcome = store.execute_forget_outcome(ForgetUnit::ContentKey(key))
            .expect("committed receipt");
        assert!(matches!(outcome.cleanup, ForgetCleanup::Pending { .. }));
        reader.conn.execute_batch("ROLLBACK").expect("release reader");
        store.conn.execute_batch(
            "CREATE TEMP TRIGGER forbid_new_destruction BEFORE DELETE ON content_keys
             BEGIN SELECT RAISE(ABORT, 'retry must not destroy'); END;"
        ).expect("install destructive-retry tripwire");
        let before = changes(&store);
        assert_eq!(store.retry_forget_cleanup(), ForgetCleanup::Complete);
        assert_eq!(store.retry_forget_cleanup(), ForgetCleanup::Complete);
        assert_eq!(changes(&store), before, "cleanup must do no DML, including audit");
        reader.close().expect("close reader");
        store.close().expect("close writer");
        let mut reopened = SqlCipherStore::open(directory.path().join("soul.db"), &keys)
            .expect("reopen");
        let before = changes(&reopened);
        assert_eq!(reopened.retry_forget_cleanup(), ForgetCleanup::Complete);
        assert_eq!(changes(&reopened), before);
        assert!(reopened.open(&sealed).is_err());
        assert_eq!(outcome.receipt.impact.content_key_ids, vec![key]);
    }

    #[test]
    fn failure_before_commit_rolls_back_the_deleted_key() {
        let (_directory, mut store, _keys, key, sealed) = seeded();
        store.conn.execute_batch(
            "CREATE TEMP TRIGGER fail_after_key_delete BEFORE INSERT ON destroyed_content_keys
             BEGIN SELECT RAISE(ABORT, 'synthetic precommit failure'); END;"
        ).expect("install failure after delete and before commit");
        assert!(store.execute_forget_outcome(ForgetUnit::ContentKey(key)).is_err());
        assert_eq!(store.open(&sealed).expect("key deletion rolled back"), b"synthetic prose only");
        let destroyed: i64 = store.conn.query_row(
            "SELECT count(*) FROM destroyed_content_keys", [], |row| row.get(0),
        ).expect("read tombstone count");
        assert_eq!(destroyed, 0);
    }

    #[test]
    fn query_failure_is_pending_not_complete() {
        let (_directory, mut store, _keys, _key, _sealed) = seeded();
        store.conn.execute_batch("BEGIN IMMEDIATE").expect("open write transaction");
        assert_eq!(store.retry_forget_cleanup(), ForgetCleanup::Pending { checkpoint: None });
        store.conn.execute_batch("ROLLBACK").expect("rollback");
    }

    #[test]
    fn legacy_forget_never_reports_success_for_busy_cleanup() {
        let (_directory, mut store, keys, key, sealed) = seeded();
        let reader = pin_reader(&store, &keys);
        let error = store.execute_forget(ForgetUnit::ContentKey(key))
            .expect_err("busy cleanup is not legacy success");
        assert!(error.to_string().contains("committed"));
        assert!(error.to_string().contains("retry_forget_cleanup"));
        assert!(store.open(&sealed).is_err());
        reader.conn.execute_batch("ROLLBACK").expect("release reader");
    }

    #[test]
    fn checkpoint_and_flush_also_reject_busy_completion() {
        let (_directory, mut store, keys, _key, _sealed) = seeded();
        let reader = pin_reader(&store, &keys);
        assert!(store.checkpoint().is_err());
        assert!(store.flush().is_err());
        reader.conn.execute_batch("ROLLBACK").expect("release reader");
        store.checkpoint().expect("confirmed truncate");
        store.flush().expect("confirmed flush");
    }
}
