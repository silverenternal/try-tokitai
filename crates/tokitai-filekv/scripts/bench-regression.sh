#!/usr/bin/env bash
# ============================================================================
# Performance Regression Detection Script
# ============================================================================
# Compares current benchmark results against stored baselines.
# Flags any metric that regressed beyond the threshold.
#
# Usage:
#   ./scripts/bench-regression.sh              # Compare all benchmarks
#   ./scripts/bench-regression.sh --threshold 5  # 5% regression threshold
#   ./scripts/bench-regression.sh --baseline v0.5.0 --bench 01_basic_ops
#
# Exit codes:
#   0 - No regressions detected
#   1 - Regressions found (see report)
#   2 - Baseline not found or benchmark failed
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BASELINE_DIR="$PROJECT_DIR/benches/baselines"
CRITERION_DIR="$PROJECT_DIR/target/criterion"

# Defaults
THRESHOLD=5.0  # percent regression threshold
BASELINE_NAME=""
BENCH_FILTER=""
JSON_OUTPUT=true

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# ============================================================================
# Argument parsing
# ============================================================================

while [[ $# -gt 0 ]]; do
    case "$1" in
        --threshold) THRESHOLD="$2"; shift 2 ;;
        --baseline) BASELINE_NAME="$2"; shift 2 ;;
        --bench) BENCH_FILTER="$2"; shift 2 ;;
        --no-json) JSON_OUTPUT=false; shift ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --threshold N     Regression threshold in percent (default: 5)"
            echo "  --baseline NAME   Baseline name (default: latest in benches/baselines/)"
            echo "  --bench NAME      Only check specific benchmark"
            echo "  --no-json         Disable JSON output"
            exit 0
            ;;
        *) echo "Unknown option: $1"; exit 2 ;;
    esac
done

# ============================================================================
# Helper functions
# ============================================================================

find_latest_baseline() {
    if [[ -n "$BASELINE_NAME" ]]; then
        echo "$BASELINE_NAME"
        return
    fi
    # Find the most recent baseline file
    ls -t "$BASELINE_DIR"/*.json 2>/dev/null | head -1 | xargs basename | sed 's/\.json$//'
}

extract_mean() {
    # Extract mean time (nanoseconds) from Criterion's estimate.json
    local criterion_json="$1"
    if [[ -f "$criterion_json" ]]; then
        python3 -c "
import json
with open('$criterion_json') as f:
    data = json.load(f)
print(data.get('mean', {}).get('point', 0))
" 2>/dev/null || echo "0"
    else
        echo "0"
    fi
}

# ============================================================================
# Main logic
# ============================================================================

echo -e "${BLUE}=== Performance Regression Detection ===${NC}"
echo ""

# Find baseline
BASELINE=$(find_latest_baseline)
if [[ -z "$BASELINE" ]]; then
    echo -e "${YELLOW}No baseline found in $BASELINE_DIR/${NC}"
    echo "Run: just save-baseline <name> to create one first."
    exit 2
fi

BASELINE_FILE="$BASELINE_DIR/${BASELINE}.json"
echo -e "Baseline: ${GREEN}${BASELINE}${NC} ($BASELINE_FILE)"
echo -e "Threshold: ${YELLOW}${THRESHOLD}%${NC}"
echo ""

# Check if baseline file exists
if [[ ! -f "$BASELINE_FILE" ]]; then
    echo -e "${RED}Error: Baseline file not found: $BASELINE_FILE${NC}"
    exit 2
fi

# Run benchmarks (output only, don't parse yet)
echo -e "${BLUE}Running benchmarks...${NC}"
BENCH_CMD="cargo bench --features benchmarks -- --noplot 2>&1"
if [[ -n "$BENCH_FILTER" ]]; then
    BENCH_CMD="cargo bench --features benchmarks -- '$BENCH_FILTER' --noplot 2>&1"
fi
eval "$BENCH_CMD" > /tmp/bench_output.log 2>&1 || {
    echo -e "${RED}Benchmark run failed. See /tmp/bench_output.log${NC}"
    exit 2
}
echo -e "${GREEN}Benchmarks completed.${NC}"
echo ""

# Parse Criterion results and compare
python3 << 'PYTHON_SCRIPT'
import json
import os
import sys
import glob

BASELINE_FILE = os.environ.get("BASELINE_FILE", "")
CRITERION_DIR = os.environ.get("CRITERION_DIR", "")
THRESHOLD = float(os.environ.get("THRESHOLD", "5.0"))
BENCH_FILTER = os.environ.get("BENCH_FILTER", "")

regressions = []
improvements = []
unchanged = []

# Walk Criterion output directories
for root, dirs, files in os.walk(CRITERION_DIR):
    if "estimate.json" in files:
        bench_name = os.path.basename(root)

        if BENCH_FILTER and BENCH_FILTER not in bench_name:
            continue

        estimate_path = os.path.join(root, "estimate.json")
        with open(estimate_path) as f:
            est = json.load(f)

        current_mean_ns = est.get("mean", {}).get("point", 0)

        # Look up baseline value
        baseline_mean_ns = None
        if os.path.exists(BASELINE_FILE):
            with open(BASELINE_FILE) as f:
                baseline_data = json.load(f)
            for entry in baseline_data.get("benchmarks", []):
                if entry["name"] == bench_name:
                    baseline_mean_ns = entry.get("mean_ns", 0)
                    break

        if baseline_mean_ns is None or baseline_mean_ns == 0:
            # No baseline for this benchmark, skip
            continue

        # Calculate percent change: positive = regression (slower), negative = improvement
        pct_change = ((current_mean_ns - baseline_mean_ns) / baseline_mean_ns) * 100

        result = {
            "name": bench_name,
            "baseline_ns": round(baseline_mean_ns),
            "current_ns": round(current_mean_ns),
            "pct_change": round(pct_change, 2)
        }

        if pct_change > THRESHOLD:
            regressions.append(result)
        elif pct_change < -THRESHOLD:
            improvements.append(result)
        else:
            unchanged.append(result)

# Output results
print("=" * 70)
print("BENCHMARK COMPARISON RESULTS")
print(f"Baseline: {os.path.basename(BASELINE_FILE).replace('.json', '')}")
print(f"Threshold: {THRESHOLD}%")
print("=" * 70)
print()

# Table header
print(f"{'Benchmark':<45} {'Baseline':>10} {'Current':>10} {'Change':>8}  Status")
print(f"{'':<45} {'(ns)':>10} {'(ns)':>10}")
print("-" * 70)

for r in regressions:
    print(f"\033[1;31m{r['name']:<45} {r['baseline_ns']:>10} {r['current_ns']:>10} {r['pct_change']:>+7.1f}%  REGRESSION\033[0m")

for r in improvements:
    print(f"\033[1;32m{r['name']:<45} {r['baseline_ns']:>10} {r['current_ns']:>10} {r['pct_change']:>+7.1f}%  IMPROVEMENT\033[0m")

for r in unchanged:
    print(f"{r['name']:<45} {r['baseline_ns']:>10} {r['current_ns']:>10} {r['pct_change']:>+7.1f}%  OK")

print()
print("=" * 70)

if regressions:
    print(f"\033[1;31mREGRESSIONS: {len(regressions)}\033[0m")
    for r in regressions:
        print(f"  - {r['name']}: {r['baseline_ns']}ns -> {r['current_ns']}ns ({r['pct_change']:+.1f}%)")

if improvements:
    print(f"\033[1;32mIMPROVEMENTS: {len(improvements)}\033[0m")
    for r in improvements:
        print(f"  + {r['name']}: {r['baseline_ns']}ns -> {r['current_ns']}ns ({r['pct_change']:+.1f}%)")

print(f"\033[0;34mUNCHANGED: {len(unchanged)}\033[0m")
print()

if regressions:
    print(f"\033[1;31mRESULT: {len(regressions)} regression(s) detected!\033[0m")
    sys.exit(1)
else:
    print(f"\033[1;32mRESULT: No regressions detected.\033[0m")
    sys.exit(0)
PYTHON_SCRIPT

exit_code=$?

# ============================================================================
# Save this run as a potential new baseline
# ============================================================================

if [[ $exit_code -eq 0 ]]; then
    echo ""
    echo -e "${GREEN}=== No regressions ===${NC}"
    echo "To save current results as a new baseline:"
    echo "  just save-baseline <name>"
fi

exit $exit_code
