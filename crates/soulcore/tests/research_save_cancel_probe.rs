//! Explicit synthetic cancellation under a synchronized real query load.
//! Request timestamps precede public method calls, not internal cancellation.
//! No assertion treats audit/store waiting as the cancellation flag's latency.

#![cfg(windows)]

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::{mpsc, Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use soul_schema::common::{
    ActorSubject, Privacy, Purpose, ResearchDisposition, SchemaVersion, SealedSubject, Subject,
    Timestamp,
};
use soul_schema::event::{EventKind, EventSource, SoulEvent};
use soul_store::research_retention::ResearchScope;
use soul_store_api::types::{SealRequest, StoreError};
use soul_store_api::{BlobStore, EventFilter, EventStore};
use soulcore::commands::session::{ResearchSavePrepare, Session};
use uuid::Uuid;

// A harness failure deadline, not a performance acceptance threshold.
const DEADLINE: Duration = Duration::from_secs(120);

fn stamp(epoch: Instant) -> u64 {
    epoch.elapsed().as_nanos().try_into().unwrap()
}

fn scope() -> ResearchScope {
    ResearchScope {
        from_day: "2025-01-01".into(),
        to_day: "2025-02-01".into(),
    }
}

fn seed(session: &Session, count: usize) {
    assert!(count <= 10000);
    let shared = session.store().unwrap();
    let key = Uuid::now_v7();
    shared
        .lock()
        .unwrap()
        .transact(|store| {
            for index in 0..count {
                let id = Uuid::now_v7();
                let body = store.seal(SealRequest::new(
                    key,
                    id,
                    "body_ref",
                    SealedSubject::Owner,
                    b"synthetic cancellation measurement".to_vec(),
                ))?;
                let mut privacy = Privacy::local_only(Subject::Owner, vec![Purpose::Research]);
                privacy.egress.research_export = ResearchDisposition::Bucket;
                store.append_event(SoulEvent {
                    schema_version: SchemaVersion,
                    event_id: id,
                    ts: Timestamp::new(format!(
                        "2025-01-{:02}T{:02}:{:02}:00Z",
                        1 + index / 1440,
                        index / 60 % 24,
                        index % 60
                    )),
                    source: EventSource::CollectorForegroundApp,
                    kind: EventKind::AppForeground,
                    actor_subject: ActorSubject::Owner,
                    consent_id: None,
                    privacy,
                    body_ref: Some(body),
                })?;
            }
            Ok::<(), StoreError>(())
        })
        .unwrap();
}

fn terminate_reader(child: &mut std::process::Child) -> std::process::ExitStatus {
    // kill may race with natural exit. try_wait both observes and reaps exit;
    // even cleanup never turns a failed kill into an unbounded wait.
    let _ = child.kill();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status,
            Ok(None) if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(5));
            }
            _ => panic!("failed to terminate and reap independent reader within cleanup deadline"),
        }
    }
}

fn independent_read(path: &Path, count: usize, rows: usize) {
    let python = std::env::var_os("SOUL_TEST_PYTHON").unwrap_or_else(|| "python".into());
    let script =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/read-research-retention.py");
    // Files avoid a full pipe blocking the child or a descendant retaining a
    // pipe after exit. Only bounded JSON is read, never stderr/body text.
    let mut stdout = tempfile::tempfile().unwrap();
    let stderr = tempfile::tempfile().unwrap();
    let started = Instant::now();
    let mut child = std::process::Command::new(python)
        .arg("-B")
        .arg(script)
        .arg(path)
        .stdout(stdout.try_clone().unwrap())
        .stderr(stderr.try_clone().unwrap())
        .spawn()
        .unwrap();
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < DEADLINE => {
                thread::sleep(Duration::from_millis(5));
            }
            Ok(None) => {
                timed_out = true;
                break terminate_reader(&mut child);
            }
            Err(_) => {
                terminate_reader(&mut child);
                panic!("failed to inspect independent reader process");
            }
        }
    };
    let stdout_bytes = stdout.metadata().unwrap().len();
    let stderr_bytes = stderr.metadata().unwrap().len();
    println!(
        "RESEARCH_SAVE_CANCEL_JSON {}",
        json!({"record": "independent_reader", "history": count, "rows": rows,
            "timed_out": timed_out, "exit_code": status.code(),
            "elapsed_ns": stamp(started), "stdout_bytes": stdout_bytes,
            "stderr_bytes": stderr_bytes})
    );
    assert!(!timed_out, "independent reader exceeded harness deadline");
    assert!(status.success(), "independent reader rejected ZIP");
    assert!(stdout_bytes <= 65536, "reader JSON exceeded bounded output");
    stdout.seek(SeekFrom::Start(0)).unwrap();
    let mut output = Vec::new();
    stdout.take(65536).read_to_end(&mut output).unwrap();
    let value: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(value["row_count"], rows);
    assert_eq!(value["included_events"], count);
    assert_eq!(value["candidate_events"], count);
    assert_eq!(value["excluded_events"], 0);
}

fn join_finished(worker: thread::JoinHandle<()>) {
    let deadline = Instant::now() + DEADLINE;
    while !worker.is_finished() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(2));
    }
    assert!(worker.is_finished(), "cancellation probe worker timed out");
    worker.join().unwrap();
}

fn observe(history: usize, trial: usize) -> (bool, bool) {
    let directory = tempfile::tempdir().unwrap();
    let mut session = Session::open(directory.path());
    assert!(session.status().store_opened);
    assert!(!session.collect_status().collector_running);
    seed(&session, history);
    let destination = directory.path().join("cancel-probe.zip");
    let preview = session
        .prepare_research_save(ResearchSavePrepare {
            scope: scope(),
            destination: destination.to_str().unwrap().into(),
        })
        .unwrap();
    let rows = history.div_ceil(60);
    assert_eq!(preview.dataset.rows.len(), rows);
    assert_eq!(preview.dataset.counts.included_events, history as u64);
    assert!(!destination.exists());
    let work = session.begin_research_save(&preview.request_id).unwrap();
    let shared = session.store().unwrap();
    let request_id = preview.request_id;
    let epoch = Instant::now();
    println!(
        "RESEARCH_SAVE_CANCEL_JSON {}",
        json!({"record": "attempt", "history": history, "trial": trial})
    );

    // The query holds the store before releasing both requesters. It waits only
    // for their announcements, then performs a real query without sleeping.
    // An announcement proves request scheduling, not entry into a private phase.
    let barrier = Arc::new(Barrier::new(3));
    let (announced_tx, announced_rx) = mpsc::channel();
    let (query_tx, query_rx) = mpsc::channel();
    let (execute_tx, execute_rx) = mpsc::channel();
    let (cancel_tx, cancel_rx) = mpsc::channel();
    let query_barrier = Arc::clone(&barrier);
    let query_store = Arc::clone(&shared);
    let query_worker = thread::spawn(move || {
        let lock_requested = stamp(epoch);
        let guard = query_store.lock().unwrap();
        let acquired = stamp(epoch);
        query_barrier.wait();
        announced_rx.recv_timeout(DEADLINE).unwrap();
        announced_rx.recv_timeout(DEADLINE).unwrap();
        let started = stamp(epoch);
        let result = guard.prepare_research_retention(&scope()).unwrap();
        let finished = stamp(epoch);
        let included = result.dataset().counts.included_events;
        let observed_rows = result.dataset().rows.len();
        drop(result);
        drop(guard);
        let released = stamp(epoch);
        query_tx
            .send((
                lock_requested,
                acquired,
                started,
                finished,
                released,
                included,
                observed_rows,
            ))
            .unwrap();
    });
    let execute_barrier = Arc::clone(&barrier);
    let execute_announced = announced_tx.clone();
    let execute_worker = thread::spawn(move || {
        execute_barrier.wait();
        let requested = stamp(epoch);
        execute_announced.send(()).unwrap();
        let completion = work.execute();
        let finished = stamp(epoch);
        execute_tx.send((requested, finished, completion)).unwrap();
    });
    let cancel_worker = thread::spawn(move || {
        barrier.wait();
        let requested = stamp(epoch);
        announced_tx.send(()).unwrap();
        let outcome = session.cancel_research_save(&request_id).unwrap();
        let returned = stamp(epoch);
        cancel_tx
            .send((requested, returned, outcome, session))
            .unwrap();
    });

    let (
        lock_requested,
        acquired,
        query_started,
        query_finished,
        released,
        included,
        observed_rows,
    ) = query_rx.recv_timeout(DEADLINE).expect("query completion");
    let (execute_request, execute_end, completion) = execute_rx
        .recv_timeout(DEADLINE)
        .expect("save worker completion");
    let (cancel_request, cancel_return, outcome, mut session) =
        cancel_rx.recv_timeout(DEADLINE).expect("cancel completion");
    join_finished(query_worker);
    join_finished(execute_worker);
    join_finished(cancel_worker);
    let complete_start = stamp(epoch);
    let result = session.complete_research_save(completion);
    let complete_end = stamp(epoch);
    let announcement_overlap = acquired <= cancel_request && cancel_request < query_finished;
    let execute_announcement_overlap =
        acquired <= execute_request && execute_request < query_finished;
    let target_exists = destination.exists();
    let zip_bytes = if target_exists {
        std::fs::metadata(&destination).unwrap().len()
    } else {
        0
    };
    let query_observation = json!({
        "lock_request_ns": lock_requested, "lock_acquired_ns": acquired,
        "started_ns": query_started, "finished_ns": query_finished,
        "release_observed_ns": released, "work_ns": query_finished - query_started,
        "included_events": included, "rows": observed_rows
    });
    let cancel_observation = json!({
        "request_announcement_ns": cancel_request, "return_ns": cancel_return,
        "announcement_to_return_ns": cancel_return - cancel_request,
        "request_announcement_overlap": announcement_overlap,
        "internal_method_entry_measured": false, "actual_store_wait_measured": false
    });
    let execute_observation = json!({
        "request_announcement_ns": execute_request, "end_ns": execute_end,
        "announcement_to_return_ns": execute_end - execute_request,
        "request_announcement_overlap": execute_announcement_overlap,
        "internal_method_entry_measured": false, "actual_store_wait_measured": false
    });
    let completion_observation = json!({
        "start_ns": complete_start, "end_ns": complete_end,
        "call_ns": complete_end - complete_start, "success": result.is_ok()
    });
    let outcome_observation = json!({
        "cancelled": outcome.cancelled, "already_published": outcome.already_published,
        "target_exists": target_exists, "zip_bytes": zip_bytes
    });
    println!(
        "RESEARCH_SAVE_CANCEL_JSON {}",
        json!({"record": "sample", "history": history, "trial": trial,
            "mode": "synchronized_query_request_announcements", "formal_q2": false,
            "query": query_observation, "cancel": cancel_observation,
            "execute": execute_observation, "completion": completion_observation,
            "outcome": outcome_observation})
    );
    assert_eq!(included, history as u64);
    assert_eq!(observed_rows, rows);
    assert!(outcome.cancelled ^ outcome.already_published);
    if outcome.cancelled {
        assert!(
            result.is_err(),
            "cancelled work must not report publication"
        );
        assert!(!target_exists, "cancelled work must not leave a ZIP");
    } else {
        let receipt = result.expect("published work must preserve its receipt");
        assert!(receipt.written_to_disk && receipt.transaction_closed);
        assert_eq!(receipt.row_count, rows);
        assert_eq!(receipt.included_events, history as u64);
        assert!(target_exists && zip_bytes > 0);
    }
    {
        let guard = shared.lock().unwrap();
        assert_eq!(
            guard.list_events(&EventFilter::all()).unwrap().len(),
            history
        );
        assert!(guard.verify_audit_chain().is_ok());
    }
    drop(shared);
    drop(session);
    if outcome.already_published {
        independent_read(&destination, history, rows);
    }
    (announcement_overlap, outcome.cancelled)
}

#[test]
#[ignore = "explicit Windows synthetic save cancellation under query load; not Q2"]
fn bounded_save_cancel_under_query_pressure() {
    let mut total_announcement_overlap = 0;
    let mut total_cancelled = 0;
    for history in [100, 1000, 10000] {
        let mut announcement_overlap = 0;
        let mut cancelled = 0;
        for trial in 1..=3 {
            let observed = observe(history, trial);
            announcement_overlap += usize::from(observed.0);
            cancelled += usize::from(observed.1);
        }
        println!(
            "RESEARCH_SAVE_CANCEL_JSON {}",
            json!({"record": "scale_summary", "history": history, "attempts": 3,
                "completed": 3, "request_announcement_overlap": announcement_overlap,
                "no_request_announcement_overlap": 3 - announcement_overlap,
                "cancelled": cancelled, "already_published": 3 - cancelled})
        );
        total_announcement_overlap += announcement_overlap;
        total_cancelled += cancelled;
    }
    println!(
        "RESEARCH_SAVE_CANCEL_JSON {}",
        json!({"record": "summary", "attempts": 9, "completed": 9,
            "request_announcement_overlap": total_announcement_overlap,
            "no_request_announcement_overlap": 9 - total_announcement_overlap,
            "cancelled": total_cancelled, "already_published": 9 - total_cancelled})
    );
    assert!(total_cancelled > 0, "no cancellation outcome was observed");
    assert!(
        total_announcement_overlap > 0,
        "no request announcement overlapped the query lock interval"
    );
}
