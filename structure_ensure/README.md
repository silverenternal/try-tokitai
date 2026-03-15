# structure_ensure - 项目结构文档

> **最新版本**: AI 原生工具选择器深化落实版 + 服务双轨架构
> **最后更新**: 2026-03-15

本目录包含项目结构和架构文档，帮助开发者快速了解项目组织。

---

## 📄 文档列表

| 文档 | 说明 | 适合人群 |
|------|------|----------|
| [SERVICES.md](SERVICES.md) | 🆕 服务架构说明 - CLI AI 助手 vs 项目自更新服务 | 所有开发者 |
| [QUICK_REFERENCE.md](QUICK_REFERENCE.md) | 快速参考卡片 - 常用命令、核心文件、工具箱速查 | 所有开发者 |
| [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md) | 完整项目结构详解 - 架构分层、模块职责、集成状态 | 新加入开发者、架构师 |
| [TOOL_SELECTOR_GUIDE.md](TOOL_SELECTOR_GUIDE.md) | 工具选择器使用指南 - API、配置、性能指标 | 工具开发者 |
| [project_structure.json](project_structure.json) | 结构化项目数据 - JSON 格式，可用于工具集成 | 工具开发者 |

---

## 🚀 快速开始

### 第一次接触项目？

1. **阅读顺序**:
   ```
   SERVICES.md (5 分钟) - 了解双轨服务架构
       ↓
   QUICK_REFERENCE.md (5 分钟)
       ↓
   PROJECT_STRUCTURE.md (20 分钟)
       ↓
   源码阅读
   ```

2. **快速上手**:
   ```bash
   # 1. 查看服务架构说明
   cat structure_ensure/SERVICES.md

   # 2. 查看快速参考
   cat structure_ensure/QUICK_REFERENCE.md

   # 3. 运行程序（CLI AI 助手模式）
   cargo run --release

   # 4. 运行测试
   cargo test
   ```

---

## 🎯 服务双轨架构

> 💡 **重要**: Tokitai 采用**双轨服务架构**，两种服务共享底层能力但定位和使用场景完全不同

| 服务 | 启动命令 | 服务对象 | 驱动方式 | 典型场景 |
|------|----------|----------|----------|----------|
| **CLI AI 助手** | `cargo run --release` | 用户（开发者） | 用户输入驱动 | 查询、分析、临时任务 |
| **项目自更新** | `cargo run --release -- --autonomous` | 项目自身 | AI 自主驱动 | 代码改进、技术债务清理 |

### 服务对比

```
┌─────────────────────────────────────────────────────────────────┐
│                        Tokitai 双轨服务                          │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────┐    ┌─────────────────────────────┐│
│  │   CLI AI 助手            │    │   项目自更新服务             ││
│  │   (面向用户)            │    │   (面向项目自身)            ││
│  │                         │    │                             ││
│  │  📱 交互式对话          │    │  🤖 自主进化循环            ││
│  │  👤 用户驱动            │    │  🧠 AI 驱动                 ││
│  │  ⚡ 即时响应            │    │  🔄 迭代执行                ││
│  │  🛠️ 完成任务            │    │  📈 持续改进                ││
│  └─────────────────────────┘    └─────────────────────────────┘│
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    共享底层能力                              ││
│  │  ToolMatrix │ Context Storage │ Orchestrator │ Autonomy    ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

**详细说明**: 请查看 [SERVICES.md](SERVICES.md)

---

## 🔧 核心模块

| 模块 | 行数 | 占比 | 状态 | 说明 |
|------|------|------|------|------|
| `tools/` | 7,114 | 26.7% | ✅ | 工具集合（文件/网络/系统/Git/数据） |
| `context/` | 4,794 | 18.0% | ✅ | 上下文存储（三层架构/哈希链/语义索引） |
| `orchestrator/` | 3,528 | 13.3% | ✅ | 编排调度（工作流/角色切换/TOML 加载器） |
| `autonomy/` | 2,684 | 10.1% | ⚠️ | 自主进化（多 Agent 协作/智能工具推荐） |
| `tool_matrix/` | 3,362 | 12.6% | ✅ | 工具矩阵（服务注册表/选择器/AI 分类器/AI 分析器） |
| `integration/` | 325 | 1.2% | ✅ | 集成模块管理器 |
| `dialogue/` | 443 | 1.7% | ✅ | 对话状态机（已集成） |
| `observability/` | 456 | 1.7% | ✅ | 可观测性（已集成） |
| `prompt_eng/` | 395 | 1.5% | ✅ | 提示词工程（已集成） |
| `main_core` | 2,326 | 8.7% | ✅ | 主程序入口和核心逻辑 |
| 其他 | 1,733 | 6.5% | ✅ | 配置/沙箱/解析器等 |

**状态说明**: ✅ 已完全集成 | ⚠️ 部分集成

---

## 📚 相关文档

### 用户文档
- [docs/QUICKSTART.md](../docs/QUICKSTART.md) - 快速启动指南
- [docs/USER_GUIDE.md](../docs/USER_GUIDE.md) - 完整用户指南
- [docs/DEMO.md](../docs/DEMO.md) - 演示指南
- [docs/CHANGELOG.md](../docs/CHANGELOG.md) - 更新日志

### 技术报告（归档）
- [docs/archive/](../docs/archive/) - 技术报告归档
  - `MODULE_INTEGRATION_REPORT.md` - 模块集成报告
  - `MODULE_IMPROVEMENT_REPORT.md` - 模块改进报告（P11 级）
  - `SERVICE_ARCHITECTURE_IMPLEMENTATION.md` - 服务化架构实施报告
  - `LIGHTWEIGHT_TOOL_SELECTION_DESIGN.md` - 轻量级工具选择器设计文档
  - `LIGHTWEIGHT_TOOL_SELECTION_DEEPENING.md` - 深化落实报告（新增）
  - `LIGHTWEIGHT_TOOL_SELECTION_FINAL_SUMMARY.md` - 总结报告（新增）

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

## 🎯 核心特性

### ✨ 纯文件上下文存储
- 无数据库依赖
- 三层存储架构（瞬时/短期/长期）
- 增量哈希链 (ICHC)
- 上下文蒸馏 (HCD)
- 语义索引 (LSFI)

### 🔒 安全沙箱
- 路径验证
- 命令黑名单
- SSRF 防护
- 内网 IP 过滤

### 🛠️ 丰富工具集
- 63+ 工具函数
- 11 个工具箱
- 覆盖文件/网络/系统/Git/数据处理

### 🚀 极致性能
- 缓存响应 <10ms (50x 提升)
- 首次请求延迟降低 50%
- 流式首字节延迟降低 60-70%

### 🤖 自主进化系统
- AI 自主发现改进点
- 规划 → 执行 → 审查 → 推送 GitHub
- 多 Agent 协作（Planner/Executor/Reviewer）

### 🧩 集成模块
- **统一生命周期管理**: IntegratedModules 统一管理三个模块
- **共享状态管理**: 使用 `Arc<RwLock>` 实现跨模块状态同步
- **与 autonomy 同步**: dialogue 状态与 autonomy 协调器自动同步
- **完整追踪查询**: observability 提供多维度追踪数据查询
- **模板管理**: prompt_engineering 支持模板预热和渲染统计

### 🌐 服务化架构

**服务元数据与生命周期**:
- **服务元数据**: ServiceMetadata 包含分类、QoS、依赖、版本、标签
- **服务生命周期**: ServiceLifecycle trait (init/health/shutdown/stats)
- **服务健康状态**: Healthy/Degraded/Unhealthy/Unknown
- **服务统计**: ServiceStats 记录调用次数/成功率/延迟
- **服务指标收集**: ServiceMetricsCollector 统一收集服务调用指标
- **服务分类**: 10 种服务类型（Utility/File/Network/System/Data/Ai/Vcs/Dialogue/Observability/Prompt）

**声明式工作流**:
- **TOML 工作流**: 支持重试/超时/错误处理
- **TOML 工作流加载器**: WorkflowLoader 从文件/目录加载工作流

**AI 原生工具选择器**（深化落实）:
- **ToolIndex**: 倒排索引，支持关键词/分类/工具箱检索
- **LightweightToolSelector**: 快速搜索 <10ms，AI 搜索 <2s，LRU 缓存命中后 ~3ms
- **AIToolboxClassifier**: AI 自主管理工具箱体系（深度集成到 ToolRegistry）
- **AIDependencyAnalyzer**: AI 自主维护工具依赖关系（静态分析 + 运行时学习）
- **ToolDispatcher**: 统一工具调用分发器
- **后台异步重建**: 不阻塞主线程，批量处理优化（100 工具 ~600ms）
- **SelectorMetrics**: 完整监控指标（搜索次数/缓存命中率/平均延迟）
- **运行时日志学习**: record_call_sequence + learn_from_runtime_logs

**性能改进**（深化落实后）:
- 缓存命中后搜索延迟：~8ms → ~3ms（降低 62.5%）
- 后台重建延迟：~800ms → ~600ms（降低 25%）
- 内存占用（10,000 工具）：~8MB → ~15MB（含缓存，可控）

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

### 了解集成改进
1. 阅读 `docs/archive/MODULE_IMPROVEMENT_REPORT.md`
2. 查看 `src/integration/modules_manager.rs`
3. 了解 tokitai ToolProvider 的使用方式

### 了解服务化架构
1. 阅读 `docs/archive/SERVICE_ARCHITECTURE_IMPLEMENTATION.md`
2. 查看 `src/tool_matrix/matrix.rs` 了解服务元数据
3. 查看 `src/orchestrator/workflow_loader.rs` 了解 TOML 工作流
4. 查看 `workflows/` 目录中的工作流示例

### 了解工具选择器（深化落实）
1. 阅读 `docs/archive/LIGHTWEIGHT_TOOL_SELECTION_DEEPENING.md`（深化落实报告）
2. 阅读 `docs/archive/LIGHTWEIGHT_TOOL_SELECTION_FINAL_SUMMARY.md`（总结报告）
3. 查看 `src/tool_matrix/tool_selector.rs` 了解核心实现（LRU 缓存/监控指标）
4. 查看 `src/tool_matrix/registry.rs` 了解 AI 分类器/分析器集成
5. 查看 `src/tool_matrix/ai_classifier.rs` 了解 AI 分类器（parking_lot RwLock）
6. 查看 `src/tool_matrix/dependency_analyzer.rs` 了解依赖分析器（运行时学习）
7. 查看 `src/tool_matrix/dispatcher.rs` 了解工具调用分发器

---

## 📈 测试状态

```
running 236 tests
✅ autonomy::...
✅ context::...
✅ tool_matrix::...
✅ tool_matrix::tool_selector::... (新增 5 个测试)
✅ tool_matrix::ai_classifier::... (新增 1 个测试)
✅ tool_matrix::dependency_analyzer::... (新增 2 个测试)
✅ tool_matrix::dispatcher::... (新增 3 个测试)
✅ dialogue::...
✅ observability::...
✅ prompt_engineering::...
✅ integration::...
✅ orchestrator::workflow_loader::...

test result: ok. 236 passed; 0 failed
```

---

**最后更新**: 2026-03-15
**测试状态**: 236/236 ✅
**构建状态**: Release ✅
