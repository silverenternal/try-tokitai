//! Hypothesis Lifecycle Module
//!
//! Manages research hypotheses with status tracking and evidence linking.

use super::object_graph::{
    generate_object_id, list_research_objects, read_research_object, write_research_object,
    ResearchObjectId, ResearchObjectType,
};
use super::timeline::{create_timeline_event, EventType};
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HypothesisStatus {
    #[serde(rename = "draft")]
    Draft,
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "validated")]
    Validated,
    #[serde(rename = "refuted")]
    Refuted,
    #[serde(rename = "abandoned")]
    Abandoned,
}

impl HypothesisStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Validated => "validated",
            Self::Refuted => "refuted",
            Self::Abandoned => "abandoned",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    pub schema_version: String,
    pub id: String,
    pub title: String,
    pub description: String,
    pub domain_id: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_hypothesis_id: Option<String>,
    #[serde(default)]
    pub child_hypothesis_ids: Vec<String>,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
    #[serde(default)]
    pub experiment_ids: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motivation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub novelty: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_result: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_confidence: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(default)]
    pub paper_ids: Vec<String>,
    #[serde(default)]
    pub dataset_ids: Vec<String>,
    #[serde(default)]
    pub model_ids: Vec<String>,
    #[serde(default)]
    pub task_ids: Vec<String>,
    #[serde(default)]
    pub visualization_ids: Vec<String>,
    #[serde(default)]
    pub publication_ids: Vec<String>,
}

const HYPOTHESIS_SCHEMA: &str = "atlas.research-os.hypothesis.v1";

pub fn create_hypothesis(
    workspace_root: &Path,
    title: &str,
    description: &str,
    domain_id: &str,
    created_by: &str,
) -> Result<Hypothesis> {
    let now = Utc::now().to_rfc3339();
    let id = generate_object_id(&format!("{}:{}:{}", title, description, now));

    let hypothesis = Hypothesis {
        schema_version: HYPOTHESIS_SCHEMA.to_string(),
        id: id.clone(),
        title: title.to_string(),
        description: description.to_string(),
        domain_id: domain_id.to_string(),
        status: HypothesisStatus::Draft.as_str().to_string(),
        parent_hypothesis_id: None,
        child_hypothesis_ids: Vec::new(),
        evidence_ids: Vec::new(),
        experiment_ids: Vec::new(),
        created_at: now.clone(),
        updated_at: now,
        created_by: created_by.to_string(),
        summary: None,
        motivation: None,
        problem: None,
        novelty: None,
        expected_result: None,
        current_confidence: None,
        owner: None,
        version: Some(1),
        tags: Vec::new(),
        priority: None,
        paper_ids: Vec::new(),
        dataset_ids: Vec::new(),
        model_ids: Vec::new(),
        task_ids: Vec::new(),
        visualization_ids: Vec::new(),
        publication_ids: Vec::new(),
    };

    write_research_object(
        workspace_root,
        ResearchObjectType::Hypothesis,
        &id,
        &hypothesis,
    )?;

    create_timeline_event(
        workspace_root,
        EventType::HypothesisCreated,
        &format!("Hypothesis created: {}", hypothesis.title),
        &hypothesis.description,
        Some(hypothesis.domain_id.clone()),
        vec![ResearchObjectId {
            object_type: "hypothesis".to_string(),
            id: hypothesis.id.clone(),
            created_at: hypothesis.created_at.clone(),
        }],
    )?;

    Ok(hypothesis)
}

pub fn update_hypothesis(
    workspace_root: &Path,
    id: &str,
    updates: HypothesisUpdate,
) -> Result<Hypothesis> {
    let mut hypothesis: Hypothesis =
        read_research_object(workspace_root, ResearchObjectType::Hypothesis, id)?;
    let previous_status = hypothesis.status.clone();

    if let Some(title) = updates.title {
        hypothesis.title = title;
    }
    if let Some(description) = updates.description {
        hypothesis.description = description;
    }
    if let Some(status) = updates.status {
        hypothesis.status = status;
    }
    if let Some(evidence_ids) = updates.evidence_ids {
        hypothesis.evidence_ids = evidence_ids;
    }
    if let Some(experiment_ids) = updates.experiment_ids {
        hypothesis.experiment_ids = experiment_ids;
    }
    if let Some(parent_hypothesis_id) = updates.parent_hypothesis_id {
        hypothesis.parent_hypothesis_id = Some(parent_hypothesis_id);
    }
    if let Some(child_hypothesis_ids) = updates.child_hypothesis_ids {
        hypothesis.child_hypothesis_ids = child_hypothesis_ids;
    }
    if let Some(summary) = updates.summary {
        hypothesis.summary = Some(summary);
    }
    if let Some(motivation) = updates.motivation {
        hypothesis.motivation = Some(motivation);
    }
    if let Some(problem) = updates.problem {
        hypothesis.problem = Some(problem);
    }
    if let Some(novelty) = updates.novelty {
        hypothesis.novelty = Some(novelty);
    }
    if let Some(expected_result) = updates.expected_result {
        hypothesis.expected_result = Some(expected_result);
    }
    if let Some(confidence) = updates.current_confidence {
        hypothesis.current_confidence = Some(confidence.clamp(0.0, 1.0));
    }
    if let Some(owner) = updates.owner {
        hypothesis.owner = Some(owner);
    }
    if updates.bump_version {
        hypothesis.version = Some(hypothesis.version.unwrap_or(1) + 1);
    }
    if let Some(tags) = updates.tags {
        hypothesis.tags = tags;
    }
    if let Some(priority) = updates.priority {
        hypothesis.priority = Some(priority);
    }
    if let Some(paper_ids) = updates.paper_ids {
        hypothesis.paper_ids = paper_ids;
    }
    if let Some(dataset_ids) = updates.dataset_ids {
        hypothesis.dataset_ids = dataset_ids;
    }
    if let Some(model_ids) = updates.model_ids {
        hypothesis.model_ids = model_ids;
    }
    if let Some(task_ids) = updates.task_ids {
        hypothesis.task_ids = task_ids;
    }
    if let Some(visualization_ids) = updates.visualization_ids {
        hypothesis.visualization_ids = visualization_ids;
    }
    if let Some(publication_ids) = updates.publication_ids {
        hypothesis.publication_ids = publication_ids;
    }

    hypothesis.updated_at = Utc::now().to_rfc3339();

    write_research_object(
        workspace_root,
        ResearchObjectType::Hypothesis,
        id,
        &hypothesis,
    )?;

    if hypothesis.status != previous_status {
        create_timeline_event(
            workspace_root,
            EventType::HypothesisTransition,
            &format!("Hypothesis moved to {}: {}", hypothesis.status, hypothesis.title),
            &format!("Lifecycle transition: {} -> {}", previous_status, hypothesis.status),
            Some(hypothesis.domain_id.clone()),
            vec![ResearchObjectId {
                object_type: "hypothesis".to_string(),
                id: hypothesis.id.clone(),
                created_at: hypothesis.updated_at.clone(),
            }],
        )?;
    }

    Ok(hypothesis)
}

pub fn get_hypothesis(workspace_root: &Path, id: &str) -> Result<Hypothesis> {
    read_research_object(workspace_root, ResearchObjectType::Hypothesis, id)
}

pub fn list_hypotheses(workspace_root: &Path) -> Result<Vec<Hypothesis>> {
    let ids = list_research_objects(workspace_root, ResearchObjectType::Hypothesis)?;
    let mut hypotheses = Vec::new();

    for id in ids {
        if let Ok(hypothesis) = get_hypothesis(workspace_root, &id) {
            hypotheses.push(hypothesis);
        }
    }

    // Sort by created_at descending (newest first)
    hypotheses.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(hypotheses)
}

#[derive(Debug, Clone, Default)]
pub struct HypothesisUpdate {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub evidence_ids: Option<Vec<String>>,
    pub experiment_ids: Option<Vec<String>>,
    pub parent_hypothesis_id: Option<String>,
    pub child_hypothesis_ids: Option<Vec<String>>,
    pub summary: Option<String>,
    pub motivation: Option<String>,
    pub problem: Option<String>,
    pub novelty: Option<String>,
    pub expected_result: Option<String>,
    pub current_confidence: Option<f64>,
    pub owner: Option<String>,
    pub bump_version: bool,
    pub tags: Option<Vec<String>>,
    pub priority: Option<String>,
    pub paper_ids: Option<Vec<String>>,
    pub dataset_ids: Option<Vec<String>>,
    pub model_ids: Option<Vec<String>>,
    pub task_ids: Option<Vec<String>>,
    pub visualization_ids: Option<Vec<String>>,
    pub publication_ids: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn creates_and_retrieves_hypothesis() {
        let dir = tempdir().unwrap();
        let hyp = create_hypothesis(
            dir.path(),
            "Test Hypothesis",
            "This is a test",
            "ai-ml",
            "agent",
        )
        .unwrap();

        assert_eq!(hyp.title, "Test Hypothesis");
        assert_eq!(hyp.status, "draft");
        assert_eq!(hyp.domain_id, "ai-ml");

        let retrieved = get_hypothesis(dir.path(), &hyp.id).unwrap();
        assert_eq!(retrieved.title, hyp.title);
    }

    #[test]
    fn updates_hypothesis_status() {
        let dir = tempdir().unwrap();
        let hyp = create_hypothesis(
            dir.path(),
            "Test",
            "Description",
            "ai-ml",
            "agent",
        )
        .unwrap();

        let updates = HypothesisUpdate {
            status: Some("active".to_string()),
            ..Default::default()
        };

        let updated = update_hypothesis(dir.path(), &hyp.id, updates).unwrap();
        assert_eq!(updated.status, "active");
        assert_ne!(updated.updated_at, updated.created_at);
        let events = crate::research_os::timeline::list_timeline_events(dir.path()).unwrap();
        assert!(events.iter().any(|event| event.event_type == "hypothesis_transition"));
    }

    #[test]
    fn lists_hypotheses_newest_first() {
        let dir = tempdir().unwrap();
        let hyp1 = create_hypothesis(dir.path(), "First", "Test", "ai-ml", "agent").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let hyp2 = create_hypothesis(dir.path(), "Second", "Test", "ai-ml", "agent").unwrap();

        let list = list_hypotheses(dir.path()).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, hyp2.id); // Newest first
        assert_eq!(list[1].id, hyp1.id);
    }
}
