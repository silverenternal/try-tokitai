# 性能回归调试 Runbook (Performance Regression Debug Runbook)

## 快速开始

当 benchmark 出现回归时，按以下步骤快速定位根因：

```
1. 确认回归  →  2. 定位模块  →  3. 找到 commit  →  4. 修复验证
```

---

## 步骤 1: 确认回归

### 1.1 跑回归检测

```bash
# 对比最新 baseline
./scripts/bench-regression.sh

# 对比特定 baseline
./scripts/bench-regression.sh --baseline v0.5.0 --threshold 3
```

**判定标准**:
- `< 3%`: 测量噪声，忽略
- `3-5%`: 轻微回归，记录到 CHANGELOG
- `> 5%`: 需要调查
- `> 15%`: P0，阻止合入

### 1.2 排除噪声

```bash
# 跑 3 次取平均，排除一次性噪声
for i in 1 2 3; do
    cargo bench --features benchmarks -- read_hot_cache --noplot
done
```

**如果 3 次结果波动 > 5%** → 环境噪声，在更稳定的环境重跑。

---

## 步骤 2: 定位模块

根据回归的 benchmark 和操作类型，定位到具体模块：

| 回归操作 | 最可能模块 | 检查路径 |
|---|---|---|
| `get (hot cache)` 变慢 | DenseIndex / BlockCache | `src/engine/read_engine.rs` |
| `get (cold cache)` 变慢 | Segment I/O / mmap | `src/core/segment.rs` |
| `put (no WAL)` 变慢 | MemTable / WAL channel | `src/core/memtable.rs`, `src/core/wal_channel.rs` |
| `put (WAL)` 变慢 | WAL sync / batcher | `src/core/wal.rs`, `src/core/wal_batcher.rs` |
| Bloom 负向查询变慢 | Bloom manager / FPR | `src/bloom/manager.rs`, `src/bloom/fpr_controller.rs` |
| 并发写入变慢 | DashMap / lock contention | `src/core/memtable.rs` (DashMap) |
| Compaction 变慢 | Compaction manager | `src/compaction/mod.rs` |
| Range scan 变慢 | Scan / pruner | `src/query/scan.rs`, `src/query/pruner.rs` |

### 2.1 使用 Per-Module Timing (如果已启用)

```bash
# 启用 metrics feature 跑 benchmark
cargo bench --features "benchmarks,metrics" -- read_hot_cache
```

查看 `FileKVMetrics` 输出中的 per-module breakdown:
- `bloom_lookup_time` → Bloom 查找耗时
- `cache_lookup_time` → BlockCache 查找耗时
- `disk_io_time` → 磁盘 I/O 耗时
- `decompress_time` → 解压耗时

### 2.2 手动二分法定位

如果 Per-Module Timing 不可用：

```bash
# 1. git log 找出最近改动相关模块的 commit
git log --oneline -20 -- src/engine/read_engine.rs

# 2. git bisect 找出引入回归的 commit
git bisect start
git bisect bad HEAD           # 当前版本慢
git bisect good v0.5.0        # 旧版本快
# 每步跑: cargo bench -- read_hot_cache
git bisect reset
```

---

## 步骤 3: 找到根因

### 3.1 常见根因分类

| 根因类型 | 症状 | 修复方向 |
|---|---|---|
| **新增热路径调用** | 某个函数调用次数 = 操作次数 | 门控、lazy init、移除 |
| **新增锁竞争** | 并发性能下降 > 串行 | 改 Arc、拆锁、lock-free |
| **新增内存分配** | GC 压力、cache miss | 预分配、对象池、zero-copy |
| **新增 syscall** | strace 中出现新调用 | 缓存结果、用 Instant 替代 |
| **缓存失效** | 缓存命中率下降 | 检查 eviction、admission policy |
| **算法复杂度退化** | 大数据集显著变慢 | 改哈希表、二分查找 |

### 3.2 快速诊断命令

```bash
# 查看热路径中的 syscall
sudo perf record -e syscalls:sys_enter_* -g -- cargo bench -- read_hot_cache
sudo perf report

# 查看锁竞争
cargo bench --features "benchmarks,metrics" -- concurrent_write
# 观察 WAF/RAF 变化

# 火焰图
cargo install flamegraph
sudo flamegraph -- cargo bench -- read_hot_cache
```

---

## 步骤 4: 修复验证

### 4.1 修复后

```bash
# 1. 编译检查
just precommit

# 2. 测试通过
just test

# 3. 跑回归检测确认修复
./scripts/bench-regression.sh --baseline v0.5.0

# 4. 保存新 baseline
./scripts/save-baseline.sh post-<fix-name>
```

### 4.2 记录到 CHANGELOG

```markdown
## 性能修复 - YYYY-MM-DD

- **回归**: `<操作>` 从 Xns 退化到 Yns (+Z%)
- **根因**: <描述>
- **修复**: <描述>
- **结果**: `<操作>` 回到 X'ns (-W% vs 退化后)
```

---

## 附录 A: 性能预算 (Performance Budget)

以下是最坏情况下的硬性限制，任何 PR 不应突破：

| 操作 | 预算 | 当前基线 | 裕度 |
|---|---|---|---|
| get (hot cache, 64B) | < 400ns | 267ns | 50% |
| get (cold cache, 64B) | < 800ns | 412ns | 94% |
| put (no WAL, 64B) | < 3µs | 1.17µs | 156% |
| put (WAL, 64B) | < 5µs | 1.57µs | 218% |
| delete (64B) | < 500ns | 135ns | 270% |
| Bloom 负向查询 | < 15µs | 7.23µs | 107% |
| 4 线程并发写入 | < 1ms | 544µs | 84% |

---

## 附录 B: Benchmark 文件索引

| Benchmark 文件 | 测什么 | 关键指标 |
|---|---|---|
| `01_basic_ops.rs` | 单点读写删批 | hot/cold cache latency, put latency |
| `02_cache_performance.rs` | 缓存性能 | hit rate, warmup effectiveness |
| `03_bloom_filter.rs` | Bloom 查找 | negative lookup latency, FPR |
| `04_concurrent_ops.rs` | 并发性能 | throughput, lock contention |
| `05_range_compaction.rs` | 范围扫描 + Compaction | scan latency, WA measurement |
| `06_large_dataset_bench.rs` | 大数据集 | scalability |
| `07_professional_benchmark.rs` | 10M keys 专业测试 | WA/SA/tail latency |
| `08_compression_bench.rs` | 压缩性能 | compression ratio, throughput |
| `09_10m_benchmark.rs` | 10M keys 规模测试 | throughput at scale |
| `block_cache_get_by_key.rs` | BlockCache 性能 | sharded cache latency |
| `custom_bloom_perf.rs` | CustomBloom 性能 | build + query latency |

---

## 附录 C: 已知性能陷阱

1. **SystemTime::now()** syscall 在热路径中 → 已替换为 Instant (Rounds 31+33)
2. **Arc::clone()** 过多 → 影响 cache admission (已优化)
3. **DashMap** 全局锁 → 并发写入瓶颈 (待优化)
4. **AtomicUsize store** 冗余 → 每次写入 2 个原子操作 (Round 32 已优化)
5. **Bloom migration access recording** 在热路径中 → 已门控 (Round 31)
