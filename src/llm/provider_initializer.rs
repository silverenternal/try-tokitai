//! Multi-Provider Initializer
//!
//! Loads and initializes LLM providers from config file and environment variables.

use super::{
    LLMManager, LLMProvider, ProviderType,
    providers::{
        OpenAIProvider, GeminiProvider, AnthropicProvider, 
        ZhipuProvider, MoonshotProvider,
    },
};
use crate::config::{Config, ProviderConfig as ConfigProviderConfig};
use std::sync::Arc;
use anyhow::Result;
use tracing::{info, warn};

/// Provider initializer - loads providers from multiple sources
pub struct ProviderInitializer {
    config: Config,
}

impl ProviderInitializer {
    /// Create a new provider initializer
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Initialize LLM manager with all configured providers
    pub fn initialize_llm_manager(&self) -> Result<LLMManager> {
        let mut manager = LLMManager::new();

        // Load providers from config file
        for (provider_name, provider_config) in &self.config.ai.providers {
            match self.create_provider(provider_name, provider_config) {
                Ok(provider) => {
                    info!("✅ 加载提供商：{} ({})", provider_name, provider_config.api_url);
                    manager.register_provider(provider);
                }
                Err(e) => {
                    warn!("⚠️  加载提供商失败 {}: {}", provider_name, e);
                }
            }
        }

        // If no providers from config, try environment variables
        if manager.list_providers().is_empty() {
            info!("配置文件中未找到提供商配置，尝试从环境变量加载...");
            if let Some(provider) = self.create_provider_from_env() {
                let provider_type = provider.provider_type().clone();
                info!("✅ 从环境变量加载提供商：{}", provider_type);
                manager.register_provider(provider);
            }
        }

        // Set default provider
        if let Some(default_name) = &self.config.ai.default_provider {
            let provider_type = ProviderType::from_str(default_name);
            if manager.has_provider(&provider_type) {
                manager.set_current(provider_type)?;
                info!("设置默认提供商：{}", default_name);
            }
        }

        Ok(manager)
    }

    /// Create a provider from config
    fn create_provider(
        &self,
        name: &str,
        config: &ConfigProviderConfig,
    ) -> Result<Arc<dyn LLMProvider>> {
        // Get API key from config or environment
        let api_key = config.api_key.clone()
            .or_else(|| self.get_api_key_from_env(name));

        let api_key = api_key.ok_or_else(|| {
            anyhow::anyhow!("API Key not configured for provider {}", name)
        })?;

        let provider: Arc<dyn LLMProvider> = match name.to_lowercase().as_str() {
            "openai" => Arc::new(OpenAIProvider::with_base_url(
                api_key.to_string(),
                config.api_url.clone(),
                Some(config.model.clone()),
            )),
            "gemini" => Arc::new(GeminiProvider::new(
                api_key.to_string(),
                Some(config.model.clone()),
            )),
            "anthropic" => Arc::new(AnthropicProvider::new(
                api_key.to_string(),
                Some(config.model.clone()),
            )),
            "zhipu" => Arc::new(ZhipuProvider::new(
                api_key.to_string(),
                Some(config.model.clone()),
            )),
            "moonshot" => Arc::new(MoonshotProvider::new(
                api_key.to_string(),
                Some(config.model.clone()),
            )),
            "ollama" => Arc::new(OpenAIProvider::with_base_url(
                api_key.to_string(),
                config.api_url.clone(),
                Some(config.model.clone()),
            )),
            other => {
                // Try to create as OpenAI-compatible provider
                info!("创建自定义提供商：{} (OpenAI 兼容)", other);
                Arc::new(OpenAIProvider::with_base_url(
                    api_key.to_string(),
                    config.api_url.clone(),
                    Some(config.model.clone()),
                ))
            }
        };

        Ok(provider)
    }

    /// Create provider from environment variables (legacy single-provider mode)
    fn create_provider_from_env(&self) -> Option<Arc<dyn LLMProvider>> {
        let api_url = std::env::var("AI_API_URL").ok()?;
        let api_key = std::env::var("AI_API_KEY").ok();
        let model = std::env::var("AI_MODEL")
            .unwrap_or_else(|_| "gpt-3.5-turbo".to_string());

        // Detect provider type from URL
        let provider: Arc<dyn LLMProvider> = if api_url.contains("openai.com") {
            Arc::new(OpenAIProvider::new(
                api_key?,
                Some(model),
            ))
        } else if api_url.contains("googleapis.com") || api_url.contains("generativelanguage") {
            Arc::new(GeminiProvider::new(
                api_key?,
                Some(model),
            ))
        } else if api_url.contains("anthropic.com") {
            Arc::new(AnthropicProvider::new(
                api_key?,
                Some(model),
            ))
        } else if api_url.contains("bigmodel.cn") {
            Arc::new(ZhipuProvider::new(
                api_key?,
                Some(model),
            ))
        } else if api_url.contains("moonshot.cn") {
            Arc::new(MoonshotProvider::new(
                api_key?,
                Some(model),
            ))
        } else if api_url.contains("ollama") {
            Arc::new(OpenAIProvider::with_base_url(
                api_key.unwrap_or_default(),
                api_url,
                Some(model),
            ))
        } else {
            // Default to OpenAI-compatible
            Arc::new(OpenAIProvider::with_base_url(
                api_key.unwrap_or_default(),
                api_url,
                Some(model),
            ))
        };

        Some(provider)
    }

    /// Get API key from environment variable
    fn get_api_key_from_env(&self, provider_name: &str) -> Option<String> {
        let env_var_name = format!("{}_API_KEY", provider_name.to_uppercase());
        std::env::var(env_var_name).ok()
    }

    /// Get all configured provider names
    pub fn get_configured_providers(&self) -> Vec<String> {
        self.config.ai.providers.keys().cloned().collect()
    }

    /// Check if a provider is configured
    pub fn is_provider_configured(&self, provider_name: &str) -> bool {
        self.config.ai.providers.contains_key(provider_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AiConfig, ProviderConfig as ConfigProviderConfig};
    use std::collections::HashMap;

    #[test]
    fn test_provider_initializer_creation() {
        let config = Config::default();
        let initializer = ProviderInitializer::new(config);

        assert!(initializer.get_configured_providers().is_empty());
    }

    #[test]
    fn test_provider_config_check() {
        let mut providers = HashMap::new();
        providers.insert(
            "openai".to_string(),
            ConfigProviderConfig {
                api_url: "https://api.openai.com/v1/chat/completions".to_string(),
                api_key: Some("test_key".to_string()),
                model: "gpt-4o".to_string(),
                cost_per_1k_tokens: 0.03,
                quality_score: 9.0,
                context_window: 128000,
            },
        );

        let config = Config {
            ai: AiConfig {
                providers,
                ..Default::default()
            },
            ..Default::default()
        };

        let initializer = ProviderInitializer::new(config);

        assert!(initializer.is_provider_configured("openai"));
        assert!(!initializer.is_provider_configured("gemini"));
        assert_eq!(initializer.get_configured_providers(), vec!["openai"]);
    }
}
