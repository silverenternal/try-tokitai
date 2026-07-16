//! FaultInjector: Decorator that wraps any FileKVFileSystem and injects failures
//!
//! Supports configurable fault injection rules:
//! - Fail after N calls (e.g., "disk full" after N writes)
//! - Fail randomly with given probability
//! - Fail with a specific error
//! - Delay operations by a given duration

use std::any::Any;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::io::{FileKVFile, FileKVFileSystem, FileMetadata, IoResult, MmapFileSystem, MmapView};

/// Strategy for fault injection
#[derive(Debug, Clone)]
pub enum FaultStrategy {
    /// Fail after N successful calls
    FailAfterN(u64),
    /// Fail randomly with given probability (0.0 to 1.0)
    FailRandom(f64),
    /// Always fail with the given error kind and message
    AlwaysFail(std::io::ErrorKind, String),
    /// Delay each operation by the given duration
    Delay(Duration),
    /// Combination: apply delay AND fault
    Combined {
        delay: Option<Duration>,
        fault: Box<FaultStrategy>,
    },
}

/// A single fault injection rule
#[derive(Debug, Clone)]
pub struct FaultRule {
    /// Which operations this rule applies to (empty = all operations)
    pub operation_prefixes: Vec<String>,
    /// The fault strategy
    pub strategy: FaultStrategy,
    /// Whether this rule is active
    pub active: bool,
}

impl FaultRule {
    pub fn new_all(strategy: FaultStrategy) -> Self {
        Self {
            operation_prefixes: vec![],
            strategy,
            active: true,
        }
    }

    pub fn new_for_ops(strategy: FaultStrategy, prefixes: &[&str]) -> Self {
        Self {
            operation_prefixes: prefixes.iter().map(|s| s.to_string()).collect(),
            strategy,
            active: true,
        }
    }

    fn matches(&self, operation: &str) -> bool {
        if self.operation_prefixes.is_empty() {
            return true;
        }
        self.operation_prefixes.iter().any(|p| operation.starts_with(p))
    }
}

/// Fault-injecting filesystem decorator
pub struct FaultInjector {
    inner: Arc<dyn FileKVFileSystem>,
    /// Optional mmap support - only set if inner implements MmapFileSystem
    mmap_inner: Option<Arc<dyn MmapFileSystem>>,
    rules: Arc<parking_lot::Mutex<Vec<FaultRule>>>,
    call_count: Arc<AtomicU64>,
    rng_seed: Arc<AtomicU64>,
}

impl FaultInjector {
    pub fn new(inner: Arc<dyn FileKVFileSystem>) -> Self {
        Self {
            inner,
            mmap_inner: None,
            rules: Arc::new(parking_lot::Mutex::new(Vec::new())),
            call_count: Arc::new(AtomicU64::new(0)),
            rng_seed: Arc::new(AtomicU64::new(42)),
        }
    }

    /// Create a FaultInjector with mmap support
    pub fn new_with_mmap(inner: Arc<dyn MmapFileSystem>) -> Self {
        Self {
            inner: inner.clone(),
            mmap_inner: Some(inner),
            rules: Arc::new(parking_lot::Mutex::new(Vec::new())),
            call_count: Arc::new(AtomicU64::new(0)),
            rng_seed: Arc::new(AtomicU64::new(42)),
        }
    }

    pub fn add_rule(&self, rule: FaultRule) {
        self.rules.lock().push(rule);
    }

    pub fn clear_rules(&self) {
        self.rules.lock().clear();
    }

    /// Convenience: set disk full after N calls
    pub fn set_disk_full_after(&self, n: u64) {
        self.clear_rules();
        self.add_rule(FaultRule::new_all(FaultStrategy::FailAfterN(n)));
    }

    /// Convenience: fail randomly with given probability
    pub fn set_random_fail(&self, probability: f64) {
        self.clear_rules();
        self.add_rule(FaultRule::new_all(FaultStrategy::FailRandom(probability)));
    }

    /// Convenience: add delay to all operations
    pub fn set_delay(&self, delay: Duration) {
        self.clear_rules();
        self.add_rule(FaultRule::new_all(FaultStrategy::Delay(delay)));
    }

    fn check_fault(&self, operation: &str) -> Option<std::io::Error> {
        let count = self.call_count.fetch_add(1, Ordering::Relaxed);
        let rules = self.rules.lock();

        for rule in rules.iter() {
            if !rule.active {
                continue;
            }
            if !rule.matches(operation) {
                continue;
            }

            match &rule.strategy {
                FaultStrategy::FailAfterN(n) => {
                    if count >= *n {
                        return Some(std::io::Error::new(
                            std::io::ErrorKind::StorageFull,
                            "FaultInjector: disk full (FailAfterN triggered)",
                        ));
                    }
                }
                FaultStrategy::FailRandom(probability) => {
                    // Simple LCG for determinism
                    let prev = self.rng_seed.fetch_add(1, Ordering::Relaxed);
                    let rand_val = ((prev * 6364136223846793005u64.wrapping_add(1442695040888963407u64)) >> 33) as f64
                        / (u32::MAX as f64);
                    if rand_val < *probability {
                        return Some(std::io::Error::other(format!(
                            "FaultInjector: random failure (probability={})",
                            probability
                        )));
                    }
                }
                FaultStrategy::AlwaysFail(kind, msg) => {
                    return Some(std::io::Error::new(*kind, format!("FaultInjector: {}", msg)));
                }
                FaultStrategy::Delay(dur) => {
                    thread::sleep(*dur);
                }
                FaultStrategy::Combined { delay, fault } => {
                    if let Some(d) = delay {
                        thread::sleep(*d);
                    }
                    // Recursively check the fault part
                    // For simplicity, handle directly:
                    if let FaultStrategy::FailAfterN(n) = fault.as_ref() {
                        if count >= *n {
                            return Some(std::io::Error::new(
                                std::io::ErrorKind::StorageFull,
                                "FaultInjector: combined fault triggered",
                            ));
                        }
                    }
                }
            }
        }

        None
    }

    fn maybe_fault(&self, operation: &str) -> IoResult<()> {
        match self.check_fault(operation) {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }
}

impl FileKVFile for FaultInjectFile {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.maybe_fault("file.read")?;
        self.inner.read(buf)
    }
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.maybe_fault("file.write")?;
        self.inner.write(buf)
    }
    fn write_all(&mut self, buf: &[u8]) -> IoResult<()> {
        self.maybe_fault("file.write_all")?;
        self.inner.write_all(buf)
    }
    fn flush(&mut self) -> IoResult<()> {
        self.maybe_fault("file.flush")?;
        self.inner.flush()
    }
    fn sync_all(&self) -> IoResult<()> {
        self.maybe_fault("file.sync_all")?;
        self.inner.sync_all()
    }
    fn try_clone(&self) -> IoResult<Box<dyn FileKVFile>> {
        self.maybe_fault("file.try_clone")?;
        self.inner.try_clone()
    }
    fn metadata(&self) -> IoResult<FileMetadata> {
        self.maybe_fault("file.metadata")?;
        self.inner.metadata()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl FileKVFileSystem for FaultInjector {
    fn create_file(&self, path: &Path) -> IoResult<Box<dyn FileKVFile>> {
        self.maybe_fault("create_file")?;
        let inner = self.inner.create_file(path)?;
        Ok(Box::new(FaultInjectFile::new(inner, self)))
    }

    fn open_file(&self, path: &Path, read: bool, write: bool, append: bool) -> IoResult<Box<dyn FileKVFile>> {
        self.maybe_fault("open_file")?;
        let inner = self.inner.open_file(path, read, write, append)?;
        Ok(Box::new(FaultInjectFile::new(inner, self)))
    }

    fn read_dir(&self, path: &Path) -> IoResult<Vec<PathBuf>> {
        self.maybe_fault("read_dir")?;
        self.inner.read_dir(path)
    }

    fn create_dir_all(&self, path: &Path) -> IoResult<()> {
        self.maybe_fault("create_dir_all")?;
        self.inner.create_dir_all(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> IoResult<()> {
        self.maybe_fault("rename")?;
        self.inner.rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> IoResult<()> {
        self.maybe_fault("remove_file")?;
        self.inner.remove_file(path)
    }

    fn file_exists(&self, path: &Path) -> bool {
        self.inner.file_exists(path)
    }

    fn file_metadata(&self, path: &Path) -> IoResult<FileMetadata> {
        self.maybe_fault("file_metadata")?;
        self.inner.file_metadata(path)
    }

    fn sync_dir(&self, path: &Path) -> IoResult<()> {
        self.maybe_fault("sync_dir")?;
        self.inner.sync_dir(path)
    }

    fn clone_as_mmap_fs(&self) -> Option<Arc<dyn MmapFileSystem>> {
        self.mmap_inner.clone()
    }
}

impl MmapFileSystem for FaultInjector {
    fn mmap(&self, file: &dyn FileKVFile) -> IoResult<Arc<dyn MmapView>> {
        self.maybe_fault("mmap")?;
        let mmap_inner = self.mmap_inner.as_ref().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "mmap not supported by inner filesystem",
            )
        })?;
        let fault_file = file
            .as_any()
            .downcast_ref::<FaultInjectFile>()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Expected FaultInjectFile"))?;
        mmap_inner.mmap(fault_file.inner.as_ref())
    }
}

/// Fault-injecting file handle decorator
pub struct FaultInjectFile {
    inner: Box<dyn FileKVFile>,
    call_count: Arc<AtomicU64>,
    rules: Arc<parking_lot::Mutex<Vec<FaultRule>>>,
    rng_seed: Arc<AtomicU64>,
}

impl FaultInjectFile {
    fn new(inner: Box<dyn FileKVFile>, parent: &FaultInjector) -> Self {
        Self {
            inner,
            call_count: parent.call_count.clone(),
            rules: parent.rules.clone(), // Arc clone, shares the same Mutex
            rng_seed: parent.rng_seed.clone(),
        }
    }

    fn maybe_fault(&self, operation: &str) -> IoResult<()> {
        let count = self.call_count.fetch_add(1, Ordering::Relaxed);
        let rules = self.rules.lock();

        for rule in rules.iter() {
            if !rule.active || !rule.matches(operation) {
                continue;
            }
            match &rule.strategy {
                FaultStrategy::FailAfterN(n) => {
                    if count >= *n {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::StorageFull,
                            "FaultInjector: disk full",
                        ));
                    }
                }
                FaultStrategy::FailRandom(probability) => {
                    let prev = self.rng_seed.fetch_add(1, Ordering::Relaxed);
                    let rand_val = ((prev.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)) >> 33)
                        as f64
                        / (u32::MAX as f64);
                    if rand_val < *probability {
                        return Err(std::io::Error::other("FaultInjector: random failure".to_string()));
                    }
                }
                FaultStrategy::AlwaysFail(kind, msg) => {
                    return Err(std::io::Error::new(*kind, format!("FaultInjector: {}", msg)));
                }
                FaultStrategy::Delay(dur) => {
                    std::thread::sleep(*dur);
                }
                FaultStrategy::Combined { delay, fault } => {
                    if let Some(d) = delay {
                        std::thread::sleep(*d);
                    }
                    if let FaultStrategy::FailAfterN(n) = fault.as_ref() {
                        if count >= *n {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::StorageFull,
                                "FaultInjector: combined fault",
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::{FileKVFileSystem, MemFs};

    #[test]
    fn test_fail_after_n_calls() {
        let memfs = Arc::new(MemFs::new());
        let injector = FaultInjector::new(memfs);
        injector.set_disk_full_after(3);

        // First 3 calls should succeed
        for i in 0..3 {
            let result = injector.create_file(std::path::Path::new(&format!("/file_{}.txt", i)));
            assert!(result.is_ok(), "Call {} should succeed", i);
        }

        // 4th call should fail
        let result = injector.create_file(std::path::Path::new("/file_3.txt"));
        assert!(result.is_err(), "4th call should fail (disk full)");
    }

    #[test]
    fn test_clear_rules_disables_fault() {
        let memfs = Arc::new(MemFs::new());
        let injector = FaultInjector::new(memfs);
        injector.set_disk_full_after(1);

        // First call succeeds
        assert!(injector.create_file(std::path::Path::new("/a.txt")).is_ok());
        // Second fails
        assert!(injector.create_file(std::path::Path::new("/b.txt")).is_err());

        // Clear rules - now all succeed
        injector.clear_rules();
        for i in 0..10 {
            let result = injector.create_file(std::path::Path::new(&format!("/clear_{}.txt", i)));
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_fault_only_on_matched_operations() {
        let memfs = Arc::new(MemFs::new());
        let injector = FaultInjector::new(memfs.clone());

        // Only fail on write operations
        injector.add_rule(FaultRule::new_for_ops(
            FaultStrategy::AlwaysFail(std::io::ErrorKind::Other, "write fault".to_string()),
            &["file.write"],
        ));

        // read_dir should succeed
        memfs.create_dir_all(std::path::Path::new("/test")).unwrap();
        assert!(injector.read_dir(std::path::Path::new("/test")).is_ok());
    }

    // ==================== IO-003: Compound Fault Rules Tests ====================

    /// Test: IO-003 - Multiple rules matching the same operation
    /// Verifies that the first matching rule takes effect (rule priority by order)
    #[test]
    fn test_multiple_fault_rules_first_matches_wins() {
        let memfs = Arc::new(MemFs::new());
        let injector = FaultInjector::new(memfs);

        // Add two rules that both match "create_file" operation:
        // Rule 1: Always fail with PermissionDenied
        injector.add_rule(FaultRule::new_all(FaultStrategy::AlwaysFail(
            std::io::ErrorKind::PermissionDenied,
            "first rule error".to_string(),
        )));
        // Rule 2: Fail after 0 (immediate fail with StorageFull)
        injector.add_rule(FaultRule::new_all(FaultStrategy::FailAfterN(0)));

        // First matching rule should win (AlwaysFail -> PermissionDenied)
        let result = injector.create_file(std::path::Path::new("/test.txt"));
        assert!(result.is_err(), "Should fail due to first matching rule");
        if let Err(err) = result {
            assert_eq!(
                err.kind(),
                std::io::ErrorKind::PermissionDenied,
                "Should use first matching rule's error"
            );
        }
    }

    /// Test: IO-003 - Rule priority: order matters when multiple rules match
    #[test]
    fn test_fault_rule_priority_order() {
        let memfs = Arc::new(MemFs::new());
        let injector = FaultInjector::new(memfs);

        // Rule 1: Delay (should be applied first, but doesn't fail)
        injector.add_rule(FaultRule::new_all(FaultStrategy::Delay(Duration::from_millis(1))));
        // Rule 2: Always fail with Interrupted
        injector.add_rule(FaultRule::new_all(FaultStrategy::AlwaysFail(
            std::io::ErrorKind::Interrupted,
            "second rule error".to_string(),
        )));

        // Delay rule runs first (no error), then AlwaysFail triggers
        let result = injector.create_file(std::path::Path::new("/priority.txt"));
        assert!(result.is_err(), "Should fail due to second rule");
        if let Err(err) = result {
            assert_eq!(
                err.kind(),
                std::io::ErrorKind::Interrupted,
                "Second rule should trigger after first (delay) doesn't fail"
            );
        }
    }

    /// Test: IO-003 - Inactive rules are skipped, next active rule takes effect
    #[test]
    fn test_inactive_rule_skipped_next_active_applied() {
        let memfs = Arc::new(MemFs::new());
        let injector = FaultInjector::new(memfs);

        // Rule 1: Inactive (should be skipped)
        let mut inactive_rule = FaultRule::new_all(FaultStrategy::AlwaysFail(
            std::io::ErrorKind::NotFound,
            "inactive rule error".to_string(),
        ));
        inactive_rule.active = false;
        injector.add_rule(inactive_rule);

        // Rule 2: Active always-fail rule
        injector.add_rule(FaultRule::new_all(FaultStrategy::AlwaysFail(
            std::io::ErrorKind::TimedOut,
            "active rule error".to_string(),
        )));

        let result = injector.create_file(std::path::Path::new("/inactive_test.txt"));
        assert!(result.is_err());
        if let Err(err) = result {
            assert_eq!(
                err.kind(),
                std::io::ErrorKind::TimedOut,
                "Inactive rule should be skipped, active rule should trigger"
            );
        }
    }

    /// Test: IO-003 - Combined fault strategy: delay + fail after N
    #[test]
    fn test_combined_fault_strategy_delay_and_fail() {
        let memfs = Arc::new(MemFs::new());
        let injector = FaultInjector::new(memfs);

        // Combined: delay 1ms + fail after 2 calls
        let combined = FaultStrategy::Combined {
            delay: Some(Duration::from_millis(1)),
            fault: Box::new(FaultStrategy::FailAfterN(2)),
        };
        injector.add_rule(FaultRule::new_all(combined));

        // First 2 calls should succeed (with delay)
        for i in 0..2 {
            let result = injector.create_file(std::path::Path::new(&format!("/combined_{}.txt", i)));
            assert!(result.is_ok(), "Call {} should succeed (with delay)", i);
        }

        // 3rd call should fail
        let result = injector.create_file(std::path::Path::new("/combined_fail.txt"));
        assert!(result.is_err(), "3rd call should fail (FailAfterN triggered)");
    }

    /// Test: IO-003 - Rules specific to file operations don't affect directory operations
    #[test]
    fn test_file_specific_rules_dont_affect_directory_ops() {
        let memfs = Arc::new(MemFs::new());
        let injector = FaultInjector::new(memfs.clone());

        // Only fail on file.write operations
        injector.add_rule(FaultRule::new_for_ops(
            FaultStrategy::AlwaysFail(std::io::ErrorKind::StorageFull, "write fault".to_string()),
            &["file.write"],
        ));

        // create_dir_all should succeed
        let dir_path = std::path::Path::new("/test_dir");
        assert!(
            injector.create_dir_all(dir_path).is_ok(),
            "create_dir_all should succeed"
        );

        // read_dir should succeed
        assert!(injector.read_dir(dir_path).is_ok(), "read_dir should succeed");

        // create_file should succeed (rule only targets write, not create)
        let file = injector.create_file(std::path::Path::new("/test_dir/file.txt"));
        assert!(file.is_ok(), "create_file should succeed");

        // But writing to the file should fail
        if let Ok(mut f) = file {
            let result = f.write_all(b"test data");
            assert!(result.is_err(), "write_all should fail due to file.write rule");
        }
    }
}
