---
name: fenjue-status
description: "焚诀 v3.1 状态查看 - 读取 todo.json + fenjue-state.json 报告状态和循环情况，不调用 Agent，0 API 调用"
user-invocable: true
---

# 焚诀 v3.1 - 状态查看

工作目录：`/home/hugo/codes/try-tokitai/crates/tokitai-filekv`

## 流程

**不调用 Agent，纯读取，0 额外费用。**

### 1. 读取 `.claude/fenjue-state.json`

提取：current_round、loop_running、termination_reason、last_updated

### 2. 读取 `todo.json`

提取：metadata、所有 critical_issues 的 id/severity/status/dev_prompt

### 3. 计算统计

- 总任务数、P0/P1/P2 各自 DONE/OPEN/BLOCKED 数
- OPEN 任务中有 dev_prompt 的数量 / 缺 dev_prompt 的 ID 列表
- 如果有 optimization_phases，计算各 Phase 完成百分比

### 4. 输出

```
=== 焚诀 v3.1 状态 ===
版本: vX.X | 轮次: N / 计划 7
循环: 运行中 / 已停止
停止原因: [...]（仅当已停止时显示）
更新: YYYY-MM-DD

任务: X (P0:Y/Z | P1:Y/Z | P2:Y/Z)
dev_prompt覆盖: N/M OPEN (缺: [ID列表])

最近一轮: [round_N_note]

未完成 P0:
| ID | 描述 | prompt | 截止 |
...

Phase 进度:
- Phase 1: N% | Phase 2: N% | ...
```
