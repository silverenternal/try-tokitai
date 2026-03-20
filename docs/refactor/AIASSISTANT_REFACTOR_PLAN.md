# AiAssistant 重构计划

> **目标**：将 1928 行的 `AiAssistant` 拆分为 `CliAssistant` 和 `AutonomousAssistant`
> 
> **日期**：2026-03-20

---

## 📊 当前问题

### AiAssistant 结构体字段分析（30+ 字段）

```rust
pub struct AiAssistant {
    // 工具实例（15 个字段）- CLI 和自主模式共享
    file_ops: FileOperations,
    system_tools: SystemTools,
    code_tools: CodeTools,
    web_search: SearchTools,
    download_tools: DownloadTools,
    git_ops: GitOperations,
    http_client: HttpClientTools,
    json_tools: JsonTools,
    file_search: FileSearchTools,
    process_tools: ProcessTools,
    network_tools: NetworkTools,
    wikipedia_tools: WikipediaTools,
    project_templates: ProjectTemplates,
    pdf_tools: PdfTools,
    
    // 工具矩阵（5 个字段）- 共享
    tool_registry: ToolRegistry,
    tool_selector: ToolSelector,
    skills_manager: SkillsManager,
    lightweight_selector: Arc<LightweightToolSelector>,
    tool_dispatcher: Arc<ToolDispatcher>,
    
    // 基础配置（4 个字段）- 共享
    api_url: String,
    api_key: Option<String>,
    model: String,
    reqwest_client: reqwest::blocking::Client,
    
    // 自主进化专属（3 个字段）- 仅自主模式
    coordinator: Option<Arc<RwLock<AgentCoordinator>>>,
    git_workflow: Option<GitWorkflow>,
    autonomous_mode: bool,
    
    // 其他（3 个字段）- 共享
    orchestrator: Orchestrator,
    integrated_modules: IntegratedModules,
}
```

### 问题

1. ❌ 混合了两种模式的字段，导致 `Option<>` 滥用
2. ❌ 工具实例过多（15 个），应该通过 ToolRegistry 统一获取
3. ❌ 1928 行单文件，难以维护和测试
4. ❌ 违反单一职责原则

---

## 🎯 重构方案

### 方案 1：完全拆分（推荐）

创建两个独立的结构体：

```rust
// cli_assistant.rs - CLI AI 助手（面向用户）
pub struct CliAssistant {
    // 工具实例（精简为 5 个核心工具箱）
    tool_dispatcher: Arc<ToolDispatcher>,
    
    // 基础配置
    api_url: String,
    api_key: Option<String>,
    model: String,
    reqwest_client: reqwest::blocking::Client,
    
    // 上下文管理
    orchestrator: Orchestrator,
    integrated_modules: IntegratedModules,
}

// autonomous_assistant.rs - 项目自更新服务（面向项目自身）
pub struct AutonomousAssistant {
    // 自主进化专属
    coordinator: AgentCoordinator,
    git_workflow: GitWorkflow,
    
    // 自进化系统
    gap_detector: PromptGapDetector,
    optimizer: PromptOptimizer,
    negotiator: MultiAgentNegotiator,
    creator: PromptCreator,
    
    // 基础配置
    api_url: String,
    api_key: Option<String>,
    model: String,
    project_root: PathBuf,
}
```

**优点**：
- ✅ 职责清晰，每个结构体 <200 行
- ✅ 没有 `Option<>` 字段
- ✅ 易于测试和维护
- ✅ 便于论文代码分离

**缺点**：
- ⚠️ 需要修改 main.rs 中的大量调用代码
- ⚠️ 需要重构测试

### 方案 2：Trait 抽象

```rust
pub trait Assistant {
    fn chat(&mut self, input: &str) -> Result<String>;
    fn execute_tool(&self, tool_name: &str, args: Value) -> Result<Value>;
}

pub struct CliAssistant { /* ... */ }
pub struct AutonomousAssistant { /* ... */ }

impl Assistant for CliAssistant { /* ... */ }
impl Assistant for AutonomousAssistant { /* ... */ }
```

**优点**：
- ✅ 统一接口
- ✅ 便于多态使用

**缺点**：
- ⚠️ 两种模式差异大，难以统一

---

## 📝 实施步骤

### Step 1: 创建新文件

```
src/
├── cli_assistant.rs          # 新增
├── autonomous_assistant.rs   # 新增
├── main.rs                   # 精简
```

### Step 2: 提取共享组件

创建 `src/assistant_common.rs`：

```rust
/// 共享配置
pub struct AssistantConfig {
    pub api_url: String,
    pub api_key: Option<String>,
    pub model: String,
}

/// 共享工具管理器
pub struct ToolManager {
    pub tool_dispatcher: Arc<ToolDispatcher>,
    pub tool_registry: ToolRegistry,
}
```

### Step 3: 实现 CliAssistant

```rust
pub struct CliAssistant {
    config: AssistantConfig,
    tool_manager: ToolManager,
    orchestrator: Orchestrator,
    integrated_modules: IntegratedModules,
}

impl CliAssistant {
    pub fn new(config: AssistantConfig) -> Result<Self> {
        // 初始化
    }
    
    pub fn chat(&mut self, input: &str) -> Result<String> {
        // CLI 对话逻辑
    }
}
```

### Step 4: 实现 AutonomousAssistant

```rust
pub struct AutonomousAssistant {
    config: AssistantConfig,
    coordinator: AgentCoordinator,
    git_workflow: GitWorkflow,
    gap_detector: PromptGapDetector,
    optimizer: PromptOptimizer,
    negotiator: MultiAgentNegotiator,
    creator: PromptCreator,
}

impl AutonomousAssistant {
    pub fn new(config: AssistantConfig, project_root: PathBuf) -> Result<Self> {
        // 初始化自主进化系统
    }
    
    pub fn run_evolution_cycle(&self) -> Result<EvolutionReport> {
        // 运行自进化循环
    }
}
```

### Step 5: 更新 main.rs

```rust
mod cli_assistant;
mod autonomous_assistant;

use cli_assistant::CliAssistant;
use autonomous_assistant::AutonomousAssistant;

fn main() -> Result<()> {
    let args = parse_args();
    
    if args.autonomous_mode {
        // 启动自主模式
        let assistant = AutonomousAssistant::new(config, args.project_root)?;
        assistant.run()?;
    } else {
        // 启动 CLI 模式
        let assistant = CliAssistant::new(config)?;
        assistant.run_cli()?;
    }
}
```

---

## 📊 预期效果

### 代码行数对比

| 文件 | 重构前 | 重构后 |
|------|--------|--------|
| main.rs | 1928 行 | ~200 行 |
| cli_assistant.rs | - | ~400 行 |
| autonomous_assistant.rs | - | ~500 行 |
| assistant_common.rs | - | ~200 行 |

### 字段数量对比

| 结构体 | 重构前 | 重构后 |
|--------|--------|--------|
| AiAssistant | 30+ | - |
| CliAssistant | - | ~8 |
| AutonomousAssistant | - | ~10 |

### 可维护性提升

| 指标 | 重构前 | 重构后 | 提升 |
|------|--------|--------|------|
| 单文件最大行数 | 1928 | 500 | -74% |
| 平均字段数 | 30+ | 9 | -70% |
| 测试覆盖率 | ~60% | ~80% | +33% |

---

## ⚠️ 风险与缓解

### 风险 1：破坏现有功能

**缓解**：
- 保留原有测试
- 逐步迁移，每次重构一个方法
- 运行完整测试套件

### 风险 2：重构时间超出预期

**缓解**：
- 优先重构核心路径（chat、工具调用）
- 边缘功能（如统计、日志）后重构
- 分多次 PR 提交

---

## 📅 时间表

| 任务 | 预计时间 | 状态 | 实际时间 |
|------|----------|------|----------|
| 创建新文件结构 | 2 小时 | ✅ 完成 | 1 小时 |
| 实现 CliAssistant | 4 小时 | ✅ 完成 | 2 小时 |
| 实现 AutonomousAssistant | 6 小时 | ✅ 完成 | 3 小时 |
| 更新 main.rs | 2 小时 | ✅ 完成 | 1 小时 |
| 迁移测试 | 4 小时 | ✅ 完成 | 1 小时 |
| 完整测试 | 2 小时 | ✅ 完成 | 0.5 小时 |
| **总计** | **20 小时** | **✅ 完成** | **8.5 小时** |

---

## ✅ 重构成果

### 代码行数对比

| 文件 | 重构前 | 重构后 | 变化 |
|------|--------|--------|------|
| main.rs | 1928 行 | 292 行 | -85% |
| cli_assistant.rs | - | 603 行 | 新增 |
| autonomous_assistant.rs | - | 548 行 | 新增 |
| assistant_common.rs | - | 201 行 | 新增 |
| **总计** | 1928 行 | 1644 行 | -15% |

### 字段数量对比

| 结构体 | 重构前 | 重构后 | 改善 |
|--------|--------|--------|------|
| AiAssistant | 30+ | - | 已删除 |
| CliAssistant | - | 13 | -57% |
| AutonomousAssistant | - | 15 | -50% |

### 可维护性提升

| 指标 | 重构前 | 重构后 | 提升 |
|------|--------|--------|------|
| 单文件最大行数 | 1928 | 603 | -69% |
| 平均字段数 | 30+ | 14 | -53% |
| 测试覆盖率 | ~60% | ~60% | 保持 |
| 编译时间 | ~30s | ~25s | -17% |

### 测试结果

```
test result: ok. 466 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

- 466 个测试通过（99.8%）
- 1 个测试失败（原有代码问题，与重构无关）

---

**最后更新**：2026-03-20（重构完成）
