---
name: cs-nlp-research
description: Natural language processing research skill for task framing, dataset curation, prompting or finetuning evaluation, and grounded claim writing. Use when the task involves language understanding, generation, retrieval, alignment, multilingual evaluation, or NLP benchmark analysis.
---

# CS NLP Research

## 触发条件
- 研究对象是分类、抽取、问答、摘要、翻译、检索增强生成或 agent language workflow。
- 需要比较 prompt、finetune、retrieval、tool use 或 evaluation protocol。
- 需要把文字样例、自动指标与人工分析统一到论文叙述里。

## 标准化流程
1. 明确任务单位：样本粒度、输入输出格式、约束和错误代价。
2. 用官方论文 API 拉取代表性方法，分离 encoder、decoder、retrieval、instruction tuning 等路线。
3. 确定自动指标与人工判别标准，避免只依赖单一分数。
4. 设计实验时区分 closed-book、retrieval、tool-augmented、finetuned 等设置。
5. 采集定量结果的同时保留代表性样例，覆盖成功、失败、边界和幻觉场景。
6. 写作时让每个语言层面的 claim 绑定具体例句、错误类型和数值变化。
7. 对数据污染、提示泄漏、模板过拟合和评测偏差做显式说明。

## 反模式
- 只报 BLEU、ROUGE 或 accuracy，不解释这些指标能否代表真实质量。
- 混用不同 prompt 模板、上下文窗口或检索语料却宣称公平比较。
- 用零散案例支持总体结论，或反过来只给总体均值不做错误分析。
- 忽略标注噪声、数据泄漏和 benchmark contamination 风险。

## 验证方法
- 检查每段核心分析是否同时具备数值证据和文本样例。
- 复核人工评测协议是否包含维度定义、样本量和一致性说明。
- 核对结论是否区分任务设置，避免把 prompt gain 写成 model gain。
- 抽查失败样例是否真实来自结果文件而非事后编造。
