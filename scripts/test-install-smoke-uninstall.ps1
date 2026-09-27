$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# Synthetic native children reproduce Windows' running-executable lock.
# This never launches NSIS and writes only under its unique temp directory.
$scriptPath = if ($args.Count -gt 0) { $args[0] } else { Join-Path $PSScriptRoot 'install-smoke.ps1' }
$tokens=$null
$errors=$null
$ast=[System.Management.Automation.Language.Parser]::ParseFile((Resolve-Path -LiteralPath $scriptPath).Path,[ref]$tokens,[ref]$errors)
if ($errors.Count) {throw 'Production script does not parse'}
foreach ($name in @('Invoke-SilentUninstaller','Add-Finding')) {
    $defs=@($ast.FindAll({param($node) $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq $name},$false))
    if ($defs.Count -ne 1) {throw "Expected one $name function"}
    . ([scriptblock]::Create($defs[0].Extent.Text))
}
$verifiers=@($ast.FindAll({param($node) $node -is [System.Management.Automation.Language.IfStatementAst] -and $node.Clauses[0].Item1.Extent.Text -eq '$AppExecutable' -and $node.Extent.Text.Contains("'soul.exe is gone'")},$false))
if ($verifiers.Count -ne 1) {throw 'Expected one uninstall artifact verdict'}
$verifier=[scriptblock]::Create($verifiers[0].Extent.Text)
$script:ProductName='Soul'
function Start-Process {
    [CmdletBinding()]
    param([string] $FilePath, [string[]] $ArgumentList, [switch] $Wait, [switch] $PassThru)
    $script:RunnerPath=$FilePath
    Microsoft.PowerShell.Management\Start-Process @PSBoundParameters
}
$scratch=Join-Path ([IO.Path]::GetTempPath()) ('soul-smoke-uninstall-'+[guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $scratch | Out-Null
try {
    $source=Join-Path $scratch 'probe.rs'
    @"
#![windows_subsystem = "windows"]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.get(1).map(String::as_str), Some("/S"));
    let tail = args[2..].join(" ");
    let dir = std::path::PathBuf::from(tail.strip_prefix("_?=").unwrap());
    if dir.join("fail.flag").exists() { std::process::exit(23); }
    // Like NSIS Delete/RMDir, continue even when an in-use exe cannot be removed.
    let _ = std::fs::remove_file(dir.join("soul.exe"));
    let removed = std::fs::remove_file(dir.join("uninstall.exe"));
    std::fs::write(dir.parent().unwrap().join("last-delete-status.txt"), format!("{:?}", removed)).unwrap();
    let _ = std::fs::remove_dir(&dir);
}
"@ | Set-Content -LiteralPath $source -Encoding UTF8
    $probe=Join-Path $scratch 'probe.exe'
    & rustc --edition 2021 $source -o $probe
    if ($LASTEXITCODE -ne 0) {throw "Probe compilation failed: $LASTEXITCODE"}
    $data=Join-Path $scratch 'Soul'
    New-Item -ItemType Directory -Path $data | Out-Null
    $witness=Join-Path $data 'soul.db'
    'synthetic data must survive' | Set-Content -LiteralPath $witness -Encoding UTF8
    $before=(Get-FileHash -LiteralPath $witness).Hash
    foreach ($case in @('success','failed','extra-file')) {
        $directory=Join-Path $scratch ($case+' with spaces 中文')
        New-Item -ItemType Directory -Path $directory | Out-Null
        Copy-Item -LiteralPath $probe -Destination (Join-Path $directory 'uninstall.exe')
        'synthetic installed app' | Set-Content -LiteralPath (Join-Path $directory 'soul.exe') -Encoding UTF8
        if ($case -eq 'failed') { 'fail' | Set-Content -LiteralPath (Join-Path $directory 'fail.flag') -Encoding UTF8 }
        if ($case -eq 'extra-file') { 'keep me' | Set-Content -LiteralPath (Join-Path $directory 'unknown.user') -Encoding UTF8 }
        $entry=[pscustomobject]@{InstallLocation=$directory}
        $code=Invoke-SilentUninstaller -InstallerPath 'synthetic-setup.exe' -LogDirectory $scratch -Entry $entry
        if ($case -eq 'success' -and ($code -ne 0 -or (Test-Path -LiteralPath $directory))) {
            $detail=Get-Content -LiteralPath (Join-Path $scratch 'last-delete-status.txt') -Raw -Encoding UTF8
            throw ("Successful waited uninstall left its own executable or install directory behind: $detail")
        }
        if ($case -eq 'failed' -and ($code -ne 23 -or -not(Test-Path -LiteralPath (Join-Path $directory 'uninstall.exe')))) {
            throw 'Nonzero exit was lost or failed installation was forcibly removed'
        }
        if ($case -eq 'extra-file' -and ($code -ne 0 -or (Get-Content -LiteralPath (Join-Path $directory 'unknown.user') -Raw -Encoding UTF8).TrimEnd() -cne 'keep me')) {
            throw 'Uninstaller cleanup removed unknown user content'
        }
        if ($script:RunnerPath -eq (Join-Path $directory 'uninstall.exe') -or
            (Test-Path -LiteralPath (Split-Path -Parent $script:RunnerPath))) {
            throw 'Uninstaller must run from a temporary copy and clean it afterward'
        }
        $AppExecutable=Join-Path $directory 'soul.exe'
        $exitCode=0
        $script:Findings=[System.Collections.Generic.List[object]]::new()
        . $verifier
        $expectedPassed=$case -eq 'success'
        if (($exitCode -eq 0) -ne $expectedPassed) {throw "$case`: real removal verdict was incorrect"}
        if ((Get-FileHash -LiteralPath $witness).Hash -ne $before) {throw 'Data witness changed'}
        Write-Output "$case`: real child exit, installation removal and data preservation verified"
    }
    Write-Output 'PASS: three waited-uninstaller cases; no NSIS or personal data touched'
} finally {
    $resolved=[IO.Path]::GetFullPath($scratch)
    $tempRoot=[IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd([IO.Path]::DirectorySeparatorChar)+[IO.Path]::DirectorySeparatorChar
    if (-not $resolved.StartsWith($tempRoot,[StringComparison]::OrdinalIgnoreCase) -or
        (Split-Path -Leaf $resolved) -notmatch '^soul-smoke-uninstall-[0-9a-f]{32}$') {throw 'Unexpected scratch cleanup path'}
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
