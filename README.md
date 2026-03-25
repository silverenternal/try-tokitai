# try-tokitai

> **AI 原生工具选择器 + 双轨服务架构**
>
> 基于 [Tokitai](https://github.com/silverenternal/tokitai) 构建的强大 AI 助手，支持 **CLI 交互**、**TUI 图形界面**、**MCP 协议** 和 **自主进化** 四种模式。

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)]()
[![Rust](https://img.shields.io/badge/rust-1.75+-orange)]()

---

## 🎯 核心特性

### 双轨服务架构

| 模式 | 启动命令 | 服务对象 | 典型场景 |
|------|----------|----------|----------|
| **📱 CLI AI 助手** | `cargo run --release` | 用户 | 查询、分析、临时任务 |
| **🎨 TUI 图形界面** | `cargo run --release -- --tui` | 用户 | 可视化交互、工具浏览 |
| **🔌 MCP Server** | `cargo run --release -- --mcp` | 外部 AI 客户端 | 工具暴露、协议兼容 |
| **🤖 自主进化** | `cargo run --release -- --autonomous` | 项目自身 | 代码改进、技术债务清理 |

### 六大 LLM 提供商支持

支持 **OpenAI**、**Gemini**、**Anthropic**、**智谱 AI**、**月之暗面**、**Ollama** 六大提供商，使用 `/model` 命令动态切换：

```bash
/model list          # 列出所有可用模型
/model switch openai # 切换到 OpenAI
/model benchmark     # 运行基准测试
/model stats         # 显示使用统计
```

### 工具市场

```bash
tokitai publish <tool>   # 发布工具
tokitai search <query>   # 搜索工具
tokitai install <tool>   # 安装工具
tokitai list             # 列出已安装工具
```

提供 **10 种工具模板**，覆盖基础操作、网络、文件、AI、代码分析、Git、数据库、搜索、Webhook、自动化等场景。

### MCP 协议支持

- **MCP Server 模式**：将所有 `#[tool]` 函数暴露为标准 MCP 工具
- **MCP Client 模式**：发现并调用外部 MCP Server 的工具

---

## 🚀 快速开始

### 1. 配置 API Key

```bash
# 复制环境变量模板
cp .env.example .env

# 编辑 .env 文件，填入你的 API Key
# 支持多提供商配置（详见 docs/QUICKSTART.md）
```

### 2. 启动程序

```bash
# CLI 模式（默认）
cargo run --release

# TUI 模式
cargo run --release -- --tui

# MCP Server 模式
cargo run --release -- --mcp

# 自主进化模式
cargo run --release -- --autonomous
```

### 3. 运行测试

```bash
cargo test
```

---

## 📊 项目规模

| 指标 | 数值 |
|------|------|
| **代码行数** | ~53,000 行 Rust |
| **核心模块** | 15 个 |
| **工具箱** | 12 个 |
| **工具函数** | 63+ 个 |
| **LLM 提供商** | 6 个 |
| **工具模板** | 10 个 |

---

## 📚 文档导航

| 文档 | 说明 |
|------|------|
| [docs/QUICKSTART.md](docs/QUICKSTART.md) | **快速启动指南**（推荐先看） |
| [docs/USER_GUIDE.md](docs/USER_GUIDE.md) | 完整用户指南 |
| [docs/PHASE_1_COMPLETION_REPORT.md](docs/PHASE_1_COMPLETION_REPORT.md) | Phase 1 完成报告 |
| [docs/STRATEGIC_IMPLEMENTATION_PLAN.json](docs/STRATEGIC_IMPLEMENTATION_PLAN.json) | 战略实施计划 |
| [structure_ensure/SERVICES.md](structure_ensure/SERVICES.md) | 双轨服务架构详解 |

---

## 🛠️ 工具箱

| 工具箱 | 工具数 | 功能 |
|--------|--------|------|
| `file_ops` | 15 | 文件读写、搜索、PDF 处理 |
| `web` | 20 | HTTP 请求、网页搜索、下载 |
| `system` | 13 | 命令执行、进程管理、代码分析 |
| `code` | 4 | 代码分析、语言检测 |
| `git` | 4 | Git 状态、日志、分支管理 |
| `data` | 5 | JSON 格式化、查询、转换 |
| `tensor` | 20+ | 张量计算（实验性） |
| `autonomy` | 2 | 自主进化（仅自主模式） |

---

## 🏗️ 项目结构

```
try-tokitai/
├── src/
│   ├── main.rs                  # 程序入口
│   ├── lib.rs                   # 库入口
│   ├── llm/                     # 多模型支持（6 提供商）
│   ├── mcp/                     # MCP 协议（Server/Client）
│   ├── tool_market/             # 工具市场
│   ├── tui/                     # TUI 图形界面
│   ├── tools/                   # 工具集合（63+ 工具）
│   ├── tool_matrix/             # 工具矩阵/服务注册表
│   ├── context/                 # 上下文存储（三层架构）
│   ├── autonomy/                # 自主进化（多 Agent 协作）
│   ├── orchestrator/            # 编排调度（工作流/角色切换）
│   ├── dialogue/                # 对话状态机
│   ├── observability/           # 可观测性
│   └── prompt_engineering/      # 提示词工程
│
├── docs/                        # 用户文档
├── tools/marketplace/templates/ # 10 种工具模板
├── workflows/                   # TOML 工作流定义
└── structure_ensure/            # 架构文档
```

---

## 📈 Phase 1 完成状态

✅ **MP-001**: 多模型支持（6 提供商 + 智能路由 + /model 命令）  
✅ **TE-001**: 工具市场（publish/search/install/list + 10 模板）  
✅ **DX-001**: TUI 界面（三面板布局 + 状态栏 + 快捷键）  
✅ **MCP-001**: MCP 协议（Server + Client 模式）

**构建状态**: `cargo build --release` ✅ 通过

---

## 🔧 技术栈

| 类别 | 依赖 |
|------|------|
| **AI 框架** | tokitai 0.4.0, tokitai-core 0.4.0, tokitai-mcp-server 0.4.0 |
| **异步运行时** | tokio 1.x (full) |
| **TUI** | ratatui 0.26, crossterm 0.27 |
| **HTTP** | reqwest 0.12 |
| **序列化** | serde 1.0, serde_json 1.0, toml 0.8 |
| **错误处理** | anyhow 1.0, thiserror 2.0 |

---

## 📄 许可证

MIT OR Apache-2.0

---

**最后更新**: 2026-03-25  
**版本**: 0.4.0
