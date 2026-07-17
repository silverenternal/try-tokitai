//! OpenAI API client implementation
//!
//! Provides integration with OpenAI's Chat Completions API.
//! Supports GPT-4, GPT-3.5-turbo, and compatible models.
//!
//! # Example
//!
//! ```rust,no_run
//! use tokitai_context::ai::clients::OpenAIClient;
//! use tokitai_context::ai::client::LLMClient;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = OpenAIClient::new("your-api-key");
//!     let llm: Arc<dyn LLMClient> = Arc::new(client);
//!
//!     // Use the client directly
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

/// Configuration for OpenAI client
#[derive(Debug, Clone)]
pub struct OpenAIConfig {
    /// API key for authentication
    pub api_key: String,
    /// Base URL (default: https://api.openai.com/v1)
    pub base_url: String,
    /// Model to use (default: gpt-4o-mini)
    pub model: String,
    /// Request timeout in milliseconds (default: 30000)
    pub timeout_ms: u64,
    /// Maximum retries for failed requests (default: 3)
    pub max_retries: u32,
}

impl Default for OpenAIConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4o-mini".to_string(),
            timeout_ms: 30000,
            max_retries: 3,
        }
    }
}

impl OpenAIConfig {
    /// Create config from environment variable or .env file
    /// 
    /// This method will:
    /// 1. Try to load from .env file if it exists
    /// 2. Read from environment variable OPENAI_API_KEY
    pub fn from_env() -> Self {
        // Load .env file if it exists (silently ignore errors)
        let _ = dotenvy::dotenv();

        Self {
            api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
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

/// OpenAI API client
pub struct OpenAIClient {
    config: OpenAIConfig,
    client: reqwest::Client,
    /// Circuit breaker for resilience
    circuit_breaker: TokioMutex<CircuitBreaker>,
    /// Client statistics
    stats: TokioMutex<ClientStats>,
    /// Retry configuration
    retry_config: RetryConfig,
}

/// OpenAI chat completion request
#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

/// Chat message for OpenAI API
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

/// OpenAI chat completion response
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

impl OpenAIClient {
    /// Create a new OpenAI client with default config
    pub fn new(api_key: &str) -> Self {
        Self::with_config(OpenAIConfig::default().with_api_key(api_key))
    }

    /// Create a new OpenAI client with custom config
    pub fn with_config(config: OpenAIConfig) -> Self {
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

    /// Create a new OpenAI client from environment variable
    pub fn from_env() -> Self {
        Self::with_config(OpenAIConfig::from_env())
    }

    /// Get the client configuration
    pub fn config(&self) -> &OpenAIConfig {
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

    /// Create a chat message
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
                            .context("Failed to parse OpenAI response")?;

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
                            warn!("OpenAI rate limit exceeded");
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
impl BaseLLMClient for OpenAIClient {
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
        // OpenAI supports JSON mode but not full schema validation
        // We use JSON mode and validate the response
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
        // Create a request with custom timeout
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
        let config = OpenAIConfig::default();
        assert_eq!(config.model, "gpt-4o-mini");
        assert_eq!(config.base_url, "https://api.openai.com/v1");
        assert_eq!(config.timeout_ms, 30000);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_config_builder() {
        let config = OpenAIConfig::default()
            .with_model("gpt-4-turbo")
            .with_api_key("test-key");

        assert_eq!(config.model, "gpt-4-turbo");
        assert_eq!(config.api_key, "test-key");
    }

    #[test]
    fn test_client_creation() {
        let client = OpenAIClient::new("test-key");
        assert_eq!(client.config.api_key, "test-key");
    }

    #[test]
    fn test_client_with_config() {
        let config = OpenAIConfig::default().with_api_key("test-key");
        let client = OpenAIClient::with_config(config);
        assert_eq!(client.config.api_key, "test-key");
    }
}
