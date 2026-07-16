#![cfg(feature = "desktop-shell")]

use std::path::PathBuf;

use ai_assistant::config::Config;
use ai_assistant::desktop_host::DesktopHostRuntime;
use ai_assistant::web::WebHostConfig;
use ai_assistant::AssistantConfig;

fn main() -> anyhow::Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let assistant_config = build_assistant_config();
    let config = Config::load(None).unwrap_or_default();
    let security_config = config.security.clone().into_security_config();

    let host = if let Some(paths) = ai_assistant::app_paths::AppPaths::for_desktop_defaults() {
        WebHostConfig {
            paths,
            bind_addr: std::net::SocketAddr::from(([127, 0, 0, 1], 0)),
            descriptor: ai_assistant::host::HostDescriptor::desktop_bridge(
                ai_assistant::host::HostCapabilities::desktop_default(),
            ),
        }
    } else {
        let state_dir = dirs::data_local_dir()
            .unwrap_or_else(|| cwd.clone())
            .join("Atlas")
            .join("desktop-shell");
        WebHostConfig::for_desktop_shell(cwd.clone(), cwd.join("frontend"), state_dir)
    };

    let runtime = DesktopHostRuntime::new(host, assistant_config, config, security_config)?;
    let meta = runtime.frontend_host_meta();

    println!("Atlas desktop shell stub is ready.");
    println!("Inject this into the frontend host bootstrap:");
    println!("{}", serde_json::to_string_pretty(&meta)?);
    println!();
    println!("Next step: wire this runtime into a real window host (Tauri/Wry/WebView2).");

    Ok(())
}

fn build_assistant_config() -> AssistantConfig {
    let api_url = std::env::var("AI_API_URL")
        .unwrap_or_else(|_| "https://api.openai.com/v1/chat/completions".to_string());
    let api_key = std::env::var("AI_API_KEY").ok();
    let model = std::env::var("AI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
    AssistantConfig::new(api_url, api_key, model)
}
