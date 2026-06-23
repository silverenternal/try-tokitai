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
use ai_assistant::web::{serve_web_listener, WebHostConfig};
use ai_assistant::AssistantConfig;
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use tao::dpi::LogicalSize;
use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::window::WindowBuilder;
use tokio::sync::mpsc::UnboundedReceiver;
use wry::http::Request;
use wry::WebViewBuilder;

#[derive(Debug)]
enum DesktopEvent {
    InvokeResult {
        request_id: String,
        response: HostBridgeResponse,
    },
    StreamEvent {
        stream_id: String,
        event: Value,
    },
    StreamClosed {
        stream_id: String,
        error: Option<String>,
    },
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
}

fn main() -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let assistant_config = build_assistant_config();
    let config = Config::load(None).unwrap_or_default();
    let security_config = config.security.clone().into_security_config();

    let desktop_paths = ai_assistant::app_paths::AppPaths::for_desktop_defaults()
        .unwrap_or_else(|| ai_assistant::app_paths::AppPaths::for_local_dev(cwd.clone()));

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
    let frontend_host_meta = desktop_runtime.frontend_host_meta();

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
    let window = WindowBuilder::new()
        .with_title("Tokitai Desktop")
        .with_inner_size(LogicalSize::new(1440.0, 920.0))
        .build(&event_loop)?;

    let initialization_script = build_initialization_script(&frontend_host_meta);
    let ipc_proxy = event_proxy.clone();
    let runtime_for_ipc = desktop_runtime.clone();
    let async_for_ipc = async_runtime.clone();

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
        .with_devtools(true)
        .build()?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {}
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                window.set_visible(false);
                window.set_minimized(true);
            }
            Event::UserEvent(DesktopEvent::InvokeResult {
                request_id,
                response,
            }) => {
                if let Err(err) = dispatch_invoke_result(&webview, &request_id, &response) {
                    eprintln!("desktop bridge invoke dispatch error: {}", err);
                }
            }
            Event::UserEvent(DesktopEvent::StreamEvent { stream_id, event }) => {
                let event_json = match serde_json::to_string(&event) {
                    Ok(value) => value,
                    Err(err) => {
                        eprintln!("desktop bridge stream serialization error: {}", err);
                        return;
                    }
                };
                let script = format!(
                    "window.__TOKITAI_BRIDGE_STREAM_PUSH__ && window.__TOKITAI_BRIDGE_STREAM_PUSH__({stream_id:?}, {event_json});",
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
                    "window.__TOKITAI_BRIDGE_STREAM_CLOSE__ && window.__TOKITAI_BRIDGE_STREAM_CLOSE__({stream_id:?}, {payload_json});",
                );
                if let Err(err) = webview.evaluate_script(&script) {
                    eprintln!("desktop bridge stream close error: {}", err);
                }
            }
            _ => {}
        }
    })
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
        while let Some(event) = receiver.recv().await {
            if event_proxy
                .send_event(DesktopEvent::StreamEvent {
                    stream_id: stream_id.clone(),
                    event,
                })
                .is_err()
            {
                return;
            }
        }

        let _ = event_proxy.send_event(DesktopEvent::StreamClosed {
            stream_id,
            error: None,
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
        "window.__TOKITAI_BRIDGE_RESOLVE__ && window.__TOKITAI_BRIDGE_RESOLVE__({request_id:?}, {response_json});"
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

  window.__TOKITAI_HOST__ = hostMeta;

  window.__TOKITAI_BRIDGE_RESOLVE__ = (id, response) => {{
    const entry = pending.get(id);
    if (!entry) return;
    pending.delete(id);
    entry.resolve(response);
  }};

  window.__TOKITAI_BRIDGE_STREAM_PUSH__ = (id, event) => {{
    const controller = streams.get(id);
    if (!controller) return;
    const chunk = controller.encoder.encode(`${{JSON.stringify(event)}}\n`);
    controller.streamController.enqueue(chunk);
  }};

  window.__TOKITAI_BRIDGE_STREAM_CLOSE__ = (id, error) => {{
    const controller = streams.get(id);
    if (!controller) return;
    streams.delete(id);
    if (error && error.message) {{
      controller.streamController.error(new Error(error.message));
    }} else {{
      controller.streamController.close();
    }}
  }};

  window.__TOKITAI_DESKTOP_BRIDGE__ = {{
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
