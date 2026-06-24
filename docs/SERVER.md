# HTTP REST API Server 使用文档

> 适用于 `cargo run --features server -- --server ...`
>
> 该服务**只监听 `127.0.0.1`**，默认不对外网卡开放；如启用 `--api-key`，则所有接口都要求 `Authorization: Bearer <token>`。

## 1. 定位

HTTP Server 是对现有 CLI / TUI / MCP / 自主进化能力的一层 REST 封装，适合：

- 作为本地进程被其他程序调用
- 集成到 Web UI、脚本、桌面自动化流程
- 让浏览器插件或本地代理程序访问 Tokitai 能力

它不是云端公共服务，也不建议绑定到 `0.0.0.0`。

## 2. 启动方式

```bash
cargo run --features server -- --server --port 8080
```

### 常用参数

- `--server` / `-s`
  - 启动 HTTP REST API 模式
- `--port` / `-P <port>`
  - 指定监听端口，默认 `8080`
- `--api-key <token>`
  - 启用 Bearer Token 鉴权
  - 提供后，所有请求都要带 `Authorization: Bearer <token>`

### 示例

```bash
# 基本启动
cargo run --features server -- --server --port 8080

# 启用鉴权
cargo run --features server -- --server --port 8080 --api-key abc123
```

## 3. 鉴权

当启动时没有传 `--api-key`：

- 所有接口都允许本机访问
- 仍然只会绑定 `127.0.0.1`

当启动时传了 `--api-key`：

```bash
curl http://127.0.0.1:8080/v1/tools
# 401 Unauthorized

curl -H 'Authorization: Bearer abc123' http://127.0.0.1:8080/v1/tools
# 200 OK
```

### 错误响应

```json
{
  "error": {
    "code": "Unauthorized",
    "message": "Bearer token 不正确",
    "request_id": "6f4c9f2a-2a0f-4df4-a1d2-2e7ef8d6b0f7"
  }
}
```

## 4. 错误模型

所有 handler 都返回统一 JSON 错误信封：

```json
{
  "error": {
    "code": "BadRequest",
    "message": "请求参数无效：...",
    "request_id": "..."
  }
}
```

### 状态码映射

| 代码 | HTTP |
|---|---:|
| `BadRequest` | 400 |
| `Unauthorized` | 401 |
| `NotFound` | 404 |
| `Conflict` | 409 |
| `ToolError` | 422 |
| `LlmError` | 502 |
| `Internal` | 500 |

## 5. 快速健康检查

```bash
curl -sS http://127.0.0.1:8080/healthz
curl -sS http://127.0.0.1:8080/v1/ping
curl -sS http://127.0.0.1:8080/v1/version
```

示例返回：

```json
{"status":"ok","service":"ai-assistant-server"}
```

## 6. 路由总览

### 6.1 健康与版本

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/healthz` | 顶层探针 |
| GET | `/v1/ping` | 存活探测 |
| GET | `/v1/version` | 版本信息 |

### 6.2 Chat

| 方法 | 路径 | 说明 |
|---|---|---|
| POST | `/v1/chat` | 非流式聊天 |
| POST | `/v1/chat/stream` | SSE 流式聊天，仅当前 provider 支持时可用 |

#### `/v1/chat` 请求示例

```bash
curl -sS http://127.0.0.1:8080/v1/chat \
  -H 'content-type: application/json' \
  -d '{
    "messages": [
      {"role":"user","content":"帮我列出 src 下的 Rust 文件"}
    ]
  }'
```

#### `/v1/chat/stream` 请求示例

```bash
curl -N http://127.0.0.1:8080/v1/chat/stream \
  -H 'content-type: application/json' \
  -d '{
    "messages": [
      {"role":"user","content":"写一个 Rust 的 hello world"}
    ]
  }'
```

### 6.3 Tools

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/v1/tools` | 列出工具 |
| POST | `/v1/tools/call` | 调用工具 |

### 6.4 Providers / Models

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/v1/providers` | 列出 provider |
| GET/POST | `/v1/providers/current` | 查看 / 切换当前 provider |
| GET | `/v1/models` | 列出模型 |

### 6.5 Orchestrator

| 方法 | 路径 | 说明 |
|---|---|---|
| POST | `/v1/orchestrator/command` | 执行编排命令 |
| GET | `/v1/orchestrator/state` | 查看编排状态 |
| GET | `/v1/orchestrator/context` | 查看编排上下文 |
| POST | `/v1/orchestrator/context/clear` | 清空上下文 |
| GET | `/v1/orchestrator/roles` | 列出角色 |
| POST | `/v1/orchestrator/role` | 切换角色 |

### 6.6 Dialogue

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/v1/dialogue/state` | 对话状态 |
| POST | `/v1/dialogue/transition` | 状态迁移 |
| POST | `/v1/dialogue/goal` | 设置目标 |
| POST | `/v1/dialogue/plan` | 设置计划 |
| GET | `/v1/dialogue/history` | 查看历史 |
| POST | `/v1/dialogue/reset` | 重置状态 |

### 6.7 Workflows

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/v1/workflows` | 列出工作流与模板 |
| POST | `/v1/workflows` | 由模板创建工作流 |
| GET | `/v1/workflows/:id` | 查看工作流 |
| GET | `/v1/workflows/:id/status` | 查看状态 |
| POST | `/v1/workflows/:id/execute` | 执行工作流 |
| POST | `/v1/workflows/:id/pause` | 暂停 |
| POST | `/v1/workflows/:id/cancel` | 取消 |

### 6.8 Sessions

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/v1/sessions` | 列出会话 |
| POST | `/v1/sessions` | 创建会话 |
| GET | `/v1/sessions/:id` | 查看会话 |
| DELETE | `/v1/sessions/:id` | 删除会话 |
| POST | `/v1/sessions/:id/messages` | 追加消息 |

### 6.9 Context

`/v1/context` 是对 `tokitai-context 0.2` 的本地 facade，handler 内部会使用 `spawn_blocking` 打开临时 `Context`，避免持有非 `Send` 对象。

| 方法 | 路径 | 说明 |
|---|---|---|
| POST | `/v1/context/store` | 存储内容 |
| GET | `/v1/context/retrieve` | 读取内容 |
| GET | `/v1/context/search` | 搜索内容 |
| GET | `/v1/context/stats` | 查看统计 |
| POST | `/v1/context/checkpoints/:id/restore` | 恢复 checkpoint |

#### `store` 示例

```bash
curl -sS http://127.0.0.1:8080/v1/context/store \
  -H 'content-type: application/json' \
  -d '{
    "session": "demo",
    "content_b64": "SGVsbG8sIFRva2l0YWk=",
    "layer": "short_term"
  }'
```

### 6.10 Autonomy

| 方法 | 路径 | 说明 |
|---|---|---|
| POST | `/v1/autonomy/start` | 启动自主进化后台任务 |
| POST | `/v1/autonomy/stop` | 停止后台任务 |
| GET | `/v1/autonomy/status` | 查看状态 |
| GET | `/v1/autonomy/gaps` | 查看缺口 |
| GET | `/v1/autonomy/iterations` | 查看迭代历史 |

> `start` 使用后台任务执行；如果 `run_autonomous_evolution` 本身不返回，则需要通过 `/stop` 主动中止。

### 6.11 Tool Market

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/v1/tool-market/list` | 列出工具 |
| POST | `/v1/tool-market/search` | 搜索工具 |
| POST | `/v1/tool-market/install` | 安装工具 |
| POST | `/v1/tool-market/publish` | 发布工具 |

### 6.12 MCP Client

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/v1/mcp/servers` | 列出已知 MCP Server |
| POST | `/v1/mcp/connect` | 连接 MCP Server |
| POST | `/v1/mcp/disconnect` | 断开连接 |
| GET | `/v1/mcp/tools` | 列出 MCP 工具 |
| POST | `/v1/mcp/call` | 调用 MCP 工具 |

### 6.13 CLI Bridge

| 方法 | 路径 | 说明 |
|---|---|---|
| POST | `/v1/cli/run` | 简单工具调用桥接 |
| POST | `/v1/cli/slash` | 将字符串命令转换为 orchestrator 命令 |

## 7. 常见调用示例

### 7.1 列出工具

```bash
curl -sS http://127.0.0.1:8080/v1/tools | jq
```

### 7.2 调用工具

```bash
curl -sS http://127.0.0.1:8080/v1/tools/call \
  -H 'content-type: application/json' \
  -d '{"name":"list_dir","arguments":{"path":"src"}}'
```

### 7.3 创建会话并追加消息

```bash
SID=$(curl -sS -X POST http://127.0.0.1:8080/v1/sessions | jq -r .id)

curl -sS -X POST http://127.0.0.1:8080/v1/sessions/$SID/messages \
  -H 'content-type: application/json' \
  -d '{"role":"user","content":"帮我总结一下 README"}'
```

### 7.4 切换 provider

```bash
curl -sS -X POST http://127.0.0.1:8080/v1/providers/current \
  -H 'content-type: application/json' \
  -d '{"provider":"openai"}'
```

## 8. 优雅停机

服务接收以下信号时会执行 graceful shutdown：

- `Ctrl+C` / `SIGINT`
- `SIGTERM`

日志会输出：

```text
收到 SIGINT，开始 graceful shutdown
```

## 9. 环境变量

与 CLI 共用的环境变量仍然生效：

- `AI_API_URL`
- `AI_API_KEY`
- `AI_MODEL`
- `PROVIDERS`
- `RUST_LOG`

HTTP Server 自身不依赖额外环境变量；端口和鉴权都通过参数指定。

## 10. 退出建议

- 本机开发：直接 `Ctrl+C`
- 自动化脚本：用进程管理器发 `SIGTERM`
- 集成测试：建议只针对 `127.0.0.1` 发请求

## 11. 备注

- 当前实现的 `/v1/chat` 是 LLM 聊天封装，不等于 CLI 的完整工具递归流程
- `/v1/chat/stream` 仅在当前 provider 支持流式时可用
- `tokitai-context` 相关接口使用 `spawn_blocking`，避免在 async handler 中持有非 `Send` 对象
- 由于服务仅监听 loopback，建议把认证当作“本地多进程协作”保护，而不是公网鉴权机制
