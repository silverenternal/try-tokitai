---
name: cs-vision-research
description: Computer vision research skill for dataset protocol control, architecture comparison, visual error analysis, and reproducible image benchmark reporting. Use when the task involves classification, detection, segmentation, multimodal vision-language evaluation, or image robustness studies.
---

# CS Vision Research

## 触发条件
- 任务涉及图像分类、检测、分割、视觉语言、多视图或视觉鲁棒性。
- 需要比较 backbone、augmentation、loss、pretraining 或推理策略。
- 需要把可视化结果与定量指标联合呈现。

## 标准化流程
1. 确定任务协议、输入分辨率、预处理和评价脚本。
2. 梳理代表性方法与公开基准，明确是否存在官方 leaderboard 或官方 split。
3. 设计主实验与消融实验，控制训练轮次、数据增强、预训练来源和推理后处理。
4. 记录视觉任务特有证据：混淆类别、检测失败图、分割边界错误、跨域退化。
5. 对效率与资源做并列报告，包括吞吐、延迟、显存和参数量。
6. 将视觉样例与表格中的 case id 对齐，保证图文证据可追溯。

## 反模式
- 换数据增强、输入分辨率或预训练数据后仍声称“同设置”。
- 只放好看的 qualitative figure，不给对应 quantitative 结果。
- 只比较参数量，不比较推理延迟和训练成本。
- 把公开可见的 benchmark 调参结果当作泛化能力。

## 验证方法
- 抽查 figure 中样例是否能追溯到实验输出文件和样本 id。
- 检查表格是否标注训练分辨率、预训练来源和测试协议。
- 验证是否至少包含一种跨噪声、跨域或扰动鲁棒性测试。
- 核对效率指标是否与同一硬件和批大小绑定。
