//! Atlas-native SSH Remote Development provider.
//!
//! OpenSSH is used only as a protocol transport. Atlas owns profiles, authorization,
//! lifecycle, object synchronization and the user-facing workspace.

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;
use uuid::Uuid;

use crate::atlas_core::{AtlasCore, LifecycleState, ScientificObject};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SshAuthMethod {
    Key,
    Password,
    Agent,
    #[default]
    SshConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RemoteHostConfig {
    pub id: String,
    pub label: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub auth_method: SshAuthMethod,
    pub identity_file: Option<String>,
    pub ssh_config_file: Option<String>,
    pub ssh_config_alias: Option<String>,
    pub jump_host: Option<String>,
    pub remote_root: String,
    pub connect_timeout_secs: u16,
    pub keepalive_secs: u16,
    pub auto_reconnect: bool,
    pub max_reconnect_attempts: u8,
    pub tags: Vec<String>,
}

impl Default for RemoteHostConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            label: String::new(),
            host: String::new(),
            port: 22,
            user: String::new(),
            auth_method: SshAuthMethod::SshConfig,
            identity_file: None,
            ssh_config_file: None,
            ssh_config_alias: None,
            jump_host: None,
            remote_root: "~".into(),
            connect_timeout_secs: 10,
            keepalive_secs: 15,
            auto_reconnect: true,
            max_reconnect_attempts: 5,
            tags: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConnection {
    pub session_id: String,
    pub host_id: String,
    pub state: ConnectionState,
    pub connected_at: Option<String>,
    pub last_heartbeat: Option<String>,
    pub latency_ms: Option<u64>,
    pub last_error: Option<String>,
    pub reconnect_attempts: u8,
    pub agent_authorized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemoteEnvironment {
    pub host_id: String,
    pub os: String,
    pub kernel: String,
    pub arch: String,
    pub shell: String,
    pub python: Vec<String>,
    pub managers: Vec<String>,
    pub git: Option<String>,
    pub docker: Option<String>,
    pub gpu_summary: Vec<String>,
    pub schedulers: Vec<String>,
    pub processes: Vec<String>,
    pub containers: Vec<String>,
    pub detected_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteTerminalView {
    pub id: String,
    pub host_id: String,
    pub title: String,
    pub output: String,
    pub running: bool,
}

struct RemoteTerminal {
    view: RemoteTerminalView,
    child: Child,
    output: Arc<Mutex<Vec<u8>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForwardKind {
    Local,
    Remote,
    Dynamic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemotePortForward {
    pub id: String,
    pub host_id: String,
    pub kind: ForwardKind,
    pub bind: String,
    pub target: Option<String>,
    pub running: bool,
}
struct ForwardRuntime {
    view: RemotePortForward,
    child: Child,
}

#[derive(Debug, Serialize)]
pub struct RemoteSshSnapshot {
    pub hosts: Vec<RemoteHostConfig>,
    pub connections: Vec<RemoteConnection>,
    pub terminals: Vec<RemoteTerminalView>,
    pub forwards: Vec<RemotePortForward>,
    pub environments: Vec<RemoteEnvironment>,
    pub transport_available: bool,
}

pub struct RemoteSshCore {
    workspace_root: PathBuf,
    storage_dir: PathBuf,
    hosts: BTreeMap<String, RemoteHostConfig>,
    connections: HashMap<String, RemoteConnection>,
    passwords: HashMap<String, String>,
    terminals: HashMap<String, RemoteTerminal>,
    forwards: HashMap<String, ForwardRuntime>,
    environments: HashMap<String, RemoteEnvironment>,
}

impl std::fmt::Debug for RemoteSshCore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteSshCore")
            .field("workspace_root", &self.workspace_root)
            .field("hosts", &self.hosts.len())
            .finish()
    }
}

impl RemoteSshCore {
    pub fn open(workspace_root: impl Into<PathBuf>) -> Result<Self> {
        let workspace_root = workspace_root.into();
        let storage_dir = workspace_root.join(".atlas").join("remote-ssh");
        fs::create_dir_all(&storage_dir)?;
        let hosts = fs::read(storage_dir.join("hosts.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Vec<RemoteHostConfig>>(&bytes).ok())
            .unwrap_or_default()
            .into_iter()
            .map(|h| (h.id.clone(), h))
            .collect();
        Ok(Self {
            workspace_root,
            storage_dir,
            hosts,
            connections: HashMap::new(),
            passwords: HashMap::new(),
            terminals: HashMap::new(),
            forwards: HashMap::new(),
            environments: HashMap::new(),
        })
    }

    pub fn snapshot(&mut self) -> RemoteSshSnapshot {
        for terminal in self.terminals.values_mut() {
            if let Ok(Some(_)) = terminal.child.try_wait() {
                terminal.view.running = false;
            }
            if let Ok(bytes) = terminal.output.lock() {
                terminal.view.output = String::from_utf8_lossy(&bytes).to_string();
            }
        }
        for forward in self.forwards.values_mut() {
            if let Ok(Some(_)) = forward.child.try_wait() {
                forward.view.running = false;
            }
        }
        RemoteSshSnapshot {
            hosts: self.hosts.values().cloned().collect(),
            connections: self.connections.values().cloned().collect(),
            terminals: self.terminals.values().map(|t| t.view.clone()).collect(),
            forwards: self.forwards.values().map(|f| f.view.clone()).collect(),
            environments: self.environments.values().cloned().collect(),
            transport_available: which::which("ssh").is_ok(),
        }
    }

    pub fn save_host(&mut self, mut host: RemoteHostConfig) -> Result<RemoteHostConfig> {
        if host.id.trim().is_empty() {
            host.id = stable_id(
                "host",
                &format!("{}:{}@{}", host.user, host.port, host.host),
            );
        }
        validate_host(&host)?;
        self.hosts.insert(host.id.clone(), host.clone());
        self.persist_hosts()?;
        Ok(host)
    }

    pub fn delete_host(&mut self, host_id: &str) -> Result<()> {
        validate_id(host_id)?;
        if self.connections.contains_key(host_id) {
            return Err(anyhow!("disconnect the remote host before deleting it"));
        }
        self.hosts
            .remove(host_id)
            .ok_or_else(|| anyhow!("remote host not found"))?;
        self.persist_hosts()
    }

    pub fn import_ssh_config(&mut self, path: Option<&str>) -> Result<Vec<RemoteHostConfig>> {
        let path = path.map(expand_home).transpose()?.unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_default()
                .join(".ssh")
                .join("config")
        });
        let text = fs::read_to_string(&path)
            .with_context(|| format!("cannot read SSH config {}", path.display()))?;
        let imported = parse_ssh_config(&text);
        let mut saved = Vec::new();
        for host in imported {
            saved.push(self.save_host(host)?);
        }
        Ok(saved)
    }

    pub fn connect(
        &mut self,
        host_id: &str,
        password: Option<String>,
        agent_authorized: bool,
    ) -> Result<RemoteConnection> {
        let host = self.host(host_id)?.clone();
        if host.auth_method == SshAuthMethod::Password
            && password.as_deref().unwrap_or("").is_empty()
        {
            return Err(anyhow!(
                "password is required and is retained only for this connection"
            ));
        }
        if let Some(secret) = password {
            self.passwords.insert(host_id.into(), secret);
        }
        let session_id = Uuid::new_v4().to_string();
        self.connections.insert(
            host_id.into(),
            RemoteConnection {
                session_id,
                host_id: host_id.into(),
                state: ConnectionState::Connecting,
                connected_at: None,
                last_heartbeat: None,
                latency_ms: None,
                last_error: None,
                reconnect_attempts: 0,
                agent_authorized,
            },
        );
        let started = Instant::now();
        match self.run_raw(&host, "printf ATLAS_SSH_READY", false) {
            Ok(output) if output.contains("ATLAS_SSH_READY") => {
                let now = Utc::now().to_rfc3339();
                let connection = self.connections.get_mut(host_id).unwrap();
                connection.state = ConnectionState::Connected;
                connection.connected_at = Some(now.clone());
                connection.last_heartbeat = Some(now);
                connection.latency_ms = Some(started.elapsed().as_millis() as u64);
                let connection = connection.clone();
                self.sync_server_object(&host, &connection)?;
                Ok(connection)
            }
            Ok(_) => {
                self.fail_connection(host_id, "remote handshake returned an unexpected response")
            }
            Err(error) => self.fail_connection(host_id, &error.to_string()),
        }
    }

    pub fn disconnect(&mut self, host_id: &str) -> Result<()> {
        self.connections
            .remove(host_id)
            .ok_or_else(|| anyhow!("remote host is not connected"))?;
        self.passwords.remove(host_id);
        let terminal_ids: Vec<_> = self
            .terminals
            .iter()
            .filter(|(_, t)| t.view.host_id == host_id)
            .map(|(id, _)| id.clone())
            .collect();
        for id in terminal_ids {
            let _ = self.close_terminal(&id);
        }
        let forward_ids: Vec<_> = self
            .forwards
            .iter()
            .filter(|(_, f)| f.view.host_id == host_id)
            .map(|(id, _)| id.clone())
            .collect();
        for id in forward_ids {
            let _ = self.stop_forward(&id);
        }
        let core = AtlasCore::open(&self.workspace_root)?;
        let mut event = crate::atlas_core::AtlasEvent::new(
            crate::atlas_core::AtlasEventKind::Custom("ssh_disconnected".into()),
            "atlas-remote-ssh",
            vec![stable_id("remote-server", host_id)],
        );
        event.data = json!({"title":"SSH Disconnected","summary":format!("Remote research host {} disconnected",host_id),"severity":"warning","category":"remote-ssh","environment_id":"ssh","restore_context":{"host_id":host_id}});
        core.record_event(event)?;
        Ok(())
    }

    pub fn heartbeat(&mut self, host_id: &str) -> Result<RemoteConnection> {
        if !self.connections.contains_key(host_id) {
            return Err(anyhow!("remote host is not connected"));
        }
        let host = self.host(host_id)?.clone();
        let started = Instant::now();
        match self.run_raw(&host, "printf ATLAS_HEARTBEAT", false) {
            Ok(_) => {
                let c = self.connections.get_mut(host_id).unwrap();
                c.state = ConnectionState::Connected;
                c.last_heartbeat = Some(Utc::now().to_rfc3339());
                c.latency_ms = Some(started.elapsed().as_millis() as u64);
                c.last_error = None;
                c.reconnect_attempts = 0;
                Ok(c.clone())
            }
            Err(error) => {
                let config = self.host(host_id)?.clone();
                let c = self.connections.get_mut(host_id).unwrap();
                c.last_error = Some(error.to_string());
                c.reconnect_attempts = c.reconnect_attempts.saturating_add(1);
                c.state = if config.auto_reconnect
                    && c.reconnect_attempts <= config.max_reconnect_attempts
                {
                    ConnectionState::Reconnecting
                } else {
                    ConnectionState::Error
                };
                Ok(c.clone())
            }
        }
    }

    pub fn execute(&self, host_id: &str, command: &str, agent: bool) -> Result<String> {
        validate_command(command)?;
        let host = self.require_connection(host_id, agent)?;
        self.run_raw(host, command, false)
    }

    pub fn list_files(&self, host_id: &str, remote_path: &str, agent: bool) -> Result<Value> {
        validate_remote_path(remote_path)?;
        let quoted = shell_quote(remote_path);
        let output = self.execute(host_id, &format!("find {quoted} -mindepth 1 -maxdepth 1 -printf '%f\\t%y\\t%s\\n' 2>/dev/null || ls -la {quoted}"), agent)?;
        let entries = output.lines().filter_map(|line| { let mut p = line.split('\t'); Some(json!({"name": p.next()?, "kind": p.next().unwrap_or("?"), "size": p.next().and_then(|v| v.parse::<u64>().ok())})) }).collect::<Vec<_>>();
        self.sync_directory_object(host_id, remote_path, &entries)?;
        Ok(json!({"path": remote_path, "entries": entries, "raw": output}))
    }

    pub fn transfer(
        &self,
        host_id: &str,
        direction: &str,
        local_path: &str,
        remote_path: &str,
        agent: bool,
    ) -> Result<Value> {
        let host = self.require_connection(host_id, agent)?;
        validate_remote_path(remote_path)?;
        let local = self.resolve_local_path(local_path, direction == "download")?;
        let mut command = Command::new("sftp");
        self.apply_sftp_args(&mut command, host)?;
        let mut batch = tempfile::NamedTempFile::new_in(&self.storage_dir)?;
        let local_text = local.to_string_lossy().replace('"', "\\\"");
        let remote_text = remote_path.replace('"', "\\\"");
        let instruction = match direction {
            "upload" | "sync" => {
                if !local.exists() {
                    return Err(anyhow!("local upload source does not exist"));
                }
                format!(
                    "put {}\"{}\" \"{}\"\n",
                    if local.is_dir() { "-r " } else { "" },
                    local_text,
                    remote_text
                )
            }
            "download" => {
                if let Some(parent) = local.parent() {
                    fs::create_dir_all(parent)?;
                }
                format!("get -r \"{}\" \"{}\"\n", remote_text, local_text)
            }
            _ => {
                return Err(anyhow!(
                    "transfer direction must be upload, download or sync"
                ))
            }
        };
        batch.write_all(instruction.as_bytes())?;
        batch.flush()?;
        command
            .args(["-b", &batch.path().to_string_lossy()])
            .arg(self.destination(host));
        let output = self.run_transport(command, host)?;
        if !output.status.success() {
            return Err(anyhow!(
                "SFTP transfer failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(
            json!({"direction": direction, "local_path": local.to_string_lossy(), "remote_path": remote_path, "protocol": "sftp"}),
        )
    }

    pub fn detect_environment(&mut self, host_id: &str, agent: bool) -> Result<RemoteEnvironment> {
        let script = "printf 'OS='; (grep '^PRETTY_NAME=' /etc/os-release 2>/dev/null | cut -d= -f2- | tr -d '\"' || uname -s); printf '\\nKERNEL='; uname -r; printf '\\nARCH='; uname -m; printf '\\nSHELL='; printf \"${SHELL:-unknown}\"; printf '\\nPYTHON='; (command -v python3 || command -v python || true); printf '\\nMANAGERS='; for x in conda uv poetry; do command -v $x >/dev/null && printf \"$x,\"; done; printf '\\nGIT='; (git --version 2>/dev/null || true); printf '\\nDOCKER='; (docker --version 2>/dev/null || true); printf '\\nGPU='; (nvidia-smi --query-gpu=index,name,memory.total --format=csv,noheader 2>/dev/null | tr '\\n' ';' || true); printf '\\nSCHED='; for x in sbatch qsub bsub; do command -v $x >/dev/null && printf \"$x,\"; done; printf '\\nPROCESSES='; (ps -eo pid=,comm= 2>/dev/null | head -40 | sed 's/^[[:space:]]*//' | tr '\\n' ';' || true); printf '\\nCONTAINERS='; (docker ps --format '{{.ID}}|{{.Image}}|{{.Names}}|{{.Status}}' 2>/dev/null | tr '\\n' ';' || true)";
        let output = self.execute(host_id, script, agent)?;
        let fields: HashMap<_, _> = output
            .lines()
            .filter_map(|line| line.split_once('='))
            .collect();
        let env = RemoteEnvironment {
            host_id: host_id.into(),
            os: field(&fields, "OS"),
            kernel: field(&fields, "KERNEL"),
            arch: field(&fields, "ARCH"),
            shell: field(&fields, "SHELL"),
            python: csv_field(&fields, "PYTHON"),
            managers: csv_field(&fields, "MANAGERS"),
            git: optional_field(&fields, "GIT"),
            docker: optional_field(&fields, "DOCKER"),
            gpu_summary: semicolon_field(&fields, "GPU"),
            schedulers: csv_field(&fields, "SCHED"),
            processes: semicolon_field(&fields, "PROCESSES"),
            containers: semicolon_field(&fields, "CONTAINERS"),
            detected_at: Utc::now().to_rfc3339(),
        };
        self.environments.insert(host_id.into(), env.clone());
        self.sync_environment_objects(&env)?;
        Ok(env)
    }

    pub fn create_terminal(
        &mut self,
        host_id: &str,
        title: Option<String>,
    ) -> Result<RemoteTerminalView> {
        let host = self.require_connection(host_id, false)?.clone();
        let id = Uuid::new_v4().to_string();
        let mut command = Command::new("ssh");
        self.apply_common_args(&mut command, &host, true)?;
        command.arg(self.destination(&host));
        let mut child = self.spawn_transport(command, &host)?;
        let output = Arc::new(Mutex::new(Vec::new()));
        if let Some(stdout) = child.stdout.take() {
            capture_terminal_stream(stdout, output.clone());
        }
        if let Some(stderr) = child.stderr.take() {
            capture_terminal_stream(stderr, output.clone());
        }
        let view = RemoteTerminalView {
            id: id.clone(),
            host_id: host_id.into(),
            title: title.unwrap_or_else(|| format!("{} shell", host.label)),
            output: String::new(),
            running: true,
        };
        self.terminals.insert(
            id,
            RemoteTerminal {
                view: view.clone(),
                child,
                output,
            },
        );
        Ok(view)
    }

    pub fn terminal_input(&mut self, terminal_id: &str, input: &str) -> Result<RemoteTerminalView> {
        if input.len() > 64 * 1024 {
            return Err(anyhow!("terminal input is too large"));
        }
        let terminal = self
            .terminals
            .get_mut(terminal_id)
            .ok_or_else(|| anyhow!("remote terminal not found"))?;
        terminal
            .child
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("remote terminal input is closed"))?
            .write_all(input.as_bytes())?;
        if let Ok(mut output) = terminal.output.lock() {
            output.extend_from_slice(input.as_bytes());
            trim_terminal_buffer(&mut output);
            terminal.view.output = String::from_utf8_lossy(&output).to_string();
        }
        Ok(terminal.view.clone())
    }

    pub fn close_terminal(&mut self, terminal_id: &str) -> Result<()> {
        let mut terminal = self
            .terminals
            .remove(terminal_id)
            .ok_or_else(|| anyhow!("remote terminal not found"))?;
        let _ = terminal.child.kill();
        let _ = terminal.child.wait();
        Ok(())
    }

    pub fn start_forward(
        &mut self,
        host_id: &str,
        kind: ForwardKind,
        bind: String,
        target: Option<String>,
    ) -> Result<RemotePortForward> {
        validate_endpoint(&bind)?;
        if let Some(value) = &target {
            validate_endpoint(value)?;
        }
        let host = self.require_connection(host_id, false)?.clone();
        let id = Uuid::new_v4().to_string();
        let spec = match kind {
            ForwardKind::Local => format!(
                "{}:{}",
                bind,
                target
                    .clone()
                    .ok_or_else(|| anyhow!("local forwarding requires a target"))?
            ),
            ForwardKind::Remote => format!(
                "{}:{}",
                bind,
                target
                    .clone()
                    .ok_or_else(|| anyhow!("remote forwarding requires a target"))?
            ),
            ForwardKind::Dynamic => bind.clone(),
        };
        let mut command = Command::new("ssh");
        self.apply_common_args(&mut command, &host, false)?;
        command.args(["-N", "-T", "-o", "ExitOnForwardFailure=yes"]);
        command
            .arg(match kind {
                ForwardKind::Local => "-L",
                ForwardKind::Remote => "-R",
                ForwardKind::Dynamic => "-D",
            })
            .arg(spec)
            .arg(self.destination(&host));
        let child = self.spawn_transport(command, &host)?;
        let view = RemotePortForward {
            id: id.clone(),
            host_id: host_id.into(),
            kind,
            bind,
            target,
            running: true,
        };
        self.sync_forward_object(&view, true)?;
        self.forwards.insert(
            id,
            ForwardRuntime {
                view: view.clone(),
                child,
            },
        );
        Ok(view)
    }

    pub fn stop_forward(&mut self, id: &str) -> Result<()> {
        let mut forward = self
            .forwards
            .remove(id)
            .ok_or_else(|| anyhow!("port forward not found"))?;
        let _ = forward.child.kill();
        let _ = forward.child.wait();
        self.sync_forward_object(&forward.view, false)?;
        Ok(())
    }

    pub fn sync_operation_object(
        &self,
        host_id: &str,
        operation: &str,
        command: &str,
        output: &str,
    ) -> Result<()> {
        let object_type = match operation {
            "training" => "remote-training-run",
            "docker" => "remote-container-operation",
            "logs" => "remote-log",
            "gpu" => "gpu-query",
            "git" => "remote-git-operation",
            "python" => "python-environment-operation",
            _ => "remote-process",
        };
        let core = AtlasCore::open(&self.workspace_root)?;
        let identity = format!(
            "{}:{}:{}",
            host_id,
            operation,
            blake3::hash(command.as_bytes())
        );
        let mut object = ScientificObject::new(
            object_type,
            format!("{} on {}", operation, host_id),
            "atlas-remote-ssh",
        );
        object.id = stable_id(object_type, &identity);
        object.lifecycle = LifecycleState::Completed;
        object.metadata.insert("host_id".into(), json!(host_id));
        object.metadata.insert("operation".into(), json!(operation));
        object.metadata.insert("command".into(), json!(command));
        object.metadata.insert(
            "output_preview".into(),
            json!(output.chars().take(2048).collect::<String>()),
        );
        object.preview.provider = "atlas.remote-ssh".into();
        object.preview.kind = operation.into();
        core.sync_external(object, "atlas-remote-ssh")?;
        Ok(())
    }

    pub fn auto_select_host(&self) -> Result<String> {
        self.connections
            .values()
            .filter(|c| matches!(c.state, ConnectionState::Connected) && c.agent_authorized)
            .min_by_key(|c| c.latency_ms.unwrap_or(u64::MAX))
            .map(|c| c.host_id.clone())
            .ok_or_else(|| anyhow!("no connected remote host is authorized for Agent access"))
    }
    pub fn connections_ready_for_restore(&self, host_id: &str) -> bool {
        self.connections
            .get(host_id)
            .is_some_and(|c| matches!(c.state, ConnectionState::Connected))
    }

    pub fn restore_connections(&mut self, host_ids: &[String]) -> Value {
        let mut restored = Vec::new();
        let mut pending = Vec::new();
        let mut failed = Vec::new();
        for id in host_ids {
            let Some(host) = self.hosts.get(id).cloned() else {
                failed.push(json!({"host_id":id,"error":"host profile missing"}));
                continue;
            };
            if host.auth_method == SshAuthMethod::Password {
                pending.push(json!({"host_id":id,"reason":"password authorization required"}));
                continue;
            }
            if self
                .connections
                .get(id)
                .is_some_and(|c| matches!(c.state, ConnectionState::Connected))
            {
                restored.push(id.clone());
                continue;
            }
            match self.connect(id, None, false) {
                Ok(_) => restored.push(id.clone()),
                Err(e) => failed.push(json!({"host_id":id,"error":e.to_string()})),
            }
        }
        json!({"restored":restored,"authorization_required":pending,"failed":failed})
    }

    fn host(&self, id: &str) -> Result<&RemoteHostConfig> {
        validate_id(id)?;
        self.hosts
            .get(id)
            .ok_or_else(|| anyhow!("remote host not found"))
    }
    fn require_connection(&self, id: &str, agent: bool) -> Result<&RemoteHostConfig> {
        let c = self
            .connections
            .get(id)
            .ok_or_else(|| anyhow!("remote host is not connected"))?;
        if !matches!(c.state, ConnectionState::Connected) {
            return Err(anyhow!("remote connection is not ready"));
        }
        if agent && !c.agent_authorized {
            return Err(anyhow!(
                "Agent access was not authorized for this connection"
            ));
        }
        self.host(id)
    }
    fn fail_connection<T>(&mut self, id: &str, message: &str) -> Result<T> {
        if let Some(c) = self.connections.get_mut(id) {
            c.state = ConnectionState::Error;
            c.last_error = Some(message.into());
        }
        Err(anyhow!(message.to_string()))
    }
    fn persist_hosts(&self) -> Result<()> {
        let hosts: Vec<_> = self.hosts.values().collect();
        fs::write(
            self.storage_dir.join("hosts.json"),
            serde_json::to_vec_pretty(&hosts)?,
        )?;
        Ok(())
    }
    fn destination(&self, host: &RemoteHostConfig) -> String {
        let target = host
            .ssh_config_alias
            .as_deref()
            .filter(|_| host.auth_method == SshAuthMethod::SshConfig)
            .unwrap_or(&host.host);
        if host.user.is_empty() || host.auth_method == SshAuthMethod::SshConfig {
            target.into()
        } else {
            format!("{}@{}", host.user, target)
        }
    }
    fn apply_common_args(
        &self,
        command: &mut Command,
        host: &RemoteHostConfig,
        tty: bool,
    ) -> Result<()> {
        command.args([
            "-p",
            &host.port.to_string(),
            "-o",
            &format!("ConnectTimeout={}", host.connect_timeout_secs),
            "-o",
            &format!("ServerAliveInterval={}", host.keepalive_secs),
            "-o",
            "ServerAliveCountMax=3",
            "-o",
            &format!(
                "UserKnownHostsFile={}",
                self.storage_dir.join("known_hosts").display()
            ),
            "-o",
            "StrictHostKeyChecking=accept-new",
        ]);
        if tty {
            command.arg("-tt");
        }
        if let Some(path) = &host.identity_file {
            command.args(["-i", path]);
        }
        if let Some(path) = &host.ssh_config_file {
            command.args(["-F", path]);
        }
        if let Some(jump) = &host.jump_host {
            command.args(["-J", jump]);
        }
        if host.auth_method == SshAuthMethod::Password {
            command.args([
                "-o",
                "PreferredAuthentications=password,keyboard-interactive",
                "-o",
                "PubkeyAuthentication=no",
            ]);
        }
        Ok(())
    }
    fn apply_sftp_args(&self, command: &mut Command, host: &RemoteHostConfig) -> Result<()> {
        command.args([
            "-P",
            &host.port.to_string(),
            "-o",
            &format!("ConnectTimeout={}", host.connect_timeout_secs),
            "-o",
            &format!(
                "UserKnownHostsFile={}",
                self.storage_dir.join("known_hosts").display()
            ),
            "-o",
            "StrictHostKeyChecking=accept-new",
        ]);
        if let Some(path) = &host.identity_file {
            command.args(["-i", path]);
        }
        if let Some(path) = &host.ssh_config_file {
            command.args(["-F", path]);
        }
        if let Some(jump) = &host.jump_host {
            command.args(["-J", jump]);
        }
        if host.auth_method == SshAuthMethod::Password {
            command.args([
                "-o",
                "PreferredAuthentications=password,keyboard-interactive",
                "-o",
                "PubkeyAuthentication=no",
            ]);
        }
        Ok(())
    }
    fn run_raw(&self, host: &RemoteHostConfig, remote_command: &str, tty: bool) -> Result<String> {
        let mut command = Command::new("ssh");
        self.apply_common_args(&mut command, host, tty)?;
        command.arg(self.destination(host)).arg(remote_command);
        let output = self.run_transport(command, host)?;
        if !output.status.success() {
            return Err(anyhow!(
                "remote command failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
    fn run_transport(
        &self,
        mut command: Command,
        host: &RemoteHostConfig,
    ) -> Result<std::process::Output> {
        self.configure_password(&mut command, host)?;
        command
            .output()
            .context("failed to start Atlas SSH transport")
    }
    fn spawn_transport(&self, mut command: Command, host: &RemoteHostConfig) -> Result<Child> {
        self.configure_password(&mut command, host)?;
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("failed to start Atlas SSH session")
    }
    fn configure_password(&self, command: &mut Command, host: &RemoteHostConfig) -> Result<()> {
        if host.auth_method != SshAuthMethod::Password {
            return Ok(());
        }
        let secret = self
            .passwords
            .get(&host.id)
            .ok_or_else(|| anyhow!("password session has expired"))?;
        let helper = self.ensure_askpass_helper()?;
        command
            .env("SSH_ASKPASS", helper)
            .env("SSH_ASKPASS_REQUIRE", "force")
            .env("ATLAS_SSH_PASSWORD", secret)
            .env("DISPLAY", "atlas:0")
            .stdin(Stdio::null());
        Ok(())
    }
    fn ensure_askpass_helper(&self) -> Result<PathBuf> {
        let path = if cfg!(windows) {
            self.storage_dir.join("askpass.cmd")
        } else {
            self.storage_dir.join("askpass.sh")
        };
        let body = if cfg!(windows) {
            "@echo off\r\npowershell.exe -NoProfile -NonInteractive -Command \"[Console]::Out.Write($env:ATLAS_SSH_PASSWORD)\"\r\n"
        } else {
            "#!/bin/sh\nprintf '%s\\n' \"$ATLAS_SSH_PASSWORD\"\n"
        };
        if fs::read_to_string(&path).ok().as_deref() != Some(body) {
            fs::write(&path, body)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
            }
        }
        Ok(path)
    }
    fn resolve_local_path(&self, value: &str, allow_missing: bool) -> Result<PathBuf> {
        let candidate = if Path::new(value).is_absolute() {
            PathBuf::from(value)
        } else {
            self.workspace_root.join(value)
        };
        let resolved = if allow_missing {
            let parent = candidate
                .parent()
                .ok_or_else(|| anyhow!("invalid local path"))?;
            fs::create_dir_all(parent)?;
            parent.canonicalize()?.join(
                candidate
                    .file_name()
                    .ok_or_else(|| anyhow!("invalid local path"))?,
            )
        } else {
            candidate.canonicalize()?
        };
        let root = self.workspace_root.canonicalize()?;
        if !resolved.starts_with(&root) {
            return Err(anyhow!(
                "local transfer path must stay inside the Atlas workspace"
            ));
        }
        Ok(resolved)
    }
    fn sync_server_object(
        &self,
        host: &RemoteHostConfig,
        connection: &RemoteConnection,
    ) -> Result<()> {
        let core = AtlasCore::open(&self.workspace_root)?;
        let mut object =
            ScientificObject::new("remote-server", host.label.clone(), "atlas-remote-ssh");
        object.id = stable_id("remote-server", &host.id);
        object.lifecycle = LifecycleState::Active;
        object.description = format!("SSH research server {}", host.host);
        object.metadata.insert("host_id".into(), json!(host.id));
        object.metadata.insert(
            "endpoint".into(),
            json!(format!("{}:{}", host.host, host.port)),
        );
        object
            .metadata
            .insert("connection_state".into(), json!(connection.state));
        object
            .metadata
            .insert("remote_root".into(), json!(host.remote_root));
        object.preview.provider = "atlas.remote-ssh".into();
        object.preview.kind = "remote-server".into();
        object.preview.payload =
            json!({"latency_ms": connection.latency_ms, "authorized": connection.agent_authorized});
        core.sync_external(object, "atlas-remote-ssh")?;
        Ok(())
    }
    fn sync_environment_objects(&self, env: &RemoteEnvironment) -> Result<()> {
        let core = AtlasCore::open(&self.workspace_root)?;
        let mut runtime = ScientificObject::new(
            "remote-runtime",
            format!("{} runtime", env.host_id),
            "atlas-remote-ssh",
        );
        runtime.id = stable_id("remote-runtime", &env.host_id);
        runtime.lifecycle = LifecycleState::Active;
        runtime
            .metadata
            .insert("host_id".into(), json!(env.host_id));
        runtime.metadata.insert("os".into(), json!(env.os));
        runtime.metadata.insert("kernel".into(), json!(env.kernel));
        runtime.metadata.insert("arch".into(), json!(env.arch));
        runtime.preview.provider = "atlas.remote-ssh".into();
        runtime.preview.kind = "environment-inspector".into();
        core.sync_external(runtime, "atlas-remote-ssh")?;
        for (index, python) in env.python.iter().enumerate() {
            let mut object =
                ScientificObject::new("python-environment", python.clone(), "atlas-remote-ssh");
            object.id = stable_id(
                "python-environment",
                &format!("{}:{index}:{python}", env.host_id),
            );
            object.lifecycle = LifecycleState::Active;
            object.metadata.insert("host_id".into(), json!(env.host_id));
            object.metadata.insert("executable".into(), json!(python));
            core.sync_external(object, "atlas-remote-ssh")?;
        }
        for gpu in &env.gpu_summary {
            let mut object = ScientificObject::new("gpu-device", gpu.clone(), "atlas-remote-ssh");
            object.id = stable_id("gpu-device", &format!("{}:{gpu}", env.host_id));
            object.lifecycle = LifecycleState::Active;
            object.metadata.insert("host_id".into(), json!(env.host_id));
            object.metadata.insert("summary".into(), json!(gpu));
            core.sync_external(object, "atlas-remote-ssh")?;
        }
        for process in &env.processes {
            let mut object =
                ScientificObject::new("remote-process", process.clone(), "atlas-remote-ssh");
            object.id = stable_id("remote-process", &format!("{}:{process}", env.host_id));
            object.lifecycle = LifecycleState::Running;
            object.metadata.insert("host_id".into(), json!(env.host_id));
            object.metadata.insert("process".into(), json!(process));
            core.sync_external(object, "atlas-remote-ssh")?;
        }
        for container in &env.containers {
            let mut object =
                ScientificObject::new("remote-container", container.clone(), "atlas-remote-ssh");
            object.id = stable_id("remote-container", &format!("{}:{container}", env.host_id));
            object.lifecycle = LifecycleState::Running;
            object.metadata.insert("host_id".into(), json!(env.host_id));
            object.metadata.insert("container".into(), json!(container));
            object.preview.provider = "atlas.remote-ssh".into();
            object.preview.kind = "container-runtime".into();
            core.sync_external(object, "atlas-remote-ssh")?;
        }
        Ok(())
    }
    fn sync_directory_object(&self, host_id: &str, path: &str, entries: &[Value]) -> Result<()> {
        let core = AtlasCore::open(&self.workspace_root)?;
        let mut object = ScientificObject::new("remote-directory", path, "atlas-remote-ssh");
        object.id = stable_id("remote-directory", &format!("{host_id}:{path}"));
        object.lifecycle = LifecycleState::Active;
        object.metadata.insert("host_id".into(), json!(host_id));
        object.metadata.insert("path".into(), json!(path));
        object
            .metadata
            .insert("entry_count".into(), json!(entries.len()));
        object.preview.provider = "atlas.remote-ssh".into();
        object.preview.kind = "remote-file-tree".into();
        object.preview.payload = json!({"entries": entries.iter().take(24).collect::<Vec<_>>()});
        core.sync_external(object, "atlas-remote-ssh")?;
        Ok(())
    }
    fn sync_forward_object(&self, forward: &RemotePortForward, active: bool) -> Result<()> {
        let core = AtlasCore::open(&self.workspace_root)?;
        let mut object = ScientificObject::new(
            "remote-port-forward",
            format!("{} forward {}", forward.host_id, forward.bind),
            "atlas-remote-ssh",
        );
        object.id = stable_id("remote-port-forward", &forward.id);
        object.lifecycle = if active {
            LifecycleState::Running
        } else {
            LifecycleState::Completed
        };
        object
            .metadata
            .insert("host_id".into(), json!(forward.host_id));
        object.metadata.insert("kind".into(), json!(forward.kind));
        object.metadata.insert("bind".into(), json!(forward.bind));
        object
            .metadata
            .insert("target".into(), json!(forward.target));
        core.sync_external(object, "atlas-remote-ssh")?;
        Ok(())
    }
}

fn validate_host(host: &RemoteHostConfig) -> Result<()> {
    validate_id(&host.id)?;
    if host.label.trim().is_empty() || host.label.len() > 80 {
        return Err(anyhow!(
            "host label is required and must be at most 80 characters"
        ));
    }
    if host.host.trim().is_empty()
        || host.host.len() > 255
        || host
            .host
            .chars()
            .any(|c| c.is_whitespace() || ";&|`$<>()".contains(c))
    {
        return Err(anyhow!("invalid SSH hostname"));
    }
    if host.port == 0 {
        return Err(anyhow!("invalid SSH port"));
    }
    if host
        .user
        .chars()
        .any(|c| !(c.is_ascii_alphanumeric() || "._-".contains(c)))
    {
        return Err(anyhow!("invalid SSH user"));
    }
    if let Some(jump) = &host.jump_host {
        if jump.len() > 255
            || jump
                .chars()
                .any(|c| c.is_whitespace() || ";&|`$<>()".contains(c))
        {
            return Err(anyhow!("invalid jump host"));
        }
    }
    validate_remote_path(&host.remote_root)
}
fn validate_id(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 96
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_".contains(c))
    {
        return Err(anyhow!("invalid remote resource id"));
    }
    Ok(())
}
fn validate_command(value: &str) -> Result<()> {
    if value.trim().is_empty() || value.len() > 128 * 1024 || value.contains('\0') {
        return Err(anyhow!("invalid remote command"));
    }
    Ok(())
}
fn validate_remote_path(value: &str) -> Result<()> {
    if value.trim().is_empty()
        || value.len() > 4096
        || value.contains('\0')
        || value.contains('\n')
        || value.contains('\r')
    {
        return Err(anyhow!("invalid remote path"));
    }
    Ok(())
}
fn validate_endpoint(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 300
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ".:-_[]".contains(c))
    {
        return Err(anyhow!("invalid port forwarding endpoint"));
    }
    Ok(())
}
fn stable_id(kind: &str, identity: &str) -> String {
    format!(
        "ssh-{}-{}",
        kind,
        &blake3::hash(identity.as_bytes()).to_hex()[..24]
    )
}
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
fn expand_home(value: &str) -> Result<PathBuf> {
    if let Some(rest) = value
        .strip_prefix("~/")
        .or_else(|| value.strip_prefix("~\\"))
    {
        Ok(dirs::home_dir()
            .ok_or_else(|| anyhow!("home directory is unavailable"))?
            .join(rest))
    } else {
        Ok(PathBuf::from(value))
    }
}
fn field(map: &HashMap<&str, &str>, key: &str) -> String {
    map.get(key).copied().unwrap_or("").trim().to_string()
}
fn optional_field(map: &HashMap<&str, &str>, key: &str) -> Option<String> {
    let v = field(map, key);
    (!v.is_empty()).then_some(v)
}
fn csv_field(map: &HashMap<&str, &str>, key: &str) -> Vec<String> {
    field(map, key)
        .split(',')
        .filter(|v| !v.trim().is_empty())
        .map(|v| v.trim().to_string())
        .collect()
}
fn semicolon_field(map: &HashMap<&str, &str>, key: &str) -> Vec<String> {
    field(map, key)
        .split(';')
        .filter(|v| !v.trim().is_empty())
        .map(|v| v.trim().to_string())
        .collect()
}
fn capture_terminal_stream<R: Read + Send + 'static>(mut reader: R, output: Arc<Mutex<Vec<u8>>>) {
    thread::spawn(move || {
        let mut chunk = [0u8; 4096];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(read) => {
                    if let Ok(mut buffer) = output.lock() {
                        buffer.extend_from_slice(&chunk[..read]);
                        trim_terminal_buffer(&mut buffer);
                    }
                }
            }
        }
    });
}
fn trim_terminal_buffer(buffer: &mut Vec<u8>) {
    const MAX: usize = 256 * 1024;
    if buffer.len() > MAX {
        let drain = buffer.len() - MAX / 2;
        buffer.drain(..drain);
    }
}

pub fn parse_ssh_config(text: &str) -> Vec<RemoteHostConfig> {
    let mut result = Vec::new();
    let mut current: Option<RemoteHostConfig> = None;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let key = key.to_ascii_lowercase();
        let value = value.trim();
        if key == "host" {
            if let Some(host) = current.take() {
                if !host.host.contains('*') && !host.host.contains('?') {
                    result.push(host);
                }
            }
            if value.contains('*') || value.contains('?') || value.split_whitespace().count() != 1 {
                current = None;
            } else {
                current = Some(RemoteHostConfig {
                    id: stable_id("host", value),
                    label: value.into(),
                    host: value.into(),
                    ssh_config_alias: Some(value.into()),
                    ..Default::default()
                });
            }
            continue;
        }
        let Some(host) = current.as_mut() else {
            continue;
        };
        match key.as_str() {
            "hostname" => host.host = value.into(),
            "user" => host.user = value.into(),
            "port" => host.port = value.parse().unwrap_or(22),
            "identityfile" => {
                host.identity_file = Some(value.into());
                host.auth_method = SshAuthMethod::Key;
            }
            "proxyjump" => host.jump_host = Some(value.into()),
            _ => {}
        }
    }
    if let Some(host) = current {
        if !host.host.contains('*') && !host.host.contains('?') {
            result.push(host);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_config_and_jump_host() {
        let hosts = parse_ssh_config("Host gpu-lab\n HostName 10.0.0.8\n User atlas\n Port 2202\n ProxyJump bastion\n IdentityFile ~/.ssh/id_ed25519\n");
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].jump_host.as_deref(), Some("bastion"));
        assert_eq!(hosts[0].auth_method, SshAuthMethod::Key);
    }
    #[test]
    fn profile_has_no_password_field() {
        let json = serde_json::to_string(&RemoteHostConfig::default()).unwrap();
        assert!(!json.contains("password"));
    }
    #[test]
    fn rejects_command_newline_in_paths() {
        assert!(validate_remote_path("/tmp/a\nrm").is_err());
    }
    #[test]
    fn ids_are_stable() {
        assert_eq!(
            stable_id("remote-server", "x"),
            stable_id("remote-server", "x")
        );
    }
}
