# try-tokitai

> **AI 原生工具选择器 + Git 分支式上下文管理**
>
> 基于 [Tokitai](https://github.com/silverenternal/tokitai) 构建的强大 AI 助手，支持 **CLI 交互**、**TUI 图形界面**、**MCP 协议**、**自主进化** 和 **Git 式上下文管理**。

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)]()
[![Rust](https://img.shields.io/badge/rust-1.75+-orange)]()
[![Tests](https://img.shields.io/badge/tests-470+-blue)]()
[![Code Size](https://img.shields.io/badge/code-88.5K%20lines-orange)]()

---

## 🎯 核心特性

### 🌿 Git 分支式上下文管理（核心创新）

像 Git 管理代码分支一样管理 AI 对话上下文，支持 **平行探索**、**多方案对比** 和 **时间旅行**：

```rust
// 创建平行分支探索不同方案
ctx.fork("refactor-approach-1")?;  // 方案 1：增量重构
ctx.fork("refactor-approach-2")?;  // 方案 2：完全重写
ctx.fork("refactor-approach-3")?;  // 方案 3：包装器模式

// 在各分支中独立探索，上下文互不污染
ctx.checkout("refactor-approach-1")?;
// ... 探索方案 1 ...

ctx.checkout("refactor-approach-2")?;
// ... 探索方案 2 ...

// 比较方案差异
ctx.diff("refactor-approach-1", "refactor-approach-2")?;

// 合并最佳方案到主分支
ctx.merge("refactor-approach-1", "main", MergeStrategy::AIAssisted)?;

// 时间旅行：回到历史状态
ctx.time_travel("main", "0xabc123...")?;
```

**核心优势**：
- ✅ **并行探索**：同时探索多个方案，保留对比数据
- ✅ **上下文隔离**：各分支独立，互不污染
- ✅ **AI 辅助合并**：5 种合并策略（FastForward/SelectiveMerge/AIAssisted/Manual/Ours/Theirs）
- ✅ **时间旅行**：精确回溯到历史状态
- ✅ **Copy-on-Write**：O(1) 分支创建，存储开销 <20%
- ✅ **性能实测**：Fork 延迟 ~6ms，Merge 延迟 ~45ms，Checkout 延迟 ~2ms

**典型场景**：
- 📝 **代码重构**：并行探索 3 种重构方案，比较后合并最佳方案
- 🐛 **多假设调试**：为每个 bug 假设创建分支，独立验证
- ⏪ **错误恢复**：回到对话早期状态，重新探索不同路径

**实现状态**: ✅ 完成 (657 行核心代码，46 个测试 100% 通过)

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

### 启动前需要安装

要启动本项目的核心能力（CLI / TUI / MCP），只需要准备下面这些：

1. `Rust 1.75+`
   - 用于编译和运行项目
   - 验证命令：`rustc --version`

2. `Cargo`
   - 通常随 Rust 一起安装
   - 验证命令：`cargo --version`

3. 一个可用的 LLM 接入方式（二选一）
   - 本地方式：安装 `Ollama` 并拉取至少一个模型
   - 远程方式：准备任一受支持提供商的 API Key（OpenAI / Anthropic / Gemini / 智谱 / 月之暗面等）

4. Windows 用户的本地编译工具链（仅 Windows 源码编译需要）
   - 推荐使用 Rust 默认的 MSVC toolchain
   - 如本机缺少链接器，需要安装 Visual Studio C++ Build Tools

下面这些 **不是项目启动前置**：

- Git 上下文增强相关：`Git`（推荐安装，但不是纯启动 CLI/TUI 的硬前置）
- 自然科学验证与科学后端：`Lean4`、`lake`、`Mathlib`、`RDKit`、`Biopython`、`ASE`、`LAMMPS`、`OpenFOAM`、`Psi4`、`Quantum ESPRESSO`
- 额外运行时：`R` / `Rscript`、Python 科学栈、MPI 工具链

也就是说，**不安装自然科学验证工具，本项目仍然可以正常启动和使用核心 Agent 能力**。

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

### 桌面宿主预接入

当前项目还不是完整打包后的桌面应用，但已经支持通过宿主注入运行参数来模拟桌面壳接入：

```bash
$env:TOKITAI_HOST_MODE="desktop"
$env:TOKITAI_FRONTEND_DIR="$PWD\\frontend"
$env:TOKITAI_STATE_DIR="$env:LOCALAPPDATA\\Tokitai\\app-state"
cargo run --release -- --web
```

可选环境变量：

- `TOKITAI_HOST_MODE=desktop|web`
- `TOKITAI_FRONTEND_DIR`
- `TOKITAI_STATE_DIR`
- `TOKITAI_BIND_ADDR`

其中 `desktop` 模式会切换到桌面宿主描述：
- transport: `bridge`
- protocol: `tokitai-host-v1`
- 默认声明支持文件对话框、终端、PTY、原生菜单

### 3. 运行测试

```bash
cargo test
```

### 历史 Session 清洗

如果早期历史会话里已经写入了英文残留、乱码正文、错误标题或错误摘要，可以运行一次性迁移清洗脚本：

```bash
cargo run --bin repair_sessions -- --dry-run
cargo run --bin repair_sessions --
```

可选参数：

- `--dry-run`：只扫描和统计，不改文件
- `--state-dir <path>`：指定包含 `sessions/` 的状态目录

脚本会：

- 清洗历史消息中的乱码/损坏正文
- 尝试恢复常见 mojibake
- 重建 session 标题与摘要
- 同步回写 `index.json`
- 自动备份原始 session 文件到 `sessions/_repair_backup/`

---

## 📊 项目规模

| 指标 | 数值 | 状态 |
|------|------|------|
| **代码行数** | ~88,500 行 Rust | ✅ 实测 |
| **核心模块** | 16 个 | ✅ 完成 |
| **工具箱** | 12 个 | ✅ 完成 |
| **工具函数** | 63+ 个 | ✅ 完成 |
| **LLM 提供商** | 6 个 | ✅ 完成 |
| **工具模板** | 10 个 | ✅ 完成 |
| **创新点** | 7 大核心创新 | ✅ 全部实现 |
| **测试数量** | 470+ 测试 | ✅ 100% 通过 |
| **使用场景** | 21 个详细场景 | 📝 文档完成 |

---

## 🏆 核心创新点

| # | 创新点 | 代码行数 | 实现状态 | 测试状态 | 性能指标 |
|---|--------|----------|----------|----------|----------|
| 1 | **Git 分支式上下文管理** | 657 行 | ✅ 完成 | ✅ 46/46 通过 | Fork ~6ms, Merge ~45ms |
| 2 | **服务化元数据 (Tool-as-a-Service)** | 912 行 | ✅ 完成 | ✅ 已实现 | 工具选择 <50ms |
| 3 | **三源融合依赖图推断** | 544 行 | ✅ 完成 | ✅ 已实现 | 推断准确率 75%+ (预期) |
| 4 | **HybridGapDetector 自主进化** | 1519 行 | ✅ 完成 | ✅ 已实现 | 成本降低 95% (理论推算) |
| 5 | **动态工具注册表** | 736 行 | ✅ 完成 | ✅ 已实现 | 热加载 <100ms |
| 6 | **三层上下文存储** | 449 行 | ✅ 完成 | ✅ 已实现 | 传输量减少 60% (理论推算) |
| 7 | **Skills 文件 (AI 可读说明书)** | 317 行 | ✅ 完成 | ✅ 已实现 | 首次正确率 45%→85% (预期) |

**说明**:
- ✅ **实现状态**: 所有核心创新点均已完成代码实现
- ✅ **测试状态**: 平行上下文模块 46 个测试 100% 通过，其他模块测试持续完善中
- 📊 **性能指标**: 部分指标为基准测试目标或理论推算，实测数据收集进行中

**详细文档**: 
- [docs/INNOVATIONS.md](docs/INNOVATIONS.md) - 7 大核心创新点详解
- [docs/INNOVATION_TO_SCENARIO_MAPPING.md](docs/INNOVATION_TO_SCENARIO_MAPPING.md) - 创新点到使用场景映射
- [docs/PARALLEL_CONTEXT_STATUS_REPORT.md](docs/PARALLEL_CONTEXT_STATUS_REPORT.md) - Git 分支上下文实现报告
- [structure_ensure/SERVICES.md](structure_ensure/SERVICES.md) - 双轨服务架构详解

## 📚 文档导航

### 🚀 快速开始
| 文档 | 说明 |
|------|------|
| [docs/QUICKSTART.md](docs/QUICKSTART.md) | **快速启动指南**（推荐先看） |
| [docs/USER_GUIDE.md](docs/USER_GUIDE.md) | 完整用户指南 |
| [docs/BUILD_CACHE.md](docs/BUILD_CACHE.md) | 编译产物与缓存清理策略 |

### 🎯 核心创新
| 文档 | 说明 |
|------|------|
| [docs/INNOVATIONS.md](docs/INNOVATIONS.md) | **7 大核心创新点详解**（21 个使用场景） |
| [docs/INNOVATION_TO_SCENARIO_MAPPING.md](docs/INNOVATION_TO_SCENARIO_MAPPING.md) | 创新点到使用场景详细映射 |
| [docs/PAPER_SPLITTING_PLAN.md](docs/PAPER_SPLITTING_PLAN.md) | **论文拆分计划**（3 篇顶会论文） |
| [docs/paper_plan/paper_draft_v01.md](docs/paper_plan/paper_draft_v01.md) | Parallel Context Architecture 论文草稿 |

### 📋 项目报告
| 文档 | 说明 |
|------|------|
| [docs/PHASE_1_COMPLETION_REPORT.md](docs/PHASE_1_COMPLETION_REPORT.md) | Phase 1 完成报告 |
| [docs/PARALLEL_CONTEXT_STATUS_REPORT.md](docs/PARALLEL_CONTEXT_STATUS_REPORT.md) | Git 分支上下文实现状态 |
| [structure_ensure/SERVICES.md](structure_ensure/SERVICES.md) | 双轨服务架构详解 |
| [experiments/README.md](experiments/README.md) | 实验框架说明 |

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
│   ├── context/                 # 上下文存储（三层架构 + Git 分支）
│   ├── autonomy/                # 自主进化（HybridGapDetector + Prompt Engineering）
│   ├── orchestrator/            # 编排调度（工作流/角色切换）
│   ├── dialogue/                # 对话状态机
│   ├── observability/           # 可观测性
│   └── prompt_engineering/      # 提示词工程
│
├── docs/                        # 用户文档
├── experiments/                 # 实验框架
│   ├── tasks/                   # 110 个基准测试任务
│   ├── logs/                    # 实验日志目录
│   ├── analysis/                # 分析结果
│   └── scripts/                 # 评估脚本
├── tools/marketplace/templates/ # 10 种工具模板
├── workflows/                   # TOML 工作流定义
└── structure_ensure/            # 架构文档
```

---

## 📦 Crates

本项目包含两个 Rust crates：

| Crate | 描述 | 状态 |
|-------|------|------|
| **tokitai-context** | Git 风格的 AI 对话上下文管理系统 | ✅ 完成 |
| **tokitai-filekv** | 高性能纯文件 KV 存储引擎（独立 crate） | ✅ 完成 |

### tokitai-filekv

**独立 Crate**: [`tokitai-filekv`](https://crates.io/crates/tokitai-filekv) - 源自 tokitai-context 的存储引擎模块，现已独立为可复用的通用 KV 存储库。

**性能表现** (公平对比 RocksDB):
- Bloom Filter 负向查询：**3.97x** 更快
- 全 KV Get (热点缓存)：**9.69x** 更快
- 写入 (64B, WAL): 9% 更快

**测试状态**: 119/119 测试通过，0 编译警告

详细文档见 [crates/tokitai-filekv/README.md](crates/tokitai-filekv/README.md)。

---

## 📈 Phase 1 完成状态

✅ **MP-001**: 多模型支持（6 提供商 + 智能路由 + /model 命令）
✅ **TE-001**: 工具市场（publish/search/install/list + 10 模板）
✅ **DX-001**: TUI 界面（三面板布局 + 状态栏 + 快捷键）
✅ **MCP-001**: MCP 协议（Server + Client 模式）
✅ **PC-001**: Git 分支式上下文管理（fork/checkout/merge/time_travel）
✅ **AG-001**: HybridGapDetector（两阶段缺口检测）

**构建状态**: `cargo build --release` ✅ 通过
**测试状态**: `cargo test` ✅ 470+ 测试通过

---

## 🧪 实验框架

项目提供完整的实验框架用于验证核心创新点的有效性：

### 实验设计
- **5 组对比实验**: Control / Ours-Full / Ours-Single / Ours-NoCoT / Ours-NoFix
- **110 个基准任务**: 覆盖文件操作、代码分析、网络请求、Git 操作、数据处理等
- **30 天自主进化实验**: 验证 HybridGapDetector 和 Prompt Engineering 自进化系统

### 评估指标
- **主要指标**: 任务完成率、平均工具调用次数、用户满意度
- **次要指标**: 缺口检测准确率、工具创建编译通过率、工具使用率
- **成本指标**: API 成本/月、平均生成时间、平均修正次数

### 实验状态
- ✅ 基准测试任务定义完成（110 个）
- ✅ 实验日志系统实现完成
- ✅ 评估脚本准备完成
- ⏳ 实验数据收集中（计划 2026-04 至 2026-06）

**详细实验说明**: [experiments/README.md](experiments/README.md)

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
| **测试框架** | proptest 1.4, mockall 0.12, insta 1.34 |
| **基准测试** | criterion 0.5 |

---

## 📄 许可证

MIT OR Apache-2.0

---

## 🎯 论文计划

项目计划发表 3 篇顶会论文：

| 论文 | 核心创新 | 目标会议 | 状态 |
|------|----------|----------|------|
| **论文 A** | Git 分支式上下文管理 | ACL 2027 | 🟡 初稿完成 |
| **论文 B** | HybridGapDetector | AAAI 2027 | 🟡 实施完成 |
| **论文 C** | Prompt Engineering 自进化 | EMNLP 2027 | 🟡 实施中 |

**详细论文计划**: [docs/PAPER_SPLITTING_PLAN.md](docs/PAPER_SPLITTING_PLAN.md)

---

**最后更新**: 2026-03-27
**版本**: 0.5.0
**项目状态**: Phase 1-5 完成，实验数据收集中
