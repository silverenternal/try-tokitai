# 统一术语表 (Unified Glossary)

> **用途**: 确保所有论文、文档、代码使用一致的术语
> **最后更新**: 2026-03-27
> **状态**: 🟡 草稿 - 待审阅

---

## 核心概念 (Core Concepts)

### AI Agent / 智能体

**定义**: 基于大语言模型 (LLM) 的自主系统，能够感知环境、做出决策、执行动作以完成目标。

**同义词避免**: 不要使用 "AI 助手"、"LLM Agent"、"智能助手" 混用

**英文**: AI Agent (首选), Intelligent Agent (正式), Language Agent (特定场景)

---

### Context / 上下文

**定义**: AI Agent 在任务执行过程中积累的信息，包括对话历史、工具调用记录、任务进度、环境状态等。

**相关术语**:
- **Context Management**: 上下文管理
- **Context Branch**: 上下文分支
- **Linear Context**: 线性上下文 (单一对话线程)
- **Parallel Context**: 平行上下文 (支持多分支)

**避免混用**: 不要与 "Memory"、"History"、"Session" 混用

---

### Tool / 工具

**定义**: AI Agent 可调用的功能模块，通常实现为 Rust trait 或函数。

**相关术语**:
- **Tool Registry**: 工具注册表
- **Tool Definition**: 工具定义 (包含名称、描述、输入输出 schema)
- **Tool Invocation**: 工具调用
- **Tool Gap**: 工具缺口 (缺少完成某任务所需的工具)

**避免混用**: 不要与 "Function"、"API"、"Service" 混用 (除非特指)

---

### Branch / 分支

**定义**: 上下文的一个独立版本，支持平行探索不同的解决方案路径。

**操作原语**:
- **fork**: 创建新分支
- **checkout**: 切换到指定分支
- **merge**: 合并两个分支
- **abort**: 废弃分支

**相关术语**:
- **Main Branch**: 主分支 (默认分支)
- **Feature Branch**: 功能分支 (为特定任务创建)
- **Branch State**: 分支状态 (Active/Merged/Abandoned/Conflicted)

**避免混用**: 不要与 Git 的 branch 概念完全等同 (虽有相似，但语义不同)

---

### Merge / 合并

**定义**: 将两个分支的上下文内容整合到一个分支的操作。

**合并策略**:
- **FastForward**: 快进合并 (源分支是目标分支的直接后代)
- **SelectiveMerge**: 选择性合并 (基于重要性评分)
- **AIAssisted**: AI 辅助合并 (LLM 解决语义冲突)
- **Manual**: 手动合并 (用户解决所有冲突)
- **Ours**: 保留目标分支版本
- **Theirs**: 保留源分支版本

**相关术语**:
- **Conflict**: 冲突 (两个分支对同一上下文项有不同修改)
- **Conflict Resolution**: 冲突解决
- **Merge Result**: 合并结果

---

### Copy-on-Write (COW) / 写时复制

**定义**: 一种优化技术，分支创建时不立即复制数据，而是在写入时才复制。

**实现方式**: 使用文件系统 symlink 实现 O(1) 分支创建

**相关术语**:
- **Symlink**: 符号链接 (Linux/macOS)
- **Junction Point**: 联接点 (Windows)
- **Storage Overhead**: 存储开销

---

### Self-Evolution / 自进化

**定义**: AI Agent 系统自主改进工具生态系统的能力。

**核心机制**:
- **Gap Detection**: 缺口检测 (发现缺少的工具)
- **Tool Creation**: 工具创建 (自动生成新工具代码)
- **Tool Optimization**: 工具优化 (改进现有工具)
- **System Reflection**: 系统反思 (定期生成"体检报告")

**相关术语**:
- **HybridGapDetector**: 混合缺口检测器 (统计 + 因果融合)
- **Prompt Engineering**: 提示工程 (无需训练的方法)
- **Statistical Filter**: 统计过滤器 (快速筛选候选缺口)
- **Causal Analysis**: 因果分析 (深度验证缺口)

---

### Prompt Engineering / 提示工程

**定义**: 通过精心设计的提示词 (Prompt) 激发 LLM 已有能力，无需额外训练。

**相关技术**:
- **Chain-of-Thought (CoT)**: 思维链 (要求 LLM 展示推理步骤)
- **Few-Shot Learning**: 少样本学习 (提供示例引导输出)
- **Self-Correction**: 自修正 (基于反馈迭代改进)
- **JSON Schema Constraint**: JSON Schema 约束 (确保输出格式)

**避免混用**: 不要与 "Fine-tuning"、"Training" 混用

---

### Three-Layer Storage / 三层存储

**定义**: 上下文的分层存储架构。

**层次**:
- **Transient Layer**: 瞬态层 (单轮对话临时数据，不清理)
- **Short-Term Layer**: 短期层 (最近 N 轮对话，频繁访问)
- **Long-Term Layer**: 长期层 (项目规则、工具配置等，跨分支共享)

---

### Tool Matrix / 工具矩阵

**定义**: 支持自进化的工具管理基础设施。

**核心组件**:
- **Service-Oriented Metadata**: 服务化元数据 (QoS、依赖、健康状态)
- **Skills Files**: AI 可读的工具说明书
- **ToolBox as Service Boundary**: 工具箱即服务边界
- **Automatic Dependency Inference**: 依赖图自动推断

---

## 性能指标 (Performance Metrics)

### 延迟指标

| 术语 | 定义 | 目标值 |
|------|------|--------|
| **Fork Latency** | 创建分支的平均时间 | <10ms |
| **Merge Latency** | 合并分支的平均时间 (不含 AI) | <100ms |
| **Checkout Latency** | 切换分支的平均时间 | <5ms |
| **Detection Latency** | 缺口检测的平均时间 | 1-5s (Hybrid) |

### 准确率指标

| 术语 | 定义 | 目标值 |
|------|------|--------|
| **Task Success Rate** | 成功完成任务的比例 | 75%+ |
| **Gap Detection Accuracy** | 缺口检测的准确率 | 72%+ |
| **AI Resolution Accuracy** | AI 冲突解决的准确率 | 85%+ |
| **Tool Compilation Rate** | 生成工具代码的编译通过率 | 80%+ |

### 成本指标

| 术语 | 定义 | 目标值 |
|------|------|--------|
| **Storage Overhead** | 多分支的存储开销 | <20% (10 branches) |
| **Memory Overhead** | 状态管理的内存开销 | <15% |
| **API Cost/Month** | 每月 API 调用成本 | <$50 (Hybrid) |

---

## 实验术语 (Experimental Terms)

### 数据标注

| 标记 | 含义 | 说明 |
|------|------|------|
| 🟢 **Preliminary** | 实测数据 | 已完成的小规模预实验数据 |
| 🟡 **Expected** | 预期数据 | 基于初步测量和理论分析的预期值，待完整实验验证 |
| 🔵 **Target** | 目标指标 | 系统设计目标 |

### 实验组别

| 组别 | 说明 |
|------|------|
| **Control** | 原始系统 (无自进化/无线性上下文) |
| **Ours-Full** | 完整系统 |
| **Ours-Single** | 单 LLM (无多智能体协商) |
| **Ours-NoCoT** | 移除 Chain-of-Thought |
| **Ours-NoFix** | 移除自修正循环 |
| **Statistical-Only** | 纯统计方法 (验证因果分析必要性) |
| **Causal-Only** | 纯因果方法 (验证统计过滤器成本优化价值) |

---

## 代码术语 (Code Terminology)

### Rust 模块命名

| 模块 | 文件路径 | 说明 |
|------|----------|------|
| `context` | `src/context/` | 上下文管理核心模块 |
| `autonomy` | `src/autonomy/` | 自进化功能模块 |
| `prompt_engineering` | `src/prompt_engineering/` | 提示工程模块 |
| `tool_market` | `src/tool_market/` | 工具市场模块 |
| `tool_matrix` | `src/tool_matrix/` | 工具矩阵模块 |

### 关键结构体

| 结构体 | 用途 |
|--------|------|
| `ContextBranch` | 表示一个上下文分支 |
| `ParallelContextManager` | 平行上下文管理器 |
| `HybridGapDetector` | 混合缺口检测器 |
| `PromptGapDetector` | 基于 Prompt 的缺口检测器 |
| `PromptCreator` | 基于 Prompt 的工具创建器 |
| `MultiAgentNegotiator` | 多智能体协商器 |

---

## 写作规范 (Writing Guidelines)

### 论文中首次出现

格式：**英文术语** (中文翻译)

示例: "We present **Parallel Context Architecture** (平行上下文架构), the first system to..."

### 后续引用

- 英文论文：使用英文术语
- 中文文档：使用中文翻译，括号标注英文

### 避免的写法

❌ "我们的 branch 功能" (中英混用)
✅ "我们的分支 (branch) 功能" 或 "我们的 branch 功能 (分支)"

---

## 变更日志

| 日期 | 版本 | 变更内容 |
|------|------|----------|
| 2026-03-27 | 1.0 | 初始版本 |

---

**维护者**: Tokitai Development Team
**审阅状态**: 🟡 待审阅 (下次更新：2026-04-03)
