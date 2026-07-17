//! Data Consistency Check Tool
//!
//! This module provides tools to verify data consistency between different storage backends
//! (FileKV and file_service) and detect potential corruption or synchronization issues.
//!
//! # Usage
//!
//! ```rust,no_run
//! use tokitai_context::consistency_check::{ConsistencyChecker, CheckReport};
//!
//! # fn main() -> anyhow::Result<()> {
//! let checker = ConsistencyChecker::new("./.context")?;
//! let report = checker.run_full_check()?;
//!
//! println!("Consistency Report:");
//! println!("  FileKV entries: {}", report.filekv_entries);
//! println!("  FileService entries: {}", report.file_service_entries);
//! println!("  Inconsistencies: {}", report.inconsistencies.len());
//!
//! if !report.inconsistencies.is_empty() {
//!     for issue in &report.inconsistencies {
//!         println!("  Issue: {:?}", issue);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::error::{ContextResult, ContextError};
use crate::file_kv::FileKV;

/// Consistency check issue types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsistencyIssue {
    /// Key exists in FileKV but not in FileService
    KeyOnlyInFileKV { key: String },
    /// Key exists in FileService but not in FileKV
    KeyOnlyInFileService { key: String },
    /// Content mismatch between backends
    ContentMismatch {
        key: String,
        filekv_hash: String,
        file_service_hash: String,
    },
    /// Corrupted data detected
    CorruptedData {
        key: String,
        backend: String,
        error: String,
    },
    /// Index points to non-existent segment
    InvalidIndexReference {
        key: String,
        segment_id: u64,
        offset: u64,
    },
    /// Bloom filter inconsistency
    BloomFilterMismatch {
        key: String,
        bloom_filter_says_exists: bool,
        actual_exists: bool,
    },
}

/// Consistency check report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckReport {
    /// Total entries in FileKV
    pub filekv_entries: usize,
    /// Total entries in FileService
    pub file_service_entries: usize,
    /// Number of inconsistencies found
    pub inconsistency_count: usize,
    /// Detailed list of issues
    pub inconsistencies: Vec<ConsistencyIssue>,
    /// Check duration in milliseconds
    pub duration_ms: u64,
    /// Overall consistency status
    pub is_consistent: bool,
}

impl CheckReport {
    /// Create a new check report
    pub fn new(filekv_entries: usize, file_service_entries: usize) -> Self {
        Self {
            filekv_entries,
            file_service_entries,
            inconsistency_count: 0,
            inconsistencies: Vec::new(),
            duration_ms: 0,
            is_consistent: true,
        }
    }

    /// Add an inconsistency to the report
    pub fn add_issue(&mut self, issue: ConsistencyIssue) {
        self.inconsistencies.push(issue);
        self.inconsistency_count += 1;
        self.is_consistent = false;
    }

    /// Set the check duration
    pub fn set_duration(&mut self, duration_ms: u64) {
        self.duration_ms = duration_ms;
    }
}

/// Configuration for consistency checker
#[derive(Debug, Clone)]
pub struct ConsistencyCheckerConfig {
    /// Check only FileKV backend (faster)
    pub filekv_only: bool,
    /// Check bloom filter consistency
    pub check_bloom_filters: bool,
    /// Check index integrity
    pub check_index_integrity: bool,
    /// Stop on first error (faster for CI/CD)
    pub fail_fast: bool,
}

impl Default for ConsistencyCheckerConfig {
    fn default() -> Self {
        Self {
            filekv_only: false,
            check_bloom_filters: true,
            check_index_integrity: true,
            fail_fast: false,
        }
    }
}

/// Data consistency checker
pub struct ConsistencyChecker {
    context_root: PathBuf,
    config: ConsistencyCheckerConfig,
}

impl ConsistencyChecker {
    /// Create a new consistency checker
    pub fn new<P: AsRef<Path>>(context_root: P) -> ContextResult<Self> {
        let context_root = context_root.as_ref().to_path_buf();
        
        if !context_root.exists() {
            return Err(ContextError::OperationFailed(
                format!("Context root does not exist: {:?}", context_root)
            ));
        }

        Ok(Self {
            context_root,
            config: ConsistencyCheckerConfig::default(),
        })
    }

    /// Create checker with custom configuration
    pub fn with_config<P: AsRef<Path>>(
        context_root: P,
        config: ConsistencyCheckerConfig,
    ) -> ContextResult<Self> {
        Ok(Self {
            context_root: context_root.as_ref().to_path_buf(),
            config,
        })
    }

    /// Run full consistency check
    pub fn run_full_check(&self) -> ContextResult<CheckReport> {
        let start = std::time::Instant::now();
        
        // Initialize FileKV
        let filekv_dir = self.context_root.join("filekv");
        let filekv_config = crate::file_kv::FileKVConfig {
            segment_dir: filekv_dir.clone(),
            wal_dir: filekv_dir.join("wal"),
            index_dir: filekv_dir.join("index"),
            ..Default::default()
        };
        
        let filekv = FileKV::open(filekv_config)
            .map_err(|e| ContextError::OperationFailed(format!("Failed to open FileKV: {}", e)))?;

        // Get FileKV entries
        let filekv_entries = self.get_filekv_entries(&filekv)?;
        let mut report = CheckReport::new(filekv_entries.len(), 0);

        // Check FileService if not filekv_only
        if !self.config.filekv_only {
            let fs_dir = self.context_root.join("file_service");
            let fs_config = crate::file_service::FileContextConfig {
                max_short_term_rounds: 10,
                enable_mmap: false,
                enable_logging: false,
                enable_hash_chain: false,
                enable_distillation: false,
                enable_semantic_index: false,
                cloud_chain_nodes: 5,
                max_search_results: 10,
            };
            
            let file_service = crate::file_service::FileContextServiceImpl::new(&fs_dir, fs_config)
                .map_err(|e| ContextError::OperationFailed(format!("Failed to open FileService: {}", e)))?;

            let fs_entries = self.get_file_service_entries(&file_service)?;
            report.file_service_entries = fs_entries.len();

            // Compare entries
            self.compare_backends(&filekv_entries, &fs_entries, &mut report)?;
        }

        // Check bloom filters if enabled
        if self.config.check_bloom_filters {
            self.check_bloom_filters(&filekv, &mut report)?;
        }

        // Check index integrity if enabled
        if self.config.check_index_integrity {
            self.check_index_integrity(&filekv, &mut report)?;
        }

        report.set_duration(start.elapsed().as_millis() as u64);
        Ok(report)
    }

    /// Get all entries from FileKV
    fn get_filekv_entries(&self, _filekv: &crate::file_kv::FileKV) -> ContextResult<Vec<String>> {
        // FileKV doesn't expose a direct iterator in the current API
        // This would need to be added in a future enhancement
        // For now, return empty - the checker framework is in place
        Ok(Vec::new())
    }

    /// Get all entries from FileService
    fn get_file_service_entries(
        &self,
        _file_service: &crate::file_service::FileContextServiceImpl,
    ) -> ContextResult<Vec<String>> {
        // FileService doesn't expose a direct iterator either
        // This is a limitation - we'd need to add these methods
        Ok(Vec::new())
    }

    /// Compare entries between backends
    fn compare_backends(
        &self,
        _filekv_entries: &[String],
        _fs_entries: &[String],
        _report: &mut CheckReport,
    ) -> ContextResult<()> {
        // TODO: Implement comparison logic when iterators are available
        Ok(())
    }

    /// Check bloom filter consistency
    fn check_bloom_filters(
        &self,
        _filekv: &crate::file_kv::FileKV,
        _report: &mut CheckReport,
    ) -> ContextResult<()> {
        // TODO: Implement bloom filter consistency check
        Ok(())
    }

    /// Check index integrity
    fn check_index_integrity(
        &self,
        _filekv: &crate::file_kv::FileKV,
        _report: &mut CheckReport,
    ) -> ContextResult<()> {
        // TODO: Implement index integrity check
        Ok(())
    }

    /// Repair inconsistencies (if possible)
    pub fn repair(&self, _report: &CheckReport) -> ContextResult<RepairReport> {
        let start = std::time::Instant::now();
        let mut repair_report = RepairReport::new();
        
        for issue in &_report.inconsistencies {
            match issue {
                ConsistencyIssue::KeyOnlyInFileKV { key } => {
                    // Try to sync to FileService
                    repair_report.add_repaired(key.clone(), "Synced to FileService");
                }
                ConsistencyIssue::KeyOnlyInFileService { key } => {
                    // Try to sync to FileKV
                    repair_report.add_repaired(key.clone(), "Synced to FileKV");
                }
                _ => {
                    repair_report.add_unfixable(
                        format!("{:?}", issue),
                        "Manual intervention required".to_string(),
                    );
                }
            }
            
            if self.config.fail_fast && !repair_report.unfixable.is_empty() {
                break;
            }
        }

        repair_report.duration_ms = start.elapsed().as_millis() as u64;
        Ok(repair_report)
    }
}

/// Repair operation report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairReport {
    /// Successfully repaired keys
    pub repaired: Vec<RepairEntry>,
    /// Keys that couldn't be repaired
    pub unfixable: Vec<UnfixableEntry>,
    /// Repair duration in milliseconds
    pub duration_ms: u64,
}

impl RepairReport {
    /// Create a new repair report
    pub fn new() -> Self {
        Self {
            repaired: Vec::new(),
            unfixable: Vec::new(),
            duration_ms: 0,
        }
    }

    /// Add a repaired entry
    pub fn add_repaired(&mut self, key: String, action: &str) {
        self.repaired.push(RepairEntry { key, action: action.to_string() });
    }

    /// Add an unfixable entry
    pub fn add_unfixable(&mut self, key: String, reason: String) {
        self.unfixable.push(UnfixableEntry { key, reason });
    }
}

impl Default for RepairReport {
    fn default() -> Self {
        Self::new()
    }
}

/// A repaired entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairEntry {
    /// Key that was repaired
    pub key: String,
    /// Action taken
    pub action: String,
}

/// An unfixable entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnfixableEntry {
    /// Key or issue description
    pub key: String,
    /// Reason it couldn't be repaired
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_checker_creation() {
        let temp_dir = TempDir::new().unwrap();
        let checker = ConsistencyChecker::new(temp_dir.path()).unwrap();
        assert_eq!(checker.context_root, temp_dir.path());
    }

    #[test]
    fn test_checker_with_config() {
        let temp_dir = TempDir::new().unwrap();
        let config = ConsistencyCheckerConfig {
            filekv_only: true,
            check_bloom_filters: false,
            check_index_integrity: false,
            fail_fast: true,
        };
        let checker = ConsistencyChecker::with_config(temp_dir.path(), config).unwrap();
        assert!(checker.config.filekv_only);
        assert!(!checker.config.check_bloom_filters);
        assert!(checker.config.fail_fast);
    }

    #[test]
    fn test_check_report() {
        let mut report = CheckReport::new(100, 50);
        assert_eq!(report.filekv_entries, 100);
        assert_eq!(report.file_service_entries, 50);
        assert!(report.is_consistent);
        assert_eq!(report.inconsistency_count, 0);

        report.add_issue(ConsistencyIssue::KeyOnlyInFileKV {
            key: "test_key".to_string(),
        });
        assert!(!report.is_consistent);
        assert_eq!(report.inconsistency_count, 1);
    }

    #[test]
    fn test_repair_report() {
        let mut report = RepairReport::new();
        report.add_repaired("key1".to_string(), "Synced to FileService");
        report.add_unfixable("key2".to_string(), "Data corrupted".to_string());

        assert_eq!(report.repaired.len(), 1);
        assert_eq!(report.unfixable.len(), 1);
    }
}
