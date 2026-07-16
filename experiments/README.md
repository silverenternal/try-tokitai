# 实验框架文档

> **项目**: Self-Evolving Tool Ecosystem for AI Agents  
> **创建日期**: 2026-03-27  
> **状态**: 实验准备中

---

## 📁 目录结构

```
experiments/
├── README.md                     # 本文件
├── paper_a/                      # Paper A: Parallel Context Architecture
│   ├── benchmark_tasks.json      # 24 个基准测试任务
│   ├── data/                     # 原始实验数据
│   ├── logs/                     # 实验日志
│   ├── analysis/                 # 分析结果
│   └── figures/                  # 生成的图表
└── paper_b/                      # Paper B: Self-Evolving Tool Ecosystem
    ├── data/                     # 原始实验数据
    ├── logs/                     # 进化日志
    ├── analysis/                 # 分析结果
    └── figures/                  # 生成的图表
```

---

## 📊 Paper A: Parallel Context Architecture 实验

### 实验目标

验证 Parallel Context Architecture 在复杂任务上的有效性。

### 实验设计

**对比实验**:
- **Control Group**: 线性上下文系统（无分支能力）
- **Treatment Group**: Parallel Context Architecture（完整分支能力）

**设计类型**: Crossover design（交叉设计）
- 每位参与者使用两种系统
- 一半参与者先用线性系统，另一半先用平行系统
- 30 分钟洗脱期（washout period）

### 基准测试任务

**24 个任务，4 个类别**:

| 类别 | 任务数 | 说明 | 难度分布 |
|------|--------|------|----------|
| **Code Refactoring** | 6 | 代码重构探索 | 3 中 + 3 难 |
| **Debugging** | 6 | 多假设调试 | 2 中 + 4 难 |
| **Creative Writing** | 6 | 创意写作探索 | 5 中 + 1 难 |
| **Research** | 6 | 研究性设计比较 | 3 中 + 3 难 |

**任务详情**: 参见 `paper_a/benchmark_tasks.json`

### 评估指标

| 指标 | 说明 | 计算方法 | 目标 |
|------|------|----------|------|
| **任务成功率** | 成功完成的任务比例 | completed/total | +40% 提升 |
| **探索深度** | 每任务平均探索分支数 | branches/tasks | 2.8+ |
| **错误恢复率** | 从失败中恢复的比例 | recovered/failures | 80%+ |
| **时间效率** | 实际时间 vs 估计时间 | estimated/actual | 1.0+ |
| **用户满意度** | 1-5 分评分 | mean(scores) | 4.5+ |

### 参与者招募

**目标**: N=12 开发者

**要求**:
- 有 Rust 编程经验
- 熟悉 Git 版本控制
- 有 AI Agent 使用经验（加分）

**激励**:
- 礼品卡（$50）
- 论文致谢
- 早期访问新工具

### 实验流程

#### 准备阶段（15 分钟）

1. 签署知情同意书
2. 填写背景调查问卷
3. 系统教程（15 分钟）
   - 线性系统组：基本操作教学
   - 平行系统组：分支/合并操作教学

#### 实验阶段（90 分钟）

1. **Condition 1** (45 分钟)
   - 完成 6 个任务（每个类别 1 个）
   - 使用系统 A（线性或平行）
   - 记录操作日志

2. **洗脱期** (30 分钟)
   - 休息
   - 填写初步反馈

3. **Condition 2** (45 分钟)
   - 完成另外 6 个任务
   - 使用系统 B（另一种系统）
   - 记录操作日志

#### 总结阶段（15 分钟）

1. 填写满意度问卷
2. 半结构化访谈（可选）
3. 致谢和报酬

### 数据收集

**自动收集**:
- 分支操作日志（fork/checkout/merge/abort）
- 任务完成状态
- 时间戳
- 系统性能指标

**手动收集**:
- 任务完成判定（人工审核）
- 满意度评分
- 定性反馈

### 数据分析

**统计方法**:
- 配对 t 检验（paired t-test）比较两种条件
- 效应量计算（Cohen's d）
- 显著性水平：α = 0.05

**预期结果**:
- 任务成功率：53% → 75% (+42%, p < 0.001)
- 探索深度：1.2 → 2.8 (+133%)
- 错误恢复率：45% → 80% (+78%)
- 用户满意度：4.6/5

---

## 📊 Paper B: Self-Evolving Tool Ecosystem 实验

### 实验目标

验证 HybridGapDetector 和 Prompt Engineering 自进化系统的有效性。

### 实验设计

**对比实验**:

| 组别 | 说明 |
|------|------|
| **Control** | 原始 tokitai（无自进化） |
| **Ours-Full** | 完整 Prompt Engineering 系统 |
| **Ours-Single** | 单 LLM（无多智能体协商） |
| **Ours-NoCoT** | 移除 Chain-of-Thought |
| **Ours-NoFix** | 移除自修正循环 |

### 30 天进化实验

**实验设置**:
- 运行 30 天自主进化
- 每天反思周期：24 小时
- 记录所有进化决策

**数据收集**:
- 每日工具库状态
- 缺口检测日志
- 优化决策记录
- API 调用成本

**预期指标变化**:

| 指标 | Day 1 | Day 15 | Day 30 | 变化 |
|------|-------|--------|--------|------|
| 工具总数 | 63 | 68 | 75 | +12 |
| 工具失败率 | 25% | 18% | 12% | -52% |
| 任务完成率 | 65% | 72% | 80% | +23% |

### 成本效益分析

**测量内容**:
- API 调用次数
- 每次调用成本
- 总成本

**预期结果**:

| 方法 | API 成本/月 | 检测延迟 | 准确率 |
|------|-------------|----------|--------|
| 纯统计 | $0 | <100ms | 60% |
| 纯 Prompt | $50-150 | 5-30s | 75% |
| **Hybrid** | **$2.25** | **1-5s** | **72%** |

### 准确率评估

**方法**:
- 人工标注 100+ 缺口
- 计算 Precision/Recall/F1
- 对比基线方法

**预期结果**:

| 方法 | Precision | Recall | F1 |
|------|-----------|--------|-----|
| 纯统计 | 0.55 | 0.65 | 0.60 |
| 纯 Prompt | 0.72 | 0.78 | 0.75 |
| **Hybrid** | **0.70** | **0.74** | **0.72** |

---

## 🛠️ 实验工具

### 日志系统

**Rust 实现** (`logger.rs`):

```rust
pub struct ExperimentLogger {
    log_dir: PathBuf,
    experiment_id: String,
}

impl ExperimentLogger {
    pub fn log_operation(&self, operation: &str, details: &Json) -> Result<()>;
    pub fn log_task_start(&self, task_id: &str, condition: &str) -> Result<()>;
    pub fn log_task_complete(&self, task_id: &str, success: bool) -> Result<()>;
    pub fn log_branch_operation(&self, op: &BranchOp) -> Result<()>;
}
```

### 评估脚本

**Python 实现** (`analyze_results.py`):

```python
#!/usr/bin/env python3
"""
Analyze experiment results for Paper A and Paper B.
"""

import json
import pandas as pd
import matplotlib.pyplot as plt
from scipy import stats

def analyze_paper_a():
    # Load data
    # Calculate metrics
    # Generate figures
    pass

def analyze_paper_b():
    # Load evolution logs
    # Calculate cost-effectiveness
    # Generate figures
    pass

if __name__ == "__main__":
    analyze_paper_a()
    analyze_paper_b()
```

### 性能基准测试

**Rust 实现** (`benchmarks.rs`):

```rust
#[cfg(test)]
mod benchmarks {
    #[bench]
    fn bench_fork_latency(b: &mut test::Bencher) {
        // Measure fork operation latency
    }

    #[bench]
    fn bench_merge_latency(b: &mut test::Bencher) {
        // Measure merge operation latency
    }

    #[bench]
    fn bench_checkout_latency(b: &mut test::Bencher) {
        // Measure checkout operation latency
    }
}
```

---

## 📅 实验时间表

### Paper A 实验

| 日期 | 任务 | 状态 |
|------|------|------|
| 2026-03-27 | 基准任务定义 | ✅ 完成 |
| 2026-04-07 | 日志系统实现 | ⏳ 待开始 |
| 2026-04-14 | 评估脚本实现 | ⏳ 待开始 |
| 2026-04-21 | 预实验 | ⏳ 待开始 |
| 2026-04-30 | 性能基准测试 | ⏳ 待开始 |
| 2026-05-15 | 用户招募完成 | ⏳ 待开始 |
| 2026-05-31 | 用户研究完成 | ⏳ 待开始 |
| 2026-06-15 | 数据分析完成 | ⏳ 待开始 |

### Paper B 实验

| 日期 | 任务 | 状态 |
|------|------|------|
| 2026-05-01 | 30 天实验启动 | ⏳ 待开始 |
| 2026-05-31 | 延迟基准测试 | ⏳ 待开始 |
| 2026-06-15 | 人工标注完成 | ⏳ 待开始 |
| 2026-06-30 | 30 天实验完成 | ⏳ 待开始 |
| 2026-07-15 | 数据分析完成 | ⏳ 待开始 |

---

## 📊 数据管理

### 数据存储

**原始数据**:
- 保存在 `experiments/paper_{a,b}/data/`
- JSON 格式
- 每日备份

**日志文件**:
- 保存在 `experiments/paper_{a,b}/logs/`
- 按日期组织
- 自动轮转（7 天保留）

### 数据备份

**策略**:
- 每日自动备份到 `.tokitai/backups/`
- 每周手动备份到外部存储
- 实验结束后归档

### 数据隐私

**参与者数据**:
- 匿名化处理
- 仅用于研究目的
- 论文发表后保留 3 年

---

## ✅ 检查清单

### 实验前

- [ ] 伦理审查通过
- [ ] 参与者招募完成
- [ ] 实验环境配置
- [ ] 日志系统测试
- [ ] 预实验完成

### 实验中

- [ ] 每日数据检查
- [ ] 每周进度报告
- [ ] 问题及时记录
- [ ] 数据定期备份

### 实验后

- [ ] 数据完整性验证
- [ ] 统计分析完成
- [ ] 图表生成
- [ ] 结果文档撰写

---

## 🔗 相关文档

- [Paper A 详细计划](../../docs/PAPER_A_DETAILED_PLAN.md)
- [Paper B 详细计划](../../docs/PAPER_B_DETAILED_PLAN.md)
- [实施计划](../../docs/IMPLEMENTATION_PLAN_2026.md)
- [基准任务定义](./paper_a/benchmark_tasks.json)

---

**文档维护者**: AI Assistant  
**最后更新**: 2026-03-27  
**下次更新**: 2026-04-07 (日志系统完成后)
