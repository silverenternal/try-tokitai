# 用户指南

**版本**: 0.4.0  
**最后更新**: 2026-03-25

---

## 目录

1. [CLI 交互模式](#cli-交互模式)
2. [TUI 图形界面](#tui-图形界面)
3. [MCP 协议](#mcp-协议)
4. [自主进化模式](#自主进化模式)
5. [工具市场](#工具市场)
6. [多模型支持](#多模型支持)
7. [工具箱参考](#工具箱参考)
8. [配置指南](#配置指南)
9. [故障排除](#故障排除)

---

## CLI 交互模式

### 启动

```bash
cargo run --release
```

### 基本对话

直接输入问题或指令：

```
👤 你：当前目录有哪些文件
🤖 AI：当前目录包含以下文件...
```

### 文件引用

使用 `@` 符号快速引用文件：

```
👤 你：@README.md 的内容是什么
👤 你：分析 @src/main.rs 的结构
```

### 可用命令

| 命令 | 说明 |
|------|------|
| `help` | 显示帮助 |
| `exit` / `quit` | 退出程序 |
| `/role <name>` | 切换角色（planner/executor/reviewer/researcher） |
| `/optimize` | 优化上下文 |
| `/context` | 显示上下文状态 |
| `/context clear` | 清空对话历史 |
| `/workflow list` | 列出可用工作流 |
| `/workflow start <name>` | 启动工作流 |
| `/model` | 模型管理（见多模型支持章节） |
| `/toolbox` | 显示工具箱状态 |

---

## TUI 图形界面

### 启动

```bash
cargo run --release -- --tui
```

### 界面布局

```
┌─────────────┬──────────────────────────┬─────────────┐
│             │                          │             │
│  工具列表   │      对话区域             │  工具详情   │
│   (20%)     │        (60%)             │   (20%)     │
│             │                          │             │
├─────────────┴──────────────────────────┴─────────────┤
│  状态栏：模型 | Token | 成本 | 延迟 | 工具调用       │
└──────────────────────────────────────────────────────┘
```

### 快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+Q` | 退出 |
| `Ctrl+L` | 清空对话 |
| `Ctrl+H` | 显示帮助 |
| `Ctrl+C` | 中断当前操作 |
| `j/k` | 上下选择工具 |
| `Enter` | 发送消息 |
| `Backspace` | 删除输入 |

---

## MCP 协议

### MCP Server 模式

将本项目的工具暴露为标准 MCP 接口：

```bash
cargo run --release -- --mcp
```

**传输模式**: stdio

**兼容客户端**: Claude Desktop、Windsurf、Cursor 等支持 MCP 的 AI 客户端

### MCP Client 模式

调用外部 MCP Server 的工具：

在代码中使用 `McpClient`：

```rust
use crate::mcp::client::{McpClient, McpServerDescription};

let mut client = McpClient::new();

// 连接外部 MCP Server
client.connect(McpServerDescription {
    name: "filesystem".to_string(),
    description: "File system operations".to_string(),
    endpoint: "http://localhost:8080".to_string(),
    transport: "http".to_string(),
}).await?;

// 调用外部工具
let result = client.call_tool("read_file", json!({
    "path": "/path/to/file"
})).await?;
```

---

## 自主进化模式

### 启动

```bash
cargo run --release -- --autonomous
```

### 工作流程

```
[Planner] 分析项目状态...
[Planner] 发现改进点：修复 Clippy 警告
[Planner] 制定改进计划...

[Executor] 执行任务 1/5: 修复 src/main.rs 的警告
[Executor] 执行任务 2/5: 添加缺失的单元测试
...

[Reviewer] 代码审查通过
[Reviewer] 运行测试... 470/470 passed ✅
[GitWorkflow] 自动提交：fix: resolve Clippy warnings

[Planner] 开始下一轮迭代...
```

### 自主进化能力

| 能力 | 说明 |
|------|------|
| **项目理解** | 分析代码结构、依赖关系、架构模式 |
| **技术债务检测** | 识别代码异味、复杂度高、测试缺失 |
| **自动修复** | 修复 Clippy 警告、格式化代码、补充测试 |
| **Git 集成** | 自动提交变更（可配置为不推送） |

### 安全机制

- ✅ 所有变更先运行测试
- ✅ 测试失败自动回滚
- ✅ 仅修改代码文件，不修改配置
- ✅ 可配置为只读模式（不提交）

---

## 工具市场

### 发布工具

```bash
# 创建工具目录
mkdir -p tools/my-tool/src

# 复制模板
cp tools/marketplace/templates/01-basic-tool.toml tools/my-tool/tool.toml

# 编辑 tool.toml 和 src/lib.rs

# 发布到社区 Registry
cargo run -- tokitai publish my-tool
```

### 搜索工具

```bash
# 搜索社区工具
cargo run -- tokitai search code-analysis

# 搜索结果示例
找到 3 个工具:

1. smart-search - 智能代码搜索
   版本：1.0.0 | 分类：code | 标签：search, code

2. code-complexity - 代码复杂度分析
   版本：0.2.0 | 分类：analysis | 标签：metrics

3. ...
```

### 安装工具

```bash
cargo run -- tokitai install smart-search
```

安装位置：`~/.local/share/tokitai/tools/`

### 列出工具

```bash
cargo run -- tokitai list

# 输出示例
📦 已安装的工具:
   • smart-search
   • code-analyzer
```

### 工具模板

提供 **10 种** 官方模板：

| 模板 | 用途 |
|------|------|
| `01-basic-tool.toml` | 基础工具 |
| `02-network-tool.toml` | 网络工具 |
| `03-file-tool.toml` | 文件操作 |
| `04-ai-tool.toml` | AI 工具 |
| `05-code-analysis-tool.toml` | 代码分析 |
| `06-git-tool.toml` | Git 操作 |
| `07-database-tool.toml` | 数据库 |
| `08-search-tool.toml` | 搜索工具 |
| `09-webhook-tool.toml` | Webhook |
| `10-automation-tool.toml` | 自动化工作流 |

详见：[tools/marketplace/templates/](../tools/marketplace/templates/README.md)

---

## 多模型支持

### 配置提供商

**方式一：环境变量（单提供商）**

```bash
export AI_API_KEY="your-api-key"
export AI_API_URL="https://api.provider.com/v1/chat/completions"
export AI_MODEL="model-name"
```

**方式二：config.toml（多提供商）**

```toml
[ai.providers.openai]
api_key = "sk-..."
model = "gpt-4o"
cost_per_1k_tokens = 0.03
quality_score = 9.0
context_window = 128000

[ai.providers.anthropic]
api_key = "sk-ant-..."
model = "claude-3-5-sonnet-20241022"
cost_per_1k_tokens = 0.03
quality_score = 9.5
context_window = 200000

[ai.providers.ollama]
api_url = "http://localhost:11434/v1/chat/completions"
model = "qwen3.5:397b"
cost_per_1k_tokens = 0.0
quality_score = 6.0
context_window = 32000
```

### 提供商对比

| 提供商 | 模型 | 成本 ($/1K) | 质量 | 延迟 |
|--------|------|------------|------|------|
| OpenAI | gpt-4o | $0.03 | 9.0/10 | 中 |
| Anthropic | claude-3-5-sonnet | $0.03 | 9.5/10 | 中 |
| Gemini | gemini-2.0-flash | $0.00075 | 8.5/10 | 低 |
| 智谱 AI | glm-4 | $0.005 | 7.5/10 | 低 |
| 月之暗面 | moonshot-v1-8k | $0.012 | 7.0/10 | 中 |
| Ollama | qwen3.5:397b | $0.00 | 6.0/10 | 低 |

### 模型路由策略

系统支持 4 种路由策略：

| 策略 | 说明 | 适用场景 |
|------|------|----------|
| **CostOptimized** | 成本优先 | 批量任务 |
| **QualityOptimized** | 质量优先 | 关键任务 |
| **LatencyOptimized** | 延迟优先 | 实时交互 |
| **Balanced** | 平衡模式（默认） | 通用场景 |

---

## 工具箱参考

### file_ops（15 工具）

| 工具 | 说明 |
|------|------|
| `read_file` | 读取文件 |
| `write_file` | 写入文件 |
| `list_dir` | 列出目录 |
| `search_files` | 搜索文件 |
| `copy_file` | 复制文件 |
| `move_file` | 移动文件 |
| `delete_file` | 删除文件 |
| `read_pdf` | 读取 PDF |

### web（20 工具）

| 工具 | 说明 |
|------|------|
| `http_get` | HTTP GET 请求 |
| `http_post` | HTTP POST 请求 |
| `web_search` | 网页搜索 |
| `download_file` | 下载文件 |
| `wikipedia_search` | Wikipedia 搜索 |
| `check_website_status` | 网站状态检查 |

### system（13 工具）

| 工具 | 说明 |
|------|------|
| `execute_command` | 执行命令 |
| `list_processes` | 列出进程 |
| `kill_process` | 终止进程 |
| `get_system_info` | 系统信息 |
| `analyze_code_complexity` | 代码复杂度分析 |

### code（4 工具）

| 工具 | 说明 |
|------|------|
| `analyze_code` | 代码分析 |
| `detect_language` | 语言检测 |
| `find_references` | 查找引用 |
| `extract_symbols` | 提取符号 |

### git（4 工具）

| 工具 | 说明 |
|------|------|
| `git_status` | Git 状态 |
| `git_log` | Git 日志 |
| `git_diff` | Git 差异 |
| `git_branch` | 分支管理 |

### data（5 工具）

| 工具 | 说明 |
|------|------|
| `format_json` | JSON 格式化 |
| `query_json` | JSON 查询 |
| `transform_data` | 数据转换 |
| `validate_json` | JSON 验证 |

### tensor（20+ 工具，实验性）

需启用 `--features tensor`：

| 工具 | 说明 |
|------|------|
| `zeros` | 零张量 |
| `ones` | 一张量 |
| `randn` | 随机张量 |
| `matmul` | 矩阵乘法 |
| `relu` | ReLU 激活 |
| `softmax` | Softmax |

---

## 配置指南

### 环境变量

| 变量 | 说明 | 示例 |
|------|------|------|
| `AI_API_KEY` | API 密钥 | `sk-...` |
| `AI_API_URL` | API 地址 | `https://api.openai.com/v1/chat/completions` |
| `AI_MODEL` | 默认模型 | `gpt-4o` |
| `RUST_LOG` | 日志级别 | `info`, `debug`, `warn` |

### config.toml

```toml
# AI 配置
[ai]
default_provider = "openai"
max_context_length = 4096

# 提供商配置
[ai.providers.openai]
api_key = "sk-..."
model = "gpt-4o"

# 上下文存储配置
[context]
storage_path = ".context"
max_history_size = 1000

# 自主进化配置
[autonomy]
auto_commit = true
auto_push = false
max_iterations = 10
```

---

## 故障排除

### 常见问题

**Q: 提示 "未设置 AI_API_KEY"**

A: 设置环境变量：
```bash
export AI_API_KEY="your-key"
```

**Q: 响应很慢**

A: 
1. 切换到更快的模型
2. 检查网络连接
3. 使用本地 Ollama

**Q: 编译失败**

A: 确保 Rust >= 1.75：
```bash
rustup update
```

**Q: TUI 显示异常**

A: 确保终端支持 Unicode，尝试：
```bash
export TERM=xterm-256color
```

### 日志调试

启用详细日志：

```bash
RUST_LOG=debug cargo run --release
```

日志输出到 stderr，不干扰交互界面。

### 清理缓存

```bash
rm -rf .context .tokitai sandbox downloads
```

---

## 附录

### 项目资源

- 📖 [快速启动](QUICKSTART.md)
- 🏗️ [服务架构](../structure_ensure/SERVICES.md)
- 📝 [更新日志](CHANGELOG.md)

### 社区资源

- 🛠️ [工具模板](../tools/marketplace/templates/README.md)
- 📊 [Phase 1 报告](PHASE_1_COMPLETION_REPORT.md)
- 🎯 [战略计划](STRATEGIC_IMPLEMENTATION_PLAN.json)
