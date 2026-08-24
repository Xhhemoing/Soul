//! Crash injection that kills the process for real.
//!
//! AC-24 asks what survives a crash mid-write. A test that returns `Err` from
//! a mocked call proves nothing about that: no unwinding happens in a real
//! power loss, destructors do not run, and the database file is left exactly
//! as the operating system last saw it.
//!
//! So the harness re-executes the current test binary, selects one `#[test]`
//! by name, arms a [`fail`] point through the `FAILPOINTS` environment
//! variable, and lets the child abort inside the write. The parent then
//! reopens whatever the child left behind and checks it.

use std::path::PathBuf;
use std::process::{Command, ExitStatus};

use anyhow::{Context, Result};

/// Set in the child so a scenario test knows it is the one meant to die.
pub const CHILD_MARKER_ENV: &str = "SOUL_CRASH_CHILD";

/// Named injection sites. Production code arms them with `fail::fail_point!`.
pub mod failpoints {
    /// Inside the audit append, after the entry is built but before commit.
    pub const AUDIT_APPEND_PRE_COMMIT: &str = "soul::audit::append::pre_commit";
    /// After the audit row is written but before the caller is told so.
    pub const AUDIT_APPEND_POST_WRITE: &str = "soul::audit::append::post_write";
    /// Halfway through committing a batch of events.
    pub const STORE_EVENT_COMMIT_MID: &str = "soul::store::event::commit_mid";
    /// Between destroying one content key and the next.
    pub const FORGET_CK_DELETE_MID: &str = "soul::forget::content_key::delete_mid";

    /// Every site, for tests that assert the set has not silently changed.
    pub const ALL: &[&str] = &[
        AUDIT_APPEND_PRE_COMMIT,
        AUDIT_APPEND_POST_WRITE,
        STORE_EVENT_COMMIT_MID,
        FORGET_CK_DELETE_MID,
    ];
}

/// A child run: which test to execute and which fail points to arm.
#[derive(Debug, Clone)]
pub struct CrashScenario {
    /// Exact test path, as `cargo test -- --exact` expects.
    pub test_name: String,
    /// `FAILPOINTS` value, for example `"name=panic"`.
    pub failpoints: String,
    /// Extra environment for the child, typically the file to write to.
    pub env: Vec<(String, String)>,
    /// Test binary to re-execute. Defaults to the current executable.
    pub binary: Option<PathBuf>,
}

impl CrashScenario {
    pub fn new(test_name: impl Into<String>, failpoints: impl Into<String>) -> Self {
        CrashScenario {
            test_name: test_name.into(),
            failpoints: failpoints.into(),
            env: Vec::new(),
            binary: None,
        }
    }

    /// Arm one fail point with `panic`, which [`abort_if_child`] turns into a
    /// process-level abort.
    pub fn panic_at(test_name: impl Into<String>, failpoint: &str) -> Self {
        CrashScenario::new(test_name, format!("{failpoint}=panic"))
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }
}

#[derive(Debug, Clone)]
pub struct CrashOutcome {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

impl CrashOutcome {
    /// The child failed to finish normally, which is the point.
    pub fn died(&self) -> bool {
        !self.status.success()
    }

    /// Panics unless the child really died, printing what it managed to say.
    pub fn assert_died(&self, context: &str) {
        assert!(
            self.died(),
            "{context}: the child exited cleanly ({:?}), so nothing was actually interrupted\n--- stdout ---\n{}\n--- stderr ---\n{}",
            self.status.code(),
            self.stdout,
            self.stderr,
        );
    }
}

/// Re-execute this test binary so one test dies inside an armed fail point.
///
/// Returns once the child is gone; the caller then inspects whatever state the
/// child left on disk.
pub fn run_crashing_subprocess(scenario: &CrashScenario) -> Result<CrashOutcome> {
    let binary = match &scenario.binary {
        Some(path) => path.clone(),
        None => std::env::current_exe().context("locating the current test binary")?,
    };

    let mut command = Command::new(&binary);
    command
        .arg(&scenario.test_name)
        .arg("--exact")
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env(CHILD_MARKER_ENV, "1")
        .env("FAILPOINTS", &scenario.failpoints)
        .env("RUST_BACKTRACE", "0");
    for (key, value) in &scenario.env {
        command.env(key, value);
    }

    let output = command.output().with_context(|| {
        format!(
            "re-executing {} for {}",
            binary.display(),
            scenario.test_name
        )
    })?;

    Ok(CrashOutcome {
        status: output.status,
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

/// True when this process is the re-executed child.
pub fn is_child() -> bool {
    std::env::var_os(CHILD_MARKER_ENV).is_some()
}

/// In the child, turn any panic into an immediate abort.
///
/// Unwinding would run destructors and let the test harness tidy up, which is
/// precisely the behaviour a crash does not have.
pub fn abort_if_child() {
    if !is_child() {
        return;
    }
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        previous(info);
        std::process::abort();
    }));
}
