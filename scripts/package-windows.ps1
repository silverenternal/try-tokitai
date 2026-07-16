param(
    [string]$Version = "0.1.0",
    [string]$Configuration = "release",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$targetProfile = if ($Configuration -eq "release") { "release" } else { "debug" }
$dist = Join-Path $root "dist"
$bundle = Join-Path $dist "Atlas-$Version-windows-x64"

if (-not $SkipBuild) {
    if ($Configuration -eq "release") {
        & cargo build --release --features desktop-shell --bin desktop_wry
    } else {
        & cargo build --features desktop-shell --bin desktop_wry
    }
    if ($LASTEXITCODE -ne 0) { throw "Atlas desktop build failed." }
}

if (Test-Path $bundle) { Remove-Item -Recurse -Force $bundle }
New-Item -ItemType Directory -Force (Join-Path $bundle "frontend") | Out-Null
Copy-Item (Join-Path $root "target\$targetProfile\desktop_wry.exe") (Join-Path $bundle "Atlas.exe")
Copy-Item (Join-Path $root "frontend\*") (Join-Path $bundle "frontend") -Recurse -Force

$license = Join-Path $root "LICENSE"
if (Test-Path $license) { Copy-Item $license $bundle }
Copy-Item (Join-Path $root "README.md") $bundle

$archive = Join-Path $dist "Atlas-$Version-windows-x64.zip"
if (Test-Path $archive) { Remove-Item -Force $archive }
Compress-Archive -Path (Join-Path $bundle "*") -DestinationPath $archive -CompressionLevel Optimal
Write-Host "Created $archive"
