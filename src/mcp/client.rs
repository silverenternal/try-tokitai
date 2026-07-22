//! MCP client support for remote Streamable HTTP JSON-RPC servers.

use anyhow::{anyhow, bail, Context, Result};
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;

const MCP_PROTOCOL_VERSION: &str = "2025-03-26";

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct McpServerDescription {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub endpoint: String,
    #[serde(default = "default_transport")]
    pub transport: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bearer_token: Option<String>,
}

fn default_transport() -> String {
    "streamable_http".to_string()
}

impl McpServerDescription {
    pub fn normalized(mut self) -> Result<Self> {
        self.id = sanitize_identifier(if self.id.trim().is_empty() {
            &self.name
        } else {
            &self.id
        });
        self.name = self.name.trim().chars().take(80).collect();
        self.description = self.description.trim().chars().take(500).collect();
        self.endpoint = self.endpoint.trim().trim_end_matches('/').to_string();
        self.transport = self.transport.trim().to_ascii_lowercase();
        self.bearer_token = self
            .bearer_token
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        if self.id.is_empty() || self.name.is_empty() {
            bail!("MCP server id and name are required");
        }
        if self.transport != "streamable_http" && self.transport != "http" {
            bail!("unsupported MCP transport '{}'; use streamable_http", self.transport);
        }
        let url = reqwest::Url::parse(&self.endpoint)
            .with_context(|| format!("invalid MCP endpoint: {}", self.endpoint))?;
        if !matches!(url.scheme(), "http" | "https") {
            bail!("MCP endpoint must use http or https");
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DiscoveredTool {
    pub name: String,
    pub remote_name: String,
    pub description: String,
    pub input_schema: Value,
    pub server_id: String,
    pub server_name: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct McpServerStatus {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub connected: bool,
    pub tool_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct McpSnapshot {
    pub servers: Vec<McpServerDescription>,
    pub statuses: Vec<McpServerStatus>,
    pub tools: Vec<DiscoveredTool>,
}

#[derive(Clone)]
pub struct McpClient {
    http: reqwest::Client,
    servers: HashMap<String, McpServerDescription>,
    discovered_tools: HashMap<String, DiscoveredTool>,
    statuses: HashMap<String, McpServerStatus>,
    sessions: HashMap<String, String>,
}

impl McpClient {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            http,
            servers: HashMap::new(),
            discovered_tools: HashMap::new(),
            statuses: HashMap::new(),
            sessions: HashMap::new(),
        }
    }

    pub fn load_servers(&mut self, servers: Vec<McpServerDescription>) -> Result<()> {
        let mut normalized = HashMap::new();
        for server in servers {
            let server = server.normalized()?;
            if normalized.insert(server.id.clone(), server).is_some() {
                bail!("duplicate MCP server id");
            }
        }
        self.servers = normalized;
        self.discovered_tools.clear();
        self.statuses.clear();
        self.sessions.clear();
        for (id, server) in &self.servers {
            self.statuses.insert(
                id.clone(),
                McpServerStatus {
                    id: id.clone(),
                    name: server.name.clone(),
                    enabled: server.enabled,
                    ..McpServerStatus::default()
                },
            );
        }
        Ok(())
    }

    pub async fn configure(&mut self, servers: Vec<McpServerDescription>) -> Result<McpSnapshot> {
        self.load_servers(servers)?;
        let ids = self.servers.keys().cloned().collect::<Vec<_>>();
        for id in ids {
            if self.servers.get(&id).is_some_and(|server| server.enabled) {
                if let Err(error) = self.refresh_server(&id).await {
                    self.record_error(&id, error.to_string());
                }
            } else if let Some(server) = self.servers.get(&id) {
                self.statuses.insert(
                    id.clone(),
                    McpServerStatus {
                        id,
                        name: server.name.clone(),
                        enabled: false,
                        ..McpServerStatus::default()
                    },
                );
            }
        }
        Ok(self.snapshot())
    }

    pub async fn test_server(&self, server: McpServerDescription) -> Result<Vec<DiscoveredTool>> {
        let server = server.normalized()?;
        let session = self.initialize(&server).await?;
        self.fetch_tools(&server, session.as_deref()).await
    }

    pub async fn refresh_server(&mut self, server_id: &str) -> Result<Vec<DiscoveredTool>> {
        let server = self
            .servers
            .get(server_id)
            .cloned()
            .with_context(|| format!("MCP server not found: {}", server_id))?;
        if !server.enabled {
            bail!("MCP server '{}' is disabled", server.name);
        }
        let session = self.initialize(&server).await?;
        let tools = self.fetch_tools(&server, session.as_deref()).await?;
        if let Some(session) = session {
            self.sessions.insert(server.id.clone(), session);
        }
        self.discovered_tools
            .retain(|_, tool| tool.server_id != server.id);
        for tool in &tools {
            self.discovered_tools.insert(tool.name.clone(), tool.clone());
        }
        self.statuses.insert(
            server.id.clone(),
            McpServerStatus {
                id: server.id.clone(),
                name: server.name.clone(),
                enabled: true,
                connected: true,
                tool_count: tools.len(),
                error: None,
            },
        );
        Ok(tools)
    }

    pub async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<Value> {
        let tool = self
            .discovered_tools
            .get(tool_name)
            .with_context(|| format!("MCP tool not found: {}", tool_name))?;
        let server = self
            .servers
            .get(&tool.server_id)
            .with_context(|| format!("MCP server not found: {}", tool.server_id))?;
        let result = self
            .rpc(
                server,
                3,
                "tools/call",
                json!({"name": tool.remote_name, "arguments": arguments}),
                self.sessions.get(&server.id).map(String::as_str),
            )
            .await?
            .result;
        Ok(result)
    }

    pub fn tool_definitions(&self) -> Vec<Value> {
        let mut tools = self.discovered_tools.values().cloned().collect::<Vec<_>>();
        tools.sort_by(|left, right| left.name.cmp(&right.name));
        tools
            .into_iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": format!("{}\nProvided by MCP server '{}'.", tool.description, tool.server_name),
                        "parameters": tool.input_schema,
                    }
                })
            })
            .collect()
    }

    pub fn snapshot(&self) -> McpSnapshot {
        let mut servers = self.servers.values().cloned().collect::<Vec<_>>();
        servers.sort_by(|left, right| left.name.cmp(&right.name));
        let mut statuses = self.statuses.values().cloned().collect::<Vec<_>>();
        statuses.sort_by(|left, right| left.name.cmp(&right.name));
        let mut tools = self.discovered_tools.values().cloned().collect::<Vec<_>>();
        tools.sort_by(|left, right| left.name.cmp(&right.name));
        McpSnapshot {
            servers,
            statuses,
            tools,
        }
    }

    fn record_error(&mut self, server_id: &str, error: String) {
        if let Some(server) = self.servers.get(server_id) {
            self.statuses.insert(
                server_id.to_string(),
                McpServerStatus {
                    id: server_id.to_string(),
                    name: server.name.clone(),
                    enabled: server.enabled,
                    connected: false,
                    tool_count: 0,
                    error: Some(error),
                },
            );
        }
    }

    async fn initialize(&self, server: &McpServerDescription) -> Result<Option<String>> {
        let response = self.rpc(
            server,
            1,
            "initialize",
            json!({
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "atlas", "version": env!("CARGO_PKG_VERSION")}
            }),
            None,
        )
        .await?;
        self.notify_initialized(server, response.session.as_deref()).await?;
        Ok(response.session)
    }

    async fn notify_initialized(
        &self,
        server: &McpServerDescription,
        session: Option<&str>,
    ) -> Result<()> {
        let mut request = self
            .http
            .post(&server.endpoint)
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json, text/event-stream")
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized",
                "params": {}
            }));
        request = add_request_auth(request, server, session);
        let response = request.send().await?;
        if !response.status().is_success() {
            bail!("MCP initialized notification returned HTTP {}", response.status());
        }
        Ok(())
    }

    async fn fetch_tools(
        &self,
        server: &McpServerDescription,
        session: Option<&str>,
    ) -> Result<Vec<DiscoveredTool>> {
        let result = self
            .rpc(server, 2, "tools/list", json!({}), session)
            .await?
            .result;
        let raw_tools = result
            .get("tools")
            .and_then(Value::as_array)
            .context("MCP tools/list response is missing result.tools")?;
        let mut tools = Vec::new();
        for raw in raw_tools {
            let remote_name = raw
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim();
            if remote_name.is_empty() {
                continue;
            }
            tools.push(DiscoveredTool {
                name: format!(
                    "mcp__{}__{}",
                    sanitize_identifier(&server.id),
                    sanitize_identifier(remote_name)
                ),
                remote_name: remote_name.to_string(),
                description: raw
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("MCP tool")
                    .to_string(),
                input_schema: raw
                    .get("inputSchema")
                    .or_else(|| raw.get("input_schema"))
                    .cloned()
                    .unwrap_or_else(|| json!({"type":"object","properties":{}})),
                server_id: server.id.clone(),
                server_name: server.name.clone(),
            });
        }
        Ok(tools)
    }

    async fn rpc(
        &self,
        server: &McpServerDescription,
        id: u64,
        method: &str,
        params: Value,
        session: Option<&str>,
    ) -> Result<McpRpcResponse> {
        let mut request = self
            .http
            .post(&server.endpoint)
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json, text/event-stream")
            .json(&json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params,
            }));
        request = add_request_auth(request, server, session);
        let response = request.send().await.with_context(|| {
            format!("MCP {} request failed for {}", method, server.endpoint)
        })?;
        let status = response.status();
        let response_session = response
            .headers()
            .get("mcp-session-id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let body = response.text().await?;
        if !status.is_success() {
            bail!("MCP {} returned HTTP {}: {}", method, status, truncate(&body, 1000));
        }
        let envelope = if content_type.contains("text/event-stream") {
            parse_sse_json(&body)?
        } else {
            serde_json::from_str::<Value>(&body)
                .with_context(|| format!("MCP {} returned invalid JSON", method))?
        };
        if let Some(error) = envelope.get("error") {
            bail!("MCP {} error: {}", method, error);
        }
        let result = envelope
            .get("result")
            .cloned()
            .ok_or_else(|| anyhow!("MCP {} response is missing result", method))?;
        Ok(McpRpcResponse {
            result,
            session: response_session.or_else(|| session.map(str::to_string)),
        })
    }
}

struct McpRpcResponse {
    result: Value,
    session: Option<String>,
}

fn add_request_auth(
    mut request: reqwest::RequestBuilder,
    server: &McpServerDescription,
    session: Option<&str>,
) -> reqwest::RequestBuilder {
    if let Some(token) = server.bearer_token.as_deref() {
        request = request.header(AUTHORIZATION, format!("Bearer {}", token));
    }
    if let Some(session) = session {
        request = request.header("mcp-session-id", session);
    }
    request
}

impl Default for McpClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct McpClientManager {
    client: McpClient,
}

impl McpClientManager {
    pub fn new() -> Self {
        Self {
            client: McpClient::new(),
        }
    }

    pub async fn initialize(&mut self, config_path: Option<&str>) -> Result<()> {
        let servers = config_path
            .map(McpClient::load_from_config)
            .transpose()?
            .unwrap_or_default();
        self.client.configure(servers).await?;
        Ok(())
    }

    pub fn client(&self) -> &McpClient {
        &self.client
    }

    pub fn client_mut(&mut self) -> &mut McpClient {
        &mut self.client
    }
}

impl McpClient {
    pub fn load_from_config(config_path: &str) -> Result<Vec<McpServerDescription>> {
        let content = std::fs::read_to_string(config_path)
            .with_context(|| format!("failed to read MCP config: {}", config_path))?;
        serde_json::from_str(&content).context("failed to parse MCP config JSON")
    }
}

impl Default for McpClientManager {
    fn default() -> Self {
        Self::new()
    }
}

fn sanitize_identifier(input: &str) -> String {
    let mut output = String::new();
    let mut previous_underscore = false;
    for character in input.trim().to_ascii_lowercase().chars() {
        let normalized = if character.is_ascii_alphanumeric() {
            character
        } else {
            '_'
        };
        if normalized == '_' && previous_underscore {
            continue;
        }
        previous_underscore = normalized == '_';
        output.push(normalized);
    }
    output.trim_matches('_').chars().take(64).collect()
}

fn parse_sse_json(body: &str) -> Result<Value> {
    for line in body.lines().rev() {
        if let Some(data) = line.trim().strip_prefix("data:") {
            let data = data.trim();
            if !data.is_empty() && data != "[DONE]" {
                if let Ok(value) = serde_json::from_str(data) {
                    return Ok(value);
                }
            }
        }
    }
    bail!("MCP SSE response did not contain a JSON data event")
}

fn truncate(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::post;
    use axum::{Json, Router};

    #[test]
    fn namespaced_tool_names_are_stable() {
        assert_eq!(sanitize_identifier(" Files & Search "), "files_search");
    }

    #[tokio::test]
    async fn client_starts_empty() {
        let client = McpClient::new();
        assert!(client.snapshot().servers.is_empty());
    }

    #[tokio::test]
    async fn streamable_http_discovers_and_calls_tools() {
        async fn handler(Json(payload): Json<Value>) -> Json<Value> {
            let id = payload.get("id").cloned().unwrap_or(Value::Null);
            let result = match payload.get("method").and_then(Value::as_str) {
                Some("initialize") => json!({
                    "protocolVersion": MCP_PROTOCOL_VERSION,
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "fixture", "version": "1"}
                }),
                Some("tools/list") => json!({"tools":[{
                    "name":"echo",
                    "description":"Echo text",
                    "inputSchema":{"type":"object","properties":{"text":{"type":"string"}},"required":["text"]}
                }]}),
                Some("tools/call") => json!({
                    "content":[{"type":"text","text":payload["params"]["arguments"]["text"]}]
                }),
                _ => Value::Null,
            };
            Json(json!({"jsonrpc":"2.0","id":id,"result":result}))
        }

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, Router::new().route("/mcp", post(handler)))
                .await
                .unwrap();
        });
        let mut client = McpClient::new();
        let snapshot = client
            .configure(vec![McpServerDescription {
                id: "fixture".to_string(),
                name: "Fixture".to_string(),
                description: String::new(),
                endpoint: format!("http://{}/mcp", address),
                transport: "streamable_http".to_string(),
                enabled: true,
                bearer_token: None,
            }])
            .await
            .unwrap();
        assert_eq!(snapshot.tools[0].name, "mcp__fixture__echo");
        assert!(client
            .tool_definitions()
            .iter()
            .any(|definition| definition["function"]["name"] == "mcp__fixture__echo"));
        let result = client
            .call_tool("mcp__fixture__echo", json!({"text":"MCP_OK"}))
            .await
            .unwrap();
        assert_eq!(result["content"][0]["text"], "MCP_OK");
        server.abort();
    }
}
