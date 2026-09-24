$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# Execute the real TCP verdict with controlled evidence, never the installer.
# Removing the nonzero-sample guard must make the zero-sample case fail.
$scriptPath = if ($args.Count -gt 0) { $args[0] } else { Join-Path $PSScriptRoot 'install-smoke.ps1' }
$tokens = $null
$parseErrors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile(
    (Resolve-Path -LiteralPath $scriptPath).Path, [ref] $tokens, [ref] $parseErrors)
if ($parseErrors.Count -ne 0) { throw "Script does not parse: $parseErrors" }
foreach ($name in @('Add-Finding', 'Assert-Finding')) {
    $definitions = @($ast.FindAll({
        param($node)
        $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
            $node.Name -eq $name
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
$cases = @(
    @{ Name = 'zero samples'; Samples = 0; Peers = @(); Passed = $false },
    @{ Name = 'one clean sample'; Samples = 1; Peers = @(); Passed = $true },
    @{ Name = 'several clean samples'; Samples = 3; Peers = @(); Passed = $true },
    @{ Name = 'observed remote peer'; Samples = 1; Peers = @('192.0.2.1:443'); Passed = $false }
)
foreach ($case in $cases) {
    $script:Findings = [System.Collections.Generic.List[object]]::new()
    $smoke = [pscustomobject]@{
        WatchedByThisScript = $true
        Samples = $case.Samples
        NonLoopbackPeers = $case.Peers
    }
    $failure = $null
    try { & $verdict } catch { $failure = $_ }
    if ($script:Findings.Count -ne 1) { throw "$($case.Name): expected one finding" }
    if ($script:Findings[0].Passed -cne $case.Passed) {
        throw "$($case.Name): expected Passed=$($case.Passed), got $($script:Findings[0].Passed)"
    }
    if (($null -eq $failure) -ne $case.Passed) {
        throw "$($case.Name): failure did not stop the gate, or success unexpectedly threw"
    }
    Write-Output "$($case.Name): verdict verified"
}
Write-Output 'PASS: four TCP evidence verdicts; no installation performed'
