//! Model configuration with actual API limits per provider/model
//!
//! Each entry stores the actual API limits from the provider's documentation.

/// Pre-defined model configurations with actual API limits
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub provider: &'static str,
    pub model: &'static str,
    pub display_name: &'static str,
    pub context_window: usize,    // total context (input + output)
    pub max_output_tokens: usize, // max output tokens
    pub supports_streaming: bool,
    pub description: &'static str,
}

/// Registry of known models with verified API limits
pub struct ModelRegistry;

impl ModelRegistry {
    /// All supported models with actual API limits
    pub fn all_models() -> Vec<ModelInfo> {
        vec![
            // === DeepSeek ===
            ModelInfo {
                provider: "deepseek",
                model: "deepseek-chat",
                display_name: "DeepSeek V3 (Chat)",
                context_window: 128_000,
                max_output_tokens: 8_192,
                supports_streaming: true,
                description: "DeepSeek V3, 128K context, 8K max output",
            },
            ModelInfo {
                provider: "deepseek",
                model: "deepseek-reasoner",
                display_name: "DeepSeek R1 (Reasoner)",
                context_window: 128_000,
                max_output_tokens: 8_192,
                supports_streaming: true,
                description: "DeepSeek R1 reasoning model, 128K context",
            },
            // === OpenAI ===
            ModelInfo {
                provider: "openai",
                model: "gpt-4o",
                display_name: "GPT-4o",
                context_window: 128_000,
                max_output_tokens: 16_384,
                supports_streaming: true,
                description: "GPT-4o multimodal, 128K context, 16K output",
            },
            ModelInfo {
                provider: "openai",
                model: "gpt-4o-mini",
                display_name: "GPT-4o Mini",
                context_window: 128_000,
                max_output_tokens: 16_384,
                supports_streaming: true,
                description: "Fast & affordable, 128K context",
            },
            ModelInfo {
                provider: "openai",
                model: "gpt-4-turbo",
                display_name: "GPT-4 Turbo",
                context_window: 128_000,
                max_output_tokens: 4_096,
                supports_streaming: true,
                description: "GPT-4 Turbo, 128K context",
            },
            // === Kimi (Moonshot) ===
            ModelInfo {
                provider: "moonshot",
                model: "moonshot-v1-8k",
                display_name: "Kimi (Moonshot v1 8K)",
                context_window: 8_192,
                max_output_tokens: 8_192,
                supports_streaming: false,
                description: "Kimi Moonshot v1, 8K context",
            },
            ModelInfo {
                provider: "moonshot",
                model: "moonshot-v1-32k",
                display_name: "Kimi (Moonshot v1 32K)",
                context_window: 32_768,
                max_output_tokens: 32_768,
                supports_streaming: false,
                description: "Kimi Moonshot v1, 32K context",
            },
            ModelInfo {
                provider: "moonshot",
                model: "moonshot-v1-128k",
                display_name: "Kimi (Moonshot v1 128K)",
                context_window: 128_000,
                max_output_tokens: 4_096,
                supports_streaming: false,
                description: "Kimi Moonshot v1, 128K context, 4K output",
            },
            // === Qwen (通义千问) ===
            ModelInfo {
                provider: "qwen",
                model: "qwen-turbo",
                display_name: "Qwen Turbo",
                context_window: 131_072,
                max_output_tokens: 8_192,
                supports_streaming: true,
                description: "Qwen Turbo, fast, 128K context",
            },
            ModelInfo {
                provider: "qwen",
                model: "qwen-plus",
                display_name: "Qwen Plus",
                context_window: 131_072,
                max_output_tokens: 8_192,
                supports_streaming: true,
                description: "Qwen Plus, balanced, 128K context",
            },
            ModelInfo {
                provider: "qwen",
                model: "qwen-max",
                display_name: "Qwen Max",
                context_window: 32_768,
                max_output_tokens: 8_192,
                supports_streaming: true,
                description: "Qwen Max, most capable, 32K context",
            },
            // === Zhipu (智谱) ===
            ModelInfo {
                provider: "zhipu",
                model: "glm-4",
                display_name: "GLM-4",
                context_window: 128_000,
                max_output_tokens: 4_096,
                supports_streaming: false,
                description: "Zhipu GLM-4, 128K context",
            },
            ModelInfo {
                provider: "zhipu",
                model: "glm-4-flash",
                display_name: "GLM-4 Flash",
                context_window: 128_000,
                max_output_tokens: 4_096,
                supports_streaming: false,
                description: "Zhipu GLM-4 Flash, fast variant",
            },
            // === Anthropic ===
            ModelInfo {
                provider: "anthropic",
                model: "claude-3-5-sonnet-20241022",
                display_name: "Claude 3.5 Sonnet",
                context_window: 200_000,
                max_output_tokens: 8_192,
                supports_streaming: false,
                description: "Claude 3.5 Sonnet, 200K context",
            },
            // === Ollama (local) ===
            ModelInfo {
                provider: "ollama",
                model: "qwen3.5:397b",
                display_name: "Ollama - Qwen 3.5",
                context_window: 32_768,
                max_output_tokens: 8_192,
                supports_streaming: true,
                description: "Local Qwen via Ollama, 32K context",
            },
        ]
    }

    /// Find a model by its identifier
    pub fn find(model: &str) -> Option<ModelInfo> {
        Self::all_models().into_iter().find(|m| m.model == model)
    }

    /// Get max_output_tokens for a given model, capped at the model's limit
    pub fn get_max_tokens(model_name: &str, requested: usize) -> usize {
        if let Some(info) = Self::find(model_name) {
            requested.min(info.max_output_tokens)
        } else {
            requested.min(8192) // conservative default
        }
    }
}
