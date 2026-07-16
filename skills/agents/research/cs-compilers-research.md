---
name: cs-compilers-research
description: Compiler and programming-language research skill for IR transformation analysis, optimization correctness, compile-time tradeoff evaluation, and benchmark-driven compiler experimentation. Use when the task involves compilation pipelines, code generation, static analysis, optimization passes, or PL performance studies.
---

# CS Compilers Research

## 触发条件
- 研究对象是编译器、程序分析、代码生成、IR 优化、类型系统或运行时协同优化。
- 需要比较优化 pass、静态分析精度、编译时间、执行时间或代码大小。
- 需要在正确性、性能和编译成本之间做论文级权衡。

## 标准化流程
1. 先明确研究层级：前端、IR、中端优化、后端代码生成、运行时或静态分析。
2. 固定 benchmark 集、编译选项、目标架构、输入规模和执行环境。
3. 对每个优化或分析分别报告正确性、编译时间、运行时间、代码大小和稳定性。
4. 把性能收益映射到 IR 变换、寄存器分配、内存访问模式或分支行为。
5. 对 correctness-sensitive 结论增加差分测试、回归测试或形式化约束说明。
6. 写作时区分“优化命中场景”“退化场景”“未命中场景”。

## 反模式
- 只选少量对自己优化有利的 benchmark。
- 只报运行时间，不报编译时间、代码大小或正确性风险。
- 将 backend 或硬件差异误写成算法优化收益。
- 不说明 benchmark、编译选项和目标架构。

## 验证方法
- 检查 benchmark、编译选项、目标架构和输入规模是否完整披露。
- 审核是否同时报告了运行性能和编译成本。
- 抽查正确性验证是否来自测试、差分比较或形式化约束。
- 核对性能解释是否与 IR/pass 级证据一致，而不是纯猜测。
