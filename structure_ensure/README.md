# structure_ensure - 项目结构文档

> **版本**: 3.1 (HybridGapDetector 实现完成 + Prompt Engineering 自进化系统)
> **最后更新**: 2026-03-20
> **目标会议**: AAAI 2027 / ACL 2027 / EMNLP 2027
> **实施方法**: Prompt Engineering（无需训练）
> **测试状态**: 470/470 通过 ✅

---

## 📄 文档列表

| 文档 | 说明 | 适合人群 |
|------|------|----------|
| [SERVICES.md](SERVICES.md) | 🆕 服务双轨架构 - CLI AI 助手 vs 项目自更新 | 所有开发者 |
| [QUICK_REFERENCE.md](QUICK_REFERENCE.md) | 快速参考卡片 - 常用命令、核心文件、工具箱 | 所有开发者 |
| [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md) | 完整项目结构 - 架构分层、模块职责、集成状态 | 新开发者、架构师 |
| [TOOL_SELECTOR_GUIDE.md](TOOL_SELECTOR_GUIDE.md) | 工具选择器指南 - API、配置、性能指标 | 工具开发者 |
| [project_structure.json](project_structure.json) | 结构化项目数据 - JSON 格式 | 工具开发者 |
| [../docs/paper_plan/](../docs/paper_plan/) | 🆕 论文计划文档 - Prompt Engineering 方案 | 研究者 |

---

## 🚀 快速开始

### 第一次接触项目？

**推荐学习路径**:

```
路径 1: 快速了解（15 分钟）
├─ SERVICES.md (5 分钟) - 双轨服务架构
├─ QUICK_REFERENCE.md (5 分钟) - 快速参考
└─ 运行 cargo run --release (5 分钟) - 体验 CLI 模式

路径 2: 深入研究（60 分钟）
├─ docs/paper_plan/EXECUTIVE_SUMMARY.md (10 分钟) - 论文计划摘要
├─ PROJECT_STRUCTURE.md (20 分钟) - 完整结构
├─ docs/paper_plan/MECHANISMS.md (30 分钟) - 核心机制设计
└─ 源码阅读

路径 3: 准备实施（30 分钟）
├─ docs/paper_plan/EXECUTIVE_SUMMARY.md (5 分钟)
├─ docs/paper_plan/MECHANISMS.md (15 分钟) - Prompt 设计
└─ docs/paper_plan/IMPLEMENTATION_GUIDE.json (10 分钟) - 实施指南
```

---

## 🎯 服务双轨架构（更新：Prompt Engineering 自进化）

> 💡 **重要**: Tokitai 采用**双轨服务架构**，两种服务共享底层能力但定位不同

| 服务 | 启动命令 | 服务对象 | 驱动方式 | 典型场景 |
|------|----------|----------|----------|----------|
| **📱 CLI AI 助手** | `cargo run --release` | 用户（开发者） | 用户输入驱动 | 查询、分析、临时任务 |
| **🤖 项目自更新** | `cargo run --release -- --autonomous` | 项目自身 | **Prompt Engineering** | 代码改进、**自主进化** |

### 架构演进（2026-03-20 更新）

```
┌─────────────────────────────────────────────────────────────────┐
│                        Tokitai 双轨服务                          │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────┐    ┌─────────────────────────────┐│
│  │   CLI AI 助手            │    │   项目自更新服务             ││
│  │   (面向用户)            │    │   (面向项目自身)            ││
│  │                         │    │                             ││
│  │  📱 交互式对话          │    │  🤖 自主进化循环            ││
│  │  👤 用户驱动            │    │  🧠 Prompt Engineering     ││
│  │  ⚡ 即时响应            │    │  🔄 迭代执行                ││
│  │  🛠️ 完成任务            │    │  📈 持续改进                ││
│  └─────────────────────────┘    └─────────────────────────────┘│
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    共享底层能力                              ││
│  │  ToolMatrix │ Context Storage │ Orchestrator │ Autonomy    ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  🆕 HybridGapDetector ⭐ (成本降低 95%, 延迟降低 83-97%)     ││
│  │  ┌──────────────────────────────────────────────────────┐  ││
│  │  │  Stage 1: Statistical Filter (<100ms, 0 API)         │  ││
│  │  │  Stage 2: Causal Analysis (5-30 秒，1-2 API)          │  ││
│  │  │  Stage 3: Merger & Prioritize (<50ms, 0 API)         │  ││
│  │  └──────────────────────────────────────────────────────┘  ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │         Prompt Engineering 自进化系统 (论文贡献)             ││
│  │  ┌──────────────────────────────────────────────────────┐  ││
│  │  │  PromptGapDetector    - 因果推理 Prompt 缺口检测      │  ││
│  │  │  PromptOptimizer      - Few-Shot 工具优化            │  ││
│  │  │  PromptCreator        - 代码生成 + 自修正循环        │  ││
│  │  │  MultiAgentNegotiator - 多智能体协商协议            │  ││
│  │  └──────────────────────────────────────────────────────┘  ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

**详细说明**: 请查看 [SERVICES.md](SERVICES.md)

---

## 🔧 核心模块（2026-03-20 更新）

| 模块 | 行数 | 占比 | 状态 | 说明 |
|------|------|------|------|------|
| `tools/` | 16,802 | 31.7% | ✅ | 工具集合（文件/网络/系统/Git/数据/张量） |
| `context/` | 7,398 | 14.0% | ✅ | 上下文存储（三层架构/哈希链/语义索引） |
| `autonomy/` | 7,072 | 13.4% | 🆕 | **自进化系统**（含 HybridGapDetector） |
| `tool_matrix/` | 8,271 | 15.6% | ✅ | 工具矩阵（服务注册表/选择器/AI 分类器） |
| `orchestrator/` | 4,419 | 8.3% | ✅ | 编排调度（工作流/角色切换/TOML 加载器） |
| `main_core` | 3,079 | 5.8% | ✅ | 主程序入口和核心逻辑 |
| `observability/` | 901 | 1.7% | ✅ | 可观测性（已集成） |
| `dialogue/` | 751 | 1.4% | ✅ | 对话状态机（已集成） |
| `prompt_engineering/` | 677 | 1.3% | ✅ | **Prompt 模板管理**（自进化基础） |
| `integration/` | 331 | 0.6% | ✅ | 集成模块管理器 |
| 其他 | 3,263 | 6.2% | ✅ | 配置/沙箱/解析器等 |

**状态说明**: ✅ 已完全集成 | 🆕 含最新 HybridGapDetector 实现

---

## 🆕 论文计划（Prompt Engineering 方案）

### 核心贡献

| 贡献 | 方法 | 创新点 | 实施周期 | 状态 |
|------|------|--------|----------|------|
| **HybridGapDetector** ⭐ | 统计筛选 + 因果分析 + 证据融合 | 首个融合统计与 Prompt Engineering 的缺口检测器 | 1 周 | ✅ 完成 |
| **PromptGapDetector** | Chain-of-Thought + 反事实推理 | 首个用于工具缺口检测的因果推理 Prompt | 2 周 | ✅ 完成 |
| **PromptOptimizer** | Few-Shot Learning + 结构化输出 | 工具库优化的系统化 Prompt 设计 | 2 周 | ✅ 完成 |
| **PromptCreator** | Code Generation + Self-Correction | 编译错误反馈的自修正循环 | 2 周 | ✅ 完成 |
| **MultiAgentNegotiator** | Role-Playing + Consensus Building | 多 LLM 智能体协商协议 | 2 周 | ✅ 完成 |

### HybridGapDetector 性能指标

| 指标 | 纯统计 | 纯 Prompt | HybridGapDetector | 提升 |
|------|--------|-----------|-------------------|------|
| 检测延迟 | <100ms | 5-30 秒 | **1-5 秒** | 83-97% ↓ |
| API 调用/周期 | 0 | 10-20 次 | **2-5 次** | 75-80% ↓ |
| 检测准确率 | 60-70% | 75-85% | **75-85%** | 持平 |
| 月 API 成本 | $0 | $45 | **$2.25** | **95% ↓** |

**详细文档**: [docs/HYBRID_GAP_DETECTOR_IMPLEMENTATION.md](../docs/HYBRID_GAP_DETECTOR_IMPLEMENTATION.md)

### 关键指标

| 维度 | 原方案（训练） | 新方案（Prompt Engineering） |
|------|---------------|---------------------------|
| **硬件** | RTX 3090/A100 ($500-2000) | ❌ 无需 GPU (<$150 API) |
| **时间** | 12-20 周 | **8 周实施 + 4 周实验** |
| **成本** | $500-2000 | **<$150** |
| **性能** | 75-90% | **70-85%** (足够好) |
| **可解释性** | 黑盒 | **透明，易调试** |

### 投稿时间线

```
2026-03-20 今天
    ↓
2026-04-03 PromptGapDetector 完成
    ↓
2026-05-15 所有模块完成
    ↓
2026-06-15 实验完成
    ↓
2026-07-15 论文初稿完成
    ↓
2026-08-01 投稿 AAAI 2027 ✅
```

**详细论文计划**: 请查看 [docs/paper_plan/](../docs/paper_plan/)

---

## 📚 相关文档

### 入门文档
| 文档 | 说明 |
|------|------|
| [docs/QUICKSTART.md](../docs/QUICKSTART.md) | 快速启动指南 |
| [docs/USER_GUIDE.md](../docs/USER_GUIDE.md) | 完整用户指南 |
| [docs/DEMO.md](../docs/DEMO.md) | 演示指南 |
| [docs/CHANGELOG.md](../docs/CHANGELOG.md) | 更新日志 |

### 架构文档
| 文档 | 说明 |
|------|------|
| [structure_ensure/SERVICES.md](SERVICES.md) | 服务双轨架构说明 |
| [structure_ensure/QUICK_REFERENCE.md](QUICK_REFERENCE.md) | 快速参考卡片 |
| [structure_ensure/PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md) | 完整项目结构 |
| [structure_ensure/TOOL_SELECTOR_GUIDE.md](TOOL_SELECTOR_GUIDE.md) | 工具选择器指南 |

### 论文计划文档（🆕）
| 文档 | 说明 |
|------|------|
| [docs/paper_plan/EXECUTIVE_SUMMARY.md](../docs/paper_plan/EXECUTIVE_SUMMARY.md) | 📋 执行摘要（5 分钟速览） |
| [docs/paper_plan/README.md](../docs/paper_plan/README.md) | 📖 完整论文规划索引 |
| [docs/paper_plan/MECHANISMS.md](../docs/paper_plan/MECHANISMS.md) | ⚙️ 核心机制设计（Prompt Engineering 版本） |
| [docs/paper_plan/PROMPT_ENGINEERING_APPROACH.json](../docs/paper_plan/PROMPT_ENGINEERING_APPROACH.json) | 💡 Prompt Engineering 方案 |
| [docs/paper_plan/IMPLEMENTATION_GUIDE.json](../docs/paper_plan/IMPLEMENTATION_GUIDE.json) | 🛠️ 实施指南（硬件/成本） |
| [docs/paper_plan/ALGORITHM_INNOVATION_PROPOSAL.json](../docs/paper_plan/ALGORITHM_INNOVATION_PROPOSAL.json) | 🎯 算法创新提案（含训练方案对比） |
| [docs/paper_plan/DOCUMENT_STRUCTURE.md](../docs/paper_plan/DOCUMENT_STRUCTURE.md) | 📖 论文计划文档结构说明 |

### 技术报告（归档）
| 文档 | 说明 |
|------|------|
| [docs/archive/](../docs/archive/) | 技术报告归档 |
| `MODULE_INTEGRATION_REPORT.md` | 模块集成报告 |
| `MODULE_IMPROVEMENT_REPORT.md` | 模块改进报告（P11 级） |
| `SERVICE_ARCHITECTURE_IMPLEMENTATION.md` | 服务化架构实施报告 |
| `LIGHTWEIGHT_TOOL_SELECTION_DESIGN.md` | 工具选择器设计 |
| `LIGHTWEIGHT_TOOL_SELECTION_DEEPENING.md` | 深化落实报告 |
| `LIGHTWEIGHT_TOOL_SELECTION_FINAL_SUMMARY.md` | 总结报告 |

---

## 📁 运行时文件夹（已添加到 .gitignore）

以下文件夹在运行时自动创建，已添加到 `.gitignore` 中，不会被提交到版本控制：

| 文件夹 | 用途 | 说明 |
|--------|------|------|
| `sandbox/` | 沙箱测试目录 | 用于测试文件操作、项目模板等功能 |
| `downloads/` | 下载文件目录 | 使用下载工具时，文件默认保存到此目录 |
| `.context/` | 上下文存储 | 三层存储架构（瞬时/短期/长期）的持久化数据 |
| `.tokitai/` | 运行时数据 | 对话状态、追踪日志、自主进化数据等 |

> 💡 **提示**：这些文件夹会在首次运行程序时自动创建，无需手动创建。如需清理缓存，可直接删除这些文件夹。

---

## ✨ 核心特性

### 🤖 AI 原生工具选择器

- **ToolIndex**: 倒排索引，支持关键词/分类/工具箱检索
- **LightweightToolSelector**: 快速搜索 <10ms，AI 搜索 <2s，LRU 缓存命中后 ~3ms
- **AIToolboxClassifier**: AI 自主管理工具箱体系
- **AIDependencyAnalyzer**: AI 自主维护工具依赖关系（静态分析 + 运行时学习）
- **后台异步重建**: 不阻塞主线程，批量处理优化（100 工具 ~600ms）
- **ToolDispatcher**: 统一工具调用分发器
- **SelectorMetrics**: 完整监控指标（搜索次数/缓存命中率/平均延迟）

### 🛠️ 完整工具矩阵 (IMP-001~004)

| 改进项目 | 功能 | 状态 |
|---------|------|------|
| **IMP-001** | 规则分类器（分层缓存 L1→L2→L3→L4） | ✅ |
| **IMP-002** | 工具生成器（tokitai 宏生成） | ✅ |
| **IMP-003** | Trie 索引 + BK-Tree 拼写纠正 | ✅ |
| **IMP-004** | 动态注册表（热加载） | ✅ |

### 📁 纯文件上下文存储

- **无数据库依赖**：纯文件存储，轻量级
- **三层存储架构**：瞬时层 → 短期层 → 长期层
- **增量哈希链 (ICHC)**：不可篡改的链式哈希结构
- **上下文蒸馏 (HCD)**：提取核心意图，过滤冗余
- **语义索引 (LSFI)**：基于 SimHash 的语义搜索

### 🔒 安全沙箱

- 路径验证、命令黑名单
- SSRF 防护、内网 IP 过滤
- 符号链接循环检测
- 递归深度限制、速率限制

### 🌐 服务化架构

- **服务元数据**: ServiceMetadata 包含分类、QoS、依赖、版本、标签
- **服务生命周期**: ServiceLifecycle trait (init/health/shutdown/stats)
- **服务健康状态**: Healthy/Degraded/Unhealthy
- **服务统计**: ServiceStats 记录调用次数/成功率/延迟
- **服务指标收集**: ServiceMetricsCollector 统一收集服务调用指标
- **服务分类**: 10 种服务类型（Utility/File/Network/System/Data/Ai/Vcs/Dialogue/Observability/Prompt）
- **声明式工作流**: TOML 定义，支持重试/超时/错误处理
- **TOML 工作流加载器**: WorkflowLoader 从文件/目录加载工作流

### 🆕 Prompt Engineering 自进化系统（论文贡献）

- **PromptGapDetector**: 因果推理 Prompt 缺口检测（Chain-of-Thought + 反事实）
- **PromptOptimizer**: Few-Shot 工具优化（历史决策示例）
- **PromptCreator**: 代码生成 + 自修正循环（cargo check → 反馈 → 修正）
- **MultiAgentNegotiator**: 多智能体协商（4 角色：Creator/Optimizer/Eliminator/Planner）
- **SelfImprovementLoop**: 完整自主改进循环（反思→缺口→优化→创造）

### 🧩 集成模块

- **IntegratedModules**: 统一管理 dialogue/observability/prompt_engineering
- **共享状态管理**: `Arc<RwLock>` 跨模块同步
- **优雅降级**: 单模块失败不影响其他

---

## 💡 使用建议

### 新功能开发
1. 查看 `PROJECT_STRUCTURE.md` 了解模块职责
2. 确定新功能所属模块
3. 参考现有代码风格（遵循 tokitai 规范）
4. 添加测试覆盖

### 问题排查
1. 查看 `QUICK_REFERENCE.md` 找到相关模块
2. 定位源代码文件
3. 查看 `docs/archive/` 中的技术报告了解历史决策

### 学习项目
1. 先阅读 `QUICK_REFERENCE.md` 建立整体印象
2. 深入阅读 `PROJECT_STRUCTURE.md` 了解架构细节
3. 从 `main.rs` 开始阅读源码
4. 按模块逐步深入

### 了解 Prompt Engineering 自进化（🆕）
1. 阅读 `docs/paper_plan/EXECUTIVE_SUMMARY.md`（5 分钟速览）
2. 阅读 `docs/paper_plan/MECHANISMS.md`（核心机制设计）
3. 查看 `src/autonomy/` 目录中的实现
4. 参考 `docs/paper_plan/IMPLEMENTATION_GUIDE.json`（实施指南）

### 了解集成改进
1. 阅读 `docs/archive/MODULE_IMPROVEMENT_REPORT.md`
2. 查看 `src/integration/modules_manager.rs`
3. 了解 tokitai ToolProvider 的使用方式

### 了解服务化架构
1. 阅读 `docs/archive/SERVICE_ARCHITECTURE_IMPLEMENTATION.md`
2. 查看 `src/tool_matrix/matrix.rs` 了解服务元数据
3. 查看 `src/orchestrator/workflow_loader.rs` 了解 TOML 工作流
4. 查看 `workflows/` 目录中的工作流示例

### 了解工具选择器
1. 阅读 `docs/archive/LIGHTWEIGHT_TOOL_SELECTION_DEEPENING.md`（深化落实报告）
2. 阅读 `docs/archive/LIGHTWEIGHT_TOOL_SELECTION_FINAL_SUMMARY.md`（总结报告）
3. 查看 `src/tool_matrix/tool_selector.rs` 了解核心实现（LRU 缓存/监控指标）
4. 查看 `src/tool_matrix/registry.rs` 了解 AI 分类器/分析器集成
5. 查看 `src/tool_matrix/ai_classifier.rs` 了解 AI 分类器（parking_lot RwLock）
6. 查看 `src/tool_matrix/dependency_analyzer.rs` 了解依赖分析器（运行时学习）
7. 查看 `src/tool_matrix/dispatcher.rs` 了解工具调用分发器

### 了解 IMP-001~004 实现
1. **IMP-001 (规则分类器)**: `src/tool_matrix/rule_classifier.rs`
2. **IMP-002 (工具生成器)**: `src/tool_matrix/tool_generator.rs`
3. **IMP-003 (Trie 索引)**: `src/tool_matrix/trie_index.rs`
4. **IMP-004 (动态注册表)**: `src/tool_matrix/dynamic_registry.rs`
5. 阅读 `docs/archive/ARCHITECTURE_IMPROVEMENT_PLAN.json` 了解完整计划
6. 阅读 `docs/ARCHITECTURE_IMPROVEMENT_REPORT.md` 了解实施报告

---

## 📈 测试状态

```
running 470 tests
✅ autonomy::hybrid_gap_detector::... (3 个新测试)
✅ autonomy::gap_detector::...
✅ autonomy::prompt_gap_detector::...
✅ autonomy::prompt_optimizer::...
✅ autonomy::multi_agent_negotiator::...
✅ autonomy::self_improvement_loop::...
✅ context::...
✅ tool_matrix::...
✅ tools::...
✅ orchestrator::...
✅ integration::...

test result: ok. 470 passed; 0 failed
```

---

**最后更新**: 2026-03-20  
**版本**: 3.1 (HybridGapDetector 实现完成)  
**测试状态**: 470/470 ✅  
**构建状态**: Release ✅  
**论文目标**: AAAI 2027 / ACL 2027 / EMNLP 2027
