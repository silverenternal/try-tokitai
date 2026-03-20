# Tokitai 功能说明文档

> **版本**: 3.0.0
> **最后更新**: 2026-03-18
> **测试状态**: 236/236 通过 ✅
> **代码规模**: ~27,500 行 Rust | 99 个源文件 | 10 个核心模块

---

## 📋 目录

1. [产品概述](#产品概述)
2. [核心功能](#核心功能)
3. [工具系统](#工具系统)
4. [双轨服务模式](#双轨服务模式)
5. [用户交互](#用户交互)
6. [安全特性](#安全特性)
7. [性能特性](#性能特性)
8. [使用场景](#使用场景)
9. [最佳实践](#最佳实践)
10. [常见问题](#常见问题)

---

## 产品概述

### 什么是 Tokitai？

Tokitai 是一个基于 Rust 和 Tokitai 框架构建的**智能 AI 助手系统**，具备以下核心能力：

- 🤖 **AI 对话交互** - 自然语言对话，理解用户意图
- 🛠️ **丰富工具集** - 63+ 工具函数，覆盖文件/网络/系统/Git/数据处理
- 🧠 **自主进化** - AI 自主发现项目改进点并实施
- 📊 **上下文管理** - 三层存储架构，纯文件上下文存储
- 🔒 **安全沙箱** - 多层安全防护，防止危险操作
- ⚡ **极致性能** - 缓存响应 <10ms，50x 性能提升
- 🎯 **AI 原生工具选择器** - 快速搜索 <10ms，AI 搜索 <2s，LRU 缓存命中后 ~3ms
- 🧩 **完整工具矩阵** - 规则分类器 (IMP-001)、工具生成器 (IMP-002)、Trie 索引 (IMP-003)、动态注册表 (IMP-004)

### 产品定位

| 维度 | 描述 |
|------|------|
| **目标用户** | 开发者、技术团队、项目维护者 |
| **核心价值** | 提升开发效率，自动化重复任务，持续改进代码质量 |
| **使用场景** | 代码分析、文件操作、网络请求、Git 管理、项目维护 |
| **部署方式** | 本地 CLI 应用，无需服务器 |

---

## 核心功能

### 1. AI 对话交互

#### 功能描述
通过自然语言与 AI 进行多轮对话，AI 理解用户意图后自动调用合适的工具完成任务。

#### 功能特点
- ✅ 支持多轮对话，保持上下文记忆
- ✅ 自动工具调用（Function Calling）
- ✅ 支持 @ 路径引用快速读取文件
- ✅ 流式响应输出

#### 使用示例

```
👤 你：当前目录有哪些文件？
🤖 AI：当前目录包含以下文件：
  - README.md
  - Cargo.toml
  - src/
  - docs/
  ...

👤 你：读取 README.md 的内容
🤖 AI：README.md 的内容如下：
  # AI Assistant powered by Tokitai
  一个使用 Rust 和 Tokitai 构建的强大 AI 助手...

👤 你：分析 @src/main.rs 的结构
🤖 AI：main.rs 的结构分析：
  - 总行数：1,230 行
  - 主要结构体：AiAssistant
  - 核心方法：chat_and_handle_tools, init_all_services...
```

---

### 2. 文件操作工具

#### 功能列表

| 工具 | 功能 | 示例 |
|------|------|------|
| `read_file` | 读取文件内容 | `读取 README.md` |
| `write_file` | 写入文件内容 | `创建 test.txt，写入 Hello World` |
| `list_directory` | 列出目录内容 | `当前目录有哪些文件` |
| `copy_file` | 复制文件 | `复制 config.toml 到 backup/` |
| `delete_file` | 删除文件 | `删除 temp.txt` |
| `create_directory` | 创建目录 | `创建目录 logs/` |
| `file_exists` | 检查文件是否存在 | `检查 config.toml 是否存在` |
| `get_file_info` | 获取文件信息 | `查看 README.md 的大小和修改时间` |

#### 使用示例

```
👤 你：在 src 目录搜索所有 .rs 文件
🤖 AI：找到以下 Rust 源文件：
  - src/main.rs (1,230 行)
  - src/config.rs (156 行)
  - src/sandbox.rs (89 行)
  ...

👤 你：创建目录 structure_ensure/
🤖 AI：已创建目录 structure_ensure/

👤 你：复制 config.toml 到 structure_ensure/
🤖 AI：已将 config.toml 复制到 structure_ensure/config.toml
```

---

### 3. 代码分析工具

#### 功能列表

| 工具 | 功能 | 示例 |
|------|------|------|
| `analyze_code_structure` | 分析代码结构 | `分析 main.rs 的结构` |
| `count_lines_of_code` | 统计代码行数 | `统计 main.rs 有多少行` |
| `find_function_definitions` | 查找函数定义 | `查找所有函数定义` |
| `detect_language` | 检测编程语言 | `这是什么语言` |
| `find_code_patterns` | 查找代码模式 | `查找所有使用 Result 的地方` |

#### 使用示例

```
👤 你：分析 @src/main.rs 的结构
🤖 AI：main.rs 结构分析：

  **文件信息**
  - 路径：src/main.rs
  - 总行数：1,230 行
  - 语言：Rust

  **主要结构体**
  1. AiAssistant (核心结构体)
     - 字段：file_ops, system_tools, tool_registry...
     - 方法：new, chat_and_handle_tools, init_all_services...

  2. Config (配置结构体)
     - 字段：api_url, api_key, model...

  **模块依赖**
  - tools/ - 工具集合
  - context/ - 上下文存储
  - tool_matrix/ - 工具矩阵

👤 你：统计 main.rs 有多少行代码
🤖 AI：main.rs 共有 1,230 行代码
  - 代码行：980 行
  - 注释行：150 行
  - 空行：100 行
```

---

### 4. 网络请求工具

#### 功能列表

| 工具 | 功能 | 示例 |
|------|------|------|
| `http_get` | HTTP GET 请求 | `GET 请求 https://api.github.com` |
| `http_post` | HTTP POST 请求 | `POST 数据到 API` |
| `download_file` | 下载文件 | `下载 https://example.com/file.pdf` |
| `search_web` | 网页搜索 | `搜索关于 transformer 的论文` |
| `search_arxiv` | arXiv 论文搜索 | `搜索 arXiv 上的 AI 论文` |
| `download_arxiv_paper` | 下载 arXiv 论文 | `下载论文 2301.12345` |
| `check_url_availability` | 检查 URL 可用性 | `检查这个链接是否可用` |
| `ping_test` | Ping 测试 | `Ping google.com` |
| `port_scan` | 端口扫描 | `扫描 localhost 的端口` |

#### 使用示例

```
👤 你：GET 请求 https://api.github.com
🤖 AI：GitHub API 响应：
  {
    "current_user_url": "https://api.github.com/user",
    "current_user_authorizations_url": "...",
    ...
  }

👤 你：下载 https://arxiv.org/pdf/2301.12345.pdf
🤖 AI：已开始下载论文...
  下载完成：downloads/2301.12345.pdf (1.2 MB)

👤 你：搜索关于 transformer 的 arXiv 论文
🤖 AI：找到以下相关论文：
  1. "Attention Is All You Need" (2017)
  2. "Transformer-XL" (2019)
  3. ...
```

---

### 5. Git 操作工具

#### 功能列表

| 工具 | 功能 | 示例 |
|------|------|------|
| `git_status` | 查看 Git 状态 | `查看 git 状态` |
| `git_log` | 查看 Git 日志 | `查看最近的提交记录` |
| `git_branch` | 查看分支信息 | `当前分支是什么` |
| `git_diff` | 查看代码变更 | `查看本次修改的内容` |

#### 使用示例

```
👤 你：查看 git 状态
🤖 AI：Git 状态：
  On branch main
  Your branch is up to date with 'origin/main'.

  Changes not staged for commit:
    modified:   src/main.rs
    modified:   Cargo.toml

  Untracked files:
    docs/TECHNICAL_SPEC.md

👤 你：查看最近的提交记录
🤖 AI：最近 5 次提交：
  1. abc1234 - 添加工具选择器功能 (2026-03-15)
  2. def5678 - 修复上下文存储 bug (2026-03-14)
  3. ghi9012 - 优化性能 (2026-03-13)
  ...

👤 你：当前分支是什么
🤖 AI：当前分支：main
```

---

### 6. 数据处理工具

#### 功能列表

| 工具 | 功能 | 示例 |
|------|------|------|
| `format_json` | 格式化 JSON | `格式化这段 JSON` |
| `minify_json` | 压缩 JSON | `压缩这个 JSON 对象` |
| `json_path_query` | JSONPath 查询 | `查询 JSON 中的 user.name 字段` |
| `validate_json` | JSON 验证 | `验证 JSON 格式是否正确` |
| `merge_json` | JSON 合并 | `合并这两个 JSON 对象` |
| `json_to_csv` | JSON 转 CSV | `将 JSON 转换为 CSV 格式` |
| `extract_pdf_text` | 提取 PDF 文本 | `提取 document.pdf 的文本内容` |

#### 使用示例

```
👤 你：格式化这段 JSON：{"name":"John","age":30}
🤖 AI：格式化后的 JSON：
  {
    "name": "John",
    "age": 30
  }

👤 你：查询 JSON 中的 user.name 字段
🤖 AI：user.name 的值是："John"

👤 你：提取 document.pdf 的文本内容
🤖 AI：已从 document.pdf 提取文本：
  第 1 页：引言...
  第 2 页：方法论...
  ...
```

---

### 7. 系统工具

#### 功能列表

| 工具 | 功能 | 示例 |
|------|------|------|
| `run_command` | 执行系统命令 | `运行 cargo --version` |
| `run_safe_command` | 执行安全命令 | `查看当前目录的文件列表` |
| `get_env_variable` | 获取环境变量 | `获取 PATH 环境变量` |
| `get_current_directory` | 获取当前目录 | `当前工作目录是什么` |
| `get_system_info` | 获取系统信息 | `显示系统信息` |
| `list_processes` | 列出进程 | `列出所有进程` |
| `get_process_info` | 查看进程详情 | `查看 rustc 进程的详情` |
| `monitor_system_resources` | 系统资源监控 | `查看 CPU 和内存使用率` |

#### 使用示例

```
👤 你：运行 cargo --version
🤖 AI：cargo 1.75.0 (1dce52613 2023-11-20)

👤 你：显示系统信息
🤖 AI：系统信息：
  - 操作系统：macOS 14.0
  - 架构：aarch64
  - CPU：Apple M2 (8 核)
  - 内存：16 GB
  - 可用磁盘空间：256 GB

👤 你：查看 CPU 和内存使用率
🤖 AI：系统资源使用情况：
  - CPU 使用率：25%
  - 内存使用率：60% (9.6 GB / 16 GB)
  - 磁盘 I/O：正常
```

---

## 双轨服务模式

### 模式一：CLI AI 助手（面向用户）

#### 启动方式
```bash
# 交互模式
cargo run --release

# 单次查询
cargo run --release -- "查看当前目录有哪些文件"
```

#### 适用场景
- ✅ 快速查询项目信息
- ✅ 分析代码结构
- ✅ 执行临时任务（文件操作、网络请求）
- ✅ 获取建议和指导
- ✅ 多轮对话讨论问题

#### 功能特点
| 特点 | 说明 |
|------|------|
| **用户驱动** | 等待用户输入，按需响应 |
| **即时响应** | 单次请求 - 响应模式 |
| **交互式** | 支持多轮对话，保持上下文 |
| **工具丰富** | 63+ 工具，覆盖多种场景 |
| **安全沙箱** | 路径验证、命令黑名单、SSRF 防护 |

#### 服务边界
- ✅ 响应用户查询
- ✅ 执行用户指定的工具调用
- ✅ 保持对话上下文
- ✅ 提供建议和指导
- ❌ 不主动修改项目代码
- ❌ 不自主发起 Git 操作
- ❌ 不自主推送代码

---

### 模式二：项目自更新服务（面向项目自身）

#### 启动方式
```bash
# 自主进化模式（默认当前目录）
cargo run --release -- --autonomous

# 指定项目路径
cargo run --release -- --autonomous --project-path ./sandbox/test-project
```

#### 适用场景
- ✅ 持续改进代码质量
- ✅ 自动修复技术问题
- ✅ 清理技术债务
- ✅ 添加常规功能
- ✅ 定期维护项目

#### 工作流程
```
1. 分析项目状态
   └─→ 读取项目结构、代码质量、测试覆盖率

2. 发现改进点
   └─→ 识别代码异味、缺失功能、性能瓶颈

3. 制定改进计划
   └─→ Planner Agent 生成任务列表（DAG 依赖分析）

4. 执行改进任务
   └─→ Executor Agent 按计划执行（工具矩阵调度）

5. 审查代码变更
   └─→ Reviewer Agent 代码审查（本地 fmt/clippy/test）

6. 提交并推送（可选）
   └─→ Git 工作流自动提交变更

7. 继续下一轮迭代
   └─→ 回到步骤 1，持续改进
```

#### 功能特点
| 特点 | 说明 |
|------|------|
| **AI 驱动** | AI 自主分析项目，发现改进点 |
| **迭代循环** | Planner → Executor → Reviewer 循环 |
| **自主执行** | 无需用户干预，自动完成任务 |
| **Git 集成** | 自动生成提交并推送（可选） |
| **持续改进** | 每次迭代优化项目 |

#### 服务边界
- ✅ 自主分析项目状态
- ✅ 自主发现改进点
- ✅ 自主制定并执行计划
- ✅ 自主代码审查
- ✅ 自主 Git 提交（可选）
- ❌ 不响应用户交互
- ❌ 不处理外部查询
- ❌ 不提供服务接口

---

## 用户交互

### 交互命令

| 命令 | 功能 | 示例 |
|------|------|------|
| `help` | 显示可用操作列表 | `help` |
| `exit` / `quit` | 退出程序 | `exit` |
| `/role <name>` | 切换角色 | `/role planner` |
| `/optimize` | 优化上下文 | `/optimize` |
| `/context` | 显示上下文状态 | `/context` |
| `/roles` | 显示角色信息 | `/roles` |
| `/workflow list` | 列出可用工作流 | `/workflow list` |
| `/workflow start` | 启动工作流 | `/workflow start code_review` |
| `/toolbox` | 显示工具箱状态 | `/toolbox` |
| 任意自然语言 | 与 AI 对话 | `查看当前目录的文件列表` |
| `@<路径>` | 快速引用文件 | `@README.md 的内容是什么` |

### @ 路径引用功能

快速引用文件内容，让 AI 直接读取和分析：

| 用法 | 示例 |
|------|------|
| **单个文件** | `@README.md 的内容是什么` |
| **代码分析** | `分析 @src/main.rs 的结构` |
| **多个文件** | `@file1.txt @file2.txt 比较这两个文件` |
| **相对路径** | `@./config.toml 的配置项有哪些` |

---

## 安全特性

### 1. 命令执行安全

#### 黑名单机制
以下命令被禁止执行：

| 类别 | 禁止命令 |
|------|----------|
| **文件操作** | `rm`, `dd`, `shred` |
| **磁盘操作** | `mkfs`, `fdisk`, `parted` |
| **权限修改** | `chmod`, `chown`, `chgrp` |
| **提权命令** | `sudo`, `su`, `pkexec`, `doas` |
| **网络工具** | `wget`, `curl`, `nc`, `netcat`, `telnet`, `ssh`, `scp`, `rsync` |
| **进程控制** | `kill`, `pkill`, `killall`, `xkill` |
| **系统控制** | `shutdown`, `reboot`, `halt`, `poweroff`, `init` |
| **挂载操作** | `mount`, `umount`, `losetup` |
| **防火墙** | `iptables`, `firewall-cmd`, `ufw`, `nft` |
| **用户管理** | `visudo`, `passwd`, `useradd`, `userdel`, `usermod`... |
| **内核模块** | `insmod`, `rmmod`, `modprobe` |

#### 安全命令模式
- `run_safe_command` 只能执行只读命令
- 执行非黑名单命令时需要 `confirmed=true` 参数

---

### 2. 路径验证

- ✅ 规范化路径，防止路径遍历攻击
- ✅ 检查是否在允许目录内
- ✅ 符号链接循环检测
- ✅ 递归深度限制

---

### 3. SSRF 防护

- ✅ 内网 IP 过滤（10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16）
- ✅ 本地回环地址过滤（127.0.0.0/8）
- ✅ URL 协议白名单（http, https）
- ✅ 重定向检查

---

### 4. 速率限制

- ✅ HTTP 请求速率限制
- ✅ 文件操作速率限制
- ✅ 递归操作深度限制

---

## 性能特性

### 性能指标

| 指标 | 数值 | 说明 |
|------|------|------|
| **缓存响应延迟** | <10ms | 50x 性能提升 |
| **首次请求延迟** | ~2s | 降低 50% |
| **流式首字节延迟** | ~0.5s | 降低 60-70% |
| **缓存容量** | 200 条目 | TTL 5 分钟 |
| **工具搜索延迟** | <10ms | 缓存命中后 ~3ms |
| **AI 搜索延迟** | <2s | 包含 LLM 调用 |
| **后台重建 (100 工具)** | ~600ms | 批量处理优化 |
| **内存占用 (10,000 工具)** | ~15MB | 含缓存 |

### 性能优化策略

| 策略 | 说明 | 效果 |
|------|------|------|
| **全局 HTTP 连接池** | 复用连接，减少握手开销 | 零连接开销 |
| **纯异步线程模型** | tokio::spawn，无线程阻塞 | 高并发 |
| **LRU 缓存** | 1000 条搜索缓存 | 缓存命中率提升 |
| **后台异步重建** | 不阻塞主线程 | 响应更快 |
| **批量处理优化** | 100 工具 ~600ms | 降低 25% 延迟 |

---

## 使用场景

### 场景一：代码分析

```
👤 你：分析当前项目的结构
🤖 AI：项目结构分析：

  **目录结构**
  try-tokitai/
  ├── src/           # 源代码 (26,600 行)
  ├── docs/          # 文档
  ├── tests/         # 测试
  └── examples/      # 示例

  **核心模块**
  1. main.rs - 程序入口 (1,230 行)
  2. tools/ - 工具集合 (7,114 行)
  3. context/ - 上下文存储 (4,794 行)
  ...

  **代码质量**
  - 测试覆盖率：85%
  - Clippy 警告：3 个
  - 技术债务：低
```

---

### 场景二：文件管理

```
👤 你：在 src 目录搜索所有包含 "async" 的 Rust 文件
🤖 AI：找到以下文件包含 "async"：
  - src/main.rs (45 处)
  - src/config.rs (12 处)
  - src/tools/io/file_ops.rs (23 处)
  ...

👤 你：统计每种文件类型的数量
🤖 AI：文件类型统计：
  - .rs 文件：78 个
  - .toml 文件：5 个
  - .md 文件：12 个
  - .json 文件：3 个
  ...
```

---

### 场景三：网络请求

```
👤 你：下载 https://example.com/data.csv
🤖 AI：已开始下载...
  下载完成：downloads/data.csv (2.5 MB)

👤 你：检查 https://api.github.com 是否可用
🤖 AI：URL 可用性检查：
  - 状态码：200 OK
  - 响应时间：156ms
  - 服务状态：正常
```

---

### 场景四：Git 管理

```
👤 你：查看最近的提交记录
🤖 AI：最近 5 次提交：
  1. abc1234 - 添加工具选择器功能 (2026-03-15)
  2. def5678 - 修复上下文存储 bug (2026-03-14)
  3. ghi9012 - 优化性能 (2026-03-13)
  4. jkl3456 - 添加文档 (2026-03-12)
  5. mno7890 - 初始提交 (2026-03-11)

👤 你：查看本次修改的内容
🤖 AI：Git Diff：
  diff --git a/src/main.rs b/src/main.rs
  + // 新增功能
  + pub fn new_feature() { ... }
  ...
```

---

### 场景五：自主进化

```bash
# 启动自主进化模式
cargo run --release -- --autonomous
```

```
🤖 AI：自主进化已启动

  **迭代 1**
  1. 分析项目状态...
  2. 发现改进点：
     - src/main.rs 有 3 个 Clippy 警告
     - 缺少单元测试
     - 文档不完整
  3. 制定改进计划...
  4. 执行改进任务...
  5. 代码审查通过 ✓
  6. 提交变更：git commit -m "修复 Clippy 警告并添加测试"
  7. 推送到 GitHub... ✓

  **迭代 2**
  继续下一轮改进...
```

---

## 最佳实践

### 1. 使用 @ 引用文件

```
✅ 推荐：
  分析 @src/main.rs 的结构
  @file1.txt @file2.txt 比较这两个文件

❌ 不推荐：
  请读取 src/main.rs 文件并分析它的结构
```

---

### 2. 明确表达意图

```
✅ 推荐：
  在 src 目录搜索所有 .rs 文件
  格式化这段 JSON 数据
  下载这个 PDF 文件

❌ 不推荐：
  帮我看看 src 里面有哪些 Rust 文件
  把这个 JSON 弄好看点
  弄个 PDF 下来
```

---

### 3. 分步骤执行复杂任务

```
✅ 推荐：
  1. 先查看当前目录结构
  2. 然后读取 README.md
  3. 最后分析项目配置

❌ 不推荐：
  帮我把整个项目分析一遍
```

---

### 4. 合理使用自主进化模式

```
✅ 推荐场景：
  - 定期代码质量改进
  - 技术债务清理
  - 添加常规功能

❌ 不推荐场景：
  - 重大架构调整
  - 核心功能修改
  - 需要人工审查的变更
```

---

## 常见问题

### Q1: 如何获取 API Key？

**A**: 访问 https://ollama.com 注册账号，然后在设置页面创建 API Key。

详细步骤：
1. 访问 https://ollama.com
2. 点击右上角 "Sign In" 注册/登录
3. 进入设置页面（头像 → API Keys）
4. 点击 "Create API Key"
5. 复制并保存 Key（刷新后将无法查看）

---

### Q2: 可以使用本地 Ollama 服务吗？

**A**: 可以。设置环境变量：
```bash
export AI_API_URL="http://localhost:11434/v1/chat/completions"
```

---

### Q3: 模型响应很慢怎么办？

**A**: 尝试切换到较小的模型：
```bash
export AI_MODEL="qwen2.5:7b"
```

---

### Q4: 自主进化模式安全吗？

**A**: 自主进化模式有多重安全保障：
- 本地代码审查（fmt/clippy/test）
- 失败自动回滚
- 可配置为仅提交不推送
- 所有操作记录到追踪日志

---

### Q5: 如何清理缓存？

**A**: 直接删除运行时文件夹：
```bash
rm -rf .context/ .tokitai/ sandbox/ downloads/
```

---

### Q6: 支持哪些 AI 模型？

**A**: 支持 OpenAI 兼容 API 的所有模型，推荐：
- `qwen3.5:397b` - 通义千问 3.5（397B 参数）
- `qwen3-coder:480b` - 通义千问代码版（480B 参数）
- `deepseek-v3.2` - DeepSeek V3.2
- `gemma3` 系列 - Google Gemma 3

---

### Q7: 如何查看日志？

**A**: 追踪日志保存在 `.tokitai/traces/` 目录：
```bash
# 查看最近的追踪日志
cat .tokitai/traces/latest.json

# 使用工具查询
cargo run --release -- "查看最近的追踪记录"
```

---

### Q8: 如何自定义工作流？

**A**: 在 `workflows/` 目录创建 TOML 文件：
```toml
[workflow]
id = "my_workflow"
name = "我的工作流"
version = "1.0.0"

[[workflow.steps]]
id = "step1"
tool = "read_file"
arguments = { path = "config.toml" }
```

---

## 相关文档

| 文档 | 说明 |
|------|------|
| [QUICKSTART.md](QUICKSTART.md) | 快速启动指南 |
| [USER_GUIDE.md](USER_GUIDE.md) | 完整用户指南 |
| [TECHNICAL_SPEC.md](TECHNICAL_SPEC.md) | 技术说明文档 |
| [CHANGELOG.md](CHANGELOG.md) | 更新日志 |
| [archive/ARCHITECTURE_IMPROVEMENT_PLAN.json](archive/ARCHITECTURE_IMPROVEMENT_PLAN.json) | 架构改进计划（已归档） |
| [archive/IMPLEMENTATION_STATUS_REPORT.md](archive/IMPLEMENTATION_STATUS_REPORT.md) | 实施状态报告（已归档） |
| [archive/](archive/) | 技术报告归档 |

---

**文档版本**: 3.0.0
**最后更新**: 2026-03-18
**维护者**: Tokitai Team
