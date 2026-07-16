# EPW 示例代码

由于本项目是 bin crate（`src/main.rs`），`examples/` 目录下的示例无法直接访问内部模块。

## 如何运行示例

将以下示例代码复制到 `src/bin/` 目录下，然后使用 `cargo run --bin <example_name>` 运行。

### 示例 1: Git CLI 封装

```rust
//! src/bin/epw_git_example.rs
//! 演示如何封装 Git CLI 工具

use crate::external_process::{
    ExternalTool,
    ProcessWrapperBuilder,
    metadata::{RiskLevel, schema_helpers},
};
use crate::tool_matrix::matrix::ToolDefinition;
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== EPW Git 示例：封装 Git CLI 工具 ===\n");

    // 创建 git commit 工具
    let git_commit_wrapper = ProcessWrapperBuilder::new("git_commit", "git")
        .description("提交代码到 Git 仓库")
        .args(vec![
            "commit".to_string(),
            "-m".to_string(),
            "{{message}}".to_string(),
        ])
        .input_schema(schema_helpers::create_string_params_schema(vec![
            ("message", "提交信息", true),
        ]))
        .domain("version_control")
        .tag("git")
        .tag("commit")
        .risk_level(RiskLevel::Medium)
        .build();

    println!("工具名称：{}", git_commit_wrapper.name());
    println!("工具描述：{}", git_commit_wrapper.description());

    // 验证输入
    let valid_input = json!({"message": "Initial commit"});
    match git_commit_wrapper.validate_input(&valid_input) {
        Ok(_) => println!("输入验证通过"),
        Err(e) => println!("输入验证失败：{}", e),
    }

    // 执行工具
    let result = git_commit_wrapper.execute(json!({"message": "Initial commit"})).await?;
    println!("执行成功：{}", result.success);
    println!("执行时间：{}ms", result.execution_time_ms);

    // 转换为 ToolDefinition
    let tool_def: ToolDefinition = git_commit_wrapper.to_tool_definition();
    println!("ToolDefinition 名称：{}", tool_def.name);

    Ok(())
}
```

### 示例 2: GitHub API 封装

```rust
//! src/bin/epw_github_example.rs
//! 演示如何封装 GitHub REST API

use crate::external_process::{
    ExternalTool,
    HTTPWrapperBuilder,
    metadata::{AuthConfig, RiskLevel, schema_helpers},
};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== EPW GitHub 示例：封装 GitHub REST API ===\n");

    // 创建获取用户信息的工具
    let github_user_wrapper = HTTPWrapperBuilder::new("github_get_user", "https://api.github.com")
        .description("获取 GitHub 用户信息")
        .method("GET")
        .path("/users/{{username}}")
        .input_schema(schema_helpers::create_string_params_schema(vec![
            ("username", "GitHub 用户名", true),
        ]))
        .domain("http_client")
        .tag("github")
        .tag("api")
        .risk_level(RiskLevel::Low)
        .build();

    println!("工具名称：{}", github_user_wrapper.name());
    println!("基础 URL: https://api.github.com");
    println!("HTTP 方法：GET");

    // 执行 API 调用
    let result = github_user_wrapper.execute(json!({"username": "torvalds"})).await?;
    
    if result.success {
        println!("API 调用成功");
        println!("执行时间：{}ms", result.execution_time_ms);
        
        if let Some(output) = result.output.as_object() {
            if let Some(name) = output.get("name").and_then(|v| v.as_str()) {
                println!("用户名：{}", name);
            }
        }
    }

    // 创建需要认证的 GitHub 工具
    let auth = AuthConfig::BearerToken {
        token_env: "GITHUB_TOKEN".to_string(),
    };

    let github_issue_wrapper = HTTPWrapperBuilder::new("github_create_issue", "https://api.github.com")
        .description("在 GitHub 仓库创建 Issue")
        .method("POST")
        .path("/repos/{{owner}}/{{repo}}/issues")
        .auth(auth)
        .input_schema(schema_helpers::create_string_params_schema(vec![
            ("owner", "仓库所有者", true),
            ("repo", "仓库名称", true),
            ("title", "Issue 标题", true),
        ]))
        .domain("http_client")
        .tag("github")
        .tag("api")
        .risk_level(RiskLevel::Medium)
        .build();

    println!("\n创建 Issue 工具已配置");
    println!("认证方式：Bearer Token");
    println!("环境变量：GITHUB_TOKEN");

    Ok(())
}
```

### 示例 3: Python 脚本封装

```rust
//! src/bin/epw_python_script_example.rs
//! 演示如何封装 Python 脚本

use crate::external_process::{
    ExternalTool,
    ScriptWrapperBuilder,
    metadata::{RiskLevel, schema_helpers},
};
use serde_json::json;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== EPW Python 脚本示例：封装数据分析脚本 ===\n");

    let script_path = PathBuf::from("scripts/analyze.py");

    // 创建脚本工具
    let analyze_wrapper = ScriptWrapperBuilder::new("analyze_data", script_path)
        .description("使用 Python 脚本分析数据文件")
        .interpreter("python3")
        .args(vec![
            "--input".to_string(),
            "{{input_file}}".to_string(),
            "--format".to_string(),
            "{{format}}".to_string(),
        ])
        .input_schema(schema_helpers::create_string_params_schema(vec![
            ("input_file", "输入文件路径", true),
            ("format", "输出格式（json/csv/text）", false),
        ]))
        .domain("data_analysis")
        .tag("python")
        .tag("data")
        .tag("analysis")
        .risk_level(RiskLevel::Low)
        .build();

    println!("工具名称：{}", analyze_wrapper.name());
    println!("脚本路径：{:?}", analyze_wrapper.metadata().tool_type);
    println!("解释器：python3");

    // 验证输入
    let valid_input = json!({
        "input_file": "/path/to/data.csv",
        "format": "json"
    });
    match analyze_wrapper.validate_input(&valid_input) {
        Ok(_) => println!("输入验证通过"),
        Err(e) => println!("输入验证失败：{}", e),
    }

    // 执行脚本
    let result = analyze_wrapper.execute(json!({
        "input_file": "data.csv",
        "format": "json"
    })).await?;

    println!("执行成功：{}", result.success);
    println!("执行时间：{}ms", result.execution_time_ms);
    
    if let Some(stdout) = &result.stdout {
        println!("脚本输出:\n{}", stdout);
    }

    Ok(())
}
```

### 示例 4: 完整演示

```rust
//! src/bin/epw_full_demo.rs
//! 完整的 EPW 功能演示

use crate::external_process::{
    ExternalTool,
    ProcessWrapperBuilder,
    HTTPWrapperBuilder,
    ScriptWrapperBuilder,
    ExternalToolDiscovery,
    ExternalToolRegistry,
    metadata::{AuthConfig, RiskLevel, schema_helpers},
};
use crate::tool_matrix::matrix::ToolDefinition;
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║   External Process Wrapper (EPW) 完整演示                ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    // ========== 第一部分：创建外部工具 ==========
    println!("📦 第一部分：创建外部工具\n");

    // 1.1 Process 工具
    let echo_tool = ProcessWrapperBuilder::new("echo_message", "echo")
        .description("Echo a message to stdout")
        .args(vec!["{{message}}".to_string()])
        .input_schema(schema_helpers::create_string_params_schema(vec![
            ("message", "Message to echo", true),
        ]))
        .domain("test")
        .build();
    println!("✓ Process 工具：{}", echo_tool.name());

    // 1.2 HTTP 工具
    let json_placeholder_tool = HTTPWrapperBuilder::new("get_posts", "https://jsonplaceholder.typicode.com")
        .description("Get posts from JSONPlaceholder API")
        .method("GET")
        .path("/posts")
        .input_schema(schema_helpers::create_string_params_schema(vec![]))
        .domain("http_client")
        .risk_level(RiskLevel::Low)
        .build();
    println!("✓ HTTP 工具：{}", json_placeholder_tool.name());

    // ========== 第二部分：注册到外部工具注册表 ==========
    println!("\n📋 第二部分：注册到外部工具注册表\n");

    let mut registry = ExternalToolRegistry::new();
    
    registry.register_from_metadata(echo_tool.metadata().clone())?;
    registry.register_from_metadata(json_placeholder_tool.metadata().clone())?;

    println!("✓ 已注册 {} 个工具到注册表", registry.count());

    // ========== 第三部分：执行工具 ==========
    println!("\n⚡ 第三部分：执行工具\n");

    // 执行 echo 工具
    let result = echo_tool.execute(json!({"message": "Hello EPW!"})).await?;
    println!("✓ echo_message 输出：{}", result.stdout.unwrap_or_default().trim());

    // 执行 HTTP GET
    let result = json_placeholder_tool.execute(json!({})).await?;
    println!("✓ get_posts 执行成功：{}", result.success);
    if let Some(output) = result.output.as_array() {
        println!("✓ 获取到 {} 篇文章", output.len());
    }

    // ========== 第四部分：工具发现 ==========
    println!("\n🔍 第四部分：自动发现工具\n");

    let mut discovery = ExternalToolDiscovery::new();
    let executables = discovery.scan_executables().await?;
    println!("✓ 发现 {} 个可执行文件", executables.len());

    // ========== 第五部分：转换为 ToolDefinition ==========
    println!("\n🔧 第五部分：转换为 ToolMatrix 兼容格式\n");

    let tool_defs: Vec<ToolDefinition> = registry.get_all_tool_definitions();
    println!("✓ 转换了 {} 个 ToolDefinition", tool_defs.len());

    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║                    演示完成                              ║");
    println!("╚═══════════════════════════════════════════════════════════╝");

    Ok(())
}
```

## 运行步骤

1. 将上面的代码复制到 `src/bin/` 目录
2. 运行：`cargo run --bin <example_name>`

例如：
```bash
cargo run --bin epw_git_example
cargo run --bin epw_github_example
cargo run --bin epw_python_script_example
cargo run --bin epw_full_demo
```

## 更多文档

- [用户指南](../docs/EXTERNAL_PROCESS_WRAPPER_USER_GUIDE.md)
- [计划文档](../docs/EXTERNAL_PROCESS_WRAPPER_PLAN.json)
