# Atlas 项目技术书

> 适用版本：当前仓库实现
>
> 读者：产品、架构、客户端、后端、Agent、插件和运维开发者
>
> 维护原则：本文以源码为准；未实现的能力必须标为“扩展方向”，不能当作现有功能承诺。

## 1. 项目定位

Atlas 是面向软件工程与计算机科学研究的 Agent-native 桌面 IDE。它把代码工作区、对话与工具调用、研究证据、实验任务、知识库、可视化、论文产出和安全治理放在同一套本地优先的运行时中。

系统的核心目标不是“让模型聊天”，而是让 Agent 在清晰的权限、持久化和验证边界内完成可追溯工作：

- 开发任务：检查工作区、编辑文件、运行验证、展示差异、保留会话和任务状态；
- 研究任务：记录问题、证据、假设、实验、负结果、决策和时间线；
- 知识任务：上传多格式资料、做语义切块与混合检索，并管理版本、时效和归档；
- 长任务：规划、执行、修复、硬验证和断点恢复，必要时进入论文工作流；
- 桌面交互：通过 Wry/Tao 原生壳承载前端，同时复用 HTTP 路由与桌面桥接协议。

当前主要实现语言为 Rust，前端为原生 HTML/CSS/JavaScript。运行时支持 Web、桌面、CLI、TUI 和 MCP 入口。

## 2. 总体架构

```text
┌─────────────────────────────────────────────────────────────────┐
│ Frontend: Chat / Agent / Editor / Knowledge / Research / Settings│
└───────────────────┬───────────────────────────────┬─────────────┘
                    │ HTTP NDJSON                   │ Desktop IPC
┌───────────────────▼───────────────────────────────▼─────────────┐
│ Web Runtime (src/web.rs) / DesktopHostRuntime                    │
│ sessions · streaming · approvals · scheduler · bridge adapters   │
└───────┬───────────┬──────────────┬───────────────┬──────────────┘
        │           │              │               │
┌───────▼───┐ ┌─────▼──────┐ ┌─────▼────────┐ ┌───▼─────────────┐
│ LLM       │ │ Tool layer │ │ Knowledge    │ │ Research system │
│ providers │ │ governance │ │ + project idx│ │ Core / RIE / OS │
└───────────┘ └─────┬──────┘ └──────────────┘ └─────────────────┘
                    │
        ┌───────────▼───────────────────────────────────────────┐
        │ workspace · .atlas · Git/SSH · MCP client · plugins   │
        └───────────────────────────────┬────────────────────────┘
                                        │ Streamable HTTP
                              ┌─────────▼─────────┐
                              │ Remote MCP servers│
                              └───────────────────┘
```

关键代码入口：

| 层 | 主要位置 | 责任 |
| --- | --- | --- |
| 应用与多入口 | `src/main.rs`、`src/lib.rs` | CLI/TUI/Web/MCP 分派、模块导出 |
| Web/Agent 运行时 | `src/web.rs` | API、会话、流式回合、审批、验证、长任务 |
| 桌面宿主 | `src/bin/desktop_wry.rs`、`src/desktop_host.rs`、`src/host.rs` | Wry/Tao 窗口、IPC、桌面流转发 |
| 前端 | `frontend/index.html`、`frontend/app.js`、`frontend/*.js` | IDE 布局、会话、流、知识库、研究工作台 |
| 工具安全 | `src/tool_governance.rs`、`src/security.rs` | 风险、参数约束、并发类别、审批和限流 |
| MCP 客户端 | `src/mcp/client.rs`、`src/mcp/mod.rs` | 外部服务器配置、协议握手、工具发现和调用 |
| 代码索引/RAG | `src/project_index.rs`、`src/knowledge_base.rs` | 增量索引、文档解析、切块与混合检索 |
| 科学对象/RIE | `src/atlas_core/`、`src/research_intelligence/` | 对象版本、关系、执行 DAG、查询和推荐 |
| Research OS | `src/research_os/` | 假设、证据、实验、记忆、论文、时间线 |

## 3. 宿主、传输与生命周期

### 3.1 Web 与桌面共用运行时

`build_web_router` 在 `src/web.rs` 集中定义 `/api/*` 路由。Web 模式由 Axum 提供静态前端和 API；桌面模式仍在环回地址启动同一套 Web 服务，但前端通过 `window.__ATLAS_DESKTOP_BRIDGE__` 调用原生桥接，避免 UI 为不同宿主分叉。

桌面端由 `desktop_wry` 构建：

1. 确定工作区并设置 `ATLAS_WORKSPACE_ROOT`；
2. 创建每工作区的 `AppPaths` 与 `DesktopHostRuntime`；
3. 启动本地 Axum listener；
4. 创建无系统边框的 Tao 窗口与 Wry WebView；
5. 注入 `__ATLAS_HOST__`、同步调用桥和流桥；
6. 通过事件循环向 WebView 批量推送流式事件。

`HostDescriptor` 声明宿主模式、传输方式和能力。`atlas-host-v1` 是当前桥接协议标识；所有非流响应使用 `HostBridgeResponse { ok, status, data, error, protocol }`。

### 3.2 桌面桥接命令

命令名称以 `HostCommand` 为单一权威来源，前端 `BRIDGE_COMMANDS` 与之保持一一对应。典型分组如下：

| 分组 | 示例 |
| --- | --- |
| 启动与设置 | `bootstrap.load`、`settings.update`；`ollama.models` 仅保留为兼容/诊断接口，不是主模型选择入口 |
| 文件与索引 | `workspace.file.open`、`workspace.file.save`、`workspace.index.search` |
| 知识库 | `knowledge_base.state`、`knowledge_base.upload`、`knowledge_base.search`、`knowledge_base.govern` |
| Agent | `chat.send`、`chat.stream`、`chat.stop`、`schedule.manage` |
| 审批 | `tool.approval.approve`、`tool.approval.deny` |
| 任务与研究 | `tasks.*`、`research.paper_workflow.run`、`reviewer_feedback.*` |
| IDE 扩展 | Git、终端、运行调试、SSH、可视化、会话 |

新增宿主能力时必须同时更新 Rust `HostCommand`、桌面分发、前端常量、桥接类型声明和至少一条 wiring test。

MCP 设置和连接测试复用 `/api/mcp`、`/api/mcp/test`；桌面端经现有 `native.request` 转发这些 HTTP API，不为每个 MCP 操作新增 `HostCommand`。

### 3.3 流式协议与终态

聊天流通过 HTTP NDJSON 或桌面桥的异步事件传输。事件至少带 `type`，通常还带 `session_id`。主要事件包括：

- `assistant_delta`：正文增量；
- `thinking_delta`：可展示的思考增量；
- `activity` / `assistant_progress`：阶段进度；
- `tool`：按 `call_id` 关联工具开始、完成、失败和审批；
- `edited_files`、`subagent`、`verifier`：结构化执行证据；
- `complete` 或 `error`：唯一合法终态。

客户端不得把连接关闭当成成功。每个流必须以 `complete` 或 `error` 结束；无法生成可接受最终回答时，服务端以 `[completion-gate]` 错误持久化，而不是伪造成功消息。

## 4. 前端与交互设计

前端主体位于 `frontend/index.html`、`app.js`、`styles.css` 与 `professional-overrides.css`。其职责包括：

- 聊天/Agent 模式、会话、消息流、工具卡片、差异和审批；
- Monaco 编辑器、工作区树、Git、终端、运行调试和远程 SSH；
- 知识库上传/检索/治理、Research OS、领域工作台、Notebook、快照和事件中心；
- RAG 开关、个性化和 Agent 总结记忆展示；
- 模型、权限、子 Agent 上下文、长任务与自动论文设置。

### 4.1 Agent 斜杠命令

斜杠命令仅在 Agent 模式启用。输入 `/` 弹出紧凑命令浮层，支持上下键、Tab、Enter 和 Esc。已实现命令：

| 命令 | 作用 |
| --- | --- |
| `/goal <目标>` | 严格持续执行，直到硬验证或真实阻塞 |
| `/plan <任务>` | 先生成可验证计划后执行 |
| `/review <范围>` | 代码/变更/研究证据审查 |
| `/status`、`/compact`、`/resume` | 状态、上下文压缩、恢复未完任务 |
| `/spec <课题>` | 进入严格研究工作流 |
| `/schedule <规则> <任务>` | 创建后台计划任务 |
| `/model`、`/permissions` | 打开相应设置页 |
| `/new`、`/help` | 新会话与命令帮助 |

聊天模式中使用斜杠命令会得到“仅 Agent 模式可用”的明确提示，不会静默转成普通对话。

## 5. Agent 运行时

### 5.1 回合执行策略

运行时把回合分为 `Direct`、`Adaptive`、`Strict`：

- `Direct`：轻量会话，较低轮次和修复预算；
- `Adaptive`：普通 Agent 任务，允许有限规划和修复；
- `Strict`：`/goal`、`/plan`、研究任务等，使用结构化工作流、硬验证和更高修复预算。

严格模式先产生 2–6 步的计划，包含目标、步骤、委派、验收项、修复策略和用户明确要求的路径。系统把该计划写入消息与运行时状态；后续回合依据工具结果调整，但不得丢失原目标。

### 5.2 长任务、停滞恢复与完成门

`RuntimeSettings` 持久化 `long_task_enabled`、`max_autonomous_rounds` 和 `auto_generate_paper`。最大自主轮数被约束在 16–360；严格研究计划会获得更高的基础上限，但仍受用户配置限制。

每个有意义的回合都持久化消息、工具结果、差异和验证证据。停滞计数在 3、6、9 个无进展回合分别触发更换查询/工具、隔离阻塞和安全退出策略。目标未达成时，系统不能仅以“下一步”或“请继续”作为成功终结。

完成前会构建 hard verifier report，检查目标路径、工具证据、差异、测试/运行结果、研究资料和工作流领域要求。严格模式允许有限修复再验；通过或经不同安全路径确认真实阻塞后才允许结束。

### 5.3 `/goal` 持续执行契约

`/goal` 使用严格策略，并追加不可绕过的运行时契约：

- 确定性验证未通过前，不得将计划或阶段进展伪装为完成；
- 相同成功的只读工具调用复用已有结果；
- 同一失败调用没有新增证据时，下一次原样重试会被拦截；
- 无进展时必须改变参数、工具或证据路径，并推进独立步骤；
- 高风险调用不能因目标而获得额外权限；
- 同一 `/goal` 回合中相同高风险调用不能重复，且总预算至多三次；
- 被拒绝、限流或连续失败后，停止该高风险动作类别，改用安全替代或报告准确阻塞。

此设计让“持续工作”有明确边界：持续的是对可验证目标的推进，而不是无限循环、重复报错或批量危险操作。

### 5.4 并行、串行与级联中止

只有独立且原生安全的只读工具可以并发，例如工作区概览、工作区索引检索、知识库检索、Research OS 快照和部分领域上下文查询。并行结果会按模型原始调用顺序重新组装，确保会话证据可复现。

写入、终端、外部副作用、审批工具和 fallback CLI assistant 工具始终串行，防止顺序冲突和审批语义歧义。用户停止时，运行时会：

1. 取消会话中的同步模型 Tokio task；
2. 标记流已取消，拒绝未决审批；
3. 保存已产生的进度、工具和部分消息；
4. 中止父 worker；
5. 丢弃其 `JoinSet` 子任务，完成对子 Agent 的级联取消。

## 6. 模型接入与模型治理

模型层位于 `src/llm/`，主链路使用 Qwen Cloud 的 OpenAI-compatible Chat Completions API 与 SSE 流。当前 Qwen API URL 由后端固定；用户在设置页配置 API Key、主模型、深度思考和推理强度。LLM Provider 的网络客户端设有连接超时、总超时、TCP keepalive 与空闲连接边界；流式实现必须收到 `[DONE]` 或 `finish_reason`，没有终止标记的断流视为错误。

### 6.1 主模型与审查模型隔离

Atlas 把执行模型和审查模型视为两个职责不同的身份：

| 角色 | 约束 | 当前值/范围 |
| --- | --- | --- |
| 主模型 | 只能从后端 allowlist 选择，不得使用审查模型 | `qwen3.7-plus`、`qwen3.7-max`、`qwen3.6-plus`、`qwen3.6-max-preview`、`qwen3.6-flash` |
| 审查模型 | 运行时固定、前端只读，不得随主模型设置变化 | `qwen3-max` |

主模型的最低代际是 Qwen 3.6。后端 `validate_primary_model` 拒绝空值、`qwen3-max`、Qwen 3.5、QwQ 以及所有 allowlist 之外的标识；该校验位于设置 API 边界，不能仅依赖 HTML 下拉框。读取旧持久化状态时，非法模型会迁移到合法默认值 `qwen3.7-plus`。

审查流程会独立构造 provider，并在普通审查调用和 `llm_generated` 子 Agent 上下文生成中都显式固定 `qwen3-max`，避免主模型配置渗入审查身份。两类模型共用同一 Qwen API URL 与 API Key，并继承一致的深度思考、推理强度、上下文隐私和任务安全约束；只有 provider/model identity 按角色隔离。

`bootstrap.load` 返回当前 `model`、`review_model`、`primary_model_minimum` 和 `primary_model_ids`，是前端展示、持久化迁移和运行时约束之间的一致信息源。增加、下线或重命名模型时，应先修改后端策略，再同步 bootstrap DTO、设置 UI 与测试，不能让前端自行推断可用模型。

### 6.2 路由、凭证与扩展边界

`ModelRouter` 提供成本、质量、延迟和均衡策略的可扩展选型框架，并记录成功率与平均延迟；它是基础设施，不绕过上述主模型 allowlist，也不改变固定审查模型。当前设置页不把本地 Ollama 作为主模型入口，遗留 `ollama.models` 只用于兼容和诊断。

API Key 只保存在本机状态并用于供应商请求。不得把它写入工作区、日志、聊天消息、子 Agent 上下文或 MCP 配置以外的字段；错误信息也不得回显完整凭证。

## 7. 工具系统与安全治理

### 7.1 工具定义

`tool_governance::enrich_definitions` 会把以下信息注入每个工具定义：执行边界、效率提示、示例参数和并发类别。这既帮助模型正确选工具，也让插件/SDK 使用者可读取一致治理元数据。

示例：

```json
{"query":"StreamSessionRuntime cancellation","limit":10,"kind":"code"}
```

应优先用于 `search_workspace_index`，而不是先递归扫描整个目录。

```json
{"command":"cargo test knowledge_base --lib","timeout_secs":120}
```

应作为聚焦验证，避免无关的环境全量输出。

### 7.2 风险与权限

工具风险分为 `Safe`、`Moderate`、`Low`，其中 `Low` 在当前命名中代表最高风险等级。默认安全策略只自动放行 `Safe`；写文件、一般命令、浏览器等通常属于 `Moderate`；删除、shell 执行、Git 写入/推送、SSH 执行、端口扫描和终止进程属于 `Low`。

安全层组合以下机制：

- workspace 与 allowed roots 路径边界；
- 禁止默认跟随符号链接、文件大小与路径深度限制；
- 交互审批与自主模式最高风险限制；
- 每分钟与突发工具调用限流；
- MCP 可选 API key 认证；
- 删除工具只能接收显式工作区相对路径，拒绝根目录、家目录、父路径、通配符和未解析变量；
- 远程、浏览器、Git 和终端的外部副作用必须在相应风险门后执行。

### 7.3 工具选择边界

工具调用应遵循“最小充分证据”原则：先复用消息、索引、已有结果和差异；路径未知时先 `inspect_path`；代码检索先查索引；只读检索可并行；变更前确认精确路径；验证命令保持短、小、可归因。禁止为了尝试工具而逐一扫描工具清单或重复执行已成功读取。

### 7.4 MCP 客户端与设置入口

Atlas 有两种方向相反的 MCP 能力，维护时不可混淆：命令行 `--mcp` 让 Atlas 自身作为 stdio MCP server 暴露能力；`src/mcp/client.rs` 则让 Agent 连接用户在“设置 → MCP”中配置的外部 MCP server。本节描述后者。

每个外部服务器配置包含稳定 ID、名称、描述、endpoint、transport、enabled 和可选 Bearer Token，随 `PersistedWebState.mcp_servers` 保存在本机。`GET /api/mcp` 读取当前客户端快照，服务器配置随设置更新接口保存，`POST /api/mcp/test` 测试连接；桌面端通过 `native.request` 复用相同 API。禁用的服务器不会参与发现，单个服务器连接失败也不应阻断应用 bootstrap。

当前客户端只接受 `streamable_http`/`http` 配置，执行以下协议序列：

1. 发送 `initialize`；
2. 发送 `notifications/initialized`；
3. 调用 `tools/list` 并把工具定义注入 Agent；
4. 在模型选用工具后调用 `tools/call`。

客户端保存并回传 `Mcp-Session-Id`，能够解析普通 JSON 和 SSE `data:` 响应。工具名在 Atlas 内统一映射为 `mcp__{server_id}__{tool}`，防止多个服务器同名碰撞，并在调用时反解回原服务器和原工具名。Bearer Token 只作为到对应 endpoint 的 Authorization header 使用，不进入模型上下文或日志。

外部 MCP 工具必须经过现有工具治理。尚未建立更精确规则的 MCP 工具按外部可变副作用处理，保持串行，并受权限、审批和失败审计约束。当前客户端不承诺 stdio 子进程传输、OAuth、resources 或 prompts；这些属于后续扩展方向。

## 8. 工作区索引、知识库与 RAG

### 8.1 项目索引

`src/project_index.rs` 构建工作区级增量索引，持久化为 `.atlas/index/manifest.json`。目录遍历会跳过 `.git`、`.atlas`、`node_modules`、`target`、虚拟环境和构建产物；解析 PDF、Office 文档和源码时使用 Rayon 并行处理独立文件。

索引按文件大小与修改时间复用未变条目，支持代码、文档、PDF、DOCX、PPTX、XLSX、CSV/TSV 等格式。检索采用路径命中加内容词频评分，适合定位代码、符号和局部资料。

### 8.2 知识库数据模型

知识库位于 `.atlas/knowledge-base/`：

```text
.atlas/knowledge-base/
├── manifest.json
└── sources/
    └── kb_<hash>-v<version>.<extension>
```

一个 `KnowledgeDocument` 保存逻辑名称、内容哈希、版本、状态、所有者、标签、有效期、校验时间、前代版本与切块。`KnowledgeChunk` 保存源位置、标题路径、实体、token 估算、文本和语义向量。

支持格式包括 PDF、DOCX、PPTX、XLSX、CSV/TSV、Markdown/文本、HTML/XML、JSON/YAML/TOML、TeX/BibTeX/SQL，以及 Rust/Python/JavaScript/TypeScript 等常见源码。单文档上限为 64 MiB。

同逻辑名称上传不同内容时，会生成新版本并把上一版本标为 `Archived`；相同内容哈希上传是幂等的。知识库没有 Agent 可调用的物理删除操作。

### 8.3 语义切块与结构化索引

切块过程保留 Markdown 标题层级、段落边界和句子边界：

- 目标块长约 1,200 字符，最大 1,800 字符；
- 相邻块保留 160 字符重叠；
- 超长段落优先按中英文句末标点拆分；
- 每块写入 `location`、`heading_path`、实体、token 数与 ordinal。

这是一种“结构保留的语义切块”：模型可引用文档名、版本、位置和标题路径，而非只有无来源的纯文本片段。

当前本地语义向量采用 192 维确定性 feature hashing + 归一化余弦相似度，不会为嵌入把文档发送到第三方。该实现是可替换的本地基线，并非声明使用了外部神经嵌入模型。

### 8.4 混合检索与时效治理

每次查询会在 Active 与 Stale 文档中生成候选：

1. BM25 计算词法相关性，覆盖精确术语、文档名和标题；
2. 哈希语义向量计算余弦相似度，覆盖部分措辞变化；
3. 对两个排序做 reciprocal-rank fusion；
4. 加入 freshness score。

当前融合权重为词法 0.48、语义 0.42、时效 0.10（RRF 常数 60）。90 天未验证的资料自动为 `Stale` 并降低时效；`Archived` 和超过 `valid_until` 的 `Expired` 文档完全不参与召回。

治理操作仅限：

- `archive`：从检索移除，但保留版本与文件；
- `restore`：恢复未过期版本；
- `verify`：刷新校验时间并使 Stale 资料回到 Active；
- `metadata`：更新 owner、tags、有效起止时间。

### 8.5 RAG 与记忆

RAG 是逐回合显式选择：前端 composer 的 `RAG` 开关只在本轮个性化载荷中设置 `rag_enabled`。开启后，系统将至多八条相关 Active/Stale 证据注入系统提示，并要求模型引用文档和位置。

用户显式记忆同样通过 `rank_memory_texts` 进行词法/语义混合排序，避免把全部记忆无差别塞进上下文。Research OS 的长期研究记忆独立保存并在个性化页面以 Markdown 渲染，可显示重要度和调用次数。

## 9. 子 Agent 信息传递

子 Agent 不接收裸对话历史，而接收版本化对象 `atlas.subagent-context.v1`。其字段包含任务、共享事实、最近对话、相关工具结果和已执行的隐私处理。

| 模式 | 传递内容 | 适用场景 |
| --- | --- | --- |
| `minimal` | 仅调用参数/任务 | 隐私最优先，允许信息不足 |
| `manual` | 任务 + 用户明确填写的事实 | 业务规则、政策摘要、人工选择上下文 |
| `automatic` | 最多 1–10 轮对话窗口及最近工具/差异 | 默认平衡模式 |
| `llm_generated` | 先构建经脱敏自动上下文，再额外调用 LLM 压缩为同一 schema | 复杂、长任务的结构化委派 |

所有模式均尝试脱敏密码、token、授权字段、支付信息、私钥和疑似长凭据；`llm_generated` 不允许读取原始完整历史。Planner、reviewer、verifier 和 repairer 应消费该对象，而不是自行假设主 Agent 仍有历史上下文。

配置在设置页持久化，包括手工上下文（上限 12,000 字符）、最近轮数（1–10）和隐私规则（上限 4,000 字符）。

## 10. Atlas Core、RIE 与 Research OS

### 10.1 Atlas Core

Atlas Core 是对科研与工程对象的通用版本化层，目录为 `.atlas/core/`：

```text
.atlas/core/
├── objects/        # 当前 head
├── history/<id>/   # 不可变编号 revision
├── relationships/  # 关系实体
└── events/         # 事件流
```

`ScientificObject` 包含标识、类型、生命周期、标签、工件、运行时绑定、预览、可视化映射、证据链接、Agent 上下文、权限和搜索索引。对象更新先保存不可变 revision，再更新 head；回滚创建新的 head revision，不能抹除较晚历史。关系单次存储并在两端投影；图是关系投影，不维护第二份图数据。

`FileObjectStore` 使用原子临时文件写入，并对对象记录限制 4 MiB。对象类型、ID、权限和关系由核心层统一校验，领域插件不应绕过它。

### 10.2 Research Intelligence Engine

RIE 位于 `src/research_intelligence/`，是无 UI 的编排层：

- `PlanningEngine`：研究目标转为版本化计划与执行 DAG；
- `ExecutionEngine`：选择有能力的 runtime adapter、记录 observation 和失败分析；
- `RecommendationEngine`：生成带证据的建议；
- `ObjectQueryEngine`：结构过滤、自然语言对象检索和 AOQL；
- `PluginRegistry`：插件安装、启用、禁用、热重载和卸载生命周期。

兼容 API 会把旧面板产生的领域数据同步进 Core。新能力优先使用 `atlas_object`、`atlas_object_query`、`atlas_research_plan`、`atlas_recommend` 等对象工具；直接文件工具保留为工件实现和兼容层。

### 10.3 Research OS

Research OS 将研究过程拆成持久化对象：假设、证据、实验、负结果、日记、知识图节点/边、决策、记忆、时间线和论文草稿。对象保存在 `.atlas/research-os/<type>/<id>.json`，单对象上限 512 KiB，单类型最多枚举 10,000 项。

Research Memory 记录内容、重要度、关联对象、可选 embedding、调用次数与最后访问时间。Agent 回合及领域任务可被摄取为研究证据；UI 的 Research OS snapshot、graph、decisions、memory、evidence、experiments 等 API 提供可视化和检索入口。

### 10.4 领域插件

`ResearchDomainRegistry` 提供声明式和程序化领域插件。内置领域覆盖 AI/ML、计算机视觉、NLP、图形、CAD、机器人、网络、操作系统、编译器、数据库、软件工程、程序分析、网络安全、HPC、分布式系统和科学计算。

插件描述领域元数据、可发现工件、工作台、执行上下文、Agent 上下文和可视化。扩展应实现 `IDomainPlugin` 及各 provider trait，并遵守 `atlas.research-domain.v1` 的 schema/API 版本；领域专有校验放在插件，版本、权限、关系和搜索仍由 Core 继承。

### 10.5 科研对象之间的关系

科研对象不是一组互相独立的 JSON。推荐的最小关系链是：

```text
ResearchGoal
  ├─ research-question
  ├─ hypothesis
  │    ├─ evidence (supports / refutes)
  │    └─ experiment
  │          ├─ execution-task
  │          │    └─ runtime / artifact / metrics
  │          └─ negative-result
  ├─ dataset-requirement
  ├─ method
  ├─ risk-analysis
  ├─ expected-result
  └─ publication
```

`ResearchGoalInput` 包含标题、描述、领域、约束、目标投稿 venue 和关联对象。`PlanningEngine` 会为一个目标生成九类计划节点：研究问题、假设、论文分析、数据集选择、方法、风险分析、执行图、预期结果和投稿结构；节点通过 `DependsOn` 关系形成可查询的 DAG。

关系的用途不是装饰 UI，而是支持以下操作：从论文追溯实验和证据；从失败实验回看假设；从一个 baseline fork 出变体；比较不同计划版本；在生成论文时只引用有来源的结果。任何自动总结都应保留源对象 ID，避免把“模型推测”写成“实验事实”。

## 11. 科研与实验执行体系

### 11.1 研究任务的输入契约

进入 Research/`/spec`/严格 Agent 工作流时，系统要求先定义：

1. 可证伪研究问题和假设；
2. 主要指标、次要指标和停止条件；
3. 数据集或 benchmark 的官方来源、版本/哈希、许可、任务定义和固定切分；
4. baseline、当前方法、消融变量和比较口径；
5. 环境、依赖、硬件、预算、随机种子和复现命令；
6. 预期产物、证据类型和失败处理策略。

这些条件由研究执行契约提示、计划 schema 和 hard verifier 共同约束。模型可以提出计划，但不能凭计划内容宣称实验已经运行。

### 11.2 研究计划与执行 DAG

RIE 的计划对象和执行对象分层：计划说明“要研究什么以及为什么”，执行任务说明“用什么能力、参数和产物完成哪一步”。`ExecutionTaskSpec` 的关键字段包括：

| 字段 | 含义 |
| --- | --- |
| `title` / `goal` | 任务名称和可验证目标 |
| `dependencies` | 必须先完成的 Core 对象 ID |
| `scientific_object_ids` | 使用的假设、数据集、方法或其他科学对象 |
| `required_capabilities` | 运行时必须提供的能力集合，如 `python`、`cuda`、`latex` |
| `expected_output_types` | 预期 metrics、artifact、evidence 等类型 |
| `metrics` | 指标名称、阈值和记录口径 |
| `parameters` | seed、split、超参、路径和运行选项 |

执行前，`ExecutionEngine` 会检查依赖对象是否为 `Completed`，再从 `RuntimeRegistry` 选择“可用且能力集合覆盖任务需求”的 adapter。选择时优先能力最小的可用 runtime，减少不必要的环境复杂度。

执行状态为 `Planned → Queued → Running → Completed/Failed/Blocked/Cancelled`，暂停可转回恢复。每次运行把 runtime ID、参数、指标、工件路径和 failure analysis 写回任务对象，并记录 `ExecutionStarted`/`ExecutionFinished` 事件。

### 11.3 Runtime Adapter 设计

科研运行时通过 `RuntimeAdapter` 解耦：

```rust
trait RuntimeAdapter {
    fn runtime_object_id(&self) -> &str;
    fn capabilities(&self) -> BTreeSet<String>;
    fn available(&self) -> bool;
    fn execute(&self, request: &RuntimeRequest) -> Result<RuntimeResult>;
}
```

Adapter 只负责在已授权、已选定的运行环境中执行；它不能自行扩大工作区路径、访问未声明凭据或绕过工具审批。`RuntimeResult` 必须返回成功标志、摘要、结构化 metrics、artifact paths 和必要的原始结果。运行时不可用、能力不匹配或输出不完整时，应返回失败/阻塞，而不是空成功。

### 11.4 数据集、manifest 与防泄漏

数据驱动研究必须先发现官方入口，再冻结 manifest。manifest 至少应记录：provider、source URL、retrieval entrypoint、版本或内容哈希、license、task hint、下载/缓存路径、字段说明和 split。实验运行只能引用冻结 manifest，不能看到结果后悄悄换数据。

split manifest 应固定 train/validation/test 或等价划分，并记录随机种子、划分算法、过滤规则和去重策略。验证阶段重点检查：训练集与测试集重复、预处理使用测试标签、时间泄漏、跨用户/跨实体泄漏和 baseline 使用了不公平额外信息。

### 11.5 环境与可复现性

首次运行前保存 environment manifest：操作系统、CPU/GPU/内存、Python/Rust/Node 版本、依赖锁文件、CUDA/驱动（适用时）、数据 manifest、随机种子、Git revision 和精确命令。产物包应至少包含：

- 可执行命令或脚本入口；
- 运行参数和配置快照；
- stdout/stderr 日志；
- metrics JSON/CSV；
- checkpoint 或模型工件路径；
- 运行时间、退出码和资源信息；
- 从干净环境重放所需的说明。

“可复现”在 Atlas 中意味着有足够证据重建运行，不等于任何机器都能获得字节级相同的浮点结果。报告应区分 deterministic seed、硬件差异和随机算法差异。

### 11.6 Baseline、变体与统计闭环

每个经验研究至少需要一个可解释 baseline 和一个当前方法/变体。适用时增加：

- 消融：移除单一组件，检验贡献归因；
- 多 seed：报告均值、标准差、置信区间或运行间范围；
- 参数敏感性：展示关键超参改变是否导致结论反转；
- 资源对照：报告延迟、吞吐、显存、CPU 时间或成本；
- 误差分析：抽样失败样本、按类别/场景分组并保存示例。

结果比较应携带 run ID、parent run ID、variant label、dataset manifest 和代码 revision。Research OS 的 experiment lineage 与论文工作流的 result bundle 都用这些信息避免把不同数据或不同 baseline 的结果拼在一张表里。

### 11.7 失败、负结果与修复

实验失败是研究证据，不是需要隐藏的日志。RIE 将失败分类为 timeout、runtime failure、memory overflow、low accuracy、bad convergence、runtime busy、invalid dataset、missing dependency 和 unknown，并给出 retryable 标志、建议策略和参数 patch。

修复必须满足三点：

1. 指出失败证据和根因假设；
2. 改变参数、环境、工具或执行路径，不能原样重复失败运行；
3. 保存修复前后的运行 ID、差异和复测结果。

不可修复或不应继续的结果写入 `NegativeResult`，并记录 failure mode、learned、配置、环境、数据集、checkpoint、运行信息、日志和相似失败分数。后续规划可检索历史负结果，避免同一错误反复消耗资源。

### 11.8 验证中心与领域验证包

`VerificationCenterTools` 先探测本地工具和平台，再按研究画像选择验证包。探测项包括 pytest、Jupyter、ruff、mypy、semgrep、Z3、MLflow、W&B、Git、DVC、hyperfine、memory_profiler、cProfile 和 Python 等；不可用工具必须标记 skipped，并提供替代说明。

验证包按画像覆盖不同硬约束：

| 画像 | 重点 |
| --- | --- |
| `classical_ml` / `deep_learning` | 数据 manifest、baseline、指标、误差分析、训练/推理可复现 |
| `systems_evaluation` | 环境、负载、吞吐/延迟、资源、重复运行和运行时摘要 |
| `agent_evaluation` | 任务集、模型/工具版本、成功率、成本、轨迹和安全失败 |
| `security_analysis` | 数据集/样本来源、威胁模型、复现实验、漏洞或攻击证据 |
| `theory` | 定义、证明、反例、形式化检查或可执行验证 |
| `literature_review` | 来源覆盖、引用、证据缺口和不可访问全文边界 |

验证中心的 score 只表示当前工具和证据覆盖度，不是论文质量分数。最终 hard verifier 仍需逐项检查目标路径、manifest、seed/split、baseline、真实运行、失败记录、指标和论文闭环。

### 11.9 研究回合的证据摄取

Agent 回合结束后，Research OS ingestion 可将目标、工具证据、文件差异、研究进度、验证结果和最终结论转为日记、证据、时间线或记忆。摄取时必须区分：

- 用户声明：需求、限制和研究目标；
- 工具事实：命令退出码、文件路径、模型返回和原始指标；
- Agent 推论：解释、风险和推荐；
- 人工确认：审稿意见、选择和批准。

这一区分使后续论文写作可以按证据强度引用，而不是把模型摘要直接升级成实验事实。

## 12. 论文工作流

论文工作流在 `src/scientist/workflow/paper_workflow.rs`，由 Research Agent、Hypothesis Agent、Experiment Agent、Verification Agent 与 Report Agent 协作。它针对可恢复产出设计，而不是一次性生成一段 Markdown。

阶段包括：文献、研究问题、假设、实验、运行时证据、初始验证、初稿、修订闭环、工件物化、PDF 编译与 paper-ready 评估。每阶段写入 `workflow_checkpoint.json`；相同 topic、会话、审稿意见和运行时证据指纹可恢复已完成阶段，`force_rewrite` 仅失效报告/修订之后的阶段。

论文阶段的输入优先级是：冻结的文献/知识证据、Research OS 假设和证据、实验 result bundle、run comparison、lineage、审稿意见和验证报告。缺少某项时，系统应产生缺口和恢复动作，而不是用占位文字填充结论。

初稿之后会构建章节 bundle 和 manuscript bundle，生成 before/after 快照与 diff。修订执行计划把 reviewer feedback 分解为可验证修改；最终 verifier 重新检查 claim anchors、引用、实验设置、结果表、局限性和复现信息。PDF 编译还依赖可用的 Tectonic 或 pdflatex/bibtex 工具链；没有工具时状态为 `missing_toolchain`，不能冒充已编译。

论文的 `paper_ready` 是门控结果，不是语言质量评分。只有 reviewer closure、hard verification、产物完整性和（若要求）PDF 编译状态都满足时，`auto_generate_paper` 才允许自动触发服务器端论文任务。

输出包含论文 Markdown/LaTex、BibTeX、附录、结果包、审稿回应、修订计划与轨迹、章节/手稿 diff、可选 PDF 和概念图。自动触发受研究闭环检查、硬验证、审稿反馈和 `auto_generate_paper` 设置共同约束；未达到 paper-ready gate 时不得声称论文已完成。

## 13. 任务、Notebook、SSH 与可观测性

### 12.1 后台任务

`TaskQueue` 保存在 `.atlas/tasks/queue.json`，每项包含状态、PID、尝试次数、日志、恢复策略和可选结果路径。启动时若发现运行中的 PID 已不存在，则标为 `Interrupted` 并要求恢复；任务日志在 `.atlas/tasks/<id>.log`。

计划任务使用 `schedule.manage`，支持 `in <n><s|m|h|d>`、`at YYYY-MM-DD HH:MM`、`daily HH:MM`、`list` 和 `cancel <id>`。应用关闭期间不执行，到期任务会在后续启动恢复调度。

### 12.2 Notebook 与工作区快照

Notebook 数据保存在 `.atlas/notebooks/`，当前内置 Markdown 与 Python 单元。Python 执行输出有大小上限，Notebook 同步为 Atlas Core 对象。工作区 Time Machine 保存在 `.atlas/snapshots/`，记录对象版本和 UI/工作区状态；创建、列出和比较为可逆操作，恢复仍必须走审批边界。

### 12.3 SSH

Remote SSH 配置位于 `.atlas/remote-ssh/`。远程命令必须先建立显式连接，并遵循 Agent 授权、known-host 和本机密钥/askpass 处理；不会把密码并入子 Agent 上下文。长训练应采用后台任务与受限输出，不能直接用无界远端 shell 代替工具治理。

## 14. 数据与状态目录

Atlas 将应用级状态与工作区状态分开：

| 范围 | 位置 | 主要内容 |
| --- | --- | --- |
| 应用/项目状态 | `AppData/Local/Atlas/projects/<project-id>/` | runtime 设置、会话、下载、WebView 数据、桌面窗口状态 |
| 工作区状态 | `<workspace>/.atlas/` | 索引、知识库、Core、Research OS、任务、Notebook、SSH、快照等 |
| 知识源 | `.atlas/knowledge-base/sources/` | 上传文件的版本化副本 |

`project_id` 由规范化工作区路径的 BLAKE3 摘要生成；Windows 上路径按大小写不敏感处理。会话迁移逻辑会检查工作区 ID，再合并旧项目状态，避免不同工作区串会话。

工作区 `.atlas/` 是运行时数据，不应纳入产品源码版本库，除非团队明确设计了可共享的研究工件策略。API key、token、SSH 密码、临时日志、截图和下载物不应提交。

## 15. API 与扩展约定

HTTP API 以 `/api/*` 提供，成功 JSON 通常为 `{ "ok": true, "data": ... }`。需要新增接口时：

1. 在 `build_web_router` 注册；
2. 定义严格的请求/响应 DTO，新增字段使用兼容默认值；
3. 若桌面需要，补充 `HostCommand`、桥接分发与前端客户端；
4. 绑定工作区、会话、路径和权限上下文；
5. 为写入/外部动作接入审批、速率限制、审计事件；
6. 添加单测或 wiring test。

插件、工具和领域协议有版本字段时，只能在升级对应主版本时做破坏性变更。Rust `pub` 类型是源码 API，不承诺稳定 C ABI；集成方应锁定 crate 版本与 `Cargo.lock`。

## 16. 构建、运行与发布

前置条件：Stable Rust/Cargo；可用的 Qwen API Key；Windows 需要 WebView2，Linux 需要 GTK3/WebKitGTK 开发包，macOS 需要 Xcode Command Line Tools。若启用外部工具，还需要可访问的 Streamable HTTP MCP endpoint 及其可选 Bearer Token。

```powershell
Copy-Item .env.example .env
cargo run --bin desktop_wry --features desktop-shell
```

常用验证：

```powershell
cargo check --lib
cargo test --lib
cargo build --release --bin desktop_wry --features desktop-shell
cargo test --lib mcp::client::tests::streamable_http_discovers_and_calls_tools
cargo test --lib web::tests::primary_model_policy_rejects_old_and_review_models
cargo test --lib web::tests::reviewer_completion_always_uses_fixed_model
node tools/test_agent_slash_commands.mjs
node tools/test_knowledge_rag_wiring.mjs
node tools/test_desktop_stream_integrity.mjs
node tools/test_chat_state_resilience.mjs
```

Windows 打包使用：

```powershell
./scripts/package-windows.ps1 -Version 0.1.0
```

CI 包括核心/集成/文档测试、前端 wiring test，以及 Windows/macOS/Linux 上的桌面编译检查。发布工作流构建 Windows portable ZIP 与 Inno Setup 安装包；公开发布前应配置代码签名，避免 SmartScreen 风险提示。

## 17. 测试策略

测试应分层执行：

| 层级 | 关注点 | 示例 |
| --- | --- | --- |
| Rust 单元测试 | 纯逻辑、序列化、安全边界 | RAG 状态、重复调用护栏、对象版本 |
| Rust 集成/E2E | 研究、论文、上下文与工作流闭环 | `tests/scientist_paper_workflow_e2e_test.rs` |
| Node wiring test | DOM、前端绑定、路由/协议接线 | `tools/test_knowledge_rag_wiring.mjs` |
| 流协议回归 | 终态、桌面 bridge、取消与状态恢复 | `test_desktop_stream_integrity.mjs`、`test_chat_state_resilience.mjs` |
| 跨平台 CI | 编译和原生 WebView 依赖 | `.github/workflows/desktop-platforms.yml` |

对 Agent 行为不应只测“模型是否说对”，还要测硬不变量：是否产生唯一终态、是否保存失败、是否拦截重复危险调用、是否拒绝不安全路径、是否在停止时取消子任务、是否将 RAG 限定为 Active/Stale 证据。

## 18. 运维与故障排查

| 现象 | 优先检查 |
| --- | --- |
| 桌面白屏或无法启动 | WebView2/GTK 依赖、`frontend/index.html`、本地 listener、桌面状态目录权限 |
| 模型无响应 | Qwen API Key、网络、供应商 endpoint 是否提供所选模型、流终态和 provider 错误 |
| 保存模型设置返回 400 | 主模型是否属于后端 Qwen 3.6+ allowlist；`qwen3-max` 是固定审查模型，不能选作主模型 |
| MCP 连接成功但没有工具 | enabled、endpoint、Bearer Token、`initialize`/`tools/list` 响应和网络可达性 |
| MCP 工具调用失败 | `mcp__{server_id}__{tool}` 命名、`Mcp-Session-Id`、服务器 `tools/call` 响应和审批状态 |
| RAG 没有证据 | RAG 开关、知识库状态、文件解析、Active/Stale 状态、查询词 |
| 文档未被召回 | 是否 Archived/Expired、有效期、标题/标签、切块文本、词法与语义候选阈值 |
| Agent 停止过早 | 当前执行策略、验证失败、轮次上限、审批状态、停滞报告 |
| 子 Agent 信息不足或泄露担忧 | `subagent_context_mode`、手工上下文、recent turns、privacy rules |
| 重启后任务异常 | `.atlas/tasks/queue.json`、任务 PID、日志和 `Interrupted` 状态 |

排障时优先读取状态和日志，不要手动删除 `.atlas`。对知识库优先 archive/restore/verify；对任务优先 cancel/retry；对工作区变更优先快照和 Git diff。

## 19. 已知边界与演进建议

以下内容是当前实现的真实边界，不应被营销表述掩盖：

- 知识库语义向量是本地确定性哈希基线；可替换为模型 embedding 或向量数据库，但替换必须保持版本、隐私和离线退化语义；
- 知识库是工作区级单机 manifest，不是多租户分布式知识服务；
- 严格 Agent 有轮次上限、审批和真实阻塞出口，不保证对所有开放问题“永不停止”；
- 论文工作流生成的是受证据 gate 约束的工件管线，最终研究有效性仍需要人类审阅；
- 主模型目录是服务器维护的静态 allowlist，不是对供应商模型的动态发现；固定审查模型 `qwen3-max` 依赖当前 Qwen endpoint 实际提供该模型；
- 外部 MCP 客户端当前只支持 Streamable HTTP，不支持 stdio 子进程、OAuth、resources 或 prompts；Atlas 自身的 `--mcp` stdio server 是另一项独立能力；
- 工具风险枚举需要持续随新工具更新，特别是新网络、文件、Git 和远端执行能力；
- 文档与 API 的中文编码应在编辑器、CI 和发布包中统一为 UTF-8，避免历史乱码影响维护。

建议的演进顺序：先强化可观测性和工具审计，再引入可配置嵌入后端与重排器；随后扩展团队级访问控制、共享知识库和插件签名。所有演进都必须保持“本地数据边界、可验证完成、可恢复状态、最小权限”的核心不变量。

## 20. 维护清单

当修改以下能力时，必须同步更新相应文档与测试：

- 新的 slash command：前端列表、解析、后端策略、帮助文案、命令 wiring test；
- 新工具：风险映射、`ToolBoundary`、参数 schema、示例、并发分类、审批/限流测试；
- 新知识格式：解析器、`supported_formats`、索引/上传测试、切块位置与治理语义；
- 新子 Agent 角色：上下文 schema 消费方式、取消传播和隐私测试；
- 模型策略变更：角色常量、主模型 allowlist、校验/迁移、DTO/bootstrap、设置 UI、provider 构造和隔离测试；
- MCP 变更：配置 DTO、持久化、协议序列、工具定义注入、调用分发、设置入口、凭证脱敏和协议测试；
- 新对象/领域：Core 同步、revision/relationship、RIE/plugin schema 与领域测试；
- 新桌面命令：`HostCommand`、IPC、前端桥接、流终态和跨平台编译检查；
- 新外部副作用：明确用户意图、审批门、审计事件、失败恢复和幂等策略。


相关参考：

- [Agent 运行时与治理](AGENT_RUNTIME_GOVERNANCE.md)
- [Atlas Core 与 RIE 架构](ATLAS_CORE_ARCHITECTURE.md)
- [SDK 参考](SDK.md)
- [桌面平台支持](DESKTOP_PLATFORMS.md)
- [Research OS 用户指南](../RESEARCH_OS_USER_GUIDE.md)
