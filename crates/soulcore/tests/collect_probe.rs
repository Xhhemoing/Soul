//! The manual probe, checked for the things a machine can check.
//!
//! What it measures — AC-09 and AC-10 against a desktop somebody is switching
//! between — is by definition not testable here; that is why it exists. What
//! is testable is everything around the measurement: that it will not sample
//! anybody without being told to, that on a host with no foreground source it
//! says so instead of reporting a comfortable zero, and that it never opens
//! the store the user's life is in.

use std::process::Command;
#[cfg(not(windows))]
use std::time::Duration;

const SOURCE: &str = include_str!("../src/collect_probe.rs");

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_soul-headless"))
}

/// Nothing samples the desktop by accident. The flag is the consent.
#[test]
fn the_probe_collects_nothing_until_the_caller_asks_for_it() {
    let output = binary()
        .arg("collect-probe")
        .output()
        .expect("run soul-headless");

    assert!(
        !output.status.success(),
        "collect-probe ran without being consented to",
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--i-consent"),
        "the refusal does not say how to consent: {stderr}",
    );
    assert!(
        output.stdout.is_empty(),
        "a refused probe printed a report anyway",
    );
}

/// A run that measured nothing has to fail, not pass quietly. On Linux there
/// is no foreground source at all, which is the strongest form of that case.
#[test]
#[cfg(not(windows))]
fn a_host_with_no_foreground_source_fails_instead_of_reporting_zero() {
    let error = soulcore::collect_probe::run(Duration::from_millis(10))
        .expect_err("Linux has no foreground collector");

    assert_eq!(error.step, "collect-probe");
    assert!(
        error.detail.contains("no foreground source"),
        "the failure does not say why: {}",
        error.detail,
    );

    // And it left nothing behind on the way out.
    let leftovers: Vec<_> = std::fs::read_dir(std::env::temp_dir())
        .expect("temp dir")
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(&format!("soul-collect-probe-{}-", std::process::id()))
        })
        .collect();
    assert!(
        leftovers.is_empty(),
        "the probe left {} scratch directory(ies) behind",
        leftovers.len(),
    );
}

/// The probe writes into a directory it made and deletes; the encrypted
/// library the user actually has is never opened. `open_test_store` takes its
/// key from a seed, so a probe that used `open_store` would be reaching for
/// the platform key provider — and for the real database beside it.
#[test]
fn the_probe_never_opens_the_users_library() {
    assert!(
        SOURCE.contains("open_test_store"),
        "the probe should be using a scratch store",
    );
    assert!(
        !SOURCE.contains("open_store("),
        "the probe opens the real store, which is the one place it must not write",
    );
    assert!(
        SOURCE.contains("remove_dir_all"),
        "the probe does not clean up after itself",
    );
}

/// AC-10 is measured by *revoking*, not by stopping: the collector has to
/// notice on its own. A probe that called `stop` first would pass on a build
/// where revocation did nothing.
#[test]
fn the_second_phase_revokes_before_it_stops() {
    let revoke = SOURCE.find("consent.revoke(").expect("a revocation");
    let stop = SOURCE
        .find("collect_commands::stop(handle)")
        .expect("a stop");
    assert!(
        revoke < stop,
        "the probe stops the collector before revoking, so it never measures the revocation",
    );
}
