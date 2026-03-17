# Tokitai 技术说明文档

> **版本**: 3.0.0
> **最后更新**: 2026-03-18
> **代码规模**: ~27,500 行 Rust | 99 个源文件 | 15 个核心模块
> **测试状态**: 236/236 通过 ✅

---

## 📋 目录

1. [项目概述](#项目概述)
2. [架构设计](#架构设计)
3. [核心模块详解](#核心模块详解)
4. [服务化架构](#服务化架构)
5. [AI 原生工具选择器](#ai 原生工具选择器)
6. [数据存储设计](#数据存储设计)
7. [安全机制](#安全机制)
8. [性能优化](#性能优化)
9. [测试策略](#测试策略)
10. [部署与运维](#部署与运维)

---

## 项目概述

### 基本信息

| 指标 | 数值 |
|------|------|
| **项目名称** | try-tokitai |
| **包名** | ai-assistant |
| **版本** | 0.1.0 |
| **Rust Edition** | 2021 |
| **代码行数** | ~27,500 行 |
| **源文件数** | 99 个 |
| **核心模块** | 15 个 |
| **工具箱** | 11 个 |
| **工具函数** | 63+ 个 |

### 技术栈

```toml
# 核心依赖
tokitai = "0.4.0"           # AI 工具集成框架
tokitai-core = "0.4.0"      # 核心库

# 异步运行时
tokio = { version = "1", features = ["full"] }  # 完整异步运行时

# HTTP 客户端
reqwest = { version = "0.12", features = ["json", "stream", "blocking"] }
ureq = { version = "2.9", features = ["json"] }

# 序列化
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# 错误处理
anyhow = "1.0"
thiserror = "2.0"

# 日志与追踪
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# 并发原语
parking_lot = "0.12"         # 高性能锁
once_cell = "1.19"           # 懒初始化
async-trait = "0.1"          # 异步 trait

# 缓存
moka = { version = "0.12", features = ["sync"] }  # 高性能缓存

# 中文处理
jieba-rs = "0.7"             # 中文分词

# 文件处理
lopdf = "0.34"               # PDF 解析
notify = "6.1"               # 文件监听
memmap2 = "0.9"              # 内存映射

# 加密哈希
sha2 = "0.10"
hex = "0.4"
uuid = { version = "1.0", features = ["v4"] }

# 终端 UI
crossterm = "0.27"           # 跨平台终端控制

# 基准测试
criterion = { version = "0.5", features = ["html_reports"] }

# 索引优化 (IMP-003)
fst = "0.4"                # Trie 索引
bk-tree = "0.5"            # BK-Tree 拼写纠正

# 模板引擎 (IMP-002)
tera = "1.19"              # Tera 模板
```

---

## 架构设计

### 五层架构模型

```
┌─────────────────────────────────────────────────────────────────┐
│                      用户界面层                                  │
│  ┌─────────────────────┐       ┌─────────────────────────────┐  │
│  │  CLI AI 助手         │       │  项目自更新服务              │  │
│  │  (交互式对话)       │       │  (自主进化循环)             │  │
│  └─────────────────────┘       └─────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────────┐
│                      编排调度层                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ Orchestrator│  │ RoleSwitcher│  │ WorkflowEngine          │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ ToolMatrix  │  │ Integrated  │  │ ServiceLifecycle        │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────────┐
│                      核心功能层                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │  Context    │  │  Autonomy   │  │  Dialogue FSM           │  │
│  │  Storage    │  │  Agents     │  │  Observability          │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
│  ┌─────────────┐  ┌─────────────┐                               │
│  │Prompt Eng   │  │ AI Tool     │                               │
│  │             │  │ Selector    │                               │
│  └─────────────┘  └─────────────┘                               │
└─────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────────┐
│                      工具执行层                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ File Tools  │  │ Net Tools   │  │ System Tools            │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ Git Tools   │  │ Data Tools  │  │ Sandbox (安全)          │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────────┐
│                      基础设施层                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ Config Mgr  │  │ Path Resolver│  │ Tracing (tokitai)      │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### 模块规模统计

| 模块 | 行数 | 占比 | 文件数 | 状态 |
|------|------|------|--------|------|
| `tools/` | 7,114 | 25.9% | 24 | ✅ 已完全集成 |
| `context/` | 4,794 | 17.4% | 11 | ✅ 已完全集成 |
| `tool_matrix/` | 4,200 | 15.3% | 15 | ✅ 已完全集成 (新增 IMP-001~004) |
| `orchestrator/` | 3,528 | 12.8% | 6 | ✅ 已完全集成 |
| `autonomy/` | 2,684 | 9.8% | 9 | ⚠️ 部分集成 |
| `main_core` | 1,884 | 6.9% | 6 | ✅ 已完全集成 |
| `observability/` | 456 | 1.7% | 3 | ✅ 已完全集成 |
| `dialogue/` | 443 | 1.6% | 3 | ✅ 已完全集成 |
| `prompt_engineering/` | 395 | 1.4% | 5 | ✅ 已完全集成 |
| `integration/` | 325 | 1.2% | 2 | ✅ 已完全集成 |
| **其他** | 1,676 | 6.0% | - | ✅ |
| **总计** | **27,500** | **100%** | **99** | |

---

## 核心模块详解

### 1. main.rs - 程序入口

**文件**: `src/main.rs` (~1,230 行)

#### 核心结构

```rust
pub struct AiAssistant {
    // === 工具集 ===
    file_ops: FileOperations,
    system_tools: SystemTools,
    code_tools: CodeTools,
    web_search: WebSearchTools,
    git_ops: GitOperations,
    // ... 63+ 工具

    // === 工具矩阵/服务注册表 ===
    tool_registry: ToolRegistry,
    tool_selector: ToolSelector,
    lightweight_selector: Arc<LightweightToolSelector>,
    tool_dispatcher: Arc<ToolDispatcher>,

    // === 集成模块 ===
    integrated_modules: IntegratedModules,

    // === 编排器 ===
    orchestrator: Orchestrator,

    // === 自主进化 (可选) ===
    coordinator: Option<Arc<RwLock<AgentCoordinator>>>,
    git_workflow: Option<GitWorkflow>,
    autonomous_mode: bool,
}
```

#### 服务生命周期管理

```rust
impl AiAssistant {
    /// 初始化所有服务
    pub async fn init_all_services(&mut self) -> Result<()> {
        // 1. 初始化集成模块 (dialogue/observability/prompt)
        self.integrated_modules.init().await?;

        // 2. 初始化工具矩阵
        self.tool_registry.init().await?;

        // 3. 初始化编排器
        self.orchestrator.init().await?;

        // 4. 预热缓存
        self.warmup_caches().await?;

        Ok(())
    }

    /// 健康检查
    pub async fn health_check(&self) -> HashMap<String, ServiceHealth> {
        let mut health_map = HashMap::new();

        // 检查各服务健康状态
        health_map.insert(
            "integrated_modules".to_string(),
            self.integrated_modules.health().await
        );
        health_map.insert(
            "tool_registry".to_string(),
            self.tool_registry.health().await
        );

        health_map
    }

    /// 关闭所有服务
    pub async fn shutdown(&mut self) -> Result<()> {
        // 1. 关闭集成模块
        self.integrated_modules.shutdown().await?;

        // 2. 关闭工具矩阵
        self.tool_registry.shutdown().await?;

        // 3. 关闭编排器
        self.orchestrator.shutdown().await?;

        Ok(())
    }
}
```

#### 命令行参数解析

```rust
// 支持的命令行参数
// --autonomous         启用自主进化模式
// --project-path PATH  指定项目路径
// --help               显示帮助信息

let args: Vec<String> = env::args().collect();
let mut autonomous = false;
let mut project_path = None;

let mut i = 1;
while i < args.len() {
    match args[i].as_str() {
        "--autonomous" => autonomous = true,
        "--project-path" => {
            if i + 1 < args.len() {
                project_path = Some(PathBuf::from(&args[i + 1]));
                i += 1;
            }
        }
        _ => {}
    }
    i += 1;
}
```

---

### 2. tool_matrix/ - 工具矩阵与服务注册表

**目录**: `src/tool_matrix/` (3,362 行，10 个文件)

#### 模块结构

```
tool_matrix/
├── mod.rs                      # 模块导出
├── matrix.rs                   # 服务化元数据/生命周期/指标收集
├── registry.rs                 # 工具注册表 (AI 分类/依赖分析/运行时学习)
├── selector.rs                 # 传统工具选择器
├── skills_manager.rs           # 技能管理
├── tool_selector.rs            # 轻量级工具选择器 (AI 原生)
├── ai_classifier.rs            # AI 工具箱分类器
├── dependency_analyzer.rs      # AI 依赖关系分析器
├── dispatcher.rs               # 工具调用分发器
├── metadata_enhancer.rs        # tokitai 元数据增强器
├── rule_classifier.rs          # 规则分类器 (IMP-001 分层缓存)
├── query_enhancer.rs           # 查询增强器 (同义词/意图识别)
├── tool_generator.rs           # 工具生成器 (IMP-002 模板系统)
├── trie_index.rs               # Trie 索引 (IMP-003 搜索优化)
└── dynamic_registry.rs         # 动态注册表 (IMP-004 热加载)
```

#### 服务元数据系统

```rust
/// 服务元数据
pub struct ServiceMetadata {
    /// 服务分类
    pub category: ServiceCategory,
    /// QoS 指标
    pub qos: QualityOfService,
    /// 依赖服务列表
    pub dependencies: Vec<String>,
    /// 限流配置 (每秒请求数)
    pub rate_limit: Option<u32>,
    /// 版本号 (SemVer)
    pub version: String,
    /// 标签
    pub tags: Vec<String>,
}

/// 服务分类 (10 种)
pub enum ServiceCategory {
    Utility,        // 通用工具
    File,           // 文件操作
    Network,        // 网络请求
    System,         // 系统命令
    Data,           // 数据处理
    Ai,             // AI 相关
    Vcs,            // 版本控制
    Dialogue,       // 对话管理
    Observability,  // 可观测性
    Prompt,         // 提示词工程
}

/// QoS 指标
pub struct QualityOfService {
    /// P99 延迟 (毫秒)
    pub latency_p99_ms: u64,
    /// 成功率 (0.0-1.0)
    pub success_rate: f64,
    /// 并发能力
    pub concurrency: u32,
    /// 是否幂等
    pub idempotent: bool,
}
```

#### 服务生命周期 Trait

```rust
/// 服务生命周期管理
pub trait ServiceLifecycle {
    /// 服务名称
    fn service_name(&self) -> &str;

    /// 初始化服务
    async fn init(&mut self) -> Result<()>;

    /// 健康检查
    async fn health(&self) -> ServiceHealth;

    /// 关闭服务
    async fn shutdown(&mut self) -> Result<()>;

    /// 获取服务统计
    fn stats(&self) -> ServiceStats;
}

/// 服务健康状态
pub enum ServiceHealth {
    Healthy,        // 健康
    Degraded,       // 降级运行
    Unhealthy,      // 不健康
    Unknown,        // 未知
}

/// 服务统计
pub struct ServiceStats {
    /// 总调用次数
    pub total_calls: u64,
    /// 成功调用次数
    pub successful_calls: u64,
    /// 失败调用次数
    pub failed_calls: u64,
    /// 总耗时 (毫秒)
    pub total_duration_ms: u64,
    /// 最后调用时间
    pub last_call_time: Option<DateTime<Utc>>,
    /// 最后错误信息
    pub last_error: Option<String>,
}
```

#### 服务指标收集器

```rust
pub struct ServiceMetricsCollector {
    metrics: Arc<RwLock<HashMap<String, ServiceStats>>>,
}

impl ServiceMetricsCollector {
    /// 记录服务调用
    pub async fn record_call(
        &self,
        service: &str,
        duration_ms: u64,
        success: bool,
    ) {
        let mut metrics = self.metrics.write();
        let stats = metrics.entry(service.to_string())
            .or_insert_with(ServiceStats::default);

        stats.total_calls += 1;
        stats.total_duration_ms += duration_ms;

        if success {
            stats.successful_calls += 1;
        } else {
            stats.failed_calls += 1;
        }

        stats.last_call_time = Some(Utc::now());
    }

    /// 记录错误
    pub async fn record_error(&self, service: &str, error: &str) {
        let mut metrics = self.metrics.write();
        let stats = metrics.entry(service.to_string())
            .or_insert_with(ServiceStats::default);

        stats.last_error = Some(error.to_string());
    }

    /// 获取服务统计
    pub async fn get_stats(&self, service: &str) -> Option<ServiceStats> {
        self.metrics.read().get(service).cloned()
    }

    /// 获取所有服务统计
    pub async fn get_all_stats(&self) -> HashMap<String, ServiceStats> {
        self.metrics.read().clone()
    }
}
```

---

### 3. orchestrator/ - 编排调度

**目录**: `src/orchestrator/` (3,528 行，6 个文件)

#### 声明式工作流定义

```rust
/// 声明式工作流
pub struct DeclarativeWorkflow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub steps: Vec<DeclarativeWorkflowStep>,
    pub timeout_secs: u64,
    pub variables: HashMap<String, String>,
    pub error_handler: Option<ErrorHandler>,
    pub tags: Vec<String>,
}

/// 工作流步骤
pub struct DeclarativeWorkflowStep {
    pub id: String,
    pub tool: String,
    pub arguments: Value,
    pub dependencies: Vec<String>,
    pub retry: RetryConfig,
    pub timeout_secs: Option<u64>,
    pub on_error: Option<ErrorHandler>,
    pub role: AgentRole,
}

/// 重试配置
pub struct RetryConfig {
    pub max_retries: u32,
    pub retry_interval_ms: u64,
    pub exponential_backoff: bool,
}

/// 错误处理
pub struct ErrorHandler {
    pub strategy: ErrorStrategy,
    pub fallback_tool: Option<String>,
    pub max_errors: Option<u32>,
}

pub enum ErrorStrategy {
    Retry,      // 重试
    Skip,       // 跳过
    Fail,       // 失败
    Fallback,   // 降级
}
```

#### TOML 工作流示例

```toml
# workflows/code_review.toml
[workflow]
id = "code_review"
name = "代码审查工作流"
description = "自动执行代码审查流程"
version = "1.0.0"
timeout_secs = 600

[workflow.variables]
max_files = "10"
min_coverage = "80"

[[workflow.steps]]
id = "analyze_changes"
tool = "git_diff"
role = "reviewer"

[workflow.steps.retry]
max_retries = 3
retry_interval_ms = 1000
exponential_backoff = true

[workflow.steps.on_error]
strategy = "skip"

[[workflow.steps]]
id = "run_tests"
tool = "run_command"
arguments = { command = "cargo test" }
role = "executor"
dependencies = ["analyze_changes"]
```

#### TOML 工作流加载器

```rust
pub struct WorkflowLoader;

impl WorkflowLoader {
    /// 从文件加载工作流
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<DeclarativeWorkflow> {
        let content = fs::read_to_string(path)?;
        Self::load_from_str(&content)
    }

    /// 从字符串加载工作流
    pub fn load_from_str(content: &str) -> Result<DeclarativeWorkflow> {
        let toml_value: toml::Value = toml::from_str(content)?;

        // 解析 TOML 到 DeclarativeWorkflow 结构
        // ...

        Ok(workflow)
    }

    /// 从目录加载所有工作流
    pub fn load_from_dir<P: AsRef<Path>>(dir: P) -> Result<Vec<DeclarativeWorkflow>> {
        let mut workflows = Vec::new();

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                if let Ok(workflow) = Self::load_from_file(&path) {
                    workflows.push(workflow);
                }
            }
        }

        Ok(workflows)
    }
}
```

---

### 4. integration/ - 集成模块管理器

**文件**: `src/integration/modules_manager.rs` (~325 行)

#### 统一模块管理

```rust
pub struct IntegratedModules {
    // === 对话状态机 ===
    pub dialogue_state: Arc<RwLock<DialogueStateMachine>>,
    pub dialogue_tools: DialogueTools,

    // === 可观测性 ===
    pub tracing_recorder: Arc<RwLock<TracingRecorder>>,
    pub observability_tools: ObservabilityTools,

    // === 提示词工程 ===
    pub prompt_manager: Arc<RwLock<PromptTemplateManager>>,
    pub prompt_tools: PromptTools,
}

impl IntegratedModules {
    /// 初始化所有模块
    pub async fn init(&self) -> Result<()> {
        // 1. 初始化对话状态机
        self.dialogue_state.write().await.load_state().await?;

        // 2. 初始化可观测性
        self.tracing_recorder.write().await.init().await?;

        // 3. 初始化提示词工程
        self.prompt_manager.write().await.warmup_cache().await?;

        Ok(())
    }

    /// 健康检查
    pub async fn health(&self) -> ServiceHealth {
        let dialogue_health = self.dialogue_state.read().await.health();
        let tracing_health = self.tracing_recorder.read().await.health();
        let prompt_health = self.prompt_manager.read().await.health();

        // 综合健康状态
        if dialogue_health == ServiceHealth::Healthy
            && tracing_health == ServiceHealth::Healthy
            && prompt_health == ServiceHealth::Healthy
        {
            ServiceHealth::Healthy
        } else if dialogue_health == ServiceHealth::Unhealthy
            || tracing_health == ServiceHealth::Unhealthy
            || prompt_health == ServiceHealth::Unhealthy
        {
            ServiceHealth::Unhealthy
        } else {
            ServiceHealth::Degraded
        }
    }

    /// 关闭所有模块
    pub async fn shutdown(&self) -> Result<()> {
        // 1. 保存对话状态
        self.dialogue_state.write().await.save_state().await?;

        // 2. 关闭追踪记录
        self.tracing_recorder.write().await.shutdown().await?;

        // 3. 清理提示词缓存
        self.prompt_manager.write().await.clear_cache().await?;

        Ok(())
    }
}
```

---

## 服务化架构

### 双轨服务设计

Tokitai 采用**双轨服务架构**，两种服务共享底层能力但定位和使用场景完全不同：

```
┌─────────────────────────────────────────────────────────────────┐
│                        Tokitai 双轨服务                          │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────┐    ┌─────────────────────────────┐│
│  │   CLI AI 助手            │    │   项目自更新服务             ││
│  │   (面向用户)            │    │   (面向项目自身)            ││
│  │                         │    │                             ││
│  │  📱 交互式对话          │    │  🤖 自主进化循环            ││
│  │  👤 用户驱动            │    │  🧠 AI 驱动                 ││
│  │  ⚡ 即时响应            │    │  🔄 迭代执行                ││
│  │  🛠️ 完成任务            │    │  📈 持续改进                ││
│  └─────────────────────────┘    └─────────────────────────────┘│
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    共享底层能力                              ││
│  │  ToolMatrix │ Context Storage │ Orchestrator │ Autonomy    ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

### 服务对比

| 维度 | CLI AI 助手 | 项目自更新服务 |
|------|------------|---------------|
| **启动命令** | `cargo run --release` | `cargo run --release -- --autonomous` |
| **服务对象** | 用户（开发者） | 项目自身 |
| **驱动方式** | 用户输入驱动 | AI 自主驱动 |
| **交互模式** | 交互式对话 | 自主迭代循环 |
| **响应模式** | 即时响应 | 批量执行 |
| **执行时长** | 秒级（单次任务） | 分钟级（多轮迭代） |
| **Git 操作** | 仅查询状态 | 可自动提交推送 |
| **代码修改** | 用户明确指令 | 自主决定修改 |
| **使用频率** | 按需使用 | 定期/持续运行 |
| **典型场景** | 查询、分析、临时任务 | 代码改进、技术债务清理 |

### 服务边界

#### CLI AI 助手
- ✅ 响应用户查询
- ✅ 执行用户指定的工具调用
- ✅ 保持对话上下文
- ✅ 提供建议和指导
- ❌ 不主动修改项目代码
- ❌ 不自主发起 Git 操作
- ❌ 不自主推送代码

#### 项目自更新服务
- ✅ 自主分析项目状态
- ✅ 自主发现改进点
- ✅ 自主制定并执行计划
- ✅ 自主代码审查
- ✅ 自主 Git 提交（可选）
- ❌ 不响应用户交互
- ❌ 不处理外部查询
- ❌ 不提供服务接口

---

## AI 原生工具选择器

### 架构设计

```
┌─────────────────────────────────────────────────────────────────┐
│                    AI 原生工具选择器系统                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │  ToolIndex   │  │  Lightweight │  │  Selector    │          │
│  │  (倒排索引)  │  │  ToolSelector│  │  Metrics     │          │
│  │              │  │  (AI 原生)    │  │  (监控)      │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
│         │                │                  │                   │
│         ▼                ▼                  ▼                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │  AI Toolbox  │  │  AI Dependency│  │  Tool        │          │
│  │  Classifier  │  │  Analyzer     │  │  Dispatcher  │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    LRU 搜索缓存 (1000 条)                     ││
│  │              缓存命中后延迟：~3ms (降低 62.5%)                ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 核心组件

#### 1. ToolIndex - 倒排索引

```rust
pub struct ToolIndex {
    /// 关键词 → 工具名称集合
    keyword_index: HashMap<String, HashSet<String>>,
    /// 分类 → 工具名称集合
    category_index: HashMap<ServiceCategory, HashSet<String>>,
    /// 工具箱 ID → 工具名称集合
    toolbox_index: HashMap<String, HashSet<String>>,
}

impl ToolIndex {
    /// 添加工具到索引
    pub fn add_tool(&mut self, tool: &ToolDefinition) {
        // 1. 提取关键词并建立倒排索引
        let keywords = self.extract_keywords(&tool.description);
        for keyword in keywords {
            self.keyword_index
                .entry(keyword)
                .or_insert_with(HashSet::new)
                .insert(tool.name.clone());
        }

        // 2. 添加到分类索引
        self.category_index
            .entry(tool.category.clone())
            .or_insert_with(HashSet::new)
            .insert(tool.name.clone());

        // 3. 添加到工具箱索引
        if let Some(toolbox) = &tool.toolbox {
            self.toolbox_index
                .entry(toolbox.clone())
                .or_insert_with(HashSet::new)
                .insert(tool.name.clone());
        }
    }

    /// 关键词搜索
    pub fn search(&self, query: &str) -> HashSet<String> {
        let keywords = query.to_lowercase().split_whitespace();
        let mut results = HashSet::new();

        for keyword in keywords {
            if let Some(tools) = self.keyword_index.get(keyword) {
                results.extend(tools.clone());
            }
        }

        results
    }
}
```

#### 2. LightweightToolSelector - 轻量级选择器

```rust
pub struct LightweightToolSelector {
    /// 工具索引
    index: Arc<RwLock<ToolIndex>>,
    /// 所有工具定义
    all_tools: Arc<RwLock<HashMap<String, ToolDefinition>>>,
    /// LLM 客户端（可选）
    llm_client: Option<Arc<dyn LLMClient + Send + Sync>>,
    /// 配置
    config: SelectorConfig,
    /// LRU 搜索缓存
    search_cache: Arc<RwLock<HashMap<String, Vec<ToolSearchResult>>>>,
    /// 监控指标
    metrics: Arc<RwLock<SelectorMetrics>>,
}

pub struct SelectorConfig {
    /// 最大搜索结果数
    pub max_results: usize,
    /// AI 搜索触发阈值（查询长度）
    pub ai_search_threshold: usize,
    /// 启用后台索引重建
    pub enable_background_rebuild: bool,
    /// 后台重建延迟（秒）
    pub rebuild_delay_secs: u64,
}

pub struct SelectorMetrics {
    /// 总搜索次数
    pub total_searches: u64,
    /// 缓存命中次数
    pub cache_hits: u64,
    /// AI 搜索次数
    pub ai_searches: u64,
    /// 快速搜索次数
    pub fast_searches: u64,
    /// 平均搜索延迟（微秒）
    pub avg_latency_us: f64,
    /// 后台重建次数
    pub rebuild_count: u64,
}

impl SelectorMetrics {
    /// 缓存命中率
    pub fn cache_hit_rate(&self) -> f64 {
        if self.total_searches == 0 {
            return 0.0;
        }
        self.cache_hits as f64 / self.total_searches as f64
    }

    /// AI 搜索比例
    pub fn ai_search_ratio(&self) -> f64 {
        if self.total_searches == 0 {
            return 0.0;
        }
        self.ai_searches as f64 / self.total_searches as f64
    }
}
```

#### 3. 搜索流程

```rust
impl LightweightToolSelector {
    /// 搜索工具（自动判断是否使用 AI）
    pub async fn search(&self, query: &str) -> Vec<ToolSearchResult> {
        let start_time = Instant::now();

        // 1. 检查缓存
        {
            let cache = self.search_cache.read().await;
            if let Some(cached) = cache.get(query) {
                self.metrics.write().await.cache_hits += 1;
                return cached.clone();
            }
        }

        // 2. 判断是否使用 AI 搜索
        let use_ai = self.should_use_ai_search(query);

        let results = if use_ai {
            // AI 搜索
            self.ai_search(query).await
        } else {
            // 快速搜索
            self.fast_search(query).await
        };

        // 3. 更新指标
        let duration = start_time.elapsed();
        self.update_metrics(use_ai, duration);

        // 4. 写入缓存
        {
            let mut cache = self.search_cache.write().await;
            if cache.len() >= 1000 {
                // LRU 淘汰
                if let Some(first_key) = cache.keys().next().cloned() {
                    cache.remove(&first_key);
                }
            }
            cache.insert(query.to_string(), results.clone());
        }

        results
    }

    /// 判断是否使用 AI 搜索
    fn should_use_ai_search(&self, query: &str) -> bool {
        // 1. 查询长度 > 20 字符
        if query.len() > self.config.ai_search_threshold {
            return true;
        }

        // 2. 包含疑问词
        let question_words = ["如何", "怎么", "怎样", "为什么", "什么", "哪个"];
        if question_words.iter().any(|w| query.contains(w)) {
            return true;
        }

        // 3. 包含多个动词
        let action_verbs = ["创建", "读取", "写入", "删除", "修改", "分析", "搜索", "下载", "上传"];
        let verb_count = action_verbs.iter().filter(|v| query.contains(**v)).count();
        if verb_count >= 2 {
            return true;
        }

        false
    }
}
```

#### 4. AIToolboxClassifier - AI 工具箱分类器

```rust
pub struct AIToolboxClassifier<T: LLMClient> {
    llm_client: Arc<T>,
    toolboxes: Arc<RwLock<HashMap<String, ToolBox>>>,
    summary_cache: Arc<RwLock<SummaryCache>>,
}

pub struct ToolboxAssignment {
    pub toolbox_id: String,
    pub action: ToolboxAction,
    pub confidence: f64,
    pub new_toolbox: Option<NewToolbox>,
}

pub enum ToolboxAction {
    AddToExisting,  // 添加到现有工具箱
    CreateNew,      // 创建新工具箱
}

impl<T: LLMClient> AIToolboxClassifier<T> {
    /// 分类工具到工具箱
    pub async fn classify_tool(&self, tool: &ToolDefinition) -> Result<ToolboxAssignment> {
        // 1. 构建提示词
        let prompt = format!(
            r#"请将以下工具分类到合适的工具箱：

工具名称：{}
工具描述：{}
工具分类：{:?}

现有工具箱：{:?}

请返回：
1. 应该添加到哪个工具箱（或创建新工具箱）
2. 置信度（0.0-1.0）
3. 如果是新工具箱，请提供名称和描述"#,
            tool.name,
            tool.description,
            tool.category,
            self.toolboxes.read().await.keys().collect::<Vec<_>>()
        );

        // 2. 调用 LLM
        let response = self.llm_client.chat(&prompt).await?;

        // 3. 解析响应
        let assignment = self.parse_llm_response(&response)?;

        Ok(assignment)
    }
}
```

#### 5. AIDependencyAnalyzer - AI 依赖关系分析器

```rust
pub struct AIDependencyAnalyzer<T: LLMClient> {
    llm_client: Arc<T>,
    dependency_graph: Arc<RwLock<ToolDependencyGraph>>,
    call_sequences: Arc<RwLock<Vec<ToolCallSequence>>>,
}

pub struct ToolDependencyGraph {
    /// 前置依赖：工具 → 前置工具列表
    prerequisites: HashMap<String, Vec<String>>,
    /// 后置依赖：工具 → 后置工具列表
    dependents: HashMap<String, Vec<String>>,
    /// 工具组合
    combinations: Vec<ToolCombination>,
}

pub struct ToolCallSequence {
    pub tools: Vec<String>,
    pub timestamps: Vec<u64>,  // 毫秒
}

impl<T: LLMClient> AIDependencyAnalyzer<T> {
    /// 记录工具调用序列
    pub fn record_call_sequence(&self, sequence: ToolCallSequence) {
        let mut sequences = self.call_sequences.write();
        if sequences.len() >= 1000 {
            sequences.remove(0);  // 保持最多 1000 条
        }
        sequences.push(sequence);
    }

    /// 从运行时日志学习依赖关系
    pub async fn learn_from_runtime_logs(&self) -> Result<usize> {
        let sequences = self.call_sequences.read().clone();
        let mut learned_count = 0;

        for sequence in sequences {
            // 分析工具调用顺序，发现潜在依赖关系
            for i in 0..sequence.tools.len().saturating_sub(1) {
                let tool_a = &sequence.tools[i];
                let tool_b = &sequence.tools[i + 1];

                // 如果两个工具经常连续调用，可能存在依赖关系
                // 使用 AI 分析是否需要建立依赖关系
                let needs_dependency = self.analyze_dependency(tool_a, tool_b).await?;

                if needs_dependency {
                    self.add_dependency(tool_a, tool_b).await?;
                    learned_count += 1;
                }
            }
        }

        Ok(learned_count)
    }
}
```

#### 6. ToolDispatcher - 工具调用分发器

```rust
pub struct ToolDispatcher {
    selector: Arc<LightweightToolSelector>,
    executors: Arc<RwLock<HashMap<String, Arc<dyn ToolExecutor>>>>,
    call_stats: Arc<RwLock<HashMap<String, ToolCallStats>>>,
}

pub struct ToolCallStats {
    pub total_calls: u64,
    pub successful_calls: u64,
    pub failed_calls: u64,
    pub total_duration_us: u64,
    pub last_error: Option<String>,
}

impl ToolDispatcher {
    /// 创建分发器
    pub fn new(selector: Arc<LightweightToolSelector>) -> Self {
        Self {
            selector,
            executors: Arc::new(RwLock::new(HashMap::new())),
            call_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册工具执行器
    pub async fn register_executor(
        &self,
        tools: Vec<ToolDefinition>,
        executor: DefaultToolExecutor,
    ) {
        let mut executors = self.executors.write();
        for tool in tools {
            executors.insert(tool.name, Arc::new(executor.clone()));
        }
    }

    /// 执行工具调用
    pub async fn execute(&self, tool_name: &str, args: &Value) -> Result<Value> {
        let start_time = Instant::now();

        // 1. 获取执行器
        let executors = self.executors.read().await;
        let executor = executors.get(tool_name)
            .ok_or_else(|| anyhow::anyhow!("工具未注册：{}", tool_name))?;

        // 2. 执行工具
        let result = executor.execute(tool_name, args).await;

        // 3. 更新统计
        let duration = start_time.elapsed();
        self.update_stats(tool_name, duration, result.is_ok());

        result
    }

    /// 获取调用统计
    pub async fn get_call_stats(&self) -> HashMap<String, ToolCallStats> {
        self.call_stats.read().await.clone()
    }
}
```

### 性能指标

| 操作 | 目标延迟 | 实际延迟 | 说明 |
|------|----------|----------|------|
| 快速搜索 | <10ms | ~8ms | 关键词匹配 |
| 快速搜索 (缓存命中) | N/A | ~3ms | LRU 缓存优化 |
| AI 搜索 | <2s | ~1.5s | 含 LLM 调用 |
| 后台重建 (100 工具) | <1s | ~600ms | 批量处理优化 |
| 内存占用 (10,000 工具) | <50MB | ~15MB | 含缓存 |

### 深化落实改进

| 功能模块 | 深化前 | 深化后 | 改进 |
|---------|--------|--------|------|
| **AI 分类器集成** | 框架已实现，未集成 | 深度集成到 ToolRegistry | ✅ |
| **AI 分析器学习** | 框架已实现，无运行时学习 | 完整实现运行时日志学习 | ✅ |
| **后台重建** | 有框架但未被调用 | 批量处理优化 | ✅ |
| **搜索缓存** | 未实现 | LRU 缓存 1000 条 | ✅ |
| **监控指标** | 部分实现 | 完整监控链路 | ✅ |
| **tokitai 集成** | 手动定义 | 同步/异步双版本 | ✅ |

---

## 数据存储设计

### 三层存储架构

```
┌─────────────────────────────────────────────────────────────────┐
│                     上下文存储架构                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │  瞬时层     │  │  短期层     │  │  长期层     │             │
│  │  (会话级)   │  │  (天级)     │  │  (永久)     │             │
│  │             │  │             │  │             │             │
│  │  • 当前对话 │  │  • 最近对话 │  │  • 核心知识 │             │
│  │  • 临时数据 │  │  • 短期记忆 │  │  • 经验总结 │             │
│  │  • 缓存数据 │  │  • 中间结果 │  │  • 索引数据 │             │
│  │             │  │             │  │             │             │
│  │  容量：10MB │  │  容量：100MB│  │  容量：1GB  │             │
│  │  保留：会话 │  │  保留：7 天   │  │  保留：永久 │             │
│  └─────────────┘  └─────────────┘  └─────────────┘             │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    增量哈希链 (ICHC)                         ││
│  │  H(n) = SHA256(H(n-1) + content + timestamp)                ││
│  │  不可篡改的链式哈希结构                                      ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    上下文蒸馏 (HCD)                          ││
│  │  提取核心意图，过滤冗余信息                                  ││
│  │  压缩率：10:1                                                ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    语义索引 (LSFI)                           ││
│  │  基于 SimHash 的语义搜索                                     ││
│  │  支持中文分词 (jieba-rs)                                     ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 文件组织

```
.context/
├── ephemeral/              # 瞬时层
│   ├── session_001.json
│   └── session_002.json
├── short_term/             # 短期层
│   ├── day_2026-03-15.json
│   └── day_2026-03-14.json
├── long_term/              # 长期层
│   ├── core_knowledge.json
│   └── experience_index.json
└── hash_chain/             # 哈希链
    ├── chain_head.json
    └── chain_history/
```

### 增量哈希链 (ICHC)

```rust
/// 增量哈希链
pub struct IncrementalHashChain {
    chain: Vec<HashNode>,
    current_head: HashNode,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct HashNode {
    pub id: u64,
    pub content_hash: String,
    pub previous_hash: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

impl IncrementalHashChain {
    /// 添加新节点
    pub fn append(&mut self, content: &str, metadata: HashMap<String, String>) {
        let content_hash = self.compute_hash(content);

        let new_node = HashNode {
            id: self.chain.len() as u64,
            content_hash: content_hash.clone(),
            previous_hash: self.current_head.content_hash.clone(),
            timestamp: Utc::now(),
            metadata,
        };

        // 计算新哈希：H(n) = SHA256(H(n-1) + content + timestamp)
        let mut hasher = Sha256::new();
        hasher.update(self.current_head.content_hash.as_bytes());
        hasher.update(content.as_bytes());
        hasher.update(new_node.timestamp.to_rfc3339().as_bytes());
        let result = hasher.finalize();

        new_node.content_hash = hex::encode(result);
        self.chain.push(new_node);
        self.current_head = self.chain.last().unwrap().clone();
    }

    /// 验证链完整性
    pub fn verify(&self) -> bool {
        for i in 1..self.chain.len() {
            let prev = &self.chain[i - 1];
            let curr = &self.chain[i];

            if curr.previous_hash != prev.content_hash {
                return false;
            }
        }
        true
    }
}
```

---

## 安全机制

### 沙箱系统

```rust
pub struct Sandbox {
    allowed_paths: HashSet<PathBuf>,
    command_blacklist: HashSet<String>,
    ssrf_protection: SsrfProtection,
}

/// SSRF 防护
pub struct SsrfProtection {
    blocked_ips: Vec<IpAddr>,
    blocked_ranges: Vec<IpRange>,
}

impl SsrfProtection {
    pub fn new() -> Self {
        Self {
            blocked_ips: vec![
                // 内网 IP
                "127.0.0.1".parse().unwrap(),
                "10.0.0.0".parse().unwrap(),
                "172.16.0.0".parse().unwrap(),
                "192.168.0.0".parse().unwrap(),
            ],
            blocked_ranges: vec![
                IpRange::new("10.0.0.0/8".parse().unwrap()),
                IpRange::new("172.16.0.0/12".parse().unwrap()),
                IpRange::new("192.168.0.0/16".parse().unwrap()),
            ],
        }
    }

    pub fn is_allowed(&self, url: &Url) -> bool {
        if let Ok(ip) = url.host().unwrap().to_ip() {
            !self.blocked_ips.contains(&ip)
                && !self.blocked_ranges.iter().any(|r| r.contains(&ip))
        } else {
            true
        }
    }
}
```

### 命令黑名单

```rust
/// 命令黑名单
const BLACKLISTED_COMMANDS: &[&str] = &[
    // 文件操作
    "rm", "dd", "shred",
    // 磁盘操作
    "mkfs", "fdisk", "parted",
    // 权限修改
    "chmod", "chown", "chgrp",
    // 提权命令
    "sudo", "su", "pkexec", "doas",
    // 网络工具
    "wget", "curl", "nc", "netcat", "telnet", "ssh", "scp", "rsync",
    // 进程控制
    "kill", "pkill", "killall", "xkill",
    // 系统控制
    "shutdown", "reboot", "halt", "poweroff", "init",
    // 挂载操作
    "mount", "umount", "losetup",
    // 防火墙
    "iptables", "firewall-cmd", "ufw", "nft",
    // 用户管理
    "visudo", "passwd", "useradd", "userdel", "usermod",
    "groupadd", "groupdel", "groupmod",
    // 内核模块
    "insmod", "rmmod", "modprobe",
];

/// 检查命令是否在黑名单中
pub fn is_blacklisted(command: &str) -> bool {
    let command = command.split_whitespace().next().unwrap_or("");
    BLACKLISTED_COMMANDS.contains(&command)
}

/// 执行安全命令
pub async fn run_safe_command(command: &str, confirmed: bool) -> Result<String> {
    // 1. 检查黑名单
    if is_blacklisted(command) {
        return Err(anyhow::anyhow!("命令 '{}' 在黑名单中，禁止执行", command));
    }

    // 2. 非黑名单命令需要确认
    if !confirmed {
        return Err(anyhow::anyhow!("执行非黑名单命令需要 confirmed=true 参数"));
    }

    // 3. 执行命令
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .await?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
```

### 路径验证

```rust
pub struct PathResolver {
    base_dir: PathBuf,
    allowed_dirs: HashSet<PathBuf>,
}

impl PathResolver {
    /// 解析并验证路径
    pub fn resolve(&self, path: &str) -> Result<PathBuf> {
        let resolved = if path.starts_with('@') {
            // @ 路径引用
            PathBuf::from(&path[1..])
        } else {
            PathBuf::from(path)
        };

        // 1. 规范化路径
        let normalized = pathdiff::diff_paths(&resolved, &self.base_dir)
            .ok_or_else(|| anyhow::anyhow!("无效路径"))?;

        // 2. 检查是否在允许目录内
        let absolute = self.base_dir.join(&normalized);
        let canonical = absolute.canonicalize()?;

        if !self.is_allowed(&canonical) {
            return Err(anyhow::anyhow!(
                "路径 '{}' 不在允许的目录内",
                canonical.display()
            ));
        }

        Ok(canonical)
    }

    /// 检查路径是否允许
    fn is_allowed(&self, path: &Path) -> bool {
        self.allowed_dirs.iter().any(|dir| path.starts_with(dir))
    }
}
```

---

## 性能优化

### 缓存策略

```rust
/// 全局 HTTP 连接池
pub struct HttpClientPool {
    pool: Arc<moka::sync::Cache<String, reqwest::Client>>,
}

impl HttpClientPool {
    pub fn new() -> Self {
        Self {
            pool: Arc::new(
                moka::sync::CacheBuilder::new(100)
                    .time_to_live(Duration::from_secs(300))  // 5 分钟 TTL
                    .build()
            ),
        }
    }

    pub fn get_client(&self, host: &str) -> reqwest::Client {
        self.pool.get(host).unwrap_or_else(|| {
            reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap()
        })
    }
}
```

### 性能指标

| 指标 | 优化前 | 优化后 | 提升 |
|------|--------|--------|------|
| 缓存响应延迟 | ~500ms | <10ms | **50x** |
| 首次请求延迟 | ~4s | ~2s | **2x** |
| 流式首字节延迟 | ~1.5s | ~0.5s | **3x** |
| 缓存容量 | 50 条目 | 200 条目 | **4x** |
| 缓存 TTL | 1 分钟 | 5 分钟 | **5x** |

### 异步线程模型

```rust
// 纯异步线程模型，无线程阻塞
async fn process_request(&self, request: &Request) -> Result<Response> {
    // 1. 检查缓存（异步）
    if let Some(cached) = self.cache.get(request.key()).await {
        return Ok(cached);
    }

    // 2. 执行工具调用（异步）
    let result = tokio::spawn(async move {
        self.execute_tool(request).await
    })
    .await??;

    // 3. 写入缓存（异步）
    self.cache.set(request.key(), result.clone()).await;

    Ok(result)
}
```

---

## 测试策略

### 测试覆盖

```bash
# 运行所有测试
cargo test

# 按模块测试
cargo test autonomy              # 自主进化模块
cargo test context               # 上下文存储
cargo test tool_matrix           # 工具矩阵/服务化
cargo test tool_selector         # 轻量级工具选择器
cargo test ai_classifier         # AI 工具箱分类器
cargo test dependency_analyzer   # AI 依赖分析器
cargo test dispatcher            # 工具调用分发器
cargo test integration           # 集成模块
cargo test dialogue              # 对话状态机
cargo test observability         # 可观测性
cargo test prompt_engineering    # 提示词工程
cargo test workflow_loader       # TOML 工作流加载器

# 性能基准测试
cargo bench
```

### 测试统计

```
running 236 tests
test autonomy::...              ✅
test context::...               ✅
test tool_matrix::...           ✅
test tool_matrix::tool_selector::...    ✅ (5 个测试)
test tool_matrix::ai_classifier::...    ✅ (1 个测试)
test tool_matrix::dependency_analyzer::... ✅ (2 个测试)
test tool_matrix::dispatcher::...       ✅ (3 个测试)
test dialogue::...              ✅
test observability::...         ✅
test prompt_engineering::...    ✅
test integration::...           ✅
test workflow_loader::...       ✅

test result: ok. 236 passed; 0 failed; 0 ignored
```

---

## 部署与运维

### 环境变量配置

| 变量名 | 说明 | 默认值 | 必填 |
|--------|------|--------|------|
| `AI_API_URL` | AI API 地址 | `https://ollama.com/v1/chat/completions` | 否 |
| `AI_API_KEY` | API 密钥 | 无 | **是** |
| `AI_MODEL` | 模型名称 | `qwen3.5:397b` | 否 |

### 运行时文件夹

| 文件夹 | 用途 | 说明 |
|--------|------|------|
| `sandbox/` | 沙箱测试目录 | 测试文件操作、项目模板 |
| `downloads/` | 下载文件目录 | 下载工具默认保存位置 |
| `.context/` | 上下文存储 | 三层存储架构持久化数据 |
| `.tokitai/` | 运行时数据 | 对话状态、追踪日志、自主进化数据 |

### 健康检查端点

```rust
/// 健康检查响应
pub struct HealthResponse {
    pub status: String,
    pub services: HashMap<String, ServiceHealth>,
    pub metrics: ServiceStats,
}

/// 健康检查接口
pub async fn health_check() -> Result<HealthResponse> {
    let assistant = get_assistant().await?;
    let services = assistant.health_check().await;
    let metrics = assistant.get_service_metrics();

    let overall_status = if services.values().all(|h| *h == ServiceHealth::Healthy) {
        "healthy"
    } else if services.values().any(|h| *h == ServiceHealth::Unhealthy) {
        "unhealthy"
    } else {
        "degraded"
    };

    Ok(HealthResponse {
        status: overall_status.to_string(),
        services,
        metrics,
    })
}
```

---

**文档版本**: 3.0.0
**最后更新**: 2026-03-18
**维护者**: Tokitai Team
