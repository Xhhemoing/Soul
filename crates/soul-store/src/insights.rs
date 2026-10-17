//! Persisted local feedback. No interpreted text enters this table.
use crate::store::{backend, enum_text, SqlCipherStore};
use rusqlite::OptionalExtension;
use soul_store_api::import::{ImportIndexStore, ImportNamespaceState, ImportOperationState};
use soul_store_api::insight::{InsightValidity, LocalInsightBundle};
use soul_store_api::types::{StoreError, StoreResult};
use uuid::Uuid;

fn invalid() -> StoreError {
    StoreError::ContractViolation("LOCAL_INSIGHT_INVALID".to_owned())
}

pub(crate) fn decode(doc: &str) -> StoreResult<LocalInsightBundle> {
    serde_json::from_str(doc).map_err(|_| invalid())
}
pub(crate) fn encode(bundle: &LocalInsightBundle) -> StoreResult<String> {
    serde_json::to_string(bundle).map_err(|_| invalid())
}

impl SqlCipherStore {
    pub fn invalidate_local_insight(&mut self, id: Uuid) -> StoreResult<LocalInsightBundle> {
        let mut bundle = self.local_insight(id)?;
        bundle.validity = InsightValidity::Invalidated;
        self.write_local_insight(&bundle)?;
        Ok(bundle)
    }
    pub fn local_insight(&self, id: Uuid) -> StoreResult<LocalInsightBundle> {
        let doc: Option<String> = self
            .conn
            .query_row(
                "SELECT doc FROM local_insights WHERE bundle_id=?1",
                [id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(backend)?;
        decode(&doc.ok_or_else(|| StoreError::not_found("local insight", id))?)
    }

    pub fn recent_local_insights(&self, limit: usize) -> StoreResult<Vec<LocalInsightBundle>> {
        let mut statement = self
            .conn
            .prepare(
                "SELECT doc FROM local_insights ORDER BY created_at DESC,bundle_id DESC LIMIT ?1",
            )
            .map_err(backend)?;
        let docs = statement
            .query_map([limit.min(100) as i64], |row| row.get::<_, String>(0))
            .map_err(backend)?;
        docs.map(|doc| decode(&doc.map_err(backend)?)).collect()
    }

    /// Automatic insights cannot outlive a failed import transaction.
    pub fn put_local_insight(&mut self, bundle: &LocalInsightBundle) -> StoreResult<()> {
        if self.conn.is_autocommit() || bundle.validity != InsightValidity::Active || bundle.ignored
        {
            return Err(invalid());
        }
        let op = self.import_operation(bundle.operation_id)?;
        let namespace = self.import_namespace(bundle.namespace_id)?;
        if op.state != ImportOperationState::Committed
            || op.namespace_id != bundle.namespace_id
            || op.receipt.as_ref() != Some(&bundle.counts)
            || namespace.state != ImportNamespaceState::Active
        {
            return Err(invalid());
        }
        self.conn.execute("INSERT INTO local_insights(bundle_id,operation_id,namespace_id,created_at,validity,ignored,doc) VALUES(?1,?2,?3,?4,'active',0,?5)",
            rusqlite::params![bundle.bundle_id.to_string(),bundle.operation_id.to_string(),bundle.namespace_id.to_string(),bundle.created_at,encode(bundle)?]).map_err(backend)?;
        Ok(())
    }

    pub fn invalidate_local_insights(&mut self) -> StoreResult<u64> {
        let mut statement = self
            .conn
            .prepare("SELECT doc FROM local_insights WHERE validity='active'")
            .map_err(backend)?;
        let docs = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(backend)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(backend)?;
        drop(statement);
        for doc in &docs {
            let mut bundle = decode(doc)?;
            bundle.validity = InsightValidity::Invalidated;
            self.write_local_insight(&bundle)?;
        }
        Ok(docs.len() as u64)
    }

    pub fn set_local_insight_ignored(
        &mut self,
        id: Uuid,
        ignored: bool,
    ) -> StoreResult<LocalInsightBundle> {
        let mut bundle = self.local_insight(id)?;
        bundle.ignored = ignored;
        self.write_local_insight(&bundle)?;
        Ok(bundle)
    }

    fn write_local_insight(&mut self, bundle: &LocalInsightBundle) -> StoreResult<()> {
        self.conn
            .execute(
                "UPDATE local_insights SET validity=?2,ignored=?3,doc=?4 WHERE bundle_id=?1",
                rusqlite::params![
                    bundle.bundle_id.to_string(),
                    enum_text(&bundle.validity)?,
                    bundle.ignored,
                    encode(bundle)?
                ],
            )
            .map_err(backend)?;
        Ok(())
    }
}
