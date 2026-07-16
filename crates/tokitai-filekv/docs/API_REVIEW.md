# API 文档审查报告

**审查日期**: 2026-04-16
**审查版本**: v0.5.0
**审查范围**: 公共 API 表面、文档完整性、稳定性承诺

---

## 1. 执行摘要

本次审查发现 tokitai-filekv 项目在 API 文档方面存在以下问题：

| 类别 | 问题数量 | 严重程度 | 说明 |
|------|---------|---------|------|
| **文档疏漏** | 12 | 🔴 高 | 公共 API 缺少文档或示例 |
| **过度暴露** | 45+ | 🔴 高 | 内部实现细节暴露给用户 |
| **不一致** | 8 | 🟡 中 | 文档与代码不一致 |
| **缺失承诺** | 1 | 🟡 中 | 缺少 API 稳定性承诺文档 |

**已修复**：
- ✅ 创建 `docs/API_STABILITY.md` - API 稳定性承诺文档
- ✅ 创建 `docs/API_REFERENCE.md` - 完整 API 参考文档

---

## 2. 文档疏漏 (Documentation Gaps)

### 2.1 缺失文档的公共 API

以下类型通过 `pub use` 导出，但缺少完整文档：

| 类型 | 模块 | 问题 | 优先级 |
|------|------|------|--------|
| `AdaptivePreallocator` | `ops::preallocator` | 缺少使用示例和配置指南 | 🔴 高 |
| `PrefetchCache` | `cache::prefetch` | 缺少 API 文档 | 🔴 高 |
| `SequentialDetector` | `query::detector` | 缺少行为说明 | 🟡 中 |
| `ZoneMapBuilder` / `ZoneMapEntry` | `query::zone_map` | 缺少 INNO-002 相关文档 | 🟡 中 |
| `PrunedBlockIterator` | `query::pruner` | 缺少迭代器使用说明 | 🟡 中 |
| `FeatureFlagController` | `ops::feature_flag` | 缺少功能开关使用指南 | 🟡 中 |
| `AuditLogger` / `AuditOperation` | `ops::audit_log` | 缺少审计日志配置说明 | 🟡 中 |
| `CheckpointChain` (完整方法) | `checkpoint` | 部分方法缺少文档 | 🔴 高 |
| `IncrementalCheckpointManager` | `checkpoint` | 缺少增量检查点说明 | 🔴 高 |
| `CompressionStrategy` trait | `compression::strategy` | 缺少自定义压缩器指南 | 🟡 中 |
| `DictionaryTrainer` | `compression::dictionary` | 内部类型但 pub | 🟢 低 |
| `CompactionManifest` | `compaction::manifest` | 内部类型但 pub | 🟢 低 |

**影响**：
- 用户无法了解如何正确使用这些 API
- 增加误用风险
- 降低开发体验

**建议**：
1. 为所有稳定层 API 补充完整文档
2. 添加使用示例
3. 标注稳定性层级

---

### 2.2 缺少文档注释的字段

以下配置字段缺少 `#[doc]` 注释：

| 类型 | 字段 | 问题 | 优先级 |
|------|------|------|--------|
| `FileKVConfig` | `l2_cache_max_bytes` | 缺少说明和推荐值 | 🔴 高 |
| `FileKVConfig` | `l2_to_l1_threshold` | 缺少说明和影响 | 🔴 高 |
| `FileKVConfig` | `segment_preallocate_size` | 缺少风险提示 | 🟡 中 |
| `AggressiveConfig` | `persistent_mmap_enabled` | 缺少持久化影响说明 | 🟡 中 |
| `BlockCacheConfig` | `frequency_aware` | 缺少性能影响说明 | 🟡 中 |
| `CompactionConfig` | `leveled_compaction_enabled` | 缺少 Level 配置说明 | 🟡 中 |

**影响**：
- 用户不知道如何配置
- 可能导致配置错误

**建议**：
1. 为所有公共字段添加 `#[doc]` 注释
2. 包含默认值和推荐值
3. 说明性能/安全性影响

---

### 2.3 缺少使用指南的模块

以下模块缺少独立的使用指南：

| 模块 | 缺少文档 | 影响 | 建议 |
|------|---------|------|------|
| **Bloom Filter** | 自适应 Bloom 配置指南 | 用户无法优化 FPR | 创建 `BLOOM_GUIDE.md` |
| **Compaction** | 压缩策略选择指南 | 用户不知道如何选择 | 创建 `COMPACTION_GUIDE.md` |
| **Cache** | 缓存调优指南 | 用户无法优化命中率 | 创建 `CACHE_TUNING.md` |
| **Checkpoint** | 检查点最佳实践 | 用户可能误用 | 创建 `CHECKPOINT_GUIDE.md` |
| **Compression** | 压缩算法选择指南 | 用户不知道如何选择 | 创建 `COMPRESSION_GUIDE.md` |

---

## 3. 过度暴露问题 (Over-Exposure Issues)

### 3.1 严重过度暴露 (Critical)

以下内部实现细节通过 `pub mod` 或 `pub use` 暴露，但用户不应直接使用：

| 类型/模块 | 暴露路径 | 问题 | 风险 |
|-----------|---------|------|------|
| `EngineState` 及其所有子状态 | `engine::state::*` | 引擎内部状态容器 | 用户可能依赖实现细节 |
| `EngineStateBuilder` | `engine::state::EngineStateBuilder` | 内部构造器 | 同上 |
| 引擎 trait | `engine::traits::*` | `ReadEngineAPI`, `WriteEngineAPI` 等 | 抽象泄漏 |
| `WalManager` / `WalEntry` | `core::wal::*` | WAL 内部组件 | 用户不应直接操作 WAL |
| `MemTable` / `MemTableConfig` | `core::memtable::*` | 内存表内部 | 同上 |
| `IndexManager` / `SparseIndex` | `core::sparse_index::*` | 索引内部实现 | 同上 |
| `GlobalKeyIndex` / `KeyLocation` | `core::global_index::*` | 全局索引内部 | 同上 |
| `FlushTrigger` | `core::flush::*` | 刷盘触发器 | 内部机制 |
| `WriteCoalescer` | `core::write_coalescer::*` | 写合并器 | 内部机制 |
| `FPRController` / `FPRAdjustedBloom` | `bloom::fpr_controller::*` | FPR 控制器 | 高级内部 |
| `MigrationController` / `MigrationThresholds` | `bloom::migration::*` | Bloom 迁移 | 高级内部 |
| `CustomBloom` / `CompressedBloom` | `bloom::custom_bloom`, `bloom::compressed` | Bloom 格式 | 实现细节 |
| `CacheBudget` / `SubBudget` | `cache::budget::*` | 缓存预算 | 内部跟踪 |
| `L2CacheManager` / `RebalanceConfig` | `cache::l2::*` | L2 缓存内部 | 内部组件 |
| `PerfTracker` / `PerfTimer` | `ops::perf::*` | 性能追踪 | 内部工具 |
| `AmplificationTracker` | `ops::amplification::*` | 放大分析 | 内部工具 |
| 具体压缩器 | `compression::strategy::*` | `ZstdCompressor` 等 | 应只暴露 trait |
| `CompactionExecutor` / `CompactionManifest` | `compaction::manifest::*` | 压缩清单 | 内部实现 |
| `BLOOM_MAGIC` / `BLOOM_VERSION` | `core::types::*` | 魔术常量 | 内部格式 |

**影响**：
- 🔴 API 表面过大，增加维护负担
- 🔴 用户可能依赖实现细节
- 🔴 未来重构受限
- 🔴 文档工作量大幅增加

**建议**：
1. **v0.6.0** 中将所有子模块改为 `pub(crate)`
2. 仅通过 `pub use` 在 crate 根导出稳定层 API
3. 内部类型标记 `#[doc(hidden)]`

---

### 3.2 中度过度暴露 (Moderate)

以下类型暴露但可能有一定合理性：

| 类型 | 问题 | 建议 |
|------|------|------|
| `SegmentFile` / `SegmentStats` | 段文件内部细节 | 高级用户可能需要，保留但标注实验 |
| `BloomManager` / `BloomConfig` | Bloom 管理器 | 高级用户可能需要，保留但标注实验 |
| `QuerySegmentProvider` trait | 范围扫描 segment 提供者 | 用户不需要自己实现，应 `pub(crate)` |
| `BlockCacheAsPrefetchCache` | 内部适配器 | 应 `pub(crate)` |
| `FilterWrapper` | 缓存内部包装 | 应 `pub(crate)` |

---

### 3.3 `#[doc(hidden)]` 矛盾

以下模块在 `core/mod.rs` 中标为 `#[doc(hidden)]`，但通过 `lib.rs` 的 `pub use` 导出，抵消了效果：

| 模块 | 矛盾点 |
|------|--------|
| `core::flush` | `#[doc(hidden)]` 但 `pub use FlushTrigger` |
| `core::wal_channel` | `#[doc(hidden)]` 但未重导出，一致 |
| `core::wal_batcher` | `#[doc(hidden)]` 但未重导出，一致 |
| `core::write_coalescer` | `#[doc(hidden)]` 但 `pub use WriteCoalescer` |
| `core::memtable_manager` | `#[doc(hidden)]` 但未重导出，一致 |

**问题**：
- `#[doc(hidden)]` 的目的是隐藏文档，但 `pub use` 又使其可见
- 语义矛盾

**建议**：
1. 如果要隐藏，不要 `pub use`
2. 如果要暴露，移除 `#[doc(hidden)]`

---

## 4. 不一致问题 (Inconsistencies)

### 4.1 文档与代码不一致

| 位置 | 问题 | 影响 |
|------|------|------|
| `lib.rs` 模块文档 | 提到 "Sparse Index" 但实际实现是 "DenseIndex" | 混淆用户 |
| `PERFORMANCE_BASELINE.md` | 部分性能数据与最新 benchmark 不一致 | 误导性能预期 |
| `README.md` | 架构描述缺少 Compression/Checkpoint/Ops 模块 | 不完整 |
| `FILEKV_GUIDE.md` | 范围扫描示例代码过时 | 示例无法运行 |

---

### 4.2 导出路径不一致

| 类型 | 问题 | 建议 |
|------|------|------|
| `FileKVConfig` | 通过 `core::config::FileKVConfig` 定义，`lib.rs` 重导出 | 统一路径 |
| `Durability` | 在 `core::types` 和 `core::wal` 都有定义 | 消除重复 |
| `CompactionStats` | 在 `compaction/mod.rs` 中 `pub` 但未 `pub use` | 显式导出 |
| `CompactionStrategy` | 枚举存在但未导出 | 如果用户需要，应导出 |

---

### 4.3 错误体系不完整

`core::error.rs` 定义了完整的错误层次：

```rust
pub enum FileKVError {
    Fatal(FatalError),
    Transient(TransientError),
    Expected(ExpectedError),
    Domain(DomainError),
}
```

但 `lib.rs` 只导出了 `FileKVConfigError`，未导出：
- `FileKVError`
- `FatalError`
- `TransientError`
- `ExpectedError`
- `DomainError`
- `ErrorCategory`

**问题**：
- 用户无法模式匹配错误类型
- 错误处理不完整

**建议**：
1. 导出完整错误层次（稳定层）
2. 或明确说明用户只需处理 `anyhow::Error`

---

## 5. 缺失的 API 稳定性承诺

**审查前状态**：❌ 无 API 稳定性承诺文档

**问题**：
- 用户不知道哪些 API 稳定
- 无法规划升级路径
- 增加维护负担

**已修复**：
- ✅ 创建 `docs/API_STABILITY.md`
- ✅ 定义三层稳定性层级 (Stable/Experimental/Internal)
- ✅ 列出稳定层 API 清单
- ✅ 制定变更政策
- ✅ 制定弃用政策

---

## 6. 优先级排序

### P0 (立即修复)

| 问题 | 修复方案 | 状态 |
|------|---------|------|
| 缺少 API 稳定性承诺 | 创建 `API_STABILITY.md` | ✅ 已修复 |
| 缺少 API 参考文档 | 创建 `API_REFERENCE.md` | ✅ 已修复 |
| `CheckpointChain` 缺少方法文档 | 补充文档注释 | ⏳ 待修复 |
| `IncrementalCheckpointManager` 缺少文档 | 补充文档 | ⏳ 待修复 |

### P1 (下次版本修复)

| 问题 | 修复方案 | 影响版本 |
|------|---------|---------|
| 过度暴露的内部类型 | 子模块改 `pub(crate)` | v0.6.0 |
| 配置字段缺少文档 | 补充 `#[doc]` 注释 | v0.5.1 |
| 错误体系未导出 | 决定策略并实施 | v0.6.0 |
| `#[doc(hidden)]` 矛盾 | 统一策略 | v0.5.1 |

### P2 (未来修复)

| 问题 | 修复方案 | 影响版本 |
|------|---------|---------|
| 缺少使用指南 | 创建各模块指南 | v0.6.0 |
| 文档与代码不一致 | 同步更新 | v0.5.1 |
| 导出路径不一致 | 统一路径 | v0.6.0 |

---

## 7. 建议的行动计划

### 短期 (v0.5.1)

- [ ] 为所有稳定层 API 补充文档注释
- [ ] 修复 `#[doc(hidden)]` 矛盾
- [ ] 同步文档与代码
- [ ] 补充配置字段文档

### 中期 (v0.6.0)

- [ ] 重构模块可见性 (`pub mod` → `pub(crate)`)
- [ ] 仅通过 `pub use` 导出稳定层 API
- [ ] 导出完整错误层次（或明确策略）
- [ ] 创建使用指南文档

### 长期 (v1.0.0)

- [ ] 稳定所有稳定层 API
- [ ] 移除实验层 API 或正式标记
- [ ] 完整文档覆盖
- [ ] 迁移指南

---

## 8. 总结

tokitai-filekv 项目在 API 文档方面存在**系统性不足**：

1. **文档疏漏**：12 个公共 API 缺少文档或示例
2. **过度暴露**：45+ 内部类型暴露给用户
3. **不一致**：8 处文档与代码不一致
4. **缺失承诺**：无 API 稳定性承诺（已修复）

**根本原因**：
- 模块可见性失控（所有子模块 `pub mod`）
- 缺乏 API 设计审查
- 文档更新滞后于代码开发

**改进建议**：
1. **建立 API 审查流程**：每次 PR 检查公共 API 变更
2. **文档先行**：新功能先写文档，再实现代码
3. **自动化检查**：CI 检查 `#[doc]` 覆盖率
4. **定期审查**：每季度审查 API 表面

---

**本报告是 tokitai-filekv API 文档改进的基准文件，后续改进应以本报告为参考。**
