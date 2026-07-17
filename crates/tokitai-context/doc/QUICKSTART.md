# 🚀 快速启动指南

> **最后更新**: 2026-04-04
> **目标**: 5 分钟内启动并运行

---

## 第一步：获取 API Key

本项目支持多个 LLM 提供商，选择一个即可开始：

| 提供商 | 获取地址 | 免费额度 |
|--------|---------|---------|
| **阿里云百炼 (Coding Plan)** | https://bailian.console.aliyun.com | ✅ 有 |
| **Ollama** | https://ollama.com | ✅ 有 |
| **OpenAI** | https://platform.openai.com | ❌ 无 |
| **Anthropic** | https://console.anthropic.com | ❌ 无 |
| **Google Gemini** | https://makersuite.google.com | ✅ 有 |
| **智谱 AI** | https://open.bigmodel.cn | ✅ 有 |
| **月之暗面** | https://platform.moonshot.cn | ✅ 有 |

---

## 第二步：配置环境变量

### 方式一：使用 .env 文件（推荐）

```bash
# 复制模板
cp .env.example .env

# 编辑 .env 文件，填入你的 API Key
```

**`.env` 文件示例**：
```bash
# 阿里云百炼 AI 编码套餐
ALIYUN_CODING_PLAN_API_KEY="your-coding-plan-api-key"
# 或
DASHSCOPE_API_KEY="your-dashscope-api-key"

# OpenAI
OPENAI_API_KEY="sk-xxxxxxxxxxxxxxxx"

# Anthropic
ANTHROPIC_API_KEY="sk-ant-xxxxxxxxxxxxxxxx"

# Ollama（本地部署）
OLLAMA_BASE_URL="http://localhost:11434"
OLLAMA_MODEL="llama3.1"
```

### 方式二：使用环境变量

```bash
# 阿里云百炼
export ALIYUN_CODING_PLAN_API_KEY="your-api-key"

# OpenAI
export OPENAI_API_KEY="sk-xxxxxxxxxxxxxxxx"

# Anthropic
export ANTHROPIC_API_KEY="sk-ant-xxxxxxxxxxxxxxxx"

# Ollama
export OLLAMA_BASE_URL="http://localhost:11434"
export OLLAMA_MODEL="llama3.1"
```

---

## 第三步：启动程序

```bash
# 编译并启动（首次需要编译，约 1-2 分钟）
cargo run --release
```

启动后看到以下提示表示成功：
```
🚀 AI Assistant 启动中...
👤 你：
```

---

## 第四步：开始对话

输入任意问题即可开始：

```
👤 你：当前目录有哪些文件
🤖 AI：当前目录包含以下文件...

👤 你：读取 README.md 的内容
🤖 AI：README.md 的内容如下...

👤 你：帮我创建一个新文件 test.txt，写入 Hello World
🤖 AI：已创建文件 test.txt...
```

---

## 其他启动模式

### TUI 图形界面模式

```bash
cargo run --release -- --tui
```

**快捷键**:
- `Ctrl+Q` - 退出
- `Ctrl+L` - 清空对话
- `j/k` - 上下选择
- `Enter` - 发送消息

### MCP Server 模式

```bash
cargo run --release -- --mcp
```

将所有工具暴露为 MCP 标准接口，供其他 AI 客户端调用。

### 自主进化模式

```bash
cargo run --release -- --autonomous
```

AI 将自主分析项目、发现改进点、执行修复并提交。

---

## 多模型支持

### 配置多提供商

编辑 `config.toml`：

```toml
[ai.providers.openai]
api_key = "sk-..."
model = "gpt-4o"
cost_per_1k_tokens = 0.03
quality_score = 9.0

[ai.providers.anthropic]
api_key = "sk-ant-..."
model = "claude-3-5-sonnet-20241022"
cost_per_1k_tokens = 0.03
quality_score = 9.5

[ai.providers.ollama]
api_url = "http://localhost:11434/v1/chat/completions"
model = "qwen3.5:397b"
cost_per_1k_tokens = 0.0
quality_score = 6.0
```

### 切换模型

在 CLI 中使用 `/model` 命令：

```bash
/model list          # 列出所有可用模型
/model switch openai # 切换到 OpenAI
/model benchmark     # 运行基准测试
/model stats         # 显示使用统计
```

---

## 工具市场命令

```bash
# 查看帮助
cargo run -- tokitai

# 搜索工具
cargo run -- tokitai search code-analysis

# 安装工具
cargo run -- tokitai install smart-search

# 列出已安装工具
cargo run -- tokitai list
```

---

## 常见问题

### Q: 编译失败怎么办？

确保 Rust 版本 >= 1.75：
```bash
rustc --version
rustup update
```

### Q: 响应很慢怎么办？

1. 切换到更快的模型：`export AI_MODEL="qwen2.5:7b"`
2. 使用本地 Ollama：`export AI_API_URL="http://localhost:11434/v1/chat/completions"`

### Q: 如何退出程序？

输入 `exit` 或 `quit`，或按 `Ctrl+C`

### Q: 如何清空对话历史？

输入 `/context clear` 或重启程序

---

## 下一步

- 📖 [完整用户指南](USER_GUIDE.md) - 深入了解所有功能
- 🏗️ [服务架构说明](../structure_ensure/SERVICES.md) - 了解双轨架构
- 🛠️ [工具模板](../tools/marketplace/templates/) - 创建自定义工具

---

**提示**: 运行时会自动创建以下文件夹（已在 `.gitignore` 中）：
- `.context/` - 上下文存储
- `.tokitai/` - 运行时数据
- `sandbox/` - 沙箱测试目录
