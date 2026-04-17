# Read Path 优化创新

> **状态**: ✅ 已实现  
> **引入版本**: v0.4.0 - v0.8.0 (多轮迭代)  
> **核心代码**: `src/engine/read_engine.rs`, `src/core/global_index.rs`

---

## 概述

读路径是 LSM-Tree 的另一关键性能路径,tokitai-filekv 实现了 6 项优化,构建多层级查找架构。

---

## 1. Multi-Layer Lookup (多层查找)

### 问题
单一查找路径无法兼顾速度和准确率,热数据也要走完整查找流程。

### 创新方案
构建多层级查找路径,热数据快速返回,冷数据降级到完整查找。

### 实现细节
- **文件**: `src/engine/read_engine.rs`
- **查找顺序**:
  1. MemTable (内存,最新数据)
  2. Prefetch Cache (预取缓存,顺序访问)
  3. Block Cache (Moka O(1),热数据)
  4. Global Key Index (O(log n) 定位 segment)
  5. Segment 遍历 (Bloom Filter + Zone Map 剪枝)

### 性能影响
- 热数据查找 <300ns (BlockCache 命中)
- 冷数据查找 ~400ns (GlobalKeyIndex 命中)
- 最差情况: 遍历所有 segments

### 相关测试
- `src/engine/read_engine.rs` 内置测试
- `benches/02_cache_performance.rs` 缓存性能基准

---

## 2. Global Key Index (全局键索引)

### 问题
遍历所有 L0 segments 慢,无法快速定位 key 所在 segment。

### 创新方案
构建全局有序索引,AHashMap O(1) 点查 + BTreeMap O(log n) 范围查询。

### 实现细节
- **文件**: `src/core/global_index.rs`
- **双索引架构**:
  - **AHashMap<Arc<str>, KeyLocation>**: O(1) 点查
  - **BTreeMap**: O(log n) 范围查询
- **缓存**: Moka query cache (500K capacity, 60s TTL)
- **Stale segments**: 标记避免 compaction 期间读到旧数据
- **内存控制**: Memory budget (默认 256MB)
- **精确 offset**: `rebuild_from_segments()` 使用 `iterate_all_with_offset()` 获取精确位置

### 性能影响
- 点查从 O(num_segments) 降低到 O(1)
- 10M keys 场景下查找延迟稳定 <1μs

### 相关测试
- `src/core/global_index.rs` 内置测试

---

## 3. Level-Aware Segment Traversal (层级感知遍历)

### 问题
重叠 segment 必须全检查,即使 key 不在该 segment 范围内。

### 创新方案
L0 从新到旧遍历,L1+ 使用 min_key/max_key range check 快速跳过无关 segment。

### 实现细节
- **文件**: `src/engine/read_engine.rs`
- **L0 遍历**: 从新到旧 (key range 可能重叠,必须检查所有)
- **L1+ 剪枝**: 使用 min_key/max_key 快速跳过无关 segment
- **Bloom Filter**: 进一步过滤不可能存在的 segment

### 性能影响
- L1+ 查询可跳过 80%+ 无关 segment
- 读放大从 O(num_segments) 降低到 O(1-2)

### 相关测试
- `src/engine/read_engine.rs` 内置测试

---

## 4. Zone Map Block-Level Pruning (Zone Map 块级剪枝)

### 问题
Segment 内全 block 扫描,即使 key 只在少数 blocks 中。

### 创新方案
Segment 内进一步按 block 的 min/max key 剪枝,只扫描可能包含 key 的 blocks。

### 实现细节
- **文件**: `src/query/zone_map.rs`, `src/query/pruner.rs`
- **结构体**: `RangeQueryPruner.find_blocks_to_scan()`
- **Zone Map**: 每个 block 记录 min_key, max_key
- **剪枝**: 查询时只扫描 min_key <= query_key <= max_key 的 blocks

### 性能影响
- 范围查询跳过 40-60% blocks
- I/O 减少 2x+

### 相关测试
- `src/query/zone_map.rs` 内置测试
- `benches/file_kv_inno002_bench.rs` INNO-002 基准

---

## 5. Dense Index Fast Path (密集索引快速路径)

### 问题
每次都走完整查找流程 (Bloom + ZoneMap),即使数据在 dense index 中。

### 创新方案
Dense Index 直接命中时跳过 Bloom Filter 和 Zone Map 检查。

### 实现细节
- **文件**: `src/core/segment.rs`
- **方法**: `SegmentFile.get_by_key()` - 通过 AHashMap 直接查找
- **快速路径**: 命中时跳过 Bloom + ZoneMap,减少 20%+ 延迟
- **降级**: 未命中时走标准查找流程

### 性能影响
- 热数据查找延迟降低 20%+
- 避免不必要的 Bloom 检查

### 相关测试
- `src/core/segment.rs` 内置测试

---

## 6. Cross-Segment Sequential Detection (跨 Segment 顺序检测)

### 问题
单 segment 内预取有局限,跨 segment 顺序访问无法受益。

### 创新方案
在 get() 查询中检测跨 segment 的顺序模式,触发跨段预取。

### 实现细节
- **文件**: `src/engine/read_engine.rs`
- **检测器**: `get_sequential_detector` 在 get() 路径中检测跨 segment 顺序
- **预取**: 检测到顺序模式时触发 `trigger_get_prefetch()`,预取后续 numeric key 的 block

### 性能影响
- 跨段顺序读取吞吐量提升 2-4x
- 预取命中率 >80% 时效果显著

### 相关测试
- `src/engine/read_engine.rs` 内置测试

---

## 📊 性能成果汇总

| 指标 | 优化前 | 优化后 | 提升 |
|------|--------|--------|------|
| 热数据查找 | O(num_segments) | **<300ns** | **Multi-Layer Lookup** |
| 点查延迟 | O(num_segments) | **O(1)** | **Global Key Index** |
| L1+ 查询 | 全扫描 | **跳过 80%+** | **Level-Aware Traversal** |
| 范围查询 I/O | 全 block 扫描 | **减少 40-60%** | **Zone Map Pruning** |
| Dense Index 命中 | 走完整流程 | **减少 20%+** | **Fast Path** |
| 跨段顺序读 | 无法预取 | **2-4x 提升** | **Cross-Segment Detection** |

---

## 🔗 相关文档

- [Global Key Index 设计](../filekv/GLOBAL_KEY_INDEX.md) (如存在)
- [Zone Map 设计](../filekv/ZONE_MAP.md) (如存在)
