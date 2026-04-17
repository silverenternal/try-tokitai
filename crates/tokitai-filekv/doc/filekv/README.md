# FileKV 文档中心

**最后更新**: 2026-04-16 (v0.5.0 完成，Round 1-38)
**版本**: 0.5.0
**项目**: tokitai-filekv - 高性能纯文件 KV 存储引擎
**状态**: 实验性生产引擎 (630 lib tests + 32 integration tests 通过，0 clippy 警告，0 ignored)

---

## 📦 关于 tokitai-filekv

**tokitai-filekv** 是一个独立的高性能 KV 存储引擎 crate，基于 LSM-Tree 架构。

**起源**: 源自 tokitai-context 项目的存储引擎模块，现已独立为可复用的通用 KV 存储库。

**适用场景**: 嵌入式 KV 存储、日志系统、配置存储、缓存层、时间序列数据等

**Crates.io**: https://crates.io/crates/tokitai-filekv

---

## 📚 文档索引

### 核心文档

| 文档 | 描述 | 语言 | 阅读时间 |
|------|------|------|----------|
| [POSITION_AND_STATUS.md](POSITION_AND_STATUS.md) | **项目定位与实现状态** (整合版) | 中文 | 15 min |
| [FILEKV_GUIDE.md](FILEKV_GUIDE.md) | **FileKV 存储引擎完全指南** | 中文 | 30 min |
| [../SCALE_CLASSIFICATION.md](../SCALE_CLASSIFICATION.md) | **测试规模分级说明** (对齐工业界标准) | 中文 | 5 min |

> 性能数据汇总已统一维护在根目录 [README.md](../../README.md) 中，避免跨文档重复。

### 性能与基准测试

| 文档 | 描述 | 语言 | 阅读时间 |
|------|------|------|----------|
| [rocksdb_fair_comparison_2026_04_08.md](rocksdb_fair_comparison_2026_04_08.md) | RocksDB 公平对比实验报告 | 英文 | 10 min |

### 技术设计文档

| 文档 | 描述 | 语言 | 阅读时间 |
|------|------|------|----------|
| [FEATURE_FLAG_RUNTIME_CONTROL.md](FEATURE_FLAG_RUNTIME_CONTROL.md) | Feature Flag 运行时控制系统 | 中文 | 15 min |
| [RFC_INNO001_L2_L3_BLOOM_IMPLEMENTATION.md](RFC_INNO001_L2_L3_BLOOM_IMPLEMENTATION.md) | L2/L3 自适应 Bloom 实现 RFC | 中文 | 10 min |

### 专利文档

| 文档 | 描述 | 语言 | 阅读时间 |
|------|------|------|----------|
| [patent_disclosure_adaptive_bloom.md](patent_disclosure_adaptive_bloom.md) | 自适应 Bloom Filter 缓存专利交底书 | 中文 | 20 min |
| [patent_disclosure_zone_map.md](patent_disclosure_zone_map.md) | Zone Map 范围查询优化专利交底书 | 中文 | 20 min |

### 历史文档

| 文档 | 描述 |
|------|------|
| [archive/](archive/) | 历史报告、设计文档和 Sprint 完成报告 (20+ 文件) |

---

## 🚀 快速开始

### 想了解 FileKV 是什么？
→ 阅读 [POSITION_AND_STATUS.md](POSITION_AND_STATUS.md) 了解项目定位和当前状态

### 想了解技术细节？
→ 阅读 [FILEKV_GUIDE.md](FILEKV_GUIDE.md) 完整技术指南

### 想了解性能表现？
→ 阅读根目录 [README.md](../../README.md#性能表现与-rocksdb-公平对比) 的性能表格和 [rocksdb_fair_comparison_2026_04_08.md](rocksdb_fair_comparison_2026_04_08.md)

### 想了解创新点和专利？
→ 阅读专利交底书和 RFC 文档

---

## 📊 FileKV 核心数据

**性能数据**: 详见根目录 [README.md](../../README.md) 性能表格和 [PERFORMANCE_BASELINE.md](PERFORMANCE_BASELINE.md)（测试日期: 2026-04-16, Round 38）

**当前测试状态**: 630 lib tests + 32 integration tests (100% 通过)，0 clippy 警告

**状态说明**:
- v0.3.0 完成了 Phase 0/1 共 8 个关键修复：rebalance 执行引擎、SequentialPrefetch 消费、BlockCache 字节级限制等
- v0.3.1 修复了示例代码编译错误（audit_log 路径）
- v0.4.0 已完成：Dense Index 快速路径 (热缓存读取 278-285 ns) + BlockCache 多分片架构 + 9 个高并发测试解除 ignored
- v0.5.0 已完成：SparseIndex Clone 消除 + Bloom 缓存 10x 扩容 + DenseIndex AHashMap 优化 + 极小规模数据集基准测试（10K/100K/1M keys，**注：100K 仅作功能验证，不代表生产性能**），100K keys 写入性能提升 33%（151ms → 101ms）
- 已知性能限制：100K keys 真实场景比 RocksDB 慢约 161x（v0.5.0 优化，比 v0.4.0 的 240x 改善），**仅限极小规模场景**，详见根目录 README.md 和 [SCALE_CLASSIFICATION.md](../SCALE_CLASSIFICATION.md)
- **Round 35-38**: Benchmark 方法全面修复 — delete 全周期测量、put_batch API、compaction 实际执行、并发 Instant 测量、压缩真实操作
- v0.6.0 规划中：**10M+ keys 大规模性能**（P0 专业 benchmark）+ 写/读/空间放大率测量 + 全局有序索引 + 24h+ 稳定性测试

---

## 🔧 核心特性与集成状态

| 特性 | 状态 | 说明 |
|------|------|------|
| LSM-Tree 架构 | ✅ 已集成 | 核心架构完整 |
| MemTable (DashMap) | ✅ 已集成 | 无锁并发 |
| BlockCache (Moka TinyLFU) | ✅ 已集成 | 频率感知热点缓存 |
| Bloom Filter | ✅ 已集成 | 快速负向查找 |
| WAL | ✅ 已集成 | 崩溃恢复 |
| Compaction | ✅ 已集成 | 后台合并 |
| Zone Map Pruning | ✅ 已集成 | 范围查询优化 |
| Adaptive Bloom Cache | ✅ 已集成 | 动态 FPR 调整 |
| WAL Batch 写入 | ✅ 已集成 | 批量 flush，定期 fsync |
| Incremental Checkpoint | ✅ 已集成 | 需手动调用 |
| Sequential Prefetch | ✅ 部分集成 | 仅 Range Scan 受益 |
| Async I/O | 🟡 可选 | 仅 `put_buffered_async` 支持 |
| Timeout Control | 🟠 规划中 | 仅保护后台操作 |
| Memory Tracker | 🟠 规划中 | 数据为估算值 |
| Compaction Trigger | 🟠 规划中 | 使用固定计数器 |

> 完整的特性集成状态表见 [FILEKV_GUIDE.md](FILEKV_GUIDE.md#特性集成状态)。

---

## ⚠️ 使用建议

### ✅ 推荐使用场景

- 学术研究、论文验证
- 教学演示、学习 LSM-Tree
- 原型验证、技术探索
- 个人学习、Rust 实战
- 开发/测试环境小规模部署

### ❌ 不推荐使用场景

- 生产环境关键数据
- 高可靠性要求场景
- 大规模部署 (>100GB, >1000 QPS)
- 商业产品后端

---

## 📝 文档更新历史

> **2026-04-16**: v0.5.0 完成，Round 1-38 全部完成
> - 630 lib tests + 32 integration tests 全部通过
> - clippy 零警告
> - Round 1-38 涵盖：性能优化、死代码清理、Async I/O 集成、精确 I/O 计数、零拷贝、AtomicU64 stats、Mutex 锁优化、**Benchmark 方法修复**（delete 全周期测量、put_batch API、compaction 实际执行、并发 Instant 测量、压缩真实操作）
> - 性能数据统一维护在 PERFORMANCE_BASELINE.md
> - 已知性能差距：100K keys 真实场景比 RocksDB 慢约 161x（v0.5.0 优化，**仅限极小规模场景**）
> - v0.6.0 规划中：**10M+ keys 大规模性能**（P0 专业 benchmark）+ 写/读/空间放大率测量 + 全局有序索引 + 24h+ 稳定性测试

> **2026-04-14**: v0.4.0 完成
> - 整理 todo.json，聚焦 v0.4.0 性能优化目标
> - 更新测试状态：570 lib tests + 32 integration tests
> - 更正 ignored 测试位置：全部 9 个在 tests/filekv_integration/high_concurrency.rs
> - 更新文档反映 Phase 0-5 全部完成
> - 删除跨文档冗余内容

> **2026-04-14**: v0.3.1 示例代码修复
> - 修复 examples/ 中 audit_log 路径错误
> - 测试数更新: 410 → 413
> - 版本更新: 0.3.0 → 0.3.1
> - 所有文档版本统一

> **2026-04-13**: 文档整合 (DOC-008)
> - 合并 FILEKV_POSITION.md 和 PROJECT_STATUS.md → POSITION_AND_STATUS.md
> - 修复断裂链接，更新文档索引
> - 版本更新: 0.1.5 → 0.2.0
> - 更新测试状态: 295+ 通过, 3 #[ignore], 0 失败
> - 编译 warnings: 0

> **2026-04-11**: 🎉 六阶段架构重构完成
> - 新增 `SIX_PHASES_COMPLETION_REPORT_2026_04_11.md`
> - 更新测试状态: 255/255 (100%)
> - 版本更新: 0.1.4 → 0.1.5

> **2026-04-10**: 本文档目录已精简
> - 从 41 个文档精简至 17 个核心文档
> - 删除重复的性能报告、优化报告、代码审查报告

---

**许可证**: MIT OR Apache-2.0

**GitHub**: https://github.com/silverenternal/tokitai
