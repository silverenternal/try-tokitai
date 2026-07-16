//! Evidence Engine Module
//!
//! Manages research evidence with strength scoring and hypothesis linking.

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
pub enum EvidenceKind {
    #[serde(rename = "experimental")]
    Experimental,
    #[serde(rename = "literature")]
    Literature,
    #[serde(rename = "artifact")]
    Artifact,
    #[serde(rename = "benchmark")]
    Benchmark,
}

impl EvidenceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Experimental => "experimental",
            Self::Literature => "literature",
            Self::Artifact => "artifact",
            Self::Benchmark => "benchmark",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub schema_version: String,
    pub id: String,
    pub kind: String,
    pub summary: String,
    pub strength: f64, // 0.0-1.0
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hypothesis_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experiment_id: Option<String>,
    pub supports: bool, // true = supports, false = refutes
    pub created_at: String,
    pub created_by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_data: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment: Option<String>,
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub source_metadata: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<String>,
}

const EVIDENCE_SCHEMA: &str = "atlas.research-os.evidence.v1";

pub fn create_evidence(
    workspace_root: &Path,
    kind: EvidenceKind,
    summary: &str,
    strength: f64,
    supports: bool,
    source_path: Option<String>,
    source_command: Option<String>,
    hypothesis_id: Option<String>,
    experiment_id: Option<String>,
    created_by: &str,
) -> Result<Evidence> {
    let now = Utc::now().to_rfc3339();
    let id = generate_object_id(&format!("{}:{}:{}", kind.as_str(), summary, now));

    let strength_clamped = strength.clamp(0.0, 1.0);

    let evidence = Evidence {
        schema_version: EVIDENCE_SCHEMA.to_string(),
        id: id.clone(),
        kind: kind.as_str().to_string(),
        summary: summary.to_string(),
        strength: strength_clamped,
        source_path,
        source_command,
        hypothesis_id,
        experiment_id,
        supports,
        created_at: now,
        created_by: created_by.to_string(),
        raw_data: None,
        attachment: None,
        source_metadata: serde_json::Value::Null,
        verification_status: None,
        verified_by: None,
        verified_at: None,
    };

    write_research_object(
        workspace_root,
        ResearchObjectType::Evidence,
        &id,
        &evidence,
    )?;

    create_timeline_event(
        workspace_root,
        EventType::EvidenceAdded,
        if evidence.supports { "Supporting evidence added" } else { "Contradicting evidence added" },
        &evidence.summary,
        None,
        vec![ResearchObjectId {
            object_type: "evidence".to_string(),
            id: evidence.id.clone(),
            created_at: evidence.created_at.clone(),
        }],
    )?;

    Ok(evidence)
}

pub fn get_evidence(workspace_root: &Path, id: &str) -> Result<Evidence> {
    read_research_object(workspace_root, ResearchObjectType::Evidence, id)
}

pub fn list_evidence(workspace_root: &Path) -> Result<Vec<Evidence>> {
    let ids = list_research_objects(workspace_root, ResearchObjectType::Evidence)?;
    let mut evidence_list = Vec::new();

    for id in ids {
        if let Ok(evidence) = get_evidence(workspace_root, &id) {
            evidence_list.push(evidence);
        }
    }

    // Sort by created_at descending (newest first)
    evidence_list.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(evidence_list)
}

pub fn list_evidence_for_hypothesis(
    workspace_root: &Path,
    hypothesis_id: &str,
) -> Result<Vec<Evidence>> {
    let all_evidence = list_evidence(workspace_root)?;
    Ok(all_evidence
        .into_iter()
        .filter(|e| {
            e.hypothesis_id
                .as_ref()
                .map(|id| id == hypothesis_id)
                .unwrap_or(false)
        })
        .collect())
}

pub fn list_evidence_for_experiment(
    workspace_root: &Path,
    experiment_id: &str,
) -> Result<Vec<Evidence>> {
    let all_evidence = list_evidence(workspace_root)?;
    Ok(all_evidence
        .into_iter()
        .filter(|e| {
            e.experiment_id
                .as_ref()
                .map(|id| id == experiment_id)
                .unwrap_or(false)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn creates_and_retrieves_evidence() {
        let dir = tempdir().unwrap();
        let evidence = create_evidence(
            dir.path(),
            EvidenceKind::Experimental,
            "Accuracy improved by 15%",
            0.85,
            true,
            Some("/path/to/results.json".to_string()),
            Some("python train.py".to_string()),
            Some("hyp-123".to_string()),
            Some("exp-456".to_string()),
            "agent",
        )
        .unwrap();

        assert_eq!(evidence.kind, "experimental");
        assert_eq!(evidence.strength, 0.85);
        assert!(evidence.supports);

        let retrieved = get_evidence(dir.path(), &evidence.id).unwrap();
        assert_eq!(retrieved.summary, evidence.summary);
    }

    #[test]
    fn clamps_strength_to_valid_range() {
        let dir = tempdir().unwrap();
        let evidence = create_evidence(
            dir.path(),
            EvidenceKind::Benchmark,
            "Test",
            1.5, // Out of range
            true,
            None,
            None,
            None,
            None,
            "agent",
        )
        .unwrap();

        assert_eq!(evidence.strength, 1.0); // Clamped to max
    }

    #[test]
    fn filters_evidence_by_hypothesis() {
        let dir = tempdir().unwrap();
        create_evidence(
            dir.path(),
            EvidenceKind::Artifact,
            "Evidence 1",
            0.7,
            true,
            None,
            None,
            Some("hyp-1".to_string()),
            None,
            "agent",
        )
        .unwrap();
        create_evidence(
            dir.path(),
            EvidenceKind::Artifact,
            "Evidence 2",
            0.8,
            true,
            None,
            None,
            Some("hyp-2".to_string()),
            None,
            "agent",
        )
        .unwrap();

        let filtered = list_evidence_for_hypothesis(dir.path(), "hyp-1").unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].summary, "Evidence 1");
        let events = crate::research_os::timeline::list_timeline_events(dir.path()).unwrap();
        assert_eq!(
            events.iter().filter(|event| event.event_type == "evidence_added").count(),
            2
        );
    }
}
