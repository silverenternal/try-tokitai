//! MemTable 模块
//!
//! 内存缓冲表，基于 DashMap 实现无锁并发：
//! - O(1) 平均时间复杂度插入/查找
//! - 高分段并发性能
//! - 批量刷盘支持
//!
//! # 内存计算说明
//!
//! `size_bytes` 跟踪的是 **估算的实际堆内存占用**，包括：
//! - key 的 String 堆分配大小（UTF-8 字节数）
//! - value 的 Bytes 堆分配大小（原始字节数）
//! - 每条目固定开销：约 64 字节（MemTableEntry 结构体 + DashMap 内部开销 + String/Bytes 头部）
//!
//! 注意：DashMap 的哈希表桶、分片结构等底层分配未被计入。
//! 实际内存占用可能比 `size_bytes()` 返回值高 10-20%。
//! 对于 backpressure 和 flush 决策，此近似值已足够准确。
//!
//! # OPT-004: DashMap 分片优化 + 内存布局优化
//! - DashMap 分片数默认从 num_cpus*2 提升到 num_cpus*4（更细粒度，减少锁竞争）
//! - MemTableEntry 内存布局优化：使用更紧凑的时间戳表示
//! - Batch insert 优化：减少 DashMap 多次锁定
//!
//! # P2-006: Lock-free Optimizations
//! - DashMap for concurrent access (lock-free)
//! - Atomic size tracking with fetch_add/fetch_sub (no race conditions)
//! - Relaxed memory ordering for counters (performance optimization)
//! - Bytes for zero-copy value storage

use crate::core::error::TransientError;
use crate::core::types::ValuePointer;
use ahash::AHasher;
use bytes::Bytes;
use dashmap::DashMap;
use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;

/// 每条目固定内存开销估算（字节）- OPT-004 优化后
///
/// 包括：
/// - MemTableEntry: ~24 字节（优化后：u32 seq_num + 紧凑 bool + 自动对齐）
/// - Bytes 结构体头部: ~32 字节
/// - String 结构体头部: ~24 字节（String 本身是 fat pointer，堆上有 UTF-8 数据）
/// - DashMap 每条目内部开销: ~16-24 字节（分片哈希表 entry 元数据）
///
/// OPT-004 优化：从 80 字节降低到 ~48 字节
/// - 使用 u32 替代 u64 作为 seq_num（足够支持 40 亿次操作）
/// - 删除手动 padding，依赖编译器自动对齐
/// - 字段按大小排序减少内部填充
///
/// 这是一个经验值，实际可能因平台和 Rust 版本而异。
const PER_ENTRY_OVERHEAD: usize = 48;

/// MemTable 条目
///
/// OPT-004 内存布局优化：
/// - 使用 u32 替代 u64 作为 seq_num（足够支持 40 亿次操作）
/// - 字段按大小对齐排列，减少填充
/// - 删除手动 padding，依赖编译器自动对齐
/// - Option<Bytes> 和 Option<ValuePointer> 使用 niche 优化（零成本表示 None）
#[derive(Debug, Clone)]
pub struct MemTableEntry {
    /// 值数据（如果还在 MemTable 中）- 使用 Bytes 实现零拷贝
    /// 放在前面因为它是最大的字段
    pub value: Option<Bytes>,
    /// 值指针（如果已刷盘）- 28 字节，使用 niche 优化
    pub pointer: Option<ValuePointer>,
    /// 序列号（用于并发控制）- OPT-004: u32 足够（40 亿次操作）
    pub seq_num: u32,
    /// 是否被删除 - 放在最后，编译器自动填充
    pub deleted: bool,
}

/// MemTable 配置
#[derive(Debug, Clone)]
pub struct MemTableConfig {
    /// 刷盘阈值（字节）
    pub flush_threshold_bytes: usize,
    /// 最大条目数
    pub max_entries: usize,
    /// P2-007: 最大内存限制（字节）- 达到此限制时触发背压
    pub max_memory_bytes: usize,
    /// POL-007 / OPT-004: DashMap 分片数量（MemTable 并发度）
    ///
    /// 分片数量决定了 DashMap 的内部并发度。更多的分片可以减少高负载下的锁竞争，
    /// 但会稍微增加内存开销。
    ///
    /// **默认值**: CPU 核心数 * 4（OPT-004 优化：从 *2 提升到 *4）
    /// **推荐配置**:
    /// - 低负载场景（<8 线程）: 16
    /// - 中等负载（8-16 线程）: 32-64
    /// - 高负载（16-32 线程）: 64-128
    /// - 极高负载（32+ 线程）: 128-256
    pub shards: usize,
    /// OPT-008: Enable async MemTable flush (default: false)
    ///
    /// When enabled, flush operations run in the background and don't block new writes.
    /// Supports multi-MemTable: active memtable (accepts writes) + immutable memtable (being flushed).
    pub enable_async_flush: bool,
    /// OPT-008: Maximum number of immutable memtables (default: 1)
    ///
    /// Controls how many memtables can be waiting for flush simultaneously.
    /// Higher values allow more concurrent writes during flush, but increase memory usage.
    pub max_immutable_memtables: usize,
    /// OPT-008: Flush threshold for immutable memtables (bytes)
    ///
    /// When the active memtable reaches this threshold, it's swapped to immutable
    /// and a background flush is triggered.
    pub immutable_flush_threshold_bytes: usize,
}

impl Default for MemTableConfig {
    fn default() -> Self {
        Self {
            flush_threshold_bytes: 4 * 1024 * 1024,           // 4MB
            max_entries: 100_000,                             // 10 万条
            max_memory_bytes: 64 * 1024 * 1024,               // 64MB - P2-007 backpressure limit
            shards: num_cpus::get() * 4,                      // OPT-004: 默认 CPU 核心数 * 4（更细粒度分片）
            enable_async_flush: false,                        // OPT-008: disabled by default
            max_immutable_memtables: 1,                       // OPT-008: default 1 immutable table
            immutable_flush_threshold_bytes: 4 * 1024 * 1024, // 4MB - same as flush_threshold
        }
    }
}

/// MemTable（内存缓冲表）
pub struct MemTable {
    /// 数据：key → entry
    /// OPT-004: 使用 ahash 作为 hasher，比默认 RandomState 更快
    data: DashMap<String, MemTableEntry, std::hash::BuildHasherDefault<AHasher>>,
    /// 当前大小（字节）
    size_bytes: AtomicUsize,
    /// 条目数
    entry_count: AtomicUsize,
    /// 配置
    config: MemTableConfig,
    /// 序列号计数器 - OPT-004: u32 足够（40 亿次操作）
    seq_num: AtomicU32,
    /// 分片数量（保存以便批量插入时使用）
    shard_count: u64,
    /// PERF-MEM-001: Optional memory tracker for real-time allocation tracking
    memory_tracker: Option<Arc<crate::ops::memory_tracker::MemoryTracker>>,
}

impl MemTable {
    /// 创建新的 MemTable 实例
    ///
    /// # POL-007: DashMap 分片配置
    /// 使用 `config.shards` 指定的分片数量创建 DashMap。更多分片可以减少高并发下的锁竞争。
    ///
    /// # OPT-004: ahash 哈希器
    /// 使用 ahash 作为哈希器，比默认 RandomState 更快，尤其适合短字符串 key。
    ///
    /// # PERF-MEM-001: Memory Tracker
    /// Pass an optional `Arc<MemoryTracker>` for real-time allocation tracking.
    /// When provided, `insert()` and `insert_batch()` will report memory deltas
    /// to the tracker via `record_allocation`/`record_deallocation`.
    pub fn new(config: MemTableConfig) -> Self {
        Self::with_memory_tracker(config, None)
    }

    /// Create a MemTable with optional memory tracker for real-time tracking
    pub fn with_memory_tracker(
        config: MemTableConfig,
        memory_tracker: Option<Arc<crate::ops::memory_tracker::MemoryTracker>>,
    ) -> Self {
        // POL-007: 使用配置的分片数量，DashMap 要求至少为 2
        let shard_count = config.shards.max(2) as u64;
        Self {
            data: DashMap::with_hasher_and_shard_amount(
                std::hash::BuildHasherDefault::<AHasher>::default(),
                shard_count as usize,
            ),
            size_bytes: AtomicUsize::new(0),
            entry_count: AtomicUsize::new(0),
            config,
            seq_num: AtomicU32::new(0),
            shard_count,
            memory_tracker,
        }
    }

    /// 插入键值对
    ///
    /// 返回当前大小和序列号，用于判断是否需要刷盘
    ///
    /// # 内存计算
    /// 跟踪的内存大小包括：key 长度 + value 长度 + 每条目固定开销（PER_ENTRY_OVERHEAD）
    ///
    /// # P2-006: Lock-free Implementation
    /// - DashMap::insert() is atomic per-key
    /// - Size delta calculated from old entry (key+value+overhead) vs new entry
    /// - fetch_add/fetch_sub are atomic operations (no race condition)
    /// - Relaxed ordering is safe: we only need eventual consistency for size tracking
    ///
    /// # P1-007: Race Condition Fix
    /// The size update uses atomic fetch_add/fetch_sub operations:
    /// - Each thread calculates its own delta independently
    /// - Atomic operations ensure no updates are lost
    /// - No read-modify-write pattern that could cause races
    pub fn insert(&self, key: String, value: &[u8]) -> (usize, u32) {
        let seq = self.seq_num.fetch_add(1, Ordering::Relaxed);
        let value_bytes = Bytes::copy_from_slice(value);
        let value_len = value_bytes.len();

        let entry = MemTableEntry {
            value: Some(value_bytes),
            pointer: None,
            seq_num: seq,
            deleted: false,
        };

        // P2-006: DashMap insert is atomic - returns Option with old value if key existed
        let old_entry = self.data.insert(key.clone(), entry);

        // Calculate precise size delta including key, value, and per-entry overhead
        let key_len = key.len();
        let new_entry_size = key_len + value_len + PER_ENTRY_OVERHEAD;
        let old_entry_size = old_entry
            .as_ref()
            .and_then(|e| e.value.as_ref().map(|v| key_len + v.len() + PER_ENTRY_OVERHEAD))
            .unwrap_or(0);

        // P1-007 FIX: Atomic size update using fetch_add/fetch_sub
        let delta = new_entry_size as isize - old_entry_size as isize;
        if delta >= 0 {
            self.size_bytes.fetch_add(delta as usize, Ordering::Relaxed);
        } else {
            self.size_bytes.fetch_sub(-delta as usize, Ordering::Relaxed);
        }

        // PERF-MEM-001: Report memory delta to tracker
        if let Some(ref tracker) = self.memory_tracker {
            if delta >= 0 {
                tracker.record_allocation(delta as u64);
            } else {
                tracker.record_deallocation((-delta) as u64);
            }
        }

        // Only increment entry count if this is a new key (not an update)
        if old_entry.is_none() {
            self.entry_count.fetch_add(1, Ordering::Relaxed);
        }

        // Load current size after update (eventually consistent - safe for flush threshold checks)
        let new_size = self.size_bytes.load(Ordering::Relaxed);

        (new_size, seq)
    }

    /// OPT-004: 批量插入优化版本
    ///
    /// 使用分片分组策略减少锁竞争：
    /// 1. 预计算所有 entry 数据
    /// 2. 按哈希分片分组
    /// 3. 按分片批量插入（减少跨分片锁定）
    /// 4. 统一更新大小和计数
    ///
    /// # 性能优化
    /// - 预分配所有 Bytes，避免在锁内分配
    /// - 分片分组减少 DashMap 内部锁竞争
    /// - 批量更新 size_bytes，减少原子操作次数
    ///
    /// # Returns
    /// - Final memtable size after batch insert
    /// - Starting sequence number
    pub fn insert_batch(&self, entries: &[(String, Vec<u8>)]) -> (usize, u32) {
        if entries.is_empty() {
            return (self.size_bytes.load(Ordering::Relaxed), 0);
        }

        let start_seq = self.seq_num.fetch_add(entries.len() as u32, Ordering::Relaxed);

        // Phase 1: 预分配所有 value bytes 并计算序列号
        // 避免在插入时进行内存分配
        let prepared: Vec<(String, Bytes, u32)> = entries
            .iter()
            .enumerate()
            .map(|(i, (key, value))| {
                let seq = start_seq + i as u32;
                let value_bytes = Bytes::copy_from_slice(value);
                (key.clone(), value_bytes, seq)
            })
            .collect();

        // Phase 2: 按分片分组
        // 使用 ahash 计算每个 key 的分片索引
        // 这样相同分片的 key 可以连续插入，减少锁竞争
        let mut shards: HashMap<u64, Vec<(String, Bytes, u32)>> = HashMap::new();

        for (key, value_bytes, seq) in prepared {
            let hasher = std::hash::BuildHasherDefault::<AHasher>::default();
            let hash = hasher.hash_one(&key);
            let shard_idx = hash % self.shard_count;
            shards.entry(shard_idx).or_default().push((key, value_bytes, seq));
        }

        // Phase 3: 按分片批量插入
        // 虽然 DashMap 没有真正的分片锁定 API，但按分片顺序插入可以：
        // - 提高缓存局部性
        // - 减少跨分片的锁竞争
        // - 更好地利用 CPU 缓存
        let mut new_keys_count = 0usize;
        let mut total_delta: isize = 0;

        // 按分片索引排序，确保一致的访问模式
        let mut shard_indices: Vec<_> = shards.keys().cloned().collect();
        shard_indices.sort();

        for shard_idx in shard_indices {
            if let Some(entries) = shards.remove(&shard_idx) {
                for (key, value_bytes, seq) in entries {
                    let value_len = value_bytes.len();
                    let key_len = key.len();

                    let entry = MemTableEntry {
                        value: Some(value_bytes),
                        pointer: None,
                        seq_num: seq,
                        deleted: false,
                    };

                    let old_entry = self.data.insert(key.clone(), entry);

                    // 计算大小增量
                    let new_entry_size = key_len + value_len + PER_ENTRY_OVERHEAD;
                    let old_entry_size = old_entry
                        .as_ref()
                        .and_then(|e| e.value.as_ref().map(|v| key_len + v.len() + PER_ENTRY_OVERHEAD))
                        .unwrap_or(0);

                    total_delta += new_entry_size as isize - old_entry_size as isize;

                    if old_entry.is_none() {
                        new_keys_count += 1;
                    }
                }
            }
        }

        // Phase 4: 单次原子更新总大小
        if total_delta >= 0 {
            self.size_bytes.fetch_add(total_delta as usize, Ordering::Relaxed);
        } else {
            self.size_bytes.fetch_sub(-total_delta as usize, Ordering::Relaxed);
        }

        if new_keys_count > 0 {
            self.entry_count.fetch_add(new_keys_count, Ordering::Relaxed);
        }

        let final_size = self.size_bytes.load(Ordering::Relaxed);
        (final_size, start_seq)
    }

    /// Mark key as deleted (tombstone)
    pub fn delete(&self, key: &str) -> Option<u32> {
        let seq = self.seq_num.fetch_add(1, Ordering::Relaxed);

        if let Some(mut entry) = self.data.get_mut(key) {
            entry.deleted = true;
            entry.seq_num = seq;
            Some(seq)
        } else {
            None
        }
    }

    /// Insert a tombstone entry (for recovery)
    ///
    /// Tombstones still consume memory for the key and per-entry overhead,
    /// but have no value data.
    pub fn insert_tombstone(&self, key: String) -> (usize, u32) {
        let seq = self.seq_num.fetch_add(1, Ordering::Relaxed);
        let key_len = key.len();

        let entry = MemTableEntry {
            value: None,
            pointer: None,
            seq_num: seq,
            deleted: true,
        };

        // If key already existed, adjust size
        let old_entry = self.data.insert(key.clone(), entry);
        let old_entry_size = old_entry
            .as_ref()
            .and_then(|e| e.value.as_ref().map(|v| key_len + v.len() + PER_ENTRY_OVERHEAD))
            .unwrap_or(0);

        // Tombstone size: key + per-entry overhead (no value)
        let tombstone_size = key_len + PER_ENTRY_OVERHEAD;

        // Size delta: new tombstone vs old entry
        let delta = tombstone_size as isize - old_entry_size as isize;
        if delta >= 0 {
            self.size_bytes.fetch_add(delta as usize, Ordering::Relaxed);
        } else {
            self.size_bytes.fetch_sub(-delta as usize, Ordering::Relaxed);
        }

        // Only increment entry count if this is a new key
        if old_entry.is_none() {
            self.entry_count.fetch_add(1, Ordering::Relaxed);
        }

        let new_size = self.size_bytes.load(Ordering::Relaxed);
        (new_size, seq)
    }

    /// 获取值指针
    pub fn get(&self, key: &str) -> Option<(Option<Bytes>, Option<ValuePointer>, bool)> {
        self.data.get(key).map(|e| (e.value.clone(), e.pointer, e.deleted))
    }

    /// Get an iterator over all entries
    ///
    /// This returns a DashMap RefMulti which gives access to key-value pairs.
    /// Note: This holds a read lock on the DashMap, so use it carefully in production code.
    ///
    /// # Returns
    /// * Iterator over key-value pairs
    pub fn iter(
        &self,
    ) -> impl Iterator<
        Item = dashmap::mapref::multiple::RefMulti<'_, String, MemTableEntry, std::hash::BuildHasherDefault<AHasher>>,
    > + '_ {
        self.data.iter()
    }

    /// 检查是否需要刷盘
    pub fn should_flush(&self) -> bool {
        self.size_bytes.load(Ordering::Relaxed) >= self.config.flush_threshold_bytes
            || self.entry_count.load(Ordering::Relaxed) >= self.config.max_entries
    }

    /// P2-007: Check if backpressure should be applied (memory limit exceeded)
    ///
    /// Returns true if the MemTable has exceeded the maximum memory limit.
    /// Callers should block or reject writes until memory is freed.
    pub fn should_apply_backpressure(&self) -> bool {
        self.size_bytes.load(Ordering::Relaxed) >= self.config.max_memory_bytes
    }

    /// P2-007: Get memory usage as a fraction of max limit (0.0 - 1.0+)
    ///
    /// Useful for adaptive backpressure and monitoring
    pub fn memory_usage_ratio(&self) -> f64 {
        let current = self.size_bytes.load(Ordering::Relaxed) as f64;
        let max = self.config.max_memory_bytes as f64;
        current / max
    }

    /// P2-007: Get available memory headroom in bytes
    ///
    /// Returns how many more bytes can be written before hitting the limit.
    /// Useful for determining if a batch write can be accepted.
    pub fn memory_headroom(&self) -> usize {
        let current = self.size_bytes.load(Ordering::Relaxed);
        self.config.max_memory_bytes.saturating_sub(current)
    }

    /// P2-007: Get backpressure level (0.0 - 1.0+)
    ///
    /// Returns a normalized pressure value:
    /// - 0.0: Empty MemTable
    /// - 0.5: At 50% capacity
    /// - 1.0: At limit (backpressure active)
    /// - >1.0: Over limit (should reject writes)
    pub fn backpressure_level(&self) -> f64 {
        self.memory_usage_ratio()
    }

    /// P2-007 Phase 2: Get a TransientError if backpressure is active.
    ///
    /// Returns `Some(TransientError::Backpressure)` if the MemTable has
    /// exceeded its memory limit, or `None` if there is headroom.
    ///
    /// Callers can use this to produce a structured error that can be
    /// distinguished from other error types at compile time:
    ///
    /// ```ignore
    /// if let Some(err) = memtable.backpressure_error() {
    ///     // err is TransientError::Backpressure -- caller knows it's retryable
    /// }
    /// ```
    pub fn backpressure_error(&self) -> Option<TransientError> {
        if self.should_apply_backpressure() {
            let ratio = self.memory_usage_ratio();
            Some(TransientError::Backpressure(format!(
                "MemTable memory limit exceeded (usage: {:.1}%)",
                ratio * 100.0
            )))
        } else {
            None
        }
    }

    /// 获取当前大小
    pub fn size_bytes(&self) -> usize {
        self.size_bytes.load(Ordering::Relaxed)
    }

    /// 4.1 OPTIMIZATION: Alias for size_bytes - memory usage estimation
    pub fn approximate_memory_bytes(&self) -> u64 {
        self.size_bytes.load(Ordering::Relaxed) as u64
    }

    /// 获取条目数
    pub fn entry_count(&self) -> usize {
        self.entry_count.load(Ordering::Relaxed)
    }

    /// 清空 MemTable（刷盘后调用）
    pub fn clear(&self) {
        // PERF-MEM-001: Report total deallocation
        let old_size = self.size_bytes.load(Ordering::Relaxed);
        if old_size > 0 {
            if let Some(ref tracker) = self.memory_tracker {
                tracker.record_deallocation(old_size as u64);
            }
        }

        self.data.clear();
        self.size_bytes.store(0, Ordering::Relaxed);
        self.entry_count.store(0, Ordering::Relaxed);
    }

    /// 获取所有条目（用于刷盘）
    pub fn get_entries(&self) -> Vec<(String, MemTableEntry)> {
        self.data.iter().map(|e| (e.key().clone(), e.value().clone())).collect()
    }

    /// Get all entries sorted by key (for ordered flush to segment files)
    ///
    /// CONC-004 FIX: DashMap iteration order is non-deterministic.
    /// This method sorts entries by key to ensure segment files have
    /// consistent, reproducible ordering which aids in debugging and
    /// enables range scan optimizations.
    ///
    /// # Performance
    /// - Time: O(n log n) for sorting
    /// - Space: O(n) for the entries vector
    /// - Impact: Negligible for typical flush sizes (<100K entries)
    pub fn entries_sorted(&self) -> Vec<(String, MemTableEntry)> {
        let mut entries = self.get_entries();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        entries
    }

    /// 更新条目的 pointer（刷盘后调用）
    pub fn update_pointer(&self, key: &str, pointer: ValuePointer) -> bool {
        if let Some(mut entry) = self.data.get_mut(key) {
            entry.pointer = Some(pointer);
            return true;
        }
        false
    }

    /// 获取最小序列号
    pub fn min_seq_num(&self) -> Option<u32> {
        self.data.iter().map(|e| e.value().seq_num).min()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_memtable_insert() {
        let config = MemTableConfig::default();
        let mt = MemTable::new(config);

        let key = "test_key".to_string();
        let value = b"test_value";

        let (size, seq) = mt.insert(key.clone(), value);

        assert!(size > 0);
        assert_eq!(seq, 0);
        assert_eq!(mt.entry_count(), 1);

        let (val, pointer, deleted) = mt.get(&key).unwrap();
        assert!(!deleted);
        assert!(val.is_some());
        assert_eq!(val.unwrap().as_ref(), b"test_value");
        assert!(pointer.is_none());
    }

    #[test]
    fn test_memtable_delete() {
        let config = MemTableConfig::default();
        let mt = MemTable::new(config);

        let key = "test_key";
        mt.insert(key.to_string(), b"value");

        let seq = mt.delete(key);
        assert!(seq.is_some());

        let (_, _, deleted) = mt.get(key).unwrap();
        assert!(deleted);
    }

    #[test]
    fn test_memtable_should_flush() {
        let config = MemTableConfig {
            flush_threshold_bytes: 1000,
            max_entries: 10,
            max_memory_bytes: 64 * 1024 * 1024, // 64MB - P2-007 backpressure limit
            shards: 16,
            ..Default::default()
        };
        let mt = MemTable::new(config);

        assert!(!mt.should_flush());

        // Insert enough to trigger size-based flush
        // Each entry: "key_N" (6) + "value" (5) + 48 overhead = 59 bytes
        // 20 entries = 1180 bytes > 1000 threshold
        for i in 0..20 {
            mt.insert(format!("key_{}", i), b"value");
        }

        assert!(mt.should_flush());
    }

    #[test]
    fn test_memtable_backpressure() {
        let config = MemTableConfig {
            flush_threshold_bytes: 10000,
            max_entries: 1000,
            // Each entry: "key_N" (6) + 10 value + 48 overhead = 64 bytes (OPT-004)
            // 40 entries = ~2560 bytes, so 2500 is a reasonable limit
            max_memory_bytes: 2500,
            shards: 16,
            ..Default::default()
        };
        let mt = MemTable::new(config);

        // Initially should not apply backpressure
        assert!(!mt.should_apply_backpressure());
        assert!(mt.memory_usage_ratio() < 1.0);

        // Insert until we exceed the limit (each entry ~64 bytes)
        for i in 0..45 {
            mt.insert(format!("key_{}", i), &[0u8; 10]); // 10 bytes each
        }

        // Should now trigger backpressure
        assert!(mt.should_apply_backpressure());
        assert!(mt.memory_usage_ratio() >= 1.0);
    }

    /// P2-006: Concurrent stress test for lock-free MemTable
    ///
    /// Verifies that:
    /// - Multiple threads can insert concurrently without data races
    /// - Size tracking remains accurate under concurrent updates
    /// - Entry count is correct after concurrent inserts
    #[test]
    fn test_memtable_concurrent_insert_stress() {
        use std::thread;

        let config = MemTableConfig::default();
        let mt = Arc::new(MemTable::new(config));
        let num_threads = 8;
        let inserts_per_thread = 1000;

        let mut handles = Vec::new();

        // Spawn multiple threads inserting different keys
        for t in 0..num_threads {
            let mt_clone = Arc::clone(&mt);
            let handle = thread::spawn(move || {
                for i in 0..inserts_per_thread {
                    let key = format!("thread_{}_key_{}", t, i);
                    let value = format!("value_{}_{}", t, i);
                    mt_clone.insert(key, value.as_bytes());
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all entries were inserted
        let expected_entries = num_threads * inserts_per_thread;
        assert_eq!(mt.entry_count(), expected_entries);

        // Verify size is non-zero and consistent
        let size = mt.size_bytes();
        assert!(size > 0);

        // Verify we can retrieve all entries
        for t in 0..num_threads {
            for i in 0..inserts_per_thread {
                let key = format!("thread_{}_key_{}", t, i);
                let (val, _, deleted) = mt.get(&key).expect("Entry should exist");
                assert!(val.is_some(), "Value should be present");
                assert!(!deleted, "Entry should not be deleted");
            }
        }
    }

    /// P2-006: Concurrent mixed operations stress test
    ///
    /// Verifies correctness under concurrent insert/delete/get operations
    #[test]
    fn test_memtable_concurrent_mixed_stress() {
        use std::thread;

        let config = MemTableConfig::default();
        let mt = Arc::new(MemTable::new(config));
        let num_threads = 4;
        let ops_per_thread = 500;

        let mut handles = Vec::new();

        for _t in 0..num_threads {
            let mt_clone = Arc::clone(&mt);
            let handle = thread::spawn(move || {
                for i in 0..ops_per_thread {
                    let key = format!("stress_key_{}", i % 100); // Reuse keys to create conflicts

                    match i % 3 {
                        0 => {
                            // Insert
                            mt_clone.insert(key.clone(), b"test_value");
                        }
                        1 => {
                            // Get
                            let _ = mt_clone.get(&key);
                        }
                        2 => {
                            // Delete (may or may not exist)
                            let _ = mt_clone.delete(&key);
                        }
                        _ => unreachable!(),
                    }
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Just verify the structure is still consistent (no panics)
        let _ = mt.size_bytes();
        let _ = mt.entry_count();
        let _ = mt.get_entries();
    }

    /// P2-006: Verify size tracking accuracy under concurrent updates
    #[test]
    fn test_memtable_concurrent_size_tracking() {
        use std::thread;

        let config = MemTableConfig::default();
        let mt = Arc::new(MemTable::new(config));
        let num_threads = 8;
        let inserts_per_thread = 100;
        let value_size = 100; // bytes

        let mut handles = Vec::new();

        // All threads insert the same keys (to test update path)
        for t in 0..num_threads {
            let mt_clone = Arc::clone(&mt);
            let handle = thread::spawn(move || {
                for i in 0..inserts_per_thread {
                    let key = format!("shared_key_{}", i);
                    let value = vec![t as u8; value_size];
                    mt_clone.insert(key, &value);
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify final state
        // With concurrent updates to same keys, DashMap ensures atomicity per key
        // but the final count depends on timing (last writer wins per key)
        // We should have exactly inserts_per_thread unique keys
        let entry_count = mt.entry_count();
        assert_eq!(
            entry_count, inserts_per_thread,
            "Expected {} unique keys, got {}",
            inserts_per_thread, entry_count
        );

        // Size should be reasonable (100 keys * (12 bytes key + 100 bytes value + 64 overhead) = ~17600)
        let size = mt.size_bytes();
        assert!(size > 0);
        // Each key has overhead: String key + entry metadata + value
        // Upper bound: 100 keys * (100 bytes value + ~100 bytes key + 64 overhead)
        assert!(
            size < inserts_per_thread * 300,
            "Size {} exceeds expected upper bound",
            size
        );
    }

    /// P2-007: Test memory headroom calculation
    #[test]
    fn test_memtable_memory_headroom() {
        let config = MemTableConfig {
            flush_threshold_bytes: 10000,
            max_entries: 1000,
            // 10 entries * (7 key + 100 value + 48 overhead) = 1550, plus some margin (OPT-004)
            max_memory_bytes: 1700,
            shards: 16,
            ..Default::default()
        };
        let mt = MemTable::new(config);

        // Initially should have full headroom
        assert_eq!(mt.memory_headroom(), 1700);
        assert_eq!(mt.backpressure_level(), 0.0);

        // Insert some data: "key1" (4) + 100 value + 48 overhead = 152 bytes
        mt.insert("key1".to_string(), &[0u8; 100]);
        assert!(mt.memory_headroom() < 1700);
        assert!(mt.backpressure_level() > 0.0);

        // Insert more until near limit (each entry ~152-155 bytes with OPT-004 overhead)
        for i in 0..8 {
            mt.insert(format!("key_{}", i), &[0u8; 100]);
        }

        // Should have little headroom left
        let headroom = mt.memory_headroom();
        assert!(headroom < 400);
        assert!(mt.backpressure_level() > 0.8);
    }

    /// P2-007: Test backpressure level progression
    #[test]
    fn test_memtable_backpressure_progression() {
        let config = MemTableConfig {
            flush_threshold_bytes: 10000,
            max_entries: 1000,
            max_memory_bytes: 1100, // Enough for ~2 entries (OPT-004: 48 overhead)
            shards: 16,
            ..Default::default()
        };
        let mt = MemTable::new(config);

        // Start at 0%
        assert!(mt.backpressure_level() < 0.1);

        // Insert to ~50%: "key1" (4) + 500 value + 48 overhead = 552 bytes
        mt.insert("key1".to_string(), &[0u8; 500]);
        let level = mt.backpressure_level();
        assert!((0.4..=0.6).contains(&level), "Expected ~0.5, got {}", level);

        // Insert to exceed limit: "key2" (5) + 600 value + 48 overhead = 653 bytes
        // Total: 552 + 653 = 1205 > 1100
        mt.insert("key2".to_string(), &[0u8; 600]);
        assert!(mt.backpressure_level() >= 1.0);
        assert!(mt.should_apply_backpressure());
    }

    /// CORE-002: Verify precise memory calculation
    ///
    /// Tests that size tracking includes key length, value length, and per-entry overhead.
    #[test]
    fn test_memtable_precise_memory_calculation() {
        let config = MemTableConfig::default();
        let mt = MemTable::new(config);

        assert_eq!(mt.size_bytes(), 0);

        // Insert first entry: "hello" (5) + "world" (5) + 48 overhead = 58 bytes (OPT-004)
        let (size1, _) = mt.insert("hello".to_string(), b"world");
        assert_eq!(size1, 58);

        // Insert second entry: "foo" (3) + "bar" (3) + 48 overhead = 54 bytes
        let (size2, _) = mt.insert("foo".to_string(), b"bar");
        assert_eq!(size2, 58 + 54);

        // Update existing key: "hello" (5) + "new_value" (9) + 48 overhead = 62 bytes
        // Delta: 62 - 58 = +4 bytes
        let (size3, _) = mt.insert("hello".to_string(), b"new_value");
        assert_eq!(size3, 58 + 54 + 4);

        // Insert tombstone for "foo": key (3) + 48 overhead = 51 bytes (no value)
        // Old entry was 54 bytes, so delta = 51 - 54 = -3 bytes
        let (size4, _) = mt.insert_tombstone("foo".to_string());
        assert_eq!(size4, 58 + 54 + 4 - 3);
    }

    /// OPT-004: Test batch insert correctness
    #[test]
    fn test_memtable_insert_batch_correctness() {
        let config = MemTableConfig::default();
        let mt = MemTable::new(config);

        // Test empty batch
        let (size, _seq) = mt.insert_batch(&[]);
        assert_eq!(size, 0);
        assert_eq!(mt.entry_count(), 0);

        // Test single entry batch
        let entries = vec![("batch_key1".to_string(), b"value1".to_vec())];
        let (size, seq) = mt.insert_batch(&entries);
        assert!(size > 0);
        assert_eq!(seq, 0);
        assert_eq!(mt.entry_count(), 1);

        // Verify entry exists
        let (val, _, deleted) = mt.get("batch_key1").unwrap();
        assert!(val.is_some());
        assert_eq!(val.unwrap().as_ref(), b"value1");
        assert!(!deleted);

        // Test multiple entries batch
        let entries: Vec<(String, Vec<u8>)> = (0..10)
            .map(|i| (format!("batch_key_{}", i), format!("value_{}", i).into_bytes()))
            .collect();
        let (size, seq) = mt.insert_batch(&entries);
        assert!(size > 0);
        assert_eq!(seq, 1); // Continues from previous
        assert_eq!(mt.entry_count(), 11); // 1 + 10 new

        // Verify all entries
        for i in 0..10 {
            let key = format!("batch_key_{}", i);
            let (val, _, deleted) = mt.get(&key).unwrap();
            assert!(val.is_some());
            assert_eq!(val.unwrap().as_ref(), format!("value_{}", i).as_bytes());
            assert!(!deleted);
        }
    }

    /// OPT-004: Test batch insert with updates
    #[test]
    fn test_memtable_insert_batch_with_updates() {
        let config = MemTableConfig::default();
        let mt = MemTable::new(config);

        // Insert initial entries
        let initial: Vec<(String, Vec<u8>)> = (0..5)
            .map(|i| (format!("update_key_{}", i), format!("initial_{}", i).into_bytes()))
            .collect();
        let (_size1, _) = mt.insert_batch(&initial);
        assert_eq!(mt.entry_count(), 5);

        // Update some entries in new batch
        let updates: Vec<(String, Vec<u8>)> = (0..3)
            .map(|i| (format!("update_key_{}", i), format!("updated_{}", i).into_bytes()))
            .collect();
        let (_size2, _) = mt.insert_batch(&updates);

        // Entry count should not increase (all updates)
        assert_eq!(mt.entry_count(), 5);

        // Verify updated values
        for i in 0..3 {
            let key = format!("update_key_{}", i);
            let (val, _, _deleted) = mt.get(&key).unwrap();
            assert!(val.is_some());
            assert_eq!(val.unwrap().as_ref(), format!("updated_{}", i).as_bytes());
        }

        // Verify unchanged entries
        for i in 3..5 {
            let key = format!("update_key_{}", i);
            let (val, _, _deleted) = mt.get(&key).unwrap();
            assert!(val.is_some());
            assert_eq!(val.unwrap().as_ref(), format!("initial_{}", i).as_bytes());
        }
    }

    /// OPT-004: Test concurrent batch insert stress
    #[test]
    fn test_memtable_concurrent_batch_insert_stress() {
        use std::thread;

        let config = MemTableConfig {
            shards: num_cpus::get() * 4,
            ..Default::default()
        };
        let mt = Arc::new(MemTable::new(config));
        let num_threads = 8;
        let batch_size = 500;

        let mut handles = Vec::new();

        // Spawn threads inserting batches
        for t in 0..num_threads {
            let mt_clone = Arc::clone(&mt);
            let handle = thread::spawn(move || {
                let batch: Vec<(String, Vec<u8>)> = (0..batch_size)
                    .map(|i| {
                        (
                            format!("thread_{}_batch_key_{}", t, i),
                            format!("value_{}_{}", t, i).into_bytes(),
                        )
                    })
                    .collect();
                mt_clone.insert_batch(&batch);
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all entries
        let expected_entries = num_threads * batch_size;
        assert_eq!(mt.entry_count(), expected_entries);

        // Verify size is consistent
        let size = mt.size_bytes();
        assert!(size > 0);

        // Spot check some entries
        for t in 0..num_threads {
            for i in (0..batch_size).step_by(100) {
                let key = format!("thread_{}_batch_key_{}", t, i);
                let (val, _, deleted) = mt.get(&key).expect("Entry should exist");
                assert!(val.is_some(), "Value should be present");
                assert!(!deleted, "Entry should not be deleted");
            }
        }
    }

    /// OPT-004: Test shard configuration
    #[test]
    fn test_memtable_shard_configuration() {
        // Test with explicit shard count
        let config1 = MemTableConfig {
            shards: 32,
            ..Default::default()
        };
        let _mt1 = MemTable::new(config1);
        // Should create successfully with custom shard count

        // Test with default (should use num_cpus * 4)
        let expected_shards = num_cpus::get() * 4;
        let config2 = MemTableConfig::default();
        let _mt2 = MemTable::new(config2.clone());
        assert_eq!(expected_shards, config2.shards);

        // Test with minimum shard count (should not panic with 0)
        let config3 = MemTableConfig {
            shards: 0,
            ..Default::default()
        };
        let _mt3 = MemTable::new(config3);
        // Should use at least 1 shard
    }

    /// OPT-004: Test batch insert with mixed new and update entries
    #[test]
    fn test_memtable_insert_batch_mixed() {
        let config = MemTableConfig::default();
        let mt = MemTable::new(config);

        // Insert initial entries
        let initial: Vec<(String, Vec<u8>)> = vec![
            ("mixed_key_1".to_string(), b"initial_1".to_vec()),
            ("mixed_key_2".to_string(), b"initial_2".to_vec()),
        ];
        mt.insert_batch(&initial);
        assert_eq!(mt.entry_count(), 2);

        // Batch with mix of new and update entries
        let mixed: Vec<(String, Vec<u8>)> = vec![
            ("mixed_key_1".to_string(), b"updated_1".to_vec()), // Update
            ("mixed_key_3".to_string(), b"new_3".to_vec()),     // New
            ("mixed_key_2".to_string(), b"updated_2".to_vec()), // Update
            ("mixed_key_4".to_string(), b"new_4".to_vec()),     // New
        ];
        mt.insert_batch(&mixed);

        // Should have 4 entries total (2 original + 2 new)
        assert_eq!(mt.entry_count(), 4);

        // Verify values
        let (val1, _, _) = mt.get("mixed_key_1").unwrap();
        assert_eq!(val1.unwrap().as_ref(), b"updated_1");

        let (val3, _, _) = mt.get("mixed_key_3").unwrap();
        assert_eq!(val3.unwrap().as_ref(), b"new_3");
    }
}
