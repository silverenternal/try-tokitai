# Atlas desktop releases

Atlas uses Wry with WebView2 on Windows, WebKit on macOS, and WebKitGTK on Linux. The UI remains regular frontend code and the Rust host remains modular, so IDE capabilities can evolve independently of the distribution model.

## Local Windows package

```powershell
./scripts/package-windows.ps1 -Version 0.1.0
```

This produces a portable ZIP under `dist/`. The package contains `Atlas.exe` and the required `frontend` resources. Windows 10/11 normally provides the WebView2 runtime; machines without it must install the Microsoft Edge WebView2 Runtime.

## GitHub release

Push a semantic version tag such as `v0.1.0`. The `Release Atlas Desktop` workflow builds both a portable ZIP and an Inno Setup installer and attaches them to the GitHub Release. It can also be run manually to create downloadable workflow artifacts without publishing a Release.

Before a public release, add code signing secrets and sign both `Atlas.exe` and the installer. Unsigned builds will trigger Windows SmartScreen warnings.
