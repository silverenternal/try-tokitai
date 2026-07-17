//! Crash Recovery Test Framework (P2-015)
//!
//! This module provides comprehensive fault injection and crash recovery testing
//! capabilities to ensure data consistency and durability guarantees under failure conditions.
//!
//! # Components
//!
//! - **Fault Injection**: Configurable fault injection at various operation points
//! - **Crash Scenarios**: Pre-defined crash scenarios for common failure modes
//! - **Integration Tests**: Automated testing of recovery procedures
//!
//! # Usage
//!
//! ```rust,no_run
//! use tokitai_context::crash_recovery::{
//!     FaultInjector, FaultType, CrashScenario, CrashRecoveryHarness,
//! };
//!
//! // Create fault injector
//! let injector = FaultInjector::new();
//! injector.enable_fault(FaultType::WalWriteFailure, 0.3); // 30% failure rate
//!
//! // Run crash scenario tests
//! let harness = CrashRecoveryHarness::new();
//! let results = harness.run_all_scenarios();
//!
//! // Check results
//! for result in results {
//!     println!("{}: {}", result.scenario_name, if result.success { "PASS" } else { "FAIL" });
//! }
//! ```
//!
//! # Fault Types
//!
//! - `WalWriteFailure`: WAL write operation failures
//! - `MemTableFlushFailure`: MemTable flush failures
//! - `SegmentWriteFailure`: Segment file write failures
//! - `IndexUpdateFailure`: Index update failures
//! - `BloomFilterWriteFailure`: Bloom filter write failures
//! - `CompactionFailure`: Compaction operation failures
//! - `CheckpointFailure`: Checkpoint operation failures
//! - `DiskFull`: Disk full simulation
//! - `RandomCrash`: Random crash simulation
//!
//! # Example: Testing WAL Crash Recovery
//!
//! ```rust,no_run
//! let harness = CrashRecoveryHarness::new();
//!
//! let scenario = CrashScenario::new(
//!     "wal_crash_during_write",
//!     "WAL fails during write operations",
//!     FaultType::WalWriteFailure,
//!     0.3, // 30% failure rate
//!     1000, // 1000 operations
//! );
//!
//! let result = harness.run_scenario(&scenario);
//!
//! assert!(result.data_consistent, "Data should be consistent after recovery");
//! ```

pub mod fault_injection;
pub mod integration_tests;

pub use fault_injection::{
    CrashRecoveryResult,
    CrashScenario,
    FaultConfig,
    FaultInjector,
    FaultStats,
    FaultType,
    InjectionError,
};

pub use integration_tests::CrashRecoveryHarness;
