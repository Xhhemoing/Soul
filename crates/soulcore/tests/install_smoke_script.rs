//! `scripts/install-smoke.ps1` reads this crate's output. This is the seam.
//!
//! The script runs on Windows and CI's Linux jobs never execute it, so the one
//! failure mode nobody would notice is a rename here: `egress.non_loopback_
//! connections` becomes something else, the script's `-eq 0` compares against
//! `$null`, and the smoke goes green on a machine that is talking to the
//! internet. Every field path the script reads is therefore resolved against a
//! real report produced by a real run.
//!
//! The rest of this file is the part of the script's behaviour that can be
//! checked without a Windows machine: that it installs and uninstalls
//! silently, and that it downloads nothing. PRODUCT_LOCK bans vendor traffic;
//! a packaging script that fetches a bootstrapper is the most likely way for
//! it to come back, and the smallest place to catch it is here.

use serde_json::Value;

const SCRIPT: &str = include_str!("../../../scripts/install-smoke.ps1");

fn report() -> Value {
    let scratch = tempfile::tempdir().expect("temp dir");
    let report = soulcore::headless::run_in(scratch.path()).expect("the main flow");
    serde_json::to_value(report).expect("serialize")
}

/// Field paths the script reads out of the report, pulled from the script
/// itself rather than listed here, so a path the script gains is a path this
/// test starts checking.
fn paths_the_script_reads(script: &str) -> Vec<String> {
    const PREFIX: &str = "$smoke.Report.";
    let mut found: Vec<String> = Vec::new();
    for (index, _) in script.match_indices(PREFIX) {
        let rest = &script[index + PREFIX.len()..];
        let end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'))
            .unwrap_or(rest.len());
        let path = rest[..end].trim_end_matches('.').to_owned();
        if !path.is_empty() && !found.contains(&path) {
            found.push(path);
        }
    }
    found
}

fn resolve<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for segment in path.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

#[test]
fn every_field_the_script_reads_exists_in_the_report() {
    let report = report();
    let paths = paths_the_script_reads(SCRIPT);

    assert!(
        paths.len() >= 6,
        "the script should be reading several fields, found {paths:?}",
    );
    for path in &paths {
        assert!(
            resolve(&report, path).is_some(),
            "install-smoke.ps1 reads `{path}`, which the report does not have: {}",
            serde_json::to_string_pretty(&report).unwrap_or_default(),
        );
    }

    // The two the whole script exists for, spelled out: a rename that kept the
    // scan above happy by deleting the read would still fail here.
    assert!(paths.iter().any(|p| p == "egress.non_loopback_connections"));
    assert!(paths.iter().any(|p| p == "config.fully_closed"));
    assert_eq!(
        resolve(&report, "egress.non_loopback_connections"),
        Some(&Value::from(0)),
    );

    // The scan is not vacuous: a path that is not there is caught.
    assert!(resolve(&report, "egress.no_such_field").is_none());
    assert_eq!(
        paths_the_script_reads("if ($smoke.Report.made_up_field) { }"),
        vec!["made_up_field".to_owned()],
    );
}

/// The step list the script prints comes out of the report, field by field.
#[test]
fn the_step_lines_the_script_prints_are_fields_of_a_step() {
    let report = report();
    let first = report["steps"]
        .as_array()
        .and_then(|steps| steps.first())
        .expect("a first step");

    for field in ["name", "detail"] {
        assert!(first.get(field).is_some(), "a step has no `{field}`");
        assert!(
            SCRIPT.contains(&format!("$step.{field}")),
            "the script does not print `{field}`",
        );
    }
}

/// Silent, in both directions. A packaging smoke that pops a dialog is a
/// packaging smoke nobody can run unattended.
///
/// `tauri.conf.json` builds NSIS, so `/S` is the switch that matters; the
/// msiexec branch is there for a WiX target that does not exist yet, and is
/// checked so it cannot rot into something interactive if it ever runs.
#[test]
fn the_install_and_the_uninstall_are_both_silent() {
    assert!(SCRIPT.contains("@('/S')"), "the NSIS install is not silent");
    assert!(
        SCRIPT.contains("@('/S', \"_?=$directory\")"),
        "the NSIS uninstall is not silent, or does not wait for itself",
    );
    assert!(SCRIPT.contains("'/qn'"), "msiexec is not run quietly");
    assert!(SCRIPT.contains("'/i', $Path"));
    assert!(SCRIPT.contains("'/x', $InstallerPath"));
    assert!(
        SCRIPT.contains("finally {"),
        "the uninstall has to run even when a check above it failed",
    );
}

/// AC-01 is that nothing asks for elevation. The script proves it by being
/// unelevated itself while the installer runs: a bundle that wanted
/// administrator would raise the prompt the criterion forbids.
#[test]
fn the_install_phase_refuses_to_run_as_administrator() {
    assert!(
        SCRIPT.contains("WindowsBuiltInRole]::Administrator"),
        "the script never asks whether it is elevated",
    );
    assert!(
        SCRIPT.contains("-Condition (-not (Test-RunningElevated))"),
        "the elevation question is asked but not asserted on",
    );
    assert!(
        SCRIPT.contains("$entry.Hive.StartsWith('HKCU:')"),
        "a machine-wide install would satisfy every other check in the script",
    );
}

/// PRODUCT_LOCK bans vendor traffic, and packaging is where it usually
/// reappears — a bundler that fetches WiX, a bootstrapper that fetches a
/// runtime. If Soul ever needs one of those, it goes in the author-manual
/// checklist as a stated gap, not into this script.
#[test]
fn the_script_downloads_nothing() {
    for cmdlet in [
        "Invoke-WebRequest",
        "Invoke-RestMethod",
        "Start-BitsTransfer",
        "System.Net.WebClient",
        "curl ",
        "wget ",
        "winget ",
        "Install-Module",
    ] {
        assert!(
            !SCRIPT.contains(cmdlet),
            "install-smoke.ps1 uses `{cmdlet}`, which reaches the network",
        );
    }
    assert!(
        !SCRIPT.contains("://"),
        "install-smoke.ps1 quotes a URL: {:?}",
        SCRIPT.lines().find(|line| line.contains("://")),
    );
}

/// The names the script looks for are the names this build produces.
#[test]
fn the_script_looks_for_the_binaries_this_build_makes() {
    let app = format!("{}.exe", soulcore::commands::shell::DESKTOP_BINARY_NAME);
    assert!(
        SCRIPT.contains(&format!("'{app}'")),
        "the script does not look for {app}, which is what PRODUCT_LOCK names",
    );

    let headless = std::path::Path::new(env!("CARGO_BIN_EXE_soul-headless"))
        .file_stem()
        .expect("a file stem")
        .to_string_lossy()
        .into_owned();
    assert!(
        SCRIPT.contains(&format!("'{headless}.exe'")),
        "the script does not look for {headless}.exe",
    );
    assert!(
        SCRIPT.contains("-ArgumentList @('smoke')"),
        "the script does not ask for the main flow by name",
    );
}
