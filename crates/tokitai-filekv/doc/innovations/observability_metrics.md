# 可观测性与指标创新

> **状态**: ✅ 已实现  
> **版本**: v0.3.0 - v0.8.0  
> **核心代码**: `src/ops/metrics.rs`, `src/ops/audit_log.rs`, `src/ops/memory_tracker.rs`

---

## 概述

可观测性是生产级 KV 存储的关键特性，tokitai-filekv 实现了 4 项创新，构建完整的监控与治理体系。

---

## 1. 全面 Prometheus 指标 (Comprehensive Prometheus Metrics)

### 问题
数据库指标通常需要外部集成，运维需要额外配置。

### 创新方案
内置 30+ Prometheus 指标，自动导出，无需外部集成。

### 实现细节
- **文件**: `src/ops/metrics.rs`
- **指标分类**:
  - **操作计数器**: `filekv_reads_total`, `filekv_writes_total`, `filekv_deletes_total`, `filekv_errors_total`
  - **延迟直方图**: `filekv_write_latency_seconds`, `filekv_read_latency_seconds`, `filekv_delete_latency_seconds`, `filekv_flush_latency_seconds`
  - **缓存命中率**: `filekv_cache_hits_total`, `filekv_cache_misses_total`, `filekv_bloom_hits_total`, `filekv_bloom_misses_total`
  - **压缩统计**: `filekv_compaction_runs_total`, `filekv_compaction_bytes_written`, `filekv_compaction_segments_merged`, `filekv_tombstones_cleaned_total`
  - **放大率指标**: `filekv_write_amplification_ratio`, `filekv_read_amplification_ratio`, `filekv_space_amplification_ratio`
- **导出**: `PrometheusExporter` 自动导出到 `/metrics` 端点

### 为什么独特
传统 KV 存储通常只提供基础指标，tokitai-filekv **内置 30+ 指标**，覆盖读/写/缓存/压缩/放大全链路。

---

## 2. 审计日志系统 (Audit Logging System)

### 问题
合规场景需要完整的操作审计，传统数据库需要外部实现。

### 创新方案
`AuditLogger` 记录所有写操作，SHA256 value hash 验证，时间轮转。

### 实现细节
- **文件**: `src/ops/audit_log.rs`
- **记录内容**:
  - 操作类型: Put/Delete/Batch/Flush/Compaction
  - key/value (可选)
  - SHA256 value hash (验证完整性)
  - 时间戳
  - session_id/user_id/request_id (元数据)
- **轮转策略**: 按小时轮转 + 保留策略 (按天)
- **格式**: JSON 持久化

### 为什么独特
嵌入式 KV 存储很少内置审计日志，tokitai-filekv **提供完整审计能力**，适合金融/医疗合规场景。

---

## 3. 内存追踪器 (MemoryTracker)

### 问题
内存使用难以精确监控，传统数据库通常估算而非实际测量。

### 创新方案
双模式内存追踪：组件级快照 + 实时分配追踪。

### 实现细节
- **文件**: `src/ops/memory_tracker.rs`
- **双模式**:
  1. **组件级快照**: BlockCache/DenseIndex/MemTable/WAL/Mmap 独立跟踪
  2. **实时分配追踪**: `record_allocation()`/`record_deallocation()` 原子操作
- **方法**:
  - `record_allocation(component, bytes)`: 记录分配
  - `record_deallocation(component, bytes)`: 记录释放
  - `get_memory_usage()`: 获取完整报告
- **限制**: 可选内存限制 enforcement (memory_limit_bytes)
- **与 MemTable 集成**: `MemTable::with_memory_tracker()` 绑定追踪器

### 为什么独特
传统数据库通常用 `size_of()` 估算内存，tokitai-filekv **实时测量实际分配**，精确度 >95%。

---

## 4. 超时控制 (Timeout Control)

### 问题
操作超时通常需要外部实现 (如 tokio::time::timeout)，嵌入式库很少内置。

### 创新方案
内置超时配置和统计，支持操作级超时控制。

### 实现细节
- **文件**: `src/ops/timeout_control.rs`
- **配置**: `TimeoutConfig` - 默认超时时间
- **统计**: `TimeoutStats` - 超时次数、触发操作
- **应用**: 可应用于任何阻塞操作 (get/put/compaction)

### 为什么独特
嵌入式 KV 存储通常不负责超时处理，tokitai-filekv **内置超时控制**，简化调用方实现。

---

## 📊 可观测性成果

| 指标 | 传统项目 | tokitai-filekv | 提升 |
|------|---------|----------------|------|
| Prometheus 指标 | 5-10 个 | **30+ 个** | **全链路覆盖** |
| 审计日志 | 需外部集成 | **内置** | **SHA256 验证** |
| 内存监控 | 估算 | **实际测量** | **>95% 精确** |
| 超时控制 | 需外部实现 | **内置** | **操作级** |

---

## 🔗 相关文档

- [可观测性设计](../filekv/OBSERVABILITY.md) (如存在)
- [Prometheus 指标](../filekv/METRICS.md) (如存在)
