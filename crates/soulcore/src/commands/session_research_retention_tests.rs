use super::*;
use soul_schema::common::{
    ActorSubject, Derivation, EgressPolicy, Privacy, Purpose, ResearchDisposition, Retention,
    SchemaVersion, SealedSubject, Subject, Timestamp,
};
use soul_schema::event::{EventKind, EventSource, SoulEvent};
use soul_store_api::types::SealRequest;
use soul_store_api::{BlobStore, EventStore, ForgetOps, ForgetUnit};

fn fixture() -> (tempfile::TempDir, Session, SoulEvent) {
    let dir = tempfile::tempdir().unwrap();
    let session = Session::open(dir.path().join("database"));
    assert!(session.status().store_opened);
    let id = Uuid::now_v7();
    let shared = session.opened_store().unwrap();
    let mut store = hold(&shared);
    let body = store
        .seal(SealRequest::new(
            Uuid::now_v7(),
            id,
            "body_ref",
            SealedSubject::Owner,
            b"SYNTHETIC_PRIVATE_APP_DURATION".to_vec(),
        ))
        .unwrap();
    let event = SoulEvent {
        schema_version: SchemaVersion,
        event_id: id,
        ts: Timestamp::new("2024-02-29T09:10:00Z"),
        source: EventSource::CollectorForegroundApp,
        kind: EventKind::AppForeground,
        actor_subject: ActorSubject::Owner,
        consent_id: None,
        privacy: Privacy {
            subject: Subject::Owner,
            derivation: Derivation::Raw,
            purposes: vec![Purpose::Research],
            retention: Retention::until_forgotten(),
            egress: EgressPolicy {
                research_export: ResearchDisposition::Bucket,
                ..EgressPolicy::default()
            },
        },
        body_ref: Some(body),
    };
    store.append_event(event.clone()).unwrap();
    drop(store);
    (dir, session, event)
}
fn request(dir: &Path, name: &str) -> ResearchSavePrepare {
    ResearchSavePrepare {
        scope: ResearchScope {
            from_day: "2024-02-29".into(),
            to_day: "2024-03-01".into(),
        },
        destination: dir.join(name).to_str().unwrap().into(),
    }
}
fn save(session: &mut Session, id: &str) -> Result<ResearchSaveReceipt, SessionRefusal> {
    let done = session.begin_research_save(id)?.execute();
    session.complete_research_save(done)
}

#[test]
fn prepare_is_memory_only_and_confirmation_is_one_use() {
    let (dir, mut session, event) = fixture();
    let view = session
        .prepare_research_save(request(dir.path(), "saved.zip"))
        .unwrap();
    assert!(!dir.path().join("saved.zip").exists());
    assert_eq!(view.dataset.rows.len(), 1);
    assert!(save(&mut session, &Uuid::now_v7().to_string()).is_err());
    let receipt = save(&mut session, &view.request_id).unwrap();
    assert!(receipt.written_to_disk && receipt.transaction_closed);
    assert_eq!(receipt.included_events, 1);
    assert!(save(&mut session, &view.request_id).is_err());
    let bytes = std::fs::read(dir.path().join("saved.zip")).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(!text.contains("SYNTHETIC_PRIVATE_APP_DURATION"));
    assert!(!text.contains(&event.event_id.to_string()));
    let audit = serde_json::to_string(&session.audit().unwrap()).unwrap();
    assert!(!audit.contains("saved.zip") && !audit.contains("2024-02-29T09"));
    assert!(audit.contains("CONSENT_GRANTED"));
}

#[test]
fn cancelling_prepared_or_started_work_never_publishes() {
    for started in [false, true] {
        let (dir, mut session, _) = fixture();
        let view = session
            .prepare_research_save(request(dir.path(), "cancel.zip"))
            .unwrap();
        let work = started.then(|| session.begin_research_save(&view.request_id).unwrap());
        assert!(
            session
                .cancel_research_save(&view.request_id)
                .unwrap()
                .cancelled
        );
        if let Some(work) = work {
            assert!(session.complete_research_save(work.execute()).is_err());
        } else {
            assert!(save(&mut session, &view.request_id).is_err());
        }
        assert!(!dir.path().join("cancel.zip").exists());
    }
}

#[test]
fn forget_after_confirmation_invalidates_work_but_saved_copy_is_independent() {
    for forget_first in [false, true] {
        let (dir, mut session, event) = fixture();
        let view = session
            .prepare_research_save(request(dir.path(), "copy.zip"))
            .unwrap();
        let work = session.begin_research_save(&view.request_id).unwrap();
        if forget_first {
            hold(&session.opened_store().unwrap())
                .execute_forget(ForgetUnit::ContentKey(
                    event.body_ref.as_ref().unwrap().content_key_id,
                ))
                .unwrap();
        }
        let result = session.complete_research_save(work.execute());
        assert_eq!(result.is_err(), forget_first);
        if !forget_first {
            hold(&session.opened_store().unwrap())
                .execute_forget(ForgetUnit::ContentKey(
                    event.body_ref.as_ref().unwrap().content_key_id,
                ))
                .unwrap();
            let cancel = session.cancel_research_save(&view.request_id).unwrap();
            assert!(!cancel.cancelled);
            assert!(dir.path().join("copy.zip").exists());
        } else {
            assert!(!dir.path().join("copy.zip").exists());
        }
    }
}

#[test]
fn replacement_failed_prepare_and_restart_do_not_reuse_approval() {
    let (dir, mut session, _) = fixture();
    let first = session
        .prepare_research_save(request(dir.path(), "first.zip"))
        .unwrap();
    let second = session
        .prepare_research_save(request(dir.path(), "second.zip"))
        .unwrap();
    assert!(save(&mut session, &first.request_id).is_err());
    let mut invalid = request(dir.path(), "bad.zip");
    invalid.scope.from_day = "not-a-day".into();
    assert!(session.prepare_research_save(invalid).is_err());
    assert!(save(&mut session, &second.request_id).is_err());
    let third = session
        .prepare_research_save(request(dir.path(), "third.zip"))
        .unwrap();
    drop(session);
    let mut session = Session::open(dir.path().join("database"));
    assert!(save(&mut session, &third.request_id).is_err());
    assert!(!dir.path().join("third.zip").exists());
}

#[test]
fn late_target_collision_preserves_file_and_consumes_approval() {
    let (dir, mut session, _) = fixture();
    let view = session
        .prepare_research_save(request(dir.path(), "existing.zip"))
        .unwrap();
    std::fs::write(dir.path().join("existing.zip"), b"prior synthetic file").unwrap();
    assert!(save(&mut session, &view.request_id).is_err());
    assert_eq!(
        std::fs::read(dir.path().join("existing.zip")).unwrap(),
        b"prior synthetic file"
    );
    assert!(save(&mut session, &view.request_id).is_err());
    let fresh = session
        .prepare_research_save(request(dir.path(), "retry.zip"))
        .unwrap();
    assert!(save(&mut session, &fresh.request_id).is_ok());
}

#[test]
fn cancel_reaches_worker_after_write_without_waiting_for_the_store_lock() {
    let (dir, mut session, _) = fixture();
    let view = session
        .prepare_research_save(request(dir.path(), "race.zip"))
        .unwrap();
    let work = session.begin_research_save(&view.request_id).unwrap();
    let control = Arc::clone(&work.control);
    let (ready, wait) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        work.execute_before_publish(|| {
            ready.send(()).unwrap();
            let deadline = Instant::now() + std::time::Duration::from_secs(5);
            while !control.cancelled.load(Ordering::Acquire) && Instant::now() < deadline {
                std::thread::yield_now();
            }
            assert!(
                control.cancelled.load(Ordering::Acquire),
                "cancel blocked behind the store"
            );
        })
    });
    wait.recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    assert!(
        session
            .cancel_research_save(&view.request_id)
            .unwrap()
            .cancelled
    );
    assert!(session
        .complete_research_save(worker.join().unwrap())
        .is_err());
    assert!(!dir.path().join("race.zip").exists());
}

#[test]
fn published_file_is_reported_even_if_cancel_arrives_before_completion() {
    let (dir, mut session, _) = fixture();
    let view = session
        .prepare_research_save(request(dir.path(), "published.zip"))
        .unwrap();
    let completion = session
        .begin_research_save(&view.request_id)
        .unwrap()
        .execute();
    let cancel = session.cancel_research_save(&view.request_id).unwrap();
    assert!(!cancel.cancelled && cancel.already_published);
    assert!(
        session
            .complete_research_save(completion)
            .unwrap()
            .written_to_disk
    );
    assert!(dir.path().join("published.zip").exists());
}

#[test]
fn stale_approval_and_changed_source_record_denial_without_prose() {
    let (dir, mut session, event) = fixture();
    let view = session
        .prepare_research_save(request(dir.path(), "denied.zip"))
        .unwrap();
    hold(&session.opened_store().unwrap())
        .execute_forget(ForgetUnit::ContentKey(
            event.body_ref.unwrap().content_key_id,
        ))
        .unwrap();
    let before = serde_json::to_string(&session.audit().unwrap())
        .unwrap()
        .matches("hitl.deny")
        .count();
    assert!(save(&mut session, &view.request_id).is_err());
    assert!(save(&mut session, &view.request_id).is_err());
    let audit = serde_json::to_string(&session.audit().unwrap()).unwrap();
    assert_eq!(audit.matches("hitl.deny").count(), before + 2);
    assert!(!audit.contains("denied.zip") && !audit.contains("2024-02-29T09"));
}

#[test]
fn dropping_the_session_cancels_an_owned_unpublished_worker() {
    let (dir, mut session, _) = fixture();
    let view = session
        .prepare_research_save(request(dir.path(), "drop.zip"))
        .unwrap();
    let work = session.begin_research_save(&view.request_id).unwrap();
    drop(session);
    assert!(work.execute().result.is_err());
    assert!(!dir.path().join("drop.zip").exists());
}

#[test]
#[ignore = "explicit synthetic cross-language check requires SOUL_TEST_PYTHON"]
fn saved_synthetic_bundle_is_read_without_session() {
    let python = std::env::var_os("SOUL_TEST_PYTHON").expect("explicit Python executable");
    let (dir, mut session, _) = fixture();
    let view = session
        .prepare_research_save(request(dir.path(), "independent.zip"))
        .unwrap();
    save(&mut session, &view.request_id).unwrap();
    drop(session);
    let output = std::process::Command::new(python)
        .arg("-B")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/read-research-retention.py"))
        .arg(dir.path().join("independent.zip"))
        .output()
        .unwrap();
    assert!(output.status.success(), "reader rejected synthetic archive");
    let summary: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(summary["row_count"], 1);
    assert_eq!(summary["included_events"], 1);
}
