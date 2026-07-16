//! Scientific Timeline Module
//!
//! Chronological tracking of research events.

use super::object_graph::{
    generate_object_id, list_research_objects, read_research_object, write_research_object,
    ResearchObjectId, ResearchObjectType,
};
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventType {
    #[serde(rename = "hypothesis_created")]
    HypothesisCreated,
    #[serde(rename = "hypothesis_transition")]
    HypothesisTransition,
    #[serde(rename = "experiment_run")]
    ExperimentRun,
    #[serde(rename = "evidence_added")]
    EvidenceAdded,
    #[serde(rename = "decision_made")]
    DecisionMade,
    #[serde(rename = "failure_recorded")]
    FailureRecorded,
    #[serde(rename = "publication_drafted")]
    PublicationDrafted,
    #[serde(rename = "publication_updated")]
    PublicationUpdated,
    #[serde(rename = "memory_captured")]
    MemoryCaptured,
    #[serde(rename = "agent_activity")]
    AgentActivity,
    #[serde(rename = "experiment_created")]
    ExperimentCreated,
    #[serde(rename = "experiment_updated")]
    ExperimentUpdated,
    #[serde(rename = "objects_linked")]
    ObjectsLinked,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HypothesisCreated => "hypothesis_created",
            Self::HypothesisTransition => "hypothesis_transition",
            Self::ExperimentRun => "experiment_run",
            Self::EvidenceAdded => "evidence_added",
            Self::DecisionMade => "decision_made",
            Self::FailureRecorded => "failure_recorded",
            Self::PublicationDrafted => "publication_drafted",
            Self::PublicationUpdated => "publication_updated",
            Self::MemoryCaptured => "memory_captured",
            Self::AgentActivity => "agent_activity",
            Self::ExperimentCreated => "experiment_created",
            Self::ExperimentUpdated => "experiment_updated",
            Self::ObjectsLinked => "objects_linked",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub schema_version: String,
    pub id: String,
    pub timestamp: String,
    pub event_type: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub related_objects: Vec<ResearchObjectId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain_id: Option<String>,
}

const TIMELINE_SCHEMA: &str = "atlas.research-os.timeline.v1";

pub fn create_timeline_event(
    workspace_root: &Path,
    event_type: EventType,
    title: &str,
    description: &str,
    domain_id: Option<String>,
    related_objects: Vec<ResearchObjectId>,
) -> Result<TimelineEvent> {
    let now = Utc::now().to_rfc3339();
    let id = generate_object_id(&format!("{}:{}:{}", event_type.as_str(), title, now));

    let event = TimelineEvent {
        schema_version: TIMELINE_SCHEMA.to_string(),
        id: id.clone(),
        timestamp: now,
        event_type: event_type.as_str().to_string(),
        title: title.to_string(),
        description: description.to_string(),
        related_objects,
        domain_id,
    };

    write_research_object(
        workspace_root,
        ResearchObjectType::Timeline,
        &id,
        &event,
    )?;

    Ok(event)
}

pub fn get_timeline_event(workspace_root: &Path, id: &str) -> Result<TimelineEvent> {
    read_research_object(workspace_root, ResearchObjectType::Timeline, id)
}

pub fn list_timeline_events(workspace_root: &Path) -> Result<Vec<TimelineEvent>> {
    let ids = list_research_objects(workspace_root, ResearchObjectType::Timeline)?;
    let mut events = Vec::new();

    for id in ids {
        if let Ok(event) = get_timeline_event(workspace_root, &id) {
            events.push(event);
        }
    }

    // Sort by timestamp ascending (oldest first for chronological order)
    events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    Ok(events)
}

pub fn list_timeline_events_by_type(
    workspace_root: &Path,
    event_type: EventType,
) -> Result<Vec<TimelineEvent>> {
    let all_events = list_timeline_events(workspace_root)?;
    Ok(all_events
        .into_iter()
        .filter(|e| e.event_type == event_type.as_str())
        .collect())
}

pub fn list_timeline_events_for_domain(
    workspace_root: &Path,
    domain_id: &str,
) -> Result<Vec<TimelineEvent>> {
    let all_events = list_timeline_events(workspace_root)?;
    Ok(all_events
        .into_iter()
        .filter(|e| {
            e.domain_id
                .as_ref()
                .map(|id| id == domain_id)
                .unwrap_or(false)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn creates_and_retrieves_timeline_event() {
        let dir = tempdir().unwrap();
        let event = create_timeline_event(
            dir.path(),
            EventType::ExperimentRun,
            "Training Run #1",
            "Initial baseline experiment with default hyperparameters",
            Some("ai-ml".to_string()),
            vec![],
        )
        .unwrap();

        assert_eq!(event.event_type, "experiment_run");
        assert_eq!(event.title, "Training Run #1");
        assert_eq!(event.domain_id, Some("ai-ml".to_string()));

        let retrieved = get_timeline_event(dir.path(), &event.id).unwrap();
        assert_eq!(retrieved.description, event.description);
    }

    #[test]
    fn lists_timeline_events_chronologically() {
        let dir = tempdir().unwrap();
        let event1 = create_timeline_event(
            dir.path(),
            EventType::HypothesisCreated,
            "First Event",
            "Description",
            None,
            vec![],
        )
        .unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let event2 = create_timeline_event(
            dir.path(),
            EventType::ExperimentRun,
            "Second Event",
            "Description",
            None,
            vec![],
        )
        .unwrap();

        let list = list_timeline_events(dir.path()).unwrap();
        assert_eq!(list[0].id, event1.id); // Oldest first
        assert_eq!(list[1].id, event2.id);
    }

    #[test]
    fn filters_timeline_events_by_type() {
        let dir = tempdir().unwrap();
        create_timeline_event(
            dir.path(),
            EventType::HypothesisCreated,
            "Hypothesis",
            "Description",
            None,
            vec![],
        )
        .unwrap();
        create_timeline_event(
            dir.path(),
            EventType::ExperimentRun,
            "Experiment",
            "Description",
            None,
            vec![],
        )
        .unwrap();

        let filtered =
            list_timeline_events_by_type(dir.path(), EventType::HypothesisCreated).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].title, "Hypothesis");
    }

    #[test]
    fn filters_timeline_events_by_domain() {
        let dir = tempdir().unwrap();
        create_timeline_event(
            dir.path(),
            EventType::ExperimentRun,
            "ML Experiment",
            "Description",
            Some("ai-ml".to_string()),
            vec![],
        )
        .unwrap();
        create_timeline_event(
            dir.path(),
            EventType::ExperimentRun,
            "CV Experiment",
            "Description",
            Some("computer-vision".to_string()),
            vec![],
        )
        .unwrap();

        let filtered = list_timeline_events_for_domain(dir.path(), "ai-ml").unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].title, "ML Experiment");
    }
}
