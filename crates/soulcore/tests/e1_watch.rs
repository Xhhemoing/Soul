//! The endpoint instrument, run.
//!
//! Its own test binary, for the reason `tests/netwatch.rs` is its own:
//! [`soulcore::e1_watch::run`] watches the sockets of the *process* it runs
//! in, so a test beside it that opened one would be a peer the instrument did
//! not create and could not explain. Cargo gives each `tests/*.rs` its own
//! process, which is the isolation that separation buys — and the two tests
//! here that do reach outside the process reach a child, whose sockets are its
//! own.
//!
//! What is being checked is not that Soul can talk to an endpoint;
//! `session_e1.rs` does that, against a mock, by counting arrivals. It is that
//! while Soul was talking to one, the kernel had nothing else to report.

use std::process::Command;

use soulcore::e1_watch::{self, EXPECTED_REQUESTS};

const SOURCE: &str = include_str!("../src/e1_watch.rs");

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_soul-headless"))
}

/// The whole claim, in one run.
#[test]
fn the_endpoint_path_reaches_the_endpoint_and_nowhere_else() {
    let report = e1_watch::run().expect("the endpoint path runs");
    assert!(report.ok);

    // What arrived: one approved generation, one summary rephrasing, both on
    // the chat-completions path, neither carrying a key this build does not
    // have.
    assert_eq!(
        report.endpoint.requests.len(),
        EXPECTED_REQUESTS,
        "{:?}",
        report.endpoint,
    );
    for request in &report.endpoint.requests {
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/v1/chat/completions");
        assert!(!request.carried_authorization, "{request:?}");
    }
    assert!(report.audit_entries > 0);
    assert!(report.audit_chain_verified);
    assert!(report.scratch_removed);

    let steps: Vec<&str> = report.steps.iter().map(|step| step.name.as_str()).collect();
    assert_eq!(
        steps,
        [
            "import", "endpoint", "prepare", "generate", "summary", "replay", "audit"
        ],
        "the run did not drive the whole path",
    );

    // What left: nothing that was not the endpoint, in either direction.
    let sockets = &report.sockets;
    assert_eq!(
        sockets.non_loopback_connections, 0,
        "the endpoint path opened a non-loopback connection: {:?}",
        sockets.non_loopback_peers,
    );
    assert!(sockets.non_loopback_peers.is_empty());
    assert!(
        sockets.unexpected_loopback_peers.is_empty(),
        "a loopback peer the run does not explain: {:?}",
        sockets.unexpected_loopback_peers,
    );

    // And the control on that zero. Without it the assertions above would be
    // just as true of a watcher that read nothing at all — which is exactly
    // what a platform with no `/proc` gives, and that case says so by name
    // rather than by passing quietly.
    if !sockets.observed {
        eprintln!("sockets not observed here: {:?}", sockets.observation_note);
        return;
    }
    assert!(
        sockets.saw_the_endpoint,
        "the watcher never caught the connection to the endpoint: {:?}",
        sockets.loopback_peers,
    );
    assert!(sockets.loopback_connections > 0);
    assert!(sockets.samples > 0);
}

/// The same thing through the binary, which is the only way to reach it.
#[test]
fn the_command_reports_ok_and_exits_zero() {
    let output = binary().arg("e1-watch").output().expect("run soul-headless");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("the report is JSON on stdout");
    assert_eq!(report["ok"], serde_json::json!(true));
    assert_eq!(report["sockets"]["non_loopback_connections"], 0);
    assert_eq!(
        report["sockets"]["unexpected_loopback_peers"],
        serde_json::json!([]),
    );
    assert_eq!(
        report["endpoint"]["requests"].as_array().map(Vec::len),
        Some(EXPECTED_REQUESTS),
    );
    assert_eq!(report["scratch_removed"], serde_json::json!(true));
}

/// Nobody hands this instrument an address.
///
/// It runs an endpoint of its own and points the session at that. A command
/// that took a URL would be a supported way to make an installed Soul open a
/// socket to somewhere the person running it chose, which is the shape of
/// thing the whole egress guard exists to prevent.
#[test]
fn the_command_takes_no_argument_of_its_own() {
    let output = binary()
        .args(["e1-watch", "--endpoint"])
        .output()
        .expect("run soul-headless");

    assert!(!output.status.success(), "e1-watch accepted an argument");
    assert!(
        output.stdout.is_empty(),
        "a refused run printed a report anyway",
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--endpoint"),
        "the refusal does not say what it did not understand",
    );
}

/// It writes into a directory it made and deletes, and never reaches for the
/// one an installed Soul keeps its life in.
///
/// `Session::open` is the product's own opener and taking it is the point —
/// this instrument is about the path the shell drives. What it must not take
/// is `open_platform_directory`, which is the same opener pointed at
/// `%LOCALAPPDATA%\Soul`.
#[test]
fn it_runs_against_a_scratch_directory_and_removes_it() {
    assert!(
        SOURCE.contains("scratch_dir_named"),
        "the instrument should be making its own directory",
    );
    assert!(
        SOURCE.contains("remove_dir_all"),
        "the instrument does not clean up after itself",
    );
    assert!(
        !SOURCE.contains("open_platform_directory"),
        "the instrument opens the real library, which is the one place it must not write",
    );
    assert!(
        !SOURCE.contains("data_directory"),
        "the instrument asks where the real library is, and it has no business knowing",
    );
}

/// The endpoint binds loopback and nothing else.
///
/// A structural check rather than a runtime one, because the failure it is
/// about is a one-character edit: `0.0.0.0` in place of `127.0.0.1` would bind
/// every interface on the machine and still pass every assertion above.
#[test]
fn the_endpoint_it_starts_is_loopback_only() {
    assert!(
        SOURCE.contains(r#"bind(("127.0.0.1", 0))"#),
        "the instrument's endpoint does not bind loopback explicitly",
    );
    for anywhere in ["0.0.0.0", "[::]", "::0"] {
        assert!(
            !SOURCE.contains(anywhere),
            "the instrument's endpoint names `{anywhere}`",
        );
    }
}
