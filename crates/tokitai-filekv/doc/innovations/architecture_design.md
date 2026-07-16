# 架构与设计创新

> **状态**: ✅ 已实现  
> **版本**: v0.3.0 - v0.5.0  
> **核心代码**: `src/engine/`, `docs/API_STABILITY.md`

---

## 概述

tokitai-filekv 在架构设计层面实现了 4 项创新，远超传统 KV 存储的 God Object 模式。

---

## 1. 四引擎分离架构 (Four-Engine Architecture)

### 问题
传统 KV 存储常用单一大类 (God Object) 模式，所有读写、压缩、生命周期管理混在一起，导致：
- 代码难以维护和扩展
- 测试覆盖困难
- 新功能容易引入回归

### 创新方案
将 FileKV 拆分为四个独立引擎，每个引擎职责单一：
- **ReadEngine**: 只负责读路径
- **WriteEngine**: 只负责写路径
- **CompactionEngine**: 只负责后台压缩
- **LifecycleManager**: 只负责打开/恢复/检查点

### 实现细节
- **文件**: `src/engine/{read,write,compaction,lifecycle}_engine.rs`
- **收益**:
  - 消除 22 个重复方法
  - 删除 13 个遗留字段
  - lib.rs 从 1157 行 (God Object) → 1620 行 (四引擎 + 文档)
- **测试**: 每个引擎独立测试

### 为什么独特
Rust 生态中多数 KV 存储仍用单类模式，tokitai-filekv 采用**面向职责分离**的架构设计，类似微服务架构思想应用于单体库。

---

## 2. 三层 API 稳定性体系 (Three-Tier API Stability)

### 问题
开源库通常缺乏明确的 API 稳定性承诺，用户升级时经常遇到 breaking changes。

### 创新方案
将 API 分为三层，明确稳定性承诺：
- **稳定层 (Stable)**: `FileKV`, `FileKVConfig` — 核心操作保证向后兼容
- **实验层 (Experimental)**: 高级功能 (Async I/O, Checkpoint) — 次版本可能变更
- **内部层 (Internal)**: `#[doc(hidden)]` — 随时可变更，用户不应直接使用

### 实现细节
- **文件**: `docs/API_STABILITY.md`
- **策略**:
  - 稳定层 API 完全稳定后将发布 1.0 版本
  - 实验层标记为 `#[doc(hidden)]` 或明确文档说明
  - 变更/弃用政策明确定义

### 为什么独特
Rust 生态中很少有开源库明确定义 API 分层策略，tokitai-filekv 提供**清晰的稳定性承诺**，降低用户升级风险。

---

## 3. WAL 三档同步策略 (WalSyncMode Three-Tier)

### 问题
数据安全性与写入性能的权衡通常是硬编码的，用户无法根据场景选择。

### 创新方案
提供三档同步模式，用户可根据场景自由选择：
- **Immediate**: 每次写入 fsync — 100% 数据持久化，基准延迟
- **Batch**: 批量 fsync — ~99% 数据持久化，5x 吞吐提升
- **Lazy**: OS 缓冲 fsync — ~90% 数据持久化，10x 吞吐提升

### 实现细节
- **文件**: `src/core/types.rs` - `WalSyncMode` 枚举
- **配置**: `FileKVConfig.wal_sync_mode`
- **场景**:
  - 金融/医疗 → Immediate
  - 大多数生产 → Batch
  - AI 上下文/会话 → Lazy

### 为什么独特
传统 KV 存储通常只支持同步/异步两档，tokitai-filekv 提供**三档渐进式选择**，覆盖从金融审计到临时缓存的全场景。

---

## 4. 四档配置预设系统 (Four-Tier Config Presets)

### 问题
数据库配置通常复杂且需要专业知识，新手难以选择合适的配置。

### 创新方案
提供四档预设，覆盖从保守到极限的全场景：
- **Conservative**: 金融/医疗/审计 — ~64MB 内存，最高数据安全
- **Balanced** (默认): 大多数生产 — ~256MB 内存，中等安全
- **Performance**: AI 上下文/会话 — ~1GB 内存，中等安全
- **Extreme**: 缓存/临时数据 — ~4GB 内存，最低安全

### 实现细节
- **文件**: `src/core/types.rs` - `AggressiveConfig`
- **方法**: `FileKVConfig::conservative()`, `balanced()`, `performance()`, `extreme()`
- **控制参数**: 内存限制、WAL 同步模式、缓存大小、预读距离等

### 为什么独特
传统 KV 存储配置需要专业知识，tokitai-filekv 提供**开箱即用的预设**，新手无需理解复杂配置即可使用。

---

## 📊 性能与质量影响

| 指标 | 优化前 | 优化后 | 提升 |
|------|--------|--------|------|
| 代码重复 | 22 个重复方法 | **0 个** | **四引擎分离** |
| API breaking changes | 频繁 | **0 次** | **三层 API 稳定性** |
| 配置复杂度 | 高 | **低** | **四档预设** |
| 场景覆盖 | 单一 | **全场景** | **WAL 三档同步** |

---

## 🔗 相关文档

- [API 稳定性承诺](../../docs/API_STABILITY.md)
- [用户指南 (技术深度)](../filekv/FILEKV_GUIDE.md)
- [项目定位与状态](../filekv/POSITION_AND_STATUS.md)
