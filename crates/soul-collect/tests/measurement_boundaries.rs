//! Measurement limits through the real encrypted store and sealed bodies.
//!
//! Scripted observations are not continuous desktop observation: event counts
//! are not active duration, and an unobserved interval is not zero activity.
//! Dropping a synchronous Collector below models loss of its open session, not
//! a process crash, OS sleep, or desktop acceptance test.

mod common;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use common::{foreground_events, granted_consent, open_shared, NOW};
use soul_collect::{Collector, FakeForegroundSource, SealedSessionBody};
use soul_store::SqlCipherStore;
use soul_store_api::BlobStore;

fn at(offset_ms: u64) -> u64 {
    NOW as u64 * 1_000 + offset_ms
}

/// Compare independently decoded stored values, never the Poll's report.
/// Sorting by app and duration avoids assuming order for equal-second events.
fn stored_sessions(store: &Arc<Mutex<SqlCipherStore>>) -> Vec<(String, u64)> {
    let events = foreground_events(store);
    let locked = store.lock().expect("store lock");
    let mut sessions: Vec<_> = events
        .iter()
        .map(|event| {
            let sealed = event.body_ref.as_ref().expect("sealed session body");
            let bytes = locked.open(sealed).expect("unseal stored session");
            let body = SealedSessionBody::from_json_bytes(&bytes).expect("decode session body");
            (body.app, body.duration_ms)
        })
        .collect();
    sessions.sort();
    sessions
}

#[test]
fn subsecond_sessions_survive_same_second_timestamps_and_observed_source_gaps() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let store = open_shared(dir.path().join("subsecond.db"));
    let source = FakeForegroundSource::showing("a.exe").expect("synthetic app");
    let mut collector = Collector::new(source.clone(), Arc::clone(&store), granted_consent());

    collector.poll_once(at(0)).expect("sample A");
    source.switch_to("b.exe").expect("synthetic app");
    collector.poll_once(at(125)).expect("sample B");
    source.clear();
    collector.poll_once(at(200)).expect("observe no source");
    collector
        .poll_once(at(500))
        .expect("observe no source again");
    source.switch_to("a.exe").expect("synthetic app");
    collector.poll_once(at(1_000)).expect("sample A again");
    collector.finish(at(1_100)).expect("finish A");

    assert_eq!(
        stored_sessions(&store),
        vec![
            ("a.exe".into(), 100),
            ("a.exe".into(), 125),
            ("b.exe".into(), 75),
        ],
        "the observed gap must not be charged to either adjacent session",
    );
    let events = foreground_events(&store);
    let ids: HashSet<_> = events.iter().map(|event| event.event_id).collect();
    assert_eq!(
        ids.len(),
        3,
        "same-second events retain distinct identities"
    );
    assert!(
        events.iter().any(|event| {
            events
                .iter()
                .any(|other| event.event_id != other.event_id && event.ts == other.ts)
        }),
        "two distinct stored sessions really share the second-resolution timestamp",
    );
}

#[test]
fn an_application_switch_between_samples_cannot_be_reconstructed() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let store = open_shared(dir.path().join("unobserved.db"));
    let source = FakeForegroundSource::showing("a.exe").expect("synthetic app");
    let mut collector = Collector::new(source.clone(), Arc::clone(&store), granted_consent());

    collector.poll_once(at(0)).expect("sample A");
    // B really appears in the scripted source, but no poll observes it.
    source.switch_to("b.exe").expect("synthetic app");
    source.switch_to("a.exe").expect("synthetic app");
    collector.poll_once(at(1_000)).expect("sample A again");
    collector.finish(at(1_100)).expect("finish A");

    assert_eq!(source.samples_taken(), 2);
    assert_eq!(source.switches(), 3);
    assert_eq!(
        stored_sessions(&store),
        vec![("a.exe".into(), 1_100)],
        "one stored session is not proof of uninterrupted A or zero B activity",
    );
}

#[test]
fn backwards_clock_stores_zero_for_the_ended_session_without_losing_the_next_one() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let store = open_shared(dir.path().join("backwards.db"));
    let source = FakeForegroundSource::showing("a.exe").expect("synthetic app");
    let mut collector = Collector::new(source.clone(), Arc::clone(&store), granted_consent());

    collector.poll_once(at(5_000)).expect("sample A");
    source.switch_to("b.exe").expect("synthetic app");
    collector
        .poll_once(at(4_000))
        .expect("sample B after clock step");
    collector.finish(at(6_000)).expect("finish B");

    assert_eq!(
        stored_sessions(&store),
        vec![("a.exe".into(), 0), ("b.exe".into(), 2_000)],
        "the saturated zero records a clock anomaly, not measured inactivity",
    );
}

#[test]
fn a_long_unsampled_interval_remains_elapsed_time_not_continuous_observation() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let store = open_shared(dir.path().join("long_gap.db"));
    let source = FakeForegroundSource::showing("a.exe").expect("synthetic app");
    let mut collector = Collector::new(source.clone(), Arc::clone(&store), granted_consent());

    collector.poll_once(at(0)).expect("sample A");
    // A scripted clock jump: no actual waiting, OS sleep, or wake is exercised.
    collector
        .poll_once(at(3_600_000))
        .expect("sample A an hour later");
    source.switch_to("b.exe").expect("synthetic app");
    collector.poll_once(at(3_601_000)).expect("sample B");
    collector.finish(at(3_602_000)).expect("finish B");

    assert_eq!(source.samples_taken(), 3);
    assert_eq!(
        stored_sessions(&store),
        vec![("a.exe".into(), 3_601_000), ("b.exe".into(), 1_000)],
        "elapsed session duration does not establish coverage inside the sampling gap",
    );
}

#[test]
fn reopening_retains_completed_sessions_but_does_not_resume_an_unfinished_one() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join("reopened.db");
    let store = open_shared(&path);
    let source = FakeForegroundSource::showing("a.exe").expect("synthetic app");
    let mut collector = Collector::new(source.clone(), Arc::clone(&store), granted_consent());

    collector.poll_once(at(0)).expect("sample A");
    source.switch_to("b.exe").expect("synthetic app");
    collector
        .poll_once(at(1_000))
        .expect("complete A and open B");
    let completed = foreground_events(&store);
    assert_eq!(completed.len(), 1);
    let completed_id = completed[0].event_id;

    // No finish: unlike dropping the runner handle, this does not flush B.
    drop(collector);
    assert_eq!(Arc::strong_count(&store), 1, "only the test owns the store");
    drop(store);

    let reopened = open_shared(&path);
    assert_eq!(
        stored_sessions(&reopened),
        vec![("a.exe".into(), 1_000)],
        "the completed body remains decryptable and B was never committed",
    );
    assert_eq!(foreground_events(&reopened)[0].event_id, completed_id);

    let next_source = FakeForegroundSource::showing("c.exe").expect("synthetic app");
    let mut next = Collector::new(next_source, Arc::clone(&reopened), granted_consent());
    next.poll_once(at(10_000)).expect("start a fresh C session");
    next.finish(at(10_500)).expect("finish C");

    assert_eq!(
        stored_sessions(&reopened),
        vec![("a.exe".into(), 1_000), ("c.exe".into(), 500)],
        "a new collector must not bridge the unwritten B or closed-store interval",
    );
    assert!(foreground_events(&reopened)
        .iter()
        .any(|event| event.event_id == completed_id));
}
