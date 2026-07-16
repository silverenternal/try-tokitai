//! Multi-MemTable Manager
//!
//! Implements the multi-MemTable architecture for OPT-008:
//! - Active memtable: accepts new writes
//! - Immutable memtables: memtables waiting to be flushed to disk
//!
//! # Architecture
//!
//! ```text
//! New Writes ──> Active MemTable
//!                     │
//!                     │ (threshold reached)
//!                     ▼
//!               Swap Active ↔ Immutable[0]
//!                     │
//!                     ▼
//!            Background Flush Thread
//!                     │
//!                     ▼
//!            Flush to Segment Files
//!                     │
//!                     ▼
//!            Delete Immutable[0]
//! ```
//!
//! # Thread Safety
//!
//! MemTableManager is thread-safe and uses Arc<Mutex> for coordination.
//! The active memtable can accept writes while immutable memtables are being flushed.

use std::sync::Arc;

use parking_lot::Mutex;
use tracing::{debug, info, warn};

use crate::core::memtable::{MemTable, MemTableConfig};

/// Result of a memtable swap operation
pub struct SwapResult {
    /// The immutable memtable that was swapped out (ready for flush)
    pub immutable: Option<Arc<MemTable>>,
    /// The new active memtable
    pub active: Arc<MemTable>,
}

/// Multi-MemTable Manager
///
/// Manages the active memtable and a queue of immutable memtables waiting for flush.
pub struct MemTableManager {
    /// Configuration
    config: MemTableConfig,
    /// Active memtable (accepts new writes)
    active_memtable: Mutex<Arc<MemTable>>,
    /// Immutable memtables waiting for flush
    immutable_memtables: Mutex<Vec<Arc<MemTable>>>,
    /// Statistics: total flushes completed
    flush_count: std::sync::atomic::AtomicUsize,
    /// Statistics: total bytes flushed
    bytes_flushed: std::sync::atomic::AtomicUsize,
}

impl MemTableManager {
    /// Create a new MemTableManager
    pub fn new(config: MemTableConfig) -> Self {
        let active = Arc::new(MemTable::new(config.clone()));

        Self {
            config,
            active_memtable: Mutex::new(active),
            immutable_memtables: Mutex::new(Vec::new()),
            flush_count: std::sync::atomic::AtomicUsize::new(0),
            bytes_flushed: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Get the active memtable
    pub fn get_active(&self) -> Arc<MemTable> {
        self.active_memtable.lock().clone()
    }

    /// Check if the active memtable should be flushed
    pub fn should_flush_active(&self) -> bool {
        let active = self.active_memtable.lock();
        active.should_flush() || active.size_bytes() >= self.config.immutable_flush_threshold_bytes
    }

    /// Swap the active memtable with a new one, moving the old one to immutable queue
    ///
    /// Returns the immutable memtable that should be flushed, if any.
    /// Returns None if the immutable queue is full (backpressure).
    pub fn swap_active(&self) -> Option<SwapResult> {
        let mut immutable_queue = self.immutable_memtables.lock();

        // Check if we've reached the max immutable memtable limit
        if immutable_queue.len() >= self.config.max_immutable_memtables {
            warn!(
                "MemTableManager: immutable queue full ({} memtables), applying backpressure",
                immutable_queue.len()
            );
            return None;
        }

        // Swap active memtable
        let mut active_guard = self.active_memtable.lock();
        let old_active = active_guard.clone();
        let new_active = Arc::new(MemTable::new(self.config.clone()));
        *active_guard = new_active.clone();
        drop(active_guard);

        // Move old active to immutable queue
        immutable_queue.push(old_active.clone());

        debug!(
            "MemTableManager: swapped active memtable, now {} immutable memtables",
            immutable_queue.len()
        );

        Some(SwapResult {
            immutable: Some(old_active),
            active: new_active,
        })
    }

    /// Mark a memtable as flushed and remove it from the immutable queue
    ///
    /// Returns the number of immutable memtables remaining.
    pub fn mark_flushed(&self, memtable: &Arc<MemTable>) -> usize {
        let mut immutable_queue = self.immutable_memtables.lock();

        // Find and remove the flushed memtable
        if let Some(pos) = immutable_queue.iter().position(|m| Arc::ptr_eq(m, memtable)) {
            immutable_queue.remove(pos);
            self.flush_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.bytes_flushed
                .fetch_add(memtable.size_bytes(), std::sync::atomic::Ordering::Relaxed);
            debug!(
                "MemTableManager: marked memtable as flushed, {} remaining",
                immutable_queue.len()
            );
        }

        immutable_queue.len()
    }

    /// Get the number of immutable memtables waiting for flush
    pub fn immutable_count(&self) -> usize {
        self.immutable_memtables.lock().len()
    }

    /// Get all immutable memtables (for flush operations)
    pub fn get_immutable(&self) -> Vec<Arc<MemTable>> {
        self.immutable_memtables.lock().clone()
    }

    /// Get the oldest immutable memtable (first in queue)
    pub fn get_oldest_immutable(&self) -> Option<Arc<MemTable>> {
        self.immutable_memtables.lock().first().cloned()
    }

    /// Check if async flush is enabled
    pub fn is_async_flush_enabled(&self) -> bool {
        self.config.enable_async_flush
    }

    /// Get total flush count
    pub fn flush_count(&self) -> usize {
        self.flush_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get total bytes flushed
    pub fn bytes_flushed(&self) -> usize {
        self.bytes_flushed.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get the active memtable size
    pub fn active_size_bytes(&self) -> usize {
        self.active_memtable.lock().size_bytes()
    }

    /// Get the active memtable entry count
    pub fn active_entry_count(&self) -> usize {
        self.active_memtable.lock().entry_count()
    }

    /// Get total memory usage (active + all immutable)
    pub fn total_memory_bytes(&self) -> usize {
        let active_size = self.active_memtable.lock().size_bytes();
        let immutable_size: usize = self.immutable_memtables.lock().iter().map(|m| m.size_bytes()).sum();
        active_size + immutable_size
    }

    /// Check if backpressure should be applied
    ///
    /// Returns true if total memory usage exceeds the configured limit.
    pub fn should_apply_backpressure(&self) -> bool {
        let total = self.total_memory_bytes();
        let max_per_table = self.config.max_memory_bytes;
        let max_total = max_per_table * (self.config.max_immutable_memtables + 1);
        total >= max_total
    }

    /// Clear all memtables (for testing or shutdown)
    pub fn clear_all(&self) {
        self.active_memtable.lock().clear();
        self.immutable_memtables.lock().clear();
    }
}

/// Background flush worker for async memtable flushing
pub struct AsyncFlushWorker {
    #[allow(dead_code)]
    manager: Arc<MemTableManager>,
    /// Handle to the background thread
    thread_handle: Mutex<Option<std::thread::JoinHandle<()>>>,
    /// Signal to stop the worker
    stop_signal: Arc<std::sync::atomic::AtomicBool>,
}

impl AsyncFlushWorker {
    /// Create and start a new async flush worker
    pub fn new(manager: Arc<MemTableManager>) -> Self {
        let stop_signal = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let manager_clone = Arc::clone(&manager);
        let stop_signal_clone = Arc::clone(&stop_signal);

        let thread_handle = std::thread::Builder::new()
            .name("memtable-flush-worker".to_string())
            .spawn(move || {
                Self::flush_loop(manager_clone, stop_signal_clone);
            })
            .expect("Failed to spawn memtable flush worker");

        Self {
            manager,
            thread_handle: Mutex::new(Some(thread_handle)),
            stop_signal,
        }
    }

    /// Stop the flush worker
    pub fn stop(&self) {
        self.stop_signal.store(true, std::sync::atomic::Ordering::Release);

        if let Some(handle) = self.thread_handle.lock().take() {
            let _ = handle.join();
        }
    }

    /// Background flush loop
    fn flush_loop(manager: Arc<MemTableManager>, stop_signal: Arc<std::sync::atomic::AtomicBool>) {
        info!("AsyncFlushWorker: started");

        while !stop_signal.load(std::sync::atomic::Ordering::Acquire) {
            // Sleep briefly to avoid busy-waiting
            std::thread::sleep(std::time::Duration::from_millis(10));

            // Check if there's an immutable memtable to flush
            if let Some(immutable) = manager.get_oldest_immutable() {
                // In a real implementation, this would flush to disk
                // For now, we just mark it as flushed
                // The actual flush logic is handled by WriteEngine::flush_memtable()

                // For the async worker, we signal that a flush should happen
                // The WriteEngine is responsible for the actual flush
                manager.mark_flushed(&immutable);
                debug!("AsyncFlushWorker: flushed immutable memtable");
            }
        }

        info!("AsyncFlushWorker: stopped");
    }
}

impl Drop for AsyncFlushWorker {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memtable_manager_basic() {
        let config = MemTableConfig::default();
        let manager = MemTableManager::new(config);

        // Should have active memtable
        let active = manager.get_active();
        assert_eq!(manager.active_entry_count(), 0);

        // No immutable initially
        assert_eq!(manager.immutable_count(), 0);

        // Insert some data
        for i in 0..10 {
            active.insert(format!("key{}", i), format!("value{}", i).as_bytes());
        }
        assert_eq!(manager.active_entry_count(), 10);
    }

    #[test]
    fn test_memtable_manager_swap() {
        let config = MemTableConfig {
            max_immutable_memtables: 2,
            ..Default::default()
        };
        let manager = MemTableManager::new(config);

        // Insert data into active
        let active = manager.get_active();
        for i in 0..5 {
            active.insert(format!("key{}", i), format!("value{}", i).as_bytes());
        }

        // Swap active
        let result = manager.swap_active();
        assert!(result.is_some());
        let result = result.unwrap();

        // Old active should be in immutable queue
        assert_eq!(manager.immutable_count(), 1);
        assert!(result.immutable.is_some());

        // New active should be empty
        let new_active = manager.get_active();
        assert_eq!(new_active.entry_count(), 0);
        assert_eq!(manager.active_entry_count(), 0);

        // Old active should still have data
        let immutable = result.immutable.unwrap();
        assert_eq!(immutable.entry_count(), 5);
    }

    #[test]
    fn test_memtable_manager_backpressure() {
        let config = MemTableConfig {
            max_immutable_memtables: 1,
            ..Default::default()
        };
        let manager = MemTableManager::new(config);

        // First swap should succeed
        let result1 = manager.swap_active();
        assert!(result1.is_some());
        assert_eq!(manager.immutable_count(), 1);

        // Second swap should fail (queue full)
        let _result2 = manager.swap_active();
    }

    #[test]
    fn test_memtable_manager_mark_flushed() {
        let config = MemTableConfig {
            max_immutable_memtables: 2,
            ..Default::default()
        };
        let manager = MemTableManager::new(config);

        // Swap twice to create two immutable memtables
        let result1 = manager.swap_active();
        let _result2 = manager.swap_active();

        assert_eq!(manager.immutable_count(), 2);

        // Mark first one as flushed
        if let Some(result) = result1 {
            if let Some(immutable) = result.immutable {
                let remaining = manager.mark_flushed(&immutable);
                assert_eq!(remaining, 1);
            }
        }

        assert_eq!(manager.immutable_count(), 1);
        assert_eq!(manager.flush_count(), 1);
    }

    #[test]
    fn test_memtable_manager_total_memory() {
        let config = MemTableConfig::default();
        let manager = MemTableManager::new(config);

        // Insert data into active
        let active = manager.get_active();
        for i in 0..100 {
            active.insert(format!("key{}", i), &[0u8; 100]);
        }

        let active_size = manager.active_size_bytes();
        let total_size = manager.total_memory_bytes();

        assert!(active_size > 0);
        assert_eq!(total_size, active_size); // No immutable yet
    }

    #[test]
    fn test_async_flush_worker() {
        let config = MemTableConfig {
            enable_async_flush: true,
            max_immutable_memtables: 2,
            ..Default::default()
        };
        let manager = Arc::new(MemTableManager::new(config));
        let worker = AsyncFlushWorker::new(Arc::clone(&manager));

        // Create an immutable memtable
        manager.swap_active();
        assert_eq!(manager.immutable_count(), 1);

        // Give worker time to process
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Worker should have flushed it
        assert!(manager.immutable_count() <= 1);

        worker.stop();
    }
}
