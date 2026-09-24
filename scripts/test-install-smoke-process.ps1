$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# Exercise only the process watcher, never install/uninstall or personal data.
# An optional path lets the same probes check an older or mutated script.
$scriptPath = if ($args.Count -gt 0) { $args[0] } else { Join-Path $PSScriptRoot 'install-smoke.ps1' }
$tokens = $null
$parseErrors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile(
    (Resolve-Path -LiteralPath $scriptPath).Path, [ref] $tokens, [ref] $parseErrors)
if ($parseErrors.Count -ne 0) { throw "Script does not parse: $parseErrors" }
$functions = @($ast.FindAll({
    param($node)
    $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
        $node.Name -eq 'Invoke-HeadlessSmoke'
}, $false))
if ($functions.Count -ne 1) { throw 'Expected one Invoke-HeadlessSmoke function' }
. ([scriptblock]::Create($functions[0].Extent.Text))
$script:LoopbackAddresses = @('127.0.0.1', '::1', '0.0.0.0', '::')

$scratch = Join-Path ([IO.Path]::GetTempPath()) ('soul-smoke-process-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $scratch | Out-Null
try {
    $source = Join-Path $scratch 'probe.rs'
    @'
use std::io::{self, Write};
fn main() {
    assert_eq!(std::env::args().nth(1).as_deref(), Some("smoke"));
    let exe = std::env::current_exe().unwrap();
    let name = exe.file_stem().unwrap().to_str().unwrap();
    let (body, code) = match name {
        "ok-exit23" => (r#"{"ok":true}"#.to_owned(), 23),
        "ok-exit0" => (r#"{"ok":true}"#.to_owned(), 0),
        "badok-exit0" => (r#"{"ok":false}"#.to_owned(), 0),
        "badjson-exit7" => ("not json".to_owned(), 7),
        "large-exit0" => (format!("{{\"ok\":true,\"payload\":\"{}\"}}", "界".repeat(100_000)), 0),
        _ => panic!("unexpected probe name"),
    };
    if name == "large-exit0" {
        eprintln!("{}", "错误".repeat(100_000));
    } else {
        eprintln!("synthetic stderr");
    }
    println!("{body}");
    io::stdout().flush().unwrap();
    io::stderr().flush().unwrap();
    std::process::exit(code);
}
'@ | Set-Content -LiteralPath $source -Encoding UTF8
    $probe = Join-Path $scratch 'probe.exe'
    & rustc --edition 2021 $source -o $probe
    if ($LASTEXITCODE -ne 0) { throw "Probe compilation failed: $LASTEXITCODE" }
    $cases = @(
        @{ Name = 'ok-exit23'; Exit = 23; Ok = $true },
        @{ Name = 'ok-exit0'; Exit = 0; Ok = $true },
        @{ Name = 'badok-exit0'; Exit = 0; Ok = $false },
        @{ Name = 'badjson-exit7'; Exit = 7; Ok = $null },
        @{ Name = 'large-exit0'; Exit = 0; Ok = $true }
    )
    foreach ($case in $cases) {
        $exe = Join-Path $scratch ($case.Name + '.exe')
        Copy-Item -LiteralPath $probe -Destination $exe
        $logs = Join-Path $scratch $case.Name
        New-Item -ItemType Directory -Path $logs | Out-Null
        $result = Invoke-HeadlessSmoke -Path $exe -LogDirectory $logs
        if ($result.ExitCode -ne $case.Exit) {
            throw "$($case.Name): expected OS exit $($case.Exit), got $($result.ExitCode)"
        }
        if ($null -eq $case.Ok) {
            if ($null -ne $result.Report) { throw 'Invalid JSON was accepted as a report' }
        } elseif ($null -eq $result.Report -or $result.Report.ok -cne $case.Ok) {
            throw "$($case.Name): report.ok was lost or rewritten"
        }
        $outText = Get-Content -LiteralPath $result.StdoutPath -Raw -Encoding UTF8
        $errText = Get-Content -LiteralPath $result.StderrPath -Raw -Encoding UTF8
        if ($case.Name -eq 'large-exit0') {
            if ($result.Report.payload -cne ('界' * 100000)) { throw 'Large UTF8 stdout corrupted or truncated' }
            if ($errText.TrimEnd() -cne ('错误' * 100000)) { throw 'Large UTF8 stderr corrupted or truncated' }
        } else {
            if ($errText.TrimEnd() -cne 'synthetic stderr') { throw 'stderr was not captured' }
        }
        if ([string]::IsNullOrWhiteSpace($outText)) { throw 'stdout was not captured' }
        if (@($result.NonLoopbackPeers).Count -ne 0) { throw 'Synthetic probe unexpectedly opened a remote connection' }
        Write-Output "$($case.Name): exit=$($result.ExitCode), report and output verified"
    }
    Write-Output 'PASS: five real child-process cases; no installation performed'
} finally {
    $resolved = [IO.Path]::GetFullPath($scratch)
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    if (-not $resolved.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase) -or
        (Split-Path -Leaf $resolved) -notmatch '^soul-smoke-process-[0-9a-f]{32}$') {
        throw "Refusing to remove unexpected scratch directory: $resolved"
    }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
