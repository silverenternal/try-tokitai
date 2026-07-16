//! Type definitions for FileKV
//!
//! This module contains core data structures:
//! - ValuePointer: Points to values in segment files
//! - FileKVConfig: Configuration with validation
//! - FileKVStats: Statistics counters

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use crate::cache::block_cache::BlockCacheConfig;
use crate::compaction::CompactionConfig;
use crate::compression::dictionary::DictionaryCompressionConfig;
use crate::core::memtable::MemTableConfig;
use crate::io::{FileKVFileSystem, StdFs};
use crate::ops::audit_log::AuditLogConfig;

/// Block compression mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlockCompressionMode {
    /// No compression
    None,
    /// Zstandard compression (level 3)
    #[default]
    Zstd,
    /// Snappy compression (low latency)
    Snappy,
    /// LZ4 compression (high throughput)
    Lz4,
}

impl BlockCompressionMode {
    /// Returns the compression algorithm ID for persistence
    pub fn algorithm_id(&self) -> u8 {
        match self {
            Self::None => 0,
            Self::Zstd => 1,
            Self::Snappy => 2,
            Self::Lz4 => 3,
        }
    }

    /// Create from algorithm ID
    pub fn from_algorithm_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(Self::None),
            1 => Some(Self::Zstd),
            2 => Some(Self::Snappy),
            3 => Some(Self::Lz4),
            _ => None,
        }
    }
}

/// Block compression configuration
#[derive(Debug, Clone)]
pub struct BlockCompressionConfig {
    /// Compression mode
    pub mode: BlockCompressionMode,
    /// Compression level (1-22 for zstd, higher = better compression but slower)
    pub compression_level: i32,
    /// Minimum block size to consider for compression (smaller blocks skip compression)
    pub min_compress_size: u64,
}

impl Default for BlockCompressionConfig {
    fn default() -> Self {
        Self {
            mode: BlockCompressionMode::Zstd,
            compression_level: 3,
            min_compress_size: 64,
        }
    }
}

/// WAL 同步策略
///
/// 控制 WAL 写入后的同步行为，在性能和数据安全性之间做权衡。
/// 不同的同步模式对写入延迟和数据持久化保证有显著影响。
///
/// ## 三种模式对比
///
/// | 模式 | 写入延迟 | 持久化保证 | 适用场景 |
/// |------|---------|-----------|---------|
/// | [`Immediate`](WalSyncMode::Immediate) | 基准 (最慢) | 100% | 金融、医疗、审计日志 |
/// | [`Batch`](WalSyncMode::Batch) | 2-3x 提升 | ~99% | 大多数生产环境 |
/// | [`Lazy`](WalSyncMode::Lazy) (默认) | 5-10x 提升 | ~90% | 缓存、临时数据 |
///
/// ## 详细说明
///
/// ### Immediate
/// 每次 WAL 写入后都调用 `fsync`，确保数据立即持久化到磁盘。
/// 提供最强的数据持久化保证，但写入延迟最高。
///
/// ### Batch
/// 每隔 `batch_sync_interval` 次写入才刷盘（不 fsync），
/// 让操作系统负责最终刷新。在正常关闭时数据不会丢失，
/// 但系统崩溃时可能丢失最近的少量数据。
///
/// ### Lazy
/// 仅写入 OS 缓冲区，不主动刷盘。由操作系统决定何时将数据
/// 写入磁盘（通常几秒到几十秒）。在系统正常关闭时数据最终
/// 会持久化，但断电或内核崩溃时可能丢失最近的数据。
///
/// ## 与 AggressiveConfig 的关系
///
/// [`AggressiveConfig`](crate::AggressiveConfig) 的四档预设各自使用不同的
/// WalSyncMode：
/// - Conservative → Immediate
/// - Balanced / Performance → Batch
/// - Extreme → Lazy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WalSyncMode {
    /// 每次写入都调用 fsync - 最安全，最慢
    /// - 数据持久化保证：100%
    /// - 写入延迟：基准
    Immediate,

    /// 批量 fsync - 折中方案
    /// - 数据持久化保证：~99%（系统崩溃可能丢少量数据）
    /// - 写入延迟：2-3x 提升
    Batch,

    /// 依赖操作系统刷新 - 最快，可能丢数据
    /// - 数据持久化保证：~90%（断电可能丢数据）
    /// - 写入延迟：5-10x 提升
    #[default]
    Lazy,
}

/// Phase 6: 写入持久性级别
///
/// 控制写入操作的数据持久性保证级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Durability {
    /// 缓冲模式（默认）：写入先进入 WriteBuffer，批量刷盘
    /// - 优点：吞吐高，p99延迟低
    /// - 缺点：崩溃时可能丢失最近的数据（由WAL恢复）
    #[default]
    Buffered,
    /// 立即模式：绕过 WriteBuffer，直接写 WAL + MemTable
    /// - 优点：数据立即持久化，崩溃恢复无丢失
    /// - 缺点：吞吐较低，每次写入都有fsync开销
    Immediate,
}

/// P3-001: I/O 模式
///
/// 控制写入操作使用的 I/O 路径
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IoMode {
    /// 同步 I/O（默认）：阻塞当前线程直到写入完成
    #[default]
    Sync,
    /// 异步 I/O：通过 AsyncWriter 非阻塞写入磁盘
    /// - 需要在 tokio runtime 中运行
    /// - 适合高并发写入场景
    Async,
}

/// 激进优化配置
///
/// 控制一系列以资源换性能的优化选项。通过调整索引策略、预读行为、WAL 同步模式、
/// 缓存大小和 mmap 策略，在数据安全和性能之间做权衡。
///
/// ## 四档预设
///
/// FileKV 提供四档预设，适用于不同场景：
///
/// | 预设 | QPS 预期 | 内存占用 | 数据规模 | 典型场景 |
/// |------|---------|---------|---------|---------|
/// | [`conservative()`](AggressiveConfig::conservative) | 低 (~1K-5K) | ~64MB | <10K keys | 金融、医疗等数据持久化要求极高的场景 |
/// | [`balanced()`](AggressiveConfig::balanced) (默认) | 中 (~10K-50K) | ~256MB | 10K-100K keys | 大多数生产环境，性能与安全折中 |
/// | [`performance()`](AggressiveConfig::performance) | 高 (~50K-200K) | ~1GB | 100K-500K keys | AI 上下文、会话存储等读取密集场景 |
/// | [`extreme()`](AggressiveConfig::extreme) | 极高 (~200K+) | ~4GB | 500K+ keys | 缓存、临时数据等可丢失场景 |
///
/// ## 各档位详细说明
///
/// ### Conservative（保守模式）
/// - **QPS 预期**: 1K-5K，每次写入都 fsync，写入延迟最高
/// - **内存占用**: 约 64MB（BlockCache 上限）
/// - **索引策略**: 稀疏索引（非密集索引），内存占用最小
/// - **预读**: 禁用预读，避免浪费 I/O 带宽
/// - **mmap**: 不使用持久 mmap，减少文件句柄占用
/// - **数据安全**: 最高，每次写入都调用 fsync 确保持久化
/// - **适用场景**: 金融交易记录、医疗数据、合规日志等不能丢数据的场景
///
/// ### Balanced（平衡模式，默认）
/// - **QPS 预期**: 10K-50K，批量 fsync 降低写入延迟
/// - **内存占用**: 约 256MB（BlockCache 上限）
/// - **索引策略**: 密集索引，读取延迟降低 50-80%
/// - **预读**: 2x 预读，顺序读取吞吐量提升约 2x
/// - **mmap**: 使用持久 mmap，读取延迟降低 80-90%
/// - **数据安全**: 中等，批量 fsync，~99% 持久化保证
/// - **适用场景**: Web 应用配置存储、用户会话、中等规模的 KV 存储
///
/// ### Performance（性能模式）
/// - **QPS 预期**: 50K-200K，读取延迟接近内存级别
/// - **内存占用**: 约 1GB（BlockCache 上限 + 全内存块索引）
/// - **索引策略**: 密集索引 + 全内存块索引
/// - **预读**: 4x 预读，顺序读取吞吐量提升约 4x
/// - **mmap**: 使用持久 mmap
/// - **数据安全**: 中等，批量 fsync
/// - **适用场景**: AI 对话上下文、推荐系统特征存储、热点数据缓存
///
/// ### Extreme（极限模式）
/// - **QPS 预期**: 200K+，不计代价追求极致性能
/// - **内存占用**: 约 4GB（BlockCache 上限 + 全内存块索引）
/// - **索引策略**: 密集索引 + 全内存块索引
/// - **预读**: 8x 激进预读，顺序读取吞吐量提升约 8x
/// - **mmap**: 使用持久 mmap
/// - **数据安全**: 最低，Lazy 同步模式依赖 OS 刷新，~90% 持久化保证
/// - **适用场景**: 临时缓存、构建产物缓存、可重新生成的数据
///
/// ## 自定义配置
///
/// 如果预设不满足需求，可以手动构造 `AggressiveConfig`：
///
/// ```rust
/// use tokitai_filekv::AggressiveConfig;
///
/// let custom = AggressiveConfig {
///     dense_index_enabled: true,
///     readahead_multiplier: 3,
///     wal_sync_mode: tokitai_filekv::WalSyncMode::Batch,
///     cache_max_memory_bytes: 512 * 1024 * 1024,
///     persistent_mmap_enabled: true,
///     in_memory_block_index_enabled: false,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct AggressiveConfig {
    /// 全内存密集索引
    /// - 收益：读取延迟降低 50-80%（跳过 header 解析）
    /// - 成本：索引大小增加 3-4x（每个 entry 多 8-12 字节）
    /// - 推荐：内存充足且读取密集场景开启
    pub dense_index_enabled: bool,

    /// 预读倍数
    /// - 收益：顺序读取吞吐量提升 2-4x
    /// - 成本：额外磁盘 IO（可能浪费带宽）
    /// - 0 = 禁用预读
    /// - 1-2 = 保守预读（读取下一个 block）
    /// - 4-8 = 激进预读（读取后续多个 blocks）
    /// - 推荐：顺序读取场景设置为 2-4
    pub readahead_multiplier: u32,

    /// WAL 同步策略
    /// - 推荐：关键数据用 Immediate，缓存用 Lazy
    pub wal_sync_mode: WalSyncMode,

    /// BlockCache 内存上限
    /// - 收益：更大的缓存 = 更高的命中率
    /// - 成本：内存占用
    /// - 推荐：64MB - 4GB（根据可用内存调整）
    pub cache_max_memory_bytes: usize,

    /// 持久 mmap（只读模式）
    /// - 收益：读取延迟降低 80-90%（避免重复 mmap 创建）
    /// - 成本：文件句柄占用
    /// - 推荐：读取密集场景开启
    pub persistent_mmap_enabled: bool,

    /// 全内存 Block 索引
    /// - 收益：读取延迟接近 RocksDB（全内存索引）
    /// - 成本：高内存占用（~100MB-1GB）
    /// - 推荐：极致读取性能场景开启
    pub in_memory_block_index_enabled: bool,
}

impl Default for AggressiveConfig {
    fn default() -> Self {
        Self {
            dense_index_enabled: true,
            readahead_multiplier: 2,
            wal_sync_mode: WalSyncMode::Batch,
            cache_max_memory_bytes: 256 * 1024 * 1024, // 256MB
            persistent_mmap_enabled: true,
            in_memory_block_index_enabled: false,
        }
    }
}

impl AggressiveConfig {
    /// 保守模式 - 数据安全优先
    ///
    /// 每次写入都调用 fsync，确保数据完全持久化。关闭密集索引、预读和持久 mmap，
    /// 以最小的内存和 I/O 开销换取最高的数据安全性。
    ///
    /// ## 配置详情
    /// - `dense_index_enabled`: false - 使用稀疏索引，内存占用最小
    /// - `readahead_multiplier`: 0 - 禁用预读
    /// - `wal_sync_mode`: [`WalSyncMode::Immediate`] - 每次写入 fsync
    /// - `cache_max_memory_bytes`: 64MB
    /// - `persistent_mmap_enabled`: false
    /// - `in_memory_block_index_enabled`: false
    ///
    /// ## 适用场景
    /// - 金融交易记录（不能丢数据）
    /// - 医疗数据存储（合规要求）
    /// - 审计日志（法律要求持久化保证）
    /// - 任何写入量 <10K keys、数据安全优先于性能的场景
    pub fn conservative() -> Self {
        Self {
            dense_index_enabled: false,
            readahead_multiplier: 0,
            wal_sync_mode: WalSyncMode::Immediate,
            cache_max_memory_bytes: 64 * 1024 * 1024,
            persistent_mmap_enabled: false,
            in_memory_block_index_enabled: false,
        }
    }

    /// 平衡模式 - 性能与安全折中
    ///
    /// 批量 fsync 降低写入延迟，开启密集索引和持久 mmap，在数据安全和性能
    /// 之间取得良好平衡。这是大多数生产环境的推荐配置。
    ///
    /// ## 配置详情
    /// - `dense_index_enabled`: true - 密集索引，读取延迟降低 50-80%
    /// - `readahead_multiplier`: 2 - 2x 预读
    /// - `wal_sync_mode`: [`WalSyncMode::Batch`] - 批量 fsync
    /// - `cache_max_memory_bytes`: 256MB
    /// - `persistent_mmap_enabled`: true
    /// - `in_memory_block_index_enabled`: false
    ///
    /// ## 适用场景
    /// - Web 应用配置存储
    /// - 用户会话管理
    /// - 中等规模 (10K-100K keys) 的通用 KV 存储
    /// - 大多数生产环境的首选配置
    pub fn balanced() -> Self {
        Self {
            dense_index_enabled: true,
            readahead_multiplier: 2,
            wal_sync_mode: WalSyncMode::Batch,
            cache_max_memory_bytes: 256 * 1024 * 1024,
            persistent_mmap_enabled: true,
            in_memory_block_index_enabled: false,
        }
    }

    /// 性能模式 - 读取速度优先
    ///
    /// 开启全内存块索引和 4x 预读，读取延迟接近内存级别。
    /// 适合 100K-500K keys 规模的读取密集场景。
    ///
    /// ## 配置详情
    /// - `dense_index_enabled`: true - 密集索引
    /// - `readahead_multiplier`: 4 - 4x 预读，顺序读取吞吐量提升约 4x
    /// - `wal_sync_mode`: [`WalSyncMode::Batch`] - 批量 fsync
    /// - `cache_max_memory_bytes`: 1GB
    /// - `persistent_mmap_enabled`: true
    /// - `in_memory_block_index_enabled`: true - 全内存块索引
    ///
    /// ## 适用场景
    /// - AI 对话上下文存储（tokitai-context 核心场景）
    /// - 推荐系统特征存储
    /// - 热点数据缓存
    /// - 读取量远大于写入量的场景（读/写比 >10:1）
    pub fn performance() -> Self {
        Self {
            dense_index_enabled: true,
            readahead_multiplier: 4,
            wal_sync_mode: WalSyncMode::Batch,
            cache_max_memory_bytes: 1024 * 1024 * 1024,
            persistent_mmap_enabled: true,
            in_memory_block_index_enabled: true,
        }
    }

    /// 极限模式 - 不计代价追求性能
    ///
    /// 使用 [`WalSyncMode::Lazy`] 依赖操作系统刷新，8x 激进预读，
    /// 4GB BlockCache 上限 + 全内存块索引。数据安全保证最低（~90% 持久化），
    /// 但写入延迟可以达到最低。
    ///
    /// ## 配置详情
    /// - `dense_index_enabled`: true - 密集索引
    /// - `readahead_multiplier`: 8 - 8x 激进预读
    /// - `wal_sync_mode`: [`WalSyncMode::Lazy`] - 依赖 OS 刷新
    /// - `cache_max_memory_bytes`: 4GB
    /// - `persistent_mmap_enabled`: true
    /// - `in_memory_block_index_enabled`: true - 全内存块索引
    ///
    /// ## 警告
    /// 此模式在系统崩溃或断电时可能丢失最近写入的数据。
    /// 仅在数据可重新生成或丢失可接受的场景下使用。
    ///
    /// ## 适用场景
    /// - 临时缓存（如 HTTP 响应缓存）
    /// - 构建产物缓存（CI/CD 流水线）
    /// - 可重新生成的中间结果
    /// - 500K+ keys 大规模场景，且数据可丢失
    pub fn extreme() -> Self {
        Self {
            dense_index_enabled: true,
            readahead_multiplier: 8,
            wal_sync_mode: WalSyncMode::Lazy,
            cache_max_memory_bytes: 4 * 1024 * 1024 * 1024,
            persistent_mmap_enabled: true,
            in_memory_block_index_enabled: true,
        }
    }

    /// 估算内存占用
    pub fn estimated_memory_usage(&self, estimated_entries: usize) -> MemoryUsageEstimate {
        let mut total = 0usize;
        let mut breakdown = Vec::new();

        // BlockCache
        total += self.cache_max_memory_bytes;
        breakdown.push(("BlockCache", self.cache_max_memory_bytes));

        // DenseIndex: 每个 entry 约 20 字节（segment_id, offset, key_len, len, checksum）
        if self.dense_index_enabled {
            let index_size = estimated_entries * 20;
            total += index_size;
            breakdown.push(("DenseIndex", index_size));
        }

        // In-memory block index: 每个 block 约 100 字节
        if self.in_memory_block_index_enabled {
            // 假设平均每 block 4KB，每个 entry 100 字节
            let block_count = (estimated_entries * 100) / 4096;
            let index_size = block_count * 100;
            total += index_size;
            breakdown.push(("BlockIndex", index_size));
        }

        MemoryUsageEstimate {
            total_bytes: total,
            breakdown,
        }
    }
}

/// 内存使用估算
#[derive(Debug, Clone)]
pub struct MemoryUsageEstimate {
    pub total_bytes: usize,
    pub breakdown: Vec<(&'static str, usize)>,
}

impl std::fmt::Display for MemoryUsageEstimate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Total: {:.2} MB", self.total_bytes as f64 / 1024.0 / 1024.0)?;
        for (name, size) in &self.breakdown {
            writeln!(f, "  - {}: {:.2} MB", name, *size as f64 / 1024.0 / 1024.0)?;
        }
        Ok(())
    }
}

/// Bloom Filter 文件魔法数 (exported for bloom module)
pub const BLOOM_MAGIC: u32 = 0x424C4F4F; // "BLOO" = Bloom Filter
/// Bloom Filter 文件版本
/// V1: [magic 4B][version 4B][num_keys 8B][keys...]
/// V2: [magic 4B][version 4B][num_bits 4B][num_hashes 4B][num_keys 8B][keys...]
/// Note: V3 (bitvector-only format) was attempted but abandoned due to hash builder serialization issues.
/// V2 remains the optimal format given bloom crate's RandomState hash builders.
pub const BLOOM_VERSION: u32 = 2;
/// Default Bloom Filter false positive rate (1%)
pub const DEFAULT_BLOOM_FPR: f32 = 0.01;

/// 值指针（指向 segment 文件中的位置）
#[derive(Debug, Clone, Copy)]
pub struct ValuePointer {
    /// 段文件 ID
    pub segment_id: u64,
    /// 段内偏移
    pub offset: u64,
    /// key 长度 - PERF-003: 用于快速读取路径
    pub key_len: u32,
    /// 值长度
    pub len: u32,
    /// CRC32 校验和
    pub checksum: u32,
}

impl ValuePointer {
    pub fn new(segment_id: u64, offset: u64, key_len: u32, len: u32, checksum: u32) -> Self {
        Self {
            segment_id,
            offset,
            key_len,
            len,
            checksum,
        }
    }

    /// 创建不带 key_len 的指针（向后兼容）
    pub fn new_legacy(segment_id: u64, offset: u64, len: u32, checksum: u32) -> Self {
        Self {
            segment_id,
            offset,
            key_len: 0, // 未知 key_len
            len,
            checksum,
        }
    }

    /// 序列化为字节（用于 WAL）- 包含 key_len
    pub fn to_bytes(&self) -> [u8; 28] {
        let mut buf = [0u8; 28];
        buf[0..8].copy_from_slice(&self.segment_id.to_le_bytes());
        buf[8..16].copy_from_slice(&self.offset.to_le_bytes());
        buf[16..20].copy_from_slice(&self.key_len.to_le_bytes());
        buf[20..24].copy_from_slice(&self.len.to_le_bytes());
        buf[24..28].copy_from_slice(&self.checksum.to_le_bytes());
        buf
    }

    /// 从字节反序列化 - 包含 key_len
    pub fn from_bytes(buf: &[u8; 28]) -> anyhow::Result<Self> {
        Ok(Self {
            segment_id: u64::from_le_bytes(
                buf[0..8]
                    .try_into()
                    .map_err(|e| anyhow::anyhow!("Invalid segment_id bytes: {}", e))?,
            ),
            offset: u64::from_le_bytes(
                buf[8..16]
                    .try_into()
                    .map_err(|e| anyhow::anyhow!("Invalid offset bytes: {}", e))?,
            ),
            key_len: u32::from_le_bytes(
                buf[16..20]
                    .try_into()
                    .map_err(|e| anyhow::anyhow!("Invalid key_len bytes: {}", e))?,
            ),
            len: u32::from_le_bytes(
                buf[20..24]
                    .try_into()
                    .map_err(|e| anyhow::anyhow!("Invalid len bytes: {}", e))?,
            ),
            checksum: u32::from_le_bytes(
                buf[24..28]
                    .try_into()
                    .map_err(|e| anyhow::anyhow!("Invalid checksum bytes: {}", e))?,
            ),
        })
    }

    /// 从字节反序列化（旧格式，24 字节，不含 key_len）- 向后兼容
    pub fn from_bytes_legacy(buf: &[u8; 24]) -> anyhow::Result<Self> {
        Ok(Self {
            segment_id: u64::from_le_bytes(
                buf[0..8]
                    .try_into()
                    .map_err(|e| anyhow::anyhow!("Invalid segment_id bytes: {}", e))?,
            ),
            offset: u64::from_le_bytes(
                buf[8..16]
                    .try_into()
                    .map_err(|e| anyhow::anyhow!("Invalid offset bytes: {}", e))?,
            ),
            len: u32::from_le_bytes(
                buf[16..20]
                    .try_into()
                    .map_err(|e| anyhow::anyhow!("Invalid len bytes: {}", e))?,
            ),
            checksum: u32::from_le_bytes(
                buf[20..24]
                    .try_into()
                    .map_err(|e| anyhow::anyhow!("Invalid checksum bytes: {}", e))?,
            ),
            key_len: 0, // 旧格式不包含 key_len
        })
    }
}

/// FileKV 配置验证错误
#[derive(Debug, Clone, thiserror::Error)]
pub enum FileKVConfigError {
    #[error("MemTable flush threshold too low: {0} bytes (minimum: {1})")]
    MemTableThresholdTooLow(usize, usize),
    #[error("MemTable max entries too low: {0} (minimum: {1})")]
    MemTableMaxEntriesTooLow(usize, usize),
    #[error("Block cache size too small: {0} bytes (minimum: {1})")]
    BlockCacheTooSmall(usize, usize),
    #[error("Block cache max items too low: {0} (minimum: {1})")]
    BlockCacheMaxItemsTooLow(usize, usize),
    #[error("Background flush interval too short: {0}ms (minimum: {1}ms)")]
    BackgroundFlushIntervalTooShort(u64, u64),
    #[error("Compaction min_segments too large: {0} (maximum: {1})")]
    CompactionMinSegmentsTooLarge(usize, usize),
    #[error("Segment max size smaller than target: max={max}, target={target}")]
    SegmentSizeMismatch { max: u64, target: u64 },
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    #[error("Path is not writable: {0}")]
    PathNotWritable(String),
}

/// FileKV 配置验证结果
#[derive(Debug, Clone, Default)]
pub struct FileKVConfigValidation {
    pub is_valid: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<FileKVConfigError>,
}

impl FileKVConfigValidation {
    pub fn is_valid(&self) -> bool {
        self.is_valid && self.errors.is_empty()
    }

    pub fn all_issues(&self) -> Vec<String> {
        let mut issues = self.warnings.clone();
        for err in &self.errors {
            issues.push(err.to_string());
        }
        issues
    }
}

/// FileKV 配置
#[derive(Clone)]
pub struct FileKVConfig {
    pub memtable: MemTableConfig,
    pub segment_dir: PathBuf,
    pub enable_wal: bool,
    pub wal_dir: PathBuf,
    pub index_dir: PathBuf,
    pub cache: BlockCacheConfig,
    pub enable_bloom: bool,
    pub compaction: CompactionConfig,
    pub enable_background_flush: bool,
    pub background_flush_interval_ms: u64,
    pub segment_preallocate_size: u64,
    /// Configurable block size (bytes). Default: 8192 (8KB).
    /// Smaller blocks = finer granularity, larger blocks = better sequential I/O.
    pub block_size: u64,
    /// Block-level compression configuration.
    pub block_compression: BlockCompressionConfig,
    // P1-013: WAL rotation configuration
    pub wal_max_size_bytes: u64,
    pub wal_max_files: usize,
    // P2-004: Cache warming configuration
    pub cache_warming_enabled: bool,
    // P2-014: Dictionary compression configuration
    pub compression: DictionaryCompressionConfig,
    // P3-001: Async I/O configuration
    pub async_io_enabled: bool,
    pub async_io_max_concurrent_writes: usize,
    pub async_io_max_queue_depth: usize,
    pub async_io_write_timeout_ms: u64,
    pub async_io_enable_coalescing: bool,
    pub async_io_coalesce_window_ms: u64,
    /// P2-009: Checkpoint directory for incremental checkpoints
    pub checkpoint_dir: PathBuf,
    /// P2-013: Audit log configuration
    pub audit_log: AuditLogConfig,
    /// P4-001: Aggressive optimization configuration
    pub aggressive: AggressiveConfig,
    /// INNO-001: Enable adaptive Bloom filter cache (default: true)
    pub enable_adaptive_bloom_cache: bool,
    /// INNO-002: Enable Zone Map range query pruning (default: true)
    pub enable_zone_map_pruning: bool,
    /// INNO-002: Enable sequential prefetching (default: true)
    pub enable_sequential_prefetch: bool,
    /// OPT-007: Enable multi-level cache (L1 + L2 mmap cache) (default: true)
    ///
    /// When enabled, evicted hot entries from L1 (BlockCache) can be demoted to L2,
    /// and frequently accessed L2 entries can be promoted back to L1.
    pub enable_multi_level_cache: bool,
    /// OPT-007: L2 cache maximum size in bytes (default: 4GB)
    ///
    /// L2 cache uses mmap-backed files, so memory usage is only metadata (~100MB max).
    pub l2_cache_max_bytes: u64,
    /// OPT-007: L2 access count threshold for L1 promotion (default: 5)
    ///
    /// When an L2 entry is accessed at least this many times, it will be promoted
    /// back to L1 cache on the next read from storage.
    pub l2_to_l1_threshold: u32,
    /// OPT-007: Enable WAL channel batching (default: false)
    ///
    /// When enabled, writes are submitted to an mpsc channel and batched
    /// by a background thread before writing to WAL. Reduces fsync overhead
    /// and improves write throughput.
    pub enable_wal_channel: bool,
    /// OPT-007: WAL channel batch interval in milliseconds (default: 2ms)
    ///
    /// Time window to collect writes before flushing to WAL.
    pub wal_channel_interval_ms: u64,
    /// OPT-007: Maximum entries per WAL channel batch (default: 1000)
    pub wal_channel_max_entries: usize,
    /// OPT-007: WAL channel capacity (default: 10000)
    ///
    /// Maximum pending submissions before applying backpressure.
    pub wal_channel_capacity: usize,
    /// Phase 1: Filesystem abstraction (default: StdFs)
    pub fs: Arc<dyn FileKVFileSystem>,
}

impl std::fmt::Debug for FileKVConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileKVConfig")
            .field("memtable", &self.memtable)
            .field("segment_dir", &self.segment_dir)
            .field("enable_wal", &self.enable_wal)
            .field("wal_dir", &self.wal_dir)
            .field("index_dir", &self.index_dir)
            .field("cache", &self.cache)
            .field("enable_bloom", &self.enable_bloom)
            .field("compaction", &self.compaction)
            .field("enable_background_flush", &self.enable_background_flush)
            .field("background_flush_interval_ms", &self.background_flush_interval_ms)
            .field("segment_preallocate_size", &self.segment_preallocate_size)
            .field("block_size", &self.block_size)
            .field("block_compression", &self.block_compression.mode)
            .field("aggressive", &self.aggressive)
            .field("enable_adaptive_bloom_cache", &self.enable_adaptive_bloom_cache)
            .field("enable_zone_map_pruning", &self.enable_zone_map_pruning)
            .field("enable_sequential_prefetch", &self.enable_sequential_prefetch)
            .finish()
    }
}

impl FileKVConfig {
    /// 验证配置
    pub fn validate(&self) -> FileKVConfigValidation {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        const MIN_MEMTABLE_THRESHOLD: usize = 64 * 1024;
        const MIN_MEMTABLE_ENTRIES: usize = 100;

        if self.memtable.flush_threshold_bytes < MIN_MEMTABLE_THRESHOLD {
            errors.push(FileKVConfigError::MemTableThresholdTooLow(
                self.memtable.flush_threshold_bytes,
                MIN_MEMTABLE_THRESHOLD,
            ));
        }

        if self.memtable.max_entries < MIN_MEMTABLE_ENTRIES {
            errors.push(FileKVConfigError::MemTableMaxEntriesTooLow(
                self.memtable.max_entries,
                MIN_MEMTABLE_ENTRIES,
            ));
        }

        const MIN_BLOCK_CACHE_SIZE: u64 = 1024 * 1024;
        const MIN_BLOCK_CACHE_ITEMS: usize = 100;

        if self.cache.max_memory_bytes < MIN_BLOCK_CACHE_SIZE {
            errors.push(FileKVConfigError::BlockCacheTooSmall(
                self.cache.max_memory_bytes as usize,
                MIN_BLOCK_CACHE_SIZE as usize,
            ));
        }

        if self.cache.max_items < MIN_BLOCK_CACHE_ITEMS {
            errors.push(FileKVConfigError::BlockCacheMaxItemsTooLow(
                self.cache.max_items,
                MIN_BLOCK_CACHE_ITEMS,
            ));
        }

        const MIN_FLUSH_INTERVAL_MS: u64 = 10;

        if self.enable_background_flush && self.background_flush_interval_ms < MIN_FLUSH_INTERVAL_MS {
            errors.push(FileKVConfigError::BackgroundFlushIntervalTooShort(
                self.background_flush_interval_ms,
                MIN_FLUSH_INTERVAL_MS,
            ));
        }

        const MAX_MIN_SEGMENTS: usize = 20;

        if self.compaction.min_segments > MAX_MIN_SEGMENTS {
            errors.push(FileKVConfigError::CompactionMinSegmentsTooLarge(
                self.compaction.min_segments,
                MAX_MIN_SEGMENTS,
            ));
        }

        if self.compaction.max_segment_size_bytes < self.compaction.target_segment_size_bytes {
            errors.push(FileKVConfigError::SegmentSizeMismatch {
                max: self.compaction.max_segment_size_bytes,
                target: self.compaction.target_segment_size_bytes,
            });
        }

        for (name, path) in [
            ("segment_dir", &self.segment_dir),
            ("wal_dir", &self.wal_dir),
            ("index_dir", &self.index_dir),
            ("checkpoint_dir", &self.checkpoint_dir),
        ] {
            if path.as_os_str().is_empty() {
                errors.push(FileKVConfigError::InvalidPath(format!("{} is empty", name)));
                continue;
            }

            if path.exists() {
                if !path.is_dir() {
                    errors.push(FileKVConfigError::InvalidPath(format!("{} is not a directory", name)));
                } else {
                    let test_file = path.join(".write_test");
                    match std::fs::File::create(&test_file) {
                        Ok(_) => {
                            let _ = std::fs::remove_file(test_file);
                        }
                        Err(_) => {
                            errors.push(FileKVConfigError::PathNotWritable(name.to_string()));
                        }
                    }
                }
            } else {
                if let Some(parent) = path.parent() {
                    if !parent.exists() {
                        warnings.push(format!("{} parent directory does not exist: {:?}", name, parent));
                    } else if !parent.is_dir() {
                        errors.push(FileKVConfigError::InvalidPath(format!(
                            "{} parent is not a directory",
                            name
                        )));
                    } else {
                        let test_file = parent.join(".write_test");
                        match std::fs::File::create(&test_file) {
                            Ok(_) => {
                                let _ = std::fs::remove_file(test_file);
                            }
                            Err(_) => {
                                errors.push(FileKVConfigError::PathNotWritable(format!("{} parent", name)));
                            }
                        }
                    }
                }
            }
        }

        if self.memtable.flush_threshold_bytes > 64 * 1024 * 1024 {
            warnings.push(format!(
                "Large MemTable threshold ({} bytes) may cause long flush pauses",
                self.memtable.flush_threshold_bytes
            ));
        }

        if self.cache.max_memory_bytes > 512 * 1024 * 1024 {
            warnings.push(format!(
                "Large block cache size ({} bytes) may cause memory pressure",
                self.cache.max_memory_bytes
            ));
        }

        FileKVConfigValidation {
            is_valid: errors.is_empty(),
            warnings,
            errors,
        }
    }

    pub fn validate_strict(&self) -> Result<(), FileKVConfigError> {
        let validation = self.validate();
        if validation.errors.is_empty() {
            Ok(())
        } else {
            // P0-003 FIX: Use expect() with clear error message instead of unwrap()
            Err(validation
                .errors
                .into_iter()
                .next()
                .expect("Validation reported errors but none were found - this is a bug in validate()"))
        }
    }

    /// 保守模式配置 - 数据安全优先
    ///
    /// 适用于：金融、医疗等对数据持久化要求极高的场景
    /// - WAL 同步：每次写入都 fsync
    /// - 缓存：64MB
    /// - 预读：禁用
    /// - 持久 mmap：禁用
    pub fn conservative() -> Self {
        Self {
            aggressive: AggressiveConfig::conservative(),
            ..Default::default()
        }
    }

    /// 平衡模式配置 - 性能与安全折中
    ///
    /// 适用于：大多数生产环境
    /// - WAL 同步：批量 fsync
    /// - 缓存：256MB
    /// - 预读：2x
    /// - 持久 mmap：开启
    pub fn balanced() -> Self {
        Self {
            aggressive: AggressiveConfig::balanced(),
            ..Default::default()
        }
    }

    /// 性能模式配置 - 读取速度优先
    ///
    /// 适用于：AI 上下文、会话存储等读取密集场景
    /// - WAL 同步：批量 fsync
    /// - 缓存：1GB
    /// - 预读：4x
    /// - 持久 mmap：开启
    /// - 全内存索引：开启
    pub fn performance() -> Self {
        Self {
            aggressive: AggressiveConfig::performance(),
            ..Default::default()
        }
    }

    /// 极限模式配置 - 不计代价追求性能
    ///
    /// 适用于：缓存、临时数据等可丢失场景
    /// - WAL 同步：Lazy（依赖操作系统）
    /// - 缓存：4GB
    /// - 预读：8x
    /// - 持久 mmap：开启
    /// - 全内存索引：开启
    pub fn extreme() -> Self {
        Self {
            aggressive: AggressiveConfig::extreme(),
            ..Default::default()
        }
    }
}

impl Default for FileKVConfig {
    fn default() -> Self {
        Self {
            memtable: MemTableConfig::default(),
            segment_dir: PathBuf::from("./segments"),
            enable_wal: true,
            wal_dir: PathBuf::from("./wal"),
            index_dir: PathBuf::from("./index"),
            cache: BlockCacheConfig::default(),
            enable_bloom: true,
            compaction: CompactionConfig::default(),
            enable_background_flush: true,
            background_flush_interval_ms: 100,
            segment_preallocate_size: 16 * 1024 * 1024,
            block_size: 8192,
            block_compression: BlockCompressionConfig::default(),
            // P1-013: WAL rotation defaults
            wal_max_size_bytes: 100 * 1024 * 1024, // 100MB
            wal_max_files: 5,
            // P2-004: Cache warming enabled by default for better read performance
            cache_warming_enabled: true,
            // P2-014: Dictionary compression enabled by default for better storage efficiency
            compression: DictionaryCompressionConfig::default(),
            // P3-001: Async I/O disabled by default (opt-in for production use)
            async_io_enabled: false,
            async_io_max_concurrent_writes: 4,
            async_io_max_queue_depth: 1024,
            async_io_write_timeout_ms: 5000,
            async_io_enable_coalescing: true,
            async_io_coalesce_window_ms: 10,
            // P2-009: Checkpoint directory default
            checkpoint_dir: PathBuf::from("./checkpoints"),
            // P2-013: Audit log disabled by default (opt-in for compliance)
            audit_log: AuditLogConfig::default(),
            // P4-001: Aggressive optimizations - balanced mode by default
            aggressive: AggressiveConfig::balanced(),
            // INNO-001: Adaptive Bloom cache enabled by default
            enable_adaptive_bloom_cache: true,
            // INNO-002: Zone Map pruning and prefetching enabled by default
            enable_zone_map_pruning: true,
            enable_sequential_prefetch: true,
            // OPT-007: Multi-level cache enabled by default
            enable_multi_level_cache: true,
            l2_cache_max_bytes: 4 * 1024 * 1024 * 1024, // 4GB
            l2_to_l1_threshold: 5,
            // OPT-007: WAL channel batching disabled by default (backward compatible)
            enable_wal_channel: false,
            wal_channel_interval_ms: 2,
            wal_channel_max_entries: 1000,
            wal_channel_capacity: 10_000,
            // Phase 1: Default to StdFs
            fs: Arc::new(StdFs),
        }
    }
}

/// FileKV 统计信息快照（用于返回）
#[derive(Debug, Clone, Default)]
pub struct FileKVStatsSnapshot {
    pub memtable_size: usize,
    pub memtable_entries: usize,
    pub segment_count: usize,
    pub total_size_bytes: u64,
    pub total_entries: u64,
    pub write_count: u64,
    pub read_count: u64,
    pub flush_count: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub bloom_filtered: u64,
    pub compaction_runs: u64,
    pub compaction_segments_merged: u64,
    pub compaction_tombstones_removed: u64,
    // Amplification statistics
    pub user_bytes_written: u64,
    pub total_bytes_written_all: u64,
    pub write_amplification_factor: f64,
    pub read_io_operations: u64,
    pub total_bytes_read: u64,
    pub read_amplification_factor: f64,
    pub space_amplification_factor: f64,
    // P2-014: Compression statistics
    pub compression_dict_trained: bool,
    pub compression_dict_size: usize,
    pub compression_ratio: f64,
    pub compressed_writes: u64,
    pub uncompressed_bytes: u64,
    pub compressed_bytes: u64,
    // FIX-001: Prefetch statistics
    pub prefetch_hits: u64,
}

/// FileKV 统计信息（使用原子计数器，无锁）
#[derive(Debug, Default)]
pub struct FileKVStats {
    pub memtable_size: AtomicUsize,
    pub memtable_entries: AtomicUsize,
    pub segment_count: AtomicUsize,
    pub total_size_bytes: AtomicU64,
    pub total_entries: AtomicU64,
    pub write_count: AtomicU64,
    pub read_count: AtomicU64,
    pub flush_count: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    pub bloom_filtered: AtomicU64,
    pub compaction_runs: AtomicU64,
    pub compaction_segments_merged: AtomicU64,
    pub compaction_tombstones_removed: AtomicU64,
    // Amplification statistics
    pub user_bytes_written: AtomicU64,
    pub wal_bytes_written: AtomicU64,
    pub index_bytes_written: AtomicU64,
    pub segment_bytes_written: AtomicU64,
    pub read_io_operations: AtomicU64,
    pub total_bytes_read: AtomicU64,
    // P2-014: Compression statistics
    pub compression_dict_trained: AtomicBool,
    pub compression_dict_size: AtomicUsize,
    pub compressed_writes: AtomicU64,
    pub uncompressed_bytes: AtomicU64,
    pub compressed_bytes: AtomicU64,
    // FIX-001: Prefetch statistics
    pub prefetch_hits: AtomicU64,
}

impl Clone for FileKVStats {
    fn clone(&self) -> Self {
        Self {
            memtable_size: AtomicUsize::new(self.memtable_size.load(Ordering::Relaxed)),
            memtable_entries: AtomicUsize::new(self.memtable_entries.load(Ordering::Relaxed)),
            segment_count: AtomicUsize::new(self.segment_count.load(Ordering::Relaxed)),
            total_size_bytes: AtomicU64::new(self.total_size_bytes.load(Ordering::Relaxed)),
            total_entries: AtomicU64::new(self.total_entries.load(Ordering::Relaxed)),
            write_count: AtomicU64::new(self.write_count.load(Ordering::Relaxed)),
            read_count: AtomicU64::new(self.read_count.load(Ordering::Relaxed)),
            flush_count: AtomicU64::new(self.flush_count.load(Ordering::Relaxed)),
            cache_hits: AtomicU64::new(self.cache_hits.load(Ordering::Relaxed)),
            cache_misses: AtomicU64::new(self.cache_misses.load(Ordering::Relaxed)),
            bloom_filtered: AtomicU64::new(self.bloom_filtered.load(Ordering::Relaxed)),
            compaction_runs: AtomicU64::new(self.compaction_runs.load(Ordering::Relaxed)),
            compaction_segments_merged: AtomicU64::new(self.compaction_segments_merged.load(Ordering::Relaxed)),
            compaction_tombstones_removed: AtomicU64::new(self.compaction_tombstones_removed.load(Ordering::Relaxed)),
            user_bytes_written: AtomicU64::new(self.user_bytes_written.load(Ordering::Relaxed)),
            wal_bytes_written: AtomicU64::new(self.wal_bytes_written.load(Ordering::Relaxed)),
            index_bytes_written: AtomicU64::new(self.index_bytes_written.load(Ordering::Relaxed)),
            segment_bytes_written: AtomicU64::new(self.segment_bytes_written.load(Ordering::Relaxed)),
            read_io_operations: AtomicU64::new(self.read_io_operations.load(Ordering::Relaxed)),
            total_bytes_read: AtomicU64::new(self.total_bytes_read.load(Ordering::Relaxed)),
            compression_dict_trained: AtomicBool::new(self.compression_dict_trained.load(Ordering::Relaxed)),
            compression_dict_size: AtomicUsize::new(self.compression_dict_size.load(Ordering::Relaxed)),
            compressed_writes: AtomicU64::new(self.compressed_writes.load(Ordering::Relaxed)),
            uncompressed_bytes: AtomicU64::new(self.uncompressed_bytes.load(Ordering::Relaxed)),
            compressed_bytes: AtomicU64::new(self.compressed_bytes.load(Ordering::Relaxed)),
            prefetch_hits: AtomicU64::new(self.prefetch_hits.load(Ordering::Relaxed)),
        }
    }
}

impl FileKVStats {
    /// Get a snapshot of statistics
    pub fn snapshot(&self) -> FileKVStatsSnapshot {
        let uncompressed = self.uncompressed_bytes.load(Ordering::Relaxed) as f64;
        let compressed = self.compressed_bytes.load(Ordering::Relaxed) as f64;
        let ratio = if uncompressed > 0.0 {
            compressed / uncompressed
        } else {
            1.0
        };

        // Calculate amplification factors
        let user_bytes = self.user_bytes_written.load(Ordering::Relaxed);
        let wal_bytes = self.wal_bytes_written.load(Ordering::Relaxed);
        let index_bytes = self.index_bytes_written.load(Ordering::Relaxed);
        let segment_bytes = self.segment_bytes_written.load(Ordering::Relaxed);
        let total_written = user_bytes + wal_bytes + index_bytes + segment_bytes;
        let waf = if user_bytes > 0 {
            total_written as f64 / user_bytes as f64
        } else {
            1.0
        };

        let read_ops = self.read_io_operations.load(Ordering::Relaxed);
        let read_count = self.read_count.load(Ordering::Relaxed);
        let raf = if read_count > 0 {
            read_ops as f64 / read_count as f64
        } else {
            1.0
        };

        let total_size = self.total_size_bytes.load(Ordering::Relaxed);
        let saf = if user_bytes > 0 {
            total_size as f64 / user_bytes as f64
        } else {
            1.0
        };

        FileKVStatsSnapshot {
            memtable_size: self.memtable_size.load(Ordering::Relaxed),
            memtable_entries: self.memtable_entries.load(Ordering::Relaxed),
            segment_count: self.segment_count.load(Ordering::Relaxed),
            total_size_bytes: self.total_size_bytes.load(Ordering::Relaxed),
            total_entries: self.total_entries.load(Ordering::Relaxed),
            write_count: self.write_count.load(Ordering::Relaxed),
            read_count: self.read_count.load(Ordering::Relaxed),
            flush_count: self.flush_count.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
            bloom_filtered: self.bloom_filtered.load(Ordering::Relaxed),
            compaction_runs: self.compaction_runs.load(Ordering::Relaxed),
            compaction_segments_merged: self.compaction_segments_merged.load(Ordering::Relaxed),
            compaction_tombstones_removed: self.compaction_tombstones_removed.load(Ordering::Relaxed),
            user_bytes_written: user_bytes,
            total_bytes_written_all: total_written,
            write_amplification_factor: waf,
            read_io_operations: read_ops,
            total_bytes_read: self.total_bytes_read.load(Ordering::Relaxed),
            read_amplification_factor: raf,
            space_amplification_factor: saf,
            compression_dict_trained: self.compression_dict_trained.load(Ordering::Relaxed),
            compression_dict_size: self.compression_dict_size.load(Ordering::Relaxed),
            compressed_writes: self.compressed_writes.load(Ordering::Relaxed),
            uncompressed_bytes: uncompressed as u64,
            compressed_bytes: compressed as u64,
            compression_ratio: ratio,
            prefetch_hits: self.prefetch_hits.load(Ordering::Relaxed),
        }
    }
}

/// Bloom Filter 文件魔法数 (exported for bloom module)
pub const BLOOM_MAGIC_PUB: u32 = BLOOM_MAGIC;
/// Bloom Filter 文件版本 (exported for bloom module)
pub const BLOOM_VERSION_PUB: u32 = BLOOM_VERSION;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggressive_config_presets() {
        // Test conservative preset
        let conservative = AggressiveConfig::conservative();
        assert!(!conservative.dense_index_enabled);
        assert_eq!(conservative.readahead_multiplier, 0);
        assert_eq!(conservative.wal_sync_mode, WalSyncMode::Immediate);
        assert_eq!(conservative.cache_max_memory_bytes, 64 * 1024 * 1024);
        assert!(!conservative.persistent_mmap_enabled);
        assert!(!conservative.in_memory_block_index_enabled);

        // Test balanced preset
        let balanced = AggressiveConfig::balanced();
        assert!(balanced.dense_index_enabled);
        assert_eq!(balanced.readahead_multiplier, 2);
        assert_eq!(balanced.wal_sync_mode, WalSyncMode::Batch);
        assert_eq!(balanced.cache_max_memory_bytes, 256 * 1024 * 1024);
        assert!(balanced.persistent_mmap_enabled);
        assert!(!balanced.in_memory_block_index_enabled);

        // Test performance preset
        let performance = AggressiveConfig::performance();
        assert!(performance.dense_index_enabled);
        assert_eq!(performance.readahead_multiplier, 4);
        assert_eq!(performance.wal_sync_mode, WalSyncMode::Batch);
        assert_eq!(performance.cache_max_memory_bytes, 1024 * 1024 * 1024);
        assert!(performance.persistent_mmap_enabled);
        assert!(performance.in_memory_block_index_enabled);

        // Test extreme preset
        let extreme = AggressiveConfig::extreme();
        assert!(extreme.dense_index_enabled);
        assert_eq!(extreme.readahead_multiplier, 8);
        assert_eq!(extreme.wal_sync_mode, WalSyncMode::Lazy);
        assert_eq!(extreme.cache_max_memory_bytes, 4 * 1024 * 1024 * 1024);
        assert!(extreme.persistent_mmap_enabled);
        assert!(extreme.in_memory_block_index_enabled);
    }

    #[test]
    fn test_filekv_config_presets() {
        // Test conservative preset
        let conservative = FileKVConfig::conservative();
        assert!(!conservative.aggressive.dense_index_enabled);
        assert_eq!(conservative.aggressive.wal_sync_mode, WalSyncMode::Immediate);

        // Test balanced preset (default)
        let balanced = FileKVConfig::balanced();
        assert!(balanced.aggressive.dense_index_enabled);
        assert_eq!(balanced.aggressive.wal_sync_mode, WalSyncMode::Batch);

        // Test performance preset
        let performance = FileKVConfig::performance();
        assert!(performance.aggressive.in_memory_block_index_enabled);

        // Test extreme preset
        let extreme = FileKVConfig::extreme();
        assert_eq!(extreme.aggressive.wal_sync_mode, WalSyncMode::Lazy);
    }

    #[test]
    fn test_memory_usage_estimate() {
        let config = AggressiveConfig::performance();
        let estimate = config.estimated_memory_usage(1_000_000);

        // Should have at least BlockCache
        assert!(estimate.total_bytes > 0);
        assert!(!estimate.breakdown.is_empty());

        // Verify display format
        let display = format!("{}", estimate);
        assert!(display.contains("Total:"));
        assert!(display.contains("MB"));
    }

    #[test]
    fn test_wal_sync_mode_default() {
        // Default is Lazy for performance
        assert_eq!(WalSyncMode::default(), WalSyncMode::Lazy);
    }
}
