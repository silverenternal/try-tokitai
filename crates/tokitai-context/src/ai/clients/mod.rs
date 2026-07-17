//! Built-in LLM client implementations
//!
//! This module provides ready-to-use LLM clients for popular providers:
//! - [`OpenAIClient`]: OpenAI API (GPT-4, GPT-3.5-turbo)
//! - [`AnthropicClient`]: Anthropic API (Claude)
//! - [`OllamaClient`]: Local Ollama API (self-hosted models)
//! - [`AliyunCodingPlanClient`]: Alibaba Cloud Model Studio Coding Plan (通义千问)

pub mod openai;
pub mod anthropic;
pub mod ollama;
pub mod aliyun_coding_plan;

pub use openai::{OpenAIClient, OpenAIConfig};
pub use anthropic::{AnthropicClient, AnthropicConfig};
pub use ollama::{OllamaClient, OllamaConfig};
pub use aliyun_coding_plan::{AliyunCodingPlanClient, AliyunCodingPlanConfig};
