# Architecture Diagrams for Papers

> **用途**: 专业架构图，替换 ASCII art
> **格式**: SVG (可缩放矢量图形)
> **状态**: 🟡 草稿

---

## Paper A: Parallel Context Architecture

### Figure 1: System Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Parallel Context Manager                              │
│                                                                          │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                    Context Graph (graph.json)                     │   │
│  │  - Branch metadata (id, name, parent, state)                      │   │
│  │  - Branch relationships (fork points, merge history)              │   │
│  │  - Hash chain references                                          │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                              │                                           │
│         ┌────────────────────┼────────────────────┐                     │
│         │                    │                    │                      │
│         ▼                    ▼                    ▼                      │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐                │
│  │   Branch    │     │   Branch    │     │   Branch    │                │
│  │    main     │     │  feature-x  │     │  feature-y  │                │
│  │  (Active)   │     │  (Active)   │     │  (Merged)   │                │
│  │             │     │             │     │             │                │
│  │ ┌─────────┐ │     │ ┌─────────┐ │     │ ┌─────────┐ │                │
│  │ │Transient│ │     │ │Transient│ │     │ │Transient│ │                │
│  │ │  (RW)   │ │     │ │  (RW)   │ │     │ │(Abandoned)│              │
│  │ └─────────┘ │     │ └─────────┘ │     │ └─────────┘ │                │
│  │ ┌─────────┐ │     │ ┌─────────┐ │     │ ┌─────────┐ │                │
│  │ │ShortTerm│ │◄────┤ShortTerm│ │     │ │ShortTerm│ │                │
│  │ │ (COW)   │ │     │ │ (COW)   │ │     │ │ (COW)   │ │                │
│  │ └─────────┘ │     │ └─────────┘ │     │ └─────────┘ │                │
│  │ ┌─────────┐ │────►│ ┌─────────┐ │     │ ┌─────────┐ │                │
│  │ │LongTerm │ │     │ │LongTerm │ │     │ │LongTerm │ │                │
│  │ │(Shared) │ │     │ │(Shared) │ │     │ │(Shared) │ │                │
│  │ └─────────┘ │     │ └─────────┘ │     │ └─────────┘ │                │
│  └─────────────┘     └─────────────┘     └─────────────┘                │
│                                                                          │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                    Operation API                                  │   │
│  │  fork() │ checkout() │ merge() │ abort() │ time_travel()          │   │
│  └──────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

### Figure 2: Copy-on-Write Mechanism

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Fork Operation (O(1))                           │
│                                                                          │
│  Before Fork:                          After Fork:                       │
│  ┌─────────────┐                       ┌─────────────┐                  │
│  │   main      │                       │   main      │                  │
│  │             │                       │             │                  │
│  │ ┌─────────┐ │                       │ ┌─────────┐ │                  │
│  │ │ file1   │ │                       │ │ file1   │ │                  │
│  │ │ file2   │ │                       │ │ file2   │ │                  │
│  │ │ file3   │ │                       │ │ file3   │ │                  │
│  │ └─────────┘ │                       │ └─────────┘ │                  │
│  └─────────────┘                       └──────┬──────┘                  │
│                                               │                          │
│                                          symlink                         │
│                                               │                          │
│                                               ▼                          │
│                                        ┌─────────────┐                   │
│                                        │  feature-x  │                   │
│                                        │             │                   │
│                                        │ ┌─────────┐ │                   │
│                                        │ │ file1   │ │ (symlink)         │
│                                        │ │ file2   │ │ (symlink)         │
│                                        │ │ file3   │ │ (symlink)         │
│                                        │ └─────────┘ │                   │
│                                        └─────────────┘                   │
│                                                                          │
│  Write to feature-x/file1:              Result:                          │
│  ┌─────────────┐                       ┌─────────────┐                  │
│  │   main      │                       │   main      │                  │
│  │             │                       │             │                  │
│  │ ┌─────────┐ │                       │ ┌─────────┐ │                  │
│  │ │ file1   │ │◄──── (unchanged)      │ │ file1   │ │                  │
│  │ │ file2   │ │                       │ │ file2   │ │                  │
│  │ │ file3   │ │                       │ │ file3   │ │                  │
│  │ └─────────┘ │                       │ └─────────┘ │                  │
│  └─────────────┘                       └──────┬──────┘                  │
│                                               │                          │
│                                          symlink                         │
│                                               │                          │
│                                               ▼                          │
│                                        ┌─────────────┐                   │
│                                        │  feature-x  │                   │
│                                        │             │                   │
│                                        │ ┌─────────┐ │                   │
│                                        │ │ file1'  │ │ (actual copy)     │
│                                        │ │ file2   │ │ (symlink)         │
│                                        │ │ file3   │ │ (symlink)         │
│                                        │ └─────────┘ │                   │
│                                        └─────────────┘                   │
└─────────────────────────────────────────────────────────────────────────┘
```

### Figure 3: Merge Strategies

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      Five Merge Strategies                               │
│                                                                          │
│  Strategy 1: FastForward          Strategy 2: SelectiveMerge            │
│  ┌──────────┐                     ┌──────────┐                          │
│  │  main ───┼──► feature          │  main    │                          │
│  │          │   (direct child)    │    ╲     │                          │
│  └──────────┘                     │     ╲    │                          │
│       │                           │      ╲   │                          │
│       ▼                           │       ╲  │                          │
│  Move pointer                     │        ╲ │                          │
│  directly                         │         ╲│                          │
│                                   │       feature                       │
│                                   └──────────┘                          │
│                                          │                               │
│                                   Merge selected                         │
│                                   items only                             │
│                                                                          │
│  Strategy 3: AIAssisted           Strategy 4: Manual                     │
│  ┌──────────┐                     ┌──────────┐                          │
│  │  main    │                     │  main    │                          │
│  │    ╲     │                     │    ╲     │                          │
│  │     ╲    │                     │     ╲    │                          │
│  │      ╲   │                     │      ╲   │                          │
│  │       ╲  │                     │       ╲  │                          │
│  │        ╲ │                     │        ╲ │                          │
│  │     [LLM]│                     │    [User]│                          │
│  │        ╱ │                     │        ╱ │                          │
│  │       ╱  │                     │       ╱  │                          │
│  │      ╱   │                     │      ╱   │                          │
│  │     ╱    │                     │     ╱    │                          │
│  │    ╱     │                     │    ╱     │                          │
│  │ feature  │                     │ feature  │                          │
│  └──────────┘                     └──────────┘                          │
│       │                               │                                  │
│  AI resolves                      User resolves                         │
│  conflicts                        all conflicts                         │
│                                                                          │
│  Strategy 5: Ours/Theirs          (Keep target/source version)          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Paper B: HybridGapDetector Architecture

### Figure 4: Two-Stage Detection Pipeline

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    HybridGapDetector Architecture                       │
│                                                                          │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │  Input: Task Execution History                                    │   │
│  │  - Task ID, description, status (success/failure)                 │   │
│  │  - Tools used, execution time, user satisfaction                  │   │
│  │  - Error messages, failure patterns                               │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                              │                                           │
│                              ▼                                           │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │  Stage 1: Statistical Filter ⚡                                   │   │
│  │  ┌────────────────────────────────────────────────────────────┐   │   │
│  │  │  Metrics (per task pattern):                                │   │   │
│  │  │  - Failure Rate: count(failures) / count(total)             │   │   │
│  │  │  - Affected Tasks: count(unique tasks with same pattern)    │   │   │
│  │  │  - Avg Satisfaction: mean(satisfaction_scores)              │   │   │
│  │  └────────────────────────────────────────────────────────────┘   │   │
│  │                              │                                      │   │
│  │                              ▼                                      │   │
│  │  ┌────────────────────────────────────────────────────────────┐   │   │
│  │  │  Filtering Rules:                                           │   │   │
│  │  │  IF failure_rate > 0.30 AND                                 │   │   │
│  │  │     affected_tasks > 5 AND                                  │   │   │
│  │  │     avg_satisfaction < 3.0                                  │   │   │
│  │  │  THEN → Candidate Gap                                       │   │   │
│  │  └────────────────────────────────────────────────────────────┘   │   │
│  │                              │                                      │   │
│  │  Performance: <100ms, 0 API calls                                   │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                              │                                           │
│                              ▼                                           │
│                    Candidate Gaps (N candidates)                         │
│                              │                                           │
│                              ▼                                           │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │  Stage 2: Causal Analysis 🧠                                     │   │
│  │  ┌────────────────────────────────────────────────────────────┐   │   │
│  │  │  For each candidate gap:                                    │   │   │
│  │  │  1. Construct counterfactual prompt                         │   │   │
│  │  │  2. LLM analyzes: "Would task succeed with this tool?"      │   │   │
│  │  │  3. Chain-of-Thought reasoning                              │   │   │
│  │  │  4. Output: causal_factors[], confidence (0-1)              │   │   │
│  │  └────────────────────────────────────────────────────────────┘   │   │
│  │                              │                                      │   │
│  │  Performance: 1-5s, 1-2 API calls per gap                          │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                              │                                           │
│                              ▼                                           │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │  Stage 3: Merger & Prioritize 🔗                                 │   │
│  │  ┌────────────────────────────────────────────────────────────┐   │   │
│  │  │  Hybrid Confidence Calculation:                             │   │   │
│  │  │  hybrid_confidence =                                        │   │   │
│  │  │    statistical_evidence × 0.4 +                             │   │   │
│  │  │    causal_evidence × 0.6                                    │   │   │
│  │  │                                                              │   │   │
│  │  │  Priority Ranking:                                          │   │   │
│  │  │  priority = hybrid_confidence × impact_score × urgency      │   │   │
│  │  └────────────────────────────────────────────────────────────┘   │   │
│  │                              │                                      │   │
│  │  Performance: <50ms, 0 API calls                                    │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                              │                                           │
│                              ▼                                           │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │  Output: Prioritized Tool Gaps                                    │   │
│  │  - Gap ID, type, description                                      │   │
│  │  - Suggested tool name & capabilities                             │   │
│  │  - Hybrid confidence, priority score                              │   │
│  │  - Statistical evidence (failure_rate, affected_tasks, ...)       │   │
│  │  - Causal evidence (counterfactual reasoning, confidence)         │   │
│  └──────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

### Figure 5: Cost-Accuracy Trade-off

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Cost vs Accuracy Comparison                          │
│                                                                          │
│  Accuracy (%)                                                            │
│     │                                                                    │
│  80 │                                    ● Pure Prompt                  │
│     │                                   (75%, $50-150/mo)               │
│     │                                                                    │
│  75 │                               ★ Hybrid                             │
│     │                              (72%, $2.25/mo)                       │
│     │                                                                    │
│  70 │                                                                    │
│     │                                                                    │
│  65 │                                                                    │
│     │                                                                    │
│  60 │  ● Pure Statistical                                                │
│     │ (60%, $0/mo)                                                       │
│  55 │                                                                    │
│     └────────────────────────────────────────────────────────────        │
│       $0      $10      $20      $30      $40      $50      $60     Cost │
│                                                                          │
│  ★ = Sweet spot: 95% cost reduction, only 3% accuracy loss              │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Usage Instructions

### For LaTeX (ACL/AAAI format)

```latex
% In preamble
\usepackage{graphicx}
\usepackage{tikz}
\usetikzlibrary{positioning, arrows, shapes}

% In document
\begin{figure}[t]
    \centering
    \includegraphics[width=\linewidth]{figures/architecture.pdf}
    \caption{Parallel Context Architecture Overview}
    \label{fig:architecture}
\end{figure}
```

### For Markdown

```markdown
![Architecture Overview](figures/architecture.svg)
*Figure 1: Parallel Context Architecture Overview*
```

---

**Next Steps**:
1. Convert ASCII diagrams to professional SVG/PDF using tools like:
   - Draw.io (free, export to SVG/PDF)
   - Excalidraw (hand-drawn style)
   - TikZ (LaTeX native, steep learning curve)
   - Mermaid (simple, Markdown-compatible)

2. Ensure consistency:
   - Same color scheme across both papers
   - Same font family (Arial/Helvetica for sans-serif)
   - Consistent icon/shape usage

3. Accessibility:
   - Add alt text for screen readers
   - Ensure color-blind friendly palette
   - Provide text descriptions for key figures

---

**Status**: 🟡 Draft - Ready for conversion to SVG/PDF
