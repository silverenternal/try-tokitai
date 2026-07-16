# 工程质量与工具链创新

> **状态**: ✅ 已实现  
> **版本**: v0.3.0 - v0.5.0  
> **核心文件**: `.rustfmt.toml`, `clippy.toml`, `justfile`, `scripts/`

---

## 概述

工程质量是 tokitai-filekv 的基石，实现了 4 项创新，构建完整的工程质量保障体系。

---

## 1. 零警告编译 (Zero-Warning Compilation)

### 问题
编译警告通常被忽略，累积后难以清理，影响代码质量。

### 创新方案
CI 强制将警告视为错误，保持 0 clippy warnings。

### 实现细节
- **文件**: `.rustfmt.toml`, `clippy.toml`
- **CI 配置**: `cargo clippy --all-features --all-targets -- -D warnings`
- **成果**:
  - 630+ tests 全部通过
  - 0 clippy warnings
  - 0 dead_code (除 feature flag stubs)

### 为什么独特
多数项目容忍警告累积，tokitai-filekv **将警告视为错误**，强制保持代码质量。

---

## 2. 完整的 just 工作流 (Justfile Workflow)

### 问题
开发命令分散且难以记忆，新人上手困难。

### 创新方案
提供完整的 just 工作流，集成所有常用操作。

### 实现细节
- **文件**: `justfile`
- **常用命令**:
  - `just check` — cargo check --all-features
  - `just lint` — cargo clippy --all-features --all-targets -- -D warnings
  - `just fmt` — cargo fmt --all
  - `just test` — cargo nextest run --all-features
  - `just bench` — cargo bench --features benchmarks
  - `just precommit` — fmt + clippy + check
  - `just save-baseline` — 保存性能基线
  - `just check-regression` — 检查性能回归

### 为什么独特
传统项目需要记忆多个 cargo 命令和参数，tokitai-filekv 提供**一站式工作流**，新人可直接使用。

---

## 3. 性能回归检测框架 (Performance Regression Framework)

### 问题
性能回归通常在发布后才发现，修复成本高。

### 创新方案
支持保存基准线、自动对比回归、PR 检查流程。

### 实现细节
- **文件**: `scripts/bench-regression.sh`, `scripts/save-baseline.sh`, `scripts/quick_perf_check.sh`
- **工作流**:
  1. `just save-baseline` — 保存当前性能基线
  2. 修改代码后 `just check-regression` — 自动对比
  3. PR 检查: >5% 回归需说明，>15% 默认阻止合入
- **覆盖**: 所有核心操作 (get/put/delete/compaction)

### 为什么独特
传统项目通常在 CI 中只跑功能测试，tokitai-filekv **将性能测试纳入 CI**，提前发现回归。

---

## 4. 性能预算体系 (Performance Budget System)

### 问题
性能目标通常模糊且不可量化，PR 审查无法判断是否突破性能底线。

### 创新方案
为每个操作设定硬性上限，附带当前基线和裕度百分比。

### 实现细节
- **文件**: `doc/filekv/PERFORMANCE_BUDGET.md`
- **预算示例**:
  - Hot cache get: < 400ns (当前: 278-285ns, 裕度: 29-30%)
  - Bloom 负向查询: < 15µs (当前: 7.23µs, 裕度: 52%)
  - 写入 (64B, WAL): < 5µs (当前: 1.57µs, 裕度: 69%)
  - Compaction 触发: < 10ms (当前: 2.95ms, 裕度: 71%)
- **PR 检查**: 任何 PR 不得突破预算

### 为什么独特
传统项目缺乏量化性能目标，tokitai-filekv 提供**硬性性能预算**，确保每次提交都不突破底线。

---

## 📊 质量成果

| 指标 | 传统项目 | tokitai-filekv | 提升 |
|------|---------|----------------|------|
| 编译警告 | 10-100+ | **0** | **-D warnings** |
| 开发命令记忆 | 10+ 个 | **8 个 just** | **一站式** |
| 性能回归发现 | 发布后 | **PR 阶段** | **提前发现** |
| 性能目标 | 模糊 | **量化预算** | **硬性上限** |

---

## 🔗 相关文档

- [性能预算](../filekv/PERFORMANCE_BUDGET.md)
- [性能基线](../filekv/PERFORMANCE_BASELINE.md)
