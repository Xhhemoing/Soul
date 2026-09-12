# G-W: the Windows gate, on the author's Windows 11 machine.
#
# There is no hosted CI for this repository (DECISIONS D50). This script is
# what the old `test-windows` and `package` jobs did, minus the runner: it
# builds the workspace, runs the desktop shell's tests, builds the release
# binaries with the frontend embedded, checks the manifest *inside* soul.exe,
# and runs the AC-21 headless smoke against those binaries.
#
# It does not run `tauri build` (NSIS is fetched over the network the first
# time) and it does not install anything. Those are steps 0-1 of
# scripts/author-manual-checklist.md, done by hand after this passes.
#
# Usage, from the repository root:
#   pwsh -NoProfile -File scripts/gate-win.ps1
#   pwsh -NoProfile -File scripts/gate-win.ps1 -SkipWorkspaceTests   # rerun only the shell + package half
#
# Write the result to docs/gates/<yyyymmdd>-<sha7>-win.md (template in
# docs/gates/README.md). The script prints the table rows it can fill in.

[CmdletBinding()]
param(
    [switch]$SkipWorkspaceTests
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = Resolve-Path (Join-Path $PSScriptRoot '..')
Set-Location $root

$sha = (git rev-parse --short=7 HEAD).Trim()
$branch = (git rev-parse --abbrev-ref HEAD).Trim()
$started = Get-Date

$rows = New-Object System.Collections.Generic.List[string]
function Step {
    param([string]$Name, [scriptblock]$Body)
    $t = [System.Diagnostics.Stopwatch]::StartNew()
    Write-Host "==> $Name" -ForegroundColor Cyan
    try {
        & $Body
        if ($LASTEXITCODE -ne $null -and $LASTEXITCODE -ne 0) {
            throw "exit code $LASTEXITCODE"
        }
        $rows.Add("| $Name | ✅ | $([int]$t.Elapsed.TotalSeconds)s |")
    } catch {
        $rows.Add("| $Name | ❌ | $($_.Exception.Message) |")
        Write-Host "" 
        Write-Host ($rows -join "`n")
        throw "G-W failed at: $Name"
    }
}

Write-Host "G-W on $branch @ $sha ($env:COMPUTERNAME)"
Write-Host ("rustc: " + (rustc -V))
Write-Host ("node:  " + (node -v) + "  pnpm: " + (pnpm -v))

# Build prerequisites for the vendored OpenSSL/SQLCipher build.
Step 'prerequisites (perl, nasm)' {
    perl --version | Select-Object -First 2 | Out-Host
    nasm -v | Out-Host
}

if (-not $SkipWorkspaceTests) {
    # The target platform, so the whole workspace runs here, including
    # crates/soul-store-api/tests/sqlcipher_smoke.rs and the cfg(windows)
    # DPAPI arm.
    Step 'cargo test --workspace --all-targets' {
        cargo test --workspace --all-targets
    }
}

# The desktop shell is its own cargo workspace. `--all-targets` is what the
# G-W definition asks for: on a real Windows 11 machine WebView2Loader is the
# system one, so ipc_roundtrip loads. If it fails to start with
# STATUS_ENTRYPOINT_NOT_FOUND here too, that is a finding for the gate file,
# not something to skip silently.
Step 'cargo test (desktop shell, --all-targets)' {
    cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets
}

Step 'pnpm install --frozen-lockfile' {
    pnpm install --frozen-lockfile
}

Step 'frontend bundle' {
    pnpm --filter '@soul/desktop' build
}

Step 'release binaries' {
    cargo build --release --manifest-path apps/desktop/src-tauri/Cargo.toml --features custom-protocol
    cargo build --release -p soulcore --bin soul-headless
}

# AC-01: the manifest in the binary, not in the config.
Step 'soul.exe manifest is asInvoker' {
    $exe = 'apps/desktop/src-tauri/target/release/soul.exe'
    if (-not (Test-Path $exe)) { throw "the release build produced no $exe" }
    $text = [System.Text.Encoding]::UTF8.GetString([System.IO.File]::ReadAllBytes($exe))
    $start = $text.IndexOf('<assembly')
    if ($start -lt 0) { throw 'soul.exe embeds no application manifest' }
    $manifest = $text.Substring($start, $text.IndexOf('</assembly>', $start) - $start)
    if ($manifest -notmatch 'level\s*=\s*"asInvoker"') { throw 'AC-01: soul.exe does not run as the invoker' }
    if ($manifest -match 'requireAdministrator|highestAvailable') { throw 'AC-01: soul.exe asks for elevation' }
    Write-Host 'soul.exe: asInvoker, no elevation requested'
}

# AC-21 on the platform it ships on, against a release build, without
# installing: the install/uninstall half is the manual checklist.
Step 'headless smoke (release, -SkipInstall)' {
    & ./scripts/install-smoke.ps1 -SkipInstall `
        -Headless ./target/release/soul-headless.exe `
        -AppExecutable ./apps/desktop/src-tauri/target/release/soul.exe
}

$elapsed = [int]((Get-Date) - $started).TotalMinutes
Write-Host ""
Write-Host "G-W green on $branch @ $sha in ${elapsed} min. Paste into docs/gates/$(Get-Date -Format yyyyMMdd)-$sha-win.md:" -ForegroundColor Green
Write-Host ""
Write-Host "| 门 | 结果 | 耗时 |"
Write-Host "|---|---|---|"
Write-Host ($rows -join "`n")
Write-Host ""
Write-Host "Next: scripts/author-manual-checklist.md section 0 (tauri build) and 1 (install / smoke / uninstall)."
