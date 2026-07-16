# API 文档快速参考

**版本**: v0.5.0+
**最后更新**: 2026-04-16

---

## 📚 文档导航

### 如果你是...

#### 🔰 新用户

**从这里开始**：
1. [README.md](../README.md) - 项目概述和快速开始
2. [FILEKV_GUIDE.md](../doc/filekv/FILEKV_GUIDE.md) - 用户指南（架构、配置、使用示例）
3. [API_REFERENCE.md](API_REFERENCE.md) - API 参考（查找你需要的功能）

#### 🛠️ 开发者

**你需要**：
- [API_REFERENCE.md](API_REFERENCE.md) - 完整 API 参考
- [API_STABILITY.md](API_STABILITY.md) - 哪些 API 稳定？哪些可能变更？
- [API_REVIEW.md](API_REVIEW.md) - 当前 API 的问题和改进建议

#### 📊 性能工程师

**你需要**：
- [PERFORMANCE_BASELINE.md](../doc/filekv/PERFORMANCE_BASELINE.md) - 性能指标数据
- [PERFORMANCE_BUDGET.md](../doc/filekv/PERFORMANCE_BUDGET.md) - 性能预算和限制
- [SCALE_CLASSIFICATION.md](../doc/SCALE_CLASSIFICATION.md) - 测试规模分级

#### 🔍 评估者

**你需要**：
- [POSITION_AND_STATUS.md](../doc/filekv/POSITION_AND_STATUS.md) - 项目定位和已知限制
- [V5_FINAL_POSITIONING.md](V5_FINAL_POSITIONING.md) - v0.5.0 最终定位
- [RocksDB 公平对比](../doc/rocksdb_fair_comparison_2026_04_08.md) - 性能对比数据

---

## 🎯 快速查找

### 我想...

| 目标 | 入口 | 文档 |
|------|------|------|
| **打开存储** | `FileKV::open()` | [API_REFERENCE.md §1.1](API_REFERENCE.md#11-filekv) |
| **写入数据** | `FileKV::put()` | [API_REFERENCE.md §1.2](API_REFERENCE.md#12-基本操作) |
| **读取数据** | `FileKV::get()` | [API_REFERENCE.md §1.2](API_REFERENCE.md#12-基本操作) |
| **批量写入** | `FileKV::put_batch()` | [API_REFERENCE.md §1.2](API_REFERENCE.md#12-基本操作) |
| **范围扫描** | `FileKV::range()` | [API_REFERENCE.md §1.3](API_REFERENCE.md#13-范围操作) |
| **配置存储** | `FileKVConfig` | [API_REFERENCE.md §2](API_REFERENCE.md#2-配置-api) |
| **调优缓存** | `BlockCacheConfig` | [API_REFERENCE.md §3](API_REFERENCE.md#3-缓存-api) |
| **配置压缩** | `BlockCompressionConfig` | [API_REFERENCE.md §2.5](API_REFERENCE.md#25-blockcompressionmode) |
| **使用检查点** | `CheckpointChain` | [API_REFERENCE.md §6](API_REFERENCE.md#6-检查点-api) |
| **监控内存** | `MemoryTracker` | [API_REFERENCE.md §7](API_REFERENCE.md#7-监控-api) |
| **审计操作** | `AuditLogger` | [API_REFERENCE.md §7.4](API_REFERENCE.md#74-auditlogger) |

### 我想知道...

| 问题 | 文档 |
|------|------|
| 哪些 API 稳定？ | [API_STABILITY.md §2](API_STABILITY.md#2-api-稳定性层级) |
| 性能指标是什么？ | [PERFORMANCE_BASELINE.md](../doc/filekv/PERFORMANCE_BASELINE.md) |
| 性能预算是什么？ | [PERFORMANCE_BUDGET.md](../doc/filekv/PERFORMANCE_BUDGET.md) |
| 如何升级版本？ | [API_STABILITY.md §10](API_STABILITY.md#10-升级指南) |
| API 变更政策？ | [API_STABILITY.md §5](API_STABILITY.md#5-变更政策) |
| 如何贡献文档？ | [API_IMPROVEMENT_SUMMARY.md §7](API_IMPROVEMENT_SUMMARY.md#7-贡献者指南) |

---

## 📋 文档清单

### 核心文档 (必须阅读)

| 文档 | 位置 | 目标读者 |
|------|------|---------|
| README | `README.md` | 所有用户 |
| API 参考 | `docs/API_REFERENCE.md` | 开发者 |
| API 稳定性承诺 | `docs/API_STABILITY.md` | 开发者、评估者 |
| 用户指南 | `doc/filekv/FILEKV_GUIDE.md` | 新用户 |

### 技术文档 (按需阅读)

| 文档 | 位置 | 目标读者 |
|------|------|---------|
| 性能基线 | `doc/filekv/PERFORMANCE_BASELINE.md` | 性能工程师 |
| 性能预算 | `doc/filekv/PERFORMANCE_BUDGET.md` | 性能工程师 |
| 规模分级 | `doc/SCALE_CLASSIFICATION.md` | 评估者 |
| 项目定位 | `doc/filekv/POSITION_AND_STATUS.md` | 评估者 |
| RocksDB 对比 | `doc/rocksdb_fair_comparison_2026_04_08.md` | 评估者 |

### 审查文档 (维护者)

| 文档 | 位置 | 目标读者 |
|------|------|---------|
| API 审查报告 | `docs/API_REVIEW.md` | 维护者 |
| API 改进总结 | `docs/API_IMPROVEMENT_SUMMARY.md` | 维护者 |

---

## 🔗 外部链接

| 资源 | 链接 |
|------|------|
| **在线 API 文档** | https://docs.rs/tokitai-filekv |
| **Crates.io** | https://crates.io/crates/tokitai-filekv |
| **GitHub** | https://github.com/silverenternal/tokitai |
| **Issues** | https://github.com/silverenternal/tokitai/issues |

---

## ⚠️ 注意

### 文档时效性

| 文档类型 | 更新频率 | 最后更新 |
|---------|---------|---------|
| README | 版本发布时 | v0.5.0 |
| API_REFERENCE | API 变更时 | 2026-04-16 |
| API_STABILITY | 版本发布时 | 2026-04-16 |
| PERFORMANCE_BASELINE | 基准测试运行后 | 2026-04-16 |
| API_REVIEW | 季度审查时 | 2026-04-16 |

### 已知问题

- 🔴 部分配置字段缺少 `#[doc]` 注释 (见 [API_REVIEW.md §2.2](API_REVIEW.md#22-缺少文档注释的字段))
- 🟡 部分内部类型过度暴露 (见 [API_REVIEW.md §3](API_REVIEW.md#3-过度暴露问题-over-exposure-issues))
- 🟡 文档与代码存在少量不一致 (见 [API_REVIEW.md §4](API_REVIEW.md#4-不一致问题-inconsistencies))

---

**本文件是 tokitai-filekv API 文档的快速入口。详细内容和承诺见各文档。**
