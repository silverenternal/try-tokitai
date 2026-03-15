# 服务化架构落地方案

> **P11 级视角**：基于 tokitai ToolProvider 的服务化架构演进，实现"一切皆服务"的 AI 原生架构
> 
> **核心理念**：工具矩阵即服务注册表，AI 即服务消费者，ToolProvider 即服务提供者

---

## 📋 执行摘要

### 当前状态（2026-03-15 更新）

| 模块 | 状态 | 代码量 | 服务化状态 |
|------|------|--------|-----------|
| `dialogue` | ✅ 已集成 | 443 行 | ✅ ToolProvider 封装 |
| `observability` | ✅ 已集成 | 456 行 | ✅ ToolProvider 封装 |
| `prompt_engineering` | ✅ 已集成 | 965 行 | ✅ ToolProvider 封装 |
| `tools/*` | ✅ 已集成 | 7,114 行 | ✅ ToolProvider 封装 |
| `context` | ✅ 已集成 | 4,794 行 | ⚠️ 服务接口待统一 |

### 架构定位

```
┌─────────────────────────────────────────────────┐
│              AI (LLM)                            │
│         服务消费者 / Service Consumer            │
└─────────────────┬───────────────────────────────┘
                  │ 自然语言调用
                  ▼
┌─────────────────────────────────────────────────┐
│           工具矩阵服务层                          │
│  ┌───────────────────────────────────────────┐  │
│  │  ToolRegistry  → 服务注册表                │  │
│  │  ToolSelector  → 服务发现/路由             │  │
│  │  SkillsManager → 服务文档                  │  │
│  └───────────────────────────────────────────┘  │
└─────────────────┬───────────────────────────────┘
                  │ 服务调用
                  ▼
┌─────────────────────────────────────────────────┐
│         ToolProvider 服务提供者                  │
│  FileOps | System | Network | Dialogue | ...   │
└─────────────────────────────────────────────────┘
```

---

## 🎯 服务化架构演进路线

### 阶段一：✅ 已完成 - ToolProvider 统一抽象

**核心成果**：
- 所有工具封装为 `tokitai::ToolProvider`
- 统一的服务接口：`tool_definitions()` + `call_tool()`
- 编译时类型安全，零运行时开销

### 阶段二：🔄 进行中 - 服务元数据增强

**目标**：为工具定义增加服务化元数据，支持 QoS、依赖、健康检查

### 阶段三：⏳ 规划中 - 服务生命周期管理

**目标**：统一服务启动、健康检查、优雅关闭

### 阶段四：⏳ 规划中 - 服务网格能力

**目标**：重试、熔断、超时、服务组合

---

## 🛠️ 具体落地方案

### 方案一：服务元数据增强（优先级 P0）

#### 1.1 扩展 ToolDefinition

**文件**: `src/tool_matrix/matrix.rs`

```rust
/// 工具定义（服务化增强版）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    // === 基础信息（现有）===
    pub name: String,
    pub description: String,
    pub input_schema: String,
    
    // === 服务元数据（新增）===
    #[serde(default)]
    pub metadata: ServiceMetadata,
}

/// 服务元数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceMetadata {
    /// 服务分类
    #[serde(default)]
    pub category: ServiceCategory,
    
    /// 服务质量指标
    #[serde(default)]
    pub qos: QualityOfService,
    
    /// 服务依赖（依赖的其他工具）
    #[serde(default)]
    pub dependencies: Vec<String>,
    
    /// 速率限制
    #[serde(default)]
    pub rate_limit: Option<RateLimitConfig>,
    
    /// 服务版本
    #[serde(default = "default_version")]
    pub version: String,
    
    /// 标签（用于服务发现）
    #[serde(default)]
    pub tags: Vec<String>,
}

/// 服务分类
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceCategory {
    #[default]
    Utility,      // 通用工具
    File,         // 文件操作
    Network,      // 网络操作
    System,       // 系统操作
    Data,         // 数据处理
    Ai,           // AI 相关
    Vcs,          // 版本控制
}

/// 服务质量指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityOfService {
    /// P99 延迟（毫秒）
    #[serde(default = "default_latency")]
    pub latency_p99_ms: u64,
    
    /// 成功率（0-1）
    #[serde(default = "default_success_rate")]
    pub success_rate: f32,
    
    /// 最大并发度
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    
    /// 是否幂等
    #[serde(default)]
    pub idempotent: bool,
}

fn default_latency() -> u64 { 1000 }
fn default_success_rate() -> f32 { 0.99 }
fn default_concurrency() -> usize { 10 }
fn default_version() -> String { "1.0.0".to_string() }

impl Default for QualityOfService {
    fn default() -> Self {
        Self {
            latency_p99_ms: default_latency(),
            success_rate: default_success_rate(),
            concurrency: default_concurrency(),
            idempotent: false,
        }
    }
}

/// 速率限制配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// 每秒最大请求数
    pub requests_per_second: u32,
    /// 突发容量
    pub burst_size: u32,
}
```

#### 1.2 在 ToolProvider 宏中自动填充元数据

**文件**: 依赖 tokitai 库升级，或本地扩展宏

```rust
// 使用示例（tokitai 宏扩展）
#[tool(metadata(
    category = "network",
    latency_p99_ms = 500,
    idempotent = true,
    tags = ["http", "read-only"]
))]
impl HttpClientTools {
    pub fn get(&self, url: String) -> Result<String, String> { ... }
}
```

#### 1.3 服务统计收集

**文件**: `src/tool_matrix/matrix.rs`（新增）

```rust
/// 服务运行时统计
#[derive(Debug, Clone, Default)]
pub struct ServiceStats {
    /// 总请求数
    pub total_requests: u64,
    /// 成功数
    pub success_count: u64,
    /// 失败数
    pub failure_count: u64,
    /// 平均延迟（毫秒）
    pub avg_latency_ms: f64,
    /// P99 延迟（毫秒）
    pub p99_latency_ms: u64,
    /// 最后调用时间
    pub last_called_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl ServiceStats {
    /// 记录请求
    pub fn record(&mut self, success: bool, latency_ms: u64) {
        self.total_requests += 1;
        if success {
            self.success_count += 1;
        } else {
            self.failure_count += 1;
        }
        
        // 更新平均延迟
        let total = self.total_requests as f64;
        self.avg_latency_ms = (self.avg_latency_ms * (total - 1.0) + latency_ms as f64) / total;
        
        // 简化版 P99 计算（实际可用 hdrhistogram）
        self.p99_latency_ms = self.p99_latency_ms.max(latency_ms);
        
        self.last_called_at = Some(chrono::Utc::now());
    }
    
    /// 获取成功率
    pub fn success_rate(&self) -> f32 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.success_count as f32 / self.total_requests as f32
        }
    }
}
```

**收益**：
- ✅ AI 可根据 QoS 选择工具
- ✅ 支持服务降级决策
- ✅ 运行时性能监控

---

### 方案二：服务生命周期管理（优先级 P1）

#### 2.1 定义服务生命周期 Trait

**文件**: `src/tool_matrix/matrix.rs`（新增）

```rust
/// 服务健康状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceHealth {
    /// 健康
    Healthy,
    /// 降级（部分功能可用）
    Degraded,
    /// 不健康（不可用）
    Unhealthy,
    /// 未知（未检查）
    Unknown,
}

/// 服务生命周期接口
pub trait ServiceLifecycle {
    /// 服务名称
    fn service_name(&self) -> &str;
    
    /// 初始化（连接池、缓存预热等）
    fn init(&mut self) -> Result<(), String> {
        Ok(())
    }
    
    /// 健康检查
    fn health(&self) -> ServiceHealth {
        ServiceHealth::Unknown
    }
    
    /// 优雅关闭
    fn shutdown(&mut self) -> Result<(), String> {
        Ok(())
    }
    
    /// 服务统计
    fn stats(&self) -> ServiceStats {
        ServiceStats::default()
    }
}
```

#### 2.2 为现有工具实现生命周期

**文件**: `src/tools/network/http_client.rs`（示例）

```rust
use crate::tool_matrix::matrix::{ServiceLifecycle, ServiceHealth, ServiceStats};

impl ServiceLifecycle for HttpClientTools {
    fn service_name(&self) -> &str {
        "http_client"
    }
    
    fn init(&mut self) -> Result<(), String> {
        // 预热 HTTP 连接池
        self.warmup_connections()
            .map_err(|e| format!("连接池预热失败：{}", e))?;
        
        tracing::info!("HTTP 客户端服务初始化完成");
        Ok(())
    }
    
    fn health(&self) -> ServiceHealth {
        if self.client.is_available() {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Degraded
        }
    }
    
    fn shutdown(&mut self) -> Result<(), String> {
        // 关闭连接池
        self.close_connections();
        tracing::info!("HTTP 客户端服务已关闭");
        Ok(())
    }
    
    fn stats(&self) -> ServiceStats {
        self.stats.clone()  // 假设内部已有统计
    }
}
```

#### 2.3 在 main.rs 中统一管理生命周期

**文件**: `src/main.rs`

```rust
impl AiAssistant {
    /// 初始化所有服务
    pub fn init_all_services(&mut self) -> Result<()> {
        tracing::info!("正在初始化所有服务...");
        
        // 初始化 HTTP 客户端
        self.http_client.init().map_err(|e| anyhow::anyhow!(e))?;
        
        // 初始化其他需要初始化的服务
        // ...
        
        tracing::info!("所有服务初始化完成");
        Ok(())
    }
    
    /// 健康检查
    pub fn health_check(&self) -> ServiceHealthReport {
        ServiceHealthReport {
            http_client: self.http_client.health(),
            // ... 其他服务
        }
    }
    
    /// 优雅关闭
    pub fn shutdown(&mut self) -> Result<()> {
        tracing::info!("正在关闭所有服务...");
        
        self.http_client.shutdown().map_err(|e| anyhow::anyhow!(e))?;
        // ... 其他服务
        
        tracing::info!("所有服务已关闭");
        Ok(())
    }
}

/// 服务健康报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealthReport {
    pub http_client: ServiceHealth,
    // ... 其他服务
}
```

**收益**：
- ✅ 统一的服务管理接口
- ✅ 支持服务预热和优雅关闭
- ✅ 健康检查支持

---

### 方案三：服务组合/编排增强（优先级 P0）

#### 3.1 声明式工作流定义

**文件**: `src/orchestrator/workflow.rs`（扩展现有）

```rust
use serde::{Deserialize, Serialize};

/// 声明式工作流定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclarativeWorkflow {
    /// 工作流 ID
    pub id: String,
    /// 工作流名称
    pub name: String,
    /// 工作流描述
    pub description: String,
    /// 工作流步骤
    pub steps: Vec<WorkflowStep>,
}

/// 工作流步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// 步骤 ID
    pub id: String,
    /// 步骤描述
    pub description: String,
    /// 使用的工具
    pub tool: String,
    /// 工具参数（支持模板）
    pub arguments: Value,
    /// 前置步骤依赖
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// 重试配置
    #[serde(default)]
    pub retry: RetryConfig,
    /// 超时配置（秒）
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    /// 错误处理
    #[serde(default)]
    pub on_error: Option<ErrorHandler>,
}

/// 重试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// 最大重试次数
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// 重试间隔（毫秒）
    #[serde(default = "default_retry_interval")]
    pub retry_interval_ms: u64,
    /// 是否指数退避
    #[serde(default)]
    pub exponential_backoff: bool,
}

fn default_max_retries() -> u32 { 3 }
fn default_retry_interval() -> u64 { 1000 }

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: default_max_retries(),
            retry_interval_ms: default_retry_interval(),
            exponential_backoff: true,
        }
    }
}

/// 错误处理器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorHandler {
    /// 错误处理策略
    pub strategy: ErrorStrategy,
    ///  fallback 工具（可选）
    pub fallback_tool: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorStrategy {
    /// 重试
    Retry,
    /// 跳过
    Skip,
    /// 失败
    Fail,
    /// 使用 fallback 工具
    Fallback,
}
```

#### 3.2 工作流执行引擎增强

**文件**: `src/orchestrator/workflow.rs`

```rust
impl WorkflowEngine {
    /// 执行声明式工作流
    pub async fn execute_declarative(
        &mut self,
        workflow: &DeclarativeWorkflow,
        context: &mut WorkflowContext,
    ) -> Result<WorkflowResult> {
        tracing::info!("执行声明式工作流：{}", workflow.name);
        
        let mut step_results = HashMap::new();
        
        for step in &workflow.steps {
            // 检查依赖
            if !self.check_dependencies(&step.depends_on, &step_results) {
                tracing::warn!("步骤 {} 依赖未满足，跳过", step.id);
                continue;
            }
            
            // 执行步骤（带重试和超时）
            match self.execute_step_with_retry(step, context).await {
                Ok(result) => {
                    step_results.insert(step.id.clone(), result);
                }
                Err(e) => {
                    // 错误处理
                    match &step.on_error {
                        Some(handler) => {
                            match handler.strategy {
                                ErrorStrategy::Skip => {
                                    tracing::warn!("跳过步骤 {}: {}", step.id, e);
                                }
                                ErrorStrategy::Fail => {
                                    return Err(e);
                                }
                                // ... 其他策略
                            }
                        }
                        None => {
                            return Err(e);
                        }
                    }
                }
            }
        }
        
        Ok(WorkflowResult {
            workflow_id: workflow.id.clone(),
            step_results,
        })
    }
    
    /// 执行步骤（带重试）
    async fn execute_step_with_retry(
        &self,
        step: &WorkflowStep,
        context: &mut WorkflowContext,
    ) -> Result<Value> {
        let mut attempts = 0;
        let mut delay = step.retry.retry_interval_ms;
        
        loop {
            match self.execute_single_step(step, context).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    attempts += 1;
                    if attempts >= step.retry.max_retries {
                        return Err(e);
                    }
                    
                    // 等待重试
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    
                    // 指数退避
                    if step.retry.exponential_backoff {
                        delay *= 2;
                    }
                }
            }
        }
    }
}
```

#### 3.3 YAML 工作流定义（可选）

**文件**: `workflows/research_and_write.yaml`（新建）

```yaml
id: research_and_write
name: 研究并撰写报告
description: 搜索网络内容，获取详细信息，总结并写入文件

steps:
  - id: search_web
    description: 搜索相关网页内容
    tool: web_search
    arguments:
      query: "{{query}}"
      num_results: 10
    retry:
      max_retries: 3
      exponential_backoff: true
    timeout_secs: 30

  - id: fetch_content
    description: 获取网页详细内容
    tool: http_get
    arguments:
      url: "{{search_web.results.0.url}}"
    depends_on:
      - search_web
    retry:
      max_retries: 2
    timeout_secs: 15

  - id: summarize
    description: 总结内容
    tool: dialogue_tools
    arguments:
      prompt: "总结以下内容：{{fetch_content.content}}"
    depends_on:
      - fetch_content

  - id: write_file
    description: 写入报告文件
    tool: write_file
    arguments:
      path: "./reports/{{query}}.md"
      content: "{{summarize.response}}"
    depends_on:
      - summarize
```

**收益**：
- ✅ 声明式工作流定义
- ✅ 内置重试/超时/错误处理
- ✅ 支持服务组合

---

### 方案四：服务可观测性增强（优先级 P1）

#### 4.1 统一服务指标

**文件**: `src/tool_matrix/matrix.rs`（新增）

```rust
/// 服务指标收集器
pub struct ServiceMetricsCollector {
    metrics: Arc<RwLock<HashMap<String, ServiceMetrics>>>,
}

impl ServiceMetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// 记录工具调用
    pub fn record_call(&self, tool_name: &str, success: bool, latency_ms: u64) {
        let mut metrics = self.metrics.write();
        let entry = metrics.entry(tool_name.to_string()).or_insert_with(|| {
            ServiceMetrics::new(tool_name.to_string())
        });
        entry.record(success, latency_ms);
    }
    
    /// 获取服务指标
    pub fn get_metrics(&self, tool_name: &str) -> Option<ServiceMetrics> {
        self.metrics.read().get(tool_name).cloned()
    }
    
    /// 获取所有服务指标
    pub fn get_all_metrics(&self) -> Vec<ServiceMetrics> {
        self.metrics.read().values().cloned().collect()
    }
}

/// 服务指标
#[derive(Debug, Clone)]
pub struct ServiceMetrics {
    pub service_name: String,
    pub stats: ServiceStats,
    /// 错误分布
    pub error_distribution: HashMap<String, u64>,
    /// 延迟直方图（简化版）
    pub latency_buckets: Vec<LatencyBucket>,
}

#[derive(Debug, Clone)]
pub struct LatencyBucket {
    pub range: String,  // "0-10ms", "10-100ms", etc.
    pub count: u64,
}
```

#### 4.2 在 call_tool 中自动记录指标

**文件**: `src/main.rs`

```rust
impl AiAssistant {
    pub fn call_tool(&self, name: &str, args: &Value) -> Result<String> {
        let start = std::time::Instant::now();
        
        // ... 现有工具调用逻辑 ...
        
        let result = try_tool!(self.file_ops, "file_ops");
        // ... 其他工具 ...
        
        // 记录指标
        let latency_ms = start.elapsed().as_millis() as u64;
        let success = result.is_ok();
        self.metrics_collector.record_call(name, success, latency_ms);
        
        result
    }
    
    /// 获取服务指标（AI 可调用）
    pub fn get_service_metrics(&self, tool_name: Option<String>) -> Value {
        if let Some(name) = tool_name {
            self.metrics_collector.get_metrics(&name)
                .map(|m| serde_json::to_value(m).unwrap_or_default())
                .unwrap_or_default()
        } else {
            // 返回所有服务指标
            serde_json::to_value(self.metrics_collector.get_all_metrics()).unwrap_or_default()
        }
    }
}
```

**收益**：
- ✅ AI 可感知服务健康状态
- ✅ 自动性能分析
- ✅ 更好的故障诊断

---

## 📊 实施优先级

| 方案 | 优先级 | 工作量 | 收益 | 依赖 |
|------|--------|--------|------|------|
| **服务元数据增强** | P0 | 2 天 | 高 | 无 |
| **服务组合/编排增强** | P0 | 3 天 | 高 | 无 |
| **服务生命周期管理** | P1 | 2 天 | 中 | 无 |
| **服务可观测性增强** | P1 | 2 天 | 中 | 无 |

---

## 🚀 快速开始

### 第一步：扩展 ToolDefinition（30 分钟）

```bash
# 编辑文件
code src/tool_matrix/matrix.rs

# 添加 ServiceMetadata、ServiceCategory、QualityOfService 结构
# 修改 ToolDefinition 添加 metadata 字段
```

### 第二步：实现服务生命周期（1 小时）

```bash
# 编辑文件
code src/tool_matrix/matrix.rs

# 添加 ServiceLifecycle trait
# 为 HttpClientTools 实现生命周期
```

### 第三步：声明式工作流（2 小时）

```bash
# 编辑文件
code src/orchestrator/workflow.rs

# 添加 DeclarativeWorkflow、WorkflowStep 结构
# 实现 execute_declarative 方法
```

---

## ✅ 验收标准

1. **服务元数据**
   - [ ] ToolDefinition 包含 metadata 字段
   - [ ] 支持服务分类、QoS、依赖声明
   - [ ] AI 可查询服务 QoS 信息

2. **服务生命周期**
   - [ ] 至少一个工具实现 ServiceLifecycle
   - [ ] 启动时调用 init()
   - [ ] 支持健康检查

3. **服务组合**
   - [ ] 支持声明式工作流定义
   - [ ] 支持重试和超时配置
   - [ ] 至少一个 YAML 工作流示例

4. **可观测性**
   - [ ] 记录服务调用指标
   - [ ] AI 可查询服务指标
   - [ ] 至少一个延迟统计示例

---

**创建时间**: 2026-03-15  
**最后更新**: 2026-03-15  
**优先级**: P0 - 立即执行

---

### 策略二：与 Orchestrator 深度整合

**核心思想**：Orchestrator 作为统一入口，整合 dialogue 和 observability

```rust
pub struct Orchestrator {
    // 现有组件
    role_switcher: RoleSwitcher,
    context_optimizer: ContextOptimizer,
    workflow_engine: Option<WorkflowEngine>,
    
    // 新增组件
    dialogue_state: DialogueStateMachine,      // 对话状态管理
    tracing_recorder: TracingRecorder,         // 全链路追踪
    prompt_manager: PromptTemplateManager,     // 提示词管理
}
```

**状态流转整合**：

```
用户输入
   │
   ▼
┌─────────────────┐
│ DialogueState   │ ◄── 记录到 TraceSpan
│ Idle → Planning │
└─────────────────┘
   │
   ▼
┌─────────────────┐
│ RoleSwitcher    │ ◄── 切换为 Planner 角色
│ → Planner       │
└─────────────────┘
   │
   ▼
┌─────────────────┐
│ PromptManager   │ ◄── 加载 Planner 提示词模板
│ load_template   │
└─────────────────┘
   │
   ▼
┌─────────────────┐
│ AgentExecutor   │ ◄── 执行工具调用（记录 TraceSpan）
│ call_tool       │
└─────────────────┘
   │
   ▼
┌─────────────────┐
│ DialogueState   │ ◄── 状态转换 (Planning → Executing)
│ Planning → Exec │
└─────────────────┘
```

---

## 🛠️ 实施方案

### Phase 1: Dialogue 模块集成（优先级 P0）

#### 1.1 封装为 tokitai ToolProvider

**文件**: `src/dialogue/dialogue_tools.rs` (新建)

```rust
use tokitai::tool;
use super::state_machine::{DialogueStateMachine, DialogueState, DialogueContext};
use anyhow::Result;

/// 对话状态工具集
#[tool]
pub struct DialogueTools {
    state_machine: DialogueStateMachine,
}

#[tool]
impl DialogueTools {
    /// 获取当前对话状态
    #[tool(description = "获取当前对话状态，用于了解任务进度")]
    pub fn get_state(&self) -> Result<String> {
        Ok(self.state_machine.current_state().to_string())
    }
    
    /// 获取对话上下文
    #[tool(description = "获取当前对话的上下文信息，包括任务目标和已执行工具")]
    pub fn get_context(&self) -> Result<DialogueContext> {
        Ok(self.state_machine.get_context().clone())
    }
    
    /// 状态转换
    #[tool(description = "切换到指定状态，用于任务流程控制")]
    pub fn transition(&self, target_state: String) -> Result<String> {
        self.state_machine.transition(target_state.clone())?;
        Ok(format!("状态已转换为：{}", target_state))
    }
    
    /// 记录工具执行
    #[tool(description = "记录已执行的工具，用于追踪任务进度")]
    pub fn record_tool_execution(&self, tool_name: String) -> Result<()> {
        self.state_machine.record_tool(tool_name)?;
        Ok(())
    }
}
```

#### 1.2 集成到 main.rs

```rust
// 在 AiAssistant 结构体中添加
pub struct AiAssistant {
    // ... 现有字段 ...
    
    // 新增：对话状态管理
    dialogue_state: DialogueStateMachine,
    dialogue_tools: DialogueTools,
}

impl AiAssistant {
    pub fn new(...) -> Self {
        // ... 现有代码 ...
        
        let dialogue_state = DialogueStateMachine::new();
        let dialogue_tools = DialogueTools::new(dialogue_state.clone());
        
        // 注册到工具矩阵
        let _ = tool_registry.register_from_provider::<DialogueTools>(
            Some("system"), 
            ToolSource::Builtin
        );
        
        Self {
            // ... 现有字段 ...
            dialogue_state,
            dialogue_tools,
        }
    }
}
```

#### 1.3 与 autonomy 模块状态同步

```rust
// 在 AgentCoordinator 中添加状态同步
impl AgentCoordinator {
    pub fn sync_dialogue_state(&self, dialogue: &mut DialogueStateMachine) {
        match self.state {
            CoordinatorState::Planning => {
                let _ = dialogue.transition(DialogueState::Planning);
            }
            CoordinatorState::Executing => {
                let _ = dialogue.transition(DialogueState::Executing);
            }
            CoordinatorState::Reviewing => {
                let _ = dialogue.transition(DialogueState::Reviewing);
            }
            _ => {}
        }
    }
}
```

---

### Phase 2: Observability 模块集成（优先级 P0）

#### 2.1 与 tracing-subscriber 整合

**文件**: `src/observability/mod.rs` (修改)

```rust
mod tracing;

pub use tracing::{TraceSpan, TracingRecorder, SpanType, TraceContext};

/// 初始化全链路追踪
pub fn init_tracing(log_dir: &str) -> Result<TracingRecorder> {
    // 初始化 tracing-subscriber
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(true)
        .with_thread_ids(true)
        .init();
    
    // 创建追踪记录器
    let recorder = TracingRecorder::new(log_dir)?;
    
    // 注册全局 trace context
    tracing::dispatcher::set_default(&recorder);
    
    Ok(recorder)
}
```

#### 2.2 封装为 tokitai ToolProvider

**文件**: `src/observability/observability_tools.rs` (新建)

```rust
use tokitai::tool;
use super::tracing::{TracingRecorder, TraceSpan};
use anyhow::Result;

/// 可观测性工具集
#[tool]
pub struct ObservabilityTools {
    recorder: TracingRecorder,
}

#[tool]
impl ObservabilityTools {
    /// 查询追踪记录
    #[tool(description = "根据 trace_id 查询完整执行链")]
    pub fn query_trace(&self, trace_id: String) -> Result<Vec<TraceSpan>> {
        self.recorder.query_by_trace_id(&trace_id)
    }
    
    /// 获取最近 N 条追踪记录
    #[tool(description = "获取最近的执行记录，用于调试和审计")]
    pub fn get_recent_traces(&self, limit: usize) -> Result<Vec<TraceSpan>> {
        self.recorder.get_recent(limit)
    }
    
    /// 导出追踪数据
    #[tool(description = "导出追踪数据为 JSON 格式")]
    pub fn export_traces(&self, output_path: String) -> Result<()> {
        self.recorder.export_json(&output_path)
    }
}
```

#### 2.3 在关键位置插入追踪点

```rust
// 在 AiAssistant::run() 中
impl AiAssistant {
    pub async fn run(&mut self, input: &str) -> Result<String> {
        // 创建 trace root span
        let trace_id = Uuid::new_v4().to_string();
        let _span = self.tracing_recorder.start_span(
            &trace_id,
            SpanType::UserRequest,
            "处理用户输入",
            Some(input.to_string()),
        );
        
        // 记录对话状态转换
        self.dialogue_state.transition(DialogueState::Planning)?;
        self.tracing_recorder.record_state_transition(
            &trace_id,
            "Idle",
            "Planning",
        );
        
        // 执行工具调用
        let tool_result = self.call_tool(tool_name, args).await?;
        self.tracing_recorder.record_tool_execution(
            &trace_id,
            tool_name,
            &tool_result,
        );
        
        Ok(response)
    }
}
```

---

### Phase 3: Prompt Engineering 模块集成（优先级 P1）

#### 3.1 与 tokitai ToolProvider 整合

**文件**: `src/prompt_engineering/prompt_tools.rs` (新建)

```rust
use tokitai::tool;
use super::manager::PromptTemplateManager;
use super::renderer::PromptRenderer;
use anyhow::Result;

/// 提示词工具集
#[tool]
pub struct PromptTools {
    manager: PromptTemplateManager,
    renderer: PromptRenderer,
}

#[tool]
impl PromptTools {
    /// 加载角色提示词
    #[tool(description = "加载指定角色的提示词模板")]
    pub fn load_role_template(&self, role: String) -> Result<String> {
        let template = self.manager.load_template(&role)?;
        Ok(template.render(&self.renderer, &json!({})))
    }
    
    /// 渲染提示词
    #[tool(description = "使用给定变量渲染提示词模板")]
    pub fn render_template(&self, role: String, variables: Value) -> Result<String> {
        let template = self.manager.load_template(&role)?;
        Ok(template.render(&self.renderer, &variables))
    }
    
    /// 缓存管理
    #[tool(description = "清除提示词缓存，用于模板更新后重新加载")]
    pub fn clear_cache(&self) -> Result<()> {
        self.manager.clear_cache();
        Ok(())
    }
}
```

#### 3.2 与 Orchestrator 整合

```rust
pub struct Orchestrator {
    // ... 现有字段 ...
    
    // 新增
    prompt_manager: PromptTemplateManager,
    prompt_tools: PromptTools,
}

impl Orchestrator {
    pub fn new() -> Self {
        let prompt_manager = PromptTemplateManager::default();
        let prompt_tools = PromptTools::new(prompt_manager.clone());
        
        Self {
            role_switcher: RoleSwitcher::new(),
            context_optimizer: ContextOptimizer::new(),
            workflow_engine: None,
            config: OrchestratorConfig::default(),
            prompt_manager,
            prompt_tools,
        }
    }
    
    /// 执行工作流（使用提示词模板）
    pub async fn execute_workflow(&self, workflow_name: &str, input: &Value) -> Result<Value> {
        // 根据角色加载提示词模板
        let role = self.role_switcher.current_role();
        let prompt = self.prompt_manager.load_template(role)?;
        
        // 渲染提示词
        let rendered = prompt.render(&self.prompt_tools.renderer, input);
        
        // 执行工作流
        self.workflow_engine.execute(workflow_name, &rendered).await
    }
}
```

---

## 📊 集成后的架构

```
┌─────────────────────────────────────────────────────────────┐
│                      AiAssistant                            │
├─────────────────────────────────────────────────────────────┤
│  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐   │
│  │ DialogueState │◄─┤  Orchestrator │──┤ PromptManager │   │
│  │   (状态机)    │  │   (编排器)    │  │  (提示词)     │   │
│  └───────────────┘  └───────────────┘  └───────────────┘   │
│         │                  │                    │           │
│         ▼                  ▼                    ▼           │
│  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐   │
│  │DialogueTools  │  │RoleSwitcher   │  │  PromptTools  │   │
│  │  (tokitai)    │  │  (角色切换)   │  │   (tokitai)   │   │
│  └───────────────┘  └───────────────┘  └───────────────┘   │
│                              │                              │
│                              ▼                              │
│  ┌───────────────────────────────────────────────────────┐ │
│  │              ToolMatrix (工具矩阵)                     │ │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────┐ │ │
│  │  │ file_   │ │ system  │ │  web    │ │ observability│ │ │
│  │  │  ops    │ │ +dialogue│ │         │ │  (tokitai)  │ │ │
│  │  └─────────┘ └─────────┘ └─────────┘ └─────────────┘ │ │
│  └───────────────────────────────────────────────────────┘ │
│                              │                              │
│                              ▼                              │
│  ┌───────────────────────────────────────────────────────┐ │
│  │           TracingRecorder (全链路追踪)                 │ │
│  │   记录：用户输入 → 状态转换 → 工具调用 → 响应生成      │ │
│  └───────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

## 🔧 具体实施步骤

### 步骤 1: 创建 tokitai ToolProvider 封装

```bash
# 创建新文件
touch src/dialogue/dialogue_tools.rs
touch src/observability/observability_tools.rs
touch src/prompt_engineering/prompt_tools.rs
```

### 步骤 2: 修改 main.rs 整合模块

```rust
// 在 src/main.rs 中添加
mod dialogue;
mod observability;
mod prompt_engineering;

use dialogue::{DialogueStateMachine, DialogueTools};
use observability::{TracingRecorder, ObservabilityTools};
use prompt_engineering::{PromptTemplateManager, PromptTools};
```

### 步骤 3: 注册到工具矩阵

```rust
// 在 AiAssistant::new() 中
let _ = tool_registry.register_from_provider::<DialogueTools>(
    Some("system"), ToolSource::Builtin
);
let _ = tool_registry.register_from_provider::<ObservabilityTools>(
    Some("system"), ToolSource::Builtin
);
let _ = tool_registry.register_from_provider::<PromptTools>(
    Some("system"), ToolSource::Builtin
);
```

### 步骤 4: 在关键位置插入追踪点

```rust
// 在 AiAssistant::run() 中
let trace_id = Uuid::new_v4().to_string();
let _span = tracing_recorder.start_span(&trace_id, ...);
```

### 步骤 5: 状态同步

```rust
// 在 autonomy 模块中
coordinator.sync_dialogue_state(&mut dialogue_state);
```

---

## 📈 预期收益

| 指标 | 集成前 | 集成后 | 提升 |
|------|--------|--------|------|
| 状态管理 | 分散 | 统一 | +100% |
| 可观测性 | 基础 tracing | 全链路追踪 | +200% |
| 提示词复用 | 硬编码 | 模板化 | +50% |
| 工具数量 | 54 | 63 | +17% |
| 代码复用 | - | tokitai 宏 | +30% |

---

## ✅ 验收标准

1. **功能完整性**
   - [ ] dialogue 状态机可在 CLI 中查询和切换
   - [ ] observability 可记录完整执行链
   - [ ] prompt_engineering 可加载和渲染模板

2. **集成质量**
   - [ ] 所有新工具注册到 tool_matrix
   - [ ] 与 autonomy 模块状态同步
   - [ ] 与 orchestrator 深度整合

3. **测试覆盖**
   - [ ] 新增测试覆盖 dialogue 工具
   - [ ] 新增测试覆盖 observability 工具
   - [ ] 新增测试覆盖 prompt_engineering 工具

4. **文档完善**
   - [ ] 更新 PROJECT_STRUCTURE.md
   - [ ] 更新 QUICK_REFERENCE.md
   - [ ] 新增工具使用示例

---

## 🚀 后续优化方向

1. **AI 自主管理状态**：让 autonomy agents 可以自动调整 dialogue state
2. **智能提示词推荐**：根据任务类型自动推荐提示词模板
3. **追踪数据分析**：使用 AI 分析追踪数据，发现性能瓶颈
4. **分布式追踪**：支持多实例追踪数据聚合

---

**创建时间**: 2026-03-15
**优先级**: P0 - 立即执行
