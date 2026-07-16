use super::{
    AgentContext, Artifact, LifecycleState, ObjectType, PreviewDescriptor, ScientificObject,
    VisualizationMapping,
};
use crate::research_domains::model::{DomainAsset, DomainWorkspace};
use crate::research_domains::DomainTaskRecord;
use serde_json::json;
use serde_json::Value;
use std::collections::BTreeSet;

pub fn domain_asset_object(asset: &DomainAsset, owner: &str) -> ScientificObject {
    let mut object = ScientificObject::new(
        ObjectType::from(asset_object_type(asset)),
        asset.name.clone(),
        owner,
    );
    object.id = stable_id("domain-asset", &asset.id);
    object.lifecycle = LifecycleState::Active;
    object.description = format!("{} asset in {}", asset.file_type, asset.domain_id);
    object.tags = BTreeSet::from([asset.domain_id.clone(), asset.file_type.clone()]);
    object.metadata.insert("domain_id".into(), json!(asset.domain_id));
    object.metadata.insert("source_id".into(), json!(asset.source_id));
    object.artifacts.push(Artifact::new(
        &asset.path,
        &asset.file_type,
        &asset.content_revision,
    ));
    object.preview = PreviewDescriptor {
        provider: "research-domain".into(),
        kind: "domain-asset".into(),
        payload: json!({"domain_id": asset.domain_id, "asset_id": asset.id}),
    };
    object.visualizations = asset
        .visualizations
        .iter()
        .map(|visualization| VisualizationMapping {
            provider: "atlas-visualization".into(),
            kind: visualization.renderer.clone(),
            source_artifact_ids: object.artifacts.iter().map(|artifact| artifact.id.clone()).collect(),
            options: json!({"visualization_id": visualization.id, "adapter": visualization.adapter}),
        })
        .collect();
    object.ai_context = AgentContext {
        summary: format!("Active {} object {}", asset.domain_id, asset.name),
        data: json!({"domain_id": asset.domain_id, "path": asset.path}),
        ..AgentContext::default()
    };
    object.rebuild_search_index();
    object
}

pub fn domain_task_object(task: &DomainTaskRecord) -> ScientificObject {
    let mut object = ScientificObject::new("experiment", task.intent_label.clone(), task.agent.clone());
    object.id = stable_id("domain-task", &task.id);
    object.description = task.prompt.clone();
    object.lifecycle = match task.status.as_str() {
        "running" | "verifying" => LifecycleState::Running,
        "completed" => LifecycleState::Completed,
        "failed" | "cancelled" => LifecycleState::Failed,
        "blocked" => LifecycleState::Blocked,
        _ => LifecycleState::Draft,
    };
    object.tags = BTreeSet::from([task.domain_id.clone(), task.intent_id.clone()]);
    object.metadata.insert("domain_id".into(), json!(task.domain_id));
    object.metadata.insert("task_id".into(), json!(task.id));
    object.metadata.insert("status".into(), json!(task.status));
    object.metadata.insert("current_stage".into(), json!(task.current_stage));
    object.metadata.insert("parameters".into(), task.parameters.clone());
    object.artifacts = task
        .artifacts
        .iter()
        .map(|artifact| Artifact::new(&artifact.path, &artifact.kind, &task.revision))
        .collect();
    object.preview = PreviewDescriptor {
        provider: "research-domain".into(),
        kind: task.preview_kind.clone(),
        payload: json!({"domain_id": task.domain_id, "task_id": task.id}),
    };
    object.ai_context = AgentContext {
        summary: task.note.clone(),
        instructions: vec![task.gate.clone()],
        data: json!({"expected_outputs": task.expected_outputs, "required_sdks": task.required_sdks}),
        ..AgentContext::default()
    };
    object.rebuild_search_index();
    object
}

pub fn domain_workspace_object(workspace: &DomainWorkspace, owner: &str) -> ScientificObject {
    let mut object = ScientificObject::new(
        "workspace",
        format!("{} Workspace", workspace.domain.metadata.label),
        owner,
    );
    object.id = stable_id("domain-workspace", &workspace.domain.metadata.id);
    object.description = workspace.domain.metadata.description.clone();
    object.lifecycle = LifecycleState::Active;
    object.tags = BTreeSet::from([
        workspace.domain.metadata.id.clone(),
        "research-domain".into(),
    ]);
    object.metadata.insert("domain_id".into(), json!(workspace.domain.metadata.id));
    object.metadata.insert("domain_version".into(), json!(workspace.domain.metadata.version));
    object.metadata.insert("workspace_revision".into(), json!(workspace.revision));
    object.ai_context = AgentContext {
        summary: format!("Active {} scientific workspace", workspace.domain.metadata.label),
        data: workspace.state.clone(),
        ..AgentContext::default()
    };
    object.rebuild_search_index();
    object
}

pub fn legacy_research_object(object_type: &str, source: &Value, owner: &str) -> Option<ScientificObject> {
    let id = source.get("id")?.as_str()?;
    let display_name = source
        .get("title")
        .or_else(|| source.get("display_name"))
        .or_else(|| source.get("summary"))
        .or_else(|| source.get("label"))
        .or_else(|| source.get("content"))
        .and_then(Value::as_str)
        .unwrap_or(object_type);
    let mut object = ScientificObject::new(ObjectType::from(object_type), display_name, owner);
    object.id = stable_id("research-os", &format!("{object_type}:{id}"));
    object.description = source
        .get("description")
        .or_else(|| source.get("summary"))
        .or_else(|| source.get("content"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    object.metadata.insert("legacy_research_os_id".into(), json!(id));
    object.metadata.insert("legacy_payload".into(), source.clone());
    object.lifecycle = match source.get("status").and_then(Value::as_str).unwrap_or_default() {
        "running" | "active" => LifecycleState::Running,
        "completed" | "validated" | "published" | "ready" => LifecycleState::Completed,
        "failed" | "refuted" => LifecycleState::Failed,
        "blocked" => LifecycleState::Blocked,
        "abandoned" | "archived" => LifecycleState::Archived,
        _ => LifecycleState::Draft,
    };
    object.rebuild_search_index();
    Some(object)
}

fn asset_object_type(asset: &DomainAsset) -> &'static str {
    match asset.domain_id.as_str() {
        "ai-ml" => "model",
        "cad" => "cad-model",
        "robotics" => "robot",
        "computer-networks" => "network-topology",
        "compiler" => "compiler-artifact",
        "database" => "database-schema",
        "cyber-security" => "security-report",
        _ => "artifact-object",
    }
}

fn stable_id(namespace: &str, source: &str) -> String {
    blake3::hash(format!("atlas:{namespace}:{source}").as_bytes())
        .to_hex()[..32]
        .to_string()
}
