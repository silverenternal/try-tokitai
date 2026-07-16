use crate::process_window::CommandWindowExt;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    Interrupted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundTask {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub command: String,
    #[serde(default)]
    pub program: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: String,
    pub status: TaskStatus,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub pid: Option<u32>,
    pub exit_code: Option<i32>,
    pub attempts: u32,
    pub log_path: String,
    pub recovery: String,
    #[serde(default)]
    pub result_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct QueueFile {
    tasks: BTreeMap<String, BackgroundTask>,
}

#[derive(Clone)]
pub struct TaskQueue {
    workspace: PathBuf,
    path: PathBuf,
    tasks: Arc<Mutex<BTreeMap<String, BackgroundTask>>>,
}

impl TaskQueue {
    pub fn open(workspace: &Path) -> Result<Self> {
        let workspace = workspace.canonicalize()?;
        let dir = workspace.join(".atlas").join("tasks");
        fs::create_dir_all(&dir)?;
        let path = dir.join("queue.json");
        let mut file: QueueFile = if path.exists() {
            serde_json::from_slice(&fs::read(&path)?)?
        } else {
            QueueFile::default()
        };
        for task in file.tasks.values_mut() {
            if task.status == TaskStatus::Running && !task.pid.is_some_and(process_is_running) {
                task.status = TaskStatus::Interrupted;
                task.completed_at = Some(now());
                task.recovery = "awaiting_user_resume".into();
                task.pid = None;
            }
        }
        let queue = Self {
            workspace,
            path,
            tasks: Arc::new(Mutex::new(file.tasks)),
        };
        queue.persist()?;
        Ok(queue)
    }

    pub fn list(&self) -> Vec<BackgroundTask> {
        let mut values: Vec<_> = self.tasks.lock().unwrap().values().cloned().collect();
        values.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        values
    }

    pub fn get(&self, id: &str) -> Option<BackgroundTask> {
        self.tasks.lock().ok()?.get(id).cloned()
    }

    pub fn enqueue(
        &self,
        title: &str,
        kind: &str,
        command: &str,
        cwd: Option<&str>,
        start: bool,
    ) -> Result<BackgroundTask> {
        if command.trim().is_empty() {
            return Err(anyhow!("task command is empty"));
        }
        let cwd = self.resolve_cwd(cwd)?;
        let id = Uuid::new_v4().to_string();
        let log_abs = self
            .workspace
            .join(".atlas")
            .join("tasks")
            .join(format!("{}.log", id));
        let task = BackgroundTask {
            id: id.clone(),
            title: title.trim().to_string(),
            kind: kind.trim().to_string(),
            command: command.trim().to_string(),
            program: None,
            args: Vec::new(),
            cwd: cwd
                .strip_prefix(&self.workspace)
                .unwrap_or(Path::new("."))
                .to_string_lossy()
                .replace('\\', "/"),
            status: TaskStatus::Queued,
            created_at: now(),
            started_at: None,
            completed_at: None,
            pid: None,
            exit_code: None,
            attempts: 0,
            log_path: log_abs
                .strip_prefix(&self.workspace)?
                .to_string_lossy()
                .replace('\\', "/"),
            recovery: "manual_resume".into(),
            result_path: None,
        };
        self.tasks.lock().unwrap().insert(id.clone(), task.clone());
        self.persist()?;
        if start {
            self.start(&id)
        } else {
            Ok(task)
        }
    }

    pub fn enqueue_process(
        &self,
        title: &str,
        kind: &str,
        program: &str,
        args: &[String],
        cwd: Option<&str>,
        start: bool,
        result_path: Option<&str>,
    ) -> Result<BackgroundTask> {
        if program.trim().is_empty() {
            return Err(anyhow!("task program is empty"));
        }
        let resolved_program = which::which(program)
            .with_context(|| format!("task program is unavailable: {program}"))?;
        let cwd = self.resolve_cwd(cwd)?;
        let result_path = result_path
            .map(|value| self.resolve_result_path(value))
            .transpose()?
            .map(|path| {
                path.strip_prefix(&self.workspace)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/")
            });
        let id = Uuid::new_v4().to_string();
        let log_abs = self
            .workspace
            .join(".atlas")
            .join("tasks")
            .join(format!("{}.log", id));
        let display_command = std::iter::once(resolved_program.to_string_lossy().to_string())
            .chain(args.iter().map(|value| display_arg(value)))
            .collect::<Vec<_>>()
            .join(" ");
        let task = BackgroundTask {
            id: id.clone(),
            title: title.trim().to_string(),
            kind: kind.trim().to_string(),
            command: display_command,
            program: Some(resolved_program.to_string_lossy().to_string()),
            args: args.to_vec(),
            cwd: cwd
                .strip_prefix(&self.workspace)
                .unwrap_or(Path::new("."))
                .to_string_lossy()
                .replace('\\', "/"),
            status: TaskStatus::Queued,
            created_at: now(),
            started_at: None,
            completed_at: None,
            pid: None,
            exit_code: None,
            attempts: 0,
            log_path: log_abs
                .strip_prefix(&self.workspace)?
                .to_string_lossy()
                .replace('\\', "/"),
            recovery: "manual_resume".into(),
            result_path,
        };
        self.tasks.lock().unwrap().insert(id.clone(), task.clone());
        self.persist()?;
        if start {
            self.start(&id)
        } else {
            Ok(task)
        }
    }

    pub fn start(&self, id: &str) -> Result<BackgroundTask> {
        let snapshot = self
            .tasks
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown task"))?;
        if snapshot.status == TaskStatus::Running {
            return Ok(snapshot);
        }
        let cwd = self.resolve_cwd(Some(&snapshot.cwd))?;
        let log_path = self.workspace.join(&snapshot.log_path);
        let stdout = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;
        let stderr = stdout.try_clone()?;
        let mut command = if let Some(program) = snapshot.program.as_deref() {
            let mut value = Command::new(program);
            value.args(&snapshot.args);
            value
        } else {
            shell_command(&snapshot.command)
        };
        command
            .current_dir(cwd)
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .stdin(Stdio::null())
            .hide_window();
        let mut child = command.spawn().context("start background task")?;
        let pid = child.id();
        {
            let mut tasks = self.tasks.lock().unwrap();
            let task = tasks.get_mut(id).unwrap();
            task.status = TaskStatus::Running;
            task.started_at = Some(now());
            task.completed_at = None;
            task.pid = Some(pid);
            task.exit_code = None;
            task.attempts += 1;
        }
        self.persist()?;
        let tasks = self.tasks.clone();
        let path = self.path.clone();
        let workspace = self.workspace.clone();
        let task_id = id.to_string();
        thread::spawn(move || {
            let result = child.wait();
            let mut guard = tasks.lock().unwrap();
            if let Some(task) = guard.get_mut(&task_id) {
                if task.status == TaskStatus::Running {
                    task.completed_at = Some(now());
                    task.pid = None;
                    match result {
                        Ok(status) => {
                            task.exit_code = status.code();
                            task.status = if status.success() {
                                TaskStatus::Completed
                            } else {
                                TaskStatus::Failed
                            };
                        }
                        Err(_) => task.status = TaskStatus::Interrupted,
                    }
                }
            }
            let result_task = guard.get(&task_id).cloned();
            let _ = persist_to(&path, &guard);
            drop(guard);
            if let Some(task) = result_task {
                let _ = write_result_artifact(&workspace, &task);
            }
        });
        Ok(self.tasks.lock().unwrap().get(id).cloned().unwrap())
    }

    pub fn cancel(&self, id: &str) -> Result<BackgroundTask> {
        let pid = self.tasks.lock().unwrap().get(id).and_then(|v| v.pid);
        if let Some(pid) = pid {
            terminate_process(pid)?;
        }
        let mut tasks = self.tasks.lock().unwrap();
        let task = tasks.get_mut(id).ok_or_else(|| anyhow!("unknown task"))?;
        task.status = TaskStatus::Cancelled;
        task.completed_at = Some(now());
        task.pid = None;
        let result = task.clone();
        drop(tasks);
        self.persist()?;
        Ok(result)
    }

    pub fn log_tail(&self, id: &str, max_bytes: usize) -> Result<String> {
        let task = self
            .tasks
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown task"))?;
        let bytes = fs::read(self.workspace.join(task.log_path)).unwrap_or_default();
        let start = bytes.len().saturating_sub(max_bytes.clamp(1, 256 * 1024));
        Ok(String::from_utf8_lossy(&bytes[start..]).into_owned())
    }

    fn resolve_cwd(&self, cwd: Option<&str>) -> Result<PathBuf> {
        let candidate = match cwd.map(str::trim).filter(|v| !v.is_empty()) {
            Some(v) => {
                let p = PathBuf::from(v);
                if p.is_absolute() {
                    p
                } else {
                    self.workspace.join(p)
                }
            }
            None => self.workspace.clone(),
        };
        let canonical = candidate
            .canonicalize()
            .context("task cwd does not exist")?;
        if !canonical.starts_with(&self.workspace) {
            return Err(anyhow!("task cwd must stay inside the project"));
        }
        Ok(canonical)
    }

    fn resolve_result_path(&self, result_path: &str) -> Result<PathBuf> {
        let relative = Path::new(result_path.trim());
        if relative.as_os_str().is_empty() || relative.is_absolute() {
            return Err(anyhow!("task result path must be workspace relative"));
        }
        let candidate = self.workspace.join(relative);
        let parent = candidate
            .parent()
            .ok_or_else(|| anyhow!("task result path has no parent"))?;
        fs::create_dir_all(parent)?;
        let canonical_parent = parent.canonicalize()?;
        if !canonical_parent.starts_with(&self.workspace) {
            return Err(anyhow!("task result path must stay inside the project"));
        }
        Ok(canonical_parent.join(
            candidate
                .file_name()
                .ok_or_else(|| anyhow!("task result path has no file name"))?,
        ))
    }
    fn persist(&self) -> Result<()> {
        persist_to(&self.path, &self.tasks.lock().unwrap())
    }
}

fn display_arg(value: &str) -> String {
    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "-_=./:\\".contains(character))
    {
        value.to_string()
    } else {
        format!("{:?}", value)
    }
}

fn persist_to(path: &Path, tasks: &BTreeMap<String, BackgroundTask>) -> Result<()> {
    let temp = path.with_extension("json.tmp");
    fs::write(
        &temp,
        serde_json::to_vec_pretty(&QueueFile {
            tasks: tasks.clone(),
        })?,
    )?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(temp, path)?;
    Ok(())
}

fn write_result_artifact(workspace: &Path, task: &BackgroundTask) -> Result<()> {
    let Some(relative) = task.result_path.as_deref() else {
        return Ok(());
    };
    let path = workspace.join(relative);
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("task result path has no parent"))?;
    fs::create_dir_all(parent)?;
    let canonical_parent = parent.canonicalize()?;
    if !canonical_parent.starts_with(workspace) {
        return Err(anyhow!("task result path must stay inside the project"));
    }
    let log = fs::read(workspace.join(&task.log_path)).unwrap_or_default();
    let truncated = log.len() > 4 * 1024 * 1024;
    let start = log.len().saturating_sub(4 * 1024 * 1024);
    let output = String::from_utf8_lossy(&log[start..]).into_owned();
    let mut payload = if path.is_file() {
        serde_json::from_slice::<serde_json::Value>(&fs::read(&path)?).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };
    if !payload.is_object() {
        payload = json!({ "result": payload });
    }
    let object = payload.as_object_mut().unwrap();
    object.insert(
        "schema_version".into(),
        json!("atlas.domain-action-result.v1"),
    );
    object.insert("task_id".into(), json!(task.id));
    object.insert("kind".into(), json!(task.kind));
    object.insert("title".into(), json!(task.title));
    object.insert("command".into(), json!(task.command));
    object.insert("program".into(), json!(task.program));
    object.insert("args".into(), json!(task.args));
    object.insert("status".into(), json!(task.status));
    object.insert("exit_code".into(), json!(task.exit_code));
    object.insert("started_at".into(), json!(task.started_at));
    object.insert("completed_at".into(), json!(task.completed_at));
    object.insert("log_path".into(), json!(task.log_path));
    object.insert("output_truncated".into(), json!(truncated));
    if !output.trim().is_empty() {
        object.insert("output".into(), json!(output));
    }
    fs::write(path, serde_json::to_vec_pretty(&payload)?)?;
    Ok(())
}
fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

#[cfg(windows)]
fn shell_command(command: &str) -> Command {
    let mut value = Command::new("powershell.exe");
    value.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        command,
    ]);
    value
}
#[cfg(not(windows))]
fn shell_command(command: &str) -> Command {
    let mut value = Command::new("sh");
    value.args(["-lc", command]);
    value
}

fn process_is_running(pid: u32) -> bool {
    #[cfg(windows)]
    {
        let mut command = Command::new("tasklist");
        command
            .args(["/FI", &format!("PID eq {}", pid), "/FO", "CSV", "/NH"])
            .hide_window();
        return command
            .output()
            .map(|v| String::from_utf8_lossy(&v.stdout).contains(&pid.to_string()))
            .unwrap_or(false);
    }
    #[cfg(not(windows))]
    {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|v| v.success())
            .unwrap_or(false)
    }
}
fn terminate_process(pid: u32) -> Result<()> {
    #[cfg(windows)]
    {
        let mut command = Command::new("taskkill");
        command
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .hide_window();
        if !command.status()?.success() {
            return Err(anyhow!("failed to stop task"));
        }
    }
    #[cfg(not(windows))]
    {
        Command::new("kill").arg(pid.to_string()).status()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queue_persists_and_requires_manual_resume() {
        let dir = tempfile::tempdir().unwrap();
        let queue = TaskQueue::open(dir.path()).unwrap();
        let task = queue
            .enqueue("batch", "batch", "echo atlas", None, false)
            .unwrap();
        assert_eq!(task.status, TaskStatus::Queued);
        let reopened = TaskQueue::open(dir.path()).unwrap();
        assert_eq!(reopened.list().len(), 1);
    }
    #[test]
    fn rejects_cwd_outside_project() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let queue = TaskQueue::open(dir.path()).unwrap();
        assert!(queue
            .enqueue(
                "bad",
                "batch",
                "echo x",
                Some(outside.path().to_str().unwrap()),
                false
            )
            .is_err());
    }
    #[test]
    fn dead_running_task_becomes_interrupted() {
        let dir = tempfile::tempdir().unwrap();
        let queue = TaskQueue::open(dir.path()).unwrap();
        let task = queue
            .enqueue("recover", "training", "echo x", None, false)
            .unwrap();
        {
            let mut tasks = queue.tasks.lock().unwrap();
            let saved = tasks.get_mut(&task.id).unwrap();
            saved.status = TaskStatus::Running;
            saved.pid = Some(u32::MAX);
        }
        queue.persist().unwrap();
        let reopened = TaskQueue::open(dir.path()).unwrap();
        assert_eq!(reopened.list()[0].status, TaskStatus::Interrupted);
        assert_eq!(reopened.list()[0].recovery, "awaiting_user_resume");
    }
}
