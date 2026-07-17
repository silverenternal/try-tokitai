//! Unified LLM client trait and common utilities
//!
//! This module provides a single, unified `LLMClient` trait that all AI
//! components use, eliminating trait fragmentation across resolver, purpose,
//! smart_merge, and summarizer modules.
//!
//! # Example
//!
//! ```rust,no_run
//! use tokitai_context::ai::client::{LLMClient, RetryConfig};
//! use tokitai_context::ai::clients::OpenAIClient;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = OpenAIClient::from_env();
//!     let llm: Arc<dyn LLMClient> = Arc::new(client);
//!
//!     // Use the unified trait
//!     let response = llm.chat("Hello").await?;
//!     println!("{}", response);
//!
//!     Ok(())
//! }
//! ```

use std::sync::Arc;
use std::time::Duration;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn, info};
use tokio::sync::Mutex as TokioMutex;

/// Unified LLM client trait for all AI components
#[async_trait]
pub trait LLMClient: Send + Sync {
    /// Send a chat message and get response
    async fn chat(&self, prompt: &str) -> Result<String>;

    /// Send a chat message with JSON schema constraint
    async fn chat_with_schema(
        &self,
        prompt: &str,
        schema: &serde_json::Value,
    ) -> Result<String>;

    /// Send a chat message with timeout override
    async fn chat_with_timeout(&self, prompt: &str, timeout_ms: u64) -> Result<String>;

    /// Get the model name
    fn model_name(&self) -> &str;

    /// Get client statistics
    fn get_stats(&self) -> ClientStats;
}

/// Client statistics for observability
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientStats {
    /// Total requests made
    pub total_requests: u32,
    /// Successful requests
    pub successful_requests: u32,
    /// Failed requests
    pub failed_requests: u32,
    /// Rate limit hits
    pub rate_limit_hits: u32,
    /// Total tokens used (approximate)
    pub total_tokens: u64,
    /// Total latency in milliseconds (for computing average)
    pub total_latency_ms: u64,
}

impl ClientStats {
    /// Average latency in milliseconds
    pub fn avg_latency_ms(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        self.total_latency_ms as f64 / self.total_requests as f64
    }

    /// Success rate (0.0 - 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 1.0;
        }
        self.successful_requests as f64 / self.total_requests as f64
    }

    /// Failure rate (0.0 - 1.0)
    pub fn failure_rate(&self) -> f64 {
        1.0 - self.success_rate()
    }
}

/// Retry configuration with exponential backoff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retries (default: 3)
    pub max_retries: u32,
    /// Initial delay in milliseconds (default: 100)
    pub initial_delay_ms: u64,
    /// Maximum delay in milliseconds (default: 10000)
    pub max_delay_ms: u64,
    /// Exponential backoff multiplier (default: 2.0)
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 10000,
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Calculate delay for a given attempt
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let delay_ms = self.initial_delay_ms as f64
            * self.backoff_multiplier.powi(attempt as i32);
        let delay_ms = delay_ms.min(self.max_delay_ms as f64) as u64;
        Duration::from_millis(delay_ms)
    }
}

/// Rate limit configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per minute (0 = no limit)
    pub max_requests_per_minute: u32,
    /// Maximum tokens per minute (0 = no limit)
    pub max_tokens_per_minute: u32,
}

/// Error types specific to LLM operations
#[derive(Debug, thiserror::Error)]
pub enum LLMError {
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Token limit exceeded: {0}")]
    TokenLimitExceeded(String),

    #[error("Invalid JSON response: {0}")]
    InvalidJson(String),

    #[error("Schema validation failed: {0}")]
    SchemaValidationFailed(String),

    #[error("API error with status {status}: {message}")]
    ApiError {
        status: u16,
        message: String,
    },

    #[error("Request timeout after {timeout_ms}ms")]
    Timeout {
        timeout_ms: u64,
    },

    #[error("Circuit breaker open: {0}")]
    CircuitBreakerOpen(String),

    #[error("Retry exhausted after {attempts} attempts: {last_error}")]
    RetryExhausted {
        attempts: u32,
        last_error: String,
    },
}

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    HalfOpen,
    Open,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Failure threshold to open circuit (default: 5)
    pub failure_threshold: u32,
    /// Success threshold to close circuit (default: 3)
    pub success_threshold: u32,
    /// Timeout before attempting half-open (default: 60 seconds)
    pub timeout_secs: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout_secs: 60,
        }
    }
}

/// Circuit breaker for resilience
#[derive(Clone)]
pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<std::time::Instant>,
    config: CircuitBreakerConfig,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
            config,
        }
    }

    pub fn can_execute(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last_failure) = self.last_failure_time {
                    if last_failure.elapsed().as_secs() >= self.config.timeout_secs {
                        self.state = CircuitState::HalfOpen;
                        debug!("Circuit breaker moved to Half-Open state");
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    pub fn record_success(&mut self) {
        self.success_count += 1;
        self.failure_count = 0;

        if self.state == CircuitState::HalfOpen
            && self.success_count >= self.config.success_threshold
        {
            self.state = CircuitState::Closed;
            self.success_count = 0;
            info!("Circuit breaker closed after {} successful requests", self.success_count);
        }
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.success_count = 0;
        self.last_failure_time = Some(std::time::Instant::now());

        if self.state == CircuitState::HalfOpen
            || self.failure_count >= self.config.failure_threshold
        {
            self.state = CircuitState::Open;
            warn!(
                "Circuit breaker opened after {} failed requests",
                self.failure_count
            );
        }
    }

    pub fn state(&self) -> CircuitState {
        self.state
    }
}

/// JSON Schema validator for structured outputs
///
/// Uses the `jsonschema` crate for full JSON Schema Draft 7 validation.
pub struct SchemaValidator;

impl SchemaValidator {
    /// Validate that a JSON string matches the expected schema
    pub fn validate(json_str: &str, schema: &serde_json::Value) -> Result<(), LLMError> {
        let value: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| LLMError::InvalidJson(e.to_string()))?;

        // Compile schema and validate
        let compiled_schema = jsonschema::JSONSchema::compile(schema)
            .map_err(|e| LLMError::SchemaValidationFailed(format!("Invalid schema: {}", e)))?;

        compiled_schema
            .validate(&value)
            .map_err(|errors| {
                let error_messages: Vec<String> = errors.map(|e| e.to_string()).collect();
                LLMError::SchemaValidationFailed(format!("Validation errors: {}", error_messages.join("; ")))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_delay() {
        let config = RetryConfig::default();

        assert_eq!(config.delay_for_attempt(0).as_millis(), 100);
        assert_eq!(config.delay_for_attempt(1).as_millis(), 200);
        assert_eq!(config.delay_for_attempt(2).as_millis(), 400);
        assert_eq!(config.delay_for_attempt(5).as_millis(), 3200);
        assert_eq!(config.delay_for_attempt(10).as_millis(), 10000); // capped
    }

    #[test]
    fn test_circuit_breaker_state_transitions() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout_secs: 1,
        };

        let mut cb = CircuitBreaker::new(config);

        // Initially closed
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.can_execute());

        // Record failures to open circuit
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.can_execute());

        // Wait for timeout (simulated)
        cb.last_failure_time = Some(std::time::Instant::now() - Duration::from_secs(2));
        assert!(cb.can_execute());
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // Record successes to close circuit
        cb.record_success();
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_schema_validation_object() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "integer", "minimum": 0, "maximum": 150 }
            },
            "required": ["name"]
        });

        // Valid object
        let valid = serde_json::json!({
            "name": "Alice",
            "age": 30
        });
        assert!(SchemaValidator::validate(&valid.to_string(), &schema).is_ok());

        // Missing required field
        let invalid = serde_json::json!({
            "age": 30
        });
        assert!(SchemaValidator::validate(&invalid.to_string(), &schema).is_err());

        // Wrong type
        let invalid_type = serde_json::json!({
            "name": 123
        });
        assert!(SchemaValidator::validate(&invalid_type.to_string(), &schema).is_err());

        // Out of range
        let out_of_range = serde_json::json!({
            "name": "Bob",
            "age": 200
        });
        assert!(SchemaValidator::validate(&out_of_range.to_string(), &schema).is_err());
    }

    #[test]
    fn test_schema_validation_array() {
        let schema = serde_json::json!({
            "type": "array",
            "items": { "type": "string" }
        });

        let valid = serde_json::json!(["a", "b", "c"]);
        assert!(SchemaValidator::validate(&valid.to_string(), &schema).is_ok());

        let invalid = serde_json::json!([1, 2, 3]);
        assert!(SchemaValidator::validate(&invalid.to_string(), &schema).is_err());
    }

    #[test]
    fn test_client_stats() {
        let stats = ClientStats {
            total_requests: 10,
            successful_requests: 8,
            failed_requests: 2,
            total_latency_ms: 1000,
            ..Default::default()
        };

        assert!((stats.avg_latency_ms() - 100.0).abs() < 0.001);
        assert!((stats.success_rate() - 0.8).abs() < 0.001);
        assert!((stats.failure_rate() - 0.2).abs() < 0.001);
    }
}
