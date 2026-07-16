//! WAL Channel - Async batch WAL submission via mpsc channel
//!
//! Implements OPT-007: Batch WAL + Async MemTable Flush
//!
//! # Architecture
//!
//! ```text
//! Thread 1 ──┐
//! Thread 2 ──┼──> WalChannel.submit() ──> mpsc channel ──> batch thread ──> WalManager.log_batch()
//! Thread 3 ──┘                                                                        │
//!                                                                                     ▼
//!                                                                  Single fsync per batch + memtable insert
//! ```
//!
//! # Thread Safety
//!
//! WalChannel is thread-safe and supports concurrent submissions from multiple threads.
//! Uses mpsc channel for submission and a dedicated background thread for batch consumption.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender, TryRecvError, TrySendError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use tracing::{debug, trace, warn};

use crate::core::memtable::MemTable;
use crate::core::types::FileKVStats;
use crate::core::wal::WalManager;

/// A single write request submitted to the WAL channel
#[derive(Debug)]
pub struct WalSubmitRequest {
    pub key: String,
    pub value: Vec<u8>,
    pub hash: u64,
    /// Optional callback to notify when the entry has been persisted
    pub notify: Option<Sender<WalNotifyResult>>,
}

/// Result of a WAL batch submission
#[derive(Debug, Clone)]
pub struct WalNotifyResult {
    pub success: bool,
    pub sequence_number: Option<u64>,
}

/// Configuration for the WAL channel batcher
#[derive(Debug, Clone)]
pub struct WalChannelConfig {
    /// Time window to collect writes before flushing (milliseconds)
    pub batch_interval_ms: u64,
    /// Maximum number of entries to collect in a single batch
    pub batch_max_entries: usize,
    /// Channel capacity (max pending submissions before backpressure)
    pub channel_capacity: usize,
    /// Enable async mode (default: true when using this module)
    pub enabled: bool,
}

impl Default for WalChannelConfig {
    fn default() -> Self {
        Self {
            batch_interval_ms: 2,
            batch_max_entries: 1000,
            channel_capacity: 10_000,
            enabled: true,
        }
    }
}

/// Statistics for the WAL channel
pub struct WalChannelStats {
    pub submissions: AtomicUsize,
    pub batches_flushed: AtomicUsize,
    pub entries_flushed: AtomicUsize,
    pub bytes_flushed: AtomicUsize,
    pub channel_drops: AtomicUsize,
}

impl Default for WalChannelStats {
    fn default() -> Self {
        Self {
            submissions: AtomicUsize::new(0),
            batches_flushed: AtomicUsize::new(0),
            entries_flushed: AtomicUsize::new(0),
            bytes_flushed: AtomicUsize::new(0),
            channel_drops: AtomicUsize::new(0),
        }
    }
}

impl WalChannelStats {
    pub fn new() -> Self {
        Self::default()
    }
}

/// WAL Channel - async batch submission to WAL
///
/// Submissions are collected into batches and flushed together with a single fsync.
pub struct WalChannel {
    /// Channel sender for submitting writes
    sender: SyncSender<WalSubmitRequest>,
    /// Channel receiver (owned by background thread)
    receiver: Arc<Mutex<Option<Receiver<WalSubmitRequest>>>>,
    /// Configuration
    config: WalChannelConfig,
    /// Signal to stop the background thread
    stop_signal: Arc<AtomicBool>,
    /// Handle to the background flush thread
    thread_handle: Mutex<Option<std::thread::JoinHandle<()>>>,
    /// Statistics
    pub stats: Arc<WalChannelStats>,
}

impl WalChannel {
    /// Create a new WalChannel and start the background flush thread
    pub fn new(config: WalChannelConfig) -> Self {
        let (tx, rx) = std::sync::mpsc::sync_channel(config.channel_capacity);
        let stop_signal = Arc::new(AtomicBool::new(false));
        let stats = Arc::new(WalChannelStats::new());

        Self {
            sender: tx,
            receiver: Arc::new(Mutex::new(Some(rx))),
            config,
            stop_signal: stop_signal.clone(),
            thread_handle: Mutex::new(None),
            stats,
        }
    }

    /// Start the background flush thread with the given WalManager
    pub fn start(&self, wal: Arc<Mutex<WalManager>>) {
        self.start_with_memtable(wal, None, None);
    }

    /// Start the background flush thread with the given WalManager, MemTable, and Stats.
    ///
    /// When memtable is provided, entries are inserted into the memtable only after
    /// the WAL batch is successfully persisted (deferred insert pattern).
    /// When stats is provided, write_count, memtable_size, and memtable_entries
    /// are updated in the background thread after each successful batch flush.
    pub fn start_with_memtable(
        &self,
        wal: Arc<Mutex<WalManager>>,
        memtable: Option<Arc<MemTable>>,
        stats: Option<Arc<FileKVStats>>,
    ) {
        let rx = {
            let mut rx_guard = self.receiver.lock();
            rx_guard
                .take()
                .expect("WalChannel::start called twice or start_with_memtable called after start")
        };

        let config = self.config.clone();
        let stop_signal = Arc::clone(&self.stop_signal);
        let stats_channel = Arc::clone(&self.stats);

        let handle = std::thread::Builder::new()
            .name("wal-channel-flush".to_string())
            .spawn(move || {
                Self::flush_loop(rx, config, stop_signal, wal, stats_channel, memtable, stats);
            })
            .expect("Failed to spawn WAL channel flush thread");

        *self.thread_handle.lock() = Some(handle);
    }

    /// Submit a write to be batched (non-blocking)
    ///
    /// Returns `Ok(true)` if submission succeeded, `Ok(false)` if channel is full (backpressure),
    /// or `Err` if the channel has been closed.
    pub fn submit(&self, key: String, value: Vec<u8>, hash: u64) -> Result<bool, String> {
        let req = WalSubmitRequest {
            key,
            value,
            hash,
            notify: None,
        };

        // Try non-blocking send
        match self.sender.try_send(req) {
            Ok(()) => {
                self.stats.submissions.fetch_add(1, Ordering::Relaxed);
                Ok(true)
            }
            Err(TrySendError::Full(_)) => {
                self.stats.channel_drops.fetch_add(1, Ordering::Relaxed);
                Ok(false)
            }
            Err(TrySendError::Disconnected(_)) => Err("WAL channel disconnected".to_string()),
        }
    }

    /// Submit a write with notification when persisted
    pub fn submit_with_notify(
        &self,
        key: String,
        value: Vec<u8>,
        hash: u64,
    ) -> Result<Receiver<WalNotifyResult>, String> {
        let (notify_tx, notify_rx) = mpsc::channel();
        let req = WalSubmitRequest {
            key,
            value,
            hash,
            notify: Some(notify_tx),
        };

        match self.sender.try_send(req) {
            Ok(()) => {
                self.stats.submissions.fetch_add(1, Ordering::Relaxed);
                Ok(notify_rx)
            }
            Err(TrySendError::Full(_)) => {
                self.stats.channel_drops.fetch_add(1, Ordering::Relaxed);
                Err("WAL channel full (backpressure)".to_string())
            }
            Err(TrySendError::Disconnected(_)) => Err("WAL channel disconnected".to_string()),
        }
    }

    /// Stop the background thread and flush any pending entries
    pub fn shutdown(&self) {
        self.stop_signal.store(true, Ordering::Release);

        if let Some(handle) = self.thread_handle.lock().take() {
            let _ = handle.join();
        }
    }

    /// Background flush loop
    fn flush_loop(
        rx: Receiver<WalSubmitRequest>,
        config: WalChannelConfig,
        stop_signal: Arc<AtomicBool>,
        wal: Arc<Mutex<WalManager>>,
        stats: Arc<WalChannelStats>,
        memtable: Option<Arc<MemTable>>,
        filekv_stats: Option<Arc<FileKVStats>>,
    ) {
        debug!(
            "WAL channel flush thread started (interval: {}ms, max_entries: {})",
            config.batch_interval_ms, config.batch_max_entries
        );

        let mut batch: Vec<WalSubmitRequest> = Vec::with_capacity(config.batch_max_entries);
        let mut last_flush = Instant::now();
        let batch_interval = Duration::from_millis(config.batch_interval_ms);

        while !stop_signal.load(Ordering::Acquire) {
            // Try to receive with timeout
            match rx.recv_timeout(batch_interval) {
                Ok(req) => {
                    batch.push(req);

                    // Drain more entries if available (up to batch_max_entries)
                    while batch.len() < config.batch_max_entries {
                        match rx.try_recv() {
                            Ok(req) => batch.push(req),
                            Err(TryRecvError::Empty) => break,
                            Err(TryRecvError::Disconnected) => {
                                stop_signal.store(true, Ordering::Release);
                                break;
                            }
                        }
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    // Timeout - flush if we have pending entries
                    let elapsed = last_flush.elapsed();
                    if !batch.is_empty() && elapsed >= batch_interval {
                        // Flush batch
                        Self::do_flush(&mut batch, &wal, &stats, &memtable, &filekv_stats);
                        last_flush = Instant::now();
                    }
                }
                Err(RecvTimeoutError::Disconnected) => {
                    stop_signal.store(true, Ordering::Release);
                }
            }

            // Check batch size threshold
            if batch.len() >= config.batch_max_entries {
                Self::do_flush(&mut batch, &wal, &stats, &memtable, &filekv_stats);
                last_flush = Instant::now();
            }

            // Periodic flush check
            if !batch.is_empty() && last_flush.elapsed() >= batch_interval * 2 {
                Self::do_flush(&mut batch, &wal, &stats, &memtable, &filekv_stats);
                last_flush = Instant::now();
            }
        }

        // Final flush of any remaining entries
        if !batch.is_empty() {
            Self::do_flush(&mut batch, &wal, &stats, &memtable, &filekv_stats);
        }

        debug!("WAL channel flush thread stopped");
    }

    /// Execute a batch flush to WAL
    ///
    /// When `memtable` is provided, entries are inserted into the memtable only after
    /// the WAL batch is successfully persisted (deferred insert pattern).
    /// When `filekv_stats` is provided, write_count/memtable_size/memtable_entries
    /// are updated in the background thread.
    fn do_flush(
        batch: &mut Vec<WalSubmitRequest>,
        wal: &Arc<Mutex<WalManager>>,
        stats: &Arc<WalChannelStats>,
        memtable: &Option<Arc<MemTable>>,
        filekv_stats: &Option<Arc<FileKVStats>>,
    ) {
        if batch.is_empty() {
            return;
        }

        let entry_count = batch.len();
        let total_bytes: usize = batch.iter().map(|r| r.value.len()).sum();

        // Collect notify senders before draining
        let notify_senders: Vec<Option<Sender<WalNotifyResult>>> = batch.iter().map(|r| r.notify.clone()).collect();

        // Build batch entries (keep copies for memtable insert after WAL succeeds)
        let batch_entries: Vec<(String, Vec<u8>)> = batch.iter().map(|r| (r.key.clone(), r.value.clone())).collect();

        // Clear the batch before writing to WAL (requests are no longer needed)
        batch.clear();

        // Write to WAL as a single batch
        let result = {
            let mut wal_guard = wal.lock();
            wal_guard.log_batch(&batch_entries)
        };

        match result {
            Ok(durability) => {
                stats.batches_flushed.fetch_add(1, Ordering::Relaxed);
                stats.entries_flushed.fetch_add(entry_count, Ordering::Relaxed);
                stats.bytes_flushed.fetch_add(total_bytes, Ordering::Relaxed);

                // Deferred memtable insert: only after WAL is persisted
                if let Some(ref mt) = memtable {
                    let (mt_size, _) = mt.insert_batch(&batch_entries);

                    // Update FileKV stats in the background thread
                    if let Some(ref fs) = filekv_stats {
                        fs.write_count.fetch_add(entry_count as u64, Ordering::Relaxed);
                        fs.memtable_size.store(mt_size, Ordering::Relaxed);
                        fs.memtable_entries.store(mt.entry_count(), Ordering::Relaxed);
                    }
                }

                // Notify senders of success
                for tx in notify_senders.into_iter().flatten() {
                    let _ = tx.send(WalNotifyResult {
                        success: true,
                        sequence_number: None,
                    });
                }

                trace!(
                    "WAL channel flushed {} entries ({} bytes), durability: {:?}",
                    entry_count,
                    total_bytes,
                    durability
                );
            }
            Err(e) => {
                warn!("WAL channel batch flush failed: {}", e);
                // Notify senders of failure
                for tx in notify_senders.into_iter().flatten() {
                    let _ = tx.send(WalNotifyResult {
                        success: false,
                        sequence_number: None,
                    });
                }
            }
        }
    }

    /// Check if the channel has been stopped
    pub fn is_stopped(&self) -> bool {
        self.stop_signal.load(Ordering::Acquire)
    }

    /// Get the current pending batch size (approximate)
    pub fn pending_count(&self) -> usize {
        // This is an approximation since we can't peek the channel
        self.stats.submissions.load(Ordering::Relaxed)
            - self.stats.entries_flushed.load(Ordering::Relaxed)
            - self.stats.channel_drops.load(Ordering::Relaxed)
    }
}

impl Drop for WalChannel {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Backpressure controller for WAL channel
///
/// Monitors the channel state and provides backpressure signals when the system
/// is overloaded.
pub struct WalBackpressureController {
    channel: Arc<WalChannel>,
    /// Maximum pending entries before applying backpressure
    max_pending: usize,
}

impl WalBackpressureController {
    pub fn new(channel: Arc<WalChannel>, max_pending: usize) -> Self {
        Self { channel, max_pending }
    }

    /// Check if backpressure should be applied
    pub fn should_apply_backpressure(&self) -> bool {
        self.channel.pending_count() >= self.max_pending
    }

    /// Get backpressure level (0.0 to 1.0+)
    pub fn pressure_level(&self) -> f64 {
        let pending = self.channel.pending_count() as f64;
        pending / self.max_pending as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    use crate::core::types::WalSyncMode;
    use crate::io::StdFs;

    fn create_test_wal_manager() -> Arc<Mutex<WalManager>> {
        let temp_dir = tempfile::tempdir().unwrap();
        let wal_dir = temp_dir.path().join("wal");
        std::fs::create_dir_all(&wal_dir).unwrap();

        let fs = Arc::new(StdFs);
        Arc::new(Mutex::new(
            WalManager::new_with_config(fs, wal_dir, true, 64 * 1024 * 1024, 10, WalSyncMode::Lazy).unwrap(),
        ))
    }

    #[test]
    fn test_wal_channel_submit_and_flush() {
        let config = WalChannelConfig {
            batch_interval_ms: 10,
            batch_max_entries: 100,
            channel_capacity: 1000,
            enabled: true,
        };
        let channel = WalChannel::new(config);
        let wal = create_test_wal_manager();

        channel.start(wal.clone());

        // Submit some entries
        for i in 0..10 {
            let result = channel.submit(format!("key{}", i), format!("value{}", i).into_bytes(), i as u64);
            assert!(result.unwrap());
        }

        // Wait for background flush
        thread::sleep(Duration::from_millis(100));

        channel.shutdown();

        // Verify stats
        assert!(channel.stats.entries_flushed.load(Ordering::Relaxed) > 0);
    }

    #[test]
    fn test_wal_channel_backpressure() {
        let config = WalChannelConfig {
            batch_interval_ms: 1000, // Long interval to ensure channel fills up
            batch_max_entries: 1000,
            channel_capacity: 5, // Small capacity for testing
            enabled: true,
        };
        let channel = WalChannel::new(config);
        let wal = create_test_wal_manager();

        channel.start(wal.clone());

        // Fill up the channel
        let mut filled_count = 0;
        for i in 0..100 {
            match channel.submit(format!("key{}", i), format!("value{}", i).into_bytes(), i as u64) {
                Ok(true) => filled_count += 1,
                Ok(false) => break, // Channel full
                Err(_) => break,
            }
        }

        // Some should have succeeded, some should have hit backpressure
        assert!(filled_count > 0);

        channel.shutdown();
    }

    #[test]
    fn test_wal_channel_concurrent_submits() {
        let config = WalChannelConfig {
            batch_interval_ms: 10,
            batch_max_entries: 1000,
            channel_capacity: 10_000,
            enabled: true,
        };
        let channel = Arc::new(WalChannel::new(config));
        let wal = create_test_wal_manager();

        channel.start(wal.clone());

        let num_threads = 4;
        let submits_per_thread = 100;
        let mut handles = Vec::new();

        for t in 0..num_threads {
            let channel_clone = Arc::clone(&channel);
            let handle = thread::spawn(move || {
                let mut success_count = 0;
                for i in 0..submits_per_thread {
                    if let Ok(true) = channel_clone.submit(
                        format!("thread_{}_key_{}", t, i),
                        format!("value_{}_{}", t, i).into_bytes(),
                        (t * 1000 + i) as u64,
                    ) {
                        success_count += 1;
                    }
                }
                success_count
            });
            handles.push(handle);
        }

        let total_success: usize = handles.into_iter().map(|h| h.join().unwrap()).sum();

        // Wait for background flush
        thread::sleep(Duration::from_millis(200));

        channel.shutdown();

        // All successful submissions should be accounted for
        let flushed = channel.stats.entries_flushed.load(Ordering::Relaxed);
        assert!(flushed <= total_success);
        assert!(flushed > 0);
    }

    #[test]
    fn test_wal_channel_stats() {
        let config = WalChannelConfig {
            batch_interval_ms: 10,
            batch_max_entries: 100,
            channel_capacity: 1000,
            enabled: true,
        };
        let channel = WalChannel::new(config);
        let wal = create_test_wal_manager();

        channel.start(wal.clone());

        assert_eq!(channel.stats.submissions.load(Ordering::Relaxed), 0);
        assert_eq!(channel.stats.batches_flushed.load(Ordering::Relaxed), 0);

        for i in 0..5 {
            let _ = channel.submit(format!("key{}", i), format!("value{}", i).into_bytes(), i as u64);
        }

        thread::sleep(Duration::from_millis(100));
        channel.shutdown();

        assert_eq!(channel.stats.submissions.load(Ordering::Relaxed), 5);
        assert!(channel.stats.batches_flushed.load(Ordering::Relaxed) >= 1);
        assert!(channel.stats.entries_flushed.load(Ordering::Relaxed) >= 1);
    }
}
