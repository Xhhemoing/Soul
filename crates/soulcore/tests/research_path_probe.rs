//! Bounded dirty-candidate engineering observations, not formal Q2 acceptance.
//! Only scripted sources and fresh encrypted temporary databases are used.
//! Timings exclude fixture generation, opening, assertions, output and cleanup.
//! Revalidation uses a no-op publisher: it does not measure ZIP or file writes.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use soul_collect::{Collector, ConsentHandle, FakeForegroundSource, COLLECTION_TOPIC};
use soul_store::research_retention::ResearchScope;
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::{EventFilter, EventStore, ForgetOps, ForgetUnit};
use soulcore::commands::collect::CollectorConfig;
use soulcore::commands::session::Session;

const START: u64 = 1_735_689_600; // 2025-01-01T00:00:00Z

fn timed<T>(operation: impl FnOnce() -> T) -> (T, u64) {
    let start = Instant::now();
    let result = operation();
    (result, start.elapsed().as_nanos().try_into().unwrap())
}

fn messages(count: usize) -> String {
    let mut text = String::from("{\"type\":\"header\",\"format\":\"soul-import-v1\",\"version\":1,\"exported_at\":\"2025-01-02T00:00:00Z\"}\n");
    for index in 0..count {
        let peer = index % 10;
        let outgoing = (index / 10) % 2 == 0;
        let row = json!({
            "type": "message", "id": format!("probe-message-{index}"),
            "occurred_at": format!("2025-01-01T00:{:02}:{:02}Z", index / 60, index % 60),
            "sender_scope": if outgoing { "self" } else { "third_party" },
            "conversation_id": format!("probe-conversation-{peer}"),
            "sender_id": if outgoing { "probe-self".to_owned() } else { format!("probe-peer-{peer}") },
            "text": "Fixed synthetic research path observation."
        });
        text.push_str(&serde_json::to_string(&row).unwrap());
        text.push('\n');
    }
    text
}

fn observe_import(count: usize) -> Value {
    let text = messages(count);
    let directory = tempfile::tempdir().unwrap();
    let mut session = Session::open(directory.path());
    assert!(session.status().store_opened);
    let (preview, preview_ns) = timed(|| session.preview_soul_import_v1(&text));
    let (receipt, commit_ns) = timed(|| session.commit_soul_import_v1(&text));
    let (people, people_ns) = timed(|| session.people());
    let preview = preview.unwrap();
    let receipt = receipt.unwrap();
    let people = people.unwrap();
    assert_eq!(preview.messages, count);
    assert_eq!(preview.participants, 11);
    assert_eq!(preview.conversations, 10);
    assert!(!preview.writes_anything);
    assert_eq!(receipt.events_written, count);
    assert_eq!(receipt.evidence_written, count);
    assert_eq!(receipt.contacts_created, 11);
    assert_eq!(receipt.ties_rebuilt, 10);
    assert_eq!(people.people.len(), 11);
    assert_eq!(people.ties.len(), 10);
    for tie in &people.ties {
        assert_eq!(tie.interaction_count, (count / 10) as u64);
        assert_eq!(tie.outgoing_count, (count / 20) as u64);
        assert_eq!(tie.incoming_count, (count / 20) as u64);
        assert_eq!(tie.evidence.len(), count / 10);
        assert!(tie.local_only);
    }
    let audit = session.audit().unwrap();
    assert!(audit.verified);
    assert!(audit
        .entries
        .iter()
        .any(|entry| entry.action == "import.commit"));
    assert!(audit
        .entries
        .iter()
        .any(|entry| entry.action == "inference.write"));
    json!({"preview_ns": preview_ns, "commit_including_rebuild_ns": commit_ns,
        "people_query_ns": people_ns, "messages": count, "peers": 10,
        "events_written": receipt.events_written, "evidence_written": receipt.evidence_written,
        "ties_rebuilt": receipt.ties_rebuilt, "audit_verified": audit.verified})
}

fn observe_activity(count: usize) -> Value {
    let directory = tempfile::tempdir().unwrap();
    let keys = TestKeyProvider::from_seed("bounded synthetic activity probe");
    let store = Arc::new(Mutex::new(
        SqlCipherStore::open(directory.path().join("probe.db"), &keys).unwrap(),
    ));
    let consent = ConsentHandle::closed();
    consent.grant(COLLECTION_TOPIC, START as i64);
    let source = FakeForegroundSource::showing("synthetic-a.exe").unwrap();
    let mut collector = Collector::new(source.clone(), Arc::clone(&store), consent);
    let content_key = collector.content_key_id();
    let (written, write_ns) = timed(|| {
        collector.poll_once(START * 1000)?;
        for index in 1..count {
            source
                .switch_to(if index % 2 == 0 {
                    "synthetic-a.exe"
                } else {
                    "synthetic-b.exe"
                })
                .unwrap();
            collector.poll_once((START + index as u64 * 3600) * 1000)?;
        }
        Ok::<(), soul_collect::CollectError>(())
    });
    let (finished, finish_ns) = timed(|| collector.finish((START + count as u64 * 3600) * 1000));
    written.unwrap();
    assert!(finished.unwrap().is_some());
    drop(collector);
    let mut store = store.lock().unwrap();
    assert_eq!(store.list_events(&EventFilter::all()).unwrap().len(), count);
    let scope = ResearchScope {
        from_day: "2025-01-01".into(),
        to_day: "2025-03-01".into(),
    };
    let (held, query_ns) = timed(|| store.prepare_research_retention(&scope));
    let held = held.unwrap();
    assert_eq!(held.dataset().counts.included_events, count as u64);
    assert_eq!(held.dataset().rows.len(), count);
    assert!(held
        .dataset()
        .rows
        .iter()
        .all(|row| row.aggregate_count == 1));
    let (publication, revalidation_ns) =
        timed(|| store.publish_research_retention(&held, || Ok(())));
    assert!(publication.unwrap().transaction_closed);
    let (forgotten, forget_ns) =
        timed(|| store.execute_forget(ForgetUnit::ContentKey(content_key)));
    forgotten.unwrap();
    let mut called = false;
    let (stale, stale_refusal_ns) = timed(|| {
        store.publish_research_retention(&held, || {
            called = true;
            Ok(())
        })
    });
    assert!(stale.is_err());
    assert!(!called);
    let (after, after_forget_query_ns) = timed(|| store.prepare_research_retention(&scope));
    let after = after.unwrap();
    assert!(after.dataset().rows.is_empty());
    assert_eq!(after.dataset().counts.included_events, 0);
    assert_eq!(
        after.dataset().counts.excluded_unavailable_events,
        count as u64
    );
    store.verify_audit_chain().unwrap();
    json!({"collector_poll_and_write_ns": write_ns, "collector_finish_ns": finish_ns,
        "retention_prepare_ns": query_ns, "revalidation_noop_publication_transaction_ns": revalidation_ns,
        "forget_ns": forget_ns, "stale_refusal_ns": stale_refusal_ns,
        "after_forget_query_ns": after_forget_query_ns, "events": count, "hour_buckets": count,
        "after_forget_included": 0, "after_forget_unavailable": after.dataset().counts.excluded_unavailable_events,
        "stale_publisher_called": called, "file_publication_measured": false})
}

fn observe_revoke() -> Value {
    let directory = tempfile::tempdir().unwrap();
    let mut session = Session::open(directory.path());
    let source = FakeForegroundSource::showing("synthetic-runner.exe").unwrap();
    session
        .grant_collect_consent_with_source(
            source.clone(),
            CollectorConfig::every(Duration::from_millis(20)),
        )
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while source.samples_taken() == 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(
        source.samples_taken() > 0,
        "scripted runner must actually sample"
    );
    let (status, revoke_ns) = timed(|| session.revoke_collect_consent());
    let status = status.unwrap();
    assert!(!status.consent_granted && !status.collector_running);
    let store = session.store().unwrap();
    let event_ids = || {
        store
            .lock()
            .unwrap()
            .list_events(&EventFilter::all())
            .unwrap()
            .into_iter()
            .map(|event| event.event_id)
            .collect::<std::collections::BTreeSet<_>>()
    };
    // This runner may have no completed event: the assertion proves no new
    // persisted event after stopping, not historical collection persistence.
    let stopped_events = event_ids();
    let samples = source.samples_taken();
    for index in 0..5 {
        source
            .switch_to(if index % 2 == 0 {
                "synthetic-a.exe"
            } else {
                "synthetic-b.exe"
            })
            .unwrap();
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(source.samples_taken(), samples);
    assert_eq!(event_ids(), stopped_events);
    assert_eq!(
        session.collect_status().events_collected,
        status.events_collected
    );
    json!({"revoke_call_to_return_ns": revoke_ns, "events_after_stop": status.events_collected,
        "samples_after_stop": samples, "source": "fake.scripted_desktop"})
}

#[test]
#[ignore = "explicit bounded synthetic dirty-candidate diagnosis; not formal Q2"]
fn bounded_research_paths() {
    // One small warmup, excluded from reported samples; all trials use fresh databases.
    observe_import(100);
    observe_activity(100);
    observe_revoke();
    for count in [100, 1000] {
        for trial in 1..=3 {
            let import = observe_import(count);
            let activity = observe_activity(count);
            let revoke = observe_revoke();
            println!(
                "\n{}",
                json!({"record": "research_path_sample", "trial": trial,
                "scale": count, "import": import, "activity": activity, "runner": revoke,
                "fresh_independent_databases": true, "formal_q2": false})
            );
        }
    }
}
