//! AC-10: switch applications with collection on, then turn it off.
//!
//! Two numbers are being watched. During the on period at least one event has
//! to appear — collection that collects nothing would satisfy every privacy
//! assertion in this repository and none of the product's purpose. After
//! collection stops, the count must not move for a second, which is
//! PRODUCT_LOCK's "关闭后 1 秒内无新事件" stated as an experiment rather than a
//! design intention.
//!
//! Both run against the real background collector: a thread, a poll interval, a
//! condition variable, and a real encrypted store. The only stand-in is the
//! foreground source, because there is no desktop on a Linux CI host.
//!
//! The second number is measured twice, because there are two ways to turn
//! collection off and they are not the same event. Stopping is an orderly
//! wind-down that writes the session the user is still in. Revoking consent is
//! the user withdrawing permission, and the session in flight is dropped.

mod common;

use std::sync::Arc;
use std::time::{Duration, Instant};

use common::*;

use soul_collect::consent::COLLECTION_TOPIC;
use soul_collect::{runner, CollectorConfig, FakeForegroundSource, StopReason, STOP_BUDGET};

/// How long an acceptance test waits for the collector to produce its first
/// event before calling it broken. Far above the poll interval, so a loaded CI
/// host is slow rather than red.
const FIRST_EVENT_DEADLINE: Duration = Duration::from_secs(10);

/// Applications to alt-tab between while the collector is running.
const APPS: [&str; 4] = ["code.exe", "chrome.exe", "wechat.exe", "excel.exe"];

/// Keep switching until the store has at least one foreground event.
fn switch_until_recorded(
    source: &FakeForegroundSource,
    store: &Arc<std::sync::Mutex<soul_store::SqlCipherStore>>,
) -> usize {
    let mut index = 0usize;
    let recorded = wait_until(FIRST_EVENT_DEADLINE, || {
        source
            .switch_to(APPS[index % APPS.len()])
            .expect("a valid application name");
        index += 1;
        foreground_event_count(store) > 0
    });
    assert!(
        recorded,
        "AC-10: with collection on, switching applications must produce at least one event \
         (the source was sampled {} times)",
        source.samples_taken(),
    );
    foreground_event_count(store)
}

#[test]
fn collection_records_while_it_is_on_and_stops_writing_the_moment_it_is_stopped() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = open_shared(dir.path().join("lifecycle.db"));
    let consent = granted_consent();
    let source = FakeForegroundSource::showing(APPS[0]).expect("a valid application name");

    let handle = runner::start(
        source.clone(),
        Arc::clone(&store),
        &consent,
        CollectorConfig::every(TEST_POLL_INTERVAL),
    )
    .expect("consent is granted, so the collector starts");

    let during = switch_until_recorded(&source, &store);

    let stopping = Instant::now();
    let report = handle.stop().expect("the collector thread winds down");
    let took = stopping.elapsed();

    assert!(
        took <= STOP_BUDGET,
        "stopping took {took:?}, over the {STOP_BUDGET:?} the product promises",
    );
    assert_eq!(report.stopped_because, StopReason::Requested);
    assert_eq!(report.last_error, None);
    assert!(report.tally.events_written >= 1);

    // From here on the desktop keeps changing and the collector must not care.
    let after_stop = foreground_event_count(&store);
    assert!(after_stop >= during);
    for (index, app) in APPS.iter().cycle().take(20).enumerate() {
        source.switch_to(app).expect("a valid application name");
        if index % 4 == 0 {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    std::thread::sleep(Duration::from_secs(1));

    assert_eq!(
        foreground_event_count(&store),
        after_stop,
        "AC-10: no event may be written in the second after collection stopped",
    );
    assert_eq!(
        report.tally.events_written as usize, after_stop,
        "the report and the store must agree about how much was collected",
    );
}

#[test]
fn revoking_consent_stops_the_collector_without_being_asked_to() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = open_shared(dir.path().join("revoked.db"));
    let consent = granted_consent();
    let source = FakeForegroundSource::showing(APPS[0]).expect("a valid application name");

    let handle = runner::start(
        source.clone(),
        Arc::clone(&store),
        &consent,
        CollectorConfig::every(TEST_POLL_INTERVAL),
    )
    .expect("consent is granted, so the collector starts");

    switch_until_recorded(&source, &store);

    let revoking = Instant::now();
    consent.revoke(COLLECTION_TOPIC, NOW + 60);
    let stopped = wait_until(STOP_BUDGET, || !handle.is_running());
    let took = revoking.elapsed();
    assert!(
        stopped,
        "the collector was still running {took:?} after consent was revoked",
    );

    let at_revocation = foreground_event_count(&store);
    for app in APPS.iter().cycle().take(12) {
        source.switch_to(app).expect("a valid application name");
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(800));

    assert_eq!(
        foreground_event_count(&store),
        at_revocation,
        "a revocation must be as final as a stop: no event in the second that follows",
    );

    let report = handle.stop().expect("the thread has already finished");
    assert_eq!(report.stopped_because, StopReason::ConsentWithdrawn);
    assert_eq!(
        report.tally.sessions_discarded, 1,
        "the session in flight when consent went away is dropped, not flushed",
    );
    assert_eq!(report.last_error, None);
}

#[test]
fn stopping_does_not_wait_for_the_poll_interval() {
    // The default interval is a second, so a collector that stopped by waking
    // up on its own schedule could not meet the budget. This is the test that
    // separates "stops quickly because it polls quickly" from "stops quickly".
    let dir = tempfile::tempdir().expect("temp dir");
    let store = open_shared(dir.path().join("prompt_stop.db"));
    let consent = granted_consent();
    let source = FakeForegroundSource::showing(APPS[0]).expect("a valid application name");

    let handle = runner::start(
        source.clone(),
        Arc::clone(&store),
        &consent,
        CollectorConfig::default(),
    )
    .expect("the collector starts");
    assert_eq!(
        soul_collect::DEFAULT_POLL_INTERVAL,
        Duration::from_secs(1),
        "this test is only meaningful while the default interval is the budget",
    );

    let stopping = Instant::now();
    let report = handle.stop().expect("stop");
    let took = stopping.elapsed();

    assert!(
        took < STOP_BUDGET / 2,
        "stopping waited {took:?}, which looks like it slept out the poll interval",
    );
    assert_eq!(report.stopped_because, StopReason::Requested);
}

#[test]
fn dropping_the_handle_stops_the_thread_rather_than_orphaning_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = open_shared(dir.path().join("dropped.db"));
    let consent = granted_consent();
    let source = FakeForegroundSource::showing(APPS[0]).expect("a valid application name");

    drop(
        runner::start(
            source.clone(),
            Arc::clone(&store),
            &consent,
            CollectorConfig::every(TEST_POLL_INTERVAL),
        )
        .expect("the collector starts"),
    );

    let sampled_by_the_time_it_was_dropped = source.samples_taken();
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(
        source.samples_taken(),
        sampled_by_the_time_it_was_dropped,
        "a dropped handle must not leave a collector running with no way to stop it",
    );
}

#[test]
fn a_collector_started_on_a_platform_without_a_source_says_so_rather_than_collecting_nothing() {
    // On Linux there is no foreground source, and `platform_source` reports
    // that instead of returning something that always answers "nothing".
    #[cfg(not(windows))]
    {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = open_shared(dir.path().join("unsupported.db"));
        let consent = granted_consent();

        let source = soul_collect::platform_source();
        assert!(
            matches!(source, Err(soul_collect::SourceError::Unsupported)),
            "a host with no desktop must not be handed a silent collector",
        );
        assert_eq!(all_event_count(&store), 0);
        drop(consent);
    }
    #[cfg(windows)]
    {
        let source = soul_collect::platform_source().expect("Windows has a foreground source");
        assert_eq!(source.describe(), "windows.foreground_process");
    }
}
