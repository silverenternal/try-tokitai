//! Negative Results Intelligence Module
//!
//! Captures and analyzes research failures for learning.

use super::object_graph::{
    generate_object_id, list_research_objects, read_research_object, write_research_object,
    ResearchObjectType,
};
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegativeResult {
    pub schema_version: String,
    pub id: String,
    pub title: String,
    pub description: String,
    pub failure_mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hypothesis_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experiment_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    pub domain_id: String,
    #[serde(default)]
    pub artifacts: Vec<String>,
    pub learned: String,
    pub similarity_score: f64,
    pub created_at: String,
    pub created_by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configuration: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_info: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logs: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_info: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_info: Option<String>,
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub hyperparameters: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classification: Option<String>,
}

const NEGATIVE_RESULT_SCHEMA: &str = "atlas.research-os.negative-result.v1";

pub fn create_negative_result(
    workspace_root: &Path,
    title: &str,
    description: &str,
    failure_mode: &str,
    domain_id: &str,
    learned: &str,
    hypothesis_id: Option<String>,
    experiment_id: Option<String>,
    task_id: Option<String>,
    artifacts: Vec<String>,
    created_by: &str,
) -> Result<NegativeResult> {
    let now = Utc::now().to_rfc3339();
    let id = generate_object_id(&format!("{}:{}:{}", domain_id, title, now));

    let similarity_score = compute_similarity_score(workspace_root, description)?;

    let negative_result = NegativeResult {
        schema_version: NEGATIVE_RESULT_SCHEMA.to_string(),
        id: id.clone(),
        title: title.to_string(),
        description: description.to_string(),
        failure_mode: failure_mode.to_string(),
        hypothesis_id,
        experiment_id,
        task_id,
        domain_id: domain_id.to_string(),
        artifacts,
        learned: learned.to_string(),
        similarity_score,
        created_at: now,
        created_by: created_by.to_string(),
        configuration: None,
        environment: None,
        dataset: None,
        checkpoint: None,
        runtime_info: None,
        logs: None,
        gpu_info: None,
        memory_info: None,
        hyperparameters: serde_json::Value::Null,
        failure_score: None,
        classification: None,
    };

    write_research_object(
        workspace_root,
        ResearchObjectType::NegativeResult,
        &id,
        &negative_result,
    )?;

    Ok(negative_result)
}

pub fn get_negative_result(workspace_root: &Path, id: &str) -> Result<NegativeResult> {
    read_research_object(workspace_root, ResearchObjectType::NegativeResult, id)
}

pub fn list_negative_results(workspace_root: &Path) -> Result<Vec<NegativeResult>> {
    let ids = list_research_objects(workspace_root, ResearchObjectType::NegativeResult)?;
    let mut results = Vec::new();

    for id in ids {
        if let Ok(result) = get_negative_result(workspace_root, &id) {
            results.push(result);
        }
    }

    // Sort by created_at descending (newest first)
    results.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(results)
}

/// Compute similarity score to detect duplicate failures
fn compute_similarity_score(workspace_root: &Path, description: &str) -> Result<f64> {
    let existing = list_negative_results(workspace_root)?;

    if existing.is_empty() {
        return Ok(0.0);
    }

    let mut max_similarity = 0.0;
    let desc_lower = description.to_lowercase();

    for result in existing {
        let other_lower = result.description.to_lowercase();
        let similarity = simple_text_similarity(&desc_lower, &other_lower);
        if similarity > max_similarity {
            max_similarity = similarity;
        }
    }

    Ok(max_similarity)
}

/// Simple text similarity using common word overlap
fn simple_text_similarity(text1: &str, text2: &str) -> f64 {
    let words1: std::collections::HashSet<&str> = text1.split_whitespace().collect();
    let words2: std::collections::HashSet<&str> = text2.split_whitespace().collect();

    if words1.is_empty() || words2.is_empty() {
        return 0.0;
    }

    let intersection = words1.intersection(&words2).count();
    let union = words1.union(&words2).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn creates_and_retrieves_negative_result() {
        let dir = tempdir().unwrap();
        let result = create_negative_result(
            dir.path(),
            "Failed Training",
            "Model diverged after epoch 5",
            "divergence",
            "ai-ml",
            "Learning rate was too high",
            Some("hyp-123".to_string()),
            Some("exp-456".to_string()),
            Some("task-789".to_string()),
            vec!["loss_curve.png".to_string()],
            "agent",
        )
        .unwrap();

        assert_eq!(result.title, "Failed Training");
        assert_eq!(result.failure_mode, "divergence");
        assert_eq!(result.artifacts.len(), 1);

        let retrieved = get_negative_result(dir.path(), &result.id).unwrap();
        assert_eq!(retrieved.learned, result.learned);
    }

    #[test]
    fn detects_similar_failures() {
        let dir = tempdir().unwrap();

        create_negative_result(
            dir.path(),
            "First Failure",
            "model training diverged with high learning rate",
            "divergence",
            "ai-ml",
            "Reduce learning rate",
            None,
            None,
            None,
            vec![],
            "agent",
        )
        .unwrap();

        let result2 = create_negative_result(
            dir.path(),
            "Second Failure",
            "training diverged with high learning rate again",
            "divergence",
            "ai-ml",
            "Same issue",
            None,
            None,
            None,
            vec![],
            "agent",
        )
        .unwrap();

        // Should have high similarity score
        assert!(result2.similarity_score > 0.5);
    }

    #[test]
    fn text_similarity_basic() {
        let sim = simple_text_similarity("hello world", "hello there");
        assert!(sim > 0.0 && sim < 1.0);

        let exact = simple_text_similarity("same text", "same text");
        assert_eq!(exact, 1.0);
    }
}
