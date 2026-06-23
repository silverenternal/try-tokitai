$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent $PSScriptRoot
$targetRoot = Join-Path $projectRoot "target"

if (-not (Test-Path -LiteralPath $targetRoot)) {
    Write-Output "target directory not found: $targetRoot"
    exit 0
}

$paths = @(
    (Join-Path $targetRoot "debug\\incremental"),
    (Join-Path $targetRoot "debug\\build"),
    (Join-Path $targetRoot "debug\\.fingerprint"),
    (Join-Path $targetRoot "debug\\examples"),
    (Join-Path $targetRoot "release\\incremental"),
    (Join-Path $targetRoot "release\\build"),
    (Join-Path $targetRoot "release\\.fingerprint"),
    (Join-Path $targetRoot "release\\examples"),
    (Join-Path $targetRoot "tmp"),
    (Join-Path $targetRoot "stream-body.json"),
    (Join-Path $targetRoot "web-direct.err.log"),
    (Join-Path $targetRoot "web-direct.out.log"),
    (Join-Path $targetRoot "web-run.err.log"),
    (Join-Path $targetRoot "web-run.log")
)

$removed = @()
foreach ($path in $paths) {
    if (Test-Path -LiteralPath $path) {
        Remove-Item -LiteralPath $path -Recurse -Force
        $removed += $path
    }
}

if ($removed.Count -eq 0) {
    Write-Output "No cache artifacts removed."
} else {
    Write-Output "Removed cache artifacts:"
    $removed | ForEach-Object { Write-Output " - $_" }
}
