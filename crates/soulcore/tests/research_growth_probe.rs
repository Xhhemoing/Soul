//! Explicit synthetic observations, not Q2 or a performance threshold.
//! Each trial grows three independent encrypted databases without resetting them.
//! Operation timings exclude fixtures, assertions, output and cleanup. Save total
//! is the sum of timed phases (the preview assertion is deliberately excluded).

#![cfg(windows)]

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use serde_json::{json, Value};
use soul_collect::{Collector, ConsentHandle, FakeForegroundSource, COLLECTION_TOPIC};
use soul_store::research_retention::{ResearchDataset, ResearchScope};
use soul_store_api::{EventFilter, EventStore, GraphStore, ProfileStore};
use soulcore::commands::session::{ResearchSavePrepare, Session};
use soulcore::commands::store::database_path;

const START: u64 = 1_735_689_600; // 2025-01-01T00:00:00Z
const CHECKPOINTS: [usize; 3] = [100, 1000, 10000];

fn timed<T>(operation: impl FnOnce() -> T) -> (T, u64) {
    let start = Instant::now();
    let result = operation();
    (result, start.elapsed().as_nanos().try_into().unwrap())
}

fn emit(value: Value) {
    println!("SOUL_RESEARCH_GROWTH {}", value);
}

fn scope() -> ResearchScope {
    ResearchScope {
        from_day: "2025-01-01".into(),
        to_day: "2027-01-01".into(),
    }
}

// This bounded fixture spans January only; handle day/hour/minute carries and
// refuse an extension beyond that month instead of emitting an invalid date.
fn message_time(index: usize) -> String {
    assert!(index < 31 * 86400);
    format!(
        "2025-01-{:02}T{:02}:{:02}:{:02}Z",
        1 + index / 86400,
        index / 3600 % 24,
        index / 60 % 60,
        index % 60
    )
}

fn messages(from: usize, to: usize) -> String {
    let mut text = String::from("{\"type\":\"header\",\"format\":\"soul-import-v1\",\"version\":1,\"exported_at\":\"2025-02-01T00:00:00Z\"}\n");
    for index in from..to {
        let peer = index % 10;
        let outgoing = (index / 10) % 2 == 0;
        let row = json!({
            "type": "message", "id": format!("growth-message-{index}"),
            "occurred_at": message_time(index),
            "sender_scope": if outgoing { "self" } else { "third_party" },
            "conversation_id": format!("growth-conversation-{peer}"),
            "sender_id": if outgoing { "growth-self".to_owned() } else { format!("growth-peer-{peer}") },
            "text": "Fixed synthetic growth observation."
        });
        text.push_str(&serde_json::to_string(&row).unwrap());
        text.push('\n');
    }
    text
}

fn sizes(directory: &Path) -> Value {
    let database = database_path(directory);
    let bytes = |suffix: &str| {
        let mut path = database.as_os_str().to_os_string();
        path.push(suffix);
        match std::fs::metadata(Path::new(&path)) {
            Ok(metadata) => metadata.len(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
            Err(error) => panic!("synthetic database metadata failed: {error}"),
        }
    };
    let db = bytes("");
    let wal = bytes("-wal");
    let shm = bytes("-shm");
    json!({"db": db, "wal": wal, "shm": shm, "total": db + wal + shm})
}

fn assert_closed_collection(session: &Session) {
    assert!(session.status().store_opened);
    let state = session.collect_status();
    assert!(!state.consent_granted && !state.collector_running);
}

fn assert_import(session: &Session, count: usize) {
    let shared = session.store().unwrap();
    let store = shared.lock().unwrap();
    assert_eq!(store.list_events(&EventFilter::all()).unwrap().len(), count);
    assert_eq!(store.list_evidence().unwrap().len(), count);
    assert_eq!(store.list_contacts().unwrap().len(), 11);
    assert_eq!(store.list_relationships().unwrap().len(), 10);
}

fn import_trial(trial: usize) {
    let directory = tempfile::tempdir().unwrap();
    let mut session = Session::open(directory.path());
    assert_closed_collection(&session);
    let mut previous = 0;
    for checkpoint in CHECKPOINTS {
        let text = messages(previous, checkpoint);
        let (preview, preview_ns) = timed(|| session.preview_soul_import_v1(&text));
        let preview = preview.unwrap();
        assert_eq!(preview.messages, checkpoint - previous);
        assert!(!preview.writes_anything);
        let (receipt, commit_ns) = timed(|| session.commit_soul_import_v1(&text));
        let receipt = receipt.unwrap();
        assert_eq!(receipt.events_written, checkpoint - previous);
        assert_eq!(receipt.evidence_written, checkpoint - previous);
        assert_eq!(receipt.contacts_created, if previous == 0 { 11 } else { 0 });
        assert_eq!(receipt.ties_rebuilt, 10);
        let (people, people_ns) = timed(|| session.people());
        let people = people.unwrap();
        assert_eq!(people.people.len(), 11);
        assert_eq!(people.ties.len(), 10);
        for tie in &people.ties {
            assert_eq!(tie.interaction_count, (checkpoint / 10) as u64);
            assert_eq!(tie.outgoing_count, (checkpoint / 20) as u64);
            assert_eq!(tie.incoming_count, (checkpoint / 20) as u64);
            assert_eq!(tie.evidence.len(), checkpoint / 10);
        }
        assert_import(&session, checkpoint);
        let opened = sizes(directory.path());
        drop(session);
        let closed = sizes(directory.path());
        let (reopened, reopen_ns) = timed(|| Session::open(directory.path()));
        session = reopened;
        assert_closed_collection(&session);
        assert_import(&session, checkpoint);
        emit(
            json!({"record": "growth_import", "trial": trial, "shape": "import",
            "checkpoint": checkpoint, "delta": checkpoint - previous,
            "preview_ns": preview_ns, "commit_including_rebuild_ns": commit_ns,
            "people_query_ns": people_ns, "reopen_ns": reopen_ns,
            "events": checkpoint, "evidence": checkpoint, "contacts": 11, "ties": 10,
            "contacts_created_delta": receipt.contacts_created,
            "opened_bytes": opened, "closed_bytes": closed, "formal_q2": false}),
        );
        previous = checkpoint;
    }
}

fn assert_dataset(dataset: &ResearchDataset, count: usize, step: u64) {
    assert_eq!(dataset.counts.candidate_events, count as u64);
    assert_eq!(dataset.counts.included_events, count as u64);
    assert_eq!(dataset.counts.excluded_policy_events, 0);
    assert_eq!(dataset.counts.excluded_unavailable_events, 0);
    assert_eq!(dataset.counts.excluded_time_events, 0);
    assert_eq!(dataset.counts.out_of_range_events, 0);
    let per_hour = (3600 / step) as usize;
    assert_eq!(dataset.rows.len(), count.div_ceil(per_hour));
    for (index, row) in dataset.rows.iter().enumerate() {
        assert_eq!(
            row.aggregate_count,
            (count - index * per_hour).min(per_hour) as u64
        );
    }
}

fn append_activity(session: &Session, from: usize, to: usize, step: u64) -> Value {
    let shared = session.store().unwrap();
    let consent = ConsentHandle::closed();
    consent.grant(COLLECTION_TOPIC, START as i64);
    let source = FakeForegroundSource::showing("synthetic-a.exe").unwrap();
    let mut collector = Collector::new(source.clone(), Arc::clone(&shared), consent);
    // Script data is constructed before timing. A new collector starts at the
    // next unrecorded index; finish closes exactly the last event in this delta.
    let script: Vec<_> = (from + 1..to)
        .map(|index| {
            (
                if (index - from) % 2 == 0 {
                    "synthetic-a.exe"
                } else {
                    "synthetic-b.exe"
                },
                (START + index as u64 * step) * 1000,
            )
        })
        .collect();
    let start_at = (START + from as u64 * step) * 1000;
    let finish_at = (START + to as u64 * step) * 1000;
    let (written, write_ns) = timed(|| {
        collector.poll_once(start_at)?;
        for (app, timestamp) in &script {
            source.switch_to(app)?;
            collector.poll_once(*timestamp)?;
        }
        Ok::<(), soul_collect::CollectError>(())
    });
    written.unwrap();
    assert_eq!(collector.tally().events_written, (to - from - 1) as u64);
    let (finished, finish_ns) = timed(|| collector.finish(finish_at));
    assert!(finished.unwrap().is_some());
    assert_eq!(collector.tally().events_written, (to - from) as u64);
    drop(collector);
    assert_eq!(
        shared
            .lock()
            .unwrap()
            .list_events(&EventFilter::all())
            .unwrap()
            .len(),
        to
    );
    json!({"poll_switch_write_ns": write_ns, "finish_write_ns": finish_ns,
        "poll_written_events": to - from - 1, "finish_written_events": 1})
}

fn independent_read(path: &Path, count: usize, rows: usize) -> u64 {
    let python = std::env::var_os("SOUL_TEST_PYTHON").unwrap_or_else(|| "python".into());
    let script =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/read-research-retention.py");
    let mut command = std::process::Command::new(python);
    command.arg("-B").arg(script).arg(path);
    let (output, reader_ns) = timed(|| command.output());
    let output = output.unwrap();
    assert!(
        output.status.success(),
        "independent reader rejected archive"
    );
    let summary: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(summary["row_count"], rows);
    assert_eq!(summary["included_events"], count);
    assert_eq!(summary["candidate_events"], count);
    assert_eq!(summary["excluded_events"], 0);
    reader_ns
}

fn activity_trial(trial: usize, shape: &str, step: u64) {
    let directory = tempfile::tempdir().unwrap();
    let mut session = Session::open(directory.path());
    assert_closed_collection(&session);
    let mut previous = 0;
    for checkpoint in CHECKPOINTS {
        let writes = append_activity(&session, previous, checkpoint, step);
        let shared = session.store().unwrap();
        let query_scope = scope();
        let (query, query_ns) = timed(|| {
            shared
                .lock()
                .unwrap()
                .prepare_research_retention(&query_scope)
        });
        let query = query.unwrap();
        assert_dataset(query.dataset(), checkpoint, step);
        let expected = query.dataset().clone();
        drop(query);
        drop(shared);
        let destination = directory.path().join(format!("{checkpoint}.zip"));
        let request = ResearchSavePrepare {
            scope: query_scope,
            destination: destination.to_str().unwrap().into(),
        };
        let (preview, prepare_ns) = timed(|| session.prepare_research_save(request));
        let preview = preview.unwrap();
        assert!(!destination.exists(), "prepare must not create a file");
        assert_eq!(preview.dataset, expected);
        let (work, begin_ns) = timed(|| session.begin_research_save(&preview.request_id));
        let work = work.unwrap();
        let (completion, execute_ns) = timed(|| work.execute());
        let (receipt, complete_ns) = timed(|| session.complete_research_save(completion));
        let receipt = receipt.unwrap();
        assert!(receipt.written_to_disk && receipt.transaction_closed);
        assert_eq!(receipt.request_id, preview.request_id);
        assert_eq!(receipt.destination, destination.to_str().unwrap());
        assert_eq!(receipt.included_events, checkpoint as u64);
        assert_eq!(receipt.row_count, expected.rows.len());
        let zip_bytes = std::fs::metadata(&destination).unwrap().len();
        assert!(zip_bytes > 0 && zip_bytes <= 1024 * 1024);
        let opened = sizes(directory.path());
        // No store Arc or collector survives this point. Reader is a separate
        // process and is invoked before the original encrypted database reopens.
        drop(session);
        let closed = sizes(directory.path());
        let reader_ns = independent_read(&destination, checkpoint, expected.rows.len());
        let (reopened, reopen_ns) = timed(|| Session::open(directory.path()));
        session = reopened;
        assert_closed_collection(&session);
        {
            let shared = session.store().unwrap();
            let store = shared.lock().unwrap();
            assert_eq!(
                store.list_events(&EventFilter::all()).unwrap().len(),
                checkpoint
            );
            assert_eq!(
                store
                    .prepare_research_retention(&scope())
                    .unwrap()
                    .dataset(),
                &expected
            );
        }
        emit(
            json!({"record": "growth_activity_zip", "trial": trial, "shape": shape,
            "checkpoint": checkpoint, "delta": checkpoint - previous, "write": writes,
            "query_ns": query_ns, "prepare_query_encode_precheck_ns": prepare_ns,
            "begin_ns": begin_ns, "execute_encode_revalidate_write_publish_ns": execute_ns,
            "complete_ns": complete_ns,
            "save_operation_sum_ns": prepare_ns + begin_ns + execute_ns + complete_ns,
            "reader_process_ns": reader_ns, "reopen_ns": reopen_ns,
            "counts": expected.counts, "rows": expected.rows.len(), "zip_bytes": zip_bytes,
            "transaction_closed": receipt.transaction_closed,
            "opened_bytes": opened, "closed_bytes": closed, "formal_q2": false}),
        );
        previous = checkpoint;
    }
    // One bounded extension, separate from successful saves and their samples.
    if trial == 3 && step == 3600 {
        let writes = append_activity(&session, previous, 12000, step);
        let shared = session.store().unwrap();
        let scope = scope();
        let (query, query_ns) = timed(|| shared.lock().unwrap().prepare_research_retention(&scope));
        let query = query.unwrap();
        assert_dataset(query.dataset(), 12000, step);
        let counts = query.dataset().counts.clone();
        drop(query);
        drop(shared);
        let destination = directory.path().join("refused-12000.zip");
        let request = ResearchSavePrepare {
            scope,
            destination: destination.to_str().unwrap().into(),
        };
        let (result, prepare_ns) = timed(|| session.prepare_research_save(request));
        let refusal =
            result.expect_err("12000 sparse rows must exceed the existing complete ZIP cap");
        assert!(refusal.explanation.contains("1 MiB"));
        assert!(!destination.exists());
        let opened = sizes(directory.path());
        drop(session);
        let closed = sizes(directory.path());
        let (reopened, reopen_ns) = timed(|| Session::open(directory.path()));
        assert_closed_collection(&reopened);
        assert_eq!(
            reopened
                .store()
                .unwrap()
                .lock()
                .unwrap()
                .list_events(&EventFilter::all())
                .unwrap()
                .len(),
            12000
        );
        emit(
            json!({"record": "growth_zip_refusal", "trial": trial, "shape": shape,
            "checkpoint": 12000, "delta": 2000, "write": writes, "query_ns": query_ns,
            "prepare_refusal_ns": prepare_ns, "counts": counts, "rows": 12000,
            "target_exists": false, "successful_save": false, "reopen_ns": reopen_ns,
            "opened_bytes": opened, "closed_bytes": closed}),
        );
    }
}

#[test]
fn growth_fixture_carries_time_and_only_appends_delta() {
    assert_eq!(message_time(3600), "2025-01-01T01:00:00Z");
    assert_eq!(message_time(86400), "2025-01-02T00:00:00Z");
    let text = messages(100, 1000);
    let rows: Vec<Value> = text
        .lines()
        .skip(1)
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(rows.len(), 900);
    assert_eq!(rows.first().unwrap()["id"], "growth-message-100");
    assert_eq!(rows.last().unwrap()["id"], "growth-message-999");
}

#[test]
#[ignore = "explicit Windows synthetic growth and real ZIP diagnosis; not Q2"]
fn cumulative_growth_and_zip_costs() {
    let trials = std::env::var("SOUL_RESEARCH_TRIALS")
        .map(|value| {
            value
                .parse::<usize>()
                .expect("SOUL_RESEARCH_TRIALS must be 1 or 3")
        })
        .unwrap_or(3);
    assert!(matches!(trials, 1 | 3));
    for trial in 1..=trials {
        import_trial(trial);
        activity_trial(trial, "sparse_hour", 3600);
        activity_trial(trial, "dense_minute", 60);
    }
}
