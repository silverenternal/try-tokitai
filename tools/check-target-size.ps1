param(
    [double] $WarnGB = 8,
    [double] $HeavyGB = 12
)

$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent $PSScriptRoot
$targetRoot = Join-Path $projectRoot "target"

if (-not (Test-Path -LiteralPath $targetRoot)) {
    Write-Output "target directory not found: $targetRoot"
    exit 0
}

function Get-DirSizeBytes {
    param([string] $Path)

    if (-not (Test-Path -LiteralPath $Path)) {
        return [int64]0
    }

    $size = (Get-ChildItem -LiteralPath $Path -Recurse -Force -File -ErrorAction SilentlyContinue |
        Measure-Object -Property Length -Sum).Sum

    if ($null -eq $size) {
        return [int64]0
    }

    return [int64]$size
}

$targetBytes = Get-DirSizeBytes -Path $targetRoot
$debugBytes = Get-DirSizeBytes -Path (Join-Path $targetRoot "debug")
$releaseBytes = Get-DirSizeBytes -Path (Join-Path $targetRoot "release")

$targetGB = [math]::Round(($targetBytes / 1GB), 3)
$targetMB = [math]::Round(($targetBytes / 1MB), 2)
$debugGB = [math]::Round(($debugBytes / 1GB), 3)
$releaseGB = [math]::Round(($releaseBytes / 1GB), 3)

Write-Output "target size: $targetGB GB ($targetMB MB)"
Write-Output "debug size:  $debugGB GB"
Write-Output "release size: $releaseGB GB"
Write-Output "thresholds: warn >= $WarnGB GB, heavy >= $HeavyGB GB"

if ($targetGB -ge $HeavyGB) {
    Write-Output ""
    Write-Output "Status: HEAVY"
    Write-Output "Recommendation: run heavy cleanup."
    Write-Output "Command: powershell -ExecutionPolicy Bypass -File tools\\clean-heavy.ps1"
    exit 2
}

if ($targetGB -ge $WarnGB) {
    Write-Output ""
    Write-Output "Status: WARN"
    Write-Output "Recommendation: run cache cleanup."
    Write-Output "Command: powershell -ExecutionPolicy Bypass -File tools\\clean-cache.ps1"
    exit 1
}

Write-Output ""
Write-Output "Status: OK"
Write-Output "Recommendation: no cleanup needed right now."
