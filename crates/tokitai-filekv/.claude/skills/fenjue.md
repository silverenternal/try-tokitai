---
name: fenjue
description: "焚诀 v3.1 完整一轮 - 审查端 pre-flight 编译检查 + 验证差距后写 prompts + 自动化更新 todo.json + 开发后回归验证。目标每轮3-4次调用。参数: continue|dev|review|status|loop"
user-invocable: true
---

# 焚诀 (FenJue) v3.1 - 低成本高效版

> **设计原则**：能用工具直接干的，不 spawn Agent；必须 spawn 的，尽量合并。目标是每轮 3-4 次调用完成 8-10 个任务。
>
> **持久化循环**：每轮完成后检查终止条件，满足则询问用户是否进入下一轮。
>
> **v3.1 新增 4 项优化**（基于 Round 1 实战反馈）：
> 1. Pre-flight 编译检查：先跑 `cargo check` 拿全量报错，按 struct 分组一次性写出覆盖所有位置的 dev_prompt
> 2. 强制验证差距：写 dev_prompt 前必须 Read/Grep 确认代码真正缺什么，禁止基于记忆/假设写
> 3. todo.json 自动更新：审查端直接 Write 更新，不手动描述变更
> 4. 执行后回归验证：开发端每个任务后跑 cargo check + nextest，发现同类问题直接修不回退

## 循环状态管理

工作目录：`/home/hugo/codes/try-tokitai/crates/tokitai-filekv`

状态文件：`.claude/fenjue-state.json`（每次 round 完成后更新）

每次 round 结束后**必须**执行：
1. 读取 `.claude/fenjue-state.json`
2. 更新 `current_round`、`last_round_status`、`last_updated`
3. 检查终止条件
4. 如果未终止 → Write 更新 `loop_running: true` → 输出报告，询问用户是否继续
5. 如果终止 → Write 更新 `loop_running: false`、`termination_reason` → 输出总结报告

## 参数路由（第一步必须做）

**读取 `$ARGUMENTS` 后，严格按下表路由，不要混合执行：**

| 参数 | 执行路径 |
|---|---|
| 无参数 | Step 0 → 检查终止 → 如未终止则 Step 2 → Step 3 → Step 4 → Step 5 |
| `continue` | 跳过 Step 1，直接进入 Step 2 → Step 3 → Step 4 → Step 5 |
| `dev` | 仅执行 Step 3（开发端），不审查、不验证 |
| `review` | 仅执行 Step 2 + Step 4（审查 + 验证），不开发 |
| `status` | 仅执行 Step 0 的状态报告，不做任何修改 |
| `loop` | 等同于无参数，但 Step 5 中若未终止则自动进入下一轮（无需询问） |

## 架构：审查端先手，开发端批量

```
┌──────────────────────── 一轮迭代 (3-4 次调用) ────────────────────────┐
│                                                                      │
│  调用① 审查端：                                                      │
│    0. cargo check --all-features ← 全量编译检查，收集所有报错           │
│    1. 直接用 Read/Grep/Glob 找差距（不 spawn Agent）                   │
│    2. 对每个 OPEN 任务 Read/Grep 验证差距 ← 先确认再写                  │
│    3. 写 dev_prompt（具体到文件+行号+步骤）                             │
│    4. 直接 Write 更新 todo.json ← 自动化                               │
│                                                                      │
│  调用② 开发端：                                                       │
│    读取 todo.json 中所有 dev_prompts                                   │
│    → 相关任务合并到 1-2 个 Agent 中执行（不是每个任务一个 Agent！）      │
│    → 按 prompt 原文执行，不改、不解读                                   │
│    → 每个任务后：cargo check + nextest 回归验证 ← 新增                  │
│    → 同类问题发现后直接修，不回退                                       │
│    → 验证编译 + 回填 exec_result                                      │
│                                                                      │
│  调用③ 审查端验证：                                                    │
│    直接用 Read/Grep 检查变更（不 spawn Agent）                          │
│    → 确认 DONE / 更新 dev_prompt → 直接 Write 更新 todo.json            │
│    → just precommit                                                  │
│                                                                      │
│  可选调用④（仅复杂任务）：                                              │
│    如果任务涉及多文件大规模改动，单独 spawn Agent 处理                    │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘

        每轮结束后询问用户是否继续，最多 5-7 轮
```

## 终止条件（每轮结束后检查）

满足任一即停止自动循环：
1. **任务完成**：所有 P0 DONE，P1 >= 80% DONE
2. **质量达标**：代码与文档一致，测试全通过，Clippy 零警告
3. **轮次耗尽**：7 轮迭代完成
4. **用户手动终止**

停止后输出最终总结报告。

## todo.json 格式（v3.1 关键字段）

```json
{
  "metadata": {
    "title": "项目名优化规划",
    "version": "v3.1",
    "created": "YYYY-MM-DD",
    "updated": "YYYY-MM-DD",
    "current_round": 1,
    "round_1_note": "本轮摘要",
    "total_rounds_planned": 7
  },
  "root_cause_analysis": {
    "critical_issues": [
      {
        "id": "OPT-001",
        "severity": "P0-CRITICAL | P1-MAJOR | P2-MINOR",
        "module": "模块名",
        "symptom": "问题症状",
        "root_cause": "根因分析",
        "status": "OPEN | DONE | BLOCKED",
        "deadline": "YYYY-MM-DD",
        "dev_prompt": "具体 prompt 内容...",
        "exec_result": {
          "status": "DONE | PARTIAL | FAIL",
          "files_changed": ["src/xxx.rs"],
          "summary": "一句话总结",
          "regression_test_passed": true
        }
      }
    ]
  },
  "optimization_phases": {
    "phase_1_core_storage": {
      "name": "Phase 1: ...",
      "completed_tasks": ["FIX-001", "OPT-008"],
      "remaining_tasks": ["OPT-002", "OPT-007"]
    }
  },
  "success_criteria": { ... }
}
```

## 工作流步骤

### Step 0: 检查状态

读取 todo.json 和 `.claude/fenjue-state.json`（如果存在），报告：
- current_round、P0/P1/P2 进度
- 有多少 OPEN 任务没有 dev_prompt
- loop_running 状态

**关键：如果 fenjue-state.json 中 termination_reason 已设置且 loop_running 为 false：**
1. 输出终止原因和最近一轮的 round_N_note
2. 询问用户："上一轮已终止。是否要开启新一轮？（将重置 current_round=1、清除 termination_reason）"
3. 如果用户确认 → 执行重置（见下方），然后继续后续步骤
4. 如果用户拒绝 → 停止，不执行任何后续步骤

**重置操作（用户确认后执行）：**
Write `.claude/fenjue-state.json`：`current_round: 1`、`termination_reason: null`、`loop_running: false`、`reset_requested: true`、`last_updated: 今天日期`。

**如果所有任务都已 DONE（无 OPEN 任务）：**
输出"所有任务已完成"，询问用户是否要添加新任务。不要进入审查或开发。

不存在 todo.json → 首轮（Step 1）。存在 → 从 Step 2 继续。

### Step 1: 主上下文了解项目（仅首轮）

**不 spawn Agent，直接在当前会话中完成：**

1. 用 Read 读取 src/lib.rs、Cargo.toml
2. 用 Glob 查看 src/ 目录结构
3. 用 Grep 搜索文档声称的功能关键词
4. 输出简要报告（< 300 字）

### Step 2: 审查端 → 找差距 + 写 dev_prompts

**核心规则：直接用工具操作，不 spawn Agent。**

以下指令供审查端执行（可以直接在主上下文中执行，或用 Agent 执行）：

---

你是刁钻的 P11 级程序员兼项目经理。工作目录：`/home/hugo/codes/try-tokitai/crates/tokitai-filekv`

#### 0. Pre-flight 编译检查（必须先做！）

运行 `cargo check --all-features 2>&1`，收集所有编译报错。
按受影响的结构体/模块分组，记录每个报错位置和缺失字段。
同时收集警告：`cargo clippy --all-features 2>&1 | grep warning`

**目的**：一次性拿到所有编译问题，按 struct 分组后写出覆盖所有位置的 dev_prompt。

**如果编译通过（0 errors）且 0 warnings**：继续下一步，但记录这个事实到报告中。

#### 1. 找差距

- 用 Grep 搜索 README/CLAUDE.md/doc/ 中声称的功能
- 用 Glob 查看相关文件是否存在
- 用 Read 查看核心实现的代码

#### 2. 强制验证差距（写 dev_prompt 前必须做！）

**对每个待修复的任务，必须先 Read/Grep 确认代码里真正缺什么。**
禁止基于记忆或假设写 dev_prompt。

验证流程：
1. 读 dev_prompt 中提到的核心文件
2. 用 Grep 确认：声称的函数/方法/字段是否存在
3. 如果已实现 → 标记 DONE，不要写 dev_prompt
4. 如果确实缺失 → 基于读到的事实写 dev_prompt

#### 3. 写 dev_prompts

每个 dev_prompt 必须包含（缺一不可）：
- 任务目标（一句话）
- 涉及文件（路径 + 行号 + 当前实现描述）
- 当前问题（具体描述，基于实际读取）
- 具体步骤（1, 2, 3...）
- 验收标准（可验证）
- 注意事项（不要动什么）

#### 4. 更新 todo.json（只写本轮需要的字段）

直接 Write 更新 todo.json 中 OPEN 任务的 dev_prompt 和 status。
**不要重写整个文件**，保留已有字段不变。
更新：metadata.current_round += 1、metadata.round_N_note、已验证为 DONE 的 status。

#### 5. 输出

```
=== 第 N 轮审查报告 ===
  总任务: X (P0: Y/Z 完成)
  Pre-flight 编译结果: 0 errors, N warnings（或具体报错数）
  本轮新增 dev_prompt: N 个
  确认完成: N 个
  剩余 P0: N 个
新发现: [...]
建议优先: [ID]
```

---

### Step 3: 开发端 → 批量执行 dev_prompts

**核心规则：同模块任务合并到 1-2 个 Agent 并行。**

如果 OPEN 的 dev_prompt 数量 ≤ 3，直接在主上下文执行（不 spawn Agent）。
如果 > 3，按模块分组，最多 2 个 Agent 并行。

以下是指令模板（传给执行 Agent 或直接执行）：

---

你是 P11 级开发者。工作目录：`/home/hugo/codes/try-tokitai/crates/tokitai-filekv`

**你的任务**：读取 todo.json，按优先级（P0 > P1 > P2）执行所有有 dev_prompt 的 OPEN 任务。

**执行规则**：
1. 不自行解读 dev_prompt — 写了什么就做什么
2. 合并执行 — 相关任务合并处理：
   - 同模块的任务 → 一个 Agent 批量执行
   - 不同模块的任务 → 最多 2 个 Agent 并行
3. 直接修改代码 — 不要 spawn Agent 来读代码，你自己读、自己改
4. 每个任务完成后必须做回归验证

**合并示例**：
- OPT-001(改cache/block_cache.rs) + OPT-002(改cache/budget.rs) → Agent A 批量执行
- OPT-003(改bloom/adaptive.rs) → Agent B 执行

**回归验证（每个任务后必须做！）**：
1. `cargo check --all-features` — 确认编译通过
2. `cargo nextest run --all-features '<相关关键词>'` — 运行相关测试
3. 如果有新报错（同类 struct 的其他位置缺失字段），直接修复，不要再回审查端
4. 全部修完后 `just precommit` 确保 0 warnings

**回填 exec_result**：为每个任务填写 status + files_changed + summary + regression_test_passed。

**输出格式**：
```
=== 第 N 轮开发报告 ===
| 任务 ID | 状态 | 改了哪些文件 | 回归测试通过 |
```

---

**开发端完成后**，Write 更新 todo.json 中对应任务的 exec_result 字段。

### Step 4: 审查端 → 验证变更

**核心规则：直接用工具操作，不 spawn Agent。**

以下指令供审查端执行：

---

你是刁钻的 P11 级程序员兼项目经理。

#### 1. 验证每个 DONE 任务

- 用 Read 查看 dev_prompt 中提到的文件，确认变更解决了问题
- 检查是否引入新问题（用 Grep 搜索被删的函数/字段是否还被引用）

#### 2. 运行验证

```bash
cd /home/hugo/codes/try-tokitai/crates/tokitai-filekv
cargo check --all-features
```

#### 3. 更新 todo.json

- 确认完成 → DONE
- PARTIAL/FAIL → 更新 dev_prompt（更具体，基于实际看到的失败原因）
- 新发现 → 添加新任务 + dev_prompt
- 更新 metadata

#### 4. 输出报告

```
=== 第 N 轮验证报告 ===
| 任务 ID | 开发端状态 | 审查端确认 | 质量评价 |
```

---

### Step 5: 循环控制（每轮结束后检查）

1. 读取 `.claude/fenjue-state.json`
2. 更新状态：`current_round += 1`、`last_round_status = "completed"`、`last_updated = 当前日期`
3. 检查终止条件：
   - 读取 todo.json，统计 P0/P1 完成比例
   - 运行 `just precommit`，检查是否 0 errors + 0 warnings
   - 如果 `current_round >= total_rounds_planned` → 终止
4. 判断：
   - **未终止** → Write 更新状态 → 输出报告
     - 如果是 `loop` 模式 → 直接回到 Step 2
     - 否则 → 询问用户是否继续下一轮
   - **已终止** → Write `loop_running: false`、`termination_reason` → 输出最终报告

最终报告格式：
```
=== 焚诀 v3.1 最终报告 ===
终止原因: [任务完成|质量达标|轮次耗尽|手动终止]
总轮次: N / 7
P0: Y/Z 完成 | P1: Y/Z 完成 | P2: Y/Z 完成
最终状态: 编译通过/测试通过/Clippy 警告数
剩余待办: [未完成的 OPEN 任务列表]
```

## 上下文管理

- context > 10% → `/compress`
- 压缩前确认 todo.json 和 fenjue-state.json 已保存

## 命令速查

| 命令 | 效果 |
|---|---|
| `/fenjue` | 完整一轮，完成后询问是否继续 |
| `/fenjue loop` | 自动循环多轮，直到终止（不询问） |
| `/fenjue continue` | 跳过了解项目，从审查开始，单轮 |
| `/fenjue dev` | 仅开发端批量执行 |
| `/fenjue review` | 仅审查端写 prompts + 验证 |
| `/fenjue status` | 仅读取报告（不调用 Agent） |
