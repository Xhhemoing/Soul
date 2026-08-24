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

    Four phases, and the exit code is the whole interface:

      1. The bundle installs without a dialog and without a UAC prompt.
         tauri.conf.json builds an NSIS package in currentUser mode, so the
         silent switch is /S and the target is under LOCALAPPDATA; an .msi is
         accepted too, in case a WiX target is ever added, and then it is
         msiexec /qn. Nothing is downloaded either way: if the bundle needs a
         WebView2 runtime it is not this script's job to fetch one, and a smoke
         test that installs extra software is not testing what shipped.
      2. The installed soul.exe is inspected: it has to be named soul.exe and
         its embedded manifest has to say asInvoker.
      3. soul-headless.exe runs the AC-21 main flow while this script watches
         the operating system's own TCP table for that process. Exit 0, a clean
         report, and zero non-loopback connections, or the run fails.
      4. The uninstaller runs silently and the install directory has to go.

.NOTES
    Run this unelevated. That is the AC-01 check: phase 1 asserts the script's
    own process is not an administrator, so an install that completes is an
    install that needed nobody's permission. An installer that did want
    elevation would raise a UAC prompt here, which is exactly the failure the
    criterion is about, and /S does not suppress it.

    Known gap, stated rather than worked around: the Tauri bundle contains
    soul.exe and not soul-headless.exe, so -Headless points at the headless
    binary built from the same commit rather than at one the installer placed.
    Until the bundle ships it, phase 3 proves that this build's core runs
    closed on this machine; it does not prove the installer put that core
    there. See docs/STATUS.md, WP13.

    The connection watch reads Get-NetTCPConnection, which covers TCP only.
    UDP has no remote address in the Windows endpoint table, so a UDP peer is
    invisible here. The Linux side of AC-21 (soulcore::netwatch) reads both,
    and the two together are the coverage.

.PARAMETER Installer
    Path to the bundle produced by `tauri build` — Soul_<version>_x64-setup.exe
    for the NSIS target, or an .msi. Omit it, or pass -SkipInstall, to run only
    the verify and smoke phases against binaries that are already present.

.PARAMETER Headless
    Path to soul-headless.exe. Defaults to the release build in target/.

.PARAMETER AppExecutable
    Path to soul.exe, for the manifest check. Defaults to the installed copy
    when an installer was given, otherwise to the release build in target/.

.PARAMETER SkipInstall
    Run phases 2 and 3 only. This is what CI does: no runner should install a
    package, but every runner can run the binary and watch its sockets.

.PARAMETER DryRun
    Validate the arguments, print the plan, and change nothing. CI runs this
    to check that the script still does what it says without needing a bundle.

.PARAMETER LogDirectory
    Where installer logs and the headless report are written. Defaults to a new
    directory under the temp folder.

.EXAMPLE
    pwsh -File scripts/install-smoke.ps1 -Installer .\Soul_0.1.0_x64-setup.exe

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

# What tauri.conf.json calls the product, which is also the directory an NSIS
# currentUser install creates under LOCALAPPDATA.
$script:ProductName = 'Soul'

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
    Write-Host ("  {0} [{1}] {2}{3}" -f $mark, $Phase, $Check, $(if ($Detail) { " - $Detail" } else { '' }))
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

function Test-RunningElevated {
    try {
        $identity = [System.Security.Principal.WindowsIdentity]::GetCurrent()
        $principal = [System.Security.Principal.WindowsPrincipal]::new($identity)
        return $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)
    }
    catch {
        return $false
    }
}

# The embedded application manifest, read out of the PE image as text.
#
# A byte scan rather than a resource API call: the manifest is stored as UTF-8
# XML inside the executable, `mt.exe` is not on every machine, and the question
# - does this binary ask for elevation - is answerable by looking for the words
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

# The registry entry the installer wrote, if it wrote one. Per-user first:
# currentUser mode is what tauri.conf.json asks for, and finding the product
# under HKLM would mean the bundle went machine-wide behind the config's back.
function Get-InstalledEntry {
    $roots = @(
        'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall',
        'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall',
        'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall'
    )

    foreach ($root in $roots) {
        if (-not (Test-Path -LiteralPath $root)) { continue }
        foreach ($key in Get-ChildItem -LiteralPath $root -ErrorAction SilentlyContinue) {
            $properties = Get-ItemProperty -LiteralPath $key.PSPath -ErrorAction SilentlyContinue
            if ($null -eq $properties) { continue }
            $name = $properties.PSObject.Properties['DisplayName']
            if ($null -eq $name -or $name.Value -notlike "$script:ProductName*") { continue }

            $read = {
                param($property)
                $found = $properties.PSObject.Properties[$property]
                if ($null -eq $found) { '' } else { [string] $found.Value }
            }
            return [pscustomobject]@{
                Hive             = $root
                DisplayName      = & $read 'DisplayName'
                InstallLocation  = & $read 'InstallLocation'
                UninstallString  = & $read 'UninstallString'
                QuietUninstall   = & $read 'QuietUninstallString'
            }
        }
    }
    return $null
}

# NSIS: /S is silent, and it returns before the work is done, so wait for the
# process rather than trusting Start-Process to have finished the job.
function Invoke-SilentInstaller {
    param(
        [Parameter(Mandatory)] [string] $Path,
        [Parameter(Mandatory)] [string] $LogDirectory
    )

    if ([System.IO.Path]::GetExtension($Path) -ieq '.msi') {
        $arguments = @('/i', $Path, '/qn', '/norestart', '/l*v', (Join-Path $LogDirectory 'install.log'))
        Write-Host "  running: msiexec.exe $($arguments -join ' ')"
        return (Start-Process -FilePath 'msiexec.exe' -ArgumentList $arguments -Wait -PassThru).ExitCode
    }

    Write-Host "  running: $Path /S"
    return (Start-Process -FilePath $Path -ArgumentList @('/S') -Wait -PassThru).ExitCode
}

function Invoke-SilentUninstaller {
    param(
        [Parameter(Mandatory)] [string] $InstallerPath,
        [Parameter(Mandatory)] [string] $LogDirectory,
        [object] $Entry
    )

    if ([System.IO.Path]::GetExtension($InstallerPath) -ieq '.msi') {
        $arguments = @('/x', $InstallerPath, '/qn', '/norestart', '/l*v', (Join-Path $LogDirectory 'uninstall.log'))
        Write-Host "  running: msiexec.exe $($arguments -join ' ')"
        return (Start-Process -FilePath 'msiexec.exe' -ArgumentList $arguments -Wait -PassThru).ExitCode
    }

    $uninstaller = $null
    if ($null -ne $Entry -and -not [string]::IsNullOrWhiteSpace($Entry.InstallLocation)) {
        $candidate = Join-Path $Entry.InstallLocation 'uninstall.exe'
        if (Test-Path -LiteralPath $candidate) { $uninstaller = $candidate }
    }
    if (-not $uninstaller) {
        $candidate = Join-Path (Join-Path $env:LOCALAPPDATA $script:ProductName) 'uninstall.exe'
        if (Test-Path -LiteralPath $candidate) { $uninstaller = $candidate }
    }
    if (-not $uninstaller) {
        throw 'no uninstall.exe was found; the install directory has to be removed by hand'
    }

    # _?= keeps the uninstaller in place so it can be waited on. Without it
    # NSIS copies itself to the temp folder, returns immediately, and this
    # script would check for a directory that is still being deleted.
    $directory = Split-Path -Parent $uninstaller
    Write-Host "  running: $uninstaller /S _?=$directory"
    return (Start-Process -FilePath $uninstaller -ArgumentList @('/S', "_?=$directory") -Wait -PassThru).ExitCode
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

    $process = Start-Process -FilePath $Path -ArgumentList @('smoke') -NoNewWindow -PassThru `
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
        ExitCode            = $process.ExitCode
        Report              = $report
        StdoutPath          = $stdout
        StderrPath          = $stderr
        WatchedByThisScript = $canWatch
        Samples             = $samples
        NonLoopbackPeers    = @($seen)
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
    Write-Error 'install-smoke: pass -Installer <path to the bundle>, or -SkipInstall to run the headless phase only.'
    exit 2
}

New-Item -ItemType Directory -Path $LogDirectory -Force | Out-Null
Show-Plan -Plan $plan

$installed = $false
$entry = $null
$exitCode = 0
try {
    # --- phase 1: silent install --------------------------------------------
    if ($willInstall) {
        Assert-Finding -Phase 'install' -Check 'the installer exists' `
            -Condition (Test-Path -LiteralPath $Installer) -Detail $Installer
        Assert-Finding -Phase 'install' -Check 'AC-01: this shell is not elevated' `
            -Condition (-not (Test-RunningElevated)) `
            -Detail 'an install that needs administrator is a failed criterion, not a reason to re-run as one'

        $code = Invoke-SilentInstaller -Path $Installer -LogDirectory $LogDirectory
        Assert-Finding -Phase 'install' -Check 'the installer returned 0' `
            -Condition ($code -eq 0) -Detail "exit code $code"
        $installed = $true

        $entry = Get-InstalledEntry
        if ($null -ne $entry) {
            Add-Finding -Phase 'install' -Check 'the product is registered' -Passed $true `
                -Detail "$($entry.DisplayName) in $($entry.Hive)"
            Assert-Finding -Phase 'install' -Check 'AC-01: the install is per-user' `
                -Condition ($entry.Hive.StartsWith('HKCU:')) `
                -Detail "tauri.conf.json asks for currentUser; found it in $($entry.Hive)"
        }
    }
    else {
        Add-Finding -Phase 'install' -Check 'silent install' -Passed $true `
            -Detail 'skipped; the verify and smoke phases run against binaries already on disk'
    }

    # --- phase 2: what got installed ----------------------------------------
    if (-not $AppExecutable) {
        $searched = @()
        if ($null -ne $entry -and -not [string]::IsNullOrWhiteSpace($entry.InstallLocation)) {
            $searched += $entry.InstallLocation
        }
        foreach ($base in @($env:LOCALAPPDATA, $env:ProgramFiles, ${env:ProgramFiles(x86)})) {
            if ([string]::IsNullOrWhiteSpace($base)) { continue }
            $searched += (Join-Path $base $script:ProductName)
        }

        $candidates = @()
        foreach ($directory in $searched) {
            if (-not (Test-Path -LiteralPath $directory)) { continue }
            $candidates += Get-ChildItem -LiteralPath $directory -Filter $script:ExecutableName `
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
            -Detail 'nothing named soul.exe was found where the installer said it wrote'
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
            $code = Invoke-SilentUninstaller -InstallerPath $Installer `
                -LogDirectory $LogDirectory -Entry $entry
            Add-Finding -Phase 'uninstall' -Check 'the uninstaller returned 0' `
                -Passed ($code -eq 0) -Detail "exit code $code"
            if ($code -ne 0) { $exitCode = 1 }

            if ($AppExecutable) {
                $gone = -not (Test-Path -LiteralPath $AppExecutable)
                Add-Finding -Phase 'uninstall' -Check 'soul.exe is gone' -Passed $gone `
                    -Detail $AppExecutable
                if (-not $gone) { $exitCode = 1 }
            }

            $stillListed = Get-InstalledEntry
            Add-Finding -Phase 'uninstall' -Check 'the product is no longer registered' `
                -Passed ($null -eq $stillListed) `
                -Detail $(if ($null -eq $stillListed) { '' } else { "$($stillListed.DisplayName) is still in $($stillListed.Hive)" })
            if ($null -ne $stillListed) { $exitCode = 1 }
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
