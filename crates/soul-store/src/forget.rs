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
    pub(crate) import_namespaces: Vec<(Uuid, String)>,
    pub(crate) import_participants: Vec<Uuid>,
    pub(crate) import_messages: Vec<Uuid>,
    pub(crate) local_insights: Vec<Uuid>,
    // Raw auxiliary identifiers remain comparable even if a damaged row no
    // longer decodes as an assistance item. Such damage must not block CK loss.
    pub(crate) assistance_items: Vec<(String, String, bool)>,
}

#[cfg(test)]
mod auxiliary_forget_tests {
    use super::*;
    use crate::TestKeyProvider;
    use soul_schema::common::SealedSubject;
    use soul_store_api::import::{
        ImportCoverage, ImportIndexStore, ImportNamespace, ImportNamespaceState,
        ImportParticipantIdentity, ImportSourceKind,
    };
    use soul_store_api::insight::{InsightValidity, LocalInsightBundle};
    use soul_store_api::types::SealRequest;
    use soul_store_api::BlobStore;

    fn namespace() -> ImportNamespace {
        ImportNamespace {
            namespace_id: Uuid::now_v7(),
            source: ImportSourceKind::SoulImportV1,
            state: ImportNamespaceState::Active,
            coverage: ImportCoverage::FullFromCreation,
            owner_contact_id: None,
            account_ref: None,
        }
    }

    fn bundle(namespace: Uuid) -> LocalInsightBundle {
        LocalInsightBundle {
            bundle_id: Uuid::now_v7(),
            operation_id: Uuid::now_v7(),
            namespace_id: namespace,
            created_at: 1,
            validity: InsightValidity::Active,
            ignored: true,
            counts: Default::default(),
            dependency_uuids: Vec::new(),
            dependency_values: Vec::new(),
            dependency_flags: Vec::new(),
        }
    }

    fn insert_bundle(store: &SqlCipherStore, bundle: &LocalInsightBundle, doc: &str) {
        store.conn.execute("INSERT INTO local_insights(bundle_id,operation_id,namespace_id,created_at,validity,ignored,doc) VALUES(?1,?2,?3,?4,?5,?6,?7)",
            rusqlite::params![bundle.bundle_id.to_string(),bundle.operation_id.to_string(),bundle.namespace_id.to_string(),bundle.created_at,enum_text(&bundle.validity).unwrap(),bundle.ignored,doc]).unwrap();
    }

    fn anchor(
        store: &mut SqlCipherStore,
        namespace: &ImportNamespace,
        key: Uuid,
    ) -> soul_schema::common::SealedText {
        let contact = Uuid::now_v7();
        let sealed = store
            .seal(SealRequest::new(
                key,
                contact,
                "import_participant_identity",
                SealedSubject::ThirdParty,
                b"[\"uid\",\"synthetic\"]".to_vec(),
            ))
            .unwrap();
        store.put_import_namespace(namespace).unwrap();
        store
            .put_import_participant(&ImportParticipantIdentity {
                identity_id: Uuid::now_v7(),
                namespace_id: namespace.namespace_id,
                contact_id: contact,
                content_key_id: key,
                identity_ref: sealed.clone(),
            })
            .unwrap();
        sealed
    }

    #[test]
    fn damaged_auxiliary_json_cannot_block_key_destruction_or_reactivate_a_source_after_reopen() {
        for (bad_namespace, bad_insight) in [(true, false), (false, true), (true, true)] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("auxiliary.db");
            let keys = TestKeyProvider::from_seed("auxiliary forget corruption");
            let mut store = SqlCipherStore::open(&path, &keys).unwrap();
            let ns = namespace();
            let key = Uuid::now_v7();
            let sealed = anchor(&mut store, &ns, key);
            let insight = bundle(ns.namespace_id);
            let doc = if bad_insight {
                "{corrupt".to_owned()
            } else {
                crate::insights::encode(&insight).unwrap()
            };
            insert_bundle(&store, &insight, &doc);
            let mut old = bundle(ns.namespace_id);
            old.validity = InsightValidity::Invalidated;
            old.ignored = false;
            insert_bundle(&store, &old, &crate::insights::encode(&old).unwrap());
            if bad_namespace {
                store
                    .conn
                    .execute(
                        "UPDATE import_namespaces SET doc='{corrupt' WHERE namespace_id=?1",
                        [ns.namespace_id.to_string()],
                    )
                    .unwrap();
            }
            let outcome = store
                .execute_forget_outcome(ForgetUnit::ContentKey(key))
                .unwrap();
            assert_eq!(outcome.receipt.impact.import_namespaces_revoked, 1);
            assert_eq!(outcome.receipt.impact.local_insights_invalidated, 1);
            assert!(!store.has_content_key(key));
            assert!(store.open(&sealed).is_err());
            let state: String = store
                .conn
                .query_row(
                    "SELECT state FROM import_namespaces WHERE namespace_id=?1",
                    [ns.namespace_id.to_string()],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(state, "revoked");
            if bad_insight {
                assert!(store.local_insight(insight.bundle_id).is_err());
            } else {
                let retained = store.local_insight(insight.bundle_id).unwrap();
                assert_eq!(retained.validity, InsightValidity::Invalidated);
                assert!(retained.ignored);
            }
            assert_eq!(store.local_insight(old.bundle_id).unwrap(), old);
            drop(store);
            let mut reopened = SqlCipherStore::open(&path, &keys).unwrap();
            assert!(!reopened.has_content_key(key));
            assert!(reopened.open(&sealed).is_err());
            if bad_namespace {
                assert!(reopened.import_namespace(ns.namespace_id).is_err());
            } else {
                assert_eq!(
                    reopened.import_namespace(ns.namespace_id).unwrap().state,
                    ImportNamespaceState::Revoked
                );
            }
            assert!(reopened.put_import_namespace(&ns).is_err());
        }
    }

    #[test]
    fn a_same_count_insight_replacement_invalidates_the_exact_forget_quote() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SqlCipherStore::open(
            dir.path().join("scope.db"),
            &TestKeyProvider::from_seed("insight scope"),
        )
        .unwrap();
        let ns = namespace();
        let key = Uuid::now_v7();
        anchor(&mut store, &ns, key);
        let first = bundle(ns.namespace_id);
        insert_bundle(&store, &first, &crate::insights::encode(&first).unwrap());
        let quote = store
            .preview_forget_scope(ForgetUnit::ContentKey(key))
            .unwrap();
        assert_eq!(quote.impact().local_insights_invalidated, 1);
        store
            .conn
            .execute(
                "DELETE FROM local_insights WHERE bundle_id=?1",
                [first.bundle_id.to_string()],
            )
            .unwrap();
        let replacement = bundle(ns.namespace_id);
        insert_bundle(
            &store,
            &replacement,
            &crate::insights::encode(&replacement).unwrap(),
        );
        assert_eq!(
            store.preview_impact(ForgetUnit::ContentKey(key)).unwrap(),
            *quote.impact()
        );
        assert!(!store.forget_scope_is_current(&quote).unwrap());
        assert!(store.has_content_key(key));
    }

    #[test]
    fn genuine_auxiliary_sql_failures_still_roll_back_the_key_deletion() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SqlCipherStore::open(
            dir.path().join("sql-failure.db"),
            &TestKeyProvider::from_seed("auxiliary sql failure"),
        )
        .unwrap();
        let ns = namespace();
        let key = Uuid::now_v7();
        let sealed = anchor(&mut store, &ns, key);
        store.conn.execute_batch("CREATE TEMP TRIGGER fail_namespace_revoke BEFORE UPDATE ON import_namespaces BEGIN SELECT RAISE(ABORT,'synthetic SQL failure'); END;").unwrap();
        assert!(store
            .execute_forget_outcome(ForgetUnit::ContentKey(key))
            .is_err());
        assert!(store.has_content_key(key));
        assert!(store.open(&sealed).is_ok());
        assert_eq!(
            store.import_namespace(ns.namespace_id).unwrap().state,
            ImportNamespaceState::Active
        );
    }
}

/// An in-memory quote of the exact rows affected by a forget.
///
/// Counts alone cannot detect a same-size replacement of evidence or a
/// dependent memory. Keep existing primary keys, not prose or a second hash
/// protocol. This is deliberately not serializable or constructible by IPC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgetScope {
    unit: ForgetUnit,
    plan: ForgetPlan,
    sealed_blobs: Vec<Uuid>,
    impact: ForgetImpact,
}

impl ForgetScope {
    pub fn impact(&self) -> &ForgetImpact {
        &self.impact
    }
}

impl SqlCipherStore {
    /// Resolve the quote while holding the session's sole store lock.
    pub fn preview_forget_scope(&self, unit: ForgetUnit) -> StoreResult<ForgetScope> {
        let plan = self.forget_plan(unit)?;
        let impact = self.impact_from_plan(unit, &plan)?;
        let sealed_blobs = if plan.content_keys.is_empty() {
            Vec::new()
        } else {
            let keys = as_text(&plan.content_keys);
            self.uuid_column(
                &format!(
                    "SELECT blob_id FROM sealed_blobs WHERE content_key_id IN ({}) ORDER BY blob_id",
                    placeholders(keys.len())
                ),
                &keys,
            )?
        };
        Ok(ForgetScope {
            unit,
            plan,
            sealed_blobs,
            impact,
        })
    }

    /// Revalidate immediately before destruction, without releasing the same
    /// store guard in between. This relies on Soul's single-writer ownership;
    /// it is not a transaction/authorization token for another connection.
    pub fn forget_scope_is_current(&self, quoted: &ForgetScope) -> StoreResult<bool> {
        Ok(&self.preview_forget_scope(quoted.unit)? == quoted)
    }

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

        let import_participants=self.uuid_column(
            &format!("SELECT identity_id FROM import_participants WHERE content_key_id IN ({key_slots}) ORDER BY identity_id"),&key_text)?;
        let import_messages=self.uuid_column(
            &format!("SELECT identity_id FROM import_message_keys WHERE content_key_id IN ({key_slots}) ORDER BY identity_id"),&key_text)?;
        let mut bound = key_text.clone();
        bound.extend(key_text.clone());
        let mut statement=self.conn.prepare(&format!(
            "SELECT namespace_id,state FROM import_namespaces WHERE state != 'revoked' AND (
                namespace_id IN (SELECT namespace_id FROM import_participants WHERE content_key_id IN ({key_slots})) OR
                namespace_id IN (SELECT namespace_id FROM import_message_keys WHERE content_key_id IN ({key_slots}))) ORDER BY namespace_id"
        )).map_err(backend)?;
        let rows = statement
            .query_map(rusqlite::params_from_iter(bound.iter()), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(backend)?;
        let mut import_namespaces = Vec::new();
        for row in rows {
            let (id, state) = row.map_err(backend)?;
            import_namespaces.push((id.parse().map_err(backend)?, state));
        }
        drop(statement);
        let local_insights = if import_namespaces.is_empty() {
            Vec::new()
        } else {
            let namespaces = as_text(
                &import_namespaces
                    .iter()
                    .map(|(id, _)| *id)
                    .collect::<Vec<_>>(),
            );
            self.uuid_column(&format!("SELECT bundle_id FROM local_insights WHERE validity='active' AND namespace_id IN ({}) ORDER BY bundle_id",placeholders(namespaces.len())),&namespaces)?
        };
        let mut assistance_items = Vec::new();
        let mut rows = self.conn.prepare("SELECT item_id,revision,memory_id,namespace_id,contact_id,bundle_id FROM assistance_items ORDER BY item_id").map_err(backend)?;
        let candidates = rows
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, Option<String>>(5)?,
                ))
            })
            .map_err(backend)?;
        let includes = |v: &Option<String>, ids: &[Uuid]| {
            v.as_ref()
                .is_some_and(|v| ids.iter().any(|id| id.to_string() == *v))
        };
        for row in candidates {
            let (id, revision, memory, namespace, contact, bundle) = row.map_err(backend)?;
            let own_memory = includes(&memory, &memories);
            if own_memory
                || includes(&contact, &contacts)
                || includes(&bundle, &local_insights)
                || namespace
                    .as_ref()
                    .is_some_and(|v| import_namespaces.iter().any(|(id, _)| id.to_string() == *v))
            {
                assistance_items.push((id, revision, own_memory));
            }
        }
        drop(rows);
        Ok(ForgetPlan {
            content_keys,
            memories,
            contacts,
            evidence,
            inferences,
            import_namespaces,
            import_participants,
            import_messages,
            local_insights,
            assistance_items,
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
            import_namespaces_revoked: plan.import_namespaces.len() as u64,
            local_insights_invalidated: plan.local_insights.len() as u64,
            assistance_items_invalidated: plan.assistance_items.len() as u64,
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
        for (item_id, _, own_memory) in &plan.assistance_items {
            tx.execute(
                "UPDATE assistance_items SET validity=?2,revision=?3 WHERE item_id=?1",
                rusqlite::params![
                    item_id,
                    if *own_memory { "forgotten" } else { "stale" },
                    Uuid::now_v7().to_string()
                ],
            )
            .map_err(backend)?;
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

        // Revocation survives removal of the sealed matching ids. Otherwise
        // their absence would turn a re-import into fresh, readable content.
        for (namespace_id, _) in &plan.import_namespaces {
            let doc: String = tx
                .query_row(
                    "SELECT doc FROM import_namespaces WHERE namespace_id=?1",
                    [namespace_id.to_string()],
                    |row| row.get(0),
                )
                .map_err(backend)?;
            let repaired = serde_json::from_str::<soul_store_api::import::ImportNamespace>(&doc)
                .ok()
                .filter(|namespace| namespace.namespace_id == *namespace_id)
                .and_then(|mut namespace| {
                    namespace.state = soul_store_api::import::ImportNamespaceState::Revoked;
                    serde_json::to_string(&namespace).ok()
                });
            // SQL state is authoritative even when auxiliary JSON is corrupt.
            // Retaining its unreadable doc makes readers fail closed instead
            // of silently restoring an active source or blocking key erasure.
            tx.execute("UPDATE import_namespaces SET state='revoked',doc=COALESCE(?2,doc) WHERE namespace_id=?1",
                rusqlite::params![namespace_id.to_string(),repaired]).map_err(backend)?;
        }
        for bundle_id in &plan.local_insights {
            let doc: String = tx
                .query_row(
                    "SELECT doc FROM local_insights WHERE bundle_id=?1",
                    [bundle_id.to_string()],
                    |row| row.get(0),
                )
                .map_err(backend)?;
            match crate::insights::decode(&doc) {
                Ok(mut bundle) if bundle.bundle_id == *bundle_id => {
                    bundle.validity = soul_store_api::insight::InsightValidity::Invalidated;
                    let encoded = crate::insights::encode(&bundle)?;
                    tx.execute("UPDATE local_insights SET validity='invalidated',doc=?2 WHERE bundle_id=?1",
                        rusqlite::params![bundle_id.to_string(),encoded]).map_err(backend)?;
                }
                _ => {
                    // Insight bundles contain no prose. A malformed cache
                    // record may be discarded; genuine SQL errors still abort
                    // the transaction and are never mistaken for JSON damage.
                    tx.execute(
                        "DELETE FROM local_insights WHERE bundle_id=?1",
                        [bundle_id.to_string()],
                    )
                    .map_err(backend)?;
                }
            }
        }
        for key in &plan.content_keys {
            tx.execute(
                "DELETE FROM import_participants WHERE content_key_id=?1",
                [key.to_string()],
            )
            .map_err(backend)?;
            tx.execute(
                "DELETE FROM import_message_keys WHERE content_key_id=?1",
                [key.to_string()],
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

    fn seeded() -> (
        tempfile::TempDir,
        SqlCipherStore,
        TestKeyProvider,
        Uuid,
        SealedText,
    ) {
        let directory = tempfile::tempdir().expect("temporary synthetic store");
        let keys = TestKeyProvider::from_seed("forget outcome regression");
        let mut store =
            SqlCipherStore::open(directory.path().join("soul.db"), &keys).expect("open SQLCipher");
        store
            .conn
            .busy_timeout(Duration::ZERO)
            .expect("no busy wait");
        store
            .conn
            .pragma_update(None, "wal_autocheckpoint", 0)
            .expect("disable auto checkpoint");
        let key = Uuid::now_v7();
        let sealed = store
            .seal(SealRequest::new(
                key,
                Uuid::now_v7(),
                "body_ref",
                SealedSubject::Owner,
                b"synthetic prose only".to_vec(),
            ))
            .expect("seal fixture");
        (directory, store, keys, key, sealed)
    }

    fn pin_reader(store: &SqlCipherStore, keys: &TestKeyProvider) -> SqlCipherStore {
        let reader = SqlCipherStore::open(store.path(), keys).expect("test-only second connection");
        reader.conn.execute_batch("BEGIN").expect("begin snapshot");
        let count: i64 = reader
            .conn
            .query_row("SELECT count(*) FROM content_keys", [], |row| row.get(0))
            .expect("pin pre-forget snapshot");
        assert_eq!(count, 1);
        reader
    }

    fn changes(store: &SqlCipherStore) -> i64 {
        store
            .conn
            .query_row("SELECT total_changes()", [], |row| row.get(0))
            .expect("read change count")
    }

    #[test]
    fn replacing_a_blob_identity_invalidates_an_equal_count_quote() {
        let (_directory, store, _keys, key, mut sealed) = seeded();
        let unit = ForgetUnit::ContentKey(key);
        let quote = store.preview_forget_scope(unit).expect("quote");
        let replacement = Uuid::now_v7();
        // An adversarial same-size replacement, without changing the key or
        // count. A real caller cannot supply an arbitrary ForgetScope.
        store
            .conn
            .execute(
                "UPDATE sealed_blobs SET blob_id = ?1 WHERE blob_id = ?2",
                [replacement.to_string(), sealed.blob_id.to_string()],
            )
            .expect("replace identity");
        sealed.blob_id = replacement;
        assert_eq!(
            store.preview_impact(unit).expect("same impact"),
            *quote.impact()
        );
        assert!(!store.forget_scope_is_current(&quote).expect("new identity"));
        assert_eq!(
            store.open(&sealed).expect("no destruction"),
            b"synthetic prose only"
        );
    }

    #[test]
    fn busy_cleanup_keeps_the_committed_receipt() {
        let (_directory, mut store, keys, key, sealed) = seeded();
        let reader = pin_reader(&store, &keys);
        let expected = store
            .preview_impact(ForgetUnit::ContentKey(key))
            .expect("preview");
        let outcome = store
            .execute_forget_outcome(ForgetUnit::ContentKey(key))
            .expect("destruction committed despite busy cleanup");
        assert_eq!(outcome.receipt.impact, expected);
        assert!(matches!(
            outcome.cleanup,
            ForgetCleanup::Pending {
                checkpoint: Some(WalCheckpoint { busy: 1, .. }),
            }
        ));
        assert!(
            store.open(&sealed).is_err(),
            "live store must no longer open the prose"
        );
        assert!(
            reader.open(&sealed).is_ok(),
            "pinned old snapshot still exposes the risk"
        );
        reader
            .conn
            .execute_batch("ROLLBACK")
            .expect("release reader");
        assert_eq!(store.retry_forget_cleanup(), ForgetCleanup::Complete);
    }

    #[test]
    fn copied_frames_with_a_reader_are_not_a_completed_truncate() {
        let (_directory, mut store, keys, _key, _sealed) = seeded();
        let reader = pin_reader(&store, &keys);
        let ForgetCleanup::Pending {
            checkpoint: Some(observed),
        } = store.retry_forget_cleanup()
        else {
            panic!("reader must prevent truncation")
        };
        assert_eq!(observed.busy, 1);
        assert!(observed.log_frames > 0);
        assert_eq!(observed.log_frames, observed.checkpointed_frames);
        reader
            .conn
            .execute_batch("ROLLBACK")
            .expect("release reader");
    }

    #[test]
    fn cleanup_retries_do_not_repeat_destruction_or_write_audit() {
        let (directory, mut store, keys, key, sealed) = seeded();
        let reader = pin_reader(&store, &keys);
        let outcome = store
            .execute_forget_outcome(ForgetUnit::ContentKey(key))
            .expect("committed receipt");
        assert!(matches!(outcome.cleanup, ForgetCleanup::Pending { .. }));
        reader
            .conn
            .execute_batch("ROLLBACK")
            .expect("release reader");
        store
            .conn
            .execute_batch(
                "CREATE TEMP TRIGGER forbid_new_destruction BEFORE DELETE ON content_keys
             BEGIN SELECT RAISE(ABORT, 'retry must not destroy'); END;",
            )
            .expect("install destructive-retry tripwire");
        let before = changes(&store);
        assert_eq!(store.retry_forget_cleanup(), ForgetCleanup::Complete);
        assert_eq!(store.retry_forget_cleanup(), ForgetCleanup::Complete);
        assert_eq!(
            changes(&store),
            before,
            "cleanup must do no DML, including audit"
        );
        reader.close().expect("close reader");
        store.close().expect("close writer");
        let mut reopened =
            SqlCipherStore::open(directory.path().join("soul.db"), &keys).expect("reopen");
        let before = changes(&reopened);
        assert_eq!(reopened.retry_forget_cleanup(), ForgetCleanup::Complete);
        assert_eq!(changes(&reopened), before);
        assert!(reopened.open(&sealed).is_err());
        assert_eq!(outcome.receipt.impact.content_key_ids, vec![key]);
    }

    #[test]
    fn failure_before_commit_rolls_back_the_deleted_key() {
        let (_directory, mut store, _keys, key, sealed) = seeded();
        store
            .conn
            .execute_batch(
                "CREATE TEMP TRIGGER fail_after_key_delete BEFORE INSERT ON destroyed_content_keys
             BEGIN SELECT RAISE(ABORT, 'synthetic precommit failure'); END;",
            )
            .expect("install failure after delete and before commit");
        assert!(store
            .execute_forget_outcome(ForgetUnit::ContentKey(key))
            .is_err());
        assert_eq!(
            store.open(&sealed).expect("key deletion rolled back"),
            b"synthetic prose only"
        );
        let destroyed: i64 = store
            .conn
            .query_row("SELECT count(*) FROM destroyed_content_keys", [], |row| {
                row.get(0)
            })
            .expect("read tombstone count");
        assert_eq!(destroyed, 0);
    }

    #[test]
    fn query_failure_is_pending_not_complete() {
        let (_directory, mut store, _keys, _key, _sealed) = seeded();
        store
            .conn
            .execute_batch("BEGIN IMMEDIATE")
            .expect("open write transaction");
        assert_eq!(
            store.retry_forget_cleanup(),
            ForgetCleanup::Pending { checkpoint: None }
        );
        store.conn.execute_batch("ROLLBACK").expect("rollback");
    }

    #[test]
    fn legacy_forget_never_reports_success_for_busy_cleanup() {
        let (_directory, mut store, keys, key, sealed) = seeded();
        let reader = pin_reader(&store, &keys);
        let error = store
            .execute_forget(ForgetUnit::ContentKey(key))
            .expect_err("busy cleanup is not legacy success");
        assert!(error.to_string().contains("committed"));
        assert!(error.to_string().contains("retry_forget_cleanup"));
        assert!(store.open(&sealed).is_err());
        reader
            .conn
            .execute_batch("ROLLBACK")
            .expect("release reader");
    }

    #[test]
    fn checkpoint_and_flush_also_reject_busy_completion() {
        let (_directory, mut store, keys, _key, _sealed) = seeded();
        let reader = pin_reader(&store, &keys);
        assert!(store.checkpoint().is_err());
        assert!(store.flush().is_err());
        reader
            .conn
            .execute_batch("ROLLBACK")
            .expect("release reader");
        store.checkpoint().expect("confirmed truncate");
        store.flush().expect("confirmed flush");
    }
}
