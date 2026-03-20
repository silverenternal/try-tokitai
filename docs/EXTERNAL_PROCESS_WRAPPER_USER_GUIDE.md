# External Process Wrapper (EPW) 用户指南

> **将外部进程/服务封装为 AI 可调度的 tokitai 工具**
>
> 本文档介绍如何使用 External Process Wrapper 系统将 CLI 工具、HTTP 服务和脚本文件封装为 AI 可调用的 tokitai 工具。

[![Status](https://img.shields.io/badge/status-stable-brightgreen)]()
[![Coverage](https://img.shields.io/badge/coverage-90%25-brightgreen)]()

---

## 📖 目录

- [概述](#概述)
- [快速开始](#快速开始)
- [封装 CLI 工具](#封装-cli-工具)
- [封装 HTTP 服务](#封装-http-服务)
- [封装脚本文件](#封装脚本文件)
- [自动发现](#自动发现)
- [集成到自进化系统](#集成到自进化系统)
- [最佳实践](#最佳实践)
- [故障排查](#故障排查)

---

## 概述

### 什么是 External Process Wrapper？

External Process Wrapper (EPW) 是一个将外部进程/服务封装为 tokitai 工具的系统，它扩展了 AI 可调用的工具范围：

| 工具类型 | 描述 | 示例 |
|---------|------|------|
| **Process** | 本地可执行文件/CLI 工具 | git, docker, npm, curl |
| **HTTP** | 远程 HTTP 服务/REST API | GitHub API, OpenAI API |
| **Script** | 脚本文件 | .sh, .py, .js 脚本 |

### 核心价值

1. **AI 可调用**：外部工具注册到工具矩阵后，AI Agent 可以自主发现和调用
2. **统一接口**：所有外部工具实现 `ExternalTool` trait，与内置工具无缝集成
3. **快速原型**：用脚本快速创建工具，无需等待 Rust 实现
4. **企业集成**：轻松封装企业内部系统的 HTTP API

### 架构设计

```
┌─────────────────────────────────────────┐
│         AI Dispatch Layer               │
│  (SelfImprovementLoop, Orchestrator)    │
├─────────────────────────────────────────┤
│         Tool Matrix Layer               │
│  (ToolMatrix, ToolSelector, Registry)   │
├─────────────────────────────────────────┤
│      EPW Wrapper Layer                  │
│  ┌──────────┬──────────┬──────────┐    │
│  │ Process  │   HTTP   │  Script  │    │
│  │ Wrapper  │  Wrapper │  Wrapper │    │
│  └──────────┴──────────┴──────────┘    │
├─────────────────────────────────────────┤
│      External Execution Layer           │
│  (CLI Tools, HTTP APIs, Scripts)        │
└─────────────────────────────────────────┘
```

---

## 快速开始

### 1. 添加依赖

在 `Cargo.toml` 中确保有：

```toml
[dependencies]
# tokitai 和相关依赖
tokitai = "0.4.0"
```

### 2. 基本使用

```rust
use ai_assistant::external_process::{
    ExternalTool,
    ProcessWrapperBuilder,
    metadata::schema_helpers,
};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 创建一个简单的 echo 工具
    let echo_wrapper = ProcessWrapperBuilder::new("echo_test", "echo")
        .description("Echo a message")
        .args(vec!["{{message}}".to_string()])
        .input_schema(schema_helpers::create_string_params_schema(vec![
            ("message", "Message to echo", true),
        ]))
        .domain("test")
        .build();

    // 执行工具
    let result = echo_wrapper.execute(json!({
        "message": "Hello, EPW!"
    })).await?;

    println!("Output: {}", result.stdout.unwrap());
    
    Ok(())
}
```

### 3. 运行示例

```bash
# Git CLI 示例
cargo run --example epw_git_example

# HTTP 服务示例
cargo run --example epw_http_example

# 脚本示例
cargo run --example epw_script_example
```

---

## 封装 CLI 工具

### 使用 Builder Pattern

```rust
use ai_assistant::external_process::{
    ProcessWrapperBuilder,
    metadata::{schema_helpers, RiskLevel},
};

let git_wrapper = ProcessWrapperBuilder::new("git_commit", "git")
    .description("Commit changes to Git repository")
    .args(vec![
        "commit".to_string(),
        "-m".to_string(),
        "{{message}}".to_string(),
    ])
    .working_dir(PathBuf::from("/workspace"))
    .timeout(30000)
    .env("GIT_AUTHOR_NAME".to_string(), "AI Agent".to_string())
    .input_schema(schema_helpers::create_string_params_schema(vec![
        ("message", "Commit message", true),
    ]))
    .domain("version_control")
    .tag("git")
    .tag("commit")
    .risk_level(RiskLevel::Medium)
    .build();
```

### 参数模板替换

支持 `{{variable}}` 语法：

```rust
.args(vec![
    "commit".to_string(),
    "-m".to_string(),
    "{{message}}".to_string(),
    "--author".to_string(),
    "{{author}}".to_string(),
])
```

执行时会自动替换：
```json
{"message": "Fix bug", "author": "AI <ai@example.com>"}
```

### 完整示例

参考 [`examples/epw_git_example.rs`](examples/epw_git_example.rs)

---

## 封装 HTTP 服务

### 基本用法

```rust
use ai_assistant::external_process::{
    HTTPWrapperBuilder,
    metadata::schema_helpers,
};

let api_wrapper = HTTPWrapperBuilder::new(
    "my_api",
    "https://api.example.com",
    "GET",
)
.description("Call my API")
.path("/users/{{user_id}}")
.header("Accept".to_string(), "application/json".to_string())
.input_schema(schema_helpers::create_string_params_schema(vec![
    ("user_id", "User ID", true),
]))
.domain("http_client")
.build();
```

### 认证方式

#### Bearer Token

```rust
use ai_assistant::external_process::metadata::AuthConfig;

let auth = AuthConfig::BearerToken {
    token_env: "API_TOKEN".to_string(),
};

let wrapper = HTTPWrapperBuilder::new("protected_api", "https://api.example.com", "GET")
    .path("/protected/resource")
    .auth(auth)
    .build();
```

#### API Key

```rust
let auth = AuthConfig::ApiKey {
    header_name: "X-API-Key".to_string(),
    key_env: "API_KEY".to_string(),
};
```

#### Basic Auth

```rust
let auth = AuthConfig::Basic {
    username_env: "API_USER".to_string(),
    password_env: "API_PASS".to_string(),
};
```

#### OAuth 2.0

```rust
let auth = AuthConfig::OAuth2 {
    client_id_env: "OAUTH_CLIENT_ID".to_string(),
    client_secret_env: "OAUTH_CLIENT_SECRET".to_string(),
    token_url: "https://auth.example.com/oauth/token".to_string(),
    scopes: vec!["read".to_string(), "write".to_string()],
};
```

### POST/PUT 请求

```rust
let create_wrapper = HTTPWrapperBuilder::new(
    "create_user",
    "https://api.example.com",
    "POST",
)
.path("/users")
.header("Content-Type".to_string(), "application/json".to_string())
.input_schema(serde_json::json!({
    "type": "object",
    "properties": {
        "name": {"type": "string"},
        "email": {"type": "string"}
    },
    "required": ["name", "email"]
}))
.build();
```

### 从 OpenAPI 自动生成

```rust
use ai_assistant::external_process::http_wrapper::openapi_parser;

let openapi_spec = reqwest::get("https://api.example.com/openapi.json")
    .await?
    .json()
    .await?;

let wrappers = openapi_parser::parse_openapi(&openapi_spec, "my_agent")?;

for wrapper in wrappers {
    println!("Generated: {}", wrapper.name());
}
```

### 完整示例

参考 [`examples/epw_http_example.rs`](examples/epw_http_example.rs)

---

## 封装脚本文件

### 基本用法

```rust
use ai_assistant::external_process::{
    ScriptWrapperBuilder,
    metadata::schema_helpers,
};
use std::path::PathBuf;

let script_wrapper = ScriptWrapperBuilder::new(
    "analyze_data",
    PathBuf::from("scripts/analyze.py"),
)
.description("Analyze data using Python script")
.interpreter("python3")
.args(vec!["--input".to_string(), "{{input_file}}".to_string()])
.working_dir(PathBuf::from("/workspace"))
.input_schema(schema_helpers::create_string_params_schema(vec![
    ("input_file", "Input file path", true),
]))
.domain("data_analysis")
.build();
```

### 自动检测解释器

```rust
// 根据文件扩展名自动选择解释器
if let Some(builder) = ScriptWrapperBuilder::with_auto_interpreter(
    "my_script",
    PathBuf::from("scripts/process.py"),
) {
    let wrapper = builder
        .description("Auto-detected interpreter")
        .build();
    
    // .py -> python3, .sh -> bash, .js -> node, etc.
}
```

### 支持的解释器

| 扩展名 | 解释器 |
|--------|--------|
| .sh, .bash | bash |
| .py, .py3 | python3 |
| .js, .mjs | node |
| .ts, .tsx | ts-node |
| .rb | ruby |
| .pl | perl |
| .php | php |
| .r, .R | Rscript |
| .jl | julia |
| .lua | lua |
| .ps1 | powershell |

### 完整示例

参考 [`examples/epw_script_example.rs`](examples/epw_script_example.rs)

---

## 自动发现

### 扫描系统可执行文件

```rust
use ai_assistant::external_process::ExternalToolDiscovery;

let mut discovery = ExternalToolDiscovery::new();

// 扫描系统 PATH 中的常见 CLI 工具
let cli_tools = discovery.scan_executables().await?;

for tool in &cli_tools {
    println!("Found CLI tool: {}", tool.name);
}
```

### 扫描脚本目录

```rust
// 扫描目录中的所有脚本
let scripts = discovery.scan_scripts("./scripts").await?;

for script in &scripts {
    println!("Found script: {}", script.name);
}
```

### 从 OpenAPI 发现 HTTP 服务

```rust
// 从 OpenAPI 规范生成 HTTP 工具
let http_tools = discovery.from_openapi(
    "https://api.github.com/swagger.json"
).await?;
```

### AI 增强元数据

```rust
// 使用 AI 生成更好的描述和参数 Schema
let enriched = discovery.ai_enrich_metadata(tool_metadata).await?;
```

---

## 集成到自进化系统

### 注册到工具矩阵

```rust
use ai_assistant::external_process::ExternalToolRegistry;
use ai_assistant::tool_matrix::matrix::ToolRegistry;

// 创建外部工具注册表
let epw_registry = ExternalToolRegistry::new()?;

// 注册工具
epw_registry.register_process(git_wrapper)?;
epw_registry.register_http(api_wrapper)?;
epw_registry.register_script(script_wrapper)?;

// 注册到工具矩阵
let mut tool_registry = ToolRegistry::new();
epw_registry.register_to_tool_matrix(&mut tool_registry)?;
```

### 自进化循环自动发现

当自进化系统检测到工具缺口时，会自动：

1. **决策**：判断是创建 Rust 工具还是封装外部工具
2. **发现**：扫描系统中是否有现成的 CLI/API/脚本
3. **封装**：创建 ExternalTool 封装器
4. **注册**：注册到工具矩阵供 AI 调用

```rust
use ai_assistant::autonomy::SelfImprovementLoop;

let evolution = SelfImprovementLoop::new(project_root)?;

// 运行进化循环（包含外部工具发现）
let report = evolution.run_evolution_cycle()?;

println!("Created tools: {:?}", report.created_tools);
println!("Registered tools: {:?}", report.registered_tools);
```

---

## 最佳实践

### 1. 参数验证

始终定义清晰的输入 Schema：

```rust
.input_schema(serde_json::json!({
    "type": "object",
    "properties": {
        "message": {
            "type": "string",
            "description": "Commit message",
            "minLength": 1,
            "maxLength": 500
        }
    },
    "required": ["message"]
}))
```

### 2. 超时设置

为所有外部工具设置合理的超时：

```rust
// CLI 工具：30 秒
.timeout(30000)

// HTTP 请求：10-60 秒（根据 API 响应时间）
.timeout(60000)

// 脚本：根据任务复杂度
.timeout(120000)
```

### 3. 错误处理

外部工具可能失败，始终检查执行结果：

```rust
let result = wrapper.execute(input).await?;

if !result.success {
    tracing::warn!("Tool failed: {}", result.error.unwrap());
    // 处理错误...
}
```

### 4. 日志记录

启用 tracing 记录工具调用：

```rust
// 在 main 中初始化
tracing_subscriber::fmt::init();

// 工具执行会自动记录日志
```

### 5. 安全考虑

- **环境变量**：敏感信息使用环境变量，不要硬编码
- **工作目录**：限制脚本和进程的工作目录
- **风险等级**：为高风险工具设置适当的 `RiskLevel`

---

## 故障排查

### 常见问题

#### "Executable not found"

```
Error: Failed to spawn process: No such file or directory
```

**解决方案**：
- 检查可执行文件是否在 PATH 中
- 使用绝对路径
- 确认文件有执行权限

#### "Input validation failed"

```
Error: Input validation failed: Missing required field: message
```

**解决方案**：
- 检查输入 JSON 是否包含所有必需字段
- 验证字段类型是否正确

#### "HTTP request timed out"

```
Error: HTTP request timed out after 30000ms
```

**解决方案**：
- 增加超时时间
- 检查网络连接
- 验证 API 端点是否可达

#### "Script interpreter not found"

```
Error: Could not determine interpreter for script
```

**解决方案**：
- 明确指定解释器：`.interpreter("python3")`
- 确认解释器已安装且在 PATH 中

### 启用调试日志

```rust
// 在 main 中
std::env::set_var("RUST_LOG", "debug");
tracing_subscriber::fmt::init();
```

---

## 相关文档

- [API Reference](docs/EXTERNAL_PROCESS_WRAPPER_API.md)
- [Developer Guide](docs/EXTERNAL_PROCESS_WRAPPER_DEVELOPER_GUIDE.md)
- [示例代码](examples/epw_*.rs)

---

**最后更新**: 2026-03-20
**版本**: 1.0.0
