# Test Strategy Notes

## Test Timeout Protection

### Problem
Rust's `#[timeout]` attribute is **nightly-only** and not available on stable Rust. Without timeout protection, tests that enter infinite loops or deadlock will hang the entire test suite.

### Current Protection

1. **CI-level timeout**: The GitHub Actions workflow (`.github/workflows/ci.yml`) sets `timeout-minutes: 10` for the unit test job (`cargo test --lib`), which will kill the job if it exceeds 10 minutes.

2. **Slow tests marked `#[ignore]`**: The following compaction tests are known to run >60 seconds and are marked `#[ignore]` to exclude them from default test runs:
   - `test_filekv_compaction` (src/tests/integration.rs)
   - `test_filekv_parallel_compaction` (src/tests/integration.rs)
   - `test_background_compaction_actually_works` (src/tests/integration.rs)

   These can be run explicitly with:
   ```bash
   cargo test --lib -- --ignored
   ```

### Running Slow Tests Locally
If you need to run slow tests locally, set a generous timeout:
```bash
# With cargo-nextest (recommended):
cargo nextest run --lib --test-timeout 300s -- --ignored

# With plain cargo test:
timeout 300 cargo test --lib -- --ignored
```

### Future Improvements

1. **cargo-nextest**: Consider adopting `cargo-nextest` which supports `--test-timeout` natively:
   ```bash
   cargo install cargo-nextest
   cargo nextest run --lib --test-timeout 120s
   ```

2. **Per-test timeout**: If per-test timeout becomes important, consider:
   - Creating integration tests in `tests/` directory with explicit timeout
   - Using `serial_test` crate to isolate slow tests

## Ignored Tests

Tests marked with `#[ignore]` are excluded from `cargo test` by default. They fall into two categories:

### Slow Compaction Tests (>60s)
These tests involve real compaction operations with background threads:
- `test_filekv_compaction`
- `test_filekv_parallel_compaction`
- `test_background_compaction_actually_works`

### Stability Tests (>3s, frequent runs cause noise)
These tests are designed for long-running validation:
- `test_short_running_stability`
- `test_memory_leak_detection`
- `test_performance_stability`
