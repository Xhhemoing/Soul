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
#   ... -SkipWorkspaceTests   # rerun only the shell + package half
#
# Write the result to docs/gates/<yyyymmdd>-<sha7>-win.md (template in
# docs/gates/README.md). The script prints the table rows it can fill in.
#
# Failure classes (so operators know which env layer broke):
#   MSVC/SDK | Perl/OpenSSL | Cargo lock | Rust (business compile/test)

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
        $rows.Add("| $Name | OK | $([int]$t.Elapsed.TotalSeconds)s |")
    } catch {
        $rows.Add("| $Name | FAIL | $($_.Exception.Message) |")
        Write-Host ""
        Write-Host ($rows -join "`n")
        throw "G-W failed at: $Name"
    }
}

function FailClass {
    param([ValidateSet('MSVC/SDK','Perl/OpenSSL','Cargo lock','Rust')]$Class, [string]$Message)
    throw "[$Class] $Message"
}

function With-NodeEnv {
    param([ValidateSet('test','production')][string]$Value, [scriptblock]$Body)
    $hadOriginal = Test-Path Env:NODE_ENV
    $original = $env:NODE_ENV
    $env:NODE_ENV = $Value
    try {
        & $Body
    } finally {
        if ($hadOriginal) {
            $env:NODE_ENV = $original
        } else {
            Remove-Item Env:NODE_ENV -ErrorAction SilentlyContinue
        }
    }
}

Write-Host "G-W on $branch @ $sha ($env:COMPUTERNAME)"
Write-Host ("rustc: " + (rustc -V))
Write-Host ("node:  " + (node -v) + "  pnpm: " + (pnpm -v))

Step 'prerequisites (MSVC/SDK)' {
    $linkCmd = Get-Command link.exe -ErrorAction SilentlyContinue
    if (-not $linkCmd) {
        FailClass MSVC/SDK "link.exe not found. Install VS 2022 Build Tools (Desktop C++) and put MSVC Hostx64\x64 ahead of Git usr\bin on PATH."
    }
    $linkPath = $linkCmd.Source
    Write-Host "link: $linkPath"
    if ($linkPath -match '[\\/]Git[\\/]usr[\\/]bin[\\/]link\.exe$') {
        FailClass MSVC/SDK "PATH resolves link.exe to Git usr\bin (GNU). Put MSVC VC\Tools\MSVC\...\bin\Hostx64\x64 before Git\usr\bin."
    }
    if ($linkPath -match '[\\/]Go[\\/]') {
        FailClass MSVC/SDK "PATH resolves link.exe to Go's linker, not MSVC."
    }
    $kitRoots = @(
        "${env:ProgramFiles(x86)}\Windows Kits\10\Lib",
        "${env:ProgramFiles}\Windows Kits\10\Lib"
    ) | Where-Object { Test-Path $_ }
    $kernel = $null
    foreach ($root in $kitRoots) {
        $x64 = Get-ChildItem -Path $root -Filter kernel32.lib -Recurse -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -match '[\\/]um[\\/]x64[\\/]kernel32\.lib$' } |
            Select-Object -First 1
        if ($x64) { $kernel = $x64; break }
    }
    if (-not $kernel) {
        $kernel = $kitRoots | ForEach-Object {
            Get-ChildItem -Path $_ -Filter kernel32.lib -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1
        } | Select-Object -First 1
    }
    if (-not $kernel) {
        FailClass MSVC/SDK "Windows SDK import libs missing (kernel32.lib)."
    }
    Write-Host "kernel32.lib: $($kernel.FullName)"
}

Step 'prerequisites (Perl/OpenSSL smoke)' {
    $perlCmd = Get-Command perl.exe -ErrorAction SilentlyContinue
    if (-not $perlCmd) {
        FailClass 'Perl/OpenSSL' "perl.exe not found (needed for vendored OpenSSL/SQLCipher Configure)."
    }
    Write-Host "perl: $($perlCmd.Source)"
    perl --version 2>&1 | Select-Object -First 2 | Out-Host
    # Locale::Maketext::Simple is required by OpenSSL's Configure on Windows.
    $lms = & perl -MLocale::Maketext::Simple -e "print 'LMS_OK'" 2>&1
    if ($LASTEXITCODE -ne 0 -or "$lms" -notmatch 'LMS_OK') {
        FailClass 'Perl/OpenSSL' "perl -MLocale::Maketext::Simple failed. Install Strawberry Perl (or add LMS); Git usr\bin perl alone is not enough. Output: $lms"
    }
    Write-Host "Locale::Maketext::Simple: OK"
    $nasmCmd = Get-Command nasm.exe -ErrorAction SilentlyContinue
    if (-not $nasmCmd) {
        FailClass 'Perl/OpenSSL' "nasm.exe not found (OpenSSL assembly)."
    }
    nasm -v | Out-Host
}

Step 'prerequisites (Cargo lock)' {
    $desktopLock = 'apps/desktop/src-tauri/Cargo.lock'
    $desktopToml = 'apps/desktop/src-tauri/Cargo.toml'
    if (-not (Test-Path $desktopLock)) {
        FailClass 'Cargo lock' "missing $desktopLock"
    }
    if (-not (Test-Path $desktopToml)) {
        FailClass 'Cargo lock' "missing $desktopToml"
    }
    # Native cargo may print MSRV-resolver warnings on stderr under 1.83; do not
    # treat those as failures under $ErrorActionPreference Stop.
    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        & cargo metadata --locked --format-version 1 --no-deps 1>$null 2>$null
        if ($LASTEXITCODE -ne 0) {
            FailClass 'Cargo lock' "workspace Cargo.lock rejects --locked (regenerate and audit the diff)."
        }
        Push-Location 'apps/desktop/src-tauri'
        try {
            & cargo metadata --locked --format-version 1 --no-deps 1>$null 2>$null
            if ($LASTEXITCODE -ne 0) {
                FailClass 'Cargo lock' "apps/desktop/src-tauri/Cargo.lock rejects --locked (cargo +stable generate-lockfile, audit diff)."
            }
        } finally {
            Pop-Location
        }
    } finally {
        $ErrorActionPreference = $prevEap
    }
    Write-Host "Cargo.lock --locked: workspace + desktop OK"
}

if (-not $SkipWorkspaceTests) {
    Step 'cargo test --workspace --all-targets --locked' {
        cargo test --workspace --all-targets --locked
        if ($LASTEXITCODE -ne 0) { FailClass Rust "workspace tests failed" }
    }
}

Step 'cargo test (desktop shell, --all-targets --locked)' {
    cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --locked
    if ($LASTEXITCODE -ne 0) { FailClass Rust "desktop shell tests failed" }
}

Step 'pnpm install --frozen-lockfile' {
    pnpm install --frozen-lockfile
}

Step 'frontend lint' {
    pnpm --filter '@soul/desktop' lint
}

Step 'frontend tests' {
    With-NodeEnv test { pnpm --filter '@soul/desktop' test }
}

Step 'frontend bundle' {
    With-NodeEnv production { pnpm --filter '@soul/desktop' build }
}

Step 'release binaries (--locked)' {
    cargo build --release --manifest-path apps/desktop/src-tauri/Cargo.toml --features custom-protocol --locked
    if ($LASTEXITCODE -ne 0) { FailClass Rust "desktop release build failed" }
    cargo build --release -p soulcore --bin soul-headless --locked
    if ($LASTEXITCODE -ne 0) { FailClass Rust "soul-headless release build failed" }
}

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

Step 'headless smoke (release, -SkipInstall)' {
    & ./scripts/install-smoke.ps1 -SkipInstall `
        -Headless ./target/release/soul-headless.exe `
        -AppExecutable ./apps/desktop/src-tauri/target/release/soul.exe
}

$elapsed = [int]((Get-Date) - $started).TotalMinutes
Write-Host ""
Write-Host "G-W green on $branch @ $sha in ${elapsed} min. Paste into docs/gates/$(Get-Date -Format yyyyMMdd)-$sha-win.md:" -ForegroundColor Green
Write-Host ""
Write-Host "| step | result | duration |"
Write-Host '|---|---|---|'
Write-Host ($rows -join "`n")
Write-Host ""
Write-Host "Next: scripts/author-manual-checklist.md section 0 (tauri build) and 1 (install / smoke / uninstall)."
Write-Host "Do not claim Goal1 / G-W closed until this paste is frozen and Reviewer PASS."
