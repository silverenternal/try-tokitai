use anyhow::{anyhow, Result};
use chrono::Utc;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

const STATE_SCHEMA: &str = "atlas.research-domain-workspace-state.v1";
const MAX_STATE_BYTES: usize = 256 * 1024;

fn validate_domain_id(domain_id: &str) -> Result<&str> {
    let id = domain_id.trim();
    if id.is_empty()
        || id.len() > 80
        || !id
            .chars()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, '-' | '_'))
    {
        return Err(anyhow!("invalid research domain id"));
    }
    Ok(id)
}

fn state_path(workspace_root: &Path, domain_id: &str) -> Result<PathBuf> {
    Ok(workspace_root
        .join(".atlas")
        .join("domain-workspaces")
        .join(format!("{}.json", validate_domain_id(domain_id)?)))
}

fn default_state(domain_id: &str) -> Value {
    json!({
        "schema_version": STATE_SCHEMA,
        "domain_id": domain_id,
        "active_tab": "overview",
        "active_asset_id": "",
        "active_visualization_id": "",
        "selected_agent": "",
        "active_task": null,
        "focus": "",
        "notes": "",
        "filters": {},
        "parameters": {},
        "ui": {},
        "revision": blake3::hash(format!("{domain_id}:default").as_bytes()).to_hex().to_string(),
        "updated_at": Utc::now().to_rfc3339(),
        "updated_by": "system"
    })
}

pub fn read_workspace_state(workspace_root: &Path, domain_id: &str) -> Result<Value> {
    let domain_id = validate_domain_id(domain_id)?;
    let path = state_path(workspace_root, domain_id)?;
    if !path.is_file() {
        return Ok(default_state(domain_id));
    }
    let bytes = fs::read(&path)?;
    if bytes.len() > MAX_STATE_BYTES {
        return Err(anyhow!(
            "research domain workspace state exceeds size limit"
        ));
    }
    let mut state = serde_json::from_slice::<Value>(&bytes)?;
    let object = state
        .as_object_mut()
        .ok_or_else(|| anyhow!("research domain workspace state must be an object"))?;
    object.insert("schema_version".to_string(), json!(STATE_SCHEMA));
    object.insert("domain_id".to_string(), json!(domain_id));
    Ok(state)
}

fn merge_patch(target: &mut Value, patch: &Value) {
    let (Some(target), Some(patch)) = (target.as_object_mut(), patch.as_object()) else {
        *target = patch.clone();
        return;
    };
    for (key, value) in patch {
        if matches!(
            key.as_str(),
            "schema_version" | "domain_id" | "revision" | "updated_at"
        ) {
            continue;
        }
        if value.is_null() {
            target.remove(key);
        } else if let Some(existing) = target.get_mut(key) {
            merge_patch(existing, value);
        } else {
            target.insert(key.clone(), value.clone());
        }
    }
}

pub fn update_workspace_state(
    workspace_root: &Path,
    domain_id: &str,
    patch: &Value,
    updated_by: &str,
) -> Result<Value> {
    let domain_id = validate_domain_id(domain_id)?;
    if !patch.is_object() {
        return Err(anyhow!("research domain workspace patch must be an object"));
    }
    let mut state = read_workspace_state(workspace_root, domain_id)?;
    merge_patch(&mut state, patch);
    let mut canonical = state.as_object().cloned().unwrap_or_else(Map::new);
    canonical.insert("schema_version".to_string(), json!(STATE_SCHEMA));
    canonical.insert("domain_id".to_string(), json!(domain_id));
    canonical.insert("updated_at".to_string(), json!(Utc::now().to_rfc3339()));
    canonical.insert(
        "updated_by".to_string(),
        json!(if updated_by.trim().is_empty() {
            "agent"
        } else {
            updated_by.trim()
        }),
    );
    canonical.remove("revision");
    let revision_input = serde_json::to_vec(&canonical)?;
    canonical.insert(
        "revision".to_string(),
        json!(blake3::hash(&revision_input).to_hex().to_string()),
    );
    let state = Value::Object(canonical);
    let encoded = serde_json::to_vec_pretty(&state)?;
    if encoded.len() > MAX_STATE_BYTES {
        return Err(anyhow!(
            "research domain workspace state exceeds size limit"
        ));
    }
    let path = state_path(workspace_root, domain_id)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, encoded)?;
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn workspace_state_round_trips_agent_and_ui_fields() {
        let root = tempdir().unwrap();
        let state = update_workspace_state(
            root.path(),
            "ai-ml",
            &json!({"active_tab":"visualization","focus":"checkpoint-42","parameters":{"seed":42}}),
            "agent",
        )
        .unwrap();
        assert_eq!(state["active_tab"], "visualization");
        assert_eq!(state["parameters"]["seed"], 42);
        let loaded = read_workspace_state(root.path(), "ai-ml").unwrap();
        assert_eq!(loaded["focus"], "checkpoint-42");
        assert_eq!(loaded["updated_by"], "agent");
    }
}
