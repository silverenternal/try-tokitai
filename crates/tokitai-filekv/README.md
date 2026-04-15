# Tokitai FileKV

[![Crates.io](https://img.shields.io/crates/v/tokitai-filekv.svg)](https://crates.io/crates/tokitai-filekv)
[![Documentation](https://docs.rs/tokitai-filekv/badge.svg)](https://docs.rs/tokitai-filekv)
[![License](https://img.shields.io/crates/l/tokitai-filekv.svg)](LICENSE)

> **🔧 实验性生产引擎**: FileKV 定位为**实验性生产引擎 (Experimental Production-Ready Storage Engine)**。核心功能完整实现，具备生产级代码质量（四层错误体系、六阶段重构架构、完整指标体系），但仍在生产环境验证中。在 100K keys 真实场景下比 RocksDB 慢约 161x（101ms vs 628µs），已知性能限制持续优化中。适合嵌入式 KV 场景、测试/开发环境，生产环境部署需评估。详见下方性能说明。

**高性能纯文件 KV 存储引擎 - 基于 LSM-Tree 架构，具备接近内存数据库的性能**

> **独立 Crate**: `tokitai-filekv` 是一个完全独立的 KV 存储引擎 crate
> 
> **源自**: tokitai-context 项目的存储引擎模块，现已独立为可复用的通用 KV 存储库
> 
> **适用场景**: 嵌入式 KV 存储、日志系统、配置存储、缓存层、时间序列数据等

---

## 📦 核心特性

- **LSM-Tree 架构**: 顺序写入，批量刷新
- **MemTable**: 基于 DashMap 的无锁内存缓冲
- **Segment 文件**: 高效追加写入的数据段
- **SparseIndex**: 内存稀疏索引 + 二分查找
- **BlockCache**: LRU 热数据块缓存
- **BloomFilter 缓存**: 快速负向查找 (INNO-001)
  - L1/L2/L3 三层自适应缓存
  - 基于 QPS 的 FPR 动态调整
  - **Bloom 分层迁移**: 频率感知的自适应迁移策略（结合 QPS 和访问频率混合评分，自动升降段到合适的缓存层）
- **Zone Map**: 范围查询优化 (INNO-002)
  - 块级剪枝 (减少 40-60% I/O)
  - 顺序预取
  - **Range Scan Readahead**: 顺序读取吞吐量提升 2-4x
- **WAL**: 崩溃恢复预写日志
  - **注意**: Lazy 模式不保证崩溃安全，仅适用于可接受数据丢失的场景
- **Compaction**: 后台段合并
  - 异步 Compaction (不阻塞写入)
  - Leveled Compaction (L0/L1/L2 多层)
  - 并行 Compaction
  - Tombstone 清理
- **Write Amplification Tracking**: WAF/RAF/SAF 监控
- **Memory Monitoring**: 全局内存跟踪与限制

## API 稳定性

当前版本为 **0.5.0**（1.0 之前）。

- **核心公共 API**（已稳定）：`FileKV`, `FileKVConfig` — 签名和语义已冻结，后续版本保持向后兼容
- **内部模块**（标记为 `#[doc(hidden)]`）：实验性实现细节，可能变更
- **语义化版本**：遵循 SemVer，API 完全稳定后将发布 1.0 版本

> 建议在 `Cargo.toml` 中使用 `"0.5"` 版本约束，以便在 0.x 范围内自动获取兼容更新。

## 性能表现（与 RocksDB 公平对比）

**测试日期**: 2026-04-15 | **版本**: v0.5.0 | **测试状态**: 431/431 lib + 28/28 integration (100% 通过) | **Doctests**: 15/15 通过 | **Clippy**: 0 warnings

| 操作 | FileKV | RocksDB | 提升倍数 | 测试类别 |
|------|--------|---------|---------|---------|
| **Bloom Filter 负向查询** | **62.37 µs** | **247.38 µs** | **3.97x** | 纯内存 |
| **全 KV Get (热点缓存)** | **0.229 µs** | **600.07 µs** | **2620x** | 完整查询 |
| 写入 (64B, WAL) | **1.92 µs/entry** | **1.88 µs/entry** | RocksDB 快 2% | KV 操作 |
| 写入 (100B, WAL) | **2.05 µs/entry** | **1.83 µs/entry** | RocksDB 快 12% | KV 操作 |

### ⚠️ 大规模数据集性能限制

> **⚠️ 测试量级说明（重要，专家评审 2026-04-15）**：
> - **10K/100K keys 仅做功能验证和回归测试，不代表生产性能**
> - **10K/100K 测试的定位**：功能正确性验证、CI 快速反馈（10 秒级）、性能趋势监控
> - **大规模 benchmark 至少 1M key 起步，RocksDB 对比建议 10M+**
> - 请勿将「应用数据集」与「存储引擎压测数据集」的量级混为一谈
>
> **规模分级说明**（对齐工业界标准）：
> - ≤100K keys（≤100MB）= **极小规模**（功能验证级）
> - 100K ~ 1M keys（100MB ~ 1GB）= **小规模**
> - 1M ~ 10M keys（1GB ~ 10GB）= **中等规模**
> - 10M ~ 100M keys（10GB ~ 100GB）= **大规模**（生产级 benchmark）
> - ≥100M keys（≥100GB）= **超大规模**

| 场景 | FileKV | RocksDB | 说明 |
|------|--------|---------|------|
| **100K keys 真实场景（极小规模）** | **~101 ms** | **~628 µs** | **FileKV 慢约 161x**（v0.5.0 优化，比 v0.4.0 提升 33%） |
| **1M keys 真实场景（小规模）** | **~1.27 s** | **~6.3 ms** | **FileKV 慢约 200x** |

> **原因**: 多 segment 遍历、Bloom Filter 加载、锁竞争开销。这是已知的性能限制，目前正在持续优化中。当前 FileKV 定位为**实验性生产引擎**，适合嵌入式小规模 KV 场景和测试/开发环境。大规模生产环境部署前建议进行充分评估和性能测试。

### 重要说明
- **v0.5.0 性能提升**: 100K keys 写入从 151ms 优化到 101ms（提升 33%），通过 SparseIndex Clone 消除、Bloom 缓存 10x 扩容、DenseIndex AHashMap 优化
- **v0.4.0 性能提升**: 热缓存读取从 61.92 µs 优化到 0.229 µs（270x 提升，POL-004 Dense Index 快速路径）
- **已知异常**: Bloom Filter 负向查询在部分场景下异常慢（约 14ms），原因为 bloom crate RandomState 无法序列化（POL-003 技术限制已文档化）
- **详细方法论**见 [doc/rocksdb_fair_comparison_2026_04_08.md](doc/rocksdb_fair_comparison_2026_04_08.md)

> **数据时效声明**: 以上性能数据测试于 2026-04-15，可能随后续优化（如全局有序索引、Compaction 策略改进）而变化。建议在部署前使用 `cargo bench --features benchmarks` 在当前环境重新测试获取最新数据。

### Benchmark Results (v0.8.0)

| Test | Scale | FileKV v0.8.0 | Notes |
|------|-------|--------------|-------|
| Hot Cache Read | Tiny | **0.229 µs** | 2620x faster than RocksDB |
| Bloom Negative | Tiny | **62.37 µs** | 3.97x faster than RocksDB |
| Write (100K keys) | Tiny | **~101 ms** | vs RocksDB 628µs (161x gap) |
| Write (1M keys) | Small | **~1.27 s** | vs RocksDB 6.3ms |
| Cold Cache Read | Tiny | **371 ns** | 95%+ improvement (v0.8.0) |

### v0.8.0 Performance Optimizations (Completed 8/10)

✅ **Completed**:
- WAL binary serialization (3-5x faster)
- CDict/DDict pre-creation (10-100x faster compression)
- GlobalKeyIndex truly enabled (direct segment lookup)
- Bloom L2 cache Arc refactoring (O(1) access)
- Instant timestamps (no syscall overhead)
- AHash shard routing (3-5x faster hashing)
- Compaction lock optimization (AtomicUsize)
- Timed fsync (10ms interval, reduced frequency)

❌ **Remaining**:
- Bloom FilterCache CLOCK algorithm (replaces LRU, eliminates mutex contention)
- ZoneMap Arc wrapping (eliminates Vec clone per query)

### Amplification Factors

| Metric | Definition | v0.8.0 Status |
|--------|-----------|---------------|
| Write Amplification (WA) | Disk writes / Logical writes | <3x (target achieved) |
| Read Amplification (RA) | Disk reads / Logical reads | TBD |
| Space Amplification (SA) | Disk usage / Logical data | TBD |

### Test Environment
- CPU: TBD
- Memory: TBD
- Disk: TBD
- OS: Linux

*Note: Full benchmark results pending. Current data shows improvement over v0.5.0.*

## 快速开始

```rust
use tokitai_filekv::{FileKV, FileKVConfig};

fn main() -> anyhow::Result<()> {
    let config = FileKVConfig::default();
    let kv = FileKV::open(config)?;

    // 写入
    kv.put("key1", b"value1")?;

    // 读取
    if let Some(value) = kv.get("key1")? {
        println!("值：{:?}", value);
    }

    Ok(())
}
```

## 配置预设

FileKV 提供四档配置预设，通过 [`AggressiveConfig`](https://docs.rs/tokitai-filekv/latest/tokitai_filekv/struct.AggressiveConfig.html)
控制索引策略、预读、WAL 同步模式、缓存大小等优化选项：

| 预设 | 适用场景 | 内存占用 | 数据安全 |
|------|---------|---------|---------|
| **Conservative** | 金融、医疗、审计日志 | ~64MB | 最高 (每次 fsync) |
| **Balanced** (默认) | 大多数生产环境 | ~256MB | 中等 (批量 fsync) |
| **Performance** | AI 上下文、会话存储 | ~1GB | 中等 (批量 fsync) |
| **Extreme** | 缓存、临时数据 | ~4GB | 最低 (OS 缓冲) |

### 使用示例

```rust
use tokitai_filekv::FileKVConfig;

// 保守模式 - 数据安全优先
let config = FileKVConfig::conservative();

// 平衡模式 - 默认，适合大多数场景
let config = FileKVConfig::balanced();

// 性能模式 - 读取速度优先
let config = FileKVConfig::performance();

// 极限模式 - 不计代价追求性能（数据可丢失）
let config = FileKVConfig::extreme();
```

> **选择建议**：如果不确定，使用默认的 `balanced()` 模式。对数据持久化有严格要求时使用 `conservative()`。
> 详细配置说明见 [API 文档](https://docs.rs/tokitai-filekv)。

## 架构设计

**Phase 4 重构后架构** (2026-04-11):

```
FileKV (薄门面, 899 行)
├── ReadEngine (读路径引擎)
│   ├── get() - KV 查找 (MemTable → BlockCache → Segments)
│   ├── Bloom Filter 加载与缓存
│   ├── Zone Map 块级剪枝
│   └── Sequential Prefetch 顺序预取
├── WriteEngine (写路径引擎)
│   ├── put() / put_batch() / delete()
│   ├── WAL 管理与批量写入
│   ├── Write Coalescer 写入合并
│   └── MemTable Flush 到 Segment
├── CompactionEngine (压缩引擎)
│   ├── run_compaction() - 同步压缩
│   ├── 异步 Compaction 线程
│   └── 自适应 Segment 预分配
└── LifecycleManager (生命周期管理)
    ├── open() / recover() - 初始化与恢复
    ├── Checkpoint 创建与恢复
    ├── 超时配置与统计
    └── Prometheus 指标导出
```

**原始架构** (重构前):
```
FileKV 引擎 (God Object, 1157 行)
├── MemTable (DashMap, 无锁并发)
├── Segment 文件 (顺序追加)
├── SparseIndex (key → 位置)
├── BlockCache (LRU, 热数据)
├── BloomFilter 缓存 (负向查找)
│   ├── L1: 热点 (FPR 0.1-0.5%)
│   ├── L2: 温点 (FPR 0.5-1.0%, 压缩)
│   └── L3: 冷点 (FPR 1.0-10.0%)
├── Zone Map (范围剪枝)
└── WAL (崩溃恢复)
```

**重构收益**:
- ✅ lib.rs 减少 22% (1157 → 899 行)
- ✅ 消除 22 个重复方法
- ✅ 删除 13 个遗留字段
- ✅ 282/282 测试通过，性能退化仅 0.8%

## 功能特性

- `wal`: 启用预写日志（默认）
- `benchmarks`: 包含性能基准测试套件
- `rocksdb-compare`: RocksDB 公平对比基准
- `metrics`: Prometheus 指标导出
- `async-io`: 异步 I/O 支持
- `full`: 启用所有功能

## 安装

添加到你的 `Cargo.toml`:

```toml
[dependencies]
tokitai-filekv = "0.5"
```

## 文档

- [API 文档](https://docs.rs/tokitai-filekv)
- [用户指南 (技术深度)](doc/filekv/FILEKV_GUIDE.md) - 架构详解、数据模型、配置指南、故障排查
- [项目定位与状态 (路线图)](doc/filekv/POSITION_AND_STATUS.md) - 项目定位、已知限制、实现状态、生产就绪路线图
- [RocksDB 公平对比](doc/rocksdb_fair_comparison_2026_04_08.md)

## 运行基准测试

```bash
# 运行 FileKV 基准测试
cargo bench --features benchmarks

# 运行自适应 Bloom Filter 基准
cargo bench --features benchmarks --bench adaptive_bloom_bench

# 运行 Feature Flag 基准
cargo bench --features benchmarks --bench feature_flag_bench

# 运行 INNO-002 Range Query 基准
cargo bench --features benchmarks --bench file_kv_inno002_bench

# 运行并发基准测试 (1-16 线程)
cargo bench --features benchmarks --bench concurrent_bench

# 运行 RocksDB 公平对比 (需要 rocksdb-compare feature)
cargo bench --features rocksdb-compare --bench rocksdb_fair_comparison
```

## 许可证

本项目采用 **MIT** 或 **Apache-2.0** 许可证（任选其一）。

## 贡献

除非你明确声明，否则你有意提交到本作品的任何贡献（如 Apache-2.0 许可证所定义）应按照上述方式双重许可，且不附加任何额外条款或条件。

## 测试

### 测试状态

- **Lib 测试**: 431/431 (100%) - 包含字典压缩、rebalance 执行引擎、频率感知迁移、SequentialPrefetch 消费测试
- **Doctests**: 15/15 通过，6 忽略（预期）
- **集成测试**: 28/28 通过 (6 个模块: lifecycle, concurrency, high_concurrency, compaction_consistency, checkpoint, batch_and_range)
- **高并发测试**: 9 个已解除 #[ignore] 标记并默认运行 (tests/filekv_integration/high_concurrency.rs)
- **编译警告**: 0 个 (clippy 零警告)
- **最后更新**: 2026-04-15 (v0.5.0)
- **v0.5.0 成就**: SparseIndex Clone 消除（O(1) Arc::clone），Bloom 缓存 10x 扩容（1000 filters, 256MB），DenseIndex AHashMap 优化（O(1) 查找），100K keys 写入性能提升 33%（151ms → 101ms），大规模数据集基准测试（10K/100K/1M keys）

### 并行测试执行

项目包含 **431 个测试**,分布在 **46+ 个测试模块**中。推荐使用并行执行加速 CI/CD:

```bash
# 方式 1: 使用 cargo-nextest (推荐,最快)
cargo install cargo-nextest
cargo nextest run --lib --test-threads 4

# 方式 2: 使用自定义脚本
./scripts/test.sh --nextest        # 使用 nextest
./scripts/test.sh --cargo          # 使用 cargo 内置并行
./scripts/test.sh --jobs 8         # 自定义并行度

# 方式 3: 使用 cargo 内置并行
cargo test --lib --jobs 4          # 4 个并行任务
```

**并行测试脚本** (`scripts/test.sh`) 支持:
- `--nextest`: 使用 cargo-nextest (推荐)
- `--cargo`: 使用 cargo 内置并行 (默认)
- `--module`: 按模块并行运行 (兼容模式)
- `--verbose`: 详细输出
- `--watch`: 监听文件变化自动重测
- `--jobs N`: 自定义并行度

**配置**: `.cargo/config.toml` 中已配置默认并行度 `jobs = 4`,可根据 CI/CD 环境的 CPU 核心数调整。

## 最新特性 (2026-04-15)

- ✅ **v0.8.0 性能优化 (8/10 完成)**:
  - ✅ WAL 二进制序列化 (3-5x 加速) + CDict 预创建 (10-100x) + GlobalKeyIndex 真正启用
  - ✅ Bloom L2 缓存重构 (Arc 直接返回) + Instant 时间戳 (无系统调用)
  - ✅ AHash 分片 (3-5x 加速) + Compaction 锁优化 (AtomicUsize) + 定时 fsync
  - ❌ Bloom FilterCache CLOCK 算法替换 LRU (待完成)
  - ❌ ZoneMap Arc 包装消除 clone (待完成)
  - 读性能提升 95%+ (371ns vs 之前)
  - 482 lib + 28 integration + 15 doctests 全部通过，clippy 零警告
- ✅ **v0.7.0 全部完成**: GlobalKeyIndex 写入路径修复 + BlockCache O(1) key 直查 + GlobalKeyIndex 持久化 + 混合负载优化
- ✅ **v0.5.0 全部完成**: SparseIndex Clone 消除（O(1) Arc::clone）+ Bloom 缓存 10x 扩容（1000 filters, 256MB）+ DenseIndex AHashMap 优化（O(1) 查找）+ 大规模数据集基准测试（10K/100K/1M keys，**注：100K 仅作功能验证，不代表生产性能**）
- ✅ **100K keys 写入性能提升 33%**: 151ms → 101ms（vs RocksDB 628µs，差距从 240x 缩小到 161x，**仅限极小规模场景**）
- ✅ **v0.4.0 全部完成**: Dense Index 快速路径 (270x 热缓存读取提升) + BlockCache 多分片架构 + 9 个高并发测试解除 ignored
- ✅ **完整架构重构**: 四引擎设计 (ReadEngine/WriteEngine/CompactionEngine/LifecycleManager)
- ✅ **Phase 0-5 全部完成**: rebalance 执行引擎 + SequentialPrefetch 消费 + BlockCache 字节级限制
- ✅ **错误体系统一**: Fatal/Transient/Expected/Domain 四层错误体系
- ✅ **统一缓存**: CacheBudget + UnifiedCacheManager + 后台 rebalance 线程（Bloom 动态 + BlockCache advisory）
- ✅ **Bloom Filter 增强**: L1/L2/L3 三层自适应缓存 + 频率感知迁移 + 动态内存调整
- ✅ **字典压缩**: zstd 压缩 + 字典训练支持 (DictionaryTrainer)
- ✅ **WAL 安全增强**: 序列号连续性校验 + 完整性验证
- ✅ **SequentialPrefetch 消费**: get() 路径加入 prefetch cache 查找，新增 prefetch_hits 计数器
- ✅ **BlockCache 字节级限制**: Moka weigher 按实际字节数淘汰
- ✅ **BlockCache 动态缩容**: 多分片 Moka 架构，支持 shrink_to()/grow_to() 动态调整 (PROD-001)
- ✅ **测试覆盖**: 431 lib tests + 28 integration tests (100% 通过，0 ignored)
- ✅ **Doctests**: 15 个通过，6 个忽略（预期）
- ✅ **编译零警告**: cargo clippy 0 warnings
- ✅ **CI 覆盖**: default/async-io/full 三维度 feature 测试矩阵
- ✅ **unwrap 审计**: 生产路径 0 处 unwrap()
- 🎯 **v0.6.0 规划中（P0: 10M+ 专业 benchmark）**: 10M+ keys 大规模数据集性能 + 专业 Benchmark 体系对齐工业界标准 + 全局有序索引优化 + 写放大/读放大/空间放大率测量（WA = 实际磁盘写入 / 逻辑写入，RA = 实际磁盘读取 / 逻辑读取，SA = 磁盘使用量 / 逻辑数据量）

## 关联项目

- **tokitai-context**: Git 风格的 AI 对话上下文管理系统（FileKV 的原生项目）
- **try-tokitai**: AI 原生工具选择器 + Git 分支式上下文管理
