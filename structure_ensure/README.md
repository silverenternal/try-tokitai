# 架构文档索引

**最后更新**: 2026-03-25

---

## 核心文档

| 文档 | 说明 | 推荐阅读 |
|------|------|----------|
| [SERVICES.md](SERVICES.md) | **双轨服务架构详解** | ⭐⭐⭐ |
| [QUICK_REFERENCE.md](QUICK_REFERENCE.md) | 快速参考卡片 | ⭐⭐ |
| [TOOL_SELECTOR_GUIDE.md](TOOL_SELECTOR_GUIDE.md) | 工具选择器指南 | ⭐⭐ |
| [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md) | 完整项目结构 | ⭐ |

---

## 双轨服务架构概览

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

---

## 四种启动模式

| 模式 | 启动命令 | 说明 |
|------|----------|------|
| **CLI AI 助手** | `cargo run --release` | 交互式对话 |
| **TUI 图形界面** | `cargo run --release -- --tui` | 终端图形界面 |
| **MCP Server** | `cargo run --release -- --mcp` | 暴露工具为 MCP 服务 |
| **自主进化** | `cargo run --release -- --autonomous` | AI 自主改进项目 |

---

## 项目结构概览

```
try-tokitai/
├── src/
│   ├── llm/                     # 多模型支持（6 提供商）
│   ├── mcp/                     # MCP 协议
│   ├── tool_market/             # 工具市场
│   ├── tui/                     # TUI 图形界面
│   ├── tools/                   # 工具集合（63+ 工具）
│   ├── tool_matrix/             # 工具矩阵/服务注册表
│   ├── context/                 # 上下文存储
│   ├── autonomy/                # 自主进化
│   ├── orchestrator/            # 编排调度
│   ├── dialogue/                # 对话状态机
│   ├── observability/           # 可观测性
│   └── prompt_engineering/      # 提示词工程
│
├── docs/                        # 用户文档
├── tools/marketplace/templates/ # 工具模板
├── workflows/                   # TOML 工作流
└── structure_ensure/            # 架构文档
```

---

## 其他资源

- 📖 [用户文档索引](../docs/README.md)
- 🚀 [快速启动](../docs/QUICKSTART.md)
- 🛠️ [工具模板](../tools/marketplace/templates/README.md)
