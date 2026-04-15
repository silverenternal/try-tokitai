#!/bin/bash
# =============================================================================
# Parallel Test Runner for tokitai-filekv
# =============================================================================
# This script runs tests in parallel by module to speed up CI/CD pipelines.
#
# Usage:
#   ./scripts/test.sh              # Run all tests in parallel
#   ./scripts/test.sh --nextest    # Use cargo-nextest (recommended, fastest)
#   ./scripts/test.sh --verbose    # Run with verbose output
#   ./scripts/test.sh --watch      # Run tests and watch for changes
#   ./scripts/test.sh --async      # Run tests with async-io feature enabled
#   ./scripts/test.sh --all-features # Run tests with all features enabled
#
# Requirements:
#   - bash 4.0+
#   - cargo (Rust toolchain)
#   - cargo-nextest (optional, for --nextest mode): cargo install cargo-nextest
# =============================================================================

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
JOBS=${JOBS:-4}  # Default parallelism, can be overridden with env var

cd "$PROJECT_DIR"

# =============================================================================
# Functions
# =============================================================================

print_header() {
    echo -e "${BLUE}============================================${NC}"
    echo -e "${BLUE}  $1${NC}"
    echo -e "${BLUE}============================================${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

# Run tests using cargo-nextest (recommended)
run_nextest() {
    print_header "Running tests with cargo-nextest (parallel)"
    
    if ! command -v cargo-nextest &> /dev/null; then
        print_error "cargo-nextest is not installed"
        echo "Install with: cargo install cargo-nextest"
        exit 1
    fi
    
    echo ""
    echo "Running $JOBS parallel jobs..."
    echo ""
    
    cargo nextest run --lib --test-threads "$JOBS" "$@"
}

# Run tests using cargo built-in parallel execution
run_cargo_parallel() {
    print_header "Running tests with cargo (parallel)"
    
    echo ""
    echo "Running $JOBS parallel jobs..."
    echo ""
    
    cargo test --lib --jobs "$JOBS" "$@"
}

# Run tests by module (fallback for older cargo versions)
run_module_parallel() {
    print_header "Running tests by module (parallel)"
    
    # Define test modules based on project structure
    MODULES=(
        "tests::checkpoint"
        "bloom"
        "cache"
        "compaction"
        "wal"
        "query"
        "index"
        "segment"
        "ops"
        "engine"
        "error"
        "config"
        "recovery"
        "gc"
        "stats"
        "tests::write_buffer"
        "tests::range_query"
        "tests::durability"
        "tests::crash_recovery"
        "tests::concurrent"
    )
    
    echo ""
    echo "Running ${#MODULES[@]} test modules in parallel (max $JOBS concurrent)..."
    echo ""
    
    local pids=()
    local failed=0
    
    for module in "${MODULES[@]}"; do
        # Limit parallelism to JOBS
        while [ ${#pids[@]} -ge $JOBS ]; do
            for i in "${!pids[@]}"; do
                if ! kill -0 "${pids[$i]}" 2>/dev/null; then
                    if wait "${pids[$i]}"; then
                        print_success "Module $module passed"
                    else
                        print_error "Module $module failed"
                        failed=$((failed + 1))
                    fi
                    unset 'pids[$i]'
                    pids=("${pids[@]}")
                    break
                fi
            done
            sleep 0.1
        done
        
        cargo test --lib "$module" 2>&1 &
        pids+=($!)
    done
    
    # Wait for remaining jobs
    for pid in "${pids[@]}"; do
        if ! wait "$pid"; then
            failed=$((failed + 1))
        fi
    done
    
    echo ""
    if [ $failed -eq 0 ]; then
        print_success "All test modules passed"
    else
        print_error "$failed test module(s) failed"
        exit 1
    fi
}

# Show help
show_help() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --nextest         Use cargo-nextest (recommended, fastest)"
    echo "  --cargo           Use cargo built-in parallel execution (default)"
    echo "  --module          Run tests by module (fallback)"
    echo "  --verbose         Run with verbose output"
    echo "  --watch           Watch mode (run on file changes)"
    echo "  --async           Run tests with async-io feature (includes tokio tests)"
    echo "  --all-features    Run tests with all features enabled"
    echo "  --jobs N          Set parallelism level (default: 4)"
    echo "  --help            Show this help message"
    echo ""
    echo "Environment variables:"
    echo "  JOBS              Set parallelism level (default: 4)"
    echo ""
    echo "Examples:"
    echo "  $0                        # Run all tests with cargo"
    echo "  $0 --nextest              # Run with cargo-nextest"
    echo "  JOBS=8 $0                 # Use 8 parallel jobs"
    echo "  $0 --verbose --nextest    # Verbose nextest run"
    echo "  $0 --async                # Run with async-io feature"
    echo "  $0 --all-features         # Run with all features"
}

# Watch mode
run_watch() {
    print_header "Watch mode - running tests on file changes"
    
    if ! command -v cargo-watch &> /dev/null; then
        print_error "cargo-watch is not installed"
        echo "Install with: cargo install cargo-watch"
        exit 1
    fi
    
    cargo watch -x "test --lib --jobs $JOBS"
}

# =============================================================================
# Main
# =============================================================================

# Parse arguments
MODE="cargo"
VERBOSE=false
WATCH=false
FEATURE_FLAG=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --nextest)
            MODE="nextest"
            shift
            ;;
        --cargo)
            MODE="cargo"
            shift
            ;;
        --module)
            MODE="module"
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --watch)
            WATCH=true
            shift
            ;;
        --async)
            FEATURE_FLAG="--features async-io"
            shift
            ;;
        --all-features)
            FEATURE_FLAG="--all-features"
            shift
            ;;
        --jobs)
            JOBS="$2"
            shift 2
            ;;
        --help|-h)
            show_help
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

# Add verbose flag if enabled
EXTRA_ARGS=""
if [ "$VERBOSE" = true ]; then
    EXTRA_ARGS="-- --show-output"
fi

# Run tests based on mode
if [ "$WATCH" = true ]; then
    run_watch
elif [ "$MODE" = "nextest" ]; then
    run_nextest $EXTRA_ARGS $FEATURE_FLAG
elif [ "$MODE" = "module" ]; then
    run_module_parallel
else
    run_cargo_parallel $EXTRA_ARGS $FEATURE_FLAG
fi

print_header "Tests completed"
