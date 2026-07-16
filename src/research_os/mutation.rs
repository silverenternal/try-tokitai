//! Research OS Mutation Module
//!
//! Provides safe, validated write operations for agents to modify Research OS objects.

use super::decision_engine::{create_decision, DecisionOption};
use super::evidence::{create_evidence, EvidenceKind};
use super::experiment_lineage::{create_experiment, update_experiment, ExperimentUpdate};
use super::hypothesis::{create_hypothesis, update_hypothesis, HypothesisUpdate};
use super::knowledge_graph::{create_kg_edge, EdgeType};
use super::memory::create_memory_entry;
use super::negative_results::create_negative_result;
use super::object_graph::{object_exists, ResearchObjectId, ResearchObjectType};
use super::publication::{create_publication, update_publication, PublicationUpdate};
use super::timeline::{create_timeline_event, EventType};
use super::{get_hypothesis, get_publication, list_evidence};
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::path::Path;

/// Execute a validated mutation operation on Research OS objects.
pub fn execute_mutation(workspace_root: &Path, operation: &str, params: &Value) -> Result<Value> {
    match operation {
        "create_hypothesis" => create_hypothesis_op(workspace_root, params),
        "update_hypothesis" => update_hypothesis_op(workspace_root, params),
        "create_evidence" => create_evidence_op(workspace_root, params),
        "create_experiment" => create_experiment_op(workspace_root, params),
        "update_experiment" => update_experiment_op(workspace_root, params),
        "create_negative_result" => create_negative_result_op(workspace_root, params),
        "create_decision" => create_decision_op(workspace_root, params),
        "create_memory" => create_memory_op(workspace_root, params),
        "create_publication" => create_publication_op(workspace_root, params),
        "update_publication" => update_publication_op(workspace_root, params),
        "link_objects" => link_objects_op(workspace_root, params),
        _ => Err(anyhow!("Unknown mutation operation: {}", operation)),
    }
}

fn create_hypothesis_op(workspace_root: &Path, params: &Value) -> Result<Value> {
    let title = params
        .get("title")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("create_hypothesis requires title"))?;
    let description = params
        .get("description")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("create_hypothesis requires description"))?;
    let domain_id = params
        .get("domain_id")
        .and_then(Value::as_str)
        .unwrap_or("general");
    let created_by = params
        .get("created_by")
        .and_then(Value::as_str)
        .unwrap_or("atlas");

    let hypothesis = create_hypothesis(workspace_root, title, description, domain_id, created_by)?;
    Ok(serde_json::to_value(&hypothesis)?)
}

fn update_hypothesis_op(workspace_root: &Path, params: &Value) -> Result<Value> {
    let id = params
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("update_hypothesis requires id"))?;

    if !object_exists(workspace_root, ResearchObjectType::Hypothesis, id) {
        return Err(anyhow!("Hypothesis not found: {}", id));
    }

    let status = params.get("status").and_then(Value::as_str).map(String::from);
    let incoming_evidence_ids: Option<Vec<String>> = params.get("evidence_ids").and_then(|v| {
        v.as_array()
            .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
    });

    // Validate status transitions
    if let Some(ref status_str) = status {
        if status_str == "validated" || status_str == "refuted" {
            let hyp = get_hypothesis(workspace_root, id)?;
            let has_evidence = incoming_evidence_ids
                .as_ref()
                .map(|ids| !ids.is_empty())
                .unwrap_or(false)
                || !hyp.evidence_ids.is_empty()
                || list_evidence(workspace_root)?
                    .iter()
                    .any(|e| e.hypothesis_id.as_deref() == Some(id));
            if !has_evidence {
                return Err(anyhow!(
                    "Cannot set hypothesis to '{}' without linked evidence. Use create_evidence first.",
                    status_str
                ));
            }
        }
        if !matches!(status_str.as_str(), "draft" | "active" | "validated" | "refuted" | "abandoned") {
            return Err(anyhow!("Invalid hypothesis status: {}. Must be draft, active, validated, refuted, or abandoned.", status_str));
        }
    }

    let updates = HypothesisUpdate {
        title: params.get("title").and_then(Value::as_str).map(String::from),
        description: params.get("description").and_then(Value::as_str).map(String::from),
        status,
        evidence_ids: incoming_evidence_ids,
        experiment_ids: params.get("experiment_ids").and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
        }),
        parent_hypothesis_id: params.get("parent_hypothesis_id").and_then(Value::as_str).map(String::from),
        child_hypothesis_ids: params.get("child_hypothesis_ids").and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
        }),
        summary: params.get("summary").and_then(Value::as_str).map(String::from),
        motivation: params.get("motivation").and_then(Value::as_str).map(String::from),
        problem: params.get("problem").and_then(Value::as_str).map(String::from),
        novelty: params.get("novelty").and_then(Value::as_str).map(String::from),
        expected_result: params.get("expected_result").and_then(Value::as_str).map(String::from),
        current_confidence: params.get("current_confidence").and_then(Value::as_f64),
        owner: params.get("owner").and_then(Value::as_str).map(String::from),
        bump_version: params.get("bump_version").and_then(Value::as_bool).unwrap_or(false),
        tags: params.get("tags").and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
        }),
        priority: params.get("priority").and_then(Value::as_str).map(String::from),
        paper_ids: params.get("paper_ids").and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
        }),
        dataset_ids: params.get("dataset_ids").and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
        }),
        model_ids: params.get("model_ids").and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
        }),
        task_ids: params.get("task_ids").and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
        }),
        visualization_ids: params.get("visualization_ids").and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
        }),
        publication_ids: params.get("publication_ids").and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
        }),
    };

    let hypothesis = update_hypothesis(workspace_root, id, updates)?;
    Ok(serde_json::to_value(&hypothesis)?)
}

fn create_evidence_op(workspace_root: &Path, params: &Value) -> Result<Value> {
    let kind = params
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("create_evidence requires kind (experimental, literature, artifact, benchmark)"))?;
    let kind_enum = match kind {
        "experimental" => EvidenceKind::Experimental,
        "literature" => EvidenceKind::Literature,
        "artifact" => EvidenceKind::Artifact,
        "benchmark" => EvidenceKind::Benchmark,
        _ => return Err(anyhow!("Invalid evidence kind: {}", kind)),
    };

    let summary = params
        .get("summary")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("create_evidence requires summary"))?;
    let strength = params
        .get("strength")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("create_evidence requires strength (0.0-1.0)"))?;
    let supports = params
        .get("supports")
        .and_then(Value::as_bool)
        .ok_or_else(|| anyhow!("create_evidence requires supports (true/false)"))?;

    if !(0.0..=1.0).contains(&strength) {
        return Err(anyhow!("Evidence strength must be between 0.0 and 1.0, got {}", strength));
    }

    let source_path = params.get("source_path").and_then(Value::as_str).map(String::from);
    let source_command = params.get("source_command").and_then(Value::as_str).map(String::from);
    let hypothesis_id = params.get("hypothesis_id").and_then(Value::as_str).map(String::from);
    let experiment_id = params.get("experiment_id").and_then(Value::as_str).map(String::from);
    let created_by = params
        .get("created_by")
        .and_then(Value::as_str)
        .unwrap_or("atlas");

    // Validate referenced objects exist
    if let Some(ref hyp_id) = hypothesis_id {
        if !object_exists(workspace_root, ResearchObjectType::Hypothesis, hyp_id) {
            return Err(anyhow!("Hypothesis not found: {}", hyp_id));
        }
    }
    if let Some(ref exp_id) = experiment_id {
        if !object_exists(workspace_root, ResearchObjectType::Experiment, exp_id) {
            return Err(anyhow!("Experiment not found: {}", exp_id));
        }
    }

    let evidence = create_evidence(
        workspace_root,
        kind_enum,
        summary,
        strength,
        supports,
        source_path,
        source_command,
        hypothesis_id,
        experiment_id,
        created_by,
    )?;
    Ok(serde_json::to_value(&evidence)?)
}

fn create_experiment_op(workspace_root: &Path, params: &Value) -> Result<Value> {
    let title = params
        .get("title")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("create_experiment requires title"))?;
    let domain_id = params
        .get("domain_id")
        .and_then(Value::as_str)
        .unwrap_or("general");
    let task_id = params.get("task_id").and_then(Value::as_str).map(String::from);
    let hypothesis_id = params.get("hypothesis_id").and_then(Value::as_str).map(String::from);
    let parameters = params.get("parameters").cloned().unwrap_or(Value::Object(Default::default()));
    let created_by = params
        .get("created_by")
        .and_then(Value::as_str)
        .unwrap_or("atlas");

    if let Some(ref hyp_id) = hypothesis_id {
        if !object_exists(workspace_root, ResearchObjectType::Hypothesis, hyp_id) {
            return Err(anyhow!("Hypothesis not found: {}", hyp_id));
        }
    }

    let experiment = create_experiment(
        workspace_root,
        title,
        domain_id,
        task_id,
        hypothesis_id,
        parameters,
        created_by,
    )?;
    Ok(serde_json::to_value(&experiment)?)
}

fn update_experiment_op(workspace_root: &Path, params: &Value) -> Result<Value> {
    let id = params
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("update_experiment requires id"))?;

    if !object_exists(workspace_root, ResearchObjectType::Experiment, id) {
        return Err(anyhow!("Experiment not found: {}", id));
    }

    let status = params.get("status").and_then(Value::as_str).map(String::from);
    if let Some(ref status_str) = status {
        if !matches!(status_str.as_str(), "planned" | "running" | "completed" | "failed") {
            return Err(anyhow!("Invalid experiment status: {}. Must be planned, running, completed, or failed.", status_str));
        }
    }

    let updates = ExperimentUpdate {
        status,
        artifacts: params.get("artifacts").and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
        }),
        evidence_ids: params.get("evidence_ids").and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
        }),
        parent_experiment_ids: params.get("parent_experiment_ids").and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
        }),
        child_experiment_ids: params.get("child_experiment_ids").and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
        }),
        hypothesis_id: params.get("hypothesis_id").and_then(Value::as_str).map(String::from),
    };

    let experiment = update_experiment(workspace_root, id, updates)?;
    Ok(serde_json::to_value(&experiment)?)
}

fn create_negative_result_op(workspace_root: &Path, params: &Value) -> Result<Value> {
    let title = params
        .get("title")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("create_negative_result requires title"))?;
    let description = params
        .get("description")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("create_negative_result requires description"))?;
    let failure_mode = params
        .get("failure_mode")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let domain_id = params
        .get("domain_id")
        .and_then(Value::as_str)
        .unwrap_or("general");
    let learned = params
        .get("learned")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("create_negative_result requires learned"))?;
    let hypothesis_id = params.get("hypothesis_id").and_then(Value::as_str).map(String::from);
    let experiment_id = params.get("experiment_id").and_then(Value::as_str).map(String::from);
    let task_id = params.get("task_id").and_then(Value::as_str).map(String::from);
    let artifacts = params
        .get("artifacts")
        .and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
        })
        .unwrap_or_default();
    let created_by = params
        .get("created_by")
        .and_then(Value::as_str)
        .unwrap_or("atlas");

    let negative_result = create_negative_result(
        workspace_root,
        title,
        description,
        failure_mode,
        domain_id,
        learned,
        hypothesis_id,
        experiment_id,
        task_id,
        artifacts,
        created_by,
    )?;
    Ok(serde_json::to_value(&negative_result)?)
}

fn create_decision_op(workspace_root: &Path, params: &Value) -> Result<Value> {
    let title = params
        .get("title")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("create_decision requires title"))?;
    let context = params
        .get("context")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("create_decision requires context"))?;
    let options_array = params
        .get("options")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("create_decision requires options array"))?;
    let chosen_option_id = params
        .get("chosen_option_id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("create_decision requires chosen_option_id"))?;
    let decision_score = params
        .get("decision_score")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("create_decision requires decision_score (0.0-1.0)"))?;
    let rationale = params
        .get("rationale")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("create_decision requires rationale"))?;
    let decided_by = params
        .get("decided_by")
        .and_then(Value::as_str)
        .unwrap_or("atlas");

    if !(0.0..=1.0).contains(&decision_score) {
        return Err(anyhow!("decision_score must be between 0.0 and 1.0, got {}", decision_score));
    }

    let mut options = Vec::new();
    for opt_val in options_array {
        let id = opt_val
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("Each option requires id"))?;
        let label = opt_val
            .get("label")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("Each option requires label"))?;
        let pros = opt_val
            .get("pros")
            .and_then(|v| {
                v.as_array()
                    .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
            })
            .unwrap_or_default();
        let cons = opt_val
            .get("cons")
            .and_then(|v| {
                v.as_array()
                    .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
            })
            .unwrap_or_default();
        let estimated_cost = opt_val
            .get("estimated_cost")
            .and_then(Value::as_str)
            .unwrap_or("unknown");

        options.push(DecisionOption {
            id: id.to_string(),
            label: label.to_string(),
            pros,
            cons,
            estimated_cost: estimated_cost.to_string(),
            expected_gain: opt_val.get("expected_gain").and_then(Value::as_f64),
            novelty_score: opt_val.get("novelty_score").and_then(Value::as_f64),
            risk_score: opt_val.get("risk_score").and_then(Value::as_f64),
        });
    }

    if !options.iter().any(|opt| opt.id == chosen_option_id) {
        return Err(anyhow!(
            "chosen_option_id '{}' not found in options",
            chosen_option_id
        ));
    }

    let decision = create_decision(
        workspace_root,
        title,
        context,
        options,
        chosen_option_id,
        decision_score,
        rationale,
        decided_by,
    )?;
    Ok(serde_json::to_value(&decision)?)
}

fn create_memory_op(workspace_root: &Path, params: &Value) -> Result<Value> {
    let content = params
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("create_memory requires content"))?;
    let importance = params
        .get("importance")
        .and_then(Value::as_f64)
        .unwrap_or(0.5);

    if !(0.0..=1.0).contains(&importance) {
        return Err(anyhow!("Memory importance must be between 0.0 and 1.0, got {}", importance));
    }

    let related_objects = params
        .get("related_objects")
        .and_then(|v| {
            v.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|obj| {
                        let object_type = obj.get("object_type").and_then(Value::as_str)?;
                        let id = obj.get("id").and_then(Value::as_str)?;
                        let created_at = obj.get("created_at").and_then(Value::as_str)?;
                        Some(ResearchObjectId {
                            object_type: object_type.to_string(),
                            id: id.to_string(),
                            created_at: created_at.to_string(),
                        })
                    })
                    .collect()
            })
        })
        .unwrap_or_default();

    let memory = create_memory_entry(workspace_root, content, importance, related_objects, None)?;
    Ok(serde_json::to_value(&memory)?)
}

fn create_publication_op(workspace_root: &Path, params: &Value) -> Result<Value> {
    let title = params
        .get("title")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("create_publication requires title"))?;
    let created_by = params
        .get("created_by")
        .and_then(Value::as_str)
        .unwrap_or("atlas");

    let publication = create_publication(workspace_root, title, created_by)?;
    Ok(serde_json::to_value(&publication)?)
}

fn update_publication_op(workspace_root: &Path, params: &Value) -> Result<Value> {
    let id = params
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("update_publication requires id"))?;

    if !object_exists(workspace_root, ResearchObjectType::Publication, id) {
        return Err(anyhow!("Publication not found: {}", id));
    }

    let status = params.get("status").and_then(Value::as_str).map(String::from);
    let incoming_evidence_ids: Option<Vec<String>> = params.get("evidence_ids").and_then(|v| {
        v.as_array()
            .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
    });

    if let Some(ref status_str) = status {
        if status_str == "ready" || status_str == "published" {
            let pub_draft = get_publication(workspace_root, id)?;
            let has_evidence = incoming_evidence_ids
                .as_ref()
                .map(|ids| !ids.is_empty())
                .unwrap_or(!pub_draft.evidence_ids.is_empty());
            if !has_evidence {
                return Err(anyhow!(
                    "Cannot set publication to '{}' without linked evidence. Link evidence first.",
                    status_str
                ));
            }
        }
        if !matches!(status_str.as_str(), "draft" | "review" | "ready" | "published") {
            return Err(anyhow!("Invalid publication status: {}. Must be draft, review, ready, or published.", status_str));
        }
    }

    let updates = PublicationUpdate {
        title: params.get("title").and_then(Value::as_str).map(String::from),
        status,
        sections: None,
        hypothesis_ids: params.get("hypothesis_ids").and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
        }),
        evidence_ids: incoming_evidence_ids,
        experiment_ids: params.get("experiment_ids").and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
        }),
        artifact_paths: params.get("artifact_paths").and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
        }),
    };

    let publication = update_publication(workspace_root, id, updates)?;
    Ok(serde_json::to_value(&publication)?)
}

fn link_objects_op(workspace_root: &Path, params: &Value) -> Result<Value> {
    let from_type = params
        .get("from_type")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("link_objects requires from_type"))?;
    let from_id = params
        .get("from_id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("link_objects requires from_id"))?;
    let to_type = params
        .get("to_type")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("link_objects requires to_type"))?;
    let to_id = params
        .get("to_id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("link_objects requires to_id"))?;
    let relation = params
        .get("relation")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("link_objects requires relation"))?;

    let edge_type = match relation {
        "cites" => EdgeType::Cites,
        "uses" => EdgeType::Uses,
        "extends" => EdgeType::Extends,
        "contradicts" => EdgeType::Contradicts,
        _ => return Err(anyhow!("Invalid relation: {}. Must be cites, uses, extends, or contradicts.", relation)),
    };

    let edge = create_kg_edge(workspace_root, from_id, to_id, edge_type)?;

    create_timeline_event(
        workspace_root,
        EventType::ObjectsLinked,
        &format!("Linked {} to {}", from_type, to_type),
        &format!("Relation: {}", relation),
        None,
        vec![
            ResearchObjectId {
                object_type: from_type.to_string(),
                id: from_id.to_string(),
                created_at: edge.created_at.clone(),
            },
            ResearchObjectId {
                object_type: to_type.to_string(),
                id: to_id.to_string(),
                created_at: edge.created_at.clone(),
            },
        ],
    )?;

    Ok(serde_json::to_value(&edge)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn creates_hypothesis_via_mutation() {
        let dir = tempdir().unwrap();
        let result = execute_mutation(
            dir.path(),
            "create_hypothesis",
            &json!({
                "title": "Scaling improves accuracy",
                "description": "Larger models generalize better on held-out data",
                "domain_id": "ai-ml",
            }),
        )
        .unwrap();
        assert_eq!(result["title"], "Scaling improves accuracy");
        assert_eq!(result["status"], "draft");
    }

    #[test]
    fn rejects_unknown_operation() {
        let dir = tempdir().unwrap();
        let err = execute_mutation(dir.path(), "delete_everything", &json!({})).unwrap_err();
        assert!(err.to_string().contains("Unknown mutation operation"));
    }

    #[test]
    fn rejects_hypothesis_validated_without_evidence() {
        let dir = tempdir().unwrap();
        let hyp = execute_mutation(
            dir.path(),
            "create_hypothesis",
            &json!({"title": "H1", "description": "D1", "domain_id": "ai-ml"}),
        )
        .unwrap();
        let id = hyp["id"].as_str().unwrap();

        let err = execute_mutation(
            dir.path(),
            "update_hypothesis",
            &json!({"id": id, "status": "validated"}),
        )
        .unwrap_err();
        assert!(err.to_string().contains("without linked evidence"));
    }

    #[test]
    fn allows_hypothesis_validated_with_evidence() {
        let dir = tempdir().unwrap();
        let hyp = execute_mutation(
            dir.path(),
            "create_hypothesis",
            &json!({"title": "H1", "description": "D1", "domain_id": "ai-ml"}),
        )
        .unwrap();
        let hyp_id = hyp["id"].as_str().unwrap().to_string();

        execute_mutation(
            dir.path(),
            "create_evidence",
            &json!({
                "kind": "experimental",
                "summary": "Accuracy improved 4pts",
                "strength": 0.8,
                "supports": true,
                "hypothesis_id": hyp_id,
            }),
        )
        .unwrap();

        let updated = execute_mutation(
            dir.path(),
            "update_hypothesis",
            &json!({"id": hyp_id, "status": "validated"}),
        )
        .unwrap();
        assert_eq!(updated["status"], "validated");
    }

    #[test]
    fn rejects_evidence_strength_out_of_range() {
        let dir = tempdir().unwrap();
        let err = execute_mutation(
            dir.path(),
            "create_evidence",
            &json!({
                "kind": "experimental",
                "summary": "bad strength",
                "strength": 1.5,
                "supports": true,
            }),
        )
        .unwrap_err();
        assert!(err.to_string().contains("strength must be between"));
    }

    #[test]
    fn rejects_evidence_for_missing_hypothesis() {
        let dir = tempdir().unwrap();
        let err = execute_mutation(
            dir.path(),
            "create_evidence",
            &json!({
                "kind": "literature",
                "summary": "cites related work",
                "strength": 0.5,
                "supports": true,
                "hypothesis_id": "nonexistent-id",
            }),
        )
        .unwrap_err();
        assert!(err.to_string().contains("Hypothesis not found"));
    }

    #[test]
    fn rejects_publication_ready_without_evidence() {
        let dir = tempdir().unwrap();
        let publication = execute_mutation(
            dir.path(),
            "create_publication",
            &json!({"title": "Paper Draft"}),
        )
        .unwrap();
        let id = publication["id"].as_str().unwrap();

        let err = execute_mutation(
            dir.path(),
            "update_publication",
            &json!({"id": id, "status": "ready"}),
        )
        .unwrap_err();
        assert!(err.to_string().contains("without linked evidence"));
    }

    #[test]
    fn allows_publication_ready_with_evidence() {
        let dir = tempdir().unwrap();
        let publication = execute_mutation(
            dir.path(),
            "create_publication",
            &json!({"title": "Paper Draft"}),
        )
        .unwrap();
        let id = publication["id"].as_str().unwrap().to_string();

        let evidence = execute_mutation(
            dir.path(),
            "create_evidence",
            &json!({
                "kind": "experimental",
                "summary": "core result",
                "strength": 0.9,
                "supports": true,
            }),
        )
        .unwrap();
        let evidence_id = evidence["id"].as_str().unwrap().to_string();

        let updated = execute_mutation(
            dir.path(),
            "update_publication",
            &json!({"id": id, "status": "ready", "evidence_ids": [evidence_id]}),
        )
        .unwrap();
        assert_eq!(updated["status"], "ready");
    }

    #[test]
    fn rejects_decision_with_invalid_chosen_option() {
        let dir = tempdir().unwrap();
        let err = execute_mutation(
            dir.path(),
            "create_decision",
            &json!({
                "title": "Pick optimizer",
                "context": "Need faster convergence",
                "options": [{"id": "opt-a", "label": "Adam", "estimated_cost": "low"}],
                "chosen_option_id": "opt-b",
                "decision_score": 0.7,
                "rationale": "Adam converges faster in our benchmarks",
            }),
        )
        .unwrap_err();
        assert!(err.to_string().contains("not found in options"));
    }

    #[test]
    fn creates_decision_with_valid_option() {
        let dir = tempdir().unwrap();
        let result = execute_mutation(
            dir.path(),
            "create_decision",
            &json!({
                "title": "Pick optimizer",
                "context": "Need faster convergence",
                "options": [{"id": "opt-a", "label": "Adam", "estimated_cost": "low"}],
                "chosen_option_id": "opt-a",
                "decision_score": 0.7,
                "rationale": "Adam converges faster in our benchmarks",
            }),
        )
        .unwrap();
        assert_eq!(result["chosen_option_id"], "opt-a");
    }

    #[test]
    fn creates_experiment_and_updates_status() {
        let dir = tempdir().unwrap();
        let experiment = execute_mutation(
            dir.path(),
            "create_experiment",
            &json!({"title": "Run A", "domain_id": "ai-ml"}),
        )
        .unwrap();
        let id = experiment["id"].as_str().unwrap().to_string();

        let updated = execute_mutation(
            dir.path(),
            "update_experiment",
            &json!({"id": id, "status": "completed"}),
        )
        .unwrap();
        assert_eq!(updated["status"], "completed");
    }

    #[test]
    fn rejects_experiment_update_for_missing_id() {
        let dir = tempdir().unwrap();
        let err = execute_mutation(
            dir.path(),
            "update_experiment",
            &json!({"id": "does-not-exist", "status": "completed"}),
        )
        .unwrap_err();
        assert!(err.to_string().contains("Experiment not found"));
    }

    #[test]
    fn rejects_invalid_experiment_status() {
        let dir = tempdir().unwrap();
        let experiment = execute_mutation(
            dir.path(),
            "create_experiment",
            &json!({"title": "Run A", "domain_id": "ai-ml"}),
        )
        .unwrap();
        let id = experiment["id"].as_str().unwrap().to_string();

        let err = execute_mutation(
            dir.path(),
            "update_experiment",
            &json!({"id": id, "status": "not-a-real-status"}),
        )
        .unwrap_err();
        assert!(err.to_string().contains("Invalid experiment status"));
    }

    #[test]
    fn creates_negative_result_via_mutation() {
        let dir = tempdir().unwrap();
        let result = execute_mutation(
            dir.path(),
            "create_negative_result",
            &json!({
                "title": "OOM on 8xA100",
                "description": "Ran out of memory with batch size 512",
                "failure_mode": "resource_exhaustion",
                "domain_id": "ai-ml",
                "learned": "Reduce batch size or use gradient checkpointing",
            }),
        )
        .unwrap();
        assert_eq!(result["title"], "OOM on 8xA100");
    }

    #[test]
    fn creates_memory_via_mutation() {
        let dir = tempdir().unwrap();
        let result = execute_mutation(
            dir.path(),
            "create_memory",
            &json!({"content": "Learning rate 3e-4 consistently outperforms 1e-3", "importance": 0.8}),
        )
        .unwrap();
        assert_eq!(result["importance"], 0.8);
    }

    #[test]
    fn links_two_objects_via_mutation() {
        let dir = tempdir().unwrap();
        let hyp_a = execute_mutation(
            dir.path(),
            "create_hypothesis",
            &json!({"title": "A", "description": "D", "domain_id": "ai-ml"}),
        )
        .unwrap();
        let hyp_b = execute_mutation(
            dir.path(),
            "create_hypothesis",
            &json!({"title": "B", "description": "D", "domain_id": "ai-ml"}),
        )
        .unwrap();

        let result = execute_mutation(
            dir.path(),
            "link_objects",
            &json!({
                "from_type": "hypothesis",
                "from_id": hyp_a["id"].as_str().unwrap(),
                "to_type": "hypothesis",
                "to_id": hyp_b["id"].as_str().unwrap(),
                "relation": "extends",
            }),
        )
        .unwrap();
        assert_eq!(result["edge_type"], "extends");
    }

    #[test]
    fn rejects_invalid_link_relation() {
        let dir = tempdir().unwrap();
        let err = execute_mutation(
            dir.path(),
            "link_objects",
            &json!({
                "from_type": "hypothesis",
                "from_id": "a",
                "to_type": "hypothesis",
                "to_id": "b",
                "relation": "loves",
            }),
        )
        .unwrap_err();
        assert!(err.to_string().contains("Invalid relation"));
    }

    #[test]
    fn rejects_hypothesis_update_for_missing_id() {
        let dir = tempdir().unwrap();
        let err = execute_mutation(
            dir.path(),
            "update_hypothesis",
            &json!({"id": "missing", "status": "active"}),
        )
        .unwrap_err();
        assert!(err.to_string().contains("Hypothesis not found"));
    }
}
