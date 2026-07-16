---
name: cs-security-research
description: Computer security research skill for threat modeling, attack-defense evaluation, exploitability analysis, and responsible empirical reporting. Use when the task involves vulnerability analysis, detection systems, privacy leakage, adversarial robustness, or security measurement studies.
---

# CS Security Research

## 触发条件
- 研究包含漏洞、攻击、防御、隐私泄漏、对抗样本、恶意样本检测或安全测量。
- 需要建立威胁模型、攻击面、攻击能力与防御边界。
- 需要在论文中平衡技术严谨性与负责任披露。

## 标准化流程
1. 先写清威胁模型：攻击者能力、目标资产、前置条件和成功定义。
2. 明确实验对象与防御边界，区分 prototype、production 和 sandbox。
3. 设计攻击和防御对比时，记录攻击成本、成功率、误报漏报和防御开销。
4. 对测量类研究，保证样本来源、采样偏差与标签质量可追溯。
5. 写作时将“可行性”“可扩展性”“现实威胁”分开陈述，避免夸大。
6. 对可能敏感的 exploit 细节做约束化呈现，保留复核所需证据但避免直接武器化。

## 反模式
- 不定义威胁模型就宣称系统“不安全”或“安全”。
- 只展示命中的攻击样例，不报告失败率与边界条件。
- 将实验室内成功案例写成现实世界普遍风险。
- 在没有伦理与披露说明时输出可直接滥用的攻击细节。

## 验证方法
- 检查每个安全结论是否绑定威胁模型前提。
- 抽查攻击成功率、误报漏报和防御开销是否同时汇报。
- 验证数据或样本来源是否合法、可追溯且说明偏差。
- 审核写作中是否清楚区分演示级证据与生产级风险。
