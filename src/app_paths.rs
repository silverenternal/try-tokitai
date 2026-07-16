use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AppPaths {
    base_dir: PathBuf,
    frontend_dir: PathBuf,
    state_dir: PathBuf,
}

impl AppPaths {
    pub fn new(base_dir: PathBuf, frontend_dir: PathBuf, state_dir: PathBuf) -> Self {
        Self {
            base_dir,
            frontend_dir,
            state_dir,
        }
    }

    pub fn for_local_dev(base_dir: PathBuf) -> Self {
        let frontend_dir = base_dir.join("frontend");
        let state_dir = local_dev_state_dir(&base_dir);
        Self::new(base_dir, frontend_dir, state_dir)
    }

    pub fn discover_project_root() -> PathBuf {
        if let Ok(explicit) =
            std::env::var("ATLAS_PROJECT_ROOT").or_else(|_| std::env::var("ATLAS_PROJECT_ROOT"))
        {
            let candidate = PathBuf::from(explicit);
            if candidate.join("Cargo.toml").exists()
                || candidate.join("frontend").join("index.html").exists()
            {
                return candidate;
            }
        }

        if let Ok(cwd) = std::env::current_dir() {
            if let Some(found) = find_project_root_from(&cwd) {
                return found;
            }
        }

        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                if let Some(found) = find_project_root_from(parent) {
                    return found;
                }
            }
        }

        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }

    pub fn for_desktop(base_dir: PathBuf, frontend_dir: PathBuf, state_dir: PathBuf) -> Self {
        Self::new(base_dir, frontend_dir, state_dir)
    }

    pub fn for_desktop_defaults() -> Option<Self> {
        let state_root = dirs::data_local_dir()?.join("Atlas");
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|value| value.to_path_buf()))?;
        let exe_frontend_dir = exe_dir.join("frontend");
        let frontend_dir = if exe_frontend_dir.join("index.html").exists() {
            exe_frontend_dir
        } else {
            let cwd = std::env::current_dir().ok()?;
            let cwd_frontend_dir = cwd.join("frontend");
            if cwd_frontend_dir.join("index.html").exists() {
                cwd_frontend_dir
            } else {
                exe_frontend_dir
            }
        };
        Some(Self::for_desktop(exe_dir, frontend_dir, state_root))
    }

    pub fn for_desktop_project(workspace: &Path) -> Option<Self> {
        let mut paths = Self::for_desktop_defaults()?;
        let legacy_state_dir = paths.state_dir.clone();
        paths.state_dir = legacy_state_dir
            .join("projects")
            .join(project_id(workspace));
        migrate_matching_legacy_sessions(&legacy_state_dir, &paths.state_dir, workspace);
        migrate_matching_project_sessions(&legacy_state_dir, &paths.state_dir, workspace);
        Some(paths)
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn frontend_dir(&self) -> &Path {
        &self.frontend_dir
    }

    pub fn state_dir(&self) -> &Path {
        &self.state_dir
    }

    pub fn web_runtime_state_path(&self) -> PathBuf {
        self.state_dir.join("web-runtime.json")
    }

    pub fn tui_log_path(&self) -> PathBuf {
        self.state_dir.join("tui.log")
    }

    pub fn sessions_dir(&self) -> PathBuf {
        self.state_dir.join("sessions")
    }

    pub fn sandbox_dir(&self) -> PathBuf {
        self.state_dir.join("sandbox")
    }

    pub fn downloads_dir(&self) -> PathBuf {
        self.state_dir.join("downloads")
    }

    pub fn sandbox_manifest_path(&self) -> PathBuf {
        self.state_dir.join("sandbox-bootstrap.json")
    }

    pub fn workspace_state_dir(&self, workspace: &Path) -> PathBuf {
        workspace.join(".atlas")
    }

    pub fn workspace_run_debug_dir(&self, workspace: &Path) -> PathBuf {
        self.workspace_state_dir(workspace).join("run-debug")
    }
}

fn migrate_matching_legacy_sessions(
    legacy_state_dir: &Path,
    project_state_dir: &Path,
    workspace: &Path,
) {
    let runtime_path = legacy_state_dir.join("web-runtime.json");
    let Ok(runtime_text) = std::fs::read_to_string(runtime_path) else {
        return;
    };
    let Ok(runtime) = serde_json::from_str::<serde_json::Value>(&runtime_text) else {
        return;
    };
    let Some(legacy_workspace) = runtime
        .get("workspace_root")
        .and_then(|value| value.as_str())
    else {
        return;
    };
    if project_id(Path::new(legacy_workspace)) != project_id(workspace) {
        return;
    }

    merge_session_dirs(
        &legacy_state_dir.join("sessions"),
        &project_state_dir.join("sessions"),
    );
}

fn migrate_matching_project_sessions(
    state_root: &Path,
    project_state_dir: &Path,
    workspace: &Path,
) {
    let projects_dir = state_root.join("projects");
    let Ok(entries) = std::fs::read_dir(projects_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let source_state = entry.path();
        if source_state == project_state_dir || !source_state.is_dir() {
            continue;
        }
        let Ok(runtime_text) = std::fs::read_to_string(source_state.join("web-runtime.json"))
        else {
            continue;
        };
        let Ok(runtime) = serde_json::from_str::<serde_json::Value>(&runtime_text) else {
            continue;
        };
        let Some(source_workspace) = runtime
            .get("workspace_root")
            .and_then(|value| value.as_str())
        else {
            continue;
        };
        if project_id(Path::new(source_workspace)) == project_id(workspace) {
            merge_session_dirs(
                &source_state.join("sessions"),
                &project_state_dir.join("sessions"),
            );
        }
    }
}

fn merge_session_dirs(source: &Path, target: &Path) {
    let Ok(entries) = std::fs::read_dir(&source) else {
        return;
    };
    let _ = std::fs::create_dir_all(&target);
    for entry in entries.flatten() {
        let source_path = entry.path();
        if !source_path.is_file()
            || source_path.file_name().and_then(|name| name.to_str()) == Some("index.json")
        {
            continue;
        }
        let target_path = target.join(entry.file_name());
        if !target_path.exists() {
            let _ = std::fs::copy(source_path, target_path);
        }
    }

    let load_index = |path: &Path| -> Vec<serde_json::Value> {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    };
    let target_index_path = target.join("index.json");
    let mut merged = load_index(&target_index_path);
    for item in load_index(&source.join("index.json")) {
        let id = item.get("id").and_then(|value| value.as_str());
        if id.is_some()
            && !merged
                .iter()
                .any(|existing| existing.get("id").and_then(|value| value.as_str()) == id)
        {
            merged.push(item);
        }
    }
    merged.sort_by(|a, b| {
        b.get("updated_at")
            .and_then(|v| v.as_str())
            .cmp(&a.get("updated_at").and_then(|v| v.as_str()))
    });
    if let Ok(json) = serde_json::to_string_pretty(&merged) {
        let _ = std::fs::write(target_index_path, json);
    }
}

pub fn project_id(workspace: &Path) -> String {
    let normalized = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    let identity = normalized
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("//?/")
        .trim_end_matches('/')
        .to_string();
    // Windows paths are case-insensitive; Linux and most macOS development volumes are not.
    // Lowercasing everywhere caused distinct Unix workspaces to share sessions and indexes.
    #[cfg(windows)]
    let identity = identity.to_lowercase();
    let digest = blake3::hash(identity.as_bytes()).to_hex().to_string();
    digest[..16].to_string()
}

fn local_dev_state_dir(base_dir: &Path) -> PathBuf {
    let state_root = dirs::data_local_dir()
        .unwrap_or_else(|| base_dir.to_path_buf())
        .join("Atlas")
        .join("dev");
    let canonical = base_dir
        .canonicalize()
        .unwrap_or_else(|_| base_dir.to_path_buf());
    let digest = blake3::hash(canonical.to_string_lossy().as_bytes())
        .to_hex()
        .to_string();
    state_root.join(&digest[..12])
}

fn find_project_root_from(start: &Path) -> Option<PathBuf> {
    for candidate in start.ancestors() {
        if candidate.join("Cargo.toml").exists()
            && candidate.join("frontend").join("index.html").exists()
        {
            return Some(candidate.to_path_buf());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::project_id;
    #[test]
    fn project_identity_is_stable_and_path_specific() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        assert_eq!(project_id(first.path()), project_id(first.path()));
        assert_ne!(project_id(first.path()), project_id(second.path()));
    }
}
