//! Experiment framework for Tokitai Prompt Engineering self-evolution system
//!
//! This module provides the infrastructure for running controlled experiments
//! to validate the effectiveness of the self-evolution system.
//!
//! # Experiment Groups
//!
//! - **Control**: Original tokitai without self-evolution
//! - **Ours-Full**: Complete Prompt Engineering system
//! - **Ours-Single**: Single LLM decision (no multi-agent negotiation)
//! - **Ours-NoCoT**: Without Chain-of-Thought reasoning
//! - **Ours-NoFix**: Without self-correction loop
//!
//! # Usage
//!
//! ```bash
//! # Run benchmark for a single group
//! cargo run -- experiment run --group Ours-Full --days 30
//!
//! # Run all comparison groups
//! cargo run -- experiment run --all-groups
//!
//! # Run ablation study
//! cargo run -- experiment run --ablation
//!
//! # Analyze results
//! cargo run -- experiment analyze
//! ```

pub mod benchmark_tasks;
pub mod cli;
pub mod collector;
pub mod runner;
// pub mod metrics;  // Temporarily disabled due to compilation issues

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Experiment group configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ExperimentGroup {
    /// Control group: original tokitai without self-evolution
    Control,
    /// Full system: complete Prompt Engineering with all features
    OursFull,
    /// Single agent: no multi-agent negotiation
    OursSingle,
    /// No CoT: without Chain-of-Thought reasoning
    OursNoCoT,
    /// No Fix: without self-correction loop
    OursNoFix,
}

impl ExperimentGroup {
    /// Get the log directory name for this group
    pub fn log_dir_name(&self) -> &'static str {
        match self {
            ExperimentGroup::Control => "control",
            ExperimentGroup::OursFull => "ours_full",
            ExperimentGroup::OursSingle => "ours_single",
            ExperimentGroup::OursNoCoT => "ours_nocot",
            ExperimentGroup::OursNoFix => "ours_nofix",
        }
    }

    /// Get description of this group
    pub fn description(&self) -> &'static str {
        match self {
            ExperimentGroup::Control => "Original tokitai (no self-evolution)",
            ExperimentGroup::OursFull => "Complete Prompt Engineering system",
            ExperimentGroup::OursSingle => "Single LLM decision (no multi-agent)",
            ExperimentGroup::OursNoCoT => "Without Chain-of-Thought reasoning",
            ExperimentGroup::OursNoFix => "Without self-correction loop",
        }
    }

    /// Check if this group has self-evolution enabled
    pub fn has_evolution(&self) -> bool {
        match self {
            ExperimentGroup::Control => false,
            _ => true,
        }
    }

    /// Check if multi-agent negotiation is enabled
    pub fn has_multi_agent(&self) -> bool {
        match self {
            ExperimentGroup::OursSingle => false,
            _ => true,
        }
    }

    /// Check if Chain-of-Thought is enabled
    pub fn has_cot(&self) -> bool {
        match self {
            ExperimentGroup::OursNoCoT => false,
            _ => true,
        }
    }

    /// Check if self-correction is enabled
    pub fn has_self_fix(&self) -> bool {
        match self {
            ExperimentGroup::OursNoFix => false,
            _ => true,
        }
    }

    /// Parse group from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "control" => Some(ExperimentGroup::Control),
            "ours-full" | "ours_full" => Some(ExperimentGroup::OursFull),
            "ours-single" | "ours_single" => Some(ExperimentGroup::OursSingle),
            "ours-nocot" | "ours_nocot" => Some(ExperimentGroup::OursNoCoT),
            "ours-nofix" | "ours_nofix" => Some(ExperimentGroup::OursNoFix),
            _ => None,
        }
    }
}

/// Task execution record for experiments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionRecord {
    /// Unique task identifier
    pub task_id: String,
    /// Task category (file_ops, code_analysis, etc.)
    pub category: String,
    /// Task difficulty (easy, medium, hard)
    pub difficulty: String,
    /// Task description
    pub description: String,
    /// Experiment group
    pub group: String,
    /// Execution timestamp
    pub timestamp: DateTime<Utc>,
    /// Whether the task was successful
    pub success: bool,
    /// List of tool calls made during execution
    pub tool_calls: Vec<ToolCallRecord>,
    /// Total number of tool calls
    pub total_tool_calls: u32,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// User satisfaction score (1-5)
    pub user_satisfaction: u8,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Number of gaps detected during this task
    pub gaps_detected: u32,
    /// Number of tools created during this task
    pub tools_created: u32,
    /// Number of tools optimized during this task
    pub tools_optimized: u32,
}

impl TaskExecutionRecord {
    /// Create a new task execution record
    pub fn new(
        task_id: String,
        category: String,
        difficulty: String,
        description: String,
        group: String,
    ) -> Self {
        Self {
            task_id,
            category,
            difficulty,
            description,
            group,
            timestamp: Utc::now(),
            success: false,
            tool_calls: Vec::new(),
            total_tool_calls: 0,
            execution_time_ms: 0,
            user_satisfaction: 0,
            error_message: None,
            gaps_detected: 0,
            tools_created: 0,
            tools_optimized: 0,
        }
    }

    /// Mark the task as completed
    pub fn complete(
        &mut self,
        success: bool,
        tool_calls: Vec<ToolCallRecord>,
        execution_time_ms: u64,
        satisfaction: u8,
    ) {
        self.success = success;
        self.total_tool_calls = tool_calls.len() as u32;
        self.tool_calls = tool_calls;
        self.execution_time_ms = execution_time_ms;
        self.user_satisfaction = satisfaction;
    }

    /// Mark the task as failed
    pub fn fail(&mut self, error: String) {
        self.success = false;
        self.error_message = Some(error);
    }
}

/// Individual tool call record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    /// Tool name
    pub tool: String,
    /// Tool arguments
    pub args: serde_json::Value,
    /// Result status
    pub result: String,
    /// Execution time in milliseconds
    pub execution_time_ms: Option<u64>,
}

/// Self-evolution cycle record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionCycleRecord {
    /// Cycle identifier
    pub cycle_id: String,
    /// Experiment group
    pub group: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Reflection results
    pub reflection: ReflectionRecord,
    /// Gaps detected in this cycle
    pub gaps_detected: Vec<GapRecord>,
    /// Actions taken in this cycle
    pub actions_taken: Vec<ActionRecord>,
    /// Metrics for this cycle
    pub metrics: EvolutionMetrics,
}

/// Reflection results from a cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionRecord {
    /// System coverage score (0-1)
    pub coverage_score: f32,
    /// Identified systemic issues
    pub systemic_issues: Vec<String>,
    /// Strategic recommendations
    pub strategic_recommendations: Vec<String>,
}

/// Detected tool gap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapRecord {
    /// Type of gap (missing_tool, performance_issue, etc.)
    pub gap_type: String,
    /// Description of the gap
    pub description: String,
    /// Suggested tool name if applicable
    pub suggested_name: Option<String>,
    /// Priority score (1-10)
    pub priority: u8,
}

/// Action taken during evolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecord {
    /// Type of action (create_tool, optimize_tool, etc.)
    pub action_type: String,
    /// Tool name if applicable
    pub tool_name: Option<String>,
    /// Result of the action
    pub result: String,
    /// Number of compilation attempts
    pub compilation_attempts: Option<u32>,
}

/// Evolution metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionMetrics {
    /// Number of API calls made
    pub api_calls: u32,
    /// API cost in USD
    pub api_cost_usd: f32,
    /// Cycle duration in milliseconds
    pub cycle_duration_ms: u64,
}

impl Default for EvolutionMetrics {
    fn default() -> Self {
        Self {
            api_calls: 0,
            api_cost_usd: 0.0,
            cycle_duration_ms: 0,
        }
    }
}

/// Experiment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentConfig {
    /// Number of days to run the experiment
    pub days: u32,
    /// Project path
    pub project_path: PathBuf,
    /// Output directory for logs
    pub log_dir: PathBuf,
    /// Enable verbose logging
    pub verbose: bool,
}

impl Default for ExperimentConfig {
    fn default() -> Self {
        Self {
            days: 1,
            project_path: std::env::current_dir().unwrap_or_default(),
            log_dir: PathBuf::from("experiments/logs"),
            verbose: false,
        }
    }
}

/// Summary statistics for an experiment group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupSummary {
    /// Group name
    pub group: String,
    /// Total tasks executed
    pub total_tasks: u32,
    /// Successful tasks
    pub successful_tasks: u32,
    /// Total tool calls made
    pub total_tool_calls: u32,
    /// Total API cost in USD
    pub api_cost_usd: f32,
    /// Total tools created
    pub tools_created: u32,
    /// Total tools optimized
    pub tools_optimized: u32,
    /// Average satisfaction score
    pub avg_satisfaction: f32,
    /// Success rate (0-1)
    pub success_rate: f32,
    /// Average tool calls per task
    pub avg_tool_calls: f32,
}

impl GroupSummary {
    /// Calculate summary from task records
    pub fn from_records(group: &str, records: &[TaskExecutionRecord]) -> Self {
        let total_tasks = records.len() as u32;
        let successful_tasks = records.iter().filter(|r| r.success).count() as u32;
        let total_tool_calls: u32 = records.iter().map(|r| r.total_tool_calls).sum();
        let total_satisfaction: u32 = records.iter().map(|r| r.user_satisfaction as u32).sum();
        let tools_created: u32 = records.iter().map(|r| r.tools_created).sum();
        let tools_optimized: u32 = records.iter().map(|r| r.tools_optimized).sum();

        Self {
            group: group.to_string(),
            total_tasks,
            successful_tasks,
            total_tool_calls,
            api_cost_usd: 0.0,
            tools_created,
            tools_optimized,
            avg_satisfaction: if total_tasks > 0 {
                total_satisfaction as f32 / total_tasks as f32
            } else {
                0.0
            },
            success_rate: if total_tasks > 0 {
                successful_tasks as f32 / total_tasks as f32
            } else {
                0.0
            },
            avg_tool_calls: if total_tasks > 0 {
                total_tool_calls as f32 / total_tasks as f32
            } else {
                0.0
            },
        }
    }
}
