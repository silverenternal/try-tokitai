# 附录: 遗漏 API 章节

本文档补充了主 API 文档中遗漏的模块 API 详情。

---

## §3 核心存储模块 API

### 3.1 MemTable 模块

**源文件**: `/home/hugo/codes/try-tokitai/crates/tokitai-filekv/src/core/memtable.rs`

MemTable 是内存缓冲表，基于 DashMap 实现无锁并发，提供 O(1) 平均时间复杂度的插入/查找。

#### 3.1.1 MemTableConfig

🔒 内部

```rust
pub struct MemTableConfig {
    pub flush_threshold_bytes: usize,
    pub max_entries: usize,
    pub max_memory_bytes: usize,
    pub shards: usize,
    pub enable_async_flush: bool,
    pub max_immutable_memtables: usize,
    pub immutable_flush_threshold_bytes: usize,
}
```

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `flush_threshold_bytes` | `usize` | 4MB | 刷盘阈值（字节） |
| `max_entries` | `usize` | 100,000 | 最大条目数 |
| `max_memory_bytes` | `usize` | 64MB | 最大内存限制，达到时触发背压 |
| `shards` | `usize` | `num_cpus * 4` | DashMap 分片数量（并发度） |
| `enable_async_flush` | `bool` | false | 启用异步 MemTable 刷盘 |
| `max_immutable_memtables` | `usize` | 1 | 最大不可变 MemTable 数量 |
| `immutable_flush_threshold_bytes` | `usize` | 4MB | 不可变 MemTable 刷盘阈值 |

#### 3.1.2 MemTableEntry

🔒 内部

```rust
pub struct MemTableEntry {
    pub value: Option<Bytes>,
    pub pointer: Option<ValuePointer>,
    pub seq_num: u32,
    pub deleted: bool,
}
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `value` | `Option<Bytes>` | 值数据（零拷贝存储） |
| `pointer` | `Option<ValuePointer>` | 值指针（已刷盘后设置） |
| `seq_num` | `u32` | 序列号（并发控制） |
| `deleted` | `bool` | 删除标记（tombstone） |

#### 3.1.3 MemTable

✅ 稳定

**核心方法**:

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new(config: MemTableConfig) -> Self` | 创建 MemTable 实例 | ✅ |
| `with_memory_tracker` | `fn with_memory_tracker(config: MemTableConfig, tracker: Option<Arc<MemoryTracker>>) -> Self` | 创建带内存追踪的 MemTable | 🔒 |
| `insert` | `fn insert(&self, key: String, value: &[u8]) -> (usize, u32)` | 插入键值对，返回 (当前大小, 序列号) | ✅ |
| `insert_batch` | `fn insert_batch(&self, entries: &[(String, Vec<u8>)]) -> (usize, u32)` | 批量插入（分片分组优化） | ✅ |
| `delete` | `fn delete(&self, key: &str) -> Option<u32>` | 标记删除（tombstone） | ✅ |
| `insert_tombstone` | `fn insert_tombstone(&self, key: String) -> (usize, u32)` | 插入 tombstone 条目 | 🔒 |
| `get` | `fn get(&self, key: &str) -> Option<(Option<Bytes>, Option<ValuePointer>, bool)>` | 获取值，返回 (value, pointer, deleted) | ✅ |
| `iter` | `fn iter(&self) -> impl Iterator<Item = RefMulti<...>> + '_` | 遍历所有条目 | ⚠️ |
| `should_flush` | `fn should_flush(&self) -> bool` | 检查是否需要刷盘 | ✅ |
| `should_apply_backpressure` | `fn should_apply_backpressure(&self) -> bool` | 检查是否应施加背压 | ✅ |
| `memory_usage_ratio` | `fn memory_usage_ratio(&self) -> f64` | 获取内存使用率 (0.0-1.0+) | ✅ |
| `memory_headroom` | `fn memory_headroom(&self) -> usize` | 获取可用内存余量（字节） | ✅ |
| `backpressure_level` | `fn backpressure_level(&self) -> f64` | 获取背压等级 (0.0-1.0+) | ✅ |
| `backpressure_error` | `fn backpressure_error(&self) -> Option<TransientError>` | 获取背压错误（如活跃） | ✅ |
| `size_bytes` | `fn size_bytes(&self) -> usize` | 获取当前大小 | ✅ |
| `approximate_memory_bytes` | `fn approximate_memory_bytes(&self) -> u64` | 获取估算内存占用 | ✅ |
| `entry_count` | `fn entry_count(&self) -> usize` | 获取条目数 | ✅ |
| `clear` | `fn clear(&self)` | 清空 MemTable | ✅ |
| `get_entries` | `fn get_entries(&self) -> Vec<(String, MemTableEntry)>` | 获取所有条目（用于刷盘） | 🔒 |
| `entries_sorted` | `fn entries_sorted(&self) -> Vec<(String, MemTableEntry)>` | 获取按 key 排序的条目 | 🔒 |
| `update_pointer` | `fn update_pointer(&self, key: &str, pointer: ValuePointer) -> bool` | 更新条目的 pointer | 🔒 |
| `min_seq_num` | `fn min_seq_num(&self) -> Option<u32>` | 获取最小序列号 | 🔒 |

**使用示例**:

```rust
use tokitai_filekv::core::memtable::{MemTable, MemTableConfig};

let config = MemTableConfig {
    flush_threshold_bytes: 4 * 1024 * 1024,  // 4MB
    max_entries: 100_000,
    max_memory_bytes: 64 * 1024 * 1024,       // 64MB
    shards: num_cpus::get() * 4,
    ..Default::default()
};

let mt = MemTable::new(config);

// 插入
let (size, seq) = mt.insert("user:123".to_string(), b"John Doe");

// 查询
if let Some((value, pointer, deleted)) = mt.get("user:123") {
    if !deleted {
        if let Some(v) = value {
            println!("Found: {:?}", v);
        }
    }
}

// 检查是否需要刷盘
if mt.should_flush() {
    let entries = mt.entries_sorted();
    // flush entries to segment file...
    mt.clear();
}
```

---

### 3.2 SegmentFile 模块

**源文件**: `/home/hugo/codes/try-tokitai/crates/tokitai-filekv/src/core/segment.rs`

Segment 文件是 LSM-Tree 中的持久化数据段文件，使用顺序写入格式。

#### 3.2.1 常量

| 常量 | 值 | 说明 |
|------|-----|------|
| `SEGMENT_MAGIC` | `0x54435347` ("TCSG") | Segment 文件魔数 |
| `SEGMENT_VERSION` | `1` | Segment 文件版本 |
| `BLOCK_HEADER_MAGIC` | `0x424C4B48` ("BLKH") | 压缩块头魔数 |
| `BLOCK_HEADER_VERSION` | `2` | 块头版本（含算法 ID） |
| `BLOCK_HEADER_SIZE` | `22` | 块头大小（字节） |
| `OPT009_BLOCK_HEADER_MAGIC` | `0x4F505432` ("OPT2") | OPT-009 V2 块头魔数 |
| `OPT009_TAIL_INDEX_MAGIC` | `0x494E4458` ("INDX") | OPT-009 尾索引魔数 |

#### 3.2.2 BlockHeader

🔒 内部

```rust
pub struct BlockHeader {
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub checksum: u32,
    pub is_compressed: bool,
    pub algorithm_id: u8,
}
```

| 方法 | 签名 | 说明 |
|------|------|------|
| `to_bytes` | `fn to_bytes(&self) -> [u8; BLOCK_HEADER_SIZE as usize]` | 序列化为字节 |
| `from_bytes` | `fn from_bytes(buf: &[u8; 22]) -> Result<Self, FatalError>` | 从字节反序列化 |
| `from_bytes_v1` | `fn from_bytes_v1(buf: &[u8; 21]) -> Result<Self, FatalError>` | 从 V1 格式反序列化（兼容旧文件） |

#### 3.2.3 Opt009BlockHeader

⚠️ 实验

```rust
pub struct Opt009BlockHeader {
    pub min_key: String,
    pub max_key: String,
    pub entry_count: u16,
    pub block_offset: u64,
    pub bloom_filter: Option<Vec<u8>>,
}
```

| 方法 | 签名 | 说明 |
|------|------|------|
| `to_bytes` | `fn to_bytes(&self) -> Vec<u8>` | 序列化 |
| `from_bytes` | `fn from_bytes(data: &[u8], offset: &mut usize) -> Result<Self, FatalError>` | 反序列化 |
| `key_might_exist` | `fn key_might_exist(&self, key: &str) -> bool` | 检查 key 是否可能在块中 |

#### 3.2.4 SegmentFile

✅ 稳定

```rust
pub struct SegmentFile {
    pub id: u64,
    pub level: u8,
    pub min_key: parking_lot::Mutex<Option<String>>,
    pub max_key: parking_lot::Mutex<Option<String>>,
    pub path: PathBuf,
    // 私有字段: fs, mmap_fs, write_file, size, entry_count, mmap, ...
}
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | `u64` | 段文件唯一 ID |
| `level` | `u8` | Compaction 等级 (L0=memtable 刷盘, L1+=compacted) |
| `min_key` | `Mutex<Option<String>>` | 段中最小 key |
| `max_key` | `Mutex<Option<String>>` | 段中最大 key |
| `path` | `PathBuf` | 文件路径 |

**核心方法**:

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `create` | `fn create(fs, id, level, path, preallocate_size, use_persistent_mmap, readahead_multiplier, dense_index_enabled) -> Result<Self, FatalError>` | 创建新段文件 | ✅ |
| `open` | `fn open(fs, id, level, path, use_persistent_mmap, readahead_multiplier, dense_index_enabled) -> Result<Self, FatalError>` | 打开现有段文件 | ✅ |
| `append` | `fn append(&self, key: &str, value: &[u8]) -> Result<u64, FatalError>` | 追加写入条目 | ✅ |
| `read_entry` | `fn read_entry(&self, offset: u64) -> Result<(String, Vec<u8>, u32), FatalError>` | 从指定偏移读取条目 | ✅ |
| `scan_next` | `fn scan_next(&self, offset: u64, start_key: &str) -> ScanResult` | 从指定位置扫描下一个条目 | ⚠️ |
| `read_segment_data` | `fn read_segment_data(&self) -> Result<Vec<u8>, FatalError>` | 读取整个段数据到内存 | 🔒 |
| `flush` | `fn flush(&self) -> Result<(), FatalError>` | 刷新段文件 | ✅ |
| `size` | `fn size(&self) -> u64` | 获取文件大小 | ✅ |
| `entry_count` | `fn entry_count(&self) -> u64` | 获取条目数 | ✅ |
| `sync_all` | `fn sync_all(&self) -> Result<(), FatalError>` | 同步到磁盘 | ✅ |
| `remove` | `fn remove(&self) -> Result<(), FatalError>` | 删除段文件 | ✅ |

**使用示例**:

```rust
use tokitai_filekv::core::segment::SegmentFile;
use tokitai_filekv::io::StdFs;
use std::sync::Arc;
use std::path::Path;

let fs = Arc::new(StdFs);
let path = Path::new("/data/segments/segment_1.log");

// 创建新段
let segment = SegmentFile::create(
    fs.clone(),
    1,           // id
    0,           // level (L0)
    path,
    0,           // 不预分配
    true,        // 使用持久 mmap
    0,           // 禁用预读
    false,       // 禁用密集索引
)?;

// 写入
let offset = segment.append("user:123", b"John Doe")?;

// 读取
let (key, value, checksum) = segment.read_entry(offset)?;

// 刷盘
segment.sync_all()?;
```

#### 3.2.5 SegmentStats

🔒 内部

段统计信息，包含在段文件内部追踪中。

---

### 3.3 SparseIndex / IndexManager

**源文件**: `/home/hugo/codes/try-tokitai/crates/tokitai-filekv/src/core/sparse_index.rs`

稀疏索引用于高效的段查找，支持 O(1) 点查和范围查询。

#### 3.3.1 SparseIndexEntry

🔒 内部

```rust
pub struct SparseIndexEntry {
    pub key: String,
    pub offset: u64,
    pub seq_num: u64,
}
```

#### 3.3.2 SparseIndex

✅ 稳定

```rust
pub struct SparseIndex {
    pub entries: Vec<SparseIndexEntry>,
    pub segment_id: u64,
    pub zone_map: Arc<Vec<ZoneMapEntry>>,
    // 私有: key_map: AHashMap<String, u64>
}
```

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new(segment_id: u64) -> Self` | 创建空索引 | ✅ |
| `add` | `fn add(&mut self, key: String, offset: u64, seq_num: u64)` | 添加索引条目 | ✅ |
| `build_key_map` | `fn build_key_map(&mut self)` | 从 entries 构建 key_map | 🔒 |
| `find` | `fn find(&self, key: &str) -> Option<u64>` | O(1) 点查找，返回 offset | ✅ |
| `key_might_exist` | `fn key_might_exist(&self, key: &str) -> bool` | 检查 key 是否可能在段中 | ✅ |
| `save` | `fn save(&self, path: &Path) -> Result<()>` | 保存到磁盘（JSON） | ✅ |
| `load` | `fn load(path: &Path) -> Result<Self>` | 从磁盘加载 | ✅ |

#### 3.3.3 DenseIndex / DenseIndexEntry

⚠️ 实验

```rust
pub struct DenseIndex {
    pub entries: AHashMap<String, DenseIndexEntry>,
    pub block_size: u64,
}

pub struct DenseIndexEntry {
    pub offset: u64,
    pub key_len: u32,
    pub value_len: u32,
    pub checksum: u32,
    pub seq_num: u64,
    pub block_id: u64,
}
```

| 方法 | 说明 |
|------|------|
| `with_block_size` | 创建带 block_size 配置的 DenseIndex |
| `block_size` | 获取配置的 block_size |
| `offset_to_block_id` | 从偏移计算 block_id |

#### 3.3.4 IndexManager

✅ 稳定

```rust
pub struct IndexManager {
    // 私有: index_dir, indexes: BTreeMap<u64, Arc<SparseIndex>>, dense_indexes: BTreeMap<u64, DenseIndex>
}
```

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new<P: AsRef<Path>>(index_dir: P) -> Result<Self>` | 创建索引管理器 | ✅ |
| `add_index` | `fn add_index(&mut self, segment_id: u64, index: Arc<SparseIndex>)` | 添加稀疏索引 | ✅ |
| `add_dense_index` | `fn add_dense_index(&mut self, segment_id: u64, index: DenseIndex)` | 添加密集索引 | ✅ |
| `get_index` | `fn get_index(&self, segment_id: u64) -> Option<Arc<SparseIndex>>` | 获取稀疏索引 | ✅ |
| `load_all_indexes` | `fn load_all_indexes(&mut self) -> Result<()>` | 加载所有 .idx 文件 | ✅ |
| `save_index` | `fn save_index(&self, segment_id: u64) -> Result<()>` | 保存指定索引 | ✅ |
| `all_indexes` | `fn all_indexes(&self) -> &BTreeMap<u64, Arc<SparseIndex>>` | 获取所有稀疏索引 | 🔒 |
| `all_dense_indexes` | `fn all_dense_indexes(&self) -> &BTreeMap<u64, DenseIndex>` | 获取所有密集索引 | 🔒 |
| `get_zone_map` | `fn get_zone_map(&self, segment_id: u64) -> Option<ZoneMapIndex>` | 获取区域地图索引 | ✅ |
| `update_zone_map` | `fn update_zone_map(&mut self, segment_id: u64, zone_map: Arc<Vec<ZoneMapEntry>>) -> Result<()>` | 更新区域地图 | ✅ |

#### 3.3.5 SparseIndexConfig

🔒 内部

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `sparse_index_interval` | `usize` | 100 | 稀疏索引采样间隔 |

---

### 3.4 WriteCoalescer / WriteBuffer

**源文件**: `/home/hugo/codes/try-tokitai/crates/tokitai-filekv/src/core/write_coalescer.rs`

写入缓冲器，批量 WAL 写入，减少 fsync 次数。

#### 3.4.1 WriteBufferConfig (= WriteCoalescerConfig)

✅ 稳定

```rust
pub struct WriteBufferConfig {
    pub time_window_us: u64,
    pub size_threshold_bytes: usize,
}
```

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `time_window_us` | `u64` | 100,000 (100ms) | 时间窗口，窗口内的写入会被合并 |
| `size_threshold_bytes` | `usize` | 64KB | 大小阈值，达到此大小立即刷盘 |

#### 3.4.2 BufferedWrite (= PendingWrite)

🔒 内部

```rust
pub struct BufferedWrite {
    pub key: String,
    pub value: Vec<u8>,
    pub timestamp: Instant,
}
```

#### 3.4.3 WriteBuffer (= WriteCoalescer)

✅ 稳定

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new(config: WriteBufferConfig) -> Self` | 创建写入缓冲器 | ✅ |
| `add` | `fn add(&self, key: String, value: Vec<u8>) -> Option<Vec<BufferedWrite>>` | 添加写入，返回 Some(batch) 表示应 flush | ✅ |
| `force_flush` | `fn force_flush(&self) -> Vec<BufferedWrite>` | 强制 flush 所有待写入 | ✅ |
| `has_pending` | `fn has_pending(&self) -> bool` | 检查是否有待处理写入 | ✅ |
| `pending_count` | `fn pending_count(&self) -> usize` | 获取待写入数量 | ✅ |
| `buffer_size` | `fn buffer_size(&self) -> usize` | 获取当前缓冲大小（字节） | ✅ |

**使用示例**:

```rust
use tokitai_filekv::core::write_coalescer::{WriteBuffer, WriteBufferConfig};

let config = WriteBufferConfig {
    time_window_us: 100_000,       // 100ms
    size_threshold_bytes: 64 * 1024, // 64KB
};
let buffer = WriteBuffer::new(config);

// 添加写入（不会立即触发 flush）
buffer.add("key1".to_string(), b"value1".to_vec());
buffer.add("key2".to_string(), b"value2".to_vec());

// 检查是否有待处理
if buffer.has_pending() {
    // 强制 flush
    let batch = buffer.force_flush();
    // batch 包含所有 BufferedWrite
}
```

---

### 3.5 FlushTrigger

**源文件**: `/home/hugo/codes/try-tokitai/crates/tokitai-filekv/src/core/flush.rs`

后台刷盘线程触发器，定期检查 MemTable 是否需要刷盘。

#### 3.5.1 FlushMessage

🔒 内部

```rust
pub enum FlushMessage {
    Trigger, // 触发立即刷盘
    Stop,    // 停止后台线程
}
```

#### 3.5.2 FlushTrigger

✅ 稳定

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new() -> Self` | 创建 FlushTrigger（无后台线程） | ✅ |
| `with_background_thread` | `fn with_background_thread(interval_ms: u64, memtable: Arc<MemTable>) -> Self` | 创建带后台线程的 FlushTrigger | ✅ |
| `is_requested` | `fn is_requested(&self) -> bool` | 检查是否请求了刷盘 | ✅ |
| `mark_completed` | `fn mark_completed(&self)` | 标记刷盘已完成 | ✅ |
| `request` | `fn request(&self)` | 请求一次刷盘 | ✅ |
| `send_trigger` | `fn send_trigger(&self) -> bool` | 发送刷盘消息（如有后台线程） | ✅ |
| `stop` | `fn stop(&self)` | 停止后台线程 | ✅ |

**使用示例**:

```rust
use tokitai_filekv::core::flush::FlushTrigger;
use tokitai_filekv::core::memtable::MemTable;
use std::sync::Arc;

let memtable = Arc::new(MemTable::new(Default::default()));

// 创建带后台线程的触发器（每 1000ms 检查一次）
let trigger = FlushTrigger::with_background_thread(1000, memtable.clone());

// 检查是否需要刷盘
if trigger.is_requested() {
    // 执行刷盘...
    trigger.mark_completed();
}

// 完成后停止
trigger.stop();
```

---

## §5 缓存 API 补充

**源文件目录**: `/home/hugo/codes/try-tokitai/crates/tokitai-filekv/src/cache/`

### 5.1 UnifiedCacheConfig

✅ 稳定

```rust
pub struct UnifiedCacheConfig {
    pub max_total_memory_bytes: u64,
    pub block_cache_ratio: f64,
    pub bloom_cache_ratio: f64,
    pub block_cache_config: Option<BlockCacheConfig>,
    pub bloom_cache_config: Option<BloomFilterCacheConfig>,
    pub bloom_index_dir: PathBuf,
    pub l2_cache_config: Option<L2CacheConfig>,
    pub enable_multi_level_cache: bool,
}
```

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `max_total_memory_bytes` | `u64` | 128MB | 总内存预算（信息性） |
| `block_cache_ratio` | `f64` | 0.60 | BlockCache 预算比例 |
| `bloom_cache_ratio` | `f64` | 0.25 | BloomFilterCache 预算比例 |
| `block_cache_config` | `Option<BlockCacheConfig>` | None | BlockCache 配置 |
| `bloom_cache_config` | `Option<BloomFilterCacheConfig>` | None | BloomCache 配置 |
| `bloom_index_dir` | `PathBuf` | `"bloom"` | Bloom 过滤器存储目录 |
| `l2_cache_config` | `Option<L2CacheConfig>` | None | L2 缓存配置 |
| `enable_multi_level_cache` | `bool` | true | 启用多级缓存 |

**说明**: 剩余 15% 预算未分配。各缓存通过自身的 `max_memory_bytes`/`max_items` 配置强制内存限制，预算框架仅用于报告和再平衡。

### 5.2 CacheBudget / SubBudget / CacheUsageReport

**源文件**: `/home/hugo/codes/try-tokitai/crates/tokitai-filekv/src/cache/budget.rs`

#### 5.2.1 SubBudget

🔒 内部

| 方法 | 签名 | 说明 |
|------|------|------|
| `max_budget` | `fn max_budget(&self) -> u64` | 获取最大预算 |

#### 5.2.2 CacheUsageReport

✅ 稳定

```rust
pub struct CacheUsageReport {
    pub total_budget: u64,
    pub total_used: u64,
    pub usage_percent: f64,
    pub block_cache_used: u64,
    pub block_cache_max: u64,
    pub block_cache_hit_rate: f64,
    pub bloom_filter_used: u64,
    pub bloom_filter_max: u64,
    pub bloom_filter_hit_rate: f64,
}
```

实现了 `Display` trait，可格式化输出缓存使用情况。

#### 5.2.3 CacheBudget

🔒 内部

| 方法 | 签名 | 说明 |
|------|------|------|
| `new` | `fn new(max_bytes: u64, block_pct: f64, bloom_pct: f64) -> Self` | 创建预算对象 |

### 5.3 L2CacheManager / L2CacheConfig / L2CacheStats

**源文件**: `/home/hugo/codes/try-tokitai/crates/tokitai-filekv/src/cache/l2_cache.rs`

#### 5.3.1 L2CacheConfig

✅ 稳定

```rust
pub struct L2CacheConfig {
    pub max_bytes: u64,
    pub cache_dir: PathBuf,
    pub l2_to_l1_threshold: u32,
}
```

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `max_bytes` | `u64` | 4GB | L2 缓存最大大小 |
| `cache_dir` | `PathBuf` | `"cache_l2"` | L2 缓存文件目录 |
| `l2_to_l1_threshold` | `u32` | 5 | L2 提升到 L1 的访问次数阈值 |

#### 5.3.2 L2CacheStats

✅ 稳定

```rust
pub struct L2CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub inserts: u64,
    pub evictions: u64,
    pub promotions: u64,
    pub demotions: u64,
    pub entry_count: u64,
    pub used_bytes: u64,
    pub max_bytes: u64,
}
```

| 方法 | 说明 |
|------|------|
| `hit_rate` | 获取命中率 (f64) |

#### 5.3.3 L2CacheManager

✅ 稳定

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new(config: L2CacheConfig) -> std::io::Result<Self>` | 创建 L2 缓存管理器 | ✅ |
| `get` | `fn get(&self, key: &str) -> Option<Bytes>` | 从 L2 获取值 | ✅ |
| `insert` | `fn insert(&self, key: &str, value: Bytes)` | 插入键值对到 L2 | ✅ |
| `contains` | `fn contains(&self, key: &str) -> bool` | 检查 key 是否存在 | ✅ |
| `remove` | `fn remove(&self, key: &str)` | 从 L2 移除键 | ✅ |
| `get_access_count` | `fn get_access_count(&self, key: &str) -> Option<u32>` | 获取访问次数（用于提升决策） | ✅ |
| `should_promote` | `fn should_promote(&self, key: &str) -> bool` | 检查是否应提升到 L1 | ✅ |
| `stats` | `fn stats(&self) -> L2CacheStats` | 获取统计信息 | ✅ |
| `record_promotion` | `fn record_promotion(&self)` | 记录一次 L2→L1 提升 | 🔒 |
| `record_demotion` | `fn record_demotion(&self)` | 记录一次 L1→L2 降级 | 🔒 |
| `flush` | `fn flush(&self) -> std::io::Result<()>` | 刷盘 L2 缓存 | ✅ |
| `get_used_bytes` | `fn get_used_bytes(&self) -> u64` | 获取已用字节数 | 🔒 |
| `index_memory_usage` | `fn index_memory_usage(&self) -> usize` | 获取内存索引占用 | 🔒 |

### 5.4 Rebalance 相关类型

**源文件**: `/home/hugo/codes/try-tokitai/crates/tokitai-filekv/src/cache/rebalance.rs`

#### 5.4.1 RebalanceConfig

✅ 稳定

```rust
pub struct RebalanceConfig {
    pub interval: Duration,
    pub low_hit_rate_threshold: f64,
    pub high_hit_rate_threshold: f64,
    pub min_hit_rate_gap: f64,
    pub max_transfer_ratio: f64,
    pub min_budget_bytes: u64,
    pub max_budget_bytes: u64,
    pub min_access_samples: u64,
}
```

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `interval` | `Duration` | 30s | 再平衡线程运行间隔 |
| `low_hit_rate_threshold` | `f64` | 0.3 | 命中率低于此值可缩小 |
| `high_hit_rate_threshold` | `f64` | 0.8 | 命中率高于此值可扩大 |
| `min_hit_rate_gap` | `f64` | 0.2 | 最小命中率差距 |
| `max_transfer_ratio` | `f64` | 0.1 | 每次循环最大转移比例 (10%) |
| `min_budget_bytes` | `u64` | 1MB | 最小预算（防止饥饿） |
| `max_budget_bytes` | `u64` | 256MB | 最大预算（防止单缓存占满） |
| `min_access_samples` | `u64` | 100 | 最小采样数（防止过早决策） |

#### 5.4.2 RebalanceDecision

🔒 内部

```rust
pub enum RebalanceDecision {
    ShrinkBlock(u64),
    GrowBlock(u64),
    ShrinkBloom(u64),
    GrowBloom(u64),
}
```

| 方法 | 说明 |
|------|------|
| `evaluate` | 评估再平衡决策，返回 Vec<RebalanceDecision> |

#### 5.4.3 RebalanceStats

✅ 稳定

```rust
pub struct RebalanceStats {
    pub block_hit_rate: f64,
    pub bloom_hit_rate: f64,
    pub block_memory_bytes: u64,
    pub bloom_memory_bytes: u64,
    pub decisions: Vec<RebalanceDecision>,
    pub status: RebalanceStatus,
}
```

| 方法 | 说明 |
|------|------|
| `disabled` | 创建禁用状态的 stats |
| `skipped` | 创建跳过状态的 stats |
| `completed` | 创建完成状态的 stats |
| `total_bytes_transferred` | 获取转移的总字节数 |
| `had_action` | 检查是否有动作发生 |

#### 5.4.4 RebalanceStatus

🔒 内部

```rust
pub enum RebalanceStatus {
    Disabled,
    SkippedInsufficientSamples,
    Completed,
}
```

### 5.5 BlockCacheAsPrefetchCache

**源文件**: `/home/hugo/codes/try-tokitai/crates/tokitai-filekv/src/cache/block_cache.rs`

⚠️ 实验

```rust
pub struct BlockCacheAsPrefetchCache {
    block_cache: Arc<BlockCache>,
    block_reader: Box<dyn Fn(u64, u64, u64) -> Option<Bytes> + Send + Sync>,
    block_size: u64,
}
```

| 方法 | 签名 | 说明 |
|------|------|------|
| `new` | `fn new(block_cache, block_size, block_reader) -> Self` | 创建预取适配器 |

实现了 `PrefetchCache` trait，提供 `prefetch`, `contains`, `get` 方法。

### 5.6 CacheWarmer / CacheWarmingConfig / CacheWarmingStats / WarmingStrategy

**源文件**: `/home/hugo/codes/try-tokitai/crates/tokitai-filekv/src/cache/warmup.rs`

#### 5.6.1 CacheWarmingConfig

✅ 稳定

```rust
pub struct CacheWarmingConfig {
    pub enabled: bool,
    pub max_entries: usize,
    pub max_memory_bytes: usize,
    pub min_entry_size: usize,
    pub max_entry_size: usize,
    pub strategy: WarmingStrategy,
    pub recent_entries_per_segment: usize,
    pub size_weight: f64,
    pub recency_weight: f64,
    pub density_weight: f64,
}
```

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `enabled` | `bool` | true | 启用缓存预热 |
| `max_entries` | `usize` | 1,000 | 最大预热条目数 |
| `max_memory_bytes` | `usize` | 16MB | 预热最大内存 |
| `min_entry_size` | `usize` | 64B | 最小条目大小（过滤小条目） |
| `max_entry_size` | `usize` | 64KB | 最大条目大小（过滤大条目） |
| `strategy` | `WarmingStrategy` | Hybrid | 预热策略 |
| `recent_entries_per_segment` | `usize` | 50 | 每段最近条目数 |
| `size_weight` | `f64` | 0.3 | 混合策略大小权重 |
| `recency_weight` | `f64` | 0.4 | 混合策略新旧权重 |
| `density_weight` | `f64` | 0.3 | 混合策略密度权重 |

#### 5.6.2 WarmingStrategy

✅ 稳定

```rust
pub enum WarmingStrategy {
    Recent,     // 加载最近写入的条目
    Frequent,   // 加载高密度段的条目
    SizeBased,  // 加载最优大小范围的条目
    Hybrid,     // 所有策略的组合
}
```

#### 5.6.3 CacheWarmingStats

✅ 稳定

```rust
pub struct CacheWarmingStats {
    pub segments_analyzed: usize,
    pub entries_scanned: usize,
    pub entries_loaded: usize,
    pub entries_skipped: usize,
    pub memory_used: usize,
    pub warming_time_ms: u64,
    pub completed: bool,
}
```

| 方法 | 说明 |
|------|------|
| `memory_used_kb` | 获取已用内存 (KB) |
| `memory_used_mb` | 获取已用内存 (MB) |
| `entries_per_mb` | 获取每 MB 条目数 |
| `skip_rate` | 获取跳过率 |

#### 5.6.4 CacheWarmer

✅ 稳定

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new(config: CacheWarmingConfig, cache: Arc<BlockCache>) -> Self` | 创建缓存预热器 | ✅ |
| `warm` | `fn warm(&self, segments: &[Arc<SegmentFile>]) -> FileKVResult<CacheWarmingStats>` | 从段文件预热缓存 | ✅ |
| `stats` | `fn stats(&self) -> CacheWarmingStats` | 获取预热统计 | ✅ |

**使用示例**:

```rust
use tokitai_filekv::cache::{CacheWarmer, CacheWarmingConfig, BlockCache, BlockCacheConfig};
use std::sync::Arc;

let config = CacheWarmingConfig {
    enabled: true,
    max_entries: 1000,
    max_memory_bytes: 16 * 1024 * 1024,
    strategy: WarmingStrategy::Hybrid,
    ..Default::default()
};

let cache = Arc::new(BlockCache::new(BlockCacheConfig::default()));
let warmer = CacheWarmer::new(config, cache.clone());

// 预热（需要提供段文件列表）
let stats = warmer.warm(&segments)?;
println!("Loaded {} entries, used {:.1}MB",
    stats.entries_loaded, stats.memory_used_mb());
```

### 5.7 SequentialPrefetcher 模块

**源文件**: `/home/hugo/codes/try-tokitai/crates/tokitai-filekv/src/cache/prefetch.rs`

#### 5.7.1 SequentialPrefetcherConfig

✅ 稳定

```rust
pub struct SequentialPrefetcherConfig {
    pub enabled: bool,
    pub sequential_threshold: u32,
    pub prefetch_distance: u32,
    pub max_prefetch_window: u32,
    pub adaptive_distance: bool,
}
```

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `enabled` | `bool` | true | 启用预取 |
| `sequential_threshold` | `u32` | 3 | 触发预取的连续访问次数 |
| `prefetch_distance` | `u32` | 2 | 预取距离（块数） |
| `max_prefetch_window` | `u32` | 10 | 最大预取窗口 |
| `adaptive_distance` | `bool` | true | 启用自适应预取距离 |

#### 5.7.2 SequentialPrefetcherStats

✅ 稳定

```rust
pub struct SequentialPrefetcherStats {
    pub total_prefetches: u64,
    pub successful_prefetches: u64,
    pub wasted_prefetches: u64,
    pub accuracy: f64,
    pub cache_hits_from_prefetch: u64,
}
```

| 方法 | 说明 |
|------|------|
| `accuracy_percent` | 获取预取准确率（百分比） |
| `record_prefetch` | 记录一次预取操作 |

#### 5.7.3 PrefetchCache trait

✅ 稳定

```rust
pub trait PrefetchCache: Send + Sync {
    fn prefetch(&self, segment_id: u64, block_id: u64) -> bool;
    fn contains(&self, segment_id: u64, block_id: u64) -> bool;
    fn get(&self, segment_id: u64, block_id: u64) -> Option<Arc<dyn Send + Sync>>;
}
```

| 方法 | 说明 |
|------|------|
| `prefetch` | 预取块到缓存 |
| `contains` | 检查块是否在缓存中 |
| `get` | 从缓存获取块 |

#### 5.7.4 SequentialPrefetcher<C: PrefetchCache>

✅ 稳定

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new(config: SequentialPrefetcherConfig, cache: Arc<C>) -> Self` | 创建顺序预取器 | ✅ |
| `with_defaults` | `fn with_defaults(cache: Arc<C>) -> Self` | 使用默认配置创建 | ✅ |
| `set_zone_map` | `fn set_zone_map(&mut self, zone_map: Arc<ZoneMapIndex>)` | 设置区域地图索引 | ✅ |
| `record_access` | `fn record_access(&mut self, key: &str, segment_id: u64, block_id: u64) -> bool` | 记录访问并可能触发预取 | ✅ |
| `record_prefetch_hit` | `fn record_prefetch_hit(&mut self, block_id: u64)` | 记录预取命中 | ✅ |
| `record_prefetch_miss` | `fn record_prefetch_miss(&mut self, block_id: u64)` | 记录预取未命中 | ✅ |
| `stats` | `fn stats(&self) -> SequentialPrefetcherStats` | 获取统计信息 | ✅ |
| `reset_stats` | `fn reset_stats(&mut self)` | 重置统计 | ✅ |
| `is_enabled` | `fn is_enabled(&self) -> bool` | 检查是否启用 | ✅ |
| `set_enabled` | `fn set_enabled(&mut self, enabled: bool)` | 启用/禁用预取 | ✅ |
| `reset_detector` | `fn reset_detector(&mut self)` | 重置连续检测器 | ✅ |
| `current_prefetch_distance` | `fn current_prefetch_distance(&self) -> u32` | 获取当前预取距离 | ✅ |

---

## §8 Compaction 系统 API

### 8.1 CompactionConfig

**源文件**: `/home/hugo/codes/try-tokitai/crates/tokitai-filekv/src/compaction/mod.rs`

✅ 稳定

```rust
pub struct CompactionConfig {
    pub min_segments: usize,
    pub auto_compact: bool,
    pub check_interval: usize,
    pub max_segment_size_bytes: u64,
    pub target_segment_size_bytes: u64,
    pub async_compaction_enabled: bool,
    pub leveled_compaction_enabled: bool,
    pub level_size_multiplier: usize,
    pub max_level: u8,
    pub l0_file_count_threshold: usize,
    pub parallel_compaction_enabled: bool,
    pub streaming_compaction_enabled: bool,
    pub write_amplification_threshold: f64,
    pub max_background_compaction_threads: usize,
    pub l0_size_bytes_threshold: u64,
    pub l0_compaction_strategy: CompactionStrategy,
    pub l0_stcs_min_segments: usize,
    pub l0_stcs_size_ratio: f64,
}
```

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `min_segments` | `usize` | 4 | 触发合并的最小段数 |
| `auto_compact` | `bool` | true | 自动合并 |
| `check_interval` | `usize` | 100 | 检查间隔（写操作数） |
| `max_segment_size_bytes` | `u64` | 256MB | 最大段大小 |
| `target_segment_size_bytes` | `u64` | 128MB | 目标段大小 |
| `async_compaction_enabled` | `bool` | true | 启用异步合并 |
| `leveled_compaction_enabled` | `bool` | true | 启用分层合并 |
| `level_size_multiplier` | `usize` | 10 | 等级大小倍数 |
| `max_level` | `u8` | 3 | 最大合并等级 (L3) |
| `l0_file_count_threshold` | `usize` | 3 | L0 文件数触发阈值 |
| `parallel_compaction_enabled` | `bool` | true | 启用并行合并 |
| `streaming_compaction_enabled` | `bool` | true | 启用流式合并 |
| `write_amplification_threshold` | `f64` | 3.0 | 写放大阈值 |
| `max_background_compaction_threads` | `usize` | `min(4, num_cpus/2)` | 最大后台线程数 |
| `l0_size_bytes_threshold` | `u64` | 64MB | L0 总大小触发阈值 |
| `l0_compaction_strategy` | `CompactionStrategy` | Leveled | L0 合并策略 |
| `l0_stcs_min_segments` | `usize` | 3 | STCS 最小段数 |
| `l0_stcs_size_ratio` | `f64` | 2.0 | STCS 大小比例阈值 |

#### 8.1.1 CompactionStrategy

✅ 稳定

```rust
pub enum CompactionStrategy {
    SizeTiered,  // 大小分层策略：合并大小相似的段（适合 L0）
    Leveled,     // 分层策略：合并到有序等级（适合 L1+）
}
```

#### 8.1.2 CompactionStats

✅ 稳定

```rust
pub struct CompactionStats {
    pub compaction_runs: u64,
    pub segments_merged: u64,
    pub bytes_compacted: u64,
    pub entries_removed: u64,
    pub tombstones_cleaned: u64,
    pub bytes_read_from_segments: u64,
    pub bytes_written_to_segment: u64,
}
```

#### 8.1.3 CompactionManager

✅ 稳定

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new(config: CompactionConfig) -> Self` | 创建合并管理器 | ✅ |
| `record_write` | `fn record_write(&self) -> bool` | 记录一次写入，返回是否应运行合并 | ✅ |
| `should_run_compaction` | `fn should_run_compaction(&self) -> bool` | 检查是否应运行合并 | ✅ |
| `reset_write_count` | `fn reset_write_count(&self)` | 重置写入计数器 | 🔒 |
| `stats` | `fn stats(&self) -> CompactionStats` | 获取合并统计 | ✅ |
| `record_compaction` | `fn record_compaction(&self, segments_merged, bytes_compacted, entries_removed, tombstones_cleaned)` | 记录一次合并 | 🔒 |
| `config` | `fn config(&self) -> &CompactionConfig` | 获取配置引用 | ✅ |
| `request_compaction` | `fn request_compaction(&self, segment_count, total_size_bytes) -> bool` | 请求后台合并 | ✅ |
| `request_level_compaction` | `fn request_level_compaction(&self, segment_count, total_size_bytes, target_level) -> bool` | 请求等级特定合并 | ✅ |
| `record_user_bytes` | `fn record_user_bytes(&self, bytes: u64)` | 记录用户写入字节 | 🔒 |
| `record_total_bytes` | `fn record_total_bytes(&self, bytes: u64)` | 记录总写入字节 | 🔒 |
| `write_amplification_factor` | `fn write_amplification_factor(&self) -> f64` | 计算当前写放大因子 | ✅ |
| `should_compact_by_amplification` | `fn should_compact_by_amplification(&self) -> bool` | 检查是否应因写放大触发合并 | ✅ |
| `reset_amplification_counters` | `fn reset_amplification_counters(&self)` | 重置写放大计数器 | 🔒 |
| `set_wa_aware_trigger` | `fn set_wa_aware_trigger(&mut self, trigger)` | 设置 WA 感知触发器 | 🔒 |
| `evaluate_wa_aware_priority` | `fn evaluate_wa_aware_priority(&self) -> Option<(bool, CompactionPriority, bool)>` | 评估 WA 优先级 | ✅ |
| `update_l0_segments` | `fn update_l0_segments(&self, count: usize, total_size_bytes: u64)` | 更新 L0 段信息 | 🔒 |
| `l0_segment_count` | `fn l0_segment_count(&self) -> usize` | 获取 L0 段数 | ✅ |
| `l0_total_size_bytes` | `fn l0_total_size_bytes(&self) -> u64` | 获取 L0 总大小 | ✅ |

### 8.2 CompactionTrigger 模块

**源文件**: `/home/hugo/codes/try-tokitai/crates/tokitai-filekv/src/compaction/trigger.rs`

#### 8.2.1 CompactionPriority

⚠️ 实验

```rust
pub enum CompactionPriority {
    None,    // 不需要合并
    Low,     // WA < 2.0x，可激进合并
    Normal,  // WA 2.0x~3.0x，保守合并
    High,    // WA > 3.0x 或 L0 段过多
    Urgent,  // 必须立即合并
}
```

#### 8.2.2 IoPressure

⚠️ 实验

```rust
pub enum IoPressure {
    Low,    // 低 I/O 压力
    Medium, // 中 I/O 压力，应节流
    High,   // 高 I/O 压力，应暂停
}
```

#### 8.2.3 WaAwareState

⚠️ 实验

```rust
pub struct WaAwareState {
    pub write_amplification: f64,
    pub io_pressure: IoPressure,
    pub write_queue_depth: usize,
    pub write_latency_p99_us: u64,
    pub l0_segment_count: usize,
    pub l0_total_size_bytes: u64,
}
```

#### 8.2.4 IoPressureTracker

⚠️ 实验

| 方法 | 说明 |
|------|------|
| `new(max_latency_samples)` | 创建 I/O 压力追踪器 |
| `record_write_start` | 记录写操作开始 |
| `record_write_complete(latency_us)` | 记录写操作完成 |
| `queue_depth` | 获取当前写队列深度 |
| `p99_latency_us` | 计算 P99 写延迟 |
| `evaluate_pressure` | 评估当前 I/O 压力等级 |
| `should_compaction_pause` | 检查合并是否应暂停 |
| `set_compaction_paused` | 强制暂停/恢复合并 |

#### 8.2.5 WaAwareTriggerConfig

⚠️ 实验

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `wa_aggressive_threshold` | 2.0 | WA 低于此值激进合并 |
| `wa_conservative_threshold` | 3.0 | WA 高于此值保守合并 |
| `wa_delay_threshold` | 4.0 | WA 高于此值延迟合并 |
| `l0_emergency_threshold` | 8 | L0 紧急段数阈值 |
| `l0_warning_threshold` | 5 | L0 警告段数阈值 |
| `io_queue_depth_threshold` | 64 | I/O 队列深度阈值 |
| `io_latency_threshold_us` | 100 | I/O P99 延迟阈值（微秒） |
| `max_l0_priority_boost` | 2 | L0 最大优先级提升 |

#### 8.2.6 WriteAmplificationAwareTrigger

⚠️ 实验

| 方法 | 说明 |
|------|------|
| `new(config, io_tracker)` | 创建 WA 感知触发器 |
| `with_defaults(io_tracker)` | 使用默认配置创建 |
| `io_tracker` | 获取 I/O 追踪器引用 |
| `evaluate_priority(wa, l0_count, l0_size)` | 评估合并优先级 |
| `should_compact(state)` | 检查是否应合并 |
| `get_compaction_delay(priority)` | 获取合并延迟 |
| `build_state(wa, l0_count, l0_size)` | 构建状态对象 |

#### 8.2.7 TriggerType

🔒 内部

```rust
pub enum TriggerType {
    WriteCount,
    SizeThreshold,
    LevelBased,
    TimeBased,
    Composite,
}
```

#### 8.2.8 TriggerResult

🔒 内部

```rust
pub struct TriggerResult {
    pub should_trigger: bool,
    pub triggered_by: TriggerType,
    pub reason: String,
}
```

| 方法 | 说明 |
|------|------|
| `none` | 创建未触发结果 |
| `triggered(by, reason)` | 创建触发结果 |

#### 8.2.9 TriggerState

🔒 内部

```rust
pub struct TriggerState {
    pub writes_since_last_check: usize,
    pub total_size_bytes: u64,
    pub l0_file_count: usize,
}
```

#### 8.2.10 CompactionTrigger

✅ 稳定

```rust
pub enum CompactionTrigger {
    WriteCount { count: usize, current_count: usize },
    SizeThreshold { max_bytes: u64 },
    LevelBased { l0_max_files: usize },
    TimeBased { interval: Duration, last_triggered: Instant },
    Composite { triggers: Vec<CompactionTrigger> },
}
```

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `write_count` | `fn write_count(n: usize) -> Self` | 创建 WriteCount 触发器 | ✅ |
| `size_threshold` | `fn size_threshold(max_bytes: u64) -> Self` | 创建 SizeThreshold 触发器 | ✅ |
| `level_based` | `fn level_based(l0_max_files: usize) -> Self` | 创建 LevelBased 触发器 | ✅ |
| `time_based` | `fn time_based(interval: Duration) -> Self` | 创建 TimeBased 触发器 | ✅ |
| `composite` | `fn composite(triggers: Vec<CompactionTrigger>) -> Self` | 创建 Composite 触发器 | ✅ |
| `evaluate` | `fn evaluate(&mut self, state: &TriggerState) -> TriggerResult` | 评估触发器 | ✅ |
| `reset` | `fn reset(&mut self)` | 重置触发器状态 | ✅ |
| `trigger_type` | `fn trigger_type(&self) -> TriggerType` | 获取触发器类型 | 🔒 |

#### 8.2.11 default_compaction_trigger

✅ 稳定

```rust
pub fn default_compaction_trigger() -> CompactionTrigger
```

创建默认复合触发器：WriteCount(100) + LevelBased(l0_max_files: 3)。

### 8.3 KVIterator / MergeIterator

**源文件**: `/home/hugo/codes/try-tokitai/crates/tokitai-filekv/src/compaction/merge_iterator.rs`

#### 8.3.1 KVIterator trait

✅ 稳定

```rust
pub trait KVIterator: Send {
    fn next(&mut self) -> Option<(String, Bytes)>;
    fn peek(&self) -> Option<&(String, Bytes)>;
    fn has_next(&self) -> bool;
}
```

| 方法 | 说明 |
|------|------|
| `next` | 获取下一个 (key, value) 对 |
| `peek` | 查看下一项但不前进 |
| `has_next` | 检查是否还有更多项（默认实现） |

#### 8.3.2 MergeIterator<I: KVIterator>

✅ 稳定

K 路合并迭代器，使用 min-heap 高效合并多个有序 KV 流。

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new(iterators: Vec<I>) -> Self` | 从 KVIterators 创建合并迭代器 | ✅ |
| `has_next` | `fn has_next(&self) -> bool` | 检查是否还有更多项 | ✅ |
| `duplicates_removed` | `fn duplicates_removed(&self) -> u64` | 获取已移除的重复 key 数 | ✅ |
| `tombstones_cleaned` | `fn tombstones_cleaned(&self) -> u64` | 获取已清理的 tombstone 数 | ✅ |

实现了 `Iterator<Item = (String, Bytes)>` trait。

#### 8.3.3 MergeIteratorBuilder<I: KVIterator>

✅ 稳定

| 方法 | 签名 | 说明 |
|------|------|------|
| `new` | `fn new() -> Self` | 创建构建器 |
| `add_iter` | `fn add_iter(self, iter: I) -> Self` | 添加一个迭代器 |
| `deduplicate` | `fn deduplicate(self, enabled: bool) -> Self` | 启用/禁用去重（默认 true） |
| `build` | `fn build(self) -> MergeIterator<I>` | 构建 MergeIterator |

**使用示例**:

```rust
use tokitai_filekv::compaction::{MergeIteratorBuilder, KVIterator};
use tokitai_filekv::compaction::segment_iterator::SegmentIteratorBuilder;

// 从段文件创建迭代器
let seg_iter_builder = SegmentIteratorBuilder::new(segments);
let seg_iters = seg_iter_builder.build_all()?;

// 使用 MergeIterator 合并
let merge_iter = MergeIteratorBuilder::new()
    .add_iter(seg_iters[0].clone())
    .add_iter(seg_iters[1].clone())
    .add_iter(seg_iters[2].clone())
    .deduplicate(true)  // 去重（保留最新）
    .build();

// 遍历合并结果
for (key, value) in merge_iter {
    // 写入新段...
}
```

### 8.4 SegmentIterator

**源文件**: `/home/hugo/codes/try-tokitai/crates/tokitai-filekv/src/compaction/segment_iterator.rs`

#### 8.4.1 SegmentIterator

✅ 稳定

流式段文件迭代器，逐个读取 KV 对而不将所有数据加载到内存。

| 方法 | 签名 | 说明 | 稳定性 |
|------|------|------|--------|
| `new` | `fn new(segment: Arc<SegmentFile>) -> Result<Self, FatalError>` | 创建段迭代器 | ✅ |
| `with_tombstone_counter` | `fn with_tombstone_counter(segment, counter: Option<Arc<AtomicU64>>) -> Result<Self, FatalError>` | 创建带 tombstone 计数器的迭代器 | ⚠️ |
| `tombstone_counter` | `fn tombstone_counter(&self) -> &Arc<AtomicU64>` | 获取共享 tombstone 计数器 | ⚠️ |

实现了 `KVIterator` 和 `Iterator<Item = (String, Bytes)>` traits。

#### 8.4.2 SegmentIteratorBuilder

✅ 稳定

| 方法 | 签名 | 说明 |
|------|------|------|
| `new` | `fn new(segments: Vec<Arc<SegmentFile>>) -> Self` | 创建构建器 |
| `build_all` | `fn build_all(self) -> Result<Vec<SegmentIterator>, FatalError>` | 构建所有迭代器，任一失败则返回错误 |
| `build_all_skip_errors` | `fn build_all_skip_errors(self) -> Vec<SegmentIterator>` | 构建迭代器，跳过失败的 |

---

## §12 I/O 抽象 API

**源文件目录**: `/home/hugo/codes/try-tokitai/crates/tokitai-filekv/src/io/`

### 12.1 FileKVFileSystem trait

✅ 稳定

```rust
pub trait FileKVFileSystem: Send + Sync + 'static {
    fn create_file(&self, path: &Path) -> IoResult<Box<dyn FileKVFile>>;
    fn open_file(&self, path: &Path, read: bool, write: bool, append: bool) -> IoResult<Box<dyn FileKVFile>>;
    fn read_dir(&self, path: &Path) -> IoResult<Vec<PathBuf>>;
    fn create_dir_all(&self, path: &Path) -> IoResult<()>;
    fn rename(&self, from: &Path, to: &Path) -> IoResult<()>;
    fn remove_file(&self, path: &Path) -> IoResult<()>;
    fn file_exists(&self, path: &Path) -> bool;
    fn file_metadata(&self, path: &Path) -> IoResult<FileMetadata>;
    fn sync_dir(&self, path: &Path) -> IoResult<()>;
    fn clone_as_mmap_fs(&self) -> Option<Arc<dyn MmapFileSystem>>;
}
```

### 12.2 FileKVFile trait

✅ 稳定

```rust
pub trait FileKVFile: Send + Sync {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize>;
    fn read_exact(&mut self, buf: &mut [u8]) -> IoResult<()>;
    fn write(&mut self, buf: &[u8]) -> IoResult<usize>;
    fn write_all(&mut self, buf: &[u8]) -> IoResult<()>;
    fn flush(&mut self) -> IoResult<()>;
    fn sync_all(&self) -> IoResult<()>;
    fn try_clone(&self) -> IoResult<Box<dyn FileKVFile>>;
    fn metadata(&self) -> IoResult<FileMetadata>;
    fn as_any(&self) -> &dyn Any;
}
```

### 12.3 MmapFileSystem trait

✅ 稳定

```rust
pub trait MmapFileSystem: FileKVFileSystem {
    fn mmap(&self, file: &dyn FileKVFile) -> IoResult<Arc<dyn MmapView>>;
}
```

### 12.4 MmapView trait

✅ 稳定

```rust
pub trait MmapView: Send + Sync {
    fn as_slice(&self) -> &[u8];
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
}
```

### 12.5 StdFs / StdFile / StdMmap

✅ 稳定

**StdFs**: 标准文件系统实现，委托给 `std::fs` 和 `memmap2`。同时实现了 `FileKVFileSystem` 和 `MmapFileSystem`。

**StdFile**: 标准文件句柄封装 `std::fs::File`。

**StdMmap**: 标准 mmap 视图封装 `memmap2::Mmap`。

**使用示例**:

```rust
use tokitai_filekv::io::{StdFs, FileKVFileSystem, FileKVFile};
use std::path::Path;
use std::sync::Arc;

let fs = Arc::new(StdFs);

// 创建文件
let mut file = fs.create_file(Path::new("/data/test.txt"))?;
file.write_all(b"hello")?;
file.sync_all()?;

// 打开文件
let mut file = fs.open_file(Path::new("/data/test.txt"), true, false, false)?;
let mut buf = [0u8; 5];
file.read(&mut buf)?;
```

### 12.6 MemFs / MemFile

⚠️ 实验（用于测试）

**MemFs**: 内存文件系统实现，使用 `BTreeMap<PathBuf, Vec<u8>>` 模拟文件系统。不实现 `MmapFileSystem`。

| 方法 | 说明 |
|------|------|
| `new` | 创建 MemFs 实例 |

**MemFile**: 内存文件句柄。

### 12.7 FaultInjector / FaultRule / FaultStrategy / FaultInjectorStats

**源文件**: `/home/hugo/codes/try-tokitai/crates/tokitai-filekv/src/io/fault_inject.rs`

#### 12.7.1 FaultStrategy

⚠️ 实验

```rust
pub enum FaultStrategy {
    FailAfterN(u64),                    // N 次成功调用后失败
    FailRandom(f64),                    // 按概率随机失败
    AlwaysFail(std::io::ErrorKind, String), // 总是以特定错误失败
    Delay(Duration),                    // 延迟操作
    Combined { delay: Option<Duration>, fault: Box<FaultStrategy> }, // 组合：延迟 + 故障
}
```

#### 12.7.2 FaultRule

⚠️ 实验

```rust
pub struct FaultRule {
    pub operation_prefixes: Vec<String>,  // 适用的操作前缀（空=所有）
    pub strategy: FaultStrategy,          // 故障策略
    pub active: bool,                     // 规则是否活跃
}
```

| 方法 | 说明 |
|------|------|
| `new_all(strategy)` | 创建适用于所有操作的规则 |
| `new_for_ops(strategy, prefixes)` | 创建适用于特定操作的规则 |

#### 12.7.3 FaultInjector

⚠️ 实验

装饰器模式，包装任何 `FileKVFileSystem` 并注入故障。

| 方法 | 签名 | 说明 |
|------|------|------|
| `new` | `fn new(inner: Arc<dyn FileKVFileSystem>) -> Self` | 创建故障注入器 |
| `new_with_mmap` | `fn new_with_mmap(inner: Arc<dyn MmapFileSystem>) -> Self` | 创建支持 mmap 的注入器 |
| `add_rule` | `fn add_rule(&self, rule: FaultRule)` | 添加故障规则 |
| `clear_rules` | `fn clear_rules(&self)` | 清除所有规则 |
| `set_disk_full_after` | `fn set_disk_full_after(&self, n: u64)` | 便捷方法：N 次调用后磁盘满 |
| `set_random_fail` | `fn set_random_fail(&self, probability: f64)` | 便捷方法：按概率随机失败 |
| `set_delay` | `fn set_delay(&self, delay: Duration)` | 便捷方法：延迟所有操作 |

**使用示例**:

```rust
use tokitai_filekv::io::{FaultInjector, FaultRule, FaultStrategy, MemFs, FileKVFileSystem};
use std::path::Path;
use std::sync::Arc;

let memfs = Arc::new(MemFs::new());
let injector = FaultInjector::new(memfs);

// 模拟磁盘满：3 次操作后失败
injector.set_disk_full_after(3);

// 前 3 次创建成功
for i in 0..3 {
    injector.create_file(&Path::new(&format!("/file_{}.txt", i))).unwrap();
}

// 第 4 次失败
assert!(injector.create_file(&Path::new("/file_3.txt")).is_err());
```

#### 12.7.4 FaultInjectFile

🔒 内部

故障注入文件句柄，实现了 `FileKVFile` trait。

### 12.8 FileMetadata

✅ 稳定

```rust
pub struct FileMetadata {
    pub len: u64,
    pub exists: bool,
}
```

| 方法 | 说明 |
|------|------|
| `new(len)` | 创建存在的文件元数据 |
| `not_exists()` | 创建不存在的文件元数据 |

### 12.9 CompactionManifest / CompactionExecutor / RecoveryAction

**源文件**: `/home/hugo/codes/try-tokitai/crates/tokitai-filekv/src/compaction/manifest.rs`

#### 12.9.1 CompactionStatus

🔒 内部

```rust
pub enum CompactionStatus {
    InProgress,  // 合并进行中
    Completed,   // 合并成功完成
    Aborted,     // 合并已中止
}
```

#### 12.9.2 CompactionManifest

⚠️ 实验

```rust
pub struct CompactionManifest {
    pub compaction_id: u64,
    pub input_segments: Vec<u64>,
    pub output_segments: Vec<u64>,
    pub output_level: u8,
    pub status: CompactionStatus,
    pub started_at: u64,
    pub completed_at: Option<u64>,
    pub estimated_output_size_bytes: Option<u64>,
}
```

| 方法 | 说明 |
|------|------|
| `new(id, inputs, outputs, level)` | 创建新的合并清单 |
| `mark_completed` | 标记为已完成 |
| `mark_aborted` | 标记为已中止 |
| `to_bytes` | 序列化为 JSON 字节（含头部） |
| `from_bytes` | 从字节反序列化 |
| `write_atomic` | 原子写入清单文件（temp + rename） |
| `read_from_file` | 从文件读取清单 |
| `persist` | 持久化清单到磁盘 |

#### 12.9.3 CompactionExecutor

⚠️ 实验

| 方法 | 说明 |
|------|------|
| `new(fs, manifest_dir)` | 创建合并执行器 |
| `prepare(manifest)` | 准备：写入清单，开始合并 |
| `commit(manifest)` | 提交：标记合并完成 |
| `abort(manifest)` | 中止：标记合并已中止 |
| `current_manifest_path` | 获取当前清单路径 |

#### 12.9.4 RecoveryAction

⚠️ 实验

```rust
pub enum RecoveryAction {
    None,
    CleanedUp {
        compaction_id: u64,
        deleted_output_segments: Vec<u64>,
        restored_input_segments: Vec<u64>,
    },
}
```

#### 12.9.5 recover_incomplete

⚠️ 实验

```rust
pub fn recover_incomplete(
    fs: &dyn FileKVFileSystem,
    manifest_dir: &Path,
    segment_dir: &Path,
) -> anyhow::Result<Vec<RecoveryAction>>
```

扫描清单目录，恢复未完成的合并操作。

---

## §13 错误类型

**源文件**: `/home/hugo/codes/try-tokitai/crates/tokitai-filekv/src/core/error.rs`

FileKV 采用四层错误体系，调用者可以在编译时区分可恢复/致命/预期/领域错误。

### 13.1 FatalError

✅ 稳定

无法恢复的错误，表明数据损坏或不可恢复的 I/O 故障。

```rust
pub enum FatalError {
    Corruption(String),       // 段文件或索引数据损坏
    Io(std::io::Error),       // 不可恢复的 I/O 错误
    WalCorrupted(String),     // WAL 文件损坏
}
```

| 方法 | 说明 |
|------|------|
| `is_retryable` | 返回 false（致命错误不可重试） |

### 13.2 TransientError

✅ 稳定

可重试的错误，表明临时资源约束。

```rust
pub enum TransientError {
    ResourceExhausted(String),  // 资源耗尽（如内存超限）
    Timeout(Duration),          // 操作超时
    Backpressure(String),       // 背压生效（如 MemTable 已满）
}
```

| 方法 | 说明 |
|------|------|
| `is_retryable` | 返回 true（ transient 错误可重试） |

### 13.3 ExpectedError

✅ 稳定

预期内的"错误"，属于正常控制流的一部分。

```rust
pub enum ExpectedError {
    KeyNotFound(String),        // Key 不存在
    SegmentNotFound(u64),       // 段 ID 不存在
    BloomNegative(u64),         // Bloom 过滤器阴性（key 不在段中）
}
```

### 13.4 DomainError

✅ 稳定

领域/逻辑错误，不可重试。

```rust
pub enum DomainError {
    Config(String),             // 配置无效
    Compaction(String),         // 合并失败
    Index(String),              // 索引错误
    Checkpoint(String),         // 检查点错误
}
```

### 13.5 FileKVError

✅ 稳定

统一错误类型，封装所有错误类别。

```rust
pub enum FileKVError {
    Fatal(FatalError),          // 致命错误
    Transient(TransientError),  // 可重试错误
    Expected(ExpectedError),    // 预期错误
    Domain(DomainError),        // 领域错误
}
```

| 方法 | 签名 | 说明 |
|------|------|------|
| `is_retryable` | `fn is_retryable(&self) -> bool` | 是否可重试（Transient） |
| `is_fatal` | `fn is_fatal(&self) -> bool` | 是否致命（Fatal） |
| `is_expected` | `fn is_expected(&self) -> bool` | 是否预期错误（Expected） |
| `is_domain_error` | `fn is_domain_error(&self) -> bool` | 是否领域错误（Domain） |
| `category` | `fn category(&self) -> ErrorCategory` | 获取错误分类 |

### 13.6 ErrorCategory

✅ 稳定

```rust
pub enum ErrorCategory {
    Io,         // I/O 相关
    Config,     // 配置错误
    Corruption, // 数据损坏
    Resource,   // 资源耗尽
    Timeout,    // 超时
    Other,      // 其他
}
```

### 13.7 类型别名

✅ 稳定

| 类型别名 | 完整类型 | 用途 |
|----------|----------|------|
| `FileKVResult<T>` | `Result<T, FileKVError>` | FileKV 内部操作的通用 Result |
| `ReadResult<T>` | `Result<T, ExpectedError>` | 读操作 Result（KeyNotFound 为正常结果） |
| `WriteResult<T>` | `Result<T, FileKVError>` | 写操作 Result（可能致命或 transient） |

### 13.8 使用示例

```rust
use tokitai_filekv::core::error::{FileKVError, FatalError, TransientError, ExpectedError, DomainError, FileKVResult};

// 处理读操作：KeyNotFound 是正常的
fn read_key(store: &FileKV, key: &str) -> ReadResult<Vec<u8>> {
    match store.get(key)? {
        Some(value) => Ok(value),
        None => Err(ExpectedError::KeyNotFound(key.to_string())),
    }
}

// 处理写操作：可能遇到背压或 I/O 错误
fn write_key(store: &FileKV, key: &str, value: &[u8]) -> WriteResult<()> {
    store.put(key, value).map_err(|e| FileKVError::from(e))
}

// 错误分类处理
fn handle_error(err: FileKVError) {
    match err {
        FileKVError::Fatal(e) => {
            // 致命错误，记录并中止
            eprintln!("Fatal error: {}", e);
            std::process::abort();
        }
        FileKVError::Transient(e) => {
            // 可重试，带退避重试
            if e.is_retryable() {
                // retry with backoff
            }
        }
        FileKVError::Expected(e) => {
            // 预期行为，正常处理
            println!("Expected: {}", e);
        }
        FileKVError::Domain(e) => {
            // 配置/逻辑问题，需修复
            eprintln!("Domain error: {}", e);
        }
    }
}

// 错误分类查询
let err = FileKVError::Transient(TransientError::Backpressure("memtable full".into()));
println!("Category: {:?}", err.category()); // ErrorCategory::Resource
println!("Retryable: {}", err.is_retryable()); // true
```

---

## 附录：稳定性标识说明

| 标识 | 含义 | 说明 |
|------|------|------|
| ✅ 稳定 | 公共 API，向后兼容 | 可安全依赖，变更需遵循 semver |
| ⚠️ 实验 | 实验性 API，可能变更 | 功能可用但可能在未来版本变更 |
| 🔒 内部 | 内部使用 | 不建议外部依赖，无兼容保证 |
