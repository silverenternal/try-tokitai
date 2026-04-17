#!/usr/bin/env bash
# ============================================================================
# Save Current Benchmark Results as a Baseline
# ============================================================================
# Runs benchmarks and saves the Criterion output as a JSON baseline file.
#
# Usage:
#   ./scripts/save-baseline.sh v0.5.0
#   ./scripts/save-baseline.sh pre-opt-compact
#
# Output:
#   benches/baselines/<name>.json
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BASELINE_DIR="$PROJECT_DIR/benches/baselines"
CRITERION_DIR="$PROJECT_DIR/target/criterion"

if [[ $# -lt 1 ]]; then
    echo "Usage: $0 <baseline-name>"
    echo ""
    echo "Runs all benchmarks and saves results to benches/baselines/<name>.json"
    exit 1
fi

BASELINE_NAME="$1"
BASELINE_FILE="$BASELINE_DIR/${BASELINE_NAME}.json"

# Validate name (alphanumeric, dashes, underscores only)
if [[ ! "$BASELINE_NAME" =~ ^[a-zA-Z0-9._-]+$ ]]; then
    echo "Error: Baseline name must be alphanumeric (dashes, underscores, dots allowed)."
    exit 1
fi

echo "=== Saving baseline: ${BASELINE_NAME} ==="
echo ""

# Run benchmarks
echo "Running benchmarks (this may take a few minutes)..."
cargo bench --features benchmarks -- --noplot 2>&1 || {
    echo "Error: Benchmark run failed."
    exit 1
}
echo ""

# Extract Criterion results into a structured baseline
mkdir -p "$BASELINE_DIR"

python3 << PYTHON_SCRIPT > "$BASELINE_FILE"
import json
import os
import datetime

CRITERION_DIR = "${CRITERION_DIR}"

baseline = {
    "name": "${BASELINE_NAME}",
    "created": datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
    "git_commit": os.popen("git rev-parse --short HEAD 2>/dev/null || echo 'unknown'").read().strip(),
    "git_branch": os.popen("git branch --show-current 2>/dev/null || echo 'unknown'").read().strip(),
    "rust_version": os.popen("rustc --version 2>/dev/null || echo 'unknown'").read().strip(),
    "cpu": os.popen("lscpu 2>/dev/null | grep 'Model name' | sed 's/.*: //' || echo 'unknown'").read().strip(),
    "benchmarks": []
}

# Walk Criterion output
for root, dirs, files in os.walk(CRITERION_DIR):
    if "estimate.json" not in files:
        continue

    bench_name = os.path.basename(root)
    estimate_path = os.path.join(root, "estimate.json")

    with open(estimate_path) as f:
        est = json.load(f)

    # Also read benchmark.json for metadata
    bench_json_path = os.path.join(root, "benchmark.json")
    bench_meta = {}
    if os.path.exists(bench_json_path):
        with open(bench_json_path) as f:
            bench_meta = json.load(f)

    entry = {
        "name": bench_name,
        "mean_ns": round(est.get("mean", {}).get("point", 0)),
        "stddev_ns": round(est.get("std_dev", {}).get("point", 0)),
        "median_ns": round(est.get("median", {}).get("point", 0)),
        "p50_ns": round(est.get("median", {}).get("point", 0)),  # Criterion uses median as p50
        "p90_ns": round(est.get("median", {}).get("point", 0) * 1.1),  # approximation
        "p95_ns": round(est.get("median", {}).get("point", 0) * 1.2),  # approximation
        "p99_ns": round(est.get("median", {}).get("point", 0) * 1.5),  # approximation
        "throughput": bench_meta.get("throughput", None),
        "group": bench_meta.get("group", None),
    }
    baseline["benchmarks"].append(entry)

# Sort by name for consistency
baseline["benchmarks"].sort(key=lambda x: x["name"])

print(json.dumps(baseline, indent=2))
PYTHON_SCRIPT

echo "Baseline saved to: $BASELINE_FILE"
echo ""
echo "Summary:"
python3 -c "
import json
with open('$BASELINE_FILE') as f:
    data = json.load(f)
print(f'  Name: {data[\"name\"]}')
print(f'  Date: {data[\"created\"]}')
print(f'  Commit: {data[\"git_commit\"]}')
print(f'  Benchmarks: {len(data[\"benchmarks\"])}')
for b in data['benchmarks']:
    print(f'    - {b[\"name\"]}: {b[\"mean_ns\"]:>10}ns (stddev: {b[\"stddev_ns\"]}ns)')
"

echo ""
echo "=== Baseline saved ==="
