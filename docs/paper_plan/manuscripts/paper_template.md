# 论文写作模板 (Markdown 版本)

> **论文标题**: [Your Paper Title]
> **目标会议**: AAAI 2027 / ACL 2027 / EMNLP 2027
> **提交日期**: [Date]

---

## 📋 写作检查清单

### 内容完整性
- [ ] Abstract (150-200 词)
- [ ] Introduction (1000-1500 词)
- [ ] Related Work (800-1000 词)
- [ ] Background (400-500 词)
- [ ] Method (2000-2500 词)
- [ ] Implementation (800-1000 词)
- [ ] Experiments (2000-2500 词)
- [ ] Discussion (400-600 词)
- [ ] Conclusion (200-300 词)
- [ ] References (40-60 篇)

### 图表
- [ ] 系统架构图 (Figure 1)
- [ ] 核心算法流程图 (Figure 2)
- [ ] 实验结果对比图 (Figure 3)
- [ ] 消融实验图 (Figure 4)
- [ ] 学习曲线 (Figure 5)

### 格式规范
- [ ] 符合目标会议格式要求
- [ ] 引用格式统一
- [ ] 图表标题规范
- [ ] 数学符号一致
- [ ] 页数限制检查

---

## Abstract (150-200 词)

**结构**: 
1. **问题** (1句): 现有方法的局限
2. **方法** (2-3句): 我们的解决方案
3. **结果** (2句): 关键实验数据
4. **意义** (1句): 研究贡献

**模板**:
```
[问题陈述]. We present [方法名称], a [方法描述]. 
Our approach comprises [核心组件]: (1) [组件1], (2) [组件2], and (3) [组件3].
[实验设置] demonstrates that our system [关键结果1] while [关键结果2].
This work represents [研究意义].
```

---

## 1. Introduction (1000-1500 词)

### 1.1 Motivation (200-300 词)
- 研究领域的背景和重要性
- 实际应用场景
- 发展趋势

### 1.2 Problem Statement (300-400 词)
- 现有方法的具体局限
- 形式化问题定义
- 挑战分析

**问题定义模板**:
```
问题: [具体问题描述]
输入: [输入定义]
输出: [输出定义]
约束: [约束条件]
```

### 1.3 Our Contributions (400-500 词)
- 主要贡献 (3-4点)
- 次要贡献 (1-2点)
- 工程贡献 (可选)

**贡献陈述模板**:
```
Our key contributions are:

1. **[贡献名称]**: [详细描述]. This is the first work to [创新点].

2. **[贡献名称]**: [详细描述]. We propose [方法], which [效果].

3. **[贡献名称]**: [详细描述]. Experimental results show [数据].
```

### 1.4 Summary of Results (200 词)
- 关键实验数据
- 与baseline的对比
- 消融实验亮点

---

## 2. Related Work (800-1000 词)

### 2.1 [相关领域1] (250-300 词)
- 3-5篇关键工作的综述
- 每篇工作的核心思想
- 与我们的方法的对比

**对比表格模板**:
```markdown
| Method | [维度1] | [维度2] | [维度3] |
|--------|---------|---------|---------|
| Method A | ✓ | ✗ | ✓ |
| Method B | ✗ | ✓ | ✓ |
| **Ours** | ✓ | ✓ | ✓ |
```

### 2.2 [相关领域2] (250-300 词)
- 类似结构

### 2.3 [相关领域3] (200-250 词)
- 类似结构

### 2.4 Summary (100 词)
- 现有工作的不足
- 我们的定位

---

## 3. Background (400-500 词)

### 3.1 [基础概念1]
- 定义和解释
- 数学符号

### 3.2 [基础概念2]
- 类似结构

---

## 4. Method (2000-2500 词)

### 4.1 Overview (300 词)
- 方法整体架构
- 核心组件介绍
- 数据流描述

### 4.2 [核心组件1] (600-700 词)
- 问题定义
- 方法设计
- 算法描述
- 理论分析

**算法描述模板**:
```markdown
**输入**: [输入描述]
**输出**: [输出描述]

1. [步骤1描述]
2. [步骤2描述]
3. [步骤3描述]
...
```

### 4.3 [核心组件2] (600-700 词)
- 类似结构

### 4.4 [核心组件3] (400-500 词)
- 类似结构

### 4.5 Summary (100 词)

---

## 5. Implementation (800-1000 词)

### 5.1 System Architecture (300 词)
- 实现架构图
- 模块说明
- 技术栈

### 5.2 Key Implementation Details (400 词)
- 关键实现技巧
- 优化策略
- 工程挑战

### 5.3 Complexity Analysis (200 词)
- 时间复杂度
- 空间复杂度

---

## 6. Experiments (2000-2500 词)

### 6.1 Experimental Setup (300-400 词)
- 数据集
- Baseline方法
- 评估指标
- 实验环境

**实验设置表格模板**:
```markdown
| Setting | Value |
|---------|-------|
| Dataset | [数据集名称] |
| # Samples | [样本数量] |
| Baselines | [baseline方法] |
| Metrics | [评估指标] |
```

### 6.2 Main Results (500-600 词)
- 主要结果表格
- 关键发现分析
- 统计显著性检验

**结果表格模板**:
```markdown
| Metric | Baseline | Ours | Improvement |
|--------|----------|------|-------------|
| Metric 1 | X% | Y% | +Z% |
| Metric 2 | X | Y | -Z% |
```

### 6.3 Ablation Study (400-500 词)
- 消融实验设计
- 各组件贡献分析
- 可视化结果

### 6.4 Case Studies (300-400 词)
- 成功案例分析
- 失败案例分析
- 定性分析

### 6.5 Cost Analysis (200 词)
- 计算成本
- 时间成本
- 资源消耗

---

## 7. Discussion (400-600 词)

### 7.1 Limitations (200 词)
- 方法局限
- 实验局限
- 适用范围

### 7.2 Future Work (200 词)
- 短期方向
- 长期愿景

### 7.3 Ethical Considerations (100-200 词)
- 伦理考量
- 潜在风险
- 缓解措施

---

## 8. Conclusion (200-300 词)

- 总结主要贡献
- 强调关键结果
- 展望未来

**结论模板**:
```
We presented [方法名称], a [方法描述]. Our approach [核心创新].
Experimental results demonstrate [关键结果].
This work [研究意义和未来方向].
```

---

## References

**格式**: ACL/AAAI 标准格式

**数量**: 40-60 篇

**分类**:
- 工具学习 (5-8篇)
- AI Agent系统 (5-8篇)
- 自进化系统 (3-5篇)
- Prompt Engineering (3-5篇)
- 其他相关 (20-30篇)

---

## Appendix (可选)

### A. Prompt Templates
- 完整Prompt模板

### B. Additional Experimental Results
- 额外实验结果

### C. Complete Tool List
- 工具完整列表

### D. Code Repository
- 代码仓库链接

---

## 📝 写作技巧

### 1. 段落结构
- 每段一个核心观点
- 首句点明主旨
- 支持论据2-3句
- 过渡句连接下段

### 2. 句子写作
- 避免过长句子 (>30词)
- 主动语态优先
- 具体数字替代模糊描述

### 3. 图表规范
- 图表自包含 (无需正文也能理解)
- 标题清晰描述内容
- 坐标轴标注完整

### 4. 引用规范
- 关键观点必须引用
- 避免过度引用
- 平衡经典文献和最新工作

---

**最后更新**: 2026-03-27
**维护者**: AI Assistant
