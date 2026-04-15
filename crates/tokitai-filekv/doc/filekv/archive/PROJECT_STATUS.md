# FileKV 项目状态报告

**最后更新**: 2026-04-13 (代码验证版 - 基于实际代码审查)
**版本**: v0.2.0
**状态**: 实验性生产引擎 (Experimental Production-Ready)

---

## 🎯 项目定位

FileKV 是一个**实验性生产级 LSM-Tree KV 存储引擎**。核心架构清晰（六阶段重构、四引擎拆分），具备生产级代码质量（四层错误体系、完整指标、崩溃安全机制），但仍在生产环境验证中。主要性能限制：100K keys 真实场景比 RocksDB 慢约 240x（151ms vs 628µs）。当前适合嵌入式 KV 场景、测试/开发环境，生产环境部署需充分评估。

---

## 当前实现状态

### ✅ 已完成特性

| 特性 | 状态 | 验证说明 |
|------|------|---------|
| **LSM-Tree 核心架构** | ✅ 100% | MemTable + Segment + SparseIndex + WAL |
| **六阶段重构** | ✅ 100% | Read/Write/Compaction/Lifecycle 四引擎拆分 |
| **审计日志** | ✅ 100% | 文件轮转、JSON 行写入（CRIT-001 已修复） |
| **Cache 统计语义** | ✅ 100% | Prometheus 精确反映 BlockCache 命中率（CRIT-003 已修复） |
| **Write Coalescer** | ✅ 100% | 100ms 时间窗口 + 64KB 阈值（MAJ-003 已修复） |
| **异步 I/O 死锁防护** | ✅ 100% | sync bridge 使用 spawn_blocking（MAJ-006 已修复） |
| **Compaction Manifest** | ✅ 100% | 崩溃安全机制 + 9 个 crash scenario 测试 |
| **错误体系** | ✅ 100% | Fatal/Transient/Expected/Domain 四层分类 |
| **I/O 抽象层** | ✅ 100% | FileKVFileSystem trait + StdFs/MemFs/FaultInjector |
| **Zone Map 块级剪枝** | ✅ 100% | 减少 40-60% I/O |
| **Examples 编译** | ✅ 100% | cargo check --examples 通过（CRIT-002 已修复） |
| **FPR 控制器接入** | ✅ 100% | record_fpr_access 在 6 处调用，pending_fpr_rebuilds lazy rebuild 机制（MAJ-001） |
| **UnifiedCacheManager** | ✅ 100% | 架构简化为直接管理独立缓存，预算 soft mode（MAJ-002） |
| **L2 压缩** | ✅ 100% | 默认启用，zstd 压缩正确（MAJ-005） |
| **Compaction metrics** | ✅ 100% | 4 参数精确统计：segments_merged, bytes_compacted, entries_removed, tombstones_cleaned（MAJ-007） |
| **Compaction 线程管理** | ✅ 100% | CompactionEngine 为唯一线程管理者，CompactionManager 仅作请求转发（MIN-008 已验证） |

### ⚠️ 部分实现/已知限制

| 特性 | 状态 | 说明 |
|------|------|------|
| **字典压缩** | 30% | 仅 zstd 压缩，字典训练为占位符（MAJ-004 已澄清） |
| **Async/Flush metrics** | 0% | put_async/delete_async 有指标，但 flush_memtable/flush_async/put_buffered_async 无专用指标（MAJ-007-PHASE2，待实现） |
| **Sequential Prefetch 消费** | 0% | 只记录访问模式，不消费 prefetch 数据（MIN-004，待实现） |
| **Bloom 迁移** | 部分 | 基于 LRU 淘汰，access_count 字段递增但未用于迁移决策（MIN-003，需添加注释） |
| **estimate_fpr_from_filter** | 占位符 | 返回硬编码 0.01，无法从 BloomFilter 反推真实 FPR（MIN-011，待修复） |
| **生产路径 unwrap()** | 待审计 | 代码库 687 处 unwrap()/expect()，大部分在测试中，生产路径需审计（MIN-001） |
| **大规模并发测试** | 0% | 32+ 线程性能待验证 |
| **完整 ACID** | 部分 | WAL 原子性有，无 MVCC/隔离级别 |

---

## 代码质量亮点

| 维度 | 状态 | 说明 |
|------|------|------|
| **架构设计** | ✅ 生产级 | 四引擎清晰拆分，EngineState 共享状态设计良好 |
| **错误体系** | ✅ 生产级 | 四层分类，thiserror 集成，is_retryable/is_fatal 分类 |
| **Compaction 统计** | ✅ 精确 | SegmentIterator 用 Arc<AtomicU64> 精确统计 tombstones，MergeIterator 有 duplicates_removed() |
| **FPR 机制** | ✅ 合理 | lazy rebuild 设计避免立即重建开销 |
| **I/O 抽象** | ✅ 优秀 | 支持故障注入，单元测试友好 |
| **崩溃安全** | ✅ 完善 | Compaction Manifest + WAL 恢复 + 9 个 crash scenario 测试 |

---

## 性能对比（公平对比，2026-04-08）

| 操作 | FileKV | RocksDB | 提升倍数 | 测试条件 |
|------|--------|---------|---------|---------|
| **Bloom Filter 负向查询** | 62.37 µs | 247.38 µs | **3.97x** | 纯内存 |
| **全 KV Get (热缓存)** | 61.92 µs | 600.07 µs | **9.69x** | 完整查询 |
| 写入 (64B, WAL) | 1.71 µs/entry | 1.88 µs/entry | FileKV 快 9% | KV 操作 |
| 写入 (100B, WAL) | 1.86 µs/entry | 1.83 µs/entry | RocksDB 快 2% | KV 操作 |
| **100K keys 真实场景** | ~151 ms | ~628 µs | **FileKV 慢 240x** ⚠️ | 完整工作负载 |

> **重要说明**：早期报告的"90-187x"优势是不公平对比（FileKV 热缓存 vs RocksDB 冷查询）。公平对比显示 **3-10x** 优势。详见 `doc/rocksdb_fair_comparison_2026_04_08.md`。

---

## 已知限制

### 数据可靠性
- ⚠️ WAL 可能丢失最近写入（操作系统缓存未 flush，依赖 fsync）
- ⚠️ 生产路径存在 unwrap()/expect() 调用（大部分在测试代码中，生产路径需审计和替换为 `?`）
- ⚠️ 无 ACID 保证，无事务隔离

### 并发控制
- ⚠️ WAL 锁可能成为高并发瓶颈
- ⚠️ 无读写隔离，Compaction 阻塞写入

### 内存管理
- ⚠️ UnifiedCacheManager 预算仅用于信息追踪（soft mode），不强制执行
- ⚠️ L1/L2 内存计算需进一步验证准确性

### 功能限制
- ⚠️ FPR BloomFilter 重建使用 lazy rebuild 机制（level 变化时标记，下次访问时重建）
- ⚠️ 字典压缩是 plain zstd（无字典训练）
- ⚠️ Sequential Prefetch 只记录访问模式，不消费预取数据
- ⚠️ flush_memtable 和 flush_async 无专用 Prometheus 延迟指标
- ⚠️ estimate_fpr_from_filter() 返回硬编码 0.01
- ⚠️ 100K keys 真实场景比 RocksDB 慢约 240x（151ms vs 628µs）

---

## 架构概览

```
FileKV (薄门面)
├── ReadEngine (读路径引擎)
│   ├── get() - KV 查找 (MemTable → BlockCache → Segments)
│   ├── Bloom Filter 加载与缓存（含 FPR 自适应）
│   ├── Zone Map 块级剪枝
│   └── Sequential Prefetch 顺序预取（仅记录模式）
├── WriteEngine (写路径引擎)
│   ├── put() / put_batch() / delete()
│   ├── WAL 管理与批量写入
│   ├── Write Coalescer 写入合并 (100ms + 64KB)
│   └── MemTable Flush 到 Segment
├── CompactionEngine (压缩引擎)
│   ├── run_compaction() - 同步压缩
│   ├── 后台异步 Compaction 线程（唯一线程管理者）
│   └── Compaction Manifest 崩溃安全机制
└── LifecycleManager (生命周期管理)
    ├── open() / recover() - 初始化与恢复
    ├── Checkpoint 创建与恢复
    ├── 审计日志（文件轮转）
    └── Prometheus 指标导出
```

**线程管理说明**：
- `CompactionEngine` 持有 `thread_handle`、`rx`（接收端）、`tx`（发送端）
- `CompactionManager` 仅作为请求转发器，其 `tx` 被 Engine 覆盖
- `run_compaction_thread_async()` 函数存在但未使用（遗留代码）

---

## 修复进度

### ✅ 已完成

| 问题 | 类型 | 验证方式 |
|------|------|---------|
| CRIT-001: 审计日志失效 | Critical | ✅ 代码验证：open() 调用 open_log_file()，log_operation() 写入 JSON 行 |
| CRIT-002: Examples 无法编译 | Critical | ✅ cargo check --examples 通过 |
| CRIT-003: Cache 统计语义错误 | Critical | ✅ 代码验证：get() 根据 CacheLookupResult 精确统计 |
| MAJ-001: FPR 控制器接入 | Major | ✅ 代码验证：record_fpr_access 在 6 处调用，pending_fpr_rebuilds 机制实现 |
| MAJ-002: 缓存预算死代码 | Major | ✅ 架构简化：adapters.rs 删除，预算转 soft mode |
| MAJ-003: Write Coalescer 时间窗口 | Major | ✅ 代码验证：time_window_us = 100_000 (100ms) |
| MAJ-004: 字典压缩命名误导 | Major | ✅ 添加注释澄清：当前是 plain zstd，字典训练为占位符 |
| MAJ-005: L2 压缩禁用 | Major | ✅ 代码验证：l2_compression_enabled = true，zstd 压缩正确 |
| MAJ-006: 异步 I/O 死锁风险 | Major | ✅ CHANGELOG 确认：sync bridge 使用 spawn_blocking |
| MAJ-007: Compaction metrics | Major | ✅ 代码验证：4 参数精确统计，tombstones 用 Arc<AtomicU64> |
| MAJ-008: ZoneMap 重复调用 | Major | ✅ 代码验证：两次调用语义不同（segment-level vs block-level） |
| MIN-002: Prefetch 预算死代码 | Minor | ✅ 架构简化副作用，纯信息追踪 |
| MIN-005/006/009/010: 代码质量 | Minor | ✅ 已修复/adapters.rs 删除 |
| MIN-007: Compaction tombstone 计数 | Minor | ✅ 精确统计：SegmentIterator + MergeIterator |
| MIN-008: Compaction 线程统一 | Minor | ✅ 代码验证：CompactionEngine 为唯一管理者，CompactionManager 仅转发 |
| DOC-001: 项目定位矛盾 | Doc | ✅ 统一为"实验性生产引擎" |
| DOC-002: 版本/性能数据过时 | Doc | ✅ 更新为 v0.2.0，公平对比 |
| DOC-004: 性能警告不醒目 | Doc | ✅ 顶部添加声明 |
| DOC-006: API 稳定性声明 | Doc | ✅ 已添加：核心 API 稳定，内部模块可能变更 |

### ❌ 剩余工作

| 问题 | 类型 | 预估工时 | 优先级 |
|------|------|---------|-------|
| MAJ-007-PHASE2: Async/Flush metrics | Major | 2-3 小时 | 🔴 高 |
| MIN-001: 生产路径 unwrap() 审计 | Minor | 4-6 小时 | 🟡 中 |
| MIN-003: access_count 注释 | Minor | 0.5 小时 | 🟡 中 |
| MIN-004: Sequential Prefetch 消费 | Minor | 3-4 小时 | 🟡 中 |
| MIN-011: estimate_fpr_from_filter | Minor | 2 小时 | 🟡 中 |
| DOC-005: CHANGELOG 修正 | Doc | 2-3 小时 | 🔴 高 |
| **总计** | | **14-20 小时** | |

---

## 未来路线图

### v0.2.1: 代码质量清理（规划中，14-20小时）
- [ ] Async/Flush metrics 实现 (MAJ-007-PHASE2, 2-3h)
- [ ] CHANGELOG v0.2.0 声明修正 (DOC-005, 2-3h)
- [ ] 生产路径 unwrap() 审计与替换 (MIN-001, 4-6h)
- [ ] access_count 注释澄清 (MIN-003, 0.5h)
- [ ] Sequential Prefetch 消费实现 (MIN-004, 3-4h)
- [ ] estimate_fpr_from_filter 修复 (MIN-011, 2h)

### v0.3.0: 功能增强（可选，20-30小时）
- [ ] 真正的字典压缩（zstd dictionary training）(MAJ-004 方案 A, 8-10h)
- [ ] 基于访问频率的 Bloom 分层迁移 (MIN-003 方案 B, 3-5h)
- [ ] 大规模并发测试（32/64 线程）(8-12h)
- [ ] 集成测试套件 (4-6h)

### v1.0.0: 稳定版（远期规划，60-80 小时）
- [ ] 完整 ACID 支持（MVCC、快照隔离）
- [ ] 企业级运维工具（备份、监控、自动化）
- [ ] 分布式支持（复制、故障转移）
- [ ] SLA 保证和压力测试

---

## 文档索引

### 核心文档
- [README.md](../../README.md) - 项目概览、快速开始、性能对比
- [CHANGELOG.md](../../CHANGELOG.md) - 版本历史（注：v0.2.0 部分声明需修正，见 DOC-005）
- [FILEKV_GUIDE.md](FILEKV_GUIDE.md) - 用户指南
- [FILEKV_POSITION.md](FILEKV_POSITION.md) - 项目定位说明
- [PROJECT_STATUS.md](PROJECT_STATUS.md) - 项目状态报告（本文档）

### 技术文档
- [FEATURE_FLAG_RUNTIME_CONTROL.md](FEATURE_FLAG_RUNTIME_CONTROL.md) - 功能特性运行时控制
- [rocksdb_fair_comparison_2026_04_08.md](rocksdb_fair_comparison_2026_04_08.md) - RocksDB 公平对比
- [RFC_INNO001_L2_L3_BLOOM_IMPLEMENTATION.md](RFC_INNO001_L2_L3_BLOOM_IMPLEMENTATION.md) - INNO-001 RFC
- [PERFORMANCE_REPORT.md](PERFORMANCE_REPORT.md) - 综合性能报告
- [patent_disclosure_adaptive_bloom.md](patent_disclosure_adaptive_bloom.md) - 自适应 Bloom 专利
- [patent_disclosure_zone_map.md](patent_disclosure_zone_map.md) - Zone Map 专利

### 历史报告（已归档）
- [archive/](archive/) 目录包含 21 个历史报告文件

---

## 联系方式

**作者**: Silverenternal
**项目**: https://github.com/silverenternal/tokitai-context
**许可证**: MIT / Apache-2.0

---

*本文档基于实际代码验证结果编写，应与代码实现保持同步。*
*最后验证：2026-04-13，通过代码审查验证关键声明的准确性。*
