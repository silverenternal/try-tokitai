# 错误处理与安全创新

> **状态**: ✅ 已实现  
> **版本**: v0.3.0 - v0.5.0  
> **核心代码**: `src/core/error.rs`, `unwrap_audit.md`

---

## 概述

错误处理和安全是 tokitai-filekv **最具特色**的创新之一，实现了 4 项创新，构建完整的错误处理体系。

---

## 1. 四层错误体系 (4-Tier Error Hierarchy)

### 问题
传统错误处理无法区分：
- 可重试错误 (超时/背压)
- 不可重试错误 (数据损坏)
- 预期错误 (KeyNotFound)
- 领域错误 (配置错误)

### 创新方案
定义四层错误体系，每层有明确语义：
- **FatalError**: 数据损坏，必须中止 (不可重试)
- **TransientError**: 资源耗尽/超时/背压 (可重试)
- **ExpectedError**: KeyNotFound 等正常控制流 (非真正错误)
- **DomainError**: 配置错误/压缩失败 (需修复后重试)

### 实现细节
- **文件**: `src/core/error.rs`
- **方法**:
  - `is_retryable()`: 是否可重试
  - `is_fatal()`: 是否致命
  - `is_expected()`: 是否预期错误
- **模式匹配**: `match error { FileKVError::Fatal(e) => ... }`

### 为什么独特
Rust 生态中多数库用 `anyhow::Error` 或简单 `io::Error`，tokitai-filekv 提供**语义化四层分类**，调用方可精确判断重试策略。

---

## 2. 生产路径 0 unwrap() (Zero unwrap() in Production)

### 问题
`unwrap()` 在生产代码中可能导致 panic，尤其边界条件未覆盖时。

### 创新方案
通过定期审计，生产路径 **0 处** `unwrap()`，全部 613 处 `unwrap()` 均在测试/文档注释中。

### 实现细节
- **文件**: `unwrap_audit.md`
- **审计范围**: 所有生产代码 (排除 `#[cfg(test)]` 和测试文件)
- **审计结果**:
  - 总 `unwrap()`: ~613
  - 测试模块: ~613
  - **生产路径**: **0**
- **审计频率**: 每轮优化后自动审计

### 为什么独特
大中型 Rust 项目中几乎不可能做到 0 unwrap()，tokitai-filekv 通过**定期审计 + 严格规范**实现这一成就。

---

## 3. 属性测试框架 (Property-Based Testing)

### 问题
传统单元测试只覆盖有限场景，无法发现边界条件 bug。

### 创新方案
使用 `proptest` 框架定义不变量 (invariants)，自动生成边界条件测试。

### 实现细节
- **文件**: `src/tests/property_tests.rs`
- **10 个属性测试**:
  1. `prop_read_your_writes_single` - 读你写的 (单条)
  2. `prop_read_your_writes_batch` - 读你写的 (批量)
  3. `prop_delete_idempotent` - 删除幂等性
  4. `prop_delete_visibility` - 删除可见性
  5. `prop_range_query_completeness` - 范围查询完整性
  6. `prop_overwrite_latest_value` - 覆盖最新值
  7. `prop_delete_put_cycle` - 删除/写入循环
  8. `prop_get_nonexistent_key` - 不存在的 key
  9. `prop_lsm_consistency_after_compaction` - Compaction 后一致性
  10. `prop_delete_persistence` - 删除持久性
- **完成时间**: 30s 内

### 为什么独特
数据库项目中很少使用属性测试，tokitai-filekv 定义 **10 个 LSM-Tree 不变量**，自动生成边界条件测试。

---

## 4. Bloom Filter 序列化 v2 格式 (Bloom Serialization v2)

### 问题
v1 格式缺乏元数据，重建 Bloom Filter 需要重新计算位数组大小，效率低。

### 创新方案
v2 格式预存 `num_bits` 和 `num_hashes`，支持直接重建而非重新计算，同时保持 v1 向后兼容。

### 实现细节
- **文件**: `src/bloom/manager.rs`, `src/bloom/migration.rs`
- **v1 格式**: `[magic 4B][version 4B][num_keys 8B][keys...]`
- **v2 格式**: `[magic 4B][version 4B][num_bits 4B][num_hashes 4B][num_keys 8B][keys...]`
- **重建速度**: v1 需重新计算，v2 直接加载
- **向后兼容**: v1 格式仍然可读取，自动升级到 v2

### 为什么独特
传统序列化格式通常缺乏元数据，tokitai-filekv 提供**向后兼容的格式升级**，确保平滑迁移。

---

## 📊 安全成果

| 指标 | 传统项目 | tokitai-filekv | 提升 |
|------|---------|----------------|------|
| 错误分类 | 1-2 层 | **4 层** | **精确语义** |
| 生产 unwrap() | 10-100+ | **0** | **零 panic** |
| 边界条件测试 | 手动编写 | **自动生成** | **属性测试** |
| 序列化兼容性 | 手动处理 | **自动升级** | **向后兼容** |

---

## 🔗 相关文档

- [unwrap 审计报告](../../unwrap_audit.md)
- [API 稳定性承诺](../../docs/API_STABILITY.md)
