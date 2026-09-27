$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# Run the actual watcher and verdict with real child processes and controlled
# OS-query results. No install/uninstall and no personal data are involved.
$scriptPath = if ($args.Count -gt 0) { $args[0] } else { Join-Path $PSScriptRoot 'install-smoke.ps1' }
$tokens = $null
$parseErrors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile(
    (Resolve-Path -LiteralPath $scriptPath).Path, [ref] $tokens, [ref] $parseErrors)
if ($parseErrors.Count -ne 0) { throw "Script does not parse: $parseErrors" }
foreach ($name in @('Invoke-HeadlessSmoke', 'Add-Finding', 'Assert-Finding')) {
    $definitions = @($ast.FindAll({
        param($node)
        $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq $name
    }, $false))
    if ($definitions.Count -ne 1) { throw "Expected one $name function" }
    . ([scriptblock]::Create($definitions[0].Extent.Text))
}
$verdicts = @($ast.FindAll({
    param($node)
    $node -is [System.Management.Automation.Language.IfStatementAst] -and
        $node.Clauses[0].Item1.Extent.Text -eq '$smoke.WatchedByThisScript'
}, $false))
if ($verdicts.Count -ne 1) { throw 'Expected one TCP watch verdict' }
$verdict = [scriptblock]::Create($verdicts[0].Extent.Text)
$script:LoopbackAddresses = @('127.0.0.1', '::1', '0.0.0.0', '::')
$script:MonitorAvailable = $true
function Get-Command {
    [CmdletBinding()]
    param([string] $Name)
    if ($Name -ne 'Get-NetTCPConnection') { throw "Unexpected discovery request: $Name" }
    if ($script:MonitorAvailable) { [pscustomobject]@{ Name = $Name } }
}
function Get-NetTCPConnection {
    [CmdletBinding()]
    param([int] $OwningProcess = 0)
    $script:QueryCalls++
    if ($script:Scenario -eq 'query-error' -or
        ($script:Scenario -eq 'late-query-error' -and $script:QueryCalls -ge 2)) {
        Write-Error 'synthetic TCP provider failure'
        return
    }
    # $process is the real watcher child, visible through PowerShell scope.
    if ($script:Scenario -eq 'unrelated-peer') {
        [pscustomobject]@{ OwningProcess = 0; RemoteAddress = '192.0.2.1'; RemotePort = 443; State = 'Established' }
    } elseif ($script:Scenario -eq 'own-peer') {
        [pscustomobject]@{ OwningProcess = $process.Id; RemoteAddress = '192.0.2.1'; RemotePort = 443; State = 'Established' }
    } elseif ($script:Scenario -eq 'local-only') {
        [pscustomobject]@{ OwningProcess = $process.Id; RemoteAddress = '127.0.0.1'; RemotePort = 45678; State = 'Established' }
        [pscustomobject]@{ OwningProcess = $process.Id; RemoteAddress = '192.0.2.1'; RemotePort = 443; State = 'Listen' }
    }
}

$scratch = Join-Path ([IO.Path]::GetTempPath()) ('soul-smoke-tcp-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $scratch | Out-Null
try {
    $source = Join-Path $scratch 'probe.rs'
    @"
fn main() {
    assert_eq!(std::env::args().nth(1).as_deref(), Some("smoke"));
    let exe = std::env::current_exe().unwrap();
    std::fs::write(exe.with_extension("started"), std::process::id().to_string()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(350));
    println!("{}", r#"{"ok":true}"#);
    eprintln!("synthetic stderr");
}
"@ | Set-Content -LiteralPath $source -Encoding UTF8
    $probe = Join-Path $scratch 'probe.exe'
    & rustc --edition 2021 $source -o $probe
    if ($LASTEXITCODE -ne 0) { throw "Probe compilation failed: $LASTEXITCODE" }
    $cases = @(
        @{ Name = 'query-error'; WatcherFails = $true; Passes = $false },
        @{ Name = 'late-query-error'; WatcherFails = $true; Passes = $false },
        @{ Name = 'empty-table'; WatcherFails = $false; Passes = $true },
        @{ Name = 'unrelated-peer'; WatcherFails = $false; Passes = $true },
        @{ Name = 'own-peer'; WatcherFails = $false; Passes = $false },
        @{ Name = 'local-only'; WatcherFails = $false; Passes = $true },
        @{ Name = 'unavailable'; WatcherFails = $true; Passes = $false }
    )
    foreach ($case in $cases) {
        $script:Scenario = $case.Name
        $script:MonitorAvailable = $case.Name -ne 'unavailable'
        $script:QueryCalls = 0
        $exe = Join-Path $scratch ($case.Name + '.exe')
        if ($case.Name -eq 'unavailable') {
            # An invalid path makes discovery-before-start deterministic: a
            # misplaced Start() would throw a file error, not monitor absence.
            if (Test-Path -LiteralPath $exe) { throw 'Unavailable-monitor probe must not exist' }
        } else {
            Copy-Item -LiteralPath $probe -Destination $exe
        }
        $logs = Join-Path $scratch $case.Name
        New-Item -ItemType Directory -Path $logs | Out-Null
        $smoke = $null
        $failure = $null
        try { $smoke = Invoke-HeadlessSmoke -Path $exe -LogDirectory $logs } catch { $failure = $_ }
        $started = [IO.Path]::ChangeExtension($exe, 'started')
        if ($case.Name -eq 'unavailable') {
            if ($null -eq $failure -or $failure.Exception.Message -notmatch 'Get-NetTCPConnection') {
                throw 'Unavailable monitor was not rejected explicitly'
            }
            if (Test-Path -LiteralPath $started) { throw 'Missing monitor was detected only after starting the child' }
            if ($script:QueryCalls -ne 0) { throw 'Unavailable monitor was queried' }
        } else {
            $childPid = [int] (Get-Content -LiteralPath $started -Raw -Encoding UTF8)
            if (Get-Process -Id $childPid -ErrorAction SilentlyContinue) { throw 'Watcher returned before the child exited' }
            $outText = Get-Content -LiteralPath (Join-Path $logs 'headless-stdout.json') -Raw -Encoding UTF8
            $errText = Get-Content -LiteralPath (Join-Path $logs 'headless-stderr.txt') -Raw -Encoding UTF8
            if ($outText.TrimEnd() -cne '{"ok":true}' -or $errText.TrimEnd() -cne 'synthetic stderr') {
                throw 'Child output was not captured before reporting the query result'
            }
            if ($case.WatcherFails) {
                if ($null -eq $failure) {
                    $script:Findings = [System.Collections.Generic.List[object]]::new()
                    & $verdict
                    throw "$($case.Name): failed TCP query accepted as clean evidence ($($smoke.Samples) samples)"
                }
                if ($failure.Exception.Message -notmatch 'synthetic TCP provider failure') {
                    throw "Unexpected watcher failure: $failure"
                }
                if ($case.Name -eq 'late-query-error' -and $script:QueryCalls -lt 2) { throw 'Late failure was not exercised' }
            } else {
                if ($null -ne $failure) { throw $failure }
                if ($smoke.ExitCode -ne 0 -or -not $smoke.Report.ok) { throw 'Real child outcome was lost' }
                if ($smoke.Samples -ne $script:QueryCalls -or $smoke.Samples -le 0) { throw 'Successful query count was not preserved' }
                $script:Findings = [System.Collections.Generic.List[object]]::new()
                $verdictFailure = $null
                try { & $verdict } catch { $verdictFailure = $_ }
                if ($script:Findings.Count -ne 1 -or $script:Findings[0].Passed -cne $case.Passes -or
                    (($null -eq $verdictFailure) -ne $case.Passes)) {
                    throw "$($case.Name): unexpected real TCP verdict"
                }
            }
        }
        Write-Output "$($case.Name): query outcome and child lifecycle verified"
    }
    Write-Output 'PASS: seven TCP provider cases; no installation performed'
} finally {
    $resolved = [IO.Path]::GetFullPath($scratch)
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    if (-not $resolved.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase) -or
        (Split-Path -Leaf $resolved) -notmatch '^soul-smoke-tcp-[0-9a-f]{32}$') {
        throw "Refusing to remove unexpected scratch directory: $resolved"
    }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
