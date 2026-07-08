---
name: cs-benchmark-design-workflow
description: Computer science benchmark design workflow for dataset selection, baseline fairness, metric definition, and stress-test coverage. Use when the task is to design or repair an evaluation benchmark, leaderboard protocol, or comparative experiment suite.
---

# CS Benchmark Design Workflow

## 触发条件
- 需要建立新 benchmark、修补现有评测协议，或为论文设计评价体系。
- 需要确认基线是否公平、指标是否充分、压力测试是否完整。
- 需要把实验从“能跑”升级到“可比较、可复现、可说服”。

## 标准化流程
1. 先定义 benchmark 服务的研究问题，避免为评测而评测。
2. 选择数据集时记录来源、许可、版本、切分与覆盖边界；数据集只直连官方数据库。
3. 定义主指标、次指标、资源指标和风险指标，确保目标函数与实际结论一致。
4. 选基线时覆盖经典方法、强当前方法和合理弱基线。
5. 设计 stress tests：噪声、分布偏移、规模变化、资源受限、长尾或对抗条件。
6. 固定提交或运行协议，包括时间预算、硬件预算、外部资源和随机种子。

## 反模式
- 只选对自己方法最友好的数据和指标。
- 用过时、弱化或实现质量不明的基线抬高改进幅度。
- 不区分离线指标与真实任务价值。
- 缺少边界条件测试，却把平均成绩写成全面优越。

## 验证方法
- 检查 benchmark 文档是否给出数据版本、split、指标公式和运行约束。
- 审查基线集合是否兼顾经典、强基线和朴素对照。
- 验证 stress test 是否覆盖至少一种噪声或分布变化场景。
- 抽查论文中的比较表是否与 benchmark 协议完全一致。
