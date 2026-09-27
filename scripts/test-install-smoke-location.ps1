$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# Exercise the real registry reader, path guards, and uninstaller selection.
# Registry/process cmdlets are controlled doubles; no installation is changed.
$scriptPath = if ($args.Count -gt 0) { $args[0] } else { Join-Path $PSScriptRoot 'install-smoke.ps1' }
$tokens = $null
$parseErrors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile(
    (Resolve-Path -LiteralPath $scriptPath).Path, [ref] $tokens, [ref] $parseErrors)
if ($parseErrors.Count -ne 0) { throw "Script does not parse: $parseErrors" }
foreach ($name in @('Get-InstalledEntry', 'Test-PathIsUnder', 'Invoke-SilentUninstaller', 'Add-Finding', 'Assert-Finding')) {
    $definitions = @($ast.FindAll({
        param($node)
        $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq $name
    }, $false))
    if ($definitions.Count -ne 1) { throw "Expected one $name function" }
    . ([scriptblock]::Create($definitions[0].Extent.Text))
}
$guards = @($ast.FindAll({
    param($node)
    $node -is [System.Management.Automation.Language.IfStatementAst] -and
        $node.Clauses[0].Item1.Extent.Text -eq '$willInstall -and $AppExecutable'
}, $false))
if ($guards.Count -ne 1) { throw 'Expected one installed-location guard' }
$guard = [scriptblock]::Create($guards[0].Extent.Text)
$script:ProductName = 'Soul'
$script:RegistryRoot = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall'
function Test-Path {
    [CmdletBinding()]
    param([string] $LiteralPath)
    if ($LiteralPath -ceq $script:RegistryRoot -or $LiteralPath -ceq $script:ExpectedUninstaller) { return $true }
    return Microsoft.PowerShell.Management\Test-Path -LiteralPath $LiteralPath
}
function Get-ChildItem {
    [CmdletBinding()]
    param([string] $LiteralPath)
    if ($LiteralPath -cne $script:RegistryRoot) { throw "Unexpected registry root: $LiteralPath" }
    [pscustomobject]@{ PSPath = 'synthetic-registry-key' }
}
function Get-ItemProperty {
    [CmdletBinding()]
    param([string] $LiteralPath)
    if ($LiteralPath -cne 'synthetic-registry-key') { throw "Unexpected registry key: $LiteralPath" }
    [pscustomobject]@{
        DisplayName = 'Soul'
        InstallLocation = $script:RawLocation
        UninstallString = '"C:\command path\uninstall.exe" /S'
        QuietUninstallString = '"C:\command path\uninstall.exe" /S /quiet'
    }
}
function Copy-Item {
    [CmdletBinding()]
    param([string] $LiteralPath, [string] $Destination)
    if ($LiteralPath -cne $script:ExpectedUninstaller) { throw 'Wrong uninstaller selected for copying' }
    $script:CopiedSource = $LiteralPath
    $script:CopiedDestination = $Destination
    'synthetic runner; never executed' | Set-Content -LiteralPath $Destination -Encoding UTF8
}
function Start-Process {
    [CmdletBinding()]
    param([string] $FilePath, [string[]] $ArgumentList, [switch] $Wait, [switch] $PassThru)
    $script:Launches.Add([pscustomobject]@{ Path=$FilePath; Arguments=$ArgumentList; Wait=[bool]$Wait; PassThru=[bool]$PassThru })
    [pscustomobject]@{ ExitCode = 0 }
}

$programs = Join-Path (Join-Path $env:LOCALAPPDATA 'Programs') 'Soul'
$data = Join-Path $env:LOCALAPPDATA 'Soul'
$withSpaces = Join-Path $programs 'folder with spaces\中文'
$cases = @(
    @{ Name='bare Programs'; Raw=$programs; Expected=$programs; Passes=$true; Unsafe=$false },
    @{ Name='quoted Programs'; Raw='"'+$programs+'"'; Expected=$programs; Passes=$true; Unsafe=$false },
    @{ Name='outer whitespace'; Raw='  "'+$programs+'"  '; Expected=$programs; Passes=$true; Unsafe=$false },
    @{ Name='spaces and Unicode'; Raw='"'+$withSpaces+'"'; Expected=$withSpaces; Passes=$true; Unsafe=$false },
    @{ Name='quoted data directory'; Raw='"'+$data+'"'; Expected=$data; Passes=$false; Unsafe=$true },
    @{ Name='quoted data trailing separator'; Raw='"'+$data+'\"'; Expected=$data+'\'; Passes=$false; Unsafe=$true },
    @{ Name='sibling prefix'; Raw='"'+$programs+'Extra"'; Expected=$programs+'Extra'; Passes=$false; Unsafe=$false },
    @{ Name='unmatched opening quote'; Raw='"'+$programs; Expected='"'+$programs; Passes=$false; Unsafe=$false },
    @{ Name='unmatched closing quote'; Raw=$programs+'"'; Expected=$programs+'"'; Passes=$false; Unsafe=$false }
)
foreach ($case in $cases) {
    $script:RawLocation = $case.Raw
    $script:ExpectedUninstaller = Join-Path $programs 'uninstall.exe'
    $script:Launches = [System.Collections.Generic.List[object]]::new()
    $script:CopiedSource = $null
    $script:CopiedDestination = $null
    $entry = Get-InstalledEntry
    if ($null -eq $entry -or $entry.InstallLocation -cne $case.Expected) {
        throw "$($case.Name): InstallLocation was not decoded as the expected directory"
    }
    if ($entry.UninstallString -cne '"C:\command path\uninstall.exe" /S' -or
        $entry.QuietUninstall -cne '"C:\command path\uninstall.exe" /S /quiet') {
        throw 'Command-line registry fields must not be decoded as paths'
    }
    $script:Findings = [System.Collections.Generic.List[object]]::new()
    $script:UninstallIsUnsafe = $false
    $willInstall = $true
    $AppExecutable = Join-Path $programs 'soul.exe'
    $failure = $null
    try { & $guard } catch { $failure = $_ }
    if (($null -eq $failure) -ne $case.Passes -or $script:UninstallIsUnsafe -cne $case.Unsafe) {
        throw "$($case.Name): path guard or data-protection decision was incorrect"
    }
    if ($case.Passes) {
        $script:ExpectedUninstaller = Join-Path $case.Expected 'uninstall.exe'
        $code = Invoke-SilentUninstaller -InstallerPath 'synthetic-setup.exe' -LogDirectory '.' -Entry $entry
        if ($code -ne 0 -or $script:Launches.Count -ne 1) { throw 'Expected one successful uninstaller invocation' }
        $launch = $script:Launches[0]
        if (Microsoft.PowerShell.Management\Test-Path -LiteralPath (Split-Path -Parent $script:CopiedDestination)) { throw 'Temporary runner was not cleaned' }
        if ($script:CopiedSource -cne $script:ExpectedUninstaller -or $launch.Path -cne $script:CopiedDestination -or -not $launch.Wait -or -not $launch.PassThru -or
            $launch.Arguments.Count -ne 2 -or $launch.Arguments[0] -cne '/S' -or
            $launch.Arguments[1] -cne ('_?='+$case.Expected)) {
            throw "$($case.Name): uninstaller path or arguments were changed"
        }
    }
    Write-Output "$($case.Name): registry decoding, guard and process boundary verified"
}
Write-Output 'PASS: nine InstallLocation cases; no registry writes or installation performed'
