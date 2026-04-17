# API 参考文档 (API Reference)

**版本**: v0.5.0  
**最后更新**: 2026-04-16  
**状态**: 活跃维护

---

## 目录

1. [核心 API](#1-核心-api)
   - 1.1 FileKV
2. [配置 API](#2-配置-api)
3. [缓存 API](#3-缓存-api) - 核心BlockCache, [完整缓存API见补充文档§5](./supplemental-api.md#§5-缓存-api-补充)
4. [压缩 API](#4-压缩-api) - CompressionStrategy, DictionaryCompressor
5. [范围扫描 API](#5-范围扫描-api) - RangeScanIterator
6. [Bloom Filter 生态 API](#6-bloom-filter-生态-api)
7. [检查点 API](#9-检查点-api)
8. [运维与可观测性 API](#10-运维与可观测性-api)
9. [I/O 抽象 API](#11-io-抽象-api) - 概述, [完整I/O API见补充文档§12](./supplemental-api.md#§12-io-抽象-api)
10. [错误类型](#12-错误类型) - 概述, [完整错误体系见补充文档§13](./supplemental-api.md#§13-错误类型)
11. [Feature Flags](#13-feature-flags)

**补充文档章节** (详见 [supplemental-api.md](./supplemental-api.md)):
- §3 核心存储模块 (MemTable, SegmentFile, SparseIndex, WriteCoalescer, FlushTrigger)
- §5 缓存API补充 (UnifiedCacheConfig, CacheBudget, L2CacheManager, Rebalance, CacheWarmer, SequentialPrefetcher)
- §8 Compaction系统API (CompactionConfig, CompactionTrigger, MergeIterator, SegmentIterator)
- §12 I/O抽象API完整 (FileKVFile, MmapFileSystem, StdFs/StdFile/StdMmap, FaultInjector 5种策略)
- §13 错误体系完整 (FatalError, TransientError, ExpectedError, DomainError四层体系)

**快速导航**:
- 📄 [补充文档](./supplemental-api.md) - 包含引擎层、全局索引、Compaction、I/O抽象、错误体系的完整API详情
- 📖 [API_STABILITY.md](./API_STABILITY.md) - API稳定性承诺
- 🏗️ [架构文档](./architecture/) - 系统架构设计

---

## 1. 核心 API

### 1.1 FileKV

主存储引擎类型。

```rust
use tokitai_filekv::{FileKV, FileKVConfig};
```

#### 构造方法

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `open` | `fn open(config: FileKVConfig) -> anyhow::Result<Self>` | 创建或打开存储 | ✅ 稳定 |

#### 基本操作

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `put` | `fn put(&self, key: &str, value: &[u8]) -> anyhow::Result<()>` | 写入键值对 | ✅ 稳定 |
| `put_with_durability` | `fn put_with_durability(&self, key: &str, value: &[u8], durability: Durability) -> anyhow::Result<()>` | 按持久性级别写入 | ✅ 稳定 |
| `put_batch` | `fn put_batch(&self, entries: &[(&str, &[u8])]) -> anyhow::Result<()>` | 批量写入 | ✅ 稳定 |
| `get` | `fn get(&self, key: &str) -> anyhow::Result<Option<Bytes>>` | 读取键值对 | ✅ 稳定 |
| `delete` | `fn delete(&self, key: &str) -> anyhow::Result<()>` | 删除键值对 | ✅ 稳定 |
| `delete_with_durability` | `fn delete_with_durability(&self, key: &str, durability: Durability) -> anyhow::Result<()>` | 按持久性级别删除 | ✅ 稳定 |

#### 异步操作 (feature: `async-io`)

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `put_async` | `async fn put_async(&self, key: &str, value: &[u8]) -> anyhow::Result<()>` | 异步写入 | ⚠️ 实验 |
| `delete_async` | `async fn delete_async(&self, key: &str) -> anyhow::Result<()>` | 异步删除 | ⚠️ 实验 |
| `flush_async` | `async fn flush_async(&self) -> anyhow::Result<()>` | 异步刷盘 | ⚠️ 实验 |

#### 范围操作

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `range` | `fn range(&self, start: &str, end: &str) -> FileKVResult<RangeScanIterator<'_>>` | 范围扫描 | ✅ 稳定 |
| `range_with_config` | `fn range_with_config(&self, config: RangeScanConfig) -> FileKVResult<RangeScanIterator<'_>>` | 配置范围扫描 | ✅ 稳定 |
| `range_collect` | `fn range_collect(&self, start: &str, end: &str, limit: usize) -> FileKVResult<Vec<(String, Vec<u8>)>>` | 范围扫描收集 | ✅ 稳定 |

#### 管理操作

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `flush_memtable` | `fn flush_memtable(&self) -> anyhow::Result<()>` | 刷盘 | ✅ 稳定 |
| `run_compaction` | `fn run_compaction(&self) -> anyhow::Result<CompactionStats>` | 手动压缩 | ✅ 稳定 |
| `start_background_compaction` | `fn start_background_compaction(self: &Arc<Self>) -> anyhow::Result<()>` | 启动后台压缩 | ⚠️ 实验 |
| `recover` | `fn recover(&self) -> FileKVResult<usize>` | WAL 恢复 | ✅ 稳定 |

#### 统计查询

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `get_stats` | `fn get_stats(&self) -> FileKVStatsSnapshot` | 获取统计 | ✅ 稳定 |
| `get_config` | `fn get_config(&self) -> &FileKVConfig` | 获取配置 | ✅ 稳定 |
| `get_memory_usage` | `fn get_memory_usage(&self) -> MemoryUsage` | 内存使用 | ✅ 稳定 |
| `get_amplification_stats` | `fn get_amplification_stats(&self) -> AmplificationStats` | 放大统计 | ✅ 稳定 |
| `get_global_index_stats` | `fn get_global_index_stats(&self) -> IndexStats` | 全局索引统计 | ⚠️ 实验 |
| `get_bloom_migration_stats` | `fn get_bloom_migration_stats(&self) -> MigrationStats` | Bloom 迁移统计 | ⚠️ 实验 |

#### 内部引用访问 (高级用户)

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `segments` | `fn segments(&self) -> &ArcSwap<BTreeMap<u64, Arc<SegmentFile>>>` | 访问段文件映射 | 🔒 内部 |
| `index_manager_ref` | `fn index_manager_ref(&self) -> &RwLock<IndexManager>` | 访问稀疏索引管理器 | 🔒 内部 |
| `write_coaleser_ref` | `fn write_coaleser_ref(&self) -> &Arc<WriteCoalescer>` | 访问写入合并器 | 🔒 内部 |
| `wal_ref` | `fn wal_ref(&self) -> Option<&Arc<Mutex<WalManager>>>` | 访问 WAL 管理器 | 🔒 内部 |
| `memtable_ref` | `fn memtable_ref(&self) -> &Arc<MemTable>` | 访问内存表 | 🔒 内部 |
| `block_cache_ref` | `fn block_cache_ref(&self) -> &Arc<BlockCache>` | 访问块缓存 | 🔒 内部 |
| `bloom_filter_cache_ref` | `fn bloom_filter_cache_ref(&self) -> &Arc<BloomFilterCache>` | 访问 Bloom 过滤器缓存 | 🔒 内部 |
| `unified_cache_ref` | `fn unified_cache_ref(&self) -> Option<&Arc<UnifiedCacheManager>>` | 访问统一缓存管理器 | 🔒 内部 |
| `load_bloom_filter` | `fn load_bloom_filter(&self, segment_id: u64) -> anyhow::Result<Option<FilterWrapper>>` | 加载 Bloom 过滤器 | ⚠️ 实验 |

#### 运行时配置

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `get_timeout_config` | `fn get_timeout_config(&self) -> MutexGuard<TimeoutConfig>` | 获取超时配置 | ✅ 稳定 |
| `set_timeout_config` | `fn set_timeout_config(&self, config: TimeoutConfig)` | 设置超时配置 | ✅ 稳定 |
| `get_timeout_stats` | `fn get_timeout_stats(&self) -> TimeoutStats` | 超时统计 | ✅ 稳定 |
| `reset_timeout_stats` | `fn reset_timeout_stats(&self)` | 重置超时统计 | ✅ 稳定 |

#### 功能开关

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `enable_inno001` | `fn enable_inno001(&self)` | 启用自适应 Bloom 缓存 | ⚠️ 实验 |
| `disable_inno001` | `fn disable_inno001(&self)` | 禁用自适应 Bloom 缓存 | ⚠️ 实验 |
| `enable_inno002` | `fn enable_inno002(&self)` | 启用 Zone Map | ⚠️ 实验 |
| `disable_inno002` | `fn disable_inno002(&self)` | 禁用 Zone Map | ⚠️ 实验 |
| `get_feature_flag_controller` | `fn get_feature_flag_controller(&self) -> Arc<FeatureFlagController>` | 获取控制器 | ⚠️ 实验 |
| `get_feature_flag_stats` | `fn get_feature_flag_stats(&self) -> FeatureFlagStats` | 获取统计 | ⚠️ 实验 |
| `generate_feature_flag_report` | `fn generate_feature_flag_report(&self) -> FeatureReport` | 生成报告 | ⚠️ 实验 |

#### 预分配器

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `get_next_preallocate_size` | `fn get_next_preallocate_size(&self) -> u64` | 获取预分配大小 | ⚠️ 实验 |
| `get_preallocator_stats` | `fn get_preallocator_stats(&self) -> Option<PreallocatorStats>` | 获取预分配器统计 | ⚠️ 实验 |

---

## 2. 配置 API

### 2.1 FileKVConfig

主配置类型。

```rust
use tokitai_filekv::FileKVConfig;
```

#### 字段

| 字段 | 类型 | 说明 | 默认值 | 稳定性 |
|------|------|------|--------|--------|
| `fs` | `Arc<dyn FileKVFileSystem>` | 文件系统抽象 | `StdFs` | ⚠️ 实验 |
| `segment_dir` | `PathBuf` | 段文件目录 | `./segments` | ✅ 稳定 |
| `wal_dir` | `PathBuf` | WAL 目录 | `./wal` | ✅ 稳定 |
| `index_dir` | `PathBuf` | 索引目录 | `./index` | ✅ 稳定 |
| `checkpoint_dir` | `PathBuf` | 检查点目录 | `./checkpoints` | ✅ 稳定 |
| `enable_wal` | `bool` | 启用 WAL | `true` | ✅ 稳定 |
| `enable_multi_level_cache` | `bool` | 启用多级缓存 | `true` | ✅ 稳定 |
| `enable_adaptive_bloom_cache` | `bool` | 启用自适应 Bloom 缓存 | `false` | ⚠️ 实验 |
| `enable_background_flush` | `bool` | 启用后台刷盘 | `true` | ✅ 稳定 |
| `background_flush_interval_ms` | `u64` | 刷盘间隔 (ms) | `1000` | ✅ 稳定 |
| `segment_preallocate_size` | `u64` | 段预分配大小 | `0` (禁用) | ⚠️ 实验 |
| `l2_cache_max_bytes` | `u64` | L2 缓存大小 | `256MB` | ⚠️ 实验 |
| `l2_to_l1_threshold` | `u64` | L2→L1 迁移阈值 | `10` | ⚠️ 实验 |
| `block_size` | `usize` | 块大小 | `4096` | ✅ 稳定 |
| `enable_bloom` | `bool` | 启用 Bloom Filter | `true` | ✅ 稳定 |
| `aggressive` | `AggressiveConfig` | 激进优化配置 | 默认 | ✅ 稳定 |
| `cache` | `BlockCacheConfig` | 块缓存配置 | 默认 | ✅ 稳定 |
| `memtable` | `MemTableConfig` | MemTable 配置 | 默认 | ✅ 稳定 |
| `compression` | `DictionaryCompressionConfig` | 压缩配置 | 默认 | ✅ 稳定 |
| `compaction` | `CompactionConfig` | 压缩配置 | 默认 | ✅ 稳定 |

#### 方法

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `validate` | `fn validate(&self) -> FileKVConfigValidation` | 验证配置 | ✅ 稳定 |
| `default` | `fn default() -> Self` | 默认配置 | ✅ 稳定 |

### 2.2 AggressiveConfig

激进优化配置。

```rust
use tokitai_filekv::AggressiveConfig;
```

#### 字段

| 字段 | 类型 | 说明 | 默认值 | 稳定性 |
|------|------|------|--------|--------|
| `persistent_mmap_enabled` | `bool` | 持久化 mmap | `false` | ✅ 稳定 |
| `readahead_multiplier` | `u32` | 预读倍数 | `2` | ✅ 稳定 |
| `dense_index_enabled` | `bool` | DenseIndex | `true` | ✅ 稳定 |
| `cache_max_memory_bytes` | `usize` | 缓存最大内存 | `512MB` | ✅ 稳定 |

### 2.3 Durability

持久性级别。

```rust
use tokitai_filekv::Durability;
```

| 变体 | 说明 | WAL 同步 | 稳定性 |
|------|------|---------|--------|
| `Relaxed` | 宽松模式 | 异步 | ✅ 稳定 |
| `Standard` | 标准模式 | 同步 | ✅ 稳定 |
| `Strict` | 严格模式 | fsync | ✅ 稳定 |

### 2.4 WalSyncMode

WAL 同步模式。

```rust
use tokitai_filekv::WalSyncMode;
```

| 变体 | 说明 | 性能 | 安全性 | 稳定性 |
|------|------|------|--------|--------|
| `Async` | 异步写入 | 最快 | 最低 | ✅ 稳定 |
| `Sync` | 同步写入 | 中等 | 中等 | ✅ 稳定 |
| `FsSync` | fsync 同步 | 最慢 | 最高 | ✅ 稳定 |

### 2.5 BlockCompressionMode

块压缩模式。

```rust
use tokitai_filekv::BlockCompressionMode;
```

| 变体 | 说明 | 压缩比 | 速度 | 稳定性 |
|------|------|--------|------|--------|
| `None` | 不压缩 | 1:1 | 最快 | ✅ 稳定 |
| `Zstd { level: i32 }` | Zstandard | 高 | 中等 | ✅ 稳定 |
| `Snappy` | Snappy | 中 | 快 | ✅ 稳定 |
| `Lz4 { level: i32 }` | LZ4 | 中 | 最快 | ✅ 稳定 |

---

## 3. 缓存 API

> **注意**: 本节仅包含核心BlockCache API。完整缓存API (UnifiedCacheConfig, CacheBudget, L2CacheManager, Rebalance, CacheWarmer, SequentialPrefetcher等) 请见 [补充文档 §5](./supplemental-api.md#5-缓存-api-补充)。

### 3.1 BlockCache

块缓存（Moka TinyLFU）。

```rust
use tokitai_filekv::BlockCache;
```

#### 方法

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new(config: BlockCacheConfig) -> Self` | 创建缓存 | ✅ 稳定 |
| `get` | `fn get(&self, key: &str) -> Option<Vec<u8>>` | 获取值 | ✅ 稳定 |
| `put` | `fn put(&self, key: &str, value: Vec<u8>)` | 插入值 | ✅ 稳定 |
| `stats` | `fn stats(&self) -> CacheStats` | 获取统计 | ✅ 稳定 |
| `clear` | `fn clear(&self)` | 清空缓存 | ✅ 稳定 |

### 3.2 BlockCacheConfig

块缓存配置。

```rust
use tokitai_filekv::BlockCacheConfig;
```

#### 字段

| 字段 | 类型 | 说明 | 默认值 | 稳定性 |
|------|------|------|--------|--------|
| `max_memory_bytes` | `u64` | 最大内存 | `512MB` | ✅ 稳定 |
| `max_items` | `u64` | 最大条目数 | `100_000` | ✅ 稳定 |
| `frequency_aware` | `bool` | 频率感知 | `true` | ✅ 稳定 |
| `time_to_live` | `Option<Duration>` | TTL | `None` | ✅ 稳定 |

### 3.3 CacheStats

缓存统计。

```rust
use tokitai_filekv::CacheStats;
```

#### 字段

| 字段 | 类型 | 说明 | 稳定性 |
|------|------|------|--------|
| `hits` | `u64` | 命中次数 | ✅ 稳定 |
| `misses` | `u64` | 未命中次数 | ✅ 稳定 |
| `evictions` | `u64` | 驱逐次数 | ✅ 稳定 |
| `hit_rate` | `f64` | 命中率 (0.0-1.0) | ✅ 稳定 |
| `memory_bytes` | `u64` | 当前内存使用 | ✅ 稳定 |
| `items` | `u64` | 当前条目数 | ✅ 稳定 |

### 3.4 UnifiedCacheManager

统一缓存管理器。

```rust
use tokitai_filekv::UnifiedCacheManager;
```

#### 方法

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new(config: UnifiedCacheConfig) -> Self` | 创建管理器 | ✅ 稳定 |
| `block_cache` | `fn block_cache(&self) -> &Arc<BlockCache>` | 获取块缓存 | ✅ 稳定 |
| `stats` | `fn stats(&self) -> CacheStats` | 获取统计 | ✅ 稳定 |
| `rebalance` | `fn rebalance(&self)` | 重新平衡 | ⚠️ 实验 |

### 3.5 CacheWarmer

缓存预热器。

```rust
use tokitai_filekv::CacheWarmer;
```

#### 方法

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `warm` | `fn warm(&self, keys: &[String]) -> anyhow::Result<()>` | 预热缓存 | ✅ 稳定 |
| `stats` | `fn stats(&self) -> CacheWarmingStats` | 获取预热统计 | ✅ 稳定 |

---

## 4. 压缩 API

> **注意**: 完整Compaction系统API (CompactionConfig, CompactionTrigger, MergeIterator等) 请见 [补充文档 §8](./supplemental-api.md#8-compaction-系统-api)。

### 4.1 CompressionStrategy

压缩策略 trait。

```rust
use tokitai_filekv::CompressionStrategy;
```

#### 方法

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `compress` | `fn compress(&self, data: &[u8]) -> anyhow::Result<Vec<u8>>` | 压缩 | ✅ 稳定 |
| `decompress` | `fn decompress(&self, data: &[u8]) -> anyhow::Result<Vec<u8>>` | 解压 | ✅ 稳定 |
| `algorithm_id` | `fn algorithm_id(&self) -> CompressionAlgorithmId` | 算法 ID | ✅ 稳定 |

### 4.2 CompressionAlgorithmId

压缩算法 ID。

```rust
use tokitai_filekv::CompressionAlgorithmId;
```

| 变体 | 说明 | 压缩比 | 速度 | 稳定性 |
|------|------|--------|------|--------|
| `None` | 无压缩 | 1:1 | 最快 | ✅ 稳定 |
| `Zstd` | Zstandard | 高 | 中等 | ✅ 稳定 |
| `Snappy` | Snappy | 中 | 快 | ✅ 稳定 |
| `Lz4` | LZ4 | 中 | 最快 | ✅ 稳定 |

### 4.3 create_compressor

创建压缩器工厂函数。

```rust
use tokitai_filekv::create_compressor;

let compressor = create_compressor(&BlockCompressionMode::Zstd { level: 3 }, 3);
```

| 参数 | 类型 | 说明 | 稳定性 |
|------|------|------|--------|
| `mode` | `&BlockCompressionMode` | 压缩模式 | ✅ 稳定 |
| `level` | `i32` | 压缩级别 | ✅ 稳定 |
| **返回** | `Box<dyn CompressionStrategy>` | 压缩器 | ✅ 稳定 |

### 4.4 DictionaryCompressor

字典压缩器。

```rust
use tokitai_filekv::DictionaryCompressor;
```

#### 方法

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new(config: DictionaryCompressionConfig) -> Self` | 创建压缩器 | ✅ 稳定 |
| `compress` | `fn compress(&self, data: &[u8]) -> anyhow::Result<Vec<u8>>` | 压缩 | ✅ 稳定 |
| `decompress` | `fn decompress(&self, data: &[u8]) -> anyhow::Result<Vec<u8>>` | 解压 | ✅ 稳定 |
| `stats` | `fn stats(&self) -> DictionaryStats` | 获取统计 | ✅ 稳定 |

---

## 5. 范围扫描 API

> **注意**: 完整查询优化API (RangeQueryPruner, ZoneMap, SequentialDetector等) 请见 [补充文档 §7](./supplemental-api.md#7-查询优化-api)。

### 5.1 RangeScanIterator

范围扫描迭代器。

```rust
use tokitai_filekv::RangeScanIterator;

let iter = kv.range("key1", "key100")?;
for (key, value) in iter {
    println!("{}: {:?}", key, value);
}
```

#### 实现

| Trait | 说明 | 稳定性 |
|-------|------|--------|
| `Iterator` | `Item = (String, Vec<u8>)` | ✅ 稳定 |

### 5.2 RangeScanConfig

范围扫描配置。

```rust
use tokitai_filekv::RangeScanConfig;
```

#### 字段

| 字段 | 类型 | 说明 | 默认值 | 稳定性 |
|------|------|------|--------|--------|
| `include_start` | `bool` | 包含起始键 | `true` | ✅ 稳定 |
| `include_end` | `bool` | 包含结束键 | `true` | ✅ 稳定 |
| `limit` | `Option<usize>` | 限制返回数 | `None` | ✅ 稳定 |
| `reverse` | `bool` | 反向扫描 | `false` | ✅ 稳定 |

### 5.3 RangeEntry

范围扫描条目。

```rust
use tokitai_filekv::RangeEntry;
```

#### 字段

| 字段 | 类型 | 说明 | 稳定性 |
|------|------|------|--------|
| `key` | `String` | 键 | ✅ 稳定 |
| `value` | `Vec<u8>` | 值 | ✅ 稳定 |

### 5.4 RangeScanStats

范围扫描统计。

```rust
use tokitai_filekv::RangeScanStats;
```

#### 字段

| 字段 | 类型 | 说明 | 稳定性 |
|------|------|------|--------|
| `scanned_entries` | `u64` | 扫描条目数 | ✅ 稳定 |
| `returned_entries` | `u64` | 返回条目数 | ✅ 稳定 |
| `blocks_scanned` | `u64` | 扫描块数 | ✅ 稳定 |
| `blocks_pruned` | `u64` | 剪枝块数 | ✅ 稳定 |

---

## 6. Bloom Filter 生态 API

### 6.1 AdaptiveBloomCache

自适应 Bloom Filter 缓存，实现 L1/L2/L3 多层架构。

```rust
use tokitai_filekv::{AdaptiveBloomCache, AdaptiveBloomCacheConfig, AdaptiveBloomCacheStats, CacheLayer};
```

#### 架构

```
Query Flow:
  ┌─────────────┐
  │ Query Key   │
  └──────┬──────┘
         │
         ▼
  ┌─────────────┐
  │   L1 Cache  │ (Hot, ~1000 filters, FPR 0.1%, <100ns)
  │  CLOCK      │
  └──────┬──────┘
         │ Miss
         ▼
  ┌─────────────┐
  │   L2 Cache  │ (Warm, ~10000 filters, FPR 1%, ~500ns)
  │ Compressed  │
  └──────┬──────┘
         │ Miss
         ▼
  ┌─────────────┐
  │   L3 Store  │ (Cold, disk-based, FPR 10%, ~10µs)
  │  On-demand  │
  └─────────────┘
```

#### AdaptiveBloomCacheConfig 字段

| 字段 | 类型 | 说明 | 默认值 | 稳定性 |
|------|------|------|--------|--------|
| `l1_max_filters` | `usize` | L1 缓存最大过滤器数 | `1_000` | ⚠️ 实验 |
| `l2_max_filters` | `usize` | L2 缓存最大过滤器数 | `10_000` | ⚠️ 实验 |
| `l1_fpr_target` | `f64` | L1 FPR 目标 | `0.001` (0.1%) | ⚠️ 实验 |
| `l2_fpr_target` | `f64` | L2 FPR 目标 | `0.01` (1%) | ⚠️ 实验 |
| `l3_fpr_target` | `f64` | L3 FPR 目标 | `0.1` (10%) | ⚠️ 实验 |
| `l2_compression_enabled` | `bool` | L2 压缩开关 | `true` | ⚠️ 实验 |
| `l3_index_dir` | `PathBuf` | L3 索引目录 | `./index` | ⚠️ 实验 |

#### AdaptiveBloomCacheStats 字段

| 字段 | 类型 | 说明 | 稳定性 |
|------|------|------|--------|
| `l1_hits` | `u64` | L1 命中次数 | ⚠️ 实验 |
| `l2_hits` | `u64` | L2 命中次数 | ⚠️ 实验 |
| `l3_hits` | `u64` | L3 命中次数 (从磁盘加载) | ⚠️ 实验 |
| `total_misses` | `u64` | 总未命中次数 | ⚠️ 实验 |
| `l1_to_l2_migrations` | `u64` | L1→L2 迁移次数 | ⚠️ 实验 |
| `l2_to_l1_migrations` | `u64` | L2→L1 迁移次数 | ⚠️ 实验 |
| `l2_to_l3_migrations` | `u64` | L2→L3 迁移次数 (驱逐) | ⚠️ 实验 |
| `l3_to_l2_migrations` | `u64` | L3→L2 迁移次数 (加载) | ⚠️ 实验 |
| `l1_cache_size` | `usize` | L1 当前大小 | ⚠️ 实验 |
| `l2_cache_size` | `usize` | L2 当前大小 | ⚠️ 实验 |
| `l1_memory_used` | `usize` | L1 内存使用 (字节) | ⚠️ 实验 |
| `l2_memory_used` | `usize` | L2 内存使用 (压缩后, 字节) | ⚠️ 实验 |
| `hit_rate` | `f64` | 总命中率 (0.0-1.0) | ⚠️ 实验 |

#### 辅助方法

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `hit_rate_percent` | `fn hit_rate_percent(&self) -> f64` | 获取命中率百分比 | ⚠️ 实验 |
| `l1_hit_rate_percent` | `fn l1_hit_rate_percent(&self) -> f64` | L1 命中率百分比 | ⚠️ 实验 |
| `l2_hit_rate_percent` | `fn l2_hit_rate_percent(&self) -> f64` | L2 命中率百分比 | ⚠️ 实验 |
| `total_memory_mb` | `fn total_memory_mb(&self) -> f64` | 总内存使用 (MB) | ⚠️ 实验 |

#### CacheLayer 枚举

| 变体 | 说明 | 稳定性 |
|------|------|--------|
| `L1` | 热缓存 (最快, 最低 FPR) | ⚠️ 实验 |
| `L2` | 温缓存 (压缩, 中等 FPR) | ⚠️ 实验 |
| `L3` | 冷存储 (磁盘, 最高 FPR) | ⚠️ 实验 |

### 6.2 BloomFilterCache

Bloom Filter 按需加载缓存，使用 CLOCK 驱逐算法。

```rust
use tokitai_filekv::{BloomFilterCache, BloomFilterCacheConfig, BloomFilterCacheStats};
```

#### BloomFilterCacheConfig 字段

| 字段 | 类型 | 说明 | 默认值 | 稳定性 |
|------|------|------|--------|--------|
| `max_filters` | `usize` | 最大缓存过滤器数 | `1000` | ⚠️ 实验 |
| `max_memory_bytes` | `usize` | 最大内存使用 (字节) | `256MB` | ⚠️ 实验 |
| `on_demand_enabled` | `bool` | 启用按需加载 | `true` | ⚠️ 实验 |

#### BloomFilterCacheStats 字段

| 字段 | 类型 | 说明 | 稳定性 |
|------|------|------|--------|
| `hits` | `u64` | 缓存命中次数 | ⚠️ 实验 |
| `misses` | `u64` | 缓存未命中次数 | ⚠️ 实验 |
| `hit_rate` | `f64` | 命中率 (0.0-1.0) | ⚠️ 实验 |
| `filters_cached` | `usize` | 当前缓存的过滤器数 | ⚠️ 实验 |
| `memory_used` | `usize` | 内存使用 (字节) | ⚠️ 实验 |
| `evictions` | `u64` | 驱逐次数 | ⚠️ 实验 |
| `loads` | `u64` | 从磁盘加载次数 | ⚠️ 实验 |

#### 方法

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new(config: BloomFilterCacheConfig, index_dir: PathBuf) -> Self` | 创建缓存 | ⚠️ 实验 |
| `get` | `fn get(&self, segment_id: u64, loader: &dyn Fn(u64) -> FileKVResult<Option<FilterWrapper>>) -> FileKVResult<Option<Arc<FilterWrapper>>>` | 获取过滤器 (按需加载) | ⚠️ 实验 |
| `insert` | `fn insert(&self, segment_id: u64, filter: FilterWrapper)` | 插入过滤器 | ⚠️ 实验 |
| `contains` | `fn contains(&self, segment_id: u64, key: &str, loader: &dyn Fn(u64) -> FileKVResult<Option<FilterWrapper>>) -> FileKVResult<Option<bool>>` | 检查键是否存在 | ⚠️ 实验 |
| `remove` | `fn remove(&self, segment_id: u64) -> Option<Arc<FilterWrapper>>` | 移除过滤器 | ⚠️ 实验 |
| `clear` | `fn clear(&self)` | 清空缓存 | ⚠️ 实验 |
| `stats` | `fn stats(&self) -> BloomFilterCacheStats` | 获取统计 | ⚠️ 实验 |
| `len` | `fn len(&self) -> usize` | 获取缓存过滤器数 | ⚠️ 实验 |
| `is_empty` | `fn is_empty(&self) -> bool` | 检查是否为空 | ⚠️ 实验 |
| `shrink_to_memory` | `fn shrink_to_memory(&self, target_memory_bytes: usize) -> u64` | 缩减到目标内存限制，返回驱逐数 | ⚠️ 实验 |
| `grow_max_memory` | `fn grow_max_memory(&self, new_max_memory_bytes: usize) -> usize` | 增加动态最大内存限制 | ⚠️ 实验 |

#### 使用说明

BloomFilterCache 采用 CLOCK 驱逐算法实现近似 LRU 行为，支持 16 个独立分片以减少竞争。过滤器按需从磁盘加载，避免启动时加载所有过滤器。

```rust
// 创建缓存
let config = BloomFilterCacheConfig::default();
let cache = BloomFilterCache::new(config, PathBuf::from("index"));

// 获取过滤器 (自动按需加载)
let filter = cache.get(segment_id, &|id| load_bloom_filter_from_disk(&index_dir, id))?;

// 检查键
let might_exist = cache.contains(segment_id, "my_key", &loader)?;

// 获取统计
let stats = cache.stats();
println!("Hit rate: {:.1}%", stats.hit_rate_percent());
```

### 6.3 CustomBloom

确定性哈希的 Bloom Filter，支持直接序列化/反序列化。

```rust
// 通过 bloom 子模块访问
use tokitai_filekv::bloom::custom_bloom::CustomBloom;
```

> **注意**: `CustomBloom` 当前不在 crate 根级别导出，需通过 `bloom::custom_bloom::CustomBloom` 路径访问。

#### 常量

| 常量 | 类型 | 值 | 说明 | 稳定性 |
|------|------|-----|------|--------|
| `CUSTOM_BLOOM_MAGIC` | `u32` | `0x424C4D33` ("BLM3") | V3 文件格式魔数 | ⚠️ 实验 |
| `CUSTOM_BLOOM_VERSION` | `u32` | `3` | 当前版本号 | ⚠️ 实验 |

#### 构造方法

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new(num_bits: usize, num_hashes: usize) -> Self` | 创建空过滤器 | ⚠️ 实验 |
| `with_capacity` | `fn with_capacity(expected_items: usize, fpr: f64) -> Self` | 按容量和目标 FPR 创建 | ⚠️ 实验 |
| `from_keys` | `fn from_keys(keys: &[String], expected_items: usize, fpr: f64) -> Self` | 从键列表构建 | ⚠️ 实验 |
| `from_bits` | `fn from_bits(num_bits: usize, num_hashes: usize, bits: Vec<u8>) -> Self` | 从原始位向量构建 | ⚠️ 实验 |
| `from_bloom_filter` | `fn from_bloom_filter(bloom: &::bloom::BloomFilter) -> Self` | 从旧版 BloomFilter 迁移 | ⚠️ 实验 |

#### 核心操作

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `insert` | `fn insert(&mut self, key: &[u8])` | 插入键 | ⚠️ 实验 |
| `contains` | `fn contains(&self, key: &[u8]) -> bool` | 检查键 (可能误报, 无假阴性) | ⚠️ 实验 |
| `num_bits` | `fn num_bits(&self) -> usize` | 获取位数 | ⚠️ 实验 |
| `num_hashes` | `fn num_hashes(&self) -> usize` | 获取哈希函数数 | ⚠️ 实验 |
| `to_bytes` | `fn to_bytes(&self) -> &[u8]` | 获取位向量字节 | ⚠️ 实验 |
| `estimated_fpr` | `fn estimated_fpr(&self, num_items: usize) -> f64` | 估算误报率 | ⚠️ 实验 |
| `memory_usage` | `fn memory_usage(&self) -> usize` | 估算内存使用 (字节) | ⚠️ 实验 |

#### 持久化

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `save_to_file` | `fn save_to_file(&self, path: &Path) -> Result<()>` | 保存到 V3 格式文件 | ⚠️ 实验 |
| `load_from_file` | `fn load_from_file(path: &Path) -> Result<Option<Self>>` | 从 V3 格式文件加载 | ⚠️ 实验 |
| `estimated_file_size` | `fn estimated_file_size(&self) -> usize` | 估算文件大小 | ⚠️ 实验 |

#### 算法

CustomBloom 使用双哈希技术模拟 k 个哈希函数:
- `h1(x)`: XXH3 with seed 0
- `h2(x)`: XXH3 with seed 0xDEADBEEF
- `h_i(x) = h1(x) + i * h2(x) mod m`

V3 文件格式: `[magic 4B][version 4B][num_bits 4B][num_hashes 4B][bitset_bytes]`

### 6.4 FPRController

假阳性率 (FPR) 自适应控制器。

```rust
use tokitai_filekv::{FPRController, FPRControllerStats, FPRLevel, AdaptationPolicy, FPRAdjustedBloom};
```

#### FPRLevel 定义

| 级别 | FPR | 内存倍数 | 最低 QPS | 说明 | 稳定性 |
|------|-----|---------|---------|------|--------|
| `LEVEL_0` | 0.1% | 2.0x | 100.0 | 最高精度 (热段) | ⚠️ 实验 |
| `LEVEL_1` | 0.5% | 1.5x | 50.0 | 高精度 | ⚠️ 实验 |
| `LEVEL_2` | 1.0% | 1.0x | 10.0 | 默认精度 | ⚠️ 实验 |
| `LEVEL_3` | 2.0% | 0.75x | 5.0 | 中等精度 | ⚠️ 实验 |
| `LEVEL_4` | 5.0% | 0.5x | 1.0 | 低精度 | ⚠️ 实验 |
| `LEVEL_5` | 10.0% | 0.25x | 0.0 | 最低精度 (冷段) | ⚠️ 实验 |

#### AdaptationPolicy 字段

| 字段 | 类型 | 说明 | 默认值 | 稳定性 |
|------|------|------|--------|--------|
| `min_level` | `u8` | 最低 FPR 级别 (0-5) | `0` | ⚠️ 实验 |
| `max_level` | `u8` | 最高 FPR 级别 (0-5) | `5` | ⚠️ 实验 |
| `hysteresis` | `f64` | 迟滞因子 (防止振荡) | `0.2` | ⚠️ 实验 |
| `stabilization_window_ms` | `u64` | 稳定窗口 (ms) | `120_000` (2分钟) | ⚠️ 实验 |
| `gradual_transitions` | `bool` | 启用渐变过渡 (跳过级别) | `true` | ⚠️ 实验 |

#### FPRControllerStats 字段

| 字段 | 类型 | 说明 | 稳定性 |
|------|------|------|--------|
| `tracked_segments` | `usize` | 跟踪的段数 | ⚠️ 实验 |
| `adjustments_made` | `u64` | FPR 调整次数 | ⚠️ 实验 |
| `upgrades` | `u64` | 升级次数 (更好精度) | ⚠️ 实验 |
| `downgrades` | `u64` | 降级次数 (更低精度) | ⚠️ 实验 |
| `avg_fpr` | `f64` | 平均 FPR | ⚠️ 实验 |
| `memory_saved_bytes` | `u64` | 节省的内存 (字节, 估算) | ⚠️ 实验 |

#### FPRController 方法

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new(policy: AdaptationPolicy, migration_thresholds: MigrationThresholds) -> Self` | 创建控制器 | ⚠️ 实验 |
| `with_defaults` | `fn with_defaults() -> Self` | 使用默认策略创建 | ⚠️ 实验 |
| `get_level` | `fn get_level(&self, segment_id: u64) -> u8` | 获取段的 FPR 级别 | ⚠️ 实验 |
| `get_current_fpr` | `fn get_current_fpr(&self, segment_id: u64) -> f64` | 获取当前 FPR | ⚠️ 实验 |
| `get_target_fpr` | `fn get_target_fpr(&self, segment_id: u64) -> f64` | 获取目标 FPR | ⚠️ 实验 |
| `get_level_info` | `fn get_level_info(&self, level: u8) -> Option<&FPRLevel>` | 获取级别详情 | ⚠️ 实验 |
| `record_access` | `fn record_access(&self, segment_id: u64, access: &AccessRecord) -> Option<u8>` | 记录访问并可能调整 FPR | ⚠️ 实验 |
| `estimate_memory` | `fn estimate_memory(&self, num_elements: usize, level: u8) -> usize` | 估算指定级别的内存 | ⚠️ 实验 |
| `update_memory_estimate` | `fn update_memory_estimate(&self, segment_id: u64, num_elements: usize)` | 更新内存估算 | ⚠️ 实验 |
| `get_memory_estimate` | `fn get_memory_estimate(&self, segment_id: u64) -> usize` | 获取内存估算 | ⚠️ 实验 |
| `get_total_memory` | `fn get_total_memory(&self) -> usize` | 获取所有段总内存 | ⚠️ 实验 |
| `stats` | `fn stats(&self) -> FPRControllerStats` | 获取统计 | ⚠️ 实验 |
| `remove_segment` | `fn remove_segment(&self, segment_id: u64)` | 移除段跟踪 | ⚠️ 实验 |
| `migration_thresholds` | `fn migration_thresholds(&self) -> &MigrationThresholds` | 获取迁移阈值配置 | ⚠️ 实验 |
| `set_policy` | `fn set_policy(&mut self, policy: AdaptationPolicy)` | 更新策略 | ⚠️ 实验 |

#### FPRAdjustedBloom 方法

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new(num_elements: usize, fpr: f64, level: u8) -> Self` | 创建 FPR 自适应 Bloom 规格 | ⚠️ 实验 |
| `from_controller` | `fn from_controller(controller: &FPRController, segment_id: u64, num_elements: usize) -> Self` | 从控制器创建 | ⚠️ 实验 |
| `build` | `fn build(&self) -> bloom::BloomFilter` | 构建 BloomFilter | ⚠️ 实验 |
| `estimated_memory` | `fn estimated_memory(&self) -> usize` | 估算内存 (字节) | ⚠️ 实验 |

#### 使用说明

FPRController 根据段的访问 QPS 动态调整 Bloom Filter 的假阳性率。热段使用低 FPR (高精度, 更多内存)，冷段使用高 FPR (低精度, 更少内存)。

```rust
// 使用默认配置
let controller = FPRController::with_defaults();

// 记录访问
let access = AccessRecord { total_count: 100, window_count: 50, window_duration_ms: 5000, current_layer: 2 };
if let Some(new_level) = controller.record_access(segment_id, &access) {
    println!("FPR level changed to {} for segment {}", new_level, segment_id);
}

// 获取统计
let stats = controller.stats();
println!("Average FPR: {:.3}%", stats.avg_fpr * 100.0);
```

### 6.5 MigrationController

Bloom Filter 缓存层迁移控制器。

```rust
// 通过 bloom 子模块访问
use tokitai_filekv::bloom::migration::{MigrationController, MigrationStats, MigrationThresholds, FrequencyTier, classify_by_frequency};
```

> **注意**: 这些类型当前不在 crate 根级别导出，需通过 `bloom::migration` 路径访问。

#### MigrationThresholds 字段

| 字段 | 类型 | 说明 | 默认值 | 稳定性 |
|------|------|------|--------|--------|
| `warm_threshold_qps` | `u64` | L3→L2 迁移 QPS 阈值 | `10` | ⚠️ 实验 |
| `hot_threshold_qps` | `u64` | L2→L1 迁移 QPS 阈值 | `100` | ⚠️ 实验 |
| `cooldown_threshold_qps` | `u64` | L1→L2 迁移 QPS 阈值 | `5` | ⚠️ 实验 |
| `cold_threshold_qps` | `u64` | L2→L3 迁移 QPS 阈值 | `1` | ⚠️ 实验 |
| `upgrade_window_ms` | `u64` | 升级窗口时间 | `60_000` (1分钟) | ⚠️ 实验 |
| `downgrade_window_ms` | `u64` | 降级窗口时间 | `300_000` (5分钟) | ⚠️ 实验 |
| `hot_tier_access_count` | `u64` | 热层访问次数阈值 | `100` | ⚠️ 实验 |
| `warm_tier_access_count` | `u64` | 温层访问次数阈值 | `10` | ⚠️ 实验 |
| `frequency_weight` | `f64` | 频率分数权重 | `0.3` (30%) | ⚠️ 实验 |

#### FrequencyTier 枚举

| 变体 | 说明 | 首选层 | 稳定性 |
|------|------|--------|--------|
| `Hot` | 高访问频率 | L1 | ⚠️ 实验 |
| `Warm` | 中等访问频率 | L2 | ⚠️ 实验 |
| `Cold` | 低访问频率 | L3 | ⚠️ 实验 |

#### MigrationStats 字段

| 字段 | 类型 | 说明 | 稳定性 |
|------|------|------|--------|
| `tracked_segments` | `usize` | 跟踪的段数 | ⚠️ 实验 |
| `pending_migrations` | `usize` | 待处理迁移数 | ⚠️ 实验 |
| `upgrades_triggered` | `u64` | 触发升级次数 | ⚠️ 实验 |
| `downgrades_triggered` | `u64` | 触发降级次数 | ⚠️ 实验 |
| `migrations_completed` | `u64` | 完成迁移次数 | ⚠️ 实验 |

#### MigrationController 方法

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new(thresholds: MigrationThresholds) -> Self` | 创建控制器 | ⚠️ 实验 |
| `get_tracker` | `fn get_tracker(&self, segment_id: u64, initial_layer: usize) -> Arc<SegmentAccessTracker>` | 获取段跟踪器 | ⚠️ 实验 |
| `record_access` | `fn record_access(&self, segment_id: u64) -> Option<MigrationDecision>` | 记录访问并检查是否需要迁移 | ⚠️ 实验 |
| `complete_migration` | `fn complete_migration(&self, segment_id: u64, target_layer: usize)` | 标记迁移完成 | ⚠️ 实验 |
| `stats` | `fn stats(&self) -> MigrationStats` | 获取统计 | ⚠️ 实验 |
| `thresholds` | `fn thresholds(&self) -> &MigrationThresholds` | 获取阈值配置 | ⚠️ 实验 |
| `set_thresholds` | `fn set_thresholds(&mut self, thresholds: MigrationThresholds)` | 更新阈值 | ⚠️ 实验 |
| `remove_tracker` | `fn remove_tracker(&self, segment_id: u64)` | 移除跟踪器 | ⚠️ 实验 |
| `get_frequency_tier` | `fn get_frequency_tier(&self, segment_id: u64) -> FrequencyTier` | 获取段的频率层级 | ⚠️ 实验 |
| `get_recommended_layer` | `fn get_recommended_layer(&self, segment_id: u64) -> usize` | 获取推荐缓存层 | ⚠️ 实验 |

#### MigrationDecision 枚举

| 变体 | 说明 | 稳定性 |
|------|------|--------|
| `Stay` | 无需迁移 | ⚠️ 实验 |
| `UpgradeToL1` | 升级到 L1 | ⚠️ 实验 |
| `UpgradeToL2` | 升级到 L2 | ⚠️ 实验 |
| `DowngradeToL2` | 降级到 L2 | ⚠️ 实验 |
| `DowngradeToL3` | 降级到 L3 (驱逐) | ⚠️ 实验 |

#### classify_by_frequency 函数

```rust
pub fn classify_by_frequency(access_count: u64, thresholds: &MigrationThresholds) -> FrequencyTier
```

根据访问次数将段分类为 Hot/Warm/Cold 层级。

---

## 9. 检查点 API

### 9.1 CheckpointChain

检查点链。

```rust
use tokitai_filekv::CheckpointChain;
```

#### 方法

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `create` | `fn create(&self, metadata: CheckpointMetadata) -> anyhow::Result<CheckpointId>` | 创建检查点 | ✅ 稳定 |
| `restore` | `fn restore(&self, id: &CheckpointId) -> anyhow::Result<()>` | 恢复检查点 | ✅ 稳定 |
| `list` | `fn list(&self) -> Vec<CheckpointMetadata>` | 列出检查点 | ✅ 稳定 |
| `delete` | `fn delete(&self, id: &CheckpointId) -> anyhow::Result<()>` | 删除检查点 | ✅ 稳定 |

### 9.2 CheckpointMetadata

检查点元数据。

```rust
use tokitai_filekv::CheckpointMetadata;
```

#### 字段

| 字段 | 类型 | 说明 | 稳定性 |
|------|------|------|--------|
| `id` | `CheckpointId` | 检查点 ID | ✅ 稳定 |
| `seq` | `CheckpointSeq` | 序列号 | ✅ 稳定 |
| `timestamp` | `SystemTime` | 创建时间 | ✅ 稳定 |
| `checkpoint_type` | `CheckpointType` | 检查点类型 | ✅ 稳定 |
| `description` | `String` | 描述 | ✅ 稳定 |

### 9.3 IncrementalCheckpointManager

增量检查点管理器。

```rust
use tokitai_filekv::IncrementalCheckpointManager;
```

#### 方法

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new(config: ...) -> Self` | 创建管理器 | ✅ 稳定 |
| `create_checkpoint` | `fn create_checkpoint(&self) -> anyhow::Result<CheckpointId>` | 创建检查点 | ✅ 稳定 |
| `restore_checkpoint` | `fn restore_checkpoint(&self, id: &CheckpointId) -> anyhow::Result<()>` | 恢复检查点 | ✅ 稳定 |

---

## 10. 运维与可观测性 API

> **注意**: 完整I/O抽象API (FileKVFileSystem, StdFs, MemFs, FaultInjector等) 请见 [补充文档 §12](./supplemental-api.md#12-io-抽象-api)。  
> **注意**: 完整错误体系 (四层错误体系, FileKVError, ErrorCategory等) 请见 [补充文档 §13](./supplemental-api.md#13-错误类型)。

### 10.1 MemoryTracker

内存追踪器。

```rust
use tokitai_filekv::MemoryTracker;
```

#### 方法

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new(max_bytes: u64) -> Self` | 创建追踪器 | ✅ 稳定 |
| `get_usage` | `fn get_usage(&self) -> MemoryUsage` | 获取内存使用 | ✅ 稳定 |
| `record_allocation` | `fn record_allocation(&self, bytes: u64)` | 记录分配 | ✅ 稳定 |
| `record_deallocation` | `fn record_deallocation(&self, bytes: u64)` | 记录释放 | ✅ 稳定 |

#### MemoryUsage 字段

| 字段 | 类型 | 说明 | 稳定性 |
|------|------|------|--------|
| `total_bytes` | `u64` | 总字节数 | ✅ 稳定 |
| `active_bytes` | `u64` | 活跃字节数 | ✅ 稳定 |
| `allocated_bytes` | `u64` | 已分配字节数 | ✅ 稳定 |
| `max_bytes` | `u64` | 最大字节数 | ✅ 稳定 |

### 10.2 AmplificationTracker

放大因子追踪器，追踪写放大 (WA)、读放大 (RA) 和空间放大 (SA)。

```rust
// 通过 ops 子模块访问
use tokitai_filekv::ops::amplification::{AmplificationTracker, AmplificationStats, AmplificationReport};
```

> **注意**: `AmplificationTracker` 和 `AmplificationReport` 当前不在 crate 根级别导出，需通过 `ops::amplification` 路径访问。`AmplificationStats` 可通过 `FileKV::get_amplification_stats()` 方法间接获取。

#### AmplificationTracker 方法

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new() -> Self` | 创建追踪器 (所有计数器归零) | ⚠️ 实验 |
| `record_logical_write` | `fn record_logical_write(&self, bytes: u64)` | 记录逻辑写入字节 (key + value) | ⚠️ 实验 |
| `record_disk_write` | `fn record_disk_write(&self, bytes: u64)` | 记录磁盘写字节 (WAL, 段, 索引) | ⚠️ 实验 |
| `record_logical_read` | `fn record_logical_read(&self, bytes: u64)` | 记录逻辑读请求字节 | ⚠️ 实验 |
| `record_disk_read` | `fn record_disk_read(&self, bytes: u64)` | 记录磁盘读字节 (块, 索引查询) | ⚠️ 实验 |
| `set_logical_data_size` | `fn set_logical_data_size(&self, bytes: u64)` | 设置当前逻辑数据大小 | ⚠️ 实验 |
| `set_disk_usage` | `fn set_disk_usage(&self, bytes: u64)` | 设置当前磁盘使用量 | ⚠️ 实验 |
| `snapshot` | `fn snapshot(&self) -> AmplificationStats` | 获取统计快照 | ⚠️ 实验 |
| `reset` | `fn reset(&self)` | 重置所有计数器 | ⚠️ 实验 |

#### AmplificationStats 字段

| 字段 | 类型 | 说明 | 计算公式 | 稳定性 |
|------|------|------|---------|--------|
| `logical_write_bytes` | `u64` | 用户逻辑写字节 (key + value) | - | ⚠️ 实验 |
| `actual_disk_write_bytes` | `u64` | 实际磁盘写字节 (WAL, 段, 索引) | - | ⚠️ 实验 |
| `logical_read_bytes` | `u64` | 用户逻辑读字节 | - | ⚠️ 实验 |
| `actual_disk_read_bytes` | `u64` | 实际磁盘读字节 | - | ⚠️ 实验 |
| `logical_data_bytes` | `u64` | 当前逻辑数据大小 (唯一有效数据) | - | ⚠️ 实验 |
| `actual_disk_usage_bytes` | `u64` | 当前磁盘使用量 (所有段文件) | - | ⚠️ 实验 |
| `write_amplification` | `f64` | 写放大因子 | `actual_disk_write / logical_write` | ⚠️ 实验 |
| `read_amplification` | `f64` | 读放大因子 | `actual_disk_read / logical_read` | ⚠️ 实验 |
| `space_amplification` | `f64` | 空间放大因子 | `actual_disk_usage / logical_data` | ⚠️ 实验 |

#### AmplificationReport

综合放大分析报告，包含写放大和读放大的完整测试结果。

```rust
// 运行综合分析
let report = AmplificationReport::run_comprehensive();
println!("WAF: {:.2}x", report.combined_waf);
println!("RAF: {:.2}x", report.combined_raf);
println!("SAF: {:.2}x", report.combined_saf);
```

| 字段 | 类型 | 说明 | 稳定性 |
|------|------|------|--------|
| `write_result` | `WriteAmplificationResult` | 写放大测试结果 | ⚠️ 实验 |
| `read_result` | `ReadAmplificationResult` | 读放大测试结果 | ⚠️ 实验 |
| `combined_waf` | `f64` | 综合写放大因子 | ⚠️ 实验 |
| `combined_raf` | `f64` | 综合读放大因子 | ⚠️ 实验 |
| `combined_saf` | `f64` | 综合空间放大因子 | ⚠️ 实验 |

#### 集成点

- `WriteEngine::put()`: 调用 `record_logical_write(key.len + value.len)`
- WAL 写入: 调用 `record_disk_write(actual_wal_bytes)`
- MemTable 刷盘: 调用 `record_disk_write(segment_bytes)`
- Compaction: 调用 `record_disk_write(new_segment_bytes)` 和 `record_disk_read(old_segment_bytes)`
- `ReadEngine::get()`: 调用 `record_logical_read(key.len)` 和 `record_disk_read(actual_read_bytes)`

### 10.3 PerfTracker

逐模块性能追踪，用于性能回归调试。

```rust
// 通过 ops 子模块访问
use tokitai_filekv::ops::perf_tracker::{PerfTracker, PerfTimer, PerfSnapshot, ModuleTiming, format_ns};
```

> **注意**: 这些类型当前不在 crate 根级别导出，需通过 `ops::perf_tracker` 路径访问。

#### 追踪模块

| 模块名 | 索引 | 说明 |
|--------|------|------|
| `dense_index` | 0 | 密集索引查找时间 |
| `bloom_lookup` | 1 | Bloom 过滤器检查时间 |
| `cache_lookup` | 2 | BlockCache 获取/插入时间 |
| `segment_io` | 3 | 段读取/mmap 访问时间 |
| `decompress` | 4 | 解压缩时间 |
| `wal_write` | 5 | WAL 提交时间 |
| `memtable_insert` | 6 | MemTable 插入时间 |
| `compaction` | 7 | 压缩执行时间 |
| `total_get` | 8 | 端到端 get() 延迟 |
| `total_put` | 9 | 端到端 put() 延迟 |
| `prefetch` | 10 | 预读时间 |
| `zone_map` | 11 | ZoneMap 剪枝时间 |

#### PerfTracker 方法

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new() -> Self` | 创建追踪器 (所有计数器归零) | ⚠️ 实验 |
| `start_timer` | `fn start_timer(&self, module: &'static str) -> PerfTimer<'_>` | 开始计时, 返回 RAII Timer | ⚠️ 实验 |
| `record` | `fn record(&self, module: &'static str, elapsed_ns: u64)` | 直接记录测量值 | ⚠️ 实验 |
| `reset` | `fn reset(&self)` | 重置所有计数器 | ⚠️ 实验 |
| `snapshot` | `fn snapshot(&self) -> PerfSnapshot` | 获取性能快照 | ⚠️ 实验 |
| `get_module` | `fn get_module(&self, module: &str) -> Option<ModuleTiming>` | 获取指定模块的计时数据 | ⚠️ 实验 |

#### PerfTimer

RAII 计时器，调用 `stop()` 或 drop 时自动记录。

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `stop` | `fn stop(&mut self) -> u64` | 停止计时并记录 (返回纳秒) | ⚠️ 实验 |
| `discard` | `fn discard(self)` | 丢弃计时器, 不记录 | ⚠️ 实验 |

#### PerfSnapshot 字段

| 字段 | 类型 | 说明 | 稳定性 |
|------|------|------|--------|
| `modules` | `Vec<ModuleTiming>` | 各模块性能数据列表 | ⚠️ 实验 |

#### ModuleTiming 字段

| 字段 | 类型 | 说明 | 稳定性 |
|------|------|------|--------|
| `name` | `&'static str` | 模块名 | ⚠️ 实验 |
| `total_ns` | `u64` | 累计时间 (纳秒) | ⚠️ 实验 |
| `count` | `u64` | 调用次数 | ⚠️ 实验 |
| `avg_ns` | `u64` | 平均时间 (纳秒) | ⚠️ 实验 |
| `max_ns` | `u64` | 最慢单次调用 (纳秒) | ⚠️ 实验 |

#### 使用说明

```rust
let tracker = PerfTracker::new();

// 方式一: 使用 RAII Timer (推荐)
let mut timer = tracker.start_timer("bloom_lookup");
// ... 执行 bloom 查找 ...
timer.stop(); // 或者 drop 自动记录

// 方式二: 直接记录
tracker.record("cache_lookup", elapsed_ns);

// 获取报告
let snapshot = tracker.snapshot();
snapshot.print();
```

#### format_ns 辅助函数

```rust
pub fn format_ns(ns: u64) -> String
```

将纳秒转换为人类可读格式: `500ns`, `1.5µs`, `1.50ms`, `1.50s`。

### 10.4 AdaptivePreallocator

自适应段预分配器，基于写模式动态调整预分配大小。

```rust
use tokitai_filekv::{AdaptivePreallocator, AdaptivePreallocatorConfig, PreallocatorStats, SharedAdaptivePreallocator};
```

#### AdaptivePreallocatorConfig 字段

| 字段 | 类型 | 说明 | 默认值 | 稳定性 |
|------|------|------|--------|--------|
| `min_preallocate_bytes` | `u64` | 最小预分配大小 | `1MB` | ⚠️ 实验 |
| `max_preallocate_bytes` | `u64` | 最大预分配大小 | `64MB` | ⚠️ 实验 |
| `initial_preallocate_bytes` | `u64` | 初始预分配大小 | `16MB` | ⚠️ 实验 |
| `ewma_alpha` | `f64` | EWMA 平滑因子 (0.0-1.0) | `0.3` | ⚠️ 实验 |
| `history_size` | `usize` | 跟踪的历史段数 | `10` | ⚠️ 实验 |
| `enabled` | `bool` | 启用自适应模式 | `true` | ⚠️ 实验 |

#### PreallocatorStats 字段

| 字段 | 类型 | 说明 | 稳定性 |
|------|------|------|--------|
| `current_preallocate_size` | `u64` | 当前预分配大小 | ⚠️ 实验 |
| `avg_utilization` | `f64` | 平均段利用率 (actual/preallocated) | ⚠️ 实验 |
| `segments_tracked` | `usize` | 已跟踪段数 | ⚠️ 实验 |
| `total_preallocated_bytes` | `u64` | 总预分配字节数 | ⚠️ 实验 |
| `total_used_bytes` | `u64` | 总使用字节数 | ⚠️ 实验 |

#### AdaptivePreallocator 方法

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new(config: AdaptivePreallocatorConfig) -> Self` | 创建预分配器 | ⚠️ 实验 |
| `next_preallocate_size` | `fn next_preallocate_size(&self) -> u64` | 获取下次预分配大小 | ⚠️ 实验 |
| `record_segment_created` | `fn record_segment_created(&self, preallocated_size: u64)` | 记录段创建事件 | ⚠️ 实验 |
| `record_segment_closed` | `fn record_segment_closed(&self, actual_size: u64)` | 记录段关闭事件 (触发自适应调整) | ⚠️ 实验 |
| `stats` | `fn stats(&self) -> PreallocatorStats` | 获取统计 | ⚠️ 实验 |
| `reset` | `fn reset(&self)` | 重置自适应状态 | ⚠️ 实验 |
| `is_enabled` | `fn is_enabled(&self) -> bool` | 检查自适应模式是否启用 | ⚠️ 实验 |
| `min_preallocate_size` | `fn min_preallocate_size(&self) -> u64` | 获取配置的最小预分配大小 | ⚠️ 实验 |
| `max_preallocate_size` | `fn max_preallocate_size(&self) -> u64` | 获取配置的最大预分配大小 | ⚠️ 实验 |

#### 算法

预分配大小使用指数加权移动平均 (EWMA) 计算:
1. 跟踪最近 N 个段的实际大小
2. 计算近期段大小的平均值
3. 更新 EWMA: `ewma = alpha * avg + (1 - alpha) * ewma`
4. 最优预分配 = `ewma * 1.1` (10% 缓冲)
5. 结果限制在 `[min, max]` 范围内

### 10.5 FeatureFlagController

功能开关运行时控制器。

```rust
use tokitai_filekv::{FeatureFlag, FeatureFlagController, FeatureFlagStats, FeatureReport, FeatureState, FeatureStateChange};
```

#### FeatureFlag 枚举

| 变体 | 名称 | 说明 | 稳定性 |
|------|------|------|--------|
| `Inno001AdaptiveBloomCache` | `inno_001_adaptive_bloom_cache` | 自适应 Bloom 缓存 | ⚠️ 实验 |
| `Inno002ZoneMapPruning` | `inno_002_zone_map_pruning` | ZoneMap 剪枝 | ⚠️ 实验 |
| `Inno002SequentialPrefetch` | `inno_002_sequential_prefetch` | 顺序预读 | ⚠️ 实验 |

#### FeatureState 字段

| 字段 | 类型 | 说明 | 稳定性 |
|------|------|------|--------|
| `enabled` | `bool` | 是否启用 | ⚠️ 实验 |
| `hits` | `u64` | 启用时命中次数 | ⚠️ 实验 |
| `misses` | `u64` | 未启用时命中次数 | ⚠️ 实验 |

#### FeatureFlagStats 字段

| 字段 | 类型 | 说明 | 稳定性 |
|------|------|------|--------|
| `total_checks` | `u64` | 总检查次数 | ⚠️ 实验 |
| `enabled_hits` | `u64` | 启用时命中次数 | ⚠️ 实验 |
| `total_toggles` | `u64` | 总切换次数 | ⚠️ 实验 |

#### FeatureReport 字段

| 字段 | 类型 | 说明 | 稳定性 |
|------|------|------|--------|
| `features` | `HashMap<String, FeatureState>` | 所有功能状态 | ⚠️ 实验 |
| `total_toggles` | `u64` | 总切换次数 | ⚠️ 实验 |

> `FeatureReport` 实现了 `Display` trait, 可直接 `println!("{}", report)` 打印报告。

#### FeatureStateChange 字段

| 字段 | 类型 | 说明 | 稳定性 |
|------|------|------|--------|
| `feature` | `FeatureFlag` | 变更的功能 | ⚠️ 实验 |
| `old_enabled` | `bool` | 旧状态 | ⚠️ 实验 |
| `new_enabled` | `bool` | 新状态 | ⚠️ 实验 |

#### FeatureFlagController 方法

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new() -> Self` | 创建控制器 (默认全部启用) | ⚠️ 实验 |
| `feature_count` | `fn feature_count(&self) -> usize` | 获取功能数量 | ⚠️ 实验 |
| `is_enabled` | `fn is_enabled(&self, flag: FeatureFlag) -> bool` | 检查功能是否启用 | ⚠️ 实验 |
| `set_enabled` | `fn set_enabled(&self, flag: FeatureFlag, enabled: bool)` | 设置功能状态, 触发回调 | ⚠️ 实验 |
| `enable_inno001` | `fn enable_inno001(&self)` | 启用 INNO-001 | ⚠️ 实验 |
| `disable_inno001` | `fn disable_inno001(&self)` | 禁用 INNO-001 | ⚠️ 实验 |
| `enable_inno002` | `fn enable_inno002(&self)` | 启用 INNO-002 (ZoneMap + Prefetch) | ⚠️ 实验 |
| `disable_inno002` | `fn disable_inno002(&self)` | 禁用 INNO-002 | ⚠️ 实验 |
| `is_inno001_fully_enabled` | `fn is_inno001_fully_enabled(&self) -> bool` | 检查 INNO-001 是否完全启用 | ⚠️ 实验 |
| `is_inno002_fully_enabled` | `fn is_inno002_fully_enabled(&self) -> bool` | 检查 INNO-002 是否完全启用 | ⚠️ 实验 |
| `is_zone_map_pruning_enabled` | `fn is_zone_map_pruning_enabled(&self) -> bool` | 检查 ZoneMap 剪枝 | ⚠️ 实验 |
| `is_sequential_prefetch_enabled` | `fn is_sequential_prefetch_enabled(&self) -> bool` | 检查顺序预读 | ⚠️ 实验 |
| `get_stats` | `fn get_stats(&self) -> FeatureFlagStats` | 获取统计 | ⚠️ 实验 |
| `generate_report` | `fn generate_report(&self) -> FeatureReport` | 生成报告 | ⚠️ 实验 |
| `register_callback` | `fn register_callback(&self, callback: FeatureCallback) -> usize` | 注册状态变更回调 | ⚠️ 实验 |
| `reset` | `fn reset(&self)` | 重置所有状态和统计 | ⚠️ 实验 |

#### 全局控制器

```rust
pub fn global_controller() -> &'static FeatureFlagController
pub fn is_enabled(flag: FeatureFlag) -> bool
pub fn set_enabled(flag: FeatureFlag, enabled: bool)
```

提供全局单例访问, 使用 `OnceLock` 初始化。

### 10.6 AuditLogger

审计日志, 记录所有写操作用于合规和调试。

```rust
use tokitai_filekv::{AuditLogger, AuditLogConfig, AuditLogStats, AuditEntry, AuditOperation};
```

#### AuditLogConfig 字段

| 字段 | 类型 | 说明 | 默认值 | 稳定性 |
|------|------|------|--------|--------|
| `log_dir` | `PathBuf` | 日志目录 | `./audit_logs` | ✅ 稳定 |
| `enabled` | `bool` | 启用审计日志 | `false` | ✅ 稳定 |
| `rotation_interval_hours` | `u64` | 日志轮转间隔 (小时) | `24` | ✅ 稳定 |
| `retention_days` | `u32` | 日志保留天数 | `30` | ✅ 稳定 |

#### AuditLogStats 字段

| 字段 | 类型 | 说明 | 稳定性 |
|------|------|------|--------|
| `entries_written` | `u64` | 写入的日志条目数 | ✅ 稳定 |
| `errors` | `u64` | 错误次数 | ✅ 稳定 |

#### AuditOperation 枚举

| 变体 | 说明 | 稳定性 |
|------|------|--------|
| `Put` | 单条写入 | ✅ 稳定 |
| `Delete` | 单条删除 | ✅ 稳定 |
| `BatchPut { count: usize }` | 批量写入 | ✅ 稳定 |
| `BatchDelete { count: usize }` | 批量删除 | ✅ 稳定 |
| `Flush` | 刷盘操作 | ✅ 稳定 |
| `Compaction` | 压缩操作 | ✅ 稳定 |

#### AuditEntry 字段

| 字段 | 类型 | 说明 | 稳定性 |
|------|------|------|--------|
| `timestamp` | `DateTime<Utc>` | 时间戳 | ✅ 稳定 |
| `operation` | `AuditOperation` | 操作类型 | ✅ 稳定 |
| `keys` | `Vec<String>` | 受影响的键 | ✅ 稳定 |
| `value_hash` | `Option<String>` | 值哈希 (SHA-256) | ✅ 稳定 |
| `value_size` | `Option<u64>` | 值大小 (字节) | ✅ 稳定 |
| `latency_us` | `Option<u64>` | 操作延迟 (微秒) | ✅ 稳定 |
| `success` | `bool` | 是否成功 | ✅ 稳定 |
| `error` | `Option<String>` | 错误信息 | ✅ 稳定 |
| `metadata` | `AuditMetadata` | 附加元数据 | ✅ 稳定 |

#### AuditLogger 方法

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `open` | `fn open(config: AuditLogConfig) -> FileKVResult<Self>` | 创建并打开日志器 | ✅ 稳定 |
| `log_operation` | `fn log_operation(&self, operation: AuditOperation, keys: Vec<String>, value_hash: Option<String>, value_size: Option<u64>, latency_us: Option<u64>, success: bool, error: Option<String>, metadata: AuditMetadata) -> FileKVResult<()>` | 记录审计操作 | ✅ 稳定 |
| `stats` | `fn stats(&self) -> AuditLogStats` | 获取统计 | ✅ 稳定 |

#### 特性

- **时间戳轮转**: 根据 `rotation_interval_hours` 自动创建新日志文件
- **JSON 格式**: 每条日志为单行 JSON, 便于解析和分析
- **值哈希**: 使用 SHA-256 计算值哈希, 用于审计完整性
- **元数据支持**: 支持 `layer`, `session_id`, `user_id`, `request_id` 及自定义键值对

### 10.7 TimeoutControl

操作超时控制, 防止 I/O 操作无限阻塞。

```rust
use tokitai_filekv::{TimeoutConfig, TimeoutStats};
```

#### TimeoutConfig 字段

| 字段 | 类型 | 说明 | 默认值 | 稳定性 |
|------|------|------|--------|--------|
| `read_timeout_ms` | `u64` | 读操作超时 (ms) | `5000` (5秒) | ✅ 稳定 |
| `write_timeout_ms` | `u64` | 写操作超时 (ms) | `10000` (10秒) | ✅ 稳定 |
| `delete_timeout_ms` | `u64` | 删除操作超时 (ms) | `10000` (10秒) | ✅ 稳定 |
| `compaction_timeout_ms` | `u64` | 压缩操作超时 (ms) | `300000` (5分钟) | ✅ 稳定 |
| `flush_timeout_ms` | `u64` | 刷盘操作超时 (ms) | `60000` (1分钟) | ✅ 稳定 |
| `checkpoint_timeout_ms` | `u64` | 检查点操作超时 (ms) | `120000` (2分钟) | ✅ 稳定 |
| `enable_retry` | `bool` | 启用超时自动重试 | `true` | ✅ 稳定 |
| `max_retry_attempts` | `u32` | 最大重试次数 | `3` | ✅ 稳定 |
| `enable_backoff` | `bool` | 启用指数退避 | `true` | ✅ 稳定 |

#### TimeoutConfig 方法

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new() -> Self` | 创建默认配置 | ✅ 稳定 |
| `with_read_timeout` | `fn with_read_timeout(mut self, timeout_ms: u64) -> Self` | 设置读超时 (Builder) | ✅ 稳定 |
| `with_write_timeout` | `fn with_write_timeout(mut self, timeout_ms: u64) -> Self` | 设置写超时 (Builder) | ✅ 稳定 |
| `with_delete_timeout` | `fn with_delete_timeout(mut self, timeout_ms: u64) -> Self` | 设置删除超时 (Builder) | ✅ 稳定 |
| `get_timeout` | `fn get_timeout(&self, op: OperationType) -> Duration` | 获取指定操作类型的超时时间 | ✅ 稳定 |
| `calculate_backoff` | `fn calculate_backoff(&self, attempt: u32) -> Duration` | 计算退避时间 | ✅ 稳定 |

#### OperationType 枚举

| 变体 | 说明 | 稳定性 |
|------|------|--------|
| `Read` | 读操作 | ✅ 稳定 |
| `Write` | 写操作 | ✅ 稳定 |
| `Delete` | 删除操作 | ✅ 稳定 |
| `Compaction` | 压缩操作 | ✅ 稳定 |
| `Flush` | 刷盘操作 | ✅ 稳定 |
| `Checkpoint` | 检查点操作 | ✅ 稳定 |

#### TimeoutStats 字段

| 字段 | 类型 | 说明 | 稳定性 |
|------|------|------|--------|
| `timeout_count` | `u64` | 超时总次数 | ✅ 稳定 |
| `retry_count` | `u64` | 重试总次数 | ✅ 稳定 |
| `successful_retries` | `u64` | 成功重试次数 | ✅ 稳定 |
| `failed_retries` | `u64` | 失败重试次数 (全部耗尽) | ✅ 稳定 |
| `total_retry_time_us` | `u64` | 重试总时间 (微秒) | ✅ 稳定 |

#### execute_with_timeout 函数

```rust
pub fn execute_with_timeout<T, F>(
    op: OperationType,
    config: &TimeoutConfig,
    stats: Option<&mut TimeoutStats>,
    f: F,
) -> Result<T>
where
    F: FnMut(Duration) -> Result<T>
```

以超时限制执行操作, 支持自动重试和指数退避。

---

## 11. I/O 抽象 API

> **完整I/O API详见**: [补充文档 §12](./supplemental-api.md#12-io-抽象-api) (FileKVFileSystem, FileKVFile, MmapFileSystem, StdFs, MemFs, FaultInjector等)

### 11.1 FileKVFileSystem

文件系统 trait。

```rust
use tokitai_filekv::{FileKVFileSystem, StdFs, MemFs};
```

#### 方法

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `create_dir_all` | `fn create_dir_all(&self, path: &Path) -> std::io::Result<()>` | 创建目录 | ⚠️ 实验 |
| `remove_file` | `fn remove_file(&self, path: &Path) -> std::io::Result<()>` | 删除文件 | ⚠️ 实验 |
| `read_dir` | `fn read_dir(&self, path: &Path) -> std::io::Result<Vec<PathBuf>>` | 读取目录 | ⚠️ 实验 |
| `open_file` | `fn open_file(&self, path: &Path) -> std::io::Result<Box<dyn FileKVFile>>` | 打开文件 | ⚠️ 实验 |

### 11.2 FaultInjector

故障注入器（测试用）。

```rust
use tokitai_filekv::FaultInjector;
```

#### 方法

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `add_rule` | `fn add_rule(&self, rule: FaultRule)` | 添加规则 | ⚠️ 实验 |
| `remove_rule` | `fn remove_rule(&self, id: &str)` | 移除规则 | ⚠️ 实验 |
| `stats` | `fn stats(&self) -> FaultInjectorStats` | 获取统计 | ⚠️ 实验 |

---

## 12. 错误类型

> **完整错误体系详见**: [补充文档 §13](./supplemental-api.md#13-错误类型) (FatalError, TransientError, ExpectedError, DomainError四层体系)

### 12.1 FileKVConfigError

配置错误。

```rust
use tokitai_filekv::FileKVConfigError;
```

| 变体 | 说明 | 稳定性 |
|------|------|--------|
| `InvalidPath(String)` | 路径无效 | ✅ 稳定 |
| `InvalidValue { field: String, reason: String }` | 值无效 | ✅ 稳定 |
| `Conflict { field1: String, field2: String, reason: String }` | 配置冲突 | ✅ 稳定 |

### 12.2 FileKVConfigValidation

配置验证结果。

```rust
use tokitai_filekv::FileKVConfigValidation;
```

#### 字段

| 字段 | 类型 | 说明 | 稳定性 |
|------|------|------|--------|
| `errors` | `Vec<String>` | 错误列表 | ✅ 稳定 |
| `warnings` | `Vec<String>` | 警告列表 | ✅ 稳定 |
| `is_valid` | `bool` | 是否有效 | ✅ 稳定 |

---

## 13. Feature Flags

### 13.1 稳定 Feature

| Feature | 默认 | 说明 | 稳定性 |
|---------|------|------|--------|
| `wal` | ✅ | WAL 支持 | ✅ 稳定 |
| `mimalloc` | ❌ | mimalloc 分配器 | ✅ 稳定 |
| `benchmarks` | ❌ | 基准测试 | ✅ 稳定 |
| `rocksdb-compare` | ❌ | RocksDB 对比 | ✅ 稳定 |

### 13.2 实验 Feature

| Feature | 默认 | 说明 | 稳定性 |
|---------|------|------|--------|
| `metrics` | ❌ | Prometheus 指标 | ⚠️ 实验 |
| `async-io` | ❌ | 异步 I/O | ⚠️ 实验 |
| `full` | ❌ | 所有功能 | ⚠️ 实验 |

---

## 附录 A：稳定性标识说明

| 标识 | 含义 | 保证 |
|------|------|------|
| ✅ 稳定 | Stable API | 主版本号升级前保证向后兼容 |
| ⚠️ 实验 | Experimental API | 当前版本内尽量稳定，次版本可能变更 |
| 🔒 内部 | Internal API | 无稳定性保证，用户不应直接使用 |

---

## 附录 B：快速索引

### 按功能分类

| 功能 | 入口类型 | 文档章节 |
|------|---------|---------|
| 写入数据 | `FileKV::put` | §1.1 |
| 读取数据 | `FileKV::get` | §1.1 |
| 批量写入 | `FileKV::put_batch` | §1.1 |
| 范围扫描 | `FileKV::range` | §5 |
| 刷盘 | `FileKV::flush_memtable` | §1.1 |
| 压缩 | `FileKV::run_compaction` | §1.1 |
| 统计查询 | `FileKV::get_stats` | §1.1 |
| 配置存储 | `FileKVConfig` | §2 |
| 缓存管理 | `BlockCache` | §3 |
| 完整缓存API | `UnifiedCacheManager`, `L2CacheManager` | [补充文档 §5](./supplemental-api.md#5-缓存-api-补充) |
| Bloom Filter | `AdaptiveBloomCache`, `BloomFilterCache` | §6 |
| 数据压缩 | `CompressionStrategy`, `DictionaryCompressor` | §4 |
| Compaction | `CompactionConfig`, `MergeIterator` | [补充文档 §8](./supplemental-api.md#8-compaction-系统-api) |
| 检查点 | `CheckpointChain`, `IncrementalCheckpointManager` | §9 |
| 内存监控 | `MemoryTracker` | §10.1 |
| 放大分析 | `AmplificationTracker` | §10.2 |
| 性能追踪 | `PerfTracker` | §10.3 |
| 预分配器 | `AdaptivePreallocator` | §10.4 |
| 功能开关 | `FeatureFlagController` | §10.5 |
| 审计日志 | `AuditLogger` | §10.6 |
| 超时控制 | `TimeoutConfig` | §10.7 |
| I/O 抽象 | `FileKVFileSystem`, `StdFs`, `FaultInjector` | [补充文档 §12](./supplemental-api.md#12-io-抽象-api) |
| 错误体系 | `FileKVError`, `FatalError`, `TransientError` | [补充文档 §13](./supplemental-api.md#13-错误类型) |

---

**本文档是 tokitai-filekv 公共 API 的主要参考。所有稳定层 API 都受 [API_STABILITY.md](API_STABILITY.md) 中的承诺约束。**

---

## 附录 C：文档结构说明

由于本项目的API表面积非常大 (630+ tests, 四引擎架构, 多层缓存系统),为了保持文档的可维护性和阅读体验,API文档分为两部分:

### 📘 主文档 (本文档)
**API_REFERENCE.md** (约1600行) 包含:
- ✅ 核心公共API (`FileKV`, `FileKVConfig`)
- ✅ 配置API (FileKVConfig, AggressiveConfig, Durability等)
- ✅ 缓存API核心 (BlockCache, BlockCacheConfig)
- ✅ Bloom Filter生态 (AdaptiveBloomCache, BloomFilterCache, FPRController等)
- ✅ 压缩API (CompressionStrategy, DictionaryCompressor)
- ✅ 范围扫描API (RangeScanIterator, RangeScanConfig)
- ✅ 检查点API (CheckpointChain, IncrementalCheckpointManager)
- ✅ 运维与可观测性 (MemoryTracker, AmplificationTracker, PerfTracker, AuditLogger, FeatureFlagController, AdaptivePreallocator, TimeoutControl)
- ✅ I/O抽象核心 (FileKVFileSystem, StdFs, MemFs, FaultInjector概述)
- ✅ 错误类型核心 (FileKVConfigError, FileKVConfigValidation概述)
- ✅ Feature Flags

### 📗 补充文档
**supplemental-api.md** (约1760行) 包含:
- 📖 §3 核心存储模块完整API (MemTable, SegmentFile, SparseIndex, WriteCoalescer, FlushTrigger)
- 📖 §5 缓存API补充 (UnifiedCacheConfig, CacheBudget, L2CacheManager, Rebalance, CacheWarmer, SequentialPrefetcher)
- 📖 §8 Compaction系统完整API (CompactionConfig 18字段, CompactionTrigger 5种类型, MergeIterator, SegmentIterator)
- 📖 §12 I/O抽象完整API (FileKVFile trait, MmapFileSystem trait, StdFs/StdFile/StdMmap实现, FaultInjector 5种策略)
- 📖 §13 错误体系详细说明 (FatalError, TransientError, ExpectedError, DomainError四层体系, ErrorCategory 6种分类)

### 🔗 如何使用

**对于一般用户**:
- 阅读主文档了解核心API
- 需要深入了解某个模块时,查看补充文档链接

**对于高级用户/贡献者**:
- 主文档 + 补充文档配合阅读
- 补充文档包含完整的类型定义、方法签名、使用示例

**文档维护**:
- 两份文档应保持同步
- API变更时同时更新主文档和补充文档
- 所有API标注稳定性标识 (✅稳定 / ⚠️实验 / 🔒内部)
