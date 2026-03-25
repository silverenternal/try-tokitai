//! Unified LLM Provider Abstraction Layer
//!
//! This module provides a unified interface for multiple LLM providers,
//! leveraging tokitai's LLMClient trait for vendor-neutral support.
//!
//! ## Supported Providers
//! - OpenAI
//! - Gemini (Google)
//! - Anthropic (Claude)
//! - Zhipu (智谱 AI)
//! - Moonshot (月之暗面)
//! - Ollama (Local)
//!
//! ## Example
//! ```rust
//! use crate::llm::{LLMProvider, ProviderType, LLMManager, ProviderInitializer};
//! use crate::config::Config;
//!
//! let config = Config::load(None)?;
//! let initializer = ProviderInitializer::new(config);
//! let manager = initializer.initialize_llm_manager()?;
//!
//! let response = manager.current_provider()
//!     .chat(messages)
//!     .await?;
//! ```

pub mod providers;
pub mod router;
pub mod performance_tracker;
pub mod provider_initializer;
pub mod model_command;

pub use providers::*;
pub use router::ModelRouter;
pub use performance_tracker::PerformanceTracker;
pub use provider_initializer::ProviderInitializer;
pub use model_command::ModelCommandHandler;

use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    OpenAI,
    Gemini,
    Anthropic,
    Zhipu,
    Moonshot,
    Ollama,
    Custom(String),
}

impl ProviderType {
    pub fn as_str(&self) -> &str {
        match self {
            ProviderType::OpenAI => "openai",
            ProviderType::Gemini => "gemini",
            ProviderType::Anthropic => "anthropic",
            ProviderType::Zhipu => "zhipu",
            ProviderType::Moonshot => "moonshot",
            ProviderType::Ollama => "ollama",
            ProviderType::Custom(name) => name,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "openai" => ProviderType::OpenAI,
            "gemini" => ProviderType::Gemini,
            "anthropic" => ProviderType::Anthropic,
            "zhipu" => ProviderType::Zhipu,
            "moonshot" => ProviderType::Moonshot,
            "ollama" => ProviderType::Ollama,
            other => ProviderType::Custom(other.to_string()),
        }
    }
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// LLM Provider trait - unified interface for all providers
#[async_trait::async_trait]
pub trait LLMProvider: Send + Sync {
    /// Get provider type
    fn provider_type(&self) -> &ProviderType;
    
    /// Get provider name
    fn name(&self) -> &str;
    
    /// Get default model
    fn default_model(&self) -> &str;
    
    /// Send a chat request
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse>;
    
    /// Send a streaming chat request
    async fn chat_stream(
        &self, 
        request: ChatRequest
    ) -> Result<Pin<Box<dyn futures::Stream<Item = Result<StreamChunk>> + Send>>>;
    
    /// Check if provider is available
    async fn health_check(&self) -> bool;
}

/// Chat request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// Model name to use
    pub model: String,
    /// Messages in conversation
    pub messages: Vec<Message>,
    /// Temperature (0.0-2.0)
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Maximum tokens to generate
    #[serde(default)]
    pub max_tokens: Option<usize>,
    /// Top-p sampling
    #[serde(default)]
    pub top_p: Option<f32>,
    /// Stop sequences
    #[serde(default)]
    pub stop: Option<Vec<String>>,
    /// Whether to stream response
    #[serde(default)]
    pub stream: bool,
}

fn default_temperature() -> f32 {
    0.7
}

/// Message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role: "system", "user", or "assistant"
    pub role: String,
    /// Message content
    pub content: String,
    /// Optional name for the participant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Message {
    pub fn system(content: &str) -> Self {
        Self {
            role: "system".to_string(),
            content: content.to_string(),
            name: None,
        }
    }

    pub fn user(content: &str) -> Self {
        Self {
            role: "user".to_string(),
            content: content.to_string(),
            name: None,
        }
    }

    pub fn assistant(content: &str) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.to_string(),
            name: None,
        }
    }
}

/// Chat response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// Generated content
    pub content: String,
    /// Model used
    pub model: String,
    /// Usage statistics
    pub usage: Option<Usage>,
    /// Finish reason
    pub finish_reason: Option<String>,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

/// Stream chunk for streaming responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub content: String,
    pub finish_reason: Option<String>,
}

/// LLM Manager - manages multiple providers
pub struct LLMManager {
    providers: HashMap<ProviderType, Arc<dyn LLMProvider>>,
    current: Option<ProviderType>,
}

impl LLMManager {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            current: None,
        }
    }

    /// Register a provider
    pub fn register_provider(&mut self, provider: Arc<dyn LLMProvider>) {
        let provider_type = provider.provider_type().clone();
        self.providers.insert(provider_type.clone(), provider);
        
        // Set as current if not set
        if self.current.is_none() {
            self.current = Some(provider_type);
        }
    }

    /// Set current provider
    pub fn set_current(&mut self, provider_type: ProviderType) -> Result<()> {
        if !self.providers.contains_key(&provider_type) {
            anyhow::bail!("Provider {} not registered", provider_type);
        }
        self.current = Some(provider_type);
        Ok(())
    }

    /// Get current provider
    pub fn current_provider(&self) -> Option<&Arc<dyn LLMProvider>> {
        self.current.as_ref().and_then(|t| self.providers.get(t))
    }

    /// Get provider by type
    pub fn get_provider(&self, provider_type: &ProviderType) -> Option<&Arc<dyn LLMProvider>> {
        self.providers.get(provider_type)
    }

    /// List all registered providers
    pub fn list_providers(&self) -> Vec<&ProviderType> {
        self.providers.keys().collect()
    }

    /// Check if a provider is registered
    pub fn has_provider(&self, provider_type: &ProviderType) -> bool {
        self.providers.contains_key(provider_type)
    }

    /// Get the current provider type
    pub fn current_provider_type(&self) -> Option<&ProviderType> {
        self.current.as_ref()
    }

    /// Get default provider (first registered or OpenAI)
    pub fn get_default_provider(&self) -> Option<&Arc<dyn LLMProvider>> {
        self.current.as_ref()
            .and_then(|t| self.providers.get(t))
            .or_else(|| self.providers.values().next())
    }
}

impl Default for LLMManager {
    fn default() -> Self {
        Self::new()
    }
}
