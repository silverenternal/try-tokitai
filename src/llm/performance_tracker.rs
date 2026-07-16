//! Performance Tracker for LLM Models
//!
//! Tracks latency, success rates, and other performance metrics for each model.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Task type classification
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    CodeGeneration,
    CodeReview,
    Refactoring,
    Debugging,
    Documentation,
    Research,
    General,
}

impl TaskType {
    pub fn as_str(&self) -> &str {
        match self {
            TaskType::CodeGeneration => "code_generation",
            TaskType::CodeReview => "code_review",
            TaskType::Refactoring => "refactoring",
            TaskType::Debugging => "debugging",
            TaskType::Documentation => "documentation",
            TaskType::Research => "research",
            TaskType::General => "general",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "code_generation" | "code_gen" => TaskType::CodeGeneration,
            "code_review" | "review" => TaskType::CodeReview,
            "refactoring" | "refactor" => TaskType::Refactoring,
            "debugging" | "debug" => TaskType::Debugging,
            "documentation" | "docs" => TaskType::Documentation,
            "research" | "search" => TaskType::Research,
            _ => TaskType::General,
        }
    }
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Model profile with cost and capability information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    /// Model name (e.g., "gpt-4o", "claude-3-5-sonnet-20241022")
    pub model_name: String,
    /// Provider type
    pub provider: crate::llm::ProviderType,
    /// Cost per 1K tokens (USD)
    pub cost_per_1k_tokens: f64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// Quality score (0-10)
    pub quality_score: f64,
    /// Context window size (tokens)
    pub context_window: usize,
    /// Supported capabilities
    pub supported_capabilities: Vec<String>,
}

impl ModelProfile {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        model_name: String,
        provider: crate::llm::ProviderType,
        cost_per_1k_tokens: f64,
        avg_latency_ms: f64,
        quality_score: f64,
        context_window: usize,
        supported_capabilities: Vec<String>,
    ) -> Self {
        Self {
            model_name,
            provider,
            cost_per_1k_tokens,
            avg_latency_ms,
            quality_score,
            context_window,
            supported_capabilities,
        }
    }

    /// Create a profile with default values for quick setup
    pub fn with_defaults(model_name: String, provider: crate::llm::ProviderType) -> Self {
        Self {
            model_name,
            provider,
            cost_per_1k_tokens: 0.0,
            avg_latency_ms: 1000.0,
            quality_score: 5.0,
            context_window: 4096,
            supported_capabilities: vec!["chat".to_string()],
        }
    }
}

/// Performance statistics for a model
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelPerformanceStats {
    /// Total number of calls
    pub total_calls: u64,
    /// Number of successful calls
    pub successful_calls: u64,
    /// Number of failed calls
    pub failed_calls: u64,
    /// Average latency (ms)
    pub avg_latency_ms: f64,
    /// Total latency (for calculating average)
    #[serde(skip)]
    total_latency_ms: f64,
    /// Latency samples (for moving average)
    #[serde(skip)]
    latency_samples: u64,
}

impl ModelPerformanceStats {
    pub fn success_rate(&self) -> f64 {
        if self.total_calls == 0 {
            return 0.0;
        }
        self.successful_calls as f64 / self.total_calls as f64
    }

    pub fn record_latency(&mut self, latency_ms: f64) {
        // Use exponential moving average
        let alpha = 0.1;
        if self.latency_samples == 0 {
            self.avg_latency_ms = latency_ms;
        } else {
            self.avg_latency_ms = alpha * latency_ms + (1.0 - alpha) * self.avg_latency_ms;
        }
        self.total_latency_ms += latency_ms;
        self.latency_samples += 1;
    }

    pub fn record_success(&mut self) {
        self.successful_calls += 1;
        self.total_calls += 1;
    }

    pub fn record_failure(&mut self) {
        self.failed_calls += 1;
        self.total_calls += 1;
    }
}

/// Performance tracker for all models
pub struct PerformanceTracker {
    models: RwLock<HashMap<String, ModelPerformanceStats>>,
}

impl PerformanceTracker {
    pub fn new() -> Self {
        Self {
            models: RwLock::new(HashMap::new()),
        }
    }

    /// Record latency for a model
    pub fn record_latency(&self, model_key: &str, latency_ms: f64) {
        let mut models = self.models.write();
        let stats = models.entry(model_key.to_string()).or_default();
        stats.record_latency(latency_ms);
    }

    /// Record a successful call
    pub fn record_success(&self, model_key: &str) {
        let mut models = self.models.write();
        let stats = models.entry(model_key.to_string()).or_default();
        stats.record_success();
    }

    /// Record a failed call
    pub fn record_failure(&self, model_key: &str) {
        let mut models = self.models.write();
        let stats = models.entry(model_key.to_string()).or_default();
        stats.record_failure();
    }

    /// Get statistics for a specific model
    pub fn get_model_stats(&self, model_key: &str) -> ModelPerformanceStats {
        let models = self.models.read();
        models.get(model_key).cloned().unwrap_or_default()
    }

    /// Get all model statistics
    pub fn get_all_stats(&self) -> HashMap<String, ModelPerformanceStats> {
        let models = self.models.read();
        models.clone()
    }

    /// Get the best performing model by success rate
    pub fn get_best_by_success_rate(&self) -> Option<(String, f64)> {
        let models = self.models.read();
        models
            .iter()
            .filter(|(_, stats)| stats.total_calls > 0)
            .max_by(|a, b| a.1.success_rate().partial_cmp(&b.1.success_rate()).unwrap())
            .map(|(key, stats)| (key.clone(), stats.success_rate()))
    }

    /// Get the fastest model by average latency
    pub fn get_fastest(&self) -> Option<(String, f64)> {
        let models = self.models.read();
        models
            .iter()
            .filter(|(_, stats)| stats.latency_samples > 0)
            .min_by(|a, b| a.1.avg_latency_ms.partial_cmp(&b.1.avg_latency_ms).unwrap())
            .map(|(key, stats)| (key.clone(), stats.avg_latency_ms))
    }

    /// Clear all statistics
    pub fn clear(&self) {
        let mut models = self.models.write();
        models.clear();
    }

    /// Export statistics to JSON
    pub fn export_to_json(&self) -> Result<String, serde_json::Error> {
        let models = self.models.read();
        serde_json::to_string_pretty(&*models)
    }

    /// Import statistics from JSON
    pub fn import_from_json(&self, json: &str) -> Result<(), serde_json::Error> {
        let stats: HashMap<String, ModelPerformanceStats> = serde_json::from_str(json)?;
        let mut models = self.models.write();
        *models = stats;
        Ok(())
    }
}

impl Default for PerformanceTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Task-to-model mapping for intelligent routing
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskModelMapping {
    mappings: HashMap<TaskType, String>,
}

impl TaskModelMapping {
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
        }
    }

    pub fn set_mapping(&mut self, task_type: TaskType, model_key: String) {
        self.mappings.insert(task_type, model_key);
    }

    pub fn get_model(&self, task_type: &TaskType) -> Option<&String> {
        self.mappings.get(task_type)
    }

    /// Get recommended model for a task type based on historical performance
    pub fn get_recommended_model(
        task_type: &TaskType,
        tracker: &PerformanceTracker,
        models: &HashMap<String, ModelProfile>,
    ) -> Option<String> {
        // For code-related tasks, prefer higher quality models
        let min_quality = match task_type {
            TaskType::CodeGeneration | TaskType::CodeReview | TaskType::Refactoring => 7.0,
            TaskType::Debugging => 7.5,
            TaskType::Documentation => 6.0,
            TaskType::Research => 6.5,
            TaskType::General => 5.0,
        };

        // Filter models by quality
        let candidates: Vec<&ModelProfile> = models
            .values()
            .filter(|m| m.quality_score >= min_quality)
            .collect();

        if candidates.is_empty() {
            return None;
        }

        // Among candidates, pick the one with best historical performance
        let best = candidates.iter().max_by(|a, b| {
            let key_a = format!("{}/{}", a.provider, a.model_name);
            let key_b = format!("{}/{}", b.provider, b.model_name);
            let stats_a = tracker.get_model_stats(&key_a);
            let stats_b = tracker.get_model_stats(&key_b);

            // Weight: 60% success rate, 40% latency
            let score_a =
                stats_a.success_rate() * 0.6 + (1.0 - stats_a.avg_latency_ms / 10000.0) * 0.4;
            let score_b =
                stats_b.success_rate() * 0.6 + (1.0 - stats_b.avg_latency_ms / 10000.0) * 0.4;
            score_a.partial_cmp(&score_b).unwrap()
        });

        best.map(|m| format!("{}/{}", m.provider, m.model_name))
    }
}
