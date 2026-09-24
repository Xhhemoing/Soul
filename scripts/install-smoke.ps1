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
         silent switch is /S and the per-user install dir is
         %LOCALAPPDATA%\Programs\Soul (user data stays in
         %LOCALAPPDATA%\Soul). An .msi is accepted too, in case a WiX target
         is ever added, and then it is msiexec /qn. Nothing is downloaded
         either way: if the bundle needs a WebView2 runtime it is not this
         script's job to fetch one, and a smoke test that installs extra
         software is not testing what shipped.
      2. The installed soul.exe is inspected: it has to be named soul.exe and
         its embedded manifest has to say asInvoker. When this run did the
         installing it also has to be under %LOCALAPPDATA%\Programs\Soul and
         not under %LOCALAPPDATA%\Soul: an installer that wrote the program
         into the data directory makes phase 4 point an uninstaller at
         keys.dpapi, and a soul.exe found there fails the run before phase 4
         gets the chance.
      3. soul-headless.exe runs the AC-21 main flow while this script watches
         the operating system's own TCP table for that process. Exit 0, a clean
         report, and zero non-loopback connections, or the run fails.
      4. The uninstaller runs silently. The install directory
         (%LOCALAPPDATA%\Programs\Soul) must go; user data under
         %LOCALAPPDATA%\Soul must not be removed by this script or the
         uninstaller, and that is asserted rather than assumed: keys.dpapi and
         soul.db are fingerprinted before the uninstaller starts and have to
         be there, byte for byte, after it finishes. On a clean machine there
         is nothing to fingerprint - soul-headless smoke works in a scratch
         directory and deletes it - so a missing file is planted as a sentinel
         first and removed again afterwards. A file that was already there is
         the person's own store and is only read.

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

# What tauri.conf.json calls the product. NSIS installs to
# %LOCALAPPDATA%\Programs\Soul; encrypted data stays in %LOCALAPPDATA%\Soul.
$script:ProductName = 'Soul'

# Addresses that are not egress. Anything else the installed process connects
# to fails the run.
$script:LoopbackAddresses = @('127.0.0.1', '::1', '0.0.0.0', '::')

$script:Findings = [System.Collections.Generic.List[object]]::new()

# Set by phase 2 when the soul.exe this run installed turned up inside the
# user data directory. Phase 4 then refuses to run that uninstaller at all:
# pointing it at the directory holding keys.dpapi is the loss this script is
# supposed to catch, not cause.
$script:UninstallIsUnsafe = $false

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

# Is $Path that directory, or something inside it?
#
# The comparison appends a directory separator before matching the prefix, so
# %LOCALAPPDATA%\Programs\SoulSomething is not read as living inside
# %LOCALAPPDATA%\Programs\Soul. Ordinal-insensitive because Windows paths are.
function Test-PathIsUnder {
    param(
        [Parameter(Mandatory)] [string] $Path,
        [Parameter(Mandatory)] [string] $Directory
    )

    $separator = [System.IO.Path]::DirectorySeparatorChar
    $full = [System.IO.Path]::GetFullPath($Path).TrimEnd($separator)
    $root = [System.IO.Path]::GetFullPath($Directory).TrimEnd($separator)
    if ($full.Equals($root, [System.StringComparison]::OrdinalIgnoreCase)) { return $true }
    return $full.StartsWith($root + $separator, [System.StringComparison]::OrdinalIgnoreCase)
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
        $candidate = Join-Path (Join-Path (Join-Path $env:LOCALAPPDATA 'Programs') $script:ProductName) 'uninstall.exe'
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

# --- the store an uninstall must not touch ---------------------------------

# The two files under %LOCALAPPDATA%\Soul whose loss is permanent. keys.dpapi
# wraps the DEK and exists nowhere else on this machine; soul.db is the only
# thing it opens. An uninstaller that takes them has not removed a program, it
# has erased the person's history.
$script:UserDataFileNames = @('keys.dpapi', 'soul.db')

# A length and a digest rather than the bytes themselves: a real soul.db is as
# large as the history in it, and this is read twice inside a finally block.
function Get-FileFingerprint {
    param([Parameter(Mandatory)] [string] $Path)

    return [pscustomobject]@{
        Length = (Get-Item -LiteralPath $Path).Length
        Sha256 = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash
    }
}

function Test-FingerprintsMatch {
    param(
        [Parameter(Mandatory)] [object] $Before,
        [Parameter(Mandatory)] [object] $After
    )

    return ($Before.Length -eq $After.Length) -and ($Before.Sha256 -eq $After.Sha256)
}

# Record the state of the user data files before the uninstaller runs.
#
# A clean machine has none of them: phase 3 runs the headless smoke against a
# scratch store and deletes it, and nothing here starts the GUI. On such a
# machine an uninstaller that wiped the data directory and one that spared it
# look exactly alike, so a file that is missing is planted with a payload
# unique to this run. Anything already on disk is the person's own store: it is
# fingerprinted and never written.
function New-UserDataWitness {
    param([Parameter(Mandatory)] [string] $DataDirectory)

    $witnesses = @()
    foreach ($name in $script:UserDataFileNames) {
        $path = Join-Path $DataDirectory $name
        $planted = $false
        if (-not (Test-Path -LiteralPath $path)) {
            New-Item -ItemType Directory -Path $DataDirectory -Force | Out-Null
            $payload = "soul-install-smoke-witness $PID $(Get-Date -Format o) $([guid]::NewGuid())"
            [System.IO.File]::WriteAllText($path, $payload)
            $planted = $true
        }

        $witnesses += [pscustomobject]@{
            Name        = $name
            Path        = $path
            Planted     = $planted
            Fingerprint = Get-FileFingerprint -Path $path
        }
        Write-Host ("  {0} {1}" -f $(if ($planted) { 'planted' } else { 'found  ' }), $path)
    }
    return , $witnesses
}

# Take back the sentinels this run wrote, and nothing else. A fake keys.dpapi
# left on the machine is the first thing a real first launch would try to open,
# and the directory itself is never a candidate for deletion.
function Remove-PlantedWitness {
    param([Parameter(Mandatory)] [AllowEmptyCollection()] [object[]] $Witnesses)

    foreach ($witness in $Witnesses) {
        if (-not $witness.Planted) { continue }
        $sentinel = $witness.Path
        if (-not (Test-Path -LiteralPath $sentinel)) { continue }
        Remove-Item -LiteralPath $sentinel -Force -ErrorAction SilentlyContinue
    }
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

    # Start-Process -PassThru + RedirectStandard* on PS 5.1 often leaves
    # ExitCode $null after HasExited. Use ProcessStartInfo so the OS exit
    # code stays available; never invent 0/1 from report.ok.
    $utf8 = New-Object System.Text.UTF8Encoding $false
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $Path
    $psi.Arguments = 'smoke'
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true
    $psi.StandardOutputEncoding = $utf8
    $psi.StandardErrorEncoding = $utf8

    # Discover/autoload the monitor before a fast child can finish.
    $canWatch = $null -ne (Get-Command -Name 'Get-NetTCPConnection' -ErrorAction SilentlyContinue)
    if (-not $canWatch) {
        throw 'Invoke-HeadlessSmoke: Get-NetTCPConnection is unavailable; TCP observation is required (fail closed)'
    }

    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $psi
    if (-not $process.Start()) {
        throw "Invoke-HeadlessSmoke: failed to start process: $Path"
    }

    $stdoutAsync = $process.StandardOutput.ReadToEndAsync()
    $stderrAsync = $process.StandardError.ReadToEndAsync()

    $seen = [System.Collections.Generic.HashSet[string]]::new()
    $samples = 0
    $watchFailure = $null

    while (-not $process.HasExited) {
        try {
            # Query the whole table: a valid empty per-process result must not
            # be confused with a provider failure from a server-side PID filter.
            $connections = @(Get-NetTCPConnection -ErrorAction Stop)
            $samples++
            foreach ($connection in $connections) {
                if ($connection.OwningProcess -ne $process.Id) { continue }
                if ($script:LoopbackAddresses -contains $connection.RemoteAddress) { continue }
                if ($connection.State -eq 'Listen') { continue }
                [void] $seen.Add("$($connection.RemoteAddress):$($connection.RemotePort)")
            }
        } catch {
            $watchFailure = $_
            break
        }
        Start-Sleep -Milliseconds 20
        $process.Refresh()
    }
    # Even a failed observation must reap the child and preserve both streams.
    $process.WaitForExit()

    $stdoutText = $stdoutAsync.GetAwaiter().GetResult()
    $stderrText = $stderrAsync.GetAwaiter().GetResult()
    [System.IO.File]::WriteAllText($stdout, $stdoutText, $utf8)
    [System.IO.File]::WriteAllText($stderr, $stderrText, $utf8)

    $report = $null
    if (-not [string]::IsNullOrWhiteSpace($stdoutText)) {
        try {
            $report = $stdoutText | ConvertFrom-Json
        } catch {
            # Leave Report null so ExitCode and report.ok stay separate checks.
            $report = $null
        }
    }

    if (-not $process.HasExited) {
        throw "Invoke-HeadlessSmoke: process has not exited; ExitCode unavailable (fail closed); see $stderr"
    }
    # Process.ExitCode is Int32 once HasExited; do not derive from report.ok.
    $exitCode = [int] $process.ExitCode
    $process.Dispose()
    if ($null -ne $watchFailure) {
        throw "Invoke-HeadlessSmoke: TCP observation failed after $samples successful sample(s): $($watchFailure.Exception.Message); see $stdout and $stderr"
    }

    return [pscustomobject]@{
        ExitCode            = $exitCode
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
        $programsInstall = Join-Path (Join-Path $env:LOCALAPPDATA 'Programs') $script:ProductName
        $searched += $programsInstall
        foreach ($base in @($env:ProgramFiles, ${env:ProgramFiles(x86)})) {
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

    # Where the install put soul.exe, not just that it put one somewhere.
    #
    # The search above reads $entry.InstallLocation before it falls back to
    # %LOCALAPPDATA%\Programs\Soul, so a bundle that registered the data
    # directory as its install location hands this script a soul.exe out of
    # %LOCALAPPDATA%\Soul. That binary passes the name and manifest checks,
    # and then phase 4 aims an uninstaller at the directory holding
    # keys.dpapi. Only this run's own install is judged: -SkipInstall in CI
    # points -AppExecutable at target/release/soul.exe, which is a built
    # binary rather than an installed one and lives under neither directory.
    if ($willInstall -and $AppExecutable) {
        $programsInstall = Join-Path (Join-Path $env:LOCALAPPDATA 'Programs') $script:ProductName
        # %LOCALAPPDATA%\Soul. Note that this is not a prefix of
        # %LOCALAPPDATA%\Programs\Soul - the segment after LOCALAPPDATA is
        # Programs, not Soul - so an executable in the install tree cannot
        # trip the data-directory check below.
        $dataDirectory = Join-Path $env:LOCALAPPDATA $script:ProductName

        if (Test-PathIsUnder -Path $AppExecutable -Directory $dataDirectory) {
            $script:UninstallIsUnsafe = $true
        }

        Assert-Finding -Phase 'verify' -Check 'the installed soul.exe is not in the data directory' `
            -Condition (-not (Test-PathIsUnder -Path $AppExecutable -Directory $dataDirectory)) `
            -Detail "$AppExecutable is inside $dataDirectory, where keys.dpapi and soul.db live"

        Assert-Finding -Phase 'verify' -Check 'the installed soul.exe is under Programs' `
            -Condition (Test-PathIsUnder -Path $AppExecutable -Directory $programsInstall) `
            -Detail "$AppExecutable is not under $programsInstall"

        if ($null -ne $entry -and -not [string]::IsNullOrWhiteSpace($entry.InstallLocation)) {
            # Phase 4 looks for uninstall.exe under this path first, so a
            # registration pointing at the store is the same danger again.
            if (Test-PathIsUnder -Path $entry.InstallLocation -Directory $dataDirectory) {
                $script:UninstallIsUnsafe = $true
            }

            Assert-Finding -Phase 'verify' -Check 'the registered InstallLocation is not the data directory' `
                -Condition (-not (Test-PathIsUnder -Path $entry.InstallLocation -Directory $dataDirectory)) `
                -Detail "the uninstaller is looked up under $($entry.InstallLocation)"

            Assert-Finding -Phase 'verify' -Check 'the registered InstallLocation is the Programs install directory' `
                -Condition (Test-PathIsUnder -Path $entry.InstallLocation -Directory $programsInstall) `
                -Detail "$($entry.InstallLocation) is not under $programsInstall"
        }
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
            -Condition ($smoke.Samples -gt 0 -and $smoke.NonLoopbackPeers.Count -eq 0) `
            -Detail "$($smoke.Samples) sample(s) of the TCP table; $($smoke.NonLoopbackPeers -join ', ')"
    }
    else {
        Assert-Finding -Phase 'smoke' -Check 'the TCP table watch' -Condition $false `
            -Detail 'Get-NetTCPConnection is not available on this host; observation is required'
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
    if ($installed -and $script:UninstallIsUnsafe) {
        Add-Finding -Phase 'uninstall' -Check 'the uninstaller was not run' -Passed $false `
            -Detail 'soul.exe was installed into the data directory, so its uninstaller would be aimed at keys.dpapi. Remove the install by hand.'
        $exitCode = 1
    }
    elseif ($installed) {
        # Recorded before the uninstaller runs, and in its own try: a machine
        # that will not hold a witness still has to end this run uninstalled.
        $witnesses = @()
        try {
            # %LOCALAPPDATA%\Soul, which is not the install directory.
            $dataDirectory = Join-Path $env:LOCALAPPDATA $script:ProductName
            $witnesses = New-UserDataWitness -DataDirectory $dataDirectory
        }
        catch {
            Add-Finding -Phase 'uninstall' -Check 'the user data files were recorded' `
                -Passed $false -Detail $_.Exception.Message
            $exitCode = 1
        }

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

            foreach ($witness in $witnesses) {
                $survived = Test-Path -LiteralPath $witness.Path
                $detail = $witness.Path
                if ($survived) {
                    $survived = Test-FingerprintsMatch -Before $witness.Fingerprint `
                        -After (Get-FileFingerprint -Path $witness.Path)
                    if (-not $survived) {
                        $detail = "$($witness.Path) is not the file that was there before the uninstall"
                    }
                }
                else {
                    $detail = "$($witness.Path) was removed by the uninstaller; losing it is permanent"
                }

                Add-Finding -Phase 'uninstall' `
                    -Check "$($witness.Name) is still in the data directory" `
                    -Passed $survived -Detail $detail
                if (-not $survived) { $exitCode = 1 }
            }
        }
        catch {
            Write-Host "install-smoke: uninstall failed: $($_.Exception.Message)" -ForegroundColor Red
            $exitCode = 1
        }
        finally {
            Remove-PlantedWitness -Witnesses $witnesses
        }
    }
}

$failed = @($script:Findings | Where-Object { -not $_.Passed })
Write-Host ''
Write-Host ("install-smoke: {0} check(s), {1} failed. Logs in {2}" -f `
        $script:Findings.Count, $failed.Count, $LogDirectory)

if ($failed.Count -gt 0) { $exitCode = 1 }
exit $exitCode
