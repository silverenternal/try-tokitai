# 性能预算 (Performance Budget)

**版本**: v0.5.0+
**生效日期**: 2026-04-16
**维护者**: P11 Performance Review Team

---

## 什么是性能预算

性能预算是**硬性限制**——任何 PR 的改动都不应突破这些限制。如果突破，必须：
1. 说明原因（为什么这是可接受的）
2. 提供补偿（其他路径的优化足以抵消）
3. 获得性能审查批准

---

## 硬性限制 (Hard Limits)

以下是最坏情况下的上限。任何 PR 不得导致以下操作**超过**这些值。

| 操作 | 预算上限 | 当前基线 | 裕度 | 测试文件 |
|---|---|---|---|---|
| get (hot cache, 64B) | **< 400ns** | 267ns | 50% | 01_basic_ops |
| get (hot cache, 1KB) | **< 400ns** | 267ns | 50% | 01_basic_ops |
| get (hot cache, 4KB) | **< 500ns** | 268ns | 87% | 01_basic_ops |
| get (cold cache, 64B) | **< 800ns** | 412ns | 94% | 01_basic_ops |
| put (no WAL, 64B) | **< 3.0 µs** | 1.17 µs | 156% | 01_basic_ops |
| put (no WAL, 1KB) | **< 6.0 µs** | 2.70 µs | 122% | 01_basic_ops |
| put (no WAL, 4KB) | **< 15 µs** | 6.86 µs | 119% | 01_basic_ops |
| put (WAL, 64B) | **< 5.0 µs** | 1.57 µs | 218% | 01_basic_ops |
| put (WAL, 1KB) | **< 10 µs** | 3.92 µs | 155% | 01_basic_ops |
| put (WAL, 4KB) | **< 25 µs** | 10.91 µs | 129% | 01_basic_ops |
| delete (64B) | **< 500ns** | 135ns | 270% | 01_basic_ops |
| Bloom 负向查询 | **< 15 µs** | 7.23 µs | 107% | 03_bloom_filter |
| 4 线程并发写入 | **< 1.0 ms** | 544 µs | 84% | 04_concurrent_ops |
| 4 线程并发读取 | **< 300 µs** | 135 µs | 122% | 04_concurrent_ops |
| 4 线程混合 (80R20W) | **< 3.0 ms** | 1.57 ms | 91% | 04_concurrent_ops |
| Compaction 触发 | **< 6.0 ms** | 2.95 ms | 103% | 05_range_compaction |

> **裕度** = (预算 - 基线) / 基线 × 100%
> 裕度越低，该操作越接近预算上限，越需要关注。

---

## 放大率限制 (Amplification Limits)

| 指标 | 预算上限 | 当前值 | 说明 |
|---|---|---|---|
| 写放大 (WAF) | < 3.0x | ~1.0-1.5x | 每次逻辑写入对应的物理写入倍数 |
| 读放大 (RAF) | < 5.0x | ~1.0-2.0x | 每次逻辑读取对应的物理 I/O 次数 |
| 空间放大 (SAF) | < 10x | 视场景 | 磁盘使用量 vs 逻辑数据量 |

---

## 尾部延迟限制 (Tail Latency)

| 操作 | p99 预算 | 说明 |
|---|---|---|
| get (hot cache) | < 1.5 µs | 99% 的请求应在 1.5µs 内完成 |
| put (WAL) | < 50 µs | 99% 的写入应在 50µs 内完成 |
| Compaction | < 20 ms | 99% 的 Compaction 应在 20ms 内完成 |

> 注意：当前基准测试仅测量平均值 (mean/median)。p99 数据需通过
> `hdrhistogram` 集成后获取（见 PERF-HISTOGRAM 任务）。

---

## PR 检查流程

### 提交 PR 前

1. 跑回归检测：`./scripts/bench-regression.sh --baseline v0.5.0`
2. 确认 0 regressions > 5%
3. 如果有 3-5% 的回归，在 PR 描述中说明

### PR 审查时

审查者应检查：
- [ ] 改动是否涉及热路径（get/put 调用链）
- [ ] 是否新增热路径调用（函数调用次数 = 操作次数）
- [ ] 是否新增锁竞争（Mutex/RwLock 在 get/put 中）
- [ ] 是否新增内存分配（Vec::new, String::from 在热路径中）
- [ ] 是否新增 syscall（SystemTime::now, fs::metadata 在热路径中）

### 合并前

- 对于 > 5% 回归的 PR：需要性能负责人批准
- 对于 > 15% 回归的 PR：P0，默认阻止合入

---

## 预算更新流程

预算不是永久的。以下情况应更新预算：

1. **基线改善**：某个操作持续稳定地优于当前基线 → 收紧预算
2. **架构变更**：引入了新路径，预算需要重新校准 → 重新测量后更新
3. **文档化例外**：某个预算被证明不切实际 → 讨论后调整

更新预算需要：
- 跑完整基准测试（3 次取平均）
- 更新 PERFORMANCE_BASELINE.md 和 PERFORMANCE_BUDGET.md
- 保存新 baseline：`just save-baseline <version>`
- PR 中标注 "performance budget update"

---

## 已知热路径规则

以下规则**必须遵守**，违反即视为性能 bug：

1. **禁止在 get() 中使用 SystemTime::now()** → 用 Instant（已消除）
2. **禁止在 get() 中分配 Vec/String** → 预分配或 zero-copy
3. **禁止在 put() 中持有多把锁** → 最多一把锁的范围 < 1µs
4. **禁止在热路径中使用 .clone() on large types** → 用 Arc::clone
5. **禁止在热路径中做文件 I/O** → 所有 I/O 必须异步或批量
6. **禁止在 get() 中遍历 segment 列表** → 用 dense_index O(1) 查找
7. **禁止在热路径中做 DashMap get + insert** → 用 get_or_insert_with

---

## 规模等级

性能预算针对以下规模等级生效：

| 等级 | Key 数量 | 预算适用性 |
|---|---|---|
| Tiny (≤100K) | ≤100K | 全部适用 |
| Small (100K~1M) | 100K~1M | 适用 |
| Medium (1M~10M) | 1M~10M | 部分适用（放大率为主） |
| Large (10M~100M) | 10M~100M | 另议 |

> v0.5.0 的性能数据主要在 Tiny 规模测量。Medium+ 规模的预算
> 将在 v0.6.0 中补充。
