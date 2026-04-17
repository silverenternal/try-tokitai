//! Model Command Handler
//!
//! Handles /model commands for switching models, listing available models, and running benchmarks.

use crate::config::Config;
use crate::llm::performance_tracker::ModelProfile;
use crate::llm::router::{RouterConfig, RoutingStrategy};
use crate::llm::{LLMManager, ModelRouter, ProviderType};
use anyhow::Result;
use std::sync::Arc;

/// Model command handler
pub struct ModelCommandHandler {
    llm_manager: Arc<LLMManager>,
    config: Config,
    router: Option<ModelRouter>,
}

impl ModelCommandHandler {
    /// Create a new model command handler
    pub fn new(llm_manager: Arc<LLMManager>, config: Config) -> Self {
        Self {
            llm_manager,
            config,
            router: None,
        }
    }

    /// Initialize the model router
    pub fn with_router(mut self) -> Result<Self> {
        let router_config = RouterConfig {
            strategy: RoutingStrategy::Balanced,
            ..Default::default()
        };

        let mut router = ModelRouter::new(router_config);

        // Register models from config
        for (provider_name, provider_config) in &self.config.ai.providers {
            let provider_type = ProviderType::from_str(provider_name);
            let model_profile = ModelProfile::new(
                provider_config.model.clone(),
                provider_type,
                provider_config.cost_per_1k_tokens,
                1000.0, // Default latency, will be updated
                provider_config.quality_score,
                provider_config.context_window,
                vec!["chat".to_string()],
            );
            router.register_model(model_profile);
        }

        self.router = Some(router);
        Ok(self)
    }

    /// Execute a model command
    pub fn execute(&self, args: &str) -> String {
        let args = args.trim();

        match args {
            "list" | "ls" => self.list_models(),
            "benchmark" | "bench" => self.run_benchmark(),
            "stats" | "statistics" => self.show_stats(),
            s if s.starts_with("switch ") => {
                self.switch_model(s.trim_start_matches("switch ").trim())
            }
            s if s.starts_with("set ") => self.set_model(s.trim_start_matches("set ").trim()),
            _ => self.show_help(),
        }
    }

    /// List all available models
    fn list_models(&self) -> String {
        let mut output = String::from("📋 可用的 AI 模型：\n\n");

        let providers = self.llm_manager.list_providers();
        if providers.is_empty() {
            return "⚠️  未配置任何 AI 提供商".to_string();
        }

        let current = self.llm_manager.current_provider_type();

        for provider_type in &providers {
            let provider_name = provider_type.as_str();
            let is_current = current == Some(*provider_type);
            let marker = if is_current { "👉" } else { "  " };

            // Get provider info from config if available
            let model = self
                .config
                .ai
                .providers
                .get(provider_name)
                .map(|p| p.model.clone())
                .unwrap_or_else(|| "unknown".to_string());

            let quality = self
                .config
                .ai
                .providers
                .get(provider_name)
                .map(|p| p.quality_score)
                .unwrap_or(5.0);

            let cost = self
                .config
                .ai
                .providers
                .get(provider_name)
                .map(|p| p.cost_per_1k_tokens)
                .unwrap_or(0.0);

            output.push_str(&format!(
                "{} {} - {} (质量：{:.1}/10, 成本：${:.4}/1K tokens)\n",
                marker, provider_name, model, quality, cost
            ));
        }

        output.push_str(&format!(
            "\n当前：{} ({}/{})\n",
            current.map(|p| p.as_str()).unwrap_or("unknown"),
            providers
                .iter()
                .position(|p| current == Some(*p))
                .map(|i| i + 1)
                .unwrap_or(0),
            providers.len()
        ));

        output.push_str("\n💡 使用 /model switch <provider> 切换模型\n");
        output.push_str("   使用 /model benchmark 运行基准测试\n");

        output
    }

    /// Switch to a different model
    fn switch_model(&self, model_name: &str) -> String {
        let provider_type = ProviderType::from_str(model_name);

        // Check if provider exists in manager
        if !self.llm_manager.has_provider(&provider_type) {
            return format!(
                "❌ 未找到提供商：{}\n\n可用的提供商：{}",
                model_name,
                self.llm_manager
                    .list_providers()
                    .iter()
                    .map(|p| p.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        // Try to set as current
        // Note: We need a mutable reference to LLMManager to switch
        // For now, we'll just show a message about how to switch via environment
        format!(
            "✅ 准备切换到：{}\n\n⚠️  注意：运行时切换需要重新初始化 LLMManager\n\
             请设置环境变量并重启：\n\
             export AI_API_URL=<url>\n\
             export AI_API_KEY=<key>\n\
             export AI_MODEL=<model>",
            model_name
        )
    }

    /// Set current model (alias for switch)
    fn set_model(&self, model_name: &str) -> String {
        self.switch_model(model_name)
    }

    /// Run benchmark on all models
    fn run_benchmark(&self) -> String {
        "⏳ 正在运行基准测试...\n\n⚠️  基准测试功能开发中，敬请期待...".to_string()
    }

    /// Show model usage statistics
    fn show_stats(&self) -> String {
        let mut output = String::from("📊 模型使用统计：\n\n");

        // TODO: Implement actual statistics tracking
        output.push_str("⚠️  统计功能开发中，敬请期待...\n\n");
        output.push_str("计划功能：\n");
        output.push_str("  • Token 使用量统计\n");
        output.push_str("  • 成本估算\n");
        output.push_str("  • 响应延迟统计\n");
        output.push_str("  • 成功率统计\n");

        output
    }

    /// Show help message
    fn show_help(&self) -> String {
        let mut output = String::from("📖 /model 命令帮助：\n\n");
        output.push_str("可用命令：\n");
        output.push_str("  /model list          - 列出所有可用的模型\n");
        output.push_str("  /model ls            - 同上（简写）\n");
        output.push_str("  /model switch <name> - 切换到指定模型\n");
        output.push_str("  /model set <name>    - 同上（别名）\n");
        output.push_str("  /model benchmark     - 运行基准测试\n");
        output.push_str("  /model stats         - 显示使用统计\n");
        output.push_str("  /model help          - 显示此帮助信息\n");
        output.push_str("\n示例：\n");
        output.push_str("  /model list\n");
        output.push_str("  /model switch openai\n");
        output.push_str("  /model benchmark\n");

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::providers::OpenAIProvider;

    #[test]
    fn test_model_command_handler_creation() {
        let config = Config::default();
        let mut manager = LLMManager::new();

        // Add a test provider
        let provider = Arc::new(OpenAIProvider::new(
            "test_key".to_string(),
            Some("gpt-4o".to_string()),
        ));
        manager.register_provider(provider);

        let handler = ModelCommandHandler::new(Arc::new(manager), config);

        let help = handler.execute("help");
        assert!(help.contains("/model"));
    }

    #[test]
    fn test_list_models_command() {
        let config = Config::default();
        let mut manager = LLMManager::new();

        let provider = Arc::new(OpenAIProvider::new(
            "test_key".to_string(),
            Some("gpt-4o".to_string()),
        ));
        manager.register_provider(provider);

        let handler = ModelCommandHandler::new(Arc::new(manager), config);

        let list = handler.execute("list");
        assert!(list.contains("可用的 AI 模型"));
        assert!(list.contains("openai"));
    }
}
