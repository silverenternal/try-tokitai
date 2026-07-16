# Test Strategy Notes

## Test Timeout Protection

### Problem
Rust's `#[timeout]` attribute is **nightly-only** and not available on stable Rust. Without timeout protection, tests that enter infinite loops or deadlock will hang the entire test suite.

### Current Protection

1. **CI-level timeout**: The GitHub Actions workflow (`.github/workflows/ci.yml`) sets `timeout-minutes: 10` for the unit test job (`cargo test --lib`), which will kill the job if it exceeds 10 minutes.

2. **All tests run by default**: All 630 tests run by default with `cargo test`. No tests are marked as `#[ignore]` (except 3 stability tests).

### Running Tests Locally
```bash
# With cargo-nextest (recommended):
cargo nextest run --lib --test-timeout 120s

# With plain cargo test:
cargo test --lib --all-features
```

### Future Improvements

1. **cargo-nextest**: Already adopted, supports `--test-timeout` natively:
   ```bash
   cargo install cargo-nextest
   cargo nextest run --lib --test-timeout 120s
   ```

2. **Per-test timeout**: If per-test timeout becomes important, consider:
   - Creating integration tests in `tests/` directory with explicit timeout
   - Using `serial_test` crate to isolate slow tests

## Test Coverage

- **Lib Tests**: 625/625 (100% pass rate)
- **Integration Tests**: 28/28 (100% pass rate)
- **Doctests**: 16/16 passed, 7 ignored (expected)
- **High Concurrency Tests**: 9 tests (previously ignored, now run by default since v0.4.0)
- **Clippy**: 0 warnings

### Test Modules

Tests are distributed across 46+ test modules for parallel execution.
