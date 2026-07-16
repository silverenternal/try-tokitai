use super::{PlanNode, ResearchEstimate, ResearchGoalInput, ScientificPlan};
use crate::atlas_core::{
    AtlasCore, ObjectType, RelationshipKind, ScientificObject,
};
use anyhow::Result;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct PlanningEngine {
    core: Arc<AtlasCore>,
}

impl PlanningEngine {
    pub fn new(core: Arc<AtlasCore>) -> Self {
        Self { core }
    }

    pub fn plan(&self, goal: ResearchGoalInput, actor: &str) -> Result<ScientificPlan> {
        let estimate = estimate_goal(&goal);
        let mut goal_object = ScientificObject::new("research-goal", &goal.title, actor);
        goal_object.description = goal.description.clone();
        goal_object.tags.extend(
            [goal.domain.clone(), "research-goal".into()]
                .into_iter()
                .filter(|value| !value.is_empty()),
        );
        goal_object.metadata.insert("constraints".into(), goal.constraints.clone());
        goal_object.metadata.insert("estimate".into(), serde_json::to_value(&estimate)?);
        goal_object.metadata.insert(
            "target_publication".into(),
            serde_json::to_value(&goal.target_publication)?,
        );
        goal_object.rebuild_search_index();
        let goal_object = self.core.create(goal_object, actor)?;

        let definitions = planning_nodes(&goal);
        let mut nodes = Vec::new();
        let mut previous = goal_object.id.clone();
        for (object_type, label, description) in definitions {
            let mut object = ScientificObject::new(ObjectType::from(object_type), label, actor);
            object.description = description;
            object.metadata.insert("research_goal_id".into(), json!(goal_object.id));
            object.rebuild_search_index();
            let object = self.core.create(object, actor)?;
            self.core.relate(
                &object.id,
                &previous,
                RelationshipKind::DependsOn,
                actor,
                BTreeMap::new(),
            )?;
            nodes.push(PlanNode {
                object_id: object.id.clone(),
                node_type: object.object_type.0.clone(),
                label: object.display_name.clone(),
                dependencies: vec![previous],
            });
            previous = object.id;
        }

        let mut plan_object = ScientificObject::new("experiment-plan", format!("Plan: {}", goal.title), actor);
        plan_object.description = "Object-backed scientific plan and execution graph".into();
        plan_object.metadata.insert("estimate".into(), serde_json::to_value(&estimate)?);
        plan_object.metadata.insert("nodes".into(), serde_json::to_value(&nodes)?);
        plan_object.rebuild_search_index();
        let plan_object = self.core.create(plan_object, actor)?;
        self.core.relate(
            &plan_object.id,
            &goal_object.id,
            RelationshipKind::BelongsTo,
            actor,
            BTreeMap::new(),
        )?;
        for related in &goal.related_object_ids {
            if self.core.get(related).is_ok() {
                self.core.relate(
                    &goal_object.id,
                    related,
                    RelationshipKind::RelatedTo,
                    actor,
                    BTreeMap::new(),
                )?;
            }
        }

        Ok(ScientificPlan {
            goal_object_id: goal_object.id,
            plan_object_id: plan_object.id,
            version: plan_object.version,
            estimate,
            execution_order: nodes.iter().map(|node| node.object_id.clone()).collect(),
            nodes,
        })
    }

    pub fn fork(&self, plan_object_id: &str, actor: &str) -> Result<ScientificObject> {
        self.core.fork(plan_object_id, actor, "planning alternative")
    }

    pub fn merge(&self, target: &str, source: &str, actor: &str) -> Result<ScientificObject> {
        self.core.merge(target, source, actor)
    }
}

fn planning_nodes(goal: &ResearchGoalInput) -> Vec<(&'static str, String, String)> {
    vec![
        ("research-question", format!("Question: {}", goal.title), "Falsifiable research question".into()),
        ("hypothesis", format!("Hypothesis: {}", goal.title), "Initial testable explanation; Agent must refine before execution".into()),
        ("paper-analysis", "Paper analysis".into(), "Literature and prior-art evidence requirements".into()),
        ("dataset-requirement", "Dataset selection".into(), "Dataset constraints, license, version and leakage checks".into()),
        ("method", "Method selection".into(), "Baseline and candidate method definitions".into()),
        ("risk-analysis", "Risk analysis".into(), "Failure probability, resources and mitigation".into()),
        ("execution-plan", "Execution graph".into(), "Runtime-independent execution DAG".into()),
        ("expected-result", "Expected results".into(), "Metrics, stopping criteria and expected contribution".into()),
        ("publication-target", goal.target_publication.clone().unwrap_or_else(|| "Publication structure".into()), "Publication outline backed by planned evidence".into()),
    ]
}

fn estimate_goal(goal: &ResearchGoalInput) -> ResearchEstimate {
    let text = format!("{} {}", goal.title, goal.description).to_ascii_lowercase();
    let gpu = ["gpu", "cuda", "training", "transformer", "vision", "llm"]
        .iter()
        .filter(|term| text.contains(**term))
        .count() as f64;
    let empirical = ["benchmark", "experiment", "accuracy", "latency", "dataset"]
        .iter()
        .filter(|term| text.contains(**term))
        .count() as f64;
    let novelty = (0.45 + 0.05 * text.matches("new").count() as f64).clamp(0.0, 0.9);
    let difficulty = (0.4 + gpu * 0.08 + empirical * 0.04).clamp(0.0, 0.95);
    ResearchEstimate {
        novelty,
        difficulty,
        gpu_cost: gpu * 0.5,
        execution_hours: (1.0 + difficulty * 12.0 + gpu * 4.0).max(1.0),
        paper_support: (empirical * 0.12).clamp(0.0, 0.8),
        scientific_confidence: (0.35 + empirical * 0.08).clamp(0.0, 0.8),
        failure_probability: (0.2 + difficulty * 0.5).clamp(0.0, 0.9),
        publication_probability: (0.12 + novelty * 0.25 + empirical * 0.04).clamp(0.0, 0.65),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn creates_object_backed_planning_dag() {
        let directory = tempdir().unwrap();
        let core = Arc::new(AtlasCore::open(directory.path()).unwrap());
        let planner = PlanningEngine::new(core.clone());
        let plan = planner
            .plan(
                ResearchGoalInput {
                    title: "Improve compiler optimization".into(),
                    description: "Benchmark a new pass against baselines".into(),
                    domain: "compiler".into(),
                    constraints: json!({}),
                    target_publication: None,
                    related_object_ids: vec![],
                },
                "agent",
            )
            .unwrap();
        assert_eq!(plan.nodes.len(), 9);
        assert!(core.graph().unwrap().relationships.len() >= 10);
    }
}
