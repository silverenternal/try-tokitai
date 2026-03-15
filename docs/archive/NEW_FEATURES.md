# 新增功能说明

本次升级为 try-tokitai 项目添加了以下实用功能：

## 🆕 新增工具集

### 1. HTTP 客户端工具 (`HttpClientTools`)

提供类似 curl 的 HTTP 请求功能：

| 函数 | 说明 | 示例 |
|------|------|------|
| `http_get` | 发送 HTTP GET 请求 | 获取 API 数据、网页内容 |
| `http_post` | 发送 HTTP POST 请求 | 提交表单、JSON 数据 |
| `check_url` | 检查 URL 是否可访问 | 快速检查网站在线状态 |
| `download_file` | 下载文件到本地 | 下载图片、文档等 |

**使用示例：**
```
# GET 请求
请 GET 请求 https://api.github.com/users/octocat

# POST 请求
POST 数据到 https://api.example.com/users，body 是 {"name": "test"}

# 检查 URL
检查 https://github.com 是否可以访问

# 下载文件
下载 https://example.com/file.pdf 保存到 /tmp/file.pdf
```

---

### 2. JSON 处理工具 (`JsonTools`)

提供 JSON 数据的解析、格式化、查询功能：

| 函数 | 说明 | 示例 |
|------|------|------|
| `format_json` | 格式化 JSON 字符串 | 美化压缩的 JSON |
| `minify_json` | 压缩 JSON 字符串 | 移除空白字符 |
| `query_json` | 查询 JSON 数据 | 使用点号路径查询 |
| `extract_keys` | 提取所有键名 | 递归获取所有键 |
| `validate_json` | 验证 JSON 格式 | 检查是否有效 JSON |
| `merge_json` | 合并多个 JSON 对象 | 将多个对象合并 |
| `json_to_csv` | JSON 转 CSV 格式 | 数组转表格数据 |

**使用示例：**
```
# 格式化 JSON
格式化这段 JSON: {"name":"test","value":123}

# 查询 JSON
查询 {"user": {"name": "John", "age": 30}} 中的 user.name

# 验证 JSON
验证这段文本是否是有效的 JSON 格式
```

---

### 3. 文件搜索工具 (`FileSearchTools`)

提供类似 grep 的文件搜索功能：

| 函数 | 说明 | 示例 |
|------|------|------|
| `grep` | 在文件中搜索文本 | 支持大小写选项 |
| `find_files` | 递归搜索文件 | 按文件名或扩展名 |
| `count_file_types` | 统计文件类型分布 | 按扩展名统计 |
| `find_large_files` | 查找大文件 | 超过指定大小的文件 |
| `get_file_info` | 获取文件详细信息 | 大小、时间、权限 |

**使用示例：**
```
# 搜索文件内容
在 src/main.rs 中搜索 "fn main"

# 查找文件
在 /home 目录搜索所有 .rs 文件

# 统计文件类型
统计当前目录有哪些类型的文件

# 查找大文件
查找当前目录下大于 100MB 的文件

# 文件信息
获取 README.md 的详细信息
```

---

### 4. 进程管理工具 (`ProcessTools`)

提供系统进程的查看和管理功能：

| 函数 | 说明 | 示例 |
|------|------|------|
| `list_processes` | 列出运行的进程 | 按 CPU 使用率排序 |
| `get_process_info` | 获取进程详细信息 | 查看指定 PID 详情 |
| `search_processes` | 按名称搜索进程 | 查找匹配的进程 |
| `get_system_resources` | 获取系统资源使用 | CPU、内存、磁盘 |
| `get_process_files` | 查看进程打开的文件 | 文件描述符列表 |
| `get_process_env` | 查看进程环境变量 | 获取环境配置 |

**使用示例：**
```
# 列出进程
列出占用 CPU 最高的 10 个进程

# 进程信息
查看进程 1234 的详细信息

# 搜索进程
搜索所有包含 "cargo" 的进程

# 系统资源
查看当前系统资源使用情况

# 打开文件
查看进程 5678 打开了哪些文件
```

---

### 5. 网络工具 (`NetworkTools`)

提供网络诊断和连接测试功能：

| 函数 | 说明 | 示例 |
|------|------|------|
| `ping_host` | Ping 主机测试连通性 | 检查主机是否可达 |
| `check_tcp_port` | 检查 TCP 端口 | 测试端口是否开放 |
| `scan_common_ports` | 扫描常用端口 | 快速扫描开放端口 |
| `get_local_network_info` | 获取本地网络信息 | IP、接口、DNS |
| `trace_route` | 追踪路由路径 | 显示到目标的路由 |
| `get_public_ip` | 获取公网 IP 地址 | 查询外部 IP |

**使用示例：**
```
# Ping 测试
ping github.com

# 端口检查
检查 localhost 的 22 端口是否开放

# 端口扫描
扫描 192.168.1.1 的常用端口

# 网络信息
查看本地网络配置信息

# 路由追踪
追踪到 google.com 的路由路径

# 公网 IP
查询我的公网 IP 地址是多少
```

---

## 📊 工具数量统计

| 工具集 | 函数数量 |
|--------|----------|
| FileOperations | 5 |
| SystemTools | 8 |
| CodeTools | 4 |
| WebSearchTools | 3 |
| DownloadTools | 3 |
| GitOperations | 4 |
| **HttpClientTools** | **4** ⭐ NEW |
| **JsonTools** | **6** ⭐ NEW |
| **FileSearchTools** | **5** ⭐ NEW |
| **ProcessTools** | **6** ⭐ NEW |
| **NetworkTools** | **6** ⭐ NEW |
| **总计** | **54** |

---

## 🔧 技术实现

### 依赖项
- `reqwest` (blocking) - HTTP 客户端
- `serde_json` - JSON 处理
- `tokitai` 0.4.0 - Tool 宏支持
- `tokitai-core` 0.4.0 - 核心类型支持

### 代码结构
```
src/tools/
├── file_ops.rs       # 文件操作
├── system.rs         # 系统命令
├── code_analysis.rs  # 代码分析
├── web_search.rs     # 网络搜索
├── download.rs       # 文件下载
├── git_ops.rs        # Git 操作
├── http_client.rs    # HTTP 客户端 ⭐ NEW
├── json_tools.rs     # JSON 处理 ⭐ NEW
├── file_search.rs    # 文件搜索 ⭐ NEW
├── process_tools.rs  # 进程管理 ⭐ NEW
├── network_tools.rs  # 网络工具 ⭐ NEW
└── mod.rs            # 模块导出
```

---

## 🚀 快速开始

```bash
# 编译
cargo build --release

# 运行
cargo run --release

# 运行测试
cargo test --release
```

---

## 📝 使用提示

1. **@ 文件引用**：在对话中使用 `@path/to/file` 快速引用文件内容
2. **help 命令**：输入 `help` 查看所有可用功能
3. **安全限制**：危险命令（如 rm、sudo 等）已被阻止
4. **文件大小限制**：@ 引用文件最大 1MB

---

## 🎯 使用场景示例

### 场景 1：API 调试
```
用户：GET 请求 https://api.github.com/repos/tokio-rs/tokio
AI: [获取仓库信息]
用户：格式化返回的 JSON 数据
AI: [美化输出]
用户：查询 JSON 中的 stargazers_count
AI: [提取星标数量]
```

### 场景 2：系统诊断
```
用户：查看系统资源使用情况
AI: [显示 CPU、内存、磁盘]
用户：列出占用内存最高的 5 个进程
AI: [列出进程]
用户：查看进程 1234 的详细信息
AI: [显示进程详情]
```

### 场景 3：网络排查
```
用户：ping github.com
AI: [测试连通性]
用户：扫描 localhost 的开放端口
AI: [显示开放端口列表]
用户：检查 8080 端口是否开放
AI: [测试结果]
```

### 场景 4：文件管理
```
用户：在 src 目录搜索所有 .rs 文件
AI: [列出 Rust 文件]
用户：查找大于 10MB 的文件
AI: [列出大文件]
用户：统计当前目录的文件类型分布
AI: [显示统计结果]
```

---

## 📋 版本信息

- **tokitai**: 0.4.0
- **tokitai-core**: 0.4.0
- **Rust**: 1.75+
- **新增工具集**: 5
- **新增函数**: 27
- **总工具函数**: 54
