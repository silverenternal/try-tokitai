param(
    [ValidateSet("check", "clean-light", "clean-heavy")]
    [string] $Action = "check"
)

$ErrorActionPreference = "Stop"

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path

switch ($Action) {
    "check" {
        & (Join-Path $scriptRoot "check-target-size.ps1")
        break
    }
    "clean-light" {
        & (Join-Path $scriptRoot "clean-cache.ps1")
        break
    }
    "clean-heavy" {
        & (Join-Path $scriptRoot "clean-heavy.ps1")
        break
    }
    default {
        throw "Unsupported action: $Action"
    }
}
