# try-tokitai

> **AI 原生工具选择器 + 双轨服务架构**
>
> 基于 [Tokitai](https://github.com/silverenternal/tokitai) 构建的强大 AI 助手，支持 **CLI 交互** 和 **自主进化** 双模式，配备 63+ 工具和 AI 原生工具选择系统。

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()
[![Tests](https://img.shields.io/badge/tests-411%20passed-brightgreen)]()
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)]()

---

## 📊 项目概览

| 指标 | 数值 |
|------|------|
| **代码行数** | ~52,964 行 Rust |
| **源代码文件** | 131 个 |
| **核心模块** | 10 个 |
| **工具箱** | 11 个 |
| **工具函数** | 63+ 个 |
| **测试状态** | 411/411 通过 ✅ |

---

## 🎯 双轨服务架构

本项目采用独特的**双轨服务架构**，两种模式共享底层能力但定位不同：

| 模式 | 启动命令 | 服务对象 | 典型场景 |
|------|----------|----------|----------|
| **📱 CLI AI 助手** | `cargo run --release` | 用户（开发者） | 查询、分析、临时任务 |
| **🤖 项目自更新** | `cargo run --release -- --autonomous` | 项目自身 | 代码改进、技术债务清理 |

```
┌─────────────────────────────────────────────────────────────────┐
│                        Tokitai 双轨服务                          │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐              ┌─────────────────────────┐   │
│  │  CLI AI 助手     │              │  项目自更新服务          │   │
│  │  (面向用户)     │              │  (面向项目自身)         │   │
│  │                 │              │                         │   │
│  │  • 交互式对话   │              │  • 自主进化循环         │   │
│  │  • 用户驱动     │              │  • AI 驱动              │   │
│  │  • 即时响应     │              │  • Planner-Executor-    │   │
│  │  • 完成任务     │              │    Reviewer 迭代        │   │
│  └─────────────────┘              └─────────────────────────┘   │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │              共享底层能力                                     ││
│  │  ToolMatrix │ Context Storage │ Orchestrator │ Autonomy    ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

详细架构说明：[structure_ensure/SERVICES.md](structure_ensure/SERVICES.md)

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

### 🧩 集成模块

- **IntegratedModules**: 统一管理 dialogue/observability/prompt_engineering
- **共享状态管理**: `Arc<RwLock>` 跨模块同步
- **优雅降级**: 单模块失败不影响其他

---

## 🚀 快速开始

### 1️⃣ 获取 API Key

本项目使用 **Ollama Cloud** 作为默认 AI 服务：

1. 访问 https://ollama.com
2. 注册/登录账号
3. 进入 Settings → API Keys
4. 创建新 Key 并复制（格式：`ollama-xxxxxxxx...`）

> 💡 Ollama Cloud 目前提供免费额度，足够个人开发和测试使用。

### 2️⃣ 配置环境变量

```bash
# 方法一：临时设置
export AI_API_KEY="ollama-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
export AI_API_URL="https://ollama.com/v1/chat/completions"
export AI_MODEL="qwen3.5:397b"

# 方法二：永久设置（推荐）
cp .env.example .env
# 编辑 .env 文件，填入你的 API Key
```

### 3️⃣ 启动程序

```bash
# CLI AI 助手模式（默认）
cargo run --release

# 项目自更新模式（自主进化）
cargo run --release -- --autonomous

# 指定项目路径
cargo run --release -- -p ./sandbox/test-project
```

### 4️⃣ 运行测试

```bash
# 所有测试
cargo test

# 特定模块测试
cargo test tool_matrix
cargo test tool_selector
cargo test ai_classifier
cargo test autonomy
```

---

## 🛠️ 工具箱

项目提供 **63+ 工具函数**，分为 **11 个工具箱**：

| 工具箱 | 工具数 | 功能 |
|--------|--------|------|
| `file_ops` | 15 | 文件读写、搜索、PDF 处理、项目模板 |
| `web` | 20 | HTTP 请求、网页搜索、下载、网络诊断、Wikipedia |
| `system` | 13 | 命令执行、进程管理、代码分析、对话状态、可观测性、提示词 |
| `code` | 4 | 代码分析、语言检测 |
| `git` | 4 | Git 状态、日志、分支管理 |
| `data` | 5 | JSON 格式化、查询、转换 |
| `autonomy` | 2 | 自主进化（仅自主模式） |

### 新增工具（已集成到 system 工具箱）

#### 对话状态管理 (DialogueTools)
- `get_state()`, `get_context()`, `get_history()`
- `set_goal()`, `set_plan()`, `record_tool_execution()`
- `transition()`, `reset()`, `get_stats()`, `sync_with_autonomy()`

#### 可观测性 (ObservabilityTools)
- `get_recent_traces()`, `get_stats()`, `query_trace()`
- `query_errors()`, `export_traces()`, `cleanup_old_traces()`

#### 提示词工程 (PromptTools)
- `load_role_template()`, `render_template()`, `has_template()`
- `list_available_templates()`, `get_render_stats()`, `warmup_cache()`

---

## 💬 交互命令

| 命令 | 说明 |
|------|------|
| `help` | 显示可用操作列表 |
| `exit` / `quit` | 退出程序 |
| `/role <name>` | 切换角色（planner/executor/reviewer/researcher） |
| `/optimize` | 优化上下文 |
| `/context` | 显示上下文状态 |
| `/workflow list` | 列出可用工作流 |
| `/workflow start` | 启动工作流 |
| `/toolbox` | 显示工具箱状态 |
| `@<路径>` | 快速引用文件（如 `@README.md`） |

---

## 📋 演示示例

### CLI 模式示例

```
👤 你：当前目录有哪些文件
🤖 AI：当前目录包含以下文件...

👤 你：读取 README.md 的内容
🤖 AI：README.md 的内容如下...

👤 你：@src/main.rs 的结构是什么
🤖 AI：main.rs 的结构分析如下...

👤 你：帮我创建一个新文件 test.txt，写入 Hello World
🤖 AI：已创建文件 test.txt...
```

### 自主模式示例

```
$ cargo run --release -- --autonomous

[Planner] 分析项目状态...
[Planner] 发现改进点：修复 Clippy 警告
[Planner] 制定改进计划...

[Executor] 执行任务 1/5: 修复 src/main.rs 的警告
[Executor] 执行任务 2/5: 添加缺失的单元测试
...

[Reviewer] 代码审查通过
[Reviewer] 运行测试... 236/236 passed ✅
[GitWorkflow] 自动提交：fix: resolve Clippy warnings

[Planner] 开始下一轮迭代...
```

---

## 📊 项目规模

| 模块 | 行数 | 占比 | 说明 |
|------|------|------|------|
| `tools/` | 16,802 | 31.7% | 工具集合（文件/网络/系统/Git/数据） |
| `context/` | 7,398 | 14.0% | 上下文存储（三层架构/哈希链/语义索引） |
| `autonomy/` | 7,072 | 13.4% | 自主进化（多 Agent 协作/智能工具推荐） |
| `orchestrator/` | 4,419 | 8.3% | 编排调度（工作流/角色切换/TOML 加载器） |
| `tool_matrix/` | 8,271 | 15.6% | 工具矩阵（服务注册表/选择器/AI 分类器/AI 分析器/规则分类器/查询增强器/工具生成器/Trie 索引/动态注册表） |
| `main_core` | 3,079 | 5.8% | 主程序入口和核心逻辑 |
| `observability/` | 901 | 1.7% | 可观测性（已集成） |
| `dialogue/` | 751 | 1.4% | 对话状态机（已集成） |
| `prompt_engineering/` | 677 | 1.3% | 提示词工程（已集成） |
| `integration/` | 331 | 0.6% | 集成模块管理器 |
| 其他 | 3,263 | 6.2% | 配置/沙箱/解析器等 |
| **总计** | **~52,964** | **100%** | **131 个源文件** |

---

## 🏗️ 项目结构

```
try-tokitai/
├── Cargo.toml                    # 项目配置和依赖
├── config.toml                   # 应用配置
├── .env.example                  # 环境变量模板
├── README.md                     # 项目说明
├── demo.sh                       # 一键演示脚本
│
├── docs/                         # 用户文档
│   ├── QUICKSTART.md            # 快速启动
│   ├── USER_GUIDE.md            # 用户指南
│   ├── DEMO.md                  # 演示指南
│   ├── CHANGELOG.md             # 更新日志
│   └── archive/                 # 技术报告归档
│       ├── MODULE_INTEGRATION_REPORT.md   - 集成报告
│       ├── MODULE_IMPROVEMENT_REPORT.md   - 改进报告
│       ├── SERVICE_ARCHITECTURE_IMPLEMENTATION.md - 服务化架构
│       ├── LIGHTWEIGHT_TOOL_SELECTION_DESIGN.md - 工具选择器设计
│       ├── LIGHTWEIGHT_TOOL_SELECTION_DEEPENING.md - 深化落实报告
│       └── LIGHTWEIGHT_TOOL_SELECTION_FINAL_SUMMARY.md - 总结
│
├── workflows/                    # TOML 工作流定义
│   ├── research_and_write.toml  - 研究并撰写报告工作流
│   └── code_review.toml         - 代码审查工作流
│
├── src/                          # 源代码
│   ├── main.rs                  # 程序入口，AiAssistant 整合
│   ├── config.rs                # 配置管理
│   ├── sandbox.rs               # 沙箱系统
│   │
│   ├── tools/                   # 工具集合 (16,802 行)
│   │   ├── io/                  # 文件 IO 工具
│   │   ├── network/             # 网络工具（服务化）
│   │   ├── system/              # 系统工具
│   │   ├── data/                # 数据处理工具
│   │   └── vcs/                 # 版本控制工具
│   │
│   ├── context/                 # 上下文存储 (7,398 行)
│   ├── autonomy/                # 自主进化模块 (7,072 行)
│   ├── orchestrator/            # 编排调度 (4,419 行)
│   │   ├── orchestrator.rs      # 编排器核心
│   │   ├── role_switcher.rs     # 角色切换
│   │   ├── workflow.rs          # 声明式工作流定义和执行引擎
│   │   └── workflow_loader.rs   # TOML 工作流加载器
│   │
│   ├── tool_matrix/             # 工具矩阵/服务注册表 (8,271 行)
│   │   ├── matrix.rs            # 服务化元数据/生命周期/指标收集
│   │   ├── registry.rs          # 工具注册表（AI 分类/依赖分析/运行时学习）
│   │   ├── tool_selector.rs     # 轻量级工具选择器（AI 原生）
│   │   ├── ai_classifier.rs     # AI 工具箱分类器
│   │   ├── dependency_analyzer.rs # AI 依赖关系分析器
│   │   ├── dispatcher.rs        # 工具调用分发器
│   │   ├── rule_classifier.rs   # 规则分类器（分层缓存 L3）
│   │   ├── query_enhancer.rs    # 查询增强器（同义词/意图识别）
│   │   ├── tool_generator.rs    # 工具生成器（模板系统）
│   │   ├── trie_index.rs        # Trie 树索引和 BK-Tree 拼写纠正
│   │   └── dynamic_registry.rs  # 动态工具注册表（热加载）
│   │
│   ├── integration/             # 集成模块管理器 (331 行)
│   ├── dialogue/                # 对话状态机 (751 行，已集成)
│   ├── observability/           # 可观测性 (901 行，已集成)
│   └── prompt_engineering/      # 提示词工程 (677 行，已集成)
│
├── structure_ensure/            # 项目结构文档
│   ├── README.md                # 结构文档索引
│   ├── SERVICES.md              # 服务架构说明
│   ├── QUICK_REFERENCE.md       # 快速参考卡片
│   ├── PROJECT_STRUCTURE.md     # 完整项目结构
│   └── TOOL_SELECTOR_GUIDE.md   # 工具选择器指南
│
├── .context/                    # 运行时上下文存储
└── .tokitai/                    # 运行时数据
```

---

## 📚 文档导航

### 入门文档
| 文档 | 说明 |
|------|------|
| [docs/QUICKSTART.md](docs/QUICKSTART.md) | 快速启动指南 |
| [docs/USER_GUIDE.md](docs/USER_GUIDE.md) | 完整用户指南 |
| [docs/DEMO.md](docs/DEMO.md) | 演示指南 |
| [docs/CHANGELOG.md](docs/CHANGELOG.md) | 更新日志 |

### 架构文档
| 文档 | 说明 |
|------|------|
| [structure_ensure/SERVICES.md](structure_ensure/SERVICES.md) | 🆕 服务双轨架构说明 |
| [structure_ensure/QUICK_REFERENCE.md](structure_ensure/QUICK_REFERENCE.md) | 快速参考卡片 |
| [structure_ensure/PROJECT_STRUCTURE.md](structure_ensure/PROJECT_STRUCTURE.md) | 完整项目结构 |
| [structure_ensure/TOOL_SELECTOR_GUIDE.md](structure_ensure/TOOL_SELECTOR_GUIDE.md) | 工具选择器指南 |
| [structure_ensure/README.md](structure_ensure/README.md) | 结构文档索引 |

### 技术报告
| 文档 | 说明 |
|------|------|
| [docs/ARCHITECTURE_IMPROVEMENT_PLAN.json](docs/ARCHITECTURE_IMPROVEMENT_PLAN.json) | 架构改进计划 |
| [docs/ARCHITECTURE_IMPROVEMENT_REPORT.md](docs/ARCHITECTURE_IMPROVEMENT_REPORT.md) | 架构改进报告 |
| [docs/archive/MODULE_INTEGRATION_REPORT.md](docs/archive/MODULE_INTEGRATION_REPORT.md) | 模块集成报告 |
| [docs/archive/MODULE_IMPROVEMENT_REPORT.md](docs/archive/MODULE_IMPROVEMENT_REPORT.md) | 模块改进报告 |
| [docs/archive/SERVICE_ARCHITECTURE_IMPLEMENTATION.md](docs/archive/SERVICE_ARCHITECTURE_IMPLEMENTATION.md) | 服务化架构实施报告 |
| [docs/archive/LIGHTWEIGHT_TOOL_SELECTION_DESIGN.md](docs/archive/LIGHTWEIGHT_TOOL_SELECTION_DESIGN.md) | 工具选择器设计 |
| [docs/archive/LIGHTWEIGHT_TOOL_SELECTION_DEEPENING.md](docs/archive/LIGHTWEIGHT_TOOL_SELECTION_DEEPENING.md) | 深化落实报告 |
| [docs/archive/LIGHTWEIGHT_TOOL_SELECTION_FINAL_SUMMARY.md](docs/archive/LIGHTWEIGHT_TOOL_SELECTION_FINAL_SUMMARY.md) | 总结报告 |

---

## 🔧 技术栈

| 类别 | 依赖 |
|------|------|
| **AI 框架** | tokitai 0.4.0, tokitai-core 0.4.0 |
| **异步运行时** | tokio 1.x (full features) |
| **HTTP 客户端** | reqwest 0.12, ureq 2.9 |
| **序列化** | serde 1.0, serde_json 1.0, toml 0.8 |
| **错误处理** | anyhow 1.0, thiserror 2.0 |
| **日志追踪** | tracing 0.1, tracing-subscriber 0.3 |
| **并发** | parking_lot 0.12, threadpool 1.8 |
| **缓存** | moka 0.12 |
| **中文分词** | jieba-rs 0.7 |
| **模板引擎** | tera 1.19 |
| **索引优化** | fst 0.4, bk-tree 0.5 |
| **PDF 处理** | lopdf 0.34 |

---

## 📈 性能指标

| 操作 | 延迟 | 说明 |
|------|------|------|
| 快速搜索 | ~8ms | 关键词匹配 |
| 快速搜索 (缓存命中) | ~3ms | LRU 缓存 1000 条 |
| AI 搜索 | ~1.5s | 含 LLM 调用 |
| 后台重建 (100 工具) | ~600ms | 批量处理优化 |
| 内存占用 (10,000 工具) | ~15MB | 含缓存 |

---

## 🧪 测试状态

```
running 236 tests
✅ autonomy::...
✅ context::...
✅ tool_matrix::...
✅ tool_matrix::tool_selector::... (5 个测试)
✅ tool_matrix::ai_classifier::... (1 个测试)
✅ tool_matrix::dependency_analyzer::... (2 个测试)
✅ tool_matrix::dispatcher::... (3 个测试)
✅ dialogue::...
✅ observability::...
✅ prompt_engineering::...
✅ integration::...
✅ orchestrator::workflow_loader::...

test result: ok. 411 passed; 0 failed
```

---

## ❓ 常见问题

### Q: 提示 "未设置 AI_API_KEY" 怎么办？
A: 参考上方「获取 API Key」步骤，获取 Ollama API Key 并设置环境变量。

### Q: 可以使用本地 Ollama 服务吗？
A: 可以。设置 `AI_API_URL="http://localhost:11434/v1/chat/completions"`

### Q: 模型响应很慢怎么办？
A: 尝试切换到较小的模型：`export AI_MODEL="qwen2.5:7b"`

### Q: 自主模式安全吗？
A: 自主模式在本地执行代码审查（fmt/clippy/test），失败时自动回滚，可配置为仅提交不推送。

---

## 📁 运行时文件夹

以下文件夹在运行时自动创建，已添加到 `.gitignore`，不会被提交到版本控制：

| 文件夹 | 用途 | 说明 |
|--------|------|------|
| `sandbox/` | 沙箱测试目录 | 用于测试文件操作、项目模板等功能 |
| `downloads/` | 下载文件目录 | 使用下载工具时，文件默认保存到此目录 |
| `.context/` | 上下文存储 | 三层存储架构（瞬时/短期/长期）的持久化数据 |
| `.tokitai/` | 运行时数据 | 对话状态、追踪日志、自主进化数据等 |

> 💡 **提示**：这些文件夹会在首次运行程序时自动创建，无需手动创建。如需清理缓存，可直接删除这些文件夹。

---

## 📄 许可证

MIT OR Apache-2.0

## 🙏 致谢

- [tokitai](https://github.com/silverenternal/tokitai) - 优秀的 AI 工具集成框架

---

**最后更新**: 2026-03-18
**测试状态**: 411/411 ✅
**构建状态**: Release ✅
