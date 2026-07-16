//! Benchmark task definitions for experiment framework
//!
//! This module provides benchmark task loading and generation for experiments.
//! Tasks are loaded from JSON files in experiments/tasks/ directory.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Simple benchmark task structure matching JSON format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkTask {
    /// Task unique identifier
    pub id: String,
    /// Task category (file_ops, code_analysis, network, git_ops, data_processing, system_monitor, composite)
    pub category: String,
    /// Task difficulty (easy, medium, hard)
    pub difficulty: String,
    /// Task description
    pub description: String,
    /// Expected tools to complete the task
    #[serde(default)]
    pub expected_tools: Vec<String>,
    /// Expected duration in milliseconds
    #[serde(default)]
    pub expected_duration_ms: u64,
    /// Success criteria description
    #[serde(default)]
    pub success_criteria: String,
}

impl BenchmarkTask {
    /// Check if task difficulty matches
    pub fn is_difficulty(&self, diff: &str) -> bool {
        self.difficulty.to_lowercase() == diff.to_lowercase()
    }

    /// Check if task category matches
    pub fn is_category(&self, cat: &str) -> bool {
        self.category.to_lowercase() == cat.to_lowercase()
    }
}

/// Load benchmark tasks from JSON file
pub fn load_benchmark_tasks_from_file(path: &Path) -> anyhow::Result<Vec<BenchmarkTask>> {
    use std::fs;

    if !path.exists() {
        anyhow::bail!("Benchmark tasks file not found: {:?}", path);
    }

    let content = fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read benchmark tasks file: {}", e))?;

    let data: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))?;

    let tasks_array = data
        .get("tasks")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("No 'tasks' array found in benchmark file"))?;

    let mut tasks = Vec::new();
    for task_json in tasks_array {
        let task = serde_json::from_value::<BenchmarkTask>(task_json.clone())
            .map_err(|e| anyhow::anyhow!("Failed to parse task: {}", e))?;
        tasks.push(task);
    }

    Ok(tasks)
}
