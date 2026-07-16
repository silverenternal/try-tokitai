# Atlas desktop platform support

Atlas uses the native Wry webview on all desktop systems: WebView2 on Windows, WebKit on macOS,
and WebKitGTK on Linux. The IDE terminal and run/debug process launcher select the host shell at
runtime (`powershell.exe` on Windows and `$SHELL`, falling back to `/bin/sh`, on Unix).

## Build

```sh
cargo build --release --features desktop-shell --bin desktop_wry
```

Linux requires WebKitGTK and GTK development packages. On Debian/Ubuntu:

```sh
sudo apt-get install libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev
```

macOS requires the Xcode Command Line Tools. Windows requires the WebView2 runtime. CI checks the
core IDE and desktop host on Windows, Ubuntu, and macOS for every pull request.
