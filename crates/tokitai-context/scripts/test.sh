#!/bin/bash
# Test runner script for tokitai-context
# Provides grouped test execution with timeout control

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Default timeout values (in seconds)
LIB_TIMEOUT=300
INTEGRATION_TIMEOUT=600
CRASH_TIMEOUT=600

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_header() {
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}$1${NC}"
    echo -e "${GREEN}========================================${NC}"
}

print_error() {
    echo -e "${RED}ERROR: $1${NC}"
}

print_info() {
    echo -e "${YELLOW}INFO: $1${NC}"
}

run_lib_tests() {
    print_header "Running Library Unit Tests (timeout: ${LIB_TIMEOUT}s)"
    cargo test --lib --timeout "${LIB_TIMEOUT}"
}

run_integration_tests() {
    print_header "Running Integration Tests (timeout: ${INTEGRATION_TIMEOUT}s)"
    cargo test --test '*' --timeout "${INTEGRATION_TIMEOUT}"
}

run_parallel_tests() {
    print_header "Running Parallel Manager Tests"
    cargo test --test parallel_manager_core_test --timeout "${INTEGRATION_TIMEOUT}"
    cargo test --test parallel_context_test --timeout "${INTEGRATION_TIMEOUT}"
}

run_kv_tests() {
    print_header "Running KV Storage Tests"
    cargo test --test file_kv_integration_test --timeout "${INTEGRATION_TIMEOUT}"
}

run_merge_tests() {
    print_header "Running Merge Strategy Tests"
    cargo test --test merge_strategies_test --timeout "${INTEGRATION_TIMEOUT}"
}

run_crash_tests() {
    print_header "Running Crash Recovery Tests (timeout: ${CRASH_TIMEOUT}s)"
    cargo test --test crash_recovery_test --timeout "${CRASH_TIMEOUT}"
}

run_quick_tests() {
    print_header "Running Quick Tests (skip slow integration tests)"
    cargo test --lib --timeout 180
}

run_all_tests() {
    print_header "Running All Tests"
    run_lib_tests
    run_integration_tests
}

show_help() {
    echo "Test Runner for tokitai-context"
    echo ""
    echo "Usage: ./scripts/test.sh [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  all          - Run all tests (default)"
    echo "  unit         - Run library unit tests only"
    echo "  integration  - Run integration tests only"
    echo "  parallel     - Run parallel manager tests"
    echo "  kv           - Run KV storage tests"
    echo "  merge        - Run merge strategy tests"
    echo "  crash        - Run crash recovery tests"
    echo "  quick        - Run quick tests (skip slow integration)"
    echo "  help         - Show this help message"
    echo ""
    echo "Examples:"
    echo "  ./scripts/test.sh           # Run all tests"
    echo "  ./scripts/test.sh unit      # Run unit tests only"
    echo "  ./scripts/test.sh parallel  # Run parallel tests only"
}

# Main entry point
case "${1:-all}" in
    all)
        run_all_tests
        ;;
    unit)
        run_lib_tests
        ;;
    integration)
        run_integration_tests
        ;;
    parallel)
        run_parallel_tests
        ;;
    kv)
        run_kv_tests
        ;;
    merge)
        run_merge_tests
        ;;
    crash)
        run_crash_tests
        ;;
    quick)
        run_quick_tests
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        print_error "Unknown command: $1"
        show_help
        exit 1
        ;;
esac

print_header "Tests Completed Successfully ✓"
