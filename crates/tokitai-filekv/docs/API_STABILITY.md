# API 稳定性承诺 (API Stability Guarantee)

**版本**: v0.5.0+
**生效日期**: 2026-04-16
**维护者**: tokitai-filekv 核心团队

---

## 1. 版本策略

tokitai-filekv 遵循 [语义化版本 2.0.0](https://semver.org/lang/zh-CN/)：

```
主版本。次版本.修订版本
MAJOR.MINOR.PATCH
```

- **主版本 (MAJOR)**：破坏性 API 变更
- **次版本 (MINOR)**：向后兼容的功能新增
- **修订版本 (PATCH)**：向后兼容的 bug 修复

**当前稳定版本**: `v0.5.0`

---

## 2. API 稳定性层级

tokitai-filekv 的公共 API 分为三个稳定性层级：

### 2.1 稳定层 (Stable API) ✅

这些 API **保证向后兼容**，除非主版本号升级。

**包含内容**：
- 核心存储操作 (`FileKV::open`, `FileKV::get`, `FileKV::put`, `FileKV::delete`, `FileKV::put_batch`, `FileKV::range`)
- 主配置类型 (`FileKVConfig`, `AggressiveConfig`)
- 配置错误类型 (`FileKVConfigError`, `FileKVConfigValidation`)
- 统计快照 (`FileKVStatsSnapshot`, `FileKVStats`)
- 配置枚举 (`Durability`, `WalSyncMode`, `BlockCompressionMode`, `BlockCompressionConfig`)
- 压缩接口 (`CompressionStrategy`, `CompressionAlgorithmId`, `create_compressor`)
- 字典压缩 (`DictionaryCompressor`, `DictionaryCompressionConfig`, `DictionaryStats`)
- 块缓存 (`BlockCache`, `BlockCacheConfig`, `CacheStats`)
- 统一缓存管理器 (`UnifiedCacheManager`, `UnifiedCacheConfig`)
- 缓存预热 (`CacheWarmer`, `CacheWarmingConfig`, `WarmingStrategy`)
- 顺序预取 (`SequentialPrefetcher`, `SequentialPrefetcherConfig`, `SequentialPrefetcherStats`)
- 范围扫描 (`RangeScanIterator`, `RangeScanConfig`, `RangeEntry`, `RangeScanStats`)
- 内存追踪 (`MemoryTracker`, `MemoryUsage`)
- 超时控制 (`TimeoutConfig`, `TimeoutStats`)
- 审计日志 (`AuditLogger`, `AuditLogConfig`, `AuditOperation`)
- 功能开关 (`FeatureFlagController`, `FeatureFlag`, `FeatureState`)
- 预分配器 (`AdaptivePreallocator`, `AdaptivePreallocatorConfig`, `PreallocatorStats`)
- 检查点 (`CheckpointChain`, `CheckpointMetadata`, `IncrementalCheckpointManager`)
- 压缩配置 (`CompactionConfig`)

**保证**：
- ✅ 方法签名不变
- ✅ 类型字段不删除
- ✅ 行为语义向后兼容
- ✅ 错误类型枚举值不移除

### 2.2 实验层 (Experimental API) ⚠️

这些 API **可能在未来版本中变更**，不保证向后兼容。

**包含内容**：
- 异步 I/O 模块 (`AsyncWriter`, `AsyncIoConfig`, `AsyncIoStats`) - feature: `async-io`
- 故障注入 (`FaultInjector`, `FaultRule`, `FaultStrategy`) - 用于测试
- 内存文件系统 (`MemFs`, `StdFs`) - 高级用法
- 文件系统 trait (`FileKVFile`, `FileKVFileSystem`, `MmapFileSystem`, `MmapView`) - 高级用法
- 自适应 Bloom 缓存 (`AdaptiveBloomCache`, `AdaptiveBloomCacheConfig`) - 高级用法
- FPR 控制器 (`FPRController`, `FPRAdjustedBloom`) - 高级用法
- Zone Map 组件 (`ZoneMapBuilder`, `ZoneMapEntry`, `ZoneMapError`) - 高级用法
- Prometheus 指标 (`FileKVMetrics`, `PrometheusExporter`) - feature: `metrics`

**保证**：
- ⚠️ 当前版本内尽量保持稳定
- ⚠️ 次版本升级时可能变更
- ⚠️ 变更会在 CHANGELOG.md 中明确标注

### 2.3 内部层 (Internal API) 🔒

这些 API **仅供内部使用**，用户不应依赖。

**包含内容**：
- 所有引擎内部状态 (`EngineState`, `SegmentState`, `IndexState`, `MemTableState`, `CacheState`, `StatsState`, `GlobalIndexState`)
- 所有引擎 trait (`ReadEngineAPI`, `WriteEngineAPI`, `CompactionEngineAPI`, `LifecycleManagerAPI`)
- WAL 内部组件 (`WalManager`, `WalEntry`, `WalChannel`, `WalBatcher`)
- MemTable 内部 (`MemTable`, `MemTableConfig`, `MemTableEntry`)
- Segment 内部 (`SegmentFile`, `SegmentStats`, `BlockHeader`, `Opt009BlockHeader`)
- 索引内部 (`IndexManager`, `SparseIndex`, `DenseIndex`, `GlobalKeyIndex`)
- Bloom 内部格式 (`CustomBloom`, `CompressedBloom`, `FilterWrapper`)
- Bloom V3 操作函数 (`save_bloom_filter_v3`, `load_bloom_filter_v3`, `migrate_to_v3`)
- 压缩具体实现 (`ZstdCompressor`, `SnappyCompressor`, `Lz4Compressor`, `NoCompression`)
- 缓存内部 (`CacheBudget`, `L2CacheManager`, `RebalanceConfig`, `BlockCacheAsPrefetchCache`)
- 性能追踪 (`PerfTracker`, `AmplificationTracker`, `WriteAmplificationAnalyzer`)
- 压缩清单 (`CompactionExecutor`, `CompactionManifest`, `RecoveryAction`)
- 写合并器 (`WriteCoalescer`, `WriteCoalescerConfig`)
- 刷盘触发器 (`FlushTrigger`)
- 所有魔术常量和内部版本号 (`BLOOM_MAGIC`, `BLOOM_VERSION`, `SEGMENT_MAGIC`, `OPT009_BLOCK_HEADER_MAGIC`)

**保证**：
- 🔒 无稳定性保证
- 🔒 可能在任何版本中变更
- 🔒 用户不应直接使用

---

## 3. 公共 API 清单 (Stable Layer)

### 3.1 FileKV 主类型

```rust
/// 核心存储引擎
pub struct FileKV {
    // 内部字段，用户不应直接访问
}

impl FileKV {
    /// 创建或打开 FileKV 存储
    pub fn open(config: FileKVConfig) -> anyhow::Result<Self>;
    
    /// 写入键值对
    pub fn put(&self, key: &str, value: &[u8]) -> anyhow::Result<()>;
    
    /// 读取键值对
    pub fn get(&self, key: &str) -> anyhow::Result<Option<Bytes>>;
    
    /// 删除键值对
    pub fn delete(&self, key: &str) -> anyhow::Result<()>;
    
    /// 批量写入
    pub fn put_batch(&self, entries: &[(&str, &[u8])]) -> anyhow::Result<()>;
    
    /// 范围扫描
    pub fn range(&self, start: &str, end: &str) -> FileKVResult<RangeScanIterator<'_>>;
    
    /// 范围扫描收集
    pub fn range_collect(&self, start: &str, end: &str, limit: usize) 
        -> FileKVResult<Vec<(String, Vec<u8>)>>;
    
    /// 刷盘
    pub fn flush_memtable(&self) -> anyhow::Result<()>;
    
    /// 手动触发压缩
    pub fn run_compaction(&self) -> anyhow::Result<CompactionStats>;
    
    /// 获取统计信息
    pub fn get_stats(&self) -> FileKVStatsSnapshot;
    
    /// 获取内存使用
    pub fn get_memory_usage(&self) -> MemoryUsage;
    
    /// 获取放大统计
    pub fn get_amplification_stats(&self) -> AmplificationStats;
    
    /// WAL 恢复
    pub fn recover(&self) -> FileKVResult<usize>;
}
```

### 3.2 配置类型

```rust
/// 主配置类型
pub struct FileKVConfig {
    pub fs: Arc<dyn FileKVFileSystem>,
    pub segment_dir: PathBuf,
    pub wal_dir: PathBuf,
    pub index_dir: PathBuf,
    pub checkpoint_dir: PathBuf,
    pub enable_wal: bool,
    pub enable_multi_level_cache: bool,
    pub enable_adaptive_bloom_cache: bool,
    pub aggressive: AggressiveConfig,
    pub cache: BlockCacheConfig,
    pub memtable: MemTableConfig,
    pub compression: DictionaryCompressionConfig,
    pub compaction: CompactionConfig,
    // ... 其他字段
}

impl FileKVConfig {
    pub fn validate(&self) -> FileKVConfigValidation;
}

/// 激进优化配置
pub struct AggressiveConfig {
    pub persistent_mmap_enabled: bool,
    pub readahead_multiplier: u32,
    pub dense_index_enabled: bool,
    pub cache_max_memory_bytes: usize,
    // ... 其他字段
}
```

### 3.3 核心操作 API

```rust
/// 持久性级别
pub enum Durability {
    Relaxed,
    Standard,
    Strict,
}

/// WAL 同步模式
pub enum WalSyncMode {
    Async,
    Sync,
    FsSync,
}

/// 块压缩模式
pub enum BlockCompressionMode {
    None,
    Zstd { level: i32 },
    Snappy,
    Lz4 { level: i32 },
}

/// 块压缩配置
pub struct BlockCompressionConfig {
    pub mode: BlockCompressionMode,
    pub min_block_size_bytes: usize,
    pub dictionary_trained: bool,
}
```

### 3.4 缓存 API

```rust
/// 块缓存
pub struct BlockCache { /* ... */ }

impl BlockCache {
    pub fn new(config: BlockCacheConfig) -> Self;
    pub fn get(&self, key: &str) -> Option<Vec<u8>>;
    pub fn put(&self, key: &str, value: Vec<u8>);
    pub fn stats(&self) -> CacheStats;
}

/// 块缓存配置
pub struct BlockCacheConfig {
    pub max_memory_bytes: u64,
    pub max_items: u64,
    pub frequency_aware: bool,
    // ... 其他字段
}

/// 缓存统计
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    // ... 其他字段
}
```

### 3.5 压缩 API

```rust
/// 压缩策略 trait
pub trait CompressionStrategy: Send + Sync {
    fn compress(&self, data: &[u8]) -> anyhow::Result<Vec<u8>>;
    fn decompress(&self, data: &[u8]) -> anyhow::Result<Vec<u8>>;
    fn algorithm_id(&self) -> CompressionAlgorithmId;
}

/// 压缩算法 ID
pub enum CompressionAlgorithmId {
    None,
    Zstd,
    Snappy,
    Lz4,
}

/// 创建压缩器工厂函数
pub fn create_compressor(mode: &BlockCompressionMode, level: i32) 
    -> Box<dyn CompressionStrategy>;
```

### 3.6 范围扫描 API

```rust
/// 范围扫描迭代器
pub struct RangeScanIterator<'a> { /* ... */ }

impl<'a> Iterator for RangeScanIterator<'a> {
    type Item = (String, Vec<u8>);
    fn next(&mut self) -> Option<Self::Item>;
}

/// 范围扫描配置
pub struct RangeScanConfig {
    pub include_start: bool,
    pub include_end: bool,
    pub limit: Option<usize>,
    pub reverse: bool,
}

/// 范围扫描条目
pub struct RangeEntry {
    pub key: String,
    pub value: Vec<u8>,
}

/// 范围扫描统计
pub struct RangeScanStats {
    pub scanned_entries: u64,
    pub returned_entries: u64,
    pub blocks_scanned: u64,
    pub blocks_pruned: u64,
}
```

---

## 4. 模块可见性策略

### 4.1 当前状态

目前 tokitai-filekv 的所有子模块都声明为 `pub mod`：

```rust
pub mod io;
pub mod cache;
pub mod engine;
pub mod core;
pub mod bloom;
pub mod query;
pub mod compaction;
pub mod checkpoint;
pub mod ops;
pub mod compression;
```

这意味着用户可以通过 `tokitai_filekv::bloom::compressed::CompressedBloom` 这样的路径访问任意内部类型。

### 4.2 未来计划

在 **v0.6.0** 中，我们将重构模块可见性：

```rust
pub mod io;          // 保持 pub (高级用户需要)
pub mod cache;       // 改为 pub(crate)
pub mod engine;      // 改为 pub(crate)
pub mod core;        // 改为 pub(crate)
pub mod bloom;       // 改为 pub(crate)
pub mod query;       // 改为 pub(crate)
pub mod compaction;  // 改为 pub(crate)
pub mod checkpoint;  // 改为 pub(crate)
pub mod ops;         // 改为 pub(crate)
pub mod compression; // 改为 pub(crate)
```

**仅通过 `pub use` 在 crate 根导出稳定层 API**。

### 4.3 迁移指南

如果你当前直接使用内部模块路径（如 `tokitai_filekv::core::WalManager`）：

```rust
// ❌ 不推荐 (v0.6.0 将失效)
use tokitai_filekv::core::wal::WalManager;

// ✅ 推荐 (使用根导出)
use tokitai_filekv::FileKV;  // 通过主类型操作
```

---

## 5. 变更政策

### 5.1 稳定层变更

**允许**：
- 新增方法到现有 trait（如果有默认实现）
- 新增配置字段（如果有默认值）
- 新增统计字段到 snapshot 类型
- bug 修复（行为修正）

**不允许**：
- 删除或重命名公共方法
- 删除或重命名公共类型
- 改变方法签名（破坏兼容性）
- 删除枚举值
- 改变错误类型语义

### 5.2 实验层变更

**允许**：
- API 签名变更
- 新增/删除功能
- 行为调整

**要求**：
- 在 CHANGELOG.md 中明确标注
- 提供迁移指南

### 5.3 内部层变更

**无限制**：
- 可随时变更
- 无文档要求
- 无迁移指南

---

## 6. 弃用政策

### 6.1 弃用流程

当某个 API 需要废弃时：

1. **标记弃用** (次版本)：
   ```rust
   #[deprecated(since = "0.6.0", note = "使用 new_method() 替代")]
   pub fn old_method(&self) { /* ... */ }
   ```

2. **保持兼容** (至少 1 个次版本)：
   - 弃用 API 继续工作
   - 编译器产生警告
   - 文档中标注弃用

3. **移除** (主版本或后续次版本)：
   - 在下一个主版本或充分的次版本后移除

### 6.2 当前弃用列表

**无**。所有当前稳定层 API 均为活跃状态。

---

## 7. Feature Flags 稳定性

### 7.1 稳定 Feature

| Feature | 状态 | 说明 |
|---------|------|------|
| `wal` | ✅ 稳定 | WAL 支持 (默认启用) |
| `mimalloc` | ✅ 稳定 | mimalloc 分配器 |
| `benchmarks` | ✅ 稳定 | 基准测试套件 |
| `rocksdb-compare` | ✅ 稳定 | RocksDB 对比测试 |

### 7.2 实验 Feature

| Feature | 状态 | 说明 |
|---------|------|------|
| `metrics` | ⚠️ 实验 | Prometheus 指标导出 |
| `async-io` | ⚠️ 实验 | 异步 I/O 支持 |

### 7.3 组合 Feature

| Feature | 状态 | 包含 |
|---------|------|------|
| `full` | ⚠️ 实验 | `wal` + `metrics` + `async-io` |

---

## 8. 文档承诺

### 8.1 文档覆盖

- ✅ 所有稳定层 API 都有文档注释
- ✅ 所有公共方法都有示例代码
- ✅ 所有配置字段都有说明

### 8.2 文档更新

- 每次 API 变更同步更新文档
- 提供 API 参考文档 (`docs/API_REFERENCE.md`)
- 提供使用指南 (`doc/filekv/FILEKV_GUIDE.md`)

---

## 9. 测试承诺

### 9.1 测试覆盖

- ✅ 稳定层 API 100% 测试覆盖
- ✅ 所有 doctest 通过
- ✅ 集成测试覆盖核心场景

### 9.2 回归测试

- 每次 PR 运行完整测试套件
- 性能回归检测 (`cargo bench`)
- 稳定性测试 (24h+ 标记为 `#[ignore]`)

---

## 10. 升级指南

### 10.1 小版本升级 (0.5.x → 0.5.y)

**通常无需代码变更**：
```toml
# Cargo.toml
tokitai-filekv = "0.5"  # 自动获取 0.5.x 最新版本
```

### 10.2 次版本升级 (0.5 → 0.6)

**可能需要少量适配**：
```toml
# Cargo.toml
tokitai-filekv = "0.6"
```

**检查清单**：
- [ ] 查看 CHANGELOG.md 中的 Breaking Changes
- [ ] 运行 `cargo build` 检查编译错误
- [ ] 运行测试确保功能正常

### 10.3 主版本升级 (0.x → 1.0)

**需要全面适配**：
```toml
# Cargo.toml
tokitai-filekv = "1.0"
```

**检查清单**：
- [ ] 详细阅读迁移指南
- [ ] 更新所有 API 调用
- [ ] 全面回归测试
- [ ] 性能基准对比

---

## 11. 支持政策

### 11.1 版本支持

| 版本 | 支持状态 | 支持期限 |
|------|---------|---------|
| 0.4.x | ❌ 停止支持 | 已过期 |
| 0.5.x | ✅ 活跃支持 | 当前版本 |
| 0.6.x | 🔜 规划中 | 未来版本 |

### 11.2 Bug 修复

- **严重 bug**：立即修复，发布 patch 版本
- **一般 bug**：随下次次版本发布
- **性能回归**：优先级高，尽快修复

### 11.3 安全漏洞

- 发现安全漏洞：**立即修复**
- 发布安全公告
- 建议用户尽快升级

---

## 12. 反馈渠道

如果你发现 API 文档疏漏或有改进建议：

- 📧 **GitHub Issues**: https://github.com/silverenternal/tokitai/issues
- 💬 **Discussions**: https://github.com/silverenternal/tokitai/discussions
- 📝 **邮件**: 项目维护者

---

**本文档是 tokitai-filekv 对用户的正式承诺，具有约束力。任何违反本承诺的变更都需要说明理由并提供迁移方案。**
