---
name: cs-experiment-reproduction-workflow
description: Computer science experiment reproduction workflow for environment locking, artifact tracing, rerun verification, and discrepancy diagnosis. Use when the task is to reproduce a paper, validate a reported result, or make an existing experiment pipeline trustworthy.
---

# CS Experiment Reproduction Workflow

## 触发条件
- 需要复现论文结果、验证现有实验、排查复现实验偏差。
- 需要把零散脚本整理成可重复执行的 workflow。
- 需要为审稿、 rebuttal 或 release 准备可验证产物。

## 标准化流程
1. 明确复现目标：复现哪张表、哪幅图、哪个结论，以及允许误差范围。
2. 锁定环境：代码版本、依赖版本、硬件、驱动、随机种子和外部资源。
3. 为每次运行记录配置、输入工件、输出工件和日志路径。
4. 先做最小 smoke test，再做主实验，再做多次重复运行。
5. 若结果不一致，按数据、代码、环境、评测脚本、随机性五类排差。
6. 输出时分别标记 fully reproduced、partially reproduced、not reproduced 与原因。

## 反模式
- 直接追求最终分数，不先验证数据管线和评测脚本。
- 复现实验时静默更改超参数、预处理或早停策略。
- 只汇报最接近原论文的一次结果。
- 结果不一致时只写“环境不同”，不做定位。

## 验证方法
- 检查每个结果文件是否能追溯到配置和日志。
- 复核 reproduction report 是否区分完全复现和趋势复现。
- 抽查至少一项偏差定位记录，确认不是口头解释。
- 审核是否显式披露 skipped steps、missing artifacts 或不可得外部依赖。
