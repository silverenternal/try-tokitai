# FileKV 性能瓶颈深度分析

> **分析日期**: 2026-04-10
> **问题**: full_kv_get 比 RocksDB 慢 ~240x (144ms vs 0.6ms / 1000 keys)
> **测试条件**: 100K entries, 16B key, 100B value, hot cache

---

## 执行摘要

经过全面代码审查，发现 FileKV 性能瓶颈主要来自以下几个方面：

1. **🔴 Critical**: 每个 key 都要遍历所有 segments（ newest to oldest）
2. **🔴 Critical**: 每次 get() 都持有 segments.read() 和 index_manager.read() 锁
3. **🔴 Critical**: 每次 get() 在 segment 级别重复获取 dense_index.read() 锁
4. **🟡 High**: 没有类似 RocksDB 的 Block Cache 按 key 快速定位
5. **🟡 High**: 每次 get() 可能触发多次 I/O（mmap 访问）
6. **🟡 Medium**: 索引数据结构重复存储（IndexManager + SegmentFile）
7. **🟢 Low**: String 分配和复制

---

## 一、Get() 方法完整执行路径分析

### 1.1 代码路径 (lib.rs:914-970)

```rust
pub fn get(&self, key: &str) -> anyhow::Result<Option<Bytes>> {
    // 步骤 1: MemTable 查找 (O(1) hash map)
    if let Some((value, _pointer, deleted)) = self.memtable.get(key) {
        // ... 返回或 tombstone
    }

    // 步骤 2: Block Cache 查找 (O(1) DashMap)
    if let Some(cached) = self.block_cache.get_by_key(key) {
        return Ok(Some(cached));
    }

    // 步骤 3: 获取全局锁 (持有一直到循环结束)
    let segments = self.segments.read();           // ← RwLock 读锁
    let index_manager = self.index_manager.read();  // ← RwLock 读锁

    // 步骤 4: 遍历所有 segments (newest → oldest)
    for (_, segment) in segments.iter().rev() {
        // 4a. Zone Map 检查 (O(1))
        if let Some(index) = index_manager.get_index(segment.id) {
            if !index.key_might_exist(key) {
                continue;  // 跳过这个 segment
            }
        }

        // 4b. Dense Index 查找 (O(log N))
        if let Some(value) = segment.get_by_key(key)? {  // ← 每次获取 RwLock
            let value_bytes = Bytes::from(value);
            self.block_cache.insert_by_key(key.to_string(), value_bytes.clone());
            return Ok(Some(value_bytes));
        }

        // 4c. Sparse Index 查找 (O(N) 线性扫描!)
        if let Some(index) = index_manager.get_index(segment.id) {
            if let Some(pos) = index.find(key) {  // ← 线性扫描!
                let value = segment.read_at(pos, 0)?;  // ← I/O
                // ...
            }
        }
    }

    Ok(None)
}
```

### 1.2 性能分析

假设场景：100K entries，分成 10 个 segments（每个 10K entries）

| 步骤 | 操作 | 时间复杂度 | 实际开销 |
|------|------|-----------|---------|
| 1 | MemTable get | O(1) | ~100ns |
| 2 | Block Cache get | O(1) | ~200ns (DashMap) |
| 3 | 获取锁 | O(1) | ~50ns (parking_lot) |
| 4 | 循环 10 次 segments | O(S) | S = 10 |
| 4a | Zone Map 检查 | O(1) × 10 | ~1μs |
| 4b | Dense Index 查找 | O(log N) × 10 | ~10 × log(10K) ≈ 140 次比较 |
| 4b-1 | 获取 dense_index.read() | O(1) × 10 | 10 次锁获取 |
| 4b-2 | BTreeMap::get() | O(log N) × 10 | String 比较 + 树遍历 |
| 4b-3 | mmap 读取 + 校验 | O(1) × 10 (如果命中) | ~1μs |
| 4c | Sparse Index (fallback) | O(N) × 10 (最坏) | **10 × 10K = 100K 次比较!** |

**关键问题**: 如果 key 在所有 segments 中都不存在（negative lookup），需要：
- 遍历全部 10 个 segments
- 每个 segment 做 dense index 查找（BTreeMap::get = O(log N)）
- 如果 dense index 没找到，可能还要做 sparse index 线性扫描
- **总开销**: ~140 次 String 比较 + 10 次锁获取 + 可能的 I/O

---

## 二、RocksDB 对比分析

### 2.1 RocksDB 的 Get() 路径

RocksDB 使用 LSM-Tree 架构，但有多个关键优化：

```
RocksDB Get(key):
1. MemTable lookup (skiplist, O(log N))
2. Immutable MemTables lookup
3. Block Cache check (LRU cache for data blocks)
4. SST File lookup:
   a. Bloom Filter check per file (O(1), 快速排除不存在)
   b. Block index lookup (hash-based, O(1))
   c. Data block read (if cache miss, but rare for hot cache)
```

**关键差异**:

| 特性 | RocksDB | FileKV | 影响 |
|------|---------|--------|------|
| MemTable | Skiplist O(log N) | Hash Map O(1) | FileKV 更快 |
| Block Cache | Block-level LRU | Key-level DashMap | RocksDB 更高效 |
| SST/SSTable 索引 | Hash-based block index | BTreeMap dense index | RocksDB O(1) vs FileKV O(log N) |
| Bloom Filter | Per-file, 快速排除 | 无 per-segment bloom filter | RocksDB 更快排除 |
| 锁策略 | Fine-grained per-file | Global segments + index_manager | RocksDB 锁竞争更少 |

### 2.2 为什么 RocksDB 快 240 倍

1. **Block Index 设计**: RocksDB 使用 hash-based block index，O(1) 查找
   - FileKV 使用 BTreeMap，O(log N) 查找
   
2. **Bloom Filter 集成**: RocksDB 每个 SST file 都有 bloom filter
   - 快速排除不存在的 key（99% FPR = 1% 误判）
   - FileKV 只有 zone map（只检查 key range，不检查存在性）

3. **Block Cache 效率**: RocksDB 缓存的是数据块（~4KB-64KB）
   - 一个 block 可能包含多个 keys
   - FileKV 缓存的是单个 key，每次 get() 都要插入 DashMap

4. **I/O 模式**: RocksDB 使用 O_DIRECT + 自己的 buffer pool
   - FileKV 使用 mmap，可能有 page fault 开销

---

## 三、核心性能瓶颈详细分析

### 3.1 🔴 Critical: 每 Key 遍历所有 Segments

**问题**: 对于每个 get(key)，即使有 zone map 剪枝，仍可能遍历多个 segments

**示例**: 100K entries, 10 segments
- 每个 segment 有 10K entries
- Dense index 查找: O(log 10K) ≈ 14 次比较
- 10 个 segments: 10 × 14 = 140 次 String 比较
- 加上锁获取: 10 次 dense_index.read()

**RocksDB 对比**: 
- Bloom Filter 先排除不存在的文件（O(1), bit check）
- 只需要检查 1-2 个 SST files（因为 leveled compaction key range 不重叠）

### 3.2 🔴 Critical: 全局锁持有时间长

**问题**: `segments.read()` 和 `index_manager.read()` 在整个循环期间持有

```rust
let segments = self.segments.read();           // 获取锁
let index_manager = self.index_manager.read(); // 获取锁

for (_, segment) in segments.iter().rev() {
    // ... 可能遍历 10+ 个 segments
    // ... 每个 segment 做 BTreeMap::get()
}
// 锁在这里才释放
```

**影响**: 
- 阻塞 flush_memtable()（需要 index_manager.write()）
- 阻塞 compaction（需要 segments.write()）
- 高并发下锁竞争激烈

### 3.3 🔴 Critical: Dense Index 重复存储

**问题**: Dense index 存储了两次：
1. `IndexManager.dense_indexes: BTreeMap<u64, DenseIndex>`
2. `SegmentFile.dense_index: Option<RwLock<BTreeMap<String, DenseIndexEntry>>>`

**内存开销**: 
- 100K entries × (String 32B + DenseIndexEntry 32B + BTreeMap node 48B) ≈ 11MB × 2 = 22MB
- 双倍内存占用

**一致性风险**: 两个副本可能不同步

### 3.4 🟡 High: 没有 Per-Segment Bloom Filter

**问题**: Zone map 只能检查 key range，不能快速判断 key 是否存在

```rust
// Zone map 只能做范围检查
if !index.key_might_exist(key) {  // 只检查 min_key <= key <= max_key
    continue;
}

// 但即使 range 匹配，key 也可能不存在
// 必须继续做 dense index 查找
```

**RocksDB 对比**: 
- 每个 SST file 都有 bloom filter
- 99% 的 negative lookups 在 bloom filter 层就被排除
- 不需要读取 index 或数据

### 3.5 🟡 High: Sparse Index 是 O(N) 线性扫描

**问题**: `SparseIndex::find()` 使用线性扫描

```rust
// sparse_index.rs:55
pub fn find(&self, key: &str) -> Option<u64> {
    self.entries
        .iter()
        .find(|e| e.key == key)  // ← O(N) 线性扫描!
        .map(|e| e.offset)
}
```

**影响**: 对于 10K entries 的 segment:
- 最坏情况: 10K 次 String 比较
- 平均情况: 5K 次 String 比较

### 3.6 🟡 Medium: 每次 Get 都可能触发 I/O

**问题**: 即使有 mmap，每次 dense index miss 后都要访问 mmap

```rust
// segment.rs:645
let mmap_guard = self.mmap.load();  // AtomicArcSwap load
let mmap = match &*mmap_guard {
    Some(m) => m,
    None => return Err(...),
};

let file_size = mmap.len();
// ... 边界检查
let value = mmap[value_pos..value_pos + value_len as usize].to_vec();  // 复制!
```

**影响**:
- mmap 访问可能触发 page fault（如果数据不在内存）
- `to_vec()` 分配新内存并复制

---

## 四、Benchmark 分析

### 4.1 测试条件公平性

** rocksdb_fair_comparison.rs 的设计**:

```rust
// 相同数据集
const NUM_ENTRIES: usize = 100_000;
const KEY_SIZE: usize = 16;
const VALUE_SIZE: usize = 100;

// 都预热缓存
for key in &dataset.keys {
    let _ = kv.get(key);  // FileKV warmup
    let _ = db.get(key.as_bytes());  // RocksDB warmup
}

// 测试 hot cache 读取
group.bench_function(BenchmarkId::new("FileKV", "hot_cache"), |b| {
    b.iter(|| {
        for key in &dataset.keys[..1000] {
            black_box(kv.get(key).unwrap());
        }
    });
});
```

**评估**: 测试条件是公平的，都使用 hot cache。

### 4.2 为什么差距这么大

在 hot cache 场景下：
- **RocksDB**: Block Cache 命中 → 直接返回缓存的 block (O(1))
- **FileKV**: Block Cache 可能命中也可能不命中
  - 如果命中: DashMap get (O(1)) ✅
  - 如果不命中: 遍历 segments + dense index 查找 ❌

**关键差异**: RocksDB 的 Block Cache 按 block 缓存（一个 block 包含多个连续 keys），而 FileKV 按单个 key 缓存。

对于 1000 个 keys 的测试：
- RocksDB: 可能只需要加载 10-20 个 blocks（每个 block ~4KB，包含 ~40 keys）
- FileKV: 可能需要查找 10 个 segments 的 dense index

---

## 五、优化建议（按优先级排序）

### P0: 立即可做（预期改进 10-50x）

#### 1. Per-Segment Bloom Filter
**方案**: 为每个 segment 创建 bloom filter，compaction 时更新
```rust
// 新增
pub struct SegmentBloomFilter {
    filter: BloomFilter,
    segment_id: u64,
}

// 在 get() 中
if let Some(bloom) = index_manager.get_bloom_filter(segment.id) {
    if !bloom.might_contain(key) {
        continue;  // 快速排除
    }
}
```
**预期改进**: Negative lookup 从 O(S × log N) → O(S × 1)，排除 99% 不必要的查找

#### 2. 使用 HashMap 替代 BTreeMap 做 Dense Index
**方案**: Dense index 只需 point lookup，不需要有序
```rust
// 之前
pub entries: BTreeMap<String, DenseIndexEntry>

// 之后  
pub entries: HashMap<String, DenseIndexEntry, AHasher>
```
**预期改进**: O(log N) → O(1)，减少 10x String 比较

#### 3. 消除 Dense Index 重复存储
**方案**: 只保留一份 index，SegmentFile 不持有 dense_index
```rust
// 删除
// SegmentFile.dense_index: Option<RwLock<BTreeMap<...>>>

// 通过 IndexManager 统一访问
if let Some(dense_idx) = index_manager.get_dense_index(segment.id) {
    if let Some(entry) = dense_idx.get(key) {
        // 直接读取
    }
}
```
**预期改进**: 减少 50% 索引内存占用

### P1: 短期可做（预期改进 2-5x）

#### 4. 缩短锁持有时间
**方案**: 在循环内不持有全局锁
```rust
// 方案 A: 快照
let segments_snapshot = {
    let segments = self.segments.read();
    segments.clone()  // 只克隆 Arc<SegmentFile>
};
drop(segments);  // 立即释放锁

for segment in segments_snapshot.iter().rev() {
    // 现在不持有全局锁
}
```

#### 5. Block Cache 改为 Block-Level 而非 Key-Level
**方案**: 缓存数据块而非单个 key
```rust
// 之前: 按 key 缓存
block_cache.insert_by_key("key_123".to_string(), value_bytes);

// 之后: 按 block 缓存
let block_offset = entry.offset / BLOCK_SIZE * BLOCK_SIZE;
block_cache.put(segment_id, block_offset, block_bytes);
```

#### 6. Sparse Index 改为二分查找
**方案**: 排序后使用 binary_search
```rust
// 构建时排序
self.entries.sort_by_key(|e| e.key.clone());

// 查找时二分
pub fn find(&self, key: &str) -> Option<u64> {
    let idx = self.entries.binary_search_by_key(&key, |e| &e.key);
    // ...
}
```
**预期改进**: O(N) → O(log N)

### P2: 中期架构改进（预期改进 5-10x）

#### 7. 实现 Leveled Compaction + Key Range 分区
**方案**: L1+ segments key range 不重叠
```
L0: [a-z], [a-m], [n-z]  (重叠)
L1: [a-f], [g-m], [n-z]  (不重叠)
```
**影响**: Get() 只需要检查 1 个 L1 segment + 少量 L0 segments

#### 8. 使用更高效的 Key 存储
**方案**: 前缀压缩 + 固定大小 key
```rust
// 之前: String (24B header + key data)
// 之后: 前缀压缩 (平均 2-4B per key)
```

---

## 六、总结

### 6.1 根本原因

FileKV 比 RocksDB 慢 240x 的**根本原因**是：

1. **索引设计差异**: RocksDB 用 hash-based block index (O(1))，FileKV 用 BTreeMap (O(log N))
2. **Bloom Filter 集成**: RocksDB 每文件一个 bloom filter 快速排除，FileKV 只有 zone map 范围检查
3. **缓存粒度**: RocksDB 缓存 block（多 keys），FileKV 缓存单个 key
4. **锁竞争**: FileKV 持有全局锁遍历所有 segments

### 6.2 预期改进路径

| 优化项 | 预期改进 | 实现难度 |
|--------|---------|---------|
| Per-Segment Bloom Filter | 5-10x | 低 |
| HashMap vs BTreeMap | 2-3x | 低 |
| 消除索引重复 | 内存 -50% | 低 |
| 缩短锁持有时间 | 1.5-2x | 中 |
| Block-Level Cache | 2-3x | 中 |
| **总计** | **30-100x** | |

实现 P0 优化后，FileKV 应该能达到 RocksDB 的 1/10 到 1/3 性能。

---

*分析完成时间: 2026-04-10*
*下一步: 实现 P0 优化项并重新运行 benchmark*
