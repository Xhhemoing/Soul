//! Service-boundary failures use synthetic data and a real temporary store.
//! The forced pending flag tests propagation, not a real busy reader; actual
//! pinned-reader cases live in soul-store's outcome_tests.

use soul_memory::{
    create, forget_with_outcome, read, retry_forget_cleanup, ForgetAudit, MemoryDraft,
};
use soul_schema::audit::SoulAuditEntry;
use soul_schema::memory::{MemoryType, SoulMemory};
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::forget::{
    ForgetCleanup, ForgetImpact, ForgetOps, ForgetOutcome, ForgetOutcomeOps, ForgetReceipt,
    ForgetUnit, WalCheckpoint,
};
use soul_store_api::types::{StoreError, StoreResult};
use soul_store_api::{AuditLog, MemoryStore};
use uuid::Uuid;

const NOW: i64 = 1_800_000_000;
const PRIVATE_ERROR: &str = "synthetic-backend-detail-must-not-cross-the-boundary";

#[derive(Debug, Clone, Copy)]
enum AuditFailure {
    None,
    BeforeWrite,
    AfterWrite,
}

#[derive(Debug)]
struct ObservedStore {
    inner: SqlCipherStore,
    audit_failure: AuditFailure,
    force_pending: bool,
    fail_before_commit: bool,
    forget_calls: usize,
    audit_calls: usize,
    cleanup_calls: usize,
}

impl MemoryStore for ObservedStore {
    fn put_memory(&mut self, memory: SoulMemory) -> StoreResult<Uuid> {
        self.inner.put_memory(memory)
    }

    fn get_memory(&self, memory_id: Uuid) -> StoreResult<SoulMemory> {
        self.inner.get_memory(memory_id)
    }

    fn list_memories(&self) -> StoreResult<Vec<SoulMemory>> {
        self.inner.list_memories()
    }
}

impl ForgetOps for ObservedStore {
    fn preview_impact(&self, unit: ForgetUnit) -> StoreResult<ForgetImpact> {
        self.inner.preview_impact(unit)
    }

    fn execute_forget(&mut self, _unit: ForgetUnit) -> StoreResult<ForgetReceipt> {
        panic!("the outcome service must not fall back to the legacy operation")
    }
}

impl ForgetOutcomeOps for ObservedStore {
    fn execute_forget_outcome(&mut self, unit: ForgetUnit) -> StoreResult<ForgetOutcome> {
        self.forget_calls += 1;
        if self.fail_before_commit {
            return Err(StoreError::Backend(PRIVATE_ERROR.into()));
        }
        let mut outcome = self.inner.execute_forget_outcome(unit)?;
        if self.force_pending {
            outcome.cleanup = ForgetCleanup::Pending {
                checkpoint: Some(WalCheckpoint {
                    busy: 1,
                    log_frames: 4,
                    checkpointed_frames: 3,
                }),
            };
        }
        Ok(outcome)
    }

    fn retry_forget_cleanup(&mut self) -> ForgetCleanup {
        self.cleanup_calls += 1;
        self.inner.retry_forget_cleanup()
    }
}

impl AuditLog for ObservedStore {
    fn append_audit(&mut self, entry: SoulAuditEntry) -> StoreResult<Uuid> {
        self.audit_calls += 1;
        if matches!(self.audit_failure, AuditFailure::BeforeWrite) {
            return Err(StoreError::Backend(PRIVATE_ERROR.into()));
        }
        let id = self.inner.append_audit(entry)?;
        if matches!(self.audit_failure, AuditFailure::AfterWrite) {
            Err(StoreError::Backend(PRIVATE_ERROR.into()))
        } else {
            Ok(id)
        }
    }

    fn list_audit(&self) -> StoreResult<Vec<SoulAuditEntry>> {
        self.inner.list_audit()
    }
}

fn seeded() -> (tempfile::TempDir, TestKeyProvider, Uuid, ObservedStore) {
    let directory = tempfile::tempdir().expect("temporary synthetic store");
    let keys = TestKeyProvider::from_seed("memory forget outcome service tests");
    let mut inner =
        SqlCipherStore::open(directory.path().join("soul.db"), &keys).expect("SQLCipher store");
    let memory = create(
        &mut inner,
        &MemoryDraft::own(
            MemoryType::Episodic,
            "synthetic title",
            "synthetic private prose",
        ),
        NOW,
    )
    .expect("synthetic memory");
    let observed = ObservedStore {
        inner,
        audit_failure: AuditFailure::None,
        force_pending: false,
        fail_before_commit: false,
        forget_calls: 0,
        audit_calls: 0,
        cleanup_calls: 0,
    };
    (directory, keys, memory.memory_id, observed)
}

#[test]
fn successful_forget_reports_cleanup_and_audit_independently() {
    let (_directory, _keys, id, mut store) = seeded();
    let expected = store.preview_impact(ForgetUnit::Memory(id)).expect("preview");
    let result = forget_with_outcome(&mut store, id, NOW).expect("committed outcome");
    assert_eq!(result.receipt.impact, expected);
    assert_eq!(result.cleanup, ForgetCleanup::Complete);
    assert_eq!(result.audit, ForgetAudit::Recorded);
    assert_eq!((store.forget_calls, store.audit_calls), (1, 1));
    assert!(read(&store.inner, id).is_err());
}

#[test]
fn audit_failure_retains_the_receipt_and_destruction_survives_reopen() {
    let (directory, keys, id, mut store) = seeded();
    store.audit_failure = AuditFailure::BeforeWrite;
    let before = store.list_audit().expect("audit before").len();
    let result = forget_with_outcome(&mut store, id, NOW).expect("audit cannot hide commit");
    assert_eq!(result.audit, ForgetAudit::Unconfirmed);
    assert_eq!(result.cleanup, ForgetCleanup::Complete);
    assert_eq!(store.list_audit().expect("audit after").len(), before);
    assert_eq!((store.forget_calls, store.audit_calls), (1, 1));
    let serialized = serde_json::to_string(&result).expect("serialize receipt");
    assert!(!serialized.contains(PRIVATE_ERROR));
    assert!(!serialized.contains("synthetic private prose"));
    store.inner.close().expect("close");
    let reopened =
        SqlCipherStore::open(directory.path().join("soul.db"), &keys).expect("reopen");
    assert!(read(&reopened, id).is_err());
}

#[test]
fn unacknowledged_audit_is_not_reappended_by_cleanup_retry() {
    let (_directory, _keys, id, mut store) = seeded();
    store.audit_failure = AuditFailure::AfterWrite;
    let before = store.list_audit().expect("audit before").len();
    let result = forget_with_outcome(&mut store, id, NOW).expect("committed outcome");
    assert_eq!(result.audit, ForgetAudit::Unconfirmed);
    assert_eq!(store.list_audit().expect("durable append").len(), before + 1);
    assert_eq!(retry_forget_cleanup(&mut store), ForgetCleanup::Complete);
    assert_eq!(retry_forget_cleanup(&mut store), ForgetCleanup::Complete);
    assert_eq!(
        (store.forget_calls, store.audit_calls, store.cleanup_calls),
        (1, 1, 2)
    );
    assert_eq!(store.list_audit().expect("no duplicate").len(), before + 1);
    assert_eq!(result.audit, ForgetAudit::Unconfirmed);
    store.inner.verify_audit_chain().expect("chain still verifies");
}

#[test]
fn pending_cleanup_still_attempts_audit_once_and_keeps_the_original_impact() {
    for failure in [
        AuditFailure::None,
        AuditFailure::BeforeWrite,
        AuditFailure::AfterWrite,
    ] {
        let (_directory, _keys, id, mut store) = seeded();
        store.force_pending = true;
        store.audit_failure = failure;
        let expected = store.preview_impact(ForgetUnit::Memory(id)).expect("preview");
        let result = forget_with_outcome(&mut store, id, NOW).expect("committed outcome");
        assert_eq!(result.receipt.impact, expected);
        assert!(matches!(result.cleanup, ForgetCleanup::Pending { .. }));
        let expected_audit = match failure {
            AuditFailure::None => ForgetAudit::Recorded,
            _ => ForgetAudit::Unconfirmed,
        };
        assert_eq!(result.audit, expected_audit);
        assert_eq!((store.forget_calls, store.audit_calls), (1, 1));
        assert!(read(&store.inner, id).is_err());
    }
}

#[test]
fn precommit_error_does_not_attempt_an_audit_or_claim_destruction() {
    let (_directory, _keys, id, mut store) = seeded();
    store.fail_before_commit = true;
    assert!(forget_with_outcome(&mut store, id, NOW).is_err());
    assert_eq!((store.forget_calls, store.audit_calls), (1, 0));
    assert!(read(&store.inner, id).is_ok());
}

#[test]
fn missing_memory_does_not_reach_destruction_or_audit() {
    let (_directory, _keys, _id, mut store) = seeded();
    assert!(forget_with_outcome(&mut store, Uuid::now_v7(), NOW).is_err());
    assert_eq!((store.forget_calls, store.audit_calls), (0, 0));
}
