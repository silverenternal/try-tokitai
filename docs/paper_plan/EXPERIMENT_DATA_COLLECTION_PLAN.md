# 实验数据收集计划

> **目的**: 为论文 A (Git 分支) 和论文 B (HybridGapDetector) 收集实测数据
> **时间**: 2026-04-01 至 2026-06-30
> **负责人**: Tokitai Development Team
> **预算**: $630 (API 调用 $130 + 用户研究 $500)
> **当前状态**: 🟡 准备阶段 (距离预实验启动还有 3 天)

---

## 🚨 预实验启动倒计时 (2026-03-28 → 2026-04-01)

### 剩余 3 天准备清单

**日期**: 2026-03-28 (今天)

| 任务 | 负责人 | 状态 | 备注 |
|------|--------|------|------|
| ✅ Paper B Implementation 章节完善 | @AI | 完成 | 5.1-5.3 节已完成 |
| ✅ Paper B Experiments 章节完善 | @AI | 完成 | 6.1-6.5 节已完成 |
| ✅ Paper A 状态检查 | @AI | 完成 | 无需额外修改 |
| ⏳ API 密钥配置检查 | @Team | 待办 | 确认 OpenAI API 密钥有效 |
| ⏳ 实验脚本准备 | @AI | 待办 | 准备自动化脚本 |
| ⏳ 日志目录创建 | @AI | 待办 | 创建数据存储目录 |

**日期**: 2026-03-29 (明天)

| 任务 | 负责人 | 状态 | 备注 |
|------|--------|------|------|
| ⏳ API 密钥配置检查 | @Team | 待办 | `tokitai config verify` |
| ⏳ 基准测试环境准备 | @AI | 待办 | 清理测试数据 |
| ⏳ 30 天实验配置审查 | @Team | 待办 | 确认阈值参数 |

**日期**: 2026-03-30 (后天)

| 任务 | 负责人 | 状态 | 备注 |
|------|--------|------|------|
| ⏳ 预运行测试 | @AI | 待办 | 运行单次完整流程 |
| ⏳ 数据收集模板验证 | @AI | 待办 | 确认 JSON 格式正确 |
| ⏳ 预算确认 | @Team | 待办 | 确认 API 余额充足 |

**日期**: 2026-03-31 (实验前一天)

| 任务 | 负责人 | 状态 | 备注 |
|------|--------|------|------|
| ⏳ 最终系统检查 | @Team | 待办 | 全面功能验证 |
| ⏳ 实验日志初始化 | @AI | 待办 | 创建日志文件 |
| ⏳ 告警配置 | @Team | 待办 | API 成本超预算告警 |

**日期**: 2026-04-01 (🚀 预实验启动)

| 任务 | 负责人 | 状态 | 备注 |
|------|--------|------|------|
| 🚀 论文 A 性能基准测试 | @AI | 待启动 | `cargo bench` |
| 🚀 论文 B 延迟基准测试 | @AI | 待启动 | `cargo test benchmarks` |
| 🚀 30-Day Evolution Day 1 | @Team | 待启动 | 启动自主进化 |

---

## 📊 数据收集总览

### 论文 A: Git 分支式上下文管理

| 实验 | 数据类型 | 收集方法 | 完成日期 | 状态 |
|------|----------|----------|----------|------|
| **性能基准测试** | Fork/Merge/Checkout延迟 | `cargo bench` | 2026-04-30 | ⏳ 待运行 |
| **存储开销分析** | 分支数 vs 存储 | 压力测试 | 2026-04-30 | ⏳ 待运行 |
| **20+ Benchmark Tasks** | 任务成功率、时间 | 自动化测试 | 2026-05-31 | ⏳ 待运行 |
| **User Study (N=12)** | 满意度问卷 | 线下实验 | 2026-05-31 | ⏳ 待招募 |

### 论文 B: HybridGapDetector

| 实验 | 数据类型 | 收集方法 | 完成日期 | 状态 |
|------|----------|----------|----------|------|
| **30-Day Evolution** | 工具库演化日志 | 自主运行 | 2026-06-30 | ⏳ 待启动 |
| **API 成本统计** | 调用次数、成本 | 日志分析 | 2026-06-30 | ⏳ 待统计 |
| **延迟基准测试** | 各阶段延迟 | 自动化测试 | 2026-05-31 | ⏳ 待运行 |
| **准确率标注** | Precision/Recall | 人工标注 | 2026-06-15 | ⏳ 待标注 |

---

## 📝 论文 A 数据收集

### 1. 性能基准测试 (2026-04-01 ~ 2026-04-30)

#### 测试项目

| 测试项 | 目标 | 测量方法 | 预期结果 |
|--------|------|----------|----------|
| `branch_creation_fork` | Fork 延迟 | `cargo bench` | ~6ms |
| `branch_checkout` | Checkout 延迟 | `cargo bench` | ~2ms |
| `simple_merge_no_conflict` | 简单合并延迟 | `cargo bench` | ~23ms |
| `merge_with_data_copy` | 带数据合并延迟 | `cargo bench` | ~45ms |
| `cow_fork_with_symlinks` | COW fork 性能 | `cargo bench` | ~5.8ms |
| `time_travel_to_hash` | 时间旅行延迟 | `cargo bench` | ~12ms |

#### 运行命令

```bash
# 运行基准测试
cargo bench --bench parallel_context_bench -- --save-baseline baseline

# 生成报告
cargo bench --bench parallel_context_bench -- --baseline baseline
```

#### 输出数据格式

```json
{
  "test_name": "branch_creation_fork",
  "mean_latency_ms": 6.2,
  "std_dev_ms": 0.8,
  "median_ms": 6.0,
  "min_ms": 5.1,
  "max_ms": 7.5,
  "samples": 1000
}
```

**负责人**: @AI Assistant
**截止日期**: 2026-04-30

---

### 2. 存储开销分析 (2026-04-01 ~ 2026-04-30)

#### 测试设计

创建不同数量的分支，测量存储开销：

| 分支数 | 测试次数 | 测量指标 |
|--------|----------|----------|
| 1 (main) | 10 次 | 基础存储 (MB) |
| 5 | 10 次 | 相对开销 (%) |
| 10 | 10 次 | 相对开销 (%) |
| 20 | 10 次 | 相对开销 (%) |

#### 运行脚本

```python
# experiments/scripts/storage_overhead.py
import os
import shutil
import json

def measure_storage_overhead():
    results = []
    branch_counts = [1, 5, 10, 20]
    
    for count in branch_counts:
        # 创建指定数量的分支
        # 测量总存储大小
        # 计算相对开销
        results.append({
            "branch_count": count,
            "total_size_mb": ...,
            "overhead_percent": ...
        })
    
    return results
```

#### 输出数据格式

```json
[
  {"branch_count": 1, "total_size_mb": 100, "overhead_percent": 0},
  {"branch_count": 5, "total_size_mb": 108, "overhead_percent": 8},
  {"branch_count": 10, "total_size_mb": 118, "overhead_percent": 18},
  {"branch_count": 20, "total_size_mb": 135, "overhead_percent": 35}
]
```

**负责人**: @AI Assistant
**截止日期**: 2026-04-30

---

### 3. 20+ Benchmark Tasks (2026-05-01 ~ 2026-05-31)

#### 任务设计

从 110 个基准任务中选择 20+ 个代表性任务：

| 类别 | 任务数 | 难度分布 |
|------|--------|----------|
| 文件操作 | 4 | 简单 2 / 中等 1 / 困难 1 |
| 代码分析 | 4 | 简单 1 / 中等 2 / 困难 1 |
| 网络请求 | 3 | 简单 2 / 中等 1 |
| Git 操作 | 3 | 简单 1 / 中等 1 / 困难 1 |
| 数据处理 | 3 | 中等 2 / 困难 1 |
| 复合任务 | 3 | 中等 1 / 困难 2 |
| **总计** | **20** | 简单 6 / 中等 8 / 困难 6 |

#### 实验组设计

| 组名 | 说明 | 任务数 |
|------|------|--------|
| **Control** | 线性上下文 (无分支) | 20 |
| **Ours-Full** | 完整 Git 分支功能 | 20 |

#### 测量指标

| 指标 | 定义 | 测量方法 |
|------|------|----------|
| **任务成功率** | 成功/总任务 | 二元判定 |
| **探索路径数** | 尝试的方案数 | 计数 |
| **错误恢复率** | 成功恢复/失败数 | 比率 |
| **执行时间** | 任务完成时间 | 秒 |
| **工具调用次数** | 总工具调用数 | 计数 |

#### 运行脚本

```python
# experiments/scripts/run_benchmark_tasks.py
def run_benchmark(group: str, tasks: List[Task]) -> Results:
    results = []
    for task in tasks:
        result = execute_task(task, group)
        results.append({
            "task_id": task.id,
            "success": result.success,
            "exploration_paths": result.paths,
            "error_recovery": result.recovery,
            "execution_time_sec": result.time,
            "tool_calls": result.calls
        })
    return results
```

#### 输出数据格式

```json
{
  "group": "Ours-Full",
  "results": [
    {
      "task_id": "benchmark_001",
      "success": true,
      "exploration_paths": 3,
      "error_recovery": true,
      "execution_time_sec": 45.2,
      "tool_calls": 12
    },
    ...
  ],
  "summary": {
    "success_rate": 0.75,
    "avg_paths": 2.8,
    "recovery_rate": 0.80,
    "avg_time_sec": 52.3,
    "avg_calls": 15.6
  }
}
```

**负责人**: @AI Assistant
**截止日期**: 2026-05-31

---

### 4. User Study (N=12) (2026-05-01 ~ 2026-05-31)

#### 参与者招募

| 渠道 | 目标人数 | 报酬 |
|------|----------|------|
| 校内学生 | 6 | $25/人 |
| 开发者论坛 | 4 | $25/人 |
| 社交媒体 | 2 | $25/人 |
| **总计** | **12** | **$300** |

#### 实验任务

每个参与者完成 4 个任务 (2 个简单 + 2 个困难)：

| 任务 ID | 类型 | 难度 | 说明 |
|--------|------|------|------|
| U01 | 文件操作 | 简单 | 批量重命名文件 |
| U02 | 代码重构 | 困难 | 多方案探索重构 |
| U03 | Bug 调试 | 困难 | 多假设验证 |
| U04 | 时间旅行 | 简单 | 回到历史状态 |

#### 实验流程

```
1. 介绍阶段 (10 分钟)
   - 系统介绍
   - 操作演示

2. 练习任务 (15 分钟)
   - 简单任务练习
   - 熟悉界面

3. 正式任务 (60 分钟)
   - 完成 4 个任务
   - 记录操作日志

4. 问卷填写 (20 分钟)
   - SUS 量表
   - NASA-TLX 量表
   - 自定义满意度

5. 访谈 (15 分钟)
   - 定性反馈
   - 改进建议
```

#### 问卷设计

**SUS (System Usability Scale)**:
1. 我愿意经常使用这个系统
2. 我觉得系统太复杂
3. 我觉得系统易用
4. 我觉得需要技术支持才能使用
5. 系统功能集成良好
6. 系统不一致
7. 新手容易上手
8. 系统笨重
9. 使用系统很自信
10. 使用前需要学习很多

**NASA-TLX (Task Load Index)**:
- 脑力需求
- 体力需求
- 时间需求
- 努力程度
- 挫折感
- 表现满意度

**自定义满意度** (1-5 分):
- 分支功能有用性
- 合并功能有用性
- 时间旅行有用性
- 总体满意度
- 推荐意愿

#### 输出数据格式

```json
{
  "participant_id": "P001",
  "demographics": {
    "age": 25,
    "experience_years": 3,
    "llm_usage": "daily"
  },
  "task_results": [
    {
      "task_id": "U01",
      "success": true,
      "time_sec": 120,
      "errors": 0
    },
    ...
  ],
  "questionnaires": {
    "sus_score": 85,
    "nasa_tlx_score": 45,
    "satisfaction_scores": {
      "branch_usefulness": 5,
      "merge_usefulness": 5,
      "time_travel_usefulness": 4,
      "overall": 5,
      "recommend": 5
    }
  },
  "feedback": {
    "positive": ["分支功能很实用", "多方案探索很方便"],
    "negative": ["学习曲线稍陡", "合并冲突解决复杂"],
    "suggestions": ["添加更多示例", "改进冲突解决界面"]
  }
}
```

**负责人**: @Team
**截止日期**: 2026-05-31

---

## 📝 论文 B 数据收集

### 1. 30-Day Evolution Study (2026-04-01 ~ 2026-06-30)

#### 实验设计

运行自主进化系统 30 天，每天记录：

| 指标 | 测量方法 | 频率 |
|------|----------|------|
| 工具总数 | 计数 | 每天 |
| 新增工具数 | 计数 | 每天 |
| 工具失败率 | 失败/总调用 | 每天 |
| 任务完成率 | 成功/总任务 | 每天 |
| API 调用次数 | 计数 | 每天 |
| API 成本 | 美元 | 每天 |
| 检测缺口数 | 计数 | 每天 |
| 创建工具数 | 计数 | 每天 |

#### 运行脚本

```python
# experiments/scripts/run_30day_evolution.py
def run_daily_evolution():
    for day in range(30):
        # 运行自主进化循环
        # 记录日志
        log_daily_metrics({
            "day": day,
            "total_tools": ...,
            "new_tools": ...,
            "failure_rate": ...,
            "task_success_rate": ...,
            "api_calls": ...,
            "api_cost_usd": ...,
            "gaps_detected": ...,
            "tools_created": ...
        })
```

#### 输出数据格式

```json
{
  "experiment_id": "30day_001",
  "start_date": "2026-05-01",
  "end_date": "2026-05-30",
  "daily_logs": [
    {
      "day": 1,
      "total_tools": 63,
      "new_tools": 0,
      "failure_rate": 0.25,
      "task_success_rate": 0.65,
      "api_calls": 150,
      "api_cost_usd": 0.15,
      "gaps_detected": 2,
      "tools_created": 0
    },
    ...
  ],
  "summary": {
    "total_new_tools": 12,
    "avg_daily_cost_usd": 0.12,
    "total_api_cost_usd": 3.60,
    "failure_rate_improvement": -0.52,
    "success_rate_improvement": 0.23
  }
}
```

**负责人**: @Team
**截止日期**: 2026-06-30

---

### 2. API 成本统计 (2026-06-01 ~ 2026-06-30)

#### 统计方法

分析 30 天实验日志，计算：

| 成本项 | 计算方法 | 预期值 |
|--------|----------|--------|
| Stage 1 成本 | 0 API × 30 天 | $0 |
| Stage 2 成本 | 2 API/天 × 30 天 × $0.0025 | $0.15 |
| PromptCreator | 3 工具 × $0.05 | $0.15 |
| MultiAgent | 1 协商 × $0.10 | $0.10 |
| **月度总成本** | - | **$2.25** |

#### 输出数据格式

```json
{
  "period": "2026-05-01 to 2026-05-30",
  "cost_breakdown": {
    "stage1_statistical": 0.00,
    "stage2_causal": 0.15,
    "prompt_creator": 0.15,
    "multi_agent": 0.10,
    "other": 0.05,
    "total": 2.25
  },
  "comparison": {
    "pure_statistical": 0.00,
    "pure_prompt_engineering": 45.00,
    "hybrid": 2.25,
    "savings_percent": 95
  }
}
```

**负责人**: @AI Assistant
**截止日期**: 2026-06-30

---

### 3. 延迟基准测试 (2026-05-01 ~ 2026-05-31)

#### 测试项目

| 测试项 | 目标 | 预期结果 |
|--------|------|----------|
| `stage1_filter` | Statistical Filter 延迟 | <100ms |
| `stage2_causal_single` | 单缺口因果分析 | 1-3 秒 |
| `stage2_causal_batch` | 批量因果分析 (10 缺口) | 5-10 秒 |
| `stage3_merge` | Merger & Prioritize | <50ms |
| `end_to_end` | 端到端检测 | 1-5 秒 |

#### 运行命令

```bash
# 运行延迟测试
cargo test --release hybrid_gap_detector::benchmarks -- --nocapture
```

**负责人**: @AI Assistant
**截止日期**: 2026-05-31

---

### 4. 准确率标注 (2026-06-01 ~ 2026-06-15)

#### 标注设计

人工标注 100+ 检测到的缺口：

| 类别 | 数量 | 标注内容 |
|------|------|----------|
| True Positive | 70+ | 确实是工具缺口 |
| False Positive | 20+ | 误报 |
| False Negative | 10+ | 漏报 (从历史中抽样) |

#### 标注流程

```
1. 从 30 天日志中抽取 100+ 缺口
2. 3 位标注者独立标注
3. 计算 Kappa 一致性系数
4. 讨论分歧，达成共识
```

#### 输出数据格式

```json
{
  "annotation_id": "anno_001",
  "gap_id": "gap_045",
  "gap_description": "缺少批量下载工具",
  "annotations": [
    {"annotator": "A1", "is_true_gap": true, "confidence": 0.9},
    {"annotator": "A2", "is_true_gap": true, "confidence": 0.85},
    {"annotator": "A3", "is_true_gap": true, "confidence": 0.95}
  ],
  "consensus": true,
  "final_label": "true_positive"
}
```

#### 计算指标

```
Precision = TP / (TP + FP)
Recall = TP / (TP + FN)
F1 = 2 × (Precision × Recall) / (Precision + Recall)
```

**负责人**: @Team
**截止日期**: 2026-06-15

---

## 📊 数据存储与管理

### 目录结构

```
experiments/logs/
├── paper_a/
│   ├── benchmarks/           # 性能基准测试数据
│   ├── storage_overhead/     # 存储开销数据
│   ├── benchmark_tasks/      # 20+ 任务结果
│   └── user_study/           # 用户研究数据
└── paper_b/
    ├── 30day_evolution/      # 30 天实验日志
    ├── api_costs/            # API 成本统计
    ├── latency_benchmarks/   # 延迟测试
    └── annotations/          # 准确率标注
```

### 数据备份

- **本地备份**: `/data/backups/experiments/`
- **云端备份**: Google Drive / OneDrive
- **版本控制**: Git LFS (大文件)

### 数据共享

- **论文 A 数据**: 投稿时公开 (Zenodo / Figshare)
- **论文 B 数据**: 投稿时公开
- **代码仓库**: GitHub 公开

---

## 💰 预算明细

| 项目 | 数量 | 单价 | 总价 |
|------|------|------|------|
| **论文 A** | | | |
| API 调用 (Benchmark) | 1000 次 | $0.002 | $2 |
| User Study 报酬 | 12 人 | $25 | $300 |
| **论文 B** | | | |
| API 调用 (30 天实验) | 30 天 | $0.12/天 | $3.60 |
| API 调用 (准确率标注) | 100 次 | $0.002 | $0.20 |
| 标注者报酬 | 3 人 | $50 | $150 |
| **总计** | | | **$455.80** |

**预算剩余**: $630 - $455.80 = **$174.20** (用于应急)

---

## ✅ 检查清单

### 论文 A

- [ ] 性能基准测试运行完成
- [ ] 存储开销分析完成
- [ ] 20+ Benchmark Tasks 运行完成
- [ ] User Study (N=12) 执行完成
- [ ] 数据备份完成

### 论文 B

- [ ] 30-Day Evolution 启动
- [ ] API 成本统计完成
- [ ] 延迟基准测试运行完成
- [ ] 准确率标注完成 (100+ 样本)
- [ ] 数据备份完成

---

**计划创建时间**: 2026-03-27
**下次更新**: 2026-04-30 (基准测试完成后)
**负责人**: Tokitai Development Team
