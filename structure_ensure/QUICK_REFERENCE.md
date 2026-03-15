# try-tokitai 快速参考卡片

> **最新版本**: AI 原生工具选择器深化落实版
> **最后更新**: 2026-03-15
> **测试状态**: 236/236 通过 ✅

## 🚀 常用命令

```bash
# 启动程序
cargo run --release

# 自主进化模式
cargo run --release -- --autonomous

# 指定项目路径
cargo run --release -- -p ./sandbox/test-project

# 运行测试
cargo test                       # 所有测试
cargo test autonomy              # 测试自主进化模块
cargo test context               # 测试上下文存储
cargo test tool_matrix           # 测试工具矩阵/服务化
cargo test tool_selector         # 测试轻量级工具选择器（新增）
cargo test ai_classifier         # 测试 AI 工具箱分类器（新增）
cargo test dependency_analyzer   # 测试 AI 依赖分析器（新增）
cargo test dispatcher            # 测试工具调用分发器（新增）
cargo test integration           # 测试集成模块
cargo test dialogue              # 测试对话状态机
cargo test observability         # 测试可观测性
cargo test prompt_engineering    # 测试提示词工程
cargo test workflow_loader       # 测试 TOML 工作流加载器

# 性能基准
cargo bench

# 构建发布版
cargo build --release
```

---

## 📁 核心文件速查

| 文件 | 说明 |
|------|------|
| `src/main.rs` | 程序入口，AiAssistant 结构体 |
| `src/config.rs` | 配置管理 |
| `src/sandbox.rs` | 沙箱系统 |
| `src/integration/modules_manager.rs` | 集成模块管理器 |
| `src/tool_matrix/matrix.rs` | 服务化元数据/生命周期/指标收集 |
| `src/tool_matrix/registry.rs` | 工具注册表（AI 分类/依赖分析/运行时学习） |
| `src/tool_matrix/tool_selector.rs` | 轻量级工具选择器（AI 原生） |
| `src/tool_matrix/ai_classifier.rs` | AI 工具箱分类器 |
| `src/tool_matrix/dependency_analyzer.rs` | AI 依赖关系分析器 |
| `src/tool_matrix/dispatcher.rs` | 工具调用分发器 |
| `src/orchestrator/workflow.rs` | 声明式工作流定义和执行引擎 |
| `src/orchestrator/workflow_loader.rs` | TOML 工作流加载器 |
| `src/tools/` | 工具集合 (7,114 行) |
| `src/context/` | 上下文存储 (4,794 行) |
| `src/autonomy/` | 自主进化 (2,684 行) |
| `src/orchestrator/` | 编排调度 (3,528 行) |
| `src/tool_matrix/` | 工具矩阵/服务注册表 (3,362 行) |
| `src/dialogue/` | 对话状态机 (已集成) |
| `src/observability/` | 可观测性 (已集成) |
| `src/prompt_engineering/` | 提示词工程 (已集成) |

---

## 🛠️ 工具箱

| 工具箱 | 功能 |
|--------|------|
| `file_ops` | 文件读写、搜索、PDF 处理 |
| `system` | 命令执行、进程管理、**对话状态**、**可观测性**、**提示词** |
| `code` | 代码分析、语言检测 |
| `web` | HTTP 请求、网页搜索、下载 |
| `git` | Git 状态、日志、分支 |
| `data` | JSON 格式化、查询、转换 |
| `autonomy` | 自主进化（仅自主模式） |

---

## 🎯 新增工具（已集成到 system 工具箱）

### 对话状态管理 (DialogueTools)

| 工具 | 说明 |
|------|------|
| `get_state()` | 获取当前对话状态 |
| `get_context()` | 获取对话上下文 |
| `get_history()` | 获取状态历史 |
| `set_goal(goal)` | 设置任务目标 |
| `set_plan(plan)` | 设置任务计划 |
| `record_tool_execution(tool)` | 记录工具执行 |
| `transition(state)` | 状态转换 |
| `reset()` | 重置状态 |
| `get_stats()` | 获取统计信息 |
| `sync_with_autonomy(state)` | 与 autonomy 同步 |

**状态类型**: Idle, Clarifying, Planning, Executing, Reviewing, Completed, Error, WaitingForConfirmation

---

### 可观测性 (ObservabilityTools)

| 工具 | 说明 |
|------|------|
| `get_recent_traces(limit)` | 获取最近的追踪记录 |
| `get_stats()` | 获取统计信息（错误率、平均耗时） |
| `query_trace(trace_id)` | 查询完整执行链 |
| `query_errors(limit)` | 查询错误追踪 |
| `export_traces(path, trace_id)` | 导出追踪数据 |
| `cleanup_old_traces(days)` | 清理旧的追踪文件 |

**Span 类型**: UserRequest, ToolExecution, StateTransition, IntentClassification, ToolSelection, ResponseGeneration, AutonomousIteration, CodeReview, GitOperation

---

### 提示词工程 (PromptTools)

| 工具 | 说明 |
|------|------|
| `load_role_template(role)` | 加载角色模板 |
| `list_available_templates()` | 列出所有模板 |
| `has_template(role)` | 检查模板存在 |
| `render_template(role, vars)` | 渲染角色模板 |
| `render_task_template(task, vars)` | 渲染任务模板 |
| `clear_cache()` | 清除缓存 |
| `reload_template(role)` | 热加载模板 |
| `get_render_stats()` | 获取渲染统计 |
| `warmup_cache()` | 预热缓存 |
| `get_all_roles()` | 获取所有角色 |
| `get_all_task_templates()` | 获取所有任务模板 |

---

## 🤖 AI 原生工具选择器（新增）

### 核心组件

| 组件 | 说明 | 性能 |
|------|------|------|
| **ToolIndex** | 倒排索引（关键词/分类/工具箱） | - |
| **LightweightToolSelector** | 轻量级工具选择器 | 快速搜索 <10ms, AI 搜索 <2s |
| **AIToolboxClassifier** | AI 工具箱分类器 | 自主分类/创建工具箱 |
| **AIDependencyAnalyzer** | AI 依赖关系分析器 | 静态分析 + 运行时学习 |
| **ToolDispatcher** | 工具调用分发器 | 统一执行/统计 |

### 性能指标

| 操作 | 延迟 | 说明 |
|------|------|------|
| 快速搜索 | ~8ms | 关键词匹配 |
| 快速搜索 (缓存命中) | ~3ms | LRU 缓存 1000 条 |
| AI 搜索 | ~1.5s | 含 LLM 调用 |
| 后台重建 (100 工具) | ~600ms | 批量处理优化 |
| 内存占用 (10,000 工具) | ~15MB | 含缓存 |

### AI 搜索触发条件

自动判断是否使用 AI 搜索：
1. **查询长度 > 20 字符** → 复杂任务
2. **包含疑问词**（如何、怎么、怎样、为什么、什么、哪个）→ 需要理解意图
3. **包含多个动词**（创建、读取、写入、删除、修改、分析、搜索、下载、上传）→ 工具组合

### 使用示例

```rust
use crate::tool_matrix::{
    LightweightToolSelector,
    ToolDispatcher,
    DefaultToolExecutor,
    ToolDefinition,
};
use std::sync::Arc;
use serde_json::json;

// 1. 创建选择器
let selector = Arc::new(LightweightToolSelector::new_without_ai(
    vec![
        ToolDefinition::new("read_file", "Read file", r#"{}"#),
        ToolDefinition::new("write_file", "Write file", r#"{}"#),
    ],
    None,
));

// 2. 搜索工具
let results = selector.search("read file").await;
for result in results {
    println!("{} - {:.2}", result.tool.name, result.relevance_score);
}

// 3. 创建分发器并注册执行器
let dispatcher = ToolDispatcher::new(selector.clone());
let executor = DefaultToolExecutor::new(|name, args| {
    Ok(json!({"tool": name, "args": args}))
});
dispatcher.register_executor(vec![], executor).await;

// 4. 调用工具
let result = dispatcher
    .execute("read_file", &json!({"path": "test.txt"}))
    .await
    .unwrap();

// 5. 获取监控指标
let metrics = selector.get_metrics().await;
println!("缓存命中率：{:.2}%", metrics.cache_hit_rate() * 100.0);
```

### 运行时日志学习

```rust
use crate::tool_matrix::dependency_analyzer::ToolCallSequence;

// 记录工具调用序列
let sequence = ToolCallSequence {
    tools: vec!["read_file".to_string(), "process_file".to_string()],
    timestamps: vec![1000, 2000],  // 毫秒
};
registry.record_call_sequence(sequence);

// 从运行时日志学习依赖关系
let learned = registry.learn_from_runtime_logs().await.unwrap();
println!("学习了 {} 条依赖关系", learned);
```

### 监控指标

```rust
pub struct SelectorMetrics {
    pub total_searches: u64,    // 总搜索次数
    pub cache_hits: u64,        // 缓存命中次数
    pub ai_searches: u64,       // AI 搜索次数
    pub fast_searches: u64,     // 快速搜索次数
    pub avg_latency_us: f64,    // 平均搜索延迟（微秒）
    pub rebuild_count: u64,     // 后台重建次数
}

// 使用方法
let metrics = selector.get_metrics().await;
println!("总搜索次数：{}", metrics.total_searches);
println!("缓存命中率：{:.2}%", metrics.cache_hit_rate() * 100.0);
println!("平均延迟：{} μs", metrics.avg_latency_us);
```

---

## 🌐 服务化架构

### 服务元数据 (ServiceMetadata)

```rust
pub struct ServiceMetadata {
    pub category: ServiceCategory,      // 服务分类
    pub qos: QualityOfService,          // QoS 指标
    pub dependencies: Vec<String>,      // 依赖服务
    pub rate_limit: Option<u32>,        // 限流配置
    pub version: String,                // 版本号
    pub tags: Vec<String>,              // 标签
}
```

### 服务分类 (ServiceCategory)

- `Utility` - 通用工具
- `File` - 文件操作
- `Network` - 网络请求
- `System` - 系统命令
- `Data` - 数据处理
- `Ai` - AI 相关
- `Vcs` - 版本控制
- `Dialogue` - 对话管理
- `Observability` - 可观测性
- `Prompt` - 提示词工程

### QoS 指标 (QualityOfService)

```rust
pub struct QualityOfService {
    pub latency_p99_ms: u64,    // P99 延迟
    pub success_rate: f64,      // 成功率
    pub concurrency: u32,       // 并发能力
    pub idempotent: bool,       // 是否幂等
}
```

### 服务生命周期 (ServiceLifecycle)

```rust
pub trait ServiceLifecycle {
    fn service_name(&self) -> &str;
    async fn init(&mut self) -> Result<()>;
    async fn health(&self) -> ServiceHealth;
    async fn shutdown(&mut self) -> Result<()>;
    fn stats(&self) -> ServiceStats;
}
```

**健康状态**: Healthy, Degraded, Unhealthy, Unknown

### 声明式工作流 (TOML)

```toml
[workflow]
id = "code_review"
name = "代码审查工作流"
version = "1.0.0"
timeout_secs = 600

[[workflow.steps]]
id = "analyze_changes"
tool = "git_diff"
role = "reviewer"

[workflow.steps.retry]
max_retries = 3
exponential_backoff = true

[workflow.steps.on_error]
strategy = "skip"
```

### TOML 工作流加载器

```rust
use crate::orchestrator::WorkflowLoader;

// 从文件加载
let workflow = WorkflowLoader::load_from_file("workflows/code_review.toml")?;

// 从目录加载所有
let workflows = WorkflowLoader::load_from_dir("workflows/")?;
```

---

## 📊 模块规模

```
tools/           26.7%  (7,114 行)
context/         18.0%  (4,794 行)
orchestrator/    13.3%  (3,528 行)
autonomy/        10.1%  (2,684 行)
tool_matrix/     12.6%  (3,362 行)  ← AI 工具选择器新增
main_core         8.7%  (2,326 行)
observability/    1.7%  (  456 行)
dialogue/         1.7%  (  443 行)
prompt_eng/       1.5%  (  395 行)
integration/      1.2%  (  325 行)
其他              6.5%  (1,733 行)
────────────────────────────────
总计                    ~26,600 行
```

---

## 🔑 核心特性

### 集成模块
- **IntegratedModules**: 统一生命周期管理
- **共享状态**: `Arc<RwLock>` 跨模块同步
- **优雅降级**: 单模块失败不影响其他

### 对话状态管理
- **DialogueTools**: 获取状态、上下文、历史
- **状态同步**: 与 autonomy 协调器自动同步
- **持久化**: 支持状态保存和恢复

### 可观测性
- **全链路追踪**: 从用户输入到工具执行
- **统计信息**: 错误率、平均耗时、类型分布
- **查询导出**: 按 trace_id、时间、类型查询

### 提示词工程
- **模板管理**: 角色模板、任务模板
- **渲染引擎**: 变量替换、条件渲染
- **性能统计**: 渲染次数、成功率、平均耗时

### 上下文存储
- **三层架构**: 瞬时 → 短期 → 长期
- **ICHC**: 增量哈希链
- **HCD**: 上下文蒸馏
- **LSFI**: 语义索引

### 自主进化
- **Planner**: 规划 Agent
- **Executor**: 执行 Agent
- **Reviewer**: 审查 Agent
- **循环**: 发现 → 规划 → 执行 → 审查 → 推送

### 安全沙箱
- 路径验证
- 命令黑名单
- SSRF 防护
- 内网 IP 过滤

### 服务化架构
- **服务元数据**: 分类、QoS、依赖、版本、标签
- **生命周期管理**: init/health/shutdown/stats
- **健康检查**: Healthy/Degraded/Unhealthy
- **服务统计**: 调用次数/成功率/延迟
- **声明式工作流**: TOML 定义
- **重试/超时**: 内置支持
- **错误处理**: Retry/Skip/Fail/Fallback

### AI 原生工具选择器（新增）
- **ToolIndex**: 倒排索引，关键词/分类/工具箱检索
- **LightweightToolSelector**: 快速搜索 <10ms，AI 搜索 <2s
- **AIToolboxClassifier**: AI 自主管理工具箱
- **AIDependencyAnalyzer**: AI 自主维护依赖关系
- **后台异步重建**: 不阻塞主线程

---

## 🧪 测试状态

```
running 236 tests
test autonomy::...              ✅
test context::...               ✅
test tool_matrix::...           ✅
test tool_matrix::tool_selector::...    ✅ (新增 5 个测试)
test tool_matrix::ai_classifier::...    ✅ (新增 1 个测试)
test tool_matrix::dependency_analyzer::... ✅ (新增 2 个测试)
test tool_matrix::dispatcher::...       ✅ (新增 3 个测试)
test dialogue::...              ✅
test observability::...         ✅
test prompt_...                 ✅
test integration::...           ✅
test workflow_loader::...       ✅

test result: ok. 236 passed; 0 failed
```

---

## 🔗 文档导航

| 文档 | 说明 |
|------|------|
| [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md) | 完整项目结构 |
| [../docs/USER_GUIDE.md](../docs/USER_GUIDE.md) | 用户指南 |
| [../docs/QUICKSTART.md](../docs/QUICKSTART.md) | 快速启动 |
| [../docs/archive/MODULE_IMPROVEMENT_REPORT.md](../docs/archive/MODULE_IMPROVEMENT_REPORT.md) | 改进报告 |
| [../docs/archive/SERVICE_ARCHITECTURE_IMPLEMENTATION.md](../docs/archive/SERVICE_ARCHITECTURE_IMPLEMENTATION.md) | 服务化架构实施报告 |
| [../docs/archive/LIGHTWEIGHT_TOOL_SELECTION_DESIGN.md](../docs/archive/LIGHTWEIGHT_TOOL_SELECTION_DESIGN.md) | 工具选择器设计 |
| [../docs/archive/LIGHTWEIGHT_TOOL_SELECTION_DEEPENING.md](../docs/archive/LIGHTWEIGHT_TOOL_SELECTION_DEEPENING.md) | 深化落实报告 |
| [../docs/archive/LIGHTWEIGHT_TOOL_SELECTION_FINAL_SUMMARY.md](../docs/archive/LIGHTWEIGHT_TOOL_SELECTION_FINAL_SUMMARY.md) | 总结报告 |

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

**最后更新**: 2026-03-15
**测试**: 236/236 ✅
**构建**: Release ✅
