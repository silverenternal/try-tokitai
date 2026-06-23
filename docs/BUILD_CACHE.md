# Build Cache Management

## Overview

This project is Rust-based, so most disk growth comes from `target/` rather than source files.

Typical breakdown:

- `target/debug/`: daily development artifacts
- `target/release/`: release build artifacts
- `incremental/`, `build/`, `.fingerprint/`: rebuild acceleration caches
- `deps/`: compiled dependency artifacts

The large caches are useful for faster recompilation, but they are not all required to keep the current executable runnable.

## Recommended Strategy

1. Use `debug` for day-to-day development.
2. Build `release` only for shipping or performance testing.
3. Keep `debug/deps` when actively developing.
4. Regularly remove `incremental`, `build`, and `.fingerprint`.
5. Remove `release/` after a release build is no longer needed.

## Thresholds

- `< 8 GB`: no cleanup needed
- `>= 8 GB`: run light cleanup
- `>= 12 GB`: run heavy cleanup

## Commands

Unified entrypoint:

```powershell
powershell -ExecutionPolicy Bypass -File tools\maintain.ps1 check
powershell -ExecutionPolicy Bypass -File tools\maintain.ps1 clean-light
powershell -ExecutionPolicy Bypass -File tools\maintain.ps1 clean-heavy
```

Check current `target` size:

```powershell
powershell -ExecutionPolicy Bypass -File tools\check-target-size.ps1
```

Light cleanup:

```powershell
powershell -ExecutionPolicy Bypass -File tools\clean-cache.ps1
```

Heavy cleanup:

```powershell
powershell -ExecutionPolicy Bypass -File tools\clean-heavy.ps1
```

## What The Scripts Do

### `tools/clean-cache.ps1`

Removes:

- `target/debug/incremental`
- `target/debug/build`
- `target/debug/.fingerprint`
- `target/debug/examples`
- `target/release/incremental`
- `target/release/build`
- `target/release/.fingerprint`
- `target/release/examples`
- temporary logs and small transient files under `target/`

This is the preferred routine cleanup.

### `tools/clean-heavy.ps1`

Removes:

- the entire `target/release`
- debug cache directories such as `incremental`, `build`, `.fingerprint`, and `examples`
- temporary logs and transient files under `target/`

Use this after release work or when `target/` becomes too large.

## Release Profile

The project disables incremental compilation for `release` builds:

```toml
[profile.release]
incremental = false
```

This helps keep release artifacts from growing unnecessarily over time.
