# 实验框架状态报告

> **报告日期**: 2026-03-27
> **状态**: 框架就绪，等待数据收集
> **基准任务**: 110 个定义完成
> **实验组**: 5 组对比实验

---

## 📋 执行摘要

### 实验框架概述

try-tokitai 项目提供完整的实验框架用于验证核心创新点的有效性，包括：
- **Git 分支式上下文管理** - 任务成功率、用户满意度
- **HybridGapDetector** - 成本效益、检测准确率
- **Prompt Engineering 自进化系统** - 工具创建质量、进化效果

### 当前状态总览

| 组件 | 状态 | 完成度 | 说明 |
|------|------|--------|------|
| **基准测试任务定义** | ✅ 完成 | 100% | 110 个任务定义完成 |
| **实验日志系统** | ✅ 完成 | 100% | 5 组实验日志目录就绪 |
| **评估脚本** | ✅ 完成 | 100% | 统计分析脚本就绪 |
| **实验数据收集** | ⏳ 待启动 | 0% | 计划 2026-04 启动 |
| **数据分析** | ⏳ 待开始 | 0% | 等待数据收集完成 |

---

## 🧪 实验设计

### 对比实验组

| 组名 | 说明 | 目的 |
|------|------|------|
| **Control** | 原始 tokitai（无自进化、无 Git 分支） | 基线性能 |
| **Ours-Full** | 完整系统（Git 分支 + HybridGapDetector + Prompt Engineering） | 验证整体效果 |
| **Ours-Single** | 单 LLM 决策（无多智能体协商） | 验证多智能体价值 |
| **Ours-NoCoT** | 移除 Chain-of-Thought 推理 | 验证 CoT 价值 |
| **Ours-NoFix** | 移除自修正循环 | 验证编译反馈价值 |

### 实验流程

```
Week 1-2 (2026-04-01 ~ 2026-04-14): 准备阶段 ✅
  - ✅ 设计 110+ 基准测试任务
  - ✅ 实现实验日志系统
  - ✅ 准备评估脚本
  - ⏳ 预实验验证（计划中）

Week 3-6 (2026-04-15 ~ 2026-05-12): 运行实验 ⏳
  - 每组运行 30 天自主进化实验
  - 每天记录关键指标
  - 收集定性案例

Week 7-8 (2026-05-13 ~ 2026-05-26): 数据分析 ⏳
  - 统计分析（t-test、ANOVA）
  - 生成可视化图表
  - 撰写实验报告
```

---

## 📊 基准测试任务集

### 任务分类统计

| 类别 | 任务数 | 难度分布 | 状态 |
|------|--------|----------|------|
| **文件操作** | 20 | 简单 50% / 中等 40% / 困难 10% | ✅ 完成 |
| **代码分析** | 20 | 简单 40% / 中等 50% / 困难 10% | ✅ 完成 |
| **网络请求** | 15 | 简单 60% / 中等 30% / 困难 10% | ✅ 完成 |
| **Git 操作** | 15 | 简单 50% / 中等 40% / 困难 10% | ✅ 完成 |
| **数据处理** | 15 | 简单 40% / 中等 50% / 困难 10% | ✅ 完成 |
| **系统监控** | 10 | 简单 70% / 中等 30% | ✅ 完成 |
| **复合任务** | 15 | 中等 50% / 困难 50% | ✅ 完成 |
| **总计** | **110** | - | ✅ 完成 |

### 任务难度定义

| 难度 | 工具调用数 | 执行时间 | 说明 |
|------|------------|----------|------|
| **简单** | 1-3 次 | <10 秒 | 单一工具可完成 |
| **中等** | 4-8 次 | 10-60 秒 | 需要多个工具组合 |
| **困难** | 9+ 次 | >60 秒 | 复杂工作流，需要规划 |

### 任务示例

**文件操作 - 简单**:
```json
{
  "task_id": "file_001",
  "category": "file_ops",
  "difficulty": "easy",
  "description": "读取 README.md 的内容",
  "expected_tool_calls": ["read_file"],
  "expected_duration_ms": 500
}
```

**代码分析 - 困难**:
```json
{
  "task_id": "code_020",
  "category": "code_analysis",
  "difficulty": "hard",
  "description": "分析整个项目的代码复杂度分布，识别最复杂的 5 个函数",
  "expected_tool_calls": ["list_files", "read_file", "analyze_complexity", ...],
  "expected_duration_ms": 30000
}
```

---

## 📈 评估指标

### 主要指标

| 指标 | 定义 | 测量方法 | 预期提升 |
|------|------|----------|----------|
| **任务完成率** | 成功任务数 / 总任务数 | 二元成功/失败判定 | +15-20% |
| **平均工具调用次数** | 完成任务的平均工具调用数 | 统计平均值 | -30% |
| **用户满意度** | 1-5 分评分 | Likert 量表 | +0.5-1.0 分 |

### 次要指标

| 指标 | 定义 | 测量方法 | 预期值 |
|------|------|----------|--------|
| **缺口检测准确率** | 正确缺口数 / 总检测数 | 人工标注验证 | >75% |
| **工具创建编译通过率** | 编译通过数 / 总创建数 | cargo check | >80% |
| **工具使用率** | 活跃工具占比 | 活跃数/总数 | +20-30% |
| **工具失败率** | 工具调用失败比例 | 失败数/总数 | -50% |

### 成本指标

| 指标 | 定义 | 测量方法 | 预期值 |
|------|------|----------|--------|
| **API 成本/月** | 美元 | 实际调用统计 | <$50 |
| **平均生成时间** | 秒/工具 | 时间戳差值 | <30 秒 |
| **平均修正次数** | 达到编译通过的修正次数 | 计数 | 1-2 次 |

---

## 🔧 实验日志格式

### 任务执行日志格式

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

### 自进化日志格式

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

## 📁 目录结构

```
experiments/
├── README.md                     ✅ 实验框架说明
├── IMPLEMENTATION_SUMMARY.md     ✅ 实现总结
├── tasks/
│   └── benchmark_tasks.json      ✅ 110 个基准任务定义
├── logs/
│   ├── control/                  ✅ 目录就绪 (空)
│   ├── ours_full/                ✅ 目录就绪 (空)
│   ├── ours_single/              ✅ 目录就绪 (空)
│   ├── ours_nocot/               ✅ 目录就绪 (空)
│   └── ours_nofix/               ✅ 目录就绪 (空)
├── analysis/
│   └── .gitkeep                  ✅ 目录就绪 (空)
└── scripts/
    ├── run_benchmark.py          ✅ 基准测试运行脚本
    ├── analyze_results.py        ✅ 结果分析脚本
    └── generate_charts.py        ✅ 图表生成脚本
```

---

## 🔬 数据分析方法

### 统计检验

| 方法 | 用途 | 实施脚本 |
|------|------|----------|
| **t-test** | 比较两组之间的性能差异 | `analyze_results.py` |
| **ANOVA** | 比较多组之间的性能差异 | `analyze_results.py` |
| **效应量 (Cohen's d)** | 衡量差异的实际意义 | `analyze_results.py` |
| **显著性检验 (p-value)** | 验证统计显著性 | `analyze_results.py` |

### 可视化

| 图表类型 | 用途 | 实施脚本 |
|----------|------|----------|
| **学习曲线** | 任务完成率随时间变化 | `generate_charts.py` |
| **箱线图** | 各组性能分布对比 | `generate_charts.py` |
| **热力图** | 工具使用模式变化 | `generate_charts.py` |
| **柱状图** | 成本对比 | `generate_charts.py` |

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

# 生成 LaTeX 表格
python experiments/scripts/generate_tables.py
```

---

## 📋 检查清单

### 实验前 ✅

- [x] 110+ 基准测试任务定义完成
- [x] 实验日志系统实现完成
- [x] 评估脚本准备完成
- [ ] API 预算确认（<$150）
- [ ] 预实验验证（计划 2026-04-01）

### 实验中 ⏳

- [ ] 每天检查日志完整性
- [ ] 每周备份实验数据
- [ ] 记录异常情况和定性案例

### 实验后 ⏳

- [ ] 数据清洗和验证
- [ ] 统计分析
- [ ] 可视化生成
- [ ] 实验报告撰写

---

## ⏳ 时间线

### 2026-04: 基准测试运行

| 周次 | 任务 | 交付物 |
|------|------|--------|
| Week 1 (04-01 ~ 04-07) | 预实验验证 | 验证脚本、问题修复 |
| Week 2 (04-08 ~ 04-14) | Control 组实验 | Control 组数据 |
| Week 3 (04-15 ~ 04-21) | Ours-Full 组实验 | Ours-Full 组数据 |
| Week 4 (04-22 ~ 04-28) | 消融实验 | Ours-Single/NoCoT/NoFix 数据 |

### 2026-05: 用户研究 + 长期实验

| 周次 | 任务 | 交付物 |
|------|------|--------|
| Week 5 (04-29 ~ 05-05) | 用户研究招募 | N=12+ 参与者 |
| Week 6 (05-06 ~ 05-12) | 用户研究执行 | 满意度问卷数据 |
| Week 7 (05-13 ~ 05-19) | 30 天实验启动 | 自主进化日志 |
| Week 8 (05-20 ~ 05-26) | 数据初步分析 | 初步统计结果 |

### 2026-06: 数据分析 + 报告

| 周次 | 任务 | 交付物 |
|------|------|--------|
| Week 9 (05-27 ~ 06-02) | 完整数据分析 | 统计分析报告 |
| Week 10 (06-03 ~ 06-09) | 可视化生成 | 图表集 |
| Week 11 (06-10 ~ 06-16) | 实验报告撰写 | 实验报告初稿 |
| Week 12 (06-17 ~ 06-23) | 内部评审 | 评审意见 |

---

## 💰 预算明细

| 项目 | 预算 | 说明 |
|------|------|------|
| API 调用 (Control 组) | $10 | 30 天基线实验 |
| API 调用 (Ours-Full 组) | $50 | 30 天完整系统实验 |
| API 调用 (消融实验) | $30 | 3 组 x 30 天 |
| API 调用 (用户研究) | $20 | N=12 参与者 |
| API 调用 (rebuttal) | $20 | 论文修改期间 |
| 用户研究报酬 | $500 | N=20 x $25/人 |
| **总计** | **$630** | 远低于总预算 $1650 |

---

## ⚠️ 风险与应对

### 技术风险

| 风险 | 概率 | 影响 | 应对方案 |
|------|------|------|----------|
| LLM 输出不稳定 | 中 | 高 | JSON Schema 约束 + 验证器 + 多轮迭代 |
| API 成本超预算 | 低 | 中 | 缓存历史结果 + 批量处理 + 本地模型备选 |
| 实验效果不佳 | 中 | 高 | 调整 Prompt + 增加 Few-Shot 示例 |

### 研究风险

| 风险 | 概率 | 影响 | 应对方案 |
|------|------|------|----------|
| 实验结果不显著 | 中 | 高 | 设计更复杂的基准任务 |
| 用户研究招募困难 | 低 | 中 | 扩大招募渠道 (社交媒体/论坛) |
| 数据收集延迟 | 中 | 中 | 提前启动预实验，预留缓冲时间 |

---

## 📊 预期结果

### Git 分支上下文管理

| 指标 | Control | Ours-Full | 提升 |
|------|---------|-----------|------|
| 任务成功率 | 53% | 75% | +42% |
| 探索路径数 | 1.2 | 2.8 | +133% |
| 错误恢复率 | 45% | 80% | +78% |
| 用户满意度 | 3.8/5 | 4.6/5 | +21% |

### HybridGapDetector

| 指标 | 纯 Prompt | Hybrid | 提升 |
|------|-----------|--------|------|
| API 成本/月 | $45 | $2.25 | -95% |
| 检测延迟 | 5-30 秒 | 1-5 秒 | -83% |
| 检测准确率 | 75% | 72% | -3% (可接受) |

### Prompt Engineering 自进化

| 指标 | Control | Ours-Full | 提升 |
|------|---------|-----------|------|
| 任务完成率 | 65% | 80%+ | +15% |
| 平均工具调用数 | 8.5 | 5.5 | -35% |
| 工具失败率 | 25% | 12% | -52% |
| 用户满意度 | 3.2/5 | 4.2/5 | +31% |

---

## 📚 相关文档

- [experiments/README.md](experiments/README.md) - 实验框架主文档
- [experiments/tasks/benchmark_tasks.json](experiments/tasks/benchmark_tasks.json) - 110 个基准任务
- [docs/PARALLEL_CONTEXT_STATUS_REPORT.md](docs/PARALLEL_CONTEXT_STATUS_REPORT.md) - Git 分支实现报告
- [PROJECT_STATUS_2026_03.md](../PROJECT_STATUS_2026_03.md) - 项目整体状态

---

**报告生成时间**: 2026-03-27
**版本**: v1.0
**状态**: 框架就绪，等待数据收集
**下次更新**: 2026-04-07 (预实验验证后)
