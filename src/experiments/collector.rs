//! Experiment data collector for gathering metrics during benchmark execution

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use tracing::info;

use crate::experiments::{EvolutionCycleRecord, GroupSummary, TaskExecutionRecord};

/// Data collector for experiment metrics
pub struct DataCollector {
    /// Base directory for experiment data
    base_dir: PathBuf,
    /// Current experiment group
    group: String,
}

impl DataCollector {
    /// Create a new data collector
    pub fn new(base_dir: PathBuf, group: String) -> Self {
        Self { base_dir, group }
    }

    /// Append a task execution record to the log file
    pub fn record_task(&self, record: &TaskExecutionRecord) -> Result<()> {
        let log_dir = self.base_dir.join("logs").join(&self.group);
        std::fs::create_dir_all(&log_dir)
            .with_context(|| format!("Failed to create log directory: {:?}", log_dir))?;

        let log_file = log_dir.join("task_executions.jsonl");

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .with_context(|| format!("Failed to open log file: {:?}", log_file))?;

        let mut writer = BufWriter::new(file);
        let line =
            serde_json::to_string(record).with_context(|| "Failed to serialize task record")?;

        writeln!(writer, "{}", line).with_context(|| "Failed to write task record")?;

        writer.flush().with_context(|| "Failed to flush writer")?;

        info!("Recorded task execution: {}", record.task_id);
        Ok(())
    }

    /// Append an evolution cycle record to the log file
    pub fn record_evolution(&self, record: &EvolutionCycleRecord) -> Result<()> {
        let log_dir = self.base_dir.join("logs").join(&self.group);
        std::fs::create_dir_all(&log_dir)?;

        let log_file = log_dir.join("evolution_cycles.jsonl");

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)?;

        let mut writer = BufWriter::new(file);
        let line = serde_json::to_string(record)?;

        writeln!(writer, "{}", line)?;
        writer.flush()?;

        info!("Recorded evolution cycle: {}", record.cycle_id);
        Ok(())
    }

    /// Save group summary statistics
    pub fn save_summary(&self, summary: &GroupSummary) -> Result<()> {
        let analysis_dir = self.base_dir.join("analysis");
        std::fs::create_dir_all(&analysis_dir)?;

        let summary_file = analysis_dir.join(format!("{}_summary.json", self.group));

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&summary_file)?;

        let mut writer = BufWriter::new(file);
        let json = serde_json::to_string_pretty(summary)?;

        writeln!(writer, "{}", json)?;
        writer.flush()?;

        info!("Saved group summary: {}", summary.group);
        Ok(())
    }

    /// Load all task records for a group
    pub fn load_task_records(&self) -> Result<Vec<TaskExecutionRecord>> {
        let log_dir = self.base_dir.join("logs").join(&self.group);

        if !log_dir.exists() {
            return Ok(Vec::new());
        }

        let log_file = log_dir.join("task_executions.jsonl");
        if !log_file.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&log_file)?;
        let mut records = Vec::new();

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let record: TaskExecutionRecord = serde_json::from_str(line)?;
            records.push(record);
        }

        Ok(records)
    }

    /// Load all evolution records for a group
    pub fn load_evolution_records(&self) -> Result<Vec<EvolutionCycleRecord>> {
        let log_dir = self.base_dir.join("logs").join(&self.group);

        if !log_dir.exists() {
            return Ok(Vec::new());
        }

        let log_file = log_dir.join("evolution_cycles.jsonl");
        if !log_file.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&log_file)?;
        let mut records = Vec::new();

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let record: EvolutionCycleRecord = serde_json::from_str(line)?;
            records.push(record);
        }

        Ok(records)
    }
}

/// Aggregated experiment metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentMetrics {
    /// Group name
    pub group: String,
    /// Total tasks executed
    pub total_tasks: u64,
    /// Successful tasks
    pub successful_tasks: u64,
    /// Success rate (0-1)
    pub success_rate: f64,
    /// Average tool calls per task
    pub avg_tool_calls: f64,
    /// Average execution time (ms)
    pub avg_execution_time_ms: f64,
    /// Average satisfaction score (1-5)
    pub avg_satisfaction: f64,
    /// Total gaps detected
    pub total_gaps_detected: u64,
    /// Total tools created
    pub total_tools_created: u64,
    /// Total tools optimized
    pub total_tools_optimized: u64,
    /// Total API calls
    pub total_api_calls: u64,
    /// Total API cost (USD)
    pub total_api_cost_usd: f64,
    /// Total evolution cycles
    pub total_evolution_cycles: u64,
}

impl ExperimentMetrics {
    /// Calculate metrics from task and evolution records
    pub fn from_records(
        group: &str,
        task_records: &[TaskExecutionRecord],
        evolution_records: &[EvolutionCycleRecord],
    ) -> Self {
        let total_tasks = task_records.len() as u64;
        let successful_tasks = task_records.iter().filter(|r| r.success).count() as u64;

        let total_tool_calls: u64 = task_records.iter().map(|r| r.total_tool_calls as u64).sum();
        let total_execution_time: u64 = task_records.iter().map(|r| r.execution_time_ms).sum();
        let total_satisfaction: u64 = task_records
            .iter()
            .map(|r| r.user_satisfaction as u64)
            .sum();
        let total_gaps: u64 = task_records.iter().map(|r| r.gaps_detected as u64).sum();
        let total_created: u64 = task_records.iter().map(|r| r.tools_created as u64).sum();
        let total_optimized: u64 = task_records.iter().map(|r| r.tools_optimized as u64).sum();

        let total_api_calls: u64 = evolution_records
            .iter()
            .map(|r| r.metrics.api_calls as u64)
            .sum();
        let total_api_cost: f64 = evolution_records
            .iter()
            .map(|r| r.metrics.api_cost_usd as f64)
            .sum();

        Self {
            group: group.to_string(),
            total_tasks,
            successful_tasks,
            success_rate: if total_tasks > 0 {
                successful_tasks as f64 / total_tasks as f64
            } else {
                0.0
            },
            avg_tool_calls: if total_tasks > 0 {
                total_tool_calls as f64 / total_tasks as f64
            } else {
                0.0
            },
            avg_execution_time_ms: if total_tasks > 0 {
                total_execution_time as f64 / total_tasks as f64
            } else {
                0.0
            },
            avg_satisfaction: if total_tasks > 0 {
                total_satisfaction as f64 / total_tasks as f64
            } else {
                0.0
            },
            total_gaps_detected: total_gaps,
            total_tools_created: total_created,
            total_tools_optimized: total_optimized,
            total_api_calls,
            total_api_cost_usd: total_api_cost,
            total_evolution_cycles: evolution_records.len() as u64,
        }
    }

    /// Save metrics to JSON file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let parent = path.parent().context("Invalid path")?;
        std::fs::create_dir_all(parent)?;

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;

        let mut writer = BufWriter::new(file);
        let json = serde_json::to_string_pretty(self)?;
        writeln!(writer, "{}", json)?;
        writer.flush()?;

        Ok(())
    }
}
