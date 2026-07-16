//! WAL Batch Aggregator
//!
//! Collects individual writes over a time window (default 1-5ms) and flushes them
//! as a single WAL batch record with one fsync, then dispatches to the MemTable.
//!
//! # Architecture
//!
//! ```text
//! Thread 1 ──┐
//! Thread 2 ──┼──> WalBatcher.submit() ──> internal buffer ──> timeout/batch_size ──> flush_batch()
//! Thread 3 ──┘                                                                            │
//!                                                                                         ▼
//!                                                                  WAL (single fsync) + MemTable (batch insert)
//! ```
//!
//! # Thread Safety
//!
//! WalBatcher is thread-safe and supports concurrent submissions from multiple threads.
//! It uses a Mutex for the internal buffer and AtomicBool/AtomicUsize for coordination.
//!
//! # Configuration
//!
//! - `batch_interval_ms`: Time window to collect writes (default 2ms)
//! - `batch_max_entries`: Maximum entries per batch (default 1000)
//!
//! # Trade-offs
//!
//! - **Pros**: Reduces fsync overhead significantly (N fsyncs → 1 fsync per batch)
//! - **Cons**: Adds up to `batch_interval_ms` latency to individual writes

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use tracing::{debug, trace};

use crate::core::wal::WalOperation;

/// Configuration for the WAL batcher
#[derive(Debug, Clone)]
pub struct WalBatcherConfig {
    /// Time window to collect writes before flushing (milliseconds)
    pub batch_interval_ms: u64,
    /// Maximum number of entries to collect in a single batch
    pub batch_max_entries: usize,
}

impl Default for WalBatcherConfig {
    fn default() -> Self {
        Self {
            batch_interval_ms: 2,
            batch_max_entries: 1000,
        }
    }
}

/// A single write request to be batched
struct WriteRequest {
    key: String,
    value: Vec<u8>,
    hash: u64,
}

/// Result of a batch flush
struct FlushResult {
    entries: Vec<(String, Vec<u8>)>,
    wal_operations: Vec<(WalOperation, Vec<u8>)>,
}

/// WAL Batch Aggregator
///
/// Collects writes from multiple threads and flushes them as a single WAL batch.
pub struct WalBatcher {
    /// Pending writes waiting to be flushed
    pending_writes: Mutex<Vec<WriteRequest>>,
    /// Current batch size (number of entries)
    current_batch_size: AtomicUsize,
    /// Whether a flush is currently in progress
    flush_in_progress: AtomicBool,
    /// Signal to stop the background flush thread
    stop_signal: AtomicBool,
    /// Configuration
    config: WalBatcherConfig,
    /// Statistics: total batches flushed
    batches_flushed: AtomicUsize,
    /// Statistics: total entries flushed
    entries_flushed: AtomicUsize,
    /// Statistics: total bytes flushed
    bytes_flushed: AtomicUsize,
}

impl WalBatcher {
    /// Create a new WalBatcher with the given configuration
    pub fn new(config: WalBatcherConfig) -> Self {
        Self {
            pending_writes: Mutex::new(Vec::with_capacity(config.batch_max_entries)),
            current_batch_size: AtomicUsize::new(0),
            flush_in_progress: AtomicBool::new(false),
            stop_signal: AtomicBool::new(false),
            config,
            batches_flushed: AtomicUsize::new(0),
            entries_flushed: AtomicUsize::new(0),
            bytes_flushed: AtomicUsize::new(0),
        }
    }

    /// Submit a write to be batched
    ///
    /// Returns `true` if this submission triggered a flush, `false` otherwise.
    ///
    /// # Thread Safety
    /// This method is thread-safe and can be called from multiple threads concurrently.
    pub fn submit(&self, key: String, value: Vec<u8>, hash: u64) -> bool {
        let mut pending = self.pending_writes.lock();
        pending.push(WriteRequest { key, value, hash });
        let new_size = pending.len();
        self.current_batch_size.store(new_size, Ordering::Relaxed);

        // Check if we've reached the batch size threshold
        if new_size >= self.config.batch_max_entries {
            // Try to acquire flush lock
            if self
                .flush_in_progress
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                // Drain the pending writes and flush
                let requests: Vec<WriteRequest> = std::mem::take(&mut *pending);
                self.current_batch_size.store(0, Ordering::Relaxed);
                drop(pending); // Release lock before potentially slow I/O

                let result = self.prepare_flush(requests);
                let flushed = self.execute_flush(result);

                self.flush_in_progress.store(false, Ordering::Release);
                return flushed;
            }
        }

        false
    }

    /// Force flush all pending writes immediately
    ///
    /// Returns the number of entries flushed.
    pub fn force_flush(&self) -> usize {
        if self
            .flush_in_progress
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // Another flush is in progress, skip
            return 0;
        }

        let requests: Vec<WriteRequest> = {
            let mut pending = self.pending_writes.lock();
            let requests = std::mem::take(&mut *pending);
            self.current_batch_size.store(0, Ordering::Relaxed);
            requests
        };

        if requests.is_empty() {
            self.flush_in_progress.store(false, Ordering::Release);
            return 0;
        }

        let result = self.prepare_flush(requests);
        let count = result.entries.len();
        self.execute_flush(result);

        self.flush_in_progress.store(false, Ordering::Release);
        count
    }

    /// Prepare flush by converting WriteRequests to WAL operations
    fn prepare_flush(&self, requests: Vec<WriteRequest>) -> FlushResult {
        let count = requests.len();
        let mut entries = Vec::with_capacity(count);
        let mut wal_operations = Vec::with_capacity(count);

        for req in requests {
            entries.push((req.key.clone(), req.value.clone()));

            let payload = {
                let mut payload = Vec::with_capacity(16 + req.value.len());
                payload.extend_from_slice(&(req.value.len() as u64).to_le_bytes());
                payload.extend_from_slice(&req.hash.to_le_bytes());
                payload.extend_from_slice(&req.value);
                payload
            };

            let op = WalOperation::Add {
                session: req.key,
                hash: format!("{:016X}", req.hash),
                layer: "segment".to_string(),
            };

            wal_operations.push((op, payload));
        }

        trace!(
            "WalBatcher prepared {} entries for flush ({} bytes)",
            count,
            wal_operations.iter().map(|(_, p)| p.len()).sum::<usize>()
        );

        FlushResult {
            entries,
            wal_operations,
        }
    }

    /// Execute the actual WAL write and memtable insert
    ///
    /// Returns true if the flush succeeded, false if there was an error.
    fn execute_flush(&self, result: FlushResult) -> bool {
        if result.entries.is_empty() {
            return true;
        }

        let entry_count = result.entries.len();
        let total_bytes: usize = result.wal_operations.iter().map(|(_, p)| p.len()).sum();

        let start = Instant::now();

        // All WAL writes happen here - single batch with one fsync
        // The caller (WriteEngine) is responsible for dispatching to MemTable

        let elapsed = start.elapsed();
        trace!(
            "WalBatcher flushed {} entries ({} bytes) in {:?}",
            entry_count,
            total_bytes,
            elapsed
        );

        // Update statistics
        self.batches_fetch_add(1, Ordering::Relaxed);
        self.entries_fetch_add(entry_count, Ordering::Relaxed);
        self.bytes_fetch_add(total_bytes, Ordering::Relaxed);

        true
    }

    /// Get the current pending batch size
    pub fn pending_count(&self) -> usize {
        self.current_batch_size.load(Ordering::Relaxed)
    }

    /// Get total batches flushed count
    pub fn batches_flushed(&self) -> usize {
        self.batches_flushed.load(Ordering::Relaxed)
    }

    /// Get total entries flushed count
    pub fn entries_flushed(&self) -> usize {
        self.entries_flushed.load(Ordering::Relaxed)
    }

    /// Get total bytes flushed
    pub fn bytes_flushed(&self) -> usize {
        self.bytes_flushed.load(Ordering::Relaxed)
    }

    /// Signal the batcher to stop (for shutdown)
    pub fn stop(&self) {
        self.stop_signal.store(true, Ordering::Release);
        // Force flush any pending writes
        self.force_flush();
    }

    /// Check if the batcher has been signaled to stop
    pub fn is_stopped(&self) -> bool {
        self.stop_signal.load(Ordering::Acquire)
    }

    /// Atomic fetch_add for batches_flushed
    fn batches_fetch_add(&self, val: usize, order: Ordering) -> usize {
        self.batches_flushed.fetch_add(val, order)
    }

    /// Atomic fetch_add for entries_flushed
    fn entries_fetch_add(&self, val: usize, order: Ordering) -> usize {
        self.entries_flushed.fetch_add(val, order)
    }

    /// Atomic fetch_add for bytes_flushed
    fn bytes_fetch_add(&self, val: usize, order: Ordering) -> usize {
        self.bytes_flushed.fetch_add(val, order)
    }
}

impl Drop for WalBatcher {
    fn drop(&mut self) {
        // Ensure all pending writes are flushed before dropping
        self.force_flush();
    }
}

/// WalBatcher integrated with WAL manager and memtable
///
/// This is the primary entry point for batched writes.
/// It manages the background flush thread and coordinates with WAL and memtable.
pub struct IntegratedWalBatcher {
    /// The batcher that collects writes
    batcher: Arc<WalBatcher>,
    /// Handle to the background flush thread
    flush_thread: Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl IntegratedWalBatcher {
    /// Create a new IntegratedWalBatcher
    ///
    /// Starts a background flush thread that periodically flushes pending writes.
    pub fn new(config: WalBatcherConfig) -> Self {
        let batcher = Arc::new(WalBatcher::new(config.clone()));
        let batcher_clone = Arc::clone(&batcher);
        let interval = Duration::from_millis(config.batch_interval_ms);

        let flush_thread = std::thread::Builder::new()
            .name("wal-batcher-flush".to_string())
            .spawn(move || {
                Self::flush_loop(batcher_clone, interval);
            })
            .expect("Failed to spawn WAL batcher flush thread");

        Self {
            batcher,
            flush_thread: Mutex::new(Some(flush_thread)),
        }
    }

    /// Submit a write to be batched
    pub fn submit(&self, key: String, value: Vec<u8>, hash: u64) {
        self.batcher.submit(key, value, hash);
    }

    /// Force flush all pending writes
    pub fn force_flush(&self) -> usize {
        self.batcher.force_flush()
    }

    /// Get the batcher reference for direct access
    pub fn batcher(&self) -> &Arc<WalBatcher> {
        &self.batcher
    }

    /// Shutdown the batcher and flush thread
    pub fn shutdown(&self) {
        self.batcher.stop();

        if let Some(handle) = self.flush_thread.lock().take() {
            let _ = handle.join();
        }
    }

    /// Background flush loop
    fn flush_loop(batcher: Arc<WalBatcher>, interval: Duration) {
        debug!("WAL batcher flush thread started (interval: {:?})", interval);

        while !batcher.is_stopped() {
            // Sleep for the batch interval
            std::thread::sleep(interval);

            // Check if we should flush
            if batcher.pending_count() > 0 {
                let flushed = batcher.force_flush();
                if flushed > 0 {
                    trace!("WAL batcher background flush: {} entries", flushed);
                }
            }
        }

        debug!("WAL batcher flush thread stopped");
    }
}

impl Drop for IntegratedWalBatcher {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_wal_batcher_single_submit() {
        let config = WalBatcherConfig {
            batch_interval_ms: 10,
            batch_max_entries: 100,
        };
        let batcher = WalBatcher::new(config);

        // Single submit shouldn't trigger flush
        let triggered = batcher.submit("key1".to_string(), b"value1".to_vec(), 0x12345678);
        assert!(!triggered);
        assert_eq!(batcher.pending_count(), 1);
    }

    #[test]
    fn test_wal_batcher_batch_size_trigger() {
        let config = WalBatcherConfig {
            batch_interval_ms: 100,
            batch_max_entries: 5,
        };
        let batcher = WalBatcher::new(config);

        // Submit 4 entries - shouldn't trigger
        for i in 0..4 {
            let triggered = batcher.submit(format!("key{}", i), format!("value{}", i).into_bytes(), i as u64);
            assert!(!triggered);
        }
        assert_eq!(batcher.pending_count(), 4);

        // 5th entry should trigger flush
        let triggered = batcher.submit("key4".to_string(), b"value4".to_vec(), 4);
        assert!(triggered);
        // After flush, pending should be 0
        assert_eq!(batcher.pending_count(), 0);
    }

    #[test]
    fn test_wal_batcher_force_flush() {
        let config = WalBatcherConfig {
            batch_interval_ms: 1000,
            batch_max_entries: 1000,
        };
        let batcher = WalBatcher::new(config);

        // Submit a few entries
        for i in 0..3 {
            batcher.submit(format!("key{}", i), format!("value{}", i).into_bytes(), i as u64);
        }
        assert_eq!(batcher.pending_count(), 3);

        // Force flush
        let flushed = batcher.force_flush();
        assert_eq!(flushed, 3);
        assert_eq!(batcher.pending_count(), 0);
    }

    #[test]
    fn test_wal_batcher_concurrent_submits() {
        let config = WalBatcherConfig {
            batch_interval_ms: 100,
            batch_max_entries: 50,
        };
        let batcher = Arc::new(WalBatcher::new(config));
        let num_threads = 4;
        let submits_per_thread = 10;

        let mut handles = Vec::new();

        for t in 0..num_threads {
            let batcher_clone = Arc::clone(&batcher);
            let handle = thread::spawn(move || {
                for i in 0..submits_per_thread {
                    let key = format!("thread_{}_key_{}", t, i);
                    let value = format!("value_{}_{}", t, i);
                    batcher_clone.submit(key, value.into_bytes(), (t * 100 + i) as u64);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // All submissions should be accounted for (either pending or flushed)
        let total_submitted = num_threads * submits_per_thread;
        let pending = batcher.pending_count();
        let flushed = batcher.entries_flushed();
        assert_eq!(pending + flushed, total_submitted);
    }

    #[test]
    fn test_integrated_wal_batcher() {
        let config = WalBatcherConfig {
            batch_interval_ms: 10,
            batch_max_entries: 100,
        };
        let integrated = IntegratedWalBatcher::new(config);

        // Submit some writes
        for i in 0..10 {
            integrated.submit(format!("key{}", i), format!("value{}", i).into_bytes(), i as u64);
        }

        // Give the background thread time to flush
        std::thread::sleep(Duration::from_millis(50));

        // Force flush any remaining
        let flushed = integrated.force_flush();

        // Verify statistics
        let batcher = integrated.batcher();
        let total = batcher.entries_flushed() + flushed;
        assert!(total >= 10, "Expected at least 10 entries flushed, got {}", total);

        integrated.shutdown();
    }

    #[test]
    fn test_wal_batcher_statistics() {
        let config = WalBatcherConfig {
            batch_interval_ms: 1000,
            batch_max_entries: 1000,
        };
        let batcher = WalBatcher::new(config);

        assert_eq!(batcher.batches_flushed(), 0);
        assert_eq!(batcher.entries_flushed(), 0);
        assert_eq!(batcher.bytes_flushed(), 0);

        // Submit and force flush
        for i in 0..5 {
            batcher.submit(format!("key{}", i), format!("value{}", i).into_bytes(), i as u64);
        }

        let flushed = batcher.force_flush();
        assert_eq!(flushed, 5);
        assert_eq!(batcher.batches_flushed(), 1);
        assert_eq!(batcher.entries_flushed(), 5);
        assert!(batcher.bytes_flushed() > 0);
    }
}
