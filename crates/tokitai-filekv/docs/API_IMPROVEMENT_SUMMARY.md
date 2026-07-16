# API 文档改进总结

**改进日期**: 2026-04-16
**改进范围**: API 文档疏漏检查、稳定性承诺、参考文档

---

## 1. 已完成的工作

### 1.1 创建的新文档

| 文档 | 路径 | 说明 | 状态 |
|------|------|------|------|
| **API 稳定性承诺** | `docs/API_STABILITY.md` | 定义三层稳定性层级，列出稳定层 API 清单，制定变更/弃用政策 | ✅ 完成 |
| **API 参考文档** | `docs/API_REFERENCE.md` | 完整 API 参考，包含方法签名、字段说明、稳定性标识 | ✅ 完成 |
| **API 审查报告** | `docs/API_REVIEW.md` | 文档疏漏、过度暴露、不一致问题分析和改进建议 | ✅ 完成 |

### 1.2 文档覆盖的 API 范围

**稳定层 API** (42 个核心类型)：
- ✅ FileKV 主类型 (21 个方法)
- ✅ 配置类型 (FileKVConfig, AggressiveConfig, 等)
- ✅ 核心操作 (put, get, delete, put_batch, range, 等)
- ✅ 缓存 API (BlockCache, UnifiedCacheManager, CacheWarmer, 等)
- ✅ 压缩 API (CompressionStrategy, DictionaryCompressor, 等)
- ✅ 范围扫描 API (RangeScanIterator, RangeScanConfig, 等)
- ✅ 检查点 API (CheckpointChain, IncrementalCheckpointManager, 等)
- ✅ 监控 API (MemoryTracker, TimeoutConfig, AuditLogger, 等)

**实验层 API** (15+ 个高级类型)：
- ⚠️ 异步 I/O (AsyncWriter, 等)
- ⚠️ 故障注入 (FaultInjector, 等)
- ⚠️ 高级 Bloom (AdaptiveBloomCache, FPRController, 等)
- ⚠️ Prometheus 指标 (FileKVMetrics, 等)

**内部层 API** (45+ 个内部类型)：
- 🔒 引擎内部 (EngineState, 等)
- 🔒 WAL 内部 (WalManager, 等)
- 🔒 索引内部 (IndexManager, 等)
- 🔒 Bloom 格式 (CustomBloom, 等)

---

## 2. 发现的问题

### 2.1 文档疏漏 (12 个)

| 优先级 | 问题 | 影响 |
|--------|------|------|
| 🔴 高 | CheckpointChain 方法缺少文档 | 用户无法使用检查点功能 |
| 🔴 高 | IncrementalCheckpointManager 缺少文档 | 同上 |
| 🔴 高 | 配置字段缺少 `#[doc]` 注释 | 用户不知道如何配置 |
| 🟡 中 | ZoneMap 相关组件缺少 INNO-002 文档 | 高级用户无法优化 |
| 🟡 中 | FeatureFlagController 缺少指南 | 用户无法使用功能开关 |

### 2.2 过度暴露 (45+ 个)

| 严重性 | 类型 | 数量 | 风险 |
|--------|------|------|------|
| 🔴 严重 | 引擎内部状态 | 7 | 用户依赖实现细节 |
| 🔴 严重 | WAL/MemTable/索引内部 | 15 | 内部机制泄漏 |
| 🟡 中等 | Bloom 格式/版本 | 8 | 实现细节暴露 |
| 🟡 中等 | 缓存内部组件 | 6 | 内部机制泄漏 |
| 🟢 低 | 性能追踪工具 | 5 | 内部工具暴露 |

### 2.3 不一致 (8 个)

| 类型 | 问题 | 影响 |
|------|------|------|
| 文档 vs 代码 | 模块文档提到 "Sparse Index" 但实现是 "DenseIndex" | 混淆 |
| 导出路径 | 部分类型导出路径不一致 | 困惑 |
| `#[doc(hidden)]` | 标记了但仍 `pub use` | 矛盾 |
| 错误体系 | 完整层次存在但未导出 | 错误处理不完整 |

---

## 3. 稳定性层级定义

### 3.1 稳定层 (Stable API) ✅

**保证**：
- 主版本号升级前保证向后兼容
- 方法签名不变
- 类型字段不删除
- 行为语义向后兼容

**包含**：42 个核心类型

### 3.2 实验层 (Experimental API) ⚠️

**保证**：
- 当前版本内尽量稳定
- 次版本升级时可能变更
- 变更会在 CHANGELOG 中标注

**包含**：15+ 个高级类型

### 3.3 内部层 (Internal API) 🔒

**保证**：
- 无稳定性保证
- 可能在任何版本中变更
- 用户不应直接使用

**包含**：45+ 个内部类型

---

## 4. 后续建议

### 4.1 短期 (v0.5.1)

**优先级**: P0/P1

- [ ] **补充文档注释**
  - 为所有稳定层 API 添加完整 `#[doc]` 注释
  - 包含使用示例
  - 说明默认值和推荐值

- [ ] **修复 `#[doc(hidden)]` 矛盾**
  - 决定策略：隐藏则不导出，导出则不隐藏
  - 统一实施

- [ ] **同步文档与代码**
  - 更新模块文档 (Sparse Index → DenseIndex)
  - 同步性能基线数据
  - 补充 Compression/Checkpoint/Ops 模块说明

- [ ] **导出配置字段文档**
  - 为所有 `FileKVConfig` 字段添加说明
  - 为 `AggressiveConfig` 字段添加说明
  - 为 `CompactionConfig` 字段添加说明

### 4.2 中期 (v0.6.0)

**优先级**: P1

- [ ] **重构模块可见性**
  ```rust
  // 当前
  pub mod io;
  pub mod cache;
  pub mod engine;
  pub mod core;
  pub mod bloom;
  // ...
  
  // 改为
  pub(crate) mod cache;
  pub(crate) mod engine;
  pub(crate) mod core;
  pub(crate) mod bloom;
  // ...
  // 仅通过 pub use 在 crate 根导出稳定层 API
  ```

- [ ] **清理内部类型暴露**
  - 移除 45+ 内部类型的 `pub` 暴露
  - 仅保留稳定层 API 公开

- [ ] **决定错误处理策略**
  - 方案 A: 导出完整错误层次 (`FileKVError`, `FatalError`, 等)
  - 方案 B: 明确说明用户只需处理 `anyhow::Error`
  - 实施选定方案

- [ ] **创建使用指南**
  - `BLOOM_GUIDE.md` - 自适应 Bloom 配置指南
  - `COMPACTION_GUIDE.md` - 压缩策略选择指南
  - `CACHE_TUNING.md` - 缓存调优指南
  - `CHECKPOINT_GUIDE.md` - 检查点最佳实践
  - `COMPRESSION_GUIDE.md` - 压缩算法选择指南

### 4.3 长期 (v1.0.0)

**优先级**: P2

- [ ] **稳定所有稳定层 API**
  - 确保无破坏性变更
  - 完整测试覆盖

- [ ] **移除或正式标记实验层 API**
  - 决定哪些 API 晋升为稳定层
  - 移除或保持实验层标记

- [ ] **完整文档覆盖**
  - 100% `#[doc]` 覆盖率
  - 所有公共 API 有示例

- [ ] **迁移指南**
  - 为 v0.x → v1.0 编写迁移指南
  - 标注所有破坏性变更

---

## 5. API 审查流程建议

### 5.1 PR 检查清单

每次 PR 应检查：

- [ ] 是否新增/修改公共 API？
- [ ] 是否破坏向后兼容性？
- [ ] 是否补充了文档注释？
- [ ] 是否更新了 API_REFERENCE.md？
- [ ] 是否更新了 CHANGELOG.md？
- [ ] 是否影响了稳定层 API？

### 5.2 自动化检查

建议 CI 集成：

```bash
# 检查文档覆盖率
cargo doc --no-deps 2>&1 | grep "missing documentation"

# 检查公共 API 变更
cargo public-items --diff-with-HEAD

# 检查破坏性变更
cargo semver-checks
```

### 5.3 定期审查

- **每月**: 检查新暴露的内部类型
- **每季度**: 全面审查 API 表面
- **每版本**: 更新 API 文档

---

## 6. 文档维护责任

| 文档 | 维护者 | 更新频率 |
|------|--------|---------|
| `API_STABILITY.md` | 核心团队 | 版本发布时 |
| `API_REFERENCE.md` | 核心团队 + 贡献者 | API 变更时 |
| `API_REVIEW.md` | 核心团队 | 季度审查时 |
| 使用指南 | 贡献者 | 按需 |

---

## 7. 贡献者指南

### 7.1 如何添加 API 文档

```rust
/// 简短说明 (一行)
///
/// 详细说明 (多行)
///
/// # Example
///
/// ```rust
/// use tokitai_filekv::ExampleType;
///
/// let example = ExampleType::new();
/// assert!(example.is_valid());
/// ```
///
/// # Panics
///
/// 说明 panic 条件
///
/// # Errors
///
/// 说明错误条件
///
/// # Stability
///
/// ✅ 稳定 | ⚠️ 实验 | 🔒 内部
pub struct ExampleType {
    /// 字段说明
    pub field: String,
}
```

### 7.2 如何标记稳定性

在文档末尾添加：

```rust
/// ...
///
/// # Stability
///
/// ✅ 稳定 (v0.5.0+)
```

或

```rust
/// ...
///
/// # Stability
///
/// ⚠️ 实验 (可能在次版本中变更)
```

---

## 8. 总结

本次 API 文档审查发现 tokitai-filekv 项目存在**系统性文档不足**和**过度暴露**问题，但已通过以下工作显著改善：

1. ✅ 创建 `docs/API_STABILITY.md` - 建立稳定性承诺
2. ✅ 创建 `docs/API_REFERENCE.md` - 完整 API 参考
3. ✅ 创建 `docs/API_REVIEW.md` - 问题分析和改进建议

**下一步**：按照本报告的建议，分阶段修复所有问题，最终实现：
- 100% 稳定层 API 文档覆盖
- 清洁的 API 表面 (无内部泄漏)
- 明确的稳定性承诺
- 完善的使用指南

---

**本文档是 tokitai-filekv API 文档改进的总结文件，后续工作应以本报告为参考。**
