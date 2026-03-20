# try-tokitai 项目结构指南

> 本文档帮助开发者快速了解项目架构、模块职责和代码组织
> **最新版本**: 3.1 (HybridGapDetector 实现完成 + Prompt Engineering 自进化系统)
> **最后更新**: 2026-03-20
> **测试状态**: 470/470 通过 ✅
> **HybridGapDetector**: ✅ 完成（769 行，成本降低 95%）

---

## 📊 项目概览

| 指标 | 数值 |
|------|------|
| **代码行数** | ~52,964 行 Rust |
| **源代码文件** | 131 个 |
| **核心模块** | 15 个（+5 个 Prompt Engineering 模块） |
| **工具箱** | 12 个 |
| **工具函数** | 63+ 个 |
| **测试状态** | 470/470 通过 ✅ |
| **HybridGapDetector** | ✅ 已实现（769 行，成本降低 95%） |

---

## 🎯 服务双轨架构

> 💡 **重要**: Tokitai 采用**双轨服务架构**，两种服务共享底层能力但定位和使用场景完全不同
> 详细文档请查看：[SERVICES.md](SERVICES.md)

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
| **Git 操作** | 仅查询状态 | 可自动提交推送 |
| **代码修改** | 用户明确指令 | 自主决定修改 |
| **典型场景** | 查询、分析、临时任务 | 代码改进、技术债务清理 |

---

## 🏗️ 架构分层

```
┌─────────────────────────────────────────────────────────────────┐
│                      用户界面层                                  │
│  ┌─────────────────────┐       ┌─────────────────────────────┐  │
│  │  CLI AI 助手         │       │  项目自更新服务              │  │
│  │  (交互式对话)       │       │  (自主进化循环)             │  │
│  │                     │       │                             │  │
│  │  • 用户输入驱动     │       │  • AI 自主驱动               │  │
│  │  • 即时响应         │       │  • 迭代执行                 │  │
│  │  • 多轮对话         │       │  • Planner-Executor-Reviewer│  │
│  │  • 工具调用         │       │  • Git 工作流                │  │
│  └─────────────────────┘       └─────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────────┐
│                      编排调度层                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ Orchestrator│  │ RoleSwitcher│  │ WorkflowEngine          │  │
│  │ (角色切换)  │  │ (planner/   │  │ (声明式工作流)          │  │
│  │             │  │  executor/  │  │                         │  │
│  │             │  │  reviewer)  │  │                         │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ ToolMatrix  │  │ Integrated  │  │ ServiceLifecycle        │  │
│  │ (服务注册表)│  │ Modules     │  │ (init/health/shutdown)  │  │
│  │             │  │             │  │                         │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
│  ┌─────────────┐  ┌─────────────┐                               │
│  │ToolDispatch │  │ AI Tool     │                               │
│  │ (调用分发)  │  │ Selector    │                               │
│  └─────────────┘  └─────────────┘                               │
└─────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────────┐
│                      核心功能层                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │  Context    │  │  Autonomy   │  │  Dialogue FSM           │  │
│  │  Storage    │  │  Agents     │  │  ✅ 已集成              │  │
│  │ (三层存储)  │  │ (多 Agent)  │  │                         │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │Observability│  │Prompt Eng   │  │ 🆕 HybridGapDetector   │  │
│  │ ✅ 已集成    │  │ ✅ 已集成    │  │  ⭐ 成本↓95%           │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ AI Tool     │  │ Runtime     │  │ ServiceLifecycle        │  │
│  │ Selector ✅ │  │ Learning ✅ │  │  ✅ 已实现              │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────────┐
│                      工具执行层                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ File Tools  │  │ Net Tools   │  │ System Tools            │  │
│  │ (服务化)    │  │ (服务化)    │  │ (服务化)                │  │
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
│  │ Command Resolver│ │ Security   │  │                         │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
│  ┌─────────────┐  ┌─────────────┐                               │
│  │ TOML 加载器  │  │ 服务指标收集 │                               │
│  └─────────────┘  └─────────────┘                               │
└─────────────────────────────────────────────────────────────────┘
```

---

## 📁 目录结构

```
try-tokitai/
├── Cargo.toml                    # 项目配置和依赖
├── config.toml                   # 应用配置
├── .env.example                  # 环境变量模板
├── README.md                     # 项目说明
├── demo.sh                       # 一键演示脚本
│
├── docs/                         # 用户文档
│   ├── QUICKSTART.md            # 快速启动
│   ├── USER_GUIDE.md            # 用户指南
│   ├── DEMO.md                  # 演示指南
│   ├── CHANGELOG.md             # 更新日志
│   └── archive/                 # 技术报告归档
│       ├── MODULE_INTEGRATION_REPORT.md   - 集成报告
│       ├── MODULE_IMPROVEMENT_REPORT.md   - 改进报告
│       ├── SERVICE_ARCHITECTURE_IMPLEMENTATION.md - 服务化架构（新增）
│       ├── LIGHTWEIGHT_TOOL_SELECTION_DESIGN.md - 工具选择器设计
│       ├── LIGHTWEIGHT_TOOL_SELECTION_DEEPENING.md - 深化落实报告
│       └── LIGHTWEIGHT_TOOL_SELECTION_FINAL_SUMMARY.md - 总结（新增）
│
├── workflows/                    # TOML 工作流定义（新增）
│   ├── research_and_write.toml  - 研究并撰写报告工作流
│   └── code_review.toml         - 代码审查工作流
│
├── src/                          # 源代码
│   ├── main.rs                  # 程序入口，AiAssistant 整合
│   ├── config.rs                # 配置管理
│   ├── command_resolver.rs      # 命令解析器
│   ├── path_resolver.rs         # 路径解析器
│   ├── sandbox.rs               # 沙箱系统
│   │
│   ├── tools/                   # 工具集合 (16,802 行)
│   │   ├── io/                  # 文件 IO 工具
│   │   ├── network/             # 网络工具（服务化）
│   │   ├── system/              # 系统工具
│   │   ├── data/                # 数据处理工具
│   │   ├── vcs/                 # 版本控制工具
│   │   └── tensor/              # 张量计算工具（可选）
│   │
│   ├── context/                 # 上下文存储 (7,398 行)
│   ├── autonomy/                # 自主进化模块 (7,072 行)
│   ├── orchestrator/            # 编排调度 (4,419 行)
│   │   ├── mod.rs               # 模块导出
│   │   ├── orchestrator.rs      # 编排器核心
│   │   ├── role_switcher.rs     # 角色切换
│   │   ├── workflow.rs          # 声明式工作流定义和执行引擎
│   │   └── workflow_loader.rs   # TOML 工作流加载器
│   │
│   ├── tool_matrix/             # 工具矩阵/服务注册表 (8,271 行)
│   │   ├── mod.rs               # 模块导出
│   │   ├── matrix.rs            # 服务化元数据/生命周期/指标收集
│   │   ├── registry.rs          # 工具注册表（AI 分类/依赖分析/运行时学习）
│   │   ├── selector.rs          # 工具选择器
│   │   ├── skills_manager.rs    # 技能管理
│   │   ├── tool_selector.rs     # 轻量级工具选择器（AI 原生）
│   │   ├── ai_classifier.rs     # AI 工具箱分类器
│   │   ├── dependency_analyzer.rs # AI 依赖关系分析器
│   │   ├── dispatcher.rs        # 工具调用分发器
│   │   ├── metadata_enhancer.rs # tokitai 元数据增强器
│   │   ├── rule_classifier.rs   # 规则分类器（分层缓存 L3）
│   │   ├── query_enhancer.rs    # 查询增强器（同义词/意图识别）
│   │   ├── tool_generator.rs    # 工具生成器（模板系统）
│   │   ├── trie_index.rs        # Trie 树索引和 BK-Tree 拼写纠正
│   │   └── dynamic_registry.rs  # 动态工具注册表（热加载）
│   │
│   ├── integration/             # 集成模块管理器 (331 行)
│   │   ├── mod.rs               # 模块导出
│   │   └── modules_manager.rs   # 统一生命周期管理
│   │
│   ├── provider_config/         # AI 提供商配置
│   │   ├── mod.rs
│   │   └── provider_queue.rs    # 提供商队列管理
│   │
│   ├── prompt_engineering/      # 提示词工程 (677 行)
│   │   ├── mod.rs
│   │   ├── manager.rs           # 模板管理器
│   │   ├── renderer.rs          # 渲染引擎
│   │   ├── template.rs          # 模板结构
│   │   └── prompt_tools.rs      # tokitai ToolProvider
│   │
│   ├── dialogue/                # 对话状态机 (751 行，已集成)
│   │   ├── mod.rs
│   │   ├── state_machine.rs     # 状态机核心
│   │   └── dialogue_tools.rs    # tokitai ToolProvider
│   │
│   └── observability/           # 可观测性 (901 行，已集成)
│       ├── mod.rs
│       ├── tracing.rs           # 全链路追踪
│       ├── observability_tools.rs # tokitai ToolProvider
│       ├── metrics_dashboard.rs  # 指标仪表板
│       ├── replay.rs             # 追踪回放
│       └── tool_timeline.rs      # 工具时间线
│
├── examples/                     # 示例代码
├── benches/                      # 性能基准测试
├── sandbox/                      # 沙箱测试项目
│
├── .context/                     # 上下文存储 (运行时)
├── .tokitai/                     # Tokitai 运行时数据
│   ├── dialogue/                # 对话状态存储
│   ├── traces/                  # 追踪日志
│   └── autonomy/                # 自主进化数据
│
└── structure_ensure/            # 项目结构文档
```

---

## 🔧 核心模块详解

### 1. main.rs - 程序入口

**文件**: `src/main.rs` (~1,230 行)

**职责**:
- `AiAssistant` 结构体 - 整合所有工具和能力
- 命令行参数解析 (`--autonomous`, `--project-path`)
- 交互式 CLI 循环
- 工具定义生成
- **服务生命周期管理** (`init_all_services`, `health_check`, `shutdown`)
- **集成模块管理** (`IntegratedModules`)

**关键结构**:
```rust
pub struct AiAssistant {
    // 工具集
    file_ops: FileOperations,
    system_tools: SystemTools,
    git_ops: GitOperations,
    // ... 其他工具

    // 工具矩阵/服务注册表
    tool_registry: ToolRegistry,
    tool_selector: ToolSelector,
    skills_manager: SkillsManager,

    // 编排器
    orchestrator: Orchestrator,

    // 自主进化
    coordinator: Option<AgentCoordinator>,

    // 集成模块 (统一管理 dialogue/observability/prompt_engineering)
    integrated_modules: IntegratedModules,
}

impl AiAssistant {
    // 服务生命周期管理
    async fn init_all_services(&mut self) -> Result<()>;
    async fn health_check(&self) -> HashMap<String, ServiceHealth>;
    async fn shutdown(&mut self) -> Result<()>;
    fn get_service_metrics(&self) -> HashMap<String, ServiceStats>;
}
```

---

### 2. tool_matrix/ - 工具矩阵/服务注册表 (4,200 行)

**文件**:
- `src/tool_matrix/matrix.rs` - 服务化元数据/生命周期/指标收集
- `src/tool_matrix/registry.rs` - 工具注册表（AI 分类/依赖分析/运行时学习）
- `src/tool_matrix/selector.rs` - 工具选择器
- `src/tool_matrix/skills_manager.rs` - 技能管理
- `src/tool_matrix/tool_selector.rs` - 轻量级工具选择器（AI 原生）
- `src/tool_matrix/ai_classifier.rs` - AI 工具箱分类器
- `src/tool_matrix/dependency_analyzer.rs` - AI 依赖关系分析器
- `src/tool_matrix/dispatcher.rs` - 工具调用分发器
- `src/tool_matrix/metadata_enhancer.rs` - tokitai 元数据增强器
- `src/tool_matrix/rule_classifier.rs` - 规则分类器（分层缓存 L3）
- `src/tool_matrix/query_enhancer.rs` - 查询增强器（同义词/意图识别）
- `src/tool_matrix/tool_generator.rs` - 工具生成器（模板系统）
- `src/tool_matrix/trie_index.rs` - Trie 树索引和 BK-Tree 拼写纠正（IMP-003）
- `src/tool_matrix/dynamic_registry.rs` - 动态工具注册表（IMP-004 热加载）

**服务化功能**:

#### 服务元数据 (ServiceMetadata)
```rust
pub struct ServiceMetadata {
    pub category: ServiceCategory,
    pub qos: QualityOfService,
    pub dependencies: Vec<String>,
    pub rate_limit: Option<u32>,
    pub version: String,
    pub tags: Vec<String>,
}
```

#### 服务分类 (ServiceCategory)
```rust
pub enum ServiceCategory {
    Utility,      // 通用工具
    File,         // 文件操作
    Network,      // 网络请求
    System,       // 系统命令
    Data,         // 数据处理
    Ai,           // AI 相关
    Vcs,          // 版本控制
    Dialogue,     // 对话管理
    Observability, // 可观测性
    Prompt,       // 提示词工程
}
```

#### QoS 指标 (QualityOfService)
```rust
pub struct QualityOfService {
    pub latency_p99_ms: u64,
    pub success_rate: f64,
    pub concurrency: u32,
    pub idempotent: bool,
}
```

#### 服务生命周期 (ServiceLifecycle)
```rust
pub trait ServiceLifecycle {
    fn service_name(&self) -> &str;
    async fn init(&mut self) -> Result<()>;
    async fn health(&self) -> ServiceHealth;
    async fn shutdown(&mut self) -> Result<()>;
    fn stats(&self) -> ServiceStats;
}

pub enum ServiceHealth {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}
```

#### 服务统计 (ServiceStats)
```rust
pub struct ServiceStats {
    pub total_calls: u64,
    pub successful_calls: u64,
    pub failed_calls: u64,
    pub total_duration_ms: u64,
    pub last_call_time: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}
```

#### 服务指标收集器 (ServiceMetricsCollector)
```rust
pub struct ServiceMetricsCollector {
    metrics: Arc<RwLock<HashMap<String, ServiceStats>>>,
}

impl ServiceMetricsCollector {
    pub async fn record_call(&self, service: &str, duration_ms: u64, success: bool);
    pub async fn record_error(&self, service: &str, error: &str);
    pub async fn get_stats(&self, service: &str) -> Option<ServiceStats>;
    pub async fn get_all_stats(&self) -> HashMap<String, ServiceStats>;
}
```

**AI 原生工具选择器功能**（新增）:

#### 工具索引 (ToolIndex)
```rust
pub struct ToolIndex {
    /// 倒排索引：关键词 → 工具名称集合
    keyword_index: HashMap<String, HashSet<String>>,
    /// 分类索引：ServiceCategory → 工具名称集合
    category_index: HashMap<ServiceCategory, HashSet<String>>,
    /// 工具箱索引：Toolbox ID → 工具名称集合
    toolbox_index: HashMap<String, HashSet<String>>,
}
```

#### 轻量级工具选择器 (LightweightToolSelector)
```rust
pub struct LightweightToolSelector {
    /// 工具索引
    index: Arc<RwLock<ToolIndex>>,
    /// 所有工具
    all_tools: Arc<RwLock<HashMap<String, ToolDefinition>>>,
    /// LLM 客户端（可选，用于 AI 搜索）
    llm_client: Option<Arc<dyn LLMClient + Send + Sync>>,
    /// 配置
    config: SelectorConfig,
    /// 搜索缓存（LRU 缓存，优化重复查询）
    search_cache: Arc<RwLock<HashMap<String, Vec<ToolSearchResult>>>>,
    /// 监控指标
    metrics: Arc<RwLock<SelectorMetrics>>,
}

pub struct SelectorMetrics {
    pub total_searches: u64,      // 总搜索次数
    pub cache_hits: u64,          // 缓存命中次数
    pub ai_searches: u64,         // AI 搜索次数
    pub fast_searches: u64,       // 快速搜索次数
    pub avg_latency_us: f64,      // 平均搜索延迟（微秒）
    pub rebuild_count: u64,       // 后台重建次数
}
```

#### AI 工具箱分类器 (AIToolboxClassifier)
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
    AddToExisting,   // 添加到现有工具箱
    CreateNew,       // 创建新工具箱
}
```

#### AI 依赖关系分析器 (AIDependencyAnalyzer)
```rust
pub struct AIDependencyAnalyzer<T: LLMClient> {
    llm_client: Arc<T>,
    dependency_graph: Arc<RwLock<ToolDependencyGraph>>,
    call_sequences: Arc<RwLock<Vec<ToolCallSequence>>>,
}

pub struct ToolDependencyGraph {
    prerequisites: HashMap<String, Vec<String>>,      // 前置依赖
    dependents: HashMap<String, Vec<String>>,         // 后置依赖
    combinations: Vec<ToolCombination>,               // 工具组合
}

pub struct ToolCallSequence {
    pub tools: Vec<String>,
    pub timestamps: Vec<u64>,  // 毫秒
}
```

#### 工具调用分发器 (ToolDispatcher)
```rust
pub struct ToolDispatcher {
    selector: Arc<LightweightToolSelector>,
    executors: Arc<RwLock<HashMap<String, Arc<dyn ToolExecutor>>>>,
    call_stats: Arc<RwLock<HashMap<String, ToolCallStats>>>,
}

impl ToolDispatcher {
    pub async fn execute(&self, tool_name: &str, args: &Value) -> Result<Value>;
    pub async fn search_tools(&self, query: &str) -> Vec<ToolSearchResult>;
    pub async fn get_call_stats(&self) -> HashMap<String, ToolCallStats>;
}
```

**性能指标**:
| 操作 | 目标延迟 | 实际延迟 | 说明 |
|------|----------|----------|------|
| 快速搜索 | <10ms | ~8ms | 关键词匹配 |
| 快速搜索 (缓存命中) | N/A | ~3ms | LRU 缓存优化 |
| AI 搜索 | <2s | ~1.5s | 含 LLM 调用 |
| 后台重建 (100 工具) | <1s | ~600ms | 批量处理 |
| 内存占用 (10,000 工具) | <50MB | ~15MB | 含缓存 |

**AI 搜索触发条件**:
1. 查询长度 > 20 字符
2. 包含疑问词（如何、怎么、怎样、为什么、什么、哪个）
3. 包含多个动词（创建、读取、写入、删除、修改、分析、搜索、下载、上传）

**运行时日志学习**:
```rust
// ToolRegistry 方法
pub fn record_call_sequence(&self, sequence: ToolCallSequence);
pub async fn learn_from_runtime_logs(&self) -> Result<usize>;
```

---

### 3. orchestrator/ - 编排调度 (3,528 行)

**文件**:
- `src/orchestrator/orchestrator.rs`
- `src/orchestrator/role_switcher.rs`
- `src/orchestrator/workflow.rs` (新增声明式工作流)
- `src/orchestrator/workflow_loader.rs` (新增 TOML 加载器)

#### 声明式工作流定义
```rust
pub struct DeclarativeWorkflow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<DeclarativeWorkflowStep>,
    pub timeout_secs: u64,
    pub variables: HashMap<String, String>,
    pub error_handler: Option<ErrorHandler>,
    pub tags: Vec<String>,
}

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
```

#### 重试配置
```rust
pub struct RetryConfig {
    pub max_retries: u32,
    pub retry_interval_ms: u64,
    pub exponential_backoff: bool,
}
```

#### 错误处理
```rust
pub struct ErrorHandler {
    pub strategy: ErrorStrategy,
    pub fallback_tool: Option<String>,
    pub max_errors: Option<u32>,
}

pub enum ErrorStrategy {
    Retry,
    Skip,
    Fail,
    Fallback,
}
```

#### TOML 工作流加载器
```rust
pub struct WorkflowLoader;

impl WorkflowLoader {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<DeclarativeWorkflow>;
    pub fn load_from_str(content: &str) -> Result<DeclarativeWorkflow>;
    pub fn load_from_dir<P: AsRef<Path>>(dir: P) -> Result<Vec<DeclarativeWorkflow>>;
}
```

---

### 4. integration/ - 集成模块管理器

**文件**: `src/integration/modules_manager.rs` (~325 行)

**职责**:
- 统一管理 dialogue、observability、prompt_engineering 三个模块
- 共享状态管理（使用 `Arc<RwLock>`）
- 统一初始化和关闭流程
- 优雅的错误处理和降级

**核心结构**:
```rust
pub struct IntegratedModules {
    pub dialogue_state: Arc<RwLock<DialogueStateMachine>>,
    pub dialogue_tools: DialogueTools,

    pub tracing_recorder: Arc<RwLock<TracingRecorder>>,
    pub observability_tools: ObservabilityTools,

    pub prompt_manager: Arc<RwLock<PromptTemplateManager>>,
    pub prompt_tools: PromptTools,
}
```

**配置选项**:
```rust
pub struct IntegratedModulesConfig {
    pub dialogue_storage_dir: PathBuf,
    pub tracing_storage_dir: PathBuf,
    pub prompt_templates_dir: PathBuf,
    pub enable_persistence: bool,
    pub tracing_retention_days: u32,
}
```

---

### 5. dialogue/ - 对话状态机 (已集成)

**文件**:
- `src/dialogue/state_machine.rs` (~440 行)
- `src/dialogue/dialogue_tools.rs` (~280 行)

**状态类型**:
```rust
pub enum DialogueState {
    Idle,               // 空闲
    Clarifying,         // 澄清中
    Planning,           // 规划中
    Executing,          // 执行中
    Reviewing,          // 审查中
    Completed,          // 完成
    Error,              // 错误
    WaitingForConfirmation, // 等待确认
}
```

**工具函数** (tokitai ToolProvider):
- `get_state()` - 获取当前状态
- `get_context()` - 获取对话上下文
- `get_history()` - 获取状态历史
- `set_goal()` - 设置任务目标
- `set_plan()` - 设置任务计划
- `record_tool_execution()` - 记录工具执行
- `transition()` - 状态转换
- `reset()` - 重置状态
- `get_stats()` - 获取统计信息
- `sync_with_autonomy()` - 与 autonomy 模块状态同步

---

### 6. observability/ - 可观测性 (已集成)

**文件**:
- `src/observability/tracing.rs` (~450 行)
- `src/observability/observability_tools.rs` (~380 行)

**Span 类型**:
```rust
pub enum SpanType {
    UserRequest,
    IntentClassification,
    ToolSelection,
    ToolExecution,
    ResponseGeneration,
    StateTransition,
    AutonomousIteration,
    CodeReview,
    GitOperation,
}
```

**工具函数** (tokitai ToolProvider):
- `get_recent_traces(limit)` - 获取最近的追踪记录
- `get_stats()` - 获取统计信息（错误率、平均耗时、类型分布）
- `query_trace(trace_id)` - 查询指定 trace_id 的完整执行链
- `query_errors(limit)` - 查询错误追踪
- `export_traces(output_path, trace_id)` - 导出追踪数据
- `cleanup_old_traces(keep_days)` - 清理旧的追踪文件

---

### 7. prompt_engineering/ - 提示词工程 (已集成)

**文件**:
- `src/prompt_engineering/manager.rs` (~420 行)
- `src/prompt_engineering/prompt_tools.rs` (~270 行)
- `src/prompt_engineering/renderer.rs` (~100 行)
- `src/prompt_engineering/template.rs` (~175 行)

**工具函数** (tokitai ToolProvider):
- `load_role_template(role)` - 加载角色提示词模板
- `list_available_templates()` - 列出所有可用模板
- `has_template(role)` - 检查模板是否存在
- `render_template(role, variables)` - 渲染角色模板
- `render_task_template(task_name, variables)` - 渲染任务模板
- `clear_cache()` - 清除模板缓存
- `reload_template(role)` - 热加载模板
- `get_render_stats()` - 获取渲染统计
- `warmup_cache()` - 预热模板缓存
- `get_all_roles()` - 获取所有角色
- `get_all_task_templates()` - 获取所有任务模板

---

### 8. tools/ - 工具集合 (7,114 行，27.6%)

**子模块**:

| 模块 | 行数 | 工具数 | 功能 |
|------|------|--------|------|
| `io/` | 1,517 | 15 | 文件读写、搜索、PDF 处理、项目模板 |
| `network/` | 4,139 | 20 | HTTP 请求、网页搜索、下载、网络诊断、Wikipedia |
| `system/` | 924 | 13 | 命令执行、进程管理、代码分析、**对话状态**、**可观测性**、**提示词** |
| `data/` | 382 | 5 | JSON 格式化、查询、转换 |
| `vcs/` | 189 | 4 | Git 状态、日志、分支管理 |

**工具箱分类**:
```
file_ops  → 文件操作工具
system    → 系统工具（含 dialogue/observability/prompt_engineering）
code      → 代码工具
web       → 网络工具
git       → Git 工具
data      → 数据处理工具
autonomy  → 自主进化（仅自主模式）
```

**服务化示例** (HttpClientTools):
```rust
impl ServiceLifecycle for HttpClientTools {
    fn service_name(&self) -> &str { "http_client" }
    
    async fn init(&mut self) -> Result<()> {
        // 初始化 HTTP 客户端
        Ok(())
    }
    
    async fn health(&self) -> ServiceHealth {
        // 检查网络连接
        ServiceHealth::Healthy
    }
    
    async fn shutdown(&mut self) -> Result<()> {
        // 清理资源
        Ok(())
    }
    
    fn stats(&self) -> ServiceStats {
        // 返回服务统计
        self.stats.clone()
    }
}
```

---

### 9. context/ - 上下文存储 (4,794 行，18.6%)

**核心特性**:
- **三层存储架构**: 瞬时层 → 短期层 → 长期层
- **增量哈希链 (ICHC)**: 不可篡改的链式哈希结构
- **上下文蒸馏 (HCD)**: 提取核心意图，过滤冗余
- **语义索引 (LSFI)**: 基于 SimHash 的语义搜索

**文件组织**:
```
context/
├── file_service.rs       # 文件服务 trait
├── hash_chain.rs         # 增量哈希链
├── layers.rs             # 存储层管理
├── logger.rs             # 增量日志
├── distiller.rs          # 上下文蒸馏
├── semantic_index.rs     # 语义索引
└── knowledge_index.rs    # 知识索引
```

---

### 10. autonomy/ - 自主进化模块 (7,072 行，13.4%) 🆕

**多 Agent 协作系统 + Prompt Engineering 自进化**:

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  Planner     │ ──▶ │  Executor    │ ──▶ │  Reviewer    │
│  规划 Agent   │     │  执行 Agent   │     │  审查 Agent   │
└──────────────┘     └──────────────┘     └──────────────┘
       ▲                                        │
       └────────────────────────────────────────┘
                    迭代循环

🆕 HybridGapDetector (成本降低 95%, 延迟降低 83-97%)
┌─────────────────────────────────────────────────────────┐
│ Stage 1: Statistical Filter (<100ms, 0 API)             │
│ Stage 2: Causal Analysis (5-30 秒，1-2 API)              │
│ Stage 3: Merger & Prioritize (<50ms, 0 API)             │
└─────────────────────────────────────────────────────────┘
```

**核心组件**:
- `hybrid_gap_detector.rs` 🆕 - 混合缺口检测器（769 行，统计 +Prompt Engineering 融合）
- `prompt_gap_detector.rs` 🆕 - 因果推理 Prompt 缺口检测器（815 行）
- `prompt_optimizer.rs` 🆕 - Few-Shot 工具优化器
- `multi_agent_negotiator.rs` 🆕 - 多智能体协商器（4 角色）
- `self_improvement_loop.rs` - 自进化闭环系统
- `TaskDecomposer` - 任务分解引擎 (DAG 依赖分析)
- `IterationTracker` - 迭代追踪器 (事件溯源)
- `GitWorkflow` - 自主 Git 工作流
- `AgentCoordinator` - Agent 协调器

**论文贡献**:
- HybridGapDetector: 首个融合统计与 Prompt Engineering 的缺口检测器
- PromptGapDetector: 因果推理 Prompt (Chain-of-Thought + 反事实)
- PromptOptimizer: Few-Shot 工具优化
- PromptCreator: 代码生成 + 自修正循环
- MultiAgentNegotiator: 4 角色协商协议

**性能指标**:
- API 成本：从$45/月降至$2.25/月（降低 95%）
- 检测延迟：从 5-30 秒降至 1-5 秒（降低 83-97%）
- 检测准确率：保持 75-85%

**详细文档**: [docs/HYBRID_GAP_DETECTOR_IMPLEMENTATION.md](../docs/HYBRID_GAP_DETECTOR_IMPLEMENTATION.md)

---

## 📈 模块统计

### 按代码行数

```
tools/           ██████████████████████████████████  31.7%  (16,802 行)
context/         ██████████████                      14.0%  (7,398 行)
autonomy/        █████████████                       13.4%  (7,072 行)
orchestrator/    ████████                             7.0%  (4,419 行)
tool_matrix/     ████████                             6.4%  (3,362 行)  ← AI 工具选择器 + 完整工具矩阵
main_core        ██████                               5.8%  (3,079 行)
observability/   █                                    1.7%  (  901 行)
dialogue/        █                                    1.4%  (  751 行)
prompt_eng/      █                                    1.3%  (  677 行)
integration/     █                                    0.6%  (  331 行)
其他             ████████████████████                16.7%  (8,832 行)
─────────────────────────────────────────────────────────────────────
总计                    100.0%  (52,964 行)
```

### 按文件数量

```
tools/:          58 个文件
context/:        13 个文件
autonomy/:       15 个文件
orchestrator/:    7 个文件 (含 workflow_loader)
tool_matrix/:    15 个文件 (含 rule_classifier/query_enhancer/tool_generator/trie_index/dynamic_registry)
prompt_engineering/: 5 个文件
dialogue/:        3 个文件
observability/:   6 个文件
integration/:     2 个文件
```

---

## 🔌 集成状态

### ✅ 已完全集成

| 模块 | 状态 | 说明 |
|------|------|------|
| `main` | ✅ | 程序入口，整合所有组件 |
| `tools/*` | ✅ | 所有工具已注册到工具矩阵，支持服务化 |
| `context/*` | ✅ | 上下文存储已集成 |
| `tool_matrix/*` | ✅ | 工具矩阵/服务注册表已使用 |
| `orchestrator/*` | ✅ | 编排器已集成，支持 TOML 工作流 |
| `dialogue/*` | ✅ | 对话状态机已封装为 tokitai ToolProvider |
| `observability/*` | ✅ | 可观测性已封装为 tokitai ToolProvider |
| `prompt_engineering/*` | ✅ | 提示词工程已封装为 tokitai ToolProvider |
| `integration/*` | ✅ | 集成模块管理器已实现 |

### ⚠️ 部分集成

| 模块 | 状态 | 说明 |
|------|------|------|
| `autonomy/*` | 部分集成 | 可通过 `--autonomous` 参数启用 |

---

## 🚀 快速上手

### 1. 查看项目结构

```bash
# 查看源码树
cargo install cargo-tree
cargo tree

# 或使用 tree 命令
tree -L 2 src/
```

### 2. 运行程序

```bash
# 普通模式
cargo run --release

# 自主进化模式
cargo run --release -- --autonomous

# 指定项目路径
cargo run --release -- -p ./sandbox/test-project
```

### 3. 运行测试

```bash
# 所有测试
cargo test

# 特定模块测试
cargo test autonomy
cargo test context
cargo test tool_matrix
cargo test integration
cargo test dialogue
cargo test observability
cargo test prompt_engineering
cargo test workflow_loader  # TOML 工作流加载器
```

### 4. 性能基准

```bash
cargo bench
```

---

## 📚 相关文档

| 文档 | 说明 |
|------|------|
| [docs/QUICKSTART.md](../docs/QUICKSTART.md) | 快速启动指南 |
| [docs/USER_GUIDE.md](../docs/USER_GUIDE.md) | 完整用户指南 |
| [docs/archive/MODULE_INTEGRATION_REPORT.md](../docs/archive/MODULE_INTEGRATION_REPORT.md) | 模块集成报告 |
| [docs/archive/MODULE_IMPROVEMENT_REPORT.md](../docs/archive/MODULE_IMPROVEMENT_REPORT.md) | 模块改进报告 |
| [docs/archive/SERVICE_ARCHITECTURE_IMPLEMENTATION.md](../docs/archive/SERVICE_ARCHITECTURE_IMPLEMENTATION.md) | 服务化架构实施报告（新增） |
| [structure_ensure/QUICK_REFERENCE.md](QUICK_REFERENCE.md) | 快速参考卡片 |

---

## 📁 运行时文件夹（已添加到 .gitignore）

以下文件夹在运行时自动创建，已添加到 `.gitignore` 中，不会被提交到版本控制：

| 文件夹 | 用途 | 说明 |
|--------|------|------|
| `sandbox/` | 沙箱测试目录 | 用于测试文件操作、项目模板等功能 |
| `downloads/` | 下载文件目录 | 使用下载工具时，文件默认保存到此目录 |
| `.context/` | 上下文存储 | 三层存储架构（瞬时/短期/长期）的持久化数据 |
| `.tokitai/` | 运行时数据 | 对话状态、追踪日志、自主进化数据等 |

> 💡 **提示**：这些文件夹会在首次运行程序时自动创建，无需手动创建。如需清理缓存，可直接删除这些文件夹。

---

## 🎯 核心特性

### ✨ 纯文件上下文存储
- 无数据库依赖
- 三层存储架构（瞬时/短期/长期）
- 自动裁剪，哈希去重

### 🔒 安全沙箱
- 路径验证
- 命令黑名单
- SSRF 防护
- 内网 IP 过滤

### 🛠️ 丰富工具集
- 63+ 工具函数
- 11 个工具箱
- 覆盖文件/网络/系统/Git/数据处理

### 🚀 极致性能
- 缓存响应 <10ms (50x 提升)
- 首次请求延迟降低 50%
- 流式首字节延迟降低 60-70%

### 🤖 自主进化系统
- AI 自主发现改进点
- 规划 → 执行 → 审查 → 推送 GitHub
- 多 Agent 协作

### 🧩 集成模块
- 统一生命周期管理
- 共享状态管理 (`Arc<RwLock>`)
- 与 autonomy 模块状态同步
- 完整的追踪查询和统计
- 模板管理和预热

### 🌐 服务化架构（新增）
- **服务元数据**: 分类、QoS、依赖、版本、标签
- **生命周期管理**: init/health/shutdown/stats
- **健康检查**: Healthy/Degraded/Unhealthy
- **服务统计**: 调用次数/成功率/延迟
- **声明式工作流**: TOML 定义，支持重试/超时/错误处理
- **TOML 工作流加载器**: 从文件/目录加载工作流
- **服务指标收集**: 统一收集服务调用指标
- **服务分类**: 10 种服务类型

---

**最后更新**: 2026-03-15
**测试状态**: 224/224 通过
**构建状态**: Release 成功
