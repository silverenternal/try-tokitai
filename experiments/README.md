# 实验框架

> **目的**：验证 Prompt Engineering 自进化系统的有效性
> 
> **实验设计**：30 天自主进化实验 + 对比实验 + 消融实验

---

## 📁 目录结构

```
experiments/
├── README.md                 # 本文档
├── tasks/                    # 基准测试任务集
│   ├── benchmark_tasks.json  # 100+ 基准任务定义
│   ├── file_ops.json         # 文件操作任务（20 个）
│   ├── code_analysis.json    # 代码分析任务（20 个）
│   ├── network.json          # 网络请求任务（15 个）
│   ├── git_ops.json          # Git 操作任务（15 个）
│   ├── data_processing.json  # 数据处理任务（15 个）
│   ├── system_monitor.json   # 系统监控任务（10 个）
│   └── composite.json        # 复合任务（15 个）
├── logs/                     # 实验日志
│   ├── control/              # Control 组日志
│   ├── ours_full/            # Ours-Full 组日志
│   ├── ours_single/          # Ours-Single 组日志
│   ├── ours_nocot/           # Ours-NoCoT 组日志
│   └── ours_nofix/           # Ours-NoFix 组日志
├── analysis/                 # 分析结果
│   ├── comparison_results.json
│   ├── ablation_results.json
│   └── visualizations/
└── scripts/                  # 评估脚本
    ├── run_benchmark.py
    ├── analyze_results.py
    └── generate_charts.py
```

---

## 🎯 实验设计

### 对比实验组

| 组名 | 说明 | 目的 |
|------|------|------|
| **Control** | 原始 tokitai（无自进化） | 基线性能 |
| **Ours-Full** | 完整 Prompt Engineering 系统 | 验证整体效果 |
| **Ours-Single** | 单 LLM 决策（无多智能体协商） | 验证多智能体价值 |
| **Ours-NoCoT** | 无 Chain-of-Thought 推理 | 验证 CoT 价值 |
| **Ours-NoFix** | 无自修正循环 | 验证编译反馈价值 |

### 实验流程

```
Week 1-2: 准备阶段
  - 设计 100+ 基准测试任务
  - 实现实验日志系统
  - 准备评估脚本

Week 3-6: 运行实验
  - 每组运行 30 天自主进化实验
  - 每天记录关键指标
  - 收集定性案例

Week 7-8: 数据分析
  - 统计分析（t-test、ANOVA）
  - 生成可视化图表
  - 撰写实验报告
```

---

## 📊 评估指标

### 主要指标

| 指标 | 定义 | 预期提升 |
|------|------|----------|
| **任务完成率** | 成功任务数 / 总任务数 | +15-20% |
| **平均工具调用次数** | 完成任务的平均工具调用数 | -30% |
| **用户满意度** | 1-5 分评分 | +0.5-1.0 分 |

### 次要指标

| 指标 | 定义 | 预期 |
|------|------|------|
| **缺口检测准确率** | 正确缺口数 / 总检测数 | >75% |
| **工具创建编译通过率** | 编译通过数 / 总创建数 | >80% |
| **工具使用率** | 活跃工具占比 | +20-30% |
| **工具失败率** | 工具调用失败比例 | -50% |

### 成本指标

| 指标 | 定义 | 预期 |
|------|------|------|
| **API 成本/月** | 美元 | <$50 |
| **平均生成时间** | 秒/工具 | <30 秒 |
| **平均修正次数** | 达到编译通过的修正次数 | 1-2 次 |

---

## 📝 基准测试任务集

### 任务分类

| 类别 | 任务数 | 难度分布 |
|------|--------|----------|
| 文件操作 | 20 | 简单 50% / 中等 40% / 困难 10% |
| 代码分析 | 20 | 简单 40% / 中等 50% / 困难 10% |
| 网络请求 | 15 | 简单 60% / 中等 30% / 困难 10% |
| Git 操作 | 15 | 简单 50% / 中等 40% / 困难 10% |
| 数据处理 | 15 | 简单 40% / 中等 50% / 困难 10% |
| 系统监控 | 10 | 简单 70% / 中等 30% |
| 复合任务 | 15 | 中等 50% / 困难 50% |

### 任务难度定义

| 难度 | 工具调用数 | 执行时间 | 说明 |
|------|------------|----------|------|
| **简单** | 1-3 次 | <10 秒 | 单一工具可完成 |
| **中等** | 4-8 次 | 10-60 秒 | 需要多个工具组合 |
| **困难** | 9+ 次 | >60 秒 | 复杂工作流，需要规划 |

---

## 🔧 实验日志格式

### 任务执行日志

```json
{
  "task_id": "task_001",
  "category": "file_ops",
  "difficulty": "medium",
  "description": "批量重命名当前目录下所有.txt 文件为.md",
  "timestamp": "2026-03-20T10:30:00Z",
  "group": "Ours-Full",
  "execution": {
    "success": true,
    "tool_calls": [
      {"tool": "list_files", "args": {"pattern": "*.txt"}, "result": "success"},
      {"tool": "batch_rename", "args": {"files": [...], "pattern": "{name}.md"}, "result": "success"}
    ],
    "total_tool_calls": 2,
    "execution_time_ms": 1250,
    "user_satisfaction": 5
  },
  "evolution": {
    "gaps_detected": 0,
    "tools_created": 0,
    "tools_optimized": 0
  }
}
```

### 自进化日志

```json
{
  "cycle_id": "cycle_001",
  "timestamp": "2026-03-20T00:00:00Z",
  "group": "Ours-Full",
  "reflection": {
    "coverage_score": 0.75,
    "systemic_issues": ["缺少批量文件处理工具"],
    "strategic_recommendations": ["优先发展文件批处理工具"]
  },
  "gaps_detected": [
    {
      "gap_type": "missing_tool",
      "description": "缺少批量重命名文件的工具",
      "suggested_name": "batch_rename_files",
      "priority": 8
    }
  ],
  "actions_taken": [
    {
      "action_type": "create_tool",
      "tool_name": "batch_rename_files",
      "result": "success",
      "compilation_attempts": 2
    }
  ],
  "metrics": {
    "api_calls": 15,
    "api_cost_usd": 0.25,
    "cycle_duration_ms": 45000
  }
}
```

---

## 📈 数据分析方法

### 统计检验

- **t-test**: 比较两组之间的性能差异
- **ANOVA**: 比较多组之间的性能差异
- **效应量（Cohen's d）**: 衡量差异的实际意义

### 可视化

- **学习曲线**: 任务完成率随时间变化
- **箱线图**: 各组性能分布对比
- **热力图**: 工具使用模式变化

---

## 🚀 快速开始

### 运行基准测试

```bash
# 运行单组基准测试
python experiments/scripts/run_benchmark.py --group Ours-Full --days 30

# 运行所有对比实验
python experiments/scripts/run_benchmark.py --all-groups

# 运行消融实验
python experiments/scripts/run_benchmark.py --ablation
```

### 分析结果

```bash
# 生成对比结果
python experiments/scripts/analyze_results.py

# 生成可视化图表
python experiments/scripts/generate_charts.py
```

---

## 📋 检查清单

### 实验前

- [ ] 100+ 基准测试任务定义完成
- [ ] 实验日志系统实现完成
- [ ] 评估脚本准备完成
- [ ] API 预算确认（<$150）

### 实验中

- [ ] 每天检查日志完整性
- [ ] 每周备份实验数据
- [ ] 记录异常情况和定性案例

### 实验后

- [ ] 数据清洗和验证
- [ ] 统计分析
- [ ] 可视化生成
- [ ] 实验报告撰写

---

**最后更新**：2026-03-20
**负责人**：待分配
