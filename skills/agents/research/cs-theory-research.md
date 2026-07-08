---
name: cs-theory-research
description: Theory and formal-methods research skill for definitions, theorem conditions, proof structure, algorithmic bounds, and counterexample-driven analysis. Use when the task involves algorithms, complexity, formal verification, proofs, lower bounds, approximation guarantees, or theorem-oriented CS papers.
---

# CS Theory Research

## 触发条件
- 任务以定义、定理、证明、复杂度、近似比、不变量、归约或形式化性质为核心。
- 需要从问题陈述推到 theorem statement、proof sketch 或 counterexample。
- 需要把直觉论证收敛成可以审阅的正式结构。

## 标准化流程
1. 先固定对象、符号、假设、输入规模和允许操作。
2. 把研究目标写成正式命题，明确上界、下界、可判定性或正确性性质。
3. 选择证明路线，例如构造法、归纳、势能、归约、对偶或反证，并先写骨架。
4. 对算法结论分别处理正确性、复杂度、边界条件和失败情形。
5. 对形式化性质给出不变量、状态转移和反例思路，而不是只给直觉。
6. 写作时区分已证明命题、猜想、启发式解释和开放问题。

## 反模式
- 省略假设条件却直接给出定理级结论。
- 把经验观察或小规模枚举结果写成一般性证明。
- 只写 proof sketch，却没有说明关键引理和依赖关系。
- 忽略边界条件、退化输入或反例。

## 验证方法
- 检查每个定理或命题是否有清晰前提、结论和符号定义。
- 审核证明结构是否覆盖正确性、复杂度和边界条件。
- 抽查是否存在潜在反例或遗漏假设，并在文中显式处理。
- 复核正文是否明确区分 theorem、conjecture、intuition 和 limitation。
