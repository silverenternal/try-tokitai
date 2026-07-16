# FileKV 模块解耦与并行开发改进计划

**创建日期**: 2026-04-13
**状态**: Phase 0-2 已完成，待执行 Phase 3+
**目标**: 通过彻底解耦、消除游离文件、定义清晰接口，使 FileKV 支持 3-5 人团队并行开发

---

## 一、现状评估

### 1.1 当前架构（Phase 2 完成后）

```
FileKV (lib.rs - 1142 行)
├── engine_state: Arc<EngineState>          # 共享状态容器（已拆分为 5 个子容器）
│   ├── segment_state: Arc<SegmentState>    # Segment 文件 + ID 分配
│   ├── index_state: Arc<IndexState>        # 稀疏索引管理
│   ├── memtable_state: Arc<MemTableState>  # 内存缓冲
│   ├── cache_state: Arc<CacheState>        # 所有缓存层
│   └── stats_state: Arc<StatsState>        # 统计计数器
├── read_engine: Arc<ReadEngine>            # 读引擎（已实现 ReadEngineAPI）
├── write_engine: Arc<WriteEngine>          # 写引擎（已实现 WriteEngineAPI）
├── compaction_engine: Arc<CompactionEngine> # 压缩引擎（已实现 CompactionEngineAPI）
├── lifecycle_manager: Arc<LifecycleManager> # 生命周期管理（已实现 LifecycleManagerAPI）
└── metrics (feature-gated)
```

### 1.2 已完成的解耦工作

| Phase | 任务 | 状态 | 备注 |
|-------|------|------|------|
| **Phase 0** | bloom.rs → BloomManager + BloomSegmentProvider | ✅ 完成 | Trait 定义清晰，可独立测试 |
| **Phase 0** | compaction/mod.rs → CompactionContext trait | ✅ 完成 | 不再直接依赖 `&FileKV` |
| **Phase 1** | 定义 4 个引擎 trait 接口 | ✅ 完成 | ReadEngineAPI, WriteEngineAPI, CompactionEngineAPI, LifecycleManagerAPI |
| **Phase 2** | EngineState 拆分为 5 个子容器 | ✅ 完成 | SegmentState, IndexState, MemTableState, CacheState, StatsState |
| **Phase 3** | 删除死代码 init_components.rs | ✅ 完成 | 已确认零引用 |
| **Bug Fix** | 修复压缩死锁（锁顺序问题） | ✅ 完成 | 添加 `drop(segments)` 释放锁 |

**测试状态**: 300/300 tests pass, cargo check passes

### 1.3 遗留问题清单

#### 🔴 问题 1：40 个游离文件在 src/ 根目录（严重程度：🔴🔴🔴）

**现状**：`src/` 目录下有 40 个 `.rs` 文件直接放在根目录，未按功能分组。

**影响**：
- 无法快速定位相关代码
- 模块边界模糊，职责不清
- 新成员难以理解架构
- 违反 Rust 模块最佳实践

**游离文件清单**（按功能分组）：

| 功能组 | 文件数 | 文件列表 | 总行数 |
|--------|--------|----------|--------|
| **存储核心** | 8 | memtable, segment, sparse_index, wal, flush, write_coalescer, types, config | ~4,500 |
| **缓存系统** | 4 | block_cache, cache_warmer, sequential_prefetcher, config (re-export) | ~1,200 |
| **Bloom 生态** | 7 | adaptive_bloom_cache, bloom_filter_cache, bloom_migration, compressed_bloom, fpr_controller, compression, amplification_analysis | ~5,500 |
| **查询优化** | 3 | zone_map, range_query_pruner, range_scan | ~2,000 |
| **Checkpoint** | 5 | checkpoints, incremental_checkpoint, incremental_manager, incremental_types, incremental_tests | ~1,300 |
| **Compaction 扩展** | 3 | compaction_manifest, compaction_manifest_crash_tests, compaction_trigger | ~1,300 |
| **运维/特性** | 8 | feature_flag, feature_flag_tests, metrics_prometheus, async_io, audit_log, timeout_control, memory_tracker, adaptive_preallocator | ~3,500 |
| **测试文件** | 5 | tests, tests_batch_atomic, tests_phase6_write_buffer, tests_range_query, tests_wal_recovery, stability_test | ~2,500 |
| **错误/工具** | 2 | error.rs, error.rs | ~750 |

#### 🟡 问题 2：lib.rs 过于臃肿（严重程度：🟡🟡）

**现状**：
- `lib.rs` 共 1142 行
- 包含 40+ 个模块声明
- `FileKV::open()` 包含 300+ 行初始化逻辑
- 大量 `pub use` 导出

**影响**：
- 单文件过长，难以维护
- 修改任何底层模块都需要重新编译 lib.rs
- 模块导出逻辑与业务逻辑混合

#### 🟡 问题 3：range_scan.rs 重度依赖 FileKV（严重程度：🟡🟡）

**现状**：
```rust
// range_scan.rs - 依赖多个 crate 级类型
use crate::{FileKV, SegmentFile, RangeQueryPruner, SequentialPrefetcher, PrefetchCache, ZoneMapIndex};
```

**问题**：
- 依赖主类型 `FileKV`，形成循环依赖风险
- 无法独立测试（需要完整 FileKV 实例）
- 应改为接受 trait 或具体类型

---

## 二、Phase 3+：彻底模块化重构方案

### 2.1 方案设计原则

1. **零游离文件**：所有 `.rs` 文件必须属于某个子目录
2. **依赖单向**：底层模块不依赖高层模块
3. **Trait 解耦**：跨模块通信通过 trait，而非具体类型
4. **渐进式**：每个子阶段可独立验证，保持测试通过
5. **保持性能**：不因模块化引入额外开销

### 2.2 目标目录结构

```
src/
├── lib.rs                          # 主入口（精简到 <200 行）
│
├── core/                           # 存储核心（新建）
│   ├── mod.rs
│   ├── types.rs                    # 核心类型定义
│   ├── config.rs                   # 配置类型
│   ├── error.rs                    # 错误体系
│   ├── memtable.rs                 # 内存缓冲
│   ├── segment.rs                  # 段文件 I/O
│   ├── sparse_index.rs             # 稀疏索引
│   ├── wal.rs                      # Write-Ahead Log
│   └── flush.rs                    # 后台刷盘
│
├── cache/                          # 缓存系统（扩展现有）
│   ├── mod.rs                      # UnifiedCacheManager
│   ├── budget.rs                   # 内存预算
│   ├── block_cache.rs              # 热数据缓存
│   ├── warmup.rs                   # cache_warmer 重命名
│   └── prefetch.rs                 # sequential_prefetcher 移入
│
├── bloom/                          # Bloom 生态（扩展现有）
│   ├── mod.rs
│   ├── manager.rs                  # BloomManager
│   ├── filter_cache.rs             # bloom_filter_cache 重命名
│   ├── adaptive.rs                 # adaptive_bloom_cache 重命名
│   ├── compressed.rs               # compressed_bloom 重命名
│   ├── migration.rs                # bloom_migration 重命名
│   └── fpr_controller.rs
│
├── query/                          # 查询优化（新建）
│   ├── mod.rs
│   ├── zone_map.rs                 # Zone Map 数据结构
│   ├── pruner.rs                   # range_query_pruner 重命名
│   └── scan.rs                     # range_scan 重命名
│
├── compaction/                     # 压缩系统（扩展现有）
│   ├── mod.rs                      # CompactionManager + execute_compaction
│   ├── merge_iterator.rs           # K-way merge
│   ├── segment_iterator.rs         # 段流式迭代
│   ├── manifest.rs                 # compaction_manifest 重命名
│   ├── manifest_crash_tests.rs     # 崩溃测试
│   └── trigger.rs                  # compaction_trigger 重命名
│
├── checkpoint/                     # 检查点系统（新建）
│   ├── mod.rs
│   ├── manager.rs                  # incremental_manager 重命名
│   ├── types.rs                    # incremental_types 重命名
│   └── tests.rs                    # incremental_tests 重命名
│
├── engine/                         # 引擎层（已存在，保持不变）
│   ├── mod.rs
│   ├── read_engine.rs
│   ├── write_engine.rs
│   ├── compaction_engine.rs
│   ├── lifecycle.rs
│   ├── traits.rs
│   ├── state.rs
│   └── tests.rs
│
├── io/                             # I/O 抽象（已存在，保持不变）
│   ├── mod.rs
│   ├── stdfs.rs
│   ├── memfs.rs
│   └── fault_inject.rs
│
├── ops/                            # 运维特性（新建）
│   ├── mod.rs
│   ├── feature_flag.rs
│   ├── feature_flag_tests.rs
│   ├── metrics.rs                  # metrics_prometheus 重命名
│   ├── async_io.rs                 # feature-gated
│   ├── audit_log.rs
│   ├── timeout_control.rs
│   ├── memory_tracker.rs
│   ├── preallocator.rs             # adaptive_preallocator 重命名
│   └── amplification.rs            # amplification_analysis 重命名
│
├── compression/                    # 压缩算法（新建，从 bloom 拆分）
│   ├── mod.rs
│   └── dictionary.rs               # compression.rs 重命名
│
└── tests/                          # 集成测试（新建）
    ├── mod.rs                      # tests.rs
    ├── batch_atomic.rs             # tests_batch_atomic
    ├── write_buffer.rs             # tests_phase6_write_buffer
    ├── range_query.rs              # tests_range_query
    ├── wal_recovery.rs             # tests_wal_recovery
    └── stability.rs                # stability_test
```

### 2.3 模块依赖关系图（目标状态）

```
lib.rs (facade)
  │
  ├── core/ (存储核心 - 叶子模块)
  │     ├── types.rs ← (无内部依赖)
  │     ├── error.rs ← (无内部依赖)
  │     ├── memtable.rs → types, error
  │     ├── segment.rs → error, io
  │     ├── wal.rs → error, io
  │     └── sparse_index.rs → zone_map (query/)
  │
  ├── cache/ (缓存系统)
  │     ├── block_cache.rs ← (叶子模块)
  │     ├── warmup.rs → block_cache
  │     └── prefetch.rs → block_cache
  │
  ├── bloom/ (Bloom 生态)
  │     ├── manager.rs → core::segment
  │     ├── filter_cache.rs → error
  │     ├── adaptive.rs → compressed, migration, fpr_controller
  │     ├── compressed.rs ← (叶子模块)
  │     ├── migration.rs → filter_cache
  │     └── fpr_controller.rs → migration
  │
  ├── query/ (查询优化)
  │     ├── zone_map.rs ← (叶子模块)
  │     ├── pruner.rs → zone_map
  │     └── scan.rs → zone_map, pruner, core::segment, cache::prefetch
  │
  ├── compaction/ (压缩系统)
  │     ├── mod.rs → core::{segment, sparse_index}, query::zone_map
  │     ├── manifest.rs → io
  │     └── trigger.rs → core::types
  │
  ├── checkpoint/ (检查点)
  │     ├── manager.rs → error, types
  │     └── types.rs ← (叶子模块)
  │
  ├── compression/ (压缩算法)
  │     └── dictionary.rs → core::types
  │
  ├── ops/ (运维特性)
  │     ├── feature_flag.rs ← (叶子模块)
  │     ├── metrics.rs ← (叶子模块, feature-gated)
  │     ├── audit_log.rs → core::types
  │     └── ... (其他叶子或轻量模块)
  │
  ├── engine/ (引擎层 - 已解耦)
  │     ├── read_engine.rs → core, cache, bloom, query
  │     ├── write_engine.rs → core, compaction, compression
  │     ├── compaction_engine.rs → core, compaction
  │     └── lifecycle.rs → core, checkpoint, ops
  │
  └── io/ (I/O 抽象 - 叶子模块)
        └── (零内部依赖)
```

**关键依赖规则**：
- `core/` 不依赖任何其他业务模块（仅依赖 `io/`, `error/`, `types/`）
- `query/scan.rs` 不依赖 `FileKV`，改为接受 trait 或具体类型参数
- `engine/` 依赖所有下层模块，但通过 trait 接口暴露
- `ops/` 全部为叶子模块或仅依赖 `core/types`

---

## 三、实施路线图

### Phase 3.1：建立目录骨架（2-3h）

| 任务 | 工作量 | 优先级 | 状态 |
|------|--------|--------|------|
| 创建 `core/` 目录及 mod.rs | 0.5h | P0 | ⬜ 未开始 |
| 创建 `query/` 目录及 mod.rs | 0.5h | P0 | ⬜ 未开始 |
| 创建 `checkpoint/` 目录及 mod.rs | 0.5h | P0 | ⬜ 未开始 |
| 创建 `ops/` 目录及 mod.rs | 0.5h | P0 | ⬜ 未开始 |
| 创建 `compression/` 目录及 mod.rs | 0.5h | P0 | ⬜ 未开始 |
| 创建 `tests/` 目录及 mod.rs | 0.5h | P1 | ⬜ 未开始 |
| 更新 `lib.rs` 模块声明 | 1h | P0 | ⬜ 未开始 |

**验收**：`cargo check` 通过（允许编译错误，仅验证模块声明）

---

### Phase 3.2：迁移存储核心到 core/（4-6h）

| 任务 | 工作量 | 优先级 | 状态 |
|------|--------|--------|------|
| 移动 `types.rs` → `core/types.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `config.rs` → `core/config.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `error.rs` → `core/error.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `memtable.rs` → `core/memtable.rs` | 1h | P0 | ⬜ 未开始 |
| 移动 `segment.rs` → `core/segment.rs` | 1h | P0 | ⬜ 未开始 |
| 移动 `sparse_index.rs` → `core/sparse_index.rs` | 1h | P0 | ⬜ 未开始 |
| 移动 `wal.rs` → `core/wal.rs` | 1h | P0 | ⬜ 未开始 |
| 移动 `flush.rs` → `core/flush.rs` | 0.5h | P1 | ⬜ 未开始 |
| 更新所有 `crate::xxx` 引用 | 1h | P0 | ⬜ 未开始 |
| 运行测试验证 | 1h | P0 | ⬜ 未开始 |

**验收**：300/300 tests pass

---

### Phase 3.3：迁移缓存系统到 cache/（2-3h）

| 任务 | 工作量 | 优先级 | 状态 |
|------|--------|--------|------|
| 移动 `block_cache.rs` → `cache/block_cache.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `cache_warmer.rs` → `cache/warmup.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `sequential_prefetcher.rs` → `cache/prefetch.rs` | 1h | P0 | ⬜ 未开始 |
| 更新 `cache/mod.rs` 导出 | 0.5h | P0 | ⬜ 未开始 |
| 更新所有引用 | 0.5h | P0 | ⬜ 未开始 |
| 运行测试验证 | 0.5h | P0 | ⬜ 未开始 |

**验收**：300/300 tests pass

---

### Phase 3.4：迁移 Bloom 生态到 bloom/（4-6h）

| 任务 | 工作量 | 优先级 | 状态 |
|------|--------|--------|------|
| 移动 `bloom_filter_cache.rs` → `bloom/filter_cache.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `adaptive_bloom_cache.rs` → `bloom/adaptive.rs` | 1h | P0 | ⬜ 未开始 |
| 移动 `compressed_bloom.rs` → `bloom/compressed.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `bloom_migration.rs` → `bloom/migration.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `fpr_controller.rs` → `bloom/fpr_controller.rs` | 1h | P0 | ⬜ 未开始 |
| 移动 `compression.rs` → `compression/dictionary.rs` (新建 compression/) | 0.5h | P0 | ⬜ 未开始 |
| 移动 `amplification_analysis.rs` → `ops/amplification.rs` | 0.5h | P1 | ⬜ 未开始 |
| 更新 `bloom/mod.rs` 导出 | 0.5h | P0 | ⬜ 未开始 |
| 更新所有引用（最复杂，涉及 2204 行 adaptive.rs） | 1h | P0 | ⬜ 未开始 |
| 运行测试验证 | 1h | P0 | ⬜ 未开始 |

**验收**：300/300 tests pass

---

### Phase 3.5：迁移查询优化到 query/（3-4h）

| 任务 | 工作量 | 优先级 | 状态 |
|------|--------|--------|------|
| 移动 `zone_map.rs` → `query/zone_map.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `range_query_pruner.rs` → `query/pruner.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `range_scan.rs` → `query/scan.rs` | 1h | P0 | ⬜ 未开始 |
| **重构 scan.rs** 消除 `FileKV` 依赖 | 1h | P0 | ⬜ 未开始 |
| 更新 `query/mod.rs` 导出 | 0.5h | P0 | ⬜ 未开始 |
| 更新所有引用 | 0.5h | P0 | ⬜ 未开始 |
| 运行测试验证 | 0.5h | P0 | ⬜ 未开始 |

**关键重构**：`range_scan.rs` 当前依赖 `FileKV`，需要改为：
```rust
// 旧代码
use crate::FileKV;

fn scan_range(kv: &FileKV, range: impl RangeBounds<String>) -> Result<...> {
    let segments = kv.segments().read();
    // ...
}

// 新代码 - 使用 trait
pub trait SegmentProvider {
    fn get_segments(&self) -> Vec<Arc<SegmentFile>>;
}

fn scan_range<P: SegmentProvider>(provider: &P, range: impl RangeBounds<String>) -> Result<...> {
    let segments = provider.get_segments();
    // ...
}
```

**验收**：300/300 tests pass

---

### Phase 3.6：迁移 Checkpoint 系统到 checkpoint/（2-3h）

| 任务 | 工作量 | 优先级 | 状态 |
|------|--------|--------|------|
| 移动 `incremental_checkpoint.rs` → `checkpoint/mod.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `incremental_manager.rs` → `checkpoint/manager.rs` | 1h | P0 | ⬜ 未开始 |
| 移动 `incremental_types.rs` → `checkpoint/types.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `incremental_tests.rs` → `checkpoint/tests.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `checkpoints.rs` → `checkpoint/filekv_impl.rs` | 0.5h | P0 | ⬜ 未开始 |
| 更新所有引用 | 0.5h | P0 | ⬜ 未开始 |
| 运行测试验证 | 0.5h | P0 | ⬜ 未开始 |

**验收**：300/300 tests pass

---

### Phase 3.7：迁移运维特性到 ops/（3-4h）

| 任务 | 工作量 | 优先级 | 状态 |
|------|--------|--------|------|
| 移动 `feature_flag.rs` → `ops/feature_flag.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `feature_flag_tests.rs` → `ops/feature_flag_tests.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `metrics_prometheus.rs` → `ops/metrics.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `async_io.rs` → `ops/async_io.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `audit_log.rs` → `ops/audit_log.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `timeout_control.rs` → `ops/timeout_control.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `memory_tracker.rs` → `ops/memory_tracker.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `adaptive_preallocator.rs` → `ops/preallocator.rs` | 0.5h | P0 | ⬜ 未开始 |
| 更新所有引用 | 0.5h | P0 | ⬜ 未开始 |
| 运行测试验证 | 0.5h | P0 | ⬜ 未开始 |

**验收**：300/300 tests pass

---

### Phase 3.8：迁移测试文件到 tests/（2-3h）

| 任务 | 工作量 | 优先级 | 状态 |
|------|--------|--------|------|
| 移动 `tests.rs` → `tests/integration.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `tests_batch_atomic.rs` → `tests/batch_atomic.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `tests_phase6_write_buffer.rs` → `tests/write_buffer.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `tests_range_query.rs` → `tests/range_query.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `tests_wal_recovery.rs` → `tests/wal_recovery.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `stability_test.rs` → `tests/stability.rs` | 0.5h | P1 | ⬜ 未开始 |
| 更新 `lib.rs` 中的 mod 声明 | 0.5h | P0 | ⬜ 未开始 |
| 运行测试验证 | 0.5h | P0 | ⬜ 未开始 |

**验收**：300/300 tests pass

---

### Phase 3.9：迁移 Compaction 扩展（2-3h）

| 任务 | 工作量 | 优先级 | 状态 |
|------|--------|--------|------|
| 移动 `compaction_manifest.rs` → `compaction/manifest.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `compaction_manifest_crash_tests.rs` → `compaction/manifest_crash_tests.rs` | 0.5h | P0 | ⬜ 未开始 |
| 移动 `compaction_trigger.rs` → `compaction/trigger.rs` | 0.5h | P0 | ⬜ 未开始 |
| 更新 `compaction/mod.rs` 导出 | 0.5h | P0 | ⬜ 未开始 |
| 更新所有引用 | 0.5h | P0 | ⬜ 未开始 |
| 运行测试验证 | 0.5h | P0 | ⬜ 未开始 |

**验收**：300/300 tests pass

---

### Phase 3.10：精简 lib.rs（4-6h）

**目标**：将 `lib.rs` 从 1142 行精简到 <200 行

| 任务 | 工作量 | 优先级 | 状态 |
|------|--------|--------|------|
| 将模块声明移至 `lib/mod.rs` 或保持顶层 | 0.5h | P0 | ⬜ 未开始 |
| 将 `pub use` 导出分组到各子模块的 `mod.rs` | 1h | P0 | ⬜ 未开始 |
| 将 `FileKV::open()` 初始化逻辑拆分到 `engine/lifecycle.rs` | 2h | P0 | ⬜ 未开始 |
| 将 `FileKV` impl 方法按功能分组委托到引擎 | 1h | P0 | ⬜ 未开始 |
| 消除 `config.rs` 冗余 re-export | 0.5h | P0 | ⬜ 未开始 |
| 运行 cargo clippy 验证 | 0.5h | P0 | ⬜ 未开始 |
| 运行测试验证 | 0.5h | P0 | ⬜ 未开始 |

**重构策略**：
```rust
// lib.rs (目标 <200 行)
mod core;
mod cache;
mod bloom;
mod query;
mod compaction;
mod checkpoint;
mod engine;
mod io;
mod ops;
mod compression;
mod tests;

// 仅导出公共 API
pub use core::{FileKVConfig, FileKVStats, ValuePointer, Durability};
pub use core::error::FileKVError;
pub use engine::{ReadEngine, WriteEngine, CompactionEngine, LifecycleManager};
pub use io::{FileKVFileSystem, StdFs, MemFs};

// FileKV 结构体保持在此
pub struct FileKV { /* ... */ }

impl FileKV {
    pub fn open(config: FileKVConfig) -> anyhow::Result<Self> {
        // 委托到 LifecycleManager 或初始化组件
        engine::LifecycleManager::open_components(config)
    }
    
    // 其他方法保持薄封装，委托到具体引擎
}
```

**验收**：
- 300/300 tests pass
- lib.rs < 200 行
- cargo clippy 无新增警告

---

## 四、总工作量估算

| Phase | 乐观 | 悲观 | 风险等级 |
|-------|------|------|---------|
| Phase 3.1: 建立骨架 | 2h | 3h | 低 |
| Phase 3.2: 迁移 core/ | 4h | 6h | 中（引用更新量大） |
| Phase 3.3: 迁移 cache/ | 2h | 3h | 低 |
| Phase 3.4: 迁移 bloom/ | 4h | 6h | 高（adaptive.rs 2204 行） |
| Phase 3.5: 迁移 query/ | 3h | 4h | 高（range_scan 依赖 FileKV） |
| Phase 3.6: 迁移 checkpoint/ | 2h | 3h | 低 |
| Phase 3.7: 迁移 ops/ | 3h | 4h | 低 |
| Phase 3.8: 迁移 tests/ | 2h | 3h | 低 |
| Phase 3.9: 迁移 compaction/ | 2h | 3h | 低 |
| Phase 3.10: 精简 lib.rs | 4h | 6h | 中（初始化逻辑复杂） |
| **总计** | **28h** | **41h** | - |

---

## 五、风险与缓解

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|---------|
| 路径更新遗漏导致编译错误 | 高 | 中 | 使用 IDE 全局搜索 + `cargo check` 快速验证 |
| `adaptive_bloom_cache.rs` (2204 行) 迁移复杂 | 中 | 中 | 拆分为更小模块后再迁移 |
| `range_scan.rs` 依赖 FileKV 难以解耦 | 高 | 低 | 提前设计 trait 接口，Phase 3.5 重点攻关 |
| 测试路径失效 | 低 | 高 | 统一使用相对路径，`cargo test` 自动发现 |
| 性能退化（模块边界增加） | 中 | 低 | Rust 内联优化，benchmark 对比验证 |
| 进度超预期 | 中 | 中 | 每个子阶段可独立验证，随时可暂停 |

---

## 六、验收标准

### 6.1 代码质量

- [ ] 所有 300 个测试通过
- [ ] 编译零 warnings（`cargo clippy -- -D warnings`）
- [ ] `lib.rs` < 200 行
- [ ] `src/` 根目录下无 `.rs` 文件（仅保留 `lib.rs`）
- [ ] 每个子目录有 `mod.rs` 导出公共 API

### 6.2 模块设计

- [ ] 零游离文件
- [ ] `core/` 不依赖任何业务模块
- [ ] `query/scan.rs` 不依赖 `FileKV` 主类型
- [ ] 所有跨模块通信通过 trait 或具体类型，无循环依赖
- [ ] 依赖图是 DAG（有向无环图）

### 6.3 并行开发

- [ ] 5 人团队可同时开发不同模块，无冲突
- [ ] 新成员可通过目录结构快速理解架构
- [ ] 每个模块都有独立的测试集

---

## 七、并行开发矩阵（Phase 3+ 完成后）

假设 5 人团队：

| 开发者 | 负责模块 | 依赖 | 可并行 |
|--------|---------|------|--------|
| **A** | core/, io/, error/ | 无 | ✅ 完全独立 |
| **B** | cache/, bloom/ | core 定稿 | ✅ 可与 A 协调后并行 |
| **C** | query/, compression/ | core, zone_map 定稿 | ✅ 可与 A 协调后并行 |
| **D** | compaction/, checkpoint/ | core, query 定稿 | ✅ 可与 A/C 协调后并行 |
| **E** | engine/, ops/ | 所有下层模块 | ⚠️ 需协调，但接口清晰 |

**可并行开发比例**：从 85% 提升到 **95%**（仅 engine/ 需协调）

---

## 八、参考资料

- [模块依赖分析报告](./doc/filekv/MODULE_DEPENDENCY_ANALYSIS.md)（待生成）
- [六阶段重构报告](./doc/filekv/SIX_PHASES_COMPLETION_REPORT_2026_04_11.md)
- [文档/实现差距分析](./todo.json)

---

**文档版本**: v0.3
**最后更新**: 2026-04-13
**作者**: Qwen Code
**状态**: Phase 0-2 已完成，Phase 3+ 方案设计完成，待执行
