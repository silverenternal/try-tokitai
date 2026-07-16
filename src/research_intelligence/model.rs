use crate::atlas_core::{ObjectId, ObjectType};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResearchGoalInput {
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub constraints: Value,
    #[serde(default)]
    pub target_publication: Option<String>,
    #[serde(default)]
    pub related_object_ids: Vec<ObjectId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResearchEstimate {
    pub novelty: f64,
    pub difficulty: f64,
    pub gpu_cost: f64,
    pub execution_hours: f64,
    pub paper_support: f64,
    pub scientific_confidence: f64,
    pub failure_probability: f64,
    pub publication_probability: f64,
}

impl Default for ResearchEstimate {
    fn default() -> Self {
        Self {
            novelty: 0.5,
            difficulty: 0.5,
            gpu_cost: 0.0,
            execution_hours: 1.0,
            paper_support: 0.0,
            scientific_confidence: 0.5,
            failure_probability: 0.5,
            publication_probability: 0.2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanNode {
    pub object_id: ObjectId,
    pub node_type: String,
    pub label: String,
    pub dependencies: Vec<ObjectId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScientificPlan {
    pub goal_object_id: ObjectId,
    pub plan_object_id: ObjectId,
    pub version: u64,
    pub estimate: ResearchEstimate,
    pub nodes: Vec<PlanNode>,
    pub execution_order: Vec<ObjectId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Planned,
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
    Blocked,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionTaskSpec {
    pub title: String,
    pub goal: String,
    pub priority: i32,
    #[serde(default)]
    pub dependencies: Vec<ObjectId>,
    #[serde(default)]
    pub scientific_object_ids: Vec<ObjectId>,
    #[serde(default)]
    pub required_capabilities: BTreeSet<String>,
    #[serde(default)]
    pub expected_output_types: Vec<ObjectType>,
    #[serde(default)]
    pub metrics: BTreeMap<String, Value>,
    #[serde(default)]
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionObservation {
    pub task_object_id: ObjectId,
    pub status: ExecutionStatus,
    #[serde(default)]
    pub metrics: BTreeMap<String, Value>,
    #[serde(default)]
    pub evidence_object_ids: Vec<ObjectId>,
    #[serde(default)]
    pub artifact_paths: Vec<String>,
    #[serde(default)]
    pub failure: Option<FailureAnalysis>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FailureAnalysis {
    pub category: FailureCategory,
    pub summary: String,
    pub retryable: bool,
    pub recommended_strategy: String,
    #[serde(default)]
    pub parameter_patch: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FailureCategory {
    Timeout,
    RuntimeFailure,
    MemoryOverflow,
    LowAccuracy,
    BadConvergence,
    RuntimeBusy,
    InvalidDataset,
    MissingDependency,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeRequest {
    pub task_object_id: ObjectId,
    pub required_capabilities: BTreeSet<String>,
    #[serde(default)]
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeResult {
    pub runtime_object_id: ObjectId,
    pub success: bool,
    pub summary: String,
    #[serde(default)]
    pub metrics: BTreeMap<String, Value>,
    #[serde(default)]
    pub artifact_paths: Vec<String>,
    #[serde(default)]
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Recommendation {
    pub object_id: ObjectId,
    pub category: RecommendationCategory,
    pub title: String,
    pub reason: String,
    pub score: RecommendationScore,
    #[serde(default)]
    pub evidence_object_ids: Vec<ObjectId>,
    #[serde(default)]
    pub related_object_ids: Vec<ObjectId>,
    #[serde(default)]
    pub estimated_runtime_hours: f64,
    #[serde(default)]
    pub expected_improvement: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationCategory {
    NextExperiment,
    NextHypothesis,
    NextDataset,
    NextPaper,
    NextBenchmark,
    NextAblation,
    NextRuntime,
    NextVisualization,
    NextReviewer,
    NextPublication,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RecommendationScore {
    pub expected_gain: f64,
    pub novelty: f64,
    pub risk: f64,
    pub scientific_confidence: f64,
    pub gpu_cost: f64,
    pub execution_time: f64,
    pub paper_support: f64,
    pub failure_probability: f64,
    pub recommendation_confidence: f64,
    pub total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObjectQuery {
    #[serde(default)]
    pub object_types: BTreeSet<ObjectType>,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub filters: Vec<QueryFilter>,
    #[serde(default = "default_query_limit")]
    pub limit: usize,
}

fn default_query_limit() -> usize {
    100
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueryFilter {
    pub field: String,
    pub operator: QueryOperator,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueryOperator {
    Eq,
    NotEq,
    Gt,
    Gte,
    Lt,
    Lte,
    Contains,
    In,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueryView {
    Table,
    Graph,
    Timeline,
    Tree,
    Dependency,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum PluginCategory {
    Domain,
    Runtime,
    Visualization,
    ScientificObject,
    Execution,
    Recommendation,
    Knowledge,
    Simulation,
    AiAgent,
    Workspace,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub author: String,
    pub categories: BTreeSet<PluginCategory>,
    #[serde(default)]
    pub capabilities: BTreeSet<String>,
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
    #[serde(default)]
    pub runtimes: Vec<String>,
    #[serde(default)]
    pub scientific_object_types: Vec<ObjectType>,
    #[serde(default)]
    pub visualizations: Vec<String>,
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub permissions: BTreeSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PluginContributions {
    #[serde(default)]
    pub object_types: Vec<ObjectType>,
    #[serde(default)]
    pub runtime_ids: Vec<String>,
    #[serde(default)]
    pub visualization_providers: Vec<String>,
    #[serde(default)]
    pub workspace_providers: Vec<String>,
    #[serde(default)]
    pub execution_strategies: Vec<String>,
    #[serde(default)]
    pub recommendation_strategies: Vec<String>,
    #[serde(default)]
    pub query_providers: Vec<String>,
    #[serde(default)]
    pub agent_context_providers: Vec<String>,
    #[serde(default)]
    pub event_listeners: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginState {
    Installed,
    Enabled,
    Disabled,
    Unloaded,
}
