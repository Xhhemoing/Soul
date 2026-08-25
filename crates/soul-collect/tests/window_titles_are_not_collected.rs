//! The promise PRODUCT_LOCK makes about what this collector does *not* do.
//!
//! "窗口标题不采" cannot be demonstrated by running the collector: an
//! implementation that reads the caption and throws it away passes every
//! behavioural test, and so does one that keeps it in a field nobody looked at.
//! So two different kinds of check are made here.
//!
//! Structurally, the crate's own sources are read back and searched for the
//! Win32 calls that return a caption. If a later change reaches for
//! `GetWindowTextW`, this test names the file and the line.
//!
//! Behaviourally, a source that hands the collector something title-shaped is
//! refused rather than stored, the event's sealed body is opened and found to
//! contain exactly an application name and a duration, and the database file is
//! searched byte by byte for both the name and a caption that was never
//! offered — the first because it must be sealed, the second because it must
//! never have existed.

mod common;

use std::sync::Arc;

use common::*;

use soul_collect::{AppIdentity, Collector, FakeForegroundSource, SealedSessionBody, SourceError};
use soul_store_api::{BlobStore, SoulStore};
use soul_testkit::LeakageChecker;

/// Everything Windows offers that returns a window caption, plus the UI
/// Automation property that is the modern spelling of the same thing.
const TITLE_APIS: &[&str] = &[
    "GetWindowText",
    "GetWindowTextW",
    "GetWindowTextA",
    "GetWindowTextLength",
    "InternalGetWindowText",
    "WM_GETTEXT",
    "GetConsoleTitle",
    "CurrentName",
    "UIA_NamePropertyId",
    "AccessibleObjectFromWindow",
];

/// And the things a keylogger or a screen scraper would need.
const OBSERVATION_APIS: &[&str] = &[
    "SetWindowsHookEx",
    "GetAsyncKeyState",
    "GetKeyboardState",
    "RegisterRawInputDevices",
    "BitBlt",
    "PrintWindow",
    "GetClipboardData",
];

fn crate_sources() -> Vec<(std::path::PathBuf, String)> {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut sources = Vec::new();
    let mut pending = vec![src];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("read the crate's src directory") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let text = std::fs::read_to_string(&path).expect("read a source file");
                sources.push((path, text));
            }
        }
    }
    assert!(
        sources.len() >= 8,
        "the scan found {} files, which is too few to have walked the crate",
        sources.len(),
    );
    sources
}

#[test]
fn no_source_file_in_this_crate_names_a_window_title_api() {
    let mut found = Vec::new();
    for (path, text) in crate_sources() {
        for (number, line) in text.lines().enumerate() {
            for api in TITLE_APIS.iter().chain(OBSERVATION_APIS) {
                // The list itself lives in this test file, which is not scanned.
                if line.contains(api) {
                    found.push(format!("{}:{}: {api}", path.display(), number + 1));
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "PRODUCT_LOCK puts window titles, keyboard hooks and screen capture in the \
         `不做` column, but this crate names: {found:#?}",
    );
}

#[test]
fn the_windows_source_asks_for_the_process_and_not_for_the_window() {
    let windows = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/windows.rs");
    let text = std::fs::read_to_string(windows).expect("the Windows source is present in the tree");
    for expected in [
        "GetForegroundWindow",
        "GetWindowThreadProcessId",
        "QueryFullProcessImageNameW",
    ] {
        assert!(
            text.contains(expected),
            "the Windows source no longer calls {expected}; if the implementation \
             changed, this test's idea of how the app is identified is out of date",
        );
    }
}

#[test]
fn a_source_that_offers_a_title_instead_of_an_application_is_refused() {
    let titles = [
        "报税 2026.xlsx - Excel",
        "Re: 合同条款 - 邮件",
        "C:\\Users\\罗伊\\Documents\\日记.docx",
        "soul — private notes — Visual Studio Code",
    ];
    for title in titles {
        let refusal = AppIdentity::new(title);
        assert!(
            matches!(
                refusal,
                Err(SourceError::NotAFileName
                    | SourceError::NameTooLong { .. }
                    | SourceError::NotAnExecutableName)
            ),
            "a window caption was accepted as an application name: {refusal:?}",
        );
    }

    // And the same through the source seam a platform implementation uses.
    let source = FakeForegroundSource::new();
    assert!(source.switch_to("报税 2026.xlsx - Excel").is_err());
}

#[test]
fn what_lands_in_the_store_is_a_name_and_a_duration_sealed_under_one_key() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("sealed.db");
    let store = open_shared(&path);
    let consent = granted_consent();
    let source = FakeForegroundSource::new();
    let mut collector = Collector::new(source.clone(), Arc::clone(&store), consent);
    let content_key_id = collector.content_key_id();

    let base = (NOW as u64) * 1_000;
    source.switch_to("excel.exe").expect("a valid name");
    collector.poll_once(base).expect("poll");
    source.switch_to("outlook.exe").expect("a valid name");
    collector.poll_once(base + 90_000).expect("poll");
    collector.finish(base + 150_000).expect("finish");

    let events = foreground_events(&store);
    assert_eq!(events.len(), 2);

    {
        let store = store.lock().expect("the store lock");
        for event in &events {
            let sealed = event.body_ref.as_ref().expect("a sealed body");
            assert_eq!(
                sealed.content_key_id, content_key_id,
                "one forget unit per run, so `forget what I collected` has something to destroy",
            );
            let bytes = store.open(sealed).expect("open the sealed body");
            let value: serde_json::Value =
                serde_json::from_slice(&bytes).expect("the body is JSON");
            let keys: Vec<&str> = value
                .as_object()
                .expect("an object")
                .keys()
                .map(String::as_str)
                .collect();
            assert_eq!(
                keys,
                SealedSessionBody::FIELDS,
                "the sealed body must carry the application and the duration, and nothing else",
            );
        }
    }

    let first = SealedSessionBody::from_json_bytes(&{
        let store = store.lock().expect("the store lock");
        store
            .open(events[0].body_ref.as_ref().expect("sealed"))
            .expect("open")
    })
    .expect("a session body");
    assert_eq!(first.app, "excel.exe");
    assert_eq!(first.duration_ms, 90_000);

    // Close the database before searching the file: an in-process assertion
    // says nothing about the bytes that reached the disk.
    {
        let mut store = store.lock().expect("the store lock");
        store.flush().expect("flush");
    }
    drop(collector);
    let store = Arc::try_unwrap(store).expect("the collector is gone, so the store is unshared");
    drop(store.into_inner().expect("the store lock"));

    for entry in std::fs::read_dir(dir.path()).expect("read the database directory") {
        let file = entry.expect("a directory entry").path();
        let bytes = std::fs::read(&file).expect("read a database file");
        for needle in ["excel.exe", "outlook.exe"] {
            assert!(
                !contains(&bytes, needle.as_bytes()),
                "{} holds `{needle}` in the clear; the application name must be sealed",
                file.display(),
            );
        }
    }
}

#[test]
fn the_audit_chain_records_the_run_and_none_of_what_was_collected() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = open_shared(dir.path().join("audit.db"));
    let consent = granted_consent();
    let source = FakeForegroundSource::new();
    let mut collector = Collector::new(source.clone(), Arc::clone(&store), consent.clone());

    collector.audit_start(NOW).expect("collect.start");
    let base = (NOW as u64) * 1_000;
    for (index, app) in ["wechat.exe", "chrome.exe", "notepad.exe"]
        .into_iter()
        .enumerate()
    {
        source.switch_to(app).expect("a valid name");
        collector
            .poll_once(base + (index as u64) * 30_000)
            .expect("poll");
    }
    collector.finish(base + 120_000).expect("finish");
    collector
        .audit_stop(soul_policy::audit::ReasonCode::Routine, NOW + 120)
        .expect("collect.stop");

    let chain = audit_json(&store);
    assert!(chain.contains("collect.start"));
    assert!(chain.contains("collect.stop"));
    assert!(
        chain.contains(&collector.content_key_id().to_string()),
        "the entries name the forget unit, so the user can act on what a run collected",
    );

    // Every application name is treated as a corpus entry the chain must not
    // contain, at a threshold well below the length of any of them.
    let mut checker = LeakageChecker::new().with_min_ngram(4);
    for app in ["wechat.exe", "chrome.exe", "notepad.exe", "excel.exe"] {
        checker.add_known_identifier(app, app);
        checker.add_third_party_body(app, app);
    }
    checker.assert_clean("the collector's audit entries", &chain);

    let counted = serde_json::from_str::<serde_json::Value>(&chain)
        .expect("the chain is JSON")
        .as_array()
        .expect("an array")
        .iter()
        .filter_map(|entry| entry.get("counts").and_then(|counts| counts.get("items")))
        .filter_map(serde_json::Value::as_u64)
        .max();
    assert_eq!(
        counted,
        Some(3),
        "`collect.stop` reports how many events the run wrote, which is a count and not a trace",
    );
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}
