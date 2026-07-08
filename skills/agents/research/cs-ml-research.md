---
name: cs-ml-research
description: Computer science machine learning research skill for model benchmarking, ablation design, robustness analysis, and empirical claim validation. Use when the task involves supervised learning, representation learning, benchmark comparison, error analysis, or reproducible ML experiment planning and reporting.
---

# CS ML Research

## 触发条件
- 研究主题涉及监督学习、表征学习、鲁棒性、泛化、噪声标签、迁移学习或模型压缩。
- 需要设计基线、消融实验、超参数策略、统计检验或误差分析。
- 需要把实验结果组织为论文式方法、实验、讨论与局限性。

## 标准化流程
1. 重述任务，固定预测目标、输入模态、约束条件与评价指标。
2. 用官方论文 API 建立文献基线，抽取数据集、模型族、训练协议与最强可比结果。
3. 明确研究假设，拆成可检验变量，例如数据扰动、模型结构、训练策略或推理策略。
4. 设计实验矩阵：主实验、基线对照、消融、稳健性、效率与失败案例。
5. 固定复现实验要素：随机种子、数据切分、预处理、指标定义、硬件与预算。
6. 运行实验并记录结构化结果，至少输出均值、方差或置信区间。
7. 将结论回链到具体表格、图和数值，避免只有定性描述没有证据。

## 反模式
- 用单次最好结果替代多次运行统计。
- 把不公平的训练预算比较写成方法优势。
- 只写“显著提升”而不提供增益幅度、波动范围和对比对象。
- 引用非官方论文源或把二手博客当作证据。
- 先写结论再倒推实验设计。

## 验证方法
- 检查每个核心 claim 是否都能映射到表格、图或日志中的具体数值。
- 抽查基线是否复现同一数据切分、同一指标和可比训练预算。
- 验证是否包含至少一类稳健性分析与一类失败案例分析。
- 核对论文引用是否全部来自 Semantic Scholar、OpenAlex、arXiv、Crossref 或 OpenReview。
