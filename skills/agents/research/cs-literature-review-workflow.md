---
name: cs-literature-review-workflow
description: Structured computer science literature review workflow for official paper-source search, paper triage, evidence synthesis, and gap mapping. Use when the task is to survey prior work, build related work sections, compare methods, or identify research gaps before experimentation.
---

# CS Literature Review Workflow

## 触发条件
- 需要做 related work、survey、gap analysis 或研究方向收敛。
- 用户给出问题很宽，需要先从文献里缩小研究面。
- 需要为后续实验、假设或论文写作建立证据底座。

## 标准化流程
1. 把主题拆成检索关键词、同义表述、任务名、数据集名和方法名。
2. 仅使用官方论文 API 搜索：Semantic Scholar、OpenAlex、arXiv、Crossref、OpenReview。
3. 按时间、方法路线、实验设置和结论类型做初筛。
4. 为每篇候选文献抽取结构化字段：问题、方法、数据、指标、主要结果、局限性。
5. 形成方法对照表与时间线，找出真正未解决的问题而不是简单缺少更高分数。
6. 输出时区分“文献事实”“综合推断”“待验证假设”三层。

## 反模式
- 用搜索引擎摘要替代论文元数据和正文证据。
- 只看最新论文，忽视奠基方法与评测协议来源。
- 把不同任务或不同 split 的数字直接横向比较。
- 把“没人做过”建立在不完整检索之上。

## 验证方法
- 检查引用清单是否全部来自允许的官方论文源。
- 抽查综述表格中的数值是否能追溯到论文原文或元数据。
- 审核 gap 分析是否基于方法局限或证据缺口，而不是空泛结论。
- 复核 related work 段落是否明确区分事实与推断。
