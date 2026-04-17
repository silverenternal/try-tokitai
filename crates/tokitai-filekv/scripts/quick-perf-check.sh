#!/usr/bin/env bash
# ============================================================================
# Quick Performance Check - Single benchmark without full Criterion run
# ============================================================================
# Runs a fast (non-statistical) performance check to validate key operations
# are within acceptable bounds. Much faster than full benchmarks.
#
# Usage:
#   ./scripts/quick-perf-check.sh
#
# Checks:
#   - get (hot cache) < 500ns
#   - get (cold cache) < 1000ns
#   - put (no WAL, 64B) < 5µs
#   - put (WAL, 64B) < 10µs
#   - delete < 1µs
# ============================================================================

set -euo pipefail

cd "$(dirname "$0")/.."

# Thresholds (conservative, ~2x the baseline to catch major regressions)
HOT_CACHE_NS=500
COLD_CACHE_NS=1000
PUT_NOWAL_US=5
PUT_WAL_US=10
DELETE_NS=1000

echo "=== Quick Performance Check ==="
echo ""

# Build first
cargo build --release --features benchmarks 2>&1 | tail -1

# Run the quick check binary
cargo run --release --features benchmarks --example quick_perf_check 2>&1

echo ""
echo "=== Quick Performance Check Complete ==="
