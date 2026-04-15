//! Write Coalescer / Write Buffer - 写入缓冲模块
//!
//! Phase 6: 默认写入缓冲路径：
//! - WriteBuffer 作为 WriteEngine 的默认组件
//! - 批量 WAL 写入（一次 fsync 多条记录）
//! - Durability::Immediate 选项可绕过缓冲
//!
//! 缓冲触发条件：
//! - 时间窗口（默认 100ms）
//! - 大小阈值（默认 64KB）
//! - 强制 flush

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use parking_lot::Mutex;
use tracing::debug;

/// 写入缓冲配置
#[derive(Debug, Clone)]
pub struct WriteBufferConfig {
    /// 时间窗口（微秒）- 窗口内的写入会被合并
    pub time_window_us: u64,
    /// 大小阈值（字节）- 达到此大小立即刷盘
    pub size_threshold_bytes: usize,
}

impl Default for WriteBufferConfig {
    fn default() -> Self {
        Self {
            time_window_us: 100_000,    // 100 毫秒窗口（与文档一致）
            size_threshold_bytes: 64 * 1024, // 64KB
        }
    }
}

/// 待缓冲的写入项
#[derive(Debug, Clone)]
pub struct BufferedWrite {
    pub key: String,
    pub value: Vec<u8>,
    /// T-011: Use Instant for relative timestamps (no syscall needed)
    pub timestamp: Instant,
}

/// WriteBuffer 内部状态
pub struct WriteBufferInner {
    /// 待写入队列
    pub pending: Vec<BufferedWrite>,
    /// 当前缓冲总大小（字节）
    pub total_bytes: usize,
    /// T-011: Use Instant for relative time tracking (no syscall needed)
    pub last_flush: Instant,
}

impl WriteBufferInner {
    fn new() -> Self {
        Self {
            pending: Vec::with_capacity(256),
            total_bytes: 0,
            last_flush: Instant::now(),
        }
    }
}

/// 写入缓冲器（Phase 6 重构版本）
pub struct WriteBuffer {
    /// 内部状态（Mutex 保护）
    inner: Mutex<WriteBufferInner>,
    /// 配置
    config: WriteBufferConfig,
    /// 待写入数量（atomic，用于快速查询）
    pending_count: AtomicUsize,
}

impl WriteBuffer {
    pub fn new(config: WriteBufferConfig) -> Self {
        Self {
            inner: Mutex::new(WriteBufferInner::new()),
            config,
            pending_count: AtomicUsize::new(0),
        }
    }

    /// 添加写入到缓冲
    ///
    /// T-011: Use Instant::now() instead of SystemTime::now() (no syscall needed)
    /// 返回 Some(batch) 表示应该 flush（时间窗口或大小阈值触发）
    /// 返回 None 表示继续缓冲
    pub fn add(&self, key: String, value: Vec<u8>) -> Option<Vec<BufferedWrite>> {
        let write_size = key.len() + value.len();
        let now = Instant::now();

        let mut inner = self.inner.lock();

        // 检查是否超过时间窗口
        let time_window_exceeded = if inner.pending.is_empty() {
            false
        } else {
            let elapsed_us = inner.last_flush.elapsed().as_micros() as u64;
            elapsed_us > self.config.time_window_us
        };

        // 如果超过时间窗口且有待 flush 的数据，返回现有批次
        if time_window_exceeded && !inner.pending.is_empty() {
            let batch = std::mem::take(&mut inner.pending);
            let elapsed = inner.last_flush.elapsed();
            inner.total_bytes = 0;
            inner.last_flush = Instant::now();
            self.pending_count.store(0, Ordering::Relaxed);
            debug!(
                "WriteBuffer: time window exceeded ({}us > {}us), flushing {} writes",
                elapsed.as_micros(),
                self.config.time_window_us,
                batch.len()
            );
            return Some(batch);
        }

        // 添加新写入
        inner.pending.push(BufferedWrite {
            key,
            value,
            timestamp: now,
        });
        inner.total_bytes += write_size;
        self.pending_count.fetch_add(1, Ordering::Relaxed);

        // 检查是否达到大小阈值
        if inner.total_bytes >= self.config.size_threshold_bytes {
            let batch = std::mem::take(&mut inner.pending);
            inner.total_bytes = 0;
            inner.last_flush = Instant::now();
            self.pending_count.store(0, Ordering::Relaxed);
            debug!(
                "WriteBuffer: size threshold exceeded ({} >= {} bytes), flushing {} writes",
                inner.total_bytes + write_size,
                self.config.size_threshold_bytes,
                batch.len()
            );
            return Some(batch);
        }

        None // 继续缓冲
    }

    /// 强制 flush 所有待写入
    pub fn force_flush(&self) -> Vec<BufferedWrite> {
        let mut inner = self.inner.lock();
        let batch = std::mem::take(&mut inner.pending);
        inner.total_bytes = 0;
        inner.last_flush = Instant::now();
        self.pending_count.store(0, Ordering::Relaxed);
        batch
    }

    /// 检查是否有待处理的写入
    pub fn has_pending(&self) -> bool {
        self.pending_count.load(Ordering::Relaxed) > 0
    }

    /// 获取待写入数量
    pub fn pending_count(&self) -> usize {
        self.pending_count.load(Ordering::Relaxed)
    }

    /// 获取当前缓冲大小（字节）
    pub fn buffer_size(&self) -> usize {
        self.inner.lock().total_bytes
    }
}

// 向后兼容别名
pub type WriteCoalescer = WriteBuffer;
pub type WriteCoalescerConfig = WriteBufferConfig;
pub type PendingWrite = BufferedWrite;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_buffer_add() {
        let config = WriteBufferConfig {
            time_window_us: 1000, // 1ms for testing
            size_threshold_bytes: 1024,
        };
        let buffer = WriteBuffer::new(config);

        // First write should not trigger flush
        let result = buffer.add("key1".to_string(), b"value1".to_vec());
        assert!(result.is_none());
        assert_eq!(buffer.pending_count(), 1);

        // Second write
        let result = buffer.add("key2".to_string(), b"value2".to_vec());
        assert!(result.is_none());
        assert_eq!(buffer.pending_count(), 2);
    }

    #[test]
    fn test_write_buffer_force_flush() {
        let config = WriteBufferConfig::default();
        let buffer = WriteBuffer::new(config);

        buffer.add("key1".to_string(), b"value1".to_vec());
        buffer.add("key2".to_string(), b"value2".to_vec());

        let flushed = buffer.force_flush();
        assert_eq!(flushed.len(), 2);
        assert_eq!(buffer.pending_count(), 0);
        assert!(!buffer.has_pending());
    }

    #[test]
    fn test_write_buffer_size_threshold() {
        let config = WriteBufferConfig {
            time_window_us: 1000000, // 1s - long enough
            size_threshold_bytes: 100, // Small threshold for testing
        };
        let buffer = WriteBuffer::new(config);

        // Add writes until we exceed threshold
        let mut flushed_batch = None;
        for i in 0..10 {
            let result = buffer.add(
                format!("key_{}", i),
                vec![0u8; 20], // 20 bytes each
            );
            if result.is_some() {
                flushed_batch = result;
                break;
            }
        }

        // Should have triggered flush due to size
        assert!(flushed_batch.is_some());
        assert!(!buffer.has_pending()); // Buffer should be empty after flush
    }

    #[test]
    fn test_write_buffer_returns_batch_on_trigger() {
        let config = WriteBufferConfig {
            time_window_us: 1000,
            size_threshold_bytes: 1000,
        };
        let buffer = WriteBuffer::new(config);

        // Add several writes
        for i in 0..5 {
            let result = buffer.add(
                format!("key_{}", i),
                format!("value_{}", i).into_bytes(),
            );
            if result.is_some() {
                // Verify batch contains all pending writes
                let batch = result.unwrap();
                assert!(!batch.is_empty());
                assert!(batch.len() <= 5);
                return;
            }
        }

        // If no flush triggered yet, force it
        let batch = buffer.force_flush();
        assert_eq!(batch.len(), 5);
    }
}
