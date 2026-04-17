# 查询优化创新深度调研

> 本文档详细分析 tokitai-filekv 的查询优化创新,包含 Zone Map 剪枝、合并迭代器、范围查询、多层缓存和短路优化。

---

## 目录

- [1. 查询优化总览](#1-查询优化总览)
- [2. Zone Map 块级剪枝](#2-zone-map-块级剪枝)
- [3. MergeIterator K 路合并](#3-mergeiterator-k-路合并)
- [4. RangeScanIterator 范围查询](#4-rangescaniterator-范围查询)
- [5. 多层查询缓存架构](#5-多层查询缓存架构)
- [6. 短路和早停优化](#6-短路和早停优化)
- [7. Cache Warmer 预热策略](#7-cache-warmer-预热策略)
- [8. 自适应预取](#8-自适应预取)
- [9. 性能测试数据](#9-性能测试数据)
- [10. 关键文件索引](#10-关键文件索引)

---

## 1. 查询优化总览

### 1.1 查询路径

tokitai-filekv 的查询路径经过多层优化:

```
get(key):
  ├── 1. MemTable (内存最新数据)
  ├── 2. Prefetch Cache (SequentialPrefetcher)
  ├── 3. Block Cache (O(1) DashMap)
  ├── 4. Global Key Index (O(log n) + Query Cache 500K)
  ├── 5. Bloom Filter (99% 负向准确率)
  ├── 6. Zone Map (块级剪枝 40-60% I/O 减少)
  └── 7. Segment 直接读取
```

### 1.2 核心优化点

| 优化 | 效果 | 性能提升 |
|------|------|---------|
| Zone Map 剪枝 | 减少扫描块数 | 40-60% I/O 减少 |
| Dense Index 快速路径 | 跳过 Bloom/Zone Map | 20%+ 延迟降低 |
| Bloom Filter | 负向查询快速排除 | 7.23µs (RocksDB 34.2x 快) |
| 顺序预取 | 范围查询批量加载 | 2-4x 吞吐量提升 |
| 多层缓存 | 减少磁盘 I/O | 85% 命中率 |

---

## 2. Zone Map 块级剪枝

### 2.1 数据结构

**文件**: `src/query/zone_map.rs`

```rust
pub struct ZoneMapEntry {
    pub block_id: u64,
    pub min_key: String,   // 块内最小键 (包含)
    pub max_key: String,   // 块内最大键 (包含)
    pub offset: u64,       // 块在段文件中的起始偏移
    pub size_bytes: u32,   // 块大小 (字节)
    pub entry_count: u32,  // 块内条目数
}
```

### 2.2 剪枝算法

**核心逻辑** (`ZoneMapEntry::overlaps`):
```rust
pub fn overlaps(&self, query_start: &str, query_end: &str) -> bool {
    query_start <= self.max_key.as_str() && query_end >= self.min_key.as_str()
}
```

**查找重叠块** (`ZoneMapIndex::find_overlapping_blocks`):
- 条目按 `min_key` 排序存储
- 使用 `partition_point` 进行**二分查找**,找到 `min_key > query_end` 的边界
- 仅检查候选条目

**时间复杂度**: `O(log n) + O(k)`,其中 `k` 是候选条目数

### 2.3 剪枝效果

根据测试数据 (`src/tests/range_query.rs`):

| 查询范围 | 选择率 | 剪枝效果 |
|----------|--------|----------|
| key_100..key_110 (11键) | 1.1% | 显著减少扫描块数 |
| key_200..key_400 (201键) | 20.1% | 中等剪枝效果 |
| key_000..key_999 (1000键) | 100% | 最小剪枝效果 |

**文档声称**: Zone Map 剪枝可减少 **40-60%** 的 I/O 操作 (针对高选择性范围查询)。

### 2.4 点查询中的 Zone Map 集成

在 `read_engine.rs` 的 `search_segment()` 中:
- 计算 `cached_blocks`: `pruner.find_blocks_to_scan(zm, key, key)`
- 如果 Zone Map 返回空块列表,直接跳过该段
- 在稀疏索引路径中复用 `cached_blocks` 验证偏移是否在目标块内

### 2.5 RangeQueryPruner 组件

**文件**: `src/query/pruner.rs`

**选择性估算** (`estimate_key_range_selectivity`):
- 使用**公共前缀长度**作为选择性度量
- `specificity = common_prefix_len / avg_key_len`
- 选择率 = `1.0 - specificity`

**剪枝启用启发式**:
- 选择率 < 10%: 几乎肯定启用剪枝
- 选择率 > 95%: 仍然启用 (Zone Map 检查开销极低)

---

## 3. MergeIterator K 路合并

### 3.1 架构设计

**文件**: `src/compaction/merge_iterator.rs`

**核心组件**:
- `KVIterator` trait: 定义流式 KV 迭代器接口 (`next()`, `peek()`, `has_next()`)
- `MergeIterator<I>`: K 路合并迭代器
- `SegmentIterator`: 段文件流式迭代器
- `MergeIteratorBuilder`: 构建器模式

### 3.2 K 路合并算法

**最小堆实现**:
```rust
pub struct MergeIterator<I: KVIterator> {
    heap: BinaryHeap<HeapItem<I>>,
    last_key: Option<String>,        // 去重用
    duplicates_removed: u64,         // 移除的重复键计数
    tombstones_cleaned: u64,         // 清理的墓碑计数
}
```

**堆排序逻辑** (`HeapItem::cmp`):
1. **主比较**: 键值比较 (反转为最小堆)
2. **平局处理**: `sequence` 高的段优先级更高 (新段值覆盖旧段)

**去重策略**:
- 收集所有相同键的条目
- 保留 `sequence` 最高的值 (最新段的数据)
- 统计 `duplicates_removed`

### 3.3 内存效率

| 方案 | 内存复杂度 |
|------|------------|
| 之前 (BTreeMap) | `O(total_keys * avg_value_size)` |
| 之后 (MergeIterator) | `O(num_segments * avg_value_size)` |

### 3.4 SegmentIterator 流式读取

从段文件 mmap 直接流式读取条目:
- 跳过墓碑条目 (空值)
- 共享 `tombstones_skipped` 计数器
- 支持 `peek()` 操作

---

## 4. RangeScanIterator 范围查询

### 4.1 核心实现

**文件**: `src/query/scan.rs`

**关键设计**:
- **惰性求值 (Lazy Evaluation)**: 条目按需获取,非一次性加载全部
- **多层级段遍历**: 按 LSM-Tree 层级顺序 (L0 最新到最旧,L1-L3 按层级)
- **接口抽象**: 通过 `QuerySegmentProvider` trait 抽象段数据访问

### 4.2 配置选项

```rust
pub struct RangeScanConfig {
    pub enable_pruning: bool,        // 启用 Zone Map 剪枝
    pub enable_prefetch: bool,       // 启用顺序预取
    pub limit: usize,                // 返回条目上限 (0=无限制)
    pub include_deleted: bool,       // 包含已删除条目
    pub prefetch_batch_size: u32,    // 预取批次大小
    pub readahead_entries: usize,    // 预读缓冲区条目数 (默认16)
}
```

### 4.3 查询路径

`FileKV::range()` -> `range_with_config()` -> `RangeScanIterator::new()`

迭代器实现了 `Iterator` trait,在 `next()` 方法中:
1. 检查 limit 限制 (`entries_returned >= config.limit`)
2. 尝试从当前段获取下一个条目
3. 当前段耗尽时自动切换到下一段

### 4.4 Readahead 机制

```rust
pub readahead_entries: usize,  // 默认 16
```

`RangeScanIterator` 维护 `readahead_buffer` (VecDeque):
- 批量预读 16 个条目到缓冲区
- 减少系统调用次数
- 提升顺序读取吞吐量

---

## 5. 多层查询缓存架构

### 5.1 7 层缓存路径

查询路径 (`read_engine.rs::get`):

```
1. MemTable (内存, 最快)
2. Prefetch Cache (SequentialPrefetcher 预取的 KV 对)
3. Block Cache (O(1) DashMap 查找, 按键索引)
4. Global Key Index (O(log n) 段位置查找)
   └── Query Result Cache (500K 容量, 60s TTL)
5. Bloom Filter (快速否定查找)
6. Zone Map (块级剪枝)
7. 段文件直接读取
```

### 5.2 Global Key Index 查询缓存

```rust
query_cache: Cache<Arc<str>, Option<KeyLocation>>
```

- **容量**: 500,000 条目
- **TTL**: 60 秒
- **缓存命中和未命中** (both hits and misses)
- **失效策略**: 插入/删除/批量操作时失效对应键

### 5.3 Block Cache

**文件**: `src/cache/block_cache.rs`

- 使用 `DashMap` 实现并发安全
- 支持按键索引 (`get_by_key`) 和按偏移索引 (`get`)
- LRU 淘汰策略

### 5.4 Cache Warmer (缓存预热)

**文件**: `src/cache/warmup.rs`

**预热策略**:
| 策略 | 描述 |
|------|------|
| Recent | 加载最近写入的条目 (段尾部) |
| Frequent | 加载高密度段的条目 |
| SizeBased | 加载最优大小范围的条目 (目标 1KB) |
| Hybrid | 组合策略 (recent=0.4, size=0.3, density=0.3) |

**配置**:
```rust
pub struct CacheWarmingConfig {
    pub max_entries: usize,           // 默认 1000
    pub max_memory_bytes: usize,      // 默认 16MB
    pub min_entry_size: usize,        // 默认 64 字节
    pub max_entry_size: usize,        // 默认 64KB
}
```

### 5.5 L2 Cache

项目包含 L2 缓存层 (`src/cache/l2_cache.rs`),支持磁盘持久化缓存。

---

## 6. 短路和早停优化

### 6.1 Bloom Filter 短路

在 `search_segment()` 中:
```rust
if !contains {
    None  // Bloom 说键不存在,跳过整个段
} else {
    Some(true)  // Bloom 说键可能存在,继续搜索
}
```

**效果**: Bloom Filter 99% 准确率,快速否定不存在的键,避免磁盘 I/O。

### 6.2 Zone Map 早停

- `find_overlapping_blocks()` 返回空列表时,直接跳过段
- 点查询中: `if blocks.is_empty() { return Ok(None); }`

### 6.3 范围查询 Limit 早停

```rust
// RangeScanIterator::next()
if self.config.limit > 0 && self.entries_returned >= self.config.limit {
    return None;  // 达到限制,立即停止
}
```

### 6.4 段级 key range 早停

在 L1+ 层级搜索中:
```rust
if let (Some(ref min_key), Some(ref max_key)) = (&*min_key, &*max_key) {
    if key < min_key.as_str() || key > max_key.as_str() {
        continue;  // 键不在段范围内,跳过
    }
}
```

### 6.5 Dense Index 快速路径

```rust
// 优先尝试 dense index,命中则跳过 Bloom/Zone Map
if let Some(raw_value) = segment.get_by_key(key)? {
    return Ok(Some(value_bytes));  // 直接返回,跳过后续所有检查
}
```

---

## 7. Cache Warmer 预热策略

### 7.1 预热触发时机

- 启动后自动预热
- Compaction 后重新预热
- 手动触发

### 7.2 Hybrid 预热算法

```rust
// 组合策略权重
let score = recent_score * 0.4 + size_score * 0.3 + density_score * 0.3;
```

**目标**: 最大化缓存命中率,同时控制内存使用

### 7.3 预热效果

| 策略 | 缓存命中率 | 预热时间 |
|------|-----------|---------|
| Recent | 65% | 快 |
| SizeBased | 70% | 中 |
| Hybrid | 85% | 慢 |

---

## 8. 自适应预取

### 8.1 SequentialDetector

检测顺序访问模式:
- 跟踪上次访问的 key
- 计算步长 (stride)
- 检测连续 N 次相同步长后触发预取

### 8.2 AdaptivePrefetcher

根据准确率动态调整:
```rust
// 高准确率 (>80%): 增加预取距离
if accuracy > 0.8 {
    prefetch_distance = (prefetch_distance * 2).min(max_window);
}
// 低准确率 (<50%): 减少预取距离
else if accuracy < 0.5 {
    prefetch_distance = (prefetch_distance / 2).max(1);
}
```

### 8.3 预取配置预设

| 模式 | prefetch_distance | max_window |
|------|-------------------|------------|
| Conservative | 1 | 2 |
| Balanced | 2 | 5 |
| Performance | 4 | 10 |
| Extreme | 8 | 20 |

---

## 9. 性能测试数据

### 9.1 基准测试文件

| 文件 | 描述 |
|------|------|
| `benches/file_kv_inno002_bench.rs` | INNO-002 功能端到端测试 |
| `benches/05_range_compaction.rs` | 范围查询和压缩性能 |
| `benches/07_professional_benchmark.rs` | 10M 键专业基准测试 |
| `benches/02_cache_performance.rs` | 缓存性能测试 |
| `benches/03_bloom_filter.rs` | Bloom Filter 性能 |
| `benches/rocksdb_fair_comparison.rs` | 与 RocksDB 公平对比 |

### 9.2 性能提升数据

| 优化 | 性能提升 |
|------|----------|
| Zone Map 剪枝 | 40-60% 减少 I/O 操作 |
| 顺序预取 | 15%+ 提高缓存命中率 |
| Dense Index 快速路径 | 20%+ 降低 get() 延迟 |
| Bloom Filter 负向查询 | 7.23µs (RocksDB 34.2x 快) |
| 全 KV Get (热点缓存) | 278-285ns (RocksDB 2107-2158x 快) |
| 全 KV Get (冷缓存) | 417-435ns (~15x 快于 RocksDB) |

### 9.3 范围查询性能

| 查询类型 | 延迟 | 吞吐量 |
|----------|------|--------|
| 点查询 (hot cache) | 278-285ns | ~3.5M ops/sec |
| 点查询 (cold cache) | 417-435ns | ~2.3M ops/sec |
| 范围查询 (100 keys) | ~50µs | ~20K ops/sec |
| 范围查询 (1000 keys) | ~500µs | ~2K ops/sec |

---

## 10. 关键文件索引

| 文件路径 | 职责 |
|---------|------|
| `src/query/mod.rs` | 查询模块入口 |
| `src/query/pruner.rs` | RangeQueryPruner 实现 |
| `src/query/zone_map.rs` | Zone Map 数据结构和算法 |
| `src/query/scan.rs` | RangeScanIterator 实现 |
| `src/compaction/merge_iterator.rs` | MergeIterator K路合并 |
| `src/compaction/segment_iterator.rs` | SegmentIterator 流式读取 |
| `src/engine/read_engine.rs` | 读取引擎 (多层缓存查询路径) |
| `src/core/global_index.rs` | 全局索引 (含 Query Cache) |
| `src/cache/warmup.rs` | Cache Warmer 预热 |
| `src/cache/prefetch.rs` | SequentialPrefetcher 预取 |
| `src/tests/range_query.rs` | 范围查询集成测试 |

---

## 总结

tokitai-filekv 的查询优化通过多层创新实现极致性能:

1. **Zone Map 剪枝**: 块级元数据减少 40-60% I/O
2. **MergeIterator**: O(n) 内存复杂度的 K 路合并
3. **RangeScanIterator**: 惰性求值 + 预读机制
4. **7 层缓存**: 从内存到磁盘的渐进式查询
5. **短路优化**: Bloom、Zone Map、Limit 早停
6. **自适应预取**: 根据访问模式动态调整

这些优化使 tokitai-filekv 在点查询和范围查询上都显著优于传统 LSM-Tree 实现。
