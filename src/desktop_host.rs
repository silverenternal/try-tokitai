use anyhow::Result;
use serde_json::{json, Value};

use crate::assistant_common::AssistantConfig;
use crate::config::Config;
use crate::host::{HostBridgeResponse, HostBridgeStream, HostDescriptor};
use crate::security::SecurityConfig;
use crate::web::{
    build_web_app_state, dispatch_bridge_command, dispatch_bridge_stream, WebAppState,
    WebHostConfig,
};

#[derive(Clone)]
pub struct DesktopHostRuntime {
    state: WebAppState,
    descriptor: HostDescriptor,
}

impl DesktopHostRuntime {
    pub fn new(
        host: WebHostConfig,
        assistant_config: AssistantConfig,
        config_file: Config,
        security_config: SecurityConfig,
    ) -> Result<Self> {
        let descriptor = host.descriptor();
        let state = build_web_app_state(host, assistant_config, config_file, security_config)?;
        Ok(Self { state, descriptor })
    }

    pub fn descriptor(&self) -> &HostDescriptor {
        &self.descriptor
    }

    pub fn frontend_host_meta(&self) -> Value {
        json!({
            "mode": self.descriptor.mode,
            "transport": self.descriptor.transport,
            "supportsStreaming": self.descriptor.capabilities.supports_streaming,
            "supportsFileDialog": self.descriptor.capabilities.supports_file_dialog,
            "supportsTerminal": self.descriptor.capabilities.supports_terminal,
            "supportsTerminalPty": self.descriptor.capabilities.supports_terminal_pty,
            "supportsNativeMenu": self.descriptor.capabilities.supports_native_menu,
            "bridgeProtocol": self.descriptor.bridge_protocol,
        })
    }

    pub fn web_state(&self) -> WebAppState {
        self.state.clone()
    }

    pub async fn invoke(&self, command: &str, payload: Value) -> HostBridgeResponse {
        dispatch_bridge_command(self.state.clone(), command, payload).await
    }

    pub fn open_stream(&self, command: &str, payload: Value) -> Result<HostBridgeStream> {
        dispatch_bridge_stream(self.state.clone(), command, payload)
    }
}

#[cfg(test)]
mod tests {
    use super::DesktopHostRuntime;
    use crate::assistant_common::AssistantConfig;
    use crate::config::Config;
    use crate::security::SecurityConfig;
    use crate::web::WebHostConfig;
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn desktop_runtime_handles_bootstrap_bridge() {
        let root = PathBuf::from(".");
        let host = WebHostConfig::for_local_dev(root);
        let runtime = DesktopHostRuntime::new(
            host,
            AssistantConfig::new(
                "http://127.0.0.1:11434/v1".to_string(),
                None,
                "test-model".to_string(),
            ),
            Config::default(),
            SecurityConfig::default(),
        )
        .expect("desktop runtime");

        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        let response = rt.block_on(runtime.invoke("bootstrap.load", json!({})));
        assert!(response.ok);
        assert_eq!(response.status, 200);
        assert!(response.data.is_some());
    }

    #[test]
    fn desktop_runtime_exposes_frontend_meta() {
        let root = PathBuf::from(".");
        let host = WebHostConfig::for_local_dev(root);
        let runtime = DesktopHostRuntime::new(
            host,
            AssistantConfig::new(
                "http://127.0.0.1:11434/v1".to_string(),
                None,
                "test-model".to_string(),
            ),
            Config::default(),
            SecurityConfig::default(),
        )
        .expect("desktop runtime");

        let meta = runtime.frontend_host_meta();
        assert_eq!(meta.get("mode").and_then(|v| v.as_str()), Some("web"));
        assert_eq!(meta.get("transport").and_then(|v| v.as_str()), Some("http"));
    }
}
