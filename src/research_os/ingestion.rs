//! Agent Ingestion Module
//!
//! Automatically captures agent turns and domain tasks into Research OS objects.

use super::diary::{create_diary_entry, create_diary_entry_with_source, list_diary_entries, DiaryEntryType};
use super::evidence::{create_evidence, list_evidence, EvidenceKind};
use super::experiment_lineage::{create_experiment, list_experiments, update_experiment, ExperimentUpdate};
use super::negative_results::{create_negative_result, list_negative_results};
use super::object_graph::ResearchObjectId;
use super::memory::{create_memory_entry, list_memory_entries};
use super::timeline::{create_timeline_event, EventType};
use anyhow::Result;
use serde_json::Value;
use std::path::Path;

/// Ingest a domain task into Research OS
pub fn ingest_domain_task(
    workspace_root: &Path,
    task: &Value,
    agent_name: &str,
) -> Result<Vec<ResearchObjectId>> {
    let mut created_objects = Vec::new();

    let domain_id = task
        .get("domain_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let task_id = task.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let intent_label = task
        .get("intent_label")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let status = task.get("status").and_then(|v| v.as_str()).unwrap_or("");
    let prompt = task.get("prompt").and_then(|v| v.as_str()).unwrap_or("");

    let existing_experiment = list_experiments(workspace_root)?
        .into_iter()
        .find(|item| item.task_id.as_deref() == Some(task_id));
    let previous_status = existing_experiment.as_ref().map(|item| item.status.clone());

    // Record one diary entry per real status transition, not per polling/update call.
    if previous_status.as_deref() != Some(status) {
        let diary_content = format!(
            "Domain task {}: {} in domain {}",
            if previous_status.is_none() { "started" } else { status },
            intent_label,
            domain_id
        );
        let diary_entry = create_diary_entry(
            workspace_root,
            DiaryEntryType::Observation,
            &diary_content,
            agent_name,
            Some(domain_id.to_string()),
            vec![],
        )?;
        created_objects.push(ResearchObjectId {
            object_type: "diary".to_string(),
            id: diary_entry.id.clone(),
            created_at: diary_entry.timestamp.clone(),
        });
    }

    // One domain task owns one lineage node across its full lifecycle.
    let experiment_title = format!("{} - {}", domain_id, intent_label);
    let parameters = task.get("parameters").cloned().unwrap_or_else(|| Value::Object(Default::default()));
    let experiment = if let Some(existing) = existing_experiment {
        existing
    } else {
        let created = create_experiment(
            workspace_root,
            &experiment_title,
            domain_id,
            Some(task_id.to_string()),
            None, // hypothesis_id can be linked later
            parameters,
            agent_name,
        )?;
        created_objects.push(ResearchObjectId {
            object_type: "experiment".to_string(),
            id: created.id.clone(),
            created_at: created.created_at.clone(),
        });
        created
    };

    let normalized_status = match status {
        "completed" => "completed",
        "failed" | "error" | "cancelled" | "blocked" => "failed",
        "running" | "verifying" => "running",
        _ => "planned",
    };
    if experiment.status != normalized_status {
        update_experiment(
            workspace_root,
            &experiment.id,
            ExperimentUpdate {
                status: Some(normalized_status.to_string()),
                ..Default::default()
            },
        )?;
    }

    // 3. Update experiment with artifacts if present
    if let Some(artifacts_array) = task.get("artifacts").and_then(|v| v.as_array()) {
        let artifact_paths: Vec<String> = artifacts_array
            .iter()
            .filter_map(|a| a.get("path").and_then(|p| p.as_str()).map(|s| s.to_string()))
            .collect();

        if !artifact_paths.is_empty() {
            update_experiment(
                workspace_root,
                &experiment.id,
                ExperimentUpdate {
                    artifacts: Some(artifact_paths.clone()),
                    status: Some(normalized_status.to_string()),
                    ..Default::default()
                },
            )?;

            // 4. Create evidence from artifacts
            let existing_evidence = list_evidence(workspace_root)?;
            for artifact in artifacts_array {
                if let Some(path) = artifact.get("path").and_then(|v| v.as_str()) {
                    if existing_evidence.iter().any(|item| {
                        item.experiment_id.as_deref() == Some(experiment.id.as_str())
                            && item.source_path.as_deref() == Some(path)
                    }) {
                        continue;
                    }
                    let evidence_summary = format!("Artifact generated: {}", path);
                    let evidence = create_evidence(
                        workspace_root,
                        EvidenceKind::Artifact,
                        &evidence_summary,
                        0.7, // Default strength for task artifacts
                        true,
                        Some(path.to_string()),
                        None,
                        None,
                        Some(experiment.id.clone()),
                        agent_name,
                    )?;
                    created_objects.push(ResearchObjectId {
                        object_type: "evidence".to_string(),
                        id: evidence.id.clone(),
                        created_at: evidence.created_at.clone(),
                    });
                }
            }
        }
    }

    // Task-level verification is distinct from the artifact itself. Preserve each
    // declared check (and its reproduction command) as evidence so Research OS
    // can show why a completed task is trustworthy, not merely what it produced.
    if let Some(task_evidence) = task.get("evidence").and_then(Value::as_array) {
        let existing_evidence = list_evidence(workspace_root)?;
        for item in task_evidence {
            let label = item.get("label").and_then(Value::as_str).unwrap_or("").trim();
            let summary = item.get("summary").and_then(Value::as_str).unwrap_or("").trim();
            if label.is_empty() || summary.is_empty() {
                continue;
            }
            let source_path = item
                .get("path")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            let source_command = item
                .get("command")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            let evidence_summary = format!("{}: {}", label, summary);
            if existing_evidence.iter().any(|evidence| {
                evidence.experiment_id.as_deref() == Some(experiment.id.as_str())
                    && evidence.summary == evidence_summary
                    && evidence.source_path == source_path
                    && evidence.source_command == source_command
            }) {
                continue;
            }
            let evidence = create_evidence(
                workspace_root,
                verification_evidence_kind(label, summary, source_command.as_deref()),
                &evidence_summary,
                0.9,
                true,
                source_path,
                source_command,
                None,
                Some(experiment.id.clone()),
                agent_name,
            )?;
            created_objects.push(ResearchObjectId {
                object_type: "evidence".to_string(),
                id: evidence.id,
                created_at: evidence.created_at,
            });
        }
    }

    // 5. If task failed, create negative result
    if (status == "failed" || status == "error")
        && previous_status.as_deref() != Some("failed")
    {
        let note = task.get("note").and_then(|v| v.as_str()).unwrap_or("");
        let failure_description = if note.is_empty() {
            format!("Task {} failed", intent_label)
        } else {
            note.to_string()
        };

        let learned = extract_learned_from_failure(&failure_description);

        if !list_negative_results(workspace_root)?
            .iter()
            .any(|item| item.task_id.as_deref() == Some(task_id))
        {
            let negative_result = create_negative_result(
                workspace_root,
                &format!("Failed: {}", intent_label),
                &failure_description,
                "task_failure",
                domain_id,
                &learned,
                None,
                Some(experiment.id.clone()),
                Some(task_id.to_string()),
                vec![],
                agent_name,
            )?;
            created_objects.push(ResearchObjectId {
                object_type: "negative-result".to_string(),
                id: negative_result.id.clone(),
                created_at: negative_result.created_at.clone(),
            });
        }

        // Create timeline event for failure
        create_timeline_event(
            workspace_root,
            EventType::FailureRecorded,
            &format!("Failed: {}", intent_label),
            &failure_description,
            Some(domain_id.to_string()),
            created_objects.clone(),
        )?;
    } else if status == "completed" && previous_status.as_deref() != Some("completed") {
        // Create timeline event for successful completion
        create_timeline_event(
            workspace_root,
            EventType::ExperimentRun,
            &format!("Completed: {}", intent_label),
            prompt,
            Some(domain_id.to_string()),
            created_objects.clone(),
        )?;
    }

    Ok(created_objects)
}

/// Ingest a general agent turn into Research OS
pub fn ingest_agent_turn(
    workspace_root: &Path,
    turn_type: &str,
    turn_data: &Value,
    agent_name: &str,
) -> Result<Vec<ResearchObjectId>> {
    let mut created_objects = Vec::new();
    let source_id = turn_data
        .get("turn_id")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    if let Some(source_id) = source_id.as_deref() {
        if list_diary_entries(workspace_root)?
            .iter()
            .any(|entry| entry.source_id.as_deref() == Some(source_id))
        {
            return Ok(created_objects);
        }
    }

    // Create diary entry for the turn
    let content = match turn_type {
        "domain_task" => {
            if let Some(task) = turn_data.get("task") {
                return ingest_domain_task(workspace_root, task, agent_name);
            }
            "Agent executed a domain task".to_string()
        }
        "chat" => {
            if let Some(message) = turn_data.get("message").and_then(|v| v.as_str()) {
                format!("Agent response: {}", summarize_text(message, 900))
            } else {
                "Agent chat turn".to_string()
            }
        }
        "tool_call" => {
            if let Some(tool) = turn_data.get("tool").and_then(|v| v.as_str()) {
                format!("Agent used tool: {}", tool)
            } else {
                "Agent tool call".to_string()
            }
        }
        _ => format!("Agent turn: {}", turn_type),
    };

    let diary_entry = create_diary_entry_with_source(
        workspace_root,
        DiaryEntryType::Observation,
        &content,
        agent_name,
        None,
        vec![],
        source_id,
    )?;

    created_objects.push(ResearchObjectId {
        object_type: "diary".to_string(),
        id: diary_entry.id.clone(),
        created_at: diary_entry.timestamp.clone(),
    });

    create_timeline_event(
        workspace_root,
        EventType::AgentActivity,
        "Atlas agent activity recorded",
        &content,
        None,
        created_objects.clone(),
    )?;

    if turn_type == "chat" {
        if let Some(message) = turn_data.get("message").and_then(|value| value.as_str()) {
            if should_capture_memory(message) {
                let memory_content = summarize_text(message, 1200);
                if !list_memory_entries(workspace_root)?
                    .iter()
                    .any(|entry| entry.content == memory_content)
                {
                    let memory = create_memory_entry(
                        workspace_root,
                        &memory_content,
                        turn_data
                            .get("importance")
                            .and_then(|value| value.as_f64())
                            .unwrap_or(0.65),
                        created_objects.clone(),
                        None,
                    )?;
                    created_objects.push(ResearchObjectId {
                        object_type: "memory".to_string(),
                        id: memory.id,
                        created_at: memory.created_at,
                    });
                }
            }
        }
    }

    Ok(created_objects)
}

fn summarize_text(text: &str, max_chars: usize) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= max_chars {
        return normalized;
    }
    let mut summary = normalized.chars().take(max_chars.saturating_sub(1)).collect::<String>();
    summary.push('…');
    summary
}

fn should_capture_memory(message: &str) -> bool {
    let lowered = message.to_lowercase();
    [
        "insight", "learned", "lesson", "finding", "conclusion", "decision", "assumption",
        "发现", "结论", "经验", "教训", "决策", "假设", "根因",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn verification_evidence_kind(label: &str, summary: &str, command: Option<&str>) -> EvidenceKind {
    let description = format!("{} {}", label, summary).to_ascii_lowercase();
    if description.contains("literature")
        || description.contains("citation")
        || description.contains("paper")
        || description.contains("reference")
    {
        EvidenceKind::Literature
    } else if description.contains("benchmark")
        || description.contains("metric")
        || description.contains("accuracy")
        || description.contains("latency")
        || description.contains("throughput")
    {
        EvidenceKind::Benchmark
    } else if command.is_some() {
        EvidenceKind::Experimental
    } else {
        EvidenceKind::Artifact
    }
}

/// Extract learned insights from failure description
fn extract_learned_from_failure(description: &str) -> String {
    // Simple heuristic: look for patterns indicating lessons learned
    let lower = description.to_lowercase();

    if lower.contains("out of memory") || lower.contains("oom") {
        "Reduce batch size or model size to fit in memory".to_string()
    } else if lower.contains("timeout") || lower.contains("timed out") {
        "Increase timeout or optimize computation".to_string()
    } else if lower.contains("not found") || lower.contains("missing") {
        "Ensure required files or dependencies are present".to_string()
    } else if lower.contains("permission denied") {
        "Check file permissions and access rights".to_string()
    } else if lower.contains("invalid") || lower.contains("error") {
        "Validate input parameters and data format".to_string()
    } else {
        "Review error logs and adjust approach".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn ingests_successful_domain_task() {
        let dir = tempdir().unwrap();
        let task = json!({
            "id": "task-123",
            "domain_id": "ai-ml",
            "intent_label": "Train Model",
            "status": "completed",
            "prompt": "Train a classification model",
            "parameters": {"epochs": 10},
            "artifacts": [
                {"path": "model.pth", "kind": "model"}
            ],
            "evidence": [{
                "label": "benchmark evaluation",
                "summary": "validation accuracy exceeded the acceptance gate",
                "path": "model.pth",
                "command": "python evaluate.py --checkpoint model.pth"
            }]
        });

        let objects = ingest_domain_task(dir.path(), &task, "agent").unwrap();

        // Should create: diary entry, experiment, evidence
        assert!(objects.len() >= 3);
        assert!(objects.iter().any(|o| o.object_type == "diary"));
        assert!(objects.iter().any(|o| o.object_type == "experiment"));
        assert!(objects.iter().any(|o| o.object_type == "evidence"));
        let evidence = list_evidence(dir.path()).unwrap();
        assert!(evidence.iter().any(|item| {
            item.kind == "benchmark"
                && item.source_command.as_deref() == Some("python evaluate.py --checkpoint model.pth")
                && item.summary.contains("validation accuracy")
        }));
    }

    #[test]
    fn ingests_failed_domain_task() {
        let dir = tempdir().unwrap();
        let task = json!({
            "id": "task-456",
            "domain_id": "ai-ml",
            "intent_label": "Train Model",
            "status": "failed",
            "prompt": "Train a model",
            "note": "Out of memory error",
            "parameters": {},
            "artifacts": []
        });

        let objects = ingest_domain_task(dir.path(), &task, "agent").unwrap();

        // Should create: diary entry, experiment, negative result
        assert!(objects.len() >= 3);
        assert!(objects.iter().any(|o| o.object_type == "negative-result"));
    }

    #[test]
    fn extracts_learned_insights() {
        assert_eq!(
            extract_learned_from_failure("Out of memory error occurred"),
            "Reduce batch size or model size to fit in memory"
        );
        assert_eq!(
            extract_learned_from_failure("File not found"),
            "Ensure required files or dependencies are present"
        );
        assert_eq!(
            extract_learned_from_failure("Operation timed out"),
            "Increase timeout or optimize computation"
        );
    }

    #[test]
    fn ingests_general_agent_turn() {
        let dir = tempdir().unwrap();
        let turn_data = json!({
            "tool": "read_file",
            "result": "success"
        });

        let objects = ingest_agent_turn(dir.path(), "tool_call", &turn_data, "agent").unwrap();

        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].object_type, "diary");
    }

    #[test]
    fn agent_turn_ingestion_is_idempotent_and_captures_memory() {
        let dir = tempdir().unwrap();
        let turn_data = json!({
            "turn_id": "session-1:turn-3",
            "message": "Key finding: smaller batches avoid the observed memory spike.",
            "importance": 0.82
        });

        let first = ingest_agent_turn(dir.path(), "chat", &turn_data, "atlas").unwrap();
        let second = ingest_agent_turn(dir.path(), "chat", &turn_data, "atlas").unwrap();

        assert!(first.iter().any(|object| object.object_type == "memory"));
        assert!(second.is_empty());
        assert_eq!(list_diary_entries(dir.path()).unwrap().len(), 1);
        assert_eq!(list_memory_entries(dir.path()).unwrap().len(), 1);
    }
}
