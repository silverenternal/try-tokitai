# PERF-006: 全局有序索引设计

## 1. 当前问题分析

### 1.1 get() 路径

当前 `ReadEngine::get()` 方法（位于 `src/engine/read_engine.rs`）的查找流程如下：

1. **MemTable 查找** - O(1) HashMap 查找
2. **Prefetch Cache 查找** - 顺序预取缓存
3. **Block Cache 查找** - O(1) DashMap 查找
4. **Segment 遍历** - 当以上缓存都未命中时：
   - 获取所有 segments 快照 (`segments_snapshot`)
   - 按 level 分组 (`by_level: BTreeMap<u8, Vec<usize>>`)
   - **L0 层**: 从新到旧遍历所有 L0 segment，对每个 segment 调用 `search_segment()`
   - **L1+ 层**: 按 key range 查找目标 segment，找到后调用 `search_segment()`

每个 `search_segment()` 调用包含：
- Dense index 快速路径（如果启用）
- Bloom filter 查找（负面过滤）
- Zone Map 检查
- Sparse index 查找 (`index.find(key)`)
- 实际磁盘读取 (`segment.read_at(pos)`)

**关键代码片段** (`read_engine.rs` L230-260):
```rust
// L0: Search newest to oldest (key ranges may overlap, must check all)
if let Some(l0_indices) = by_level.get(&0) {
    let mut l0_sorted: Vec<_> = l0_indices.clone();
    l0_sorted.sort_by(|&a, &b| segment_data[b].0.id.cmp(&segment_data[a].0.id));

    for &idx in &l0_sorted {
        // 对每个 L0 segment 都执行 search_segment
        if let Some(value) = self.search_segment(segment, key, ...)? {
            return Ok((Some(value), CacheLookupResult::DiskHit));
        }
    }
}
```

### 1.2 为什么需要遍历

**L0 Segment 重叠问题**:

在 LSM-Tree 架构中，L0 层（最新层）的 segment 之间存在 **key range 重叠**，因为：
- MemTable flush 到 L0 时，每个 segment 包含完整的 memtable 快照
- 不同 time window 的 memtable 可能包含相同 key 的不同版本
- L0 segment 之间没有经过 compaction，key range 无法保证不重叠

因此，对于任意 key 的 get() 请求，**必须检查所有 L0 segment** 才能确定 key 是否存在。

相比之下，L1+ 层经过 compaction 后 key range 不重叠，可以通过 min_key/max_key 快速定位到唯一可能包含该 key 的 segment。

**当前代码中的体现** (`read_engine.rs` L262-285):
```rust
// L1+: 使用 key range 快速定位
for &idx in level_indices {
    let key_in_range = {
        if let (Some(ref min_key), Some(ref max_key)) = (&*min_key, &*max_key) {
            key >= min_key.as_str() && key <= max_key.as_str()
        } else {
            true
        }
    };
    if !key_in_range { continue; }  // 不在范围内，跳过
    // 找到后直接返回，不需要遍历其他 segment
}
```

### 1.3 性能影响

**10M keys 场景的预估**:

| 指标 | 当前（无全局索引） | 有全局索引 |
|------|-------------------|-----------|
| L0 segment 数量 | 假设 10 个 | 1 个（直接定位） |
| 每个 get() 的 segment 遍历 | 10 次 search_segment | 0-1 次 |
| 每次 search_segment 开销 | Bloom + ZoneMap + SparseIndex + 可能磁盘 I/O | N/A |
| 单次 get() 延迟（缓存未命中） | ~50-200ms | ~5-20ms |
| 索引内存开销 | 每 segment 独立 SparseIndex | ~440MB 全局索引 |

**热点分析**:
- 当 L0 有 N 个 segment 时，get() 最坏情况需要执行 N 次 `search_segment()`
- 每次 `search_segment()` 包含：Bloom filter 加载（如果未缓存）、Zone Map 检查、Sparse Index 查找
- 即使 Bloom filter 能快速否定，也需要为每个 segment 加载/检查 Bloom filter

## 2. 设计方案：GlobalKeyIndex

### 2.1 数据结构

```rust
use std::collections::BTreeMap;
use parking_lot::RwLock;

/// 全局 key 位置索引
/// 
/// 维护所有 key 到 segment 位置的映射，避免 get() 时遍历多个 segment。
/// 使用 BTreeMap 保证 key 的有序性，支持范围查询。
pub struct GlobalKeyIndex {
    /// key -> segment 位置映射
    positions: BTreeMap<String, KeyLocation>,
    /// 分代计数，Compaction 后用于区分新旧条目
    generation: u64,
    /// 读写锁，保护并发访问
    lock: RwLock<()>,
}

#[derive(Clone, Debug)]
pub struct KeyLocation {
    /// segment ID
    pub segment_id: u64,
    /// segment 内的 block offset
    pub block_offset: u64,
    /// value 长度
    pub value_len: usize,
    /// 所属 level（用于 level-aware 查找）
    pub level: u8,
    /// 分代号，compaction 后递增
    pub generation: u64,
}
```

### 2.2 更新策略

**MemTable 写入时：同步更新全局索引**

```
put(key, value) -> MemTable.insert(key, value)
                 -> GlobalKeyIndex.insert(key, KeyLocation {
                       segment_id: pending,  // 尚未写入 segment
                       level: 0,
                       ...
                   })
```

- MemTable flush 到 L0 segment 时，批量更新 GlobalKeyIndex
- 使用写锁保护，但批量操作减少锁竞争

**Compaction 完成后：异步批量更新**

```
Compaction 完成 -> 生成新 segment
                -> 遍历新 segment 的所有 key
                -> GlobalKeyIndex.batch_update(new_keys)
                -> 删除旧 segment 的 key 条目（按 generation 清理）
```

- Compaction 后通过 channel 发送更新请求到后台线程
- 后台线程批量应用更新，不阻塞读取路径

### 2.3 Compaction 集成

**Compaction 流程集成点** (`src/compaction/mod.rs`):

1. **Compaction 前**: 记录被合并 segment 的 generation
2. **Compaction 中**: 正常合并 key，写入新 segment
3. **Compaction 后**:
   - 从 GlobalKeyIndex 中删除旧 segment 的 key 条目（generation 匹配）
   - 添加新 segment 的 key 条目（新 generation）
   - 递增全局 generation 计数

```rust
/// Compaction 后的索引更新
pub fn update_global_index_after_compaction(
    global_index: &GlobalKeyIndex,
    removed_segments: &[u64],
    new_segment_id: u64,
    new_segment_keys: &[(String, u64, u64)],  // (key, offset, value_len)
) -> Result<()> {
    // 1. 删除旧 key
    global_index.remove_by_segments(removed_segments);
    
    // 2. 添加新 key
    global_index.batch_insert(new_segment_id, new_segment_keys);
    
    // 3. 递增 generation
    global_index.advance_generation();
}
```

### 2.4 get() 路径优化

**优化后的 get() 流程**:

```rust
pub fn get_with_global_index(&self, key: &str) -> Result<Option<Bytes>> {
    // 1. MemTable
    if let Some(value) = self.memtable.get(key) {
        return Some(value);
    }
    
    // 2. Block Cache
    if let Some(value) = self.block_cache.get(key) {
        return Some(value);
    }
    
    // 3. GlobalKeyIndex 查找（替代遍历所有 segment）
    if let Some(location) = self.global_index.get(key) {
        // 直接定位到 segment + offset
        let segment = self.get_segment(location.segment_id)?;
        let value = segment.read_at(location.block_offset, location.value_len)?;
        return Some(value);
    }
    
    None
}
```

## 3. 内存开销

### 3.1 基础估算

```
10M keys × (String 平均 20 bytes + KeyLocation 24 bytes) ≈ 440MB
```

详细分解：
- `String`: 24 bytes (指针 8 + 长度 8 + 容量 8) + 实际内容 ~20 bytes = 44 bytes
- `KeyLocation`: segment_id(8) + block_offset(8) + value_len(8) + level(1) + generation(8) + 对齐 = ~40 bytes
- BTreeMap 节点开销: ~48 bytes/entry
- **总计**: ~132 bytes/entry × 10M = **~1.3GB**

### 3.2 优化方案

#### 方案 A: 紧凑 key 表示

使用固定长度编码或 interned string：

```rust
/// 紧凑 key 存储
pub struct CompactKey {
    /// key 的哈希值（用于快速比较）
    hash: u64,
    /// key 数据（短 key 内联存储，长 key 引用全局 string table）
    data: InlineOrRef,
}

enum InlineOrRef {
    Inline([u8; 16]),  // 短 key 直接存储
    Ref(u32),           // 长 key 引用 string table
}
```

**优化后内存**: ~60 bytes/entry × 10M = **~600MB**

#### 方案 B: 分级索引（仅索引热数据）

```rust
pub struct TieredGlobalIndex {
    /// L0 + L1 热数据索引（必须完整）
    hot_index: GlobalKeyIndex,
    
    /// L2+ 冷数据索引（可采样或延迟加载）
    cold_index: Option<SparseIndexSummary>,
}
```

**优化后内存**: 热数据 2M keys × 60 bytes + 冷数据摘要 ~50MB = **~170MB**

#### 方案 C: 布隆过滤器辅助

在 GlobalKeyIndex 前加一层全局 Bloom Filter，快速判断 key 不存在：

```
get(key) -> GlobalBloom.might_contain(key)?
         -> GlobalKeyIndex.get(key)
         -> segment.read_at(...)
```

## 4. 实施计划

### Phase 1: 基础实现（2 周）

**目标**: GlobalKeyIndex 核心结构 + 同步更新路径

| 任务 | 详情 | 预计时间 |
|------|------|----------|
| GlobalKeyIndex 结构定义 | `src/core/global_index.rs` | 2 天 |
| MemTable 写入集成 | 修改 `put()` 路径 | 2 天 |
| get() 路径优化 | 修改 `ReadEngine::get()` | 3 天 |
| 基本测试 | 单元测试 + 集成测试 | 3 天 |

**交付物**:
- `GlobalKeyIndex` 核心结构
- 同步更新路径（MemTable -> 全局索引）
- `get()` 使用全局索引快速定位
- 基本功能测试

### Phase 2: 异步更新（1 周）

**目标**: 后台线程批量更新 + Compaction 集成

| 任务 | 详情 | 预计时间 |
|------|------|----------|
| 后台更新线程 | channel + 批量处理 | 2 天 |
| Compaction 集成 | `execute_compaction_inner` 后更新 | 2 天 |
| Generation 清理 | 旧条目清理逻辑 | 1 天 |

**交付物**:
- 异步批量更新机制
- Compaction 后的索引更新
- Generation 管理

### Phase 3: 内存优化（1 周）

**目标**: 紧凑 key 表示 + 分级索引

| 任务 | 详情 | 预计时间 |
|------|------|----------|
| 紧凑 key 编码 | InlineOrRef 实现 | 2 天 |
| 分级索引 | Hot/Cold 分离 | 2 天 |
| 内存监控 | Prometheus metrics | 1 天 |

**交付物**:
- 内存占用降低 50%+
- 分级索引策略
- 内存使用监控

## 5. 预期性能提升

| 指标 | 目标 |
|------|------|
| 10M keys get() 延迟（缓存未命中） | 降低 80%+ |
| L0 segment 遍历次数 | 从 N 次降至 0-1 次 |
| 索引更新开销 | <5% 写入开销 |
| 内存占用（10M keys） | <600MB（紧凑编码后） |

**性能分析**:

- **当前**: get() 最坏情况遍历 10 个 L0 segment，每个 segment 执行 Bloom + ZoneMap + SparseIndex 查找
- **优化后**: get() 通过 GlobalKeyIndex O(log N) 直接定位到目标 segment + offset
- **延迟降低**: 避免 9 次不必要的 segment 查找，节省 ~80-90% 延迟

## 6. 风险与缓解

### 6.1 写入路径阻塞

**风险**: GlobalKeyIndex 更新可能阻塞 put() 操作

**缓解**:
- Phase 1: 使用细粒度锁（RwLock），读取不阻塞
- Phase 2: 异步批量更新，写入路径只发送 channel 消息

### 6.2 内存压力

**风险**: 10M keys 的全局索引占用过大内存

**缓解**:
- 紧凑 key 编码减少 50% 内存
- 分级索引仅保留热数据
- 可配置是否启用全局索引

### 6.3 一致性

**风险**: GlobalKeyIndex 与实际 segment 数据不一致

**缓解**:
- MemTable flush 时原子更新
- Compaction 后使用 generation 验证
- Crash recovery 时重建全局索引

## 7. 向后兼容

- GlobalKeyIndex 为可选功能，通过配置开关控制
- 不启用时，回退到当前 segment 遍历逻辑
- 现有 segment 文件格式、索引格式不变
- 升级时自动重建全局索引（从现有 segment 扫描）

```toml
# FileKV 配置示例
[global_index]
enabled = true
memory_limit_mb = 512
tiered = true  # 启用分级索引
```
