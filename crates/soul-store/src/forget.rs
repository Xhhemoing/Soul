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
use soul_store_api::forget::{ForgetImpact, ForgetOps, ForgetReceipt, ForgetUnit};
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
            ForgetUnit::Contact(id) => self.uuid_column(
                "SELECT display_label_key_id FROM contacts
                 WHERE contact_id = ? AND display_label_key_id IS NOT NULL",
                &[id.to_string()],
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
        let contacts = self.uuid_column(
            &format!(
                "SELECT contact_id FROM contacts
                 WHERE display_label_key_id IN ({key_slots}) ORDER BY contact_id"
            ),
            &key_text,
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

        Ok(ForgetReceipt { unit, impact })
    }
}
