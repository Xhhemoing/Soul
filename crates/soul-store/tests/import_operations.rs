use soul_store::import_ops::ImportRuntimeLease;
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::import::*;
use soul_store_api::types::{StoreError, StoreResult};
use std::sync::Arc;
use uuid::Uuid;

fn operation() -> ImportOperation {
    ImportOperation {
        operation_id: Uuid::now_v7(),
        preview_id: Uuid::now_v7(),
        source: ImportSourceKind::SoulImportV1,
        namespace_id: Uuid::now_v7(),
        legacy_choice: LegacyImportChoice::Reject,
        state: ImportOperationState::Reserved,
        preview: ImportCounts::default(),
        receipt: None,
        error_code: None,
        audit_state: ImportAuditState::NotApplicable,
        created_at: 1,
    }
}

#[test]
fn bookkeeping_hook_failure_rolls_back_transition_and_retry_runs_hook_only_once() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("soul.db");
    let lease = ImportRuntimeLease::claim(&db).unwrap();
    let mut store = SqlCipherStore::open(
        &db,
        &TestKeyProvider::from_seed("synthetic bookkeeping hook"),
    )
    .unwrap();
    store.recover_import_operations(&lease).unwrap();
    let op = operation();
    store.reserve_import_operation(&lease, &op).unwrap();
    let failed: StoreResult<ImportOperation> = store.end_import_operation_with(
        &lease,
        op.operation_id,
        ImportOperationState::Reserved,
        ImportOperationState::Discarded,
        None,
        |_, changed| {
            assert_eq!(changed.state, ImportOperationState::Discarded);
            Err(StoreError::ContractViolation(
                "synthetic audit hook refusal".into(),
            ))
        },
    );
    assert!(failed.is_err());
    assert_eq!(store.import_operation(op.operation_id).unwrap(), op);
    let mut calls = 0;
    let accepted: StoreResult<ImportOperation> = store.end_import_operation_with(
        &lease,
        op.operation_id,
        ImportOperationState::Reserved,
        ImportOperationState::Discarded,
        None,
        |_, _| {
            calls += 1;
            Ok(())
        },
    );
    assert_eq!(accepted.unwrap().state, ImportOperationState::Discarded);
    let replay: StoreResult<ImportOperation> = store.end_import_operation_with(
        &lease,
        op.operation_id,
        ImportOperationState::Reserved,
        ImportOperationState::Discarded,
        None,
        |_, _| {
            calls += 1;
            Ok(())
        },
    );
    assert!(replay.is_err());
    assert_eq!(calls, 1);
}

#[test]
fn a_worker_keeps_the_cross_session_claim_until_its_last_owner_exits() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("soul.db");
    let owner = Arc::new(ImportRuntimeLease::claim(&db).unwrap());
    let worker = owner.clone();
    drop(owner);
    assert!(ImportRuntimeLease::claim(&db).is_err());
    drop(worker);
    assert!(ImportRuntimeLease::claim(&db).is_ok());
}

#[cfg(unix)]
#[test]
fn a_symlink_to_an_existing_database_uses_the_same_runtime_claim() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("soul.db");
    let store = SqlCipherStore::open(&db, &TestKeyProvider::from_seed("symlink runtime")).unwrap();
    let alias = dir.path().join("alias.db");
    std::os::unix::fs::symlink(&db, &alias).unwrap();
    let lease = ImportRuntimeLease::claim(&db).unwrap();
    assert!(ImportRuntimeLease::claim(&alias).is_err());
    drop(lease);
    assert!(ImportRuntimeLease::claim(&alias).is_ok());
    drop(store);
}

#[test]
fn an_attached_runtime_lease_survives_as_long_as_a_raw_store_arc_survives() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("soul.db");
    let lease = Arc::new(ImportRuntimeLease::claim(&db).unwrap());
    let mut store =
        SqlCipherStore::open(&db, &TestKeyProvider::from_seed("attached runtime")).unwrap();
    store.recover_import_operations(&lease).unwrap();
    store.attach_import_runtime(lease.clone()).unwrap();
    let session = Arc::new(std::sync::Mutex::new(store));
    let raw = session.clone();
    drop(lease);
    drop(session);
    assert!(ImportRuntimeLease::claim(&db).is_err());
    drop(raw);
    assert!(ImportRuntimeLease::claim(&db).is_ok());
}

#[test]
fn recovery_is_once_per_lease_and_cannot_interrupt_its_own_live_work() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("soul.db");
    let lease = ImportRuntimeLease::claim(&db).unwrap();
    let mut store =
        SqlCipherStore::open(&db, &TestKeyProvider::from_seed("import operations")).unwrap();
    let op = operation();
    assert!(store.reserve_import_operation(&lease, &op).is_err());
    assert_eq!(store.recover_import_operations(&lease).unwrap(), 0);
    store.reserve_import_operation(&lease, &op).unwrap();
    store
        .start_import_operation(&lease, op.operation_id, op.preview_id)
        .unwrap();
    assert!(store.recover_import_operations(&lease).is_err());
    assert_eq!(
        store.import_operation(op.operation_id).unwrap().state,
        ImportOperationState::Running
    );
    assert!(ImportRuntimeLease::claim(&db).is_err());
}

#[test]
fn preview_ids_and_operation_ids_are_consumed_once_and_foreign_leases_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("soul.db");
    let lease = ImportRuntimeLease::claim(&db).unwrap();
    let mut store =
        SqlCipherStore::open(&db, &TestKeyProvider::from_seed("import operations")).unwrap();
    store.recover_import_operations(&lease).unwrap();
    let op = operation();
    store.reserve_import_operation(&lease, &op).unwrap();
    assert!(store
        .start_import_operation(&lease, op.operation_id, Uuid::now_v7())
        .is_err());
    assert_eq!(
        store.import_operation(op.operation_id).unwrap().state,
        ImportOperationState::Reserved
    );
    store
        .start_import_operation(&lease, op.operation_id, op.preview_id)
        .unwrap();
    assert!(store
        .start_import_operation(&lease, op.operation_id, op.preview_id)
        .is_err());
    let other = ImportRuntimeLease::claim(dir.path().join("other.db")).unwrap();
    assert!(store.recover_import_operations(&other).is_err());
}

#[test]
fn domain_rows_and_receipt_commit_or_roll_back_together() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("soul.db");
    let lease = ImportRuntimeLease::claim(&db).unwrap();
    let mut store =
        SqlCipherStore::open(&db, &TestKeyProvider::from_seed("import operations")).unwrap();
    store.recover_import_operations(&lease).unwrap();
    let op = operation();
    store.reserve_import_operation(&lease, &op).unwrap();
    store
        .start_import_operation(&lease, op.operation_id, op.preview_id)
        .unwrap();
    let namespace = ImportNamespace {
        namespace_id: op.namespace_id,
        source: op.source,
        state: ImportNamespaceState::Active,
        coverage: ImportCoverage::FullFromCreation,
        owner_contact_id: None,
        account_ref: None,
    };
    assert!(store
        .commit_import_operation(&lease, op.operation_id, ImportCounts::default())
        .is_err());
    let failed: StoreResult<()> = store.transact(|store| {
        store.put_import_namespace(&namespace)?;
        store.commit_import_operation(&lease, op.operation_id, ImportCounts::default())?;
        Err(StoreError::ContractViolation(
            "synthetic precommit failure".into(),
        ))
    });
    assert!(failed.is_err());
    assert!(store.import_namespace(namespace.namespace_id).is_err());
    assert_eq!(
        store.import_operation(op.operation_id).unwrap().state,
        ImportOperationState::Running
    );
    let committed: StoreResult<_> = store.transact(|store| {
        store.put_import_namespace(&namespace)?;
        store.commit_import_operation(
            &lease,
            op.operation_id,
            ImportCounts {
                events_written: 3,
                ..Default::default()
            },
        )
    });
    let committed = committed.unwrap();
    assert_eq!(committed.state, ImportOperationState::Committed);
    assert_eq!(committed.receipt.unwrap().events_written, 3);
    assert!(store
        .end_import_operation(
            &lease,
            op.operation_id,
            ImportOperationState::Running,
            ImportOperationState::Rejected,
            Some(ImportOperationError::WorkerFailed)
        )
        .is_err());
}

#[test]
fn reopening_reports_interrupted_work_and_preserves_completed_receipts() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("soul.db");
    let keys = TestKeyProvider::from_seed("import operations");
    let lease = ImportRuntimeLease::claim(&db).unwrap();
    let mut store = SqlCipherStore::open(&db, &keys).unwrap();
    store.recover_import_operations(&lease).unwrap();
    let running = operation();
    store.reserve_import_operation(&lease, &running).unwrap();
    store
        .start_import_operation(&lease, running.operation_id, running.preview_id)
        .unwrap();
    let reserved = operation();
    store.reserve_import_operation(&lease, &reserved).unwrap();
    drop(store);
    drop(lease);
    let lease = ImportRuntimeLease::claim(&db).unwrap();
    let mut store = SqlCipherStore::open(&db, &keys).unwrap();
    assert_eq!(store.recover_import_operations(&lease).unwrap(), 2);
    for id in [running.operation_id, reserved.operation_id] {
        assert_eq!(
            store.import_operation(id).unwrap().state,
            ImportOperationState::Interrupted
        );
    }
    assert!(store
        .start_import_operation(&lease, running.operation_id, running.preview_id)
        .is_err());
    assert_eq!(store.recent_import_operations(100).unwrap().len(), 2);
}
