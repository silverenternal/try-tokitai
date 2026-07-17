//! AI integration tests
//!
//! These tests verify AI client implementations and AIContext wrapper.
//! Run with: cargo test --features ai --test ai_integration_tests

#![cfg(feature = "ai")]

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use tokitai_context::facade::{Context, Layer, AIContext};
    use tokitai_context::ai::client::LLMClient;
    use tokitai_context::ai::clients::{OpenAIClient, AnthropicClient, OllamaClient, OllamaConfig, AliyunCodingPlanClient, AliyunCodingPlanConfig};
    use tokitai_context::parallel::{ParallelContextManager, ParallelContextManagerConfig};
    use tokitai_context::parallel::branch::MergeStrategy;
    use tempfile::TempDir;

    // ========================================================================
    // Client Creation Tests
    // ========================================================================

    #[test]
    fn test_openai_client_creation() {
        let client = OpenAIClient::new("test-key");
        assert_eq!(client.config().api_key, "test-key");
        assert_eq!(client.config().model, "gpt-4o-mini");
    }

    #[test]
    fn test_anthropic_client_creation() {
        let client = AnthropicClient::new("test-key");
        assert_eq!(client.config().api_key, "test-key");
        assert_eq!(client.config().model, "claude-3-haiku-20240307");
    }

    #[test]
    fn test_ollama_client_creation() {
        let client = OllamaClient::new("http://localhost:11434", "llama3.1");
        assert_eq!(client.config().model, "llama3.1");
        assert_eq!(client.config().base_url, "http://localhost:11434");
    }

    // ========================================================================
    // Aliyun Coding Plan Client Tests
    // ========================================================================

    #[test]
    fn test_aliyun_coding_plan_client_creation() {
        let client = AliyunCodingPlanClient::new("test-key");
        assert_eq!(client.config().api_key, "test-key");
        assert_eq!(client.config().model, "qwen-plus");
        assert_eq!(client.config().base_url, "https://dashscope.aliyuncs.com/compatible-mode/v1");
    }

    #[test]
    fn test_aliyun_coding_plan_config_builder() {
        let config = AliyunCodingPlanConfig::default()
            .with_model("qwen-max")
            .with_api_key("test-key")
            .with_base_url("https://custom-endpoint.com/v1");

        assert_eq!(config.model, "qwen-max");
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.base_url, "https://custom-endpoint.com/v1");
    }

    #[test]
    fn test_aliyun_coding_plan_from_env() {
        std::env::set_var("ALIYUN_CODING_PLAN_API_KEY", "test-env-key");
        let client = AliyunCodingPlanClient::from_env();
        assert_eq!(client.config().api_key, "test-env-key");
        std::env::remove_var("ALIYUN_CODING_PLAN_API_KEY");
    }

    #[test]
    fn test_aliyun_coding_plan_model_variants() {
        let models = vec!["qwen-plus", "qwen-max", "qwen-turbo", "qwen-long"];
        
        for model in models {
            let config = AliyunCodingPlanConfig::default().with_model(model);
            let client = AliyunCodingPlanClient::with_config(config);
            assert_eq!(client.model_name(), model);
        }
    }

    #[test]
    fn test_openai_config_builder() {
        let config = OpenAIClient::new("test-key")
            .config()
            .clone()
            .with_model("gpt-4-turbo")
            .with_api_key("new-key");

        assert_eq!(config.model, "gpt-4-turbo");
        assert_eq!(config.api_key, "new-key");
    }

    #[test]
    fn test_anthropic_config_builder() {
        let config = AnthropicClient::new("test-key")
            .config()
            .clone()
            .with_model("claude-3-opus-20240229")
            .with_api_key("new-key");

        assert_eq!(config.model, "claude-3-opus-20240229");
        assert_eq!(config.api_key, "new-key");
    }

    // ========================================================================
    // AIContext Wrapper Tests
    // ========================================================================

    #[test]
    fn test_ai_context_creation() {
        let temp_dir = TempDir::new().unwrap();
        let mut ctx = Context::open(temp_dir.path()).unwrap();
        let llm = Arc::new(OpenAIClient::new("test-key"));
        
        let ai_ctx = AIContext::new(&mut ctx, llm);
        
        // Verify we can access inner context
        assert!(ai_ctx.inner().root().exists());
    }

    #[test]
    fn test_ai_context_inner_mut() {
        let temp_dir = TempDir::new().unwrap();
        let mut ctx = Context::open(temp_dir.path()).unwrap();
        let llm = Arc::new(OpenAIClient::new("test-key"));
        
        let mut ai_ctx = AIContext::new(&mut ctx, llm);
        
        // Verify we can modify inner context
        let _hash = ai_ctx.inner_mut()
            .store("test-session", b"Test content", Layer::ShortTerm)
            .unwrap();
    }

    // ========================================================================
    // Integration Tests (Mock)
    // ========================================================================

    #[tokio::test]
    async fn test_ai_context_store_and_retrieve() {
        let temp_dir = TempDir::new().unwrap();
        let mut ctx = Context::open(temp_dir.path()).unwrap();
        let llm = Arc::new(OpenAIClient::new("test-key"));
        let mut ai_ctx = AIContext::new(&mut ctx, llm);
        
        // Store content
        let hash = ai_ctx.inner_mut()
            .store("session-1", b"Hello, AI!", Layer::ShortTerm)
            .unwrap();
        
        // Retrieve content
        let item = ai_ctx.inner()
            .retrieve("session-1", &hash)
            .unwrap();
        
        assert_eq!(item.content, b"Hello, AI!");
    }

    #[tokio::test]
    async fn test_parallel_manager_with_ai_context() {
        let temp_dir = TempDir::new().unwrap();
        let mut ctx = Context::open(temp_dir.path()).unwrap();
        let llm = Arc::new(OpenAIClient::new("test-key"));
        let mut ai_ctx = AIContext::new(&mut ctx, llm);
        
        // Create parallel manager
        let config = ParallelContextManagerConfig {
            context_root: temp_dir.path().to_path_buf(),
            default_merge_strategy: MergeStrategy::SelectiveMerge,
            ..Default::default()
        };
        let mut manager = ParallelContextManager::new(config).unwrap();
        
        // Create branch
        let branch = manager.create_branch("test-branch", "main").unwrap();
        let branch_id = branch.branch_id.clone();
        
        // Verify branch exists
        let branches = manager.list_branches();
        assert_eq!(branches.len(), 2); // main + test-branch
        
        // Use AI context to store in branch
        manager.checkout(&branch_id).unwrap();
        let _hash = ai_ctx.inner_mut()
            .store("session-1", b"Branch content", Layer::ShortTerm)
            .unwrap();
    }

    // ========================================================================
    // Client Configuration Tests
    // ========================================================================

    #[test]
    fn test_openai_timeout_config() {
        let config = OpenAIClient::new("test-key")
            .config()
            .clone()
            .with_model("gpt-4o");

        assert_eq!(config.timeout_ms, 30000);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_ollama_custom_config() {
        let config = OllamaConfig::default()
            .with_base_url("http://192.168.1.100:11434")
            .with_model("mistral:7b");

        assert_eq!(config.base_url, "http://192.168.1.100:11434");
        assert_eq!(config.model, "mistral:7b");
    }

    // ========================================================================
    // Error Handling Tests
    // ========================================================================

    #[tokio::test]
    async fn test_ai_context_with_invalid_api_key() {
        let temp_dir = TempDir::new().unwrap();
        let mut ctx = Context::open(temp_dir.path()).unwrap();
        let llm = Arc::new(OpenAIClient::new("invalid-key"));
        let mut ai_ctx = AIContext::new(&mut ctx, llm);

        // This should fail gracefully when calling API
        // Note: We don't assert failure since network might be unavailable
        let result = ai_ctx.infer_branch_purpose("nonexistent").await;

        // Either error (expected) or Ok (if mock) is acceptable
        match result {
            Ok(_) => println!("Unexpected success (mock?)"),
            Err(_) => println!("Expected error (invalid API key)"),
        }
    }

    // ========================================================================
    // Circuit Breaker and Retry Tests
    // ========================================================================

    #[test]
    fn test_circuit_breaker_initial_state() {
        use tokitai_context::ai::client::{CircuitBreaker, CircuitBreakerConfig, CircuitState};

        let config = CircuitBreakerConfig::default();
        let mut cb = CircuitBreaker::new(config);

        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.can_execute());
    }

    #[test]
    fn test_circuit_breaker_opens_after_failures() {
        use tokitai_context::ai::client::{CircuitBreaker, CircuitBreakerConfig, CircuitState};

        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout_secs: 60,
        };
        let mut cb = CircuitBreaker::new(config);

        // Record failures
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.can_execute());
    }

    #[test]
    fn test_circuit_breaker_half_open_after_timeout() {
        use tokitai_context::ai::client::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
        use std::time::Duration;

        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout_secs: 1,
        };
        let mut cb = CircuitBreaker::new(config);

        // Open the circuit
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Wait for timeout (real sleep for integration test)
        std::thread::sleep(Duration::from_secs(2));

        // Should move to half-open
        assert!(cb.can_execute());
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_circuit_breaker_closes_after_successes() {
        use tokitai_context::ai::client::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
        use std::time::Duration;

        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 3,
            timeout_secs: 1,
        };
        let mut cb = CircuitBreaker::new(config);

        // Open the circuit
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Wait for timeout
        std::thread::sleep(Duration::from_secs(2));

        // Move to half-open
        cb.can_execute();
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // Record successes to close
        cb.record_success();
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_clone() {
        use tokitai_context::ai::client::{CircuitBreaker, CircuitBreakerConfig};

        let config = CircuitBreakerConfig::default();
        let cb = CircuitBreaker::new(config);
        let cb_clone = cb.clone();

        assert_eq!(cb.state(), cb_clone.state());
    }

    #[test]
    fn test_retry_config_exponential_backoff() {
        use tokitai_context::ai::client::RetryConfig;
        use std::time::Duration;

        let config = RetryConfig {
            max_retries: 5,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
        };

        // Verify exponential backoff
        assert_eq!(config.delay_for_attempt(0), Duration::from_millis(100));
        assert_eq!(config.delay_for_attempt(1), Duration::from_millis(200));
        assert_eq!(config.delay_for_attempt(2), Duration::from_millis(400));
        assert_eq!(config.delay_for_attempt(3), Duration::from_millis(800));
        assert_eq!(config.delay_for_attempt(4), Duration::from_millis(1600));

        // Verify cap at max_delay
        assert_eq!(config.delay_for_attempt(10), Duration::from_millis(5000));
    }

    #[test]
    fn test_retry_config_default() {
        use tokitai_context::ai::client::RetryConfig;

        let config = RetryConfig::default();

        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay_ms, 100);
        assert_eq!(config.max_delay_ms, 10000);
        assert!((config.backoff_multiplier - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_rate_limit_config_default() {
        use tokitai_context::ai::client::RateLimitConfig;

        let config = RateLimitConfig::default();

        assert_eq!(config.max_requests_per_minute, 0);
        assert_eq!(config.max_tokens_per_minute, 0);
    }
}
