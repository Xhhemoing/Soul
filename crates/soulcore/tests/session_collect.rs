//! AC-09 and AC-10, proven through the object the desktop shell actually holds.
//!
//! `soul-collect` already measures both against its own runner, and those tests
//! are the ones that watch the clock. What they cannot show is that a running
//! Soul is wired to any of it: until the session owned a consent ledger and a
//! collector, every one of those guarantees was true of a crate nobody in the
//! product called. That is the gap this file closes — same ledger, same
//! collector, same encrypted store, reached the way `collect_status` and
//! `grant_collect_consent` are reached from a click.
//!
//! The one substitution is the foreground source, because a Linux CI host has
//! no desktop to be in front of. Everything below it is the code a Windows
//! machine runs.
//!
//! The other half of the file is about the file on disk. AC-02 says a restart
//! reopens nothing, and for collection that is a claim about `config.json`
//! having nowhere to write consent down: the tests reopen the directory and
//! read the bytes.

use std::path::{Path, PathBuf};
use std::time::Duration;

use soulcore::commands::collect::{CollectorConfig, FakeForegroundSource};
use soulcore::commands::session::{
    read_stored_config, Session, StoredConfig, CONFIG_FILE_NAME, NO_FOREGROUND_SOURCE,
};
use soulcore::commands::shell::WizardAnswers;

/// Fast enough that the test does not spend its life asleep, slow enough that
/// the collector is polling rather than spinning. Same value `soul-collect`'s
/// own acceptance tests use.
const TEST_POLL_INTERVAL: Duration = Duration::from_millis(20);

/// How long to wait for the first event before calling the wiring broken. Far
/// above the poll interval, so a loaded CI host is slow rather than red.
const FIRST_EVENT_DEADLINE: Duration = Duration::from_secs(10);

/// Applications to alt-tab between. Names only; that is the whole of what a
/// sample may carry.
const APPS: [&str; 4] = ["code.exe", "chrome.exe", "wechat.exe", "excel.exe"];

fn scratch() -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let canonical = std::fs::canonicalize(directory.path()).expect("canonical temp dir");
    (directory, canonical)
}

fn collected(session: &Session) -> usize {
    session
        .collect_status()
        .events_collected
        .expect("the store opened, so the events can be counted")
}

fn wait_until(deadline: Duration, mut predicate: impl FnMut() -> bool) -> bool {
    let started = std::time::Instant::now();
    while started.elapsed() < deadline {
        if predicate() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    predicate()
}

/// The wizard, so there is a `config.json` on disk to inspect.
fn finish_the_wizard(session: &mut Session) {
    session
        .complete_wizard(&WizardAnswers {
            acknowledged_defaults_are_off: true,
        })
        .expect("the wizard hands back a closed configuration");
}

fn config_bytes(directory: &Path) -> Vec<u8> {
    std::fs::read(directory.join(CONFIG_FILE_NAME)).expect("the configuration file")
}

/// AC-10 through the session: with consent given, switching applications
/// produces events, and taking the consent back stops them inside a second.
///
/// The revocation is what is being measured, not the stop. `revoke_collect_consent`
/// withdraws first and stops second precisely so that a build where revocation
/// did nothing would still be caught here by the count moving afterwards.
#[test]
fn granting_collects_and_revoking_stops_within_the_second() {
    let (keep, directory) = scratch();
    let mut session = Session::open(&directory);
    let source = FakeForegroundSource::showing(APPS[0]).expect("a valid application name");

    let started = session
        .grant_collect_consent_with_source(
            source.clone(),
            CollectorConfig::every(TEST_POLL_INTERVAL),
        )
        .expect("the store opened, so consent can be recorded");
    assert!(started.consent_granted);
    assert!(
        started.collector_running,
        "a source was handed in, so something should be watching: {}",
        started.notice,
    );
    assert_eq!(started.source, "fake.scripted_desktop");
    assert!(!started.survives_restart);

    let mut index = 0usize;
    let recorded = wait_until(FIRST_EVENT_DEADLINE, || {
        source
            .switch_to(APPS[index % APPS.len()])
            .expect("a valid application name");
        index += 1;
        collected(&session) > 0
    });
    assert!(
        recorded,
        "AC-10: with collection on, switching applications must produce at least one event \
         (the source was sampled {} times)",
        source.samples_taken(),
    );

    let after_revoking = session
        .revoke_collect_consent()
        .expect("revoking works whenever the store is open");
    assert!(!after_revoking.consent_granted);
    assert!(!after_revoking.collector_running);
    let at_revocation = after_revoking
        .events_collected
        .expect("the store is still open");

    // The desktop carries on changing, and nothing may notice.
    for (turn, app) in APPS.iter().cycle().take(20).enumerate() {
        source.switch_to(app).expect("a valid application name");
        if turn % 4 == 0 {
            std::thread::sleep(Duration::from_millis(25));
        }
    }
    std::thread::sleep(Duration::from_secs(1));

    assert_eq!(
        collected(&session),
        at_revocation,
        "AC-10: no event may be written in the second after the user took consent back",
    );
    drop(keep);
}

/// AC-09 through the session: nobody granted anything, so nothing is sampled.
///
/// The strong form of the assertion is the sample count rather than the event
/// count. Zero events would also be true of a collector that ran and happened
/// to write nothing; zero samples says the desktop was never looked at.
#[test]
fn a_session_nobody_consented_to_never_looks_at_the_desktop() {
    let (keep, directory) = scratch();
    let session = Session::open(&directory);
    let source = FakeForegroundSource::showing(APPS[0]).expect("a valid application name");

    let status = session.collect_status();
    assert!(!status.consent_granted, "consent starts closed");
    assert!(!status.collector_running);
    assert_eq!(collected(&session), 0);

    for app in APPS.iter().cycle().take(10) {
        source.switch_to(app).expect("a valid application name");
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(300));

    assert_eq!(
        source.samples_taken(),
        0,
        "AC-09: with consent withheld, nothing may sample the foreground",
    );
    assert_eq!(
        collected(&session),
        0,
        "AC-09: with consent withheld, ten application switches must write no event",
    );
    drop(keep);
}

/// On a host with no foreground source the grant is honest about what happened:
/// the consent is recorded, and nothing is watching.
///
/// This is the normal outcome on Linux and on a developer machine, and it is
/// the state a build must not paper over. Reporting "采集已打开" while sampling
/// nothing would be worse than either of the other two states.
#[test]
#[cfg(not(windows))]
fn a_grant_on_a_machine_with_no_foreground_source_says_so() {
    let (keep, directory) = scratch();
    let mut session = Session::open(&directory);

    let status = session
        .grant_collect_consent()
        .expect("the store opened, so consent can be recorded");

    assert!(
        status.consent_granted,
        "the user said yes; the platform is a separate question",
    );
    assert!(!status.collector_running);
    assert_eq!(status.source, NO_FOREGROUND_SOURCE);
    assert_eq!(status.events_collected, Some(0));
    assert!(
        status.notice.contains("没有东西在采"),
        "the notice does not say that nothing is watching: {}",
        status.notice,
    );
    drop(keep);
}

/// AC-02 for collection: a restart cannot reopen it, because there is no field
/// in the file it could have been written to.
#[test]
fn a_restart_reopens_a_closed_collection() {
    let (keep, directory) = scratch();
    let source = FakeForegroundSource::showing(APPS[0]).expect("a valid application name");
    {
        let mut session = Session::open(&directory);
        finish_the_wizard(&mut session);
        session
            .grant_collect_consent_with_source(
                source.clone(),
                CollectorConfig::every(TEST_POLL_INTERVAL),
            )
            .expect("consent is recorded");
        assert!(session.collect_status().consent_granted);
    }

    let next_launch = Session::open(&directory);
    let status = next_launch.collect_status();
    assert!(
        !status.consent_granted,
        "AC-02: a restart may not carry a granted consent across",
    );
    assert!(!status.collector_running);
    assert!(!status.survives_restart);

    let stored = read_stored_config(&directory).expect("the file still parses as a StoredConfig");
    assert_eq!(
        stored,
        StoredConfig {
            wizard_completed: true,
            authorized_roots: Vec::new(),
        },
    );
    drop(keep);
}

/// And the same claim read off the bytes rather than off the parse.
///
/// `deny_unknown_fields` already refuses to *read* a collection key, which is
/// most of AC-02. What it cannot catch is a version of this code that writes
/// one — the file would round-trip through a future build that knew the field.
/// So the test looks at what was actually written: neither word appears.
#[test]
fn the_configuration_file_never_learns_the_word() {
    let (keep, directory) = scratch();
    let mut session = Session::open(&directory);
    finish_the_wizard(&mut session);
    session
        .authorize(&directory.to_string_lossy())
        .expect("a directory that exists");

    let source = FakeForegroundSource::showing(APPS[0]).expect("a valid application name");
    session
        .grant_collect_consent_with_source(
            source.clone(),
            CollectorConfig::every(TEST_POLL_INTERVAL),
        )
        .expect("consent is recorded");
    // Grant, collect, and revoke: every write the file could possibly see.
    wait_until(FIRST_EVENT_DEADLINE, || {
        source.switch_to(APPS[1]).expect("a valid application name");
        source.switch_to(APPS[2]).expect("a valid application name");
        collected(&session) > 0
    });
    session.revoke_collect_consent().expect("revoking works");

    let bytes = config_bytes(&directory);
    let text = String::from_utf8(bytes).expect("the configuration file is utf-8");
    for word in ["collect", "consent"] {
        assert!(
            !text.contains(word),
            "`{word}` reached config.json, which is the one file that must not carry it:\n{text}",
        );
    }

    let value: serde_json::Value = serde_json::from_str(&text).expect("parse");
    let object = value.as_object().expect("the file is a JSON object");
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["authorized_roots", "wizard_completed"],
        "config.json grew a field, and the whole argument for AC-02 is that it has two",
    );
    drop(keep);
}

/// Granting twice does not start a second collector, and the status the second
/// call hands back describes the one that is already running.
#[test]
fn a_second_grant_does_not_start_a_second_collector() {
    let (keep, directory) = scratch();
    let mut session = Session::open(&directory);
    let source = FakeForegroundSource::showing(APPS[0]).expect("a valid application name");

    session
        .grant_collect_consent_with_source(
            source.clone(),
            CollectorConfig::every(TEST_POLL_INTERVAL),
        )
        .expect("consent is recorded");
    let sampled_once = wait_until(FIRST_EVENT_DEADLINE, || source.samples_taken() > 0);
    assert!(sampled_once, "the first collector never sampled anything");

    let again = session
        .grant_collect_consent_with_source(
            source.clone(),
            CollectorConfig::every(TEST_POLL_INTERVAL),
        )
        .expect("a second grant is not an error");
    assert!(again.consent_granted);
    assert!(again.collector_running);

    session.revoke_collect_consent().expect("revoking works");
    let at_revocation = collected(&session);
    for app in APPS.iter().cycle().take(12) {
        source.switch_to(app).expect("a valid application name");
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_secs(1));
    assert_eq!(
        collected(&session),
        at_revocation,
        "one revocation must be enough, which it is not if a second collector was started",
    );
    drop(keep);
}

/// The status the interface is sent has nowhere to put an application name.
///
/// `soul-collect` keeps window titles out of `AppIdentity`; what is checked
/// here is the next hop, where a well-meaning field could put the name of the
/// application on screen. The serialized status is read back as JSON and the
/// names that were actually collected are looked for in it.
#[test]
fn the_status_the_interface_receives_carries_no_application_name() {
    let (keep, directory) = scratch();
    let mut session = Session::open(&directory);
    let source = FakeForegroundSource::showing(APPS[0]).expect("a valid application name");

    session
        .grant_collect_consent_with_source(
            source.clone(),
            CollectorConfig::every(TEST_POLL_INTERVAL),
        )
        .expect("consent is recorded");
    let mut index = 0usize;
    wait_until(FIRST_EVENT_DEADLINE, || {
        source
            .switch_to(APPS[index % APPS.len()])
            .expect("a valid application name");
        index += 1;
        collected(&session) > 0
    });

    let json = serde_json::to_string(&session.collect_status()).expect("serialize the status");
    for app in APPS {
        assert!(
            !json.contains(app),
            "`{app}` was collected and then handed to the interface: {json}",
        );
    }
    assert!(!json.contains(".exe"), "an executable name reached the interface: {json}");
    assert!(
        !json.to_lowercase().contains("title"),
        "the status has a field that sounds like a window title: {json}",
    );

    session.revoke_collect_consent().expect("revoking works");
    drop(keep);
}
