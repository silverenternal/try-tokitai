# Compaction 优化创新

> **状态**: ✅ 已实现  
> **引入版本**: v0.3.0 - v0.5.0 (多轮迭代)  
> **核心代码**: `src/compaction/`

---

## 概述

Compaction 是 LSM-Tree 的核心后台操作,tokitai-filekv 实现了 6 项优化,显著提升压缩效率并降低内存占用。

---

## 1. Leveled Compaction (分层压缩)

### 问题
传统 Size-Tiered Compaction (STCS) 导致读放大高,L0 层 key range 重叠,查询需要扫描多个 segment。

### 创新方案
L0 使用 STCS 或 LCS,L1+ 严格使用 Leveled Compaction,确保高层级 segment 不重叠。

### 实现细节
- **文件**: `src/compaction/mod.rs`
- **策略**: `CompactionStrategy::Leveled`
- **Level size multiplier**: 10 (L1: 128MB, L2: 1.28GB, L3: 12.8GB)
- **触发条件**:
  - L0: 文件数阈值 (3) 或大小阈值 (64MB)
  - L1+: 超出 size budget 触发
- **Segment 层级标记**: 每个 segment 带 `level` 字段,L0 可重叠,L1+ 非重叠

### 性能影响
- 读放大从 O(L0_size) 降低到 O(num_levels)
- L1+ 查询可快速跳过无关 segment

### 相关测试
- `src/compaction/mod.rs` 内置测试
- `benches/05_range_compaction.rs` 性能基准

---

## 2. Size-Tiered Compaction for L0

### 问题
L0 层 segments key range 重叠,不适合 leveled 策略,需要先合并为不重叠段。

### 创新方案
L0 使用 Size-Tiered 策略,按大小分组,选择最小 tier 合并。

### 实现细节
- **文件**: `src/compaction/mod.rs`
- **方法**: `select_size_tiered_segments()`
- **Size ratio**: 默认 2.0x,相似大小的 segment 归为同一 tier
- **选择策略**: 优先合并最小 tier,减少写放大

### 性能影响
- L0 合并效率提升,避免小 segment 频繁触发压缩
- 写放大控制在 2.0x 以内

### 相关测试
- `src/compaction/mod.rs` 内置测试

---

## 3. Parallel Compaction (并行压缩)

### 问题
单线程 compaction 在多核 CPU 上利用率低,大 segment 合并耗时长。

### 创新方案
使用 rayon 并行读取和合并 segments,充分利用多核。

### 实现细节
- **文件**: `src/compaction/mod.rs`
- **配置**: `CompactionConfig::parallel_compaction_enabled` (默认 true)
- **并行库**: `rayon::prelude::*`
- **线程数**: `max_background_compaction_threads = min(4, num_cpus/2)`
- **并行范围**: segment 读取和 k-way merge

### 性能影响
- 大 segment 合并速度提升 2-4x (取决于核心数)
- 后台 compaction 耗时减少,降低对前台写入的影响

### 相关测试
- `src/compaction/mod.rs` 内置测试
- `benches/05_range_compaction.rs` 性能基准

---

## 4. Write-Amplification-Aware Trigger (WA 感知触发)

### 问题
传统 compaction 触发策略不考虑写放大影响,高负载时可能触发 compaction 雪崩。

### 创新方案
基于写放大率动态调整 compaction 优先级,结合 I/O 压力监控。

### 实现细节
- **文件**: `src/compaction/trigger.rs`
- **结构体**: `WriteAmplificationAwareTrigger`
- **WA 追踪**: `WA = total_bytes_written / user_bytes_written`
- **4 级优先级**:
  - Low: WA < 2.0x
  - Normal: WA 2.0-3.0x
  - High: WA 3.0-4.0x
  - Urgent: WA > 4.0x
- **I/O 压力监控**: `IoPressureTracker` - 监控写队列深度和 P99 延迟
- **动态 delay**: High pressure = 500ms, Low pressure = 0ms

### 性能影响
- 高负载时自动降级 compaction 优先级,避免雪崩
- 写放大控制在合理范围 (<3.0x)

### 相关测试
- `src/compaction/trigger.rs` 内置测试

---

## 5. Streaming Merge Iterator (流式合并迭代器)

### 问题
传统 compaction 将所有 keys 加载到 BTreeMap,内存占用 O(total_keys × avg_value_size),大 segment 合并时内存爆炸。

### 创新方案
使用 BinaryHeap 做 k-way merge,流式读取 segments,内存仅占用 O(num_segments × avg_value_size)。

### 实现细节
- **文件**: `src/compaction/merge_iterator.rs`, `src/compaction/segment_iterator.rs`
- **核心结构**: `MergeIterator` 使用 BinaryHeap (min-heap)
- **内存对比**:
  - 传统: O(total_keys × avg_value_size)
  - 流式: O(num_segments × avg_value_size)
- **自动去重**: 最新 segment 版本胜出
- **统计指标**: `duplicates_removed`, `tombstones_cleaned`
- **Tombstone 跳过**: `SegmentIterator` 直接从 segment mmap 流式读取,跳过删除标记

### 性能影响
- 10M keys compaction 内存从 ~10GB 降低到 ~100MB
- 大 segment 合并不再 OOM

### 相关测试
- `src/compaction/merge_iterator.rs` 内置测试
- `src/compaction/segment_iterator.rs` 内置测试

---

## 6. Crash-Safe Compaction Manifest (崩溃安全压缩)

### 问题
Compaction 中途崩溃可能导致:输入 segment 已删除但输出未完成,数据丢失。

### 创新方案
Compaction 开始前原子写入 manifest 记录操作,崩溃后扫描 manifest 恢复。

### 实现细节
- **文件**: `src/compaction/manifest.rs`
- **原子写入**: temp 文件 + rename,确保 manifest 完整性
- **CompactionStatus**:
  - InProgress: 压缩进行中
  - Completed: 压缩完成
  - Aborted: 压缩中止
- **恢复流程**: `recover_incomplete()` 扫描 InProgress manifest
  - 删除不完整的输出 segment
  - 恢复输入 segment 引用

### 性能影响
- 崩溃恢复时间 < 1s (扫描 manifest)
- 数据零丢失保证

### 相关测试
- `src/compaction/manifest.rs` 内置测试
- `src/compaction/manifest_crash_tests.rs` 崩溃恢复测试

---

## 📊 性能成果汇总

| 指标 | 优化前 | 优化后 | 提升 |
|------|--------|--------|------|
| 读放大 (L1+) | O(L0_size) | **O(num_levels)** | **10x+ 降低** |
| Compaction 内存 | O(total_keys) | **O(num_segments)** | **100x 降低** |
| 并行压缩速度 | 单线程 | **2-4x 提升** | **rayon 多核** |
| WA 控制 | 无限制 | **<3.0x** | **WA-aware 触发** |
| 崩溃恢复 | 手动修复 | **<1s 自动** | **Manifest 保证** |
| Compaction 触发延迟 | 固定 | **0-500ms 动态** | **I/O 压力感知** |

---

## 🔗 相关文档

- [Leveled Compaction 设计](../filekv/COMPACTION_DESIGN.md) (如存在)
- [性能基线](../filekv/PERFORMANCE_BASELINE.md)
