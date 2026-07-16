//! ExperimentAgent — Implementation, benchmark, and evaluation design

use crate::scientist::tools::data::{
    build_default_benchmark_plan, build_default_benchmark_plan_with_paper_hints,
    infer_benchmark_profile,
};
use ai_scientist_core::agent::{
    Agent, AgentContext, AgentError, AgentMessage, AgentResponse, AgentRole, Capability,
};
use async_trait::async_trait;

pub struct ExperimentAgent {
    id: String,
}

impl ExperimentAgent {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

#[async_trait]
impl Agent for ExperimentAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn role(&self) -> AgentRole {
        AgentRole::Experimenter
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability {
            name: "benchmark_design".into(),
            description: "Design a reproducible implementation and evaluation plan for a CS research question".into(),
            required_tools: vec![
                "run_python".into(),
                "inspect_dataset".into(),
                "search_public_datasets".into(),
                "fetch_public_dataset_manifest".into(),
                "git_diff".into(),
            ],
        }]
    }

    async fn handle_message(
        &self,
        msg: AgentMessage,
        _ctx: &AgentContext,
    ) -> Result<AgentResponse, AgentError> {
        let problem_formulation = msg
            .payload
            .get("problem_formulation")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let paper_dataset_hints = msg
            .payload
            .get("paper_dataset_hints")
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str())
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let benchmark_profile = infer_benchmark_profile(problem_formulation);
        let benchmark_plan = if paper_dataset_hints.is_empty() {
            build_default_benchmark_plan(problem_formulation)
        } else {
            build_default_benchmark_plan_with_paper_hints(problem_formulation, &paper_dataset_hints)
        };

        Ok(AgentResponse::ok(serde_json::json!({
            "agent": self.id,
            "problem_formulation": problem_formulation,
            "benchmark_profile": benchmark_profile,
            "paper_dataset_hints": paper_dataset_hints,
            "benchmark_plan": benchmark_plan,
            "experiment": {
                "design": "Reproducible benchmark pipeline",
                "artifacts": ["dataset split", "training or execution script", "evaluation report"],
                "metrics": ["accuracy", "latency", "memory", "task-specific score"],
                "methodology": "Baseline comparison with fixed seeds and documented configuration",
                "execution_schema": benchmark_plan["execution_schema"].clone(),
                "result_bundle_schema": benchmark_plan["result_bundle_schema"].clone(),
                "lineage_schema": benchmark_plan["lineage_schema"].clone(),
                "dataset_acquisition": benchmark_plan["dataset_acquisition"].clone(),
                "dataset_next_steps": if paper_dataset_hints.is_empty() {
                    serde_json::json!([
                        "Search official dataset databases and provider sites before freezing the split or benchmark corpus.",
                        "Materialize a dataset manifest with provider, path, format, and task hint before running evaluation.",
                        "Keep paper retrieval on official paper APIs even when dataset discovery uses direct provider/database search."
                    ])
                } else {
                    serde_json::json!([
                        "Start with dataset names recovered from official paper APIs, then resolve them through search_public_datasets.",
                        "Materialize a dataset manifest with provider, path, format, and task hint before running evaluation.",
                        "Keep paper retrieval on official paper APIs even when dataset discovery uses direct provider/database search."
                    ])
                },
                "profile_summary": match benchmark_profile {
                    "classical_ml" => "Focus on dataset entrypoint selection, fixed splits or cross-validation, simple reproducible baselines, and concise metric reporting.",
                    "deep_learning" => "Focus on dataset entrypoint selection, training configuration, checkpointing, validation monitoring, and resource-aware evaluation.",
                    "systems_evaluation" => "Focus on workload-trace selection, instrumentation, latency/throughput analysis, and reproducible runtime conditions.",
                    "agent_evaluation" => "Focus on task-suite acquisition, trajectory quality, tool-use behavior, and task success accounting.",
                    "security_analysis" => "Focus on benchmark-corpus acquisition, target coverage, actionable findings, false-positive control, and reproducible detection settings.",
                    _ => "Focus on a reproducible CS benchmark with explicit dataset acquisition, baselines, artifacts, and validation criteria.",
                }
            },
            "status": "Benchmark plan designed"
        }))
        .with_next_role(AgentRole::Verifier))
    }
}
