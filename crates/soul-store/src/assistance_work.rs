//! One durable local intent per assistance item; no model capability or prose.
use crate::store::{backend, SqlCipherStore};
use rusqlite::{params, Connection, OptionalExtension};
use soul_store_api::assistance::{
    AssistanceWorkClaim, AssistanceWorkMetadata, AssistanceWorkState,
};
use soul_store_api::types::{StoreError, StoreResult};
use uuid::Uuid;

/// Raw columns selected by `assistance_item_work`, in query order.
type AssistanceWorkRow = (
    Option<String>, // pending_revision
    Option<String>, // claimed_revision
    Option<String>, // claim_id
    Option<String>, // result_revision
    Option<String>, // result_status
    Option<String>, // failure_code
);

pub(crate) fn enqueue(conn: &Connection, item: &str) -> StoreResult<()> {
    conn.execute("INSERT INTO assistance_work(item_id,pending_revision) SELECT item_id,revision FROM assistance_items WHERE item_id=?1 AND handling='open' AND validity!='forgotten' ON CONFLICT(item_id) DO UPDATE SET pending_revision=excluded.pending_revision,result_revision=NULL,result_status=NULL,failure_code=NULL",[item]).map_err(backend)?;
    conn.execute("UPDATE assistance_work SET pending_revision=NULL,result_revision=NULL,result_status=NULL,failure_code=NULL WHERE item_id=?1 AND EXISTS(SELECT 1 FROM assistance_items WHERE item_id=?1 AND (handling!='open' OR validity='forgotten'))",[item]).map_err(backend)?;
    crate::reminders::invalidate_item(conn, item)?;
    crate::preparation::invalidate_item(conn, item)?;
    Ok(())
}

/// Store-level writes coalesce candidates; only domain validation decides stale.
pub(crate) fn enqueue_sources(conn: &Connection) -> StoreResult<()> {
    conn.execute("INSERT INTO assistance_work(item_id,pending_revision) SELECT item_id,revision FROM assistance_items WHERE bundle_id IS NOT NULL AND handling='open' AND validity!='forgotten' ON CONFLICT(item_id) DO UPDATE SET pending_revision=excluded.pending_revision,result_revision=NULL,result_status=NULL,failure_code=NULL",[]).map_err(backend)?;
    crate::preparation::invalidate_sources(conn)
}

pub(crate) fn invalidate_bundle(conn: &Connection, bundle: Uuid) -> StoreResult<()> {
    conn.execute("UPDATE assistance_items SET validity='stale',revision=?2 WHERE bundle_id=?1 AND validity='current'",params![bundle.to_string(),Uuid::now_v7().to_string()]).map_err(backend)?;
    let mut statement = conn
        .prepare("SELECT item_id FROM assistance_items WHERE bundle_id=?1")
        .map_err(backend)?;
    let ids = statement
        .query_map([bundle.to_string()], |r| r.get::<_, String>(0))
        .map_err(backend)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(backend)?;
    drop(statement);
    for id in ids {
        enqueue(conn, &id)?;
    }
    Ok(())
}

impl SqlCipherStore {
    pub(crate) fn recover_assistance_work(&self) -> StoreResult<()> {
        self.conn.execute("UPDATE assistance_work SET pending_revision=(SELECT CASE WHEN handling='open' AND validity!='forgotten' THEN revision END FROM assistance_items WHERE assistance_items.item_id=assistance_work.item_id),claim_id=NULL,claimed_revision=NULL WHERE claim_id IS NOT NULL",[]).map_err(backend)?;
        Ok(())
    }
    pub fn assistance_work_counts(&self) -> StoreResult<(u64, u64)> {
        self.conn.query_row("SELECT count(CASE WHEN pending_revision IS NOT NULL THEN 1 END),count(CASE WHEN claim_id IS NOT NULL THEN 1 END) FROM assistance_work",[],|r|Ok((r.get::<_,i64>(0)? as u64,r.get::<_,i64>(1)? as u64))).map_err(backend)
    }
    pub fn assistance_item_work(&self, id: Uuid) -> StoreResult<AssistanceWorkMetadata> {
        let item = self.assistance_item(id)?;
        let row:Option<AssistanceWorkRow>=self.conn.query_row("SELECT pending_revision,claimed_revision,claim_id,result_revision,result_status,failure_code FROM assistance_work WHERE item_id=?1",[id.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional().map_err(backend)?;
        let (pending, claimed, claim, result, status, failure) = row.unwrap_or_default();
        let parse = |v: Option<String>| v.map(|v| Uuid::parse_str(&v).map_err(backend)).transpose();
        let pending_revision = parse(pending)?;
        let claimed_revision = parse(claimed)?;
        let claim_id = parse(claim)?;
        let result_revision = parse(result)?;
        let state = if claim_id.is_some() {
            AssistanceWorkState::Running
        } else if pending_revision.is_some() {
            AssistanceWorkState::Pending
        } else if result_revision == Some(item.revision) {
            match status.as_deref() {
                Some("ready") => AssistanceWorkState::Ready,
                Some("source_unavailable") => AssistanceWorkState::SourceUnavailable,
                Some("failed") => AssistanceWorkState::Failed,
                _ => AssistanceWorkState::None,
            }
        } else {
            AssistanceWorkState::None
        };
        Ok(AssistanceWorkMetadata {
            item_id: id,
            item_revision: item.revision,
            state,
            pending_revision,
            claimed_revision,
            claim_id,
            result_revision,
            failure_code: failure,
        })
    }
    pub fn retry_assistance_work(
        &mut self,
        id: Uuid,
        revision: Uuid,
    ) -> StoreResult<AssistanceWorkMetadata> {
        self.transact(|store| {
            let item = store.assistance_item(id)?;
            if item.revision != revision
                || !item.is_open()
                || item.validity == soul_store_api::assistance::AssistanceValidity::Forgotten
            {
                return Err(StoreError::ContractViolation("ASSISTANCE_CHANGED".into()));
            }
            let metadata = store.assistance_item_work(id)?;
            if metadata.claim_id.is_some() {
                return Err(StoreError::ContractViolation(
                    "ASSISTANCE_WORK_RUNNING".into(),
                ));
            }
            if metadata.state != AssistanceWorkState::Failed {
                return Err(StoreError::ContractViolation(
                    "ASSISTANCE_WORK_NOT_FAILED".into(),
                ));
            }
            if metadata.pending_revision != Some(revision) {
                enqueue(&store.conn, &id.to_string())?;
            }
            store.assistance_item_work(id)
        })
    }
    pub fn claim_assistance_work(&mut self) -> StoreResult<Option<AssistanceWorkClaim>> {
        self.transact(|store|{
            let running:bool=store.conn.query_row("SELECT EXISTS(SELECT 1 FROM assistance_work WHERE claim_id IS NOT NULL)",[],|r|r.get(0)).map_err(backend)?;
            if running{return Ok(None)}
            let next:Option<(String,String)>=store.conn.query_row("SELECT w.item_id,w.pending_revision FROM assistance_work w JOIN assistance_items i ON i.item_id=w.item_id WHERE w.pending_revision=i.revision AND i.handling='open' AND i.validity!='forgotten' ORDER BY i.created_at,i.item_id LIMIT 1",[],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(backend)?;
            let Some((id,revision))=next else{return Ok(None)};
            let item_id=id.parse().map_err(backend)?;
            let preference=store.local_preparation_preference(item_id)?;
            let claim=AssistanceWorkClaim{item_id,revision:revision.parse().map_err(backend)?,claim_id:Uuid::now_v7(),preparation_revision:if preference.enabled{preference.revision}else{None}};
            store.conn.execute("UPDATE assistance_work SET claim_id=?2,claimed_revision=pending_revision,pending_revision=NULL WHERE item_id=?1 AND claim_id IS NULL",params![id,claim.claim_id.to_string()]).map_err(backend)?;Ok(Some(claim))
        })
    }
    /// Must be in the transaction that performed final domain revalidation.
    pub fn finish_assistance_work(
        &self,
        claim: &AssistanceWorkClaim,
        status: AssistanceWorkState,
        failure: Option<&str>,
    ) -> StoreResult<bool> {
        if self.conn.is_autocommit() {
            return Err(StoreError::ContractViolation(
                "ASSISTANCE_RESULT_REQUIRES_TRANSACTION".into(),
            ));
        }
        let failure = if status == AssistanceWorkState::Failed {
            Some(match failure {
                Some("ASSISTANCE_LOCAL_READ_FAILED") => "ASSISTANCE_LOCAL_READ_FAILED",
                Some("ASSISTANCE_LOCAL_COMMIT_FAILED") => "ASSISTANCE_LOCAL_COMMIT_FAILED",
                _ => "ASSISTANCE_LOCAL_WORKER_FAILED",
            })
        } else {
            None
        };
        let status = match status {
            AssistanceWorkState::Ready => "ready",
            AssistanceWorkState::SourceUnavailable => "source_unavailable",
            AssistanceWorkState::Failed => "failed",
            _ => {
                return Err(StoreError::ContractViolation(
                    "ASSISTANCE_RESULT_INVALID".into(),
                ))
            }
        };
        let item = self.assistance_item(claim.item_id)?;
        let accepted = item.revision == claim.revision
            && item.is_open()
            && item.validity != soul_store_api::assistance::AssistanceValidity::Forgotten;
        let changed=self.conn.execute("UPDATE assistance_work SET claim_id=NULL,claimed_revision=NULL,last_completed_claim_id=?2,result_revision=CASE WHEN ?4 THEN ?3 ELSE result_revision END,result_status=CASE WHEN ?4 THEN ?5 ELSE result_status END,failure_code=CASE WHEN ?4 THEN ?6 ELSE failure_code END WHERE item_id=?1 AND claim_id=?2 AND claimed_revision=?3",params![claim.item_id.to_string(),claim.claim_id.to_string(),claim.revision.to_string(),accepted,status,failure]).map_err(backend)?;
        if changed != 1 {
            return Err(StoreError::ContractViolation(
                "ASSISTANCE_CLAIM_CHANGED".into(),
            ));
        }
        Ok(accepted)
    }
    pub fn assistance_claim_completed(&self, claim: &AssistanceWorkClaim) -> StoreResult<bool> {
        let id: Option<String> = self
            .conn
            .query_row(
                "SELECT last_completed_claim_id FROM assistance_work WHERE item_id=?1",
                [claim.item_id.to_string()],
                |r| r.get(0),
            )
            .optional()
            .map_err(backend)?
            .flatten();
        Ok(id.as_deref() == Some(claim.claim_id.to_string().as_str()))
    }
    /// Only the Session which observed the physical worker finish may call this.
    pub fn abandon_assistance_claim(&mut self, claim: &AssistanceWorkClaim) -> StoreResult<()> {
        self.transact(|store|{
            if store.assistance_claim_completed(claim)?{return Ok(())}
            let changed=store.conn.execute("UPDATE assistance_work SET claim_id=NULL,claimed_revision=NULL WHERE item_id=?1 AND claim_id=?2 AND claimed_revision=?3",params![claim.item_id.to_string(),claim.claim_id.to_string(),claim.revision.to_string()]).map_err(backend)?;
            if changed!=1{return Err(StoreError::ContractViolation("ASSISTANCE_COORDINATION_UNKNOWN".into()))}
            // A newer pending revision belongs to its own event and is retained.
            store.conn.execute("UPDATE assistance_work SET pending_revision=(SELECT revision FROM assistance_items WHERE item_id=?1 AND handling='open' AND validity!='forgotten') WHERE item_id=?1 AND pending_revision IS NULL",[claim.item_id.to_string()]).map_err(backend)?;Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soul_store_api::assistance::{
        AssistanceHandling, AssistanceItem, AssistanceSource, AssistanceValidity,
    };
    fn setup() -> (tempfile::TempDir, SqlCipherStore, AssistanceItem) {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SqlCipherStore::open(
            dir.path().join("work.db"),
            &crate::TestKeyProvider::from_seed("synthetic local work"),
        )
        .unwrap();
        let item = AssistanceItem {
            item_id: Uuid::now_v7(),
            revision: Uuid::now_v7(),
            source: AssistanceSource::ManualGoal {
                memory_id: Uuid::now_v7(),
                observation: None,
            },
            validity: AssistanceValidity::Current,
            handling: AssistanceHandling::Open,
            deadline_at: None,
            created_at: 1,
            updated_at: 1,
        };
        store.insert_assistance_item(&item).unwrap();
        (dir, store, item)
    }
    #[test]
    fn running_claim_never_erases_new_pending() {
        let (_dir, mut store, mut item) = setup();
        let claim = store.claim_assistance_work().unwrap().unwrap();
        let old = item.revision;
        item.revision = Uuid::now_v7();
        store.replace_assistance_item(&item, old).unwrap();
        store
            .transact(|s| s.finish_assistance_work(&claim, AssistanceWorkState::Ready, None))
            .unwrap();
        let metadata = store.assistance_item_work(item.item_id).unwrap();
        assert_eq!(metadata.pending_revision, Some(item.revision));
        assert_eq!(metadata.state, AssistanceWorkState::Pending);
        assert!(store.assistance_claim_completed(&claim).unwrap());
        store.abandon_assistance_claim(&claim).unwrap();
        assert_eq!(store.assistance_item_work(item.item_id).unwrap(), metadata);
    }
    #[test]
    fn retry_rejects_pending_work_without_changing_the_intent() {
        let (_dir, mut store, item) = setup();
        let before = store.assistance_item_work(item.item_id).unwrap();
        assert_eq!(before.state, AssistanceWorkState::Pending);
        let error = store
            .retry_assistance_work(item.item_id, item.revision)
            .unwrap_err();
        assert!(
            matches!(error, StoreError::ContractViolation(code) if code == "ASSISTANCE_WORK_NOT_FAILED")
        );
        assert_eq!(store.assistance_item_work(item.item_id).unwrap(), before);
        assert_eq!(store.assistance_work_counts().unwrap(), (1, 0));
    }
    #[test]
    fn retry_rejects_completed_or_missing_work_without_replaying_a_claim() {
        for state in [
            AssistanceWorkState::Ready,
            AssistanceWorkState::SourceUnavailable,
        ] {
            let (_dir, mut store, item) = setup();
            let claim = store.claim_assistance_work().unwrap().unwrap();
            store
                .transact(|s| s.finish_assistance_work(&claim, state, None))
                .unwrap();
            let before = store.assistance_item_work(item.item_id).unwrap();
            assert_eq!(before.state, state);
            let error = store
                .retry_assistance_work(item.item_id, item.revision)
                .unwrap_err();
            assert!(
                matches!(error, StoreError::ContractViolation(code) if code == "ASSISTANCE_WORK_NOT_FAILED")
            );
            assert_eq!(store.assistance_item_work(item.item_id).unwrap(), before);
            assert!(store.assistance_claim_completed(&claim).unwrap());
            assert_eq!(store.assistance_work_counts().unwrap(), (0, 0));
            store
                .conn
                .execute("DELETE FROM assistance_work", [])
                .unwrap();
            let before = store.assistance_item_work(item.item_id).unwrap();
            assert_eq!(before.state, AssistanceWorkState::None);
            let error = store
                .retry_assistance_work(item.item_id, item.revision)
                .unwrap_err();
            assert!(
                matches!(error, StoreError::ContractViolation(code) if code == "ASSISTANCE_WORK_NOT_FAILED")
            );
            assert_eq!(store.assistance_item_work(item.item_id).unwrap(), before);
            assert_eq!(store.assistance_work_counts().unwrap(), (0, 0));
        }
    }
    #[test]
    fn failed_same_revision_needs_explicit_retry_and_new_claim() {
        let (_dir, mut store, item) = setup();
        let claim = store.claim_assistance_work().unwrap().unwrap();
        store
            .transact(|s| {
                s.finish_assistance_work(
                    &claim,
                    AssistanceWorkState::Failed,
                    Some("ASSISTANCE_SOURCE_READ_FAILED"),
                )
            })
            .unwrap();
        assert!(store.claim_assistance_work().unwrap().is_none());
        assert_eq!(
            store.assistance_item_work(item.item_id).unwrap().state,
            AssistanceWorkState::Failed
        );
        store
            .retry_assistance_work(item.item_id, item.revision)
            .unwrap();
        assert!(matches!(
            store.retry_assistance_work(item.item_id, item.revision),
            Err(StoreError::ContractViolation(code)) if code == "ASSISTANCE_WORK_NOT_FAILED"
        ));
        let next = store.claim_assistance_work().unwrap().unwrap();
        assert_ne!(next.claim_id, claim.claim_id);
        assert_eq!(next.revision, claim.revision);
        assert!(store
            .retry_assistance_work(item.item_id, item.revision)
            .is_err());
    }
    #[test]
    fn dropped_claim_requeue_keeps_newer_pending() {
        let (_dir, mut store, mut item) = setup();
        let claim = store.claim_assistance_work().unwrap().unwrap();
        let old = item.revision;
        item.revision = Uuid::now_v7();
        store.replace_assistance_item(&item, old).unwrap();
        store.abandon_assistance_claim(&claim).unwrap();
        assert_eq!(
            store
                .assistance_item_work(item.item_id)
                .unwrap()
                .pending_revision,
            Some(item.revision)
        );
        assert_ne!(
            store.claim_assistance_work().unwrap().unwrap().claim_id,
            claim.claim_id
        );
    }
    #[test]
    fn schema_four_seeds_once_and_reopen_never_requeues_a_consumed_result() {
        let (dir, store, item) = setup();
        let path = dir.path().join("work.db");
        store
            .conn
            .execute_batch(
                "DROP TABLE assistance_work; UPDATE meta SET value='4' WHERE key='schema_version'",
            )
            .unwrap();
        drop(store);
        let keys = crate::TestKeyProvider::from_seed("synthetic local work");
        let mut store = SqlCipherStore::open(&path, &keys).unwrap();
        assert_eq!(store.assistance_item(item.item_id).unwrap(), item);
        assert_eq!(
            store.assistance_item_work(item.item_id).unwrap().state,
            AssistanceWorkState::Pending
        );
        let claim = store.claim_assistance_work().unwrap().unwrap();
        store
            .transact(|s| s.finish_assistance_work(&claim, AssistanceWorkState::Ready, None))
            .unwrap();
        drop(store);
        let store = SqlCipherStore::open(&path, &keys).unwrap();
        assert_eq!(
            store.assistance_item_work(item.item_id).unwrap().state,
            AssistanceWorkState::Ready
        );
        assert_eq!(store.assistance_work_counts().unwrap(), (0, 0));
        store
            .conn
            .execute("DELETE FROM assistance_work", [])
            .unwrap();
        drop(store);
        let store = SqlCipherStore::open(&path, &keys).unwrap();
        assert_eq!(
            store.assistance_item_work(item.item_id).unwrap().state,
            AssistanceWorkState::None
        );
        assert_eq!(store.assistance_work_counts().unwrap(), (0, 0));
    }
}
