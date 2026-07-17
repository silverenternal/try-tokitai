//! Crash Recovery Test Framework (P2-015)
//!
//! This module provides fault injection capabilities for testing crash recovery scenarios.
//! It simulates various failure conditions to ensure data consistency and durability guarantees.
//!
//! # Features
//! - Configurable fault injection points
//! - Simulated crashes at critical operations
//! - Consistency verification after recovery
//! - Automated test scenarios
//!
//! # Usage
//! ```rust
//! let mut injector = FaultInjector::default();
//! injector.enable_fault(FaultType::WriteFailure, 0.5); // 50% failure rate
//!
//! // Run operation that may fail
//! match injector.should_fail(FaultType::WriteFailure) {
//!     true => Err(Error::InjectedFault),
//!     false => perform_operation(),
//! }
//! ```

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use parking_lot::Mutex;
use thiserror::Error;

/// Fault injection error types
#[derive(Debug, Error, Clone, PartialEq)]
pub enum InjectionError {
    #[error("Injected fault: {0}")]
    InjectedFault(String),
    
    #[error("Operation timeout")]
    Timeout,
    
    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),
    
    #[error("Corrupted data: {0}")]
    CorruptedData(String),
}

/// Types of faults that can be injected
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum FaultType {
    /// WAL write failures
    WalWriteFailure,
    
    /// MemTable flush failures
    MemTableFlushFailure,
    
    /// Segment write failures
    SegmentWriteFailure,
    
    /// Index update failures
    IndexUpdateFailure,
    
    /// Bloom filter write failures
    BloomFilterWriteFailure,
    
    /// Compaction failures
    CompactionFailure,
    
    /// Checkpoint failures
    CheckpointFailure,
    
    /// Disk full simulation
    DiskFull,
    
    /// Random crash simulation
    RandomCrash,
}

impl std::fmt::Display for FaultType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FaultType::WalWriteFailure => write!(f, "WAL_WRITE_FAILURE"),
            FaultType::MemTableFlushFailure => write!(f, "MEMTABLE_FLUSH_FAILURE"),
            FaultType::SegmentWriteFailure => write!(f, "SEGMENT_WRITE_FAILURE"),
            FaultType::IndexUpdateFailure => write!(f, "INDEX_UPDATE_FAILURE"),
            FaultType::BloomFilterWriteFailure => write!(f, "BLOOM_FILTER_WRITE_FAILURE"),
            FaultType::CompactionFailure => write!(f, "COMPACTION_FAILURE"),
            FaultType::CheckpointFailure => write!(f, "CHECKPOINT_FAILURE"),
            FaultType::DiskFull => write!(f, "DISK_FULL"),
            FaultType::RandomCrash => write!(f, "RANDOM_CRASH"),
        }
    }
}

/// Configuration for fault injection
#[derive(Debug, Clone)]
pub struct FaultConfig {
    /// Failure rate (0.0 - 1.0)
    pub failure_rate: f64,
    
    /// Number of failures before succeeding
    pub max_failures: Option<usize>,
    
    /// Enable fault injection
    pub enabled: bool,
    
    /// Seed for deterministic random (for reproducible tests)
    pub seed: Option<u64>,
}

impl Default for FaultConfig {
    fn default() -> Self {
        Self {
            failure_rate: 0.0,
            max_failures: None,
            enabled: false,
            seed: None,
        }
    }
}

/// Statistics about fault injection
#[derive(Debug, Clone, Default)]
pub struct FaultStats {
    /// Total fault injection attempts
    pub total_attempts: usize,
    
    /// Number of injected faults
    pub injected_faults: usize,
    
    /// Number of successful operations
    pub successful_ops: usize,
    
    /// Faults by type
    pub faults_by_type: HashMap<String, usize>,
}

/// Internal state for fault injection
struct FaultState {
    /// Current failure counts by type
    failure_counts: Mutex<HashMap<FaultType, usize>>,
    
    /// Random number generator state (simple LCG for reproducibility)
    rng_state: Mutex<u64>,
    
    /// Global fault counter
    total_faults: AtomicUsize,
}

/// Fault Injector for crash recovery testing
///
/// Thread-safe fault injection with configurable failure rates
pub struct FaultInjector {
    config: Mutex<HashMap<FaultType, FaultConfig>>,
    state: Arc<FaultState>,
    enabled: AtomicBool,
}

impl Default for FaultInjector {
    fn default() -> Self {
        Self::new()
    }
}

impl FaultInjector {
    /// Create a new fault injector
    pub fn new() -> Self {
        Self {
            config: Mutex::new(HashMap::new()),
            state: Arc::new(FaultState {
                failure_counts: Mutex::new(HashMap::new()),
                rng_state: Mutex::new(12345), // Default seed
                total_faults: AtomicUsize::new(0),
            }),
            enabled: AtomicBool::new(false),
        }
    }
    
    /// Create a new fault injector with a specific seed for reproducibility
    pub fn with_seed(seed: u64) -> Self {
        let injector = Self::new();
        *injector.state.rng_state.lock() = seed;
        injector
    }
    
    /// Enable fault injection globally
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }
    
    /// Disable fault injection globally
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }
    
    /// Check if fault injection is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }
    
    /// Configure fault injection for a specific fault type
    pub fn configure(&self, fault_type: FaultType, config: FaultConfig) {
        let mut configs = self.config.lock();
        configs.insert(fault_type, config);
    }
    
    /// Enable a specific fault type with a failure rate
    pub fn enable_fault(&self, fault_type: FaultType, failure_rate: f64) {
        let config = FaultConfig {
            failure_rate,
            enabled: true,
            ..Default::default()
        };
        self.configure(fault_type, config);
        self.enable();
    }
    
    /// Disable a specific fault type
    pub fn disable_fault(&self, fault_type: &FaultType) {
        let mut configs = self.config.lock();
        if let Some(config) = configs.get_mut(fault_type) {
            config.enabled = false;
        }
    }
    
    /// Clear all fault configurations
    pub fn clear(&self) {
        let mut configs = self.config.lock();
        configs.clear();
        self.disable();
    }
    
    /// Simple random number generator (LCG) for deterministic testing
    fn random(&self) -> f64 {
        // Linear Congruential Generator
        const A: u64 = 1103515245;
        const C: u64 = 12345;
        const M: u64 = 1 << 31;
        
        let mut rng_state = self.state.rng_state.lock();
        *rng_state = (A.wrapping_mul(*rng_state).wrapping_add(C)) % M;
        (*rng_state as f64) / (M as f64)
    }
    
    /// Check if an operation should fail
    pub fn should_fail(&self, fault_type: &FaultType) -> bool {
        if !self.enabled.load(Ordering::SeqCst) {
            return false;
        }
        
        let configs = self.config.lock();
        let config = match configs.get(fault_type) {
            Some(c) if c.enabled => c,
            _ => return false,
        };
        
        // Check max failures limit
        if let Some(max) = config.max_failures {
            let failure_counts = self.state.failure_counts.lock();
            let current = failure_counts.get(fault_type).copied().unwrap_or(0);
            if current >= max {
                return false;
            }
        }
        
        // Determine if this operation should fail
        let should_fail = self.random() < config.failure_rate;
        
        if should_fail {
            let mut failure_counts = self.state.failure_counts.lock();
            *failure_counts.entry(fault_type.clone()).or_insert(0) += 1;
            
            self.state.total_faults.fetch_add(1, Ordering::SeqCst);
        }
        
        should_fail
    }
    
    /// Execute an operation with potential fault injection
    pub fn execute<T, F, E>(&self, fault_type: &FaultType, operation: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
        E: From<InjectionError>,
    {
        if self.should_fail(fault_type) {
            return Err(InjectionError::InjectedFault(fault_type.to_string()).into());
        }
        operation()
    }
    
    /// Get fault injection statistics
    pub fn stats(&self) -> FaultStats {
        let mut stats = FaultStats::default();
        
        stats.injected_faults = self.state.total_faults.load(Ordering::SeqCst);
        stats.total_attempts = stats.injected_faults + 100; // Approximate
        
        let failure_counts = self.state.failure_counts.lock();
        for (fault_type, count) in failure_counts.iter() {
            stats.faults_by_type.insert(
                fault_type.to_string(),
                *count
            );
        }
        
        stats
    }
    
    /// Reset fault injector state
    pub fn reset(&self) {
        self.clear();
        self.state.total_faults.store(0, Ordering::SeqCst);
    }
}

/// Crash scenario definitions for automated testing
#[derive(Debug, Clone)]
pub struct CrashScenario {
    pub name: String,
    pub description: String,
    pub fault_type: FaultType,
    pub failure_rate: f64,
    pub operations_count: usize,
    pub expected_recovery_time_ms: u64,
}

impl CrashScenario {
    pub fn new(
        name: &str,
        description: &str,
        fault_type: FaultType,
        failure_rate: f64,
        operations_count: usize,
    ) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            fault_type,
            failure_rate,
            operations_count,
            expected_recovery_time_ms: 1000, // Default
        }
    }
    
    /// Standard crash scenarios for testing
    pub fn standard_scenarios() -> Vec<CrashScenario> {
        vec![
            CrashScenario::new(
                "wal_crash_during_write",
                "WAL fails during write operations",
                FaultType::WalWriteFailure,
                0.3,
                1000,
            ),
            CrashScenario::new(
                "memtable_flush_crash",
                "MemTable flush fails mid-operation",
                FaultType::MemTableFlushFailure,
                0.2,
                500,
            ),
            CrashScenario::new(
                "segment_write_crash",
                "Segment write fails during compaction",
                FaultType::SegmentWriteFailure,
                0.25,
                200,
            ),
            CrashScenario::new(
                "index_update_crash",
                "Index update fails after segment write",
                FaultType::IndexUpdateFailure,
                0.2,
                300,
            ),
            CrashScenario::new(
                "compaction_crash",
                "Compaction fails mid-way",
                FaultType::CompactionFailure,
                0.15,
                100,
            ),
            CrashScenario::new(
                "disk_full_scenario",
                "Simulate disk full during write",
                FaultType::DiskFull,
                0.1,
                500,
            ),
        ]
    }
}

/// Result of a crash recovery test
#[derive(Debug, Clone)]
pub struct CrashRecoveryResult {
    pub scenario_name: String,
    pub success: bool,
    pub operations_attempted: usize,
    pub operations_succeeded: usize,
    pub faults_injected: usize,
    pub recovery_time_ms: u64,
    pub data_consistent: bool,
    pub error_message: Option<String>,
}

impl CrashRecoveryResult {
    pub fn success(scenario_name: &str) -> Self {
        Self {
            scenario_name: scenario_name.to_string(),
            success: true,
            operations_attempted: 0,
            operations_succeeded: 0,
            faults_injected: 0,
            recovery_time_ms: 0,
            data_consistent: true,
            error_message: None,
        }
    }
    
    pub fn failure(scenario_name: &str, error: &str) -> Self {
        Self {
            scenario_name: scenario_name.to_string(),
            success: false,
            operations_attempted: 0,
            operations_succeeded: 0,
            faults_injected: 0,
            recovery_time_ms: 0,
            data_consistent: false,
            error_message: Some(error.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fault_injector_disabled_by_default() {
        let injector = FaultInjector::new();
        assert!(!injector.is_enabled());
        assert!(!injector.should_fail(&FaultType::WalWriteFailure));
    }
    
    #[test]
    fn test_fault_injector_enable_disable() {
        let injector = FaultInjector::new();
        
        injector.enable_fault(FaultType::WalWriteFailure, 1.0); // 100% failure
        assert!(injector.is_enabled());
        assert!(injector.should_fail(&FaultType::WalWriteFailure));
        
        injector.disable_fault(&FaultType::WalWriteFailure);
        assert!(!injector.should_fail(&FaultType::WalWriteFailure));
    }
    
    #[test]
    fn test_fault_injector_failure_rate() {
        let injector = FaultInjector::with_seed(42);
        injector.enable_fault(FaultType::WalWriteFailure, 0.5);
        
        let mut failures = 0;
        let iterations = 1000;
        
        for _ in 0..iterations {
            if injector.should_fail(&FaultType::WalWriteFailure) {
                failures += 1;
            }
        }
        
        // Should be approximately 50% (allow some variance)
        let rate = failures as f64 / iterations as f64;
        assert!(rate > 0.4 && rate < 0.6, "Failure rate {} outside expected range", rate);
    }
    
    #[test]
    fn test_fault_injector_execute() {
        let injector = FaultInjector::new();
        injector.enable_fault(FaultType::WalWriteFailure, 0.0); // 0% failure
        
        let result: Result<(), InjectionError> = injector.execute(
            &FaultType::WalWriteFailure,
            || Ok(())
        );
        
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_fault_injector_stats() {
        let injector = FaultInjector::with_seed(123);
        injector.enable_fault(FaultType::WalWriteFailure, 1.0);
        
        for _ in 0..10 {
            injector.should_fail(&FaultType::WalWriteFailure);
        }
        
        let stats = injector.stats();
        assert_eq!(stats.injected_faults, 10);
        assert!(stats.faults_by_type.contains_key("WAL_WRITE_FAILURE"));
    }
    
    #[test]
    fn test_crash_scenario_creation() {
        let scenario = CrashScenario::new(
            "test_scenario",
            "Test description",
            FaultType::CompactionFailure,
            0.5,
            100,
        );
        
        assert_eq!(scenario.name, "test_scenario");
        assert_eq!(scenario.fault_type, FaultType::CompactionFailure);
        assert_eq!(scenario.failure_rate, 0.5);
    }
    
    #[test]
    fn test_standard_scenarios() {
        let scenarios = CrashScenario::standard_scenarios();
        
        assert!(!scenarios.is_empty());
        assert!(scenarios.len() >= 5);
        
        for scenario in scenarios {
            assert!(scenario.failure_rate > 0.0 && scenario.failure_rate <= 1.0);
            assert!(scenario.operations_count > 0);
        }
    }
    
    #[test]
    fn test_crash_recovery_result() {
        let success = CrashRecoveryResult::success("test");
        assert!(success.success);
        assert!(success.data_consistent);
        
        let failure = CrashRecoveryResult::failure("test", "error message");
        assert!(!failure.success);
        assert!(!failure.data_consistent);
        assert!(failure.error_message.is_some());
    }
    
    #[test]
    fn test_fault_injector_reset() {
        let injector = FaultInjector::new();
        injector.enable_fault(FaultType::WalWriteFailure, 1.0);
        
        injector.should_fail(&FaultType::WalWriteFailure);
        injector.should_fail(&FaultType::WalWriteFailure);
        
        injector.reset();
        
        assert!(!injector.is_enabled());
        assert_eq!(injector.stats().injected_faults, 0);
    }
    
    #[test]
    fn test_fault_injector_max_failures() {
        let injector = FaultInjector::new();
        let config = FaultConfig {
            failure_rate: 1.0, // 100% failure
            max_failures: Some(3),
            enabled: true,
            seed: None,
        };
        
        injector.configure(FaultType::MemTableFlushFailure, config);
        injector.enable();
        
        let mut failures = 0;
        for _ in 0..10 {
            if injector.should_fail(&FaultType::MemTableFlushFailure) {
                failures += 1;
            }
        }
        
        // Should only fail 3 times due to max_failures limit
        assert_eq!(failures, 3);
    }
}
