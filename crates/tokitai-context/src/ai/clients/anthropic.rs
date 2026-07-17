//! Anthropic API client implementation
//!
//! Provides integration with Anthropic's Messages API.
//! Supports Claude 3 family models (Opus, Sonnet, Haiku).
//!
//! # Example
//!
//! ```rust,no_run
//! use tokitai_context::ai::clients::AnthropicClient;
//! use tokitai_context::ai::client::LLMClient;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = AnthropicClient::new("your-api-key");
//!     let llm: Arc<dyn LLMClient> = Arc::new(client);
//!
//!     let response = llm.chat("Hello").await?;
//!     println!("{}", response);
//!     Ok(())
//! }
//! ```

use std::sync::Arc;
use std::time::{Duration, Instant};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn, info};
use tokio::sync::Mutex as TokioMutex;

use crate::ai::client::{LLMClient as BaseLLMClient, ClientStats, RetryConfig, CircuitBreaker, CircuitBreakerConfig, SchemaValidator, LLMError};

/// Configuration for Anthropic client
#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    /// API key for authentication
    pub api_key: String,
    /// Base URL (default: https://api.anthropic.com)
    pub base_url: String,
    /// Model to use (default: claude-3-haiku-20240307)
    pub model: String,
    /// Request timeout in milliseconds (default: 30000)
    pub timeout_ms: u64,
    /// Maximum retries for failed requests (default: 3)
    pub max_retries: u32,
    /// Maximum tokens in response (default: 4096)
    pub max_tokens: u32,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://api.anthropic.com".to_string(),
            model: "claude-3-haiku-20240307".to_string(),
            timeout_ms: 30000,
            max_retries: 3,
            max_tokens: 4096,
        }
    }
}

impl AnthropicConfig {
    /// Create config from environment variable or .env file
    /// 
    /// This method will:
    /// 1. Try to load from .env file if it exists
    /// 2. Read from environment variable ANTHROPIC_API_KEY
    pub fn from_env() -> Self {
        // Load .env file if it exists (silently ignore errors)
        let _ = dotenvy::dotenv();

        Self {
            api_key: std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            ..Default::default()
        }
    }

    /// Set the model
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    /// Set the API key
    pub fn with_api_key(mut self, api_key: &str) -> Self {
        self.api_key = api_key.to_string();
        self
    }
}

/// Anthropic API client
pub struct AnthropicClient {
    config: AnthropicConfig,
    client: reqwest::Client,
    /// Circuit breaker for resilience
    circuit_breaker: TokioMutex<CircuitBreaker>,
    /// Client statistics
    stats: TokioMutex<ClientStats>,
    /// Retry configuration
    retry_config: RetryConfig,
}

/// Anthropic messages request
#[derive(Debug, Serialize)]
struct MessagesRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

/// Message for Anthropic API
#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

/// Anthropic messages response
#[derive(Debug, Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
    stop_reason: Option<String>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
}

impl AnthropicClient {
    /// Create a new Anthropic client with default config
    pub fn new(api_key: &str) -> Self {
        Self::with_config(AnthropicConfig::default().with_api_key(api_key))
    }

    /// Create a new Anthropic client with custom config
    pub fn with_config(config: AnthropicConfig) -> Self {
        let timeout = std::time::Duration::from_millis(config.timeout_ms);
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to create HTTP client");

        let retry_config = RetryConfig {
            max_retries: config.max_retries,
            initial_delay_ms: 100,
            max_delay_ms: 10000,
            backoff_multiplier: 2.0,
        };

        Self {
            config,
            client,
            circuit_breaker: TokioMutex::new(CircuitBreaker::new(CircuitBreakerConfig::default())),
            stats: TokioMutex::new(ClientStats::default()),
            retry_config,
        }
    }

    /// Create a new Anthropic client from environment variable
    pub fn from_env() -> Self {
        Self::with_config(AnthropicConfig::from_env())
    }

    /// Get the client configuration
    pub fn config(&self) -> &AnthropicConfig {
        &self.config
    }

    /// Build the authorization header value
    fn auth_header(&self) -> String {
        format!("x-api-key {}", self.config.api_key)
    }

    /// Build the API URL
    fn api_url(&self) -> String {
        format!("{}/v1/messages", self.config.base_url)
    }

    /// Create a message
    fn create_message(&self, content: &str) -> AnthropicMessage {
        AnthropicMessage {
            role: "user".to_string(),
            content: content.to_string(),
        }
    }

    /// Send a chat request with retry logic and circuit breaker
    async fn send_chat_request(&self, request: &MessagesRequest) -> Result<String> {
        let mut last_error: Option<LLMError> = None;
        let start_time = Instant::now();

        // Check circuit breaker
        {
            let mut cb = self.circuit_breaker.lock().await;
            if !cb.can_execute() {
                return Err(LLMError::CircuitBreakerOpen(
                    "Circuit breaker is open, requests are temporarily blocked".to_string()
                ).into());
            }
        }

        for attempt in 0..self.retry_config.max_retries {
            let response = self
                .client
                .post(self.api_url())
                .header("Authorization", self.auth_header())
                .header("Content-Type", "application/json")
                .header("anthropic-version", "2023-06-01")
                .json(request)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();

                    if status.is_success() {
                        let result: MessagesResponse = resp
                            .json()
                            .await
                            .context("Failed to parse Anthropic response")?;

                        // Record success
                        {
                            let mut cb = self.circuit_breaker.lock().await;
                            cb.record_success();
                            let mut stats = self.stats.lock().await;
                            stats.successful_requests += 1;
                            stats.total_requests += 1;
                            stats.total_latency_ms += start_time.elapsed().as_millis() as u64;
                            if let Some(usage) = result.usage {
                                stats.total_tokens += (usage.input_tokens + usage.output_tokens) as u64;
                            }
                        }

                        return result
                            .content
                            .first()
                            .and_then(|c| c.text.clone())
                            .context("No content in response");
                    } else {
                        let error_text = resp.text().await.unwrap_or_default();

                        if status.as_u16() == 429 {
                            warn!("Anthropic rate limit exceeded");
                            {
                                let mut stats = self.stats.lock().await;
                                stats.rate_limit_hits += 1;
                            }
                            last_error = Some(LLMError::RateLimitExceeded(error_text));
                        } else {
                            last_error = Some(LLMError::ApiError {
                                status: status.as_u16(),
                                message: error_text
                            });
                        }
                    }
                }
                Err(e) => {
                    if e.is_timeout() {
                        last_error = Some(LLMError::Timeout { timeout_ms: self.config.timeout_ms });
                    } else {
                        last_error = Some(LLMError::ApiError {
                            status: 0,
                            message: format!("Request failed: {}", e)
                        });
                    }
                }
            }

            // Wait before retry (exponential backoff)
            if attempt < self.retry_config.max_retries - 1 {
                let delay = self.retry_config.delay_for_attempt(attempt);
                debug!("Retry attempt {} after {:?}", attempt + 1, delay);
                tokio::time::sleep(delay).await;
            }
        }

        // Record failure
        {
            let mut cb = self.circuit_breaker.lock().await;
            cb.record_failure();
            let mut stats = self.stats.lock().await;
            stats.failed_requests += 1;
            stats.total_requests += 1;
        }

        let last_error_msg = last_error.as_ref().map(|e| e.to_string()).unwrap_or_default();
        Err(LLMError::RetryExhausted {
            attempts: self.retry_config.max_retries,
            last_error: last_error_msg
        }.into())
    }
}

#[async_trait]
impl BaseLLMClient for AnthropicClient {
    /// Send a chat message and get response
    async fn chat(&self, prompt: &str) -> Result<String> {
        let messages = vec![self.create_message(prompt)];

        let request = MessagesRequest {
            model: self.config.model.clone(),
            messages,
            system: Some("You are a helpful AI assistant. Respond concisely and accurately.".to_string()),
            max_tokens: self.config.max_tokens,
            temperature: Some(0.7),
        };

        self.send_chat_request(&request).await
    }

    /// Send a chat message with JSON schema constraint
    async fn chat_with_schema(&self, prompt: &str, schema: &serde_json::Value) -> Result<String> {
        let messages = vec![self.create_message(&format!(
            "Schema:\n{}\n\nPrompt:\n{}\n\nRespond ONLY with valid JSON matching the schema. No markdown, no explanation.",
            serde_json::to_string_pretty(schema)?,
            prompt
        ))];

        let request = MessagesRequest {
            model: self.config.model.clone(),
            messages,
            system: Some("You are a JSON generator. Output ONLY valid JSON matching the provided schema. No explanations, no markdown formatting.".to_string()),
            max_tokens: self.config.max_tokens,
            temperature: Some(0.3),
        };

        let response = self.send_chat_request(&request).await?;

        // Validate response against schema
        SchemaValidator::validate(&response, schema)
            .map_err(|e| anyhow::anyhow!("Response schema validation failed: {}", e))?;

        Ok(response)
    }

    /// Send a chat message with timeout override
    async fn chat_with_timeout(&self, prompt: &str, timeout_ms: u64) -> Result<String> {
        let messages = vec![self.create_message(prompt)];

        let request = MessagesRequest {
            model: self.config.model.clone(),
            messages,
            system: Some("You are a helpful AI assistant. Respond concisely and accurately.".to_string()),
            max_tokens: self.config.max_tokens,
            temperature: Some(0.7),
        };

        tokio::time::timeout(Duration::from_millis(timeout_ms), self.send_chat_request(&request))
            .await
            .map_err(|_| LLMError::Timeout { timeout_ms })?
    }

    /// Get the model name
    fn model_name(&self) -> &str {
        &self.config.model
    }

    /// Get client statistics
    fn get_stats(&self) -> ClientStats {
        match self.stats.try_lock() {
            Ok(stats) => stats.clone(),
            Err(_) => ClientStats::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = AnthropicConfig::default();
        assert_eq!(config.model, "claude-3-haiku-20240307");
        assert_eq!(config.base_url, "https://api.anthropic.com");
        assert_eq!(config.timeout_ms, 30000);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.max_tokens, 4096);
    }

    #[test]
    fn test_config_builder() {
        let config = AnthropicConfig::default()
            .with_model("claude-3-opus-20240229")
            .with_api_key("test-key");

        assert_eq!(config.model, "claude-3-opus-20240229");
        assert_eq!(config.api_key, "test-key");
    }

    #[test]
    fn test_client_creation() {
        let client = AnthropicClient::new("test-key");
        assert_eq!(client.config.api_key, "test-key");
    }

    #[test]
    fn test_client_with_config() {
        let config = AnthropicConfig::default().with_api_key("test-key");
        let client = AnthropicClient::with_config(config);
        assert_eq!(client.config.api_key, "test-key");
    }
}
