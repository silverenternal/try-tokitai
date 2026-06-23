//! Smart Model Router
//!
//! Automatically selects the optimal model based on task type, cost, latency, and quality.

use super::performance_tracker::{ModelProfile, PerformanceTracker, TaskType};
use super::{ChatRequest, Message, ProviderType};
use anyhow::{bail, Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Routing strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingStrategy {
    /// Optimize for lowest cost
    CostOptimized,
    /// Optimize for highest quality
    QualityOptimized,
    /// Optimize for lowest latency
    LatencyOptimized,
    /// Balanced approach (weighted combination)
    Balanced,
}

impl Default for RoutingStrategy {
    fn default() -> Self {
        Self::Balanced
    }
}

/// Model router configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    /// Default routing strategy
    pub strategy: RoutingStrategy,
    /// Maximum acceptable latency (ms)
    pub max_latency_ms: Option<f64>,
    /// Maximum acceptable cost per 1K tokens ($)
    pub max_cost_per_1k: Option<f64>,
    /// Minimum quality score (0-10)
    pub min_quality_score: Option<f64>,
    /// Task type overrides (specific models for specific tasks)
    pub task_overrides: HashMap<TaskType, String>,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            strategy: RoutingStrategy::Balanced,
            max_latency_ms: Some(5000.0),
            max_cost_per_1k: Some(0.10),
            min_quality_score: Some(6.0),
            task_overrides: HashMap::new(),
        }
    }
}

/// Model Router - intelligently selects the best model for each task
pub struct ModelRouter {
    config: RouterConfig,
    performance_tracker: Arc<RwLock<PerformanceTracker>>,
    models: HashMap<String, ModelProfile>,
    current_model: Option<String>,
}

impl ModelRouter {
    pub fn new(config: RouterConfig) -> Self {
        Self {
            config,
            performance_tracker: Arc::new(RwLock::new(PerformanceTracker::new())),
            models: HashMap::new(),
            current_model: None,
        }
    }

    /// Register a model profile
    pub fn register_model(&mut self, profile: ModelProfile) {
        let key = format!("{}/{}", profile.provider, profile.model_name);
        self.models.insert(key, profile);
    }

    /// Get all registered models
    pub fn list_models(&self) -> Vec<&ModelProfile> {
        self.models.values().collect()
    }

    /// Set current model
    pub fn set_current_model(&mut self, model_key: &str) -> Result<()> {
        if !self.models.contains_key(model_key) {
            bail!("Model {} not registered", model_key);
        }
        self.current_model = Some(model_key.to_string());
        Ok(())
    }

    /// Get current model
    pub fn current_model(&self) -> Option<&str> {
        self.current_model.as_deref()
    }

    /// Select the best model for a task type
    pub fn select_model(&self, task_type: &TaskType) -> Result<&ModelProfile> {
        // Check for task-specific override
        if let Some(override_model) = self.config.task_overrides.get(task_type) {
            if let Some(profile) = self.models.get(override_model) {
                return Ok(profile);
            }
        }

        // Filter models based on constraints
        let candidates: Vec<&ModelProfile> = self
            .models
            .values()
            .filter(|m| {
                // Apply latency constraint
                if let Some(max_latency) = self.config.max_latency_ms {
                    if m.avg_latency_ms > max_latency {
                        return false;
                    }
                }
                // Apply cost constraint
                if let Some(max_cost) = self.config.max_cost_per_1k {
                    if m.cost_per_1k_tokens > max_cost {
                        return false;
                    }
                }
                // Apply quality constraint
                if let Some(min_quality) = self.config.min_quality_score {
                    if m.quality_score < min_quality {
                        return false;
                    }
                }
                true
            })
            .collect();

        if candidates.is_empty() {
            bail!("No models match the specified constraints");
        }

        // Select based on strategy
        let selected = match self.config.strategy {
            RoutingStrategy::CostOptimized => candidates.iter().min_by(|a, b| {
                a.cost_per_1k_tokens
                    .partial_cmp(&b.cost_per_1k_tokens)
                    .unwrap()
            }),
            RoutingStrategy::QualityOptimized => candidates
                .iter()
                .max_by(|a, b| a.quality_score.partial_cmp(&b.quality_score).unwrap()),
            RoutingStrategy::LatencyOptimized => candidates
                .iter()
                .min_by(|a, b| a.avg_latency_ms.partial_cmp(&b.avg_latency_ms).unwrap()),
            RoutingStrategy::Balanced => {
                // Weighted score: 40% quality, 30% cost (inverted), 30% latency (inverted)
                candidates.iter().max_by(|a, b| {
                    let score_a = self.calculate_balanced_score(a);
                    let score_b = self.calculate_balanced_score(b);
                    score_a.partial_cmp(&score_b).unwrap()
                })
            }
        };

        selected.copied().context("Failed to select model")
    }

    /// Calculate balanced score for a model
    fn calculate_balanced_score(&self, model: &ModelProfile) -> f64 {
        // Normalize scores (0-10 scale)
        let quality_norm = model.quality_score / 10.0;

        // Cost: lower is better, invert and normalize (assume max $10/1K tokens)
        let cost_norm = 1.0 - (model.cost_per_1k_tokens / 10.0).min(1.0);

        // Latency: lower is better, invert and normalize (assume max 10s)
        let latency_norm = 1.0 - (model.avg_latency_ms / 10000.0).min(1.0);

        // Weighted combination
        0.4 * quality_norm + 0.3 * cost_norm + 0.3 * latency_norm
    }

    /// Record performance metrics after a call
    pub fn record_performance(&self, model_key: &str, latency_ms: f64, success: bool) {
        let tracker = self.performance_tracker.write();
        tracker.record_latency(model_key, latency_ms);
        if success {
            tracker.record_success(model_key);
        } else {
            tracker.record_failure(model_key);
        }
    }

    /// Get performance tracker reference
    pub fn performance_tracker(&self) -> Arc<RwLock<PerformanceTracker>> {
        self.performance_tracker.clone()
    }

    /// Get model statistics
    pub fn get_model_stats(&self, model_key: &str) -> Option<ModelStats> {
        let tracker = self.performance_tracker.read();
        let profile = self.models.get(model_key)?;

        let perf_stats = tracker.get_model_stats(model_key);

        Some(ModelStats {
            model_name: profile.model_name.clone(),
            provider: profile.provider.to_string(),
            avg_latency_ms: perf_stats.avg_latency_ms,
            success_rate: perf_stats.success_rate(),
            total_calls: perf_stats.total_calls,
            cost_per_1k_tokens: profile.cost_per_1k_tokens,
            quality_score: profile.quality_score,
        })
    }

    /// Run benchmark for all models
    pub async fn run_benchmarks(
        &self,
        providers: &HashMap<ProviderType, Arc<dyn super::LLMProvider>>,
    ) -> Result<Vec<BenchmarkResult>> {
        use tokio::time::{timeout, Duration};

        let mut results = Vec::new();
        let test_prompt = "Explain what is Rust programming language in one sentence.";

        for (model_key, profile) in &self.models {
            let provider = match providers.get(&profile.provider) {
                Some(p) => p,
                None => continue,
            };

            let request = ChatRequest {
                model: profile.model_name.clone(),
                messages: vec![Message::user(test_prompt)],
                temperature: 0.7,
                max_tokens: Some(100),
                top_p: None,
                stop: None,
                stream: false,
                tools: None,
                thinking_mode: None,
                reasoning_effort: None,
            };

            let start = std::time::Instant::now();
            let result = timeout(Duration::from_secs(30), provider.chat(request)).await;
            let latency_ms = start.elapsed().as_millis() as f64;

            let benchmark_result = match result {
                Ok(Ok(response)) => BenchmarkResult {
                    model: model_key.clone(),
                    provider: profile.provider.to_string(),
                    latency_ms,
                    success: true,
                    tokens_generated: response.usage.map(|u| u.completion_tokens).unwrap_or(0),
                    error: None,
                },
                Ok(Err(e)) => BenchmarkResult {
                    model: model_key.clone(),
                    provider: profile.provider.to_string(),
                    latency_ms,
                    success: false,
                    tokens_generated: 0,
                    error: Some(e.to_string()),
                },
                Err(_) => BenchmarkResult {
                    model: model_key.clone(),
                    provider: profile.provider.to_string(),
                    latency_ms,
                    success: false,
                    tokens_generated: 0,
                    error: Some("Timeout".to_string()),
                },
            };

            results.push(benchmark_result);
        }

        Ok(results)
    }
}

/// Model statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStats {
    pub model_name: String,
    pub provider: String,
    pub avg_latency_ms: f64,
    pub success_rate: f64,
    pub total_calls: u64,
    pub cost_per_1k_tokens: f64,
    pub quality_score: f64,
}

/// Benchmark result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub model: String,
    pub provider: String,
    pub latency_ms: f64,
    pub success: bool,
    pub tokens_generated: usize,
    pub error: Option<String>,
}
