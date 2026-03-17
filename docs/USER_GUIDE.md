# tokitai 用户指南

**版本**: 2.0.0
**最后更新**: 2026-03-18
**测试状态**: 236/236 通过 ✅

---

## 目录

1. [快速入门](#快速入门)
2. [配置指南](#配置指南)
3. [使用指南](#使用指南)
4. [工具系统](#工具系统)
5. [多模型支持](#多模型支持)
6. [上下文存储](#上下文存储)
7. [安全特性](#安全特性)
8. [故障排除](#故障排除)
9. [最佳实践](#最佳实践)

---

## 快速入门

### 1. 获取 API Key

**方式一：Ollama Cloud（推荐）**
```bash
# 访问 https://ollama.com 获取 Key
export AI_API_KEY="ollama-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
```

**方式二：Qwen OAuth（免费额度）**
```bash
cargo run --release -- --qwen-login
```

**方式三：其他服务商**

| 服务商 | 获取 Key 地址 |
|--------|--------------|
| OpenAI | https://platform.openai.com |
| Anthropic | https://console.anthropic.com |
| Google Gemini | https://makersuite.google.com |
| 阿里云百炼 | https://dashscope.console.aliyun.com |

### 2. 配置环境变量

```bash
# 临时设置（当前终端会话有效）
export AI_API_KEY="your-api-key"
export AI_MODEL="qwen3-coder-plus"

# 永久设置（复制示例文件）
cp .env.example .env
# 编辑 .env 文件，填入配置
```

### 3. 启动程序

```bash
cargo run --release
```

---

## 配置指南

### 环境变量

| 变量名 | 说明 | 默认值 | 示例 |
|--------|------|--------|------|
| `AI_API_KEY` | AI API 密钥 | 必填 | `ollama-xxx` |
| `AI_MODEL` | 默认模型 | `qwen3-coder-plus` | `gpt-4o` |
| `AI_API_URL` | API 端点 | Ollama 默认 | `https://api.openai.com/v1/chat/completions` |
| `AI_IDENTITY` | 身份模式 | `assistant` | `developer` |
| `TOKITAI_CONFIG` | 配置文件路径 | `./config.toml` | `/etc/tokitai/config.toml` |

### 配置文件 (config.toml)

```toml
# 模型配置
[model]
provider = "openai"  # openai / anthropic / gemini / qwen
default_model = "qwen3-coder-plus"
max_tokens = 4096
temperature = 0.7

# 上下文存储配置
[context]
storage_type = "file"  # file / memory
cache_dir = ".tokitai"
max_cache_size_mb = 100

# 安全配置
[security]
enable_sandbox = true
allowed_hosts = ["api.openai.com", "api.anthropic.com"]
blocked_paths = ["/etc", "/root", ".ssh"]

# 性能配置
[performance]
enable_cache = true
cache_ttl_seconds = 3600
max_concurrent_requests = 10
```

---

## 使用指南

### 基本对话

```
👤 你：你好，请介绍一下自己
🤖 AI：我是 tokitai AI 助手，一个基于 Rust 构建的高性能 AI 助手...
```

### @路径引用

引用文件内容进行分析：

```
👤 你：@README.md 的主要内容是什么？
🤖 AI：[调用 read_file] README.md 包含以下内容...

👤 你：分析 @src/main.rs 的代码结构
🤖 AI：[调用 find_functions] main.rs 包含以下函数...

👤 你：@file1.txt @file2.txt 比较这两个文件的差异
🤖 AI：[调用 read_file x2] 两个文件的主要差异是...
```

### 工具调用

AI 会自动选择合适的工具完成任务：

```
👤 你：帮我看看 config.toml 的配置项
🤖 AI：[调用 read_file] config.toml 包含以下配置...

👤 你：检查 Git 状态
🤖 AI：[调用 git_status] 当前 Git 状态如下...

👤 你：下载并分析 https://example.com/data.json
🤖 AI：[调用 download_and_analyze] 已下载并分析文件...
```

### 多轮对话

AI 会记住上下文进行多轮对话：

```
👤 你：当前目录有哪些文件？
🤖 AI：[调用 list_dir] 当前目录有以下内容...

👤 你：其中最大的文件是哪个？
🤖 AI：[调用 get_file_info] 最大的文件是...

👤 你：删除它
🤖 AI：[调用 delete_file] 已删除文件。请注意，这个操作不可逆。
```

---

## 工具系统

tokitai 提供 50+ 工具，分为以下类别：

### 文件操作

| 工具 | 说明 | 示例 |
|------|------|------|
| `read_file` | 读取文件 | `read_file("config.toml")` |
| `write_file` | 写入文件 | `write_file("output.txt", content)` |
| `copy_file` | 复制文件 | `copy_file("src", "dst")` |
| `delete_file` | 删除文件 | `delete_file("temp.txt")` |
| `list_dir` | 列出目录 | `list_dir(".")` |

### 系统命令

| 工具 | 说明 | 示例 |
|------|------|------|
| `run_command` | 执行命令 | `run_command("ls -la")` |
| `get_env_vars` | 获取环境变量 | `get_env_vars()` |
| `get_current_dir` | 当前目录 | `get_current_dir()` |
| `get_system_info` | 系统信息 | `get_system_info()` |

### 代码分析

| 工具 | 说明 | 示例 |
|------|------|------|
| `count_lines` | 统计代码行数 | `count_lines("src/")` |
| `find_functions` | 查找函数定义 | `find_functions("main.rs")` |
| `detect_language` | 检测编程语言 | `detect_language("src/")` |

### 网络搜索

| 工具 | 说明 | 示例 |
|------|------|------|
| `web_search` | 网络搜索 | `web_search("Rust async")` |
| `search_images` | 图片搜索 | `search_images("Rust logo")` |
| `fetch_url` | 获取网页 | `fetch_url("https://example.com")` |

### Git 操作

| 工具 | 说明 | 示例 |
|------|------|------|
| `git_status` | Git 状态 | `git_status()` |
| `git_log` | Git 日志 | `git_log(10)` |
| `git_branch` | Git 分支 | `git_branch()` |
| `git_diff` | Git 差异 | `git_diff("HEAD~1")` |

### HTTP 客户端

| 工具 | 说明 | 示例 |
|------|------|------|
| `http_get` | GET 请求 | `http_get("https://api.example.com")` |
| `http_post` | POST 请求 | `http_post(url, body)` |
| `check_url` | 检查 URL | `check_url("https://example.com")` |

### JSON 处理

| 工具 | 说明 | 示例 |
|------|------|------|
| `format_json` | 格式化 JSON | `format_json(json_str)` |
| `minify_json` | 压缩 JSON | `minify_json(json_str)` |
| `query_json` | 查询 JSON | `query_json(json, "$.data")` |
| `validate_json` | 验证 JSON | `validate_json(json_str)` |

### 文件搜索

| 工具 | 说明 | 示例 |
|------|------|------|
| `grep` | 文本搜索 | `grep("pattern", "src/")` |
| `find_files` | 查找文件 | `find_files("*.rs")` |
| `count_file_types` | 统计文件类型 | `count_file_types(".")` |

### 进程管理

| 工具 | 说明 | 示例 |
|------|------|------|
| `list_processes` | 列出进程 | `list_processes()` |
| `get_process_info` | 进程信息 | `get_process_info(pid)` |
| `get_system_resources` | 系统资源 | `get_system_resources()` |

### 网络工具

| 工具 | 说明 | 示例 |
|------|------|------|
| `ping_host` | Ping 测试 | `ping_host("8.8.8.8")` |
| `check_tcp_port` | 端口检查 | `check_tcp_port("localhost", 8080)` |
| `scan_common_ports` | 端口扫描 | `scan_common_ports("localhost")` |

---

## 多模型支持

### 支持的模型提供商

| 提供商 | 支持模型 | 免费额度 | 配置方式 |
|--------|---------|---------|---------|
| **OpenAI 兼容** | Qwen3.5, GPT-4o, GLM-4 | 取决于 API 商 | `export AI_API_URL="..."` |
| **Anthropic** | Claude Sonnet 4, Claude Opus 4 | ❌ | `provider = "anthropic"` |
| **Google Gemini** | Gemini 2.5 Pro, Gemini 2.5 Flash | ✅ 有限免费 | `provider = "gemini"` |
| **Qwen OAuth** | Qwen3-Coder-Plus, Qwen3.5-Plus | ✅ 每日 1000 次 | `--qwen-login` |

### 切换模型

**命令行**: 设置环境变量
```bash
export AI_MODEL="claude-sonnet-4-20250514"
```

**配置文件**:
```toml
[model]
provider = "anthropic"
default_model = "claude-sonnet-4-20250514"
```

---

## 上下文存储

### 三层存储架构

1. **瞬时层**: 当前会话的临时上下文，会话结束清除
2. **短期层**: 最近 N 次会话的上下文，自动裁剪
3. **长期层**: 重要知识和约定，永久保存

### 存储位置

```
.tokitai/
├── context/          # 上下文缓存
├── sessions/         # 会话历史
├── reports/          # 合规报告
└── project_conventions.md  # 项目约定
```

### 缓存管理

```bash
# 手动删除缓存目录
rm -rf .tokitai/context/
```

---

## 安全特性

### 文件沙箱

- 仅允许访问项目目录内的文件
- 阻止访问敏感路径：`/etc`, `/root`, `.ssh`
- 路径遍历检测：阻止 `../` 攻击

### 命令沙箱

- 40+ 命令白名单
- 20+ 危险模式检测（`rm -rf`, `dd`, `chmod 777`）
- 命令参数验证

### 网络沙箱

- 20+ 白名单主机
- 9 个内网 IP 段阻止（SSRF 防护）
- 重定向检查

### 身份系统

7 种预定义身份，每种身份有不同的工具权限：

| 身份 | 权限范围 | 代表工具 |
|------|---------|---------|
| `assistant` | 只读工具 | read_file, list_dir |
| `developer` | 开发工具 | run_command, git_* |
| `researcher` | 搜索工具 | web_search, fetch_url |
| `analyst` | 分析工具 | count_lines, find_functions |
| `operator` | 系统工具 | list_processes, get_system_info |
| `auditor` | 审计工具 | 所有只读工具 |
| `admin` | 全部工具 | 所有工具 |

切换身份：
```bash
export AI_IDENTITY="developer"
```

---

## 故障排除

### 常见问题

#### Q: 提示 "未设置 API Key"

**A**: 获取 API Key 并设置：
```bash
export AI_API_KEY="your-api-key"
```

或使用 Qwen OAuth：
```bash
cargo run --release -- --qwen-login
```

#### Q: API Key 安全吗？

**A**: API Key 仅保存在本地 `.env` 文件中，不会上传到第三方服务器。

#### Q: 可以使用本地 Ollama 服务吗？

**A**: 可以。设置：
```bash
export AI_API_URL="http://localhost:11434/v1/chat/completions"
```

#### Q: 工具调用失败

**A**: 
1. 检查身份权限是否足够
2. 检查文件路径是否在允许范围内
3. 查看错误日志获取详细信息

#### Q: 性能缓慢

**A**:
1. 启用缓存：`enable_cache = true`
2. 检查网络连接
3. 降低 `max_tokens` 配置

---

## 最佳实践

### 1. 使用 @路径引用

直接引用文件比复制粘贴更高效：
```
# 推荐
@src/main.rs 分析这个文件的结构

# 不推荐
[粘贴整个文件内容]
```

### 2. 增量确认

对于重要操作，使用增量确认：
```
请先分析代码，我会审查后再执行修改
```

### 3. 明确任务边界

清晰描述任务范围：
```
# 推荐
只读取文件，不要修改

# 不推荐
看看这个文件
```

### 4. 使用合适的身份

根据任务选择身份：
```bash
# 只读分析
export AI_IDENTITY="analyst"

# 开发任务
export AI_IDENTITY="developer"
```

### 5. 定期清理缓存

```bash
# 每周清理一次
rm -rf .tokitai/context/
```

### 6. 使用工具链

复杂任务使用工具链：
```
# 推荐
请执行 download_and_analyze https://example.com/data.json

# 不推荐
先下载，再读取，再分析...
```

---

## 附录

### 相关文档

- [开发者指南](docs/developer-guide/tools-guide.md)
- [多模型配置](docs/user-guide/providers.md)
- [上下文存储详解](docs/developer-guide/context-storage.md)
- [性能基准报告](docs/BENCHMARK_REPORT.md)

### 获取帮助

- GitHub Issues: https://github.com/silverenternal/tokitai/issues
- 文档：docs/README.md
