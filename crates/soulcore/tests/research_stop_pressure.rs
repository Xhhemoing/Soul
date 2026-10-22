//! Explicit synthetic synchronized-query contention, not natural stop traffic,
//! ZIP publication, formal Q2, or a measurement of the internal consent instant.
//! All timestamps are nanoseconds on one trial-local monotonic clock.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use soul_collect::{CollectorConfig, FakeForegroundSource, STOP_BUDGET};
use soul_schema::common::{
    ActorSubject, Privacy, Purpose, ResearchDisposition, SchemaVersion, SealedSubject, Subject,
    Timestamp,
};
use soul_schema::event::{EventKind, EventSource, SoulEvent};
use soul_store::research_retention::ResearchScope;
use soul_store::SqlCipherStore;
use soul_store_api::types::{SealRequest, StoreResult};
use soul_store_api::{BlobStore, EventFilter, EventStore};
use soulcore::commands::session::Session;
use uuid::Uuid;

const POLL: Duration = Duration::from_millis(20);
// Harness failure deadlines, not performance acceptance thresholds.
const DEADLINE: Duration = Duration::from_secs(120);
const POST_CYCLES: usize = 8;

fn stamp(epoch: Instant) -> u64 {
    epoch.elapsed().as_nanos().try_into().unwrap()
}

fn append_synthetic(store: &mut SqlCipherStore, key: Uuid) -> StoreResult<()> {
    let id = Uuid::now_v7();
    let sealed = store.seal(SealRequest::new(
        key,
        id,
        "body_ref",
        SealedSubject::Owner,
        b"fixed synthetic pressure fixture".to_vec(),
    ))?;
    let mut privacy = Privacy::local_only(Subject::Owner, vec![Purpose::Research]);
    privacy.egress.research_export = ResearchDisposition::Bucket;
    store.append_event(SoulEvent {
        schema_version: SchemaVersion,
        event_id: id,
        ts: Timestamp::new("2025-01-01T12:00:00Z"),
        source: EventSource::CollectorForegroundApp,
        kind: EventKind::AppForeground,
        actor_subject: ActorSubject::Owner,
        consent_id: None,
        privacy,
        body_ref: Some(sealed),
    })?;
    Ok(())
}

fn runner_ids(store: &SqlCipherStore, key: Uuid) -> BTreeSet<Uuid> {
    store
        .list_events(&EventFilter::all())
        .unwrap()
        .into_iter()
        .filter(|event| {
            event
                .body_ref
                .as_ref()
                .is_some_and(|body| body.content_key_id == key)
        })
        .map(|event| event.event_id)
        .collect()
}

struct Operation {
    kind: &'static str,
    mode: &'static str,
    request: u64,
    acquired: u64,
    work_started: u64,
    work_finished: u64,
    released: u64,
    included: u64,
}

impl Operation {
    fn value(&self, index: usize, stop_request: u64, stop_return: u64) -> Value {
        json!({"operation": index, "kind": self.kind, "mode": self.mode,
            "lock_request_ns": self.request, "lock_acquired_ns": self.acquired,
            "work_started_ns": self.work_started, "work_finished_ns": self.work_finished,
            "lock_release_observed_ns": self.released,
            "lock_wait_ns": self.acquired - self.request,
            "lock_hold_observed_ns": self.released - self.acquired,
            "work_ns": self.work_finished - self.work_started,
            "held_at_stop_request": self.acquired <= stop_request && stop_request < self.work_finished,
            "started_after_stop_return": self.request >= stop_return,
            "included_events": self.included, "successful": true})
    }
}

// Sets the cancellation flag on all exits, including a harness assertion failure.
struct Cancel(Arc<AtomicBool>);
impl Drop for Cancel {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

fn join_finished<T>(worker: thread::JoinHandle<T>) -> T {
    let deadline = Instant::now() + DEADLINE;
    while !worker.is_finished() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(5));
    }
    // Never call an unbounded join on a worker that has not completed. A failed
    // harness detaches it; cancellation and channel disconnect release our waits.
    assert!(
        worker.is_finished(),
        "bounded pressure worker did not finish"
    );
    worker
        .join()
        .unwrap_or_else(|_| panic!("pressure worker panicked"))
}

fn trial(history: usize, trial_number: usize) {
    let directory = tempfile::tempdir().unwrap();
    let mut session = Session::open(directory.path());
    let store = session.store().expect("temporary encrypted Session store");
    let seed_key = Uuid::now_v7();
    let writer_key = Uuid::now_v7();
    store
        .lock()
        .unwrap()
        .transact(|store| {
            for _ in 0..history {
                append_synthetic(store, seed_key)?;
            }
            Ok::<(), soul_store_api::types::StoreError>(())
        })
        .unwrap();
    let source = FakeForegroundSource::showing("synthetic-a.exe").unwrap();
    session
        .grant_collect_consent_with_source(source.clone(), CollectorConfig::every(POLL))
        .unwrap();
    let setup_deadline = Instant::now() + Duration::from_secs(10);
    let mut turn = 0;
    let runner_key = loop {
        source
            .switch_to(if turn % 2 == 0 {
                "synthetic-a.exe"
            } else {
                "synthetic-b.exe"
            })
            .unwrap();
        turn += 1;
        let keys: BTreeSet<_> = store
            .lock()
            .unwrap()
            .list_events(&EventFilter::all())
            .unwrap()
            .into_iter()
            .filter_map(|event| event.body_ref.map(|body| body.content_key_id))
            .filter(|key| *key != seed_key)
            .collect();
        if let Some(key) = keys.first() {
            assert_eq!(keys.len(), 1, "one real runner CK");
            break *key;
        }
        assert!(
            Instant::now() < setup_deadline,
            "runner never completed an event"
        );
        thread::sleep(POLL);
    };
    assert!(source.samples_taken() > 0);
    assert_ne!(runner_key, writer_key);
    let initial_runner_events = runner_ids(&store.lock().unwrap(), runner_key).len();
    assert!(initial_runner_events > 0);

    println!(
        "RESEARCH_STOP_PRESSURE_JSON {}",
        json!({
            "record": "research_stop_pressure_attempt", "trial": trial_number,
            "history": history, "mode": "synchronized_query_contention"
        })
    );

    let epoch = Instant::now();
    let cancelled = Arc::new(AtomicBool::new(false));
    let _cancel_on_exit = Cancel(Arc::clone(&cancelled));
    let returned = Arc::new(AtomicBool::new(false));
    let (locked_tx, locked_rx) = mpsc::sync_channel(1);
    let (request_tx, request_rx) = mpsc::sync_channel(1);
    let (result_tx, result_rx) = mpsc::sync_channel(1);
    let load_store = Arc::clone(&store);
    let load_returned = Arc::clone(&returned);
    let load_cancelled = Arc::clone(&cancelled);
    let load_source = source.clone();
    let background = thread::spawn(move || {
        let mut operations = Vec::new();
        let mut request_snapshot = None;
        let mut post_cycles = 0;
        // A hard operation cap additionally bounds accidental failure to signal.
        for cycle in 0..128 {
            if load_cancelled.load(Ordering::SeqCst) || epoch.elapsed() > DEADLINE {
                break;
            }
            let post_return = load_returned.load(Ordering::SeqCst);
            load_source
                .switch_to(if cycle % 2 == 0 {
                    "synthetic-a.exe"
                } else {
                    "synthetic-b.exe"
                })
                .unwrap();
            for kind in ["sealed_write", "retention_query"] {
                let request = stamp(epoch);
                let mut guard = load_store.lock().unwrap();
                let acquired = stamp(epoch);
                let synchronized = cycle == 2 && kind == "retention_query";
                if synchronized {
                    // Inspect before notifying the stopper, so this full scan
                    // cannot inflate request-to-return. No writer can change
                    // these IDs before the handshake: this guard stays held.
                    request_snapshot = Some(runner_ids(&guard, runner_key));
                    locked_tx.send(()).unwrap();
                    request_rx
                        .recv_timeout(DEADLINE)
                        .expect("stop request handshake");
                }
                let work_started = stamp(epoch);
                let included = if kind == "sealed_write" {
                    append_synthetic(&mut guard, writer_key).unwrap();
                    0
                } else {
                    guard
                        .prepare_research_retention(&ResearchScope {
                            from_day: "2025-01-01".into(),
                            to_day: "2025-01-02".into(),
                        })
                        .unwrap()
                        .dataset()
                        .counts
                        .included_events
                };
                let work_finished = stamp(epoch);
                drop(guard);
                let released = stamp(epoch);
                if kind == "retention_query" {
                    assert!(included >= (history + cycle + 1) as u64);
                }
                operations.push(Operation {
                    kind,
                    mode: if synchronized {
                        "synchronized_query_contention"
                    } else {
                        "natural_background_loop"
                    },
                    request,
                    acquired,
                    work_started,
                    work_finished,
                    released,
                    included,
                });
            }
            // Simulates continued fake switching; never sleeps with store held.
            thread::sleep(POLL);
            if post_return {
                post_cycles += 1;
                if post_cycles == POST_CYCLES {
                    break;
                }
            }
        }
        (operations, request_snapshot, post_cycles)
    });
    let stop_source = source.clone();
    let stop_store = Arc::clone(&store);
    let stopper = thread::spawn(move || {
        locked_rx
            .recv_timeout(DEADLINE)
            .expect("query must hold store first");
        let request = stamp(epoch);
        request_tx.send(()).unwrap();
        let status = session.revoke_collect_consent().unwrap();
        let returned_at = stamp(epoch);
        let samples = stop_source.samples_taken();
        returned.store(true, Ordering::SeqCst);
        let ids = runner_ids(&stop_store.lock().unwrap(), runner_key);
        result_tx
            .send((session, status, request, returned_at, samples, ids))
            .unwrap();
    });
    let result = result_rx.recv_timeout(DEADLINE);
    if result.is_err() {
        cancelled.store(true, Ordering::SeqCst);
    }
    let (session, status, request, returned_at, samples_at_return, ids_at_return) =
        result.expect("stop must complete within the harness deadline");
    join_finished(stopper);
    let (operations, request_snapshot, post_cycles) = join_finished(background);
    let final_samples = source.samples_taken();
    let guard = store.lock().unwrap();
    let final_ids = runner_ids(&guard, runner_key);
    let writer_events = runner_ids(&guard, writer_key).len();
    let seed_events = runner_ids(&guard, seed_key).len();
    let audit_verified = guard.verify_audit_chain().is_ok();
    drop(guard);
    let sync_query = operations
        .iter()
        .find(|op| op.mode == "synchronized_query_contention")
        .unwrap();
    let overlap = sync_query.acquired <= request && request < sync_query.work_started;
    let writes = operations
        .iter()
        .filter(|op| op.kind == "sealed_write")
        .count();
    let queries = operations
        .iter()
        .filter(|op| op.kind == "retention_query")
        .count();
    let post_writes = operations
        .iter()
        .filter(|op| op.kind == "sealed_write" && op.request >= returned_at)
        .count();
    let post_queries = operations
        .iter()
        .filter(|op| op.kind == "retention_query" && op.request >= returned_at)
        .count();
    let load_start = operations.first().unwrap().request;
    let load_end = operations.last().unwrap().released;
    let mut sample = json!({
            "record": "research_stop_pressure_sample", "trial": trial_number, "history": history,
            "mode": "synchronized_query_contention", "attempts": 1,
            "completed": 1, "overlapped": usize::from(overlap), "nonoverlapped": usize::from(!overlap),
            "valid_for_synchronized_distribution": overlap,
            "request_ns": request, "return_ns": returned_at, "stop_latency_ns": returned_at - request,
            "query_lock_release_observed_ns": sync_query.released,
            "request_to_competing_release_ns": sync_query.released.saturating_sub(request),
            "competing_release_to_return_ns": i128::from(returned_at) - i128::from(sync_query.released),
            "actual_unlock_between_work_finished_and_release_observed": true,
            "intervals_are_not_internal_mutex_wait_or_audit_timings": true,
            "stop_budget_reference_ns": STOP_BUDGET.as_nanos() as u64,
            "call_return_within_reference": returned_at - request <= STOP_BUDGET.as_nanos() as u64,
            "consent_effective_instant_measured": false, "last_runner_append_instant_measured": false,
    });
    let observations = json!({
            "initial_runner_events": initial_runner_events, "runner_events_at_return": ids_at_return.len(),
            "runner_events_after_load": final_ids.len(), "runner_ids_unchanged": ids_at_return == final_ids,
            "runner_ids_unchanged_since_request_snapshot": request_snapshot.as_ref() == Some(&final_ids),
            "request_snapshot_not_consent_instant": true,
            "samples_at_return": samples_at_return, "samples_after_load": final_samples,
            "post_return_sample_delta": final_samples.saturating_sub(samples_at_return),
            "post_return_runner_event_delta": final_ids.difference(&ids_at_return).count(),
            "background_writes": writes, "background_queries": queries, "background_events": writer_events,
            "post_return_writes": post_writes, "post_return_queries": post_queries,
            "post_return_cycles": post_cycles, "load_observation_start_ns": load_start,
            "load_observation_end_ns": load_end, "load_observation_ns": load_end - load_start,
            "audit_verified": audit_verified, "formal_q2": false, "zip_publication_measured": false,
            "operations": operations.iter().enumerate().map(|(i, op)| op.value(i, request, returned_at)).collect::<Vec<_>>()
    });
    if let Value::Object(fields) = observations {
        sample.as_object_mut().unwrap().extend(fields);
    }
    println!("RESEARCH_STOP_PRESSURE_JSON {sample}");
    assert!(
        overlap,
        "non-overlapping attempts cannot enter the distribution"
    );
    assert!(!status.consent_granted && !status.collector_running);
    assert_eq!(post_cycles, POST_CYCLES);
    assert!(post_writes >= POST_CYCLES && post_queries >= POST_CYCLES);
    assert_eq!(writer_events, writes);
    assert_eq!(seed_events, history);
    assert!(queries > 0);
    assert_eq!(final_samples, samples_at_return);
    assert_eq!(final_ids, ids_at_return);
    assert!(audit_verified);
    assert!(!session.collect_status().collector_running);
}

#[test]
#[ignore = "explicit bounded synthetic synchronized-query stop experiment; not formal Q2"]
fn bounded_stop_under_query_and_write_pressure() {
    let trials = match std::env::var("SOUL_RESEARCH_TRIALS") {
        Ok(value) => value
            .parse::<usize>()
            .expect("SOUL_RESEARCH_TRIALS integer"),
        Err(std::env::VarError::NotPresent) => 3,
        Err(error) => panic!("SOUL_RESEARCH_TRIALS: {error}"),
    };
    assert!(
        (1..=3).contains(&trials),
        "bounded trial count must be 1..=3"
    );
    for history in [1_000, 10_000] {
        for trial_number in 1..=trials {
            trial(history, trial_number);
        }
    }
}
