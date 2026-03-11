# AI Assistant powered by Tokitai

一个使用 Rust 和 Tokitai 构建的强大 AI 助手，可以让 AI 调用各种工具来完成实际任务。

## 🚀 快速开始

### 1️⃣ 获取 API Key（首次使用必读）

本项目使用 **Ollama Cloud** 作为默认 AI 服务，需要 API Key 才能运行。

#### 如何获取 Ollama API Key：

1. **访问官网**：打开 https://ollama.com
2. **注册/登录**：点击右上角 "Sign In"，使用 GitHub 或邮箱注册账号
3. **进入设置页面**：登录后点击右上角头像 → "API Keys"
4. **创建新 Key**：点击 "Create API Key" 按钮
5. **复制 Key**：生成的 Key 格式类似 `ollama-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx`
6. **保存 Key**：⚠️ **立即复制并妥善保存**（页面刷新后将无法再次查看完整 Key）

> 💡 **提示**：Ollama Cloud 目前提供免费额度，足够个人开发和测试使用。

#### 可选：使用其他 AI 服务

| 服务商 | API URL | 说明 |
|--------|---------|------|
| Ollama Cloud | `https://ollama.com/v1/chat/completions` | 默认推荐，支持多种开源模型 |
| OpenAI | `https://api.openai.com/v1/chat/completions` | 需要 OpenAI 账号 |
| Azure OpenAI | `https://YOUR_RESOURCE.openai.azure.com/openai/deployments/YOUR_DEPLOYMENT/chat/completions` | 需要 Azure 账号 |

---

### 2️⃣ 配置环境变量

#### 方法一：临时设置（当前终端会话有效）

```bash
# 设置 API Key（替换为你的真实 Key）
export AI_API_KEY="ollama-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"

# 设置 API URL（可选，默认使用 Ollama Cloud）
export AI_API_URL="https://ollama.com/v1/chat/completions"

# 设置模型（可选，默认使用 qwen3.5:397b）
export AI_MODEL="qwen3.5:397b"
```

#### 方法二：永久设置（推荐）

```bash
# 复制示例文件
cp .env.example .env

# 编辑 .env 文件，填入你的 API Key
nano .env  # 或使用你喜欢的编辑器

# 文件内容示例：
# AI_API_KEY=ollama-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
# AI_API_URL=https://ollama.com/v1/chat/completions
# AI_MODEL=qwen3.5:397b
```

---

### 3️⃣ 启动程序

#### 命令行模式

```bash
cargo run --release
```

#### TUI 界面模式（⚠️ 实验性功能）

> **注意**：TUI 功能目前处于实验性阶段，可能存在不稳定的情况。

```bash
# 使用命令行参数启动
cargo run --release -- --tui

# 或使用短选项
cargo run --release -- -t
```

#### 一键启动演示

```bash
./demo.sh
```

---

## 功能特性

### 🛠️ 工具系统

基于 [tokitai](https://github.com/silverenternal/tokitai) 库实现编译时工具定义：

- **文件操作** - 读取/写入文件、列出目录、复制/删除文件
- **系统命令** - 执行 shell 命令、获取环境变量、获取当前目录
- **代码分析** - 统计代码行数、查找函数定义、检测编程语言
- **网络搜索** - 搜索网页内容、获取 URL 内容
- **文件下载** - 下载网络文件、下载 arXiv 论文、搜索 arXiv 论文
- **Git 操作** - 查看 git 状态、git 日志、git 分支信息

### 🤖 AI 集成

- 支持 OpenAI 兼容 API（Ollama Cloud、OpenAI、Azure 等）
- 自动工具调用（Function Calling）
- 多轮对话历史记忆

### 🖥️ TUI 界面（实验性）

- 现代化的终端用户界面
- 消息历史滚动浏览
- 流式响应显示
- 快捷键支持（PageUp/PageDown 快速滚动、Ctrl+L 清除历史等）

> **注意**：TUI 功能目前处于实验性阶段，推荐使用稳定的命令行模式。

---

## 💬 交互命令

### 命令行模式

| 命令 | 说明 |
|------|------|
| `help` | 显示可用操作列表 |
| `exit` / `quit` | 退出程序 |
| 任意自然语言 | 与 AI 对话 |

### TUI 界面模式

> ⚠️ **实验性功能**：TUI 界面目前处于实验性阶段，可能存在不稳定的情况。

| 快捷键 | 说明 |
|--------|------|
| `Enter` | 发送消息 |
| `↑` / `↓` | 滚动消息历史 |
| `PageUp` / `PageDown` | 快速滚动 |
| `End` | 滚动到底部 |
| `Ctrl+L` | 清除历史记录 |
| `Ctrl+C` / `Ctrl+Q` | 退出程序 |

---

## 📋 演示示例

### 1. 查看帮助
```
👤 你：help
```

### 2. 查看目录
```
👤 你：当前目录有哪些文件
```

### 3. 读取文件
```
👤 你：读取 README.md 的内容
```

### 4. 执行命令
```
👤 你：运行 cargo --version
```

### 5. 分析代码
```
👤 你：分析 src/main.rs 的结构
```

### 6. 多步骤任务
```
👤 你：帮我看看 Cargo.toml 的内容，然后统计一下有多少行
```

---

## 项目结构

```
.
├── Cargo.toml              # 项目依赖配置
├── demo.sh                 # 一键启动脚本
├── src/
│   ├── main.rs             # 主程序入口
│   ├── config.rs           # 配置管理
│   ├── sandbox.rs          # 沙箱模块
│   ├── tools/              # 工具模块
│   │   ├── mod.rs          # 模块导出
│   │   ├── file_ops.rs     # 文件操作工具
│   │   ├── system.rs       # 系统命令工具
│   │   ├── code_analysis.rs # 代码分析工具
│   │   ├── web_search.rs   # 网络搜索工具
│   │   ├── download.rs     # 文件下载工具
│   │   └── git_ops.rs      # Git 操作工具
│   └── tui/                # TUI 界面模块（实验性）
│       ├── mod.rs          # 模块导出
│       ├── app.rs          # 应用状态管理
│       ├── ui.rs           # UI 渲染
│       ├── event.rs        # 事件处理
│       ├── api_client.rs   # API 客户端
│       └── assistant.rs    # AI 助手集成
├── examples/               # 示例代码
└── README.md
```

---

## 环境变量

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| `AI_API_URL` | AI API 地址 | `https://ollama.com/v1/chat/completions` |
| `AI_API_KEY` | API 密钥（**必填**） | 无 |
| `AI_MODEL` | 模型名称 | `qwen3.5:397b` |

---

## ❓ 常见问题

### Q: 提示 "未设置 AI_API_KEY" 怎么办？

A: 你需要先获取 Ollama API Key，参考上方「获取 API Key」步骤。

### Q: API Key 安全吗？会上传到服务器吗？

A: API Key 仅保存在本地 `.env` 文件中，不会上传到任何第三方服务器（除了你配置的 AI 服务提供商）。

### Q: 可以使用本地 Ollama 服务吗？

A: 可以。如果你本地运行了 Ollama 服务，设置：
```bash
export AI_API_URL="http://localhost:11434/v1/chat/completions"
```

### Q: 模型响应很慢怎么办？

A: 尝试切换到较小的模型：
```bash
export AI_MODEL="qwen2.5:7b"
```

### Q: TUI 界面显示异常怎么办？

A: TUI 目前处于实验性阶段，如遇问题请：
1. 确保终端支持 UTF-8 编码
2. 尝试调整终端窗口大小
3. 使用命令行模式（更稳定）

---

## 可用模型

在 Ollama Cloud 上可用的模型：
- `qwen3.5:397b` - 通义千问 3.5（397B 参数）
- `qwen3-coder:480b` - 通义千问代码版（480B 参数）
- `deepseek-v3.2` - DeepSeek V3.2
- `gemma3` 系列 - Google Gemma 3

---

## 技术栈

- **Rust** - 系统编程语言
- **tokitai** - AI 工具集成框架
- **reqwest** - HTTP 客户端
- **serde_json** - JSON 处理
- **anyhow** - 错误处理
- **ratatui** - TUI 框架（实验性）
- **crossterm** - 终端操作（实验性）

---

## 许可证

MIT OR Apache-2.0

## 致谢

- [tokitai](https://github.com/silverenternal/tokitai) - 优秀的 AI 工具集成库
