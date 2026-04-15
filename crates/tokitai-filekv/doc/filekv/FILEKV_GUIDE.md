# FileKV 存储引擎完全指南

**版本**: 0.5.0
**最后更新**: 2026-04-15 (v0.5.0 完成)
**状态**: 实验性生产引擎 (431 lib tests + 28 integration tests 通过，0 clippy 警告，核心 API 稳定)

---

## 📋 目录

1. [概述](#概述)
2. [核心架构](#核心架构)
3. [数据模型](#数据模型)
4. [写入路径](#写入路径)
5. [读取路径](#读取路径)
6. [核心组件](#核心组件)
7. [配置指南](#配置指南)
8. [性能基准](#性能基准)
9. [最佳实践](#最佳实践)
10. [故障排查](#故障排查)
11. [高级特性](#高级特性)
12. [API 参考](#api 参考)

---

## 概述

### 什么是 FileKV？

FileKV 是一个**纯 Rust 实现的 LSM-Tree 风格 KV 存储引擎**，专为 Tokitai-Context 系统设计。它借鉴了 RocksDB 的核心思想，同时针对 AI 对话上下文场景进行了深度优化。

### 核心设计思想

```
┌─────────────────────────────────────────────────────────────┐
│                    FileKV Storage Engine                    │
│                                                             │
│  Write Path:                                                │
│  put() → WAL → MemTable → Flush → Segment Files            │
│                                                             │
│  Read Path:                                                 │
│  get() → MemTable → BlockCache → BloomFilter → Segment     │
│                                                             │
│  Background:                                                │
│  Compaction → Merge Segments → Reclaim Space               │
└─────────────────────────────────────────────────────────────┘
```

### 关键特性

| 特性 | 描述 | 收益 | 集成状态 |
|------|------|------|---------|
| **LSM-Tree 架构** | 顺序写入，批量刷盘 | 写放大降至最低 | ✅ |
| **MemTable** | DashMap 无锁并发 | O(1) 写入延迟 | ✅ |
| **BlockCache** | LRU 热点缓存 | 读延迟降低 9.69x (公平对比) | ✅ |
| **BloomFilter** | 快速负向查找 | 避免无效磁盘 IO | ✅ |
| **Sparse/Dense Index** | 灵活索引策略 | 空间/时间权衡 | ✅ |
| **WAL** | Write-Ahead Log | 崩溃恢复保证 | ✅ |
| **Compaction** | 后台合并 | 空间回收，读性能提升 | ✅ |
| **Bloom Migration** | 自适应缓存分层 | 热数据自动加速 | ✅ |
| **字典压缩 (T-024)** | DictionaryTrainer + zstd 字典训练 | 压缩率提升 20-40% | ✅ (框架已实现，生产路径待默认启用) |
| **Rebalance 决策+执行 (T-025)** | UnifiedCacheManager 后台 rebalance 线程 | 缓存预算自动调整 | ✅ (决策引擎完整实现；执行引擎：BloomFilterCache 支持动态缩容/扩容，BlockCache 仅 advisory mode — Moka capacity 不可变，通过 `run_pending_tasks()` 施加驱逐压力) |
| **频率感知 Bloom (T-026)** | Hot/Warm/Cold 频率分层 + QPS 混合评分 | 更精准的缓存层分配 | ✅ (框架已实现，消费逻辑待完善) |
| **WAL 序列号校验 (T-018)** | 序列号连续性校验 + 完整性验证 | 崩溃恢复安全增强 | ✅ |
| **Zone Map Pruning** | Range 查询优化 | 减少 40-60% I/O | ✅ |
| **Range Scan Readahead** | 顺序读取预读 | 吞吐量提升 2-4x | ✅ (仅范围扫描) |
| **Write Amplification Tracking** | WAF/RAF/SAF 监控 | 运维可观测性 | 🟡 (WAF/RAF 为估算值，非精确测量) |
| **Memory Monitoring** | 内存使用跟踪 | 内存压力管理 | 🟠 (数据为估算值) |
| **Async I/O** | 非阻塞异步写入 | 高吞吐场景优化 | 🟡 (仅 put_buffered_async) |
| **Timeout Control** | 操作超时保护 | 后台操作保护 | 🟠 (仅后台操作) |
| **WAL Batch** | 批量写入优化 | 减少 WAL 同步次数 | ✅ (批量 flush，定期 fsync) |
| **Incremental Checkpoint** | 增量检查点备份 | 时间点恢复基础 | ✅ (需手动调用) |

> **文档职责说明**:
> - **README.md** = 快速参考（核心特性、性能数据、快速开始、配置预设）
> - **FILEKV_GUIDE.md** (本文档) = 技术深度（架构详解、数据模型、读写路径、配置指南、故障排查、API 参考）
> - **POSITION_AND_STATUS.md** = 路线图与状态（项目定位、已知限制、实现状态清单、生产就绪路线图）

### 性能亮点

> 完整性能数据（含与 RocksDB 公平对比、测试日期）详见 [README.md](../../README.md#-性能表现与-rocksdb-公平对比)。关键数据摘要：
> - Bloom Filter 负向查询：**62.37 µs**（比 RocksDB 快 3.97x）
> - 全 KV Get (热点缓存)：**61.92 µs**（比 RocksDB 快 9.69x）
> - 写入 (64B, WAL)：**1.71 µs/entry**（比 RocksDB 快 9%）
> - 100K keys 真实场景：比 RocksDB 慢约 240x（已知性能限制）

---

## 核心架构

### 系统架构图

```
┌─────────────────────────────────────────────────────────────┐
│                      Application Layer                      │
│                   (Tokitai-Context Facade)                  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      FileKV Engine                          │
│                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │  MemTable    │    │ BlockCache   │    │ BloomFilter  │  │
│  │  (DashMap)   │◀──▶│  (LRU)       │◀──▶│  (Adaptive)  │  │
│  └──────┬───────┘    └──────────────┘    └──────────────┘  │
│         │                                                   │
│         ▼ (Flush)                                           │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              Segment Files (Append-Only)             │  │
│  │  segment_0.log  segment_1.log  segment_2.log  ...    │  │
│  └──────────────────────────────────────────────────────┘  │
│         │                                                   │
│         ▼ (Index)                                           │
│  ┌──────────────────────────────────────────────────────┐  │
│  │         Sparse/Dense Index (In-Memory)               │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │     WAL      │    │ Compaction   │    │  Checkpoint  │  │
│  │  (Recovery)  │    │  (Merge)     │    │  (Backup)    │  │
│  └──────────────┘    └──────────────┘    └──────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Persistence Layer                        │
│            (Segment Files, WAL, Index Files)                │
└─────────────────────────────────────────────────────────────┘
```

### 模块组织

```
src/file_kv/
├── mod.rs                    # 主模块，FileKV 核心实现
├── types.rs                  # 类型定义 (配置，指针，统计)
├── memtable.rs               # 内存表 (DashMap 实现)
├── segment.rs                # Segment 文件管理
├── wal.rs                    # WAL 操作辅助
├── bloom.rs                  # Bloom Filter 实现
├── bloom_filter_cache.rs     # Bloom Filter 缓存
├── adaptive_bloom_cache.rs   # 自适应 Bloom 缓存 (INNO-001)
├── fpr_controller.rs         # FPR 控制器
├── sparse_index.rs           # 稀疏/密集索引
├── block_cache.rs            # BlockCache (LRU 缓存)
├── flush.rs                  # Flush 触发器
├── compaction.rs             # Compaction 管理器
├── checkpoints.rs            # 检查点管理
├── incremental_manager.rs    # 增量检查点
├── recovery.rs               # 崩溃恢复
├── write_coalescer.rs        # 写入合并 (P2-012)
├── adaptive_preallocator.rs  # 自适应预分配 (P2-008)
├── async_io.rs               # 异步 IO (P3-001)
├── cache_warmer.rs           # 缓存预热
├── zone_map.rs               # ZoneMap 索引
├── range_scan.rs             # 范围扫描
├── sequential_prefetcher.rs  # 顺序预读
├── range_query_pruner.rs     # 范围查询剪枝
├── compressed_bloom.rs       # 压缩 Bloom Filter
├── timeout_control.rs        # 超时控制 (P1-015)
├── audit_log.rs              # 审计日志 (P2-013)
└── tests.rs                  # 单元测试
```

---

## 数据模型

### 核心数据结构

#### ValuePointer (值指针)

指向 Segment 文件中值的物理位置：

```rust
pub struct ValuePointer {
    pub segment_id: u64,    // Segment 文件 ID
    pub offset: u64,        // 段内偏移量
    pub key_len: u32,       // Key 长度 (PERF-003 优化)
    pub len: u32,           // Value 长度
    pub checksum: u32,      // CRC32C 校验和
}
```

**内存布局**:
```
┌─────────────┬─────────────┬─────────────┬─────────────┬─────────────┐
│ segment_id  │   offset    │   key_len   │     len     │  checksum   │
│  (8 bytes)  │  (8 bytes)  │  (4 bytes)  │  (4 bytes)  │  (4 bytes)  │
└─────────────┴─────────────┴─────────────┴─────────────┴─────────────┘
                    总计：28 字节
```

#### MemTableEntry (内存表条目)

MemTable 中的完整条目：

```rust
pub struct MemTableEntry {
    pub value: Option<Bytes>,       // 值数据 (零拷贝)
    pub pointer: Option<ValuePointer>, // Segment 指针 (刷盘后)
    pub seq_num: u64,               // 序列号 (版本控制)
    pub deleted: bool,              // 删除标记
}
```

**状态转换**:
```
新写入 → [value=Some, pointer=None, deleted=false]
刷盘后 → [value=None, pointer=Some, deleted=false]
删除后 → [value=None, pointer=Some, deleted=true]
```

#### Segment 文件格式

顺序追加的二进制格式：

```
┌──────────────────────────────────────────┐
│ Magic Number (4 bytes) = 0x54435347      │ "TCSG"
├──────────────────────────────────────────┤
│ Version (4 bytes) = 1                    │
├──────────────────────────────────────────┤
│ Entry 1                                  │
│ ├─ Key Length (4 bytes)                  │
│ ├─ Key Data (variable)                   │
│ ├─ Value Length (4 bytes)                │
│ ├─ Value Data (variable)                 │
│ └─ Checksum (4 bytes, CRC32C)            │
├──────────────────────────────────────────┤
│ Entry 2                                  │
│ └─ ...                                   │
└──────────────────────────────────────────┘
```

**单个 Entry 布局**:
```
┌─────────────┬──────────┬─────────────┬──────────┬─────────────┐
│ Key Length  │ Key Data │ Value Length│Val Data  │  Checksum   │
│   (u32)     │ (bytes)  │   (u32)     │ (bytes)  │   (u32)     │
└─────────────┴──────────┴─────────────┴──────────┴─────────────┘
```

### 索引结构

#### SparseIndex (稀疏索引)

每隔 N 条记录建立一个索引点：

```
Segment 文件:
[Entry 0] [Entry 1] ... [Entry 99] [Entry 100] ...
                             ↑
                        索引点 (key="foo", offset=12345)

内存索引:
[(key="foo", offset=12345, seq=100), ...]
```

**特点**:
- 空间效率高 (1/100 的存储开销)
- 查找需扫描 (最坏 O(interval))
- 适合读少写多场景

#### DenseIndex (密集索引)

每个条目都有精确索引：

```
Segment 文件:
[Entry 0] [Entry 1] [Entry 2] ...
   ↑         ↑         ↑
[index 0] [index 1] [index 2] ... (全内存)

内存索引:
[
  (key="a", offset=8, key_len=1, val_len=10, checksum=0x123),
  (key="b", offset=26, key_len=1, val_len=12, checksum=0x456),
  ...
]
```

**特点**:
- 查找 O(log n) (使用 BTreeMap 实现)
- 内存开销大 (每 entry 约 20-40 字节)
- 适合读密集场景


**设计说明**:
DenseIndex 使用 `BTreeMap<String, DenseIndexEntry>` 而非 `HashMap`，虽然点查找复杂度为 O(log n) 而非 O(1)，但 BTreeMap 的有序性带来了以下优势:
- **范围查询优化**: 可以高效执行范围扫描和有序遍历
- **内存局部性**: BTreeMap 的节点布局更利于 CPU 缓存
- **确定性迭代顺序**: 键按字典序排列，便于调试和一致性扫描
**性能对比**:

| 索引类型 | 查找延迟 | 内存开销 | 适用场景 |
|---------|---------|---------|---------|
| SparseIndex | O(log n) + 扫描 | 低 (1/100) | 写密集 |
| DenseIndex | O(log n) | 高 (每 entry 20-40B) | 读密集 |

---

## 写入路径

### 完整写入流程

```rust
put(key, value)
    │
    ├─ 1. 背压检查
    │   └─ if memtable.size > max_memory: 拒绝写入
    │
    ├─ 2. WAL 写入 (可选)
    │   ├─ 计算 value hash (xxh3)
    │   ├─ Base64 编码 value (用于恢复)
    │   └─ 追加到 WAL 文件
    │
    ├─ 3. MemTable 插入
    │   ├─ DashMap::insert(key, entry)
    │   ├─ 原子更新 size_bytes
    │   └─ 返回 (size, seq_num)
    │
    ├─ 4. BlockCache 更新
    │   └─ cache.put(key, value)
    │
    ├─ 5. Flush 检查
    │   ├─ if size > threshold: 触发刷盘
    │   └─ if background_flush: 异步刷盘
    │
    └─ 6. Compaction 检查
        └─ if write_count % N == 0: 触发合并
```

### 代码示例

```rust
use tokitai_context::file_kv::{FileKV, FileKVConfig};

let config = FileKVConfig::default();
let kv = FileKV::open(config)?;

// 单次写入
kv.put("user:123", b"John Doe")?;

// 批量写入 (推荐)
let entries = vec![
    ("user:124", "Jane Doe".as_bytes()),
    ("user:125", "Bob Smith".as_bytes()),
];
kv.put_batch(&entries)?;
```

### 写入优化技术

#### 1. Write Coalescing (P2-012)

快速写入合并，减少 WAL 同步次数：

```rust
// 配置
write_coalescing_enabled: true,
coalesce_window_ms: 100,  // 100ms 时间窗口
coalesce_size_kb: 64,     // 64KB 大小阈值

// 效果：1000 次写入 → 1 次 WAL sync
```

#### 2. Adaptive Pre-allocation (P2-008)

智能预分配 Segment 空间：

```rust
// 自动学习写入模式
预分配大小 = f(历史写入大小，波动率)

// 效果：减少文件扩展系统调用
```

#### 3. Async I/O (P3-001)

非阻塞异步写入：

```rust
// 后台异步写入 Segment
async_writer.spawn(write_task);

// 主线程立即返回
```

### WAL (Write-Ahead Log)

#### WAL 操作类型

```rust
pub enum WalOperation {
    Add {
        session: String,
        hash: String,
        layer: String,
    },
    Delete {
        session: String,
        hash: String,
        content: Option<Vec<u8>>,
    },
}
```

#### WAL 文件格式

```
┌──────────────────────────────────────────┐
│ WAL Entry 1                              │
│ ├─ Timestamp (8 bytes)                   │
│ ├─ Operation Type (1 byte)               │
│ ├─ Session Length (4 bytes)              │
│ ├─ Session Data (variable)               │
│ ├─ Hash (16 bytes, hex string)           │
│ ├─ Payload Length (4 bytes)              │
│ └─ Payload Data (variable, base64)       │
├──────────────────────────────────────────┤
│ WAL Entry 2                              │
│ └─ ...                                   │
└──────────────────────────────────────────┘
```

#### WAL 同步模式

```rust
pub enum WalSyncMode {
    Immediate,  // 每次 fsync (最安全)
    Batch,      // 批量 fsync (折中)
    Lazy,       // 依赖 OS (最快)
}
```

**对比**:

| 模式 | 持久化保证 | 延迟 | 推荐场景 |
|------|-----------|------|---------|
| Immediate | 100% | 基准 | 关键数据 |
| Batch | ~99% | 2-3x 快 | 默认推荐 |
| Lazy | ~90% | 5-10x 快 | 缓存数据 |

---

## 读取路径

### 完整读取流程

```rust
get(key)
    │
    ├─ 1. MemTable 查找
    │   ├─ DashMap::get(key)
    │   └─ if found: return value (最快)
    │
    ├─ 2. BlockCache 查找
    │   ├─ CacheKey::new(segment_id, offset)
    │   ├─ DashMap::get(key)
    │   └─ if hit: return Arc<[u8]> (零拷贝)
    │
    ├─ 3. BloomFilter 检查
    │   ├─ bloom.contains(key)
    │   └─ if false: return None (快速路径)
    │
    ├─ 4. Index 查找
    │   ├─ DenseIndex: O(log n) 定位 (BTreeMap)
    │   └─ SparseIndex: O(log n) + 扫描
    │
    ├─ 5. Segment 读取
    │   ├─ mmap 映射文件
    │   ├─ 读取数据块
    │   └─ 校验 checksum
    │
    └─ 6. BlockCache 回填
        └─ cache.put(key, value)
```

### 代码示例

```rust
// 单次读取
if let Some(value) = kv.get("user:123")? {
    println!("Found: {:?}", value);
}

// 批量读取
let keys = vec!["user:123", "user:124", "user:125"];
for key in &keys {
    if let Some(value) = kv.get(key)? {
        println!("{}: {:?}", key, value);
    }
}

// 范围扫描
let iter = kv.range_scan("user:", "user:~")?;
for (key, value) in iter {
    println!("{}: {:?}", key, value);
}
```

### 读取优化技术

#### 1. Persistent Mmap (PERF-002)

持久内存映射，避免重复创建：

```rust
// 开启后，Segment 文件在打开时创建一次 mmap
persistent_mmap_enabled: true

// 效果：读取延迟降低 80-90%
```

#### 2. Read-ahead (P4-001)

智能预读后续数据块：

```rust
// 配置
readahead_multiplier: 4  // 预读 4 个 block

// 效果：顺序读取吞吐量提升 2-4x
```

#### 3. DenseIndex (PERF-003)

全内存密集索引：

```rust
// 开启后，每个 entry 都有精确索引
in_memory_block_index_enabled: true

// 效果：读取延迟接近 RocksDB (15µs vs 10µs)
```

### Bloom Filter 优化

#### 自适应 Bloom 缓存 (INNO-001)

多层 Bloom Filter 缓存，动态调整误报率：

```
┌─────────────────────────────────────┐
│  L1 Cache (全内存，FPR=0.001)       │
│  ├─ 最近使用的 Bloom Filter         │
│  └─ 访问延迟：~35ns                 │
├─────────────────────────────────────┤
│  L2 Cache (RLE 压缩，FPR=0.01)      │
│  ├─ 压缩 Bloom Filter               │
│  └─ 访问延迟：~4µs (含解压)         │
├─────────────────────────────────────┤
│  L3 Cache (磁盘，FPR=0.05)          │
│  ├─ 持久化 Bloom Filter 文件        │
│  └─ 访问延迟：~100µs (磁盘 IO)      │
└─────────────────────────────────────┘
```

**FPR 控制器**:

```rust
// 自动调整 Bloom Filter 误报率
level = f(访问频率，内存压力)

// 6 个等级：
// L0: FPR=0.0001 (极致性能)
// L1: FPR=0.001
// L2: FPR=0.01
// L3: FPR=0.05
// L4: FPR=0.1
// L5: FPR=0.2 (节省内存)
```

---

## 核心组件

### 1. MemTable (内存表)

**实现**: DashMap 无锁并发  
**复杂度**: O(1) 插入/查找  
**配置**:

```rust
MemTableConfig {
    flush_threshold_bytes: 4 * 1024 * 1024,  // 4MB
    max_entries: 100_000,                     // 10 万条
    max_memory_bytes: 64 * 1024 * 1024,       // 64MB (背压阈值)
}
```

**关键方法**:

```rust
// 插入键值对
fn insert(&self, key: String, value: &[u8]) -> (usize, u64)

// 获取值
fn get(&self, key: &str) -> Option<(Option<Bytes>, Option<ValuePointer>, bool)>

// 检查是否需要刷盘
fn should_flush(&self) -> bool

// 检查背压
fn should_apply_backpressure(&self) -> bool
```

**优化技术**:
- DashMap 实现无锁并发
- AtomicUsize 原子更新大小
- Bytes 零拷贝存储
- Relaxed 内存序 (性能优化)

### 2. SegmentFile (段文件)

**实现**: 顺序追加写入  
**特点**: 不可变 (append-only)  
**方法**:

```rust
// 追加写入
fn append(&self, key: &str, value: &[u8]) -> ContextResult<(u64, u32, u32)>

// 按位置读取
fn get_at(&self, offset: u64, len: u32) -> ContextResult<Vec<u8>>

// 范围扫描
fn scan_range(&self, start: &str, end: &str) -> ContextResult<Vec<(String, Vec<u8>)>>
```

**优化技术**:
- BufWriter 缓冲写入
- mmap 零拷贝读取
- 预分配空间 (减少扩展)
- 持久 mmap (避免重复创建)

### 3. BlockCache (块缓存)

**实现**: DashMap + LRU  
**策略**: 最近最少使用淘汰  
**配置**:

```rust
BlockCacheConfig {
    max_items: 50_000,                        // 5 万项
    max_memory_bytes: 256 * 1024 * 1024,      // 256MB
    min_block_size: 32,                       // 最小 32 字节
    max_block_size: 4 * 1024 * 1024,          // 最大 4MB
}
```

**性能**:
- 命中：~115 ns (DashMap 无锁读取)
- 未命中：O(1) (返回 None)

**统计**:

```rust
CacheStats {
    hits: 10000,
    misses: 1000,
    hit_rate: 0.909,  // 90.9% 命中率
    items: 5000,
    memory_usage: 128_000_000,  // 128MB
}
```

### 4. SparseIndex / DenseIndex (索引)

**SparseIndex**:
```rust
SparseIndexConfig {
    index_interval: 100,  // 每 100 条一个索引点
}
```

**DenseIndex**:
```rust
DenseIndexConfig {
    enabled: true,
    max_entries: 1_000_000,  // 100 万条上限
}
```

**IndexManager**:
```rust
// 管理所有 Segment 的索引
pub struct IndexManager {
    indexes: RwLock<BTreeMap<u64, IndexType>>,
    index_dir: PathBuf,
}
```

### 5. BloomFilter (布隆过滤器)

**实现**: bloom crate (ASMS 算法)  
**误报率**: 可配置 (0.001 - 0.2)  
**大小**: `n * log2(1/p) / ln(2)` bits

**示例**:
```rust
// 100 万个元素，1% 误报率
// 大小 ≈ 1000000 * log2(1/0.01) / 0.69 ≈ 9.6 Mbits ≈ 1.2 MB
```

**操作**:
```rust
// 插入
bloom.insert(&key);

// 查询
if bloom.contains(&key) {
    // 可能存在 (需要进一步检查)
} else {
    // 肯定不存在 (快速路径)
}
```

### 6. CompactionManager (合并管理器)

**策略**: 后台合并小 Segment  
**阈值**:

```rust
CompactionConfig {
    min_segments: 4,   // 最少 4 个 Segment 触发
    max_segments: 8,   // 最多合并 8 个
    size_threshold: 100 * 1024 * 1024,  // 100MB
}
```

**过程**:
1. 选择待合并 Segment (按大小/数量)
2. 创建新 Segment (合并所有 Entry)
3. 更新索引 (指向新 Segment)
4. 删除旧 Segment (回收空间)

**效果**:
- 减少 Segment 数量
- 回收删除数据
- 提升读取性能

---

## 配置指南

### 基础配置

```rust
use tokitai_context::file_kv::{FileKV, FileKVConfig};

let config = FileKVConfig {
    // 目录配置
    segment_dir: PathBuf::from("./data/segments"),
    wal_dir: PathBuf::from("./data/wal"),
    index_dir: PathBuf::from("./data/index"),
    checkpoint_dir: PathBuf::from("./data/checkpoints"),
    
    // MemTable 配置
    memtable_flush_threshold_bytes: 4 * 1024 * 1024,  // 4MB
    memtable_max_entries: 100_000,
    
    // BlockCache 配置
    block_cache_max_items: 50_000,
    block_cache_max_memory_bytes: 256 * 1024 * 1024,  // 256MB
    
    // WAL 配置
    enable_wal: true,
    wal_max_size_bytes: 100 * 1024 * 1024,  // 100MB
    wal_max_files: 5,
    
    // Compaction 配置
    compaction_min_segments: 4,
    compaction_max_segments: 8,
    
    // Bloom Filter 配置
    enable_bloom: true,
    bloom_fpr: 0.01,  // 1% 误报率
    
    // 后台任务
    enable_background_flush: true,
    background_flush_interval_ms: 1000,  // 1 秒
    
    ..Default::default()
};

let kv = FileKV::open(config)?;
```

### 性能预设配置

#### 保守模式 (数据安全优先)

```rust
use tokitai_context::file_kv::AggressiveConfig;

let config = FileKVConfig {
    aggressive: AggressiveConfig::conservative(),
    ..Default::default()
};

// AggressiveConfig::conservative():
// - dense_index_enabled: false
// - readahead_multiplier: 0
// - wal_sync_mode: WalSyncMode::Immediate
// - cache_max_memory_bytes: 64MB
// - persistent_mmap_enabled: false
```

#### 平衡模式 (默认推荐)

```rust
let config = FileKVConfig {
    aggressive: AggressiveConfig::balanced(),
    ..Default::default()
};

// AggressiveConfig::balanced():
// - dense_index_enabled: true
// - readahead_multiplier: 2
// - wal_sync_mode: WalSyncMode::Batch
// - cache_max_memory_bytes: 256MB
// - persistent_mmap_enabled: true
```

#### 性能模式 (读取速度优先)

```rust
let config = FileKVConfig {
    aggressive: AggressiveConfig::performance(),
    ..Default::default()
};

// AggressiveConfig::performance():
// - dense_index_enabled: true
// - readahead_multiplier: 4
// - wal_sync_mode: WalSyncMode::Batch
// - cache_max_memory_bytes: 1GB
// - persistent_mmap_enabled: true
// - in_memory_block_index_enabled: true
```

#### 极限模式 (不计代价)

```rust
let config = FileKVConfig {
    aggressive: AggressiveConfig::extreme(),
    ..Default::default()
};

// AggressiveConfig::extreme():
// - dense_index_enabled: true
// - readahead_multiplier: 8
// - wal_sync_mode: WalSyncMode::Lazy
// - cache_max_memory_bytes: 4GB
// - persistent_mmap_enabled: true
// - in_memory_block_index_enabled: true
```

### 内存占用估算

```rust
let config = AggressiveConfig::performance();
let estimate = config.estimated_memory_usage(1_000_000);

println!("{}", estimate);
// 输出:
// Total: 1024.00 MB
//   - BlockCache: 1024.00 MB
//   - DenseIndex: 20.00 MB
//   - BlockIndex: 24.41 MB
```

---

## 性能基准

> **性能数据汇总**: 完整的性能对比数据（含与 RocksDB 的公平对比、测试日期、时效声明）已统一维护在 [README.md](../../README.md#-性能表现与-rocksdb-公平对比) 中。本节仅保留 FileKV 特有的纯内存微基准数据和内存分析数据。

### 测试环境

- **OS**: Linux
- **构建**: Release (`cargo build --release`)
- **测试**: `cargo bench --features benchmarks`

### 单次写入性能

| 操作 | 后端 | 延迟 | 对比目标 | 测试类别 |
|------|------|------|---------|---------|
| Write 64B | No WAL | **92.5 ns** | 54x 超越 (非生产配置) | Category 2 |
| Write 1KB | No WAL | **105.6 ns** | 47x 超越 (非生产配置) | Category 2 |
| Write 4KB | No WAL | **174.2 ns** | 29x 超越 (非生产配置) | Category 2 |
| Write 64B | With WAL | **1.68 µs** | 生产配置 | Category 2 |
| Write 1KB | With WAL | **3.83 µs** | 生产配置 | Category 2 |
| Write 4KB | With WAL | **9.89 µs** | 生产配置 | Category 2 |

> **注**: "No WAL" 模式不适用于需要数据持久化的场景。与 RocksDB 的公平对比数据见 [README.md](../../README.md#-性能表现与-rocksdb-公平对比)。

### 批量写入性能

| 批量大小 | 无 WAL | 有 WAL | 每项延迟 (无 WAL) |
|---------|--------|-------|----------------|
| 10 items | 91 µs | 120 µs | 9.1 µs |
| 100 items | 103 µs | 279 µs | 1.03 µs |
| 1000 items | **228 µs** | **1.87 ms** | **0.228 µs** |

### 读取性能

> 读取性能对比数据（含与 RocksDB 的公平对比）详见 [README.md](../../README.md#-性能表现与-rocksdb-公平对比)。
> FileKV 特有微基准数据：
> - Hot Read (1KB, Cache Hit): **120 ns** (Category 2)
> - DenseIndex Direct: **~15 µs** (Category 2)

### 自适应 Bloom 缓存性能

| 操作 | 延迟 | 吞吐量 | 测试类别 |
|------|------|--------|---------|
| L1 Insert | ~50 µs | ~20 Kelem/s | Category 1 |
| L1 Get (Hit) | **35.8 ns** | **27.9 Melem/s** | Category 1 (纯内存) |
| L2 Get (Hit) | **35.9 ns** | **27.8 Melem/s** | Category 1 (纯内存) |
| Contains (Positive) | **70.3 ns** | **14.2 Melem/s** | Category 1 (纯内存) |
| Contains (Negative) | **72.5 ns** | **13.8 Melem/s** | Category 1 (纯内存) |

> **注意**: 自适应 Bloom 缓存性能数据来自**纯内存微基准测试** (Category 1)，不包含磁盘 IO 或完整 KV 查询。这展示了 Bloom Filter 缓存架构的效率，而非完整 KV 操作性能。

### 内存使用分析

| 阶段 | 内存占用 | 增量 |
|------|---------|------|
| 空实例 | 3.87 MB | +780 KB |
| 1K 写入后 | 4.19 MB | +324 KB |
| 10K 写入后 | 6.73 MB | +2.54 MB |
| 100K 写入后 | 28.79 MB | +22.06 MB |
| 100K 读取后 | 31.84 MB | +3.05 MB (缓存) |

**内存效率**:
- 总开销 (100K 条目): 28.73 MB
- 原始数据大小：6.87 MB
- **开销比率：4.18x** (良好，2-5x 范围)
- 每条目内存：301 B

### 对比 RocksDB

> 与 RocksDB 的完整公平对比数据、测试日期、公平性说明详见 [README.md](../../README.md#-性能表现与-rocksdb-公平对比)。

---

## 最佳实践

### 1. 批量写入

```rust
// ❌ 不推荐：循环单次写入
for i in 0..1000 {
    kv.put(&format!("key_{}", i), b"value")?;
}

// ✅ 推荐：批量写入
let entries: Vec<(&str, &[u8])> = (0..1000)
    .map(|i| (format!("key_{}", i).as_str(), b"value"))
    .collect();
kv.put_batch(&entries)?;  // 38% 性能提升
```

### 2. 合理配置 WAL

```rust
// 关键数据：Immediate 模式
let config = FileKVConfig {
    aggressive: AggressiveConfig {
        wal_sync_mode: WalSyncMode::Immediate,
        ..Default::default()
    },
    ..Default::default()
};

// 缓存数据：Lazy 模式
let config = FileKVConfig {
    aggressive: AggressiveConfig {
        wal_sync_mode: WalSyncMode::Lazy,
        ..Default::default()
    },
    enable_wal: false,  // 可完全禁用
    ..Default::default()
};
```

### 3. 范围扫描优化

```rust
// 使用前缀扫描
let iter = kv.range_scan("user:", "user:~")?;

// 使用 ZoneMap 剪枝 (自动)
// ZoneMap 记录每个 Segment 的 key 范围
// 自动跳过不包含目标 key 的 Segment
```

### 4. 缓存预热

```rust
// 启动时预热缓存
let config = FileKVConfig {
    cache_warming_enabled: true,
    cache_warming_strategy: CacheWarmingStrategy::HotKeys,
    ..Default::default()
};
```

### 5. 背压处理

```rust
// 检查背压
if kv.memtable_should_backpressure() {
    // 等待或拒绝写入
    std::thread::sleep(Duration::from_millis(100));
}

// 或者使用批量写入
let entries = vec![...];
kv.put_batch(&entries)?;  // 自动处理背压
```

### 6. 监控统计

```rust
// 定期获取统计
let stats = kv.get_stats();
println!("写入：{}", stats.write_count);
println!("读取：{}", stats.read_count);
println!("缓存命中率：{:.2}%", stats.cache_hit_rate * 100.0);
println!("MemTable 大小：{} MB", stats.memtable_size / 1024 / 1024);
```

---

## 故障排查

### 常见问题

#### 1. 写入性能下降

**症状**: 写入延迟从 100ns 增加到 10µs+

**可能原因**:
- MemTable 频繁刷盘
- WAL 同步瓶颈
- Compaction 占用资源

**排查步骤**:
```rust
let stats = kv.get_stats();
println!("MemTable 大小：{}", stats.memtable_size);
println!("刷盘次数：{}", stats.flush_count);
println!("Compaction 次数：{}", stats.compaction_count);
```

**解决方案**:
- 增加 `memtable_flush_threshold_bytes`
- 使用 `WalSyncMode::Batch` 或 `Lazy`
- 调整 `compaction_min_segments`

#### 2. 读取性能下降

**症状**: 读取延迟从 100ns 增加到 10µs+

**可能原因**:
- BlockCache 命中率低
- Segment 文件过多
- Index 未命中

**排查步骤**:
```rust
let stats = kv.get_stats();
println!("缓存命中率：{:.2}%", stats.cache_hit_rate * 100.0);
println!("Segment 数量：{}", stats.segment_count);
println!("Bloom 负向次数：{}", stats.bloom_negative_count);
```

**解决方案**:
- 增加 `block_cache_max_items`
- 手动触发 Compaction
- 启用 DenseIndex

#### 3. 内存占用过高

**症状**: 进程内存持续增长

**可能原因**:
- MemTable 未及时刷盘
- BlockCache 过大
- DenseIndex 占用高

**排查步骤**:
```rust
let estimate = config.estimated_memory_usage(1_000_000);
println!("{}", estimate);
```

**解决方案**:
- 降低 `memtable_max_memory_bytes`
- 降低 `block_cache_max_memory_bytes`
- 禁用 `dense_index_enabled`

#### 4. 崩溃恢复失败

**症状**: 重启后数据丢失

**可能原因**:
- WAL 文件损坏
- 未正确配置 WAL

**排查步骤**:
```bash
# 检查 WAL 文件
ls -la ./data/wal/

# 检查 WAL 内容
cargo run -- example wal_inspect ./data/wal/000001.wal
```

**解决方案**:
- 使用 `WalSyncMode::Immediate`
- 增加 `wal_max_files`
- 启用增量检查点

---

## 特性集成状态

> **说明**: 下表汇总了 FileKV 所有特性的实际集成状态。文档中的功能描述可能包含框架已实现但尚未集成到生产路径的特性，请以本表为准。

### 集成状态图例

| 状态 | 含义 |
|------|------|
| ✅ 已集成 | 功能已集成到生产路径，正常使用即可生效 |
| 🟡 可选/实验性 | 功能代码完整，需显式启用，或有使用限制 |
| 🟠 规划/进行中 | 框架已实现，但尚未集成到热路径 |

### 特性集成状态总表

| # | 特性 | 集成状态 | 启用方式 | 限制说明 |
|---|------|---------|---------|---------|
| 1 | LSM-Tree 架构 | ✅ 已集成 | 默认启用 | 无 |
| 2 | MemTable (DashMap) | ✅ 已集成 | 默认启用 | 无 |
| 3 | BlockCache (LRU) | ✅ 已集成 | 默认启用 | 无 |
| 4 | Bloom Filter | ✅ 已集成 | 默认启用 | 无 |
| 5 | WAL (Write-Ahead Log) | ✅ 已集成 | `enable_wal=true` | 无 |
| 6 | Compaction | ✅ 已集成 | 默认启用 (后台线程) | 无 |
| 7 | Sparse/Dense Index | ✅ 已集成 | `dense_index_enabled` | DenseIndex 内存开销较大 |
| 8 | Zone Map Pruning | ✅ 已集成 | 默认启用 (Range Scan) | 仅范围扫描受益 |
| 9 | Adaptive Bloom Cache | ✅ 已集成 | 默认启用 | 纯内存性能数据，非完整 KV 操作 |
| 10 | Range Scan Readahead | ✅ 已集成 | `readahead_multiplier > 0` | 仅顺序扫描受益 |
| 11 | Write Coalescing | 🟡 可选/实验性 | `write_coalescing_enabled=true` | 框架完整，需验证 |
| 12 | Adaptive Pre-allocation | 🟡 可选/实验性 | 默认启用 | 学习算法简单 |
| 13 | Async I/O | 🟡 可选/实验性 | `async_io_enabled=true` | 仅 `put_buffered_async` 支持，主写入路径仍为同步 |
| 14 | Timeout Control | 🟠 规划/进行中 | 自动 (仅后台操作) | 框架完整，仅保护 checkpoint 等后台操作，热路径集成规划中 |
| 15 | Memory Tracker | 🟠 规划/进行中 | 自动 | 框架已实现，`set_*_bytes()` 存在但未被调用，当前数据为估算值 |
| 16 | Compaction Trigger | 🟠 规划/进行中 | 自动 | 自适应策略已实现，当前使用固定 write count 计数器触发 |
| 17 | Amplification Analysis | 🟡 可选/实验性 | `get_stats()` | WAF/RAF 为公式估算值，仅 SAF 为真实测量 |
| 18 | Sequential Prefetch | ✅ 部分集成 | 自动 (Range Scan) | 范围扫描已集成，单点查询 (`get()`) 未使用预取 |
| 19 | Cache Warmer | ✅ 已集成 | `cache_warming_enabled=true` | Frequent 策略当前为 SizeBased 实现，非真正的访问频率 |
| 20 | WAL Batch | ✅ 已集成 | `put_batch()` | 批量 flush，定期 fsync，非批量 fsync |
| 21 | Incremental Checkpoint | ✅ 已集成 | 手动调用 `create_incremental_checkpoint()` | 需调用方传入状态，非自动快照 |
| 22 | Audit Log | 🟡 可选/实验性 | `audit_log.enabled=true` | 框架完整，需显式启用 |
| 23 | Persistent Mmap | ✅ 已集成 | `persistent_mmap_enabled=true` | 无 |
| 24 | FPR Controller | ✅ 已集成 | 默认启用 | 自动调整 Bloom Filter 误报率 |

---

## 高级特性

### 1. 增量检查点 (P2-009)

**功能**: 增量备份，支持时间点恢复

```rust
// 创建增量检查点
kv.create_incremental_checkpoint()?;

// 恢复到检查点
kv.restore_checkpoint(checkpoint_id)?;

// 列出检查点
let checkpoints = kv.list_checkpoints()?;
```

**原理**:
```
Checkpoint 1 (Full): [所有数据]
Checkpoint 2 (Delta): [仅变更数据]
Checkpoint 3 (Delta): [仅变更数据]

恢复：Checkpoint 1 + Delta 2 + Delta 3
```

> **实现状态**: ✅ 已集成 | **限制**: 需调用方手动传入状态，非自动快照。

### 2. 审计日志 (P2-013)

**功能**: 合规性审计，操作追踪

```rust
let config = FileKVConfig {
    audit_log: AuditLogConfig {
        enabled: true,
        log_dir: PathBuf::from("./data/audit"),
        max_file_size_mb: 100,
        retention_days: 30,
    },
    ..Default::default()
};
```

**审计内容**:
- 操作类型 (Put/Delete/Get)
- 操作时间戳
- Key 列表
- Value 哈希
- 延迟统计

> **实现状态**: 🟡 可选/实验性 | **启用**: `audit_log.enabled=true` | 框架完整，需显式启用。

### 3. 超时控制 (P1-015)

**功能**: 操作超时保护

```rust
let config = FileKVConfig {
    timeout_config: TimeoutConfig {
        read_timeout_ms: 1000,
        write_timeout_ms: 5000,
        compaction_timeout_ms: 60000,
    },
    ..Default::default()
};
```

> **实现状态**: 🟠 框架已实现，热路径集成规划中 | **当前**: 仅保护 checkpoint 等后台操作，主读写路径尚未集成超时控制。

### 4. 范围查询优化

**ZoneMap 索引**:
```rust
// 自动记录每个 Segment 的 key 范围
ZoneMap {
    min_key: "user:001",
    max_key: "user:999",
}

// 查询时自动跳过不相关的 Segment
```

**SequentialDetector**:
```rust
// 检测顺序读取模式
// 自动触发预读
if detector.is_sequential() {
    prefetcher.prefetch_next_blocks();
}
```

> **实现状态**: ✅ 已集成 | ZoneMap 默认启用，Sequential Prefetch 仅在 Range Scan 中生效，单点查询 (`get()`) 未使用预取。

### 5. 异步 IO (P3-001)

**功能**: 非阻塞写入

```rust
let config = FileKVConfig {
    async_io_enabled: true,
    async_io_max_concurrent_writes: 4,
    async_io_max_queue_depth: 1000,
    async_io_write_timeout_ms: 5000,
    ..Default::default()
};
```

**效果**:
- 主线程立即返回
- 后台异步写入 Segment
- 适合高吞吐场景

> **实现状态**: 🟡 可选/实验性功能 | **启用**: `async_io_enabled=true` | **限制**: 仅 `put_buffered_async()` 方法支持，主 `put()` 写入路径仍为同步 WAL。

---

## API 参考

### FileKV 核心 API

```rust
impl FileKV {
    // 打开存储
    pub fn open(config: FileKVConfig) -> ContextResult<Self>;
    
    // 单次写入
    pub fn put(&self, key: &str, value: &[u8]) -> ContextResult<()>;
    
    // 批量写入
    pub fn put_batch(&self, entries: &[(&str, &[u8])]) -> ContextResult<usize>;
    
    // 单次读取
    pub fn get(&self, key: &str) -> ContextResult<Option<Vec<u8>>>;
    
    // 批量读取
    pub fn get_batch(&self, keys: &[&str]) -> ContextResult<Vec<Option<Vec<u8>>>>;
    
    // 删除
    pub fn delete(&self, key: &str) -> ContextResult<()>;
    
    // 范围扫描
    pub fn range_scan(&self, start: &str, end: &str) -> ContextResult<RangeScanIterator>;
    
    // 获取统计
    pub fn get_stats(&self) -> FileKVStats;
    
    // 重置统计
    pub fn reset_stats(&self);
    
    // 手动刷盘
    pub fn flush_memtable(&self) -> ContextResult<()>;
    
    // 手动 Compaction
    pub fn run_compaction(&self) -> ContextResult<()>;
    
    // 检查点
    pub fn create_checkpoint(&self) -> ContextResult<u64>;
    pub fn restore_checkpoint(&self, id: u64) -> ContextResult<()>;
    pub fn list_checkpoints(&self) -> ContextResult<Vec<CheckpointInfo>>;
}
```

### 配置 API

```rust
impl FileKVConfig {
    // 默认配置
    pub fn default() -> Self;
    
    // 验证配置
    pub fn validate(&self) -> FileKVConfigValidation;
}

impl AggressiveConfig {
    // 预设配置
    pub fn conservative() -> Self;
    pub fn balanced() -> Self;
    pub fn performance() -> Self;
    pub fn extreme() -> Self;
    
    // 内存估算
    pub fn estimated_memory_usage(&self, entries: usize) -> MemoryUsageEstimate;
}
```

### 统计 API

```rust
impl FileKVStats {
    // 读取计数
    pub read_count: AtomicU64,
    
    // 写入计数
    pub write_count: AtomicU64,
    
    // MemTable 大小
    pub memtable_size: AtomicUsize,
    
    // MemTable 条目数
    pub memtable_entries: AtomicUsize,
    
    // Segment 数量
    pub segment_count: AtomicUsize,
    
    // 总大小
    pub total_size_bytes: AtomicUsize,
    
    // 刷盘次数
    pub flush_count: AtomicU64,
    
    // Compaction 次数
    pub compaction_count: AtomicU64,
    
    // 缓存命中率
    pub fn cache_hit_rate(&self) -> f64;
}
```

---

## 附录

### A. 术语表

| 术语 | 英文 | 解释 |
|------|------|------|
| MemTable | MemTable | 内存缓冲表 |
| Segment | Segment | 数据段文件 |
| WAL | Write-Ahead Log | 预写日志 |
| Bloom Filter | Bloom Filter | 布隆过滤器 |
| Compaction | Compaction | 合并压缩 |
| Checkpoint | Checkpoint | 检查点备份 |
| Backpressure | Backpressure | 背压控制 |

### B. 参考资料

- [LSM-Tree 原始论文](https://www.cs.umb.edu/~poneil/lsmtree.pdf)
- [RocksDB 官方文档](https://rocksdb.org/docs/)
- [Bloom Filter 论文](https://www.eecs.harvard.edu/~michaelm/postscripts/im2005b.pdf)

### C. 相关文档

- [USAGE.md](../../USAGE.md) - 完整使用指南
- [ARCHITECTURE.md](../ARCHITECTURE.md) - 系统架构
- [PERFORMANCE_BENCHMARK_REPORT.md](PERFORMANCE_BENCHMARK_REPORT.md) - 性能报告

---

**最后更新**: 2026-04-14 (v0.4.0)
**维护者**: Tokitai Team
**许可证**: MIT OR Apache-2.0

### D. 版本历史

#### v0.4.0 (2026-04-14) - 性能优化
- Dense Index 快速路径实现，热缓存读取 270x 提升 (61.92µs → 0.229µs)
- BlockCache 多分片架构，支持 shrink_to()/grow_to() 动态调整
- 9 个高并发 ignored 测试解除，28 个集成测试全部通过
- Bloom Filter V2 序列化格式实现（技术限制已文档化）
- 测试数 431/431 (100%)，doctests 15/15，编译零警告

#### v0.3.1 (2026-04-14) - 示例代码修复
- 修复 examples/basic_usage.rs 和 performance_demo.rs 的 audit_log 路径
- 从 `tokitai_filekv::audit_log` 修正为 `tokitai_filekv::ops::audit_log`
- 测试数 410 → 431

#### v0.3.0 (2026-04-13) - Phase 4 特性 + Phase 0/1 修复
- Phase 4 特性完整：字典压缩、rebalance 决策+执行引擎、频率感知迁移
- Phase 0/1 关键修复：SequentialPrefetch 消费、BlockCache 字节级限制、rebalance 执行引擎
- 测试覆盖 410/410 (100%)，编译零警告
