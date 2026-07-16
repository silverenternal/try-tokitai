//! Core object graph identity and persistence utilities

use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const MAX_OBJECT_BYTES: usize = 512 * 1024;
const MAX_OBJECTS_PER_TYPE: usize = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ResearchObjectId {
    pub object_type: String,
    pub id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResearchObjectType {
    Hypothesis,
    Evidence,
    Experiment,
    NegativeResult,
    DiaryEntry,
    KnowledgeGraphNode,
    KnowledgeGraphEdge,
    Decision,
    Memory,
    Timeline,
    Publication,
}

impl ResearchObjectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Hypothesis => "hypothesis",
            Self::Evidence => "evidence",
            Self::Experiment => "experiment",
            Self::NegativeResult => "negative-result",
            Self::DiaryEntry => "diary",
            Self::KnowledgeGraphNode => "kg-node",
            Self::KnowledgeGraphEdge => "kg-edge",
            Self::Decision => "decision",
            Self::Memory => "memory",
            Self::Timeline => "timeline",
            Self::Publication => "publication",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "hypothesis" => Some(Self::Hypothesis),
            "evidence" => Some(Self::Evidence),
            "experiment" => Some(Self::Experiment),
            "negative-result" => Some(Self::NegativeResult),
            "diary" => Some(Self::DiaryEntry),
            "kg-node" => Some(Self::KnowledgeGraphNode),
            "kg-edge" => Some(Self::KnowledgeGraphEdge),
            "decision" => Some(Self::Decision),
            "memory" => Some(Self::Memory),
            "timeline" => Some(Self::Timeline),
            "publication" => Some(Self::Publication),
            _ => None,
        }
    }
}

/// Get storage path for Research OS objects: .atlas/research-os/{type}/{id}.json
pub fn research_os_path(workspace_root: &Path, object_type: ResearchObjectType) -> PathBuf {
    workspace_root
        .join(".atlas")
        .join("research-os")
        .join(object_type.as_str())
}

/// Generate unique ID for a research object
pub fn generate_object_id(content: &str) -> String {
    let timestamp = Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let input = format!("{}{}", content, timestamp);
    blake3::hash(input.as_bytes()).to_hex().to_string()
}

/// Write a research object to disk
pub fn write_research_object<T: Serialize>(
    workspace_root: &Path,
    object_type: ResearchObjectType,
    id: &str,
    object: &T,
) -> Result<PathBuf> {
    let dir = research_os_path(workspace_root, object_type);
    fs::create_dir_all(&dir)?;

    let path = dir.join(format!("{}.json", id));
    let json = serde_json::to_vec_pretty(object)?;

    if json.len() > MAX_OBJECT_BYTES {
        return Err(anyhow!(
            "research object exceeds size limit ({} > {})",
            json.len(),
            MAX_OBJECT_BYTES
        ));
    }

    fs::write(&path, json)?;
    Ok(path)
}

/// Read a research object from disk
pub fn read_research_object<T: for<'de> Deserialize<'de>>(
    workspace_root: &Path,
    object_type: ResearchObjectType,
    id: &str,
) -> Result<T> {
    let path = research_os_path(workspace_root, object_type).join(format!("{}.json", id));

    if !path.exists() {
        return Err(anyhow!(
            "research object not found: {}:{}",
            object_type.as_str(),
            id
        ));
    }

    let bytes = fs::read(&path)?;
    if bytes.len() > MAX_OBJECT_BYTES {
        return Err(anyhow!("research object exceeds size limit"));
    }

    Ok(serde_json::from_slice(&bytes)?)
}

/// Check whether a research object of the given type exists on disk without deserializing it.
pub fn object_exists(workspace_root: &Path, object_type: ResearchObjectType, id: &str) -> bool {
    research_os_path(workspace_root, object_type)
        .join(format!("{}.json", id))
        .exists()
}

/// List all objects of a given type
pub fn list_research_objects(
    workspace_root: &Path,
    object_type: ResearchObjectType,
) -> Result<Vec<String>> {
    let dir = research_os_path(workspace_root, object_type);

    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut ids = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                ids.push(stem.to_string());
            }
        }

        if ids.len() >= MAX_OBJECTS_PER_TYPE {
            break;
        }
    }

    Ok(ids)
}

/// Delete a research object
pub fn delete_research_object(
    workspace_root: &Path,
    object_type: ResearchObjectType,
    id: &str,
) -> Result<()> {
    let path = research_os_path(workspace_root, object_type).join(format!("{}.json", id));
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn generates_unique_ids() {
        let id1 = generate_object_id("test content");
        let id2 = generate_object_id("test content");
        assert_ne!(id1, id2); // Different due to timestamp
    }

    #[test]
    fn writes_and_reads_research_object() {
        let dir = tempdir().unwrap();
        let data = json!({"title": "Test Hypothesis", "status": "draft"});

        let path =
            write_research_object(dir.path(), ResearchObjectType::Hypothesis, "test-id", &data)
                .unwrap();

        assert!(path.exists());

        let loaded: serde_json::Value =
            read_research_object(dir.path(), ResearchObjectType::Hypothesis, "test-id").unwrap();

        assert_eq!(loaded["title"], "Test Hypothesis");
    }

    #[test]
    fn lists_research_objects() {
        let dir = tempdir().unwrap();
        let data = json!({"test": true});

        write_research_object(dir.path(), ResearchObjectType::Evidence, "id1", &data).unwrap();
        write_research_object(dir.path(), ResearchObjectType::Evidence, "id2", &data).unwrap();

        let ids = list_research_objects(dir.path(), ResearchObjectType::Evidence).unwrap();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"id1".to_string()));
        assert!(ids.contains(&"id2".to_string()));
    }
}
