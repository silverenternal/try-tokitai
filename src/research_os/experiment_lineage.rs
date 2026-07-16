//! Experiment Lineage Module
//!
//! Tracks experiment derivation and relationships.

use super::object_graph::{
    generate_object_id, list_research_objects, read_research_object, write_research_object,
    ResearchObjectType,
};
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExperimentStatus {
    #[serde(rename = "planned")]
    Planned,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
}

impl ExperimentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentNode {
    pub schema_version: String,
    pub id: String,
    pub title: String,
    pub domain_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default)]
    pub parent_experiment_ids: Vec<String>,
    #[serde(default)]
    pub child_experiment_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hypothesis_id: Option<String>,
    #[serde(default)]
    pub parameters: Value,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: String,
}

const EXPERIMENT_SCHEMA: &str = "atlas.research-os.experiment.v1";

pub fn create_experiment(
    workspace_root: &Path,
    title: &str,
    domain_id: &str,
    task_id: Option<String>,
    hypothesis_id: Option<String>,
    parameters: Value,
    created_by: &str,
) -> Result<ExperimentNode> {
    let now = Utc::now().to_rfc3339();
    let id = generate_object_id(&format!("{}:{}:{}", domain_id, title, now));

    let experiment = ExperimentNode {
        schema_version: EXPERIMENT_SCHEMA.to_string(),
        id: id.clone(),
        title: title.to_string(),
        domain_id: domain_id.to_string(),
        task_id,
        parent_experiment_ids: Vec::new(),
        child_experiment_ids: Vec::new(),
        hypothesis_id,
        parameters,
        artifacts: Vec::new(),
        evidence_ids: Vec::new(),
        status: ExperimentStatus::Planned.as_str().to_string(),
        created_at: now.clone(),
        updated_at: now,
        created_by: created_by.to_string(),
    };

    write_research_object(
        workspace_root,
        ResearchObjectType::Experiment,
        &id,
        &experiment,
    )?;

    super::timeline::create_timeline_event(
        workspace_root,
        super::timeline::EventType::ExperimentCreated,
        &format!("Experiment created: {}", experiment.title),
        &format!("Domain {}; lineage node initialized.", experiment.domain_id),
        Some(experiment.domain_id.clone()),
        vec![super::object_graph::ResearchObjectId {
            object_type: "experiment".to_string(),
            id: experiment.id.clone(),
            created_at: experiment.created_at.clone(),
        }],
    )?;

    Ok(experiment)
}

pub fn update_experiment(
    workspace_root: &Path,
    id: &str,
    updates: ExperimentUpdate,
) -> Result<ExperimentNode> {
    let mut experiment: ExperimentNode =
        read_research_object(workspace_root, ResearchObjectType::Experiment, id)?;
    let previous_status = experiment.status.clone();
    let previous_artifact_count = experiment.artifacts.len();
    let mut changed = false;

    if let Some(status) = updates.status {
        if experiment.status != status {
            experiment.status = status;
            changed = true;
        }
    }
    if let Some(artifacts) = updates.artifacts {
        if experiment.artifacts != artifacts {
            experiment.artifacts = artifacts;
            changed = true;
        }
    }
    if let Some(evidence_ids) = updates.evidence_ids {
        if experiment.evidence_ids != evidence_ids {
            experiment.evidence_ids = evidence_ids;
            changed = true;
        }
    }
    if let Some(parent_ids) = updates.parent_experiment_ids {
        if experiment.parent_experiment_ids != parent_ids {
            experiment.parent_experiment_ids = parent_ids;
            changed = true;
        }
    }
    if let Some(child_ids) = updates.child_experiment_ids {
        if experiment.child_experiment_ids != child_ids {
            experiment.child_experiment_ids = child_ids;
            changed = true;
        }
    }
    if let Some(hypothesis_id) = updates.hypothesis_id {
        if experiment.hypothesis_id.as_deref() != Some(hypothesis_id.as_str()) {
            experiment.hypothesis_id = Some(hypothesis_id);
            changed = true;
        }
    }

    if !changed {
        return Ok(experiment);
    }

    experiment.updated_at = Utc::now().to_rfc3339();

    write_research_object(
        workspace_root,
        ResearchObjectType::Experiment,
        id,
        &experiment,
    )?;

    if experiment.status != previous_status || experiment.artifacts.len() != previous_artifact_count {
        super::timeline::create_timeline_event(
            workspace_root,
            super::timeline::EventType::ExperimentUpdated,
            &format!("Experiment updated: {}", experiment.title),
            &format!("Status: {}; artifacts: {}", experiment.status, experiment.artifacts.len()),
            Some(experiment.domain_id.clone()),
            vec![super::object_graph::ResearchObjectId {
                object_type: "experiment".to_string(),
                id: experiment.id.clone(),
                created_at: experiment.updated_at.clone(),
            }],
        )?;
    }

    Ok(experiment)
}

pub fn get_experiment(workspace_root: &Path, id: &str) -> Result<ExperimentNode> {
    read_research_object(workspace_root, ResearchObjectType::Experiment, id)
}

pub fn list_experiments(workspace_root: &Path) -> Result<Vec<ExperimentNode>> {
    let ids = list_research_objects(workspace_root, ResearchObjectType::Experiment)?;
    let mut experiments = Vec::new();

    for id in ids {
        if let Ok(experiment) = get_experiment(workspace_root, &id) {
            experiments.push(experiment);
        }
    }

    // Sort by created_at descending (newest first)
    experiments.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(experiments)
}

pub fn list_experiments_for_hypothesis(
    workspace_root: &Path,
    hypothesis_id: &str,
) -> Result<Vec<ExperimentNode>> {
    let all_experiments = list_experiments(workspace_root)?;
    Ok(all_experiments
        .into_iter()
        .filter(|e| {
            e.hypothesis_id
                .as_ref()
                .map(|id| id == hypothesis_id)
                .unwrap_or(false)
        })
        .collect())
}

#[derive(Debug, Clone, Default)]
pub struct ExperimentUpdate {
    pub status: Option<String>,
    pub artifacts: Option<Vec<String>>,
    pub evidence_ids: Option<Vec<String>>,
    pub parent_experiment_ids: Option<Vec<String>>,
    pub child_experiment_ids: Option<Vec<String>>,
    pub hypothesis_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn creates_and_retrieves_experiment() {
        let dir = tempdir().unwrap();
        let params = json!({"learning_rate": 0.001, "batch_size": 32});
        let exp = create_experiment(
            dir.path(),
            "Training Run 1",
            "ai-ml",
            Some("task-123".to_string()),
            Some("hyp-456".to_string()),
            params.clone(),
            "agent",
        )
        .unwrap();

        assert_eq!(exp.title, "Training Run 1");
        assert_eq!(exp.status, "planned");
        assert_eq!(exp.parameters, params);

        let retrieved = get_experiment(dir.path(), &exp.id).unwrap();
        assert_eq!(retrieved.title, exp.title);
    }

    #[test]
    fn updates_experiment_status_and_artifacts() {
        let dir = tempdir().unwrap();
        let exp = create_experiment(
            dir.path(),
            "Test",
            "ai-ml",
            None,
            None,
            json!({}),
            "agent",
        )
        .unwrap();

        let updates = ExperimentUpdate {
            status: Some("completed".to_string()),
            artifacts: Some(vec!["model.pth".to_string(), "metrics.json".to_string()]),
            ..Default::default()
        };

        let updated = update_experiment(dir.path(), &exp.id, updates).unwrap();
        assert_eq!(updated.status, "completed");
        assert_eq!(updated.artifacts.len(), 2);
    }

    #[test]
    fn filters_experiments_by_hypothesis() {
        let dir = tempdir().unwrap();
        create_experiment(
            dir.path(),
            "Exp 1",
            "ai-ml",
            None,
            Some("hyp-1".to_string()),
            json!({}),
            "agent",
        )
        .unwrap();
        create_experiment(
            dir.path(),
            "Exp 2",
            "ai-ml",
            None,
            Some("hyp-2".to_string()),
            json!({}),
            "agent",
        )
        .unwrap();

        let filtered = list_experiments_for_hypothesis(dir.path(), "hyp-1").unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].title, "Exp 1");
    }
}
