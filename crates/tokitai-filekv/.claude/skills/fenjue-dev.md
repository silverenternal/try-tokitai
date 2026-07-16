---
name: fenjue-dev
description: "焚诀 v3.1 开发端 - 批量执行 dev_prompts，同模块任务合并到1-2个Agent，执行后回归验证"
user-invocable: true
---

# 焚诀 v3.1 开发端

你是 P11 级开发者。工作目录：`/home/hugo/codes/try-tokitai/crates/tokitai-filekv`

## 前置检查

1. 读取 `todo.json`，确认文件存在
2. 如果 todo.json 不存在或为空 → 输出"无 todo.json 或无任务"，停止
3. 找出所有 status="OPEN" 且有 dev_prompt 的任务
4. 如果没有这样的任务 → 输出"没有待执行的 dev_prompt"，列出所有 DONE 任务 ID，停止

## 流程

### 1. 读取 todo.json

找出所有 status="OPEN" 且有 dev_prompt 的任务。按 P0 → P1 → P2 排序。

### 2. 合并执行

**任务数 ≤ 3：直接在主上下文执行（不 spawn Agent）。**

**任务数 > 3：按模块分组，最多 2 个 Agent 并行。**

合并执行模板：
```
批量执行以下任务（按顺序）：

任务 1 (ID-XXX): [dev_prompt 原文]
任务 2 (ID-YYY): [dev_prompt 原文]

注意：
- 按顺序执行，不要跳
- 每个任务完成后运行 cargo check --all-features
- 完成后报告每个任务的状态和改了哪些文件
```

**执行规则**：
- 不自行解读 dev_prompt — 写了什么就做什么
- 直接修改代码，不要 spawn Agent 来读代码
- 每个任务完成后必须做回归验证

### 3. 回归验证（每个任务后必须做！）

```bash
# 1. 编译检查
cargo check --all-features 2>&1

# 2. 运行相关测试（用任务 ID 或模块名作为关键词）
cargo nextest run --all-features '<相关关键词>'
```

**如果发现新报错**：
- 同类 struct 的其他位置缺失字段 → 用 Grep 找出所有初始化位置，一次性全部修复
- 只有真正无法解决的问题才回审查端

**全部修完后**：
```bash
just precommit
```

确保 0 errors + 0 warnings。

### 4. 回填 exec_result

Write 更新 todo.json 中每个任务的 exec_result 字段：status + files_changed + summary + regression_test_passed。

### 5. 报告

```
=== 开发报告 ===
| 任务 ID | 状态 | 改了哪些文件 | 回归测试通过 |
| ID-XXX  | DONE | src/xxx.rs, src/yyy.rs | ✅ / ❌ |
```
