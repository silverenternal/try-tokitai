//! Alibaba Cloud Coding Plan client
//!
//! Provides integration with Alibaba Cloud Model Studio (百炼) Coding Plan.
//! Uses the dedicated Base URL and API Key for the Coding Plan subscription.
//!
//! # Important
//!
//! You MUST use the Coding Plan exclusive Base URL and API Key.
//! Using standard Base URLs or API Keys will be charged via pay-as-you-go
//! instead of deducting from the plan quota.
//!
//! # Example
//!
//! ```rust,no_run
//! use tokitai_context::ai::clients::AliyunCodingPlanClient;
//! use tokitai_context::ai::client::LLMClient;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = AliyunCodingPlanClient::from_env();
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

/// Configuration for Alibaba Cloud Coding Plan client
#[derive(Debug, Clone)]
pub struct AliyunCodingPlanConfig {
    /// API key for authentication (Coding Plan exclusive)
    pub api_key: String,
    /// Base URL for Coding Plan (default: https://dashscope.aliyuncs.com/compatible-mode/v1)
    /// IMPORTANT: Must use the Coding Plan exclusive Base URL
    pub base_url: String,
    /// Model to use (default: qwen-plus)
    /// Supported models: qwen-plus, qwen-max, qwen-turbo, etc.
    pub model: String,
    /// Request timeout in milliseconds (default: 30000)
    pub timeout_ms: u64,
    /// Maximum retries for failed requests (default: 3)
    pub max_retries: u32,
}

impl Default for AliyunCodingPlanConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string(),
            model: "qwen-plus".to_string(),
            timeout_ms: 30000,
            max_retries: 3,
        }
    }
}

impl AliyunCodingPlanConfig {
    /// Create config from environment variable or .env file
    /// 
    /// This method will:
    /// 1. Try to load from .env file if it exists
    /// 2. Read from environment variable ALIYUN_CODING_PLAN_API_KEY or DASHSCOPE_API_KEY
    pub fn from_env() -> Self {
        // Load .env file if it exists (silently ignore errors)
        let _ = dotenvy::dotenv();

        Self {
            api_key: std::env::var("ALIYUN_CODING_PLAN_API_KEY")
                .or_else(|_| std::env::var("DASHSCOPE_API_KEY"))
                .unwrap_or_default(),
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

    /// Set the base URL
    pub fn with_base_url(mut self, base_url: &str) -> Self {
        self.base_url = base_url.to_string();
        self
    }
}

/// Alibaba Cloud Coding Plan client
pub struct AliyunCodingPlanClient {
    config: AliyunCodingPlanConfig,
    client: reqwest::Client,
    /// Circuit breaker for resilience
    circuit_breaker: TokioMutex<CircuitBreaker>,
    /// Client statistics
    stats: TokioMutex<ClientStats>,
    /// Retry configuration
    retry_config: RetryConfig,
}

/// Chat completion request (OpenAI-compatible format)
#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

/// Chat message for API
#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

/// Response format for JSON mode
#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

/// Chat completion response
#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

impl AliyunCodingPlanClient {
    /// Create a new client with default config
    pub fn new(api_key: &str) -> Self {
        Self::with_config(AliyunCodingPlanConfig::default().with_api_key(api_key))
    }

    /// Create a new client with custom config
    pub fn with_config(config: AliyunCodingPlanConfig) -> Self {
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

    /// Create a new client from environment variable
    pub fn from_env() -> Self {
        Self::with_config(AliyunCodingPlanConfig::from_env())
    }

    /// Get the client configuration
    pub fn config(&self) -> &AliyunCodingPlanConfig {
        &self.config
    }

    /// Build the authorization header value
    fn auth_header(&self) -> String {
        format!("Bearer {}", self.config.api_key)
    }

    /// Build the API URL
    fn api_url(&self) -> String {
        format!("{}/chat/completions", self.config.base_url)
    }

    /// Create a user message
    fn create_message(&self, content: &str) -> ChatMessage {
        ChatMessage {
            role: "user".to_string(),
            content: content.to_string(),
        }
    }

    /// Create a system message
    fn create_system_message(&self, content: &str) -> ChatMessage {
        ChatMessage {
            role: "system".to_string(),
            content: content.to_string(),
        }
    }

    /// Send a chat request with retry logic and circuit breaker
    async fn send_chat_request(&self, request: &ChatCompletionRequest) -> Result<String> {
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
                .json(request)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();

                    if status.is_success() {
                        let completion: ChatCompletionResponse = resp
                            .json()
                            .await
                            .context("Failed to parse response")?;

                        // Record success
                        {
                            let mut cb = self.circuit_breaker.lock().await;
                            cb.record_success();
                            let mut stats = self.stats.lock().await;
                            stats.successful_requests += 1;
                            stats.total_requests += 1;
                            stats.total_latency_ms += start_time.elapsed().as_millis() as u64;
                            if let Some(usage) = completion.usage {
                                stats.total_tokens += usage.total_tokens as u64;
                            }
                        }

                        return completion
                            .choices
                            .first()
                            .and_then(|c| c.message.content.clone())
                            .context("No content in response");
                    } else {
                        // Handle specific error codes
                        let error_text = resp.text().await.unwrap_or_default();

                        if status.as_u16() == 429 {
                            // Rate limit
                            warn!("Aliyun rate limit exceeded");
                            {
                                let mut stats = self.stats.lock().await;
                                stats.rate_limit_hits += 1;
                            }
                            last_error = Some(LLMError::RateLimitExceeded(error_text));
                        } else if status.as_u16() == 400 {
                            last_error = Some(LLMError::ApiError {
                                status: status.as_u16(),
                                message: format!("Bad request: {}", error_text)
                            });
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
impl BaseLLMClient for AliyunCodingPlanClient {
    /// Send a chat message and get response
    async fn chat(&self, prompt: &str) -> Result<String> {
        let messages = vec![
            self.create_system_message("You are a helpful AI assistant. Respond concisely and accurately."),
            self.create_message(prompt),
        ];

        let request = ChatCompletionRequest {
            model: self.config.model.clone(),
            messages,
            temperature: 0.7,
            response_format: None,
        };

        self.send_chat_request(&request).await
    }

    /// Send a chat message with JSON schema constraint
    async fn chat_with_schema(&self, prompt: &str, schema: &serde_json::Value) -> Result<String> {
        let messages = vec![
            self.create_system_message(
                "You are a helpful AI assistant. Respond ONLY with valid JSON matching the provided schema. Do not include any explanation or markdown formatting."
            ),
            self.create_message(&format!(
                "Schema:\n{}\n\nPrompt:\n{}",
                serde_json::to_string_pretty(schema)?,
                prompt
            )),
        ];

        let request = ChatCompletionRequest {
            model: self.config.model.clone(),
            messages,
            temperature: 0.3, // Lower temperature for more deterministic JSON
            response_format: Some(ResponseFormat {
                format_type: "json_object".to_string(),
            }),
        };

        let response = self.send_chat_request(&request).await?;

        // Validate response against schema
        SchemaValidator::validate(&response, schema)
            .map_err(|e| anyhow::anyhow!("Response schema validation failed: {}", e))?;

        Ok(response)
    }

    /// Send a chat message with timeout override
    async fn chat_with_timeout(&self, prompt: &str, timeout_ms: u64) -> Result<String> {
        let messages = vec![
            self.create_system_message("You are a helpful AI assistant. Respond concisely and accurately."),
            self.create_message(prompt),
        ];

        let request = ChatCompletionRequest {
            model: self.config.model.clone(),
            messages,
            temperature: 0.7,
            response_format: None,
        };

        // Use tokio::time::timeout to enforce the timeout
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
        // Try to get stats without blocking - in sync context
        match self.stats.try_lock() {
            Ok(stats) => stats.clone(),
            Err(_) => ClientStats::default(), // Return default if lock is contested
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = AliyunCodingPlanConfig::default();
        assert_eq!(config.model, "qwen-plus");
        assert_eq!(config.base_url, "https://dashscope.aliyuncs.com/compatible-mode/v1");
        assert_eq!(config.timeout_ms, 30000);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_config_builder() {
        let config = AliyunCodingPlanConfig::default()
            .with_model("qwen-max")
            .with_api_key("test-key")
            .with_base_url("https://custom-endpoint.com/v1");

        assert_eq!(config.model, "qwen-max");
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.base_url, "https://custom-endpoint.com/v1");
    }

    #[test]
    fn test_client_creation() {
        let client = AliyunCodingPlanClient::new("test-key");
        assert_eq!(client.config.api_key, "test-key");
    }

    #[test]
    fn test_client_with_config() {
        let config = AliyunCodingPlanConfig::default().with_api_key("test-key");
        let client = AliyunCodingPlanClient::with_config(config);
        assert_eq!(client.config.api_key, "test-key");
    }

    #[test]
    fn test_client_from_env() {
        // Set env var for test
        std::env::set_var("ALIYUN_CODING_PLAN_API_KEY", "test-env-key");
        let client = AliyunCodingPlanClient::from_env();
        assert_eq!(client.config.api_key, "test-env-key");
        std::env::remove_var("ALIYUN_CODING_PLAN_API_KEY");
    }

    #[test]
    fn test_model_name() {
        let config = AliyunCodingPlanConfig::default().with_model("qwen-turbo");
        let client = AliyunCodingPlanClient::with_config(config);
        assert_eq!(client.model_name(), "qwen-turbo");
    }

    #[test]
    fn test_stats_default() {
        let client = AliyunCodingPlanClient::new("test-key");
        let stats = client.get_stats();
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.successful_requests, 0);
        assert_eq!(stats.failed_requests, 0);
    }
}
