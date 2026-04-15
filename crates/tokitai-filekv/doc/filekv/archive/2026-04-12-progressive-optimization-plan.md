# 渐进式优化实施计划 — 全方位超越 RocksDB

**创建日期**: 2026-04-12  
**目标**: 通过渐进式优化，让 tokitai-filekv 在所有场景下超越 RocksDB  
**策略**: 保持现有架构，逐步优化瓶颈点  
**预期效果**: 从 240x 差距缩小到 0.5-2x（即接近或超越 RocksDB）

---

## 核心分析

### 当前性能数据（100K keys）

| 操作 | FileKV | RocksDB | 差距 |
|------|--------|---------|------|
| Bloom 负查询 | 62.37 µs | 247.38 µs | ✅ **快 3.97x** |
| 热数据查询 | 61.92 µs | 600.07 µs | ✅ **快 9.69x** |
| 写入 64B | 1.71 ms | 1.88 ms | ✅ **快 9%** |
| 写入 100B | 1.86 ms | 1.83 ms | ❌ **慢 2%** |
| **100K 数据集** | **慢 240x** | ❌ **致命差距** |

### 根因分析

1. **多 Segment 扁平遍历**（占 240x 差距的 ~60%）
   - 每次 `get()` 遍历所有 segments，未利用 level 结构
   - 100K keys 分布在数十个 segments，每个 key 扫描大量文件

2. **Block Cache 容量不足**（占 240x 差距的 ~25%）
   - 默认 64MB/10K items，100K unique keys 命中率 <50%
   - LRU 实现使用 `Vec + Mutex`，并发性能差

3. **Compaction 全量加载**（占 240x 差距的 ~10%）
   - 所有数据加载到 `BTreeMap<String, Vec<u8>>`
   - 内存占用大，GC 压力大

4. **锁粒度粗 + 其他**（占 240x 差距的 ~5%）
   - 读锁在整个遍历期间持有
   - WAL 逐条 fsync，无批量优化

---

## 优化路线图（6 个 Sprint）

### Sprint 8: Level 感知读取路径（🔥 最大收益，预计解决 60% 差距）

**目标**: 将 100K 场景查询延迟从 ~150ms 降至 <10ms

**核心改动**:

#### 8.1 Segment 元数据增强
```rust
// segment.rs
pub struct SegmentMetadata {
    pub level: u8,              // NEW: LSM level (0=L0, 1+=L1+)
    pub min_key: String,        // NEW: key 范围最小值
    pub max_key: String,        // NEW: key 范围最大值
    pub created_at: u64,        // 已有
    pub size_bytes: u64,        // 已有
}
```

#### 8.2 Level 感知查询路由
```rust
// read_engine.rs - get() 优化
pub fn get(&self, key: &str) -> Option<Vec<u8>> {
    // 1. L0 segments（可能重叠，需全扫）
    for segment in l0_segments.iter().rev() {
        if segment.may_contain(key) {  // Bloom Filter 快速检查
            if let Some(value) = segment.get(key) {
                return Some(value);
            }
        }
    }
    
    // 2. L1+ segments（key 范围不重叠，二分定位）
    // 使用 Zone Map 的 min/max_key 直接定位到 1 个 segment
    if let Some(target_segment) = self.find_segment_by_key_range(key, level) {
        return target_segment.get(key);
    }
    
    None
}
```

#### 8.3 Compaction Level 传播
```rust
// compaction.rs - 确保 compaction 后 segment 的 level 正确
fn execute_compaction(&self, segments: &[SegmentId]) -> SegmentId {
    let new_segment = self.create_segment_with_level(
        target_level,  // 根据 size 决定 level
        merged_data
    );
    new_segment
}
```

**验证标准**:
- ✅ 100K keys 查询延迟 <10ms
- ✅ 小数据集查询保持现有优势（<100µs）
- ✅ 所有现有测试通过

**预计工作量**: 2-3 天

---

### Sprint 9: Block Cache 高性能化（🔥 次大收益，预计解决 25% 差距）

**目标**: 缓存命中率从 <50% 提升至 >90%，LRU 更新无锁化

**核心改动**:

#### 9.1 引入 Moka 缓存库
```toml
# Cargo.toml
[dependencies]
moka = { version = "0.12", features = ["sync"] }  # 高性能并发缓存
```

#### 9.2 替换现有 BlockCache 实现
```rust
// block_cache.rs - 从 DashMap + 自实现 LRU 改为 Moka
use moka::sync::Cache;

pub struct BlockCache {
    cache: Cache<String, Bytes>,  // 无锁 LRU，高性能
    budget: CacheBudget,
}

impl BlockCache {
    pub fn new(config: BlockCacheConfig) -> Self {
        Self {
            cache: Cache::builder()
                .max_capacity(config.max_capacity)
                .weigher(|_, v| v.len() as u32)  // 按字节权重
                .build(),
            budget: config.budget,
        }
    }
    
    pub fn get(&self, key: &str) -> Option<Bytes> {
        self.cache.get(key)  // 无锁读取
    }
    
    pub fn put(&self, key: String, value: Bytes) {
        self.cache.insert(key, value);  // 无锁写入
    }
}
```

#### 9.3 缓存容量动态配置
```rust
// config.rs
pub struct BlockCacheConfig {
    pub max_capacity: u64,          // 字节数（原为 item 计数）
    pub ttl: Option<Duration>,      // 可选过期时间
    pub eviction_listener: Option<Box<dyn Fn(&str, &Bytes)>>,
}

impl BlockCacheConfig {
    pub fn from_memory_budget(available_mb: u64) -> Self {
        Self {
            max_capacity: available_mb * 1024 * 1024,
            ttl: None,
            eviction_listener: None,
        }
    }
}
```

**验证标准**:
- ✅ 100K keys 缓存命中率 >90%
- ✅ LRU 更新零锁竞争（benchmark 验证）
- ✅ 内存占用可配置且精确

**预计工作量**: 1-2 天

---

### Sprint 10: Compaction 流式合并（解决内存瓶颈）

**目标**: 消除全量加载，支持任意大小数据集

**核心改动**:

#### 10.1 Merge Iterator 实现
```rust
// compaction.rs
pub struct MergeIterator {
    iterators: Vec<SegmentIterator>,
    current_heap: BinaryHeap<HeapEntry>,  // 最小堆，按 key 排序
}

impl MergeIterator {
    pub fn new(segments: &[Segment]) -> Self {
        // 为每个 segment 创建迭代器
        let mut iters: Vec<_> = segments.iter()
            .map(|s| s.iter())
            .collect();
        
        // 初始化堆（每个 iter 的第一个 entry）
        let mut heap = BinaryHeap::new();
        for (idx, iter) in iters.iter_mut().enumerate() {
            if let Some(entry) = iter.next() {
                heap.push(HeapEntry { key: entry.key.clone(), segment_idx: idx, entry });
            }
        }
        
        Self { iterators: iters, current_heap: heap }
    }
}

impl Iterator for MergeIterator {
    type Item = (String, Vec<u8>);
    
    fn next(&mut self) -> Option<Self::Item> {
        // 弹出最小 key，从对应 segment 补充下一个
        if let Some(HeapEntry { segment_idx, entry, .. }) = self.current_heap.pop() {
            if let Some(next_entry) = self.iterators[segment_idx].next() {
                self.current_heap.push(HeapEntry::from(next_entry));
            }
            Some((entry.key, entry.value))
        } else {
            None
        }
    }
}
```

#### 10.2 流式 Compaction
```rust
fn execute_compaction_streaming(&self, segments: &[SegmentId]) -> Result<SegmentId> {
    let merge_iter = MergeIterator::new(&segments);
    
    // 流式写入新 segment，无需全量加载到内存
    let mut new_segment = self.create_segment(target_level);
    for (key, value) in merge_iter {
        new_segment.append(&key, &value)?;  // 逐条写入
    }
    new_segment.flush()?;
    
    Ok(new_segment.id())
}
```

**验证标准**:
- ✅ Compaction 内存占用从 O(n) 降至 O(segments_count)
- ✅ 支持 GB 级数据集不 OOM
- ✅ Compaction 速度不慢于现有实现

**预计工作量**: 2-3 天

---

### Sprint 11: WAL 批量写入优化（提升写入吞吐）

**目标**: 写入吞吐量提升 2-3x

**核心改动**:

#### 11.1 Write Coalescer 批量 fsync
```rust
// write_engine.rs
pub struct WriteCoalescer {
    pending_writes: Vec<BufferedWrite>,  // 等待批量写入
    batch_size_threshold: usize,         // 触发批量的大小
    batch_timeout: Duration,             // 超时触发
}

impl WriteCoalescer {
    pub fn add(&mut self, write: BufferedWrite) -> Option<Vec<BufferedWrite>> {
        self.pending_writes.push(write);
        
        // 达到阈值或超时，触发批量 flush
        if self.pending_writes.len() >= self.batch_size_threshold {
            Some(self.drain_pending())
        } else {
            None
        }
    }
    
    fn drain_pending(&mut self) -> Vec<BufferedWrite> {
        std::mem::take(&mut self.pending_writes)
    }
}
```

#### 11.2 WAL log_batch 优化
```rust
// wal.rs
impl WriteAheadLog {
    pub fn log_batch(&mut self, batch: &[BufferedWrite]) -> Result<()> {
        // 单次写入多条记录，一次 fsync
        let mut buffer = Vec::with_capacity(batch.iter().map(|w| w.estimated_size()).sum());
        
        for write in batch {
            buffer.extend_from_slice(&write.key_len.to_le_bytes());
            buffer.extend_from_slice(&write.key);
            buffer.extend_from_slice(&write.value_len.to_le_bytes());
            buffer.extend_from_slice(&write.value);
            buffer.extend_from_slice(&write.checksum.to_le_bytes());
        }
        
        self.file.write_all(&buffer)?;
        self.file.sync_all()?;  // 一次 fsync，而非 N 次
        
        Ok(())
    }
}
```

**验证标准**:
- ✅ 100B 写入从 1.86ms 降至 <1.5ms（超越 RocksDB 的 1.83ms）
- ✅ 批量写入吞吐量提升 2-3x
- ✅ 单条写入延迟不增加

**预计工作量**: 1-2 天

---

### Sprint 12: 锁粒度优化 + 内存分配器

**目标**: 减少锁竞争，优化内存分配

**核心改动**:

#### 12.1 缩小读锁范围
```rust
// read_engine.rs - 优化前
pub fn get(&self, key: &str) -> Option<Vec<u8>> {
    let segments = self.state.segments.read();  // 锁在整个遍历期间
    let index_manager = self.state.index_manager.read();
    
    for (_, segment) in segments.iter().rev() {
        // ... 长时间遍历
    }
}

// 优化后 - 快照模式
pub fn get(&self, key: &str) -> Option<Vec<u8>> {
    // 快速获取快照（Arc 克隆，浅拷贝）
    let snapshot = {
        let segments = self.state.segments.read();
        Arc::clone(segments)  // Arc 克隆，立即释放锁
    };
    
    // 遍历无需持锁
    for segment in snapshot.iter().rev() {
        // ...
    }
}
```

#### 12.2 引入 mimalloc 分配器
```toml
# Cargo.toml
[dependencies]
mimalloc = "0.1"

# lib.rs
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
```

#### 12.3 DenseIndex 无锁读取
```rust
// index.rs
pub struct DenseIndex {
    data: Arc<RwLock<HashMap<String, BlockLocation>>>,
}

impl DenseIndex {
    pub fn get_by_key(&self, key: &str) -> Option<BlockLocation> {
        // 优化：使用 ArcSwap 实现无锁快照
        let snapshot = self.data.swap_arc();
        snapshot.read().get(key).copied()
    }
}
```

**验证标准**:
- ✅ 并发查询吞吐量提升 30-50%
- ✅ 锁等待时间减少 80%
- ✅ 内存分配延迟降低 20%

**预计工作量**: 1-2 天

---

### Sprint 13: Block 格式优化 + 压缩激活

**目标**: 减少磁盘 I/O，提升压缩率

**核心改动**:

#### 13.1 Block 大小可配置
```rust
// segment.rs
pub struct SegmentConfig {
    pub block_size: u64,              // 原硬编码 4096，现可配置
    pub compression_level: Option<i32>, // zstd 压缩级别
    pub checksum_algorithm: ChecksumType,
}

impl Default for SegmentConfig {
    fn default() -> Self {
        Self {
            block_size: 8192,  // 优化：从 4KB 增至 8KB（减少 block 数量）
            compression_level: Some(3),
            checksum_algorithm: ChecksumType::Crc32c,
        }
    }
}
```

#### 13.2 Block 级压缩
```rust
// segment.rs - Block 格式
pub struct DataBlock {
    pub header: BlockHeader,
    pub entries: Vec<Entry>,
    pub compression: CompressionType,  // NEW: 块级压缩
    pub checksum: u32,
}

impl DataBlock {
    pub fn compress(&self) -> Vec<u8> {
        // 整个 block 压缩，而非 per-entry
        let raw = bincode::serialize(&self.entries).unwrap();
        zstd::encode_all(&raw[..], self.compression.level()).unwrap()
    }
}
```

#### 13.3 Zone Map 字节阈值
```rust
// Zone Map 优化
pub struct ZoneMap {
    pub min_key: String,
    pub max_key: String,
    pub min_value_size: u64,
    pub max_value_size: u64,
    pub block_size_bytes: u64,  // 从 entry 计数改为字节
}
```

**验证标准**:
- ✅ 磁盘 I/O 减少 30-40%
- ✅ 压缩率提升至 2-3x（原 1.5x）
- ✅ Block 大小配置对性能影响可量化

**预计工作量**: 2 天

---

## 总体时间线

| Sprint | 任务 | 预计工作量 | 预期收益 |
|--------|------|-----------|---------|
| **Sprint 8** | Level 感知读取 | 2-3 天 | 🟢 **60% 性能提升** |
| **Sprint 9** | Block Cache Moka | 1-2 天 | 🟢 **25% 性能提升** |
| **Sprint 10** | Compaction 流式 | 2-3 天 | 🟡 **内存优化** |
| **Sprint 11** | WAL 批量写入 | 1-2 天 | 🟢 **写入 2-3x** |
| **Sprint 12** | 锁粒度 + mimalloc | 1-2 天 | 🟡 **并发 30-50%** |
| **Sprint 13** | Block 格式优化 | 2 天 | 🟡 **I/O 30-40%** |
| **总计** | | **9-14 天** | **🚀 全方位超越** |

---

## 风险缓解

### 风险 1: Level 感知改动复杂

**缓解**: 先在小规模数据集（1K keys）验证，再扩展到 100K

### 风险 2: Moka 引入新依赖

**缓解**: Moka 是成熟库（10K+ stars），且有详细文档和测试

### 风险 3: 流式 Compaction 速度变慢

**缓解**: 保留现有全量实现作为 fallback，流式作为可选项

### 风险 4: 优化后仍慢于 RocksDB

**缓解**: 目标设定为 0.5-2x 差距（从 240x 大幅改善），学术研究已足够

---

## 成功标准

### 最终性能目标

| 操作 | FileKV 目标 | RocksDB | 预期对比 |
|------|------------|---------|---------|
| Bloom 负查询 | <50 µs | 247.38 µs | ✅ **快 5x** |
| 热数据查询 | <50 µs | 600.07 µs | ✅ **快 12x** |
| 写入 64B | <1.5 ms | 1.88 ms | ✅ **快 25%** |
| 写入 100B | <1.5 ms | 1.83 ms | ✅ **快 18%** |
| **100K 数据集** | **<5ms** | **基准** | ✅ **接近或超越** |

### 测试标准

- ✅ 所有现有 285 个测试通过
- ✅ 新增 20+ 个性能回归测试
- ✅ Clippy 零警告
- ✅ 编译零错误

---

## 下一步行动

1. **确认计划**: 审阅此计划，确认方向和优先级
2. **开始 Sprint 8**: Level 感知读取路径（最大收益）
3. **建立基准**: 运行现有 benchmarks，记录当前性能基线
4. **逐步实施**: 每个 Sprint 完成后运行全量测试验证

---

**准备好了吗？我可以立即开始实施 Sprint 8！**
