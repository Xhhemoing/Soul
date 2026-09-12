//! AC-21: the default configuration, the whole main flow, and no connection
//! that leaves this machine.
//!
//! The flow itself lives in `soulcore::headless` because it also has to run on
//! a Windows machine that has no repository — `scripts/install-smoke.ps1` runs
//! the same binary after an installer. What is here is CI's side of it, plus
//! the part that no amount of running the flow can establish on its own: that
//! the socket observation would notice a connection if there were one.
//!
//! That last point is the whole reason this file is longer than one test. A
//! watcher that always reports zero is indistinguishable from a program that
//! never connects to anything, and only one of those is worth shipping.

use std::collections::BTreeSet;

use soulcore::headless::{self, MainFlowReport};

/// Every step the flow is expected to walk. A step that quietly stops running
/// takes its assertions with it, so the list is pinned here rather than left
/// to whatever the flow happened to produce.
const EXPECTED_STEPS: &[&str] = &[
    "wizard", "store", "import", "graph", "profile", "memory", "draft", "summary", "fileplan",
    "research", "collect", "cloud", "audit",
];

#[test]
fn the_main_flow_runs_end_to_end_and_holds_no_non_loopback_connection() {
    let scratch = tempfile::tempdir().expect("temp dir");
    let report = headless::run_in(scratch.path()).expect("the main flow");

    assert!(report.ok);
    assert!(report.config.fully_closed, "AC-02: {:?}", report.config);
    assert!(report.config.open_capabilities.is_empty());
    assert!(!report.config.llm_endpoint_configured);
    assert!(report.audit_chain_verified);
    assert!(report.audit_entries >= 6);

    let walked: Vec<&str> = report.steps.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(walked, EXPECTED_STEPS);
    for step in &report.steps {
        assert!(!step.detail.trim().is_empty(), "{} said nothing", step.name);
    }

    // AC-21, the number the criterion actually names.
    assert_eq!(
        report.egress.non_loopback_connections, 0,
        "this process reached {:?}",
        report.egress.non_loopback_peers,
    );
    assert!(!report.egress.endpoint_configured);
    assert!(
        report.egress.samples > 1,
        "the watcher took {} sample(s), so it barely looked",
        report.egress.samples,
    );

    // Socket observation is a /proc read. Linux CI is the place that can do
    // it in-process. Windows CI watches the same binary from outside, in
    // `scripts/install-smoke.ps1`, because this crate forbids `unsafe` and
    // the Windows TCP table is an unsafe binding. Claiming Observed here on
    // Windows would be a fabricated pass; claiming a clean zero because we
    // did not look would be the other one. Each platform asserts the half
    // it can actually see.
    if cfg!(target_os = "linux") {
        assert!(
            report.egress.observed,
            "the sockets were not read on this host: {:?}",
            report.egress.observation_note,
        );
    } else {
        assert!(
            !report.egress.observed,
            "this crate reads /proc; Observed on a machine without it is a fabricated pass",
        );
        let note = report
            .egress
            .observation_note
            .as_deref()
            .expect("an unobserved run has to say why");
        assert!(
            note.contains("/proc"),
            "the note has to name the reason, not be an empty apology: {note}",
        );
    }

    // The closed guard was asked about three places that cannot exist, and
    // refused all three with a reason rather than a panic.
    assert_eq!(report.egress.refused.len(), 3);
    for refusal in &report.egress.refused {
        assert!(!refusal.reason_code.is_empty());
        assert!(!refusal.reason_code.contains(' '));
    }
}

/// The scratch store is a real encrypted database, and the flow is the thing
/// that has to have created it. Running twice in one process must also work:
/// the installer script runs the binary once, but a developer will not.
#[test]
fn two_runs_in_one_process_do_not_interfere() {
    let first = tempfile::tempdir().expect("temp dir");
    let second = tempfile::tempdir().expect("temp dir");

    let one = headless::run_in(first.path()).expect("first run");
    let two = headless::run_in(second.path()).expect("second run");

    assert_eq!(one.audit_entries, two.audit_entries);

    // The `store` step names the directory it opened, which is the one thing
    // that legitimately differs between two runs.
    let details = |report: &MainFlowReport| -> Vec<String> {
        report
            .steps
            .iter()
            .filter(|step| step.name != "store")
            .map(|step| format!("{}: {}", step.name, step.detail))
            .collect()
    };
    assert_eq!(
        details(&one),
        details(&two),
        "the flow reads a fixed clock and embedded corpora, so two runs should \
         agree about every count they report",
    );
    assert!(first.path().join("soul.db").is_file());
    assert!(second.path().join("soul.db").is_file());
}

/// A step that cannot do its job fails the run rather than being skipped.
#[test]
fn a_flow_that_cannot_open_its_store_fails_rather_than_reporting_success() {
    let file = tempfile::NamedTempFile::new().expect("temp file");
    let error = headless::run_in(file.path()).expect_err("a file is not a directory");
    assert!(
        !error.step.is_empty() && !error.detail.is_empty(),
        "a failure has to say where it happened: {error:?}",
    );
    assert!(error.to_string().contains(error.step));
}

/// `headless::run` picks its own directory and takes it away again.
#[test]
fn the_scratch_directory_does_not_outlive_the_run() {
    let report = headless::run().expect("the main flow");
    assert!(report.ok);
    assert!(
        our_scratch_directories().is_empty(),
        "the smoke left a database behind in the temp directory: {:?}",
        our_scratch_directories(),
    );
}

/// Scratch directories this process made. Named by pid, so the concurrent
/// `soul-headless` subprocess a sibling test spawns is not mistaken for a leak.
fn our_scratch_directories() -> BTreeSet<String> {
    let ours = format!("soul-headless-{}-", std::process::id());
    let mut found = BTreeSet::new();
    let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
        return found;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(&ours) {
            found.insert(name);
        }
    }
    found
}

// ---------------------------------------------------------- the binary ---

/// What `scripts/install-smoke.ps1` runs, run the same way: as a process,
/// reading only the exit code and stdout.
#[test]
fn the_binary_exits_zero_and_prints_a_report_the_installer_script_can_read() {
    let binary = std::path::Path::new(env!("CARGO_BIN_EXE_soul-headless"));
    let output = std::process::Command::new(binary)
        .output()
        .expect("run soul-headless");

    assert!(
        output.status.success(),
        "soul-headless exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    let report: MainFlowReport = serde_json::from_str(&stdout).expect("stdout is the report");
    assert!(report.ok);
    assert_eq!(report.egress.non_loopback_connections, 0);
    assert!(report.config.fully_closed);

    // The readable half goes to stderr, so a script may pipe stdout into a
    // JSON parser without stripping anything first.
    let stderr = String::from_utf8(output.stderr).expect("utf-8");
    assert!(stderr.contains("egress"));
    assert!(!stderr.contains("\"ok\":"));

    let unknown = std::process::Command::new(binary)
        .arg("definitely-not-a-command")
        .output()
        .expect("run soul-headless");
    assert!(
        !unknown.status.success(),
        "an unknown command must not pass"
    );
}
