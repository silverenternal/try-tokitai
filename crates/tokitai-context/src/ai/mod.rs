//! AI integration modules (optional feature)
//!
//! AI-powered features for conflict resolution, purpose inference,
//! smart merge recommendations, and branch summarization.
//!
//! Requires the `ai` feature to be enabled.
//!
//! # Unified LLM Client
//!
//! This module provides a single, unified `LLMClient` trait that all AI
//! components use. Built-in implementations for popular providers:
//!
//! ```rust,no_run
//! use tokitai_context::ai::client::LLMClient;
//! use tokitai_context::ai::clients::{OpenAIClient, AnthropicClient, OllamaClient, AliyunCodingPlanClient};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // OpenAI
//!     let openai: Arc<dyn LLMClient> = Arc::new(OpenAIClient::from_env());
//!
//!     // Anthropic
//!     let anthropic: Arc<dyn LLMClient> = Arc::new(AnthropicClient::from_env());
//!
//!     // Ollama (self-hosted)
//!     let ollama: Arc<dyn LLMClient> = Arc::new(OllamaClient::new("http://localhost:11434", "llama3.1"));
//!
//!     // Alibaba Cloud Coding Plan (通义千问)
//!     let aliyun: Arc<dyn LLMClient> = Arc::new(AliyunCodingPlanClient::from_env());
//!
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod clients;
pub mod enhanced_manager;
pub mod purpose;
pub mod resolver;
pub mod smart_merge;
pub mod summarizer;

// Re-export unified client trait
pub use client::{LLMClient, ClientStats, RetryConfig, CircuitBreaker, SchemaValidator, LLMError};

#[cfg(feature = "ai")]
pub use clients::{OpenAIClient, AnthropicClient, OllamaClient, AliyunCodingPlanClient, AliyunCodingPlanConfig};

#[cfg(feature = "ai")]
pub use enhanced_manager::{AIEnhancedContextManager, AIStats};

#[cfg(feature = "ai")]
pub use resolver::{
    AIConflictResolver, ConflictResolutionRequest, ConflictResolutionResponse,
    ConflictAnalysisReport, ResolverStats,
};

#[cfg(feature = "ai")]
pub use purpose::{
    AIPurposeInference, PurposeInferenceRequest, PurposeInferenceResult,
    BranchType, InferenceStats,
};

#[cfg(feature = "ai")]
pub use smart_merge::{
    AISmartMergeRecommender, MergeRecommendationRequest, MergeRecommendation,
    TimingRecommendation, RiskAssessment, ChecklistItem, ChecklistStatus,
    QuickAssessment, RecommenderStats,
};

#[cfg(feature = "ai")]
pub use summarizer::{
    AIBranchSummarizer, SummaryGenerationRequest, SummaryGenerationResult,
    TimelineEvent, StatusAssessment, MergeReadiness, QuickSummary, SummarizerStats,
};

// Semantic index is available without AI feature for basic search
pub use crate::semantic_index::{
    SemanticIndex, SemanticIndexConfig, SemanticIndexManager,
    FingerprintIndexEntry as SearchIndexEntry, IndexStats, SearchResult,
};
