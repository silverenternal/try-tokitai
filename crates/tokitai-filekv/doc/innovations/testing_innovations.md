# 测试创新

> **状态**: ✅ 已实现  
> **版本**: v0.3.0 - v0.5.0  
> **核心代码**: `tests/`, `src/tests/`, `src/io/fault_inject.rs`, `src/io/memfs.rs`

---

## 概述

测试是 tokitai-filekv 质量保障的核心，实现了 3 项创新，构建完整的测试体系。

---

## 1. 630+ 分布式测试组织 (630+ Tests Distributed Organization)

### 问题
大型测试套件执行慢且难以维护，传统项目测试组织混乱。

### 创新方案
分布式组织测试，按功能模块分组，支持并行执行。

### 实现细节
- **分布**:
  - **Lib tests**: ~600 个 (分布式在源文件模块中)
  - **Integration tests**: 28 个 (`tests/filekv_integration/` 7 个独立文件)
  - **Perf tests**: 4 个 (`tests/opt004_perf_test.rs`)
  - **High concurrency tests**: 9 个 (`tests/filekv_integration/high_concurrency.rs`, 默认运行)
  - **Stability tests**: 3 个 (`tests/stability_24h.rs`, 默认 `#[ignore]`, 需手动运行 24h+)
- **并行执行**:
  - 推荐: `cargo nextest run --lib --test-threads 4`
  - 内置: `cargo test --lib --jobs 4`
  - 脚本: `./scripts/test.sh --nextest`
- **模块分布**: 46+ 测试模块

### 为什么独特
传统项目测试通常集中在 `tests/` 目录，tokitai-filekv **将测试分布式组织在源文件模块中**，每个模块自包含测试。

---

## 2. 故障注入测试 (Fault Injection Testing)

### 问题
难以测试崩溃恢复和错误处理，传统测试依赖真实故障场景。

### 创新方案
Decorator 模式包装任何 FileKVFileSystem，支持多种故障策略。

### 实现细节
- **文件**: `src/io/fault_inject.rs`
- **故障策略**:
  - `FailAfterN`: N 次后失败 (测试恢复逻辑)
  - `FailRandom`: 随机失败概率 (测试容错性)
  - `AlwaysFail`: 指定错误类型 (测试错误处理)
  - `Delay`: 操作延迟 (测试超时控制)
  - `Combined`: 组合策略 (复杂场景)
- **规则匹配**: 按操作前缀匹配规则 (read/write/open/close)
- **使用**: `FaultInjector::new().with_rule(FaultRule::new("write", FailStrategy::FailAfter(3)))`

### 为什么独特
数据库项目中很少内置故障注入框架，tokitai-filekv **提供完整的故障注入测试能力**，确保崩溃恢复正确性。

---

## 3. 内存文件系统抽象 (In-Memory Filesystem Abstraction)

### 问题
测试依赖真实磁盘 I/O，慢且不可控，CI 环境难以提供稳定磁盘。

### 创新方案
完整的内存文件系统实现 FileKVFileSystem trait，测试无磁盘 I/O 依赖。

### 实现细节
- **文件**: `src/io/memfs.rs`
- **实现**:
  - `MemFs`: 实现 `FileKVFileSystem` trait
  - `MemFile`: 实现 `FileKVFile` trait
  - 内存中模拟所有文件操作 (open/read/write/close/remove)
- **使用**: 测试中用 `MemFs` 替代 `StdFs`，无需磁盘 I/O
- **覆盖**: 所有文件操作 (除 mmap 外)

### 为什么独特
传统数据库测试依赖真实磁盘或临时文件，tokitai-filekv **提供完整内存文件系统**，测试速度提升 10-100x。

---

## 📊 测试成果

| 指标 | 传统项目 | tokitai-filekv | 提升 |
|------|---------|----------------|------|
| 测试数量 | 100-300 | **630+** | **2-6x 覆盖** |
| 测试组织 | 集中在 tests/ | **分布式模块** | **自包含** |
| 故障注入 | 手动模拟 | **内置框架** | **自动化** |
| 磁盘依赖 | 必需 | **可选 (MemFs)** | **10-100x 快** |
| 并行执行 | 顺序 | **4+ 并行** | **4x 快** |

---

## 🔗 相关文档

- [测试规则](../../CLAUDE.md#测试规则)
- [性能测试](tests/opt004_perf_test.rs)
