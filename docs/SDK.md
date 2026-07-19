# Atlas IDE SDK Reference

本文档面向 Atlas 的宿主集成开发者、工具作者、研究域插件作者、可视化适配器作者及科学运行时开发者。它描述当前仓库中可直接使用的源码级 SDK、显式版本化协议，以及扩展实现必须遵守的安全与兼容性约束。

## 1. SDK 范围与兼容性

Atlas SDK 不是单一包，而是一组分层契约：

| 层级 | 主要入口 | 兼容性标识 | 典型用途 |
| --- | --- | --- | --- |
| 桌面宿主桥 | `ai_assistant::host`、`DesktopHostRuntime` | `atlas-host-v1` | 原生壳、前端命令、Agent 流 |
| 工具 SDK | `ai_assistant::tool_matrix` | Rust 源码 API + JSON Schema | 注册、发现和执行 Agent 工具 |
| 研究域 SDK | `ai_assistant::research_domains` | `atlas.research-domain.v1` / API `1` | 声明领域、资产、工作台、动作和上下文 |
| 可视化 SDK | `ai_assistant::visualization` | `atlas.visualization.v1` / API `1` | 发现数据源并生成统一可视化文档 |
| Atlas Core | `ai_assistant::atlas_core` | Rust 源码 API | 版本化科学对象、关系、事件和查询 |
| RIE | `ai_assistant::research_intelligence` | Rust 源码 API | 规划、运行时、执行、推荐和插件生命周期 |
| MCP | `ai_assistant::mcp` | MCP stdio + Atlas 安全策略 | 连接外部 MCP 工具或将 Atlas 作为 MCP 服务 |
| HTTP API | `build_web_router` 暴露的 `/api/*` | 应用级 JSON 契约 | Web/测试/兼容客户端 |

兼容性规则：

- 带显式版本字段的协议只有在升级对应版本号时才允许破坏性变更。
- `pub` Rust 类型是源码兼容 API，不承诺稳定 C ABI；集成方应锁定 crate 版本和 `Cargo.lock`。
- `src/web.rs` 中未导出的 DTO、内部活动标签和存储文件布局属于实现细节。
- 新字段应保持向后兼容，反序列化结构应优先使用 `#[serde(default)]`。
- 插件必须检查收到的 schema/API 版本；不支持的主版本应明确拒绝，不能静默猜测。

## 2. 依赖与功能开关

在同一 Cargo workspace 内：

```toml
[dependencies]
ai-assistant = { path = ".." }
anyhow = "1"
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
```

可选功能：

| Feature | 说明 |
| --- | --- |
| `desktop-shell` | 启用 Wry/Tao 原生桌面宿主 |
| `domain-science` | 启用可选科学领域依赖 |
| `yaml` | 启用 YAML/OpenAPI 解析能力 |
| `tensor` | 实验性 Candle/ndarray 张量能力 |
| `test-allow-all-paths` | 仅供测试；不得用于生产构建 |

桌面构建示例：

```powershell
cargo build --release --bin desktop_wry --features desktop-shell
```

## 3. 运行目录与持久化约定

`AppPaths` 将安装目录、前端资源和可变状态分离。桌面项目状态位于系统本地数据目录下的 `Atlas/projects/<project_id>/`，而工作区科学状态位于 `<workspace>/.atlas/`。

常用路径：

| API | 内容 |
| --- | --- |
| `AppPaths::state_dir()` | 项目级应用状态根目录 |
| `sessions_dir()` | 会话 JSON |
| `sandbox_dir()` | Atlas 沙箱 |
| `downloads_dir()` | 受控下载目录 |
| `web_runtime_state_path()` | 当前模型、工作区、会话等运行配置 |
| `workspace_state_dir(workspace)` | 工作区 `.atlas/` |
| `workspace_run_debug_dir(workspace)` | 运行/调试状态 |

`project_id(workspace)` 基于规范化工作区路径生成稳定项目标识。不要自行拼接桌面项目状态目录。

## 4. 原生宿主桥 SDK

### 4.1 核心类型

```rust
use ai_assistant::host::{
    HostBridgeResponse, HostBridgeStream, HostCapabilities, HostCommand,
    HostDescriptor, BRIDGE_PROTOCOL_V1,
};
```

- `HostDescriptor` 声明 `mode`、`transport` 和宿主能力。
- `HostBridgeResponse` 是非流式命令响应，包含 `ok`、HTTP 风格 `status`、`data`、`error` 和协议名。
- `HostBridgeStream` 包含命令、会话 ID 及无界事件接收器。
- `DesktopHostRuntime::invoke()` 调用非流式命令。
- `DesktopHostRuntime::open_stream()` 打开流式命令。

创建宿主：

```rust
use ai_assistant::{AssistantConfig, desktop_host::DesktopHostRuntime};
use ai_assistant::{config::Config, security::SecurityConfig, web::WebHostConfig};
use std::path::PathBuf;

let host = WebHostConfig::for_desktop_shell(
    PathBuf::from("."),
    PathBuf::from("frontend"),
    PathBuf::from(".atlas-sdk-state"),
);
let runtime = DesktopHostRuntime::new(
    host,
    AssistantConfig::new(
        "http://127.0.0.1:11434/v1/chat/completions".into(),
        None,
        "local-model".into(),
    ),
    Config::default(),
    SecurityConfig::default(),
)?;
```

### 4.2 命令表

`HostCommand` 是命令名称的权威来源。当前分组如下：

| 分组 | 命令 |
| --- | --- |
| 引导/设置 | `bootstrap.load`, `settings.update` |
| 工作区 | `workspace.pick`, `workspace.file.open`, `workspace.file.save`, `workspace.file.undo`, `workspace.file.complete`, `workspace.review.file` |
| 索引 | `workspace.index.state`, `workspace.index.update`, `workspace.index.search` |
| Agent | `chat.send`, `chat.stream`, `chat.stop`, `prompt.optimize`, `schedule.manage` |
| 授权 | `tool.approval.approve`, `tool.approval.deny` |
| 后台任务 | `tasks.state`, `tasks.enqueue`, `tasks.start`, `tasks.cancel`, `tasks.log` |
| 可视化 | `visualization.catalog`, `visualization.snapshot` |
| 检索 | `search.health`, `search.web`, `search.papers`, `search.models`, `search.tracking`, `search.benchmarks`, `search.github`, `search.github_preview`, `search.datasets`, `search.dataset_manifest` |
| Git/调试 | `git.state`, `git.action`, `run_debug.state`, `run_debug.action` |
| 终端 | `terminals.state`, `terminals.create`, `terminals.input`, `terminals.close` |
| 会话 | `sessions.create`, `sessions.select`, `sessions.rename`, `sessions.delete` |
| 研究审查 | `reviewer_feedback.state`, `reviewer_feedback.add`, `reviewer_feedback.resolve`, `research.paper_workflow.run` |
| 其他 | `browser.open`, `native.request` |

调用示例：

```rust
use serde_json::json;

let response = runtime.invoke("workspace.index.search", json!({
    "query": "StreamSessionRuntime",
    "limit": 20
})).await;
if !response.ok {
    anyhow::bail!(response.error.unwrap_or_else(|| "bridge failure".into()));
}
```

### 4.3 前端 TypeScript 桥

桌面注入对象：

```ts
interface AtlasDesktopBridge {
  invoke<T = unknown>(
    command: string,
    payload?: Record<string, unknown>,
  ): Promise<AtlasDesktopBridgeResponse<T>>;

  openStream?(
    command: string,
    payload?: Record<string, unknown>,
  ): Promise<Response>;
}
```

非流式调用：

```ts
const result = await window.__ATLAS_DESKTOP_BRIDGE__!.invoke(
  "workspace.file.open",
  { path: "src/lib.rs" },
);
if (result.ok === false) throw new Error(result.error);
```

流式调用：

```ts
const response = await window.__ATLAS_DESKTOP_BRIDGE__!.openStream!(
  "chat.stream",
  { content: "分析当前工作区", mode: "agent", language: "zh" },
);
const reader = response.body!.getReader();
const decoder = new TextDecoder();
let buffer = "";

for (;;) {
  const { value, done } = await reader.read();
  if (value) buffer += decoder.decode(value, { stream: !done });
  let newline;
  while ((newline = buffer.indexOf("\n")) >= 0) {
    const line = buffer.slice(0, newline).trim();
    buffer = buffer.slice(newline + 1);
    if (line) handleAtlasEvent(JSON.parse(line));
  }
  if (done) break;
}
```

### 4.4 Agent 流事件

每个事件至少包含 `type`，并可包含 `session_id`。已定义事件类型：

| `type` | 主要字段 | 语义 |
| --- | --- | --- |
| `assistant_delta` | `delta` | 助手正文增量 |
| `thinking_delta` | `thinking_delta` | 可显示推理增量 |
| `assistant_progress` | `activity` | 阶段性用户可见进度 |
| `activity` | `activity` | 上下文、计划、执行、验证等运行事件 |
| `tool` | `tool` | 工具开始、完成、拒绝或失败 |
| `permission_required` | `permission` | 需要用户批准的工具调用 |
| `edited_files` | `edited_files` | 文件差异和统计 |
| `subagent` | `subagents` | 子代理状态与证据 |
| `verifier` | `verifier` | 确定性验证报告 |
| `complete` | `messages`, `activity` | 成功或用户停止的终态 |
| `error` | `error`, `messages` | 失败终态；已保存中断内容 |

传输不变量：

- 每个流必须恰好以 `complete` 或 `error` 结束。
- 客户端必须按 `call_id` 关联工具事件，不能按显示名称关联。
- 增量可能是纯 delta，也可能由兼容提供商返回累计文本；客户端合并必须幂等。
- 未收到终态即连接关闭应视为协议错误，不能假定成功。
- `permission_required` 后通过 `tool.approval.approve/deny` 回传同一 `session_id` 和 `call_id`。

### 4.5 定时任务

`schedule.manage` 请求：

```json
{
  "command": "daily 09:00 检查构建和测试状态",
  "session_id": "optional-session-id"
}
```

支持：`in <n><s|m|h|d> <prompt>`、`at YYYY-MM-DD HH:MM <prompt>`、`daily HH:MM <prompt>`、`list`、`cancel <id>`。任务持久化在项目状态目录的 `schedules.json`，到期执行仍使用原生 Agent 流。Atlas 进程关闭期间不执行；重新启动后，已到期任务会被恢复执行。

## 5. 工具 SDK

### 5.1 编程式工具

```rust
use ai_assistant::tool_matrix::{
    dispatcher::{ToolDispatcher, ToolExecutor},
    matrix::ToolDefinition,
    tool_selector::LightweightToolSelector,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

struct EchoExecutor;

#[async_trait]
impl ToolExecutor for EchoExecutor {
    async fn execute(&self, tool_name: &str, args: &Value) -> Result<Value, String> {
        if tool_name != "sdk_echo" { return Err("unsupported tool".into()); }
        Ok(json!({ "text": args.get("text").and_then(Value::as_str) }))
    }
}

let definition = ToolDefinition::new(
    "sdk_echo",
    "Return a structured echo response.",
    r#"{
      "type":"object",
      "properties":{"text":{"type":"string"}},
      "required":["text"],
      "additionalProperties":false
    }"#,
)
.with_source("sdk")
.with_risk_level("safe")
.with_tag("example");

let selector = Arc::new(LightweightToolSelector::new_without_ai(Vec::new(), None));
let dispatcher = ToolDispatcher::new(selector);
dispatcher.register_executor(vec![definition], EchoExecutor).await;
let value = dispatcher
    .execute("sdk_echo", &json!({"text":"hello"}))
    .await
    .map_err(anyhow::Error::msg)?;
```

工具名必须在注册表内唯一。JSON Schema 应使用 `additionalProperties: false`，明确必填字段并限制数组长度、枚举和字符串格式。执行器应返回结构化 JSON；传输成功不等于语义成功，失败必须返回 `Err` 或明确失败字段。

### 5.2 TOML 工具描述

`TomlToolLoader` 从 `tools/**/*.toml` 递归加载并按修改时间热更新：

```toml
[tool]
name = "dataset_summary"
version = "1.0.0"
description = "Summarize a bounded dataset artifact"
author = "Example Lab"
category = "data"
entry_point = "example::DatasetSummary"
tags = ["dataset", "read-only"]
license = "MIT"

[[parameters]]
name = "path"
type = "string"
description = "Workspace-relative dataset path"
required = true

[parameters.validation]
min_length = 1
max_length = 512

[permissions]
file_read = true
file_write = false
network_access = false
execute_command = false
env_access = false

[rate_limit]
requests_per_minute = 60
requests_per_second = 4

dependencies = []
```

TOML 定义负责元数据、参数、权限和发现；实际执行仍需注册 `ToolExecutor` 或 `tokitai::ToolProvider`。

### 5.3 动态工具

`DynamicToolRegistry` 使用：

```text
<workspace>/.atlas/tools/*.json       动态元数据
<workspace>/src/tools/generated/*.rs  生成的 Rust 源码
```

`DynamicToolMetadata` 记录版本、依赖、作者、源文件、签名和启用状态。动态源码不是运行时加载的任意二进制：生成后仍应经过代码审查、权限检查、编译和测试。签名验证失败的工具不得自动启用。

### 5.4 工具风险

Atlas 使用 `Safe < Moderate < Low` 的风险顺序。新增工具必须：

1. 在工具定义中给出风险等级；
2. 在 `default_tool_risk_map()` 中登记生产风险；
3. 对文件路径使用允许根和规范化解析；
4. 对命令执行使用参数数组或安全解析器，禁止拼接不受信任 shell；
5. 为写操作、网络操作和外部副作用提供审计结果；
6. 遵守全局每分钟和 burst 限流。

## 6. 研究域插件 SDK

### 6.1 编程式插件

`IDomainPlugin` 聚合六个 provider trait：

- `IDataProvider::discover_assets`
- `IVisualizationProvider::visualization_document`
- `IAgentContextProvider::agent_context`
- `IPreviewProvider::preview_metadata`
- `IRenderProvider::renderers`
- `IExecutionProvider::execution_context`

生命周期钩子为 `on_register`、`on_activate`、`on_deactivate`、`on_workspace_change`。

```rust
use ai_assistant::research_domains::{
    DomainProviderContext, IDomainPlugin, IDataProvider, IAgentContextProvider,
    IExecutionProvider, IPreviewProvider, IRenderProvider, IVisualizationProvider,
    ResearchDomainRegistry,
};

let registry = ResearchDomainRegistry::default();
registry.register(MyDomainPlugin::new())?;
```

插件 descriptor 必须使用 `RESEARCH_DOMAIN_SCHEMA_VERSION` 和 `RESEARCH_DOMAIN_PLUGIN_API_VERSION`，领域 ID 应稳定、全局唯一并使用小写短横线形式。资产 ID 应由领域 ID 与工作区相对路径稳定派生，不能使用每次扫描变化的随机 UUID。

### 6.2 声明式工作区插件

无需重新编译的插件可放置在：

```text
<workspace>/.atlas/domains/<domain-id>.json
```

顶层结构：

```json
{
  "plugin": {
    "metadata": {
      "id": "example-domain",
      "label": "Example Domain",
      "description": "Example scientific workspace",
      "version": "1.0.0",
      "category": "science"
    },
    "capabilities": ["artifact-discovery", "visualization"],
    "supported_file_types": ["csv", "json"],
    "supported_visualizations": [{
      "id": "example-table",
      "label": "Example Table",
      "renderer": "table",
      "compatible_file_types": ["csv", "json"],
      "adapter": "generic",
      "workbench_region": "primary",
      "requires_sdk": []
    }],
    "supported_agents": ["researcher"],
    "context_provider": {"id":"example.context","api_version":"1","provider_type":"native"},
    "preview_provider": {"id":"example.preview","api_version":"1","provider_type":"native"},
    "execution_provider": {"id":"example.execution","api_version":"1","provider_type":"native"},
    "data_provider": {"id":"example.data","api_version":"1","provider_type":"native"},
    "visualization_provider": {"id":"example.visualization","api_version":"1","provider_type":"native"},
    "render_provider": {"id":"example.render","api_version":"1","provider_type":"native"},
    "lifecycle": {
      "states": ["active", "archived"],
      "supports_hot_reload": true,
      "supports_workspace_sync": true
    },
    "sdk_adapters": [],
    "plugin_api_version": "1",
    "workbench": {
      "layout": "explorer-primary-inspector-bottom",
      "explorer_label": "Assets",
      "primary_label": "Workspace",
      "inspector_label": "Evidence",
      "bottom_panel_label": "Runs",
      "tools": [],
      "workflow": [],
      "intents": [],
      "object_model": ["artifact", "run", "evidence"],
      "interaction_model": "select-inspect-run",
      "inspector_model": "artifact metadata and verification evidence",
      "visualization_model": "table",
      "runtime": "atlas-native",
      "preview_kind": "object-card"
    }
  },
  "match_keywords": ["example"],
  "content_markers": ["example_schema"]
}
```

发现器不会跟随符号链接，并跳过 `.git`、`node_modules`、`target`、`dist`、`build`、`vendor`、虚拟环境、缓存和 `.atlas`。扫描受最大文件数、深度和文本大小限制。插件不得依赖遍历被跳过目录。

### 6.3 工作区状态、任务和动作

- 工作区 UI/Agent 状态通过 `read_workspace_state`、`update_workspace_state` 共享。
- `begin_task` 创建真实领域任务；`update_task` 记录阶段、状态、产物和证据。
- `list_actions` 只列出已注册且环境满足的原生动作。
- `run_action` 只能调用白名单动作，不能把 Agent 文本伪装成 SDK 执行结果。
- 完成状态必须附带存在的工作区产物和验证证据。

## 7. 可视化适配器 SDK

实现 `VisualizationAdapter`：

```rust
use ai_assistant::visualization::{
    type_descriptor, VisualizationAdapter, VisualizationContext, VisualizationRegistry,
};
use ai_assistant::visualization::model::{
    VisualizationDocument, VisualizationSource, VisualizationTypeDescriptor,
};
use anyhow::Result;

struct ExampleAdapter;

impl VisualizationAdapter for ExampleAdapter {
    fn descriptor(&self) -> VisualizationTypeDescriptor {
        type_descriptor("example", "Example", "Example graph", "example.adapter")
    }

    fn discover(&self, _context: &VisualizationContext<'_>) -> Result<Vec<VisualizationSource>> {
        Ok(Vec::new())
    }

    fn parse(&self, context: &VisualizationContext<'_>) -> Result<VisualizationDocument> {
        let source = VisualizationSource {
            id: context.source_id.unwrap_or("example:live").to_string(),
            kind: "example".into(),
            label: "Example source".into(),
            source_type: "workspace".into(),
            live: false,
            metadata: Default::default(),
        };
        Ok(VisualizationDocument::empty("example", "Example", source))
    }
}

let mut registry = VisualizationRegistry::default();
registry.register(ExampleAdapter);
```

`VisualizationDocument` 是解析器与渲染器之间的唯一稳定边界，包含 `nodes`、`edges`、`series`、`events`、`frames`、`diagnostics` 和 `metadata`。适配器不得把私有解析器对象泄漏给前端。节点/边 ID 必须在同一文档内稳定；时间序列使用毫秒时间戳；无法解析的部分应生成 diagnostics，而不是伪造数据。

## 8. Atlas Core SDK

Atlas Core 提供存储无关的科学对象引擎：

```rust
use ai_assistant::atlas_core::{
    AtlasCore, RelationshipKind, ScientificObject,
};
use std::collections::BTreeMap;
use std::path::Path;

let core = AtlasCore::open(Path::new("/workspace"))?;
let mut object = ScientificObject::new("dataset", "Training data", "sdk.example");
object.description = "Curated training dataset".into();
object.metadata.insert("rows".into(), serde_json::json!(12000));
let created = core.create(object, "sdk.example")?;

core.relate(
    &created.id,
    "experiment-id",
    RelationshipKind::Uses,
    "sdk.example",
    BTreeMap::new(),
)?;
```

主要操作：`create`、`sync_external`、`get`、`list`、`update`、`archive`、`delete`、`clone_object`、`fork`、`merge`、`export`、`serialize`、`deserialize`、`preview`、`visualize`、`rollback`、`compare`、`relate`、`graph`、`timeline`、`record_event`、`search`。

持久化实现 `ObjectStore` 即可替换文件存储。写入必须满足：

- head 指向最新对象状态；
- 每次变更追加不可变 revision；
- 关系写入和删除具有稳定 ID；
- 事件只追加不覆盖；
- 对象 ID 和关系 ID必须经过路径安全验证；
- 持久化应采用临时文件加原子替换，避免部分 JSON。

## 9. Research Intelligence Engine SDK

```rust
use ai_assistant::research_intelligence::{
    ResearchGoalInput, ResearchIntelligenceEngine, RuntimeAdapter,
    RuntimeRequest, RuntimeResult,
};

let rie = ResearchIntelligenceEngine::open(Path::new("/workspace"))?;
let plan = rie.planning.plan(ResearchGoalInput {
    title: "Evaluate indexing latency".into(),
    description: "Compare baseline and candidate implementations".into(),
    domain: "systems".into(),
    constraints: serde_json::json!({"repetitions": 10}),
    target_publication: None,
    related_object_ids: Vec::new(),
}, "sdk.example")?;
```

`RuntimeAdapter` 声明运行时对象 ID、能力集合、可用性和执行函数。`RuntimeRegistry::select()` 选择满足请求能力子集且额外能力最少的可用运行时。注册运行时时，RIE 同步创建 `runtime` 科学对象。

RIE 插件实现 `Plugin + EventListener`，通过 `PluginManifest` 声明类别、能力、依赖、运行时、对象类型、可视化、命令和权限。生命周期：

```text
install -> enable <-> disable -> unload -> remove
                    \-> hot_reload (disable + enable)
```

插件贡献通过 `PluginContributions` 发布。生命周期函数失败必须保持可诊断错误；插件不得在 `manifest()` 或 `contributions()` 中执行高成本或有副作用的工作。

## 10. MCP SDK

### 10.1 连接外部 MCP 服务

`McpClientManager` 管理发现的服务和工具：

```rust
use ai_assistant::mcp::client::McpClientManager;

let mut manager = McpClientManager::new();
manager.initialize(Some(Path::new("mcp.json"))).await?;
let tools = manager.client().get_all_tools();
```

服务描述包含名称、命令、参数、环境变量、启用状态和描述。环境变量可包含 `${VAR}` 引用，必须在启动子进程前解析；不得把解析后的秘密写入日志或错误正文。

### 10.2 作为 MCP 服务运行

```sh
cargo run --release -- --mcp
```

生产环境应设置 `mcp_auth_required=true` 并通过 `MCP_API_KEY` 或安全配置提供密钥。MCP 采用 stdio 传输；危险等级工具受 `authorize_tool_call(ExecutionMode::Mcp)` 限制，不能仅依赖客户端自律。

## 11. HTTP API

HTTP API 与桌面桥复用相同后端状态。主要端点：

- `/api/bootstrap`、`/api/settings`
- `/api/send`、`/api/send-stream`、`/api/send-stop`、`/api/schedule`
- `/api/workspace/file*`、`/api/workspace/index*`
- `/api/visualizations*`
- `/api/research-domains*`、`/api/research-os/*`
- `/api/tasks*`、`/api/sessions*`
- `/api/git*`、`/api/run-debug*`、`/api/terminals*`
- `/api/ssh/*`、`/api/notebooks*`
- `/api/event-center*`、`/api/workspace-snapshots*`

统一非流式响应：

```json
{"ok": true, "data": {}}
```

错误使用非 2xx 状态或桥响应中的 `ok=false`/`error`。流使用 `application/x-ndjson; charset=utf-8`，并设置 `Cache-Control: no-cache` 与 `X-Accel-Buffering: no`。插件前端不应直接依赖环回端口；桌面环境优先使用原生桥。

## 12. 安全模型

所有扩展都必须遵守以下边界：

- 文件访问限定在 `SecurityConfig.allowed_roots`，默认不跟随符号链接。
- 规范化路径后再进行前缀检查，禁止 `..`、设备路径和路径穿越。
- 写入前检查文件大小、路径深度和工具风险。
- 外部命令隐藏窗口，使用结构化参数并记录退出码、stdout 和 stderr。
- API key 只允许来自环境变量或本地忽略配置；不得进入 workspace、事件、会话或工具结果。
- MCP、CLI、TUI 和 Autonomous 使用不同授权策略；不要绕过 `authorize_tool_call`。
- 网络、文件写入、命令执行和 Git 推送属于有副作用操作，必须具备显式授权和可审计结果。
- 插件输出是不可信数据；前端渲染必须转义 HTML，URL 必须验证 scheme。

## 13. 错误、并发和可恢复性

- 公共 Rust API 使用 `anyhow::Result` 或具体错误类型；错误应包含动作和目标，但不能包含凭据。
- Agent 工作任务必须由监督任务等待；异常取消、panic 和静默返回必须转换为 `error` 终态。
- 用户停止属于正常终态，应保存部分文本、推理、工具轨迹和文件差异。
- 工具执行需要幂等键或 `call_id`；重试不得重复产生外部副作用。
- 注册表使用锁保护共享状态，调用插件代码时避免长期持有写锁。
- 大目录发现必须限制深度、条目数、单文件大小并跳过缓存/构建目录。
- 持久化文件采用原子替换；读取损坏文件时应隔离单项并继续加载其他项。

## 14. 测试与发布检查

最低验证集：

```powershell
cargo check --lib
cargo check --features desktop-shell --bin desktop_wry
cargo test --lib
node --check frontend/app.js
node tools/test_desktop_stream_integrity.mjs
node tools/test_native_ui_wiring.mjs
```

扩展测试建议：

- 工具：schema 校验、缺参、风险、权限拒绝、超时、幂等和语义失败。
- 研究域：manifest 反序列化、资产发现上限、稳定 ID、状态迁移、产物验证。
- 可视化：schema 版本、引用完整性、空数据、超大数据、diagnostics。
- Core：revision 单调性、rollback、关系双向投影、原子写、损坏恢复。
- 宿主桥：所有命令 parse/as_str 往返一致，流恰好一个终态，关闭/停止无悬挂任务。
- 安全：秘密扫描、路径穿越、符号链接、命令注入、MCP 无密钥启动拒绝。

发布前禁止提交：`.env`、`web-runtime.json`、`desktop-session.json`、`.atlas/`、下载、沙箱、日志、截图、`target*`、`output/`、真实数据凭据和本地 SSH 配置。

## 15. 扩展提交清单

1. 选择正确扩展层，不在领域插件中复制 Core 或宿主功能。
2. 声明稳定 ID、版本、能力、风险和权限。
3. 使用统一 schema/模型，不向前端泄漏私有解析类型。
4. 对输入、路径、URL、命令和产物进行验证。
5. 为失败、取消、超时和重试提供确定行为。
6. 为生命周期、持久化和并发补充测试。
7. 更新本 SDK 文档或相应专题文档。
8. 执行秘密扫描、`git diff --check` 和目标平台构建。

源码入口：

- `src/host.rs`, `src/desktop_host.rs`, `src/web.rs`
- `src/tool_matrix/`
- `src/research_domains/`
- `src/visualization/`
- `src/atlas_core/`
- `src/research_intelligence/`
- `src/mcp/`
- `src/security.rs`, `src/app_paths.rs`
