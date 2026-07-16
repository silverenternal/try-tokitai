# Atlas Experiment Framework

> **Purpose**: Validate the effectiveness of the Prompt Engineering self-evolution system through controlled experiments
> 
> **Status**: ✅ Framework implemented, ready for data collection
> 
> **Last Updated**: 2026-03-27

---

## 🎯 Overview

The experiment framework provides infrastructure for running controlled experiments to validate the self-evolution system's effectiveness. It supports:

- **5 experiment groups** (Control + 4 ablation variants)
- **110 benchmark tasks** across 7 categories
- **Automated data collection** and logging
- **Statistical analysis** tools

---

## 📁 Directory Structure

```
experiments/
├── README.md                 # This file
├── tasks/                    # Benchmark task definitions
│   └── benchmark_tasks.json  # 110 tasks (✅ Complete)
├── logs/                     # Experiment logs (created at runtime)
│   ├── control/              # Control group logs
│   ├── ours_full/            # Full system logs
│   ├── ours_single/          # Single-agent logs
│   ├── ours_nocot/           # No-CoT logs
│   └── ours_nofix/           # No-self-fix logs
├── analysis/                 # Analysis results
│   ├── comparison_results.json
│   ├── ablation_results.json
│   └── visualizations/
└── scripts/                  # Analysis scripts
    ├── run_benchmark.py      # Python benchmark runner
    ├── analyze_results.py    # Statistical analysis
    └── generate_charts.py    # Visualization
```

---

## 🔬 Experiment Design

### Experiment Groups

| Group | Description | Purpose |
|-------|-------------|---------|
| **Control** | Original tokitai (no self-evolution) | Baseline performance |
| **Ours-Full** | Complete Prompt Engineering system | Validate overall effectiveness |
| **Ours-Single** | Single LLM decision (no multi-agent) | Validate multi-agent value |
| **Ours-NoCoT** | Without Chain-of-Thought reasoning | Validate CoT value |
| **Ours-NoFix** | Without self-correction loop | Validate self-fix value |

### Task Categories

| Category | Count | Difficulty Distribution |
|----------|-------|------------------------|
| File Operations | 20 | Easy 50% / Medium 40% / Hard 10% |
| Code Analysis | 20 | Easy 40% / Medium 50% / Hard 10% |
| Network Requests | 15 | Easy 60% / Medium 30% / Hard 10% |
| Git Operations | 15 | Easy 50% / Medium 40% / Hard 10% |
| Data Processing | 15 | Easy 40% / Medium 50% / Hard 10% |
| System Monitor | 10 | Easy 70% / Medium 30% |
| Composite Tasks | 15 | Medium 50% / Hard 50% |

---

## 🚀 Quick Start

### Running Experiments (Rust)

```bash
# Run single group benchmark
cargo run --release -- experiment run --group Ours-Full --days 1

# Run all comparison groups
cargo run --release -- experiment run --all-groups

# Run ablation study
cargo run --release -- experiment run --ablation

# Analyze results
cargo run --release -- experiment analyze
```

### Running Experiments (Python)

```bash
# Run single group
python experiments/scripts/run_benchmark.py --group Ours-Full --days 30

# Run all groups
python experiments/scripts/run_benchmark.py --all-groups

# Analyze results
python experiments/scripts/analyze_results.py

# Generate charts
python experiments/scripts/generate_charts.py
```

---

## 📊 Evaluation Metrics

### Primary Metrics

| Metric | Definition | Expected Improvement |
|--------|------------|---------------------|
| **Task Success Rate** | Successful tasks / Total tasks | +15-20% |
| **Average Tool Calls** | Tool calls per task | -30% |
| **User Satisfaction** | 1-5 rating | +0.5-1.0 |

### Secondary Metrics

| Metric | Definition | Target |
|--------|------------|--------|
| **Gap Detection Precision** | Correct gaps / Total detected | >75% |
| **Tool Creation Success** | Compiled tools / Total created | >80% |
| **Tool Utilization** | Active tools / Total tools | +20-30% |
| **Tool Failure Rate** | Failed calls / Total calls | -50% |

### Cost Metrics

| Metric | Definition | Target |
|--------|------------|--------|
| **API Cost/Month** | USD | <$50 |
| **Generation Time** | Seconds per tool | <30s |
| **Correction Cycles** | Attempts to compile | 1-2 |

---

## 📝 Log Format

### Task Execution Log

```json
{
  "task_id": "file_001",
  "category": "file_ops",
  "difficulty": "medium",
  "description": "Batch rename .txt files to .md",
  "timestamp": "2026-03-27T10:30:00Z",
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

### Evolution Cycle Log

```json
{
  "cycle_id": "cycle_001",
  "timestamp": "2026-03-27T00:00:00Z",
  "group": "Ours-Full",
  "reflection": {
    "coverage_score": 0.75,
    "systemic_issues": ["Missing batch file processing"],
    "strategic_recommendations": ["Prioritize file batch tools"]
  },
  "gaps_detected": [
    {
      "gap_type": "missing_tool",
      "description": "Missing batch rename tool",
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

## 🔧 Implementation Details

### Core Components

1. **ExperimentRunner** (`src/experiments/runner.rs`)
   - Executes benchmark tasks
   - Manages experiment groups
   - Collects execution logs

2. **DataCollector** (`src/experiments/collector.rs`)
   - Records task executions
   - Saves evolution cycles
   - Exports metrics to JSON

3. **HybridGapDetector** (`src/autonomy/hybrid_gap_detector.rs`)
   - Detects tool gaps statistically
   - Optional causal analysis with LLM
   - Tracks evolution metrics

4. **Benchmark Tasks** (`src/experiments/benchmark_tasks.rs`)
   - Loads 110 tasks from JSON
   - Categorizes by type/difficulty
   - Provides task metadata

### CLI Commands

```
cargo run --release -- experiment <command> [options]

Commands:
  run       Run benchmark experiments
  analyze   Analyze existing results
  help      Show help message

Options:
  --group, -g      Experiment group (control, ours-full, etc.)
  --days, -d       Experiment duration
  --all-groups, -a Run all comparison groups
  --ablation       Run ablation study
  --project-path   Project root directory
```

---

## 📈 Analysis Workflow

### 1. Run Experiments

```bash
# Run all groups (takes ~4-6 hours for 1 day experiment)
cargo run --release -- experiment run --all-groups
```

### 2. Collect Logs

Logs are automatically saved to:
```
experiments/logs/<group>/task_logs_<timestamp>.jsonl
experiments/logs/<group>/evolution_logs_<timestamp>.jsonl
```

### 3. Analyze Results

```bash
# Built-in analysis
cargo run --release -- experiment analyze

# Python analysis (more detailed)
python experiments/scripts/analyze_results.py
```

### 4. Generate Visualizations

```bash
# Generate charts (requires matplotlib)
python experiments/scripts/generate_charts.py --all
```

### 5. Export Results

Results are saved to:
```
experiments/analysis/comparison_results.json
experiments/analysis/comparison_report.md
experiments/analysis/visualizations/
```

---

## 🎓 Academic Use

### For Paper Submission

1. **Run 30-day experiments** for each group
2. **Collect daily metrics** (success rate, tool calls, satisfaction)
3. **Perform statistical tests** (t-test, ANOVA)
4. **Generate visualizations** (learning curves, box plots)
5. **Document qualitative cases** (successful tool creations)

### Statistical Methods

- **t-test**: Compare two groups (e.g., Control vs Ours-Full)
- **ANOVA**: Compare multiple groups (all 5 groups)
- **Effect Size (Cohen's d)**: Measure practical significance
- **Learning Curves**: Show improvement over time

### Expected Results (Hypotheses)

- **H1**: Ours-Full has higher success rate than Control (+15-20%)
- **H2**: Ours-Full uses fewer tool calls than Control (-30%)
- **H3**: Multi-agent negotiation improves decisions (Ours-Single ablation)
- **H4**: Chain-of-Thought improves reasoning (Ours-NoCoT ablation)
- **H5**: Self-correction improves reliability (Ours-NoFix ablation)

---

## ⚠️ Important Notes

### Before Running

- [ ] Ensure API budget is sufficient ($50-150 for 30-day experiment)
- [ ] Backup existing experiment data
- [ ] Verify benchmark tasks are loaded (110 tasks)
- [ ] Check disk space for logs (~100MB/day)

### During Experiments

- [ ] Monitor log file sizes
- [ ] Check for errors daily
- [ ] Backup logs weekly
- [ ] Track API costs

### After Experiments

- [ ] Validate data completeness
- [ ] Clean outliers if needed
- [ ] Document anomalies
- [ ] Archive raw data

---

## 🐛 Troubleshooting

### Common Issues

**Issue**: "No benchmark tasks found"
- **Solution**: Check `experiments/tasks/benchmark_tasks.json` exists

**Issue**: "Gap detector initialization failed"
- **Solution**: Ensure `.tokitai/evolution` directory is writable

**Issue**: "Out of memory"
- **Solution**: Reduce `--days` parameter or increase system memory

**Issue**: "API rate limit exceeded"
- **Solution**: Reduce `max_causal_analyses_per_cycle` in config

---

## 📚 Related Documentation

- [Innovation Points](../../docs/INNOVATIONS.md)
- [Gap Analysis](../../docs/README_VS_IMPLEMENTATION_GAP_ANALYSIS.md)
- [Paper Splitting Plan](../../docs/PAPER_SPLITTING_PLAN.md)
- [Hybrid Gap Detector](../../docs/HYBRID_GAP_DETECTOR.md)

---

**Last Updated**: 2026-03-27  
**Maintainer**: Tokitai Team  
**Status**: ✅ Ready for Data Collection
