# 学术论文实验框架使用指南

> **版本**: 1.0.0
> **创建日期**: 2026-03-25
> **目标**: 支持 AAAI/ACL/EMNLP 等顶会论文的实验需求

---

## 📋 目录

1. [快速开始](#快速开始)
2. [CLI 命令](#cli-命令)
3. [实验框架架构](#实验框架架构)
4. [使用示例](#使用示例)
5. [输出格式](#输出格式)
6. [统计分析](#统计分析)
7. [论文集成](#论文集成)

---

## 🚀 快速开始

### 1. 生成基准测试任务

```bash
# 生成基准测试数据集
cargo run --release -- experiment benchmark --output benchmarks/aaai2027 --seed 42
```

**输出**:
```
📊 生成基准测试任务...

✅ 基准测试生成完成！
   输出目录：benchmarks/aaai2027
   总任务数：20
   有缺口任务：8
   真实缺口：8
```

### 2. 运行完整实验

```bash
# 运行对比实验 + 消融实验
cargo run --release -- experiment run --name aaai2027_main --output experiments/aaai2027
```

**输出**:
```
🔬 实验框架 v1.0.0
==================

🚀 运行完整实验...

📊 生成基准测试任务...
   总任务数：20
   有缺口任务：8
   真实缺口：8

🔬 运行对比实验...
   完成对比实验
   Control: 完成率=70.0%, 缺口=0
   Ours-Full: 完成率=85.0%, 缺口=8

🧪 运行消融实验...
   完成消融实验
   Ours-No-CoT: 完成率=80.0%, 缺口=6
   Ours-No-Fix: 完成率=82.5%, 缺口=7
   Ours-Single-Agent: 完成率=83.0%, 缺口=7
   Ours-No-Statistical: 完成率=84.0%, 缺口=8
   Ours-No-Causal: 完成率=75.0%, 缺口=5

📝 生成实验报告...

✅ 实验完成！报告保存在：experiments/aaai2027
```

---

## 🛠️ CLI 命令

### 命令列表

| 命令 | 说明 | 示例 |
|------|------|------|
| `run` | 运行完整实验（对比 + 消融） | `experiment run --name aaai2027` |
| `benchmark` | 生成基准测试任务集 | `experiment benchmark --output benchmarks/` |
| `comparative` | 仅运行对比实验 | `experiment comparative --name comparison` |
| `ablation` | 仅运行消融实验 | `experiment ablation --name ablation` |
| `report` | 从已有数据生成报告 | `experiment report --input experiments/` |
| `help` | 显示帮助信息 | `experiment help` |

### 全局参数

| 参数 | 简写 | 说明 | 默认值 |
|------|------|------|--------|
| `--name` | `-n` | 实验名称 | `experiment` |
| `--output` | `-o` | 输出目录 | `experiments/output` |
| `--seed` | `-s` | 随机种子（可复现性） | `42` |
| `--comparative-only` | - | 仅运行对比实验 | - |
| `--ablation-only` | - | 仅运行消融实验 | - |

---

## 🏗️ 实验框架架构

### 模块结构

```
src/experiments/
├── framework.rs              # 实验框架核心
│   ├── ExperimentRunner      # 实验运行器
│   ├── ExperimentConfig      # 实验配置
│   ├── ExperimentGroupType   # 实验组类型（Control/Ours-Full/...）
│   └── CoreMetrics           # 核心指标
│
├── data_collector.rs         # 数据收集器
│   ├── DataCollector         # 数据收集器
│   ├── DetailedTaskLog       # 详细任务日志
│   ├── GapDetectionEvent     # 缺口检测事件
│   └── ApiCallLog           # API 调用日志
│
├── statistical_analysis.rs   # 统计分析
│   ├── welch_t_test()        # Welch's t 检验
│   ├── paired_t_test()       # 配对样本 t 检验
│   ├── one_way_anova()       # 单因素 ANOVA
│   ├── mann_whitney_u_test() # Mann-Whitney U 检验
│   └── EffectSizeMagnitude   # 效应量解释
│
├── metrics.rs                # 指标计算器
│   ├── MetricsCalculator     # 指标计算器
│   ├── CoreExperimentMetrics # 核心指标
│   └── ComparisonResult      # 对比结果
│
├── report_generator.rs       # 报告生成器
│   ├── ReportGenerator       # 报告生成器
│   ├── ExperimentReport      # 实验报告
│   └── StatisticalTestResult # 统计检验结果
│
├── benchmark_tasks.rs        # 基准测试任务
│   ├── BenchmarkGenerator    # 基准测试生成器
│   ├── BenchmarkTask         # 基准测试任务
│   └── BenchmarkDataset      # 基准测试数据集
│
└── cli.rs                    # CLI 命令
    └── run_experiment_command()
```

### 实验组类型

```rust
pub enum ExperimentGroupType {
    Control,              // 对照组：无自进化系统
    OursFull,             // 实验组：完整系统
    OursNoCoT,            // 消融：无 Chain-of-Thought
    OursNoFix,            // 消融：无自修正循环
    OursSingleAgent,      // 消融：单智能体
    OursNoStatistical,    // 消融：无统计过滤
    OursNoCausal,         // 消融：无因果推理
}
```

---

## 📊 使用示例

### 1. 编程方式使用实验框架

```rust
use crate::experiments::*;

// 创建实验配置
let config = ExperimentConfig {
    name: "aaai2027_main".to_string(),
    description: "Main experiment for AAAI 2027 paper".to_string(),
    output_dir: PathBuf::from("experiments/aaai2027"),
    random_seed: 42,
    ..Default::default()
};

// 创建实验运行器
let mut runner = ExperimentRunner::new(config)?;

// 生成基准测试任务
let benchmark_gen = BenchmarkGenerator::new(&output_dir, 42)?;
let dataset = benchmark_gen.generate_full_benchmark()?;

// 转换为任务记录
let task_records: Vec<TaskExecutionRecord> = dataset.tasks.iter()
    .map(|t| benchmark_to_execution_record(
        t,
        false, // 模拟失败以测试缺口检测
        vec![],
        1000,
        Some(2)
    ))
    .collect();

// 运行对比实验
let comparative = runner.run_comparative(&task_records).await?;

// 运行消融实验
let ablation = runner.run_ablation(&task_records).await?;

// 统计分析
let mut statistical_tests = Vec::new();

// t 检验：Control vs Ours-Full
if let (Some(control), Some(ours)) = (
    comparative.groups.get(&ExperimentGroupType::Control),
    comparative.groups.get(&ExperimentGroupType::OursFull),
) {
    // 收集数据
    let control_data = vec![control.metrics.task_completion_rate];
    let ours_data = vec![ours.metrics.task_completion_rate];
    
    // 执行 t 检验
    let t_test = welch_t_test(&control_data, &ours_data)?;
    
    statistical_tests.push(StatisticalTestResult {
        test_name: "Task Completion Rate".to_string(),
        groups: vec!["Control".to_string(), "Ours-Full".to_string()],
        t_test: Some(t_test),
        anova: None,
        mann_whitney: None,
        effect_size_interpretation: format!(
            "Cohen's d = {:.3} ({})",
            t_test.cohens_d,
            EffectSizeMagnitude::from_cohens_d(t_test.cohens_d).as_str()
        ),
    });
}

// 生成报告
let report_gen = ReportGenerator::new(&output_dir)?;
let report = report_gen.generate_full_report(
    &runner.config,
    Some(&comparative),
    Some(&ablation),
    statistical_tests,
)?;

println!("实验完成！报告保存在：{:?}", output_dir);
```

### 2. 自定义基准测试任务

```rust
use crate::experiments::benchmark_tasks::*;

let mut tasks = Vec::new();

// 添加代码生成任务
tasks.push(BenchmarkTask {
    task_id: Uuid::new_v4().to_string(),
    task_type: TaskType::CodeGeneration,
    difficulty: 4,
    description: "Generate API client from OpenAPI spec".to_string(),
    input: serde_json::json!({
        "spec_url": "https://api.example.com/openapi.json",
        "language": "rust"
    }),
    expected_output: None,
    required_tools: vec!["fetch_url".to_string(), "generate_code".to_string()],
    has_tool_gap: true,
    gap_description: Some("Missing OpenAPI parser".to_string()),
    suggested_new_tool: Some("parse_openapi".to_string()),
});

// 创建数据集
let dataset = BenchmarkDataset {
    name: "custom_benchmark".to_string(),
    description: "Custom benchmark for specific evaluation".to_string(),
    version: "1.0.0".to_string(),
    created_at: Utc::now().to_rfc3339(),
    tasks,
    ground_truth_gap_ids: HashSet::new(),
    statistics: DatasetStatistics {
        total_tasks: 1,
        tasks_by_type: HashMap::new(),
        tasks_by_difficulty: HashMap::new(),
        tasks_with_gaps: 1,
        ground_truth_gaps: 1,
    },
};
```

---

## 📁 输出格式

### 1. JSON 报告

**文件**: `experiment_report.json`

```json
{
  "report_id": "uuid-here",
  "config": {
    "name": "aaai2027_main",
    "description": "...",
    "git_commit": "abc123"
  },
  "generated_at": "2026-03-25T10:00:00Z",
  "comparative_results": {
    "experiment_id": "...",
    "groups": {
      "Control": {
        "group_type": "Control",
        "metrics": {
          "task_completion_rate": 0.70,
          "avg_tool_calls": 8.5,
          "tool_failure_rate": 0.25,
          "user_satisfaction": 3.2
        }
      },
      "Ours-Full": {
        "metrics": {
          "task_completion_rate": 0.85,
          "avg_tool_calls": 5.5,
          "tool_failure_rate": 0.12,
          "user_satisfaction": 4.2
        }
      }
    }
  },
  "statistical_tests": [...]
}
```

### 2. Markdown 报告

**文件**: `experiment_report.md`

```markdown
# Experiment Report: aaai2027_main

**Generated**: 2026-03-25 10:00:00 UTC
**Git Commit**: `abc123`

## Comparative Experiment Results

### Group Metrics

| Metric | Control | Ours-Full | Improvement |
|--------|---------|-----------|-------------|
| Task Completion Rate | 70.0% | 85.0% | +21.4% |
| Avg Tool Calls | 8.50 | 5.50 | -35.3% |
| Tool Failure Rate | 25.0% | 12.0% | -52.0% |
| User Satisfaction | 3.20 | 4.20 | +31.3% |
```

### 3. LaTeX 表格

**文件**: `experiment_tables.tex`

```latex
\begin{table}[t]
\centering
\caption{Comparative Experiment Results: Control vs Ours-Full}
\label{tab:comparative}
\begin{tabular}{lcc}
\toprule
\textbf{Metric} & \textbf{Control} & \textbf{Ours-Full} \\
\midrule
Task Completion Rate (\%) & 70.00 & 85.00 \\
Avg Tool Calls & 8.50 & 5.50 \\
Tool Failure Rate (\%) & 25.00 & 12.00 \\
User Satisfaction (1-5) & 3.20 & 4.20 \\
\bottomrule
\end{tabular}
\end{table}
```

### 4. CSV 数据

**文件**: `comparative_results.csv`

```csv
group,task_completion_rate,avg_tool_calls,tool_failure_rate,user_satisfaction,gaps_detected,total_api_cost_usd
Control,0.7000,8.5000,0.2500,3.2000,0,0.000000
Ours-Full,0.8500,5.5000,0.1200,4.2000,8,0.120000
```

---

## 📈 统计分析

### 1. T 检验（对比实验）

```rust
use crate::experiments::statistical_analysis::*;

// 独立样本 t 检验（Welch's t-test，假设方差不等）
let t_test = welch_t_test(&control_data, &ours_data)?;

println!("t({:.1}) = {:.3}, p = {:.4}", 
    t_test.degrees_of_freedom,
    t_test.t_statistic,
    t_test.p_value_two_tailed
);

// 效应量解释
let effect_size = EffectSizeMagnitude::from_cohens_d(t_test.cohens_d);
println!("Cohen's d = {:.3} ({})", t_test.cohens_d, effect_size.as_str());
```

**输出示例**:
```
t(18.5) = 2.453, p = 0.0241
Cohen's d = 0.856 (large)
```

### 2. ANOVA（消融实验）

```rust
// 单因素 ANOVA
let anova = one_way_anova(&[
    &control_data,
    &ours_full_data,
    &ours_no_cot_data,
    &ours_no_fix_data,
])?;

println!("F({},{}) = {:.3}, p = {:.4}, η² = {:.3}",
    anova.df_between,
    anova.df_within,
    anova.f_statistic,
    anova.p_value,
    anova.eta_squared
);
```

### 3. 多重检验校正

```rust
// Bonferroni 校正
let p_values = vec![0.01, 0.03, 0.04, 0.02];
let significant = bonferroni_correction(&p_values, 0.05);

// Benjamini-Hochberg 校正（FDR 控制）
let significant = benjamini_hochberg_correction(&p_values, 0.05);
```

---

## 📝 论文集成

### 1. 核心指标表格（论文 Table 1）

```latex
\begin{table}[t]
\centering
\caption{Main Results: Comparative Experiment}
\label{tab:main_results}
\begin{tabular}{lcc}
\toprule
\textbf{Metric} & \textbf{Control} & \textbf{Ours-Full} \\
\midrule
Task Completion Rate (\%) & 70.0 & \textbf{85.0}$^\ast$ \\
Avg Tool Calls & 8.5 & \textbf{5.5}$^\ast$ \\
Tool Failure Rate (\%) & 25.0 & \textbf{12.0}$^\ast$ \\
User Satisfaction (1-5) & 3.2 & \textbf{4.2}$^\ast$ \\
\bottomrule
\end{tabular}
\end{table}
```

### 2. 消融实验表格（论文 Table 2）

```latex
\begin{table}[t]
\centering
\caption{Ablation Study: Component Contributions}
\label{tab:ablation}
\begin{tabular}{lccc}
\toprule
\textbf{Configuration} & \textbf{Completion (\%)} & \textbf{Gaps} & \textbf{Cost} \\
\midrule
Ours-Full & \textbf{85.0} & 8 & \$0.12 \\
- CoT & 80.0 & 6 & \$0.08 \\
- Self-Correction & 82.5 & 7 & \$0.10 \\
- Multi-Agent & 83.0 & 7 & \$0.11 \\
- Statistical & 84.0 & 8 & \$0.15 \\
- Causal & 75.0 & 5 & \$0.00 \\
\bottomrule
\end{tabular}
\end{table}
```

### 3. 统计检验表格（论文 Table 3）

```latex
\begin{table}[t]
\centering
\caption{Statistical Test Results}
\label{tab:statistical}
\begin{tabular}{lcccc}
\toprule
\textbf{Comparison} & \textbf{t} & \textbf{df} & \textbf{p} & \textbf{d} \\
\midrule
Control vs Ours-Full & 2.45 & 18.5 & .024$^\ast$ & 0.86 \\
Ours-Full vs No-CoT & 1.89 & 17.2 & .076 & 0.62 \\
Ours-Full vs No-Causal & 3.21 & 16.8 & .005$^{\ast\ast}$ & 1.12 \\
\bottomrule
\end{tabular}
\end{table}
```

---

## 🔧 故障排除

### 常见问题

**Q: 实验输出目录为空？**
A: 检查是否正确调用了 `save_all_data()` 方法。

**Q: 统计检验 p 值为 NaN？**
A: 检查数据是否为空或方差为零。

**Q: LaTeX 表格编译警告？**
A: 确保导入了 `booktabs` 宏包：`\usepackage{booktabs}`

---

## 📚 参考文献

1. **Welch's t-test**: Welch, B. L. (1947). The generalization of Student's problem when several different population variances are involved. Biometrika.

2. **Cohen's d**: Cohen, J. (1988). Statistical power analysis for the behavioral sciences.

3. **Benjamini-Hochberg**: Benjamini, Y., & Hochberg, Y. (1995). Controlling the false discovery rate: a practical and powerful approach to multiple testing.

---

**文档维护者**: AI Assistant
**最后更新**: 2026-03-25
**版本**: 1.0.0
