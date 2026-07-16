# 可观测性与治理优化创新

> **状态**: ✅ 已实现  
> **引入版本**: v0.3.0 - v0.8.0 (多轮迭代)  
> **核心代码**: `src/ops/`

---

## 概述

可观测性是生产级 LSM-Tree 的关键特性,tokitai-filekv 实现了 4 项优化,构建完整的监控与治理体系。

---

## 1. Write/Read/Space Amplification Tracking (放大率追踪)

### 问题
写放大 (WA)、读放大 (RA)、空间放大 (SA) 无精确测量,运维无法获取真实放大率数据。

### 创新方案
`AmplificationTracker` 记录 logical vs disk vs WAL 的实际 I/O,精确计算放大率。

### 实现细节
- **文件**: `src/ops/amplification.rs`
- **放大率定义**:
  - **WA (Write Amplification)**: `total_bytes_written / user_bytes_written`
  - **RA (Read Amplification)**: `actual_disk_read / logical_read`
  - **SA (Space Amplification)**: `disk_usage / logical_data_size`
- **精确测量**:
  - ReadEngine.search_segment() I/O 精确计数
  - dense index 路径记录实际 entry 大小
  - sparse index 路径记录实际读取字节数
- **零除保护**: 计算公式带 zero-division protection

### 性能影响
- 运维可获取真实放大率数据
- 性能调优有据可依

### 相关测试
- `src/ops/amplification.rs` 内置测试
- `benches/05_range_compaction.rs` write_amplification 基准

---

## 2. Feature Flag Runtime Control (运行时特性开关)

### 问题
优化功能无法运行时开关,需要重新编译或重启。

### 创新方案
`FeatureFlagController` 支持运行时开启/关闭 INNO-001/INNO-002 等优化。

### 实现细节
- **文件**: `src/ops/feature_flag.rs`
- **核心结构**:
  - `FeatureFlag`: 特性标志定义
  - `FeatureFlagController`: 运行时控制
  - `FeatureState`: 状态枚举 (Enabled/Disabled)
  - `FeatureStateChange`: 状态变更记录
- **支持标志**:
  - INNO-001: Adaptive Bloom Cache
  - INNO-002: Zone Map
  - 其他优化功能
- **统计**: `FeatureFlagStats` 追踪标志使用

### 性能影响
- 运行时灵活调整优化策略
- A/B 测试支持

### 相关测试
- `src/ops/feature_flag.rs` 内置测试
- `tests/feature_flag_tests.rs` 集成测试

---

## 3. Audit Logging (审计日志)

### 问题
写入操作无审计,无法追踪数据变更历史。

### 创新方案
`AuditLogger` 记录所有写入操作,支持审计和故障排查。

### 实现细节
- **文件**: `src/ops/audit_log.rs`
- **结构体**:
  - `AuditLogger`: 审计日志记录器
  - `AuditEntry`: 审计条目
  - `AuditOperation`: 审计操作枚举
- **记录内容**:
  - 操作类型 (put/delete/compaction)
  - key/value (可选)
  - 时间戳
  - 操作来源
- **配置**: `AuditLogConfig` 控制日志级别和输出目标
- **统计**: `AuditLogStats` 追踪日志数量

### 性能影响
- 审计能力增强
- 故障排查效率提升

### 相关测试
- `src/ops/audit_log.rs` 内置测试

---

## 4. Memory Tracker (内存追踪器)

### 问题
内存使用无监控,无法准确追踪 MemTable 内存分配/释放。

### 创新方案
`MemoryTracker` 实时追踪 MemTable 内存分配/释放,精确测量实际使用量。

### 实现细节
- **文件**: `src/ops/memory_tracker.rs`
- **结构体**:
  - `MemoryTracker`: 内存追踪器
  - `MemoryUsage`: 内存使用报告
- **核心方法**:
  - `record_allocation()`: 记录分配
  - `record_deallocation()`: 记录释放
  - `actual_memory_bytes`: AtomicU64 实时追踪
- **与 MemTable 集成**:
  - `MemTable::with_memory_tracker()` 绑定追踪器
  - `insert()` 记录分配/释放 delta
  - `clear()` 报告总释放
- **限制配置**: `memory_limit_bytes` 控制最大内存

### 性能影响
- 内存使用精确监控
- OOM 风险降低

### 相关测试
- `src/ops/memory_tracker.rs` 内置测试
- `src/core/memtable.rs` 集成测试

---

## 📊 性能成果汇总

| 指标 | 优化前 | 优化后 | 提升 |
|------|--------|--------|------|
| WA 测量 | 公式估算 | **精确测量** | **AmplificationTracker** |
| RA 测量 | 无 | **精确测量** | **AmplificationTracker** |
| SA 测量 | 无 | **精确测量** | **AmplificationTracker** |
| 运行时开关 | 需重新编译 | **支持** | **FeatureFlag Controller** |
| 审计能力 | 无 | **完整审计** | **Audit Logger** |
| 内存监控 | 估算 | **精确测量** | **Memory Tracker** |

---

## 🔗 相关文档

- [可观测性设计](../filekv/OBSERVABILITY.md) (如存在)
- [Prometheus 指标](../filekv/METRICS.md) (如存在)
