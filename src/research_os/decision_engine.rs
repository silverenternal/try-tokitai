//! Decision Engine Module
//!
//! Records major research decisions with options and rationale.

use super::object_graph::{
    generate_object_id, list_research_objects, read_research_object, write_research_object,
    ResearchObjectId, ResearchObjectType,
};
use super::timeline::{create_timeline_event, EventType};
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionOption {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub pros: Vec<String>,
    #[serde(default)]
    pub cons: Vec<String>,
    pub estimated_cost: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_gain: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub novelty_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub schema_version: String,
    pub id: String,
    pub title: String,
    pub context: String,
    #[serde(default)]
    pub options: Vec<DecisionOption>,
    pub chosen_option_id: String,
    pub decision_score: f64,
    pub rationale: String,
    pub timestamp: String,
    pub decided_by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_gain: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub novelty: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_time: Option<String>,
    #[serde(default)]
    pub paper_support: Vec<String>,
    #[serde(default)]
    pub experiment_support: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_risk: Option<f64>,
}

const DECISION_SCHEMA: &str = "atlas.research-os.decision.v1";

pub fn create_decision(
    workspace_root: &Path,
    title: &str,
    context: &str,
    options: Vec<DecisionOption>,
    chosen_option_id: &str,
    decision_score: f64,
    rationale: &str,
    decided_by: &str,
) -> Result<DecisionRecord> {
    let now = Utc::now().to_rfc3339();
    let id = generate_object_id(&format!("{}:{}:{}", title, context, now));

    let decision = DecisionRecord {
        schema_version: DECISION_SCHEMA.to_string(),
        id: id.clone(),
        title: title.to_string(),
        context: context.to_string(),
        options,
        chosen_option_id: chosen_option_id.to_string(),
        decision_score: decision_score.clamp(0.0, 1.0),
        rationale: rationale.to_string(),
        timestamp: now,
        decided_by: decided_by.to_string(),
        expected_gain: None,
        novelty: None,
        cost: None,
        risk: None,
        gpu_time: None,
        paper_support: Vec::new(),
        experiment_support: Vec::new(),
        failure_risk: None,
    };

    write_research_object(
        workspace_root,
        ResearchObjectType::Decision,
        &id,
        &decision,
    )?;

    create_timeline_event(
        workspace_root,
        EventType::DecisionMade,
        &format!("Decision made: {}", decision.title),
        &decision.rationale,
        None,
        vec![ResearchObjectId {
            object_type: "decision".to_string(),
            id: decision.id.clone(),
            created_at: decision.timestamp.clone(),
        }],
    )?;

    Ok(decision)
}

pub fn get_decision(workspace_root: &Path, id: &str) -> Result<DecisionRecord> {
    read_research_object(workspace_root, ResearchObjectType::Decision, id)
}

pub fn list_decisions(workspace_root: &Path) -> Result<Vec<DecisionRecord>> {
    let ids = list_research_objects(workspace_root, ResearchObjectType::Decision)?;
    let mut decisions = Vec::new();

    for id in ids {
        if let Ok(decision) = get_decision(workspace_root, &id) {
            decisions.push(decision);
        }
    }

    // Sort by timestamp descending (newest first)
    decisions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(decisions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn creates_and_retrieves_decision() {
        let dir = tempdir().unwrap();
        let options = vec![
            DecisionOption {
                id: "opt1".to_string(),
                label: "Use GPT-4".to_string(),
                pros: vec!["High quality".to_string()],
                cons: vec!["Expensive".to_string()],
                estimated_cost: "$100/day".to_string(),
                expected_gain: None,
                novelty_score: None,
                risk_score: None,
            },
            DecisionOption {
                id: "opt2".to_string(),
                label: "Use GPT-3.5".to_string(),
                pros: vec!["Cheaper".to_string()],
                cons: vec!["Lower quality".to_string()],
                estimated_cost: "$20/day".to_string(),
                expected_gain: None,
                novelty_score: None,
                risk_score: None,
            },
        ];

        let decision = create_decision(
            dir.path(),
            "Choose Language Model",
            "Need to select LLM for experiment",
            options.clone(),
            "opt1",
            0.85,
            "Quality is more important than cost for this experiment",
            "agent",
        )
        .unwrap();

        assert_eq!(decision.title, "Choose Language Model");
        assert_eq!(decision.chosen_option_id, "opt1");
        assert_eq!(decision.decision_score, 0.85);
        assert_eq!(decision.options.len(), 2);

        let retrieved = get_decision(dir.path(), &decision.id).unwrap();
        assert_eq!(retrieved.rationale, decision.rationale);
    }

    #[test]
    fn clamps_decision_score() {
        let dir = tempdir().unwrap();
        let decision = create_decision(
            dir.path(),
            "Test",
            "Context",
            vec![],
            "opt1",
            1.5, // Out of range
            "Rationale",
            "agent",
        )
        .unwrap();

        assert_eq!(decision.decision_score, 1.0); // Clamped
        let events = crate::research_os::timeline::list_timeline_events(dir.path()).unwrap();
        assert!(events.iter().any(|event| event.event_type == "decision_made"));
    }

    #[test]
    fn lists_decisions_newest_first() {
        let dir = tempdir().unwrap();
        let dec1 = create_decision(
            dir.path(),
            "Decision 1",
            "Context",
            vec![],
            "opt1",
            0.8,
            "Rationale",
            "agent",
        )
        .unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let dec2 = create_decision(
            dir.path(),
            "Decision 2",
            "Context",
            vec![],
            "opt1",
            0.9,
            "Rationale",
            "agent",
        )
        .unwrap();

        let list = list_decisions(dir.path()).unwrap();
        assert_eq!(list[0].id, dec2.id); // Newest first
        assert_eq!(list[1].id, dec1.id);
    }
}
