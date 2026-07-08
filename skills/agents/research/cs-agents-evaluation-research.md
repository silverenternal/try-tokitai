---
name: cs-agents-evaluation-research
description: Agent evaluation research skill for tool-using LLM workflows, trajectory analysis, verifier design, and reproducible task-suite benchmarking. Use when the task involves autonomous agents, coding agents, tool orchestration, verifier-guided repair, or multi-step agent benchmark studies.
---

# CS Agents Evaluation Research

## 触发条件
- 研究对象是 tool-using agent、coding agent、多代理协作、verifier-guided repair 或任务规划。
- 需要设计任务集、轨迹记录、成功标准、工具预算或 step-level 评测。
- 需要把 agent 行为过程而不只是最终结果写成可审计证据。

## 标准化流程
1. 明确 agent 的行动空间、工具集、预算约束、停止条件和成功定义。
2. 将 benchmark 拆成任务类型、难度层级、依赖前置和可验证产物。
3. 同时记录终局指标和过程指标，例如 pass rate、修复轮次、工具调用数、时延和失败模式。
4. 为 verifier、reviewer 或 repair 环节定义独立输入输出，避免和主代理逻辑混淆。
5. 采样典型成功与失败轨迹，提炼规划错误、工具误用、证据不足和过度操作等模式。
6. 写作时让每个关于 agent 能力的 claim 同时对应结果表和轨迹证据。

## 反模式
- 只看最终成功率，不分析工具预算、步骤稳定性和失败类型。
- 任务不可验证，却把主观印象写成 agent 能力提升。
- 更换工具权限、上下文预算或 verifier 规则后仍声称同设置对比。
- 只展示最漂亮的一条轨迹，忽略整体行为分布。

## 验证方法
- 检查每个任务是否有清晰的成功判定和可验证产物。
- 复核结果是否同时包含终局指标和过程指标。
- 抽查轨迹样例是否真实来自运行记录而非手工润色。
- 审核 verifier 和 repair 结论是否有独立证据，而不是主代理自证。
