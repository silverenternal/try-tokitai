# FileKV 性能基线 (Performance Baseline)

**版本**: v0.5.0
**测试日期**: 2026-04-16 (Rounds 29-38 累计)
**测试环境**: 见下方

---

## 测试环境

| 项目 | 规格 |
|------|------|
| **CPU** | AMD Ryzen 9 8945HS w/ Radeon 780M Graphics |
| **核心数** | 16 线程 |
| **内存** | 64 GiB DDR5 |
| **磁盘** | NVMe SSD (831G, 703G 可用) |
| **OS** | Linux 6.19.11-zen1-1-zen (Arch Linux) |
| **Rust** | stable (cargo bench, release profile) |
| **Feature flags** | `--features benchmarks` |

---

## 核心性能指标 (v0.5.0 实测)

### 单点写入

| 操作 | Value 大小 | 延迟 | QPS (推算) | 基准文件 |
|------|-----------|------|-----------|---------|
| put (no WAL) | 64B | 1.17 µs | 854K | 01_basic_ops |
| put (no WAL) | 1KB | 2.70 µs | 370K | 01_basic_ops |
| put (no WAL) | 4KB | 6.86 µs | 146K | 01_basic_ops |
| put (WAL) | 64B | 1.57 µs | 637K | 01_basic_ops |
| put (WAL) | 1KB | 3.92 µs | 255K | 01_basic_ops |
| put (WAL) | 4KB | 10.91 µs | 92K | 01_basic_ops |

### 单点读取

| 操作 | Value 大小 | 延迟 | QPS (推算) | 基准文件 |
|------|-----------|------|-----------|---------|
| get (hot cache) | 64B | **278-285 ns** | **3.50-3.60M** | 01_basic_ops |
| get (hot cache) | 1KB | **278-281 ns** | **3.56M** | 01_basic_ops |
| get (hot cache) | 4KB | **277-278 ns** | **3.60M** | 01_basic_ops |
| get (cold cache) | 64B | **417-435 ns** | **2.30-2.40M** | 01_basic_ops + 02_cache_performance |
| delete (write+delete 全周期) | 64B | **1.18-1.20 µs** | **832-851K** | 01_basic_ops |

> **注意**: `read_hot_cache` (01_basic_ops, ~278-285ns) 测量的是 put 后 BlockCache 已有数据的 get() 路径。
> `cache_hit/hot_cache_get_64B` (02_cache_performance, ~387-393ns) 测量的是 warm_cache() 显式预热后的 get() 路径。
> 两者差异源于 benchmark setup 方式不同，**278-285ns 是 DenseIndex 快速路径测量**。

### 批量操作

| 操作 | 规模 | 延迟 | QPS | 基准文件 | 备注 |
|------|------|------|-----|---------|------|
| put_batch (真实 API) | 100 keys | 38-42 µs | 2.39-2.64M | 01_basic_ops | Round 38: 改用 put_batch() API |

> **Round 38 变更**: 此前 batch_write 使用循环 `put()` 测量 (117-119 µs)，
> 现改为 `put_batch()` API，性能提升 ~3x。旧数据不代表真实批处理性能。

### 并发操作 (4 线程)

| 操作 | 延迟 | 吞吐 | 基准文件 | 变化趋势 |
|------|------|------|---------|---------|
| 4 线程并发写入 | **532-548 µs** | **182-188K ops/sec** | 04_concurrent_ops | 稳定 |
| 4 线程并发读取 | **135-137 µs** | **731-738K ops/sec** | 04_concurrent_ops | 稳定 |
| 4 线程混合 (80R20W) | **1.57-1.58 ms** | **63.2-63.7K ops/sec** | 04_concurrent_ops | 稳定 |

> **Round 38 变更**: 并发 benchmark 改为 `Instant` 测量真实并发时间 (排除 thread spawn/join 开销)。
> 此前测量值包含线程创建/销毁开销，无法反映真实并发性能。

### 范围扫描

| 操作 | 规模 | 延迟 | 吞吐 | 基准文件 |
|------|------|------|------|---------|
| Range Scan | 10 keys | **3.94 µs** | 2.54M ops/sec | 05_range_compaction |
| Range Scan | 50 keys | **20.5 µs** | 2.44M ops/sec | 05_range_compaction |
| Range Scan | 100 keys | **42.5-42.9 µs** | 2.33-2.35M ops/sec | 05_range_compaction |
| Compaction 触发 | 2000 keys | **5.31-5.37 ms** | - | 05_range_compaction |
| 写放大 (100 entries) | 100 entries | **126-131 µs** | 761-794K ops/sec | 05_range_compaction |

> **Round 38 变更**: `trigger_compaction` 改为调用 `run_compaction()` 实际执行 compaction。
> 此前只读取 stats 未真正触发 compaction。新值 5.34ms 反映真实 compaction 耗时。

### BlockCache 性能

| 操作 | 延迟 | 吞吐 | 基准文件 |
|------|------|------|---------|
| BlockCache Get (8 分片) | 174-176 ns | 5.68-5.73M ops/sec | block_cache_get_by_key |
| 16 线程并发 Get | 205-206 µs | 4.86-4.87K ops/sec | block_cache_get_by_key |

### Bloom Filter 性能

| 操作 | 延迟 | 对比 RocksDB | 基准文件 |
|------|------|-------------|---------|
| 负向查询 (negative lookup) | **7.23 µs** | **34.2x 更快** | 03_bloom_filter |
| 多段负向查询 | **10.46-10.59 µs** | **23.6x 更快** | 03_bloom_filter |
| 正向查询 (cold cache) | **410-423 ns** | - | 03_bloom_filter |
| 构建 + 查询 (CustomBloom) | 见详细报告 | - | 03_bloom_filter |

### 大规模写入 (含放大率)

| Key 数量 | Value 大小 | 耗时 | QPS | WA | SA | 基准文件 |
|----------|-----------|------|-----|----|----|---------|
| 100K | 64B | 149ms | 670K | 1.0 | 568x | 09_10m |
| 100K | 256B | 150ms | 664K | 1.0 | 162x | 09_10m |
| 100K | 1KB | 198ms | 503K | 1.0 | 43x | 09_10m |
| 100K | 4KB | 300ms | 333K | 1.0 | 11x | 09_10m |

> **SA 说明**: 空间放大率 (SA) 高是因为 100K keys 写入固定大小的 segment 文件，小 value 场景下
> segment 元数据 (header, index, zone map) 占比较大。随数据量增大，SA 会趋近于正常值 (2-5x)。

### 压缩性能

| 操作 | 数据大小 | 算法 | 延迟 | 吞吐 | 基准文件 |
|------|---------|------|------|------|---------|
| compress | 100B | zstd (level 3) | ~390 ns | 244 MB/s | 08_compression |
| compress | 100B | snappy | ~158 ns | 605 MB/s | 08_compression |
| compress | 100B | lz4 (level 0) | ~131 ns | 729 MB/s | 08_compression |
| compress | 100KB JSON | zstd (level 3) | ~12.2 µs | 7.78 GB/s | 08_compression |
| compress | 100KB JSON | snappy | ~105 µs | 907 MB/s | 08_compression |
| compress | 100KB JSON | lz4 (level 0) | ~6.1 µs | 15.7 GB/s | 08_compression |
| decompress | 100KB JSON | zstd (89B→100KB) | ~8.87 µs | 9.56 MiB/s | 08_compression |
| decompress | 100KB JSON | snappy (4956B→100KB) | ~6.54 µs | 722 MiB/s | 08_compression |
| decompress | 100KB JSON | lz4 (496B→100KB) | ~6.42 µs | 73.7 MiB/s | 08_compression |

> **Round 38 变更**: compression_ratio benchmark 改为测量实际 compress/decompress 操作，
> 此前测量的是 `format!()` 字符串拼接性能。

---

## 版本间性能对比

| 指标 | v0.4.0 (历史基线) | v0.5.0 Round 38 (当前) | 变化 |
|------|-------------------|------------------------|------|
| 热缓存读取 (64B) | ~230 ns (POL-004) | **278-285 ns** | +21-24% |
| 冷缓存读取 (64B) | ~371 ns (CHANGELOG) | **417-435 ns** | +12-17% |
| 写入 WAL 64B | ~1.92 µs (CHANGELOG) | **1.57 µs** | -18% (提升) |
| 写入 no-WAL 64B | ~2.05 µs (CHANGELOG) | **1.17 µs** | -43% (提升) |
| 删除操作 (全周期) | **135 ns** (仅 delete) | **1.18-1.20 µs** (write+delete) | 测量方式变更 |
| 批量写入 100 | 117-119 µs (循环 put) | **38-42 µs** (put_batch) | ~3x 提升 |
| 4 线程并发写入 | ~440 µs (推算) | **532-548 µs** | +21-25% |
| 4 线程并发读取 | ~150 µs (推算) | **135-137 µs** | -9% (提升) |
| 4 线程混合 (80R20W) | ~1.7 ms (推算) | **1.57-1.58 ms** | -7% (提升) |
| Compaction 触发 | ~3.2 ms (推算) | **5.31-5.37 ms** | +67% (测量修正) |
| 10M 顺序写入 | ~360K ops/sec | **355K ops/sec** | -1.4% |
| 批量写入 100K | ~190 ms (推算) | **147 ms** | -23% (提升) |
| 批量写入 1M | ~2.95 s (推算) | **2.23 s** | -24% (提升) |

> **变化说明**:
> - 写入性能提升源于 WAL channel 重构 (OPT-007) 和零拷贝优化
> - 大批量写入显著提升：100K 提升 29.6%，1M 提升 24.4%（Rounds 31-32 写入路径优化）
> - 读取轻微回归源于新增的精确 I/O 计数、AmplificationTracker 原子操作、MemoryTracker 开销
> - 混合并发和 Compaction 性能提升源于 Round 29-33 优化
> - **Round 38 Benchmark 逻辑修复**: delete 改为 write+delete 全周期测量，batch_write 改用 put_batch() API，
>   trigger_compaction 改为实际执行 run_compaction()，并发 benchmark 排除 thread spawn/join 开销，
>   compression_ratio 测量实际压缩操作而非 format!()

---

## 关键路径分析

### get() 热路径调用链 (BlockCache hit)

```
FileKV.get(key)
├── read_engine.get(key)
│   ├── global_index.get(key)          # moka query_cache, Arc::from 仅在 miss
│   ├── segments_snapshot.load()        # ArcSwap::load(), 零拷贝
│   ├── search_segment()               # 核心路径
│   │   ├── dense_index.get_by_key()   # AHashMap O(1)
│   │   ├── amplification_tracker      # AtomicU64 fetch_add
│   │   ├── decompressor check          # Arc<DictionaryCompressor>, 无 Mutex
│   │   ├── block_cache.insert()        # 缓存回填
│   │   └── sequential_detector.record() # 访问模式跟踪
│   └── (if dense miss) bloom + zone map 路径
```

### put() 热路径调用链

```
FileKV.put(key, value)
├── write_engine.put(key, value)
│   ├── wal_channel.put_buffered()     # WAL 提交
│   │   └── log_batch.submit()          # 批量 WAL 写入
│   └── check_flush_and_compaction()    # 后台检查
```

---

## 与 RocksDB 公平对比

| 场景 | tokitai-filekv | RocksDB | 差距 | 备注 |
|------|---------------|---------|------|------|
| 10M 顺序写入 | 355K ops/sec | 500K-1M ops/sec | 1.4-2.8x | RocksDB 专职团队 15+ 年优化 |
| 热缓存读取 | 278-393 ns | 600.07 µs (公平对比测试) | **1521-2158x 更快** | DenseIndex + BlockCache |
| 冷缓存读取 | 417-435 ns | 视场景 | 相当 | mmap 零拷贝 |
| Bloom 负向查询 | 7.23 µs | 247.38 µs | **34.2x 更快** | 三层自适应缓存 |

> 数据来源: benches/rocksdb_fair_comparison.rs (2026-04-08)
> RocksDB 测试使用默认配置，未针对特定场景调优

---

## 测试来源映射

| 基准文件 | 覆盖场景 | 测试日期 |
|---------|---------|---------|
| benches/01_basic_ops.rs | 单点读写、删除、批量 | 2026-04-16 |
| benches/02_cache_performance.rs | 热/冷缓存、缓存命中/未命中 | 2026-04-16 |
| benches/03_bloom_filter.rs | Bloom Filter 构建和查询 | 2026-04-16 |
| benches/04_concurrent_ops.rs | 4 线程并发读写 | 2026-04-16 |
| benches/05_range_compaction.rs | 范围扫描、Compaction | 2026-04-16 |
| benches/07_professional_benchmark.rs | 10M 大规模写入 + 放大率 | 2026-04-16 |
| benches/08_compression_bench.rs | 压缩/解压性能 | 2026-04-16 |
| benches/09_10m_benchmark.rs | 不同 Value 大小对比 | 2026-04-15 |
| benches/block_cache_get_by_key.rs | BlockCache 性能 | 2026-04-16 |
| benches/rocksdb_fair_comparison.rs | RocksDB 公平对比 | 2026-04-08 |

---

## 规模分类 (对齐工业界标准)

| 规模等级 | Key 数量 | 数据量 | 适用场景 |
|---------|----------|--------|---------|
| Tiny (极小) | ≤100K | ≤100MB | 功能正确性、单元测试 |
| Small (小) | 100K~1M | 100MB~1GB | 基础性能验证 |
| Medium (中) | 1M~10M | 1GB~10GB | 核心性能、放大率 |
| Large (大) | 10M~100M | 10GB~100GB | 极限性能 |
| XLarge (超大) | ≥100M | ≥100GB | 长期稳定性 |

> **重要**: v0.5.0 的性能数据大部分在 Tiny/Small 规模测量。
> Medium+ 规模的极限性能仍在优化中 (见 v0.6.0 规划)。

---

*本文档由 Round 38 基准测试更新 (2026-04-16)。*
*基准测试命令: `cargo bench --bench <name> --features benchmarks -- --noplot`*
