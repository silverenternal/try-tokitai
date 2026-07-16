//! WriteEngine - handles all write-path operations for FileKV
//!
//! Responsibilities:
//! - `put()` - KV write with backpressure, coalescer, WAL, memtable
//! - `put_batch()` - Atomic batch write
//! - `delete()` - Tombstone write
//! - `flush_memtable()` - MemTable → segment file flush
//! - Write coalescer management
//! - WAL management
//! - Dictionary compression
//! - Compaction triggering (via `Weak<CompactionEngine>`)

use std::hash::Hasher;
use std::io::Write;
use std::sync::Arc;

use parking_lot::Mutex;
use tracing::{debug, warn};

use crate::compaction::CompactionManager;
use crate::core::flush::FlushTrigger;
use crate::core::segment::SegmentFile;
use crate::core::segment::{SEGMENT_MAGIC, SEGMENT_VERSION};
use crate::core::sparse_index;
use crate::core::sparse_index::{DenseIndex, SparseIndex};
use crate::core::types::FileKVStatsSnapshot;
use crate::core::wal::WalOperation;
use crate::engine::EngineState;
use crate::query::zone_map::ZoneMapEntry;

/// Result of the shared flush entry-writing logic.
/// Returned by `flush_entries_to_writer()` and consumed by both sync and async flush paths.
pub(crate) struct FlushResult {
    pub(crate) sparse_index: SparseIndex,
    pub(crate) dense_index: DenseIndex,
    pub(crate) zone_map_entries: Vec<ZoneMapEntry>,
    pub(crate) current_pos: u64,
    pub(crate) zone_map_block_count: usize,
}

/// Shared flush logic: writes entries to a segment file, builds indexes and zone maps.
/// Used by both `flush_memtable()` (sync) and `flush_memtable_async()` (async).
fn flush_entries_to_writer<W: std::io::Write>(
    writer: &mut W,
    entries: &[(String, crate::core::memtable::MemTableEntry)],
    segment_id: u64,
    block_size: u64,
    compressor: Option<&crate::compression::dictionary::DictionaryCompressor>,
) -> anyhow::Result<FlushResult> {
    let mut sparse_index = SparseIndex::new(segment_id);
    let mut dense_index = DenseIndex::with_block_size(block_size);

    let mut current_pos = 8u64;
    let mut current_block_entry_count = 0u32;
    let mut current_block_min_key: Option<String> = None;
    let mut current_block_max_key: Option<String> = None;
    let mut zone_map_entries: Vec<ZoneMapEntry> = Vec::new();
    let mut current_block_start = 8u64;
    let estimated_avg_entry_size = 100u64;
    let block_entry_threshold = if estimated_avg_entry_size > 0 {
        (block_size / estimated_avg_entry_size).max(1) as u32
    } else {
        100u32
    };

    for (key, entry) in entries {
        if let Some(value) = &entry.value {
            let key_bytes = key.as_bytes();

            let compressed_value;
            let value_bytes: &[u8] = if let Some(compressor) = compressor {
                match compressor.compress(value.as_ref()) {
                    Ok(compressed) => {
                        compressed_value = compressed;
                        compressed_value.as_slice()
                    }
                    Err(e) => {
                        warn!(
                            "Dictionary compression failed for key '{}': {}, using uncompressed value",
                            key, e
                        );
                        value.as_ref()
                    }
                }
            } else {
                value.as_ref()
            };

            let key_len = key_bytes.len() as u32;
            let compressed_value_len = value_bytes.len() as u32;

            let mut hasher = crc32c::Crc32cHasher::default();
            hasher.write(key_bytes);
            hasher.write(value_bytes);
            let checksum = hasher.finish() as u32;

            writer.write_all(&key_len.to_le_bytes())?;
            writer.write_all(key_bytes)?;
            writer.write_all(&compressed_value_len.to_le_bytes())?;
            writer.write_all(value_bytes)?;
            writer.write_all(&checksum.to_le_bytes())?;

            sparse_index.add(key.clone(), current_pos, entry.seq_num as u64);

            let block_id = dense_index.offset_to_block_id(current_pos);
            dense_index.entries.insert(
                key.clone(),
                sparse_index::DenseIndexEntry {
                    offset: current_pos,
                    key_len: key.len() as u32,
                    value_len: compressed_value_len,
                    checksum,
                    seq_num: entry.seq_num as u64,
                    block_id,
                },
            );

            current_block_entry_count += 1;
            match &mut current_block_min_key {
                None => current_block_min_key = Some(key.clone()),
                Some(min_key) => {
                    if key.as_str() < min_key.as_str() {
                        *min_key = key.clone();
                    }
                }
            }
            match &mut current_block_max_key {
                None => current_block_max_key = Some(key.clone()),
                Some(max_key) => {
                    if key.as_str() > max_key.as_str() {
                        *max_key = key.clone();
                    }
                }
            }

            current_pos += (4 + key_bytes.len() + 4 + value_bytes.len() + 4) as u64;

            if current_block_entry_count >= block_entry_threshold {
                if let (Some(min_key), Some(max_key)) = (current_block_min_key.take(), current_block_max_key.take()) {
                    let block_id = (zone_map_entries.len() + 1) as u64;
                    zone_map_entries.push(ZoneMapEntry::new(
                        block_id,
                        min_key,
                        max_key,
                        current_block_start,
                        (current_pos - current_block_start) as u32,
                        current_block_entry_count,
                    ));
                    current_block_start = current_pos;
                    current_block_entry_count = 0;
                }
            }
        }
    }

    // Finalize last block
    if let (Some(min_key), Some(max_key)) = (current_block_min_key.take(), current_block_max_key.take()) {
        let block_id = (zone_map_entries.len() + 1) as u64;
        zone_map_entries.push(ZoneMapEntry::new(
            block_id,
            min_key,
            max_key,
            current_block_start,
            (current_pos - current_block_start) as u32,
            current_block_entry_count,
        ));
    }

    let zone_map_block_count = zone_map_entries.len();
    Ok(FlushResult {
        sparse_index,
        dense_index,
        zone_map_entries,
        current_pos,
        zone_map_block_count,
    })
}

/// Write engine for KV operations
pub struct WriteEngine {
    pub state: Arc<EngineState>,
    /// Write-Ahead Log manager (wrapped in Arc for sharing with WalChannel)
    wal: Option<Arc<Mutex<crate::core::wal::WalManager>>>,
    /// OPT-007: WAL channel for async batch submission
    wal_channel: Option<Arc<crate::core::wal_channel::WalChannel>>,
    /// Write coalescer for batching rapid writes (Phase 6: always present)
    write_coalescer: Arc<crate::core::write_coalescer::WriteCoalescer>,
    /// Dictionary compressor (S2-1: shared between read and write engines)
    compressor: Option<Arc<crate::compression::dictionary::DictionaryCompressor>>,
    /// Async writer (feature-gated)
    #[cfg(feature = "async-io")]
    async_writer: Option<Arc<crate::ops::async_io::AsyncWriter>>,
    #[cfg(not(feature = "async-io"))]
    #[allow(dead_code)]
    async_writer: Option<Arc<()>>,
    /// Flush lock - serializes flush operations
    flush_lock: Mutex<()>,
    /// Flush trigger for background flush
    flush_trigger: FlushTrigger,
    /// Compaction manager reference (for triggering)
    compaction_manager: Arc<CompactionManager>,
    /// Audit logger
    audit_logger: Option<Arc<crate::ops::audit_log::AuditLogger>>,
    /// Adaptive preallocator
    adaptive_preallocator: Option<Arc<crate::ops::preallocator::AdaptivePreallocator>>,
    /// Compaction engine weak reference (to trigger compaction without cycle)
    compaction_engine: Option<std::sync::Weak<CompactionEngine>>,
}

// Forward declaration to avoid circular dependency
use crate::engine::compaction_engine::CompactionEngine;

impl WriteEngine {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        state: Arc<EngineState>,
        wal: Option<Arc<Mutex<crate::core::wal::WalManager>>>,
        wal_channel: Option<Arc<crate::core::wal_channel::WalChannel>>,
        write_coalescer: Arc<crate::core::write_coalescer::WriteCoalescer>,
        compressor: Option<Arc<crate::compression::dictionary::DictionaryCompressor>>,
        #[cfg(feature = "async-io")] async_writer: Option<Arc<crate::ops::async_io::AsyncWriter>>,
        #[cfg(not(feature = "async-io"))] async_writer: Option<Arc<()>>,
        flush_trigger: FlushTrigger,
        compaction_manager: Arc<CompactionManager>,
        audit_logger: Option<Arc<crate::ops::audit_log::AuditLogger>>,
        adaptive_preallocator: Option<Arc<crate::ops::preallocator::AdaptivePreallocator>>,
    ) -> Self {
        Self {
            state,
            wal,
            wal_channel,
            write_coalescer,
            compressor,
            async_writer,
            flush_lock: Mutex::new(()),
            flush_trigger,
            compaction_manager,
            audit_logger,
            adaptive_preallocator,
            compaction_engine: None,
        }
    }

    /// Set weak reference to CompactionEngine (called after construction to avoid cycle)
    pub fn set_compaction_engine(&mut self, engine: std::sync::Weak<CompactionEngine>) {
        self.compaction_engine = Some(engine);
    }

    /// Get reference to WAL manager (for recovery)
    pub fn wal_ref(&self) -> Option<&Arc<Mutex<crate::core::wal::WalManager>>> {
        self.wal.as_ref()
    }

    /// Record segment closed with actual size (for preallocator)
    pub fn record_segment_closed(&self, actual_size: u64) {
        if let Some(ref preallocator) = self.adaptive_preallocator {
            preallocator.record_segment_closed(actual_size);
        }
    }

    /// Get next adaptive preallocate size
    pub fn get_next_preallocate_size(&self) -> u64 {
        self.adaptive_preallocator
            .as_ref()
            .map(|p| p.next_preallocate_size())
            .unwrap_or(self.state.config.segment_preallocate_size)
    }

    /// Get adaptive preallocator statistics
    pub fn get_preallocator_stats(&self) -> Option<crate::ops::preallocator::PreallocatorStats> {
        self.adaptive_preallocator.as_ref().map(|p| p.stats())
    }

    /// Get reference to the write coalescer (for testing)
    pub fn write_coalescer(&self) -> &Arc<crate::core::write_coalescer::WriteCoalescer> {
        &self.write_coalescer
    }

    /// OPT-007: Shutdown the WAL channel (call during engine shutdown)
    pub fn shutdown_wal_channel(&self) {
        if let Some(ref wal_channel) = self.wal_channel {
            wal_channel.shutdown();
        }
    }

    /// Flush a batch of writes to WAL and memtable (Phase 6: uses batch WAL)
    /// GAP-M6 FIX: This method is now used when write_coalescer returns a ready batch.
    ///
    /// IMPORTANT: In the current put_buffered() flow, writes are already inserted
    /// into memtable and counted. This method only ensures the WAL gets a batched write.
    /// The memtable insertion here would be a duplicate, so we skip it.
    fn flush_batch_to_wal_and_memtable(
        &self,
        batch: &[crate::core::write_coalescer::BufferedWrite],
    ) -> anyhow::Result<()> {
        if batch.is_empty() {
            return Ok(());
        }

        debug!(
            "Flushing batch with {} writes to WAL (memtable already updated)",
            batch.len()
        );

        if let Some(ref wal) = self.wal {
            let mut wal_guard = wal.lock();

            // Phase 6 Task 6.5: Use batch WAL for single fsync
            let batch_entries: Vec<(String, Vec<u8>)> = batch
                .iter()
                .map(|write| (write.key.clone(), write.value.clone()))
                .collect();

            // Write all entries as a single batch
            wal_guard.log_batch(&batch_entries)?;
        }

        // NOTE: memtable insertion SKIPPED - these entries were already inserted in put_buffered()
        // when each individual put() was called. Only WAL gets batched here.

        Ok(())
    }

    /// Run compaction if needed (delegates to CompactionEngine if available)
    fn maybe_run_compaction(&self) -> anyhow::Result<()> {
        // Note: WriteEngine doesn't have FileKV reference, so it can't execute compaction directly
        // It can only signal the compaction engine to trigger compaction
        if let Some(ref engine) = self.compaction_engine {
            if let Some(engine) = engine.upgrade() {
                // Signal compaction engine to check if compaction is needed
                // The actual compaction will be executed by FileKV::run_compaction
                let segment_count = self
                    .state
                    .segment_state
                    .segment_count
                    .load(std::sync::atomic::Ordering::Relaxed);
                let total_size = self
                    .state
                    .segment_state
                    .total_size_bytes
                    .load(std::sync::atomic::Ordering::Relaxed);

                if self.state.config.compaction.async_compaction_enabled
                    && engine
                        .compaction_manager()
                        .request_compaction(segment_count, total_size)
                {
                    tracing::debug!("Compaction requested via async channel");
                    return Ok(());
                }
            }
        }

        // Fallback: direct compaction via compaction_manager
        let segment_count = self
            .state
            .segment_state
            .segment_count
            .load(std::sync::atomic::Ordering::Relaxed);
        let total_size = self
            .state
            .segment_state
            .total_size_bytes
            .load(std::sync::atomic::Ordering::Relaxed);

        if self.state.config.compaction.async_compaction_enabled
            && self.compaction_manager.request_compaction(segment_count, total_size)
        {
            return Ok(());
        }

        // Fallback to synchronous - this shouldn't happen in production
        // but is needed for tests
        Ok(())
    }

    /// Common buffered write path: WAL + memtable + coalescer + flush/compaction check.
    /// Used by both `put()` and `put_with_durability(Buffered)`.
    ///
    /// ENG-001 FIX: WAL is written BEFORE memtable insertion to ensure durability semantics.
    /// If WAL write fails, data is not inserted into memtable, avoiding inconsistency.
    ///
    /// OPT-007: When WAL channel is enabled, writes are submitted to the channel for
    /// async batch processing instead of direct WAL writes.
    fn put_buffered(&self, key: &str, value: &[u8]) -> anyhow::Result<()> {
        let value_len = value.len() as u64;

        // OPT-008: Record logical write (key + value)
        let logical_bytes = key.len() as u64 + value_len;
        self.state
            .stats_state
            .amplification_tracker
            .record_logical_write(logical_bytes);

        // OPT-007: Use WAL channel if enabled
        if let Some(ref wal_channel) = self.wal_channel {
            let mut hasher = xxhash_rust::xxh3::Xxh3::default();
            hasher.write(value);
            let hash = hasher.finish();

            // Submit to WAL channel (non-blocking)
            match wal_channel.submit(key.to_string(), value.to_vec(), hash) {
                Ok(true) => {
                    // Successfully submitted to channel.
                    // Memtable insert and stats update are deferred to the background
                    // flush thread (do_flush), which performs them only after the WAL
                    // batch is successfully persisted.
                }
                Ok(false) => {
                    // Channel full - fall back to direct WAL write
                    self.put_buffered_direct(key, value)?;
                    return Ok(());
                }
                Err(e) => {
                    // Channel error - fall back to direct WAL write
                    warn!("WAL channel error, falling back to direct write: {}", e);
                    self.put_buffered_direct(key, value)?;
                    return Ok(());
                }
            }

            // Record WAL byte estimates (actual WAL write happens in background thread)
            let wal_bytes: u64 = 16 + value_len + 100;
            self.state
                .stats_state
                .stats
                .wal_bytes_written
                .fetch_add(wal_bytes, std::sync::atomic::Ordering::Relaxed);
            self.state
                .stats_state
                .stats
                .segment_bytes_written
                .fetch_add(wal_bytes, std::sync::atomic::Ordering::Relaxed);
            // OPT-008: Record disk write for WAL
            self.state
                .stats_state
                .amplification_tracker
                .record_disk_write(wal_bytes);
            self.compaction_manager.record_user_bytes(value_len);
            self.compaction_manager.record_total_bytes(wal_bytes + value_len);

            // Check flush/compaction (note: memtable doesn't have this entry yet,
            // so flush check is based on current memtable state only)
            self.check_flush_and_compaction(value_len)?;

            // Audit log
            if let Some(ref audit_logger) = self.audit_logger {
                let _ = audit_logger.log_operation(
                    crate::ops::audit_log::AuditOperation::Put,
                    vec![key.to_string()],
                    None,
                    Some(value_len),
                    None,
                    true,
                    None,
                    crate::ops::audit_log::AuditMetadata::default(),
                );
            }

            return Ok(());
        }

        // Standard path: direct WAL write
        self.put_buffered_direct(key, value)?;

        // Step 3: Add to write coalescer; flush batch if ready (GAP-M6 fix)
        if let Some(batch) = self.write_coalescer.add(key.to_string(), value.to_vec()) {
            self.flush_batch_to_wal_and_memtable(&batch)?;
        }

        // Check if we should trigger memtable flush or compaction
        self.check_flush_and_compaction(value_len)?;

        // Audit log the put operation
        if let Some(ref audit_logger) = self.audit_logger {
            let _ = audit_logger.log_operation(
                crate::ops::audit_log::AuditOperation::Put,
                vec![key.to_string()],
                None,
                Some(value_len),
                None,
                true,
                None,
                crate::ops::audit_log::AuditMetadata::default(),
            );
        }

        Ok(())
    }

    /// Direct WAL write path (used when WAL channel is disabled or full)
    fn put_buffered_direct(&self, key: &str, value: &[u8]) -> anyhow::Result<()> {
        let value_len = value.len() as u64;

        // OPT-008: Record logical write (key + value)
        let logical_bytes = key.len() as u64 + value_len;
        self.state
            .stats_state
            .amplification_tracker
            .record_logical_write(logical_bytes);

        // Step 1: Write to WAL FIRST (ENG-001 fix: WAL before memtable)
        if let Some(ref wal) = self.wal {
            let mut hasher = xxhash_rust::xxh3::Xxh3::default();
            hasher.write(value);
            let hash = hasher.finish();

            let hash_bytes = hash.to_le_bytes();
            let len_bytes = (value.len() as u64).to_le_bytes();

            let mut payload = Vec::with_capacity(16 + value.len());
            payload.extend_from_slice(&len_bytes);
            payload.extend_from_slice(&hash_bytes);
            payload.extend_from_slice(value);

            let op = WalOperation::Add {
                session: key.to_string(),
                hash: format!("{:016X}", hash),
                layer: "segment".to_string(),
            };

            let mut wal_guard = wal.lock();
            wal_guard.log_with_payload(op, payload).map_err(|e| {
                anyhow::anyhow!("WAL write failed before memtable insertion (data not persisted): {}", e)
            })?;
            drop(wal_guard);

            let wal_bytes = 16 + value_len + 100;
            self.state
                .stats_state
                .stats
                .wal_bytes_written
                .fetch_add(wal_bytes, std::sync::atomic::Ordering::Relaxed);
            self.state
                .stats_state
                .stats
                .segment_bytes_written
                .fetch_add(wal_bytes, std::sync::atomic::Ordering::Relaxed);
            // OPT-008: Record disk write for WAL
            self.state
                .stats_state
                .amplification_tracker
                .record_disk_write(wal_bytes);
        }

        // Step 2: Insert into memtable AFTER WAL is safely written (ENG-001 fix)
        self.state.memtable_state.memtable.insert(key.to_string(), value);
        self.state
            .stats_state
            .stats
            .write_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // PERF-REGRESSION-003: memtable_size and memtable_entries are now read on-demand
        // in get_stats() instead of being stored on every write (eliminates 2 atomic stores)

        // OPT-003 FIX: Record amplification metrics
        let wal_bytes: u64 = if self.state.config.enable_wal {
            16 + value_len + 100
        } else {
            0
        };
        self.compaction_manager.record_user_bytes(value_len);
        self.compaction_manager.record_total_bytes(value_len + wal_bytes);

        Ok(())
    }

    /// Check if flush and compaction should be triggered
    fn check_flush_and_compaction(&self, _value_len: u64) -> anyhow::Result<()> {
        let should_flush = self.flush_trigger.is_requested() || self.state.memtable_state.memtable.should_flush();
        if should_flush {
            self.flush_trigger.mark_completed();
            self.flush_memtable()?;
        }

        if self.compaction_manager.record_write() {
            self.maybe_run_compaction()?;
        }

        // Amplification metrics are recorded by the write path (put_buffered_direct, put_buffered, etc.)
        // to avoid double-counting when this function is called from multiple callers.

        Ok(())
    }

    /// Write key-value pair
    ///
    /// Phase 6: Default durability is Buffered - inserts into memtable immediately,
    /// WAL write without fsync. Data is readable right after put() returns.
    pub fn put(&self, key: &str, value: &[u8]) -> anyhow::Result<()> {
        let value_len = value.len() as u64;

        // Record amplification statistics
        self.state
            .stats_state
            .stats
            .user_bytes_written
            .fetch_add(value_len, std::sync::atomic::Ordering::Relaxed);

        if self.state.memtable_state.memtable.should_apply_backpressure() {
            self.flush_memtable()?;
            if self.state.memtable_state.memtable.should_apply_backpressure() {
                return Err(anyhow::anyhow!("Backpressure: MemTable memory limit exceeded"));
            }
        }

        self.put_buffered(key, value)
    }

    /// Write key-value pair with specified durability
    ///
    /// Phase 6: Allows caller to choose between Buffered (default, high throughput)
    /// and Immediate (bypasses buffer, writes directly to WAL + MemTable)
    pub fn put_with_durability(
        &self,
        key: &str,
        value: &[u8],
        durability: crate::core::types::Durability,
    ) -> anyhow::Result<()> {
        let value_len = value.len() as u64;

        // Record amplification statistics
        self.state
            .stats_state
            .stats
            .user_bytes_written
            .fetch_add(value_len, std::sync::atomic::Ordering::Relaxed);

        if self.state.memtable_state.memtable.should_apply_backpressure() {
            self.flush_memtable()?;
            if self.state.memtable_state.memtable.should_apply_backpressure() {
                return Err(anyhow::anyhow!("Backpressure: MemTable memory limit exceeded"));
            }
        }

        match durability {
            crate::core::types::Durability::Buffered => self.put_buffered(key, value),
            crate::core::types::Durability::Immediate => {
                // Bypass WriteBuffer: write directly to WAL + MemTable
                self.put_buffered_direct(key, value)?;
                self.check_flush_and_compaction(value_len)?;

                // Audit log the put operation
                if let Some(ref audit_logger) = self.audit_logger {
                    let _ = audit_logger.log_operation(
                        crate::ops::audit_log::AuditOperation::Put,
                        vec![key.to_string()],
                        None,
                        Some(value_len),
                        None,
                        true,
                        None,
                        crate::ops::audit_log::AuditMetadata::default(),
                    );
                }

                Ok(())
            }
        }
    }

    /// P3-001: Write key-value pair with specified I/O mode
    ///
    /// Allows caller to choose between Sync (default, blocking) and Async
    /// (non-blocking via AsyncWriter) at call time.
    ///
    /// # Async Mode Requirements
    /// - Requires `async-io` feature flag
    /// - Must be called from within a Tokio runtime
    /// - Falls back to Sync if AsyncWriter is not available
    #[cfg(feature = "async-io")]
    pub async fn put_with_io_mode(
        &self,
        key: &str,
        value: &[u8],
        mode: crate::core::types::IoMode,
    ) -> anyhow::Result<()> {
        match mode {
            crate::core::types::IoMode::Sync => self.put(key, value),
            crate::core::types::IoMode::Async => {
                if self.async_writer.is_some() {
                    self.put_async(key, value).await
                } else {
                    // Fallback to sync if async writer not initialized
                    warn!("Async I/O requested but AsyncWriter not available, falling back to sync");
                    self.put(key, value)
                }
            }
        }
    }

    /// Batch write key-value pairs atomically
    ///
    /// Optimized batch write path:
    /// - Single WAL batch record (single serialization + single fsync)
    /// - Batch memtable insertion (single size calculation pass)
    /// - Reduced allocation overhead
    pub fn put_batch(&self, entries: &[(&str, &[u8])]) -> anyhow::Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        // Check backpressure before batch write
        if self.state.memtable_state.memtable.should_apply_backpressure() {
            self.flush_memtable()?;
            if self.state.memtable_state.memtable.should_apply_backpressure() {
                return Err(anyhow::anyhow!("Backpressure: MemTable memory limit exceeded"));
            }
        }

        // Step 1: Build batch data once (avoid double allocation)
        let batch_data: Vec<(String, Vec<u8>)> = entries.iter().map(|(k, v)| (k.to_string(), v.to_vec())).collect();

        // OPT-003 FIX: Calculate user bytes for amplification tracking
        let total_user_bytes: u64 = batch_data.iter().map(|(k, v)| (k.len() + v.len()) as u64).sum();

        // OPT-008: Record batch logical write
        self.state
            .stats_state
            .amplification_tracker
            .record_logical_write(total_user_bytes);

        // Step 2: Write all entries to WAL as a single batch record (atomic)
        if let Some(ref wal) = self.wal {
            let mut wal_guard = wal.lock();
            wal_guard.log_batch(&batch_data)?;
            drop(wal_guard);

            // Update WAL stats
            let total_bytes: u64 = batch_data.iter().map(|(_, v)| v.len() as u64).sum();
            let wal_overhead = 100 + 16 * entries.len() as u64; // entry overhead + checksums
            self.state
                .stats_state
                .stats
                .wal_bytes_written
                .fetch_add(total_bytes + wal_overhead, std::sync::atomic::Ordering::Relaxed);
            self.state
                .stats_state
                .stats
                .segment_bytes_written
                .fetch_add(total_bytes + wal_overhead, std::sync::atomic::Ordering::Relaxed);
            // OPT-008: Record batch disk write
            self.state
                .stats_state
                .amplification_tracker
                .record_disk_write(total_bytes + wal_overhead);
        }

        // Step 3: Batch insert into memtable (single pass)
        let (_final_size, start_seq) = self.state.memtable_state.memtable.insert_batch(&batch_data);

        // Update statistics (batched update instead of per-entry)
        self.state
            .stats_state
            .stats
            .write_count
            .fetch_add(entries.len() as u64, std::sync::atomic::Ordering::Relaxed);
        // PERF-REGRESSION-003: memtable_size and memtable_entries are now read on-demand in get_stats()

        // Trigger compaction if needed (once per batch, not per entry)
        if self.compaction_manager.record_write() {
            self.maybe_run_compaction()?;
        }

        // OPT-003 FIX: Record amplification metrics for batch writes
        let wal_overhead = 100 + 16 * entries.len() as u64;
        self.compaction_manager.record_user_bytes(total_user_bytes);
        self.compaction_manager
            .record_total_bytes(total_user_bytes + wal_overhead);

        // Audit log the batch operation
        if let Some(ref audit_logger) = self.audit_logger {
            let keys: Vec<String> = entries.iter().map(|(k, _)| k.to_string()).collect();
            let _ = audit_logger.log_operation(
                crate::ops::audit_log::AuditOperation::BatchPut { count: entries.len() },
                keys,
                None,
                None,
                None,
                true,
                None,
                crate::ops::audit_log::AuditMetadata::default(),
            );
        }

        tracing::debug!("Batch wrote {} entries, starting seq={}", entries.len(), start_seq);

        Ok(())
    }

    /// Batch delete multiple keys
    ///
    /// Optimized batch delete path:
    /// - Single WAL lock acquisition (all tombstone writes under one lock)
    /// - Batch memtable tombstone insertion (single insert_batch call)
    /// - Batch global index removal
    ///
    /// Fails fast: the first error aborts the batch and returns immediately.
    pub fn delete_batch(&self, keys: &[&str]) -> anyhow::Result<()> {
        if keys.is_empty() {
            return Ok(());
        }

        // Check backpressure before batch delete
        if self.state.memtable_state.memtable.should_apply_backpressure() {
            self.flush_memtable()?;
            if self.state.memtable_state.memtable.should_apply_backpressure() {
                return Err(anyhow::anyhow!("Backpressure: MemTable memory limit exceeded"));
            }
        }

        // Step 1: Build tombstone batch (empty value = tombstone)
        let tombstones: Vec<(String, Vec<u8>)> = keys.iter().map(|k| (k.to_string(), Vec::new())).collect();

        // OPT-008: Record batch logical write (tombstones count as writes)
        let total_key_bytes: u64 = keys.iter().map(|k| k.len() as u64).sum();
        self.state
            .stats_state
            .amplification_tracker
            .record_logical_write(total_key_bytes);

        // Step 2: Write all tombstones to WAL as a single batch record
        if let Some(ref wal) = self.wal {
            let mut wal_guard = wal.lock();
            wal_guard.log_batch(&tombstones)?;
            drop(wal_guard);

            // Update WAL stats (approximate)
            let wal_overhead: u64 = 50 + 16 * keys.len() as u64;
            self.state
                .stats_state
                .stats
                .wal_bytes_written
                .fetch_add(wal_overhead, std::sync::atomic::Ordering::Relaxed);
            self.state
                .stats_state
                .stats
                .segment_bytes_written
                .fetch_add(wal_overhead, std::sync::atomic::Ordering::Relaxed);
            // OPT-008: Record batch disk write
            self.state
                .stats_state
                .amplification_tracker
                .record_disk_write(wal_overhead);
        }

        // Step 3: Batch insert tombstones into memtable
        let (_final_size, start_seq) = self.state.memtable_state.memtable.insert_batch(&tombstones);

        // Step 4: Batch remove from global index
        for key in keys {
            self.state.global_index_state.global_index.remove(key);
        }

        // Update statistics (batched)
        self.state
            .stats_state
            .stats
            .write_count
            .fetch_add(keys.len() as u64, std::sync::atomic::Ordering::Relaxed);

        // Trigger compaction if needed (once per batch)
        if self.compaction_manager.record_write() {
            self.maybe_run_compaction()?;
        }

        // Audit log the batch delete operation
        if let Some(ref audit_logger) = self.audit_logger {
            let keys_vec: Vec<String> = keys.iter().map(|k| k.to_string()).collect();
            let _ = audit_logger.log_operation(
                crate::ops::audit_log::AuditOperation::BatchDelete { count: keys.len() },
                keys_vec,
                None,
                None,
                None,
                true,
                None,
                crate::ops::audit_log::AuditMetadata::default(),
            );
        }

        tracing::debug!("Batch deleted {} keys, starting seq={}", keys.len(), start_seq);

        Ok(())
    }

    /// Delete key (tombstone)
    pub fn delete(&self, key: &str) -> anyhow::Result<()> {
        let key_str = key.to_string();
        self.put_with_durability(key, &[], crate::core::types::Durability::Immediate)?;

        // Remove key from global index after writing tombstone
        self.state.global_index_state.global_index.remove(key);

        // Audit log the delete operation
        if let Some(ref audit_logger) = self.audit_logger {
            let _ = audit_logger.log_operation(
                crate::ops::audit_log::AuditOperation::Delete,
                vec![key_str],
                None,
                Some(0),
                None,
                true,
                None,
                crate::ops::audit_log::AuditMetadata::default(),
            );
        }

        Ok(())
    }

    /// Delete key with specified durability
    pub fn delete_with_durability(&self, key: &str, durability: crate::core::types::Durability) -> anyhow::Result<()> {
        let key_str = key.to_string();
        self.put_with_durability(key, &[], durability)?;

        // Remove key from global index after writing tombstone
        self.state.global_index_state.global_index.remove(key);

        // Audit log the delete operation
        if let Some(ref audit_logger) = self.audit_logger {
            let _ = audit_logger.log_operation(
                crate::ops::audit_log::AuditOperation::Delete,
                vec![key_str],
                None,
                Some(0),
                None,
                true,
                None,
                crate::ops::audit_log::AuditMetadata::default(),
            );
        }

        Ok(())
    }

    /// Async delete key with full async I/O
    #[cfg(feature = "async-io")]
    pub async fn delete_async(&self, key: &str) -> anyhow::Result<()> {
        self.put_async(key, &[]).await?;
        Ok(())
    }

    /// Flush MemTable to segment file
    ///
    /// OPTIMIZATION: Uses batched WAL writes and larger BufWriter (256KB) to reduce syscalls
    pub fn flush_memtable(&self) -> anyhow::Result<()> {
        let _guard = self.flush_lock.lock();

        // Phase 6: Drain write buffer before flushing memtable
        let pending = self.write_coalescer.force_flush();
        if !pending.is_empty() {
            debug!(
                "Flushing {} pending writes from write buffer before memtable flush",
                pending.len()
            );

            // OPTIMIZATION: Batch write all pending entries to WAL in single operation
            if let Some(ref wal) = self.wal {
                let mut wal_guard = wal.lock();
                let batch_entries: Vec<(String, Vec<u8>)> = pending
                    .iter()
                    .map(|write| (write.key.clone(), write.value.clone()))
                    .collect();
                // Single batch WAL write instead of individual log() calls
                wal_guard.log_batch(&batch_entries)?;
            }

            // Insert into memtable
            for write in pending {
                self.state
                    .memtable_state
                    .memtable
                    .insert(write.key.clone(), &write.value);
                self.state
                    .stats_state
                    .stats
                    .write_count
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                // PERF-REGRESSION-003: memtable_size now read on-demand in get_stats()
            }
        }

        // Get sorted entries after draining write buffer
        let entries = self.state.memtable_state.memtable.entries_sorted();
        if entries.is_empty() {
            return Ok(());
        }

        let len_before_flush = entries.len();

        let segment_id = self
            .state
            .segment_state
            .next_segment_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        // DATA-002 FIX: Use atomic write pattern
        let temp_path = self
            .state
            .config
            .segment_dir
            .join(format!(".segment_{}.log.tmp", segment_id));
        let segment_path = self
            .state
            .config
            .segment_dir
            .join(format!("segment_{}.log", segment_id));

        let file = self.state.config.fs.create_file(&temp_path)?;
        // OPTIMIZATION: Use larger BufWriter buffer (256KB) to reduce syscalls during flush
        let mut writer = std::io::BufWriter::with_capacity(256 * 1024, file);

        // Write segment header
        writer.write_all(&SEGMENT_MAGIC.to_le_bytes())?;
        writer.write_all(&SEGMENT_VERSION.to_le_bytes())?;

        let block_size = self.state.config.block_size;
        let flush_result = flush_entries_to_writer(
            &mut writer,
            &entries,
            segment_id,
            block_size,
            self.compressor.as_deref(),
        )?;

        writer.flush()?;

        // Sync and drop
        {
            let file = writer
                .into_inner()
                .map_err(|e| anyhow::anyhow!("Failed to get inner file from writer: {}", e))?;
            file.sync_all()?;
        }

        // Atomic rename
        self.state.config.fs.rename(&temp_path, &segment_path)?;
        self.state.config.fs.sync_dir(&self.state.config.segment_dir)?;

        // OPT-008: Record disk write for memtable flush (segment file)
        self.state
            .stats_state
            .amplification_tracker
            .record_disk_write(flush_result.current_pos);

        // Create segment file - memtable flushes always go to L0
        let segment = SegmentFile::create(
            self.state.config.fs.clone(),
            segment_id,
            0, // L0: memtable flush
            &segment_path,
            0,
            self.state.config.aggressive.persistent_mmap_enabled,
            self.state.config.aggressive.readahead_multiplier,
            self.state.config.aggressive.dense_index_enabled,
        )?;

        segment.update_size(flush_result.current_pos);
        segment.flush()?;

        // Update sparse_index with zone_map entries
        let zone_map_block_count = flush_result.zone_map_block_count;
        let mut sparse_index = flush_result.sparse_index;
        sparse_index.zone_map = Arc::new(flush_result.zone_map_entries);

        // Save indexes
        {
            let mut index_manager = self.state.index_state.index_manager.write();
            index_manager.add_index(segment_id, std::sync::Arc::new(sparse_index.clone()));

            // Persist dense index
            let dense_idx_path = self
                .state
                .config
                .segment_dir
                .join(format!("segment_{}.dense_idx", segment_id));
            match SegmentFile::save_dense_index(
                self.state.config.fs.as_ref(),
                &flush_result.dense_index,
                &dense_idx_path,
            ) {
                Ok(_) => {
                    tracing::debug!(segment_id, "Saved dense index to {}", dense_idx_path.display());
                }
                Err(e) => {
                    tracing::warn!(segment_id, "Failed to save dense index: {}", e);
                }
            }

            index_manager.add_dense_index(segment_id, flush_result.dense_index);
            index_manager.save_index(segment_id)?;
            // Lock released here after write
        }

        // Update segment state
        {
            let mut segments = self.state.segment_state.segments.load().as_ref().clone();
            let segment_size = flush_result.current_pos;
            segments.insert(segment_id, Arc::new(segment));
            self.state
                .segment_state
                .segment_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.state
                .segment_state
                .total_size_bytes
                .fetch_add(segment_size, std::sync::atomic::Ordering::Relaxed);
            self.state.segment_state.segments.store(Arc::new(segments));
        }

        // V0.6.0: Update global key index for flushed entries (bulk insert)
        {
            let generation = self.state.global_index_state.global_index.current_generation();
            let mut key_locations: Vec<(Arc<str>, crate::core::global_index::KeyLocation)> =
                Vec::with_capacity(entries.len());
            for (key, entry) in &entries {
                if entry.value.is_some() {
                    // Find the offset from the sparse index
                    if let Some(offset) = sparse_index.find(key) {
                        let loc = crate::core::global_index::KeyLocation {
                            segment_id,
                            offset,
                            generation,
                            value_len: entry.value.as_ref().map(|v| v.len()).unwrap_or(0),
                        };
                        key_locations.push((Arc::from(key.as_str()), loc));
                    }
                }
            }
            if !key_locations.is_empty() {
                self.state.global_index_state.global_index.bulk_upsert(key_locations);
            }
            self.state.global_index_state.global_index.increment_generation();
        }

        self.state.memtable_state.memtable.clear();
        self.state
            .stats_state
            .stats
            .flush_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.state
            .stats_state
            .stats
            .segment_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.state
            .stats_state
            .stats
            .total_size_bytes
            .fetch_add(flush_result.current_pos, std::sync::atomic::Ordering::Relaxed);
        if self.state.config.enable_bloom {
            // This will be called via the LifecycleManager in the full integration
            // For now, we skip it here and let the caller handle it
        }

        tracing::info!(
            "Flushed segment {} with {} entries, {} zone map blocks",
            segment_id,
            len_before_flush,
            zone_map_block_count
        );

        // Audit log the flush operation
        if let Some(ref audit_logger) = self.audit_logger {
            let _ = audit_logger.log_operation(
                crate::ops::audit_log::AuditOperation::Flush,
                vec![],
                None,
                Some(flush_result.current_pos),
                None,
                true,
                None,
                crate::ops::audit_log::AuditMetadata::default(),
            );
        }

        Ok(())
    }

    /// Get statistics snapshot
    pub fn get_stats(&self) -> FileKVStatsSnapshot {
        let mut snapshot = self.state.stats_state.stats.snapshot();
        // PERF-REGRESSION-003: Read memtable size/entries on-demand instead of
        // storing on every write (eliminates 2 AtomicUsize stores from hot path)
        snapshot.memtable_size = self.state.memtable_state.memtable.size_bytes();
        snapshot.memtable_entries = self.state.memtable_state.memtable.entry_count();
        snapshot
    }

    // === Async I/O Methods (feature-gated) ===

    /// Async write path with full async I/O
    ///
    /// This method uses AsyncWriter for non-blocking WAL and segment writes.
    /// Prefer this over `put()` when running in an async runtime for better throughput.
    #[cfg(feature = "async-io")]
    pub async fn put_async(&self, key: &str, value: &[u8]) -> anyhow::Result<()> {
        let value_len = value.len() as u64;

        // Record amplification statistics
        self.state
            .stats_state
            .stats
            .user_bytes_written
            .fetch_add(value_len, std::sync::atomic::Ordering::Relaxed);

        if self.state.memtable_state.memtable.should_apply_backpressure() {
            self.flush_memtable_async().await?;
            if self.state.memtable_state.memtable.should_apply_backpressure() {
                return Err(anyhow::anyhow!("Backpressure: MemTable memory limit exceeded"));
            }
        }

        self.put_buffered_async(key, value).await
    }

    /// Async buffered write path using AsyncWriter
    #[cfg(feature = "async-io")]
    async fn put_buffered_async(&self, key: &str, value: &[u8]) -> anyhow::Result<()> {
        use crate::core::wal::WalOperation;
        use bytes::Bytes;

        let value_len = value.len() as u64;

        // Step 1: Insert into memtable immediately (data is readable)
        self.state.memtable_state.memtable.insert(key.to_string(), value);
        self.state
            .stats_state
            .stats
            .write_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // PERF-REGRESSION-003: memtable_size and memtable_entries now read on-demand in get_stats()

        // Step 2: Async WAL write via AsyncWriter
        if let (Some(ref _wal), Some(ref async_writer)) = (&self.wal, &self.async_writer) {
            let mut hasher = xxhash_rust::xxh3::Xxh3::default();
            hasher.write(value);
            let hash = hasher.finish();

            let hash_bytes = hash.to_le_bytes();
            let len_bytes = (value.len() as u64).to_le_bytes();

            let mut payload = Vec::with_capacity(16 + value.len());
            payload.extend_from_slice(&len_bytes);
            payload.extend_from_slice(&hash_bytes);
            payload.extend_from_slice(value);

            let op = WalOperation::Add {
                session: key.to_string(),
                hash: format!("{:016X}", hash),
                layer: "segment".to_string(),
            };

            // Serialize WAL operation for async writer
            // CORE-001 FIX: Return error on serialization failure instead of writing empty data
            let wal_data =
                bincode::serialize(&op).map_err(|e| anyhow::anyhow!("Failed to serialize WAL operation: {}", e))?;
            let mut wal_payload = Vec::with_capacity(wal_data.len() + payload.len());
            wal_payload.extend_from_slice(&wal_data);
            wal_payload.extend_from_slice(&payload);

            // Async WAL write
            let bytes = Bytes::from(wal_payload);
            match async_writer.write_wal(bytes, false).await {
                Ok(result) if result.success => {
                    let wal_bytes = 16 + value_len + 100;
                    self.state
                        .stats_state
                        .stats
                        .wal_bytes_written
                        .fetch_add(wal_bytes, std::sync::atomic::Ordering::Relaxed);
                    self.state
                        .stats_state
                        .stats
                        .segment_bytes_written
                        .fetch_add(wal_bytes, std::sync::atomic::Ordering::Relaxed);
                }
                Ok(result) => {
                    // ERR-WAL-001 FIX: Retry via direct WAL write instead of silently dropping
                    warn!("Async WAL write failed, retrying direct write: {:?}", result.error);
                    self.put_buffered_direct(key, value)?;
                    return Ok(());
                }
                Err(e) => {
                    // ERR-WAL-001 FIX: Retry via direct WAL write instead of silently dropping
                    warn!("Async WAL write error, retrying direct write: {}", e);
                    self.put_buffered_direct(key, value)?;
                    return Ok(());
                }
            }
        } else if let Some(ref wal) = self.wal {
            // Fallback to sync WAL if async_writer not available
            let mut hasher = xxhash_rust::xxh3::Xxh3::default();
            hasher.write(value);
            let hash = hasher.finish();

            let hash_bytes = hash.to_le_bytes();
            let len_bytes = (value.len() as u64).to_le_bytes();

            let mut payload = Vec::with_capacity(16 + value.len());
            payload.extend_from_slice(&len_bytes);
            payload.extend_from_slice(&hash_bytes);
            payload.extend_from_slice(value);

            let op = WalOperation::Add {
                session: key.to_string(),
                hash: format!("{:016X}", hash),
                layer: "segment".to_string(),
            };

            let mut wal_guard = wal.lock();
            if let Err(e) = wal_guard.log_with_payload(op, payload) {
                warn!("Buffered WAL write failed (non-fatal, data in memtable): {}", e);
            }
            drop(wal_guard);

            let wal_bytes = 16 + value_len + 100;
            self.state
                .stats_state
                .stats
                .wal_bytes_written
                .fetch_add(wal_bytes, std::sync::atomic::Ordering::Relaxed);
            self.state
                .stats_state
                .stats
                .segment_bytes_written
                .fetch_add(wal_bytes, std::sync::atomic::Ordering::Relaxed);
        }

        // Step 3: Add to write coalescer; flush batch if ready
        if let Some(batch) = self.write_coalescer.add(key.to_string(), value.to_vec()) {
            self.flush_batch_to_wal_and_memtable(&batch)?;
        }

        // Check if we should trigger memtable flush or compaction
        let should_flush = self.flush_trigger.is_requested() || self.state.memtable_state.memtable.should_flush();
        if should_flush {
            self.flush_trigger.mark_completed();
            self.flush_memtable_async().await?;
        }

        if self.compaction_manager.record_write() {
            self.maybe_run_compaction()?;
        }

        Ok(())
    }

    /// Async memtable flush to segment file
    ///
    /// Uses AsyncWriter for non-blocking segment writes when available.
    /// Falls back to synchronous flush if async_writer is not available.
    #[cfg(feature = "async-io")]
    pub async fn flush_memtable_async(&self) -> anyhow::Result<()> {
        // Acquire flush lock to serialize flush operations
        let _lock = self.flush_lock.lock();

        let entries = self.state.memtable_state.memtable.entries_sorted();
        if entries.is_empty() {
            return Ok(());
        }

        let len_before_flush = entries.len();

        let segment_id = self
            .state
            .segment_state
            .next_segment_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let temp_path = self
            .state
            .config
            .segment_dir
            .join(format!(".segment_{}.log.tmp", segment_id));
        let segment_path = self
            .state
            .config
            .segment_dir
            .join(format!("segment_{}.log", segment_id));

        // Write segment to temp file (same as sync path)
        let file = self.state.config.fs.create_file(&temp_path)?;
        // OPTIMIZATION: Use larger BufWriter buffer (256KB) to reduce syscalls during async flush
        let mut writer = std::io::BufWriter::with_capacity(256 * 1024, file);

        writer.write_all(&SEGMENT_MAGIC.to_le_bytes())?;
        writer.write_all(&SEGMENT_VERSION.to_le_bytes())?;

        let block_size = self.state.config.block_size;
        let flush_result = flush_entries_to_writer(
            &mut writer,
            &entries,
            segment_id,
            block_size,
            self.compressor.as_deref(),
        )?;

        writer.flush()?;

        {
            let file = writer
                .into_inner()
                .map_err(|e| anyhow::anyhow!("Failed to get inner file from writer: {}", e))?;
            file.sync_all()?;
        }

        // Async rename via AsyncWriter if available
        if let Some(ref async_writer) = self.async_writer {
            // For rename, we use the sync bridge since it's a metadata operation
            self.state.config.fs.rename(&temp_path, &segment_path)?;
            self.state.config.fs.sync_dir(&self.state.config.segment_dir)?;

            // Async flush to ensure all writes are persisted
            async_writer.flush_all_sync()?;
        } else {
            self.state.config.fs.rename(&temp_path, &segment_path)?;
            self.state.config.fs.sync_dir(&self.state.config.segment_dir)?;
        }

        // Create segment file and update state (same as sync path)
        let segment = crate::core::segment::SegmentFile::create(
            self.state.config.fs.clone(),
            segment_id,
            0,
            &segment_path,
            0,
            self.state.config.aggressive.persistent_mmap_enabled,
            self.state.config.aggressive.readahead_multiplier,
            self.state.config.aggressive.dense_index_enabled,
        )?;

        segment.update_size(flush_result.current_pos);
        segment.flush()?;

        let zone_map_block_count = flush_result.zone_map_block_count;
        let mut sparse_index = flush_result.sparse_index;
        sparse_index.zone_map = Arc::new(flush_result.zone_map_entries);

        {
            // SPRINT-12: Update ArcSwap with new index_manager state
            let current_index = self.state.index_state.index_manager.read();
            let mut index_manager = (*current_index).clone();
            index_manager.add_index(segment_id, std::sync::Arc::new(sparse_index.clone()));

            let dense_idx_path = self
                .state
                .config
                .segment_dir
                .join(format!("segment_{}.dense_idx", segment_id));
            match crate::core::segment::SegmentFile::save_dense_index(
                self.state.config.fs.as_ref(),
                &flush_result.dense_index,
                &dense_idx_path,
            ) {
                Ok(_) => {
                    tracing::debug!(segment_id, "Saved dense index to {}", dense_idx_path.display());
                }
                Err(e) => {
                    tracing::warn!(segment_id, "Failed to save dense index: {}", e);
                }
            }

            index_manager.add_dense_index(segment_id, flush_result.dense_index);
            index_manager.save_index(segment_id)?;
            // Lock released here
        }

        // Update segment state
        {
            let mut segments = self.state.segment_state.segments.load().as_ref().clone();
            let segment_size = flush_result.current_pos;
            segments.insert(segment_id, Arc::new(segment));
            self.state
                .segment_state
                .segment_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.state
                .segment_state
                .total_size_bytes
                .fetch_add(segment_size, std::sync::atomic::Ordering::Relaxed);
            self.state.segment_state.segments.store(Arc::new(segments));
        }

        // V0.6.0: Update global key index for async flushed entries
        {
            let generation = self.state.global_index_state.global_index.current_generation();
            for (key, entry) in &entries {
                if entry.value.is_some() {
                    if let Some(offset) = sparse_index.find(key) {
                        let loc = crate::core::global_index::KeyLocation {
                            segment_id,
                            offset,
                            generation,
                            value_len: entry.value.as_ref().map(|v| v.len()).unwrap_or(0),
                        };
                        self.state
                            .global_index_state
                            .global_index
                            .insert(Arc::from(key.as_str()), loc);
                    }
                }
            }
        }

        self.state.memtable_state.memtable.clear();
        self.state
            .stats_state
            .stats
            .flush_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.state
            .stats_state
            .stats
            .segment_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.state
            .stats_state
            .stats
            .total_size_bytes
            .fetch_add(flush_result.current_pos, std::sync::atomic::Ordering::Relaxed);

        if self.state.config.enable_bloom {
            // Bloom rebuild handled by caller
        }

        tracing::info!(
            "Async flushed segment {} with {} entries, {} zone map blocks",
            segment_id,
            len_before_flush,
            zone_map_block_count
        );

        Ok(())
    }
}

// ============================================================================
// Phase 1: WriteEngineAPI trait implementation
// ============================================================================

impl crate::engine::traits::WriteEngineAPI for WriteEngine {
    fn put(&self, key: &str, value: &[u8]) -> anyhow::Result<()> {
        WriteEngine::put(self, key, value)
    }

    fn put_with_durability(
        &self,
        key: &str,
        value: &[u8],
        durability: crate::core::types::Durability,
    ) -> anyhow::Result<()> {
        WriteEngine::put_with_durability(self, key, value, durability)
    }

    fn put_batch(&self, entries: &[(&str, &[u8])]) -> anyhow::Result<()> {
        WriteEngine::put_batch(self, entries)
    }

    fn delete(&self, key: &str) -> anyhow::Result<()> {
        WriteEngine::delete(self, key)
    }

    fn flush_memtable(&self) -> anyhow::Result<()> {
        WriteEngine::flush_memtable(self)
    }

    fn wal_ref(&self) -> Option<&Arc<parking_lot::Mutex<crate::core::wal::WalManager>>> {
        WriteEngine::wal_ref(self)
    }

    fn get_stats(&self) -> crate::engine::traits::WriteStats {
        crate::engine::traits::WriteStats {
            write_count: self
                .state
                .stats_state
                .stats
                .write_count
                .load(std::sync::atomic::Ordering::Relaxed),
            // PERF-REGRESSION-003: Read memtable size/entries on-demand
            memtable_size: self.state.memtable_state.memtable.size_bytes() as u64,
            memtable_entries: self.state.memtable_state.memtable.entry_count() as u64,
            wal_bytes_written: self
                .state
                .stats_state
                .stats
                .wal_bytes_written
                .load(std::sync::atomic::Ordering::Relaxed),
            flush_count: self
                .state
                .stats_state
                .stats
                .flush_count
                .load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}
