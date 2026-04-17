//! Model Command Handler
//!
//! Handles /model commands for listing, switching, and benchmarking LLM models.

use crate::llm::{LLMManager, ProviderType};
use parking_lot::Mutex;
use std::sync::Arc;

/// Model command result
pub enum ModelCommandResult {
    Success(String),
    Error(String),
}

/// Model command handler
pub struct ModelCommandHandler {
    llm_manager: Arc<Mutex<LLMManager>>,
}

impl ModelCommandHandler {
    /// Create a new model command handler
    pub fn new(llm_manager: Arc<Mutex<LLMManager>>) -> Self {
        Self { llm_manager }
    }

    /// Execute a model command
    pub fn execute(&self, args: &[&str]) -> ModelCommandResult {
        if args.is_empty() {
            return ModelCommandResult::Error(
                "Usage: /model <list|switch|benchmark|stats> [args]".to_string(),
            );
        }

        match args[0].to_lowercase().as_str() {
            "list" => self.handle_list(),
            "switch" => {
                if args.len() < 2 {
                    ModelCommandResult::Error("Usage: /model switch <provider>".to_string())
                } else {
                    self.handle_switch(args[1])
                }
            }
            "benchmark" => {
                ModelCommandResult::Success("Benchmark command - coming soon.\n".to_string())
            }
            "stats" => self.handle_stats(),
            _ => ModelCommandResult::Error(
                "Unknown command. Use: list, switch, benchmark, stats".to_string(),
            ),
        }
    }

    /// Handle /model list
    fn handle_list(&self) -> ModelCommandResult {
        let manager = self.llm_manager.lock();
        let mut output = String::from("📋 Available Models:\n\n");

        let providers = manager.list_providers();

        for provider_type in providers {
            output.push_str(&format!("  - {}\n", provider_type));
        }

        if let Some(current) = manager.current_provider() {
            output.push_str(&format!("\n👉 Current: {}\n", current.provider_type()));
        }

        ModelCommandResult::Success(output)
    }

    /// Handle /model switch
    fn handle_switch(&self, target: &str) -> ModelCommandResult {
        let mut manager = self.llm_manager.lock();

        // Try to parse as provider type
        let provider_type = ProviderType::from_str(target);

        if manager.has_provider(&provider_type) {
            match manager.set_current(provider_type.clone()) {
                Ok(_) => ModelCommandResult::Success(format!(
                    "✅ Switched to provider: {}",
                    provider_type
                )),
                Err(e) => ModelCommandResult::Error(format!("Failed to switch: {}", e)),
            }
        } else {
            ModelCommandResult::Error(format!(
                "Provider '{}' not found. Use /model list to see available options.",
                target
            ))
        }
    }

    /// Handle /model stats
    fn handle_stats(&self) -> ModelCommandResult {
        let output =
            String::from("📊 Model Statistics:\n\n  Statistics tracking not yet implemented.\n");
        ModelCommandResult::Success(output)
    }
}

/// Parse model command from input
pub fn parse_model_command(input: &str) -> Option<Vec<&str>> {
    if !input.starts_with("/model") {
        return None;
    }

    let parts: Vec<&str> = input
        .trim_start_matches("/model")
        .trim()
        .split_whitespace()
        .collect();

    Some(parts)
}
