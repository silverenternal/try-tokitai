# Bloom Filter 优化创新

> **状态**: ✅ 已实现  
> **引入版本**: v0.4.0 - v0.8.0 (多轮迭代)  
> **核心代码**: `src/bloom/`

---

## 概述

Bloom Filter 是 tokitai-filekv **最具原创性**的优化模块,实现了 7 项创新,负向查询性能**超越 RocksDB 34.2x**。

---

## 1. Custom Bloom V3 (确定性哈希)

### 问题
`::bloom` crate 使用 `RandomState` 做哈希,导致 bitset 无法序列化到磁盘,每次重启都要重建。

### 创新方案
使用 XXH3 确定性哈希算法,支持 Bloom Filter 直接持久化和快速加载。

### 实现细节
- **文件**: `src/bloom/custom_bloom.rs`
- **V3 文件格式**: `[magic 4B][version 4B][num_bits 4B][num_hashes 4B][bitset_bytes]`
- **双哈希模拟 k 个函数**: XXH3(seed=0) + XXH3(seed=0xDEADBEEF)
- **加载时间**: < 100μs (直接加载 bitset,无需重建)

### 性能影响
- 重启恢复时间从 O(n_keys) 降低到 O(1)
- contains (负例) 性能 < 1μs

### 相关测试
- `src/bloom/custom_bloom.rs` 内置测试
- `benches/custom_bloom_perf.rs` 性能基准

---

## 2. L1/L2/L3 多层自适应 Bloom 缓存

### 问题
所有 Segment 的 Bloom Filter 常驻内存,10M keys 场景下内存占用巨大。

### 创新方案
三级自适应缓存,根据访问频率动态调整 Bloom Filter 的存储位置。

### 实现细节
- **文件**: `src/bloom/adaptive.rs`
- **L1 (Hot)**: DashMap, ~1000 filters, FPR 0.1%, 访问延迟 <100ns
- **L2 (Warm)**: 压缩存储, ~10000 filters, FPR 1%, 访问延迟 ~500ns
- **L3 (Cold)**: 磁盘按需加载, FPR 10%, 访问延迟 ~10μs
- **CLOCK 算法淘汰**: 16 shards 减少锁竞争,近似 LRU

### 性能影响
- 热数据 FPR 从 1% 降低到 0.1%,误报率降低 10x
- 冷数据内存减少 75%

### 相关测试
- `src/bloom/adaptive.rs` 内置测试
- `benches/adaptive_bloom_bench.rs` 性能基准

---

## 3. FPR 自适应控制器

### 问题
固定 FPR (False Positive Rate) 对冷热数据不均衡:热数据需要低 FPR 保证精度,冷数据可以接受高 FPR 节省内存。

### 创新方案
基于 QPS 动态调整每个 Segment 的 Bloom Filter FPR。

### 实现细节
- **文件**: `src/bloom/fpr_controller.rs`
- **6 级 FPR**: Level 0 (0.1%) → Level 5 (10%)
- **动态调整**: 热 segment 用低 FPR,冷 segment 用高 FPR
- **迟滞机制**: 20% hysteresis 防止振荡
- **稳定窗口**: 2 分钟防止频繁调整
- **逐级过渡**: 跳过中间级别直接升降

### 性能影响
- 热 segment 内存增加 2x 但精度提升 10x
- 冷数据内存减少 50%

### 相关测试
- `src/bloom/fpr_controller.rs` 内置测试 (10+ tests)

---

## 4. Bloom Filter 压缩

### 问题
L2 缓存中 Bloom Filter 占用内存大,尤其大 FPR 场景。

### 创新方案
RLE (Run-Length Encoding) + Huffman 编码压缩 Bloom bitset。

### 实现细节
- **文件**: `src/bloom/compressed.rs`
- **RLE**: 连续 0/1 序列压缩
- **Huffman**: 基于频率的可变长度编码
- **压缩比**: 2-5x (取决于 Bloom 稀疏度)
- **解压延迟**: < 500ns

### 性能影响
- L2 缓存容量从 1000 filters 提升到 10000 filters
- 解压开销可接受 (<500ns)

### 相关测试
- `src/bloom/compressed.rs` 内置测试

---

## 5. 缓存迁移 (频率感知)

### 问题
数据冷热变化后,Bloom Filter 需要跨层迁移,但传统 LRU 只考虑最近访问时间。

### 创新方案
结合 QPS 和访问频率的混合评分系统,自动升降段到合适的缓存层。

### 实现细节
- **文件**: `src/bloom/migration.rs`
- **迁移规则**:
  - L3→L2: QPS > 10 (warm_threshold)
  - L2→L1: QPS > 100 (hot_threshold)
  - L1→L2 (cooldown): QPS < 5
  - L2→L3 (eviction): QPS < 1
- **混合评分**: 70% QPS + 30% access_count
- **迟滞窗口**: 升级 60s,降级 300s

### 性能影响
- 热数据自动升温,命中率提升 15%+
- 冷数据自动降温,内存释放 30%+

### 相关测试
- `src/bloom/migration.rs` 内置测试

---

## 6. V1/V2→V3 自动版本迁移

### 问题
旧版 Bloom Filter 格式 (V1/V2) 与新 V3 格式不兼容,需要手动迁移。

### 创新方案
加载时自动检测格式版本,透明迁移到 V3。

### 实现细节
- **文件**: `src/bloom/manager.rs`
- **方法**: `load_custom_bloom_with_migration()`
- **向后兼容**: V1/V2 格式仍然可读取
- **自动升级**: 保存时统一用 V3 格式

### 性能影响
- 零手动操作,升级无感知
- 旧格式加载后自动缓存为 V3,后续访问加速

### 相关测试
- `src/bloom/manager.rs` 内置测试

---

## 7. CLOCK 分片淘汰算法

### 问题
LRU 需要维护访问顺序链表,高并发下锁竞争严重。

### 创新方案
CLOCK 算法做近似 LRU,无需维护顺序,分片设计进一步减少锁竞争。

### 实现细节
- **文件**: `src/bloom/filter_cache.rs`
- **CLOCK 算法**: 循环扫描,reference bit=1 保留并清零,=0 淘汰
- **分片设计**: 16 shards,一致性哈希路由
- **容量限制**: 最大 1000 filters / 256MB

### 性能影响
- 并发读取提升 7.4x (vs 单锁 LRU)
- 近似 LRU 命中率与真 LRU 差距 <5%

### 相关测试
- `src/bloom/filter_cache.rs` 内置测试

---

## 📊 性能成果汇总

| 指标 | 优化前 | 优化后 | 提升 |
|------|--------|--------|------|
| Bloom 负向查询延迟 | 62.37 μs (v0.4.0 前) | **7.23 μs** | **8.6x** |
| vs RocksDB | 247.38 μs | 7.23 μs | **34.2x 快** |
| 热数据 FPR | 1% | **0.1%** | **10x 精度提升** |
| 冷数据内存 | 全量 | **减少 75%** | **4x 节省** |
| 重启恢复时间 | O(n_keys) | **<100μs** | **O(1)** |
| 并发读取 | 单锁瓶颈 | **7.4x 提升** | **CLOCK 分片** |

---

## 🔗 相关文档

- [自适应 Bloom 架构设计](../filekv/ADAPTIVE_BLOOM_DESIGN.md) (如存在)
- [性能基线](../filekv/PERFORMANCE_BASELINE.md)
- [RocksDB 公平对比](../rocksdb_fair_comparison_2026_04_08.md)
