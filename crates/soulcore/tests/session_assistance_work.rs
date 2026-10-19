//! Fault boundaries of durable local work and actual collector handoff.
use soul_store::{DpapiKeyProvider, KeyProvider, TestKeyProvider};
use soul_store_api::import::{ImportSourceKind, LegacyImportChoice};
use soul_store_api::MemoryStore;
use soul_testkit::fixtures;
use soulcore::commands::collect::{AppIdentity, CollectorConfig, ForegroundSource, SourceError};
use soulcore::commands::session::*;
use soulcore::commands::store::database_path;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

fn setup() -> (tempfile::TempDir, Session) {
    let dir = tempfile::tempdir().unwrap();
    let s = Session::open(dir.path());
    assert!(s.status().store_opened);
    (dir, s)
}
fn goal(s: &mut Session) -> AssistanceItemRow {
    s.create_assistance_goal(CreateAssistanceGoal {
        title: "synthetic work goal".into(),
        summary: "synthetic goal body".into(),
        deadline_at: None,
        source_observation: None,
    })
    .unwrap()
    .item
}
fn req(i: &AssistanceItemRow) -> AssistanceItemRequest {
    AssistanceItemRequest {
        item_id: i.item_id,
        expected_revision: i.revision,
    }
}
fn raw(dir: &std::path::Path) -> rusqlite::Connection {
    let keys: Box<dyn KeyProvider> = if cfg!(windows) {
        Box::new(DpapiKeyProvider::new(dir.join(KEY_BLOB_FILE_NAME)))
    } else {
        Box::new(TestKeyProvider::in_dir(dir))
    };
    let c = rusqlite::Connection::open(database_path(dir)).unwrap();
    c.execute_batch(&format!(
        "PRAGMA key = \"x'{}'\";",
        keys.database_key().unwrap().to_hex()
    ))
    .unwrap();
    c
}
fn text() -> String {
    fixtures::read_text("import/soul-import-v1/valid_basic.jsonl").unwrap()
}
fn import(s: &mut Session) {
    let w = s
        .begin_import_prepare(
            text(),
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    let p = s.complete_import_prepare(w.execute()).unwrap();
    let w = s
        .begin_import(
            &p.operation.operation_id.to_string(),
            &p.operation.preview_id.to_string(),
        )
        .unwrap();
    s.complete_import(w.execute()).unwrap();
}

#[test]
fn background_respects_foreground_context_and_dropped_claim_requeues_only_after_finish() {
    let (_dir, mut s) = setup();
    let i = goal(&mut s);
    let context = s.prepare_assistance_context(req(&i)).unwrap();
    assert!(s.begin_assistance_work().unwrap().is_none());
    let AssistanceContextView::Memory { memory, .. } = context else {
        panic!("memory")
    };
    s.draft_pasted_with_memory("synthetic paste", &memory.context_id.unwrap().to_string())
        .unwrap();
    let w = s.begin_assistance_work().unwrap().unwrap();
    assert!(s.begin_assistance_work().unwrap().is_none());
    drop(w);
    s.reconcile_assistance_work().unwrap();
    assert_eq!(
        s.assistance_item_work(&i.item_id.to_string())
            .unwrap()
            .state,
        AssistanceWorkState::Pending
    );
    let w = s.begin_assistance_work().unwrap().unwrap();
    let out = s.complete_assistance_work(w.execute()).unwrap();
    assert_eq!(out.status, AssistanceWorkDisposition::Ready);
    assert_eq!(
        s.assistance_item_work(&i.item_id.to_string())
            .unwrap()
            .state,
        AssistanceWorkState::Ready
    );
    assert!(!s
        .audit()
        .unwrap()
        .entries
        .iter()
        .any(|e| e.action == "egress.request"));
}

#[test]
fn durable_result_survives_lost_completion_and_new_pending_survives_old_result() {
    let (_dir, mut s) = setup();
    let i = goal(&mut s);
    let w = s.begin_assistance_work().unwrap().unwrap();
    drop(w.execute());
    s.reconcile_assistance_work().unwrap();
    assert!(s.begin_assistance_work().unwrap().is_none());
    s.retry_assistance_work(req(&i)).unwrap();
    let shared = s.store().unwrap();
    let w = s.begin_assistance_work().unwrap().unwrap();
    {
        let mut store = shared.lock().unwrap();
        let AssistanceSource::ManualGoal { memory_id, .. } = i.source else {
            panic!("memory")
        };
        let m = store.get_memory(memory_id).unwrap();
        store.put_memory(m).unwrap();
    }
    let out = s.complete_assistance_work(w.execute()).unwrap();
    assert_eq!(out.status, AssistanceWorkDisposition::Superseded);
    let current = s.assistance_item(&i.item_id.to_string()).unwrap().item;
    let metadata = s.assistance_item_work(&i.item_id.to_string()).unwrap();
    assert_eq!(metadata.pending_revision, Some(current.revision));
    assert_eq!(metadata.state, AssistanceWorkState::Pending);
    assert!(s.retry_assistance_work(req(&i)).is_err());
    let w = s.begin_assistance_work().unwrap().unwrap();
    s.complete_assistance_work(w.execute()).unwrap();
}

#[test]
fn failed_read_is_visible_without_hot_retry_and_explicit_same_revision_retry_can_succeed() {
    let (dir, mut s) = setup();
    let i = goal(&mut s);
    let conn = raw(dir.path());
    let AssistanceSource::ManualGoal { memory_id, .. } = i.source else {
        panic!("memory")
    };
    let shared = s.store().unwrap();
    let blob = shared
        .lock()
        .unwrap()
        .get_memory(memory_id)
        .unwrap()
        .title_ref
        .unwrap()
        .blob_id
        .to_string();
    conn.execute_batch("CREATE TEMP TABLE saved AS SELECT * FROM sealed_blobs WHERE 0")
        .unwrap();
    conn.execute(
        "INSERT INTO saved SELECT * FROM sealed_blobs WHERE blob_id=?1",
        [&blob],
    )
    .unwrap();
    conn.execute("DELETE FROM sealed_blobs WHERE blob_id=?1", [&blob])
        .unwrap();
    let w = s.begin_assistance_work().unwrap().unwrap();
    let out = s.complete_assistance_work(w.execute()).unwrap();
    assert_eq!(out.status, AssistanceWorkDisposition::Failed);
    let old = s.assistance_item_work(&i.item_id.to_string()).unwrap();
    assert_eq!(old.state, AssistanceWorkState::Failed);
    assert_eq!(
        old.failure_code.as_deref(),
        Some("ASSISTANCE_LOCAL_READ_FAILED")
    );
    for _ in 0..3 {
        s.reconcile_assistance_work().unwrap();
        assert!(s.begin_assistance_work().unwrap().is_none());
    }
    conn.execute_batch("INSERT INTO sealed_blobs SELECT * FROM saved")
        .unwrap();
    s.retry_assistance_work(req(&i)).unwrap();
    let w = s.begin_assistance_work().unwrap().unwrap();
    s.complete_assistance_work(w.execute()).unwrap();
    assert_eq!(
        s.assistance_item_work(&i.item_id.to_string())
            .unwrap()
            .state,
        AssistanceWorkState::Ready
    );
}

#[test]
fn unknown_commit_reconciles_original_claim_without_reexecution_even_if_completion_is_lost() {
    for lose_completion in [false, true] {
        let (dir, mut s) = setup();
        let i = goal(&mut s);
        let conn = raw(dir.path());
        conn.execute_batch("CREATE TRIGGER fail_work_result BEFORE UPDATE OF result_status ON assistance_work WHEN NEW.result_status IS NOT NULL BEGIN SELECT RAISE(ABORT,'synthetic result failure'); END").unwrap();
        let w = s.begin_assistance_work().unwrap().unwrap();
        let completed = w.execute();
        if lose_completion {
            drop(completed)
        } else {
            assert!(s.complete_assistance_work(completed).is_err());
        }
        for _ in 0..3 {
            assert!(s.reconcile_assistance_work().is_err());
            assert!(s.begin_assistance_work().is_err());
            assert!(s.retry_assistance_work(req(&i)).is_err());
        }
        let before = s.assistance_item_work(&i.item_id.to_string()).unwrap();
        assert_eq!(before.state, AssistanceWorkState::Unknown);
        assert!(before.pending_revision.is_none());
        conn.execute_batch("DROP TRIGGER fail_work_result").unwrap();
        s.reconcile_assistance_work().unwrap();
        let after = s.assistance_item_work(&i.item_id.to_string()).unwrap();
        assert_eq!(after.state, AssistanceWorkState::Failed);
        assert_eq!(
            after.failure_code.as_deref(),
            Some("ASSISTANCE_LOCAL_COMMIT_FAILED")
        );
        assert!(s.begin_assistance_work().unwrap().is_none());
    }
}

#[test]
fn active_store_work_does_not_block_controls_or_audit_busy_every_tick() {
    let (_dir, mut s) = setup();
    goal(&mut s);
    let shared = s.store().unwrap();
    let work = s.begin_assistance_work().unwrap().unwrap();
    let guard = shared.lock().unwrap();
    let start = Instant::now();
    for _ in 0..3 {
        assert!(s.begin_assistance_work().unwrap().is_none());
        assert!(s.assistance_items().is_err());
    }
    assert!(s.assistance_work_status().unwrap().pending_items.is_none());
    s.clear_user_endpoint();
    let revoked = s.revoke_collect_consent().unwrap();
    assert!(!revoked.consent_granted);
    assert!(start.elapsed() < Duration::from_secs(1));
    drop(guard);
    drop(work);
    s.reconcile_assistance_work().unwrap();
    assert!(!s
        .audit()
        .unwrap()
        .entries
        .iter()
        .any(|e| e.action == "hitl.deny"));
}

#[test]
fn restart_recovers_local_claim_once_without_restoring_contexts_or_permissions() {
    let (dir, mut s) = setup();
    let i = goal(&mut s);
    let w = s.begin_assistance_work().unwrap().unwrap();
    drop(w);
    drop(s);
    let mut s = Session::open(dir.path());
    assert_eq!(
        s.assistance_item_work(&i.item_id.to_string())
            .unwrap()
            .state,
        AssistanceWorkState::Pending
    );
    assert!(!s.collect_status().consent_granted);
    let w = s.begin_assistance_work().unwrap().unwrap();
    s.complete_assistance_work(w.execute()).unwrap();
}

#[test]
fn graph_source_hooks_are_transactional_and_read_fault_never_invalidates_bundle() {
    let (dir, mut s) = setup();
    import(&mut s);
    let item = s.assistance_items().unwrap().items.remove(0);
    let AssistanceSource::Observation { bundle_id, .. } = item.source else {
        panic!("observation")
    };
    let detail = s.assistance_item(&item.item_id.to_string()).unwrap();
    let reference = detail.observation.unwrap();
    let event = reference
        .sources
        .iter()
        .flat_map(|s| &s.event_ids)
        .next()
        .copied()
        .unwrap();
    let conn = raw(dir.path());
    let shared = s.store().unwrap();
    let before_graph = s.people().unwrap();
    let before_audit = s.audit().unwrap().entries.len();
    use soul_store_api::EventStore;
    let blob = shared
        .lock()
        .unwrap()
        .get_event(event)
        .unwrap()
        .body_ref
        .unwrap()
        .blob_id
        .to_string();
    conn.execute_batch("CREATE TEMP TABLE saved AS SELECT * FROM sealed_blobs WHERE 0")
        .unwrap();
    conn.execute(
        "INSERT INTO saved SELECT * FROM sealed_blobs WHERE blob_id=?1",
        [&blob],
    )
    .unwrap();
    conn.execute("DELETE FROM sealed_blobs WHERE blob_id=?1", [&blob])
        .unwrap();
    assert!(s.local_insights().is_err());
    assert!(s
        .correct_tie(&reference.relationship_id.to_string(), "strong")
        .is_err());
    assert_eq!(s.people().unwrap(), before_graph);
    assert_eq!(s.audit().unwrap().entries.len(), before_audit);
    assert_eq!(
        shared
            .lock()
            .unwrap()
            .assistance_item(item.item_id)
            .unwrap(),
        item
    );
    assert_eq!(
        shared
            .lock()
            .unwrap()
            .local_insight(bundle_id)
            .unwrap()
            .validity,
        soul_store_api::insight::InsightValidity::Active
    );
    conn.execute_batch("INSERT INTO sealed_blobs SELECT * FROM saved")
        .unwrap();
    s.correct_tie(&reference.relationship_id.to_string(), "strong")
        .unwrap();
    let current = s.assistance_item(&item.item_id.to_string()).unwrap();
    assert_eq!(current.item.validity, AssistanceValidity::Stale);
    assert_ne!(current.item.revision, item.revision);
    assert_eq!(
        s.assistance_item_work(&item.item_id.to_string())
            .unwrap()
            .pending_revision,
        Some(current.item.revision)
    );
}

#[derive(Debug)]
struct NonCloneSource {
    samples: Arc<AtomicUsize>,
}
impl ForegroundSource for NonCloneSource {
    fn sample(&mut self) -> Result<Option<AppIdentity>, SourceError> {
        self.samples.fetch_add(1, Ordering::SeqCst);
        Ok(Some(AppIdentity::new("synthetic.exe")?))
    }
    fn describe(&self) -> &'static str {
        "synthetic.non_clone"
    }
}
fn factory(s: &mut Session, created: Arc<AtomicUsize>, samples: Arc<AtomicUsize>) {
    s.grant_collect_consent_with_factory(
        move || {
            created.fetch_add(1, Ordering::SeqCst);
            Ok(Box::new(NonCloneSource {
                samples: samples.clone(),
            }))
        },
        CollectorConfig::every(Duration::from_millis(10)),
    )
    .unwrap();
}
fn wait_samples(samples: &AtomicUsize) {
    let end = Instant::now() + Duration::from_secs(3);
    while samples.load(Ordering::SeqCst) == 0 && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(samples.load(Ordering::SeqCst) > 0);
    std::thread::sleep(Duration::from_millis(20));
}

#[test]
fn import_really_joins_flushes_and_resumes_a_new_nonclone_source_under_same_grant() {
    let (_dir, mut s) = setup();
    let created = Arc::new(AtomicUsize::new(0));
    let samples = Arc::new(AtomicUsize::new(0));
    factory(&mut s, created.clone(), samples.clone());
    wait_samples(&samples);
    let work = s
        .begin_import_prepare(
            text(),
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    assert!(
        s.assistance_work_status()
            .unwrap()
            .collector_paused_for_import
    );
    let completed = work.execute();
    let n = samples.load(Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(25));
    assert_eq!(samples.load(Ordering::SeqCst), n);
    let prepared = s.complete_import_prepare(completed).unwrap();
    assert_eq!(created.load(Ordering::SeqCst), 2);
    assert!(s.collect_status().collector_running);
    let work = s
        .begin_import(
            &prepared.operation.operation_id.to_string(),
            &prepared.operation.preview_id.to_string(),
        )
        .unwrap();
    s.complete_import(work.execute()).unwrap();
    assert_eq!(created.load(Ordering::SeqCst), 3);
    assert!(s.collect_status().collector_running);
    s.revoke_collect_consent().unwrap();
}

#[test]
fn revoke_and_dropped_completion_never_recreate_old_factory_and_one_shot_source_is_honestly_paused()
{
    let (_dir, mut s) = setup();
    let created = Arc::new(AtomicUsize::new(0));
    let samples = Arc::new(AtomicUsize::new(0));
    factory(&mut s, created.clone(), samples.clone());
    wait_samples(&samples);
    let work = s
        .begin_import_prepare(
            text(),
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    s.revoke_collect_consent().unwrap();
    assert!(s
        .grant_collect_consent_with_factory(
            || Ok(Box::new(NonCloneSource {
                samples: Arc::new(AtomicUsize::new(0))
            })),
            CollectorConfig::default()
        )
        .is_err());
    s.complete_import_prepare(work.execute()).unwrap();
    assert_eq!(created.load(Ordering::SeqCst), 1);
    assert!(!s.collect_status().collector_running);
    let held = s
        .import_recovery_state()
        .unwrap()
        .held_operation_id
        .unwrap();
    s.discard_import(&held.to_string()).unwrap();
    factory(&mut s, created.clone(), samples.clone());
    let w = s
        .begin_import_prepare(
            text(),
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    drop(w.execute());
    assert!(!s.collect_status().collector_running);
    assert!(
        s.assistance_work_status()
            .unwrap()
            .collector_paused_for_import
    );
    assert_eq!(created.load(Ordering::SeqCst), 2);
    s.revoke_collect_consent().unwrap();
    // The old injection API deliberately cannot recreate a consumed source.
    let (_dir, mut s) = setup();
    s.grant_collect_consent_with_source(
        NonCloneSource {
            samples: Arc::new(AtomicUsize::new(0)),
        },
        CollectorConfig::default(),
    )
    .unwrap();
    let w = s
        .begin_import_prepare(
            text(),
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    s.complete_import_prepare(w.execute()).unwrap();
    assert!(!s.collect_status().collector_running);
    assert!(
        s.assistance_work_status()
            .unwrap()
            .collector_paused_for_import
    );
    assert!(s.collect_status().notice.contains("一次性"));
    s.revoke_collect_consent().unwrap();
}

#[derive(Debug)]
struct BlockingSource {
    entered: Option<std::sync::mpsc::Sender<()>>,
    release: std::sync::mpsc::Receiver<()>,
    panic_after: bool,
}
impl ForegroundSource for BlockingSource {
    fn sample(&mut self) -> Result<Option<AppIdentity>, SourceError> {
        if let Some(entered) = self.entered.take() {
            entered.send(()).unwrap();
            self.release.recv_timeout(Duration::from_secs(5)).unwrap();
            if self.panic_after {
                panic!("synthetic collector thread failure")
            }
        }
        Ok(Some(AppIdentity::new("synthetic.exe")?))
    }
    fn describe(&self) -> &'static str {
        "synthetic.blocking"
    }
}

#[test]
fn dropping_unexecuted_import_keeps_slot_until_collector_really_joins() {
    let (_dir, mut s) = setup();
    let (entered_tx, entered) = std::sync::mpsc::channel();
    let (release, release_rx) = std::sync::mpsc::channel();
    s.grant_collect_consent_with_source(
        BlockingSource {
            entered: Some(entered_tx),
            release: release_rx,
            panic_after: false,
        },
        CollectorConfig::every(Duration::from_millis(1)),
    )
    .unwrap();
    entered.recv_timeout(Duration::from_secs(3)).unwrap();
    let work = s
        .begin_import_prepare(
            text(),
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    let dropped = std::thread::spawn(move || drop(work));
    std::thread::sleep(Duration::from_millis(20));
    assert!(s.assistance_items().is_err());
    assert!(s.begin_assistance_work().unwrap().is_none());
    assert!(s.store().is_none());
    s.revoke_collect_consent().unwrap();
    release.send(()).unwrap();
    dropped.join().unwrap();
    assert!(!s.assistance_work_status().unwrap().active_worker);
    assert!(!s.collect_status().consent_granted);
}

#[test]
fn failed_physical_collector_join_is_not_reported_as_successful_import_handoff() {
    let (_dir, mut s) = setup();
    let (entered_tx, entered) = std::sync::mpsc::channel();
    let (release, release_rx) = std::sync::mpsc::channel();
    s.grant_collect_consent_with_source(
        BlockingSource {
            entered: Some(entered_tx),
            release: release_rx,
            panic_after: true,
        },
        CollectorConfig::every(Duration::from_millis(1)),
    )
    .unwrap();
    entered.recv_timeout(Duration::from_secs(3)).unwrap();
    let work = s
        .begin_import_prepare(
            text(),
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    let worker = std::thread::spawn(move || work.execute());
    release.send(()).unwrap();
    let completion = worker.join().unwrap();
    assert_eq!(
        s.complete_import_prepare(completion)
            .unwrap_err()
            .reason_code,
        "IMPORT_COLLECTOR_HANDOFF_FAILED"
    );
    assert!(s.import_namespaces().unwrap().is_empty());
    assert!(s.recent_import_operations().unwrap().is_empty());
    assert!(!s.collect_status().collector_running);
    assert!(
        s.assistance_work_status()
            .unwrap()
            .collector_paused_for_import
    );
    s.revoke_collect_consent().unwrap();
}
