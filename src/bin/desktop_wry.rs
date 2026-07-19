#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
#![cfg(feature = "desktop-shell")]

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use ai_assistant::config::Config;
use ai_assistant::desktop_host::DesktopHostRuntime;
use ai_assistant::host::HostBridgeResponse;
use ai_assistant::process_window::CommandWindowExt;
use ai_assistant::web::{serve_web_listener, WebHostConfig};
use ai_assistant::AssistantConfig;
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use tao::dpi::{LogicalPosition, LogicalSize};
use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::window::{Icon, Window, WindowBuilder};
use tokio::sync::mpsc::UnboundedReceiver;
use wry::http::Request;
use wry::{Rect, WebContext, WebView, WebViewBuilder};

#[derive(Debug)]
enum DesktopEvent {
    InvokeResult {
        request_id: String,
        response: HostBridgeResponse,
    },
    StreamEvents {
        stream_id: String,
        events: Vec<Value>,
    },
    StreamClosed {
        stream_id: String,
        error: Option<String>,
    },
    NativeBrowserCommand(NativeBrowserRequest),
    NativeBrowserNavigated {
        url: String,
    },
    NativeWindowCommand(NativeWindowRequest),
}

#[derive(Debug, Deserialize, Clone)]
struct NativeBrowserBounds {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Debug, Deserialize, Clone)]
struct NativeBrowserRequest {
    action: String,
    #[serde(default)]
    url: String,
    bounds: Option<NativeBrowserBounds>,
}

#[derive(Debug, Deserialize)]
struct NativeWindowRequest {
    action: String,
    #[serde(default)]
    workspace: String,
    #[serde(default)]
    state: Value,
}

#[derive(Debug, Deserialize)]
struct BridgeInvokeRequest {
    id: String,
    command: String,
    #[serde(default)]
    payload: Value,
}

#[derive(Debug, Deserialize)]
struct BridgeStreamRequest {
    id: String,
    command: String,
    #[serde(default)]
    payload: Value,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum BridgeIpcMessage {
    Invoke(BridgeInvokeRequest),
    OpenStream(BridgeStreamRequest),
    NativeBrowser(NativeBrowserRequest),
    NativeWindow(NativeWindowRequest),
}

fn main() -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let requested_workspace = command_line_workspace()
        .or_else(last_desktop_workspace)
        .unwrap_or_else(|| cwd.clone());
    std::env::set_var("ATLAS_WORKSPACE_ROOT", &requested_workspace);
    let project_id = ai_assistant::app_paths::project_id(&requested_workspace);
    let window_id = uuid::Uuid::new_v4().to_string();
    let assistant_config = build_assistant_config();
    let config = Config::load(None).unwrap_or_default();
    let security_config = config.security.clone().into_security_config();

    let desktop_paths =
        ai_assistant::app_paths::AppPaths::for_desktop_project(&requested_workspace)
            .unwrap_or_else(|| ai_assistant::app_paths::AppPaths::for_local_dev(cwd.clone()));
    let desktop_session_path = desktop_paths.state_dir().join("desktop-session.json");
    let desktop_restore_state =
        load_desktop_session_state(&desktop_session_path, &requested_workspace);

    let host = WebHostConfig {
        paths: desktop_paths.clone(),
        bind_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
        descriptor: ai_assistant::host::HostDescriptor::desktop_bridge(
            ai_assistant::host::HostCapabilities::desktop_default(),
        ),
    };
    let frontend_entry = host.frontend_dir().join("index.html");
    if !frontend_entry.exists() {
        return Err(anyhow!(
            "desktop frontend entry was not found: {}",
            frontend_entry.display()
        ));
    }

    let desktop_runtime = Arc::new(DesktopHostRuntime::new(
        host.clone(),
        assistant_config.clone(),
        config.clone(),
        security_config.clone(),
    )?);
    let mut frontend_host_meta = desktop_runtime.frontend_host_meta();
    if let Some(meta) = frontend_host_meta.as_object_mut() {
        meta.insert("projectId".into(), json!(project_id));
        meta.insert("windowId".into(), json!(window_id));
        meta.insert("workspaceRoot".into(), json!(requested_workspace));
        meta.insert(
            "restoreState".into(),
            desktop_restore_state.clone().unwrap_or(Value::Null),
        );
    }

    let async_runtime = Arc::new(tokio::runtime::Runtime::new()?);
    let shared_web_state = desktop_runtime.web_state();

    let (server_tx, server_rx) = mpsc::channel::<Result<SocketAddr, String>>();
    let host_for_server = host.clone();
    let shared_web_state_for_server = shared_web_state.clone();
    let server_tx_err = server_tx.clone();
    thread::spawn(move || {
        let runtime_result = tokio::runtime::Runtime::new()
            .map_err(|err| err.to_string())
            .and_then(|rt| {
                rt.block_on(async move {
                    let listener = tokio::net::TcpListener::bind(host_for_server.bind_addr)
                        .await
                        .map_err(|err| err.to_string())?;
                    let addr = listener.local_addr().map_err(|err| err.to_string())?;
                    server_tx.send(Ok(addr)).map_err(|err| err.to_string())?;
                    serve_web_listener(
                        listener,
                        shared_web_state_for_server,
                        host_for_server.frontend_dir().to_path_buf(),
                    )
                    .await
                    .map_err(|err| err.to_string())
                })
            });

        if let Err(err) = runtime_result {
            let _ = server_tx_err.send(Err(err));
        }
    });

    let server_addr = server_rx
        .recv()
        .map_err(|err| anyhow!("failed to receive desktop web server address: {}", err))?
        .map_err(|err| anyhow!("desktop web server failed: {}", err))?;
    wait_for_web_ready(server_addr)?;

    let event_loop = EventLoopBuilder::<DesktopEvent>::with_user_event().build();
    let event_proxy = event_loop.create_proxy();
    let mut window_builder = WindowBuilder::new()
        .with_title("Atlas IDE")
        .with_window_icon(build_atlas_window_icon())
        .with_decorations(false)
        .with_resizable(true)
        .with_min_inner_size(LogicalSize::new(900.0, 600.0))
        .with_inner_size(restored_window_size(&desktop_restore_state));
    if let Some(position) = restored_window_position(&desktop_restore_state) {
        window_builder = window_builder.with_position(position);
    }
    let window = window_builder.build(&event_loop)?;
    if restored_window_maximized(&desktop_restore_state) {
        window.set_maximized(true);
    }

    let initialization_script = build_initialization_script(&frontend_host_meta);
    let ipc_proxy = event_proxy.clone();
    let runtime_for_ipc = desktop_runtime.clone();
    let async_for_ipc = async_runtime.clone();
    let shell_data_dir = desktop_paths.state_dir().join("WebView2").join("Shell");
    let browser_data_dir = desktop_paths.state_dir().join("WebView2").join("Browser");
    std::fs::create_dir_all(&shell_data_dir)?;
    std::fs::create_dir_all(&browser_data_dir)?;
    let mut shell_web_context = WebContext::new(Some(shell_data_dir));
    let mut browser_web_context = WebContext::new(Some(browser_data_dir));

    #[cfg(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    ))]
    let builder = WebViewBuilder::new(&window);

    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    )))]
    let builder = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().unwrap();
        WebViewBuilder::new_gtk(vbox)
    };

    let url = format!("http://{}", server_addr);
    let webview = builder
        .with_web_context(&mut shell_web_context)
        .with_background_color((24, 24, 24, 255))
        .with_initialization_script(&initialization_script)
        .with_ipc_handler(move |req: Request<String>| {
            if let Err(err) = handle_ipc_message(
                req.body(),
                runtime_for_ipc.clone(),
                async_for_ipc.clone(),
                ipc_proxy.clone(),
            ) {
                eprintln!("desktop bridge IPC error: {}", err);
            }
        })
        .with_url(&url)
        .with_devtools(cfg!(debug_assertions))
        .build()?;

    // The browser child WebView is intentionally lazy. Creating two WebView2 controllers at
    // startup made chat-only sessions vulnerable to native controller/message-loop failures.
    let mut native_browser: Option<WebView> = None;
    let mut close_deadline: Option<Instant> = None;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = close_deadline
            .map(ControlFlow::WaitUntil)
            .unwrap_or(ControlFlow::Wait);

        match event {
            Event::NewEvents(StartCause::Init) => {}
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                if close_deadline.is_none() {
                    close_deadline = Some(Instant::now() + Duration::from_secs(2));
                    if let Err(err) = webview.evaluate_script(
                        "window.__ATLAS_REQUEST_CLOSE__ && window.__ATLAS_REQUEST_CLOSE__();",
                    ) {
                        eprintln!("desktop state capture request failed: {}", err);
                        let _ = persist_desktop_session_state(
                            &desktop_session_path,
                            &requested_workspace,
                            Value::Null,
                            &window,
                        );
                        std::process::exit(0);
                    }
                }
            }
            Event::MainEventsCleared if close_deadline.is_some_and(|deadline| Instant::now() >= deadline) => {
                let _ = persist_desktop_session_state(
                    &desktop_session_path,
                    &requested_workspace,
                    Value::Null,
                    &window,
                );
                std::process::exit(0);
            }
            Event::UserEvent(DesktopEvent::InvokeResult {
                request_id,
                response,
            }) => {
                if let Err(err) = dispatch_invoke_result(&webview, &request_id, &response) {
                    eprintln!("desktop bridge invoke dispatch error: {}", err);
                }
            }
            Event::UserEvent(DesktopEvent::StreamEvents { stream_id, events }) => {
                let events_json = match serde_json::to_string(&events) {
                    Ok(value) => value,
                    Err(err) => {
                        eprintln!("desktop bridge stream serialization error: {}", err);
                        return;
                    }
                };
                let script = format!(
                    "window.__ATLAS_BRIDGE_STREAM_PUSH_BATCH__ && window.__ATLAS_BRIDGE_STREAM_PUSH_BATCH__({stream_id:?}, {events_json});",
                );
                if let Err(err) = webview.evaluate_script(&script) {
                    eprintln!("desktop bridge stream push error: {}", err);
                }
            }
            Event::UserEvent(DesktopEvent::StreamClosed { stream_id, error }) => {
                let payload = match error {
                    Some(message) => json!({ "message": message }),
                    None => Value::Null,
                };
                let payload_json = serde_json::to_string(&payload).unwrap_or_else(|_| "null".to_string());
                let script = format!(
                    "window.__ATLAS_BRIDGE_STREAM_CLOSE__ && window.__ATLAS_BRIDGE_STREAM_CLOSE__({stream_id:?}, {payload_json});",
                );
                if let Err(err) = webview.evaluate_script(&script) {
                    eprintln!("desktop bridge stream close error: {}", err);
                }
            }
            Event::UserEvent(DesktopEvent::NativeBrowserCommand(request)) => {
                if request.action == "open" && native_browser.is_none() {
                    let native_browser_proxy = event_proxy.clone();
                    let native_browser_new_window_proxy = event_proxy.clone();
                    match WebViewBuilder::new_as_child(&window)
                        .with_web_context(&mut browser_web_context)
                        .with_bounds(Rect {
                            position: LogicalPosition::new(0.0, 0.0).into(),
                            size: LogicalSize::new(1.0, 1.0).into(),
                        })
                        .with_url("about:blank")
                        .with_on_page_load_handler(move |_event, url| {
                            let _ = native_browser_proxy.send_event(
                                DesktopEvent::NativeBrowserNavigated { url },
                            );
                        })
                        .with_new_window_req_handler(move |url| {
                            if url.starts_with("http://") || url.starts_with("https://") {
                                let _ = native_browser_new_window_proxy.send_event(
                                    DesktopEvent::NativeBrowserCommand(NativeBrowserRequest {
                                        action: "open".to_string(),
                                        url,
                                        bounds: None,
                                    }),
                                );
                            }
                            // Always consume the request. Authentication and target=_blank
                            // navigations stay inside Atlas instead of spawning an OS window.
                            false
                        })
                        .with_devtools(cfg!(debug_assertions))
                        .build()
                    {
                        Ok(browser) => native_browser = Some(browser),
                        Err(err) => eprintln!("native browser creation failed: {}", err),
                    }
                }
                if let Some(bounds) = request.bounds {
                    if let Some(browser) = native_browser.as_ref() {
                        let _ = browser.set_bounds(Rect {
                            position: LogicalPosition::new(bounds.x.max(0.0), bounds.y.max(0.0)).into(),
                            size: LogicalSize::new(bounds.width.max(1.0), bounds.height.max(1.0)).into(),
                        });
                    }
                }
                if let Some(browser) = native_browser.as_ref() {
                    match request.action.as_str() {
                        "open" => {
                            if request.url.starts_with("http://") || request.url.starts_with("https://") {
                                if let Err(err) = browser.load_url(&request.url) {
                                    eprintln!("native browser navigation failed: {}", err);
                                } else {
                                    let _ = browser.set_visible(true);
                                }
                            }
                        }
                        "show" | "layout" => { let _ = browser.set_visible(true); }
                        "hide" | "close" => { let _ = browser.set_visible(false); }
                        "back" => { let _ = browser.evaluate_script("history.back()"); }
                        "forward" => { let _ = browser.evaluate_script("history.forward()"); }
                        "refresh" => { let _ = browser.evaluate_script("location.reload()"); }
                        _ => {}
                    }
                }
            }
            Event::UserEvent(DesktopEvent::NativeBrowserNavigated { url }) => {
                let script = format!(
                    "window.__ATLAS_NATIVE_BROWSER_NAVIGATED__ && window.__ATLAS_NATIVE_BROWSER_NAVIGATED__({url:?});"
                );
                if let Err(err) = webview.evaluate_script(&script) {
                    eprintln!("native browser navigation dispatch failed: {}", err);
                }
            }
            Event::UserEvent(DesktopEvent::NativeWindowCommand(request)) => {
                match request.action.as_str() {
                    "drag" => { let _ = window.drag_window(); }
                    "minimize" => window.set_minimized(true),
                    "toggle_maximize" => window.set_maximized(!window.is_maximized()),
                    "new_project_window" => {
                        let target = if request.workspace.trim().is_empty() {
                            rfd::FileDialog::new().set_title("Open project in new Atlas window").pick_folder().unwrap_or_default()
                        } else { PathBuf::from(request.workspace.trim()) };
                        if target.is_dir() {
                            if let Ok(exe) = std::env::current_exe() {
                                let mut command = std::process::Command::new(exe);
                                command.arg("--workspace").arg(target).hide_window();
                                if let Err(err) = command.spawn() { eprintln!("new Atlas window failed: {}", err); }
                            }
                        }
                    }
                    "open_vscode" => {
                        let target = PathBuf::from(request.workspace.trim());
                        if target.exists() {
                            let mut command = std::process::Command::new("code");
                            command.arg(target).hide_window();
                            if let Err(err) = command.spawn() {
                                eprintln!("open in VS Code failed: {}", err);
                            }
                        }
                    }
                    "open_system" => {
                        let target = PathBuf::from(request.workspace.trim());
                        if target.exists() {
                            let mut command = std::process::Command::new("explorer.exe");
                            if target.is_file() {
                                command.arg(format!("/select,{}", target.display()));
                            } else {
                                command.arg(target);
                            }
                            command.hide_window();
                            if let Err(err) = command.spawn() {
                                eprintln!("open in system explorer failed: {}", err);
                            }
                        }
                    }
                    "close" => {
                        if let Err(err) = persist_desktop_session_state(
                            &desktop_session_path,
                            &requested_workspace,
                            request.state,
                            &window,
                        ) {
                            eprintln!("desktop session state save failed: {}", err);
                        }
                        if let Some(browser) = native_browser.as_ref() {
                            let _ = browser.set_visible(false);
                        }
                        window.set_visible(false);
                        std::process::exit(0);
                    }
                    _ => {}
                }
                let maximized = window.is_maximized();
                let script = format!(
                    "window.__ATLAS_WINDOW_STATE__ && window.__ATLAS_WINDOW_STATE__({maximized});"
                );
                let _ = webview.evaluate_script(&script);
            }
            _ => {}
        }
    })
}

fn command_line_workspace() -> Option<PathBuf> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let mut index = 0usize;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--workspace" {
            // Windows launchers occasionally pass an unquoted path containing spaces. Rejoin
            // adjacent non-option arguments and prefer the longest existing directory.
            let mut best = None;
            let mut joined = String::new();
            for value in args.iter().skip(index + 1) {
                let part = value.to_string_lossy();
                if part.starts_with("--") {
                    break;
                }
                if !joined.is_empty() {
                    joined.push(' ');
                }
                joined.push_str(&part);
                let candidate = PathBuf::from(&joined);
                if candidate.is_dir() {
                    best = Some(candidate);
                }
            }
            return best;
        }
        if let Some(value) = arg.to_string_lossy().strip_prefix("--workspace=") {
            let path = PathBuf::from(value);
            if path.is_dir() {
                return Some(path);
            }
        }
        index += 1;
    }
    None
}

fn last_desktop_workspace() -> Option<PathBuf> {
    let state_path = dirs::data_local_dir()?
        .join("Atlas")
        .join("web-runtime.json");
    let content = std::fs::read_to_string(state_path).ok()?;
    let value: Value = serde_json::from_str(&content).ok()?;
    value
        .get("workspace_root")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
}

fn load_desktop_session_state(
    path: &std::path::Path,
    workspace: &std::path::Path,
) -> Option<Value> {
    let content = std::fs::read_to_string(path).ok()?;
    let value: Value = serde_json::from_str(&content).ok()?;
    let saved_workspace = value.get("workspace_root")?.as_str()?;
    if PathBuf::from(saved_workspace) != workspace {
        return None;
    }
    Some(value)
}

fn restored_window_size(state: &Option<Value>) -> LogicalSize<f64> {
    let width = state
        .as_ref()
        .and_then(|value| value.pointer("/window/width"))
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
        .unwrap_or(1440.0)
        .max(900.0);
    let height = state
        .as_ref()
        .and_then(|value| value.pointer("/window/height"))
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
        .unwrap_or(920.0)
        .max(600.0);
    LogicalSize::new(width, height)
}

fn restored_window_position(state: &Option<Value>) -> Option<LogicalPosition<f64>> {
    let value = state.as_ref()?;
    let x = value.pointer("/window/x")?.as_f64()?;
    let y = value.pointer("/window/y")?.as_f64()?;
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    Some(LogicalPosition::new(x, y))
}

fn restored_window_maximized(state: &Option<Value>) -> bool {
    state
        .as_ref()
        .and_then(|value| value.pointer("/window/maximized"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn persist_desktop_session_state(
    path: &std::path::Path,
    workspace: &std::path::Path,
    client_state: Value,
    window: &Window,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let scale = window.scale_factor();
    let size = window.inner_size().to_logical::<f64>(scale);
    let position = window
        .outer_position()
        .ok()
        .map(|position| position.to_logical::<f64>(scale));
    let saved_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let value = json!({
        "version": 1,
        "saved_at": saved_at,
        "workspace_root": workspace,
        "window": {
            "x": position.map(|value| value.x),
            "y": position.map(|value| value.y),
            "width": size.width,
            "height": size.height,
            "maximized": window.is_maximized(),
        },
        "client": client_state,
    });
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, serde_json::to_vec_pretty(&value)?)?;
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    std::fs::rename(temporary, path)?;
    Ok(())
}

fn build_atlas_window_icon() -> Option<Icon> {
    const SIZE: u32 = 64;
    let decoder =
        png::Decoder::new(include_bytes!("../../frontend/atlas-lockup-light.png").as_slice());
    let mut reader = decoder.read_info().ok()?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer).ok()?;
    let source = match info.color_type {
        png::ColorType::Rgba => buffer[..info.buffer_size()].to_vec(),
        png::ColorType::Rgb => buffer[..info.buffer_size()]
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        _ => return None,
    };
    let mut rgba = vec![0; (SIZE * SIZE * 4) as usize];
    let scale = (SIZE as f32 / info.width as f32).min(SIZE as f32 / info.height as f32);
    let rw = (info.width as f32 * scale).round() as u32;
    let rh = (info.height as f32 * scale).round() as u32;
    let ox = (SIZE - rw) / 2;
    let oy = (SIZE - rh) / 2;
    for y in 0..rh {
        for x in 0..rw {
            let sx = (x as u64 * info.width as u64 / rw as u64) as u32;
            let sy = (y as u64 * info.height as u64 / rh as u64) as u32;
            let s = ((sy * info.width + sx) * 4) as usize;
            let d = (((oy + y) * SIZE + ox + x) * 4) as usize;
            rgba[d..d + 4].copy_from_slice(&source[s..s + 4]);
        }
    }
    Icon::from_rgba(rgba, SIZE, SIZE).ok()
}

fn handle_ipc_message(
    body: &str,
    runtime: Arc<DesktopHostRuntime>,
    async_runtime: Arc<tokio::runtime::Runtime>,
    event_proxy: EventLoopProxy<DesktopEvent>,
) -> Result<()> {
    let message: BridgeIpcMessage =
        serde_json::from_str(body).with_context(|| format!("invalid IPC payload: {}", body))?;

    match message {
        BridgeIpcMessage::Invoke(request) => {
            async_runtime.spawn(async move {
                let response = runtime.invoke(&request.command, request.payload).await;
                let _ = event_proxy.send_event(DesktopEvent::InvokeResult {
                    request_id: request.id,
                    response,
                });
            });
        }
        BridgeIpcMessage::OpenStream(request) => {
            // dispatch_bridge_stream creates Tokio tasks synchronously. IPC callbacks run on
            // Wry's UI thread, so enter the owned runtime before opening the native stream.
            let _runtime_guard = async_runtime.enter();
            match runtime
                .open_stream(&request.command, request.payload)
                .with_context(|| format!("failed to open stream '{}'", request.command))
            {
                Ok(stream) => {
                    let stream_id = request.id.clone();
                    forward_stream(stream_id, stream.receiver, async_runtime, event_proxy);
                }
                Err(err) => {
                    let _ = event_proxy.send_event(DesktopEvent::StreamClosed {
                        stream_id: request.id,
                        error: Some(err.to_string()),
                    });
                }
            }
        }
        BridgeIpcMessage::NativeBrowser(request) => {
            event_proxy
                .send_event(DesktopEvent::NativeBrowserCommand(request))
                .map_err(|_| anyhow!("desktop native browser event loop is unavailable"))?;
        }
        BridgeIpcMessage::NativeWindow(request) => {
            event_proxy
                .send_event(DesktopEvent::NativeWindowCommand(request))
                .map_err(|_| anyhow!("desktop native window event loop is unavailable"))?;
        }
    }

    Ok(())
}

fn forward_stream(
    stream_id: String,
    mut receiver: UnboundedReceiver<Value>,
    async_runtime: Arc<tokio::runtime::Runtime>,
    event_proxy: EventLoopProxy<DesktopEvent>,
) {
    async_runtime.spawn(async move {
        let mut saw_terminal_event = false;
        while let Some(first_event) = receiver.recv().await {
            let mut events = vec![first_event];
            tokio::time::sleep(Duration::from_millis(12)).await;
            while events.len() < 128 {
                match receiver.try_recv() {
                    Ok(event) => events.push(event),
                    Err(_) => break,
                }
            }
            saw_terminal_event |= events.iter().any(|event| {
                event
                    .get("type")
                    .and_then(Value::as_str)
                    .is_some_and(|kind| matches!(kind, "complete" | "error"))
            });
            if event_proxy
                .send_event(DesktopEvent::StreamEvents {
                    stream_id: stream_id.clone(),
                    events,
                })
                .is_err()
            {
                return;
            }
        }

        let _ = event_proxy.send_event(DesktopEvent::StreamClosed {
            stream_id,
            error: (!saw_terminal_event).then(|| {
                "desktop agent stream closed before a complete or error event".to_string()
            }),
        });
    });
}

fn dispatch_invoke_result(
    webview: &wry::WebView,
    request_id: &str,
    response: &HostBridgeResponse,
) -> Result<()> {
    let response_json = serde_json::to_string(response)?;
    let script = format!(
        "window.__ATLAS_BRIDGE_RESOLVE__ && window.__ATLAS_BRIDGE_RESOLVE__({request_id:?}, {response_json});"
    );
    webview.evaluate_script(&script)?;
    Ok(())
}

fn build_initialization_script(host_meta: &Value) -> String {
    let host_meta_json = serde_json::to_string(host_meta).unwrap_or_else(|_| "{}".to_string());
    format!(
        r#"
(() => {{
  const hostMeta = {host_meta_json};
  const pending = new Map();
  const streams = new Map();

  window.__ATLAS_HOST__ = hostMeta;

  window.__ATLAS_BRIDGE_RESOLVE__ = (id, response) => {{
    const entry = pending.get(id);
    if (!entry) return;
    pending.delete(id);
    entry.resolve(response);
  }};

  window.__ATLAS_BRIDGE_STREAM_PUSH__ = (id, event) => {{
    const controller = streams.get(id);
    if (!controller) return;
    const chunk = controller.encoder.encode(`${{JSON.stringify(event)}}\n`);
    controller.streamController.enqueue(chunk);
  }};

  window.__ATLAS_BRIDGE_STREAM_CLOSE__ = (id, error) => {{
    const controller = streams.get(id);
    if (!controller) return;
    streams.delete(id);
    if (error && error.message) {{
      controller.streamController.error(new Error(error.message));
    }} else {{
      controller.streamController.close();
    }}
  }};

  window.__ATLAS_DESKTOP_BRIDGE__ = {{
    invoke(command, payload = {{}}) {{
      const id = `invoke:${{Date.now()}}:${{Math.random().toString(36).slice(2)}}`;
      return new Promise((resolve, reject) => {{
        pending.set(id, {{ resolve, reject }});
        try {{
          window.ipc.postMessage(JSON.stringify({{
            kind: "invoke",
            id,
            command,
            payload,
          }}));
        }} catch (error) {{
          pending.delete(id);
          reject(error);
        }}
      }});
    }},
    openStream(command, payload = {{}}) {{
      const id = `stream:${{Date.now()}}:${{Math.random().toString(36).slice(2)}}`;
      const encoder = new TextEncoder();
      let streamController = null;
      const stream = new ReadableStream({{
        start(controller) {{
          streamController = controller;
          streams.set(id, {{ encoder, streamController: controller }});
          window.ipc.postMessage(JSON.stringify({{
            kind: "open_stream",
            id,
            command,
            payload,
          }}));
        }},
        cancel() {{
          streams.delete(id);
        }},
      }});
      return new Response(stream, {{
        headers: {{
          "Content-Type": "application/x-ndjson; charset=utf-8",
        }},
      }});
    }},
  }};
  window.__ATLAS_NATIVE_BROWSER__ = {{
    send(payload = {{}}) {{
      window.ipc.postMessage(JSON.stringify({{ kind: "native_browser", ...payload }}));
    }}
  }};
  window.__ATLAS_NATIVE_WINDOW__ = {{
    send(payload = {{}}) {{
      window.ipc.postMessage(JSON.stringify({{ kind: "native_window", ...payload }}));
    }}
  }};

  window.__ATLAS_BRIDGE_STREAM_PUSH_BATCH__ = (id, events) => {{
    if (!Array.isArray(events)) return;
    for (const event of events) {{
      window.__ATLAS_BRIDGE_STREAM_PUSH__(id, event);
    }}
  }};
  let closeRequested = false;
  window.__ATLAS_REQUEST_CLOSE__ = () => {{
    if (closeRequested) return;
    closeRequested = true;
    let state = {{}};
    try {{
      state = window.AtlasWorkspaceBridge?.capture?.() || {{}};
    }} catch (error) {{
      console.warn("Atlas desktop state capture failed", error);
    }}
    window.__ATLAS_NATIVE_WINDOW__.send({{ action: "close", state }});
  }};
}})();
"#
    )
}

fn build_assistant_config() -> AssistantConfig {
    let api_url = std::env::var("AI_API_URL")
        .unwrap_or_else(|_| "https://api.openai.com/v1/chat/completions".to_string());
    let api_key = std::env::var("AI_API_KEY").ok();
    let model = std::env::var("AI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
    AssistantConfig::new(api_url, api_key, model)
}

fn wait_for_web_ready(server_addr: SocketAddr) -> Result<()> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(1))
        .no_proxy()
        .build()?;
    let url = format!("http://{}/index.html", server_addr);
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut last_error = String::new();

    while Instant::now() < deadline {
        match client.get(&url).send() {
            Ok(response) if response.status().is_success() => return Ok(()),
            Ok(response) => {
                last_error = format!("frontend returned HTTP {}", response.status());
            }
            Err(err) => {
                last_error = err.to_string();
            }
        }
        thread::sleep(Duration::from_millis(150));
    }

    Err(anyhow!(
        "desktop frontend did not become ready at {} within 10s: {}",
        url,
        last_error
    ))
}
