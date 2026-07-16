# FileKV 项目定位与状态

**最后更新**: 2026-04-16 (v0.5.0, Round 38 完成)
**版本**: v0.5.0
**状态**: 实验性生产引擎 (Experimental Production-Ready), 630+ tests 100% 通过

---

## 📋 目录

1. [项目定位](#项目定位)
2. [设计目标](#设计目标)
3. [使用场景](#使用场景)
4. [非设计目标](#非设计目标)
5. [当前已知限制](#当前已知限制)
6. [实现状态清单](#实现状态清单)
7. [v0.4.0 规划](#v040-规划)
8. [生产就绪路线图](#生产就绪路线图)

---

## 项目定位

**FileKV 是一个正在向实验性生产引擎转型的 LSM-Tree KV 存储引擎**。核心架构清晰（六阶段重构、四引擎拆分），代码质量达到生产级标准（四层错误体系、完整指标体系、崩溃安全机制），但仍有已知限制需解决，正在向生产就绪方向持续演进。

### 🎯 核心定位

| 维度 | 定位 |
|------|------|
| **目标** | 面向 Rust 生态和 AI 场景的下一代 KV 存储引擎 |
| **用户** | Rust 开发者、AI 应用开发者、系统架构师、研究人员 |
| **场景** | Rust 原生嵌入、AI 上下文存储、会话历史、开发/测试环境、学术研究 |
| **可靠性** | 代码质量生产级，核心 API 已稳定，但需在实际环境验证，已知限制明确 |

### 💡 为什么选择 tokitai-filekv？

**不是"更快的 RocksDB"，而是"更智能、更安全、更易用的 Rust 原生引擎"**。

#### 核心优势（已超越 RocksDB 的场景）

| 优势维度 | 具体表现 | 对比 RocksDB | 价值 |
|---------|---------|-------------|------|
| **🚀 自适应 Bloom** | L1/L2/L3 三层 + 频率感知迁移 | **34.2x 更快**（7.23 µs vs 247.38 µs） | 热数据自动加速 |
| **⚡ 热点缓存** | Dense Index 快速路径 + BlockCache | **2107-2158x 更快**（278-285 ns vs 600.07 µs） | 内存数据库级别读取 |
| **🛡️ Rust 原生** | 0 warnings, 0 unwrap(), 630+ tests | C++ 需手动审计 | 编译期安全保证 |
| **📊 可观测性** | Prometheus + WA/RA/SA 内置 | 需外部集成 | 运维开箱即用 |
| **🏗️ 架构清晰** | 四引擎分离，非 God Object | db_impl.cc 5000+ 行 | 学习/维护成本低 |

#### 已知差距（持续优化中）

| 差距维度 | tokitai-filekv | RocksDB | 优化路径 |
|---------|----------------|---------|---------|
| **10M 顺序写入** | ~355K ops/sec | 500K-1M ops/sec | GlobalKeyIndex 优化、io_uring |
| **100K 真实场景** | ~101 ms | 628 µs | 读路径优化、segment 遍历减少 |
| **工业成熟度** | 实验性生产 | 15+ 年生产验证 | 24h+ 稳定性测试、生产验证 |

### 项目演进历程

| 阶段 | 版本 | 定位 | 状态 |
|------|------|------|------|
| v0.0.x | 初始原型 | 功能验证 | ✅ 已完成 |
| v0.1.0-v0.1.6 | 六阶段重构 | 架构完善 | ✅ 已完成 |
| v0.1.7 | 代码质量清理 | Critical/Major 问题修复 | ✅ 已完成 |
| **v0.2.0** | **实验性生产引擎** | **剩余问题修复、文档对齐、API 稳定** | **✅ 已完成** |
| **v0.3.0** | **实验性生产引擎** | **Phase 4 特性 + Phase 0/1 关键修复完成，核心 API 稳定** | **✅ 已完成** |
| **v0.3.1** | **实验性生产引擎** | **示例代码编译错误修复 (audit_log 路径)** | **✅ 已完成** |
| **v0.4.0** | **性能优化版本** | **Dense Index 270x 提升 + BlockCache 多分片 + 9 个高并发测试解除** | **✅ 已完成** |
| **v0.5.0** | **极小规模数据集优化** | **100K keys 场景性能优化 (240x → 161x) + SparseIndex + DashMap + 基准测试** | **✅ 已完成** |
| **v0.5.0 (Round 31-38)** | **性能优化与代码质量** | **SystemTime 消除 + 写入路径优化 + Benchmark 方法修复 + 全面代码审查，630 tests** | **✅ 已完成** |
| **v0.6.0** | **专业 Benchmark + 全局索引** | **10M+ keys 性能 + 写/读/空间放大率测量 + 全局有序索引** | **🎯 规划中** |
| v1.0.0 | 稳定版 | 生产就绪 | 📋 远期规划 |

---

## 设计目标

### ✅ 已实现的目标

1. **性能验证**
   - 验证 LSM-Tree 架构的写优化特性
   - 实现 4MB 以下 value 的零拷贝 mmap 读取
   - 实现 Write Coalescer 合并连续写入

2. **自适应 Bloom Filter**
   - 基于访问模式动态调整缓存策略
   - L1/L2/L3 三层自适应缓存架构
   - False Positive Rate 控制器自动调节

3. **Zone Map 范围查询优化**
   - 块级元数据加速范围查询
   - 顺序访问模式检测与预取
   - 多 Segment 并行扫描

4. **崩溃安全机制**
   - WAL (Write-Ahead Log) 保证持久性
   - Compaction Manifest 原子提交
   - 9 种崩溃场景恢复测试覆盖

5. **完整指标体系**
   - Prometheus 指标导出
   - 内存使用跟踪
   - 写放大分析

### 🎯 当前版本目标 (v0.3.1)

1. 示例代码编译错误修复（audit_log 路径修正）✅
2. 测试覆盖 570 lib tests + 32 integration tests (100%)
3. 编译零警告 (clippy 0 warnings)
4. 文档全面对齐 ✅

### ✅ v0.4.0 已完成 (2026-04-14)

1. **TEST-001**: ✅ 解除 9 个高并发 ignored 测试（全部在 tests/filekv_integration/high_concurrency.rs）
2. **POL-003**: ✅ Bloom Filter V2 序列化格式实现（技术限制：bloom crate RandomState 无法序列化，已文档化）
3. **POL-004**: ✅ Dense Index 快速路径实现，热缓存读取优化到 256-388 ns 范围
4. **PROD-001**: ✅ BlockCache 多分片架构实现，支持 shrink_to()/grow_to() 动态调整

### ✅ v0.5.0 已完成 (2026-04-16)

> **⚠️ 规模说明**（专家评审 2026-04-15）：100K keys（~11MB）属于**极小规模**（≤100MB），仅做功能验证，不代表生产性能。但为保持版本连续性，保留此命名。

1. **PERF-005**: ✅ 极小规模数据集性能优化（100K keys 151ms → 101ms，提升 33%，vs RocksDB 从 240x 缩小到 161x）
   - P0: 消除 SparseIndex Clone（O(n) → O(1) Arc::clone）
   - P1: Bloom Filter 缓存扩容（100 → 1000 filters, 64MB → 256MB）
   - P2: DenseIndex AHashMap 优化（O(log n) → O(1)）
2. **POL-005**: ✅ SparseIndex AHashMap 优化（内存减少 50%+）
3. **POL-006**: ✅ DashMap 高负载优化（BlockCache 多分片架构间接优化）
4. **TEST-002**: ✅ 极小规模数据集基准测试（benches/06_large_dataset_bench.rs，10K/100K/1M keys，**注：100K 仅作功能验证**）

### ✅ v0.5.0 (Rounds 31-34) 已完成 (2026-04-16)

> **Rounds 31-34**: 性能优化与全面代码质量审查

1. **Round 31**: ✅ SystemTime syscall 消除
   - bloom_migration_controller.record_access() 被 is_adaptive_bloom_cache_enabled() 门控
   - record_sequential_access() 被 is_sequential_prefetch_enabled() 门控
   - SegmentAccessTracker 使用 Instant 替代 SystemTime::now()
   - 基准结果: Compaction 触发 3.17ms→2.83ms (-11%)，热缓存 ~265-385ns (稳定)
2. **Round 32**: ✅ 写入路径优化
   - 移除 put()/put_batch() 中冗余 AtomicUsize store
   - get_stats() 按需从 memtable 读取
   - 基准结果: 混合并发 1.56ms→1.53ms (-2%)，mixed_workload_100k 改善 17% (118ms→99ms)
3. **Round 33**: ✅ SystemTime::now() 残留清理
   - src/bloom/adaptive.rs 中 2 个剩余 syscall 替换为 Instant + LazyLock
   - 至此整个代码库所有 SystemTime::now() 热路径调用完全消除
4. **Round 34**: ✅ 全面代码质量审查
   - 确认 0 clippy warnings, 630 tests pass, 0 production unwrap()
   - CHANGELOG/README/CLAUDE.md 文档同步更新
5. **Round 35-37**: ✅ Benchmark 方法修复（见 PERFORMANCE_BASELINE.md Round 38 说明）
6. **Round 38**: ✅ Benchmark 逻辑修复 + 性能文档全面更新
   - delete 改为 write+delete 全周期测量（135ns → 1.18-1.20 µs）
   - batch_write 改用 put_batch() API（117-119 µs → 38-42 µs，~3x 提升）
   - trigger_compaction 改为实际执行 run_compaction()（~3ms → 5.31-5.37 ms）
   - 并发 benchmark 排除线程 spawn/join 开销（Instant 测量）
   - compression_ratio 测量实际压缩操作而非 format!()
   - 630 tests pass, 0 clippy warnings

### 🎯 v0.6.0 规划目标

> **P0 优先级**：10M+ keys 专业 benchmark（对齐专家评审标准，聚焦 10GB+ 中等规模场景）

1. **BENCH-001** (P0, 16h): 专业 Benchmark 体系（10M keys + 写/读/空间放大率测量 + 混合负载测试 + RocksDB 公平对比）
2. **PERF-006** (P0, 24h): 全局有序索引优化（减少 segment 遍历，参考 RocksDB Version/Edit）
3. **PERF-007** (P0, 20h): 10M keys 写入性能优化（目标 <10x RocksDB 差距）
4. **TEST-003** (P1, 8h): 24h+ 稳定性测试（性能衰减 <20%，无内存泄漏）
5. **DOC-001** (P1, 6h): 性能文档重写（修正规模分级，补充放大率数据）
6. **POL-007** (P2, 10h): MemTable DashMap 高负载优化（分片数量可配置）

**放大率定义**（对齐工业界标准）：
- **WA (Write Amplification)** = 实际磁盘写入字节数 / 逻辑写入字节数
- **RA (Read Amplification)** = 实际磁盘读取字节数 / 逻辑读取字节数
- **SA (Space Amplification)** = 磁盘使用量 / 逻辑数据量

---

## 使用场景

### ✅ 适用场景

| 场景 | 说明 |
|------|------|
| **开发/测试环境** | 作为 KV 存储原型进行功能验证 |
| **学术研究** | LSM-Tree、Bloom Filter、Zone Map 算法验证 |
| **小规模部署** | 数据量 < 100GB，QPS < 1000 的场景 |
| **技术评估** | 对比 RocksDB、LevelDB 等引擎的性能特征 |

### ❌ 暂不适用场景

| 场景 | 原因 |
|------|------|
| **大规模生产部署** | 10M keys 性能 ~355K ops/sec，比 RocksDB (~500K-1M ops/sec) 慢约 1.4-2.8x（已知性能差距，持续优化中） |
| **高并发场景** | 32/64 线程并发测试已完成 (PROD-002)，需验证通过 |
| **长时间运行** | 长期稳定性测试框架已完成 (PROD-003)，需定期运行 |
| **关键业务数据** | 部分边缘情况恢复机制未充分验证 |

---

## 规模分类

> **对齐工业界标准**（专家评审 2026-04-15）：存储引擎 benchmark 的规模分类应对齐 RocksDB、LevelDB 等工业界标准。

| 规模等级 | Key 数量 | 数据量 | 测试目标 |
|---------|----------|--------|---------|
| Tiny (极小) | ≤100K | ≤100MB | 功能正确性、单元测试 |
| Small (小) | 100K~1M | 100MB~1GB | 基础性能验证 |
| Medium (中) | 1M~10M | 1GB~10GB | 核心性能、放大率 |
| Large (大) | 10M~100M | 10GB~100GB | 极限性能 |
| XLarge (超大) | ≥100M | ≥100GB | 长期稳定性 |

**历史修正说明**：
- v0.5.0 文档中曾将 100K keys 称为"大规模"，这是分类错误。100K keys 属于**极小规模**（Tiny），仅适合功能验证。
- 10M+ keys 才是真正的中等规模（Medium），100M+ keys 才是大规模（Large）。
- 本文档及 CHANGELOG 已修正所有相关表述。

---

## 非设计目标

FileKV **不试图**成为：

1. **RocksDB 的完全替代品**
   - RocksDB 有 10+ 年优化历史和专职团队
   - FileKV 聚焦于特定场景和算法创新

2. **通用 KV 存储**
   - 专注于 LSM-Tree 架构的研究价值
   - 不追求支持所有可能的用例

3. **云服务级别 SLA**
   - 当前为单机引擎，无分布式能力
   - 无自动故障转移或高可用设计

---

## 当前已知限制

### 🔴 Critical 限制

| 限制 | 影响 | 计划 |
|------|------|------|
| **100K keys 性能** | 比 RocksDB 慢 240x | v0.2.0 文档中明确声明 |

### 🟡 Major 限制

| 限制 | 影响 | 计划 |
|------|------|------|
| **Bloom Filter 重建性能** | 重复重建占 40-50% 时间，负向查询异常慢 (14ms) | ✅ v0.4.0 V2 格式已最优（技术限制已文档化） |
| **Segment 线性遍历** | 占 25-30% 性能时间 | ✅ v0.4.0 Dense Index 快速路径已实现 (270x 提升) |
| **BlockCache rebalance** | 仅 advisory mode，Moka capacity 不可变，无法真正缩容 | ✅ v0.4.0 多分片架构已实现 (PROD-001) |

### 🟢 Minor 限制

| 限制 | 影响 | 计划 |
|------|------|------|
| **生产路径 unwrap()** | 0 处 unwrap() | ✅ 已审计并消除所有 unwrap() |
| **测试超时** | 3 个 compaction 测试 >60s | ✅ 已优化并默认运行 |
| **无集成测试目录** | 所有测试在 src/ 内 | ✅ tests/ 目录已创建 (28 个测试，6 个模块) |

---

## 实现状态清单

### ✅ 已完成特性

| 特性 | 状态 | 验证方法 |
|------|------|----------|
| 六阶段重构架构 | ✅ 完成 | 四引擎拆分：Read/Write/Compaction/Lifecycle |
| Critical 问题修复 | ✅ 完成 | CRIT-001/002/003 全部验证 |
| Major 问题修复 | ✅ 基本完成 | MAJ-001~MAJ-008 仅余 MAJ-007-PHASE2 |
| Prometheus 指标 | ✅ 完成 | compaction/flush/cache/bloom 全覆盖 |
| Compaction Manifest 崩溃安全 | ✅ 完成 | 9 crash scenario 测试 |
| 四层错误体系 | ✅ 完成 | Fatal/Transient/Expected/Domain |
| WAL 持久性 | ✅ 完成 | 原子写入 + 恢复测试 |
| Zone Map 块级剪枝 | ✅ 完成 | 范围查询性能测试 |
| 自适应 Bloom 缓存 | ✅ 完成 | L1/L2/L3 + FPR 控制器 |
| 文档定位 | ✅ 完成 | PROJECT_STATUS.md + FILEKV_POSITION.md 整合 |

### ⏳ 待完成特性

| 特性 | 优先级 | 预计工作量 | 跟踪 |
|------|--------|------------|------|
| MAJ-007-PHASE2: Async/Flush metrics | 高 | 2-3h | todo.json |
| ~~MIN-004: Sequential Prefetch 消费~~ | ~~中~~ | ~~3-4h~~ | ✅ 已完成 (v0.5.0) |
| ~~TEST-005: 集成测试目录~~ | ~~中~~ | ~~4-6h~~ | ✅ 已完成 (tests/ 目录 28 个测试) |
| ~~BENCH-001: CI 集成~~ | ~~低~~ | ~~4-6h~~ | ✅ 已完成 (.github/workflows/ci.yml) |

**新增待办 (v0.5.0 之后发现):**
| 特性 | 优先级 | 跟踪 |
|------|--------|------|
| PERF-ZONEMAP-001: ZoneMap 重复扫描消除 | P0 | todo.json |
| PERF-PREFETCH-ALLOC-001: get_prefetch 零分配 | P0 | todo.json |
| PERF-LOCK-GKI-001: GlobalKeyIndex 锁一致性 | P1 | todo.json |
| DUP-BATCH-001: put_batch 代码去重 | P1 | todo.json |
| ERR-WAL-001: WAL async 错误处理 | P1 | todo.json |
| DOC-SYNC: 文档全面更新 | P1 | todo.json |

### 📊 质量指标

| 指标 | 当前值 | 目标 |
|------|--------|------|
| 编译 warnings | **0** | 0 ✅ |
| Lib 测试通过数 | **570** | 300+ ✅ |
| 集成测试通过数 | **28** | 10+ ✅ |
| 测试失败数 | 0 | 0 ✅ |
| 忽略测试数 | 0 (原 9 个高并发测试已解除 #[ignore]) | ✅ v0.4.0 已完成 |
| Doctest 通过数 | **15** | 10+ ✅ |
| 生产路径 unwrap() | 0 处 | <10 ✅ |
| 文档覆盖率 | ~95% | 95% ✅ |
| v0.3.1 修复 | 示例编译错误 (audit_log 路径) | ✅ 已完成 |
| v0.4.0 完成 | Dense Index 270x + BlockCache 多分片 + 9 高并发测试解除 | ✅ 已完成 |
| v0.5.0 完成 | Round 1-9 全部完成 (Phase 1-4) | ✅ 已完成 |

---

## v0.4.0 已完成总结 (2026-04-14)

v0.4.0 聚焦三大性能优化任务，全部完成。Phase 0-5 所有规划任务已全部完成。

### ✅ TEST-001: 解除 9 个高并发 ignored 测试 (P0)

**完成状态**: 9 个测试全部解除 #[ignore]，28 个集成测试全部通过

### ✅ POL-003: Bloom Filter V2 序列化格式 (P0)

**完成状态**: V2 格式已实现，技术限制已文档化（bloom crate RandomState 无法序列化）

### ✅ POL-004: Dense Index 快速路径 (P1)

**完成状态**: 热缓存读取优化到 256-388 ns 范围 (Dense Index 快速路径)

### ✅ PROD-001: BlockCache 多分片架构 (P1)

**完成状态**: 支持 shrink_to()/grow_to() 动态调整

---

## v0.5.0 已完成总结 (2026-04-16)

v0.5.0 聚焦极小规模数据集性能改进（100K keys 仅作功能验证，不代表生产性能），6 个任务全部完成。

### ✅ PERF-005: 极小规模数据集性能优化 (P0)

**完成状态**: 100K keys 写入从 151ms 优化到 101ms（提升 33%，vs RocksDB 628µs 差距从 240x 缩小到 161x）

**⚠️ 重要说明**: 此性能数据仅适用于极小规模场景（≤100MB），不代表生产环境性能。

**P0 - 消除 SparseIndex Clone**: IndexManager.indexes 使用 `BTreeMap<u64, Arc<SparseIndex>>`，`get_index()` 返回 `Arc::clone`（O(1) 操作）

**P1 - Bloom Filter 缓存扩容**: `max_filters: 100 → 1000`（10x），`max_memory_bytes: 64MB → 256MB`（4x），减少 40-50% 重建开销

**P2 - DenseIndex AHashMap 优化**: `DenseIndex.entries` 从 `BTreeMap` 改为 `AHashMap`（O(log n) → O(1)），哈希性能提升 2-3x

### ✅ POL-005: SparseIndex AHashMap 优化 (P1)

**完成状态**: `SparseIndex.key_map` 使用 `AHashMap<String, u64>` 替代 `HashMap`，内存减少 50%+

### ✅ POL-006: DashMap 高负载优化 (P2)

**完成状态**: BlockCache 多分片架构（前期已完成），MemTable DashMap 未直接改动

### ✅ TEST-002: 极小规模数据集基准测试 (P1)

**完成状态**: `benches/06_large_dataset_bench.rs` 覆盖 10K/100K/1M keys（**注：100K 仅作功能验证，生产级 benchmark 需 10M+**）

---

## v0.6.0 规划

v0.6.0 对齐专家评审标准，聚焦 10M+ keys（10GB+）中等规模场景，预计 4-8 周完成。**P0 优先级为专业 Benchmark 体系**。

### BENCH-001: 专业 Benchmark 体系对齐工业界标准 (P0, 16h)

**目标**: 10M keys 级别基准测试 + 写/读/空间放大率测量 + 混合负载测试 + RocksDB 公平对比

**放大率定义**：
- WA (Write Amplification) = 实际磁盘写入字节数 / 逻辑写入字节数
- RA (Read Amplification) = 实际磁盘读取字节数 / 逻辑读取字节数
- SA (Space Amplification) = 磁盘使用量 / 逻辑数据量

### PERF-006: 全局有序索引优化（减少 segment 遍历）(P0, 24h)

**目标**: 实现类似 RocksDB Version/Edit 的机制，10M keys 场景 get() 延迟降低 80%+

### PERF-007: 10M keys 写入性能优化（目标 <10x RocksDB 差距）(P0, 20h)

**目标**: 批量写入优化、Compaction 策略改进、内存分配优化

### TEST-003: 24h+ 稳定性测试 (P1, 8h)

**目标**: 24h 持续写入，性能衰减 <20%，无内存泄漏，数据一致性校验通过

### DOC-001: 性能文档重写（对齐专家评审标准）(P1, 6h)

**目标**: 修正规模分级（100K=极小规模，10M=中等规模），补充写/读/空间放大率数据

### POL-007: MemTable DashMap 高负载优化 (P2, 10h)

**目标**: 分片数量可配置，高并发场景（32+ 线程）吞吐量提升 15%+

---

## 生产就绪路线图

### Phase 1: Critical Hotfixes ✅

- [x] CRIT-001: 审计日志完全失效
- [x] CRIT-002: Examples 无法编译
- [x] CRIT-003: Cache hit/miss 统计语义错误

### Phase 2: Major Fixes ✅

- [x] MAJ-001: FPR 控制器接入
- [x] MAJ-002: UnifiedCacheManager 预算
- [x] MAJ-003: Write Coalescer 时间窗口
- [x] MAJ-004: 字典压缩文档澄清
- [x] MAJ-005: L2 压缩启用
- [x] MAJ-006: 异步 I/O sync bridge
- [x] MAJ-007: Compaction Prometheus 指标
- [x] MAJ-008: ZoneMap 调用语义注释

### Phase 3: Minor Polish ✅

- [x] MIN-002: Prefetch 预算类别简化
- [x] MIN-005: drop(mutex_guard) 修复
- [x] MIN-006: 未使用导入清理
- [x] MIN-007: tombstones_cleaned 精确统计
- [x] MIN-009/010: 死代码删除
- [x] MIN-001: unwrap 审计 (已完成报告)
- [x] MIN-004: Sequential Prefetch 消费逻辑 ✅ (FIX-001, v0.3.0)

### Phase 4: Documentation ✅

- [x] DOC-001: 项目定位统一
- [x] DOC-002: 版本号更新
- [x] DOC-004: 性能表格警告
- [x] DOC-006: API 稳定性声明
- [x] DOC-007: PROJECT_STATUS 重写
- [x] DOC-008: doc/filekv/ 整合
- [x] DOC-003: POSITION.md 验证
- [x] T-024: 字典压缩训练完整实现
- [x] T-025: UnifiedCacheManager 后台 rebalance（决策引擎 + 执行引擎：Bloom 动态调整 + BlockCache advisory mode）
- [x] T-026: 频率感知 Bloom Filter L1/L2/L3 迁移
- [x] FIX-001: SequentialPrefetch 消费逻辑 (get() 路径)
- [x] FIX-002: BlockCache 字节级内存限制
- [x] FIX-004: CacheWarmer Recent 策略精确化

### Phase 5: Test Pipeline ✅

- [x] TEST-003: stability_test 标记 #[ignore]
- [x] TEST-004: 清理 compiler warnings
- [x] TEST-ERR-001: 修复测试 bug
- [x] TEST-001: 验证测试超时
- [ ] TEST-002: 添加 #[timeout] 标记
- [ ] TEST-005: 创建 tests/ 目录

### Phase 6: Benchmark Improvements ✅

- [x] BENCH-001: 修复基准测试编译错误
- [x] BENCH-002: 添加回归检测阈值
- [x] BENCH-003: Top 5 性能瓶颈分析

### Phase 6: v0.4.0 Performance Optimization ✅ (已完成)

- [x] TEST-001: 解除 9 个高并发 ignored 测试
- [x] POL-003: Bloom Filter V2 序列化格式（技术限制已文档化）
- [x] POL-004: Dense Index 快速路径 (270x 热缓存读取提升)
- [x] PROD-001: BlockCache 多分片架构（支持动态缩容）

### Phase 7: v0.5.0 Large-Scale Performance ✅ (已完成)

- [x] PERF-005: 极小规模数据集性能优化 (100K keys 151ms → 101ms, 33% 提升)
- [x] POL-005: SparseIndex AHashMap 优化
- [x] POL-006: DashMap 高负载优化 (BlockCache 多分片间接优化)
- [x] TEST-002: 极小规模数据集基准测试 (10K/100K/1M keys)

### Phase 8: v0.6.0 Professional Benchmark 📋 (规划中)

- [ ] BENCH-001: 专业 Benchmark 体系 (10M keys + 写/读/空间放大率)
- [ ] PERF-006: 全局有序索引优化 (减少 segment 遍历)
- [ ] PERF-007: 10M keys 写入性能优化 (目标 <10x RocksDB 差距)
- [ ] TEST-003: 24h+ 稳定性测试
- [ ] DOC-001: 性能文档重写 (修正规模分级)
- [ ] POL-007: MemTable DashMap 高负载优化

---

## 相关文档

| 文档 | 职责 | 链接 |
|------|------|------|
| [README.md](../../README.md) | 快速参考：核心特性、性能数据、快速开始、配置预设 | 项目根目录 |
| [FILEKV_GUIDE.md](FILEKV_GUIDE.md) | 技术深度：架构详解、数据模型、读写路径、配置指南、故障排查、API 参考 | 本文档同级 |
| [POSITION_AND_STATUS.md](POSITION_AND_STATUS.md) | 路线图与状态：项目定位、已知限制、实现状态清单、生产就绪路线图 | 本文档 |

---

*本文档整合了原 FILEKV_POSITION.md 和 PROJECT_STATUS.md，消除内容重叠。*
*更新日期: 2026-04-16 (v0.5.0)*
