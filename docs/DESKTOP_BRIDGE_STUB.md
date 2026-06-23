# Desktop Bridge Stub

当前项目已经提供可复用的桌面宿主运行时内核：

- `ai_assistant::desktop_host::DesktopHostRuntime`
- `ai_assistant::web::dispatch_bridge_command`
- `ai_assistant::web::dispatch_bridge_stream`
- `ai_assistant::host::HostCommand`

## 最小接线方式

1. 创建 `WebHostConfig`
2. 创建 `DesktopHostRuntime`
3. 把宿主收到的前端命令转发到 `runtime.invoke(...)`
4. 把宿主收到的流式命令转发到 `runtime.open_stream(...)`
5. 把 `runtime.frontend_host_meta()` 注入到前端的 `window.__TOKITAI_HOST__`

## 伪代码

```rust
use ai_assistant::desktop_host::DesktopHostRuntime;
use ai_assistant::web::WebHostConfig;

let host = WebHostConfig::for_desktop_shell(base_dir, frontend_dir, state_dir);
let runtime = DesktopHostRuntime::new(host, assistant_config, config_file, security_config)?;

let host_meta = runtime.frontend_host_meta();

let response = runtime.invoke("bootstrap.load", serde_json::json!({})).await;

let stream = runtime.open_stream(
    "chat.stream",
    serde_json::json!({
        "content": "hello",
        "mode": "chat"
    }),
)?;
```

## 前端对应注入点

- `window.__TOKITAI_HOST__`
- `window.__TOKITAI_DESKTOP_BRIDGE__`

其中：

- `invoke(command, payload)` 对应同步命令
- `openStream(command, payload)` 对应 `chat.stream`

## 当前状态

已经完成：

- 宿主能力描述
- bridge 命令解析
- 同步 bridge 命令分发
- `chat.stream` 流式 bridge 分发
- desktop runtime smoke test
- `desktop-shell` feature 下的独立宿主入口：`src/bin/desktop_shell.rs`

尚未完成：

- 具体桌面壳实现（Tauri / Wry / Electron / WebView2）
- 宿主级文件对话框、菜单、窗口事件与系统托盘接线
- 前端桥对象的实际原生注入

## 当前可运行的桌面宿主 stub

```bash
cargo run --features desktop-shell --bin desktop_shell
```

这个入口当前会：

- 创建 `DesktopHostRuntime`
- 输出可注入前端的 host meta
- 作为后续接入真实桌面窗口宿主的起点

## 当前可运行的原生窗口骨架

```bash
cargo run --features desktop-shell --bin desktop_wry
```

这个入口当前会：

- 创建 `DesktopHostRuntime`
- 启动内部 Web 服务
- 用 `wry` 打开原生桌面窗口
- 加载本地 `http://127.0.0.1:*` 页面

说明：

- 这是桌面窗口宿主骨架，不是最终桥接完成版
- 目前还没有把 `window.__TOKITAI_DESKTOP_BRIDGE__` 直接原生注入到窗口里
- 当前更像“原生窗口 + 内部服务 + 后续 bridge 接线点”
