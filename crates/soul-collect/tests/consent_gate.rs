//! AC-09: with collection off, switching applications ten times leaves no trace.
//!
//! The interesting part of this test is what it refuses to do. It does not
//! assert that a flag is false, and it does not insert anything itself: it runs
//! the collector that a Windows machine runs, over a source that reports ten
//! application switches, into a real encrypted store — and then counts the rows.
//! A gate that were merely decorative would still let those rows land.
//!
//! The last test is the control. The same ten switches with consent granted
//! produce ten events, so "zero" above is the gate's doing rather than a
//! pipeline that never worked.

mod common;

use std::sync::Arc;
use std::time::Duration;

use common::*;

use soul_collect::consent::COLLECTION_TOPIC;
use soul_collect::{
    runner, CollectError, Collector, CollectorConfig, ConsentHandle, FakeForegroundSource, Poll,
    SealedSessionBody,
};
use soul_store_api::BlobStore;

/// The ten applications the user alt-tabs between.
const APPS: [&str; 10] = [
    "explorer.exe",
    "notepad.exe",
    "code.exe",
    "chrome.exe",
    "excel.exe",
    "wechat.exe",
    "term.exe",
    "figma.exe",
    "music.exe",
    "mail.exe",
];

/// One tick per switch, spaced far enough apart that a session has a duration.
fn tick(index: usize) -> u64 {
    (NOW as u64) * 1_000 + (index as u64) * 5_000
}

#[test]
fn ten_application_switches_with_consent_withheld_leave_no_events() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = open_shared(dir.path().join("withheld.db"));

    let consent = ConsentHandle::closed();
    assert!(
        !consent.is_granted(COLLECTION_TOPIC),
        "a fresh ledger must grant nothing; that is what `采集默认关` means",
    );

    let source = FakeForegroundSource::new();
    let mut collector = Collector::new(source.clone(), Arc::clone(&store), consent.clone());

    for (index, app) in APPS.iter().enumerate() {
        source.switch_to(app).expect("a valid application name");
        let outcome = collector
            .poll_once(tick(index))
            .expect("a withheld gate is a refusal, not a failure");
        assert_eq!(outcome, Poll::ConsentWithheld, "switch {index}");
    }

    assert_eq!(
        foreground_event_count(&store),
        0,
        "AC-09: ten switches with collection off must leave no foreground events",
    );
    assert_eq!(
        all_event_count(&store),
        0,
        "and no events of any other kind either",
    );
    assert_eq!(collector.tally().events_written, 0);
    assert_eq!(collector.tally().consent_refusals, APPS.len() as u64);
    assert_eq!(
        source.samples_taken(),
        0,
        "the gate is in front of the source: with collection off the foreground \
         is never even looked at",
    );
    assert_eq!(source.switches(), APPS.len() as u64);
}

#[test]
fn a_collector_cannot_be_started_at_all_while_consent_is_withheld() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = open_shared(dir.path().join("refused_start.db"));
    let source = FakeForegroundSource::showing("code.exe").expect("a valid application name");

    let refusal = runner::start(
        source.clone(),
        Arc::clone(&store),
        &ConsentHandle::closed(),
        CollectorConfig::every(TEST_POLL_INTERVAL),
    );

    match refusal {
        Err(CollectError::Consent(missing)) => {
            assert_eq!(missing.topic, COLLECTION_TOPIC);
        }
        Err(other) => panic!("expected a consent refusal, got {other}"),
        Ok(_) => panic!("a collector started without consent"),
    }

    // Nothing spawned, so nothing to wait for; the assertion is that the wait
    // times out rather than that a sleep was long enough.
    assert!(
        !wait_until(Duration::from_millis(200), || source.samples_taken() > 0),
        "a refused start must not leave a thread sampling in the background",
    );
    assert_eq!(all_event_count(&store), 0);
}

#[test]
fn revoking_between_the_poll_and_the_write_discards_the_session_in_flight() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = open_shared(dir.path().join("revoked_mid_session.db"));
    let consent = granted_consent();
    let source = FakeForegroundSource::showing("code.exe").expect("a valid application name");
    let mut collector = Collector::new(source.clone(), Arc::clone(&store), consent.clone());

    assert_eq!(
        collector.poll_once(tick(0)).expect("poll"),
        Poll::SessionStarted,
    );

    // The user turns collection off while `code.exe` is still in front.
    consent.revoke(COLLECTION_TOPIC, NOW + 1);

    assert_eq!(
        collector.finish(tick(1)).expect("finish"),
        None,
        "the session that was in flight when consent went away must not be written",
    );
    assert_eq!(foreground_event_count(&store), 0);
    assert_eq!(collector.tally().sessions_discarded, 1);
}

#[test]
fn the_same_ten_switches_with_consent_granted_are_recorded() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = open_shared(dir.path().join("granted.db"));
    let consent = granted_consent();
    let source = FakeForegroundSource::new();
    let mut collector = Collector::new(source.clone(), Arc::clone(&store), consent);

    for (index, app) in APPS.iter().enumerate() {
        source.switch_to(app).expect("a valid application name");
        collector.poll_once(tick(index)).expect("poll");
    }
    collector
        .finish(tick(APPS.len()))
        .expect("the last session is written by an orderly stop");

    let events = foreground_events(&store);
    assert_eq!(
        events.len(),
        APPS.len(),
        "one event per completed session, so the zero above is the gate's doing",
    );

    let bodies: Vec<SealedSessionBody> = {
        let store = store.lock().expect("the store lock");
        events
            .iter()
            .map(|event| {
                let sealed = event.body_ref.as_ref().expect("a sealed body");
                let bytes = store.open(sealed).expect("open the sealed body");
                SealedSessionBody::from_json_bytes(&bytes).expect("a session body")
            })
            .collect()
    };

    let names: Vec<&str> = bodies.iter().map(|body| body.app.as_str()).collect();
    assert_eq!(
        names, APPS,
        "the sessions come back in the order they ended"
    );
    for body in &bodies {
        assert_eq!(
            body.duration_ms, 5_000,
            "each session lasted exactly one five-second tick",
        );
    }
}
