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
        if let Ok(explicit) = std::env::var("TOKITAI_PROJECT_ROOT") {
            let candidate = PathBuf::from(explicit);
            if candidate.join("Cargo.toml").exists() || candidate.join("frontend").join("index.html").exists() {
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
        let state_root = dirs::data_local_dir()?.join("Tokitai");
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
        workspace.join(".tokitai")
    }

    pub fn workspace_run_debug_dir(&self, workspace: &Path) -> PathBuf {
        self.workspace_state_dir(workspace).join("run-debug")
    }
}

fn local_dev_state_dir(base_dir: &Path) -> PathBuf {
    let state_root = dirs::data_local_dir()
        .unwrap_or_else(|| base_dir.to_path_buf())
        .join("Tokitai")
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
        if candidate.join("Cargo.toml").exists() && candidate.join("frontend").join("index.html").exists() {
            return Some(candidate.to_path_buf());
        }
    }
    None
}
