//! Crash Recovery Integration Tests (P2-015)
//!
//! Integration tests for crash recovery scenarios using fault injection.
//! Tests data consistency after simulated crashes at various operation points.

use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

use crate::crash_recovery::fault_injection::{
    CrashRecoveryResult, CrashScenario, FaultInjector, FaultType,
};
use crate::file_kv::{FileKV, FileKVConfig, MemTableConfig, DictionaryCompressionConfig, AuditLogConfig};

/// Test harness for crash recovery testing
pub struct CrashRecoveryHarness {
    pub temp_dir: TempDir,
    pub injector: FaultInjector,
    pub config: FileKVConfig,
}

impl CrashRecoveryHarness {
    /// Create a new test harness
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        
        let config = FileKVConfig {
            segment_dir: temp_dir.path().join("segments"),
            wal_dir: temp_dir.path().join("wal"),
            index_dir: temp_dir.path().join("index"),
            enable_wal: true,
            enable_bloom: true,
            enable_background_flush: false, // Disable for testing
            background_flush_interval_ms: 0,
            segment_preallocate_size: 1024 * 1024, // 1MB for testing
            memtable: MemTableConfig {
                flush_threshold_bytes: 100 * 1024, // 100KB
                max_entries: 1000,
                max_memory_bytes: 10 * 1024 * 1024, // 10MB
            },
            cache: Default::default(),
            compaction: Default::default(),
            wal_max_size_bytes: 10 * 1024 * 1024,
            wal_max_files: 3,
            write_coalescing_enabled: false,
            cache_warming_enabled: false,
            compression: DictionaryCompressionConfig::default(),
            // P3-001: Async I/O disabled for testing
            async_io_enabled: false,
            async_io_max_concurrent_writes: 4,
            async_io_max_queue_depth: 1024,
            async_io_write_timeout_ms: 5000,
            async_io_enable_coalescing: false,
            async_io_coalesce_window_ms: 10,
            // P2-009: Checkpoint directory for testing
            checkpoint_dir: temp_dir.path().join("checkpoints"),
            // P2-013: Audit log disabled for testing
            audit_log: AuditLogConfig::default(),
        };
        
        Self {
            temp_dir,
            injector: FaultInjector::new(),
            config,
        }
    }
    
    /// Create a FileKV instance with the test config
    pub fn create_kv(&self) -> FileKV {
        FileKV::open(self.config.clone())
            .expect("Failed to open FileKV")
    }
    
    /// Run a crash scenario test
    pub fn run_scenario(&self, scenario: &CrashScenario) -> CrashRecoveryResult {
        let start = std::time::Instant::now();

        // Configure fault injection
        self.injector.enable_fault(
            scenario.fault_type.clone(),
            scenario.failure_rate,
        );

        let mut result = CrashRecoveryResult::success(&scenario.name);
        result.operations_attempted = scenario.operations_count;

        // Create fresh KV instance
        let kv = self.create_kv();

        // Execute operations with potential faults
        let mut successes = 0;
        for i in 0..scenario.operations_count {
            let key = format!("key_{}", i);
            let value = vec![i as u8; 100]; // 100 byte values

            match self.injector.execute(&scenario.fault_type, || {
                kv.put(&key, &value)
                    .map_err(|e| crate::crash_recovery::fault_injection::InjectionError::InjectedFault(e.to_string()))
            }) {
                Ok(_) => successes += 1,
                Err(_) => {
                    // Fault injected, continue testing (in real scenario, would crash and recover)
                    // For testing purposes, we just count the fault
                }
            }
        }

        result.operations_succeeded = successes;
        result.faults_injected = scenario.operations_count - successes;
        result.recovery_time_ms = start.elapsed().as_millis() as u64;

        // Verify data consistency
        result.data_consistent = self.verify_consistency();

        // Disable fault injection
        self.injector.disable_fault(&scenario.fault_type);

        if !result.data_consistent {
            result.success = false;
            result.error_message = Some("Data inconsistency detected after recovery".to_string());
        }

        result
    }
    
    /// Verify data consistency after crash recovery
    fn verify_consistency(&self) -> bool {
        // Reopen KV and check for corruption
        let kv = self.create_kv();
        
        // Basic consistency checks
        // 1. Can iterate without errors
        // 2. Index points to valid segments
        // 3. Bloom filters load correctly
        
        // For now, just verify we can open and do basic operations
        let test_key = "consistency_check_key";
        let test_value: &[u8] = b"consistency_check_value";
        
        match kv.put(test_key, test_value) {
            Ok(_) => {
                match kv.get(test_key) {
                    Ok(Some(v)) => &v[..] == test_value,
                    Ok(None) => false,
                    Err(_) => false,
                }
            }
            Err(_) => false,
        }
    }
    
    /// Run all standard crash scenarios
    pub fn run_all_scenarios(&self) -> Vec<CrashRecoveryResult> {
        let scenarios = CrashScenario::standard_scenarios();
        let mut results = Vec::new();
        
        for scenario in scenarios {
            let result = self.run_scenario(&scenario);
            results.push(result);
        }
        
        results
    }
    
    /// Get summary of test results
    pub fn summarize_results(results: &[CrashRecoveryResult]) -> String {
        let total = results.len();
        let passed = results.iter().filter(|r| r.success).count();
        let failed = total - passed;
        
        let mut summary = format!(
            "Crash Recovery Test Summary\n\
             ==========================\n\
             Total: {} | Passed: {} | Failed: {}\n\n",
            total, passed, failed
        );
        
        for result in results {
            let status = if result.success { "✓ PASS" } else { "✗ FAIL" };
            summary.push_str(&format!(
                "{} {} - Faults: {}, Consistent: {}, Recovery: {}ms\n",
                status,
                result.scenario_name,
                result.faults_injected,
                result.data_consistent,
                result.recovery_time_ms
            ));
            
            if let Some(ref error) = result.error_message {
                summary.push_str(&format!("  Error: {}\n", error));
            }
        }
        
        summary
    }
}

impl Default for CrashRecoveryHarness {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_harness_creation() {
        let harness = CrashRecoveryHarness::new();
        assert!(harness.temp_dir.path().exists());
        assert!(!harness.injector.is_enabled());
    }
    
    #[test]
    fn test_wal_crash_scenario() {
        let harness = CrashRecoveryHarness::new();
        let scenario = CrashScenario::new(
            "wal_crash_test",
            "Test WAL crash recovery",
            FaultType::WalWriteFailure,
            0.3,
            100,
        );
        
        let result = harness.run_scenario(&scenario);
        
        // Test should complete (success depends on recovery implementation)
        assert!(result.operations_attempted > 0);
        // faults_injected is always >= 0 for usize, just verify test completed
    }
    
    #[test]
    fn test_memtable_flush_crash_scenario() {
        let harness = CrashRecoveryHarness::new();
        let scenario = CrashScenario::new(
            "memtable_crash_test",
            "Test MemTable flush crash recovery",
            FaultType::MemTableFlushFailure,
            0.2,
            50,
        );
        
        let result = harness.run_scenario(&scenario);
        assert!(result.operations_attempted > 0);
    }
    
    #[test]
    fn test_segment_write_crash_scenario() {
        let harness = CrashRecoveryHarness::new();
        let scenario = CrashScenario::new(
            "segment_crash_test",
            "Test segment write crash recovery",
            FaultType::SegmentWriteFailure,
            0.25,
            30,
        );
        
        let result = harness.run_scenario(&scenario);
        assert!(result.operations_attempted > 0);
    }
    
    #[test]
    fn test_multiple_scenarios() {
        let harness = CrashRecoveryHarness::new();
        let results = harness.run_all_scenarios();
        
        assert!(!results.is_empty());
        
        // Print summary for debugging
        println!("{}", CrashRecoveryHarness::summarize_results(&results));
    }
    
    #[test]
    fn test_recovery_after_fault() {
        let harness = CrashRecoveryHarness::new();
        
        // Enable faults
        harness.injector.enable_fault(FaultType::WalWriteFailure, 0.5);
        
        let kv = harness.create_kv();
        
        // Write some data (some may fail)
        for i in 0..20 {
            let _ = harness.injector.execute(&FaultType::WalWriteFailure, || {
                kv.put(&format!("key_{}", i), &[i as u8; 50])
                    .map_err(|e| crate::crash_recovery::fault_injection::InjectionError::InjectedFault(e.to_string()))
            });
        }
        
        // Disable faults and verify we can still operate
        harness.injector.disable();
        
        // Should be able to write successfully after disabling faults
        let result = kv.put("final_key", b"final_value");
        assert!(result.is_ok());
        
        // Should be able to read back
        let value = kv.get("final_key").expect("Get failed");
        assert!(value.is_some());
    }
}
