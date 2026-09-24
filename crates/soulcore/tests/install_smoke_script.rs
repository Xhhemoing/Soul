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
    // F1: Invoke-HeadlessSmoke uses ProcessStartInfo so ExitCode is real on
    // PS 5.1; Start-Process -ArgumentList @('smoke') was the old form.
    assert!(
        SCRIPT.contains("-ArgumentList @('smoke')") || SCRIPT.contains("Arguments = 'smoke'"),
        "the script does not ask for the main flow by name",
    );
}

/// The install fallback must not treat the data directory as the install dir.
#[test]
fn the_script_finds_the_installer_under_programs_not_the_data_directory() {
    assert!(
        SCRIPT.contains("'Programs'") || SCRIPT.contains("\"Programs\""),
        "install-smoke.ps1 must fall back to %LOCALAPPDATA%\\Programs\\Soul for soul.exe",
    );
    assert!(
        !SCRIPT.contains("Join-Path $env:LOCALAPPDATA $script:ProductName)"),
        "install-smoke.ps1 must not search %LOCALAPPDATA%\\Soul as the install location",
    );
}

/// Finding that the fallback list is right is not the same as checking what
/// was found. `$entry.InstallLocation` is searched before the Programs
/// fallback, so a bundle that registered `%LOCALAPPDATA%\Soul` as its install
/// location, which the `2e72ddf` NSIS bug did, hands the script a `soul.exe`
/// out of the data directory. It passes the name and manifest checks, and
/// then the uninstaller is aimed at `keys.dpapi`. The verify phase therefore
/// asserts where the installed binary actually is.
#[test]
fn the_verify_phase_asserts_the_installed_soul_exe_is_in_programs_not_the_data_directory() {
    // Only for a run that installed a bundle: CI's -SkipInstall points
    // -AppExecutable at a built target/release/soul.exe, which is under
    // neither directory and must not be required to be under Programs.
    assert!(
        SCRIPT.contains("if ($willInstall -and $AppExecutable) {"),
        "the install-location checks are not gated on this run having installed something",
    );

    assert!(
        SCRIPT.contains(
            "$programsInstall = Join-Path (Join-Path $env:LOCALAPPDATA 'Programs') $script:ProductName"
        ),
        "the verify phase never builds %LOCALAPPDATA%\\Programs\\Soul to compare against",
    );
    assert!(
        SCRIPT.contains("$dataDirectory = Join-Path $env:LOCALAPPDATA $script:ProductName"),
        "the verify phase never builds %LOCALAPPDATA%\\Soul to compare against",
    );

    // Under Programs\Soul...
    assert!(
        SCRIPT.contains("'the installed soul.exe is under Programs'"),
        "nothing records where the installed soul.exe came from",
    );
    assert!(
        SCRIPT.contains(
            "-Condition (Test-PathIsUnder -Path $AppExecutable -Directory $programsInstall)"
        ),
        "the Programs finding is recorded but not asserted on",
    );

    // ...and not under the data directory. The two are different questions:
    // %LOCALAPPDATA%\Soul is not a prefix of %LOCALAPPDATA%\Programs\Soul.
    assert!(
        SCRIPT.contains("'the installed soul.exe is not in the data directory'"),
        "nothing fails the run when soul.exe was installed into %LOCALAPPDATA%\\Soul",
    );
    assert!(
        SCRIPT.contains(
            "-Condition (-not (Test-PathIsUnder -Path $AppExecutable -Directory $dataDirectory))"
        ),
        "the data-directory finding is recorded but not asserted on",
    );

    // Both are Assert-Finding, which throws, rather than Add-Finding.
    for check in [
        "'the installed soul.exe is under Programs'",
        "'the installed soul.exe is not in the data directory'",
    ] {
        assert!(
            SCRIPT
                .lines()
                .any(|line| line.contains(check) && line.contains("Assert-Finding")),
            "{check} does not fail the run",
        );
    }

    // The comparison is a prefix match with a separator appended, so
    // Programs\SoulSomething is not mistaken for Programs\Soul.
    assert!(
        SCRIPT.contains("$root + $separator"),
        "Test-PathIsUnder matches a bare prefix, so Programs\\SoulSomething would pass",
    );
    assert!(
        SCRIPT.contains("$separator = [System.IO.Path]::DirectorySeparatorChar"),
        "Test-PathIsUnder does not use a directory separator at all",
    );

    // And what the registry says, since that is what phase 4 hands the
    // uninstaller when it looks for uninstall.exe.
    assert!(
        SCRIPT.contains("'the registered InstallLocation is not the data directory'")
            && SCRIPT
                .contains("'the registered InstallLocation is the Programs install directory'"),
        "InstallLocation is trusted to find uninstall.exe but never checked",
    );
    assert!(
        SCRIPT.contains(
            "-Condition (-not (Test-PathIsUnder -Path $entry.InstallLocation -Directory $dataDirectory))"
        ),
        "an InstallLocation pointing at the data directory would still be uninstalled from",
    );
}

/// The uninstaller a bad install registered does not get to run: refusing is
/// the difference between reporting that `keys.dpapi` was deleted and not
/// deleting it.
#[test]
fn a_soul_exe_in_the_data_directory_stops_the_uninstaller_running() {
    assert!(
        SCRIPT.contains("$script:UninstallIsUnsafe = $true"),
        "nothing marks an install that landed in the data directory as unsafe to uninstall",
    );
    for guard in [
        "if (Test-PathIsUnder -Path $AppExecutable -Directory $dataDirectory) {",
        "if (Test-PathIsUnder -Path $entry.InstallLocation -Directory $dataDirectory) {",
    ] {
        assert!(
            SCRIPT.contains(guard),
            "nothing checks `{guard}` before phase 4 runs an uninstaller",
        );
    }
    assert!(
        SCRIPT.contains("if ($installed -and $script:UninstallIsUnsafe) {"),
        "phase 4 runs the uninstaller even when soul.exe came out of the data directory",
    );
    assert!(
        SCRIPT.contains("'the uninstaller was not run'"),
        "the skipped uninstall is not recorded as a finding",
    );
    assert!(
        SCRIPT.contains("-Check 'the uninstaller was not run' -Passed $false"),
        "a skipped uninstall has to fail the run, not pass it quietly",
    );
}

/// Uninstall has to be shown to have spared the store, not assumed to have.
///
/// The NSIS `PREUNINSTALL` hook refuses when `$INSTDIR` is the data directory,
/// and the test below proves this script deletes nothing there itself. Neither
/// says what the uninstaller did to `keys.dpapi`, and on a clean machine there
/// is nothing to say: `soul-headless smoke` runs in a scratch store and
/// deletes it, so phase 3 never creates the real one. Hence the witness — a
/// sentinel planted when the file is absent, a fingerprint when it is not, and
/// a byte-identity check on the far side of the uninstall.
#[test]
fn the_uninstall_phase_proves_the_user_data_files_survived() {
    let blob = soulcore::commands::session::KEY_BLOB_FILE_NAME;
    let database = soulcore::commands::store::DATABASE_FILE_NAME;
    assert_eq!(blob, "keys.dpapi");
    assert_eq!(database, "soul.db");

    for name in [blob, database] {
        assert!(
            SCRIPT.contains(&format!("'{name}'")),
            "install-smoke.ps1 never names {name}, so nothing checks it outlived the uninstaller",
        );
    }
    assert!(
        SCRIPT.contains("is still in the data directory"),
        "the uninstall phase records no finding about the data directory",
    );

    // The directory it looks in is %LOCALAPPDATA%\Soul, not the install tree
    // under %LOCALAPPDATA%\Programs\Soul.
    assert!(
        SCRIPT.contains("Join-Path $env:LOCALAPPDATA $script:ProductName"),
        "the uninstall phase does not look at the data directory at all",
    );

    // Existing and still there are two different questions, and both are asked.
    assert!(SCRIPT.contains("New-UserDataWitness"));
    assert!(
        SCRIPT.contains("Test-FingerprintsMatch"),
        "the files are checked for existence but not for content",
    );
    assert!(
        SCRIPT.contains("soul-install-smoke-witness"),
        "a clean machine has no store to spare, so the script has to plant one",
    );

    // And the sentinel is taken back: a fake keys.dpapi left on the machine is
    // the first thing a real first launch would try to open.
    assert!(
        SCRIPT.contains("Remove-PlantedWitness"),
        "a planted sentinel is never removed",
    );
    assert!(
        SCRIPT.contains("if (-not $witness.Planted) { continue }"),
        "the cleanup does not distinguish a sentinel from the user's own store",
    );
}

/// Uninstall must remove only the install tree, never the DPAPI/database
/// directory. The sentinel cleanup above deletes a path held in a variable,
/// which is fine; what is banned is naming the data directory on a line that
/// deletes.
#[test]
fn the_script_does_not_delete_the_data_directory() {
    for pattern in ["Remove-Item", "Remove–Item", "RMDir", "RmDir"] {
        if !SCRIPT.contains(pattern) {
            continue;
        }
        for line in SCRIPT.lines() {
            if !line.contains(pattern) {
                continue;
            }
            assert!(
                !line.contains("Join-Path $env:LOCALAPPDATA $script:ProductName")
                    && !line.contains("%LOCALAPPDATA%\\Soul")
                    && !line.contains("\\Soul'")
                    && !line.contains("\\Soul\""),
                "install-smoke.ps1 must not delete the data directory: {line}",
            );
        }
    }
}

/// A JSON success claim must never replace the real child-process status.
/// Runs the actual PowerShell watcher with synthetic executables, including
/// invalid JSON and large UTF-8 stdout/stderr.
#[cfg(windows)]
#[test]
fn the_process_watcher_preserves_exit_status_and_captures_both_streams() {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/test-install-smoke-process.ps1");
    let output = std::process::Command::new("pwsh")
        .args(["-NoProfile", "-File"])
        .arg(script)
        .output()
        .expect("pwsh is required by the Windows gate");
    assert!(
        output.status.success(),
        "process watcher regression failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// No TCP sample is missing evidence, not a clean observation.
#[cfg(windows)]
#[test]
fn the_tcp_watch_requires_observation_before_it_can_pass() {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/test-install-smoke-watch.ps1");
    let output = std::process::Command::new("pwsh")
        .args(["-NoProfile", "-File"])
        .arg(script)
        .output()
        .expect("pwsh is required by the Windows gate");
    assert!(
        output.status.success(),
        "TCP evidence regression failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// Provider errors and missing monitor access must not become clean samples.
#[cfg(windows)]
#[test]
fn the_tcp_watch_fails_closed_on_provider_errors() {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/test-install-smoke-tcp-provider.ps1");
    let output = std::process::Command::new("pwsh")
        .args(["-NoProfile", "-File"])
        .arg(script)
        .output()
        .expect("pwsh is required by the Windows gate");
    assert!(
        output.status.success(),
        "TCP provider regression failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// Quoted NSIS registry paths must reach guards and uninstaller as paths.
#[cfg(windows)]
#[test]
fn the_registry_reader_decodes_install_location_without_changing_commands() {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/test-install-smoke-location.ps1");
    let output = std::process::Command::new("pwsh")
        .args(["-NoProfile", "-File"])
        .arg(script)
        .output()
        .expect("pwsh is required by the Windows gate");
    assert!(
        output.status.success(),
        "InstallLocation regression failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// Waiting must cover actual cleanup without locking the installed uninstaller.
#[cfg(windows)]
#[test]
fn the_waited_uninstaller_removes_its_tree_without_deleting_unknown_files() {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/test-install-smoke-uninstall.ps1");
    let output = std::process::Command::new("pwsh")
        .args(["-NoProfile", "-File"])
        .arg(script)
        .output()
        .expect("pwsh is required by the Windows gate");
    assert!(
        output.status.success(),
        "Waited-uninstaller regression failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
