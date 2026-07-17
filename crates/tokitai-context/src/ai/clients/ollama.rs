//! Ollama API client implementation
//!
//! Provides integration with self-hosted Ollama instances.
//! Supports any model available in Ollama (Llama, Mistral, etc.).
//!
//! # Example
//!
//! ```rust,no_run
//! use tokitai_context::ai::clients::OllamaClient;
//! use tokitai_context::ai::client::LLMClient;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = OllamaClient::new("http://localhost:11434", "llama3.1");
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

/// Configuration for Ollama client
#[derive(Debug, Clone)]
pub struct OllamaConfig {
    /// Base URL (default: http://localhost:11434)
    pub base_url: String,
    /// Model to use (default: llama3.1)
    pub model: String,
    /// Request timeout in milliseconds (default: 60000)
    pub timeout_ms: u64,
    /// Maximum retries for failed requests (default: 3)
    pub max_retries: u32,
    /// Maximum tokens in response (default: 4096)
    pub max_tokens: u32,
    /// Temperature (default: 0.7)
    pub temperature: f32,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            model: "llama3.1".to_string(),
            timeout_ms: 60000,
            max_retries: 3,
            max_tokens: 4096,
            temperature: 0.7,
        }
    }
}

impl OllamaConfig {
    /// Create new config with specified base URL and model
    pub fn new(base_url: &str, model: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            model: model.to_string(),
            ..Default::default()
        }
    }

    /// Create config from environment variable or .env file
    /// 
    /// This method will:
    /// 1. Try to load from .env file if it exists
    /// 2. Read from environment variables OLLAMA_BASE_URL and OLLAMA_MODEL
    pub fn from_env() -> Self {
        // Load .env file if it exists (silently ignore errors)
        let _ = dotenvy::dotenv();

        Self {
            base_url: std::env::var("OLLAMA_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:11434".to_string()),
            model: std::env::var("OLLAMA_MODEL")
                .unwrap_or_else(|_| "llama3.1".to_string()),
            ..Default::default()
        }
    }

    /// Set the model
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    /// Set the base URL
    pub fn with_base_url(mut self, base_url: &str) -> Self {
        self.base_url = base_url.to_string();
        self
    }
}

/// Ollama API client
pub struct OllamaClient {
    config: OllamaConfig,
    client: reqwest::Client,
    /// Circuit breaker for resilience
    circuit_breaker: TokioMutex<CircuitBreaker>,
    /// Client statistics
    stats: TokioMutex<ClientStats>,
    /// Retry configuration
    retry_config: RetryConfig,
}

/// Ollama chat request
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    options: Option<ChatOptions>,
}

/// Message for Ollama API
#[derive(Debug, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

/// Chat options for Ollama
#[derive(Debug, Serialize)]
struct ChatOptions {
    temperature: f32,
    num_predict: u32,
}

/// Ollama chat response
#[derive(Debug, Deserialize)]
struct ChatResponse {
    message: OllamaMessage,
    done: bool,
}

impl OllamaClient {
    /// Create a new Ollama client with default config
    pub fn new(base_url: &str, model: &str) -> Self {
        Self::with_config(OllamaConfig::new(base_url, model))
    }

    /// Create a new Ollama client with custom config
    pub fn with_config(config: OllamaConfig) -> Self {
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

    /// Get the client configuration
    pub fn config(&self) -> &OllamaConfig {
        &self.config
    }

    /// Build the API URL
    fn api_url(&self) -> String {
        format!("{}/api/chat", self.config.base_url)
    }

    /// Create a message
    fn create_message(&self, content: &str) -> OllamaMessage {
        OllamaMessage {
            role: "user".to_string(),
            content: content.to_string(),
        }
    }

    /// Create a system message
    fn create_system_message(&self, content: &str) -> OllamaMessage {
        OllamaMessage {
            role: "system".to_string(),
            content: content.to_string(),
        }
    }

    /// Send a chat request with retry logic and circuit breaker
    async fn send_chat_request(&self, request: &ChatRequest) -> Result<String> {
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
                .header("Content-Type", "application/json")
                .json(request)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();

                    if status.is_success() {
                        let result: ChatResponse = resp
                            .json()
                            .await
                            .context("Failed to parse Ollama response")?;

                        // Record success
                        {
                            let mut cb = self.circuit_breaker.lock().await;
                            cb.record_success();
                            let mut stats = self.stats.lock().await;
                            stats.successful_requests += 1;
                            stats.total_requests += 1;
                            stats.total_latency_ms += start_time.elapsed().as_millis() as u64;
                        }

                        return Ok(result.message.content);
                    } else {
                        let error_text = resp.text().await.unwrap_or_default();
                        last_error = Some(LLMError::ApiError {
                            status: status.as_u16(),
                            message: error_text
                        });
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
impl BaseLLMClient for OllamaClient {
    /// Send a chat message and get response
    async fn chat(&self, prompt: &str) -> Result<String> {
        let messages = vec![
            self.create_system_message("You are a helpful AI assistant. Respond concisely and accurately."),
            self.create_message(prompt),
        ];

        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            stream: false,
            options: Some(ChatOptions {
                temperature: self.config.temperature,
                num_predict: self.config.max_tokens,
            }),
        };

        self.send_chat_request(&request).await
    }

    /// Send a chat message with JSON schema constraint
    async fn chat_with_schema(&self, prompt: &str, schema: &serde_json::Value) -> Result<String> {
        let messages = vec![
            self.create_system_message(
                "You are a JSON generator. Output ONLY valid JSON matching the provided schema. No explanations, no markdown formatting."
            ),
            self.create_message(&format!(
                "Schema:\n{}\n\nPrompt:\n{}",
                serde_json::to_string_pretty(schema)?,
                prompt
            )),
        ];

        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            stream: false,
            options: Some(ChatOptions {
                temperature: 0.3, // Lower temperature for more deterministic JSON
                num_predict: self.config.max_tokens,
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

        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            stream: false,
            options: Some(ChatOptions {
                temperature: self.config.temperature,
                num_predict: self.config.max_tokens,
            }),
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
        let config = OllamaConfig::default();
        assert_eq!(config.model, "llama3.1");
        assert_eq!(config.base_url, "http://localhost:11434");
        assert_eq!(config.timeout_ms, 60000);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.max_tokens, 4096);
        assert!((config.temperature - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_config_builder() {
        let config = OllamaConfig::default()
            .with_model("mistral")
            .with_base_url("http://127.0.0.1:11434");

        assert_eq!(config.model, "mistral");
        assert_eq!(config.base_url, "http://127.0.0.1:11434");
    }

    #[test]
    fn test_client_creation() {
        let client = OllamaClient::new("http://localhost:11434", "llama3.1");
        assert_eq!(client.config.model, "llama3.1");
        assert_eq!(client.config.base_url, "http://localhost:11434");
    }

    #[test]
    fn test_client_with_config() {
        let config = OllamaConfig::default().with_model("mistral");
        let client = OllamaClient::with_config(config);
        assert_eq!(client.config.model, "mistral");
    }
}
