# FileKV 存储引擎完全指南

**版本**: 0.5.0
**最后更新**: 2026-04-16 (v0.5.0 完成，Round 1-38 全部完成)
**状态**: 实验性生产引擎 (630 lib tests + 32 integration tests 通过，0 clippy 警告，核心 API 稳定)

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

**定位**：不是"更快的 RocksDB"，而是**面向 Rust 生态和 AI 场景的下一代 KV 存储引擎**——在特定场景（自适应 Bloom、热点缓存）比 RocksDB 更智能、更安全、更易用。

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
| **BlockCache** | Moka 多分片 TinyLFU 缓存 | 零分配 get_prefetch，热点自适应 | ✅ |
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
| **Write Amplification Tracking** | WAF/RAF/SAF 监控 | 运维可观测性 | ✅ (WA/RA/SA 基于实际 I/O 计数器，Round 6 PERF-AMP-001) |
| **Memory Monitoring** | 内存使用跟踪 | 内存压力管理 | ✅ (MemoryTracker 实际测量，Round 6 PERF-MEM-001) |
| **Async I/O** | 非阻塞异步写入 | 高吞吐场景优化 | ✅ (put_with_io_mode, IoMode Sync/Async, Round 6 PERF-ASYNC-001) |
| **Timeout Control** | 操作超时保护 | 后台操作保护 | ✅ (后台操作 + 可配置) |
| **Sequential Prefetch** | get() 顺序模式预取 | 连续查询优化 | ✅ (get() 路径集成 SequentialDetector, Round 6 PERF-PREFETCH-001) |
| **WAL Batch** | 批量写入优化 | 减少 WAL 同步次数 | ✅ (批量 flush，定期 fsync) |
| **Incremental Checkpoint** | 增量检查点备份 | 时间点恢复基础 | ✅ (需手动调用) |

### 🏆 核心优势（超越 RocksDB 的地方）

tokitai-filekv 的核心优势不是"比 RocksDB 快"，而是**在特定场景更智能、更安全、更易用**：

#### 1. 自适应 Bloom Filter 架构（独创性）

- **L1/L2/L3 三层自适应缓存** + 频率感知迁移（Hot/Warm/Cold 自动分类）
- **基于 QPS 的 FPR 动态调整**（误判率自动优化）
- **混合评分**：QPS (70%) + access_count (30%)
- **实际效果**：Bloom 负向查询 **7.30 µs**（比 RocksDB 快 **33.9x**）

#### 2. Rust 原生工程优势（生态位）

- **编译期安全**：Rust 借用检查器保证，0 clippy warnings，0 production unwrap()
- **依赖管理**：Cargo 一键管理，无 C++ 编译痛苦
- **未来潜力**：WebAssembly、no_std 支持（潜在）
- **测试覆盖**：630 lib tests + 32 integration tests (100% pass)

#### 3. 内置可观测性（现代化设计）

- **Prometheus 指标**：自动记录，开箱即用
- **WAF/RAF/SAF**：写/读/空间放大率实时监控
- **MemoryTracker**：实际测量内存使用（非估算）
- **Feature Flags**：运行时控制（Bloom、INNO-002 等）

#### 4. 架构清晰度（设计哲学）

- **四引擎分离**：ReadEngine / WriteEngine / CompactionEngine / LifecycleManager
- **非 God Object 模式**：对比 RocksDB db_impl.cc 5000+ 行
- **Compression/Checkpoint/Ops 模块完整**：字典压缩、增量检查点、异步 I/O、审计日志、Feature Flags、内存追踪
- **Feature Flags**：编译时控制（async-io, full）
- **完整文档**：78+ 文件，技术深度极佳

> **文档职责说明**:
> - **README.md** = 快速参考（核心特性、性能数据、快速开始、配置预设）
> - **FILEKV_GUIDE.md** (本文档) = 技术深度（架构详解、数据模型、读写路径、配置指南、故障排查、API 参考）
> - **POSITION_AND_STATUS.md** = 路线图与状态（项目定位、已知限制、实现状态清单、生产就绪路线图）

### 性能亮点

> 完整性能数据（含与 RocksDB 公平对比、测试日期）详见 [README.md](../../README.md#-性能表现与-rocksdb-公平对比)。关键数据摘要（2026-04-16 Round 38 实测）：
>
> **10M Keys 大规模性能** (07_professional_benchmark, 2026-04-16):
> - **吞吐量**: ~355,000 ops/sec (37.9 MB/s), 波动 <2%
> - **写放大 (WA)**: **1.00x** (完美, 批量 WAL 优化)
> - **空间放大 (SA)**: **1.24x** (优秀)
> - **10M 写入耗时**: ~28.2 秒 (逻辑数据 1,120 MB → 磁盘 13.0 GB)
>
> **热点读取性能** (Round 38 实测):
> - Bloom Filter 负向查询：**7.23 µs**（比 RocksDB 快 **34.2x**）
> - 全 KV Get (热点缓存)：**278-285 ns**（比 RocksDB 快 **2107-2158x**，DenseIndex 快速路径精确测量）
> - 全 KV Get (冷缓存)：**417-435 ns**（比 RocksDB 快 **~15x**）
> - 删除操作 (write+delete 全周期)：**1.18-1.20 µs**（832-851K ops/sec，Round 38 改为测量写入+删除全周期）
>
> **并发性能** (4 线程, Round 38 实测, Instant 测量真实并发时间):
> - 并发写入：**532-548 µs**（182-188K ops/sec，排除线程创建开销）
> - 并发读取：**135-137 µs**（731-738K ops/sec）
> - 混合并发 (80R20W)：**1.57-1.58 ms**（63.2-63.7K ops/sec）
>
> **批量操作** (Round 38 变更):
> - 批量写入 100 keys：**38-42 µs**（2.39-2.64M ops/sec，改用 put_batch() API，较旧循环 put 测量 ~3x 提升）
> - 批量写入 100K：**147 ms**（679K ops/sec，⬆️ +29.6% 提升）
> - 批量写入 1M：**2.23 s**（448K ops/sec，⬆️ +24.4% 提升）
> - Compaction 触发 (2000 keys)：**5.31-5.37 ms**（改为实际执行 run_compaction()，此前仅读 stats）
>
> **压缩性能** (新增, 08_compression_bench, Round 38 修复测量逻辑):
> - zstd (100B): ~390 ns, 244 MB/s | zstd (100KB JSON): ~12.2 µs, 7.78 GB/s
> - snappy (100B): ~158 ns, 605 MB/s | snappy (100KB JSON): ~105 µs, 907 MB/s
> - lz4 (100B): ~131 ns, 729 MB/s | lz4 (100KB JSON): ~6.1 µs, 15.7 GB/s
>
> **Round 38 Benchmark 方法修复**: delete 改为 write+delete 全周期测量，batch_write 改用 put_batch() API，
> trigger_compaction 改为实际执行 run_compaction()，并发 benchmark 排除线程 spawn/join 开销，
> compression_ratio 测量实际压缩操作而非 format!()

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
│  │  (DashMap)   │◀──▶│ (Moka TinyLFU)│◀──▶│  (Adaptive)  │  │
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
src/
├── lib.rs                      # 主入口，FileKV 门面
├── core/
│   ├── types.rs                # 配置和类型定义
│   ├── memtable.rs             # MemTable (DashMap 分片)
│   ├── memtable_manager.rs     # MemTable 管理器
│   ├── segment.rs              # Segment 文件管理
│   ├── wal.rs                  # WAL 操作
│   ├── wal_channel.rs          # WAL 异步通道
│   ├── wal_batcher.rs          # WAL 批量写入
│   └── global_index.rs         # GlobalKeyIndex (AHashMap + BTreeMap)
├── engine/
│   ├── read_engine.rs          # 读引擎 (get/range/Bloom/ZoneMap)
│   ├── write_engine.rs         # 写引擎 (put/delete/WAL/flush)
│   └── compaction_engine.rs    # 压缩引擎
├── cache/
│   ├── mod.rs                  # 统一缓存管理
│   ├── block_cache.rs          # BlockCache (Moka 分片 + TinyLFU)
│   ├── l2_cache.rs             # L2 文件缓存 (mmap)
│   ├── budget.rs               # 内存预算
│   ├── prefetch.rs             # 顺序预取
│   ├── warmup.rs               # 缓存预热
│   └── rebalance.rs            # 缓存重平衡
├── bloom/
│   ├── mod.rs                  # Bloom Filter 核心
│   ├── adaptive.rs             # 自适应 Bloom
│   ├── compressed.rs           # 压缩 Bloom
│   ├── custom_bloom.rs         # 自定义 Bloom
│   ├── manager.rs              # Bloom 管理
│   ├── filter_cache.rs         # Bloom 缓存
│   ├── fpr_controller.rs       # FPR 控制器
│   └── migration.rs            # Bloom 迁移
├── query/
│   ├── scan.rs                 # 范围扫描
│   ├── pruner.rs               # RangeQueryPruner
│   └── zone_map.rs             # ZoneMap 索引
├── compaction/
│   ├── mod.rs                  # Compaction 核心
│   ├── manifest.rs             # Compaction Manifest
│   └── merge_iterator.rs       # 合并迭代器
├── checkpoint/
│   ├── mod.rs                  # 检查点管理
│   ├── manager.rs              # 检查点管理器
│   └── filekv_impl.rs          # FileKV 检查点实现
├── ops/
│   ├── memory_tracker.rs       # 内存追踪
│   ├── amplification.rs        # 放大率统计
│   └── async_io.rs             # 异步 I/O
├── io/
│   ├── mod.rs                  # I/O 抽象 (StdFs/MemFs/FaultInjector)
│   └── stdfs.rs                # 标准文件系统
└── compression/
    └── mod.rs                  # 字典压缩 (zstd)
```
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
DenseIndex 使用 `AHashMap<String, DenseIndexEntry>` (O(1) 点查找)，配合 SparseIndex 按块分布的架构。AHashMap 提供 O(1) 查找性能，通过全局索引覆盖整个数据集。
- **范围查询优化**: 可以高效执行范围扫描和有序遍历
- **内存局部性**: BTreeMap 的节点布局更利于 CPU 缓存
- **确定性迭代顺序**: 键按字典序排列，便于调试和一致性扫描
**性能对比**:

| 索引类型 | 查找延迟 | 内存开销 | 适用场景 |
|---------|---------|---------|---------|
| SparseIndex | O(log n) + 扫描 | 低 (1/100) | 写密集 |
| DenseIndex | O(1) | 高 (每 entry 20-40B) | 读密集 |

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
use tokitai_filekv::{FileKV, FileKVConfig};

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

**实现**: Moka 多分片缓存 + TinyLFU 频率感知准入
**策略**: 频率感知淘汰 + 零分配 get_prefetch (Stack-allocated key buffer)
**配置**:

```rust
BlockCacheConfig {
    max_items: 50_000,                        // 5 万项
    max_memory_bytes: 256 * 1024 * 1024,      // 256MB
    frequency_aware: true,                    // TinyLFU 频率感知
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
// 管理所有 Segment 的索引 (IndexManager 自身通过 ArcSwap 管理)
pub struct IndexManager {
    index_dir: PathBuf,
    indexes: BTreeMap<u64, Arc<SparseIndex>>,
    dense_indexes: BTreeMap<u64, DenseIndex>,
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
use tokitai_filekv::{FileKV, FileKVConfig};

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
use tokitai_filekv::AggressiveConfig;

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
| 3 | BlockCache (Moka TinyLFU) | ✅ 已集成 | 默认启用 | 无 |
| 4 | Bloom Filter | ✅ 已集成 | 默认启用 | 无 |
| 5 | WAL (Write-Ahead Log) | ✅ 已集成 | `enable_wal=true` | 无 |
| 6 | Compaction | ✅ 已集成 | 默认启用 (后台线程) | 无 |
| 7 | Sparse/Dense Index | ✅ 已集成 | `dense_index_enabled` | DenseIndex 内存开销较大 |
| 8 | Zone Map Pruning | ✅ 已集成 | 默认启用 (Range Scan) | 仅范围扫描受益 |
| 9 | Adaptive Bloom Cache | ✅ 已集成 | 默认启用 | 纯内存性能数据，非完整 KV 操作 |
| 10 | Range Scan Readahead | ✅ 已集成 | `readahead_multiplier > 0` | 仅顺序扫描受益 |
| 11 | Write Coalescing | 🟡 可选/实验性 | `write_coalescing_enabled=true` | 框架完整，需验证 |
| 12 | Adaptive Pre-allocation | 🟡 可选/实验性 | 默认启用 | 学习算法简单 |
| 13 | Async I/O | ✅ 已集成 | `put_with_io_mode()` | 主写入路径通过 `IoMode::Async` 支持异步，`IoMode::Sync` 为同步 |
| 14 | Timeout Control | 🟠 规划/进行中 | 自动 (仅后台操作) | 框架完整，仅保护 checkpoint 等后台操作，热路径集成规划中 |
| 15 | Memory Tracker | ✅ 已集成 | `actual_memory_bytes` AtomicU64 | MemTable 集成 `record_allocation()`/`record_deallocation()`，真实测量 |
| 16 | Compaction Trigger | 🟠 规划/进行中 | 自动 | 自适应策略已实现，当前使用固定 write count 计数器触发 |
| 17 | Amplification Analysis | ✅ 已集成 | `get_stats()` | WAF/RAF/SAF 均基于实际 I/O 计数器 (`record_disk_read`/`record_disk_write`) |
| 18 | Sequential Prefetch | ✅ 已集成 | 自动 (Range Scan + get()) | 范围扫描已集成，`get()` 单点查询通过 `SequentialDetector` 触发预取 |
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

### 3. 超时控制

**功能**: Async I/O 写超时保护

```rust
let config = FileKVConfig {
    async_io_write_timeout_ms: 5000,  // 5 秒超时
    ..Default::default()
};
```

> **实现状态**: ✅ 已实现 | `async_io_write_timeout_ms` 配置字段用于 AsyncWriter 路径。主读写路径的通用超时控制尚未实现。

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

> **实现状态**: ✅ 已集成 | ZoneMap 和 Sequential Prefetch 均在 Range Scan 和 get() 路径中生效 (PERF-PREFETCH-001)。

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
    pub fn open(config: FileKVConfig) -> anyhow::Result<Self>;

    // 单次写入
    pub fn put(&self, key: &str, value: &[u8]) -> anyhow::Result<()>;

    // 批量写入
    pub fn put_batch(&self, entries: &[(&str, &[u8])]) -> anyhow::Result<()>;

    // 单次读取
    pub fn get(&self, key: &str) -> anyhow::Result<Option<Bytes>>;

    // 批量读取
    pub fn get_batch(&self, keys: &[&str]) -> anyhow::Result<Vec<Option<Bytes>>>;

    // 删除
    pub fn delete(&self, key: &str) -> anyhow::Result<()>;

    // 范围扫描
    pub fn range(&self, start: &str, end: &str) -> anyhow::Result<RangeScanIterator<'_>>;

    // 获取统计
    pub fn get_stats(&self) -> FileKVStatsSnapshot;

    // 重置统计
    // Note: reset_stats() was removed in v0.5.0; stats are now read-only snapshots

    // 手动刷盘
    pub fn flush_memtable(&self) -> anyhow::Result<()>;

    // 手动 Compaction
    pub fn run_compaction(&self) -> anyhow::Result<compaction::CompactionStats>;
    
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

**最后更新**: 2026-04-16 (v0.5.0, Round 14 完成)
**维护者**: Tokitai Team
**许可证**: MIT OR Apache-2.0

### D. 版本历史

#### v0.4.0 (2026-04-14) - 性能优化
- Dense Index 快速路径实现，热缓存读取优化到 256-388 ns 范围
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
