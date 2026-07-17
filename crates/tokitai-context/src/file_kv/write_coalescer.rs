//! Write Coalescer - 写入合并模块
//!
//! P2-012: 合并短时间内的多次写入，提高吞吐量：
//! - 时间窗口合并（默认 100µs）
//! - 大小阈值触发（默认 64KB）
//! - 自动刷盘
//!
//! 使用场景：
//! - 高频小写入场景
//! - 日志记录
//! - 指标收集

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use parking_lot::Mutex;
use tracing::debug;

/// 写入合并配置
#[derive(Debug, Clone)]
pub struct WriteCoalescerConfig {
    /// 时间窗口（微秒）- 窗口内的写入会被合并
    pub time_window_us: u64,
    /// 大小阈值（字节）- 达到此大小立即刷盘
    pub size_threshold_bytes: usize,
    /// 是否启用
    pub enabled: bool,
}

impl Default for WriteCoalescerConfig {
    fn default() -> Self {
        Self {
            time_window_us: 100,        // 100 微秒窗口
            size_threshold_bytes: 64 * 1024, // 64KB
            enabled: true,
        }
    }
}

/// 待合并的写入项
#[derive(Debug, Clone)]
pub struct PendingWrite {
    pub key: String,
    pub value: Vec<u8>,
    pub timestamp_us: u64,
}

/// 写入合并器
pub struct WriteCoalescer {
    /// 待写入队列
    pending_writes: Mutex<Vec<PendingWrite>>,
    /// 当前缓冲大小
    buffer_size: AtomicUsize,
    /// 是否有待处理的写入
    has_pending: AtomicBool,
    /// 配置
    config: WriteCoalescerConfig,
    /// 起始时间（微秒）
    start_time_us: u64,
}

impl WriteCoalescer {
    pub fn new(config: WriteCoalescerConfig) -> Self {
        let start_time_us = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(std::time::Duration::ZERO)
            .as_micros() as u64;
        
        Self {
            pending_writes: Mutex::new(Vec::with_capacity(256)),
            buffer_size: AtomicUsize::new(0),
            has_pending: AtomicBool::new(false),
            config,
            start_time_us,
        }
    }

    /// 获取当前时间（微秒）
    #[inline]
    fn now_us(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(std::time::Duration::ZERO)
            .as_micros() as u64
    }

    /// 添加写入到缓冲队列
    ///
    /// 返回 true 表示应该立即刷盘
    pub fn add(&self, key: String, value: Vec<u8>) -> bool {
        if !self.config.enabled {
            return true; // 未启用时直接返回 true，让调用方立即写入
        }

        let write_size = key.len() + value.len();
        let now = self.now_us();

        let mut pending = self.pending_writes.lock();

        // 检查是否超过时间窗口
        let should_flush = if let Some(oldest) = pending.first() {
            (now - oldest.timestamp_us) > self.config.time_window_us
        } else {
            false
        };

        // 如果超过时间窗口，先返回现有缓冲让调用方刷盘
        if should_flush && !pending.is_empty() {
            let elapsed = pending.first()
                .map(|p| now - p.timestamp_us)
                .unwrap_or(0);
            debug!(
                "Write coalescer: time window exceeded ({}us > {}us), flushing {} pending writes",
                elapsed,
                self.config.time_window_us,
                pending.len()
            );
            return true;
        }

        // 添加新写入
        pending.push(PendingWrite {
            key,
            value,
            timestamp_us: now,
        });

        self.buffer_size.fetch_add(write_size, Ordering::Relaxed);
        self.has_pending.store(true, Ordering::Relaxed);

        // 检查是否达到大小阈值
        let current_size = self.buffer_size.load(Ordering::Relaxed);
        if current_size >= self.config.size_threshold_bytes {
            debug!(
                "Write coalescer: size threshold exceeded ({} >= {} bytes), flushing",
                current_size,
                self.config.size_threshold_bytes
            );
            return true;
        }

        false // 继续缓冲
    }

    /// 获取所有待写入项并清空缓冲
    pub fn drain(&self) -> Vec<PendingWrite> {
        let mut pending = self.pending_writes.lock();
        self.buffer_size.store(0, Ordering::Relaxed);
        self.has_pending.store(false, Ordering::Relaxed);
        std::mem::take(&mut *pending)
    }

    /// 检查是否有待处理的写入
    pub fn has_pending(&self) -> bool {
        self.has_pending.load(Ordering::Relaxed)
    }

    /// 获取待写入数量
    pub fn pending_count(&self) -> usize {
        self.pending_writes.lock().len()
    }

    /// 获取当前缓冲大小
    pub fn buffer_size(&self) -> usize {
        self.buffer_size.load(Ordering::Relaxed)
    }

    /// 强制清空缓冲（用于关闭或手动刷盘）
    pub fn force_flush(&self) -> Vec<PendingWrite> {
        self.drain()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_coalescer_add() {
        let config = WriteCoalescerConfig {
            time_window_us: 1000, // 1ms for testing
            size_threshold_bytes: 1024,
            enabled: true,
        };
        let coalescer = WriteCoalescer::new(config);

        // First write should not trigger flush
        let should_flush = coalescer.add("key1".to_string(), b"value1".to_vec());
        assert!(!should_flush);
        assert_eq!(coalescer.pending_count(), 1);

        // Second write
        let should_flush = coalescer.add("key2".to_string(), b"value2".to_vec());
        assert!(!should_flush);
        assert_eq!(coalescer.pending_count(), 2);
    }

    #[test]
    fn test_write_coalescer_drain() {
        let config = WriteCoalescerConfig::default();
        let coalescer = WriteCoalescer::new(config);

        coalescer.add("key1".to_string(), b"value1".to_vec());
        coalescer.add("key2".to_string(), b"value2".to_vec());

        let drained = coalescer.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(coalescer.pending_count(), 0);
        assert!(!coalescer.has_pending());
    }

    #[test]
    fn test_write_coalescer_disabled() {
        let config = WriteCoalescerConfig {
            enabled: false,
            ..Default::default()
        };
        let coalescer = WriteCoalescer::new(config);

        // Should always return true when disabled
        let should_flush = coalescer.add("key1".to_string(), b"value1".to_vec());
        assert!(should_flush);
        assert_eq!(coalescer.pending_count(), 0); // Not buffered
    }

    #[test]
    fn test_write_coalescer_size_threshold() {
        let config = WriteCoalescerConfig {
            time_window_us: 1000000, // 1s - long enough
            size_threshold_bytes: 100, // Small threshold for testing
            enabled: true,
        };
        let coalescer = WriteCoalescer::new(config);

        // Add writes until we exceed threshold
        let mut should_flush = false;
        for i in 0..10 {
            should_flush = coalescer.add(
                format!("key_{}", i),
                vec![0u8; 20], // 20 bytes each
            );
            if should_flush {
                break;
            }
        }

        // Should have triggered flush due to size
        assert!(should_flush);
    }
}
