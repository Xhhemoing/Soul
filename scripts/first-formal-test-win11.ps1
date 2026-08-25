# Preflight for the first formal test day. Does not install Soul.
# Run in Windows PowerShell 5+ as a non-admin user, from the repo root.
$ErrorActionPreference = "Stop"

function Fail([string]$Message) {
    Write-Error $Message
    exit 1
}

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if ($principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Fail "AC-01 requires a non-admin account. Close this elevated window."
}

$root = git rev-parse --show-toplevel
if ($LASTEXITCODE -ne 0) { Fail "Run this from a Soul checkout." }
Set-Location $root

$sha = (git rev-parse HEAD).Trim()
$short = (git rev-parse --short HEAD).Trim()
Write-Host "HEAD $short ($sha)"

if ($short -eq "2e72ddf" -or $sha.StartsWith("2e72ddf")) {
    Fail "Do not test 2e72ddf. That installer put soul.exe in the data directory."
}

foreach ($p in @(
    "docs/FIRST_FORMAL_TEST.md",
    "scripts/author-manual-checklist.md",
    "scripts/install-smoke.ps1"
)) {
    if (-not (Test-Path $p)) { Fail "Missing $p" }
}

Write-Host "First formal test preflight ok."
Write-Host "Build this SHA (478f19f or later on cursor/soul-goal1-7b1c)."
Write-Host "Then walk scripts/author-manual-checklist.md and write what you saw into docs/STATUS.md."
Write-Host "This script is not AC-01. The installer UAC shield is still for your eyes."
exit 0
