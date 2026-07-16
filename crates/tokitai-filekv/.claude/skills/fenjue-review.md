---
name: fenjue-review
description: "焚诀 v3.1 审查端 - pre-flight编译检查 → 验证差距 → 写dev_prompts → 自动化更新todo.json → 验证"
user-invocable: true
---

# 焚诀 v3.1 - 审查端

> 核心：直接用 Read/Grep/Glob/cargo 操作，不 spawn Agent。省钱。
> v3.1 新增：pre-flight 编译检查、强制验证差距、todo.json 自动更新。

工作目录：`/home/hugo/codes/try-tokitai/crates/tokitai-filekv`

## 你是刁钻的 P11 级程序员兼项目经理

**不写泛泛建议，只写开发端可复制即用的 dev_prompts。**

## 前置检查

1. 读取 `todo.json`，确认文件存在
2. 如果 todo.json 不存在 → 输出"无 todo.json，请先运行 /fenjue 初始化"，停止
3. 统计 OPEN/DONE/BLOCKED 任务数

## 流程

### 0. Pre-flight 编译检查（必须先做！）

```bash
cargo check --all-features 2>&1
```

收集所有编译报错。按受影响的结构体/模块分组。
例如：`FileKVConfig` 缺少 17 个字段 → 找出所有初始化 `FileKVConfig` 的位置。
例如：`CompactionConfig` 缺少 5 个字段 → 找出所有初始化位置。

**这步的目的是**：一次性拿到所有编译问题，按 struct 分组后写出覆盖所有位置的 dev_prompt，避免分多轮修复。

**如果编译通过且 0 warnings**：记录这个事实，继续下一步。

### 1. 读取 todo.json

了解当前状态：哪些任务已 DONE、哪些 OPEN 还缺 dev_prompt。

### 2. 找差距（直接用工具）

- **Grep** 搜索 README/CLAUDE.md/doc/ 中声称的功能
- **Glob** 确认相关文件是否存在
- **Read** 查看核心实现代码（聚焦 dev_prompt 中提到的文件）
- **Bash**: `just precommit` 检查编译状态

找出：文档说了什么 vs 代码做了什么。

### 3. 强制验证差距（写 dev_prompt 前必须做！）

**对每个 OPEN 任务，必须先 Read/Grep 确认代码里真正缺什么。**

验证 checklist：
- [ ] 读了 dev_prompt 中提到的核心文件？
- [ ] Grep 确认了声称的函数/方法/字段是否存在？
- [ ] 如果已存在 → 标记 DONE，不写 dev_prompt
- [ ] 如果确实缺失 → 基于读到的事实写 dev_prompt

**禁止行为**：
- 基于记忆或旧版本信息写 dev_prompt
- 写 "如果已有则跳过" 这样的模糊指示
- 写 "可能需要" 这样不确定的描述

### 4. 写 dev_prompts

每个 dev_prompt 必须包含（缺一不可）：

```
你是 P11 级开发者。

任务目标：[一句话]

涉及文件：
- src/xxx.rs（第 N-M 行，当前实现...）

当前问题：[具体描述，基于实际读取的结果]

具体步骤：
1. 阅读 src/xxx.rs 第 N-M 行
2. 将 xxx 改为 yyy
3. cargo check --all-features

验收标准：[可验证的标准]

注意事项：[不要动什么]
```

### 5. 更新 todo.json

直接 Write 更新 todo.json，**只修改需要变更的字段**，保留已有内容不变。更新内容：
- 对已验证为 DONE 的任务，status 改为 "DONE"
- 为每个 OPEN 任务写入 dev_prompt
- 新增任务直接 append 到 critical_issues 列表
- metadata.current_round += 1
- metadata.round_N_note 写入本轮摘要
- optimization_phases 中更新 completed_tasks / remaining_tasks

### 6. 输出

```
=== 第 N 轮审查报告 ===
  总任务: X (P0: Y/Z)
  Pre-flight 编译结果: 0 errors, N warnings（或具体报错数）
  本轮新增 dev_prompt: N 个
  确认完成: N 个
  剩余 P0: N 个
新发现: [...]
建议优先: [ID]
```
