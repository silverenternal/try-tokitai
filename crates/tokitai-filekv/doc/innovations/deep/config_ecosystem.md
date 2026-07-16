# 配置预设与生态系统创新深度调研

> 本文档详细分析 tokitai-filekv 的配置预设系统、Feature Flag 机制、基准测试套件和生态工具链集成。

---

## 目录

- [1. 配置预设系统](#1-配置预设系统)
- [2. Feature Flag 运行时控制](#2-feature-flag-运行时控制)
- [3. 基准测试套件](#3-基准测试套件)
- [4. RocksDB 公平对比](#4-rocksdb-公平对比)
- [5. 性能报告数据](#5-性能报告数据)
- [6. 生态工具链集成](#6-生态工具链集成)
- [7. Cargo Features 配置](#7-cargo-features-配置)
- [8. 关键文件索引](#8-关键文件索引)

---

## 1. 配置预设系统

### 1.1 四档配置预设

tokitai-filekv 提供四档配置预设,适应不同场景需求:

| 预设 | 构造函数 | 内存占用 | 数据安全 | WAL 同步模式 | 适用场景 |
|------|---------|---------|---------|-------------|---------|
| **Conservative** | `conservative()` | ~64MB | 最高 | 每次 fsync | 金融、医疗、审计日志 |
| **Balanced** | `balanced()` | ~256MB | 中等 | 批量 fsync | 大多数生产环境 (默认) |
| **Performance** | `performance()` | ~1GB | 中等 | 批量 fsync | AI 上下文、会话存储 |
| **Extreme** | `extreme()` | ~4GB | 最低 | OS 缓冲 | 缓存、临时数据 |

### 1.2 配置维度

每档预设控制以下配置维度:

1. **索引策略**: Dense Index、GlobalKeyIndex
2. **预读设置**: SequentialPrefetch 距离
3. **WAL 同步模式**: sync per write vs batch vs OS buffered
4. **缓存大小**: BlockCache、BloomFilterCache 预算
5. **Compaction 策略**: Leveled、Size-Tiered
6. **内存限制**: MemTable 大小阈值

### 1.3 AggressiveConfig

项目使用 `AggressiveConfig` 结构体控制激进优化选项:

```rust
pub struct AggressiveConfig {
    pub readahead_multiplier: u32,    // 0-8
    pub persistent_mmap_enabled: bool, // 持久 mmap
    pub dense_index_enabled: bool,     // 全内存密集索引
    pub wal_sync_mode: WalSyncMode,
    pub cache_max_memory_bytes: usize,
    // ...
}
```

**预设实现**:
- `AggressiveConfig::performance()`: 性能优化
- `AggressiveConfig::balanced()`: 平衡模式

### 1.4 配置示例

```rust
// 默认配置 (Balanced)
let config = FileKVConfig::balanced();

// 性能配置
let config = FileKVConfig::performance();

// 自定义配置
let config = FileKVConfig {
    memtable_size: 256 * 1024 * 1024,  // 256MB
    block_cache_size: 512 * 1024 * 1024, // 512MB
    readahead_multiplier: 4,
    wal_sync_mode: WalSyncMode::Batch,
    // ...
};
```

---

## 2. Feature Flag 运行时控制

### 2.1 Cargo Features

**文件**: `Cargo.toml`

```toml
[features]
default = ["wal"]
wal = []                                    # WAL 崩溃恢复
mimalloc = ["dep:mimalloc"]                 # mimalloc 分配器
benchmarks = ["dep:criterion"]              # 性能基准测试
rocksdb-compare = ["dep:rocksdb", "benchmarks"]  # RocksDB公平对比
metrics = ["dep:prometheus", "dep:metrics", "dep:metrics-exporter-prometheus"]  # Prometheus
async-io = ["dep:tokio"]                    # 异步 I/O
full = ["wal", "metrics", "async-io"]       # 全功能
```

### 2.2 功能组合

| Feature 组合 | 包含功能 | 适用场景 |
|-------------|---------|---------|
| **default** | WAL 启用 | 基本使用 |
| **full** | wal + metrics + async-io | 生产环境 |
| **rocksdb-compare** | benchmarks + rocksdb | 性能对比 |
| **minimal** | 仅核心功能 | 嵌入场景 |

### 2.3 运行时 Feature 控制

项目实现运行时 Feature Flag 系统:

**文件**: `src/ops/feature_flag.rs`

```rust
pub enum FeatureFlag {
    Inno001AdaptiveBloomCache,   // 三层 Bloom Filter 缓存
    Inno002ZoneMapPruning,       // Zone Map 剪枝
    Inno002SequentialPrefetch,   // 顺序预取
}
```

**API**:
```rust
impl FeatureFlagManager {
    pub fn is_enabled(&self, flag: FeatureFlag) -> bool;
    pub fn set_enabled(&self, flag: FeatureFlag, enabled: bool);
    pub fn get_stats(&self) -> FeatureFlagStats;
}
```

### 2.4 Feature Flag 统计

```rust
pub struct FeatureFlagStats {
    pub total_checks: u64,      // 总检查次数
    pub enabled_hits: u64,      // 启用命中
    pub total_toggles: u64,     // 总切换次数
}
```

### 2.5 性能开销

| 操作 | 延迟 | 说明 |
|------|------|------|
| `is_enabled()` | ~5-10ns | 原子读取 |
| `set_enabled()` | ~50-100ns | 原子写入 + 回调 |
| 回调执行 | 可变 | 用户定义 |

---

## 3. 基准测试套件

### 3.1 Bench 文件完整列表 (19 个)

#### 核心基准 (9 个)

| 文件 | 描述 | 数据规模 |
|------|------|---------|
| `01_basic_ops.rs` | 基本 KV 操作 | 小数据集 |
| `02_cache_performance.rs` | 缓存性能 (BlockCache/BloomFilterCache) | - |
| `03_bloom_filter.rs` | Bloom Filter 性能 (L1/L2/L3 三层) | - |
| `04_concurrent_ops.rs` | 并发操作 (1-16 线程混合读写) | 多线程 |
| `05_range_compaction.rs` | 范围查询和压缩性能 | - |
| `06_large_dataset_bench.rs` | 大数据集 (10K/100K/1M keys) | 大规模 |
| `07_professional_benchmark.rs` | **10M keys 专业基准** | **10M** |
| `08_compression_bench.rs` | 压缩算法 (zstd/snap/lz4) | - |
| `09_10m_benchmark.rs` | 10M keys 全面测试 | **10M** |

#### 专项基准 (10 个)

| 文件 | 描述 |
|------|------|
| `file_kv_bench.rs` | 主 KV 操作基准 |
| `adaptive_bloom_bench.rs` | 自适应 Bloom Filter 专项 |
| `file_kv_inno002_bench.rs` | INNO-002 (Zone Map) 专项 |
| `feature_flag_bench.rs` | Feature Flag 系统性能 |
| `concurrent_bench.rs` | 并发负载 (1/2/4/8/16 线程) |
| `block_cache_get_by_key.rs` | BlockCache 按 key 查找优化 |
| `custom_bloom_perf.rs` | CustomBloom V3 格式性能 |
| `rocksdb_fair_comparison.rs` | RocksDB 公平对比 |
| `rocksdb_comprehensive_bench.rs` | RocksDB 全面对比 |

### 3.2 测试规模分级

| 规模 | Keys 数量 | 数据量 | 用途 |
|------|----------|--------|------|
| 极小规模 | ≤100K | ≤100MB | 功能验证、CI 快速反馈 |
| 小规模 | 100K~1M | 100MB~1GB | 趋势监控 |
| 中等规模 | 1M~10M | 1GB~10GB | 生产 benchmark |
| 大规模 | 10M~100M | 10GB~100GB | 生产级 benchmark |
| 超大规模 | ≥100M | ≥100GB | 工业级对比 |

### 3.3 Criterion 集成

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "07_professional_benchmark"
harness = false
```

**运行**:
```bash
cargo bench --bench 07_professional_benchmark
```

---

## 4. RocksDB 公平对比

### 4.1 测试方法论

公平对比测试在 `benches/rocksdb_fair_comparison.rs` 和 `benches/rocksdb_comprehensive_bench.rs` 中实现:

**公平性保证**:
- ✓ 相同硬件环境
- ✓ 相同数据量
- ✓ 相同 key/value 分布
- ✓ 独立测量方法
- ✓ 相同预热时间

### 4.2 对比数据 (v0.5.0 Round 38, 2026-04-16 实测)

| 操作 | FileKV | RocksDB | 提升倍数 | 说明 |
|------|--------|---------|---------|------|
| **Bloom Filter 负向查询** | **7.23 µs** | **247.38 µs** | **34.2x** | 纯内存 |
| **全 KV Get (热点缓存)** | **278-285 ns** | **600.07 µs** | **2107-2158x** | Dense Index 快速路径 |
| **全 KV Get (冷缓存)** | **417-435 ns** | **~6 µs** | **~15x** | 完整查询 |
| 写入 (64B, WAL) | **1.57 µs/entry** | **1.88 µs/entry** | FileKV 快 17% | KV 操作 |
| **10M 顺序写入** | **~355K ops/sec** | **500K-1M ops/sec** | RocksDB 快 1.4-2.8x | 成熟度差距 |
| **100K keys 真实场景** | **~101 ms** | **~628 µs** | FileKV 慢约 161x | 多 segment 遍历开销 |
| **1M keys 真实场景** | **~1.27 s** | **~6.3 ms** | FileKV 慢约 200x | 已知限制 |

### 4.3 优势与劣势分析

**FileKV 优势**:
- ✓ Bloom Filter 负向查询快 34.2x
- ✓ 热点缓存查询快 2107x
- ✓ 写入 (64B) 快 17%
- ✓ 写放大 1.00x (完美)

**FileKV 劣势**:
- ✗ 大数据集遍历慢 (161-200x)
- ✗ 成熟度不如 RocksDB
- ✗ 工具链生态少

**v0.6.0 优化目标**:
- 100K keys: 从 161x 缩小到 50x
- 1M keys: 从 200x 缩小到 30x

---

## 5. 性能报告数据

### 5.1 10M Keys 大规模写入

**文件**: `07_professional_benchmark.rs`

**数据**:
- **吞吐量**: ~355,000 ops/sec (平均,20 轮采样波动 <2%)
- **吞吐带宽**: ~37.9 MB/s
- **写放大 (WA)**: 1.00x (完美)
- **空间放大 (SA)**: 1.24x (优秀)
- **10M 写入耗时**: ~28.2 秒
- **逻辑数据量**: 1,120 MB
- **实际磁盘占用**: 13,350 MB (~13.0 GB)

### 5.2 不同 Value 大小对比

**文件**: `09_10m_benchmark.rs` (100K keys)

| Value 大小 | ops/sec | 空间放大 |
|-----------|---------|---------|
| 64B | ~803K | 567.75x |
| 256B | ~819K | 161.72x |
| 1KB | ~669K | 42.58x |
| 4KB | ~422K | 11.49x |

### 5.3 并发性能

**文件**: `04_concurrent_ops.rs`

| 场景 | 延迟 | 吞吐量 |
|------|------|--------|
| 4 线程并发写入 | 544 µs | 184K ops/sec |
| 4 线程并发读取 | 135 µs | 741K ops/sec |
| 4 线程混合 (80R20W) | 1.57 ms | 63.5K ops/sec |

### 5.4 v0.6.0 性能提升

根据 `docs/archive/v050-v070/V060_PERFORMANCE_REPORT.md`:

| 指标 | v0.5.0 | v0.6.0 | 提升 |
|------|--------|--------|------|
| **写入吞吐** | ~1,000 ops/sec | **357,000 ops/sec** | **357x** |
| **持续带宽** | ~0.1 MB/s | **38.2 MB/s** | **382x** |
| **写放大** | 未测量 | **1.00x** | 优秀 |
| **空间放大** | 未测量 | **1.24x** | 良好 |
| **测试覆盖** | - | **471 tests, 0 failures** | - |

---

## 6. 生态工具链集成

### 6.1 Prometheus 指标导出

```toml
metrics = ["dep:prometheus", "dep:metrics", "dep:metrics-exporter-prometheus"]
```

**功能**:
- WAF/RAF/SAF 放大率实时监控
- MemoryTracker 内存跟踪
- 内置指标自动记录

### 6.2 异步 I/O 支持

```toml
async-io = ["dep:tokio"]
```

**功能**:
- SequentialPrefetch 消费路径
- 异步 Compaction 线程
- 异步 MemTable Flush

### 6.3 mimalloc 分配器

```toml
mimalloc = ["dep:mimalloc"]
```

**功能**:
- 高并发场景内存分配优化
- 减少内存碎片

### 6.4 Compression 压缩

支持的压缩算法:
- **zstd**: 字典压缩 + 字典训练支持
- **snap**: 快速压缩
- **lz4**: 低延迟压缩

### 6.5 Benchmark 工具

- **Criterion**: 统计基准测试
- **cargo-nextest**: 并行测试执行
- **proptest**: 属性测试

### 6.6 CI/CD 配置

- `.cargo/config.toml`: 默认并行度 jobs=4
- `scripts/test.sh`: 自定义测试脚本
- 三维度 feature 测试矩阵 (default/async-io/full)

---

## 7. Cargo Features 配置

### 7.1 Feature 矩阵

| Feature | 依赖 | 启用功能 | 编译时间 |
|---------|------|---------|---------|
| **default** | - | WAL | 基准 |
| **wal** | - | WAL 崩溃恢复 | +5% |
| **metrics** | prometheus, metrics | Prometheus 导出 | +15% |
| **async-io** | tokio | 异步 I/O | +20% |
| **mimalloc** | mimalloc | 内存分配器 | +10% |
| **full** | wal + metrics + async-io | 全功能 | +30% |
| **rocksdb-compare** | rocksdb | 对比测试 | +50% |

### 7.2 编译配置示例

```toml
# Cargo.toml
[dependencies]
tokitai-filekv = { version = "0.5.0", features = ["full"] }
```

```bash
# 命令行
cargo build --features "full"
cargo test --features "metrics,async-io"
cargo bench --features "rocksdb-compare"
```

---

## 8. 关键文件索引

| 文件路径 | 职责 |
|---------|------|
| `Cargo.toml` | Feature 定义 |
| `src/lib.rs` | 预设构造函数 |
| `src/core/types.rs` | 配置结构体 |
| `src/ops/feature_flag.rs` | 运行时 Feature 控制 |
| `benches/` | 基准测试套件 |
| `docs/archive/v050-v070/V060_PERFORMANCE_REPORT.md` | 性能报告 |

---

## 总结

tokitai-filekv 的配置预设和生态系统通过以下创新实现灵活性:

1. **四档预设**: 适应不同场景需求
2. **Feature Flag**: 编译期 + 运行时双层控制
3. **19 个基准**: 全面性能覆盖
4. **RocksDB 对比**: 公平对比测试
5. **工具链集成**: Prometheus、Tokio、mimalloc

这些设计使 tokitai-filekv 既适合嵌入场景,又适合生产环境使用。
