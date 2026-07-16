use super::actions::DomainActionDescriptor;
use super::model::{DomainAsset, DomainIntentDescriptor, DomainPluginDescriptor};
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Component, Path, PathBuf};

const TASK_SCHEMA: &str = "atlas.research-domain-task.v1";
const MAX_TASK_BYTES: usize = 512 * 1024;
const MAX_TASKS_PER_DOMAIN: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainTaskArtifact {
    pub path: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visualization_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainTaskEvidence {
    pub label: String,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainTaskRecord {
    pub schema_version: String,
    pub id: String,
    pub domain_id: String,
    pub intent_id: String,
    pub intent_label: String,
    pub prompt: String,
    pub agent: String,
    pub status: String,
    pub current_stage: String,
    pub workflow_stages: Vec<String>,
    pub input_contract: String,
    pub expected_outputs: Vec<String>,
    pub recommended_actions: Vec<String>,
    pub required_sdks: Vec<String>,
    pub preview_kind: String,
    pub gate: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_revision: Option<String>,
    #[serde(default)]
    pub parameters: Value,
    #[serde(default)]
    pub artifacts: Vec<DomainTaskArtifact>,
    #[serde(default)]
    pub evidence: Vec<DomainTaskEvidence>,
    #[serde(default)]
    pub note: String,
    pub created_at: String,
    pub updated_at: String,
    pub updated_by: String,
    pub revision: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DomainTaskBeginRequest {
    pub domain_id: String,
    pub intent_id: String,
    pub prompt: String,
    #[serde(default)]
    pub asset_id: Option<String>,
    #[serde(default)]
    pub parameters: Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DomainTaskUpdateRequest {
    pub domain_id: String,
    pub task_id: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub current_stage: Option<String>,
    #[serde(default)]
    pub artifacts: Option<Vec<DomainTaskArtifact>>,
    #[serde(default)]
    pub evidence: Option<Vec<DomainTaskEvidence>>,
    #[serde(default)]
    pub note: Option<String>,
}

pub fn intent_catalog(
    plugin: &DomainPluginDescriptor,
    actions: &[DomainActionDescriptor],
    execution: &Value,
) -> Value {
    let adapter_statuses = execution
        .get("adapter_status")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let intents = plugin
        .workbench
        .intents
        .iter()
        .map(|intent| {
            let sdk_statuses = intent
                .required_sdks
                .iter()
                .map(|sdk| {
                    let normalized = sdk.to_ascii_lowercase();
                    let status = adapter_statuses.iter().find(|status| {
                        let candidate = status
                            .get("sdk")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_ascii_lowercase();
                        candidate == normalized
                            || candidate.contains(&normalized)
                            || normalized.contains(&candidate)
                    });
                    json!({
                        "sdk": sdk,
                        "available": status.and_then(|value| value.get("available")).and_then(Value::as_bool),
                        "reason": status.and_then(|value| value.get("reason")).and_then(Value::as_str),
                        "version": status.and_then(|value| value.get("version")).and_then(Value::as_str),
                    })
                })
                .collect::<Vec<_>>();
            let native_actions = intent
                .recommended_actions
                .iter()
                .map(|id| {
                    actions
                        .iter()
                        .find(|action| action.id == *id)
                        .map(|action| json!({
                            "id": action.id,
                            "label": action.label,
                            "sdk": action.sdk,
                            "ready": action.ready,
                            "reason": action.reason,
                        }))
                        .unwrap_or_else(|| json!({ "id": id, "ready": false, "reason": "Action becomes available after the Agent creates a compatible real artifact." }))
                })
                .collect::<Vec<_>>();
            let available = sdk_statuses
                .iter()
                .filter_map(|value| value.get("available").and_then(Value::as_bool))
                .filter(|value| *value)
                .count();
            let resolved = sdk_statuses
                .iter()
                .filter(|value| value.get("available").is_some_and(|value| !value.is_null()))
                .count();
            let toolchain_status = if sdk_statuses.is_empty() {
                "workspace"
            } else if available == sdk_statuses.len() {
                "ready"
            } else if available > 0 {
                "partial"
            } else if resolved > 0 {
                "unavailable"
            } else {
                "unresolved"
            };
            json!({
                "contract": intent,
                "toolchain_status": toolchain_status,
                "sdk_statuses": sdk_statuses,
                "native_actions": native_actions,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema_version": TASK_SCHEMA,
        "domain_id": plugin.metadata.id,
        "intents": intents,
    })
}

pub fn begin_task(
    workspace_root: &Path,
    plugin: &DomainPluginDescriptor,
    assets: &[DomainAsset],
    request: &DomainTaskBeginRequest,
    updated_by: &str,
) -> Result<DomainTaskRecord> {
    validate_domain_id(&request.domain_id)?;
    if request.domain_id != plugin.metadata.id {
        return Err(anyhow!("task domain does not match the active plugin"));
    }
    let intent = find_intent(plugin, &request.intent_id)?;
    let prompt = request.prompt.trim();
    if prompt.is_empty() || prompt.len() > 16_000 {
        return Err(anyhow!(
            "domain task prompt must contain 1..16000 characters"
        ));
    }
    if !request.parameters.is_null() && !request.parameters.is_object() {
        return Err(anyhow!("domain task parameters must be an object"));
    }
    let asset = request
        .asset_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(|id| find_asset(assets, id))
        .transpose()?;
    if intent.asset_required && asset.is_none() {
        return Err(anyhow!(
            "this domain intent requires a selected real workspace asset"
        ));
    }
    let now = Utc::now().to_rfc3339();
    let mut task = DomainTaskRecord {
        schema_version: TASK_SCHEMA.into(),
        id: uuid::Uuid::new_v4().to_string(),
        domain_id: request.domain_id.clone(),
        intent_id: intent.id.clone(),
        intent_label: intent.label.clone(),
        prompt: prompt.to_string(),
        agent: intent.agent.clone(),
        status: "planning".into(),
        current_stage: intent
            .workflow_stages
            .first()
            .cloned()
            .unwrap_or_else(|| "plan".into()),
        workflow_stages: intent.workflow_stages.clone(),
        input_contract: intent.input_contract.clone(),
        expected_outputs: intent.expected_outputs.clone(),
        recommended_actions: intent.recommended_actions.clone(),
        required_sdks: intent.required_sdks.clone(),
        preview_kind: intent.preview_kind.clone(),
        gate: intent.gate.clone(),
        asset_id: asset.map(|value| value.id.clone()),
        asset_path: asset.map(|value| value.path.clone()),
        asset_revision: asset.map(|value| value.content_revision.clone()),
        parameters: if request.parameters.is_null() {
            json!({})
        } else {
            request.parameters.clone()
        },
        artifacts: Vec::new(),
        evidence: Vec::new(),
        note: String::new(),
        created_at: now.clone(),
        updated_at: now,
        updated_by: normalized_actor(updated_by),
        revision: String::new(),
    };
    write_task(workspace_root, &mut task)?;
    Ok(task)
}

pub fn read_tasks(workspace_root: &Path, domain_id: &str) -> Result<Vec<DomainTaskRecord>> {
    let directory = task_directory(workspace_root, domain_id)?;
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths = fs::read_dir(&directory)?
        .filter_map(|entry| entry.ok().map(|value| value.path()))
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort_by(|left, right| right.file_name().cmp(&left.file_name()));
    let mut tasks = Vec::new();
    for path in paths.into_iter().take(MAX_TASKS_PER_DOMAIN) {
        let Ok(bytes) = fs::read(&path) else { continue };
        if bytes.len() > MAX_TASK_BYTES {
            continue;
        }
        let Ok(task) = serde_json::from_slice::<DomainTaskRecord>(&bytes) else {
            continue;
        };
        if task.schema_version == TASK_SCHEMA && task.domain_id == domain_id {
            tasks.push(task);
        }
    }
    tasks.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Ok(tasks)
}

pub fn read_task(
    workspace_root: &Path,
    domain_id: &str,
    task_id: &str,
) -> Result<DomainTaskRecord> {
    let path = task_path(workspace_root, domain_id, task_id)?;
    let bytes = fs::read(&path)
        .with_context(|| format!("domain task is unavailable: {}", path.display()))?;
    if bytes.len() > MAX_TASK_BYTES {
        return Err(anyhow!("domain task exceeds the size limit"));
    }
    let task = serde_json::from_slice::<DomainTaskRecord>(&bytes)?;
    if task.schema_version != TASK_SCHEMA || task.domain_id != domain_id || task.id != task_id {
        return Err(anyhow!("domain task identity is invalid"));
    }
    Ok(task)
}

pub fn update_task(
    workspace_root: &Path,
    plugin: &DomainPluginDescriptor,
    request: &DomainTaskUpdateRequest,
    updated_by: &str,
) -> Result<DomainTaskRecord> {
    validate_domain_id(&request.domain_id)?;
    if request.domain_id != plugin.metadata.id {
        return Err(anyhow!("task domain does not match the active plugin"));
    }
    let mut task = read_task(workspace_root, &request.domain_id, &request.task_id)?;
    let intent = find_intent(plugin, &task.intent_id)?;
    if let Some(status) = request.status.as_deref() {
        validate_status_transition(&task.status, status)?;
        task.status = status.to_string();
    }
    if let Some(stage) = request.current_stage.as_deref() {
        if !intent
            .workflow_stages
            .iter()
            .any(|candidate| candidate == stage)
        {
            return Err(anyhow!(
                "unknown workflow stage for this domain intent: {stage}"
            ));
        }
        task.current_stage = stage.to_string();
    }
    if let Some(artifacts) = &request.artifacts {
        if artifacts.len() > 128 {
            return Err(anyhow!("domain task artifact list is too large"));
        }
        for artifact in artifacts {
            validate_artifact(workspace_root, artifact)?;
        }
        task.artifacts = artifacts.clone();
    }
    if let Some(evidence) = &request.evidence {
        if evidence.len() > 256 {
            return Err(anyhow!("domain task evidence list is too large"));
        }
        for item in evidence {
            if item.label.trim().is_empty() || item.summary.trim().is_empty() {
                return Err(anyhow!("each evidence item requires a label and summary"));
            }
            if let Some(path) = item.path.as_deref() {
                validate_existing_workspace_file(workspace_root, path)?;
            }
        }
        task.evidence = evidence.clone();
    }
    if let Some(note) = request.note.as_deref() {
        if note.len() > 32_000 {
            return Err(anyhow!("domain task note is too large"));
        }
        task.note = note.to_string();
    }
    if task.status == "completed" {
        if task.artifacts.is_empty() {
            return Err(anyhow!(
                "a completed domain task requires at least one real artifact"
            ));
        }
        if task.evidence.is_empty() {
            return Err(anyhow!(
                "a completed domain task requires verification evidence"
            ));
        }
        for artifact in &task.artifacts {
            validate_artifact(workspace_root, artifact)?;
        }
        task.current_stage = intent
            .workflow_stages
            .last()
            .cloned()
            .unwrap_or_else(|| "complete".into());
    }
    task.updated_at = Utc::now().to_rfc3339();
    task.updated_by = normalized_actor(updated_by);
    write_task(workspace_root, &mut task)?;
    Ok(task)
}

fn write_task(workspace_root: &Path, task: &mut DomainTaskRecord) -> Result<()> {
    task.revision.clear();
    let revision_input = serde_json::to_vec(task)?;
    task.revision = blake3::hash(&revision_input).to_hex()[..20].to_string();
    let bytes = serde_json::to_vec_pretty(task)?;
    if bytes.len() > MAX_TASK_BYTES {
        return Err(anyhow!("domain task exceeds the size limit"));
    }
    let path = task_path(workspace_root, &task.domain_id, &task.id)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)?;
    Ok(())
}

fn find_intent<'a>(
    plugin: &'a DomainPluginDescriptor,
    intent_id: &str,
) -> Result<&'a DomainIntentDescriptor> {
    plugin
        .workbench
        .intents
        .iter()
        .find(|intent| intent.id == intent_id)
        .ok_or_else(|| anyhow!("unknown domain intent: {intent_id}"))
}

fn find_asset<'a>(assets: &'a [DomainAsset], id: &str) -> Result<&'a DomainAsset> {
    assets
        .iter()
        .find(|asset| asset.id == id || asset.source_id == id || asset.path == id)
        .ok_or_else(|| anyhow!("domain asset is no longer available: {id}"))
}

fn validate_status_transition(current: &str, next: &str) -> Result<()> {
    let allowed = [
        "planning",
        "ready",
        "running",
        "verifying",
        "completed",
        "blocked",
        "failed",
        "cancelled",
    ];
    if !allowed.contains(&next) {
        return Err(anyhow!("invalid domain task status: {next}"));
    }
    if matches!(current, "completed" | "cancelled") && current != next {
        return Err(anyhow!(
            "terminal domain task status cannot transition from {current} to {next}"
        ));
    }
    Ok(())
}

fn validate_artifact(workspace_root: &Path, artifact: &DomainTaskArtifact) -> Result<()> {
    if artifact.kind.trim().is_empty() {
        return Err(anyhow!("domain task artifact kind is required"));
    }
    validate_existing_workspace_file(workspace_root, &artifact.path)
}

fn validate_existing_workspace_file(workspace_root: &Path, relative: &str) -> Result<()> {
    let relative = validate_relative_path(relative)?;
    let root = workspace_root
        .canonicalize()
        .context("workspace root is unavailable")?;
    let candidate = root
        .join(relative)
        .canonicalize()
        .context("domain task evidence file is unavailable")?;
    if !candidate.starts_with(&root) || !candidate.is_file() {
        return Err(anyhow!(
            "domain task evidence must be a real file inside the workspace"
        ));
    }
    Ok(())
}

fn validate_relative_path(value: &str) -> Result<PathBuf> {
    let path = Path::new(value.trim());
    if value.trim().is_empty() || path.is_absolute() {
        return Err(anyhow!("domain task path must be workspace-relative"));
    }
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(anyhow!("domain task path contains an unsafe component"));
    }
    Ok(path.to_path_buf())
}

fn task_directory(workspace_root: &Path, domain_id: &str) -> Result<PathBuf> {
    validate_domain_id(domain_id)?;
    Ok(workspace_root
        .join(".atlas")
        .join("domain-tasks")
        .join(domain_id))
}

fn task_path(workspace_root: &Path, domain_id: &str, task_id: &str) -> Result<PathBuf> {
    validate_identifier(task_id, "task id")?;
    Ok(task_directory(workspace_root, domain_id)?.join(format!("{task_id}.json")))
}

fn validate_domain_id(value: &str) -> Result<()> {
    validate_identifier(value, "research domain id")
}

fn validate_identifier(value: &str, label: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 100
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(anyhow!("invalid {label}"));
    }
    Ok(())
}

fn normalized_actor(value: &str) -> String {
    let actor = value.trim();
    if actor.is_empty() {
        "agent".into()
    } else {
        actor.chars().take(64).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::research_domains::model::{
        DomainLifecycleDescriptor, DomainMetadata, DomainProviderDescriptor,
        DomainWorkbenchDescriptor,
    };
    use tempfile::tempdir;

    fn plugin() -> DomainPluginDescriptor {
        let provider = DomainProviderDescriptor {
            id: "test".into(),
            api_version: "1".into(),
            provider_type: "test".into(),
        };
        DomainPluginDescriptor {
            metadata: DomainMetadata {
                id: "ai-ml".into(),
                label: "ML".into(),
                description: String::new(),
                version: "1".into(),
                category: "test".into(),
            },
            capabilities: Vec::new(),
            supported_file_types: Vec::new(),
            supported_visualizations: Vec::new(),
            supported_agents: vec!["training".into()],
            context_provider: provider.clone(),
            preview_provider: provider.clone(),
            execution_provider: provider.clone(),
            data_provider: provider.clone(),
            visualization_provider: provider.clone(),
            render_provider: provider,
            lifecycle: DomainLifecycleDescriptor {
                states: Vec::new(),
                supports_hot_reload: true,
                supports_workspace_sync: true,
            },
            sdk_adapters: Vec::new(),
            plugin_api_version: "1".into(),
            workbench: DomainWorkbenchDescriptor {
                intents: vec![DomainIntentDescriptor {
                    id: "train".into(),
                    label: "Train".into(),
                    description: String::new(),
                    agent: "training".into(),
                    input_contract: "config".into(),
                    expected_outputs: vec!["checkpoint".into()],
                    recommended_actions: Vec::new(),
                    required_sdks: vec!["PyTorch".into()],
                    workflow_stages: vec!["plan".into(), "verify".into()],
                    preview_kind: "run".into(),
                    gate: "metrics pass".into(),
                    asset_required: false,
                }],
                ..Default::default()
            },
        }
    }

    #[test]
    fn task_requires_real_artifact_and_evidence_before_completion() {
        let root = tempdir().unwrap();
        let task = begin_task(
            root.path(),
            &plugin(),
            &[],
            &DomainTaskBeginRequest {
                domain_id: "ai-ml".into(),
                intent_id: "train".into(),
                prompt: "Train a classifier".into(),
                asset_id: None,
                parameters: json!({}),
            },
            "ui",
        )
        .unwrap();
        let failed = update_task(
            root.path(),
            &plugin(),
            &DomainTaskUpdateRequest {
                domain_id: "ai-ml".into(),
                task_id: task.id.clone(),
                status: Some("completed".into()),
                current_stage: None,
                artifacts: None,
                evidence: None,
                note: None,
            },
            "agent",
        );
        assert!(failed.is_err());
        fs::write(root.path().join("checkpoint.json"), "{}").unwrap();
        let complete = update_task(
            root.path(),
            &plugin(),
            &DomainTaskUpdateRequest {
                domain_id: "ai-ml".into(),
                task_id: task.id,
                status: Some("completed".into()),
                current_stage: None,
                artifacts: Some(vec![DomainTaskArtifact {
                    path: "checkpoint.json".into(),
                    kind: "checkpoint".into(),
                    visualization_id: None,
                }]),
                evidence: Some(vec![DomainTaskEvidence {
                    label: "evaluation".into(),
                    summary: "metric gate passed".into(),
                    path: Some("checkpoint.json".into()),
                    command: None,
                }]),
                note: None,
            },
            "agent",
        )
        .unwrap();
        assert_eq!(complete.status, "completed");
    }
}
