# Test groups for tokitai-context
# Usage: just test-unit, just test-integration, just test-all

# Default: run all tests with reasonable timeout
test-all:
    @echo "=== Running all tests (timeout: 600s) ==="
    cargo test --lib --timeout 600
    cargo test --test '*' --timeout 600

# Unit tests only (fast, no external dependencies)
test-unit:
    @echo "=== Running unit tests only ==="
    cargo test --lib --timeout 300

# Integration tests only (slower, may have external dependencies)
test-integration:
    @echo "=== Running integration tests only ==="
    cargo test --test '*' --timeout 600

# Specific test groups
test-parallel:
    @echo "=== Running parallel manager tests ==="
    cargo test --test parallel_manager_core_test --timeout 300
    cargo test --test parallel_context_test --timeout 300

test-kv:
    @echo "=== Running KV storage tests ==="
    cargo test --test file_kv_integration_test --timeout 300

test-merge:
    @echo "=== Running merge strategy tests ==="
    cargo test --test merge_strategies_test --timeout 300

test-crash:
    @echo "=== Running crash recovery tests ==="
    cargo test --test crash_recovery_test --timeout 600

# Quick tests (only critical path, no slow tests)
test-quick:
    @echo "=== Running quick tests (skip slow integration) ==="
    cargo test --lib --skip integration --timeout 180

# Coverage report (requires cargo-tarpaulin)
coverage:
    @echo "=== Generating coverage report ==="
    cargo tarpaulin --out Html --out Lcov --timeout 600

# Clean and rebuild
rebuild:
    cargo clean
    cargo build --tests

# =============================================================================
# Baseline Setup (for experiments)
# =============================================================================

# Setup all baseline systems
setup-baseline:
    @echo "=== Setting up all baseline systems ==="
    cd experiment/baseline && python setup_all.py

# Setup individual baseline systems
setup-baseline-langchain:
    @echo "=== Setting up LangChain baseline ==="
    cd experiment/baseline && python scripts/setup_langchain.py

setup-baseline-chromadb:
    @echo "=== Setting up ChromaDB baseline ==="
    cd experiment/baseline && python scripts/setup_chromadb.py

setup-baseline-sqlite:
    @echo "=== Setting up SQLite baseline ==="
    cd experiment/baseline && python scripts/setup_sqlite.py

setup-baseline-rocksdb:
    @echo "=== Setting up RocksDB baseline ==="
    cd experiment/baseline && python scripts/setup_rocksdb.py

# Verify baseline installation
verify-baseline:
    @echo "=== Verifying baseline installation ==="
    cd experiment/baseline && python verify_install.py

# Help
default:
    @echo "Available test commands:"
    @echo "  just test-unit        - Run unit tests only (fast)"
    @echo "  just test-integration - Run integration tests only (slow)"
    @echo "  just test-all         - Run all tests"
    @echo "  just test-parallel    - Run parallel manager tests"
    @echo "  just test-kv          - Run KV storage tests"
    @echo "  just test-merge       - Run merge strategy tests"
    @echo "  just test-crash       - Run crash recovery tests"
    @echo "  just test-quick       - Run quick tests (skip slow)"
    @echo "  just coverage         - Generate coverage report"
    @echo "  just rebuild          - Clean and rebuild tests"
    @echo ""
    @echo "Baseline setup commands:"
    @echo "  just setup-baseline         - Setup all baseline systems"
    @echo "  just setup-baseline-langchain  - Setup LangChain baseline"
    @echo "  just setup-baseline-chromadb   - Setup ChromaDB baseline"
    @echo "  just setup-baseline-sqlite     - Setup SQLite baseline"
    @echo "  just setup-baseline-rocksdb    - Setup RocksDB baseline"
    @echo "  just verify-baseline        - Verify baseline installation"
