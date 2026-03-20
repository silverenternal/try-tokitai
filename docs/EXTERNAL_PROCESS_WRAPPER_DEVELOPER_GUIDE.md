# External Process Wrapper (EPW) 开发者指南

> **扩展 tokitai 工具生态系统的高级指南**
>
> 本文档面向希望扩展 EPW 系统或深度集成的开发者。

[![Status](https://img.shields.io/badge/status-stable-brightgreen)]()
[![Coverage](https://img.shields.io/badge/coverage-90%25-brightgreen)]()

---

## 📖 目录

- [架构设计](#架构设计)
- [核心 API 参考](#核心-api-参考)
- [扩展指南](#扩展指南)
- [最佳实践](#最佳实践)
- [故障排查](#故障排查)

---

## 架构设计

### 分层架构

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

### 模块结构

```
src/external_process/
├── mod.rs              # 模块入口，导出公共 API
├── wrapper.rs          # 核心 ExternalTool trait 定义
├── metadata.rs         # 元数据结构定义
├── process_wrapper.rs  # 本地进程封装实现
├── http_wrapper.rs     # HTTP 服务封装实现
├── script_wrapper.rs   # 脚本文件封装实现
├── discovery.rs        # 自动发现器
├── registry.rs         # 外部工具注册表
└── tests/              # 测试目录
```

### 数据流

```
Task Failure → ToolGapDetector → Decision Tree
                                           ├──→ ExternalToolDiscovery
                                           │        ↓
                                           │    ExternalToolRegistry
                                           │        ↓
                                           └──→ ToolMatrix → AI Agent
```

---

## 核心 API 参考

### ExternalTool Trait

所有外部工具必须实现的核心 trait：

```rust
#[async_trait::async_trait]
pub trait ExternalTool: Send + Sync {
    /// 获取工具元数据
    fn metadata(&self) -> &ExternalToolMetadata;

    /// 执行工具
    async fn execute(&self, input: Value) -> Result<ToolExecutionResult>;

    /// 验证输入
    fn validate_input(&self, input: &Value) -> Result<()>;

    /// 转换为 ToolDefinition
    fn to_tool_definition(&self) -> ToolDefinition;

    // 默认实现的方法
    fn domain(&self) -> &str { ... }
    fn name(&self) -> &str { ... }
    fn description(&self) -> &str { ... }
    fn is_enabled(&self) -> bool { ... }
    fn risk_level(&self) -> RiskLevel { ... }
    fn tags(&self) -> &[String] { ... }
}
```

### ExternalToolMetadata

工具元数据结构：

```rust
pub struct ExternalToolMetadata {
    pub name: String,                      // 工具名称
    pub description: String,               // 工具描述
    pub tool_type: ExternalToolType,       // 工具类型
    pub input_schema: Value,               // 输入 Schema (JSON Schema)
    pub output_schema: Option<Value>,      // 输出 Schema
    pub domain: String,                    // 所属领域
    pub tags: Vec<String>,                 // 标签
    pub risk_level: RiskLevel,             // 风险等级
    pub created_by: String,                // 创建者
    pub created_at: u64,                   // 创建时间戳
    pub enabled: bool,                     // 是否启用
}
```

### ExternalToolType

工具类型枚举：

```rust
pub enum ExternalToolType {
    Process { config: ProcessConfig },     // 本地进程
    Http { config: HttpConfig },           // HTTP 服务
    Script { config: ScriptConfig },       // 脚本文件
}
```

### Builder 模式

所有包装器都提供 Builder 模式：

```rust
// ProcessWrapperBuilder
let wrapper = ProcessWrapperBuilder::new("git", "git")
    .description("Git version control")
    .args(vec!["{{command}}".to_string()])
    .input_schema(schema)
    .domain("version_control")
    .tag("git")
    .risk_level(RiskLevel::Medium)
    .build();

// HTTPWrapperBuilder
let wrapper = HTTPWrapperBuilder::new("github_api", "https://api.github.com")
    .description("GitHub REST API")
    .method("GET")
    .path("/users/{{username}}")
    .auth(AuthConfig::BearerToken { token_env: "GITHUB_TOKEN".to_string() })
    .input_schema(schema)
    .domain("http_client")
    .risk_level(RiskLevel::Low)
    .build();

// ScriptWrapperBuilder
let wrapper = ScriptWrapperBuilder::new("analyze", "scripts/analyze.py")
    .description("Data analysis script")
    .interpreter("python3")
    .args(vec!["--input".to_string(), "{{input}}".to_string()])
    .input_schema(schema)
    .domain("data_analysis")
    .build();
```

---

## 扩展指南

### 添加新的工具类型

1. **扩展 ExternalToolType 枚举**

```rust
// 在 metadata.rs 中添加
pub enum ExternalToolType {
    Process { config: ProcessConfig },
    Http { config: HttpConfig },
    Script { config: ScriptConfig },
    // 新增类型
    WebSocket { config: WebSocketConfig },
}
```

2. **实现 ExternalTool trait**

```rust
pub struct WebSocketWrapper {
    metadata: ExternalToolMetadata,
    client: WebSocketClient,
}

#[async_trait::async_trait]
impl ExternalTool for WebSocketWrapper {
    fn metadata(&self) -> &ExternalToolMetadata {
        &self.metadata
    }

    async fn execute(&self, input: Value) -> Result<ToolExecutionResult> {
        // 实现 WebSocket 调用逻辑
    }

    fn validate_input(&self, input: &Value) -> Result<()> {
        validation::validate_json_schema(input, &self.metadata.input_schema)
    }

    fn to_tool_definition(&self) -> ToolDefinition {
        // 转换为 ToolDefinition
    }
}
```

3. **添加 Builder**

```rust
pub struct WebSocketWrapperBuilder {
    name: String,
    url: String,
    description: Option<String>,
    // ...
}

impl WebSocketWrapperBuilder {
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Self {
        // ...
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    // ... 其他 builder 方法

    pub fn build(self) -> WebSocketWrapper {
        let metadata = ExternalToolMetadata::new(
            self.name,
            self.description.unwrap_or_default(),
            ExternalToolType::WebSocket { config: self.config },
            self.input_schema.unwrap_or(default_schema()),
            self.domain.unwrap_or_else(|| "websocket".to_string()),
            self.created_by.unwrap_or_else(|| "builder".to_string()),
        );
        WebSocketWrapper::new(metadata)
    }
}
```

### 集成到自进化系统

在 `SelfImprovementLoop` 中添加新的决策逻辑：

```rust
impl SelfImprovementLoop {
    fn decide_tool_creation_strategy(&self, gap: &ToolGap) -> ToolCreationStrategy {
        let desc_lower = gap.description.to_lowercase();

        // 检查是否有匹配的 WebSocket 服务
        if self.check_existing_websocket(&desc_lower) {
            ToolCreationStrategy::ExternalTool
        }
        // ... 其他决策逻辑
        else {
            ToolCreationStrategy::RustTool
        }
    }

    fn wrap_websocket_for_gap(&self, gap: &ToolGap) -> Result<String> {
        // 创建 WebSocket 工具封装
        let metadata = self.create_websocket_tool_metadata(gap)?;
        let registry = self.external_tool_registry.read();
        registry.register_from_metadata(metadata.clone())?;
        Ok(metadata.name)
    }
}
```

### 自定义发现器

扩展 `ExternalToolDiscovery` 以支持新的发现源：

```rust
impl ExternalToolDiscovery {
    /// 从 Docker 容器发现工具
    pub async fn from_docker(&mut self, container_name: &str) -> Result<Vec<ExternalToolMetadata>> {
        // 扫描 Docker 容器中的可执行文件
        // 生成工具元数据
    }

    /// 从 Kubernetes 服务发现工具
    pub async fn from_k8s_service(&mut self, namespace: &str) -> Result<Vec<ExternalToolMetadata>> {
        // 扫描 K8s 服务
        // 生成 HTTP 工具元数据
    }
}
```

---

## 最佳实践

### 1. 输入验证

始终验证输入参数：

```rust
fn validate_input(&self, input: &Value) -> Result<()> {
    // 使用 JSON Schema 验证
    validation::validate_json_schema(input, &self.metadata.input_schema)?;

    // 添加自定义验证逻辑
    if let Some(msg) = input.get("message").and_then(|v| v.as_str()) {
        if msg.is_empty() {
            bail!("Message cannot be empty");
        }
        if msg.len() > 1000 {
            bail!("Message too long (max 1000 chars)");
        }
    }

    Ok(())
}
```

### 2. 错误处理

提供清晰的错误信息：

```rust
async fn execute(&self, input: Value) -> Result<ToolExecutionResult> {
    let start = Instant::now();

    match self.execute_internal(&input).await {
        Ok(output) => Ok(ToolExecutionResult {
            success: true,
            output,
            error: None,
            execution_time_ms: start.elapsed().as_millis() as u64,
            stdout: None,
            stderr: None,
        }),
        Err(e) => Ok(ToolExecutionResult {
            success: false,
            output: Value::Null,
            error: Some(format!("Execution failed: {}", e)),
            execution_time_ms: start.elapsed().as_millis() as u64,
            stdout: None,
            stderr: None,
        }),
    }
}
```

### 3. 超时处理

防止进程/请求挂起：

```rust
async fn execute_with_timeout(&self, input: &Value) -> Result<ToolExecutionResult> {
    let timeout_ms = self.config().timeout_ms;

    tokio::time::timeout(Duration::from_millis(timeout_ms), async {
        self.execute_internal(input).await
    })
    .await
    .unwrap_or_else(|_| Err(anyhow::anyhow!("Execution timed out after {}ms", timeout_ms)))
}
```

### 4. 日志记录

使用 tracing 记录执行信息：

```rust
use tracing::{debug, info, warn, error};

async fn execute(&self, input: Value) -> Result<ToolExecutionResult> {
    debug!("Executing tool {}: {:?}", self.name(), input);

    match self.execute_internal(&input).await {
        Ok(result) => {
            info!("Tool {} executed successfully in {}ms",
                  self.name(), result.execution_time_ms);
            Ok(result)
        }
        Err(e) => {
            error!("Tool {} failed: {}", self.name(), e);
            Ok(ToolExecutionResult::failure(&e.to_string()))
        }
    }
}
```

### 5. 安全风险控制

根据风险等级采取不同措施：

```rust
pub enum RiskLevel {
    Low,      // 直接执行
    Medium,   // 记录日志
    High,     // 需要用户确认
    Critical, // 需要沙箱执行
}

impl ExternalToolRegistry {
    pub async fn execute_with_risk_check(&self, tool_name: &str, input: Value) -> Result<ToolExecutionResult> {
        let tool = self.get_tool(tool_name)?;

        match tool.risk_level() {
            RiskLevel::Low | RiskLevel::Medium => {
                tool.execute(input).await
            }
            RiskLevel::High => {
                // 需要用户确认
                if !self.confirm_with_user(&tool).await? {
                    bail!("User denied execution of high-risk tool");
                }
                tool.execute(input).await
            }
            RiskLevel::Critical => {
                // 沙箱执行
                self.execute_in_sandbox(tool, input).await
            }
        }
    }
}
```

---

## 故障排查

### 常见问题

#### 1. 工具执行失败

**症状**: `execute()` 返回错误

**排查步骤**:
```rust
let result = tool.execute(input).await?;
if !result.success {
    println!("Error: {:?}", result.error);
    println!("Stderr: {:?}", result.stderr);
}
```

**解决方案**:
- 检查可执行文件是否在 PATH 中
- 验证输入参数格式
- 检查环境变量是否设置
- 查看超时设置是否合理

#### 2. 输入验证失败

**症状**: `validate_input()` 返回错误

**排查步骤**:
```rust
match tool.validate_input(&input) {
    Ok(_) => println!("Valid"),
    Err(e) => println!("Validation error: {}", e),
}
```

**解决方案**:
- 检查 JSON Schema 定义是否正确
- 验证必需字段是否存在
- 确认字段类型匹配

#### 3. HTTP 认证失败

**症状**: 401/403 错误

**排查步骤**:
```rust
// 检查环境变量
println!("Token set: {}", std::env::var("GITHUB_TOKEN").is_ok());

// 检查认证配置
println!("Auth config: {:?}", tool.metadata().tool_type);
```

**解决方案**:
- 确保环境变量已设置
- 验证 token 是否有效
- 检查认证类型是否正确

#### 4. 脚本解释器找不到

**症状**: "No such file or directory"

**排查步骤**:
```rust
// 检查解释器是否存在
which::which("python3").ok();
which::which("node").ok();
```

**解决方案**:
- 安装相应解释器
- 使用绝对路径
- 检查 PATH 环境变量

### 调试技巧

#### 启用详细日志

```rust
// 在 main.rs 中设置
use tracing_subscriber::{fmt, EnvFilter};

fmt()
    .with_env_filter(EnvFilter::from_default_env())
    .init();

// 运行前设置环境变量
// export RUST_LOG=debug
```

#### 捕获执行输出

```rust
let result = tool.execute(input).await?;
println!("Stdout: {}", result.stdout.unwrap_or_default());
println!("Stderr: {}", result.stderr.unwrap_or_default());
println!("Output: {}", result.output);
```

#### 性能分析

```rust
use std::time::Instant;

let start = Instant::now();
let result = tool.execute(input).await?;
println!("Execution time: {}ms (reported: {}ms)",
         start.elapsed().as_millis(),
         result.execution_time_ms);
```

---

## 测试指南

### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_metadata_creation() {
        let config = ProcessConfig::new("test");
        let metadata = ExternalToolMetadata::new(
            "test_tool",
            "Test",
            ExternalToolType::process(config),
            json!({"type": "object"}),
            "test",
            "test_user",
        );
        assert_eq!(metadata.name, "test_tool");
    }

    #[tokio::test]
    async fn test_execute_success() {
        let wrapper = create_test_wrapper();
        let result = wrapper.execute(json!({})).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_execute_timeout() {
        let wrapper = create_slow_wrapper();
        let result = wrapper.execute(json!({})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timed out"));
    }

    #[test]
    fn test_validate_input_missing_field() {
        let wrapper = create_wrapper_with_schema();
        let input = json!({});  // Missing required field
        assert!(wrapper.validate_input(&input).is_err());
    }
}
```

### 集成测试

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::tool_matrix::registry::ToolRegistry;

    #[tokio::test]
    async fn test_register_to_matrix() {
        let wrapper = create_test_wrapper();
        let registry = ToolRegistry::new().await;

        let tool_def = wrapper.to_tool_definition();
        let result = registry.register_tool(tool_def, ToolSource::Dynamic).await;

        assert!(result.is_ok());

        // Verify tool is registered
        let tools = registry.list_tools().await;
        assert!(tools.iter().any(|t| t.name == wrapper.name()));
    }

    #[tokio::test]
    async fn test_discovery_and_registration() {
        let mut discovery = ExternalToolDiscovery::new();
        let tools = discovery.scan_executables().await.unwrap();

        let registry = ExternalToolRegistry::new();
        for tool in &tools[..5] {  // Register first 5 tools
            registry.register_from_metadata(tool.clone()).unwrap();
        }

        assert!(registry.count() >= 5);
    }
}
```

---

## 性能优化

### 1. 连接池（HTTP 工具）

```rust
lazy_static! {
    static ref HTTP_CLIENT: reqwest::Client = reqwest::Client::builder()
        .pool_max_idle_per_host(10)
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
}
```

### 2. 进程池（Process 工具）

```rust
pub struct ProcessPool {
    pool: moka::future::Cache<String, Child>,
}

impl ProcessPool {
    pub async fn get_or_spawn(&self, cmd: &str) -> Result<Child> {
        if let Some(child) = self.pool.get(cmd).await {
            Ok(child)
        } else {
            let child = Command::new(cmd).spawn()?;
            self.pool.insert(cmd.to_string(), child).await;
            Ok(self.pool.get(cmd).await.unwrap())
        }
    }
}
```

### 3. 结果缓存

```rust
use moka::future::Cache;

pub struct CachedTool {
    cache: Cache<Value, ToolExecutionResult>,
    inner: Box<dyn ExternalTool>,
}

#[async_trait::async_trait]
impl ExternalTool for CachedTool {
    async fn execute(&self, input: Value) -> Result<ToolExecutionResult> {
        if let Some(cached) = self.cache.get(&input).await {
            return Ok(cached);
        }

        let result = self.inner.execute(input.clone()).await?;
        self.cache.insert(input, result.clone()).await;
        Ok(result)
    }
}
```

---

## 贡献指南

### 提交新的工具封装

1. 在 `src/external_process/` 创建新文件
2. 实现 `ExternalTool` trait
3. 添加 Builder 模式
4. 编写单元测试（覆盖率 >90%）
5. 编写集成测试
6. 更新文档

### 代码风格

- 遵循 Rust 官方风格指南
- 使用 `rustfmt` 格式化代码
- 使用 `clippy` 检查代码质量
- 所有公共 API 必须有文档注释

### 测试要求

```bash
# 运行所有测试
cargo test --release

# 检查覆盖率
cargo tarpaulin --out Html

# 运行 clippy
cargo clippy -- -D warnings
```

---

## 参考资料

- [用户指南](EXTERNAL_PROCESS_WRAPPER_USER_GUIDE.md)
- [计划文档](EXTERNAL_PROCESS_WRAPPER_PLAN.json)
- [Tokitai 文档](https://docs.rs/tokitai)
- [Rust Async 编程](https://rust-lang.github.io/async-book/)
