//! Shared infrastructure for the Atlas scientific event center, universal search navigation,
//! and complete workspace snapshots.

use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::atlas_core::{AtlasCore, AtlasEvent, AtlasEventKind, ScientificObject};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NavigationTarget {
    pub workspace: Option<String>,
    pub object_id: Option<String>,
    pub runtime_id: Option<String>,
    pub environment_id: Option<String>,
    pub artifact_path: Option<String>,
    pub visualization_kind: Option<String>,
    pub highlight: Option<String>,
    #[serde(default)]
    pub restore_context: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSeverity {
    Info,
    Success,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventUserState {
    pub read: bool,
    pub pinned: bool,
    pub archived: bool,
    #[serde(default)]
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScientificNotification {
    pub id: String,
    pub event_id: String,
    pub title: String,
    pub summary: String,
    pub severity: EventSeverity,
    pub category: String,
    pub timestamp: String,
    pub related_object_ids: Vec<String>,
    pub workspace: String,
    pub runtime_id: Option<String>,
    pub actions: Vec<NotificationAction>,
    pub navigation: NavigationTarget,
    pub read: bool,
    pub pinned: bool,
    pub archived: bool,
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAction {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub target: NavigationTarget,
}

#[derive(Debug)]
pub struct ScientificEventCenter {
    workspace: PathBuf,
    state_path: PathBuf,
}

impl ScientificEventCenter {
    pub fn open(workspace: &Path) -> Result<Self> {
        let dir = workspace.join(".atlas").join("event-center");
        fs::create_dir_all(&dir)?;
        Ok(Self {
            workspace: workspace.to_path_buf(),
            state_path: dir.join("state.json"),
        })
    }
    pub fn list(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        include_archived: bool,
    ) -> Result<Vec<ScientificNotification>> {
        let core = AtlasCore::open(&self.workspace)?;
        let states = self.load_state();
        let objects: BTreeMap<_, _> = core
            .list()?
            .into_iter()
            .map(|o| (o.id.clone(), o))
            .collect();
        let query = query.unwrap_or("").trim().to_ascii_lowercase();
        let mut result = core
            .timeline(None)?
            .into_iter()
            .map(|event| {
                let state = states.get(&event.id).cloned().unwrap_or_default();
                project_event(&self.workspace, event, &objects, state)
            })
            .filter(|n| include_archived || !n.archived)
            .filter(|n| {
                category
                    .map(|c| c == "all" || n.category == c)
                    .unwrap_or(true)
            })
            .filter(|n| {
                query.is_empty()
                    || format!("{} {} {}", n.title, n.summary, n.category)
                        .to_ascii_lowercase()
                        .contains(&query)
            })
            .collect::<Vec<_>>();
        result.sort_by(|a, b| {
            b.pinned
                .cmp(&a.pinned)
                .then_with(|| b.priority.cmp(&a.priority))
                .then_with(|| b.timestamp.cmp(&a.timestamp))
        });
        Ok(result)
    }
    pub fn mutate(
        &self,
        id: &str,
        operation: &str,
        priority: Option<i32>,
    ) -> Result<EventUserState> {
        validate_id(id)?;
        let mut all = self.load_state();
        let state = all.entry(id.into()).or_default();
        match operation {
            "read" => state.read = true,
            "unread" => state.read = false,
            "pin" => state.pinned = true,
            "unpin" => state.pinned = false,
            "archive" => state.archived = true,
            "restore" => state.archived = false,
            "priority" => state.priority = priority.unwrap_or(0).clamp(-100, 100),
            _ => return Err(anyhow!("unknown event center operation")),
        };
        let result = state.clone();
        atomic_json(&self.state_path, &all)?;
        Ok(result)
    }
    fn load_state(&self) -> BTreeMap<String, EventUserState> {
        fs::read(&self.state_path)
            .ok()
            .and_then(|v| serde_json::from_slice(&v).ok())
            .unwrap_or_default()
    }
}

fn project_event(
    workspace: &Path,
    event: AtlasEvent,
    objects: &BTreeMap<String, ScientificObject>,
    state: EventUserState,
) -> ScientificNotification {
    let primary = event.object_ids.first().and_then(|id| objects.get(id));
    let kind = format!("{:?}", event.kind).to_ascii_lowercase();
    let category = event
        .data
        .get("category")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| infer_category(&event, primary));
    let severity = event
        .data
        .get("severity")
        .and_then(Value::as_str)
        .map(parse_severity)
        .unwrap_or_else(|| {
            if kind.contains("failed") || kind.contains("deleted") {
                EventSeverity::Error
            } else if kind.contains("finished") || kind.contains("created") {
                EventSeverity::Success
            } else {
                EventSeverity::Info
            }
        });
    let title = event
        .data
        .get("title")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| event_title(&event, primary));
    let summary = event
        .data
        .get("summary")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| primary.map(|o| o.description.clone()))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| format!("Atlas event from {}", event.actor));
    let navigation = NavigationTarget {
        workspace: Some(workspace.to_string_lossy().replace('\\', "/")),
        object_id: event.object_ids.first().cloned(),
        runtime_id: event
            .data
            .get("runtime_id")
            .and_then(Value::as_str)
            .map(str::to_string),
        environment_id: event
            .data
            .get("environment_id")
            .and_then(Value::as_str)
            .map(str::to_string),
        artifact_path: event
            .data
            .get("artifact_path")
            .and_then(Value::as_str)
            .map(str::to_string),
        visualization_kind: event
            .data
            .get("visualization_kind")
            .and_then(Value::as_str)
            .map(str::to_string),
        highlight: event
            .data
            .get("highlight")
            .and_then(Value::as_str)
            .map(str::to_string),
        restore_context: event
            .data
            .get("restore_context")
            .cloned()
            .unwrap_or(Value::Null),
    };
    let mut actions = vec![NotificationAction {
        id: "open-object".into(),
        label: "Open Object".into(),
        kind: "navigate".into(),
        target: navigation.clone(),
    }];
    if navigation.visualization_kind.is_some() {
        actions.push(NotificationAction {
            id: "open-visualization".into(),
            label: "Open Visualization".into(),
            kind: "visualize".into(),
            target: navigation.clone(),
        });
    }
    if category == "experiment" {
        actions.push(NotificationAction {
            id: "compare-checkpoints".into(),
            label: "Compare Checkpoints".into(),
            kind: "compare".into(),
            target: navigation.clone(),
        });
        actions.push(NotificationAction {
            id: "export-results".into(),
            label: "Export Results".into(),
            kind: "export".into(),
            target: navigation.clone(),
        });
    }
    ScientificNotification {
        id: event.id.clone(),
        event_id: event.id,
        title,
        summary,
        severity,
        category,
        timestamp: event.timestamp,
        related_object_ids: event.object_ids,
        workspace: workspace.to_string_lossy().replace('\\', "/"),
        runtime_id: navigation.runtime_id.clone(),
        actions,
        navigation,
        read: state.read,
        pinned: state.pinned,
        archived: state.archived,
        priority: state.priority,
    }
}
fn infer_category(event: &AtlasEvent, object: Option<&ScientificObject>) -> String {
    let text = format!(
        "{:?} {}",
        event.kind,
        object.map(|o| o.object_type.0.as_str()).unwrap_or("")
    );
    for c in [
        "experiment",
        "runtime",
        "workspace",
        "knowledge",
        "dataset",
        "paper",
        "simulation",
        "visualization",
        "security",
        "remote-ssh",
        "plugin",
        "execution",
    ] {
        if text.to_ascii_lowercase().contains(c) {
            return c.into();
        }
    }
    "system".into()
}
fn event_title(event: &AtlasEvent, object: Option<&ScientificObject>) -> String {
    let subject = object
        .map(|o| o.display_name.clone())
        .unwrap_or_else(|| "Atlas".into());
    format!(
        "{} · {}",
        subject,
        format!("{:?}", event.kind)
            .replace("Custom(\"", "")
            .replace("\")", "")
    )
}
fn parse_severity(v: &str) -> EventSeverity {
    match v {
        "success" => EventSeverity::Success,
        "warning" => EventSeverity::Warning,
        "error" => EventSeverity::Error,
        "critical" => EventSeverity::Critical,
        _ => EventSeverity::Info,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceSnapshotType {
    Manual,
    Auto,
    Milestone,
    Experiment,
    BeforeAgent,
    BeforeExecution,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
    pub id: String,
    pub name: String,
    pub snapshot_type: WorkspaceSnapshotType,
    pub created_at: String,
    pub parent_id: Option<String>,
    pub archived: bool,
    pub workspace: String,
    pub state: Value,
    pub object_versions: BTreeMap<String, u64>,
    pub tags: BTreeSet<String>,
}
#[derive(Debug)]
pub struct WorkspaceTimeMachine {
    workspace: PathBuf,
    dir: PathBuf,
}
impl WorkspaceTimeMachine {
    pub fn open(workspace: &Path) -> Result<Self> {
        let dir = workspace.join(".atlas").join("snapshots");
        fs::create_dir_all(&dir)?;
        Ok(Self {
            workspace: workspace.into(),
            dir,
        })
    }
    pub fn list(&self) -> Result<Vec<WorkspaceSnapshot>> {
        let mut v: Vec<WorkspaceSnapshot> = read_json_dir(&self.dir)?;
        v.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(v)
    }
    pub fn create(
        &self,
        name: &str,
        snapshot_type: WorkspaceSnapshotType,
        state: Value,
        parent_id: Option<String>,
        tags: BTreeSet<String>,
    ) -> Result<WorkspaceSnapshot> {
        let core = AtlasCore::open(&self.workspace)?;
        let versions = core
            .list()?
            .into_iter()
            .map(|o| (o.id, o.version))
            .collect();
        let s = WorkspaceSnapshot {
            id: Uuid::new_v4().to_string(),
            name: name.trim().chars().take(120).collect(),
            snapshot_type,
            created_at: Utc::now().to_rfc3339(),
            parent_id,
            archived: false,
            workspace: self.workspace.to_string_lossy().replace('\\', "/"),
            state,
            object_versions: versions,
            tags,
        };
        atomic_json(&self.dir.join(format!("{}.json", s.id)), &s)?;
        let mut e = AtlasEvent::new(
            AtlasEventKind::Custom("workspace_snapshot_created".into()),
            "workspace-time-machine",
            vec![],
        );
        e.data = json!({"title":format!("Snapshot created · {}",s.name),"summary":"Complete scientific workspace state captured","category":"workspace","snapshot_id":s.id,"severity":"success"});
        core.record_event(e)?;
        Ok(s)
    }
    pub fn get(&self, id: &str) -> Result<WorkspaceSnapshot> {
        validate_id(id)?;
        Ok(serde_json::from_slice(&fs::read(
            self.dir.join(format!("{id}.json")),
        )?)?)
    }
    pub fn rename(&self, id: &str, name: &str) -> Result<WorkspaceSnapshot> {
        let mut s = self.get(id)?;
        s.name = name.trim().chars().take(120).collect();
        atomic_json(&self.dir.join(format!("{id}.json")), &s)?;
        Ok(s)
    }
    pub fn archive(&self, id: &str, value: bool) -> Result<WorkspaceSnapshot> {
        let mut s = self.get(id)?;
        s.archived = value;
        atomic_json(&self.dir.join(format!("{id}.json")), &s)?;
        Ok(s)
    }
    pub fn fork(&self, id: &str, name: Option<&str>) -> Result<WorkspaceSnapshot> {
        let source = self.get(id)?;
        self.create(
            name.unwrap_or(&format!("{} fork", source.name)),
            WorkspaceSnapshotType::Manual,
            source.state,
            Some(source.id),
            source.tags,
        )
    }
    pub fn diff(&self, left: &str, right: &str) -> Result<Value> {
        let a = self.get(left)?;
        let b = self.get(right)?;
        Ok(
            json!({"left":a.id,"right":b.id,"state":diff_values(&a.state,&b.state),"objects":{"added":b.object_versions.keys().filter(|k|!a.object_versions.contains_key(*k)).collect::<Vec<_>>(),"removed":a.object_versions.keys().filter(|k|!b.object_versions.contains_key(*k)).collect::<Vec<_>>(),"changed":b.object_versions.iter().filter(|(k,v)|a.object_versions.get(*k).is_some_and(|old|old!=*v)).collect::<BTreeMap<_,_>>()}}),
        )
    }
    pub fn record_restore(&self, s: &WorkspaceSnapshot) -> Result<()> {
        let core = AtlasCore::open(&self.workspace)?;
        let mut e = AtlasEvent::new(
            AtlasEventKind::WorkspaceOpened,
            "workspace-time-machine",
            s.object_versions.keys().cloned().collect(),
        );
        e.data = json!({"title":format!("Workspace restored · {}",s.name),"summary":"Scientific runtime context restored from snapshot","category":"workspace","snapshot_id":s.id,"severity":"success","restore_context":s.state});
        core.record_event(e)
    }
    pub fn restore_object_versions(&self, s: &WorkspaceSnapshot) -> Result<Value> {
        let core = AtlasCore::open(&self.workspace)?;
        let mut restored = Vec::new();
        let mut unchanged = Vec::new();
        let mut missing = Vec::new();
        for (id, version) in &s.object_versions {
            match core.get(id) {
                Ok(current) if current.version == *version => unchanged.push(id.clone()),
                Ok(_) => match core.rollback(id, *version, "workspace-time-machine") {
                    Ok(object) => restored.push(
                        json!({"id":id,"snapshot_version":version,"new_version":object.version}),
                    ),
                    Err(error) => {
                        missing.push(json!({"id":id,"version":version,"error":error.to_string()}))
                    }
                },
                Err(error) => {
                    missing.push(json!({"id":id,"version":version,"error":error.to_string()}))
                }
            }
        }
        Ok(json!({"restored":restored,"unchanged":unchanged,"missing":missing}))
    }
}
fn diff_values(a: &Value, b: &Value) -> Value {
    if a == b {
        return json!({});
    }
    match (a, b) {
        (Value::Object(am), Value::Object(bm)) => {
            let keys = am.keys().chain(bm.keys()).collect::<BTreeSet<_>>();
            Value::Object(
                keys.into_iter()
                    .filter_map(|k| {
                        let d = diff_values(
                            am.get(k).unwrap_or(&Value::Null),
                            bm.get(k).unwrap_or(&Value::Null),
                        );
                        (d != json!({})).then(|| (k.clone(), d))
                    })
                    .collect(),
            )
        }
        _ => json!({"before":a,"after":b}),
    }
}
fn validate_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_".contains(c))
    {
        return Err(anyhow!("invalid infrastructure id"));
    }
    Ok(())
}
fn atomic_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let temp = path.with_extension("tmp");
    fs::write(&temp, serde_json::to_vec_pretty(value)?)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(temp, path)?;
    Ok(())
}
fn read_json_dir<T: for<'de> Deserialize<'de>>(dir: &Path) -> Result<Vec<T>> {
    let mut out = vec![];
    for e in fs::read_dir(dir)? {
        let p = e?.path();
        if p.extension().and_then(|v| v.to_str()) == Some("json") {
            out.push(serde_json::from_slice(&fs::read(p)?)?);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    #[test]
    fn snapshot_roundtrip_and_diff() {
        let d = tempdir().unwrap();
        let tm = WorkspaceTimeMachine::open(d.path()).unwrap();
        let a = tm
            .create(
                "a",
                WorkspaceSnapshotType::Manual,
                json!({"editor":{"file":"a"}}),
                None,
                BTreeSet::new(),
            )
            .unwrap();
        let b = tm
            .create(
                "b",
                WorkspaceSnapshotType::Auto,
                json!({"editor":{"file":"b"}}),
                Some(a.id.clone()),
                BTreeSet::new(),
            )
            .unwrap();
        assert!(tm.diff(&a.id, &b.id).unwrap()["state"]["editor"]["file"].is_object());
    }
}
