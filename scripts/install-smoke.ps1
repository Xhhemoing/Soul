#Requires -Version 5.1
<#
.SYNOPSIS
    Install Soul silently, prove the installed build starts and reaches
    nothing, and uninstall it again.

.DESCRIPTION
    AC-01 says: on a clean Windows 11 machine, install and start Soul; the tray
    icon appears and nothing asks for elevation. AC-21 says: run the main flow
    on the default configuration and observe zero non-loopback connections.
    A CI runner can prove neither by compiling code, so this is the script that
    does it on a real machine.

    Three phases, and the exit code is the whole interface:

      1. msiexec /qn installs the package. Nothing is downloaded: if the
         bundle needs a WebView2 runtime or a bootstrapper it is not this
         script's job to fetch one, and a smoke test that installs extra
         software is not testing the thing that shipped.
      2. The installed soul.exe is inspected — its embedded manifest must say
         asInvoker — and soul-headless.exe runs the AC-21 main flow while this
         script watches the operating system's own TCP table for that process.
         Exit 0 and zero non-loopback connections, or the run fails.
      3. msiexec /x /qn removes it, and the install directory has to be gone.

.NOTES
    Known gap, stated rather than worked around: the Tauri bundle contains
    soul.exe and not soul-headless.exe, so -Headless points at the headless
    binary built from the same commit rather than at one the installer placed.
    Until the bundle ships it, phase 2 proves that this build's core runs
    closed on this machine; it does not prove the installer put that core
    there. See docs/STATUS.md, WP13.

    The connection watch reads Get-NetTCPConnection, which covers TCP only.
    UDP has no remote address in the Windows endpoint table, so a UDP peer is
    invisible here. The Linux side of AC-21 (soulcore::netwatch) reads both,
    and the two together are the coverage.

.PARAMETER Installer
    Path to the .msi produced by `tauri build`. Omit it, or pass -SkipInstall,
    to run only the headless phase against binaries that are already present.

.PARAMETER Headless
    Path to soul-headless.exe. Defaults to the release build in target/.

.PARAMETER AppExecutable
    Path to soul.exe, for the manifest check. Defaults to the installed copy
    when an installer was given, otherwise to the release build in target/.

.PARAMETER SkipInstall
    Run phase 2 only. This is what CI does: no runner may install a package
    machine-wide, but every runner can run the binary and watch its sockets.

.PARAMETER DryRun
    Validate the arguments, print the plan, and change nothing. CI runs this
    to check that the script still does what it says without needing an MSI.

.PARAMETER LogDirectory
    Where msiexec logs and the headless report are written. Defaults to a new
    directory under the temp folder.

.EXAMPLE
    pwsh -File scripts/install-smoke.ps1 -Installer .\Soul_0.1.0_x64_en-US.msi

.EXAMPLE
    pwsh -File scripts/install-smoke.ps1 -SkipInstall `
        -Headless .\target\release\soul-headless.exe
#>
[CmdletBinding()]
param(
    [string] $Installer,
    [string] $Headless,
    [string] $AppExecutable,
    [switch] $SkipInstall,
    [switch] $DryRun,
    [string] $LogDirectory
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# The name PRODUCT_LOCK pins. A bundle that installs something else has
# renamed the process the user sees in Task Manager.
$script:ExecutableName = 'soul.exe'
$script:HeadlessName = 'soul-headless.exe'

# Addresses that are not egress. Anything else the installed process connects
# to fails the run.
$script:LoopbackAddresses = @('127.0.0.1', '::1', '0.0.0.0', '::')

$script:Findings = [System.Collections.Generic.List[object]]::new()

function Add-Finding {
    param(
        [Parameter(Mandatory)] [string] $Phase,
        [Parameter(Mandatory)] [string] $Check,
        [Parameter(Mandatory)] [bool] $Passed,
        [string] $Detail = ''
    )

    $script:Findings.Add([pscustomobject]@{
            Phase  = $Phase
            Check  = $Check
            Passed = $Passed
            Detail = $Detail
        })

    $mark = if ($Passed) { 'ok  ' } else { 'FAIL' }
    Write-Host ("  {0} [{1}] {2}{3}" -f $mark, $Phase, $Check, $(if ($Detail) { " — $Detail" } else { '' }))
}

function Assert-Finding {
    param(
        [Parameter(Mandatory)] [string] $Phase,
        [Parameter(Mandatory)] [string] $Check,
        [Parameter(Mandatory)] [bool] $Condition,
        [string] $Detail = ''
    )

    Add-Finding -Phase $Phase -Check $Check -Passed $Condition -Detail $Detail
    if (-not $Condition) {
        throw "$Phase / $Check failed: $Detail"
    }
}

function Resolve-OptionalPath {
    param([string] $Path)

    if ([string]::IsNullOrWhiteSpace($Path)) { return $null }
    try { return (Resolve-Path -LiteralPath $Path -ErrorAction Stop).Path }
    catch { return [System.IO.Path]::GetFullPath($Path) }
}

function Get-RepositoryRoot {
    return (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
}

# The embedded application manifest, read out of the PE image as text.
#
# A byte scan rather than a resource API call: the manifest is stored as UTF-8
# XML inside the executable, `mt.exe` is not on every machine, and the question
# — does this binary ask for elevation — is answerable by looking for the words
# that would ask.
function Get-EmbeddedManifest {
    param([Parameter(Mandatory)] [string] $Path)

    $bytes = [System.IO.File]::ReadAllBytes($Path)
    $text = [System.Text.Encoding]::UTF8.GetString($bytes)
    $start = $text.IndexOf('<assembly')
    if ($start -lt 0) { return $null }
    $end = $text.IndexOf('</assembly>', $start)
    if ($end -lt 0) { return $null }
    return $text.Substring($start, $end - $start + '</assembly>'.Length)
}

function Test-ManifestIsAsInvoker {
    param([Parameter(Mandatory)] [string] $Path)

    $manifest = Get-EmbeddedManifest -Path $Path
    Assert-Finding -Phase 'verify' -Check 'the executable embeds a manifest' `
        -Condition ($null -ne $manifest) -Detail $Path

    Assert-Finding -Phase 'verify' -Check 'requestedExecutionLevel is asInvoker' `
        -Condition ($manifest -match 'level\s*=\s*"asInvoker"') `
        -Detail 'AC-01: Soul does not ask for elevation to run'

    foreach ($elevated in @('requireAdministrator', 'highestAvailable')) {
        Assert-Finding -Phase 'verify' -Check "the manifest does not ask for $elevated" `
            -Condition (-not ($manifest -match [regex]::Escape($elevated)))
    }

    Assert-Finding -Phase 'verify' -Check 'uiAccess is false' `
        -Condition ($manifest -match 'uiAccess\s*=\s*"false"')
}

function Invoke-Msiexec {
    param(
        [Parameter(Mandatory)] [string[]] $Arguments,
        [Parameter(Mandatory)] [string] $LogPath
    )

    $all = $Arguments + @('/qn', '/norestart', '/l*v', $LogPath)
    Write-Host "  running: msiexec.exe $($all -join ' ')"
    $process = Start-Process -FilePath 'msiexec.exe' -ArgumentList $all -Wait -PassThru
    return $process.ExitCode
}

# Run the headless smoke and watch what the operating system says its sockets
# are doing while it runs.
function Invoke-HeadlessSmoke {
    param(
        [Parameter(Mandatory)] [string] $Path,
        [Parameter(Mandatory)] [string] $LogDirectory
    )

    $stdout = Join-Path $LogDirectory 'headless-stdout.json'
    $stderr = Join-Path $LogDirectory 'headless-stderr.txt'

    $process = Start-Process -FilePath $Path -NoNewWindow -PassThru `
        -RedirectStandardOutput $stdout -RedirectStandardError $stderr

    $canWatch = $null -ne (Get-Command -Name 'Get-NetTCPConnection' -ErrorAction SilentlyContinue)
    $seen = [System.Collections.Generic.HashSet[string]]::new()
    $samples = 0

    while (-not $process.HasExited) {
        if ($canWatch) {
            $samples++
            foreach ($connection in Get-NetTCPConnection -OwningProcess $process.Id -ErrorAction SilentlyContinue) {
                if ($script:LoopbackAddresses -contains $connection.RemoteAddress) { continue }
                if ($connection.State -eq 'Listen') { continue }
                [void] $seen.Add("$($connection.RemoteAddress):$($connection.RemotePort)")
            }
        }
        Start-Sleep -Milliseconds 20
    }
    $process.WaitForExit()

    $report = $null
    if (Test-Path -LiteralPath $stdout) {
        $text = Get-Content -LiteralPath $stdout -Raw
        if (-not [string]::IsNullOrWhiteSpace($text)) {
            $report = $text | ConvertFrom-Json
        }
    }

    return [pscustomobject]@{
        ExitCode         = $process.ExitCode
        Report           = $report
        StdoutPath       = $stdout
        StderrPath       = $stderr
        WatchedByThisScript = $canWatch
        Samples          = $samples
        NonLoopbackPeers = @($seen)
    }
}

function Show-Plan {
    param([Parameter(Mandatory)] [hashtable] $Plan)

    Write-Host 'install-smoke plan:'
    foreach ($key in $Plan.Keys | Sort-Object) {
        Write-Host ("  {0,-16} {1}" -f $key, $Plan[$key])
    }
}

# ----------------------------------------------------------------- main ---

$root = Get-RepositoryRoot
$Installer = Resolve-OptionalPath $Installer
$Headless = Resolve-OptionalPath $Headless
$AppExecutable = Resolve-OptionalPath $AppExecutable

if (-not $Headless) {
    $Headless = Join-Path $root "target/release/$script:HeadlessName"
}
if ([string]::IsNullOrWhiteSpace($LogDirectory)) {
    $LogDirectory = Join-Path ([System.IO.Path]::GetTempPath()) "soul-install-smoke-$PID"
}

$willInstall = (-not $SkipInstall) -and $Installer

$plan = @{
    'installer'  = if ($willInstall) { $Installer } else { '(skipped)' }
    'headless'   = $Headless
    'executable' = if ($AppExecutable) { $AppExecutable } else { '(from the install directory)' }
    'logs'       = $LogDirectory
    'phases'     = if ($willInstall) { 'install, verify, smoke, uninstall' } else { 'verify, smoke' }
}

if ($DryRun) {
    Show-Plan -Plan $plan
    Write-Host 'install-smoke: dry run, nothing was installed or started.'
    exit 0
}

if ((-not $SkipInstall) -and (-not $Installer)) {
    Write-Error 'install-smoke: pass -Installer <path to .msi>, or -SkipInstall to run the headless phase only.'
    exit 2
}

New-Item -ItemType Directory -Path $LogDirectory -Force | Out-Null
Show-Plan -Plan $plan

$installed = $false
$exitCode = 0
try {
    # --- phase 1: silent install --------------------------------------------
    if ($willInstall) {
        Assert-Finding -Phase 'install' -Check 'the installer exists' `
            -Condition (Test-Path -LiteralPath $Installer) -Detail $Installer
        Assert-Finding -Phase 'install' -Check 'the installer is an MSI package' `
            -Condition ([System.IO.Path]::GetExtension($Installer) -ieq '.msi') `
            -Detail 'this script drives msiexec; an NSIS bundle needs its own /S path'

        $code = Invoke-Msiexec -Arguments @('/i', $Installer) `
            -LogPath (Join-Path $LogDirectory 'install.log')
        Assert-Finding -Phase 'install' -Check 'msiexec /i returned 0' `
            -Condition ($code -eq 0) -Detail "exit code $code"
        $installed = $true
    }
    else {
        Add-Finding -Phase 'install' -Check 'silent install' -Passed $true `
            -Detail 'skipped; the verify and smoke phases run against binaries already on disk'
    }

    # --- phase 2: what got installed, and what it does ----------------------
    if (-not $AppExecutable) {
        $candidates = @()
        foreach ($base in @($env:ProgramFiles, ${env:ProgramFiles(x86)})) {
            if ([string]::IsNullOrWhiteSpace($base)) { continue }
            $candidates += Get-ChildItem -LiteralPath $base -Filter $script:ExecutableName `
                -Recurse -ErrorAction SilentlyContinue | Select-Object -ExpandProperty FullName
        }
        if ($candidates.Count -eq 0) {
            $fallback = Join-Path $root "apps/desktop/src-tauri/target/release/$script:ExecutableName"
            if (Test-Path -LiteralPath $fallback) { $candidates = @($fallback) }
        }
        if ($candidates.Count -gt 0) { $AppExecutable = $candidates[0] }
    }

    if ($AppExecutable -and (Test-Path -LiteralPath $AppExecutable)) {
        Assert-Finding -Phase 'verify' -Check 'the process is named soul.exe' `
            -Condition ([System.IO.Path]::GetFileName($AppExecutable) -ieq $script:ExecutableName) `
            -Detail $AppExecutable
        Test-ManifestIsAsInvoker -Path $AppExecutable
    }
    elseif ($willInstall) {
        Assert-Finding -Phase 'verify' -Check 'the installer placed soul.exe' -Condition $false `
            -Detail 'nothing named soul.exe was found under Program Files'
    }
    else {
        Add-Finding -Phase 'verify' -Check 'the manifest check' -Passed $true `
            -Detail 'skipped; no soul.exe was given or found. Build it first to include this check'
    }

    # --- phase 3: the headless smoke, watched -------------------------------
    Assert-Finding -Phase 'smoke' -Check 'the headless binary exists' `
        -Condition (Test-Path -LiteralPath $Headless) -Detail $Headless

    $smoke = Invoke-HeadlessSmoke -Path $Headless -LogDirectory $LogDirectory
    Assert-Finding -Phase 'smoke' -Check 'soul-headless exited 0' `
        -Condition ($smoke.ExitCode -eq 0) `
        -Detail "exit code $($smoke.ExitCode); see $($smoke.StderrPath)"

    Assert-Finding -Phase 'smoke' -Check 'the run produced a report' `
        -Condition ($null -ne $smoke.Report) -Detail $smoke.StdoutPath
    Assert-Finding -Phase 'smoke' -Check 'every step of the main flow passed' `
        -Condition ([bool] $smoke.Report.ok)
    Assert-Finding -Phase 'smoke' -Check 'AC-02: nothing is switched on' `
        -Condition ([bool] $smoke.Report.config.fully_closed) `
        -Detail "open: $($smoke.Report.config.open_capabilities -join ', ')"
    Assert-Finding -Phase 'smoke' -Check 'AC-21: the process reported no non-loopback connection' `
        -Condition ($smoke.Report.egress.non_loopback_connections -eq 0) `
        -Detail "$($smoke.Report.egress.non_loopback_peers -join ', ')"
    Assert-Finding -Phase 'smoke' -Check 'the audit chain verified' `
        -Condition ([bool] $smoke.Report.audit_chain_verified) `
        -Detail "$($smoke.Report.audit_entries) entries"

    if ($smoke.WatchedByThisScript) {
        Assert-Finding -Phase 'smoke' -Check 'AC-21: Windows saw no non-loopback connection either' `
            -Condition ($smoke.NonLoopbackPeers.Count -eq 0) `
            -Detail "$($smoke.Samples) sample(s) of the TCP table; $($smoke.NonLoopbackPeers -join ', ')"
    }
    else {
        Add-Finding -Phase 'smoke' -Check 'the TCP table watch' -Passed $true `
            -Detail 'skipped; Get-NetTCPConnection is not available on this host'
    }

    foreach ($step in $smoke.Report.steps) {
        Write-Host ("       {0,-9} {1}" -f $step.name, $step.detail)
    }
}
catch {
    Write-Host "install-smoke: $($_.Exception.Message)" -ForegroundColor Red
    $exitCode = 1
}
finally {
    # --- phase 4: uninstall, whatever happened above ------------------------
    if ($installed) {
        try {
            $code = Invoke-Msiexec -Arguments @('/x', $Installer) `
                -LogPath (Join-Path $LogDirectory 'uninstall.log')
            Add-Finding -Phase 'uninstall' -Check 'msiexec /x returned 0' `
                -Passed ($code -eq 0) -Detail "exit code $code"
            if ($code -ne 0) { $exitCode = 1 }

            if ($AppExecutable) {
                $gone = -not (Test-Path -LiteralPath $AppExecutable)
                Add-Finding -Phase 'uninstall' -Check 'soul.exe is gone' -Passed $gone `
                    -Detail $AppExecutable
                if (-not $gone) { $exitCode = 1 }
            }
        }
        catch {
            Write-Host "install-smoke: uninstall failed: $($_.Exception.Message)" -ForegroundColor Red
            $exitCode = 1
        }
    }
}

$failed = @($script:Findings | Where-Object { -not $_.Passed })
Write-Host ''
Write-Host ("install-smoke: {0} check(s), {1} failed. Logs in {2}" -f `
        $script:Findings.Count, $failed.Count, $LogDirectory)

if ($failed.Count -gt 0) { $exitCode = 1 }
exit $exitCode
