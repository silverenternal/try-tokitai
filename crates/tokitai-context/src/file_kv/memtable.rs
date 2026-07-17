//! MemTable 模块
//!
//! 内存缓冲表，基于 DashMap 实现无锁并发：
//! - O(1) 平均时间复杂度插入/查找
//! - 高分段并发性能
//! - 批量刷盘支持
//!
//! # P2-006: Lock-free Optimizations
//! - DashMap for concurrent access (lock-free)
//! - Atomic size tracking with fetch_add/fetch_sub (no race conditions)
//! - Relaxed memory ordering for counters (performance optimization)
//! - Bytes for zero-copy value storage

use std::sync::atomic::{AtomicUsize, AtomicU64, Ordering};
use bytes::Bytes;
use dashmap::DashMap;
use crate::file_kv::ValuePointer;

/// MemTable 条目
#[derive(Debug, Clone)]
pub struct MemTableEntry {
    /// 值数据（如果还在 MemTable 中）- 使用 Bytes 实现零拷贝
    pub value: Option<Bytes>,
    /// 值指针（如果已刷盘）
    pub pointer: Option<ValuePointer>,
    /// 序列号（用于并发控制）
    pub seq_num: u64,
    /// 是否被删除
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
}

impl Default for MemTableConfig {
    fn default() -> Self {
        Self {
            flush_threshold_bytes: 4 * 1024 * 1024, // 4MB
            max_entries: 100_000,                   // 10 万条
            max_memory_bytes: 64 * 1024 * 1024,     // 64MB - P2-007 backpressure limit
        }
    }
}

/// MemTable（内存缓冲表）
pub struct MemTable {
    /// 数据：key → entry
    data: DashMap<String, MemTableEntry>,
    /// 当前大小（字节）
    size_bytes: AtomicUsize,
    /// 条目数
    entry_count: AtomicUsize,
    /// 配置
    config: MemTableConfig,
    /// 序列号计数器
    seq_num: AtomicU64,
}

impl MemTable {
    pub fn new(config: MemTableConfig) -> Self {
        Self {
            data: DashMap::new(),
            size_bytes: AtomicUsize::new(0),
            entry_count: AtomicUsize::new(0),
            config,
            seq_num: AtomicU64::new(0),
        }
    }

    /// 插入键值对
    ///
    /// 返回当前大小和序列号，用于判断是否需要刷盘
    ///
    /// # P2-006: Lock-free Implementation
    /// - DashMap::insert() is atomic per-key
    /// - Size delta calculated from old value (if exists) vs new value
    /// - fetch_add/fetch_sub are atomic operations (no race condition)
    /// - Relaxed ordering is safe: we only need eventual consistency for size tracking
    ///
    /// # P1-007: Race Condition Fix
    /// The size update uses atomic fetch_add/fetch_sub operations:
    /// - Each thread calculates its own delta independently
    /// - Atomic operations ensure no updates are lost
    /// - No read-modify-write pattern that could cause races
    pub fn insert(&self, key: String, value: &[u8]) -> (usize, u64) {
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
        // This gives us the exact size delta for this specific update
        let old_entry = self.data.insert(key.clone(), entry);
        let old_size = old_entry
            .as_ref()
            .and_then(|e| e.value.as_ref().map(|v| v.len()))
            .unwrap_or(0);

        // P1-007 FIX: Atomic size update using fetch_add/fetch_sub
        // Each thread calculates its own delta, then atomically updates the counter
        // No race condition: fetch_add(n) always adds exactly n, regardless of other threads
        let delta = value_len as isize - old_size as isize;
        if delta >= 0 {
            self.size_bytes.fetch_add(delta as usize, Ordering::Relaxed);
        } else {
            self.size_bytes.fetch_sub(-delta as usize, Ordering::Relaxed);
        }

        // Only increment entry count if this is a new key (not an update)
        if old_entry.is_none() {
            self.entry_count.fetch_add(1, Ordering::Relaxed);
        }

        // Load current size after update (eventually consistent - safe for flush threshold checks)
        let new_size = self.size_bytes.load(Ordering::Relaxed);

        (new_size, seq)
    }

    /// 标记删除
    pub fn delete(&self, key: &str) -> Option<u64> {
        let seq = self.seq_num.fetch_add(1, Ordering::Relaxed);

        if let Some(mut entry) = self.data.get_mut(key) {
            entry.deleted = true;
            entry.seq_num = seq;
            Some(seq)
        } else {
            None
        }
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
    pub fn iter(&self) -> impl Iterator<Item = dashmap::mapref::multiple::RefMulti<'_, String, MemTableEntry>> + '_ {
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

    /// 获取当前大小
    pub fn size_bytes(&self) -> usize {
        self.size_bytes.load(Ordering::Relaxed)
    }

    /// 获取条目数
    pub fn entry_count(&self) -> usize {
        self.entry_count.load(Ordering::Relaxed)
    }

    /// 清空 MemTable（刷盘后调用）
    pub fn clear(&self) {
        self.data.clear();
        self.size_bytes.store(0, Ordering::Relaxed);
        self.entry_count.store(0, Ordering::Relaxed);
    }

    /// 获取所有条目（用于刷盘）
    pub fn get_entries(&self) -> Vec<(String, MemTableEntry)> {
        self.data.iter().map(|e| (e.key().clone(), e.value().clone())).collect()
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
    pub fn min_seq_num(&self) -> Option<u64> {
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
        };
        let mt = MemTable::new(config);

        assert!(!mt.should_flush());

        // Insert enough to trigger size-based flush
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
            max_memory_bytes: 500, // Very low limit for testing
        };
        let mt = MemTable::new(config);

        // Initially should not apply backpressure
        assert!(!mt.should_apply_backpressure());
        assert!(mt.memory_usage_ratio() < 1.0);

        // Insert until we exceed the limit
        for i in 0..100 {
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
        assert_eq!(entry_count, inserts_per_thread,
            "Expected {} unique keys, got {}", inserts_per_thread, entry_count);

        // Size should be reasonable (100 keys * ~100 bytes + overhead)
        let size = mt.size_bytes();
        assert!(size > 0);
        // Each key has overhead: String key + entry metadata + value
        // Upper bound: 100 keys * (100 bytes value + ~200 bytes overhead)
        assert!(size < inserts_per_thread * 300,
            "Size {} exceeds expected upper bound", size);
    }

    /// P2-007: Test memory headroom calculation
    #[test]
    fn test_memtable_memory_headroom() {
        let config = MemTableConfig {
            flush_threshold_bytes: 10000,
            max_entries: 1000,
            max_memory_bytes: 1000, // 1KB limit
        };
        let mt = MemTable::new(config);

        // Initially should have full headroom
        assert_eq!(mt.memory_headroom(), 1000);
        assert_eq!(mt.backpressure_level(), 0.0);

        // Insert some data
        mt.insert("key1".to_string(), &[0u8; 100]);
        assert!(mt.memory_headroom() < 1000);
        assert!(mt.backpressure_level() > 0.0);

        // Insert until near limit
        for i in 0..8 {
            mt.insert(format!("key_{}", i), &[0u8; 100]);
        }

        // Should have little headroom left
        let headroom = mt.memory_headroom();
        assert!(headroom < 200);
        assert!(mt.backpressure_level() > 0.8);
    }

    /// P2-007: Test backpressure level progression
    #[test]
    fn test_memtable_backpressure_progression() {
        let config = MemTableConfig {
            flush_threshold_bytes: 10000,
            max_entries: 1000,
            max_memory_bytes: 1000,
        };
        let mt = MemTable::new(config);

        // Start at 0%
        assert!(mt.backpressure_level() < 0.1);

        // Insert to 50%
        mt.insert("key1".to_string(), &[0u8; 500]);
        let level = mt.backpressure_level();
        assert!((0.4..=0.6).contains(&level), "Expected ~0.5, got {}", level);

        // Insert to exceed limit
        mt.insert("key2".to_string(), &[0u8; 600]);
        assert!(mt.backpressure_level() >= 1.0);
        assert!(mt.should_apply_backpressure());
    }
}
