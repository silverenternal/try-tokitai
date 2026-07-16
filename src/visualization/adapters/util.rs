use crate::visualization::model::VisualizationSource;
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

const MAX_SOURCE_BYTES: u64 = 16 * 1024 * 1024;

pub fn workspace_files<F>(root: &Path, mut accept: F, limit: usize) -> Vec<PathBuf>
where
    F: FnMut(&Path) -> bool,
{
    if !root.exists() {
        return Vec::new();
    }
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(include_entry)
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| {
            fs::metadata(path)
                .map(|meta| meta.len() <= MAX_SOURCE_BYTES)
                .unwrap_or(false)
        })
        .filter(|path| accept(path))
        .take(limit)
        .collect()
}

fn include_entry(entry: &DirEntry) -> bool {
    if entry.depth() == 0 {
        return true;
    }
    let name = entry.file_name().to_string_lossy();
    !(entry.file_type().is_dir() && name.starts_with('.'))
        && !name.starts_with("target")
        && !matches!(
            name.as_ref(),
            ".git"
                | "node_modules"
                | "target"
                | "dist"
                | "build"
                | "vendor"
                | ".cache"
                | ".venv"
                | "venv"
                | "__pycache__"
        )
}

pub fn source_for_path(
    root: &Path,
    path: &Path,
    kind: &str,
    source_type: &str,
) -> VisualizationSource {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let display = relative.to_string_lossy().replace('\\', "/");
    let mut metadata = BTreeMap::new();
    metadata.insert("path".to_string(), Value::String(display.clone()));
    if let Ok(meta) = fs::metadata(path) {
        metadata.insert("bytes".to_string(), Value::from(meta.len()));
    }
    VisualizationSource {
        id: format!("workspace:{display}"),
        kind: kind.to_string(),
        label: display,
        source_type: source_type.to_string(),
        live: false,
        metadata,
    }
}

pub fn live_source(id: &str, kind: &str, label: &str, source_type: &str) -> VisualizationSource {
    VisualizationSource {
        id: id.to_string(),
        kind: kind.to_string(),
        label: label.to_string(),
        source_type: source_type.to_string(),
        live: true,
        metadata: BTreeMap::new(),
    }
}

pub fn selected_path(root: &Path, source_id: Option<&str>) -> Result<PathBuf> {
    let relative = source_id
        .and_then(|value| value.strip_prefix("workspace:"))
        .ok_or_else(|| anyhow!("a workspace visualization source is required"))?;
    let path = root.join(relative);
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let canonical_path = path
        .canonicalize()
        .map_err(|error| anyhow!("visualization source is unavailable: {error}"))?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(anyhow!("visualization source is outside the workspace"));
    }
    Ok(canonical_path)
}

pub fn read_source(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    if bytes.len() > MAX_SOURCE_BYTES as usize {
        return Err(anyhow!("visualization source exceeds 16 MiB"));
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

pub fn stable_id(prefix: &str, value: &str) -> String {
    let normalized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let mut compact = normalized
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    compact.truncate(72);
    format!(
        "{prefix}:{}",
        if compact.is_empty() { "item" } else { &compact }
    )
}

pub fn ext(path: &Path) -> String {
    path.extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn number(value: Option<&Value>) -> Option<f64> {
    value.and_then(|value| value.as_f64().or_else(|| value.as_str()?.parse().ok()))
}
