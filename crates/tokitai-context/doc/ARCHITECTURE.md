# Atlas-Context 架构文档

**最后更新**: 2026-04-03  
**版本**: 2.0  
**状态**: ✅ 生产就绪

---

## 📋 目录

1. [系统概览](#系统概览)
2. [核心架构](#核心架构)
3. [存储引擎](#存储引擎)
4. [平行上下文管理](#平行上下文管理)
5. [AI 增强功能](#ai 增强功能)
6. [数据流](#数据流)
7. [并发模型](#并发模型)
8. [错误处理](#错误处理)

---

## 系统概览

Atlas-Context 是一个**Git 风格的 AI 对话上下文管理系统**，支持平行分支、智能合并和崩溃恢复。

### 核心设计思想

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                        │
│  (AI Agents, Chat Systems, Context-Aware Applications)      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Facade API Layer                         │
│  (Context, FileKV, FileService - Unified Interface)         │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              ▼                               ▼
┌─────────────────────────┐     ┌─────────────────────────┐
│   Parallel Context      │     │    FileKV Storage       │
│   Management            │     │    Engine               │
│  - Branch Management    │     │  - MemTable             │
│  - Git-style Merge      │     │  - Segment Files        │
│  - Conflict Resolution  │     │  - BlockCache           │
│  - Time Travel          │     │  - BloomFilter          │
└─────────────────────────┘     │  - Compaction           │
                                └─────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Persistence Layer                        │
│  (WAL, Segment Files, Index Files, Bloom Filters)           │
└─────────────────────────────────────────────────────────────┘
```

### 技术栈

| 层级 | 技术选型 | 理由 |
|------|----------|------|
| 语言 | Rust 2021 | 内存安全、零拷贝、高性能 |
| 并发 | DashMap + parking_lot | 无锁并发、细粒度锁 |
| 存储 | LSM-Tree | 写优化、顺序 I/O |
| 缓存 | LRU + Arc | 零拷贝、高效淘汰 |
| 索引 | Sparse Index + Bloom Filter | 空间效率、快速负向查找 |
| 日志 | tracing | 结构化日志、生产可观测性 |

---

## 核心架构

### 模块组织

```
tokitai-context/
├── facade.rs              # 统一 API 入口
├── error.rs               # 统一错误处理
│
├── Parallel Context/      # 平行上下文管理
│   ├── branch.rs          # 分支生命周期
│   ├── graph.rs           # DAG 上下文图
│   ├── merge.rs           # 6 种合并策略
│   ├── parallel_manager.rs # Git 风格管理器
│   ├── cow.rs             # Copy-on-Write 分支
│   ├── three_way_merge.rs # 三路合并
│   └── optimized_merge.rs # diff3 + LCS 算法
│
├── FileKV/                # LSM-Tree 存储引擎
│   ├── mod.rs             # 主模块
│   ├── memtable.rs        # 内存表 (DashMap)
│   ├── segment.rs         # 段文件 (顺序追加)
│   ├── block_cache.rs     # LRU 块缓存
│   ├── sparse_index.rs    # 稀疏索引
│   └── bloom.rs           # 布隆过滤器
│
├── Crash Recovery/        # 崩溃恢复
│   ├── wal.rs             # Write-Ahead Log
│   ├── compaction.rs      # 后台合并
│   └── fault_injection.rs # 故障注入测试
│
├── AI Enhanced/           # AI 增强功能
│   ├── ai_resolver.rs     # AI 冲突解决
│   ├── purpose_inference.rs # 分支目的推断
│   ├── smart_merge.rs     # 智能合并推荐
│   └── auto_tuner.rs      # 自动调参
│
└── Advanced Features/     # 高级特性
    ├── column_family.rs   # 列族支持
    ├── mvcc.rs            # 多版本并发
    ├── pitr.rs            # 时间点恢复
    └── consistency_check.rs # 一致性校验
```

---

## 存储引擎

### FileKV 架构

```
┌─────────────────────────────────────────────────────────────┐
│                      Write Path                             │
│                                                             │
│  put(key, value)                                            │
│      │                                                      │
│      ▼                                                      │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │  MemTable   │───▶│  WAL Log    │───▶│  BlockCache │     │
│  │  (DashMap)  │    │  (Append)   │    │  (LRU)      │     │
│  └─────────────┘    └─────────────┘    └─────────────┘     │
│      │                                                      │
│      ▼ (threshold reached)                                  │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │   Flush     │───▶│   Segment   │───▶│   Index     │     │
│  │             │    │   File      │    │   Update    │     │
│  └─────────────┘    └─────────────┘    └─────────────┘     │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                      Read Path                              │
│                                                             │
│  get(key)                                                   │
│      │                                                      │
│      ▼                                                      │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │ MemTable    │───▶│ BlockCache  │───▶│ BloomFilter │     │
│  │ Lookup      │    │ Lookup      │    │ Check       │     │
│  └─────────────┘    └─────────────┘    └─────────────┘     │
│      │                   │                  │               │
│      │ hit               │ hit              │ negative      │
│      ▼                   ▼                  ▼               │
│   Return             Return            Return None         │
│      │                   │                                  │
│      │                   ▼                                  │
│      │            ┌─────────────┐                          │
│      │            │ Segment     │                          │
│      │            │ Scan        │                          │
│      │            └─────────────┘                          │
│      │                   │                                  │
│      └───────────────────┴──────────────────────────────────┘
│                          │
│                          ▼
│                      Return Value
└─────────────────────────────────────────────────────────────┘
```

### 核心数据结构

#### MemTable Entry

```rust
pub struct MemTableEntry {
    pub value: Option<Bytes>,      // 值 (零拷贝)
    pub pointer: Option<DataPointer>, // 段文件指针
    pub seq_num: u64,               // 序列号
    pub deleted: bool,              // 删除标记
}

pub struct DataPointer {
    pub segment_id: u64,   // 段 ID
    pub offset: u64,        // 偏移量
    pub len: u64,           // 长度
    pub checksum: u32,      // CRC32C 校验和
}
```

#### Segment 文件格式

```
┌──────────────────────────────────────────┐
│ Magic Number (4 bytes) = 0x544F4B49      │ "TOKI"
├──────────────────────────────────────────┤
│ Version (4 bytes)                        │
├──────────────────────────────────────────┤
│ Entry Count (8 bytes)                    │
├──────────────────────────────────────────┤
│ Entries...                               │
│ ┌────────────────────────────────────┐   │
│ │ Key Length (4 bytes)               │   │
│ ├────────────────────────────────────┤   │
│ │ Key Data (variable)                │   │
│ ├────────────────────────────────────┤   │
│ │ Value Length (4 bytes)             │   │
│ ├────────────────────────────────────┤   │
│ │ Value Data (variable)              │   │
│ ├────────────────────────────────────┤   │
│ │ Checksum (4 bytes)                 │   │
│ └────────────────────────────────────┘   │
├──────────────────────────────────────────┤
│ Index Offset (8 bytes)                   │
└──────────────────────────────────────────┘
```

---

## 平行上下文管理

### 分支状态机

```
                    ┌─────────────┐
                    │   Created   │
                    └──────┬──────┘
                           │
                           ▼
                    ┌─────────────┐
              ┌────▶│   Active    │◀─────┐
              │     └──────┬──────┘      │
              │            │             │
         fork │       checkout           │ merge
              │            │             │
              │            ▼             │
              │     ┌─────────────┐      │
              │     │  CheckedOut │      │
              │     └──────┬──────┘      │
              │            │             │
              └────────────┴─────────────┘
                           │
                      abort │
                           ▼
                    ┌─────────────┐
                    │  Aborted    │
                    └─────────────┘
```

### 6 种合并策略

| 策略 | 描述 | 适用场景 | 冲突率 |
|------|------|----------|--------|
| **FastForward** | 直接移动指针 | 线性开发 | 0% |
| **SelectiveMerge** | 基于重要性选择 | 默认策略 | 10% |
| **AIAssisted** | AI 辅助冲突解决 | 复杂合并 | 5% |
| **Manual** | 用户解决所有冲突 | 关键变更 | 100% |
| **Ours** | 保留目标版本 | 保守策略 | 0% |
| **Theirs** | 保留源版本 | 实验性合并 | 0% |

### diff3 三向合并算法

```
传统两路合并:
  Source vs Target → 高误报率

diff3 三向合并:
  Source vs Base vs Target → 减少误报

算法流程:
1. 计算 Base 和 Source 的 LCS (最长公共子序列)
2. 计算 Base 和 Target 的 LCS
3. 识别真正冲突 (两者都修改)
4. 自动生成 Git 风格冲突标记
```

---

## AI 增强功能

### AI 冲突解决架构

```
┌─────────────────────────────────────────────────────────────┐
│                   AI Conflict Resolver                      │
│                                                             │
│  Input: Conflict { source, target, base, context }          │
│      │                                                      │
│      ▼                                                      │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  1. Context Analysis                                │   │
│  │     - Extract semantic meaning                      │   │
│  │     - Identify intent                               │   │
│  └─────────────────────────────────────────────────────┘   │
│      │                                                      │
│      ▼                                                      │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  2. Purpose Inference                               │   │
│  │     - Analyze branch purpose                        │   │
│  │     - Match with target goals                       │   │
│  └─────────────────────────────────────────────────────┘   │
│      │                                                      │
│      ▼                                                      │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  3. Resolution Generation                           │   │
│  │     - Generate fused version                        │   │
│  │     - Preserve semantic consistency                 │   │
│  └─────────────────────────────────────────────────────┘   │
│      │                                                      │
│      ▼                                                      │
│  Output: ResolvedConflict { resolution, confidence }        │
└─────────────────────────────────────────────────────────────┘
```

### Auto-Tuner 自动调参

```rust
pub struct AutoTuner {
    metrics_collector: MetricsCollector,
    workload_analyzer: WorkloadAnalyzer,
    parameter_optimizer: ParameterOptimizer,
    anomaly_detector: AnomalyDetector,
}

// 优化参数
- memtable_size: (64MB, 4GB)
- block_cache_size: (128MB, 2GB)
- compaction_threshold: (4, 16)
- write_batch_size: (10, 1000)
```

---

## 数据流

### 写入流程

```rust
pub fn put(&self, key: &str, value: &[u8]) -> Result<()> {
    // 1. 写入 WAL (崩溃恢复)
    if let Some(ref wal) = self.wal {
        wal.log(WalOperation::Add { key, value })?;
    }

    // 2. 插入 MemTable (无锁并发)
    let (size, seq) = self.memtable.insert(key, value);

    // 3. 更新 BlockCache (热数据缓存)
    self.block_cache.put(key, value);

    // 4. 检查是否需要 Flush
    if self.memtable.should_flush() {
        self.flush_memtable()?;
    }

    // 5. 检查是否需要 Compaction
    if self.compaction_manager.should_compact() {
        self.run_compaction()?;
    }

    Ok(())
}
```

### 读取流程

```rust
pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
    // 1. MemTable 查找 (最快)
    if let Some(value) = self.memtable.get(key) {
        return Ok(value);
    }

    // 2. BlockCache 查找 (零拷贝)
    if let Some(cached) = self.block_cache.get(key) {
        return Ok(cached);
    }

    // 3. Bloom Filter 检查 (快速负向查找)
    if !self.bloom_filter.contains(key) {
        return Ok(None);  // 肯定不存在
    }

    // 4. Segment 扫描 (最慢)
    for segment in self.segments.iter() {
        if let Some(value) = segment.get(key) {
            // 5. 回填缓存
            self.block_cache.put(key, &value);
            return Ok(value);
        }
    }

    Ok(None)
}
```

---

## 并发模型

### 锁策略

| 组件 | 锁类型 | 理由 |
|------|--------|------|
| MemTable | DashMap (无锁) | 高并发写入 |
| BlockCache | DashMap + Mutex | 读多写少 |
| Segment 列表 | RwLock | 读多写少 |
| Index | RwLock | 读多写少 |
| WAL | Mutex | 顺序写入 |

### 线程安全保证

```rust
// FileKV 是 Send + Sync
pub struct FileKV {
    memtable: Arc<MemTable>,           // 线程安全
    segments: Arc<RwLock<BTreeMap<...>>>, // 线程安全
    block_cache: Arc<BlockCache>,      // 线程安全
    wal: Option<Arc<Mutex<WalManager>>>, // 线程安全
    // ...
}
```

---

## 错误处理

### 错误类型层次

```
ContextError
├── Io(io::Error)
├── Corruption { location, reason }
├── KeyNotFound(String)
├── Wal(WalError)
│   ├── Io(io::Error)
│   ├── Serialization(serde_json::Error)
│   └── LockPoisoned
├── Compaction(String)
├── Cache(String)
└── OperationFailed(String)
```

### 错误处理最佳实践

```rust
// ✅ 推荐：使用 ? 操作符
pub fn load_index(&self) -> Result<SparseIndex> {
    let file = File::open(&path)?;  // 自动转换 io::Error
    let index = SparseIndex::deserialize(&file)?;
    Ok(index)
}

// ✅ 推荐：自定义错误映射
pub fn parse_magic(data: &[u8]) -> Result<u32> {
    let magic = u32::from_le_bytes(data[..4].try_into()?)
        .map_err(|_| FileKVError::Corruption {
            location: "magic_number".to_string(),
            reason: "Invalid magic bytes".to_string(),
        })?;
    Ok(magic)
}

// ❌ 避免：生产代码使用 unwrap()
let magic = u32::from_le_bytes(data[..4].try_into().unwrap());
```

---

## 性能特征

### 时间复杂度

| 操作 | 时间复杂度 | 说明 |
|------|------------|------|
| `fork` | O(n) | n = 文件数 (COW) |
| `checkout` | O(1) | 指针更新 |
| `merge` (FastForward) | O(1) | 指针更新 |
| `merge` (Selective) | O(n) | n = 变更数 |
| `put` (MemTable) | O(1) | DashMap 插入 |
| `get` (MemTable) | O(1) | DashMap 查找 |
| `get` (Cache Hit) | O(1) | DashMap 查找 |
| `get` (Bloom Negative) | O(k) | k = 哈希函数数 |
| `get` (Segment Scan) | O(n) | n = segment 数 |

### 空间复杂度

| 组件 | 空间复杂度 | 优化 |
|------|------------|------|
| MemTable | O(n) | 定期 Flush |
| Segment | O(n) | 顺序追加 |
| BlockCache | O(m) | LRU 淘汰 |
| BloomFilter | O(m/8) | 位数组 |
| SparseIndex | O(n/k) | k = 间隔 |

---

## 配置示例

```rust
use tokitai_context::facade::{Context, ContextConfig};

let config = ContextConfig {
    // FileKV 后端
    enable_filekv_backend: true,
    memtable_flush_threshold_bytes: 4 * 1024 * 1024,  // 4MB
    block_cache_size_bytes: 64 * 1024 * 1024,         // 64MB
    
    // WAL
    enable_wal: true,
    wal_max_size_bytes: 100 * 1024 * 1024,            // 100MB
    
    // Compaction
    compaction_min_segments: 4,
    compaction_max_segments: 8,
    
    // 高级特性
    enable_auto_tuner: true,
    enable_column_family: false,
    
    ..Default::default()
};

let mut ctx = Context::open_with_config("./.context", config)?;
```

---

## 参考文档

- [QUICKSTART.md](QUICKSTART.md) - 快速开始
- [USAGE.md](../USAGE.md) - 完整使用指南
- [AI_ASSISTANT.md](../AI_ASSISTANT.md) - AI 助手配置指南

---

**最后更新**: 2026-04-06
**维护者**: Atlas Team
**许可证**: MIT OR Apache-2.0
