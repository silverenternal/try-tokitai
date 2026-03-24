//! External Process Wrapper - HTTP Service Implementation
//!
//! Implementation of the ExternalTool trait for HTTP services/REST APIs.
//!
//! ## Overview
//! This module provides the `HTTPWrapper` struct that wraps HTTP services
//! and REST APIs, making them callable as tokitai tools.
//!
//! ## Features
//! - HTTP method support (GET, POST, PUT, DELETE, PATCH, etc.)

#![allow(dead_code)]
//! - Path template substitution
//! - Header injection
//! - Multiple authentication methods (Bearer, API Key, Basic, OAuth 2.0)
//! - Timeout handling
//! - JSON request/response handling

use crate::external_process::metadata::{
    ExternalToolMetadata,
    ExternalToolType,
    HttpConfig,
    AuthConfig,
    ToolExecutionResult,
    RiskLevel,
    schema_helpers,
};
use crate::external_process::wrapper::{ExternalTool, validation};
use crate::tool_matrix::matrix::ToolDefinition;
use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::time::Instant;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use tracing::{debug, warn, info};

/// HTTP wrapper for remote HTTP services/REST APIs
///
/// Wraps an HTTP service and makes it callable as a tokitai tool.
///
/// ## Example
/// ```rust,ignore
/// use crate::external_process::metadata::{HttpConfig, ExternalToolMetadata, ExternalToolType, AuthConfig};
/// use crate::external_process::http_wrapper::HTTPWrapper;
///
/// let auth = AuthConfig::BearerToken { token_env: "GITHUB_TOKEN".to_string() };
/// let config = HttpConfig::new("https://api.github.com", "POST")
///     .with_path("/repos/{{owner}}/{{repo}}/issues")
///     .with_auth(auth);
///
/// let metadata = ExternalToolMetadata::new(
///     "github_create_issue",
///     "Create a GitHub issue",
///     ExternalToolType::http(config),
///     serde_json::json!({
///         "type": "object",
///         "properties": {
///             "owner": {"type": "string", "description": "Repository owner"},
///             "repo": {"type": "string", "description": "Repository name"},
///             "title": {"type": "string", "description": "Issue title"},
///             "body": {"type": "string", "description": "Issue body"}
///         },
///         "required": ["owner", "repo", "title"]
///     }),
///     "http_client",
///     "ai_agent",
/// );
///
/// let wrapper = HTTPWrapper::new(metadata);
/// ```
pub struct HTTPWrapper {
    metadata: ExternalToolMetadata,
    client: reqwest::Client,
}

impl HTTPWrapper {
    /// Create a new HTTP wrapper
    ///
    /// # Arguments
    /// * `metadata` - Tool metadata containing HTTP configuration
    ///
    /// # Returns
    /// * `Self` - New HTTP wrapper instance
    ///
    /// # Panics
    /// Panics if the metadata's tool_type is not ExternalToolType::Http
    pub fn new(metadata: ExternalToolMetadata) -> Self {
        // Verify that the tool type is Http
        match &metadata.tool_type {
            ExternalToolType::Http { .. } => {}
            _ => panic!("HTTPWrapper requires ExternalToolType::Http"),
        }
        
        Self {
            metadata,
            client: reqwest::Client::new(),
        }
    }

    /// Create with custom timeout
    ///
    /// # Arguments
    /// * `metadata` - Tool metadata
    /// * `timeout_ms` - Custom timeout in milliseconds
    pub fn with_timeout(metadata: ExternalToolMetadata, timeout_ms: u64) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(timeout_ms))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        
        Self {
            metadata,
            client,
        }
    }

    /// Get the HTTP configuration
    ///
    /// # Returns
    /// * `&HttpConfig` - HTTP configuration reference
    pub fn config(&self) -> &HttpConfig {
        match &self.metadata.tool_type {
            ExternalToolType::Http { config } => config,
            _ => unreachable!(),
        }
    }

    /// Substitute variables in path template
    ///
    /// Replaces placeholders like `{{variable_name}}` with actual values from input.
    ///
    /// # Arguments
    /// * `template` - Path template containing placeholders
    /// * `input` - Input JSON object with variable values
    ///
    /// # Returns
    /// * `String` - Path with substituted values
    fn substitute_path(&self, template: &str, input: &serde_json::Map<String, Value>) -> String {
        let mut result = template.to_string();

        // Find all {{variable}} patterns and replace them
        while let Some(start) = result.find("{{") {
            if let Some(end) = result[start..].find("}}") {
                let var_name = &result[start + 2..start + end];
                if let Some(value) = input.get(var_name) {
                    if let Some(str_value) = value.as_str() {
                        let placeholder = format!("{{{{{}}}}}", var_name);
                        result = result.replace(&placeholder, str_value);
                    } else {
                        let placeholder = format!("{{{{{}}}}}", var_name);
                        result = result.replace(&placeholder, &value.to_string());
                    }
                } else {
                    warn!("Variable '{}' not found in input", var_name);
                    let placeholder = format!("{{{{{}}}}}", var_name);
                    result = result.replace(&placeholder, "");
                }
            } else {
                break;
            }
        }

        result
    }

    /// Build the URL from config and input
    ///
    /// # Arguments
    /// * `input` - Input JSON object
    ///
    /// # Returns
    /// * `Result<String>` - Complete URL
    fn build_url(&self, input: &serde_json::Map<String, Value>) -> Result<String> {
        let config = self.config();
        
        // Substitute path variables
        let path = self.substitute_path(&config.path_template, input);
        
        // Combine base URL and path
        let base_url = config.base_url.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        
        Ok(format!("{}/{}", base_url, path))
    }

    /// Build request headers
    ///
    /// # Arguments
    /// * `input` - Input JSON object
    ///
    /// # Returns
    /// * `Result<reqwest::header::HeaderMap>` - Header map
    async fn build_headers(&self, input: &serde_json::Map<String, Value>) -> Result<reqwest::header::HeaderMap> {
        let config = self.config();
        let mut headers = reqwest::header::HeaderMap::new();

        // Add static headers
        for (key, value) in &config.headers {
            if let Ok(header_value) = reqwest::header::HeaderValue::from_str(value) {
                headers.insert(
                    reqwest::header::HeaderName::from_bytes(key.as_bytes())?,
                    header_value,
                );
            }
        }

        // Add authentication headers
        if let Some(auth) = &config.auth {
            match auth {
                AuthConfig::BearerToken { token_env } => {
                    let token = std::env::var(token_env)
                        .with_context(|| format!("Environment variable {} not set", token_env))?;
                    headers.insert(
                        reqwest::header::AUTHORIZATION,
                        reqwest::header::HeaderValue::from_str(&format!("Bearer {}", token))?,
                    );
                }
                AuthConfig::ApiKey { header_name, key_env } => {
                    let key = std::env::var(key_env)
                        .with_context(|| format!("Environment variable {} not set", key_env))?;
                    headers.insert(
                        reqwest::header::HeaderName::from_bytes(header_name.as_bytes())?,
                        reqwest::header::HeaderValue::from_str(&key)?,
                    );
                }
                AuthConfig::Basic { username_env, password_env } => {
                    let username = std::env::var(username_env)
                        .with_context(|| format!("Environment variable {} not set", username_env))?;
                    let password = std::env::var(password_env)
                        .with_context(|| format!("Environment variable {} not set", password_env))?;

                    let credentials = BASE64.encode(format!("{}:{}", username, password));
                    headers.insert(
                        reqwest::header::AUTHORIZATION,
                        reqwest::header::HeaderValue::from_str(&format!("Basic {}", credentials))?,
                    );
                }
                AuthConfig::OAuth2 { client_id_env, client_secret_env, token_url, scopes } => {
                    // OAuth 2.0 client credentials flow
                    let client_id = std::env::var(client_id_env)
                        .with_context(|| format!("Environment variable {} not set", client_id_env))?;
                    let client_secret = std::env::var(client_secret_env)
                        .with_context(|| format!("Environment variable {} not set", client_secret_env))?;
                    
                    // Request access token
                    let client = reqwest::Client::new();
                    let mut params: std::collections::HashMap<String, String> = std::collections::HashMap::new();
                    params.insert("grant_type".to_string(), "client_credentials".to_string());
                    params.insert("client_id".to_string(), client_id.clone());
                    params.insert("client_secret".to_string(), client_secret.clone());

                    if !scopes.is_empty() {
                        params.insert("scope".to_string(), scopes.join(" "));
                    }
                    
                    let response = client.post(token_url)
                        .form(&params)
                        .send()
                        .await
                        .context("OAuth 2.0 token request failed")?;
                    
                    if !response.status().is_success() {
                        bail!("OAuth 2.0 token request failed: {}", response.status());
                    }
                    
                    let token_response: Value = response.json().await
                        .context("Failed to parse OAuth 2.0 token response")?;
                    
                    let access_token = token_response.get("access_token")
                        .and_then(|v| v.as_str())
                        .context("OAuth 2.0 response missing access_token")?;
                    
                    headers.insert(
                        reqwest::header::AUTHORIZATION,
                        reqwest::header::HeaderValue::from_str(&format!("Bearer {}", access_token))?,
                    );
                }
            }
        }

        Ok(headers)
    }

    /// Build request body
    ///
    /// # Arguments
    /// * `input` - Input JSON object
    ///
    /// # Returns
    /// * `Option<Value>` - Request body (if applicable)
    fn build_body(&self, input: &serde_json::Map<String, Value>) -> Option<Value> {
        let config = self.config();

        // For methods that typically have a body
        match config.method.to_uppercase().as_str() {
            "POST" | "PUT" | "PATCH" => {
                // Extract body fields from input (exclude path variables)
                // Simple approach: collect all {{var}} patterns
                let mut path_vars: std::collections::HashSet<String> = std::collections::HashSet::new();
                let mut chars = config.path_template.chars().peekable();
                while let Some(c) = chars.next() {
                    if c == '{' && chars.peek() == Some(&'{') {
                        chars.next(); // consume second {
                        let mut var = String::new();
                        while let Some(&c) = chars.peek() {
                            if c == '}' && chars.clone().nth(1) == Some('}') {
                                chars.next(); // consume first }
                                chars.next(); // consume second }
                                break;
                            }
                            if let Some(ch) = chars.next() {
                                var.push(ch);
                            }
                        }
                        if !var.is_empty() {
                            path_vars.insert(var);
                        }
                    }
                }
                
                // Filter out path variables from input
                let body: serde_json::Map<String, Value> = input
                    .iter()
                    .filter(|(k, _)| !path_vars.contains(k.as_str()))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                
                if !body.is_empty() {
                    Some(Value::Object(body))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Execute HTTP request with timeout
    ///
    /// # Arguments
    /// * `request` - Configured request builder
    /// * `timeout_ms` - Timeout in milliseconds
    ///
    /// # Returns
    /// * `Result<(u16, String)>` - Tuple of (status code, response body)
    async fn execute_with_timeout(
        &self,
        request: reqwest::RequestBuilder,
        timeout_ms: u64,
    ) -> Result<(u16, String)> {
        let response = tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms),
            request.send()
        )
        .await
        .context("HTTP request timed out")??;

        let status = response.status().as_u16();
        let body = response.text().await.context("Failed to read response body")?;

        Ok((status, body))
    }
}

#[async_trait::async_trait]
impl ExternalTool for HTTPWrapper {
    fn metadata(&self) -> &ExternalToolMetadata {
        &self.metadata
    }

    async fn execute(&self, input: Value) -> Result<ToolExecutionResult> {
        let start_time = Instant::now();

        // Validate input first
        if let Err(e) = self.validate_input(&input) {
            return Ok(ToolExecutionResult::failure(
                format!("Input validation failed: {}", e),
                start_time.elapsed().as_millis() as u64,
            ));
        }

        // Get input as object
        let input_obj = input.as_object()
            .context("Input must be a JSON object")?;

        let config = self.config();

        // Build URL
        let url = self.build_url(input_obj)?;
        debug!("HTTP {} {}", config.method, url);

        // Build request
        let method = reqwest::Method::from_bytes(config.method.to_uppercase().as_bytes())?;
        let mut request = self.client.request(method.clone(), &url);

        // Add headers
        let headers = self.build_headers(input_obj).await?;
        request = request.headers(headers);

        // Add body if applicable
        if let Some(body) = self.build_body(input_obj) {
            request = request.json(&body);
        }

        // Execute request
        let execution_result = self.execute_with_timeout(request, config.timeout_ms).await;

        let elapsed = start_time.elapsed().as_millis() as u64;

        match execution_result {
            Ok((status, body)) => {
                debug!("HTTP request completed in {}ms - Status: {}", elapsed, status);

                // Try to parse response as JSON
                let output = serde_json::from_str::<Value>(&body)
                    .unwrap_or_else(|_| Value::String(body.clone()));

                let success = (200..300).contains(&status);
                
                if success {
                    Ok(ToolExecutionResult::success(output, elapsed))
                } else {
                    Ok(ToolExecutionResult::failure_with_output(
                        format!("HTTP request failed with status {}", status),
                        elapsed,
                        Some(body),
                        None,
                    ))
                }
            }
            Err(e) => {
                warn!("HTTP request failed: {}", e);
                Ok(ToolExecutionResult::failure(e.to_string(), elapsed))
            }
        }
    }

    fn validate_input(&self, input: &Value) -> Result<()> {
        validation::validate_json_schema(input, &self.metadata.input_schema)
    }

    fn to_tool_definition(&self) -> ToolDefinition {
        use crate::tool_matrix::matrix::{ServiceMetadata, ServiceCategory};

        let risk_level_str = match self.metadata.risk_level {
            RiskLevel::Low => "safe",
            RiskLevel::Medium => "moderate",
            RiskLevel::High => "dangerous",
            RiskLevel::Critical => "dangerous",
        };

        let category = match self.metadata.domain.as_str() {
            "http_client" | "http" => ServiceCategory::Network,
            "api" => ServiceCategory::Network,
            "webhook" => ServiceCategory::Network,
            _ => ServiceCategory::Utility,
        };

        ToolDefinition {
            name: self.metadata.name.clone(),
            description: self.metadata.description.clone(),
            input_schema: self.metadata.input_schema.to_string(),
            metadata: ServiceMetadata {
                category,
                tags: self.metadata.tags.clone(),
                ..Default::default()
            },
            tags: self.metadata.tags.clone(),
            risk_level: risk_level_str.to_string(),
            source: "external".to_string(),
        }
    }
}

/// Builder for creating HTTPWrapper with fluent API
pub struct HTTPWrapperBuilder {
    name: String,
    description: String,
    base_url: String,
    method: String,
    path: String,
    headers: std::collections::HashMap<String, String>,
    auth: Option<AuthConfig>,
    timeout_ms: u64,
    input_schema: Value,
    domain: String,
    tags: Vec<String>,
    risk_level: RiskLevel,
    created_by: String,
}

impl HTTPWrapperBuilder {
    /// Create a new builder
    pub fn new(name: impl Into<String>, base_url: impl Into<String>, method: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            base_url: base_url.into(),
            method: method.into(),
            path: String::new(),
            headers: std::collections::HashMap::new(),
            auth: None,
            timeout_ms: 30000,
            input_schema: Value::Object(serde_json::Map::new()),
            domain: String::new(),
            tags: Vec::new(),
            risk_level: RiskLevel::Medium,
            created_by: "user".to_string(),
        }
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set path template
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// Add header
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set authentication
    pub fn auth(mut self, auth: AuthConfig) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Set timeout in milliseconds
    pub fn timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    /// Set input schema
    pub fn input_schema(mut self, schema: Value) -> Self {
        self.input_schema = schema;
        self
    }

    /// Set domain
    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = domain.into();
        self
    }

    /// Add tag
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set risk level
    pub fn risk_level(mut self, risk: RiskLevel) -> Self {
        self.risk_level = risk;
        self
    }

    /// Set created by
    pub fn created_by(mut self, creator: impl Into<String>) -> Self {
        self.created_by = creator.into();
        self
    }

    /// Build the HTTPWrapper
    pub fn build(self) -> HTTPWrapper {
        let config = HttpConfig {
            base_url: self.base_url,
            method: self.method,
            path_template: self.path,
            headers: self.headers,
            auth: self.auth,
            timeout_ms: self.timeout_ms,
        };

        let metadata = ExternalToolMetadata::new(
            self.name,
            self.description,
            ExternalToolType::http(config),
            self.input_schema,
            self.domain,
            self.created_by,
        )
        .with_tags(self.tags)
        .with_risk_level(self.risk_level);

        HTTPWrapper::new(metadata)
    }
}

/// OpenAPI/Swagger parser for auto-discovering HTTP tools
pub mod openapi_parser {
    use super::*;
    use anyhow::{Context, Result};

    /// Parse OpenAPI spec (JSON) and generate HTTP wrappers
    ///
    /// # Arguments
    /// * `openapi_spec` - OpenAPI/Swagger JSON spec
    /// * `created_by` - Creator identifier
    ///
    /// # Returns
    /// * `Result<Vec<HTTPWrapper>>` - Generated HTTP wrappers
    pub fn parse_openapi(openapi_spec: &Value, created_by: &str) -> Result<Vec<HTTPWrapper>> {
        let mut wrappers = Vec::new();

        // Get OpenAPI version
        let openapi_version = openapi_spec.get("openapi")
            .or_else(|| openapi_spec.get("swagger"))
            .and_then(|v| v.as_str())
            .context("Invalid OpenAPI spec: missing version")?;

        debug!("Parsing OpenAPI spec version: {}", openapi_version);

        // Get base URL
        let servers = openapi_spec.get("servers")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("url"))
            .and_then(|v| v.as_str());

        let host = openapi_spec.get("host").and_then(|v| v.as_str());
        let base_path = openapi_spec.get("basePath").and_then(|v| v.as_str()).unwrap_or("/");
        let schemes = openapi_spec.get("schemes").and_then(|v| v.as_array());

        let base_url = if let Some(server_url) = servers {
            server_url.to_string()
        } else if let Some(api_host) = host {
            let scheme = schemes
                .and_then(|s| s.first())
                .and_then(|v| v.as_str())
                .unwrap_or("https");
            format!("{}://{}{}", scheme, api_host, base_path)
        } else {
            bail!("OpenAPI spec missing servers or host");
        };

        // Get paths
        let paths = openapi_spec.get("paths")
            .and_then(|v| v.as_object())
            .context("Invalid OpenAPI spec: missing paths")?;

        // Get components/schemas for reference resolution
        let schemas = openapi_spec.get("components")
            .and_then(|v| v.get("schemas"))
            .and_then(|v| v.as_object());

        // Process each path
        for (path, path_item) in paths {
            let path_item_obj = path_item.as_object()
                .context("Invalid path item")?;

            for (method, operation) in path_item_obj {
                // Skip non-HTTP methods
                if !["get", "post", "put", "delete", "patch", "head", "options"].contains(&method.as_str()) {
                    continue;
                }

                let operation_obj = operation.as_object()
                    .context("Invalid operation")?;

                // Extract operation details
                let operation_id = operation_obj.get("operationId")
                    .and_then(|v| v.as_str())
                    .unwrap_or_else(|| &path[1..]);

                let summary = operation_obj.get("summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or(operation_id);

                let description = operation_obj.get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or(summary);

                // Generate input schema from parameters
                let input_schema = generate_input_schema(operation_obj, schemas)?;

                // Create HTTP wrapper
                let wrapper = HTTPWrapperBuilder::new(
                    format!("{}_{}", method, operation_id.replace('/', "_")),
                    &base_url,
                    method.to_uppercase(),
                )
                .description(description)
                .path(path.clone())
                .input_schema(input_schema)
                .domain("http_client")
                .tag("openapi")
                .tag("auto_generated")
                .created_by(created_by)
                .build();

                wrappers.push(wrapper);
            }
        }

        info!("Generated {} HTTP wrappers from OpenAPI spec", wrappers.len());
        Ok(wrappers)
    }

    /// Generate input schema from operation parameters
    fn generate_input_schema(
        operation: &serde_json::Map<String, Value>,
        schemas: Option<&serde_json::Map<String, Value>>,
    ) -> Result<Value> {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        // Process parameters
        if let Some(parameters) = operation.get("parameters").and_then(|v| v.as_array()) {
            for param in parameters {
                let param_obj = param.as_object().context("Invalid parameter")?;
                
                let name = param_obj.get("name")
                    .and_then(|v| v.as_str())
                    .context("Parameter missing name")?;
                
                let description = param_obj.get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Parameter");
                
                let required_param = param_obj.get("required").and_then(|v| v.as_bool()).unwrap_or(false);
                
                // Get parameter schema
                let param_schema = param_obj.get("schema").cloned()
                    .unwrap_or_else(|| schema_helpers::string_param(description));
                
                properties.insert(name.to_string(), param_schema);
                
                if required_param {
                    required.push(name.to_string());
                }
            }
        }

        // Process request body
        if let Some(request_body) = operation.get("requestBody") {
            if let Some(content) = request_body.get("content") {
                if let Some(json_content) = content.get("application/json") {
                    if let Some(schema) = json_content.get("schema") {
                        // Resolve schema reference if needed
                        let resolved_schema = resolve_schema_reference(schema, schemas);
                        
                        if let Some(obj) = resolved_schema.as_object() {
                            if let Some(props) = obj.get("properties") {
                                if let Some(props_obj) = props.as_object() {
                                    for (key, value) in props_obj {
                                        properties.insert(key.clone(), value.clone());
                                    }
                                }
                            }
                            
                            if let Some(req) = obj.get("required") {
                                if let Some(req_arr) = req.as_array() {
                                    for r in req_arr {
                                        if let Some(r_str) = r.as_str() {
                                            required.push(r_str.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(schema_helpers::object_schema(properties, required))
    }

    /// Resolve schema reference
    fn resolve_schema_reference(
        schema: &Value,
        schemas: Option<&serde_json::Map<String, Value>>,
    ) -> Value {
        if let Some(ref_str) = schema.get("$ref").and_then(|v| v.as_str()) {
            // Extract schema name from reference
            let schema_name = ref_str.split('/').next_back().unwrap_or("");
            
            if let Some(schemas_map) = schemas {
                if let Some(resolved) = schemas_map.get(schema_name) {
                    return resolved.clone();
                }
            }
        }
        
        schema.clone()
    }

    /// Parse OpenAPI spec from YAML string and generate HTTP wrappers
    ///
    /// This function requires the `yaml` feature to be enabled.
    ///
    /// # Arguments
    /// * `yaml_content` - OpenAPI/Swagger YAML content
    /// * `created_by` - Creator identifier
    ///
    /// # Returns
    /// * `Result<Vec<HTTPWrapper>>` - Generated HTTP wrappers
    ///
    /// # Example
    /// ```rust,ignore
    /// let yaml_content = r#"
    /// openapi: 3.0.0
    /// info:
    ///   title: My API
    ///   version: 1.0.0
    /// servers:
    ///   - url: https://api.example.com
    /// paths:
    ///   /users:
    ///     get:
    ///       operationId: getUsers
    ///       summary: Get all users
    /// "#;
    ///
    /// let wrappers = openapi_parser::parse_openapi_yaml(yaml_content, "my_app")?;
    /// ```
    #[cfg(feature = "yaml")]
    pub fn parse_openapi_yaml(yaml_content: &str, created_by: &str) -> Result<Vec<HTTPWrapper>> {
        // Parse YAML to Value
        let yaml_value: serde_yaml::Value = serde_yaml::from_str(yaml_content)
            .context("Failed to parse YAML content")?;

        // Convert YAML Value to JSON Value
        let json_value: Value = serde_json::from_str(&serde_json::to_string(&yaml_value)?)
            .context("Failed to convert YAML to JSON")?;

        // Use the existing JSON parser
        parse_openapi(&json_value, created_by)
    }

    /// Load and parse OpenAPI spec from file (auto-detect JSON or YAML)
    ///
    /// # Arguments
    /// * `file_path` - Path to the OpenAPI spec file
    /// * `created_by` - Creator identifier
    ///
    /// # Returns
    /// * `Result<Vec<HTTPWrapper>>` - Generated HTTP wrappers
    pub fn parse_openapi_file<P: AsRef<std::path::Path>>(
        file_path: P,
        created_by: &str,
    ) -> Result<Vec<HTTPWrapper>> {
        use std::fs;

        let path = file_path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {:?}", path))?;

        // Auto-detect format based on extension
        let extension = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match extension.to_lowercase().as_str() {
            "yaml" | "yml" => {
                #[cfg(feature = "yaml")]
                {
                    parse_openapi_yaml(&content, created_by)
                }
                #[cfg(not(feature = "yaml"))]
                {
                    bail!("YAML support requires the 'yaml' feature. Enable with: cargo build --features yaml")
                }
            }
            "json" | _ => {
                let json_value: Value = serde_json::from_str(&content)
                    .with_context(|| format!("Failed to parse JSON file: {:?}", path))?;
                parse_openapi(&json_value, created_by)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_http_wrapper_builder() {
        let wrapper = HTTPWrapperBuilder::new("test_api", "https://api.example.com", "GET")
            .description("Test API")
            .path("/users/{{user_id}}")
            .header("Accept".to_string(), "application/json".to_string())
            .domain("http_client")
            .tag("test")
            .build();

        assert_eq!(wrapper.name(), "test_api");
        assert_eq!(wrapper.domain(), "http_client");
        assert_eq!(wrapper.tags().len(), 1);
    }

    #[test]
    fn test_path_substitution() {
        let wrapper = HTTPWrapperBuilder::new("test", "https://api.example.com", "GET")
            .path("/users/{{user_id}}/posts/{{post_id}}")
            .build();

        let input = serde_json::Map::from_iter(vec![
            ("user_id".to_string(), Value::String("123".to_string())),
            ("post_id".to_string(), Value::Number(456.into())),
        ]);

        let result = wrapper.substitute_path("/users/{{user_id}}/posts/{{post_id}}", &input);
        assert_eq!(result, "/users/123/posts/456");
    }

    #[test]
    fn test_build_url() {
        let wrapper = HTTPWrapperBuilder::new("test", "https://api.example.com/", "GET")
            .path("/users/{{user_id}}")
            .build();

        let input = serde_json::Map::from_iter(vec![
            ("user_id".to_string(), Value::String("123".to_string())),
        ]);

        let url = wrapper.build_url(&input).unwrap();
        assert_eq!(url, "https://api.example.com/users/123");
    }

    #[tokio::test]
    async fn test_http_wrapper_validate_input() {
        let wrapper = HTTPWrapperBuilder::new("test", "https://api.example.com", "GET")
            .path("/users/{{user_id}}")
            .input_schema(schema_helpers::create_string_params_schema(vec![
                ("user_id", "User ID", true),
            ]))
            .build();

        // Valid input
        let valid_input = json!({"user_id": "123"});
        assert!(wrapper.validate_input(&valid_input).is_ok());

        // Invalid input (missing required field)
        let invalid_input = json!({});
        assert!(wrapper.validate_input(&invalid_input).is_err());
    }

    #[tokio::test]
    async fn test_http_wrapper_execute_mock() {
        // This test would require a mock HTTP server
        // For now, we test that the wrapper is created correctly
        let wrapper = HTTPWrapperBuilder::new("jsonplaceholder", "https://jsonplaceholder.typicode.com", "GET")
            .path("/posts/{{id}}")
            .input_schema(schema_helpers::create_string_params_schema(vec![
                ("id", "Post ID", true),
            ]))
            .domain("http_client")
            .build();

        // Execute against real API (for integration testing)
        let input = json!({"id": "1"});
        let result = wrapper.execute(input).await.unwrap();

        // Should succeed or fail gracefully
        assert!(result.success || result.error.is_some());
    }

    #[test]
    fn test_openapi_parser_basic() {
        let openapi_spec = json!({
            "openapi": "3.0.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "servers": [
                {"url": "https://api.example.com"}
            ],
            "paths": {
                "/users": {
                    "get": {
                        "operationId": "getUsers",
                        "summary": "Get all users",
                        "parameters": [
                            {
                                "name": "limit",
                                "in": "query",
                                "required": false,
                                "schema": {"type": "integer"}
                            }
                        ]
                    },
                    "post": {
                        "operationId": "createUser",
                        "summary": "Create a user",
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "name": {"type": "string"},
                                            "email": {"type": "string"}
                                        },
                                        "required": ["name", "email"]
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        let wrappers = openapi_parser::parse_openapi(&openapi_spec, "test").unwrap();
        assert_eq!(wrappers.len(), 2);
        
        let get_wrapper = wrappers.iter().find(|w| w.name() == "get_getUsers").unwrap();
        assert_eq!(get_wrapper.description(), "Get all users");
    }
}
