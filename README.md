# AI Assistant powered by Tokitai

一个使用 Rust 和 Tokitai 构建的强大 AI 助手，可以让 AI 调用各种工具来完成实际任务。

## ✨ 核心特性

- **📁 纯文件上下文存储**：无数据库依赖，三层存储架构（瞬时/短期/长期），自动裁剪，哈希去重
- **🔒 安全沙箱**：路径验证、命令黑名单、SSRF 防护、内网 IP 过滤
- **🛠️ 丰富工具集**：文件操作、网络请求、代码分析、Git 操作、进程管理
- **🚀 极致性能**：
  - 缓存响应延迟 **<10ms**（50x 提升）
  - 首次请求延迟 **降低 50%**（2x 提升）
  - 流式首字节延迟 **降低 60-70%**（2-3x 提升）
  - 缓存容量 **200 条目**，TTL **5 分钟**，避免过期数据
  - 全局 HTTP 连接池复用，零连接开销
  - 纯异步线程模型（`tokio::spawn`），无线程阻塞
  - 实时延迟监控，平均延迟指标可视化
- **📊 增量日志**：所有上下文变更可追溯、可审计

---

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

#### 一键启动演示

```bash
./demo.sh
```

---

## 功能特性

### 🛠️ 工具系统

基于 [tokitai](https://github.com/silverenternal/tokitai) 库实现编译时工具定义：

- **文件操作** - 读取/写入文件、列出目录、复制/删除文件
- **系统命令** - 执行 shell 命令（带安全检查）、获取环境变量、获取当前目录、获取系统信息
- **代码分析** - 统计代码行数、查找函数定义、检测编程语言
- **网络搜索** - 搜索网页内容、获取 URL 内容
- **文件下载** - 下载网络文件、下载 arXiv 论文、搜索 arXiv 论文
- **Git 操作** - 查看 git 状态、git 日志、git 分支信息
- **HTTP 客户端** - 发送 HTTP GET/POST 请求、检查 URL 可用性、下载文件（带 SSRF 防护）
- **JSON 处理** - 格式化/压缩 JSON、JSONPath 查询、JSON 验证、合并、JSON 转 CSV
- **文件搜索** - grep 文本搜索（支持正则）、递归查找文件、统计文件类型、查找大文件
- **进程管理** - 列出进程、查看进程详情、搜索进程、系统资源监控（带权限检查）
- **网络工具** - Ping 测试、TCP/UDP 端口检查、端口扫描、路由追踪、获取公网 IP

> 💡 **安全增强**：所有工具均经过安全加固，包括输入验证、SSRF 防护、符号链接循环检测、递归深度限制、速率限制等。

### 🔐 命令执行安全机制

为了保护系统安全，命令执行功能实现了多层安全防护：

- **黑名单机制**：禁止执行危险命令（如 `rm`, `sudo`, `kill`, `chmod` 等）
- **安全命令模式**：`run_safe_command` 只能执行只读命令
- **确认机制**：执行非黑名单命令时需要 `confirmed=true` 参数
- **系统信息隔离**：无法访问敏感目录（`/etc`, `/root`, `/proc` 等）

#### 黑名单命令列表

以下命令被禁止执行：
- 文件操作：`rm`, `dd`, `shred`
- 磁盘操作：`mkfs`, `fdisk`, `parted`
- 权限修改：`chmod`, `chown`, `chgrp`
- 提权命令：`sudo`, `su`, `pkexec`, `doas`
- 网络工具：`wget`, `curl`, `nc`, `netcat`, `telnet`, `ssh`, `scp`, `rsync`
- 进程控制：`kill`, `pkill`, `killall`, `xkill`
- 系统控制：`shutdown`, `reboot`, `halt`, `poweroff`, `init`
- 挂载操作：`mount`, `umount`, `losetup`
- 防火墙：`iptables`, `firewall-cmd`, `ufw`, `nft`
- 用户管理：`visudo`, `passwd`, `useradd`, `userdel`, `usermod`, `groupadd`, `groupdel`, `groupmod`
- 内核模块：`insmod`, `rmmod`, `modprobe`

### 📎 @ 路径引用功能

快速引用文件内容，让 AI 直接读取和分析：

- **单个文件**：`@README.md 的内容是什么`
- **代码分析**：`分析 @src/main.rs 的结构`
- **多个文件**：`@file1.txt @file2.txt 比较这两个文件`
- **相对路径**：`@./config.toml 的配置项有哪些`

> 💡 提示：使用 `@` 符号后跟文件路径，系统会自动读取文件内容并附加到问题中。

### 🤖 AI 集成

- 支持 OpenAI 兼容 API（Ollama Cloud、OpenAI、Azure 等）
- 自动工具调用（Function Calling）
- 多轮对话历史记忆

---

## 💬 交互命令

| 命令 | 说明 |
|------|------|
| `help` | 显示可用操作列表 |
| `exit` / `quit` | 退出程序 |
| 任意自然语言 | 与 AI 对话 |
| `@<路径>` | 快速引用文件（如 `@README.md`） |

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

### 4. 执行安全命令
```
👤 你：运行 cargo --version
👤 你：查看当前目录的文件列表
👤 你：显示系统信息
👤 你：检查 python 命令是否可用
👤 你：列出系统中可用的命令
```

### 5. 分析代码
```
👤 你：分析 src/main.rs 的结构
```

### 6. 多步骤任务
```
👤 你：帮我看看 Cargo.toml 的内容，然后统计一下有多少行
```

### 7. 使用 @ 引用文件
```
👤 你：@README.md 的内容是什么
👤 你：分析 @src/main.rs 的结构
👤 你：@file1.txt @file2.txt 比较这两个文件
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
│   ├── command_resolver.rs # 命令解析器（黑名单/白名单）
│   ├── path_resolver.rs    # 路径解析器
│   ├── context/            # 纯文件上下文存储系统（NEW）
│   │   ├── mod.rs          # 模块导出
│   │   ├── file_service.rs # 核心服务 trait 及实现
│   │   ├── hash_index.rs   # 哈希索引（符号链接映射）
│   │   ├── layers.rs       # 三层存储管理（瞬时/短期/长期）
│   │   └── logger.rs       # 增量日志系统
│   ├── tools/              # 工具模块（分类组织）
│   │   ├── mod.rs          # 模块导出
│   │   ├── io/             # I/O 相关工具
│   │   │   ├── mod.rs
│   │   │   ├── file_ops.rs     # 文件操作（读/写/复制/删除）
│   │   │   └── file_search.rs  # 文件搜索（grep/查找）
│   │   ├── network/        # 网络相关工具
│   │   │   ├── mod.rs
│   │   │   ├── http_client.rs  # HTTP 客户端（SSRF 防护）
│   │   │   ├── web_search.rs   # 网络搜索
│   │   │   ├── download.rs     # 文件下载
│   │   │   └── network_tools.rs # 网络工具（ping/端口扫描）
│   │   ├── system/         # 系统相关工具
│   │   │   ├── mod.rs
│   │   │   ├── system.rs       # 系统命令/信息
│   │   │   ├── process_tools.rs # 进程管理
│   │   │   └── code_analysis.rs # 代码分析
│   │   ├── data/           # 数据处理工具
│   │   │   ├── mod.rs
│   │   │   └── json_tools.rs   # JSON 处理
│   │   └── vcs/            # 版本控制工具
│   │       ├── mod.rs
│   │       └── git_ops.rs      # Git 操作
├── examples/               # 示例代码
├── CONTEXT_STORAGE.md      # 上下文存储系统详细文档
└── README.md
```

### 运行时文件夹（已添加到 .gitignore）

以下文件夹在运行时自动创建，已添加到 `.gitignore` 中，不会被提交到版本控制：

| 文件夹 | 用途 | 说明 |
|--------|------|------|
| `sandbox/` | 沙箱测试目录 | 用于测试文件操作、项目模板等功能 |
| `downloads/` | 下载文件目录 | 使用下载工具时，文件默认保存到此目录 |
| `.context/` | 上下文存储 | 三层存储架构（瞬时/短期/长期）的持久化数据 |
| `.tokitai/` | 运行时数据 | 对话状态、追踪日志、自主进化数据等 |

> 💡 **提示**：这些文件夹会在首次运行程序时自动创建，无需手动创建。如需清理缓存，可直接删除这些文件夹。

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
- **tracing** - 日志和追踪

---

## 文档

### 入门文档
| 文档 | 说明 |
|------|------|
| [docs/QUICKSTART.md](docs/QUICKSTART.md) | 快速启动指南 |
| [docs/USER_GUIDE.md](docs/USER_GUIDE.md) | 完整用户指南 |
| [docs/DEMO.md](docs/DEMO.md) | 演示指南 |
| [docs/CHANGELOG.md](docs/CHANGELOG.md) | 更新日志 |

### 开发者文档
| 文档 | 说明 |
|------|------|
| [structure_ensure/README.md](structure_ensure/README.md) | 结构文档索引 |
| [structure_ensure/QUICK_REFERENCE.md](structure_ensure/QUICK_REFERENCE.md) | 快速参考卡片 |
| [structure_ensure/PROJECT_STRUCTURE.md](structure_ensure/PROJECT_STRUCTURE.md) | 完整项目结构详解 |

### 技术报告
| 文档 | 说明 |
|------|------|
| [docs/archive/](docs/archive/) | 技术报告归档（集成/优化/审查报告） |

---

## 许可证

MIT OR Apache-2.0

## 致谢

- [tokitai](https://github.com/silverenternal/tokitai) - 优秀的 AI 工具集成框架
