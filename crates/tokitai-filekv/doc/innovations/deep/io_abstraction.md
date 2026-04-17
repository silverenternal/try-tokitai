# I/O 抽象层创新深度调研

> 本文档详细分析 tokitai-filekv 的 I/O 抽象层设计,包含文件系统抽象、内存映射、异步I/O、缓冲策略和预取机制。

---

## 目录

- [1. I/O 抽象层总览](#1-io-抽象层总览)
- [2. 文件系统抽象接口](#2-文件系统抽象接口)
- [3. 内存映射 (mmap) 优化](#3-内存映射-mmap-优化)
- [4. 异步 I/O 实现](#4-异步-io-实现)
- [5. 缓冲策略](#5-缓冲策略)
- [6. 预取和预读机制](#6-预取和预读机制)
- [7. 批量 I/O 操作](#7-批量-io-操作)
- [8. 性能优化点](#8-性能优化点)
- [9. 关键文件索引](#9-关键文件索引)

---

## 1. I/O 抽象层总览

### 1.1 设计目标

tokitai-filekv 的 I/O 抽象层设计目标:

1. **可移植性**: 通过抽象层支持不同平台和文件系统
2. **可测试性**: 内存文件系统和故障注入支持测试
3. **高性能**: mmap、零拷贝、异步 I/O 优化
4. **灵活性**: 可配置缓冲、预取、同步策略

### 1.2 核心架构

```
┌─────────────────────────────────────────────┐
│         应用层 (ReadEngine/WriteEngine)      │
├─────────────────────────────────────────────┤
│         I/O 抽象层 (FileKVFileSystem)        │
├──────────┬──────────┬──────────┬────────────┤
│  StdFs   │  MemFs   │ FaultInj│ MmapFs     │
│ (生产)   │ (测试)   │ (测试)   │ (mmap)     │
└──────────┴──────────┴──────────┴────────────┘
```

---

## 2. 文件系统抽象接口

### 2.1 核心 Trait 定义

**文件**: `src/io/mod.rs`

```rust
/// 核心文件系统 trait
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
    fn clone_as_mmap_fs(&self) -> Option<Arc<dyn MmapFileSystem>> { None }
}
```

### 2.2 文件句柄 Trait

```rust
/// 核心文件句柄 trait
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

**设计特点**:
- 所有文件操作通过 `FileKVFileSystem` 而非直接调用 `std::fs`
- 支持动态类型转换 (`as_any()`) 实现特定文件系统底层操作
- `Box<dyn FileKVFile>` 实现 `std::io::Read` 和 `std::io::Write`,可与标准库互操作

### 2.3 具体实现

#### StdFs (生产环境)

**文件**: `src/io/stdfs.rs`

```rust
pub struct StdFs;

impl FileKVFileSystem for StdFs {
    fn create_file(&self, path: &Path) -> IoResult<Box<dyn FileKVFile>> {
        let file = std::fs::File::create(path)?;
        Ok(Box::new(StdFile(file)))
    }
    
    fn clone_as_mmap_fs(&self) -> Option<Arc<dyn MmapFileSystem>> {
        Some(Arc::new(StdMmapFs))
    }
}

pub struct StdFile(File);
pub struct StdMmap(memmap2::Mmap);
```

#### MemFs (测试用)

**文件**: `src/io/memfs.rs`

```rust
pub struct MemFs {
    files: Arc<Mutex<BTreeMap<PathBuf, Vec<u8>>>>,
}
```

- 使用 `BTreeMap<PathBuf, Vec<u8>>` 模拟文件系统
- 不支持 `MmapFileSystem` trait (无真实文件描述符)
- 用于单元测试和集成测试

#### FaultInjector (故障注入)

**文件**: `src/io/fault_inject.rs`

```rust
pub struct FaultInjector {
    inner: Arc<dyn FileKVFileSystem>,
    rules: Vec<FaultRule>,
}

pub enum FaultStrategy {
    FailAfterN { n: usize },      // N 次后失败
    FailRandom { probability: f64 }, // 随机失败
    AlwaysFail,                    // 总是失败
    Delay { duration: Duration },  // 延迟注入
    Combined { ... },              // 组合策略
}
```

**用途**: 测试磁盘满、随机 I/O 错误、延迟等场景

---

## 3. 内存映射 (mmap) 优化

### 3.1 Mmap 抽象接口

```rust
pub trait MmapFileSystem: FileKVFileSystem {
    fn mmap(&self, file: &dyn FileKVFile) -> IoResult<Arc<dyn MmapView>>;
}

pub trait MmapView: Send + Sync {
    fn as_slice(&self) -> &[u8];
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool { self.len() == 0 }
}
```

### 3.2 mmap 在 Segment 中的核心应用

**文件**: `src/core/segment.rs`

```rust
struct SegmentFile {
    /// 使用 ArcSwapOption 实现无锁 mmap 管理
    mmap: Arc<ArcSwapOption<Arc<dyn MmapView>>>,
    /// 是否使用持久 mmap (false = 每次读取时临时创建)
    use_persistent_mmap: bool,
    mmap_fs: Option<Arc<dyn crate::io::MmapFileSystem>>,
}
```

### 3.3 关键优化点

#### PERF-002: 持久化 mmap 策略

**一次性创建**: 在 `SegmentFile::open()` 时创建持久 mmap,所有读取操作复用

**无锁并发**: 使用 `ArcSwapOption` 替代 `RwLock<Option<Arc<Mmap>>>`,读取时无需加锁

```rust
// 读取时通过 ArcSwapOption 加载,无锁
if let Some(mmap) = self.mmap.load() {
    let slice = mmap.as_slice();
    // 直接访问 mmap 数据
}
```

#### PERF-ZEROCOPY-001: 零拷贝快速读取

```rust
pub fn read_at_zero_copy(&self, offset: u64, key_len: u32, value_len: u32) 
    -> Result<bytes::Bytes, FatalError> 
{
    // 创建持有 mmap 引用的 Bytes,value 切片零拷贝
    let mmap_owner = MmapSliceOwner { mmap, offset, len };
    Ok(bytes::Bytes::from_owner(mmap_owner))
}
```

**优势**: 
- 零内存分配 (借用 mmap 内存)
- 零内存拷贝 (直接返回切片)
- 线程安全 (Arc 管理生命周期)

### 3.4 安全性验证

1. **文件大小验证**: < 8 字节视为损坏
2. **Magic bytes 校验**: 验证文件格式
3. **Version 校验**: 验证格式版本
4. **边界检查**: 所有 mmap 访问包含边界检查

### 3.5 读写分离

- **mmap**: 使用只读文件句柄,用于读取
- **写入**: 使用单独的追加模式句柄
- **刷新**: `refresh_mmap()` 在数据写入后更新映射

---

## 4. 异步 I/O 实现

### 4.1 架构设计

**文件**: `src/ops/async_io.rs`

```
Write API --> AsyncWriter --> WriteQueue (mpsc channel)
                  |                |
                  v                v
            Semaphore        Worker Task (spawn_blocking)
                                  |
                                  v
                            FileHandleCache --> Disk (SSD)
```

### 4.2 核心组件

```rust
pub struct AsyncWriter {
    config: AsyncIoConfig,
    write_tx: mpsc::Sender<WriteMessage>,
    stats: Arc<RwLock<AsyncIoStats>>,
    write_semaphore: Arc<Semaphore>,      // 限制并发写入
    file_handles: Arc<Mutex<FileHandleCache>>,
    runtime_handle: tokio::runtime::Handle,
}

pub enum AsyncWriteOp {
    SegmentWrite { segment_id: u64, offset: u64, data: Bytes },
    WalWrite { data: Bytes, sync: bool },
    Flush { path: PathBuf },
    CreateSegment { segment_id: u64, preallocate_bytes: u64 },
}
```

### 4.3 同步桥接机制 (MAJ-006 防死锁)

```rust
fn block_on_sync<F, T>(&self, fut: F) -> Result<T> {
    let in_runtime = tokio::runtime::Handle::try_current().is_ok();
    if in_runtime {
        // 在 runtime 内使用 std::thread::scope 避免死锁
        std::thread::scope(|s| s.spawn(|| handle.block_on(fut)).join())
    } else {
        Ok(self.runtime_handle.block_on(fut))
    }
}
```

### 4.4 文件句柄缓存

```rust
struct FileHandleCache {
    max_handles: usize,
    writers: VecDeque<(u64, BufWriter<File>)>,  // LRU 缓存
}
```

- 最多缓存 16 个打开的 segment 文件句柄
- 使用 LRU 策略淘汰旧句柄
- 减少频繁 open/close 的开销

### 4.5 配置选项

```rust
pub struct AsyncIoConfig {
    pub enabled: bool,
    pub max_concurrent_writes: usize,     // 默认 4
    pub max_queue_depth: usize,           // 默认 1024
    pub write_timeout_ms: u64,            // 默认 5000
    pub enable_coalescing: bool,          // 默认 true
    pub coalesce_window_ms: u64,          // 默认 10
}
```

---

## 5. 缓冲策略

### 5.1 写缓冲 - WriteBuffer

**文件**: `src/core/write_coalescer.rs`

```rust
pub struct WriteBufferConfig {
    pub time_window_us: u64,          // 默认 100ms
    pub size_threshold_bytes: usize,   // 默认 64KB
}
```

**触发条件**:
1. 时间窗口超过 100ms
2. 缓冲区大小达到 64KB

### 5.2 WAL 批量写入 - WalBatcher

**文件**: `src/core/wal_batcher.rs`

```rust
pub struct WalBatcherConfig {
    pub batch_interval_ms: u64,     // 默认 2ms
    pub batch_max_entries: usize,    // 默认 1000
}
```

**优势**: 将 N 次 fsync 减少为 1 次 fsync 每批次

### 5.3 写合并器 - WriteCoalescer

Phase 6 引入的写入路径优化:
- 批量 WAL 写入 (一次 fsync 多条记录)
- 支持 `Durability::Immediate` 绕过缓冲
- 通过 `write_coalescer.add()` 提交,返回 `Some(batch)` 时触发 flush

### 5.4 读缓冲 - BlockCache

**文件**: `src/cache/block_cache.rs`

- 使用 Moka 分片缓存 (默认每分片 16MB)
- 统一缓存管理器协调 BlockCache、BloomFilterCache 预算
- 支持后台内存预算再平衡线程

---

## 6. 预取和预读机制

### 6.1 Segment 级别预读 (Readahead)

**文件**: `src/core/segment.rs`

```rust
struct SegmentFile {
    readahead_multiplier: u32,  // 0 = 禁用, 1-8 = 预读倍数
}
```

**API**:
```rust
pub fn read_at_with_readahead(
    &self, offset: u64, key_len: u32, value_len: u32, readahead_blocks: u32
) -> Result<(Vec<u8>, Vec<Vec<u8>>), FatalError> {
    // 先读取目标值,再预读后续多个 block
}

pub fn read_at_with_configured_readahead(&self, offset: u64, key_len: u32, value_len: u32)
    -> Result<(Vec<u8>, Vec<Vec<u8>>), FatalError> {
    self.read_at_with_readahead(offset, key_len, value_len, self.readahead_multiplier)
}
```

### 6.2 顺序预取 (Sequential Prefetcher)

**文件**: `src/cache/prefetch.rs`

```rust
pub struct SequentialPrefetcherConfig {
    pub enabled: bool,               // 默认 true
    pub sequential_threshold: u32,   // 连续访问次数阈值 (默认 3)
    pub prefetch_distance: u32,      // 预取距离 (默认 2 blocks)
    pub max_prefetch_window: u32,    // 最大预取窗口 (默认 10 blocks)
    pub adaptive_distance: bool,     // 自适应预取距离
}
```

**算法**:
1. 跟踪上次访问的 key,检测步长模式 (`SequentialDetector`)
2. 检测到连续 N 次访问后触发预取
3. 根据检测到的步长预取后续 K 个 block
4. 自适应调整: 准确率 > 80% 时增加距离,< 50% 时减少

### 6.3 自适应预取

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

### 6.4 预读配置预设

| 模式 | readahead_multiplier | 说明 |
|------|---------------------|------|
| Conservative | 0 | 禁用预读 |
| Balanced | 2 | 2x 预读 |
| Performance | 4 | 4x 预读 |
| Extreme | 8 | 8x 激进预读 |

---

## 7. 批量 I/O 操作

### 7.1 批量写入路径

```
put_batch(entries)
    |
    +-> write_engine.put_batch()
        |
        +-> WAL: 单条 batch 记录写入 (原子性)
        |   wal.log_batch(ops_with_payloads)
        |
        +-> MemTable: 批量插入
            memtable.insert_batch(&batch_entries)
```

### 7.2 全局索引批量更新

**文件**: `src/core/global_index.rs`

```rust
pub fn bulk_insert(&self, keys: Vec<(Arc<str>, KeyLocation)>);
pub fn bulk_upsert(&self, keys: Vec<(Arc<str>, KeyLocation)>);
```

用于 compaction 后批量更新全局键索引。

---

## 8. 性能优化点

### 8.1 优化汇总

| 优化项 | 位置 | 效果 |
|--------|------|------|
| 持久 mmap + ArcSwapOption | segment.rs | 读取无锁,避免重复映射 |
| 零拷贝 Bytes | segment.rs::read_at_zero_copy | 减少内存分配和拷贝 |
| mmap 边界检查 | segment.rs 所有读取方法 | 防止越界访问 |
| 文件句柄 LRU 缓存 | async_io.rs::FileHandleCache | 减少 open/close 开销 |
| spawn_blocking | async_io.rs | 避免阻塞 Tokio 运行时 |
| Semaphore 限流 | async_io.rs | 控制并发写入数量 |
| WriteBuffer 时间+大小触发 | write_coalescer.rs | 合并小写入 |
| WalBatcher 批量 fsync | wal_batcher.rs | N fsyncs -> 1 fsync |
| 自适应预取距离 | prefetch.rs | 根据准确率动态调整 |
| BufWriter 256KB | wal.rs | 减少系统调用次数 |

### 8.2 性能数据

**WAL 写入性能**:
| 操作 | 值大小 | FileKV 性能 |
|------|--------|-----------|
| Write (WAL, 64B) | 64 bytes | **1.57 µs/entry** (637K ops/sec) |
| Write (WAL, 1KB) | 1 KB | **3.92 µs/entry** (255K ops/sec) |
| Write (WAL, 4KB) | 4 KB | **10.91 µs/entry** (92K ops/sec) |
| Write (no WAL, 64B) | 64 bytes | **1.17 µs/entry** (854K ops/sec) |

**WAL 优化效果**:
| 优化 | 加速比 |
|------|--------|
| 二进制序列化 (vs JSON) | 3-5x |
| 批量 WAL + 定时 fsync | 显著减少 fsync 开销 |
| Channel 异步写入 | 非阻塞 put() |

---

## 9. 关键文件索引

| 文件路径 | 职责 |
|---------|------|
| `src/io/mod.rs` | I/O 抽象层核心 trait 定义 |
| `src/io/stdfs.rs` | StdFs 生产实现 |
| `src/io/memfs.rs` | MemFs 内存文件系统 |
| `src/io/fault_inject.rs` | 故障注入装饰器 |
| `src/core/segment.rs` | Segment 文件管理 (mmap 核心使用) |
| `src/ops/async_io.rs` | 异步 I/O 实现 |
| `src/cache/prefetch.rs` | 顺序预取器 |
| `src/core/write_coalescer.rs` | 写缓冲/合并器 |
| `src/core/wal_batcher.rs` | WAL 批量写入 |
| `src/core/types.rs` | 配置定义 (AggressiveConfig 等) |
| `src/cache/block_cache.rs` | 块缓存 |
| `src/cache/mod.rs` | 统一缓存管理器 |

---

## 总结

tokitai-filekv 的 I/O 抽象层通过以下创新实现高性能和灵活性:

1. **抽象层设计**: 支持多种文件系统实现,便于测试和移植
2. **mmap 优化**: 持久映射、零拷贝、无锁并发
3. **异步 I/O**: 避免阻塞运行时,控制并发
4. **缓冲策略**: 写缓冲、批量 fsync、写合并
5. **智能预取**: 自适应距离,顺序检测

这些优化使 tokitai-filekv 在现代 SSD 上达到接近硬件极限的性能。
