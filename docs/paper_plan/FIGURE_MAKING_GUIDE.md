# 论文图表制作指南

> **目的**: 为论文 A 和论文 B 制作高质量的图表
> **工具**: TikZ (架构图), Python/matplotlib (统计图表)
> **风格**: ACL/AAAI 会议标准 (黑白可打印，彩色电子版)

---

## 📊 论文 A 图表清单

### Figure 1: Parallel Context Architecture 总览

**类型**: 架构图 (TikZ)
**尺寸**: 双栏宽度 (480pt)
**内容**:
- 三层存储架构 (transient/short-term/long-term)
- ContextGraph 管理
- Branch 结构
- Merge 流程

**草图**:
```
┌─────────────────────────────────────────────────────────────┐
│              Parallel Context Architecture                   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐   │
│  │  main       │     │  feature-x  │     │  bugfix-1   │   │
│  │  Branch     │────▶│  Branch     │     │  Branch     │   │
│  │             │     │             │     │             │   │
│  │ ┌─────────┐ │     │ ┌─────────┐ │     │ ┌─────────┐ │   │
│  │ │Transient│ │     │ │Transient│ │     │ │Transient│ │   │
│  │ ├─────────┤ │     │ ├─────────┤ │     │ ├─────────┤ │   │
│  │ │Short    │ │     │ │Short    │ │     │ │Short    │ │   │
│  │ ├─────────┤ │     │ ├─────────┤ │     │ ├─────────┤ │   │
│  │ │Long     │ │     │ │Long     │ │     │ │Long     │ │   │
│  │ └─────────┘ │     │ └─────────┘ │     │ └─────────┘ │   │
│  └─────────────┘     └─────────────┘     └─────────────┘   │
│         │                   │                   │           │
│         └───────────────────┼───────────────────┘           │
│                             ▼                               │
│                  ┌──────────────────┐                       │
│                  │  ContextGraph    │                       │
│                  │  - Merge History │                       │
│                  │  - Branch Points │                       │
│                  │  - Conflicts     │                       │
│                  └──────────────────┘                       │
└─────────────────────────────────────────────────────────────┘
```

**TikZ 代码框架**:
```latex
\begin{figure*}[t]
\centering
\begin{tikzpicture}[
    branch/.style={rectangle, draw, rounded corners, minimum width=3cm, minimum height=4cm},
    layer/.style={rectangle, draw, minimum width=2.5cm, minimum height=0.8cm},
    graph/.style={rectangle, draw, rounded corners, minimum width=4cm, minimum height=2cm}
]
% Branches
\node[branch] (main) at (0,0) {main Branch};
\node[branch] (feature) at (5,0) {feature-x Branch};
\node[branch] (bugfix) at (10,0) {bugfix-1 Branch};

% Layers in main branch
\node[layer] at (0,1) {Transient};
\node[layer] at (0,0) {Short-term};
\node[layer] at (0,-1) {Long-term};

% ContextGraph
\node[graph] (graph) at (5,-3) {ContextGraph};

% Arrows
\draw[->] (main) -- (graph);
\draw[->] (feature) -- (graph);
\draw[->] (bugfix) -- (graph);
\end{tikzpicture}
\caption{Parallel Context Architecture Overview. Each branch maintains independent three-layer storage, managed by ContextGraph.}
\label{fig:architecture}
\end{figure*}
```

**负责人**: @Designer
**截止日期**: 2026-06-30
**状态**: ⏳ 待制作

---

### Figure 2: Fork/Checkout/Merge/Abort 流程图

**类型**: 流程图 (TikZ)
**尺寸**: 单栏宽度 (240pt)
**内容**: 4 个核心原语的操作流程

**草图**:
```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│    fork()    │     │  checkout()  │     │    merge()   │     │    abort()   │
│              │     │              │     │              │     │              │
│ 1. Create    │     │ 1. Validate  │     │ 1. Detect    │     │ 1. Validate  │
│    symlink   │     │    branch    │     │    conflicts │     │    branch    │
│              │     │              │     │              │     │              │
│ 2. Copy      │     │ 2. Update    │     │ 2. Resolve   │     │ 2. Remove    │
│    metadata  │     │    pointer   │     │    (AI)      │     │    symlink   │
│              │     │              │     │              │     │              │
│ 3. Return    │     │ 3. Return    │     │ 3. Update    │     │ 3. Cleanup   │
│    O(1)      │     │    O(1)      │     │    graph     │     │    O(n)      │
└──────────────┘     └──────────────┘     └──────────────┘     └──────────────┘
```

**负责人**: @AI Assistant
**截止日期**: 2026-06-30
**状态**: ⏳ 待制作

---

### Figure 3: COW Performance Comparison

**类型**: 柱状图 (Python/matplotlib)
**尺寸**: 单栏宽度 (240pt)
**数据**: COW vs Full Copy 的 fork 延迟对比

**预期数据**:
```json
{
  "method": ["COW (Ours)", "Full Copy"],
  "latency_ms": [6.2, 250],
  "std_dev": [0.8, 35]
}
```

**Python 代码**:
```python
import matplotlib.pyplot as plt
import numpy as np

methods = ['COW (Ours)', 'Full Copy']
latency = [6.2, 250]
std_dev = [0.8, 35]

fig, ax = plt.subplots(figsize=(6, 4))
bars = ax.bar(methods, latency, yerr=std_dev, capsize=5, color=['#2E86AB', '#A23B72'])

ax.set_ylabel('Latency (ms)')
ax.set_title('Copy-on-Write vs Full Copy: Fork Performance')
ax.grid(axis='y', alpha=0.3)

plt.tight_layout()
plt.savefig('cow_performance.pdf', dpi=300)
```

**负责人**: @AI Assistant
**截止日期**: 2026-04-30
**状态**: ⏳ 待数据

---

### Figure 4: Task Success Rate Comparison

**类型**: 分组柱状图 (Python/matplotlib)
**尺寸**: 双栏宽度 (480pt)
**数据**: Control vs Ours-Full 的任务成功率对比

**预期数据**:
```json
{
  "categories": ["File Ops", "Code Analysis", "Network", "Git", "Data", "Composite"],
  "control": [0.55, 0.50, 0.60, 0.55, 0.50, 0.45],
  "ours_full": [0.75, 0.70, 0.80, 0.75, 0.70, 0.65]
}
```

**Python 代码**:
```python
import matplotlib.pyplot as plt
import numpy as np

categories = ['File Ops', 'Code Analysis', 'Network', 'Git', 'Data', 'Composite']
control = [0.55, 0.50, 0.60, 0.55, 0.50, 0.45]
ours_full = [0.75, 0.70, 0.80, 0.75, 0.70, 0.65]

x = np.arange(len(categories))
width = 0.35

fig, ax = plt.subplots(figsize=(12, 5))
bars1 = ax.bar(x - width/2, control, width, label='Control (Linear)', color='#A23B72')
bars2 = ax.bar(x + width/2, ours_full, width, label='Ours-Full (Parallel)', color='#2E86AB')

ax.set_ylabel('Success Rate')
ax.set_title('Task Success Rate by Category')
ax.set_xticks(x)
ax.set_xticklabels(categories)
ax.legend()
ax.grid(axis='y', alpha=0.3)

plt.tight_layout()
plt.savefig('task_success_rate.pdf', dpi=300)
```

**负责人**: @AI Assistant
**截止日期**: 2026-05-31
**状态**: ⏳ 待数据

---

### Figure 5: Operation Latency Distribution

**类型**: 箱线图 (Python/matplotlib)
**尺寸**: 单栏宽度 (240pt)
**数据**: Fork/Checkout/Merge/TimeTravel 的延迟分布

**预期数据**:
```json
{
  "operations": ["Fork", "Checkout", "Merge (Simple)", "Merge (Data)", "Time Travel"],
  "median_ms": [6.0, 2.0, 23, 45, 12],
  "q1_ms": [5.5, 1.8, 20, 40, 10],
  "q3_ms": [6.8, 2.3, 26, 50, 14],
  "whisker_min_ms": [5.1, 1.5, 18, 35, 9],
  "whisker_max_ms": [7.5, 2.8, 30, 60, 16]
}
```

**负责人**: @AI Assistant
**截止日期**: 2026-04-30
**状态**: ⏳ 待数据

---

### Figure 6: Storage Overhead vs Branch Count

**类型**: 折线图 (Python/matplotlib)
**尺寸**: 单栏宽度 (240pt)
**数据**: 分支数 vs 存储开销

**预期数据**:
```json
{
  "branch_counts": [1, 5, 10, 20],
  "overhead_percent": [0, 8, 18, 35],
  "std_dev": [0, 1.2, 2.1, 3.5]
}
```

**Python 代码**:
```python
import matplotlib.pyplot as plt

branch_counts = [1, 5, 10, 20]
overhead = [0, 8, 18, 35]
std_dev = [0, 1.2, 2.1, 3.5]

fig, ax = plt.subplots(figsize=(6, 4))
ax.plot(branch_counts, overhead, marker='o', linewidth=2, markersize=8, color='#2E86AB')
ax.fill_between(branch_counts, 
                [o - s for o, s in zip(overhead, std_dev)],
                [o + s for o, s in zip(overhead, std_dev)],
                alpha=0.3, color='#2E86AB')

ax.set_xlabel('Number of Branches')
ax.set_ylabel('Storage Overhead (%)')
ax.set_title('Storage Overhead vs Branch Count')
ax.grid(alpha=0.3)

plt.tight_layout()
plt.savefig('storage_overhead.pdf', dpi=300)
```

**负责人**: @AI Assistant
**截止日期**: 2026-04-30
**状态**: ⏳ 待数据

---

### Figure 7: User Study Satisfaction

**类型**: 热力图 (Python/matplotlib)
**尺寸**: 双栏宽度 (480pt)
**数据**: N=12 参与者的各项满意度评分

**预期数据** (12 参与者 × 5 指标):
```json
{
  "participants": ["P01", "P02", ..., "P12"],
  "metrics": ["Branch Usefulness", "Merge Usefulness", "Time Travel", "Overall", "Recommend"],
  "scores": [[5,5,4,5,5], [4,5,5,5,5], ...]  # 12×5 matrix
}
```

**负责人**: @AI Assistant
**截止日期**: 2026-05-31
**状态**: ⏳ 待数据

---

## 📊 论文 B 图表清单

### Figure 1: HybridGapDetector Architecture

**类型**: 架构图 (TikZ)
**尺寸**: 双栏宽度 (480pt)
**内容**: 三阶段流水线架构

**TikZ 代码框架**:
```latex
\begin{figure*}[t]
\centering
\begin{tikzpicture}[
    stage/.style={rectangle, draw, rounded corners, minimum width=10cm, minimum height=2cm},
    arrow/.style={->, thick, >=stealth}
]
% Stage 1
\node[stage, fill=blue!10] (stage1) at (0,4) {
    \textbf{Stage 1: Statistical Filter} \\
    \small Failure Rate, Satisfaction, Affected Tasks \\
    \small $<$100ms, 0 API calls
};

% Stage 2
\node[stage, fill=green!10] (stage2) at (0,1) {
    \textbf{Stage 2: Causal Analysis} \\
    \small Counterfactual Reasoning, Chain-of-Thought \\
    \small 5-30s, 1-2 API calls
};

% Stage 3
\node[stage, fill=orange!10] (stage3) at (0,-2) {
    \textbf{Stage 3: Merger \& Prioritize} \\
    \small Hybrid Confidence = Statistical$\times$0.4 + Causal$\times$0.6 \\
    \small $<$50ms, 0 API calls
};

% Arrows
\draw[arrow] (stage1) -- (stage2);
\draw[arrow] (stage2) -- (stage3);
\end{tikzpicture}
\caption{HybridGapDetector Architecture. Three-stage pipeline fusing statistical and causal evidence.}
\label{fig:hybrid_arch}
\end{figure*}
```

**负责人**: @Designer
**截止日期**: 2026-06-30
**状态**: ⏳ 待制作

---

### Figure 2: Cost Comparison

**类型**: 柱状图 (Python/matplotlib)
**尺寸**: 单栏宽度 (240pt)
**数据**: 纯统计 vs 纯 Prompt vs Hybrid 的成本对比

**预期数据**:
```json
{
  "methods": ["Statistical", "Prompt Engineering", "Hybrid (Ours)"],
  "cost_usd": [0, 45, 2.25],
  "savings_percent": [100, 0, 95]
}
```

**负责人**: @AI Assistant
**截止日期**: 2026-06-30
**状态**: ⏳ 待数据

---

### Figure 3: Latency Comparison

**类型**: 柱状图 (Python/matplotlib)
**尺寸**: 单栏宽度 (240pt)
**数据**: 各方法的检测延迟对比

**预期数据**:
```json
{
  "methods": ["Statistical", "Prompt Engineering", "Hybrid (Ours)"],
  "latency_sec": [0.1, 15, 2],
  "improvement_percent": [99, 0, 87]
}
```

**负责人**: @AI Assistant
**截止日期**: 2026-05-31
**状态**: ⏳ 待数据

---

### Figure 4: Accuracy Comparison

**类型**: 分组柱状图 (Python/matplotlib)
**尺寸**: 单栏宽度 (240pt)
**数据**: Precision/Recall/F1对比

**预期数据**:
```json
{
  "methods": ["Statistical", "Prompt Engineering", "Hybrid (Ours)"],
  "precision": [0.55, 0.72, 0.70],
  "recall": [0.65, 0.78, 0.74],
  "f1": [0.60, 0.75, 0.72]
}
```

**负责人**: @AI Assistant
**截止日期**: 2026-06-30
**状态**: ⏳ 待数据

---

### Figure 5: 30-Day Tool Evolution

**类型**: 多轴折线图 (Python/matplotlib)
**尺寸**: 双栏宽度 (480pt)
**数据**: 30 天内工具库演化

**预期数据**:
```json
{
  "days": [1, 5, 10, 15, 20, 25, 30],
  "total_tools": [63, 65, 67, 69, 71, 73, 75],
  "failure_rate": [0.25, 0.22, 0.19, 0.16, 0.14, 0.13, 0.12],
  "success_rate": [0.65, 0.67, 0.70, 0.73, 0.76, 0.78, 0.80]
}
```

**负责人**: @AI Assistant
**截止日期**: 2026-06-30
**状态**: ⏳ 待数据

---

## 🎨 图表风格指南

### 颜色方案

**可打印黑白友好**:
```python
# 主色调
BLUE = '#2E86AB'      # 我们的方法
PINK = '#A23B72'      # 基线/对照
ORANGE = '#F18F01'    # 强调/高亮
GREEN = '#2ECC71'     # 正面指标
RED = '#E74C3C'       # 负面指标

# 渐变色 (用于热力图)
from matplotlib.cm import Blues, Reds, Oranges
```

### 字体设置

```python
import matplotlib.pyplot as plt

plt.rcParams.update({
    'font.size': 10,
    'font.family': 'serif',
    'font.serif': ['Times New Roman'],
    'axes.labelsize': 10,
    'axes.titlesize': 12,
    'xtick.labelsize': 9,
    'ytick.labelsize': 9,
    'legend.fontsize': 9,
    'figure.titlesize': 12
})
```

### 尺寸规范

| 类型 | 宽度 | 高度 | 说明 |
|------|------|------|------|
| 单栏图 | 240pt (8cm) | 120-180pt | 简单对比、分布 |
| 双栏图 | 480pt (16cm) | 180-240pt | 架构图、复杂对比 |
| 全页图 | 480pt (16cm) | 600pt (20cm) | 大型架构图 |

---

## 📁 文件组织

```
docs/paper_plan/figures/
├── paper_a/
│   ├── fig1_architecture.tex       # TikZ 源码
│   ├── fig1_architecture.pdf       # 编译后 PDF
│   ├── fig2_operations.tex
│   ├── fig2_operations.pdf
│   ├── fig3_cow_performance.py     # Python 脚本
│   ├── fig3_cow_performance.pdf
│   ├── fig4_task_success.py
│   ├── fig4_task_success.pdf
│   ├── fig5_latency_boxplot.py
│   ├── fig5_latency_boxplot.pdf
│   ├── fig6_storage_overhead.py
│   ├── fig6_storage_overhead.pdf
│   ├── fig7_user_study.py
│   └── fig7_user_study.pdf
└── paper_b/
    ├── fig1_hybrid_arch.tex
    ├── fig1_hybrid_arch.pdf
    ├── fig2_cost_comparison.py
    ├── fig2_cost_comparison.pdf
    ├── fig3_latency_comparison.py
    ├── fig3_latency_comparison.pdf
    ├── fig4_accuracy_comparison.py
    ├── fig4_accuracy_comparison.pdf
    ├── fig5_30day_evolution.py
    └── fig5_30day_evolution.pdf
```

---

## ✅ 检查清单

### 论文 A

- [ ] Figure 1: 架构图 (TikZ)
- [ ] Figure 2: 操作流程图 (TikZ)
- [ ] Figure 3: COW 性能对比 (柱状图)
- [ ] Figure 4: 任务成功率 (分组柱状图)
- [ ] Figure 5: 操作延迟分布 (箱线图)
- [ ] Figure 6: 存储开销 (折线图)
- [ ] Figure 7: 用户满意度 (热力图)

### 论文 B

- [ ] Figure 1: Hybrid 架构图 (TikZ)
- [ ] Figure 2: 成本对比 (柱状图)
- [ ] Figure 3: 延迟对比 (柱状图)
- [ ] Figure 4: 准确率对比 (分组柱状图)
- [ ] Figure 5: 30 天演化 (多轴折线图)

---

**指南创建时间**: 2026-03-27
**下次更新**: 2026-04-30 (首批图表完成后)
**负责人**: Tokitai Development Team
