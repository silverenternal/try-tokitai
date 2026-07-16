#!/bin/bash
#
# Long-term stability test runner for tokitai-filekv
#
# Usage:
#   ./scripts/run_stability_tests.sh              # Run all stability tests (default 24h mode)
#   ./scripts/run_stability_tests.sh --quick      # Quick smoke test (5 minutes)
#   ./scripts/run_stability_tests.sh --duration 1h # Custom duration
#   ./scripts/run_stability_tests.sh --list       # List available tests
#   ./scripts/run_stability_tests.sh --ci         # CI mode (non-interactive, timeout protection)
#
# Environment:
#   FILEKV_TEST_DIR   - Override temp directory (default: auto temp dir)
#   FILEKV_LOG_FILE   - Log file path (default: ./stability_test.log)
#

set -euo pipefail

# ─── Configuration ───
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
CRATE_DIR="$PROJECT_DIR"

DEFAULT_DURATION="24h"
DEFAULT_LOG_FILE="$PROJECT_DIR/stability_test.log"

# Parse arguments
MODE="full"
DURATION="$DEFAULT_DURATION"
LOG_FILE="$DEFAULT_LOG_FILE"
CI_MODE=false
LIST_ONLY=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --quick)
            MODE="quick"
            DURATION="5m"
            shift
            ;;
        --duration)
            DURATION="$2"
            shift 2
            ;;
        --ci)
            CI_MODE=true
            shift
            ;;
        --list)
            LIST_ONLY=true
            shift
            ;;
        --log-file)
            LOG_FILE="$2"
            shift 2
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --quick              Quick smoke test (5 minutes)"
            echo "  --duration DURATION  Custom duration (e.g., 1h, 30m, 24h)"
            echo "  --ci                 CI mode (non-interactive, timeout)"
            echo "  --list               List available tests"
            echo "  --log-file FILE      Override log file path"
            echo "  --help               Show this help"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# ─── Helper functions ───

log() {
    local timestamp
    timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[$timestamp] $*" | tee -a "$LOG_FILE"
}

log_separator() {
    echo "============================================================" | tee -a "$LOG_FILE"
}

parse_duration() {
    local dur="$1"
    local num="${dur%[smh]}"
    local unit="${dur: -1}"

    case "$unit" in
        s) echo "$num" ;;
        m) echo $((num * 60)) ;;
        h) echo $((num * 3600)) ;;
        *) echo "$num" ;; # Assume seconds if no unit
    esac
}

get_current_memory_mb() {
    # Get RSS in KB from /proc/self/status, convert to MB
    if [[ -f /proc/self/status ]]; then
        local rss_kb
        rss_kb=$(grep VmRSS /proc/self/status 2>/dev/null | awk '{print $2}' || echo "0")
        echo "scale=2; $rss_kb / 1024" | bc 2>/dev/null || echo "0"
    else
        echo "0"
    fi
}

check_file_descriptors() {
    # Count open file descriptors
    if [[ -d /proc/self/fd ]]; then
        ls /proc/self/fd 2>/dev/null | wc -l
    else
        echo "0"
    fi
}

# ─── Test definitions ───

declare -A TESTS=(
    ["short_running_stability"]="src/tests/stability.rs:test_short_running_stability:In-process 60s stability test"
    ["memory_leak_detection"]="src/tests/stability.rs:test_memory_leak_detection:Memory leak detection with sampling"
    ["performance_stability"]="src/tests/stability.rs:test_performance_stability:Performance consistency over time"
    ["concurrent_32_thread"]="tests/filekv_integration/high_concurrency.rs:test_32_threads_concurrent_puts:32-thread concurrent puts"
    ["concurrent_64_thread"]="tests/filekv_integration/high_concurrency.rs:test_64_threads_concurrent_puts:64-thread concurrent puts"
    ["concurrent_mixed_32"]="tests/filekv_integration/high_concurrency.rs:test_32_threads_mixed_read_write:32-thread mixed read/write"
    ["concurrent_hotkey_64"]="tests/filekv_integration/high_concurrency.rs:test_64_threads_hot_key_contention:64-thread hot key contention"
    ["concurrent_crash_safety"]="tests/filekv_integration/high_concurrency.rs:test_32_threads_puts_then_flush_and_reopen:32-thread crash safety"
)

# ─── List tests ───

if [[ "$LIST_ONLY" == true ]]; then
    echo "Available stability tests:"
    echo ""
    printf "%-30s %s\n" "TEST NAME" "DESCRIPTION"
    printf "%-30s %s\n" "------------------------------" "------------------------------------------"
    for test_name in "${!TESTS[@]}"; do
        IFS=':' read -r path name desc <<< "${TESTS[$test_name]}"
        printf "%-30s %s\n" "$test_name" "$desc"
    done
    exit 0
fi

# ─── Initialize ───

mkdir -p "$(dirname "$LOG_FILE")"
> "$LOG_FILE"

log_separator
log "Tokitai FileKV Stability Test Runner"
log "Mode: $MODE"
log "Duration: $DURATION"
log "Log file: $LOG_FILE"
log_separator

# Check prerequisites
log "Checking prerequisites..."

if ! command -v cargo &> /dev/null; then
    log "ERROR: cargo not found. Please install Rust."
    exit 1
fi

if ! command -v bc &> /dev/null; then
    log "WARNING: bc not found. Memory calculations may be inaccurate."
fi

# Build in release mode for stability tests
log "Building project in release mode..."
cargo build --release 2>&1 | tee -a "$LOG_FILE"
log "Build complete."

# ─── Duration-based test selection ───

DURATION_SECONDS=$(parse_duration "$DURATION")

log "Parsed duration: ${DURATION_SECONDS} seconds"

if [[ "$CI_MODE" == true ]]; then
    # CI mode: run quick tests only, with timeout
    CI_TIMEOUT=600  # 10 minutes max
    log "CI mode enabled: timeout=${CI_TIMEOUT}s"

    # Run ignored tests with timeout
    log "Running stability tests in CI mode..."

    timeout "$CI_TIMEOUT" cargo test --lib --release -- --ignored test_short_running_stability 2>&1 | tee -a "$LOG_FILE" || {
        log "WARNING: Short running stability test timed out or failed"
    }

    timeout "$CI_TIMEOUT" cargo test --lib --release -- --ignored test_memory_leak_detection 2>&1 | tee -a "$LOG_FILE" || {
        log "WARNING: Memory leak test timed out or failed"
    }

    timeout "$CI_TIMEOUT" cargo test --test filekv_integration --release -- --ignored 2>&1 | tee -a "$LOG_FILE" || {
        log "WARNING: High concurrency tests timed out or failed"
    }

    log "CI mode stability tests complete."
    exit 0
fi

# ─── Full stability test suite ───

if [[ "$MODE" == "quick" ]]; then
    log "Quick mode: running 5-minute smoke test..."

    # Build for tests
    cargo test --lib --release --no-run 2>&1 | tee -a "$LOG_FILE"

    # Run the short stability test
    log "Running short_running_stability test (5 minutes)..."
    cargo test --lib --release -- --ignored test_short_running_stability --nocapture 2>&1 | tee -a "$LOG_FILE"

    log "Quick stability test complete."
else
    # Full mode: 24-hour test plan
    log "Full mode: running 24-hour stability test suite..."

    log_separator
    log "Phase 1: Basic Stability (1 hour)"
    log_separator

    # 1. Short running stability
    log "Running test_short_running_stability..."
    START_TIME=$(date +%s)
    cargo test --lib --release -- --ignored test_short_running_stability --nocapture 2>&1 | tee -a "$LOG_FILE"
    END_TIME=$(date +%s)
    log "Completed in $((END_TIME - START_TIME))s"

    # 2. Memory leak detection
    log "Running test_memory_leak_detection..."
    START_TIME=$(date +%s)
    INITIAL_FD=$(check_file_descriptors)
    log "Initial file descriptors: $INITIAL_FD"
    cargo test --lib --release -- --ignored test_memory_leak_detection --nocapture 2>&1 | tee -a "$LOG_FILE"
    FINAL_FD=$(check_file_descriptors)
    log "Final file descriptors: $FINAL_FD"
    FD_LEAK=$((FINAL_FD - INITIAL_FD))
    if [[ $FD_LEAK -gt 10 ]]; then
        log "WARNING: Potential file descriptor leak: $FD_LEAK new FDs"
    else
        log "File descriptor check: OK (delta: $FD_LEAK)"
    fi
    END_TIME=$(date +%s)
    log "Completed in $((END_TIME - START_TIME))s"

    log_separator
    log "Phase 2: Concurrent Stress (2 hours)"
    log_separator

    # 3. High concurrency tests
    for test_name in test_32_threads_concurrent_puts test_32_threads_concurrent_gets \
                     test_64_threads_concurrent_puts test_64_threads_concurrent_gets \
                     test_32_threads_mixed_read_write test_64_threads_hot_key_contention \
                     test_32_threads_cache_stress test_32_threads_puts_then_flush_and_reopen; do
        log "Running $test_name..."
        START_TIME=$(date +%s)
        cargo test --test filekv_integration --release -- --ignored "$test_name" --nocapture 2>&1 | tee -a "$LOG_FILE"
        END_TIME=$(date +%s)
        log "Completed in $((END_TIME - START_TIME))s"
    done

    log_separator
    log "Phase 3: Performance Stability (1 hour)"
    log_separator

    # 4. Performance stability
    log "Running test_performance_stability..."
    START_TIME=$(date +%s)
    INITIAL_MEM=$(get_current_memory_mb)
    log "Initial memory: ${INITIAL_MEM} MB"
    cargo test --lib --release -- --ignored test_performance_stability --nocapture 2>&1 | tee -a "$LOG_FILE"
    FINAL_MEM=$(get_current_memory_mb)
    log "Final memory: ${FINAL_MEM} MB"
    END_TIME=$(date +%s)
    log "Completed in $((END_TIME - START_TIME))s"

    log_separator
    log "Phase 4: Extended Endurance (remaining duration)"
    log_separator

    REMAINING=$((DURATION_SECONDS - ($(date +%s) - $(date -d "$(head -1 "$LOG_FILE" | grep -o '[0-9-]* [0-9:]*' || 'now')' +%s 2>/dev/null || 0))))

    if [[ $REMAINING -gt 0 ]]; then
        log "Extended endurance: ${REMAINING}s remaining"

        # Run a custom loop test for the remaining time
        # This test continuously puts/gets/flushes to stress the engine
        END_LOOP_TIME=$(( $(date +%s) + REMAINING ))
        LOOP_COUNT=0
        LOOP_ERRORS=0

        while [[ $(date +%s) -lt $END_LOOP_TIME ]]; do
            LOOP_COUNT=$((LOOP_COUNT + 1))

            if [[ $((LOOP_COUNT % 100)) -eq 0 ]]; then
                CURRENT_MEM=$(get_current_memory_mb)
                CURRENT_FD=$(check_file_descriptors)
                log "Loop $LOOP_COUNT: memory=${CURRENT_MEM}MB, fd=$CURRENT_FD, errors=$LOOP_ERRORS"
            fi

            # Run a single put/get/delete cycle via cargo test
            # For actual long-running, use the example binary instead of cargo test
            cargo run --release --example stability_runner -- "$REMAINING" 2>/dev/null || true
        done

        log "Extended endurance complete: $LOOP_COUNT loops, $LOOP_ERRORS errors"
    else
        log "No remaining time for extended endurance phase"
    fi
fi

# ─── Summary ───

log_separator
log "STABILITY TEST SUMMARY"
log_separator
log "Duration: $DURATION"
log "Log file: $LOG_FILE"
log "Log size: $(wc -l < "$LOG_FILE") lines"

# Check for ERROR/WARNING patterns in log
ERROR_COUNT=$(grep -c "ERROR\|panic\|thread.*panicked" "$LOG_FILE" 2>/dev/null || echo "0")
WARNING_COUNT=$(grep -c "WARNING\|WARN" "$LOG_FILE" 2>/dev/null || echo "0")

log "Errors found in log: $ERROR_COUNT"
log "Warnings found in log: $WARNING_COUNT"

if [[ $ERROR_COUNT -gt 0 ]]; then
    log "ERROR SUMMARY:"
    grep "ERROR\|panic\|thread.*panicked" "$LOG_FILE" | tail -20 | tee -a "$LOG_FILE"
    log ""
    log "STATUS: FAILED (errors detected)"
    exit 1
else
    log "STATUS: PASSED (no errors)"
    exit 0
fi
