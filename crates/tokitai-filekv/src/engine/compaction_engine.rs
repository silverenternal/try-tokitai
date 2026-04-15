//! CompactionEngine - handles all compaction operations for FileKV
//!
//! Responsibilities:
//! - `run_compaction()` - Synchronous compaction
//! - `maybe_run_compaction()` - Decide and trigger compaction
//! - `start_background_compaction()` - Spawn background compaction thread
//! - Adaptive segment preallocation
//! - Leveled/size-tiered compaction strategy selection

use std::sync::Arc;
use std::sync::mpsc;
use std::sync::Weak;
use parking_lot::Mutex;

use crate::engine::EngineState;
use crate::query::zone_map::ZoneMapEntry;
use crate::compaction::{
    CompactionConfig, CompactionManager, CompactionRequest, CompactionStats,
};

/// Compaction engine for merging and cleaning up segments
pub struct CompactionEngine {
    pub state: Arc<EngineState>,
    /// Compaction manager for scheduling
    compaction_manager: parking_lot::Mutex<CompactionManager>,
    /// Adaptive segment pre-allocator
    adaptive_preallocator: Option<Arc<crate::ops::preallocator::AdaptivePreallocator>>,
    /// Background compaction thread handles (wrapped in Mutex for interior mutability)
    /// OPT-003: Now supports multiple threads for parallel compaction
    thread_handles: Mutex<Vec<std::thread::JoinHandle<()>>>,
    /// Channel sender for requesting async compaction (created here, forwarded to CompactionManager's tx).
    /// Kept alive to ensure the channel stays open as long as CompactionEngine exists.
    #[allow(dead_code)]
    tx: Option<mpsc::Sender<CompactionRequest>>,
    /// Channel receiver for the background compaction thread (wrapped in Mutex for interior mutability)
    rx: Mutex<Option<mpsc::Receiver<CompactionRequest>>>,
    /// Weak reference to FileKV facade for executing actual compaction.
    /// This avoids circular references while allowing access to FileKV methods.
    /// Wrapped in Mutex for interior mutability since CompactionEngine is behind Arc.
    kv_weak: Mutex<Option<Weak<crate::FileKV>>>,
}

impl CompactionEngine {
    pub fn new(
        state: Arc<EngineState>,
        compaction_config: CompactionConfig,
        adaptive_preallocator: Option<Arc<crate::ops::preallocator::AdaptivePreallocator>>,
    ) -> Self {
        let (tx, rx) = if compaction_config.async_compaction_enabled {
            let (tx, rx) = mpsc::channel();
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };

        // Create CompactionManager with the tx for request_compaction() forwarding
        let mut compaction_manager = CompactionManager::new(compaction_config);
        // Override the manager's tx with our channel's tx
        compaction_manager.tx = tx.clone();

        Self {
            state,
            compaction_manager: parking_lot::Mutex::new(compaction_manager),
            adaptive_preallocator,
            thread_handles: Mutex::new(Vec::new()),
            tx,
            rx: Mutex::new(rx),
            kv_weak: Mutex::new(None),
        }
    }

    /// Set weak reference to FileKV facade for executing actual compaction.
    /// This should be called by FileKV after the FileKV instance is constructed.
    pub fn set_filekv_ref(&self, kv_weak: Weak<crate::FileKV>) {
        *self.kv_weak.lock() = Some(kv_weak);
    }

    /// Get compaction manager reference
    pub fn compaction_manager(&self) -> &parking_lot::Mutex<CompactionManager> {
        &self.compaction_manager
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

    /// Start background compaction threads
    ///
    /// OPT-003: Now spawns multiple threads based on max_background_compaction_threads config.
    /// Threads share the same channel receiver using a sync channel pattern for work distribution.
    pub fn start_background_compaction(&self, _state: Arc<EngineState>) -> anyhow::Result<()> {
        if !self.state.config.compaction.async_compaction_enabled {
            tracing::debug!("Async compaction disabled, not starting background threads");
            return Ok(());
        }

        let rx = match self.rx.lock().take() {
            Some(rx) => rx,
            None => {
                tracing::debug!("Compaction receiver not available (already consumed or async disabled)");
                return Ok(());
            }
        };

        // OPT-003: Determine number of compaction threads
        let num_threads = self.state.config.compaction.max_background_compaction_threads.max(1);
        tracing::info!(
            "Starting {} background compaction threads (max_background_compaction_threads={})",
            num_threads,
            self.state.config.compaction.max_background_compaction_threads
        );

        // OPT-003 FIX: Use Arc<Mutex<Receiver>> to share receiver across threads
        let shared_rx = Arc::new(std::sync::Mutex::new(rx));
        let kv_weak = self.kv_weak.lock().clone();

        let mut handles = Vec::with_capacity(num_threads);
        for thread_idx in 0..num_threads {
            let rx_clone = Arc::clone(&shared_rx);
            let kv_weak_clone = kv_weak.clone();

            let handle = std::thread::Builder::new()
                .name(format!("filekv-compaction-{}", thread_idx))
                .spawn(move || {
                    loop {
                        let req = {
                            let lock = rx_clone.lock().unwrap();
                            lock.recv()
                        };

                        match req {
                            Ok(req) => {
                                if let Some(kv) = kv_weak_clone.as_ref().and_then(|w| w.upgrade()) {
                                    tracing::info!(
                                        "[Async-{}] Background compaction triggered: {} segments, {} bytes total",
                                        thread_idx, req.segment_count, req.total_size_bytes
                                    );

                                    match crate::compaction::execute_compaction(&*kv, &req) {
                                        Ok(stats) => {
                                            tracing::info!(
                                                "[Async-{}] Compaction completed: merged {} segments, compacted {} bytes, removed {} entries, cleaned {} tombstones",
                                                thread_idx, stats.segments_merged, stats.bytes_compacted,
                                                stats.entries_removed, stats.tombstones_cleaned
                                            );
                                        }
                                        Err(e) => {
                                            tracing::error!("[Async-{}] Compaction failed: {}", thread_idx, e);
                                        }
                                    }
                                } else {
                                    tracing::info!("[Async-{}] Engine dropped, exiting compaction thread", thread_idx);
                                    break;
                                }
                            }
                            Err(_) => {
                                tracing::info!("[Async-{}] Compaction thread exiting (channel closed)", thread_idx);
                                break;
                            }
                        }
                    }
                })?;

            handles.push(handle);
        }

        *self.thread_handles.lock() = handles;
        tracing::info!("{} background compaction threads started successfully", num_threads);
        Ok(())
    }

    /// Execute compaction - delegates to the actual compaction logic
    ///
    /// This method requires access to the full FileKV facade because the compaction
    /// logic in compaction.rs uses FileKV methods. The caller (FileKV::run_compaction)
    /// should provide a callback that executes the actual compaction.
    pub fn run_compaction<F>(&self, executor: F) -> anyhow::Result<CompactionStats>
    where
        F: FnOnce(&parking_lot::Mutex<CompactionManager>) -> anyhow::Result<CompactionStats>,
    {
        // Check if compaction is actually needed before executing
        let segment_count = self.state.segment_state.segment_count.load(std::sync::atomic::Ordering::Relaxed);
        let compaction_manager = &self.compaction_manager;

        if segment_count < compaction_manager.lock().config().min_segments {
            tracing::debug!(
                "Skipping compaction: {} segments < min_segments threshold ({})",
                segment_count,
                compaction_manager.lock().config().min_segments
            );
            return Ok(CompactionStats::default());
        }

        // Execute the actual compaction logic provided by caller
        executor(compaction_manager)
    }

    /// Run compaction if needed
    ///
    /// GAP-C2 FIX: This method now actually executes compaction when kv_weak is available.
    /// OPT-003: Also checks write amplification factor and triggers compaction if WA > threshold.
    pub fn maybe_run_compaction(&self) -> anyhow::Result<()> {
        let segment_count = self.state.segment_state.segment_count.load(std::sync::atomic::Ordering::Relaxed);
        let total_size = self.state.segment_state.total_size_bytes.load(std::sync::atomic::Ordering::Relaxed);

        let compaction_mgr = self.compaction_manager.lock();

        // OPT-003: Check if compaction should be triggered due to high write amplification
        if compaction_mgr.should_compact_by_amplification() {
            let wa = compaction_mgr.write_amplification_factor();
            tracing::info!(
                "Write amplification trigger: WA = {:.2}x > {:.2}x threshold, forcing compaction",
                wa, compaction_mgr.config().write_amplification_threshold
            );
            drop(compaction_mgr);
            // Proceed to trigger compaction below
        } else if !compaction_mgr.request_compaction(segment_count, total_size) {
            // No async compaction requested, release lock
            drop(compaction_mgr);
            return Ok(());
        } else {
            // Async compaction requested, release lock and return
            tracing::debug!(
                "Compaction request sent to background thread: {} segments",
                segment_count
            );
            return Ok(());
        }

        // Try async compaction first
        if self.state.config.compaction.async_compaction_enabled
            && self.compaction_manager.lock().request_compaction(segment_count, total_size) {
                tracing::debug!(
                    "Compaction request sent to background thread: {} segments",
                    segment_count
                );
                return Ok(());
            }

        // Fallback to synchronous - execute actual compaction if we have FileKV reference
        if let Some(kv) = self.kv_weak.lock().as_ref().and_then(|w| w.upgrade()) {
            tracing::debug!("Falling back to synchronous compaction");
            let req = crate::compaction::CompactionRequest {
                segment_count,
                total_size_bytes: total_size,
                target_level: None,
            };
            let stats = crate::compaction::execute_compaction(&*kv, &req)?;
            // OPT-003: Reset amplification counters after successful compaction
            if stats.bytes_compacted > 0 {
                self.compaction_manager.lock().reset_amplification_counters();
            }
            return Ok(());
        }

        // No FileKV reference available - log and return
        tracing::debug!("Synchronous compaction requested but FileKV reference not available");
        Ok(())
    }

    /// Record a write and potentially trigger compaction
    pub fn record_write(&self) -> bool {
        self.compaction_manager.lock().record_write()
    }
}

// ============================================================================
// Phase 1: CompactionEngineAPI trait implementation
// ============================================================================

impl crate::engine::traits::CompactionEngineAPI for CompactionEngine {
    fn maybe_run_compaction(&self) -> anyhow::Result<()> {
        CompactionEngine::maybe_run_compaction(self)
    }

    fn run_compaction(&self, level: Option<u8>) -> anyhow::Result<crate::compaction::CompactionStats> {
        // ENG-002 FIX: Support level-specific compaction.
        // When level is None, run compaction on all levels (default behavior).
        // When level is Some, only compact segments at that specific level.
        let segment_count = self.state.segment_state.segment_count.load(std::sync::atomic::Ordering::Relaxed);
        let total_size: u64 = self.state.segment_state.total_size_bytes.load(std::sync::atomic::Ordering::Relaxed);

        // Try to execute compaction via FileKV reference
        if let Some(kv) = self.kv_weak.lock().as_ref().and_then(|w| w.upgrade()) {
            let req = crate::compaction::CompactionRequest {
                segment_count,
                total_size_bytes: total_size,
                target_level: level,
            };
            return crate::compaction::execute_compaction(&*kv, &req);
        }

        // No FileKV reference available - return an error instead of silent empty stats
        Err(anyhow::anyhow!(
            "Cannot run compaction: FileKV reference not available. \
             Ensure set_filekv_ref() was called after CompactionEngine construction."
        ))
    }

    fn start_background_compaction(&self) -> anyhow::Result<()> {
        // start_background_compaction requires EngineState, but we use config from state
        CompactionEngine::start_background_compaction(self, self.state.clone())
    }

    fn record_write(&self) -> bool {
        CompactionEngine::record_write(self)
    }

    fn get_stats(&self) -> crate::compaction::CompactionStats {
        // Return compaction manager stats
        self.compaction_manager.lock().stats()
    }

    fn compaction_manager(&self) -> &parking_lot::Mutex<crate::compaction::CompactionManager> {
        CompactionEngine::compaction_manager(self)
    }
}

/// Helper to finalize a block during compaction
pub fn finalize_block(
    zone_map_entries: &mut Vec<ZoneMapEntry>,
    current_block_min_key: &mut Option<String>,
    current_block_max_key: &mut Option<String>,
    current_block_start: &mut u64,
    current_block_entry_count: &mut u32,
    current_pos: u64,
) {
    if let (Some(min_key), Some(max_key)) = (current_block_min_key.take(), current_block_max_key.take()) {
        let block_id = (zone_map_entries.len() + 1) as u64;
        zone_map_entries.push(ZoneMapEntry::new(
            block_id,
            min_key,
            max_key,
            *current_block_start,
            (current_pos - *current_block_start) as u32,
            *current_block_entry_count,
        ));
        *current_block_start = current_pos;
        *current_block_entry_count = 0;
    }
}
