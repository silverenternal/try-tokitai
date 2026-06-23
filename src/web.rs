use std::convert::Infallible;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
use anyhow::{anyhow, Result};
use axum::body::{Body, Bytes};
use axum::extract::{Query, State};
use axum::http::header;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::{FutureExt, StreamExt};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tower_http::services::ServeDir;
use chrono::Local;
use regex::Regex;
use tokio::sync::oneshot;
use tokio::task::JoinSet;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::app_paths::AppPaths;
use crate::assistant_common::AssistantConfig;
use crate::cli_assistant::{ChatRunResult, CliAssistant};
use crate::config::Config;
use crate::domain_prompt::{agent_mode_system_prompt, chat_mode_system_prompt, research_mode_system_prompt, strip_emoji};
use crate::host::{
    HostBridgeResponse, HostBridgeStream, HostCapabilities, HostCommand, HostDescriptor,
};
use crate::llm::providers::OpenAIProvider;
use crate::llm::{ChatRequest, LLMProvider, Message};
use crate::provider_config::ProviderManager;
use crate::sandbox::initialize_app_sandbox;
use crate::security::{default_tool_risk_map, RateLimiter, SecurityConfig};
use crate::text_encoding::{decode_bytes, ensure_json_text, normalize_json_strings, read_text_file};
use crate::toolchain::{
    auto_detect_toolchain_paths, command_is_available, default_toolchain_command,
    normalize_toolchain_paths, resolve_toolchain_value,
};
use crate::tool_matrix::matrix::RiskLevel;
use crate::tui::components::message_block::{
    AgentSubagentRecord, AgentVerifierCheck, AgentVerifierReport, MessageBlock, ToolCallStatus,
};
use crate::tui::components::diff_viewer::{DiffLine, FileDiff};
use crate::tui::streaming::{build_conversation, is_tool_call_finish};
use crate::tui::session::{SessionBranch, SessionManager, SessionMeta};

#[derive(Clone)]
pub struct WebAppState {
    assistant: Arc<Mutex<Option<CliAssistant>>>,
    assistant_api_url: String,
    assistant_api_key: Option<String>,
    runtime_settings: Arc<Mutex<RuntimeSettings>>,
    persisted_state_path: PathBuf,
    session_manager: Arc<Mutex<SessionManager>>,
    base_security_config: SecurityConfig,
    run_debug_runtime: Arc<Mutex<RunDebugRuntime>>,
    terminal_runtime: Arc<Mutex<TerminalRuntime>>,
    stream_runtime: Arc<Mutex<HashMap<String, StreamSessionRuntime>>>,
    host: Arc<WebHostConfig>,
}

#[derive(Debug, Clone)]
pub struct WebHostConfig {
    pub paths: AppPaths,
    pub bind_addr: SocketAddr,
    pub descriptor: HostDescriptor,
}

impl WebHostConfig {
    pub fn for_local_dev(base_dir: PathBuf) -> Self {
        let paths = AppPaths::for_local_dev(base_dir);
        Self {
            paths,
            bind_addr: SocketAddr::from(([127, 0, 0, 1], 3001)),
            descriptor: HostDescriptor::web_http(HostCapabilities::web_default()),
        }
    }

    pub fn for_desktop_shell(
        base_dir: PathBuf,
        frontend_dir: PathBuf,
        state_dir: PathBuf,
    ) -> Self {
        let paths = AppPaths::for_desktop(base_dir, frontend_dir, state_dir);
        Self {
            paths,
            bind_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
            descriptor: HostDescriptor::desktop_bridge(HostCapabilities::desktop_default()),
        }
    }

    pub fn from_env_or_local_dev(base_dir: PathBuf) -> Self {
        let frontend_dir = std::env::var("TOKITAI_FRONTEND_DIR").ok().map(PathBuf::from);
        let state_dir = std::env::var("TOKITAI_STATE_DIR").ok().map(PathBuf::from);
        let host_mode = std::env::var("TOKITAI_HOST_MODE").unwrap_or_else(|_| "web".to_string());

        let bind_addr = std::env::var("TOKITAI_BIND_ADDR")
            .ok()
            .and_then(|value| value.parse::<SocketAddr>().ok());

        match host_mode.as_str() {
            "desktop" => {
                let mut config = match (frontend_dir, state_dir) {
                    (Some(frontend_dir), Some(state_dir)) => {
                        Self::for_desktop_shell(base_dir.clone(), frontend_dir, state_dir)
                    }
                    _ => {
                        if let Some(paths) = AppPaths::for_desktop_defaults() {
                            Self {
                                paths,
                                bind_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
                                descriptor: HostDescriptor::desktop_bridge(HostCapabilities::desktop_default()),
                            }
                        } else {
                            Self::for_local_dev(base_dir.clone())
                        }
                    }
                };
                if let Some(addr) = bind_addr {
                    config.bind_addr = addr;
                }
                config
            }
            _ => {
                let mut config = Self::for_local_dev(base_dir);
                if let Some(addr) = bind_addr {
                    config.bind_addr = addr;
                }
                config
            }
        }
    }

    pub fn persisted_state_path(&self) -> PathBuf {
        self.paths.web_runtime_state_path()
    }

    pub fn sessions_dir(&self) -> PathBuf {
        self.paths.sessions_dir()
    }

    pub fn base_dir(&self) -> &Path {
        self.paths.base_dir()
    }

    pub fn frontend_dir(&self) -> &Path {
        self.paths.frontend_dir()
    }

    pub fn workspace_run_debug_dir(&self, workspace: &Path) -> PathBuf {
        self.paths.workspace_run_debug_dir(workspace)
    }

    pub fn descriptor(&self) -> HostDescriptor {
        self.descriptor.clone()
    }
}

#[derive(Debug, Clone)]
struct RuntimeSettings {
    api_url: String,
    model: String,
    deep_think: bool,
    reasoning_effort: String,
    competition_mode: bool,
    privacy_mode: bool,
    api_key: Option<String>,
    providers: Vec<String>,
    workspace_root: String,
    auto_approve_tools: bool,
    max_auto_approve_risk: RiskLevel,
    max_tool_calls_per_minute: u32,
    burst_limit: u32,
    toolchains: BTreeMap<String, String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct PersistedWebState {
    workspace_root: Option<String>,
    current_session_id: Option<String>,
    api_url: Option<String>,
    model: Option<String>,
    reasoning_effort: Option<String>,
    auto_approve_tools: Option<bool>,
    max_auto_approve_risk: Option<String>,
    max_tool_calls_per_minute: Option<u32>,
    burst_limit: Option<u32>,
    toolchains: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    ok: bool,
    data: T,
}

fn json_api_response<T: Serialize>(ok: bool, data: T) -> Json<Value> {
    let mut value = json!({
        "ok": ok,
        "data": data,
    });
    normalize_json_strings(&mut value);
    Json(value)
}

#[derive(Debug, Serialize)]
struct WebBootstrap {
    host: HostDescriptor,
    workspace_root: String,
    sandbox: WebSandboxBootstrapPayload,
    config: WebConfigPayload,
    research: WebResearchPayload,
    review: WebReviewPayload,
    git: WebGitPayload,
    workspace_browser: WebWorkspaceBrowser,
    sessions: Vec<SessionMeta>,
    active_sessions: Vec<WebActiveSession>,
    runtime_snapshots: Vec<WebSessionRuntimeSnapshot>,
    current_session_id: Option<String>,
    branches: Vec<SessionBranch>,
    messages: Vec<WebMessage>,
}

#[derive(Debug, Serialize, Clone, Default)]
struct WebActiveSession {
    session_id: String,
    status: String,
    waiting_approval: bool,
}

#[derive(Debug, Serialize, Clone, Default)]
struct WebSandboxBootstrapPayload {
    initialized: bool,
    first_run: bool,
    sandbox_root: String,
    downloads_root: String,
    sessions_root: String,
}

#[derive(Debug, Serialize)]
struct WebConfigPayload {
    api_url: String,
    model: String,
    deep_think: bool,
    reasoning_effort: String,
    competition_mode: bool,
    privacy_mode: bool,
    api_key: Option<String>,
    providers: Vec<String>,
    workspace_root: String,
    max_tool_calls_per_minute: u32,
    burst_limit: u32,
    auto_approve_tools: bool,
    max_auto_approve_risk: String,
    toolchains: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, Clone)]
struct WebResearchPayload {
    active: bool,
    topic: String,
    phase: String,
    phase_index: usize,
    phase_total: usize,
    next_phase: Option<String>,
    workspace: Option<String>,
    security_level: String,
    waiting_approval: bool,
    competition_mode: bool,
    workflow_kind: String,
    overall_state: String,
    rationale: String,
    blocker: Option<String>,
    recovery_hint: Option<String>,
    resume_points: Vec<String>,
    resource_summary: Option<String>,
    graph: ResearchGraphPayload,
    review: Vec<String>,
    runtime: WebResearchRuntimeEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionResearchState {
    Inactive,
    Agent,
    Research,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TurnLanguage {
    Zh,
    En,
}

#[derive(Debug, Serialize, Clone, Default)]
struct ResearchGraphPayload {
    nodes: Vec<ResearchGraphNode>,
    edges: Vec<ResearchGraphEdge>,
}

#[derive(Debug, Serialize, Clone)]
struct ResearchGraphNode {
    id: String,
    label: String,
    detail: String,
    status: String,
    lane: String,
    x: i32,
    y: i32,
}

#[derive(Debug, Serialize, Clone)]
struct ResearchGraphEdge {
    from: String,
    to: String,
    label: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct ResearchRuntimeAssessment {
    overall_state: String,
    blocker: Option<String>,
    recovery_hint: Option<String>,
    resume_points: Vec<String>,
    resource_summary: Option<String>,
    current_node_override: Option<String>,
    next_phase_override: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct SystemCapability {
    cpu_cores: u32,
    total_memory_mb: Option<u64>,
    available_memory_mb: Option<u64>,
    gpu_hint: Option<String>,
}

#[derive(Debug, Clone)]
struct CachedSystemCapability {
    signature: String,
    fetched_at: Instant,
    capability: SystemCapability,
}

#[derive(Debug, Serialize, Clone, Default)]
struct WebReviewPayload {
    available: bool,
    total_files: usize,
    total_additions: u32,
    total_deletions: u32,
    files: Vec<WebReviewFileSummary>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct WebReviewFileSummary {
    path: String,
    status: String,
    additions: u32,
    deletions: u32,
}

#[derive(Debug, Serialize, Clone)]
struct WebReviewFileDetail {
    path: String,
    status: String,
    additions: u32,
    deletions: u32,
    hunks: Vec<WebReviewHunk>,
    #[serde(default)]
    preview_kind: String,
    #[serde(default)]
    mime_type: String,
    #[serde(default)]
    is_binary: bool,
}

#[derive(Debug, Serialize, Clone)]
struct WebWorkspaceBrowser {
    root_name: String,
    root_path: String,
    entries: Vec<WebWorkspaceEntry>,
}

#[derive(Debug, Serialize, Clone)]
struct WebWorkspaceEntry {
    path: String,
    name: String,
    kind: String,
    children: Option<Vec<WebWorkspaceEntry>>,
}

#[derive(Debug, Serialize, Clone)]
struct WebWorkspaceFileResponse {
    path: String,
    name: String,
    language: String,
    content: String,
    truncated: bool,
    line_count: usize,
    mime_type: String,
    preview_kind: String,
    is_binary: bool,
}

#[derive(Debug, Serialize, Clone)]
struct WebReviewHunk {
    header: String,
    lines: Vec<WebReviewLine>,
}

#[derive(Debug, Serialize, Clone)]
struct WebReviewLine {
    kind: String,
    old_number: Option<usize>,
    new_number: Option<usize>,
    content: String,
}

#[derive(Debug, Serialize, Clone)]
struct WebMessage {
    kind: String,
    role: String,
    content: String,
    call_id: Option<String>,
    success: Option<bool>,
    collapsed: Option<bool>,
    tool_name: Option<String>,
    tool_args: Option<Value>,
    status: Option<String>,
    file_path: Option<String>,
    added: Option<usize>,
    removed: Option<usize>,
    before_content: Option<String>,
    subagent: Option<WebSubagentEvent>,
    verifier: Option<WebVerifierReport>,
}

#[derive(Debug, Serialize, Clone)]
struct WebActivityEvent {
    label: String,
    detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    meta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delegates: Option<Vec<AgentWorkflowDelegate>>,
}

#[derive(Debug, Serialize, Clone, Default)]
struct WebSubagentEvent {
    id: String,
    name: String,
    purpose: String,
    input: String,
    output: String,
    status: String,
    kind: String,
    started_at: Option<String>,
    completed_at: Option<String>,
    evidence: Vec<String>,
}

#[derive(Debug, Serialize, Clone, Default)]
struct WebVerifierCheck {
    id: String,
    title: String,
    status: String,
    detail: String,
    evidence: Vec<String>,
}

#[derive(Debug, Serialize, Clone, Default)]
struct WebVerifierReport {
    status: String,
    summary: String,
    checks: Vec<WebVerifierCheck>,
    issues: Vec<String>,
    evidence: Vec<String>,
    next_actions: Vec<String>,
    deterministic: bool,
}

#[derive(Debug, Serialize, Clone, Default)]
struct WebResearchRuntimeEvent {
    subagents: Vec<WebSubagentEvent>,
    verifier: Option<WebVerifierReport>,
    checkpoints: Vec<String>,
    branch_notes: Vec<String>,
    timeline: Vec<WebTimelineEvent>,
    resumable: bool,
}

#[derive(Debug, Serialize, Clone)]
struct WebToolEvent {
    call_id: String,
    name: String,
    status: String,
    risk: String,
    args: Option<Value>,
    result: Option<String>,
    success: Option<bool>,
    file_path: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct WebPermissionRequest {
    call_id: String,
    name: String,
    risk: String,
    reason: String,
    args: Value,
}

#[derive(Debug, Serialize, Clone)]
struct WebEditedFile {
    path: String,
    added: usize,
    removed: usize,
    before_content: String,
    after_content: String,
}

#[derive(Debug, Clone)]
struct PendingFileSnapshot {
    display_path: String,
    absolute_path: PathBuf,
    old_content: String,
}

#[derive(Debug, Clone)]
struct VerifiedWorkspaceWrite {
    diff: FileDiff,
    absolute_path: PathBuf,
}

#[derive(Debug, Clone)]
struct RequiredPathSnapshot {
    display_path: String,
    absolute_path: PathBuf,
    existed_before: bool,
    fingerprint_before: Option<u64>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct AgentWorkflowPlan {
    #[serde(default)]
    workflow_kind: String,
    #[serde(default)]
    goal: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    steps: Vec<AgentWorkflowStep>,
    #[serde(default)]
    delegates: Vec<AgentWorkflowDelegate>,
    #[serde(default)]
    verification: Vec<String>,
    #[serde(default)]
    repair_strategy: String,
    #[serde(default)]
    required_paths: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct AgentWorkflowStep {
    #[serde(default)]
    title: String,
    #[serde(default)]
    purpose: String,
    #[serde(default)]
    owner: String,
    #[serde(default)]
    kind: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct AgentWorkflowDelegate {
    #[serde(default)]
    name: String,
    #[serde(default)]
    purpose: String,
    #[serde(default)]
    input: String,
    #[serde(default)]
    output: String,
    #[serde(default)]
    status: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct AgentCritiqueReport {
    #[serde(default)]
    status: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    issues: Vec<String>,
    #[serde(default)]
    evidence: Vec<String>,
    #[serde(default)]
    next_actions: Vec<String>,
}

#[derive(Debug, Clone)]
struct AnalysisSubagentSpec {
    id: &'static str,
    name: &'static str,
    purpose: &'static str,
    kind: &'static str,
    system_prompt: &'static str,
    focus: &'static str,
    focus_zh: &'static str,
}

#[derive(Debug, Clone)]
struct ParallelAnalysisResult {
    review_report: AgentCritiqueReport,
    verifier_report: AgentVerifierReport,
    subagent_records: Vec<AgentSubagentRecord>,
    checkpoints: Vec<String>,
    branch_notes: Vec<String>,
    needs_repair: bool,
    summary: String,
    issues: Vec<String>,
    next_actions: Vec<String>,
    evidence: Vec<String>,
    hard_failed: bool,
}

#[derive(Debug, Clone)]
struct ParallelAnalysisProgress {
    verifier_report: Option<AgentVerifierReport>,
    subagent_record: Option<AgentSubagentRecord>,
    checkpoints: Vec<String>,
    branch_notes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SendMessageRequest {
    content: String,
    mode: Option<String>,
    language: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamControlRequest {
    session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ToolApprovalRequest {
    session_id: String,
    call_id: String,
}

#[derive(Debug, Deserialize)]
struct SessionSwitchRequest {
    session_id: String,
}

#[derive(Debug, Deserialize)]
struct SessionDeleteRequest {
    session_id: String,
}

#[derive(Debug, Deserialize)]
struct SessionRenameRequest {
    session_id: String,
    title: String,
}

#[derive(Debug, Deserialize)]
struct ReviewFileRequest {
    path: String,
}

#[derive(Debug, Deserialize)]
struct WorkspaceFileSaveRequest {
    path: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct WorkspaceFileUndoRequest {
    path: String,
    before_content: String,
}

#[derive(Debug, Deserialize)]
struct WorkspaceFileCompleteRequest {
    path: String,
    language: String,
    token_prefix: String,
    prefix: String,
    suffix: String,
    cursor_line: usize,
    cursor_column: usize,
}

#[derive(Debug, Deserialize)]
struct WebSettingsRequest {
    api_url: String,
    model: String,
    deep_think: bool,
    reasoning_effort: String,
    competition_mode: bool,
    privacy_mode: bool,
    workspace_root: String,
    api_key: Option<String>,
    auto_approve_tools: bool,
    max_auto_approve_risk: String,
    max_tool_calls_per_minute: u32,
    burst_limit: u32,
    toolchains: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Serialize)]
struct SendMessageResponse {
    messages: Vec<WebMessage>,
    session_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct SessionMutationResponse {
    session_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct SettingsMutationResponse {
    config: WebConfigPayload,
}

#[derive(Debug, Serialize)]
struct WorkspacePickerResponse {
    workspace_root: String,
}

#[derive(Debug, Serialize)]
struct ReviewFileResponse {
    file: WebReviewFileDetail,
}

#[derive(Debug, Serialize)]
struct WorkspaceFileEnvelope {
    file: WebWorkspaceFileResponse,
}

#[derive(Debug, Serialize)]
struct WorkspaceFileSaveResponse {
    file: WebWorkspaceFileResponse,
}

#[derive(Debug, Serialize)]
struct WorkspaceFileCompleteResponse {
    items: Vec<WorkspaceCodeCompletionItem>,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
struct WorkspaceCodeCompletionItem {
    label: String,
    insert_text: String,
    detail: String,
    source: String,
}

#[derive(Debug, Deserialize)]
struct WorkspaceRawFileQuery {
    path: String,
}

#[derive(Debug, Deserialize, Clone, Copy, Default)]
struct GitStateQuery {
    diff: Option<bool>,
    graph: Option<bool>,
}

#[derive(Debug, Serialize, Clone, Default)]
struct WebGitPayload {
    available: bool,
    repository_root: String,
    status: Option<WebGitStatus>,
    branches: Vec<WebGitBranch>,
    commits: Vec<WebGitCommit>,
    graph: Vec<WebGitGraphRow>,
    staged_diff: Option<String>,
    working_diff: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Clone, Default)]
struct WebGitStatus {
    branch: String,
    upstream: Option<String>,
    ahead: u32,
    behind: u32,
    has_conflicts: bool,
    has_staged_changes: bool,
    has_unstaged_changes: bool,
    has_untracked_files: bool,
    repository_clean: bool,
    summary: String,
    changed_files: Vec<WebGitChangedFile>,
}

#[derive(Debug, Serialize, Clone, Default)]
struct WebGitChangedFile {
    path: String,
    original_path: Option<String>,
    change_type: String,
    staged: bool,
    unstaged: bool,
    untracked: bool,
    conflicted: bool,
}

#[derive(Debug, Serialize, Clone, Default)]
struct WebGitBranch {
    name: String,
    upstream: Option<String>,
    is_current: bool,
    is_remote: bool,
    last_updated: Option<String>,
}

#[derive(Debug, Serialize, Clone, Default)]
struct WebGitCommit {
    hash: String,
    author: String,
    author_email: String,
    message: String,
    date: String,
}

#[derive(Debug, Serialize, Clone)]
struct WebGitGraphRow {
    hash: String,
    full_hash: String,
    parents: Vec<String>,
    refs: Vec<String>,
    subject: String,
    relative_time: String,
    author: String,
}

#[derive(Debug, Serialize)]
struct GitActionResponse {
    git: WebGitPayload,
}

#[derive(Debug, Deserialize)]
struct GitActionRequest {
    action: String,
    branch: Option<String>,
    pathspecs: Option<Vec<String>>,
    message: Option<String>,
    reference: Option<String>,
    diff: Option<bool>,
    graph: Option<bool>,
}

#[derive(Debug, Serialize, Clone, Default)]
struct WebExtensionsPayload {
    items: Vec<WebExtensionItem>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct WebExtensionItem {
    id: String,
    title: String,
    source: String,
    version: String,
    description: String,
}

#[derive(Debug, Serialize, Clone, Default)]
struct WebRunDebugPayload {
    configs: Vec<WebRunConfig>,
    active: Option<WebRunSession>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct WebMissingDependency {
    key: String,
    executable: String,
    configured: String,
}

#[derive(Debug, Serialize, Clone)]
struct WebRunConfig {
    id: String,
    title: String,
    command: String,
    runtime_executable: String,
    category: String,
    file_hint: Option<String>,
    language: String,
    task_type: String,
    available: bool,
    missing: Vec<String>,
    missing_dependencies: Vec<WebMissingDependency>,
    detail: String,
    task: Value,
    launch: Value,
}

#[derive(Debug, Serialize, Clone)]
struct WebRunSession {
    config_id: String,
    title: String,
    pid: u32,
    started_at: String,
    stdout_tail: String,
    stderr_tail: String,
    command: String,
    cwd: String,
}

#[derive(Debug, Deserialize)]
struct RunDebugActionRequest {
    action: String,
    config_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct ExtensionsEnvelope {
    extensions: WebExtensionsPayload,
}

#[derive(Debug, Serialize)]
struct RunDebugEnvelope {
    run_debug: WebRunDebugPayload,
}

#[derive(Debug, Default)]
struct RunDebugRuntime {
    active: Option<RunDebugSessionRuntime>,
}

#[derive(Debug, Clone)]
struct RunDebugSessionRuntime {
    config_id: String,
    title: String,
    pid: u32,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
    started_at: String,
    command: String,
    cwd: PathBuf,
}

#[derive(Debug, Serialize, Clone, Default)]
struct WebTerminalPayload {
    sessions: Vec<WebTerminalSession>,
    active_id: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct WebTerminalSession {
    id: String,
    title: String,
    cwd: String,
    created_at: String,
    command: String,
    status: String,
    buffer: String,
}

#[derive(Debug, Deserialize)]
struct TerminalInputRequest {
    terminal_id: String,
    input: String,
}

#[derive(Debug, Deserialize)]
struct TerminalCloseRequest {
    terminal_id: String,
}

#[derive(Debug, Serialize)]
struct TerminalEnvelope {
    terminals: WebTerminalPayload,
}

#[derive(Debug, Default)]
struct TerminalRuntime {
    sessions: Vec<TerminalSessionRuntime>,
    active_id: Option<String>,
    next_id: u64,
}

#[derive(Debug)]
struct TerminalSessionRuntime {
    id: String,
    title: String,
    cwd: String,
    created_at: String,
    command: String,
    status: Arc<Mutex<String>>,
    buffer: Arc<Mutex<Vec<u8>>>,
    stdin: Arc<Mutex<ChildStdin>>,
    child: Arc<Mutex<Child>>,
}

#[derive(Debug)]
struct StreamSessionRuntime {
    abort_handle: tokio::task::AbortHandle,
    event_tx: tokio::sync::mpsc::UnboundedSender<StreamEnvelope>,
    pending_approvals: HashMap<String, PendingApprovalRuntime>,
    latest_activity: Option<WebActivityEvent>,
    tool_events: Vec<WebToolEvent>,
    edited_files: Vec<WebEditedFile>,
    message_blocks: Vec<MessageBlock>,
    partial_text: String,
    progress_updates: Vec<String>,
    recent_progress_keys: Vec<String>,
    recent_progress_emitted_at: HashMap<String, Instant>,
    required_path_snapshots: Vec<RequiredPathSnapshot>,
    subagents: Vec<WebSubagentEvent>,
    verifier: Option<WebVerifierReport>,
    checkpoints: Vec<String>,
    branch_notes: Vec<String>,
    timeline: Vec<WebTimelineEvent>,
}

#[derive(Debug)]
struct PendingApprovalRuntime {
    sender: oneshot::Sender<bool>,
    name: String,
    risk: String,
    args: Value,
}

#[derive(Debug, Serialize)]
struct StreamEnvelope {
    r#type: String,
    session_id: Option<String>,
    messages: Option<Vec<WebMessage>>,
    delta: Option<String>,
    error: Option<String>,
    activity: Option<WebActivityEvent>,
    tool: Option<WebToolEvent>,
    permission: Option<WebPermissionRequest>,
    edited_files: Option<Vec<WebEditedFile>>,
    research: Option<WebResearchPayload>,
    subagents: Option<Vec<WebSubagentEvent>>,
    verifier: Option<WebVerifierReport>,
}

#[derive(Debug, Serialize, Clone, Default)]
struct WebSessionRuntimeSnapshot {
    session_id: String,
    partial_text: String,
    progress_updates: Vec<String>,
    latest_activity: Option<WebActivityEvent>,
    tool_events: Vec<WebToolEvent>,
    edited_files: Vec<WebEditedFile>,
    permission: Option<WebPermissionRequest>,
    subagents: Vec<WebSubagentEvent>,
    verifier: Option<WebVerifierReport>,
    checkpoints: Vec<String>,
    branch_notes: Vec<String>,
    timeline: Vec<WebTimelineEvent>,
}

fn clear_stream_runtime_session(state: &WebAppState, session_id: &str) {
    if let Ok(mut runtime) = lock_stream_runtime(state) {
        runtime.remove(session_id);
    }
}

fn sync_stream_runtime_messages(
    state: &WebAppState,
    session_id: &str,
    persisted_blocks: &[MessageBlock],
) {
    if let Ok(mut sessions) = lock_stream_runtime(state) {
        if let Some(session) = sessions.get_mut(session_id) {
            session.message_blocks = persisted_blocks.to_vec();
        }
    }
}

fn load_stream_required_path_snapshots(state: &WebAppState, session_id: &str) -> Vec<RequiredPathSnapshot> {
    lock_stream_runtime(state)
        .ok()
        .and_then(|sessions| sessions.get(session_id).map(|session| session.required_path_snapshots.clone()))
        .unwrap_or_default()
}

pub async fn start_web_mode(
    host: WebHostConfig,
    assistant_config: AssistantConfig,
    config_file: Config,
    security_config: SecurityConfig,
) -> Result<()> {
    let frontend_dir = host.frontend_dir().to_path_buf();
    let state = build_web_app_state(host.clone(), assistant_config, config_file, security_config)?;

    let addr = host.bind_addr;
    println!("鍚姩 Web Workspace 妯″紡");
    println!("http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    serve_web_listener(listener, state, host.frontend_dir().to_path_buf()).await
}

pub fn build_web_router(state: WebAppState, frontend_dir: PathBuf) -> Router {
    Router::new()
        .route("/api/bootstrap", get(api_bootstrap))
        .route("/api/send", post(api_send_message))
        .route("/api/send-stream", post(api_send_message_stream))
        .route("/api/send-stop", post(api_stop_message_stream))
        .route("/api/tool/approve", post(api_approve_tool_call))
        .route("/api/tool/deny", post(api_deny_tool_call))
        .route("/api/workspace/pick", post(api_pick_workspace))
        .route("/api/review/file", post(api_review_file))
        .route("/api/workspace/file", post(api_workspace_file))
        .route("/api/workspace/file/save", post(api_workspace_file_save))
        .route("/api/workspace/file/undo", post(api_workspace_file_undo))
        .route("/api/workspace/file/complete", post(api_workspace_file_complete))
        .route("/api/workspace/file/raw", get(api_workspace_file_raw))
        .route("/api/sessions", post(api_create_session))
        .route("/api/sessions/select", post(api_select_session))
        .route("/api/sessions/delete", post(api_delete_session))
        .route("/api/sessions/rename", post(api_rename_session))
        .route("/api/settings", post(api_update_settings))
        .route("/api/git", get(api_git_state))
        .route("/api/git/action", post(api_git_action))
        .route("/api/extensions", get(api_extensions))
        .route("/api/run-debug", get(api_run_debug_state))
        .route("/api/run-debug/action", post(api_run_debug_action))
        .route("/api/terminals", get(api_terminals))
        .route("/api/terminals/create", post(api_create_terminal))
        .route("/api/terminals/input", post(api_terminal_input))
        .route("/api/terminals/close", post(api_close_terminal))
        .nest_service("/", ServeDir::new(frontend_dir))
        .with_state(state)
}

pub async fn serve_web_listener(
    listener: tokio::net::TcpListener,
    state: WebAppState,
    frontend_dir: PathBuf,
) -> Result<()> {
    let app = build_web_router(state, frontend_dir);
    axum::serve(listener, app).await?;
    Ok(())
}

pub fn build_web_app_state(
    host: WebHostConfig,
    assistant_config: AssistantConfig,
    config_file: Config,
    security_config: SecurityConfig,
) -> Result<WebAppState> {
    let sandbox_bootstrap = initialize_app_sandbox(&host.paths)?;
    if sandbox_bootstrap.first_run {
        tracing::info!(
            "initialized IDE sandbox at {}",
            sandbox_bootstrap.manifest.sandbox_root
        );
    } else {
        tracing::debug!(
            "reusing IDE sandbox at {}",
            sandbox_bootstrap.manifest.sandbox_root
        );
    }
    let mut security_config = security_config;
    extend_security_allowed_roots(&mut security_config, &host.paths);
    let persisted_state_path = host.persisted_state_path();
    let mut session_manager = SessionManager::from_sessions_dir(host.sessions_dir())?;
    restore_session_selection(&mut session_manager, &persisted_state_path);
    let runtime_settings = initial_runtime_settings(
        &config_file,
        &security_config,
        &assistant_config,
        &persisted_state_path,
        &host.paths,
    );

    let state = WebAppState {
        assistant: Arc::new(Mutex::new(None)),
        assistant_api_url: assistant_config.api_url.clone(),
        assistant_api_key: assistant_config.api_key.clone(),
        runtime_settings: Arc::new(Mutex::new(runtime_settings)),
        persisted_state_path,
        session_manager: Arc::new(Mutex::new(session_manager)),
        base_security_config: security_config,
        run_debug_runtime: Arc::new(Mutex::new(RunDebugRuntime::default())),
        terminal_runtime: Arc::new(Mutex::new(TerminalRuntime::default())),
        stream_runtime: Arc::new(Mutex::new(HashMap::new())),
        host: Arc::new(host),
    };

    {
        let runtime = lock_runtime_settings(&state)?;
        let current_session_id = {
            let session_manager = lock_session_manager(&state)?;
            session_manager.current_id.clone()
        };
        let _ = persist_web_state(&state, &runtime, current_session_id);
    }

    Ok(state)
}

pub async fn dispatch_bridge_command(
    state: WebAppState,
    command: &str,
    payload: Value,
) -> HostBridgeResponse {
    let command = match HostCommand::parse(command) {
        Some(command) => command,
        None => return HostBridgeResponse::error(404, format!("unknown bridge command: {}", command)),
    };

    let result = match command {
        HostCommand::BootstrapLoad => build_bootstrap(&state).map(|bootstrap| json!({ "ok": true, "data": bootstrap })),
        HostCommand::SettingsUpdate => {
            let parsed = parse_bridge_payload::<WebSettingsRequest>(payload);
            match parsed {
                Ok(req) => {
                    let response = bridge_update_settings(&state, req).await;
                    response.map(|value| json!({ "ok": true, "data": value }))
                }
                Err(err) => Err(err),
            }
        }
        HostCommand::WorkspacePick => bridge_pick_workspace(&state).await.map(|value| json!({ "ok": true, "data": value })),
        HostCommand::WorkspaceFileOpen => match parse_bridge_payload::<ReviewFileRequest>(payload) {
            Ok(req) => bridge_workspace_file(&state, req).await.map(|value| json!({ "ok": true, "data": value })),
            Err(err) => Err(err),
        },
        HostCommand::WorkspaceFileSave => match parse_bridge_payload::<WorkspaceFileSaveRequest>(payload) {
            Ok(req) => bridge_workspace_file_save(&state, req).map(|value| json!({ "ok": true, "data": value })),
            Err(err) => Err(err),
        },
        HostCommand::WorkspaceFileUndo => match parse_bridge_payload::<WorkspaceFileUndoRequest>(payload) {
            Ok(req) => bridge_workspace_file_undo(&state, req).map(|value| json!({ "ok": true, "data": value })),
            Err(err) => Err(err),
        },
        HostCommand::WorkspaceFileComplete => match parse_bridge_payload::<WorkspaceFileCompleteRequest>(payload) {
            Ok(req) => bridge_workspace_file_complete(&state, req).await.map(|value| json!({ "ok": true, "data": value })),
            Err(err) => Err(err),
        },
        HostCommand::WorkspaceReviewFile => match parse_bridge_payload::<ReviewFileRequest>(payload) {
            Ok(req) => bridge_review_file(&state, req).await.map(|value| json!({ "ok": true, "data": value })),
            Err(err) => Err(err),
        },
        HostCommand::ChatSend => match parse_bridge_payload::<SendMessageRequest>(payload) {
            Ok(req) => bridge_chat_send(&state, req).await.map(|value| json!({ "ok": true, "data": value })),
            Err(err) => Err(err),
        },
        HostCommand::ChatStream => Err(anyhow!("chat.stream must use dispatch_bridge_stream")),
        HostCommand::ChatStop => match parse_bridge_payload::<StreamControlRequest>(payload) {
            Ok(req) => bridge_stop_message_stream(&state, req).map(|value| json!({ "ok": true, "data": value })),
            Err(err) => Err(err),
        },
        HostCommand::ToolApprovalApprove => match parse_bridge_payload::<ToolApprovalRequest>(payload) {
            Ok(req) => bridge_tool_approval(&state, req, true).map(|value| json!({ "ok": true, "data": value })),
            Err(err) => Err(err),
        },
        HostCommand::ToolApprovalDeny => match parse_bridge_payload::<ToolApprovalRequest>(payload) {
            Ok(req) => bridge_tool_approval(&state, req, false).map(|value| json!({ "ok": true, "data": value })),
            Err(err) => Err(err),
        },
        HostCommand::GitState => match parse_bridge_payload::<GitStateQuery>(payload) {
            Ok(req) => bridge_git_state(&state, req).map(|value| json!({ "ok": true, "data": value })),
            Err(err) => Err(err),
        },
        HostCommand::GitAction => match parse_bridge_payload::<GitActionRequest>(payload) {
            Ok(req) => bridge_git_action(&state, req).map(|value| json!({ "ok": true, "data": value })),
            Err(err) => Err(err),
        },
        HostCommand::ExtensionsList => bridge_extensions_list().map(|value| json!({ "ok": true, "data": value })),
        HostCommand::RunDebugState => bridge_run_debug_state(&state).map(|value| json!({ "ok": true, "data": value })),
        HostCommand::RunDebugAction => match parse_bridge_payload::<RunDebugActionRequest>(payload) {
            Ok(req) => bridge_run_debug_action(&state, req).map(|value| json!({ "ok": true, "data": value })),
            Err(err) => Err(err),
        },
        HostCommand::TerminalsState => bridge_terminals_state(&state).map(|value| json!({ "ok": true, "data": value })),
        HostCommand::TerminalsCreate => bridge_terminals_create(&state).await.map(|value| json!({ "ok": true, "data": value })),
        HostCommand::TerminalsInput => match parse_bridge_payload::<TerminalInputRequest>(payload) {
            Ok(req) => bridge_terminals_input(&state, req).await.map(|value| json!({ "ok": true, "data": value })),
            Err(err) => Err(err),
        },
        HostCommand::TerminalsClose => match parse_bridge_payload::<TerminalCloseRequest>(payload) {
            Ok(req) => bridge_terminals_close(&state, req).await.map(|value| json!({ "ok": true, "data": value })),
            Err(err) => Err(err),
        },
        HostCommand::SessionsCreate => bridge_sessions_create(&state).map(|value| json!({ "ok": true, "data": value })),
        HostCommand::SessionsSelect => match parse_bridge_payload::<SessionSwitchRequest>(payload) {
            Ok(req) => bridge_sessions_select(&state, req).map(|value| json!({ "ok": true, "data": value })),
            Err(err) => Err(err),
        },
        HostCommand::SessionsRename => match parse_bridge_payload::<SessionRenameRequest>(payload) {
            Ok(req) => bridge_sessions_rename(&state, req).map(|value| json!({ "ok": true, "data": value })),
            Err(err) => Err(err),
        },
        HostCommand::SessionsDelete => match parse_bridge_payload::<SessionDeleteRequest>(payload) {
            Ok(req) => bridge_sessions_delete(&state, req).map(|value| json!({ "ok": true, "data": value })),
            Err(err) => Err(err),
        },
    };

    match result {
        Ok(data) => HostBridgeResponse::success(data),
        Err(err) => HostBridgeResponse::error(500, err.to_string()),
    }
}

pub fn dispatch_bridge_stream(
    state: WebAppState,
    command: &str,
    payload: Value,
) -> Result<HostBridgeStream> {
    let command = HostCommand::parse(command)
        .ok_or_else(|| anyhow!("unknown bridge command: {}", command))?;
    if !command.is_stream() {
        return Err(anyhow!("bridge stream is not supported for '{}'", command.as_str()));
    }

    let request = parse_bridge_payload::<SendMessageRequest>(payload)?;
    let session_id = ensure_current_session(&state)?;
    let (tx_json, rx_json) = tokio::sync::mpsc::unbounded_channel::<Value>();
    let (tx_stream, mut rx_stream) = tokio::sync::mpsc::unbounded_channel::<StreamEnvelope>();

    tokio::spawn(async move {
        while let Some(event) = rx_stream.recv().await {
            let value = serde_json::to_value(event)
                .unwrap_or_else(|_| json!({ "type": "error", "error": "serialization failed" }));
            if tx_json.send(value).is_err() {
                break;
            }
        }
    });

    let state_for_task = state.clone();
    let content = request.content;
    let mode = request.mode;
    let language = request.language;
    let recovery_user_content = content.clone();
    let recovery_mode = mode.clone();
    let recovery_language = language.clone();
    let session_id_for_task = session_id.clone();
    let tx_stream_for_runtime = tx_stream.clone();
    let state_for_cleanup = state_for_task.clone();
    let handle = tokio::spawn(async move {
        let result = AssertUnwindSafe(run_chat_request_stream(
            state_for_task,
            session_id_for_task.clone(),
            content,
            mode,
            language,
            tx_stream.clone(),
        ))
        .catch_unwind()
        .await;

        match result {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                if let Ok((runtime, persisted_blocks)) =
                    recover_stream_finalize_context(&state_for_cleanup, &session_id_for_task)
                {
                    if let Some(plan) = recover_plan_for_finalize(
                        &state_for_cleanup,
                        &runtime,
                        &persisted_blocks,
                        &recovery_user_content,
                        recovery_mode.as_deref(),
                        turn_language_from_option(recovery_language.as_deref()),
                    ) {
                        let required_path_snapshots =
                            load_stream_required_path_snapshots(&state_for_cleanup, &session_id_for_task);
                        if let Some((report, checkpoints, branch_notes)) = can_finalize_from_hard_verifier_only(
                            &plan,
                            &persisted_blocks,
                            state_for_cleanup.host.base_dir(),
                            &runtime,
                            &required_path_snapshots,
                            turn_language_from_option(recovery_language.as_deref()),
                        ) {
                            emit_verifier_update(
                                &tx_stream,
                                &state_for_cleanup,
                                &session_id_for_task,
                                &runtime,
                                &persisted_blocks,
                                recovery_mode.as_deref(),
                                &report,
                                &checkpoints,
                                &branch_notes,
                                turn_language_from_option(recovery_language.as_deref()),
                            );
                            let mut finalized_blocks = persisted_blocks.clone();
                            finalized_blocks.push(MessageBlock::Verification { report });
                            let _ = finalize_stream_success(
                                &tx_stream,
                                &state_for_cleanup,
                                &session_id_for_task,
                                &runtime,
                                &finalized_blocks,
                                recovery_mode.as_deref(),
                                turn_language_from_option(recovery_language.as_deref()),
                                &localized_text(
                                    turn_language_from_option(recovery_language.as_deref()),
                                    "本轮 Agent 已在确定性恢复后完成",
                                    "Agent turn finished after deterministic recovery",
                                ),
                            );
                            return;
                        }
                    }
                }
                finalize_stream_failure(
                    &tx_stream,
                    &state_for_cleanup,
                    &session_id_for_task,
                    turn_language_from_option(recovery_language.as_deref()),
                    &err.to_string(),
                );
            }
            Err(payload) => {
                if let Ok((runtime, persisted_blocks)) =
                    recover_stream_finalize_context(&state_for_cleanup, &session_id_for_task)
                {
                    if let Some(plan) = recover_plan_for_finalize(
                        &state_for_cleanup,
                        &runtime,
                        &persisted_blocks,
                        &recovery_user_content,
                        recovery_mode.as_deref(),
                        turn_language_from_option(recovery_language.as_deref()),
                    ) {
                        let required_path_snapshots =
                            load_stream_required_path_snapshots(&state_for_cleanup, &session_id_for_task);
                        if let Some((report, checkpoints, branch_notes)) = can_finalize_from_hard_verifier_only(
                            &plan,
                            &persisted_blocks,
                            state_for_cleanup.host.base_dir(),
                            &runtime,
                            &required_path_snapshots,
                            turn_language_from_option(recovery_language.as_deref()),
                        ) {
                            emit_verifier_update(
                                &tx_stream,
                                &state_for_cleanup,
                                &session_id_for_task,
                                &runtime,
                                &persisted_blocks,
                                recovery_mode.as_deref(),
                                &report,
                                &checkpoints,
                                &branch_notes,
                                turn_language_from_option(recovery_language.as_deref()),
                            );
                            let mut finalized_blocks = persisted_blocks.clone();
                            finalized_blocks.push(MessageBlock::Verification { report });
                            let _ = finalize_stream_success(
                                &tx_stream,
                                &state_for_cleanup,
                                &session_id_for_task,
                                &runtime,
                                &finalized_blocks,
                                recovery_mode.as_deref(),
                                turn_language_from_option(recovery_language.as_deref()),
                                &localized_text(
                                    turn_language_from_option(recovery_language.as_deref()),
                                    "本轮 Agent 已在确定性恢复后完成",
                                    "Agent turn finished after deterministic recovery",
                                ),
                            );
                            return;
                        }
                    }
                }
                finalize_stream_failure(
                    &tx_stream,
                    &state_for_cleanup,
                    &session_id_for_task,
                    turn_language_from_option(recovery_language.as_deref()),
                    &format!("stream task panicked: {}", panic_payload_to_string(payload)),
                );
            }
        }
    });

    {
        let mut runtime = lock_stream_runtime(&state)?;
        runtime.insert(
            session_id.clone(),
            StreamSessionRuntime {
                abort_handle: handle.abort_handle(),
                event_tx: tx_stream_for_runtime,
                pending_approvals: HashMap::new(),
                latest_activity: None,
                tool_events: Vec::new(),
                edited_files: Vec::new(),
                message_blocks: Vec::new(),
                partial_text: String::new(),
                progress_updates: Vec::new(),
                recent_progress_keys: Vec::new(),
                recent_progress_emitted_at: HashMap::new(),
                required_path_snapshots: Vec::new(),
                subagents: Vec::new(),
                verifier: None,
                checkpoints: Vec::new(),
                branch_notes: Vec::new(),
                timeline: Vec::new(),
            },
        );
    }

    Ok(HostBridgeStream {
        command,
        session_id: Some(session_id),
        receiver: rx_json,
    })
}

async fn api_bootstrap(State(state): State<WebAppState>) -> impl IntoResponse {
    match build_bootstrap(&state) {
        Ok(bootstrap) => json_api_response(true, bootstrap).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "ok": false, "error": err.to_string() })),
        )
            .into_response(),
    }
}

async fn api_create_session(
    State(state): State<WebAppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let model = {
        let runtime = lock_runtime_settings(&state).map_err(internal_error)?;
        runtime.model.clone()
    };

    let session_id = {
        let mut session_manager = lock_session_manager(&state).map_err(internal_error)?;
        let meta = session_manager
            .create_session(&model)
            .map_err(internal_error)?;
        let session_id = meta.id.clone();
        session_manager.current_id = Some(session_id.clone());
        session_id
    };
    {
        let runtime = lock_runtime_settings(&state).map_err(internal_error)?;
        persist_web_state(&state, &runtime, Some(session_id.clone())).map_err(internal_error)?;
    }

    Ok(Json(ApiResponse {
        ok: true,
        data: SessionMutationResponse {
            session_id: Some(session_id),
        },
    }))
}

async fn api_select_session(
    State(state): State<WebAppState>,
    Json(payload): Json<SessionSwitchRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    {
        let mut session_manager = lock_session_manager(&state).map_err(internal_error)?;
        session_manager
            .resume_session(&payload.session_id)
            .map_err(internal_error)?;
    }
    {
        let runtime = lock_runtime_settings(&state).map_err(internal_error)?;
        persist_web_state(&state, &runtime, Some(payload.session_id.clone()))
            .map_err(internal_error)?;
    }

    Ok(Json(ApiResponse {
        ok: true,
        data: SessionMutationResponse {
            session_id: Some(payload.session_id),
        },
    }))
}

async fn api_delete_session(
    State(state): State<WebAppState>,
    Json(payload): Json<SessionDeleteRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let next_session_id = {
        let mut session_manager = lock_session_manager(&state).map_err(internal_error)?;
        session_manager
            .delete_session(&payload.session_id)
            .map_err(internal_error)?;

        if session_manager.current_id.as_deref() == Some(payload.session_id.as_str()) {
            session_manager.current_id = preferred_session_id(&session_manager);
        }
        session_manager.current_id.clone()
    };
    {
        let runtime = lock_runtime_settings(&state).map_err(internal_error)?;
        persist_web_state(&state, &runtime, next_session_id.clone()).map_err(internal_error)?;
    }

    Ok(Json(ApiResponse {
        ok: true,
        data: SessionMutationResponse {
            session_id: next_session_id,
        },
    }))
}

async fn api_rename_session(
    State(state): State<WebAppState>,
    Json(payload): Json<SessionRenameRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let current_session_id = {
        let mut session_manager = lock_session_manager(&state).map_err(internal_error)?;
        session_manager
            .rename_session(&payload.session_id, &payload.title)
            .map_err(internal_error)?;
        session_manager.current_id.clone()
    };
    {
        let runtime = lock_runtime_settings(&state).map_err(internal_error)?;
        persist_web_state(&state, &runtime, current_session_id.clone()).map_err(internal_error)?;
    }

    Ok(Json(ApiResponse {
        ok: true,
        data: SessionMutationResponse {
            session_id: current_session_id,
        },
    }))
}

async fn api_update_settings(
    State(state): State<WebAppState>,
    Json(payload): Json<WebSettingsRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let requested_workspace_root = payload.workspace_root.trim().to_string();

    let updated_payload = {
        let mut runtime = lock_runtime_settings(&state).map_err(internal_error)?;
        runtime.api_url = non_empty_or(payload.api_url.trim(), &runtime.api_url);
        runtime.model = non_empty_or(payload.model.trim(), &runtime.model);
        runtime.deep_think = payload.deep_think;
        runtime.reasoning_effort = non_empty_or(payload.reasoning_effort.trim(), &runtime.reasoning_effort);
        runtime.competition_mode = payload.competition_mode;
        runtime.privacy_mode = payload.privacy_mode;
        runtime.workspace_root = resolve_workspace_root(&requested_workspace_root, &runtime.workspace_root)
            .map_err(internal_error)?;
        runtime.api_key = payload
            .api_key
            .and_then(|key| if key.trim().is_empty() { None } else { Some(key) });
        runtime.auto_approve_tools = payload.auto_approve_tools;
        runtime.max_auto_approve_risk =
            parse_risk_level(&payload.max_auto_approve_risk).map_err(internal_error)?;
        runtime.max_tool_calls_per_minute = payload.max_tool_calls_per_minute;
        runtime.burst_limit = payload.burst_limit;
        if let Some(toolchains) = payload.toolchains {
            runtime.toolchains = sanitize_toolchain_paths(toolchains);
        }
        let current_session_id = {
            let session_manager = lock_session_manager(&state).map_err(internal_error)?;
            session_manager.current_id.clone()
        };
        persist_web_state(&state, &runtime, current_session_id).map_err(internal_error)?;
        runtime_to_payload(&runtime)
    };

    {
        let mut assistant_slot = lock_assistant_slot(&state).map_err(internal_error)?;
        *assistant_slot = None;
    }

    Ok(Json(ApiResponse {
        ok: true,
        data: SettingsMutationResponse {
            config: updated_payload,
        },
    }))
}

async fn api_pick_workspace(
    State(state): State<WebAppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let current_root = {
        let runtime = lock_runtime_settings(&state).map_err(internal_error)?;
        runtime.workspace_root.clone()
    };
    let dialog_root = current_root.clone();
    let base_dir = state.host.base_dir().to_path_buf();

    let picked = tokio::task::spawn_blocking(move || {
        let mut dialog = rfd::FileDialog::new();
        if let Ok(dir) = canonical_workspace_dir_from(&base_dir, &dialog_root) {
            dialog = dialog.set_directory(dir);
        }
        dialog.pick_folder()
    })
    .await
    .map_err(|err| internal_error(anyhow!("workspace picker task failed: {}", err)))?;

    let picked = picked.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "workspace selection cancelled".to_string(),
        )
    })?;

    let resolved = resolve_workspace_root(&picked.display().to_string(), &current_root)
        .map_err(internal_error)?;

    {
        let mut runtime = lock_runtime_settings(&state).map_err(internal_error)?;
        runtime.workspace_root = resolved.clone();
        let current_session_id = {
            let session_manager = lock_session_manager(&state).map_err(internal_error)?;
            session_manager.current_id.clone()
        };
        persist_web_state(&state, &runtime, current_session_id).map_err(internal_error)?;
    }

    {
        let mut assistant_slot = lock_assistant_slot(&state).map_err(internal_error)?;
        *assistant_slot = None;
    }
    {
        let mut run_debug_runtime = state
            .run_debug_runtime
            .lock()
            .map_err(|_| internal_error(anyhow!("failed to lock run/debug runtime")))?;
        if let Some(session) = run_debug_runtime.active.take() {
            stop_run_debug_session(&session);
        }
    }
    {
        let sessions = {
            let mut terminal_runtime = state
                .terminal_runtime
                .lock()
                .map_err(|_| internal_error(anyhow!("failed to lock terminal runtime")))?;
            terminal_runtime.active_id = None;
            std::mem::take(&mut terminal_runtime.sessions)
        };
        for session in sessions {
            if let Ok(mut child) = session.child.lock() {
                let _ = child.kill();
            }
        }
    }

    Ok(Json(ApiResponse {
        ok: true,
        data: WorkspacePickerResponse {
            workspace_root: resolved,
        },
    }))
}

async fn api_review_file(
    State(state): State<WebAppState>,
    Json(payload): Json<ReviewFileRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let (workspace_root, messages, runtime_paths) = {
        let runtime = lock_runtime_settings(&state).map_err(internal_error)?;
        let workspace_root = runtime.workspace_root.clone();
        drop(runtime);

        let current_id = {
            let session_manager = lock_session_manager(&state).map_err(internal_error)?;
            session_manager.current_id.clone()
        };

        let messages = if let Some(ref id) = current_id {
            let session_manager = lock_session_manager(&state).map_err(internal_error)?;
            session_manager.load_messages(id).map_err(internal_error)?
        } else {
            Vec::new()
        };

        let runtime_paths = current_id
            .as_ref()
            .and_then(|id| lock_stream_runtime(&state).ok().and_then(|sessions| {
                sessions.get(id).map(|session| {
                    session
                        .edited_files
                        .iter()
                        .map(|file| file.path.clone())
                        .collect::<Vec<_>>()
                })
            }))
            .unwrap_or_default();

        (workspace_root, messages, runtime_paths)
    };

    let touched_files = collect_review_paths_for_current_turn(&messages, &runtime_paths);
    if !touched_files.iter().any(|path| path == &payload.path) {
        return Err((
            StatusCode::NOT_FOUND,
            format!("review file is not part of the current agent turn: {}", payload.path),
        ));
    }

    let detail = build_review_file_detail(state.host.base_dir(), &workspace_root, &payload.path).map_err(internal_error)?;

    Ok(Json(ApiResponse {
        ok: true,
        data: ReviewFileResponse { file: detail },
    }))
}

async fn api_workspace_file(
    State(state): State<WebAppState>,
    Json(payload): Json<ReviewFileRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let workspace_root = {
        let runtime = lock_runtime_settings(&state).map_err(internal_error)?;
        runtime.workspace_root.clone()
    };

    let file = build_workspace_file_response(state.host.base_dir(), &workspace_root, &payload.path).map_err(internal_error)?;

    Ok(Json(ApiResponse {
        ok: true,
        data: WorkspaceFileEnvelope { file },
    }))
}

async fn api_workspace_file_save(
    State(state): State<WebAppState>,
    Json(payload): Json<WorkspaceFileSaveRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let file = save_workspace_file(&state, &payload.path, &payload.content).map_err(internal_error)?;

    Ok(Json(ApiResponse {
        ok: true,
        data: WorkspaceFileSaveResponse { file },
    }))
}

async fn api_workspace_file_undo(
    State(state): State<WebAppState>,
    Json(payload): Json<WorkspaceFileUndoRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let file = save_workspace_file(&state, &payload.path, &payload.before_content).map_err(internal_error)?;

    Ok(Json(ApiResponse {
        ok: true,
        data: WorkspaceFileSaveResponse { file },
    }))
}

fn save_workspace_file(
    state: &WebAppState,
    path: &str,
    content: &str,
) -> Result<WebWorkspaceFileResponse> {
    let workspace_root = {
        let runtime = lock_runtime_settings(state)?;
        runtime.workspace_root.clone()
    };

    let workspace = canonical_workspace_dir_from(state.host.base_dir(), &workspace_root)?;
    let relative_path = sanitize_review_path(path)?;
    let absolute = workspace.join(&relative_path);

    if !absolute.exists() {
        return Err(anyhow!("workspace file does not exist: {}", relative_path));
    }
    if !absolute.is_file() {
        return Err(anyhow!("workspace path is not a file: {}", relative_path));
    }

    std::fs::write(&absolute, content.as_bytes()).map_err(|err| {
        anyhow!(
            "failed to save workspace file '{}': {}",
            absolute.display(),
            err
        )
    })?;

    build_workspace_file_response(state.host.base_dir(), &workspace_root, &relative_path)
}

async fn api_workspace_file_complete(
    State(state): State<WebAppState>,
    Json(payload): Json<WorkspaceFileCompleteRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let runtime = {
        let runtime = lock_runtime_settings(&state).map_err(internal_error)?;
        runtime.clone()
    };

    let workspace_root = runtime.workspace_root.trim().to_string();
    if workspace_root.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "workspace root is empty".to_string()));
    }

    let workspace = canonical_workspace_dir_from(state.host.base_dir(), &workspace_root).map_err(internal_error)?;
    let relative_path = sanitize_review_path(&payload.path).map_err(internal_error)?;
    let absolute = workspace.join(&relative_path);
    if !absolute.exists() {
        return Err((StatusCode::NOT_FOUND, format!("workspace file does not exist: {}", relative_path)));
    }
    if !absolute.is_file() {
        return Err((StatusCode::BAD_REQUEST, format!("workspace path is not a file: {}", relative_path)));
    }

    let provider = build_streaming_provider(&state, &runtime).map_err(internal_error)?;
    let request = build_workspace_completion_request(&runtime, &payload);
    let response = provider.chat(request).await.map_err(internal_error)?;
    let items = parse_workspace_completion_items(&response.content).unwrap_or_default();

    Ok(Json(ApiResponse {
        ok: true,
        data: WorkspaceFileCompleteResponse { items },
    }))
}

async fn api_workspace_file_raw(
    State(state): State<WebAppState>,
    Query(query): Query<WorkspaceRawFileQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let workspace_root = {
        let runtime = lock_runtime_settings(&state).map_err(internal_error)?;
        runtime.workspace_root.clone()
    };

    let workspace = canonical_workspace_dir_from(state.host.base_dir(), &workspace_root).map_err(internal_error)?;
    let relative_path = sanitize_review_path(&query.path).map_err(internal_error)?;
    let absolute = workspace.join(&relative_path);

    if !absolute.exists() {
        return Err((StatusCode::NOT_FOUND, format!("workspace file does not exist: {}", relative_path)));
    }
    if !absolute.is_file() {
        return Err((StatusCode::BAD_REQUEST, format!("workspace path is not a file: {}", relative_path)));
    }

    let bytes = std::fs::read(&absolute)
        .map_err(|err| internal_error(anyhow!("failed to read workspace file '{}': {}", absolute.display(), err)))?;
    let mime_type = workspace_mime_type(&relative_path);

    Ok((
        [(header::CONTENT_TYPE, mime_type)],
        bytes,
    ))
}

async fn api_git_state(
    State(state): State<WebAppState>,
    Query(query): Query<GitStateQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let workspace_root = {
        let runtime = lock_runtime_settings(&state).map_err(internal_error)?;
        runtime.workspace_root.clone()
    };

    let git = build_git_payload(
        state.host.base_dir(),
        &workspace_root,
        query.diff.unwrap_or(false),
        query.graph.unwrap_or(false),
    )
    .map_err(internal_error)?;

    Ok(Json(ApiResponse {
        ok: true,
        data: GitActionResponse { git },
    }))
}

async fn api_git_action(
    State(state): State<WebAppState>,
    Json(payload): Json<GitActionRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let workspace_root = {
        let runtime = lock_runtime_settings(&state).map_err(internal_error)?;
        runtime.workspace_root.clone()
    };

    run_git_action(state.host.base_dir(), &workspace_root, &payload).map_err(internal_error)?;
    let git = build_git_payload(
        state.host.base_dir(),
        &workspace_root,
        payload.diff.unwrap_or(false),
        payload.graph.unwrap_or(false),
    )
    .map_err(internal_error)?;

    Ok(Json(ApiResponse {
        ok: true,
        data: GitActionResponse { git },
    }))
}

async fn api_extensions(
    State(_state): State<WebAppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let extensions = build_extensions_payload().map_err(internal_error)?;
    Ok(Json(ApiResponse {
        ok: true,
        data: ExtensionsEnvelope { extensions },
    }))
}

async fn api_run_debug_state(
    State(state): State<WebAppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let workspace_root = {
        let runtime = lock_runtime_settings(&state).map_err(internal_error)?;
        runtime.workspace_root.clone()
    };
    let run_debug = build_run_debug_payload(&state, &workspace_root).map_err(internal_error)?;
    Ok(Json(ApiResponse {
        ok: true,
        data: RunDebugEnvelope { run_debug },
    }))
}

async fn api_run_debug_action(
    State(state): State<WebAppState>,
    Json(payload): Json<RunDebugActionRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let workspace_root = {
        let runtime = lock_runtime_settings(&state).map_err(internal_error)?;
        runtime.workspace_root.clone()
    };
    run_debug_action(&state, &workspace_root, &payload).map_err(internal_error)?;
    let run_debug = build_run_debug_payload(&state, &workspace_root).map_err(internal_error)?;
    Ok(Json(ApiResponse {
        ok: true,
        data: RunDebugEnvelope { run_debug },
    }))
}

async fn api_terminals(
    State(state): State<WebAppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let terminals = build_terminal_payload(&state).map_err(internal_error)?;
    Ok(Json(ApiResponse {
        ok: true,
        data: TerminalEnvelope { terminals },
    }))
}

async fn api_create_terminal(
    State(state): State<WebAppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let workspace_root = {
        let runtime = lock_runtime_settings(&state).map_err(internal_error)?;
        runtime.workspace_root.clone()
    };
    create_terminal_session(&state, &workspace_root)
        .await
        .map_err(internal_error)?;
    let terminals = build_terminal_payload(&state).map_err(internal_error)?;
    Ok(Json(ApiResponse {
        ok: true,
        data: TerminalEnvelope { terminals },
    }))
}

async fn api_terminal_input(
    State(state): State<WebAppState>,
    Json(payload): Json<TerminalInputRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    write_terminal_input(&state, &payload.terminal_id, &payload.input)
        .await
        .map_err(internal_error)?;
    let terminals = build_terminal_payload(&state).map_err(internal_error)?;
    Ok(Json(ApiResponse {
        ok: true,
        data: TerminalEnvelope { terminals },
    }))
}

async fn api_close_terminal(
    State(state): State<WebAppState>,
    Json(payload): Json<TerminalCloseRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    close_terminal_session(&state, &payload.terminal_id)
        .await
        .map_err(internal_error)?;
    let terminals = build_terminal_payload(&state).map_err(internal_error)?;
    Ok(Json(ApiResponse {
        ok: true,
        data: TerminalEnvelope { terminals },
    }))
}

async fn api_send_message(
    State(state): State<WebAppState>,
    Json(payload): Json<SendMessageRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let (current_id, next_blocks) =
        run_chat_request(
            &state,
            payload.content.clone(),
            payload.mode.clone(),
            payload.language.clone(),
        )
        .await
        .map_err(internal_error)?;

    Ok(Json(ApiResponse {
        ok: true,
        data: SendMessageResponse {
            messages: messages_to_web(&next_blocks),
            session_id: Some(current_id),
        },
    }))
}

async fn api_send_message_stream(
    State(state): State<WebAppState>,
    Json(payload): Json<SendMessageRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let session_id = ensure_current_session(&state).map_err(internal_error)?;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<StreamEnvelope>();
    let state_for_task = state.clone();
    let content = payload.content;
    let mode = payload.mode;
    let language = payload.language;
    let recovery_user_content = content.clone();
    let recovery_mode = mode.clone();
    let recovery_language = language.clone();
    let session_id_for_task = session_id.clone();
    let tx_for_runtime = tx.clone();
    let state_for_cleanup = state_for_task.clone();
    let handle = tokio::spawn(async move {
        let result = AssertUnwindSafe(run_chat_request_stream(
            state_for_task,
            session_id_for_task.clone(),
            content,
            mode,
            language,
            tx.clone(),
        ))
        .catch_unwind()
        .await;

        match result {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                if let Ok((runtime, persisted_blocks)) =
                    recover_stream_finalize_context(&state_for_cleanup, &session_id_for_task)
                {
                    if let Some(plan) = recover_plan_for_finalize(
                        &state_for_cleanup,
                        &runtime,
                        &persisted_blocks,
                        &recovery_user_content,
                        recovery_mode.as_deref(),
                        turn_language_from_option(recovery_language.as_deref()),
                    ) {
                        let required_path_snapshots =
                            load_stream_required_path_snapshots(&state_for_cleanup, &session_id_for_task);
                        if let Some((report, checkpoints, branch_notes)) = can_finalize_from_hard_verifier_only(
                            &plan,
                            &persisted_blocks,
                            state_for_cleanup.host.base_dir(),
                            &runtime,
                            &required_path_snapshots,
                            turn_language_from_option(recovery_language.as_deref()),
                        ) {
                            emit_verifier_update(
                                &tx,
                                &state_for_cleanup,
                                &session_id_for_task,
                                &runtime,
                                &persisted_blocks,
                                recovery_mode.as_deref(),
                                &report,
                                &checkpoints,
                                &branch_notes,
                                turn_language_from_option(recovery_language.as_deref()),
                            );
                            let mut finalized_blocks = persisted_blocks.clone();
                            finalized_blocks.push(MessageBlock::Verification { report });
                            let _ = finalize_stream_success(
                                &tx,
                                &state_for_cleanup,
                                &session_id_for_task,
                                &runtime,
                                &finalized_blocks,
                                recovery_mode.as_deref(),
                                turn_language_from_option(recovery_language.as_deref()),
                                &localized_text(
                                    turn_language_from_option(recovery_language.as_deref()),
                                    "本轮 Agent 已在确定性恢复后完成",
                                    "Agent turn finished after deterministic recovery",
                                ),
                            );
                            return;
                        }
                    }
                }
                finalize_stream_failure(
                    &tx,
                    &state_for_cleanup,
                    &session_id_for_task,
                    turn_language_from_option(recovery_language.as_deref()),
                    &err.to_string(),
                );
            }
            Err(payload) => {
                if let Ok((runtime, persisted_blocks)) =
                    recover_stream_finalize_context(&state_for_cleanup, &session_id_for_task)
                {
                    if let Some(plan) = recover_plan_for_finalize(
                        &state_for_cleanup,
                        &runtime,
                        &persisted_blocks,
                        &recovery_user_content,
                        recovery_mode.as_deref(),
                        turn_language_from_option(recovery_language.as_deref()),
                    ) {
                        let required_path_snapshots =
                            load_stream_required_path_snapshots(&state_for_cleanup, &session_id_for_task);
                        if let Some((report, checkpoints, branch_notes)) = can_finalize_from_hard_verifier_only(
                            &plan,
                            &persisted_blocks,
                            state_for_cleanup.host.base_dir(),
                            &runtime,
                            &required_path_snapshots,
                            turn_language_from_option(recovery_language.as_deref()),
                        ) {
                            emit_verifier_update(
                                &tx,
                                &state_for_cleanup,
                                &session_id_for_task,
                                &runtime,
                                &persisted_blocks,
                                recovery_mode.as_deref(),
                                &report,
                                &checkpoints,
                                &branch_notes,
                                turn_language_from_option(recovery_language.as_deref()),
                            );
                            let mut finalized_blocks = persisted_blocks.clone();
                            finalized_blocks.push(MessageBlock::Verification { report });
                            let _ = finalize_stream_success(
                                &tx,
                                &state_for_cleanup,
                                &session_id_for_task,
                                &runtime,
                                &finalized_blocks,
                                recovery_mode.as_deref(),
                                turn_language_from_option(recovery_language.as_deref()),
                                &localized_text(
                                    turn_language_from_option(recovery_language.as_deref()),
                                    "本轮 Agent 已在确定性恢复后完成",
                                    "Agent turn finished after deterministic recovery",
                                ),
                            );
                            return;
                        }
                    }
                }
                finalize_stream_failure(
                    &tx,
                    &state_for_cleanup,
                    &session_id_for_task,
                    turn_language_from_option(recovery_language.as_deref()),
                    &format!("stream task panicked: {}", panic_payload_to_string(payload)),
                );
            }
        }
    });

    {
        let mut runtime = lock_stream_runtime(&state).map_err(internal_error)?;
        runtime.insert(
            session_id.clone(),
            StreamSessionRuntime {
                abort_handle: handle.abort_handle(),
                event_tx: tx_for_runtime,
                pending_approvals: HashMap::new(),
                latest_activity: None,
                tool_events: Vec::new(),
                edited_files: Vec::new(),
                message_blocks: Vec::new(),
                partial_text: String::new(),
                progress_updates: Vec::new(),
                recent_progress_keys: Vec::new(),
                recent_progress_emitted_at: HashMap::new(),
                required_path_snapshots: Vec::new(),
                subagents: Vec::new(),
                verifier: None,
                checkpoints: Vec::new(),
                branch_notes: Vec::new(),
                timeline: Vec::new(),
            },
        );
    }

    let stream = async_stream::stream! {
        while let Some(event) = rx.recv().await {
            yield Ok::<Bytes, Infallible>(event_bytes(event));
        }
    };

    Ok((
        [
            (header::CONTENT_TYPE, "application/x-ndjson; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        Body::from_stream(stream),
    ))
}

async fn api_stop_message_stream(
    State(state): State<WebAppState>,
    Json(payload): Json<StreamControlRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let session_id = resolve_stream_session_id(&state, payload.session_id).map_err(internal_error)?;
    stop_stream_session(&state, &session_id).map_err(internal_error)?;
    Ok(Json(ApiResponse {
        ok: true,
        data: SessionMutationResponse {
            session_id: Some(session_id),
        },
    }))
}

async fn api_approve_tool_call(
    State(state): State<WebAppState>,
    Json(payload): Json<ToolApprovalRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    respond_to_tool_permission(&state, &payload.session_id, &payload.call_id, true)
        .map_err(internal_error)?;
    Ok(Json(ApiResponse {
        ok: true,
        data: SessionMutationResponse {
            session_id: Some(payload.session_id),
        },
    }))
}

async fn api_deny_tool_call(
    State(state): State<WebAppState>,
    Json(payload): Json<ToolApprovalRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    respond_to_tool_permission(&state, &payload.session_id, &payload.call_id, false)
        .map_err(internal_error)?;
    Ok(Json(ApiResponse {
        ok: true,
        data: SessionMutationResponse {
            session_id: Some(payload.session_id),
        },
    }))
}

fn parse_bridge_payload<T: DeserializeOwned>(payload: Value) -> Result<T> {
    if payload.is_null() {
        return Err(anyhow!("bridge payload is empty"));
    }
    serde_json::from_value(payload).map_err(|err| anyhow!("failed to parse bridge payload: {}", err))
}

async fn bridge_update_settings(
    state: &WebAppState,
    payload: WebSettingsRequest,
) -> Result<SettingsMutationResponse> {
    let requested_workspace_root = payload.workspace_root.trim().to_string();

    let updated_payload = {
        let mut runtime = lock_runtime_settings(state)?;
        runtime.api_url = non_empty_or(payload.api_url.trim(), &runtime.api_url);
        runtime.model = non_empty_or(payload.model.trim(), &runtime.model);
        runtime.deep_think = payload.deep_think;
        runtime.reasoning_effort = non_empty_or(payload.reasoning_effort.trim(), &runtime.reasoning_effort);
        runtime.competition_mode = payload.competition_mode;
        runtime.privacy_mode = payload.privacy_mode;
        runtime.workspace_root = resolve_workspace_root(&requested_workspace_root, &runtime.workspace_root)?;
        runtime.api_key = payload
            .api_key
            .and_then(|key| if key.trim().is_empty() { None } else { Some(key) });
        runtime.auto_approve_tools = payload.auto_approve_tools;
        runtime.max_auto_approve_risk = parse_risk_level(&payload.max_auto_approve_risk)?;
        runtime.max_tool_calls_per_minute = payload.max_tool_calls_per_minute;
        runtime.burst_limit = payload.burst_limit;
        if let Some(toolchains) = payload.toolchains {
            runtime.toolchains = sanitize_toolchain_paths(toolchains);
        }
        let current_session_id = {
            let session_manager = lock_session_manager(state)?;
            session_manager.current_id.clone()
        };
        persist_web_state(state, &runtime, current_session_id)?;
        runtime_to_payload(&runtime)
    };

    {
        let mut assistant_slot = lock_assistant_slot(state)?;
        *assistant_slot = None;
    }

    Ok(SettingsMutationResponse {
        config: updated_payload,
    })
}

async fn bridge_pick_workspace(state: &WebAppState) -> Result<WorkspacePickerResponse> {
    let current_root = {
        let runtime = lock_runtime_settings(state)?;
        runtime.workspace_root.clone()
    };
    let dialog_root = current_root.clone();
    let base_dir = state.host.base_dir().to_path_buf();

    let picked = tokio::task::spawn_blocking(move || {
        let mut dialog = rfd::FileDialog::new();
        if let Ok(dir) = canonical_workspace_dir_from(&base_dir, &dialog_root) {
            dialog = dialog.set_directory(dir);
        }
        dialog.pick_folder()
    })
    .await
    .map_err(|err| anyhow!("workspace picker task failed: {}", err))?
    .ok_or_else(|| anyhow!("workspace selection cancelled"))?;

    let resolved = resolve_workspace_root(&picked.display().to_string(), &current_root)?;

    {
        let mut runtime = lock_runtime_settings(state)?;
        runtime.workspace_root = resolved.clone();
        let current_session_id = {
            let session_manager = lock_session_manager(state)?;
            session_manager.current_id.clone()
        };
        persist_web_state(state, &runtime, current_session_id)?;
    }

    {
        let mut assistant_slot = lock_assistant_slot(state)?;
        *assistant_slot = None;
    }
    {
        let mut run_debug_runtime = state
            .run_debug_runtime
            .lock()
            .map_err(|_| anyhow!("failed to lock run/debug runtime"))?;
        if let Some(session) = run_debug_runtime.active.take() {
            stop_run_debug_session(&session);
        }
    }
    {
        let sessions = {
            let mut terminal_runtime = state
                .terminal_runtime
                .lock()
                .map_err(|_| anyhow!("failed to lock terminal runtime"))?;
            terminal_runtime.active_id = None;
            std::mem::take(&mut terminal_runtime.sessions)
        };
        for session in sessions {
            if let Ok(mut child) = session.child.lock() {
                let _ = child.kill();
            }
        }
    }

    Ok(WorkspacePickerResponse {
        workspace_root: resolved,
    })
}

async fn bridge_review_file(state: &WebAppState, payload: ReviewFileRequest) -> Result<ReviewFileResponse> {
    let (workspace_root, messages, runtime_paths) = {
        let runtime = lock_runtime_settings(state)?;
        let workspace_root = runtime.workspace_root.clone();
        drop(runtime);

        let current_id = {
            let session_manager = lock_session_manager(state)?;
            session_manager.current_id.clone()
        };

        let messages = if let Some(ref id) = current_id {
            let session_manager = lock_session_manager(state)?;
            session_manager.load_messages(id)?
        } else {
            Vec::new()
        };

        let runtime_paths = current_id
            .as_ref()
            .and_then(|id| lock_stream_runtime(state).ok().and_then(|sessions| {
                sessions.get(id).map(|session| {
                    session
                        .edited_files
                        .iter()
                        .map(|file| file.path.clone())
                        .collect::<Vec<_>>()
                })
            }))
            .unwrap_or_default();

        (workspace_root, messages, runtime_paths)
    };

    let touched_files = collect_review_paths_for_current_turn(&messages, &runtime_paths);
    if !touched_files.iter().any(|path| path == &payload.path) {
        return Err(anyhow!(
            "review file is not part of the current agent turn: {}",
            payload.path
        ));
    }

    let detail = build_review_file_detail(state.host.base_dir(), &workspace_root, &payload.path)?;
    Ok(ReviewFileResponse { file: detail })
}

async fn bridge_workspace_file(state: &WebAppState, payload: ReviewFileRequest) -> Result<WorkspaceFileEnvelope> {
    let workspace_root = {
        let runtime = lock_runtime_settings(state)?;
        runtime.workspace_root.clone()
    };
    let file = build_workspace_file_response(state.host.base_dir(), &workspace_root, &payload.path)?;
    Ok(WorkspaceFileEnvelope { file })
}

fn bridge_workspace_file_save(
    state: &WebAppState,
    payload: WorkspaceFileSaveRequest,
) -> Result<WorkspaceFileSaveResponse> {
    let file = save_workspace_file(state, &payload.path, &payload.content)?;
    Ok(WorkspaceFileSaveResponse { file })
}

fn bridge_workspace_file_undo(
    state: &WebAppState,
    payload: WorkspaceFileUndoRequest,
) -> Result<WorkspaceFileSaveResponse> {
    let file = save_workspace_file(state, &payload.path, &payload.before_content)?;
    Ok(WorkspaceFileSaveResponse { file })
}

async fn bridge_workspace_file_complete(
    state: &WebAppState,
    payload: WorkspaceFileCompleteRequest,
) -> Result<WorkspaceFileCompleteResponse> {
    let runtime = {
        let runtime = lock_runtime_settings(state)?;
        runtime.clone()
    };

    let workspace_root = runtime.workspace_root.trim().to_string();
    if workspace_root.is_empty() {
        return Err(anyhow!("workspace root is empty"));
    }

    let workspace = canonical_workspace_dir_from(state.host.base_dir(), &workspace_root)?;
    let relative_path = sanitize_review_path(&payload.path)?;
    let absolute = workspace.join(&relative_path);
    if !absolute.exists() {
        return Err(anyhow!("workspace file does not exist: {}", relative_path));
    }
    if !absolute.is_file() {
        return Err(anyhow!("workspace path is not a file: {}", relative_path));
    }

    let provider = build_streaming_provider(state, &runtime)?;
    let request = build_workspace_completion_request(&runtime, &payload);
    let response = provider.chat(request).await?;
    let items = parse_workspace_completion_items(&response.content).unwrap_or_default();
    Ok(WorkspaceFileCompleteResponse { items })
}

async fn bridge_chat_send(state: &WebAppState, payload: SendMessageRequest) -> Result<SendMessageResponse> {
    let (current_id, next_blocks) =
        run_chat_request(
            state,
            payload.content.clone(),
            payload.mode.clone(),
            payload.language.clone(),
        )
        .await?;

    Ok(SendMessageResponse {
        messages: messages_to_web(&next_blocks),
        session_id: Some(current_id),
    })
}

fn bridge_stop_message_stream(state: &WebAppState, payload: StreamControlRequest) -> Result<SessionMutationResponse> {
    let session_id = resolve_stream_session_id(state, payload.session_id)?;
    stop_stream_session(state, &session_id)?;
    Ok(SessionMutationResponse {
        session_id: Some(session_id),
    })
}

fn bridge_tool_approval(
    state: &WebAppState,
    payload: ToolApprovalRequest,
    approved: bool,
) -> Result<SessionMutationResponse> {
    respond_to_tool_permission(state, &payload.session_id, &payload.call_id, approved)?;
    Ok(SessionMutationResponse {
        session_id: Some(payload.session_id),
    })
}

fn bridge_git_state(state: &WebAppState, query: GitStateQuery) -> Result<GitActionResponse> {
    let workspace_root = {
        let runtime = lock_runtime_settings(state)?;
        runtime.workspace_root.clone()
    };
    Ok(GitActionResponse {
        git: build_git_payload(
            state.host.base_dir(),
            &workspace_root,
            query.diff.unwrap_or(false),
            query.graph.unwrap_or(false),
        )?,
    })
}

fn bridge_git_action(state: &WebAppState, payload: GitActionRequest) -> Result<GitActionResponse> {
    let workspace_root = {
        let runtime = lock_runtime_settings(state)?;
        runtime.workspace_root.clone()
    };

    run_git_action(state.host.base_dir(), &workspace_root, &payload)?;
    Ok(GitActionResponse {
        git: build_git_payload(
            state.host.base_dir(),
            &workspace_root,
            payload.diff.unwrap_or(false),
            payload.graph.unwrap_or(false),
        )?,
    })
}

fn bridge_extensions_list() -> Result<ExtensionsEnvelope> {
    Ok(ExtensionsEnvelope {
        extensions: build_extensions_payload()?,
    })
}

fn bridge_run_debug_state(state: &WebAppState) -> Result<RunDebugEnvelope> {
    let workspace_root = {
        let runtime = lock_runtime_settings(state)?;
        runtime.workspace_root.clone()
    };
    Ok(RunDebugEnvelope {
        run_debug: build_run_debug_payload(state, &workspace_root)?,
    })
}

fn bridge_run_debug_action(state: &WebAppState, payload: RunDebugActionRequest) -> Result<RunDebugEnvelope> {
    let workspace_root = {
        let runtime = lock_runtime_settings(state)?;
        runtime.workspace_root.clone()
    };
    run_debug_action(state, &workspace_root, &payload)?;
    Ok(RunDebugEnvelope {
        run_debug: build_run_debug_payload(state, &workspace_root)?,
    })
}

fn bridge_terminals_state(state: &WebAppState) -> Result<TerminalEnvelope> {
    Ok(TerminalEnvelope {
        terminals: build_terminal_payload(state)?,
    })
}

async fn bridge_terminals_create(state: &WebAppState) -> Result<TerminalEnvelope> {
    let workspace_root = {
        let runtime = lock_runtime_settings(state)?;
        runtime.workspace_root.clone()
    };
    create_terminal_session(state, &workspace_root).await?;
    Ok(TerminalEnvelope {
        terminals: build_terminal_payload(state)?,
    })
}

async fn bridge_terminals_input(
    state: &WebAppState,
    payload: TerminalInputRequest,
) -> Result<TerminalEnvelope> {
    write_terminal_input(state, &payload.terminal_id, &payload.input).await?;
    Ok(TerminalEnvelope {
        terminals: build_terminal_payload(state)?,
    })
}

async fn bridge_terminals_close(
    state: &WebAppState,
    payload: TerminalCloseRequest,
) -> Result<TerminalEnvelope> {
    close_terminal_session(state, &payload.terminal_id).await?;
    Ok(TerminalEnvelope {
        terminals: build_terminal_payload(state)?,
    })
}

fn bridge_sessions_create(state: &WebAppState) -> Result<SessionMutationResponse> {
    let model = {
        let runtime = lock_runtime_settings(state)?;
        runtime.model.clone()
    };
    let session_id = {
        let mut session_manager = lock_session_manager(state)?;
        let meta = session_manager.create_session(&model)?;
        let session_id = meta.id.clone();
        session_manager.current_id = Some(session_id.clone());
        session_id
    };
    {
        let runtime = lock_runtime_settings(state)?;
        persist_web_state(state, &runtime, Some(session_id.clone()))?;
    }
    Ok(SessionMutationResponse {
        session_id: Some(session_id),
    })
}

fn bridge_sessions_select(state: &WebAppState, payload: SessionSwitchRequest) -> Result<SessionMutationResponse> {
    {
        let mut session_manager = lock_session_manager(state)?;
        session_manager.resume_session(&payload.session_id)?;
    }
    {
        let runtime = lock_runtime_settings(state)?;
        persist_web_state(state, &runtime, Some(payload.session_id.clone()))?;
    }
    Ok(SessionMutationResponse {
        session_id: Some(payload.session_id),
    })
}

fn bridge_sessions_rename(state: &WebAppState, payload: SessionRenameRequest) -> Result<SessionMutationResponse> {
    let current_session_id = {
        let mut session_manager = lock_session_manager(state)?;
        session_manager.rename_session(&payload.session_id, &payload.title)?;
        session_manager.current_id.clone()
    };
    {
        let runtime = lock_runtime_settings(state)?;
        persist_web_state(state, &runtime, current_session_id.clone())?;
    }
    Ok(SessionMutationResponse {
        session_id: current_session_id,
    })
}

fn bridge_sessions_delete(state: &WebAppState, payload: SessionDeleteRequest) -> Result<SessionMutationResponse> {
    let next_session_id = {
        let mut session_manager = lock_session_manager(state)?;
        session_manager.delete_session(&payload.session_id)?;
        if session_manager.current_id.as_deref() == Some(payload.session_id.as_str()) {
            session_manager.current_id = preferred_session_id(&session_manager);
        }
        session_manager.current_id.clone()
    };
    {
        let runtime = lock_runtime_settings(state)?;
        persist_web_state(state, &runtime, next_session_id.clone())?;
    }
    Ok(SessionMutationResponse {
        session_id: next_session_id,
    })
}

async fn run_chat_request(
    state: &WebAppState,
    content: String,
    mode: Option<String>,
    language: Option<String>,
) -> Result<(String, Vec<MessageBlock>)> {
    let current_id = ensure_current_session(state)?;

    let existing_blocks = {
        let session_manager = lock_session_manager(state)?;
        session_manager.load_messages(&current_id)?
    };

    let turn_language = turn_language_from_option(language.as_deref());
    let system_prompt = format!(
        "{}\n\nLanguage policy:\n- Respond in {} for this turn.\n- Keep planner, verifier, repair, and final summaries in {} as well.",
        system_prompt_for_mode(mode.as_deref()),
        turn_language_name(turn_language),
        turn_language_name(turn_language),
    );
    let mut llm_messages = vec![json!({
        "role": "system",
        "content": system_prompt
    })];

    for block in &existing_blocks {
        match block {
            MessageBlock::User { content, .. } => {
                llm_messages.push(json!({ "role": "user", "content": content }));
            }
            MessageBlock::Assistant { content } => {
                llm_messages.push(json!({ "role": "assistant", "content": content }));
            }
            _ => {}
        }
    }

    llm_messages.push(json!({
        "role": "user",
        "content": content
    }));

    let runtime_for_chat = {
        let runtime = lock_runtime_settings(state)?;
        runtime.clone()
    };
    let assistant_api_url = runtime_for_chat.api_url.clone();
    let assistant_api_key = runtime_for_chat
        .api_key
        .clone()
        .or_else(|| state.assistant_api_key.clone());
    let assistant_state = state.assistant.clone();
    let host_base_dir = state.host.base_dir().to_path_buf();
    let base_security = state.base_security_config.clone();
    let messages_for_thread = llm_messages;

    let chat_result: ChatRunResult = tokio::task::spawn_blocking(move || -> Result<ChatRunResult> {
        let _cwd_guard = enter_workspace_dir_from(&host_base_dir, &runtime_for_chat.workspace_root)?;
        let mut assistant_slot = lock_assistant_mutex(&assistant_state)?;

        if assistant_slot.is_none() {
            let assistant_config = AssistantConfig::new_with_runtime(
                assistant_api_url,
                assistant_api_key,
                runtime_for_chat.model.clone(),
                effort_temperature(&runtime_for_chat),
                effort_max_tokens(&runtime_for_chat),
            );
            let security_config = runtime_security_config(&base_security, &runtime_for_chat);
            let assistant = CliAssistant::new(assistant_config, security_config)?;
            *assistant_slot = Some(assistant);
        }

        let assistant = assistant_slot
            .take()
            .ok_or_else(|| anyhow!("assistant initialization failed"))?;
        drop(assistant_slot);

        let mut messages = messages_for_thread;
        let chat_attempt = catch_unwind(AssertUnwindSafe(|| assistant.chat_with_trace(&mut messages)));
        match chat_attempt {
            Ok(result) => {
                let mut assistant_slot = lock_assistant_mutex(&assistant_state)?;
                *assistant_slot = Some(assistant);
                result
            }
            Err(payload) => {
                let message = panic_payload_to_string(payload);
                let mut assistant_slot = lock_assistant_mutex(&assistant_state)?;
                *assistant_slot = None;
                Err(anyhow!("assistant panicked during chat: {}", message))
            }
        }
    })
    .await
    .map_err(|err| anyhow!("assistant task join error: {}", err))??;

    let mut next_blocks = existing_blocks;
    next_blocks.push(MessageBlock::User {
        content,
        branch_id: "main".to_string(),
    });
    next_blocks.extend(chat_result.trace);
    next_blocks.push(MessageBlock::Assistant {
        content: strip_emoji(&chat_result.content),
    });

    {
        let mut session_manager = lock_session_manager(state)?;
        session_manager.save_messages(&next_blocks)?;
    }

    Ok((current_id, next_blocks))
}

async fn run_chat_request_stream(
    state: WebAppState,
    session_id: String,
    content: String,
    mode: Option<String>,
    language: Option<String>,
    tx: tokio::sync::mpsc::UnboundedSender<StreamEnvelope>,
) -> Result<()> {
    let abort_after_first_workspace_edit = content.contains("[[TEST_ABORT_AFTER_FIRST_EDIT]]");
    let content = content.replace("[[TEST_ABORT_AFTER_FIRST_EDIT]]", "");
    let existing_blocks = {
        let session_manager = lock_session_manager(&state)?;
        session_manager.load_messages(&session_id)?
    };

    if let Ok(mut runtime) = state.stream_runtime.lock() {
        if let Some(session) = runtime.get_mut(&session_id) {
            session.pending_approvals.clear();
        }
    }
    let runtime = {
        let runtime = lock_runtime_settings(&state)?;
        runtime.clone()
    };

    let provider = build_streaming_provider(&state, &runtime)?;
    let tool_definitions = assistant_tool_definitions(&state, &runtime).await?;
    let user_content = content.clone();
    let turn_language = turn_language_from_option(language.as_deref());

    let mut persisted_blocks = existing_blocks.clone();
    persisted_blocks.push(MessageBlock::User {
        content,
        branch_id: "main".to_string(),
    });
    sync_stream_runtime_messages(&state, &session_id, &persisted_blocks);

    let visible_history = messages_to_web(&persisted_blocks);
    let mut visible_assistant = String::new();
    let stream_mode = mode.as_deref();
    let base_system_prompt = format!(
        "{}\n\nLanguage policy:\n- Respond in {} for this turn.\n- Keep planner, verifier, repair, tool-use summaries, and final summaries in {} as well.",
        system_prompt_for_mode(stream_mode),
        turn_language_name(turn_language),
        turn_language_name(turn_language),
    );
    let mut dynamic_system_prompt = base_system_prompt.clone();
    let max_repair_attempts = 3usize;
    let mut repair_attempts = 0usize;
    let mut pseudo_tool_repair_attempted = false;

        emit_activity(
            &tx,
            &state,
            &session_id,
            &runtime,
            &persisted_blocks,
            stream_mode,
            workflow_activity_event(
                "starting",
                Some(localized_text(
                    turn_language,
                    "正在准备本轮 Agent 执行",
                    "Preparing agent turn",
                )),
                Some("initialize".to_string()),
                Some("running".to_string()),
                None,
                Some("main".to_string()),
            ),
    );

    let structured_workflow = should_run_structured_workflow(stream_mode, &user_content);
    let plan = if structured_workflow {
        emit_activity(
            &tx,
            &state,
            &session_id,
            &runtime,
            &persisted_blocks,
            stream_mode,
            workflow_activity_event(
                "planning",
                Some(localized_text(
                    turn_language,
                    "Planning execution",
                    "Planner subagent is building an execution plan",
                )),
                Some("plan".to_string()),
                Some("running".to_string()),
                None,
                Some("planner".to_string()),
            ),
        );
        match generate_agent_plan(
            provider.clone(),
            &runtime,
            &persisted_blocks,
            &user_content,
            stream_mode,
            turn_language,
        )
        .await
        {
            Ok(plan) => {
                let delegate_meta = if plan.delegates.is_empty() {
                    None
                } else {
                    Some(
                        plan.delegates
                            .iter()
                            .map(|delegate| format!("{}: {}", delegate.name, delegate.purpose))
                            .collect::<Vec<_>>()
                            .join(" | "),
                    )
                };
                let summary_detail = if plan.steps.is_empty() {
                    plan.summary.clone()
                } else {
                    plan.steps
                        .iter()
                        .map(|step| step.title.clone())
                        .collect::<Vec<_>>()
                        .join(" -> ")
                };
                emit_activity(
                    &tx,
                    &state,
                    &session_id,
                    &runtime,
                    &persisted_blocks,
                    stream_mode,
                    workflow_activity_event(
                        "planning",
                        Some(summary_detail.clone()),
                        Some("plan".to_string()),
                        Some("complete".to_string()),
                        delegate_meta,
                        Some("planner".to_string()),
                    )
                    .with_delegates(build_delegate_statuses(
                        &plan.delegates,
                        "planner",
                        "complete",
                        user_content.as_str(),
                        summary_detail.as_str(),
                    )),
                );
                Some(plan)
            }
            Err(err) => {
                emit_activity(
                    &tx,
                    &state,
                    &session_id,
                    &runtime,
                    &persisted_blocks,
                    stream_mode,
                    workflow_activity_event(
                        "planning",
                        Some(localized_string(
                            turn_language,
                            format!("Planner fallback: {}", err),
                            format!("Planner fallback: {}", err),
                        )),
                        Some("plan".to_string()),
                        Some("failed".to_string()),
                        None,
                        Some("planner".to_string()),
                    ),
                );
                None
            }
        }
    } else {
        None
    };

    let required_path_snapshots = plan
        .as_ref()
        .map(|plan| capture_required_path_snapshots(state.host.base_dir(), &runtime, &plan.required_paths))
        .unwrap_or_default();
    if let Ok(mut sessions) = lock_stream_runtime(&state) {
        if let Some(session) = sessions.get_mut(&session_id) {
            session.required_path_snapshots = required_path_snapshots.clone();
        }
    }

    if let Some(plan) = &plan {
        let verifier_meta = if plan.verification.is_empty() {
            None
        } else {
            Some(plan.verification.join(" | "))
        };
        emit_activity(
            &tx,
            &state,
            &session_id,
            &runtime,
            &persisted_blocks,
            stream_mode,
            workflow_activity_event(
                "delegation",
                Some(plan.summary.clone()),
                Some("delegate".to_string()),
                Some("ready".to_string()),
                verifier_meta,
                Some("main".to_string()),
            )
            .with_delegates(plan.delegates.clone()),
        );

        dynamic_system_prompt = match turn_language {
            TurnLanguage::Zh => format!(
                "{base}\n\nPlanner execution plan:\n- Goal: {goal}\n- Workflow kind: {kind}\n- Summary: {summary}\n- Steps:\n{steps}\n- Verification targets:\n{verification}\n- Repair strategy: {repair}\n\nExecution policy:\n- Follow this plan unless tool evidence requires adaptation.\n- If you adapt, preserve the intent and continue.\n- Prefer real file edits and real verification over speculative answers.\n- Do not dump large source code into chat after writing files.\n- Keep dependency checks compact: prefer short import/probe commands such as `python -c` checks over `pip show` or long package metadata dumps.\n- Avoid commands that print license text, full environment inventories, or other bulky output unless they are strictly required.",
                base = base_system_prompt,
                goal = plan.goal,
                kind = plan.workflow_kind,
                summary = plan.summary,
                steps = plan.steps.iter().enumerate().map(|(index, step)| {
                    format!("  {}. {} [{} / {}] - {}", index + 1, step.title, step.owner, step.kind, step.purpose)
                }).collect::<Vec<_>>().join("\n"),
                verification = if plan.verification.is_empty() {
                    "  - none".to_string()
                } else {
                    plan.verification.iter().map(|item| format!("  - {}", item)).collect::<Vec<_>>().join("\n")
                },
                repair = if plan.repair_strategy.trim().is_empty() {
                    "repair the most important blocker, then re-verify".to_string()
                } else {
                    plan.repair_strategy.clone()
                },
            ),
            TurnLanguage::En => format!(
                "{base}\n\nExecution plan from planner subagent:\n- Goal: {goal}\n- Workflow kind: {kind}\n- Summary: {summary}\n- Steps:\n{steps}\n- Verification targets:\n{verification}\n- Repair strategy: {repair}\n\nExecution policy:\n- Follow this plan unless tool evidence requires adaptation.\n- If you adapt, preserve the intent and continue.\n- Prefer real file edits and real verification over speculative answers.\n- Do not dump large source code into chat after writing files.\n- Keep dependency checks compact: prefer short import/probe commands such as `python -c` checks over `pip show` or long package metadata dumps.\n- Avoid commands that print license text, full environment inventories, or other bulky output unless they are strictly required.",
                base = base_system_prompt,
                goal = plan.goal,
                kind = plan.workflow_kind,
                summary = plan.summary,
                steps = plan.steps.iter().enumerate().map(|(index, step)| {
                    format!("  {}. {} [{} / {}] - {}", index + 1, step.title, step.owner, step.kind, step.purpose)
                }).collect::<Vec<_>>().join("\n"),
                verification = if plan.verification.is_empty() {
                    "none".to_string()
                } else {
                    plan.verification.iter().map(|item| format!("  - {}", item)).collect::<Vec<_>>().join("\n")
                },
                repair = if plan.repair_strategy.trim().is_empty() {
                    "repair the most important blocker, then re-verify".to_string()
                } else {
                    plan.repair_strategy.clone()
                },
            ),
        };
        if !plan.required_paths.is_empty() {
            dynamic_system_prompt.push_str(&localized_text(
                turn_language,
                "\n- Required workspace paths that must be created or updated exactly:\n",
                "\n- Required workspace paths that must be created/updated exactly:\n",
            ));
            dynamic_system_prompt.push_str(
                &plan
                    .required_paths
                    .iter()
                    .map(|path| format!("  - {}", path))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
            dynamic_system_prompt.push_str(&localized_text(
                turn_language,
                "\n- These exact paths are mandatory. Do not substitute older or neighboring files.\n- The turn is incomplete until these exact paths are verified by tool evidence or filesystem evidence.",
                "\n- These exact paths are mandatory. Do not substitute older or neighboring files.\n- The turn is incomplete until these exact paths are verified by tool evidence or filesystem evidence.",
            ));
        }
    }

    let max_turn_rounds = dynamic_turn_round_limit(plan.as_ref(), structured_workflow);
    let mut stagnant_rounds = 0usize;
    for _ in 0..max_turn_rounds {
        let turn_start = persisted_blocks.len();
        let request = build_stream_chat_request_with_prompt(
            &persisted_blocks,
            &runtime,
            &dynamic_system_prompt,
            turn_language,
            &tool_definitions,
            state.host.base_dir(),
        )?;
        emit_activity(
            &tx,
            &state,
            &session_id,
            &runtime,
            &persisted_blocks,
            stream_mode,
            workflow_activity_event(
                "execution",
                Some(localized_text(
                    turn_language,
                    "Main agent is executing the current step",
                    "Main agent is executing the current step",
                )),
                Some("execute".to_string()),
                Some("running".to_string()),
                None,
                Some("main".to_string()),
            ),
        );
        let has_workspace_edits = persisted_blocks
            .iter()
            .rev()
            .take(24)
            .any(|block| matches!(block, MessageBlock::Diff { .. }));
        let turn = stream_provider_turn(
            provider.clone(),
            request,
            &state,
            &session_id,
            &visible_history,
            &visible_assistant,
            has_workspace_edits,
            stream_mode,
            turn_language,
            &tx,
        )
        .await?;
        let raw_assistant_text = assistant_text_for_workspace_control_channel(
            &turn.text,
            has_workspace_edits,
            stream_mode,
            turn_language,
        );
        let assistant_text = summarize_workspace_turn_for_chat(
            &raw_assistant_text,
            &persisted_blocks[turn_start..],
            stream_mode,
            turn_language,
        )
        .unwrap_or(raw_assistant_text);
        if !assistant_text.is_empty() {
            visible_assistant = combine_assistant_segments(&visible_assistant, &assistant_text);
            persisted_blocks.push(MessageBlock::Assistant {
                content: assistant_text,
            });
            sync_stream_runtime_messages(&state, &session_id, &persisted_blocks);
            if let Ok(mut sessions) = lock_stream_runtime(&state) {
                if let Some(session) = sessions.get_mut(&session_id) {
                    session.partial_text.clear();
                    session.progress_updates.clear();
                    session.recent_progress_keys.clear();
                    session.recent_progress_emitted_at.clear();
                }
            }
        }
        let made_progress_this_round = turn_made_real_progress(
            &persisted_blocks[turn_start..],
            state.host.base_dir(),
            &runtime.workspace_root,
        );
        if made_progress_this_round {
            stagnant_rounds = 0;
        } else {
            stagnant_rounds = stagnant_rounds.saturating_add(1);
        }

        let needs_tools = is_tool_call_finish(&turn.finish_reason)
            || (turn.finish_reason.is_some() && turn.tool_calls.is_some());

        if needs_tools {
            let tool_calls = turn.tool_calls.unwrap_or_default();
            if tool_calls.is_empty() {
                emit_activity(
                    &tx,
                    &state,
                    &session_id,
                    &runtime,
                    &persisted_blocks,
                    stream_mode,
                    workflow_activity_event(
                        "execution",
                        Some(localized_text(
                            turn_language,
                            "Model signaled tool usage without concrete tool calls; continuing review.",
                            "Model signaled tool usage without concrete tool calls; continuing review",
                        )),
                        Some("execute".to_string()),
                        Some("running".to_string()),
                        None,
                        Some("main".to_string()),
                    ),
                );
            } else {
                if structured_workflow {
                    let tool_names = tool_calls
                        .iter()
                        .filter_map(|call| {
                            call.get("function")
                                .and_then(|function| function.get("name"))
                                .and_then(|value| value.as_str())
                                .map(str::to_string)
                        })
                        .collect::<Vec<_>>();
                    let delegate_snapshot = plan
                        .as_ref()
                        .map(|value| {
                            build_delegate_statuses(
                                &value.delegates,
                                "executor",
                                "running",
                                user_content.as_str(),
                                &localized_text(turn_language, "Dispatching tool work", "Dispatching tool work"),
                            )
                        })
                        .unwrap_or_default();
                    emit_activity(
                        &tx,
                        &state,
                        &session_id,
                        &runtime,
                        &persisted_blocks,
                        stream_mode,
                        workflow_activity_event(
                            "delegation",
                            Some(localized_text(
                                turn_language,
                                "Dispatching tool work",
                                "Dispatching tool work",
                            )),
                            Some("delegate".to_string()),
                            Some("running".to_string()),
                            Some(tool_names.join(", ")),
                            Some("executor".to_string()),
                        )
                        .with_delegates(delegate_snapshot),
                    );
                }
                execute_tool_calls(
                    &state,
                    &runtime,
                    &session_id,
                    &tool_calls,
                    &mut persisted_blocks,
                    &tx,
                    stream_mode,
                    turn_language,
                    abort_after_first_workspace_edit,
                )
                .await?;
                sync_stream_runtime_messages(&state, &session_id, &persisted_blocks);
                if try_finalize_current_turn_with_hard_verifier(
                    &tx,
                    &state,
                    &session_id,
                    &runtime,
                    &mut persisted_blocks,
                    stream_mode,
                    plan.as_ref(),
                    &required_path_snapshots,
                    turn_language,
                    &localized_text(
                        turn_language,
                        "Agent turn finished after tool execution and hard verification",
                        "Agent turn finished after tool execution and hard verification",
                    ),
                    true,
                )? {
                    return Ok(());
                }
                continue;
            }
        }

        if !turn.pseudo_tool_names.is_empty() {
            let pseudo_tools = turn.pseudo_tool_names.join(", ");
            if !pseudo_tool_repair_attempted {
                pseudo_tool_repair_attempted = true;
                dynamic_system_prompt = localized_string(
                    turn_language,
                    format!(
                        "{existing}\n\nNative tool-call retry directive:\n- Your previous response started narrating tool usage as plain text ({pseudo_tools}).\n- Do not print Tool, Arguments, DSML, XML, or shell wrapper text in assistant output.\n- Resume from the exact pending step and use only native tool/function calls for every required action.\n- If prior file edits already succeeded, continue from the current workspace state instead of restarting from scratch.",
                        existing = dynamic_system_prompt,
                        pseudo_tools = pseudo_tools,
                    ),
                    format!(
                        "{existing}\n\nNative tool-call retry directive:\n- Your previous response started narrating tool usage as plain text ({pseudo_tools}).\n- Do not print lines such as Tool, Arguments, DSML, XML, or shell wrapper text in assistant output.\n- Resume from the exact pending step and use only native tool/function calls for every required action.\n- If prior file edits already succeeded, continue from the current workspace state instead of restarting from scratch.",
                        existing = dynamic_system_prompt,
                        pseudo_tools = pseudo_tools,
                    ),
                );
                emit_activity(
                    &tx,
                    &state,
                    &session_id,
                    &runtime,
                    &persisted_blocks,
                    stream_mode,
                    workflow_activity_event(
                        "execution",
                        Some(localized_string(
                            turn_language,
                            format!(
                                "Model narrated tool usage as plain text ({}); requesting a native tool-call retry.",
                                pseudo_tools
                            ),
                            format!(
                                "Model narrated tool usage as plain text ({}); requesting a native tool-call retry",
                                pseudo_tools
                            ),
                        )),
                        Some("execute".to_string()),
                        Some("repair".to_string()),
                        None,
                        Some("main".to_string()),
                    ),
                );
                continue;
            }

            return Err(anyhow!(
                "model emitted pseudo tool narration instead of native tool calls: {}",
                pseudo_tools
            ));
        }

        if let Some(plan) = &plan {
            if !required_paths_ready_for_review(state.host.base_dir(), &runtime, plan)
                && !required_paths_likely_satisfied_by_evidence(
                    plan,
                    &persisted_blocks,
                    state.host.base_dir(),
                    &runtime.workspace_root,
                    &required_path_snapshots,
                )
            {
                emit_activity(
                    &tx,
                    &state,
                    &session_id,
                    &runtime,
                    &persisted_blocks,
                    stream_mode,
                    workflow_activity_event(
                        "execution",
                        Some(localized_text(
                            turn_language,
                            "Required workspace targets are still incomplete; continuing execution.",
                            "Required workspace targets are still incomplete; continuing execution",
                        )),
                        Some("execute".to_string()),
                        Some("running".to_string()),
                        Some(plan.required_paths.join(" | ")),
                        Some("main".to_string()),
                    ),
                );
                continue;
            }
            if try_finalize_current_turn_with_hard_verifier(
                &tx,
                &state,
                &session_id,
                &runtime,
                &mut persisted_blocks,
                stream_mode,
                Some(plan),
                &required_path_snapshots,
                turn_language,
                &localized_text(
                    turn_language,
                    "Agent turn finished after hard verification",
                    "Agent turn finished after hard verification",
                ),
                true,
            )? {
                return Ok(());
            }
        }

        if let Some(plan) = &plan {
            emit_activity(
                &tx,
                &state,
                &session_id,
                &runtime,
                &persisted_blocks,
                stream_mode,
                workflow_activity_event(
                    "review",
                    Some(localized_text(
                        turn_language,
                        "Reviewer subagent is checking the turn",
                        "Reviewer subagent is checking the turn",
                    )),
                    Some("review".to_string()),
                    Some("running".to_string()),
                    None,
                    Some("reviewer".to_string()),
                ),
            );
            let running_subagents = vec![
                AgentSubagentRecord {
                    id: "reviewer".to_string(),
                    name: "reviewer".to_string(),
                    purpose: localized_text(
                        turn_language,
                        "Review the turn for completeness and whether the result satisfies the request.",
                        "Review the turn for completeness and whether the result satisfies the request.",
                    ),
                    input: tail_string(user_content.trim(), 280),
                    output: String::new(),
                    status: "running".to_string(),
                    kind: "review".to_string(),
                    started_at: Some(web_now_iso()),
                    completed_at: None,
                    evidence: Vec::new(),
                },
                AgentSubagentRecord {
                    id: "critic".to_string(),
                    name: "critic".to_string(),
                    purpose: localized_text(
                        turn_language,
                        "Search for hidden gaps, weak assumptions, and places where the result could still be wrong.",
                        "Search for hidden gaps, weak assumptions, and places where the result could still be wrong.",
                    ),
                    input: tail_string(user_content.trim(), 280),
                    output: String::new(),
                    status: "running".to_string(),
                    kind: "critique".to_string(),
                    started_at: Some(web_now_iso()),
                    completed_at: None,
                    evidence: Vec::new(),
                },
                AgentSubagentRecord {
                    id: "researcher".to_string(),
                    name: "researcher".to_string(),
                    purpose: localized_text(
                        turn_language,
                        "Assess whether the execution produced enough evidence, artifacts, and follow-through for the current workflow.",
                        "Assess whether the execution produced enough evidence, artifacts, and follow-through for the current workflow.",
                    ),
                    input: tail_string(user_content.trim(), 280),
                    output: String::new(),
                    status: "running".to_string(),
                    kind: "research".to_string(),
                    started_at: Some(web_now_iso()),
                    completed_at: None,
                    evidence: Vec::new(),
                },
                AgentSubagentRecord {
                    id: "verifier".to_string(),
                    name: "verifier".to_string(),
                    purpose: localized_text(
                        turn_language,
                        "Verify the turn using deterministic tool, runtime, diff, and execution evidence.",
                        "Verify the turn using deterministic tool, runtime, diff, and execution evidence.",
                    ),
                    input: tail_string(user_content.trim(), 280),
                    output: String::new(),
                    status: "running".to_string(),
                    kind: "verify".to_string(),
                    started_at: Some(web_now_iso()),
                    completed_at: None,
                    evidence: Vec::new(),
                },
            ];
            for record in &running_subagents {
                emit_subagent_update(
                    &tx,
                    &state,
                    &session_id,
                    &runtime,
                    &persisted_blocks,
                    stream_mode,
                    record,
                );
            }
            match run_parallel_analysis_subagents(
                provider.clone(),
                &runtime,
                plan,
                &persisted_blocks,
                &user_content,
                state.host.base_dir(),
                &required_path_snapshots,
                turn_language,
                |progress: ParallelAnalysisProgress| {
                    if let Some(record) = progress.subagent_record.as_ref() {
                        emit_subagent_update(
                            &tx,
                            &state,
                            &session_id,
                            &runtime,
                            &persisted_blocks,
                            stream_mode,
                            record,
                        );
                    }
                    if let Some(report) = progress.verifier_report.as_ref() {
                        emit_verifier_update(
                            &tx,
                            &state,
                            &session_id,
                            &runtime,
                            &persisted_blocks,
                            stream_mode,
                            report,
                            &progress.checkpoints,
                            &progress.branch_notes,
                            turn_language,
                        );
                    }
                },
            )
            .await
            {
                Ok(result) => {
                    for record in &result.subagent_records {
                        persisted_blocks.push(MessageBlock::Subagent {
                            record: record.clone(),
                        });
                    }
                    persisted_blocks.push(MessageBlock::Verification {
                        report: result.verifier_report.clone(),
                    });
                    sync_stream_runtime_messages(&state, &session_id, &persisted_blocks);
                    for checkpoint in &result.checkpoints {
                        push_runtime_checkpoint(&state, &session_id, checkpoint.clone(), turn_language);
                    }
                    for note in &result.branch_notes {
                        push_runtime_branch_note(&state, &session_id, note.clone(), turn_language);
                    }

                    let near_completion = (required_paths_likely_satisfied_by_evidence(
                        plan,
                        &persisted_blocks,
                        state.host.base_dir(),
                        &runtime.workspace_root,
                        &required_path_snapshots,
                    ) || required_paths_ready_for_review(state.host.base_dir(), &runtime, plan))
                        && (result.verifier_report.issues.len() <= 1
                            || verifier_report_has_only_soft_evidence_gaps(&result.verifier_report));
                    let needs_repair = result.needs_repair && !near_completion;
                    let summary = result.summary.clone();
                    let meta = if !result.issues.is_empty() {
                        Some(result.issues.join(" | "))
                    } else {
                        result.evidence.first().cloned()
                    };
                    emit_activity(
                        &tx,
                        &state,
                        &session_id,
                        &runtime,
                        &persisted_blocks,
                        stream_mode,
                        workflow_activity_event(
                            "review",
                            Some(summary.clone()),
                            Some("review".to_string()),
                            Some(if needs_repair { "repair".to_string() } else { "pass".to_string() }),
                            meta,
                            Some("reviewer".to_string()),
                        )
                        .with_delegates(build_delegate_statuses(
                            &plan.delegates,
                            "reviewer",
                            if needs_repair { "repair" } else { "pass" },
                            user_content.as_str(),
                            summary.as_str(),
                        )),
                    );

                    if needs_repair && repair_attempts < max_repair_attempts {
                        repair_attempts = repair_attempts.saturating_add(1);
                        let repair_actions = if !result.next_actions.is_empty() {
                            result.next_actions.clone()
                        } else if !plan.repair_strategy.trim().is_empty() {
                            vec![plan.repair_strategy.clone()]
                        } else {
                            vec![localized_text(
                                turn_language,
                                "repair the main blocker and re-run verification",
                                "repair the main blocker and re-run verification",
                            )]
                        };
                        let repair_hint = repair_actions.join(" | ");
                        dynamic_system_prompt = match turn_language {
                            TurnLanguage::Zh => format!(
                                "{base}\n\nRepair directive from reviewer and verifier:\n- Reviewer summary: {review_summary}\n- Hard verifier summary: {verifier_summary}\n- Issues:\n{issues}\n- Required next actions:\n{actions}\n\nBefore finalizing, fix the issues above and verify again.",
                                base = base_system_prompt,
                                review_summary = result.review_report.summary,
                                verifier_summary = result.verifier_report.summary,
                                issues = {
                                    let combined = result
                                        .issues
                                        .iter()
                                        .map(|item| format!("  - {}", item))
                                        .collect::<Vec<_>>();
                                    if combined.is_empty() {
                                        "  - Missing concrete verification evidence.".to_string()
                                    } else {
                                        combined.join("\n")
                                    }
                                },
                                actions = repair_actions
                                    .iter()
                                    .map(|item| format!("  - {}", item))
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            ),
                            TurnLanguage::En => format!(
                                "{base}\n\nRepair directive from reviewer and verifier:\n- Reviewer summary: {review_summary}\n- Hard verifier summary: {verifier_summary}\n- Issues:\n{issues}\n- Required next actions:\n{actions}\n\nBefore finalizing, fix the issues above and verify again.",
                                base = base_system_prompt,
                                review_summary = result.review_report.summary,
                                verifier_summary = result.verifier_report.summary,
                                issues = {
                                    let combined = result
                                        .issues
                                        .iter()
                                        .map(|item| format!("  - {}", item))
                                        .collect::<Vec<_>>();
                                    if combined.is_empty() {
                                        "  - Missing concrete verification evidence.".to_string()
                                    } else {
                                        combined.join("\n")
                                    }
                                },
                                actions = repair_actions
                                    .iter()
                                    .map(|item| format!("  - {}", item))
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            ),
                        };
                        emit_activity(
                            &tx,
                            &state,
                            &session_id,
                            &runtime,
                            &persisted_blocks,
                            stream_mode,
                            workflow_activity_event(
                                "repair",
                                Some(localized_text(
                                    turn_language,
                                    "Repairer subagent requested another execution pass",
                                    "Repairer subagent requested another execution pass",
                                )),
                                Some("repair".to_string()),
                                Some("running".to_string()),
                                Some(format!(
                                    "{} | repair pass {}/{}",
                                    repair_hint,
                                    repair_attempts,
                                    max_repair_attempts
                                )),
                                Some("repairer".to_string()),
                            )
                            .with_delegates(build_delegate_statuses(
                                &plan.delegates,
                                "repairer",
                                "running",
                                user_content.as_str(),
                                &localized_text(turn_language, "Requested another execution pass", "Requested another execution pass"),
                            )),
                        );
                        continue;
                    }
                    if needs_repair {
                        let failure_summary = if result.summary.trim().is_empty() {
                            localized_text(
                                turn_language,
                                "Research turn stopped because verification did not pass after the allowed repair attempts.",
                                "Research turn stopped because verification did not pass after the allowed repair attempts.",
                            )
                        } else {
                            localized_string(
                                turn_language,
                                format!(
                                    "Research turn stopped because verification did not pass after {} repair attempt(s): {}",
                                    repair_attempts,
                                    result.summary
                                ),
                                format!(
                                    "Research turn stopped because verification did not pass after {} repair attempt(s): {}",
                                    repair_attempts,
                                    result.summary
                                ),
                            )
                        };
                        persisted_blocks.push(MessageBlock::Assistant {
                            content: failure_summary,
                        });
                        sync_stream_runtime_messages(&state, &session_id, &persisted_blocks);
                    }
                }
                Err(err) => {
                    emit_activity(
                        &tx,
                        &state,
                        &session_id,
                        &runtime,
                        &persisted_blocks,
                        stream_mode,
                        workflow_activity_event(
                            "review",
                            Some(localized_string(
                                turn_language,
                                format!("Reviewer fallback: {}", err),
                                format!("Reviewer fallback: {}", err),
                            )),
                            Some("review".to_string()),
                            Some("failed".to_string()),
                            None,
                            Some("reviewer".to_string()),
                        ),
                    );
                }
            }
        }

        finalize_stream_success(
            &tx,
            &state,
            &session_id,
            &runtime,
            &persisted_blocks,
            stream_mode,
            turn_language,
            &localized_text(turn_language, "Agent turn finished", "Agent turn finished"),
        )?;
        return Ok(());
    }

    if let Some(plan) = &plan {
        if try_finalize_current_turn_with_hard_verifier(
            &tx,
            &state,
            &session_id,
            &runtime,
            &mut persisted_blocks,
            stream_mode,
            Some(plan),
            &required_path_snapshots,
            turn_language,
            &localized_text(
                turn_language,
                "Agent turn finished after hard verification at round limit",
                "Agent turn finished after hard verification at round limit",
            ),
            false,
        )? {
            return Ok(());
        }
    }

    let incomplete_required_paths = plan
        .as_ref()
        .map(|plan| {
            !required_paths_ready_for_review(state.host.base_dir(), &runtime, plan)
                && !required_paths_likely_satisfied_by_evidence(
                    plan,
                    &persisted_blocks,
                    state.host.base_dir(),
                    &runtime.workspace_root,
                    &required_path_snapshots,
                )
        })
        .unwrap_or(false);
    if incomplete_required_paths && stagnant_rounds <= 2 {
        emit_activity(
            &tx,
            &state,
            &session_id,
            &runtime,
            &persisted_blocks,
            stream_mode,
            workflow_activity_event(
                "execution",
                Some(localized_text(
                    turn_language,
                    "This turn is still making concrete progress, so continuing with the remaining writes and verification.",
                    "This turn is still making concrete progress, so continuing with the remaining writes and verification.",
                )),
                Some("execute".to_string()),
                Some("running".to_string()),
                None,
                Some("main".to_string()),
            ),
        );
        let extra_rounds = if structured_workflow { 48usize } else { 24usize };
        for _ in 0..extra_rounds {
            let turn_start = persisted_blocks.len();
            let request = build_stream_chat_request_with_prompt(
                &persisted_blocks,
                &runtime,
                &dynamic_system_prompt,
                turn_language,
                &tool_definitions,
                state.host.base_dir(),
            )?;
            emit_activity(
                &tx,
                &state,
                &session_id,
                &runtime,
                &persisted_blocks,
                stream_mode,
                workflow_activity_event(
                    "execution",
                    Some(localized_text(
                        turn_language,
                        "Main agent is continuing the current step",
                        "Main agent is continuing the current step",
                    )),
                    Some("execute".to_string()),
                    Some("running".to_string()),
                    None,
                    Some("main".to_string()),
                ),
            );
            let has_workspace_edits = persisted_blocks
                .iter()
                .rev()
                .take(24)
                .any(|block| matches!(block, MessageBlock::Diff { .. }));
            let turn = stream_provider_turn(
                provider.clone(),
                request,
                &state,
                &session_id,
                &visible_history,
                &visible_assistant,
                has_workspace_edits,
                stream_mode,
                turn_language,
                &tx,
            )
            .await?;
            let raw_assistant_text = assistant_text_for_workspace_control_channel(
                &turn.text,
                has_workspace_edits,
                stream_mode,
                turn_language,
            );
            let assistant_text = summarize_workspace_turn_for_chat(
                &raw_assistant_text,
                &persisted_blocks[turn_start..],
                stream_mode,
                turn_language,
            )
            .unwrap_or(raw_assistant_text);
            if !assistant_text.is_empty() {
                visible_assistant = combine_assistant_segments(&visible_assistant, &assistant_text);
                persisted_blocks.push(MessageBlock::Assistant {
                    content: assistant_text,
                });
                sync_stream_runtime_messages(&state, &session_id, &persisted_blocks);
                if let Ok(mut sessions) = lock_stream_runtime(&state) {
                    if let Some(session) = sessions.get_mut(&session_id) {
                        session.partial_text.clear();
                        session.progress_updates.clear();
                        session.recent_progress_keys.clear();
                        session.recent_progress_emitted_at.clear();
                    }
                }
            }
            if is_tool_call_finish(&turn.finish_reason)
                || (turn.finish_reason.is_some() && turn.tool_calls.is_some())
            {
                let tool_calls = turn.tool_calls.unwrap_or_default();
                if !tool_calls.is_empty() {
                    execute_tool_calls(
                        &state,
                        &runtime,
                        &session_id,
                        &tool_calls,
                        &mut persisted_blocks,
                        &tx,
                        stream_mode,
                        turn_language,
                        abort_after_first_workspace_edit,
                    )
                    .await?;
                    sync_stream_runtime_messages(&state, &session_id, &persisted_blocks);
                    if try_finalize_current_turn_with_hard_verifier(
                        &tx,
                        &state,
                        &session_id,
                        &runtime,
                        &mut persisted_blocks,
                        stream_mode,
                        plan.as_ref(),
                        &required_path_snapshots,
                        turn_language,
                        &localized_text(
                            turn_language,
                            "Agent turn finished after extended tool execution and hard verification",
                            "Agent turn finished after extended tool execution and hard verification",
                        ),
                        false,
                    )? {
                        return Ok(());
                    }
                }
            }
            if let Some(plan) = &plan {
                if try_finalize_current_turn_with_hard_verifier(
                    &tx,
                    &state,
                    &session_id,
                    &runtime,
                    &mut persisted_blocks,
                    stream_mode,
                    Some(plan),
                    &required_path_snapshots,
                    turn_language,
                    &localized_text(
                        turn_language,
                        "Agent turn finished after extended execution and verification",
                        "Agent turn finished after extended execution and verification",
                    ),
                    false,
                )? {
                    return Ok(());
                }
            }
            let still_progressing = turn_made_real_progress(
                &persisted_blocks[turn_start..],
                state.host.base_dir(),
                &runtime.workspace_root,
            );
            if !still_progressing {
                stagnant_rounds = stagnant_rounds.saturating_add(1);
                if stagnant_rounds > 2 {
                    break;
                }
            } else {
                stagnant_rounds = 0;
            }
        }
    }
    let round_limit_message = if incomplete_required_paths {
        localized_text(
            turn_language,
            "本轮 Agent 已运行较长时间，当前先停在一个安全检查点。仍有少量必需工作区产物等待验证；继续本轮后，Agent 会从当前工作区状态恢复，完成剩余写入与验证。",
            "This agent turn has been running for a while and is pausing at a safe checkpoint. A few required workspace artifacts still need verification; continue the turn and the agent will resume from the current workspace state to finish the remaining writes and validation.",
        )
    } else {
        localized_text(
            turn_language,
            "本轮 Agent 已运行较长时间，当前先停在一个安全检查点。主要工作已经保留在当前工作区；继续本轮即可完成剩余验证与收尾。",
            "This agent turn has been running for a while and is pausing at a safe checkpoint. The main work is already preserved in the current workspace; continue the turn to finish the remaining verification and wrap-up.",
        )
    };
    persisted_blocks.push(MessageBlock::Assistant {
        content: round_limit_message,
    });
    sync_stream_runtime_messages(&state, &session_id, &persisted_blocks);

    finalize_stream_success(
        &tx,
        &state,
        &session_id,
        &runtime,
        &persisted_blocks,
        stream_mode,
        turn_language,
        &localized_text(
            turn_language,
            "本轮 Agent 已暂停在可恢复的检查点",
            "Agent turn paused at a resumable checkpoint",
        ),
    )?;
    Ok(())
}

fn finalize_stream_success(
    tx: &tokio::sync::mpsc::UnboundedSender<StreamEnvelope>,
    state: &WebAppState,
    session_id: &str,
    runtime: &RuntimeSettings,
    persisted_blocks: &[MessageBlock],
    stream_mode: Option<&str>,
    language: TurnLanguage,
    completion_detail: &str,
) -> Result<()> {
    let runtime_files = lock_stream_runtime(state)
        .ok()
        .and_then(|sessions| sessions.get(session_id).map(|session| session.edited_files.clone()))
        .unwrap_or_default();
    let finalized_blocks = merge_runtime_edited_files_into_messages(
        state.host.base_dir(),
        &runtime.workspace_root,
        persisted_blocks,
        &runtime_files,
    );
    let finalized_blocks =
        ensure_final_turn_assistant_summary(&finalized_blocks, stream_mode, language);
    {
        emit_activity(
            tx,
            state,
            session_id,
            runtime,
            &finalized_blocks,
            stream_mode,
            workflow_activity_event(
                "finalize",
                Some(localized_text(
                    language,
                    "正在持久化会话消息",
                    "Persisting session messages",
                )),
                Some("finalize".to_string()),
                Some("running".to_string()),
                None,
                Some("main".to_string()),
            ),
        );
        let mut session_manager = lock_session_manager(state)?;
        session_manager.save_messages_for(session_id, &finalized_blocks)?;
    }

    emit_activity(
        tx,
        state,
        session_id,
        runtime,
        &finalized_blocks,
        stream_mode,
        workflow_activity_event(
            "finalize",
            Some(localized_text(
                language,
                "正在准备最终消息载荷",
                "Preparing final message payload",
            )),
            Some("finalize".to_string()),
            Some("running".to_string()),
            None,
            Some("main".to_string()),
        ),
    );
    let final_messages = messages_to_web(&finalized_blocks);
    emit_activity(
        tx,
        state,
        session_id,
        runtime,
        &finalized_blocks,
        stream_mode,
        workflow_activity_event(
            "finalize",
            Some(localized_text(
                language,
                "正在发送完成事件",
                "Sending complete event",
            )),
            Some("finalize".to_string()),
            Some("running".to_string()),
            None,
            Some("main".to_string()),
        ),
    );
    let _ = tx.send(StreamEnvelope {
        r#type: "complete".to_string(),
        session_id: Some(session_id.to_string()),
        messages: Some(final_messages),
        delta: None,
        error: None,
        activity: Some(workflow_activity_event(
            "complete",
            Some(completion_detail.to_string()),
            Some("finalize".to_string()),
            Some("complete".to_string()),
            None,
            Some("main".to_string()),
        )),
        tool: None,
        permission: None,
        edited_files: None,
        research: current_research_payload(
            state,
            Some(session_id),
            runtime,
            &finalized_blocks,
            stream_mode,
        ),
        subagents: None,
        verifier: None,
    });
    clear_stream_runtime_session(state, session_id);
    Ok(())
}

fn recover_stream_finalize_context(
    state: &WebAppState,
    session_id: &str,
) -> Result<(RuntimeSettings, Vec<MessageBlock>)> {
    let runtime = {
        let runtime = lock_runtime_settings(state)?;
        runtime.clone()
    };
    let messages = if let Ok(sessions) = lock_stream_runtime(state) {
        sessions
            .get(session_id)
            .map(|session| session.message_blocks.clone())
            .filter(|messages| !messages.is_empty())
    } else {
        None
    }
    .unwrap_or_else(|| {
        let session_manager = lock_session_manager(state)
            .expect("failed to lock session manager during stream recovery");
        session_manager.load_messages(session_id).unwrap_or_default()
    });
    Ok((runtime, messages))
}

fn merge_runtime_edited_files_into_messages(
    base_dir: &Path,
    workspace_root: &str,
    messages: &[MessageBlock],
    runtime_files: &[WebEditedFile],
) -> Vec<MessageBlock> {
    if runtime_files.is_empty() {
        return messages.to_vec();
    }

    let mut merged = messages.to_vec();
    let mut known_paths = collect_review_paths_for_current_turn(&merged, &[]);

    for file in runtime_files {
        if known_paths.iter().any(|existing| existing == &file.path) {
            continue;
        }

        let diff = build_workspace_artifact_diff(base_dir, workspace_root, &file.path).unwrap_or_else(|| {
            FileDiff::compute(&file.path, &file.before_content, &file.after_content)
        });
        upsert_diff_block(&mut merged, diff);
        known_paths.push(file.path.clone());
    }

    merged
}

fn finalize_stream_failure(
    tx: &tokio::sync::mpsc::UnboundedSender<StreamEnvelope>,
    state: &WebAppState,
    session_id: &str,
    language: TurnLanguage,
    err: &str,
) {
    let mut final_messages = None;
    if let Ok((runtime, persisted_blocks)) = recover_stream_finalize_context(state, session_id) {
        let runtime_files = lock_stream_runtime(state)
            .ok()
            .and_then(|sessions| sessions.get(session_id).map(|session| session.edited_files.clone()))
            .unwrap_or_default();
        let mut merged_messages = merge_runtime_edited_files_into_messages(
            state.host.base_dir(),
            &runtime.workspace_root,
            &persisted_blocks,
            &runtime_files,
        );
        append_stream_failure_message(&mut merged_messages, language, err);
        sync_stream_runtime_messages(state, session_id, &merged_messages);
        if let Ok(mut session_manager) = lock_session_manager(state) {
            let _ = session_manager.save_messages_for(session_id, &merged_messages);
        }
        final_messages = Some(messages_to_web(&merged_messages));
    }
    clear_stream_runtime_session(state, session_id);
    let _ = tx.send(StreamEnvelope {
        r#type: "error".to_string(),
        session_id: Some(session_id.to_string()),
        messages: final_messages,
        delta: None,
        error: Some(err.to_string()),
        activity: None,
        tool: None,
        permission: None,
        edited_files: None,
        research: None,
        subagents: None,
        verifier: None,
    });
}

fn merge_required_paths_into_plan(
    mut plan: AgentWorkflowPlan,
    required_paths: &[String],
    language: TurnLanguage,
) -> AgentWorkflowPlan {
    if required_paths.is_empty() {
        plan.required_paths = normalize_required_workspace_paths(plan.required_paths);
        return plan;
    }

    let mut merged_required_paths = plan.required_paths.clone();
    merged_required_paths.extend(required_paths.iter().cloned());
    let merged_required_paths = normalize_required_workspace_paths(merged_required_paths);

    let mut verification = plan
        .verification
        .into_iter()
        .filter(|item| verification_item_matches_required_paths(item, &merged_required_paths))
        .collect::<Vec<_>>();

    for path in &merged_required_paths {
        if !verification
            .iter()
            .any(|item| item.to_ascii_lowercase().contains(&path.to_ascii_lowercase()))
        {
            verification.push(localized_required_target_text(language, path));
        }
    }

    plan.required_paths = merged_required_paths;
    plan.verification = verification;
    plan
}

fn append_stream_failure_message(
    messages: &mut Vec<MessageBlock>,
    language: TurnLanguage,
    err: &str,
) {
    let trimmed = err.trim();
    if trimmed.is_empty() {
        return;
    }

    let already_recorded = messages.iter().rev().any(|block| match block {
        MessageBlock::Assistant { content } | MessageBlock::AssistantStreaming { content } => {
            content.contains(trimmed)
        }
        _ => false,
    });
    if already_recorded {
        return;
    }

    messages.push(MessageBlock::Assistant {
        content: localized_string(
            language,
            format!(
                "这轮执行被外部服务打断了：{}。我已经保留当前工作区改动和执行痕迹，继续本轮后会从这里接着完成剩余写入与验证。",
                trimmed
            ),
            format!(
                "This turn was interrupted by an external service error: {}. I preserved the current workspace changes and execution trace, and continuing the turn will resume the remaining writes and verification from here.",
                trimmed
            ),
        ),
    });
}

fn recover_plan_for_finalize(
    state: &WebAppState,
    runtime: &RuntimeSettings,
    messages: &[MessageBlock],
    user_content: &str,
    stream_mode: Option<&str>,
    language: TurnLanguage,
) -> Option<AgentWorkflowPlan> {
    if !should_run_structured_workflow(stream_mode, user_content) {
        return None;
    }
    let required_paths = collect_latest_required_workspace_paths(messages);
    let mut plan = AgentWorkflowPlan {
        workflow_kind: workflow_mode(stream_mode).to_string(),
        goal: user_content.trim().to_string(),
        summary: user_content.trim().to_string(),
        steps: vec![
            AgentWorkflowStep {
                title: localized_text(language, "执行请求", "Execute request"),
                purpose: localized_text(
                    language,
                    "完成请求要求的工作区修改与运行验证。",
                    "Complete the requested workspace changes and runtime verification.",
                ),
                owner: "main".to_string(),
                kind: "execute".to_string(),
            },
            AgentWorkflowStep {
                title: localized_text(language, "验证产物", "Verify artifacts"),
                purpose: localized_text(
                    language,
                    "确认所需工作区文件与运行证据。",
                    "Confirm required workspace files and runtime evidence.",
                ),
                owner: "verifier".to_string(),
                kind: "verify".to_string(),
            },
        ],
        delegates: vec![AgentWorkflowDelegate {
            name: "verifier".to_string(),
            purpose: localized_text(
                language,
                "基于确定性证据验证所需产物。",
                "Verify required artifacts from deterministic evidence.",
            ),
            input: tail_string(user_content.trim(), 280),
            output: String::new(),
            status: "planned".to_string(),
        }],
        verification: Vec::new(),
        repair_strategy: localized_text(
            language,
            "修复失败的运行或测试命令，并重新执行。",
            "repair the failing runtime/test command and re-run it",
        ),
        required_paths: Vec::new(),
    };
    plan = merge_required_paths_into_plan(plan, &required_paths, language);
    if plan.required_paths.is_empty() {
        return None;
    }
    if plan.workflow_kind.trim().is_empty() {
        plan.workflow_kind = if should_run_structured_workflow(stream_mode, user_content) {
            "research".to_string()
        } else {
            "agent".to_string()
        };
    }
    let _ = runtime;
    Some(plan)
}

fn can_finalize_from_hard_verifier_only(
    plan: &AgentWorkflowPlan,
    messages: &[MessageBlock],
    base_dir: &Path,
    runtime: &RuntimeSettings,
    required_path_snapshots: &[RequiredPathSnapshot],
    language: TurnLanguage,
) -> Option<(AgentVerifierReport, Vec<String>, Vec<String>)> {
    if !required_paths_ready_for_review(base_dir, runtime, plan)
        && !required_paths_likely_satisfied_by_evidence(
            plan,
            messages,
            base_dir,
            &runtime.workspace_root,
            required_path_snapshots,
        )
    {
        return None;
    }
    let (report, checkpoints, branch_notes) = build_hard_verifier_report(
        plan,
        messages,
        base_dir,
        &runtime.workspace_root,
        required_path_snapshots,
        language,
    );
    if report.status.eq_ignore_ascii_case("pass") {
        Some((report, checkpoints, branch_notes))
    } else {
        None
    }
}

fn provider_supports_streaming_tools(runtime: &RuntimeSettings) -> bool {
    let api_url = runtime.api_url.trim().to_ascii_lowercase();
    if api_url.is_empty() {
        return true;
    }

    if api_url.contains("api.deepseek.com") {
        return true;
    }

    if api_url.contains("api.openai.com") {
        return true;
    }

    false
}

fn provider_supports_native_tool_history(runtime: &RuntimeSettings) -> bool {
    let api_url = runtime.api_url.trim().to_ascii_lowercase();
    if api_url.is_empty() {
        return true;
    }

    if api_url.contains("api.deepseek.com") {
        return false;
    }

    provider_supports_streaming_tools(runtime)
}

fn summarize_tool_history_for_provider(messages: &[MessageBlock]) -> Vec<MessageBlock> {
    let mut summarized = Vec::with_capacity(messages.len());
    let mut pending_tool_calls: BTreeMap<String, (String, Value)> = BTreeMap::new();

    for block in messages {
        match block {
            MessageBlock::ToolCall {
                call_id,
                name,
                args,
                ..
            } => {
                pending_tool_calls.insert(call_id.clone(), (name.clone(), args.clone()));
            }
            MessageBlock::ToolResult {
                call_id,
                result,
                success,
            } => {
                let (name, args) = pending_tool_calls
                    .remove(call_id)
                    .unwrap_or_else(|| ("tool".to_string(), json!({})));
                let args_preview = summarize_tool_args_for_provider(&name, &args);
                let summary = summarize_tool_result_for_provider_memory(&name, result, *success);
                let context = format!(
                    "Tool {} {}.\nArguments: {}\nResult summary: {}",
                    name,
                    if *success { "succeeded" } else { "failed" },
                    if args_preview.trim().is_empty() { "{}" } else { args_preview.as_str() },
                    if summary.trim().is_empty() { "(empty)" } else { summary.as_str() }
                );
                summarized.push(MessageBlock::Assistant { content: context });
            }
            other => summarized.push(other.clone()),
        }
    }

    summarized
}

fn build_streaming_provider(
    state: &WebAppState,
    runtime: &RuntimeSettings,
) -> Result<Arc<dyn LLMProvider>> {
    let api_url = if runtime.api_url.trim().is_empty() {
        state.assistant_api_url.clone()
    } else {
        runtime.api_url.clone()
    };
    let api_key = runtime
        .api_key
        .clone()
        .or_else(|| state.assistant_api_key.clone())
        .unwrap_or_default();

    Ok(Arc::new(OpenAIProvider::with_base_url(
        api_key,
        api_url,
        Some(runtime.model.clone()),
    )))
}

fn build_workspace_completion_request(
    runtime: &RuntimeSettings,
    payload: &WorkspaceFileCompleteRequest,
) -> ChatRequest {
    let system_prompt = "You are a lightweight code completion engine. Return only valid JSON with the schema {\"items\":[{\"label\":\"...\",\"insert_text\":\"...\",\"detail\":\"...\",\"source\":\"llm\"}]}. Produce 1-5 concise continuations that fit the current code context. Do not include markdown fences or explanations. If there is no useful completion, return {\"items\":[]}.";
    let user_prompt = format!(
        "Path: {path}\nLanguage: {language}\nCursor line: {line}\nCursor column: {column}\nToken prefix: {token_prefix}\n\nCode before cursor:\n{prefix}\n\nCode after cursor:\n{suffix}",
        path = payload.path,
        language = payload.language,
        line = payload.cursor_line,
        column = payload.cursor_column,
        token_prefix = payload.token_prefix,
        prefix = payload.prefix,
        suffix = payload.suffix,
    );

    ChatRequest {
        model: runtime.model.clone(),
        messages: vec![
            Message::system(system_prompt),
            Message::user(&user_prompt),
        ],
        temperature: 0.2,
        max_tokens: Some(180),
        top_p: Some(0.95),
        stop: None,
        stream: false,
        tools: None,
        thinking_mode: provider_thinking_mode(runtime, false),
        reasoning_effort: provider_reasoning_effort(runtime),
    }
}

fn parse_workspace_completion_items(content: &str) -> Result<Vec<WorkspaceCodeCompletionItem>> {
    let raw = content.trim();
    if raw.is_empty() {
        return Ok(Vec::new());
    }

    let json_text = extract_json_object(raw).unwrap_or_else(|| raw.to_string());
    let value: Value = serde_json::from_str(&json_text)
        .or_else(|_| serde_json::from_str(raw))
        .map_err(|err| anyhow!("failed to parse completion json: {}", err))?;

    Ok(value
        .get("items")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| {
            let label = item
                .get("label")
                .and_then(|value| value.as_str())
                .or_else(|| item.get("insert_text").and_then(|value| value.as_str()))?
                .trim()
                .to_string();
            let insert_text = item
                .get("insert_text")
                .and_then(|value| value.as_str())
                .unwrap_or(&label)
                .trim()
                .to_string();
            if label.is_empty() || insert_text.is_empty() {
                return None;
            }
            Some(WorkspaceCodeCompletionItem {
                label,
                insert_text,
                detail: item
                    .get("detail")
                    .and_then(|value| value.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string(),
                source: item
                    .get("source")
                    .and_then(|value| value.as_str())
                    .unwrap_or("llm")
                    .trim()
                    .to_string(),
            })
        })
        .take(5)
        .collect())
}

fn extract_json_object(input: &str) -> Option<String> {
    let start = input.find('{')?;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;

    for (offset, ch) in input[start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let end = start + offset + ch.len_utf8();
                    return Some(input[start..end].to_string());
                }
            }
            _ => {}
        }
    }

    None
}

fn build_stream_chat_request_with_prompt(
    messages: &[MessageBlock],
    runtime: &RuntimeSettings,
    system_prompt: &str,
    language: TurnLanguage,
    tool_definitions: &[Value],
    base_dir: &Path,
) -> Result<ChatRequest> {
    let supports_streaming_tools = provider_supports_streaming_tools(runtime);
    let supports_native_tool_history = provider_supports_native_tool_history(runtime);
    let compressed_messages = sanitize_messages_for_stream_provider(
        compress_messages_for_request(messages, language),
        supports_native_tool_history,
    );
    let provider_messages = if supports_native_tool_history {
        compressed_messages
    } else {
        summarize_tool_history_for_provider(&compressed_messages)
    };
    let effective_system_prompt = if tool_definitions.is_empty() {
        system_prompt.to_string()
    } else {
        format!(
            "{}\n\n{}",
            system_prompt,
            localized_text(
                language,
                "工具使用策略:\n- 当前环境提供可用工具。\n- 需要工具时，使用原生 tool/function call 机制。\n- 创建或编辑工作区文件时，优先使用 write_file、edit_file、read_file 等直接文件工具，而不是终端 shell 重定向。\n- 不要在 assistant 正文里输出伪工具语法、XML、DSML 或包装标签。\n- 当真实工具调用可用时，不要把 Bash 或 shell 包装器当作普通文本输出。",
                "Tool-use policy:\n- Tools are available in this environment.\n- When a tool is needed, use the native tool/function call mechanism.\n- For creating or editing workspace files, prefer direct file tools such as write_file, edit_file, and read_file over terminal shell redirection.\n- Never emit pseudo tool syntax, XML, DSML, or wrapper tags in assistant text.\n- Never emit Bash or shell tool wrappers as plain text when a real tool call can be used."
            )
        )
    };
    let api_messages = {
        let _cwd_guard = enter_workspace_dir_from(base_dir, &runtime.workspace_root)?;
        build_conversation(&provider_messages, Some(&effective_system_prompt))
    };

    Ok(ChatRequest {
        model: runtime.model.clone(),
        messages: api_messages,
        temperature: effort_temperature(runtime),
        max_tokens: Some(effort_max_tokens(runtime)),
        top_p: None,
        stop: None,
        stream: true,
        tools: if tool_definitions.is_empty() || !supports_streaming_tools {
            None
        } else {
            Some(tool_definitions.to_vec())
        },
        thinking_mode: provider_thinking_mode(
            runtime,
            !tool_definitions.is_empty() && supports_streaming_tools,
        ),
        reasoning_effort: provider_reasoning_effort(runtime),
    })
}

fn sanitize_messages_for_stream_provider(
    messages: Vec<MessageBlock>,
    supports_streaming_tools: bool,
) -> Vec<MessageBlock> {
    let mut sanitized = Vec::with_capacity(messages.len());

    for block in messages {
        if matches!(block, MessageBlock::ToolCall { .. } | MessageBlock::ToolResult { .. })
            && !supports_streaming_tools
        {
            sanitized.push(block);
            continue;
        }

        if let Some(cleaned) = sanitize_message_block_for_stream_provider(block) {
            sanitized.push(cleaned);
        }
    }

    sanitized
}

fn compress_messages_for_request(messages: &[MessageBlock], language: TurnLanguage) -> Vec<MessageBlock> {
    const MAX_BLOCKS_BEFORE_COMPRESSION: usize = 48;
    const RECENT_BLOCKS_TO_KEEP: usize = 18;

    if messages.len() <= MAX_BLOCKS_BEFORE_COMPRESSION {
        return messages.to_vec();
    }

    let split_index = messages.len().saturating_sub(RECENT_BLOCKS_TO_KEEP);
    let (older, recent) = messages.split_at(split_index);
    let summary = summarize_message_blocks(older, language);
    if summary.trim().is_empty() {
        return messages.to_vec();
    }

    let mut compressed = Vec::with_capacity(recent.len() + 2);
    compressed.push(MessageBlock::System {
        content: localized_string(
            language,
            format!(
                "以下是更早对话轮次压缩后的上下文.\n请保留这些约束与已完成工作:\n{}",
                summary
            ),
            format!(
                "Context compressed from earlier conversation turns.\nPreserve these constraints and completed work:\n{}",
                summary
            ),
        ),
    });
    compressed.extend_from_slice(recent);
    compressed
}

fn summarize_message_blocks(messages: &[MessageBlock], language: TurnLanguage) -> String {
    let mut user_points = Vec::new();
    let mut assistant_points = Vec::new();
    let mut tool_points = Vec::new();
    let mut diff_points = Vec::new();

    for block in messages {
        match block {
            MessageBlock::User { content, .. } => {
                let line = tail_string(content.trim(), 220);
                if !line.is_empty() {
                    user_points.push(localized_string(
                        language,
                        format!("- 用户请求: {}", line),
                        format!("- User request: {}", line),
                    ));
                }
            }
            MessageBlock::Assistant { content } => {
                let line = tail_string(content.trim(), 220);
                if !line.is_empty() {
                    assistant_points.push(localized_string(
                        language,
                        format!("- 助手回复: {}", line),
                        format!("- Assistant response: {}", line),
                    ));
                }
            }
            MessageBlock::ToolCall { name, status, .. } => {
                tool_points.push(localized_string(
                    language,
                    format!(
                        "- 工具调用: {} [{}]",
                        name,
                        localized_status_text(language, tool_status_name(status))
                    ),
                    format!(
                        "- Tool call: {} [{}]",
                        name,
                        localized_status_text(language, tool_status_name(status))
                    ),
                ));
            }
            MessageBlock::ToolResult { result, success, .. } => {
                tool_points.push(localized_string(
                    language,
                    format!(
                        "- 工具结果({}): {}",
                        if *success { "成功" } else { "失败" },
                        tail_string(result.trim(), 180)
                    ),
                    format!(
                        "- Tool result ({}): {}",
                        if *success { "success" } else { "failure" },
                        tail_string(result.trim(), 180)
                    ),
                ));
            }
            MessageBlock::Diff { diff } => {
                diff_points.push(localized_string(
                    language,
                    format!(
                        "- 文件变更: {} (+{} / -{})",
                        diff.file_path, diff.added, diff.removed
                    ),
                    format!(
                        "- File changed: {} (+{} / -{})",
                        diff.file_path, diff.added, diff.removed
                    ),
                ));
            }
            MessageBlock::Thinking { content, .. } => {
                let line = tail_string(content.trim(), 160);
                if !line.is_empty() {
                    assistant_points.push(localized_string(
                        language,
                        format!("- 先前推理备注: {}", line),
                        format!("- Prior reasoning note: {}", line),
                    ));
                }
            }
            MessageBlock::System { content } => {
                let line = tail_string(content.trim(), 180);
                if !line.is_empty() {
                    assistant_points.push(localized_string(
                        language,
                        format!("- 系统备注: {}", line),
                        format!("- System note: {}", line),
                    ));
                }
            }
            MessageBlock::Error { content } => {
                let line = tail_string(content.trim(), 180);
                if !line.is_empty() {
                    tool_points.push(localized_string(
                        language,
                        format!("- 观察到错误: {}", line),
                        format!("- Error observed: {}", line),
                    ));
                }
            }
            MessageBlock::Subagent { record } => {
                let line = tail_string(
                    if !record.output.trim().is_empty() {
                        record.output.trim()
                    } else {
                        record.purpose.trim()
                    },
                    180,
                );
                if !line.is_empty() {
                    assistant_points.push(localized_string(
                        language,
                        format!(
                            "- 子代理 {} [{}]: {}",
                            record.name,
                            localized_status_text(language, &record.status),
                            line
                        ),
                        format!(
                            "- Subagent {} [{}]: {}",
                            record.name,
                            localized_status_text(language, &record.status),
                            line
                        ),
                    ));
                }
            }
            MessageBlock::Verification { report } => {
                let line = tail_string(report.summary.trim(), 180);
                if !line.is_empty() {
                    tool_points.push(localized_string(
                        language,
                        format!(
                            "- 验证器 {}: {}",
                            localized_status_text(language, &report.status),
                            line
                        ),
                        format!(
                            "- Verifier {}: {}",
                            localized_status_text(language, &report.status),
                            line
                        ),
                    ));
                }
            }
            MessageBlock::AssistantStreaming { .. } => {}
        }
    }

    let mut sections = Vec::new();
    if !user_points.is_empty() {
        let recent_user_points = user_points
            .iter()
            .cloned()
            .rev()
            .take(8)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        sections.push(localized_string(
            language,
            format!("更早的目标与请求:\n{}", recent_user_points),
            format!("Earlier goals and requests:\n{}", recent_user_points),
        ));
    }
    if !assistant_points.is_empty() {
        let recent_assistant_points = assistant_points
            .iter()
            .cloned()
            .rev()
            .take(8)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        sections.push(localized_string(
            language,
            format!("已建立的上下文:\n{}", recent_assistant_points),
            format!("Established context:\n{}", recent_assistant_points),
        ));
    }
    if !tool_points.is_empty() {
        let recent_tool_points = tool_points
            .iter()
            .cloned()
            .rev()
            .take(10)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        sections.push(localized_string(
            language,
            format!("工具执行记录:\n{}", recent_tool_points),
            format!("Tool execution record:\n{}", recent_tool_points),
        ));
    }
    if !diff_points.is_empty() {
        let recent_diff_points = diff_points
            .iter()
            .cloned()
            .rev()
            .take(8)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        sections.push(localized_string(
            language,
            format!("已完成的工作区变更:\n{}", recent_diff_points),
            format!("Workspace changes already made:\n{}", recent_diff_points),
        ));
    }

    sections.join("\n\n")
}

fn upsert_tool_call_block(
    persisted_blocks: &mut Vec<MessageBlock>,
    call_id: &str,
    name: &str,
    args: &Value,
    status: ToolCallStatus,
) {
    if let Some(existing) = persisted_blocks.iter_mut().rev().find(|block| {
        matches!(
            block,
            MessageBlock::ToolCall {
                call_id: existing_call_id,
                ..
            } if existing_call_id == call_id
        )
    }) {
        if let MessageBlock::ToolCall {
            name: existing_name,
            args: existing_args,
            status: existing_status,
            ..
        } = existing
        {
            *existing_name = name.to_string();
            *existing_args = args.clone();
            *existing_status = status;
        }
        return;
    }

    persisted_blocks.push(MessageBlock::ToolCall {
        name: name.to_string(),
        args: args.clone(),
        call_id: call_id.to_string(),
        status,
    });
}

fn upsert_diff_block(persisted_blocks: &mut Vec<MessageBlock>, diff: FileDiff) {
    if let Some(existing) = persisted_blocks.iter_mut().rev().find(|block| {
        matches!(
            block,
            MessageBlock::Diff { diff: existing_diff }
                if existing_diff.file_path == diff.file_path
        )
    }) {
        if let MessageBlock::Diff { diff: existing_diff } = existing {
            *existing_diff = diff;
        }
        return;
    }

    persisted_blocks.push(MessageBlock::Diff { diff });
}

fn system_prompt_for_mode(mode: Option<&str>) -> String {
    match mode.unwrap_or("chat").trim().to_ascii_lowercase().as_str() {
        "research" => research_mode_system_prompt(),
        "agent" => agent_mode_system_prompt(),
        _ => chat_mode_system_prompt(),
    }
}

fn turn_language_from_option(language: Option<&str>) -> TurnLanguage {
    match language.unwrap_or("zh").trim().to_ascii_lowercase().as_str() {
        "en" | "english" => TurnLanguage::En,
        _ => TurnLanguage::Zh,
    }
}

fn turn_language_name(language: TurnLanguage) -> &'static str {
    match language {
        TurnLanguage::Zh => "Chinese",
        TurnLanguage::En => "English",
    }
}

fn localized_text(language: TurnLanguage, zh: &'static str, en: &'static str) -> String {
    match language {
        TurnLanguage::Zh => zh.to_string(),
        TurnLanguage::En => en.to_string(),
    }
}

fn localized_string(language: TurnLanguage, zh: impl Into<String>, en: impl Into<String>) -> String {
    match language {
        TurnLanguage::Zh => zh.into(),
        TurnLanguage::En => en.into(),
    }
}

fn localized_status_text(language: TurnLanguage, status: &str) -> String {
    match status.trim().to_ascii_lowercase().as_str() {
        "pending" => localized_text(language, "等待中", "Pending"),
        "approved" => localized_text(language, "已批准", "Approved"),
        "denied" => localized_text(language, "已拒绝", "Denied"),
        "executing" | "running" => localized_text(language, "进行中", "Running"),
        "complete" | "completed" => localized_text(language, "已完成", "Complete"),
        "pass" | "passed" => localized_text(language, "通过", "Pass"),
        "repair" => localized_text(language, "待修复", "Repair"),
        "failed" | "failure" => localized_text(language, "失败", "Failed"),
        "edited" => localized_text(language, "已编辑", "Edited"),
        "skipped" => localized_text(language, "已跳过", "Skipped"),
        other => other.to_string(),
    }
}

fn try_finalize_current_turn_with_hard_verifier(
    tx: &tokio::sync::mpsc::UnboundedSender<StreamEnvelope>,
    state: &WebAppState,
    session_id: &str,
    runtime: &RuntimeSettings,
    persisted_blocks: &mut Vec<MessageBlock>,
    stream_mode: Option<&str>,
    plan: Option<&AgentWorkflowPlan>,
    required_path_snapshots: &[RequiredPathSnapshot],
    language: TurnLanguage,
    completion_detail: &str,
    emit_review_pass_activity: bool,
) -> Result<bool> {
    let Some(plan) = plan else {
        return Ok(false);
    };
    let Some((report, checkpoints, branch_notes)) = can_finalize_from_hard_verifier_only(
        plan,
        persisted_blocks,
        state.host.base_dir(),
        runtime,
        required_path_snapshots,
        language,
    ) else {
        return Ok(false);
    };

    persisted_blocks.push(MessageBlock::Verification {
        report: report.clone(),
    });
    sync_stream_runtime_messages(state, session_id, persisted_blocks);
    for checkpoint in &checkpoints {
        push_runtime_checkpoint(state, session_id, checkpoint.clone(), language);
    }
    for note in &branch_notes {
        push_runtime_branch_note(state, session_id, note.clone(), language);
    }
    emit_verifier_update(
        tx,
        state,
        session_id,
        runtime,
        persisted_blocks,
        stream_mode,
        &report,
        &checkpoints,
        &branch_notes,
        language,
    );
    if emit_review_pass_activity {
        emit_activity(
            tx,
            state,
            session_id,
            runtime,
            persisted_blocks,
            stream_mode,
            workflow_activity_event(
                "review",
                Some(report.summary.clone()),
                Some("review".to_string()),
                Some("pass".to_string()),
                report.evidence.first().cloned(),
                Some("verifier".to_string()),
            ),
        );
    }
    finalize_stream_success(
        tx,
        state,
        session_id,
        runtime,
        persisted_blocks,
        stream_mode,
        language,
        completion_detail,
    )?;
    Ok(true)
}

fn localized_required_target_text(language: TurnLanguage, path: &str) -> String {
    match language {
        TurnLanguage::Zh => format!("所需工作区目标已存在并已更新: {}", path),
        TurnLanguage::En => format!("required workspace target exists and is updated: {}", path),
    }
}

fn is_deepseek_runtime(runtime: &RuntimeSettings) -> bool {
    runtime
        .api_url
        .trim()
        .to_ascii_lowercase()
        .contains("api.deepseek.com")
}

fn deepseek_supports_reasoning_controls(runtime: &RuntimeSettings) -> bool {
    if !is_deepseek_runtime(runtime) {
        return false;
    }

    let model = runtime.model.trim().to_ascii_lowercase();
    model.contains("reasoner")
}

fn provider_thinking_mode(runtime: &RuntimeSettings, has_tools: bool) -> Option<String> {
    if !deepseek_supports_reasoning_controls(runtime) {
        return None;
    }

    if has_tools {
        return Some("disabled".to_string());
    }

    Some(if runtime.deep_think { "enabled" } else { "disabled" }.to_string())
}

fn provider_reasoning_effort(runtime: &RuntimeSettings) -> Option<String> {
    if !deepseek_supports_reasoning_controls(runtime) {
        return None;
    }

    Some(match runtime.reasoning_effort.trim().to_ascii_lowercase().as_str() {
        "low" => "low",
        "high" => "high",
        "max" => "high",
        _ => "medium",
    }.to_string())
}

fn workflow_mode(mode: Option<&str>) -> &'static str {
    match mode.unwrap_or("chat").trim().to_ascii_lowercase().as_str() {
        "research" | "spec" => "research",
        "agent" => "agent",
        _ => "chat",
    }
}

fn is_spec_request(content: &str) -> bool {
    content.trim_start().to_ascii_lowercase().starts_with("/spec")
}

fn should_run_structured_workflow(mode: Option<&str>, content: &str) -> bool {
    if is_spec_request(content) {
        return true;
    }

    let normalized_mode = workflow_mode(mode);
    if normalized_mode == "research" {
        return true;
    }

    let lowered = content.to_ascii_lowercase();
    normalized_mode == "agent"
        && (lowered.contains("implement")
            || lowered.contains("fix")
            || lowered.contains("refactor")
            || lowered.contains("debug")
            || content.contains("研究")
            || content.contains("实现")
            || content.contains("修复"))
}

fn infer_session_research_state(messages: &[MessageBlock]) -> SessionResearchState {
    let mut saw_user = false;
    for message in messages.iter().rev() {
        match message {
            MessageBlock::User { content, .. } => {
                saw_user = true;
                let trimmed = content.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if trimmed.to_ascii_lowercase().starts_with("/spec") {
                    return SessionResearchState::Research;
                }
                return SessionResearchState::Agent;
            }
            MessageBlock::Subagent { .. } | MessageBlock::Verification { .. } => {
                return SessionResearchState::Research;
            }
            MessageBlock::Thinking { content, .. } | MessageBlock::Assistant { content } => {
                if saw_user {
                    let lowered = content.to_ascii_lowercase();
                    if lowered.contains("verification target")
                        || lowered.contains("planner subagent")
                        || lowered.contains("hard verifier")
                        || lowered.contains("research workflow")
                        || content.contains("验证目标")
                        || content.contains("规划子代理")
                        || content.contains("硬验证器")
                        || content.contains("研究流程")
                    {
                        return SessionResearchState::Research;
                    }
                }
            }
            _ => {}
        }
    }
    SessionResearchState::Inactive
}

fn activity_event(
    label: impl Into<String>,
    detail: impl Into<Option<String>>,
) -> WebActivityEvent {
    WebActivityEvent {
        label: label.into(),
        detail: detail.into(),
        phase: None,
        status: None,
        meta: None,
        agent: None,
        delegates: None,
    }
}

fn workflow_activity_event(
    label: impl Into<String>,
    detail: impl Into<Option<String>>,
    phase: impl Into<Option<String>>,
    status: impl Into<Option<String>>,
    meta: impl Into<Option<String>>,
    agent: impl Into<Option<String>>,
) -> WebActivityEvent {
    WebActivityEvent {
        label: label.into(),
        detail: detail.into(),
        phase: phase.into(),
        status: status.into(),
        meta: meta.into(),
        agent: agent.into(),
        delegates: None,
    }
}

impl WebActivityEvent {
    fn with_delegates(mut self, delegates: Vec<AgentWorkflowDelegate>) -> Self {
        if !delegates.is_empty() {
            self.delegates = Some(delegates);
        }
        self
    }
}

fn to_web_subagent(record: &AgentSubagentRecord) -> WebSubagentEvent {
    WebSubagentEvent {
        id: record.id.clone(),
        name: record.name.clone(),
        purpose: record.purpose.clone(),
        input: record.input.clone(),
        output: record.output.clone(),
        status: record.status.clone(),
        kind: record.kind.clone(),
        started_at: record.started_at.clone(),
        completed_at: record.completed_at.clone(),
        evidence: record.evidence.clone(),
    }
}

fn to_web_verifier_check(check: &AgentVerifierCheck) -> WebVerifierCheck {
    WebVerifierCheck {
        id: check.id.clone(),
        title: check.title.clone(),
        status: check.status.clone(),
        detail: check.detail.clone(),
        evidence: check.evidence.clone(),
    }
}

fn to_web_verifier(report: &AgentVerifierReport) -> WebVerifierReport {
    WebVerifierReport {
        status: report.status.clone(),
        summary: report.summary.clone(),
        checks: report.checks.iter().map(to_web_verifier_check).collect(),
        issues: report.issues.clone(),
        evidence: report.evidence.clone(),
        next_actions: report.next_actions.clone(),
        deterministic: report.deterministic,
    }
}

fn upsert_runtime_subagent(state: &WebAppState, session_id: &str, subagent: &WebSubagentEvent) {
    if let Ok(mut sessions) = lock_stream_runtime(state) {
        if let Some(session) = sessions.get_mut(session_id) {
            if let Some(existing) = session.subagents.iter_mut().find(|item| item.id == subagent.id) {
                *existing = subagent.clone();
            } else {
                session.subagents.push(subagent.clone());
            }
        }
    }
}

fn set_runtime_verifier(
    state: &WebAppState,
    session_id: &str,
    report: &WebVerifierReport,
    checkpoints: &[String],
    branch_notes: &[String],
) {
    if let Ok(mut sessions) = lock_stream_runtime(state) {
        if let Some(session) = sessions.get_mut(session_id) {
            session.verifier = Some(report.clone());
            session.checkpoints = checkpoints.to_vec();
            session.branch_notes = branch_notes.to_vec();
        }
    }
}

fn build_research_runtime_event(
    messages: &[MessageBlock],
    session_runtime: Option<&StreamSessionRuntimeView>,
) -> WebResearchRuntimeEvent {
    let mut subagents = Vec::new();
    let mut verifier = None;

    if let Some(runtime) = session_runtime {
        subagents = runtime.subagents.clone();
        verifier = runtime.verifier.clone();
        return WebResearchRuntimeEvent {
            subagents,
            verifier,
            checkpoints: runtime.checkpoints.clone(),
            branch_notes: runtime.branch_notes.clone(),
            timeline: runtime.timeline.clone(),
            resumable: runtime
                .verifier
                .as_ref()
                .is_some_and(|report| report.status.eq_ignore_ascii_case("repair")),
        };
    }

    let start_index = messages
        .iter()
        .rposition(|message| matches!(message, MessageBlock::User { .. }))
        .unwrap_or(0);
    let tail = &messages[start_index..];
    let mut checkpoints = Vec::new();
    let mut branch_notes = Vec::new();
    for block in tail {
        match block {
            MessageBlock::Subagent { record } => subagents.push(to_web_subagent(record)),
            MessageBlock::Verification { report } => verifier = Some(to_web_verifier(report)),
            MessageBlock::Diff { diff } => checkpoints.push(format!(
                "{} (+{} / -{})",
                diff.file_path, diff.added, diff.removed
            )),
            MessageBlock::ToolResult { result, success, .. } if !success => {
                branch_notes.push(tail_string(result, 180));
            }
            _ => {}
        }
    }
    WebResearchRuntimeEvent {
        resumable: verifier
            .as_ref()
            .is_some_and(|report| report.status.eq_ignore_ascii_case("repair")),
        subagents,
        verifier,
        checkpoints,
        branch_notes,
        timeline: Vec::new(),
    }
}

fn clone_stream_runtime_view(runtime: &StreamSessionRuntime) -> StreamSessionRuntimeView {
    StreamSessionRuntimeView {
        subagents: runtime.subagents.clone(),
        verifier: runtime.verifier.clone(),
        checkpoints: runtime.checkpoints.clone(),
        branch_notes: runtime.branch_notes.clone(),
        timeline: runtime.timeline.clone(),
    }
}

fn push_runtime_timeline_event(
    state: &WebAppState,
    session_id: &str,
    kind: impl Into<String>,
    title: impl Into<String>,
    detail: impl Into<String>,
    status: impl Into<String>,
    agent: impl Into<String>,
) {
    if let Ok(mut sessions) = lock_stream_runtime(state) {
        if let Some(session) = sessions.get_mut(session_id) {
            session.timeline.push(WebTimelineEvent {
                kind: kind.into(),
                title: title.into(),
                detail: detail.into(),
                status: status.into(),
                agent: agent.into(),
                ts: web_now_iso(),
            });
            if session.timeline.len() > 40 {
                let drop_count = session.timeline.len().saturating_sub(40);
                session.timeline.drain(0..drop_count);
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
struct StreamSessionRuntimeView {
    subagents: Vec<WebSubagentEvent>,
    verifier: Option<WebVerifierReport>,
    checkpoints: Vec<String>,
    branch_notes: Vec<String>,
    timeline: Vec<WebTimelineEvent>,
}

struct ProgressNarration {
    text: String,
    dedupe_key: String,
}

fn build_delegate_statuses(
    delegates: &[AgentWorkflowDelegate],
    focus_name: &str,
    focus_status: &str,
    fallback_input: &str,
    fallback_output: &str,
) -> Vec<AgentWorkflowDelegate> {
    delegates
        .iter()
        .cloned()
        .map(|mut delegate| {
            if delegate.input.trim().is_empty() {
                delegate.input = fallback_input.trim().to_string();
            }
            if delegate.name.trim().eq_ignore_ascii_case(focus_name) {
                delegate.status = focus_status.trim().to_string();
                if !fallback_output.trim().is_empty() {
                    delegate.output = fallback_output.trim().to_string();
                }
            }
            delegate
        })
        .collect()
}

fn emit_activity(
    tx: &tokio::sync::mpsc::UnboundedSender<StreamEnvelope>,
    state: &WebAppState,
    session_id: &str,
    runtime: &RuntimeSettings,
    persisted_blocks: &[MessageBlock],
    mode: Option<&str>,
    activity: WebActivityEvent,
) {
    if let Ok(mut sessions) = lock_stream_runtime(state) {
        if let Some(session) = sessions.get_mut(session_id) {
            session.latest_activity = Some(activity.clone());
        }
    }
    let _ = tx.send(StreamEnvelope {
        r#type: "activity".to_string(),
        session_id: Some(session_id.to_string()),
        messages: None,
        delta: None,
        error: None,
        activity: Some(activity),
        tool: None,
        permission: None,
        edited_files: None,
        research: current_research_payload(state, Some(session_id), runtime, persisted_blocks, mode),
        subagents: None,
        verifier: None,
    });
}

fn web_now_iso() -> String {
    Local::now().to_rfc3339()
}

fn summarize_tool_target_for_chat(path: Option<&str>) -> String {
    let Some(path) = path.map(str::trim).filter(|value| !value.is_empty()) else {
        return String::new();
    };
    let normalized = path.replace('\\', "/");
    normalized
        .rsplit('/')
        .next()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .unwrap_or(normalized)
}

fn progress_key_with_target(base: &str, target: &str) -> String {
    let trimmed_base = base.trim();
    let trimmed_target = target.trim();
    if trimmed_base.is_empty() {
        return String::new();
    }
    if trimmed_target.is_empty() {
        return trimmed_base.to_string();
    }
    format!("{}:{}", trimmed_base, trimmed_target.to_ascii_lowercase())
}

fn normalize_progress_fingerprint(text: &str) -> String {
    text.to_ascii_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ('\u{4e00}'..='\u{9fff}').contains(&ch) {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn tool_progress_narration(
    language: TurnLanguage,
    tool_name: &str,
    status: &str,
    file_path: Option<&str>,
    result: Option<&str>,
) -> Option<ProgressNarration> {
    let target = summarize_tool_target_for_chat(file_path);
    let target_suffix = if target.is_empty() {
        String::new()
    } else if matches!(language, TurnLanguage::Zh) {
        format!("，目标是 {}", target)
    } else {
        format!(" for {}", target)
    };
    let normalized_name = tool_name.trim().to_ascii_lowercase();
    let normalized_status = status.trim().to_ascii_lowercase();
    let (summary, dedupe_key) = match (normalized_name.as_str(), normalized_status.as_str()) {
        ("list_dir", "pending") | ("find_files", "pending") | ("read_file", "pending") => {
            (
                localized_string(
                    language,
                    format!("我先检查当前工作区状态{}。", target_suffix),
                    format!("I’m first checking the current workspace state{}.", target_suffix),
                ),
                progress_key_with_target("inspect", &target),
            )
        }
        ("list_dir", "failed") | ("find_files", "failed") | ("read_file", "failed") => {
            (
                localized_string(
                    language,
                    "目标路径还没准备好，我先调整一下目录或路径再继续。".to_string(),
                    "That target path is not ready yet, so I’m adjusting the directory or path before continuing.".to_string(),
                ),
                progress_key_with_target("inspect-repair", &target),
            )
        }
        ("write_file", "complete") | ("edit_file", "complete") => (
            localized_string(
                language,
                format!("我已经把内容写进工作区文件了{}。", target_suffix),
                format!("I’ve written the content into the workspace file{}.", target_suffix),
            ),
            progress_key_with_target("workspace-write", &target),
        ),
        ("run_python", "executing") | ("run_python_file", "executing") => (
            localized_string(
                language,
                "脚本已经开始跑了，我现在看运行结果和生成的产物。".to_string(),
                "The script is running now, and I’m checking the result and generated artifacts.".to_string(),
            ),
            "script-running".to_string(),
        ),
        ("run_python", "complete") | ("run_python_file", "complete") => (
            localized_string(
                language,
                "脚本已经跑完了，我开始核对输出文件和实验结果。".to_string(),
                "The script finished, and I’m now checking the output files and experiment results.".to_string(),
            ),
            "script-complete".to_string(),
        ),
        ("run_command", "complete") | ("run_safe_command", "complete") | ("terminal_run", "complete") => {
            let result_hint = result
                .map(|raw| tail_string(raw, 120))
                .filter(|text| !text.trim().is_empty())
                .unwrap_or_default();
            let text = if matches!(language, TurnLanguage::Zh) {
                if result_hint.is_empty() {
                    "这一步命令已经完成，我继续根据输出往下推进。".to_string()
                } else {
                    format!("这一步命令已经完成，我先根据输出继续推进：{}。", result_hint)
                }
            } else if result_hint.is_empty() {
                "That command finished, and I’m continuing from its output.".to_string()
            } else {
                format!("That command finished, and I’m continuing from its output: {}.", result_hint)
            };
            (text, format!("command-complete:{}", normalized_name))
        }
        ("terminal_create", "complete") => (
            localized_string(
                language,
                "终端已经准备好了，后面需要的命令我会直接在这里接着跑。".to_string(),
                "The terminal is ready, so I’ll keep running the next commands there.".to_string(),
            ),
            "terminal-ready".to_string(),
        ),
        _ => return None,
    };

    Some(ProgressNarration {
        text: summary,
        dedupe_key,
    })
}

fn emit_assistant_progress_delta(
    tx: &tokio::sync::mpsc::UnboundedSender<StreamEnvelope>,
    state: &WebAppState,
    session_id: &str,
    dedupe_key: impl Into<String>,
    delta: impl Into<String>,
) {
    let dedupe_key = dedupe_key.into();
    let trimmed_key = dedupe_key.trim();
    let delta = delta.into();
    let trimmed = delta.trim();
    let normalized_fingerprint = normalize_progress_fingerprint(trimmed);
    let cooldown = Duration::from_secs(8);
    if trimmed.is_empty() || trimmed_key.is_empty() || normalized_fingerprint.is_empty() {
        return;
    }
    if let Ok(mut sessions) = lock_stream_runtime(state) {
        if let Some(session) = sessions.get_mut(session_id) {
            if let Some(last_emitted_at) = session.recent_progress_emitted_at.get(trimmed_key) {
                if last_emitted_at.elapsed() < cooldown {
                    return;
                }
            }
            if session
                .recent_progress_keys
                .iter()
                .rev()
                .take(3)
                .any(|existing| existing == trimmed_key)
            {
                return;
            }
            if session
                .progress_updates
                .iter()
                .rev()
                .take(4)
                .map(|item| normalize_progress_fingerprint(item))
                .any(|existing| {
                    !existing.is_empty()
                        && (existing == normalized_fingerprint
                            || existing.contains(&normalized_fingerprint)
                            || normalized_fingerprint.contains(&existing))
                })
            {
                return;
            }
            session.recent_progress_keys.push(trimmed_key.to_string());
            session
                .recent_progress_emitted_at
                .insert(trimmed_key.to_string(), Instant::now());
            if session.recent_progress_keys.len() > 6 {
                let drop_count = session.recent_progress_keys.len().saturating_sub(6);
                let removed: Vec<String> = session.recent_progress_keys.drain(0..drop_count).collect();
                for key in removed {
                    session.recent_progress_emitted_at.remove(&key);
                }
            }
            session.progress_updates.push(trimmed.to_string());
            if session.progress_updates.len() > 8 {
                let drop_count = session.progress_updates.len().saturating_sub(8);
                session.progress_updates.drain(0..drop_count);
            }
        }
    }
    let _ = tx.send(StreamEnvelope {
        r#type: "assistant_progress".to_string(),
        session_id: Some(session_id.to_string()),
        messages: None,
        delta: Some(trimmed.to_string()),
        error: None,
        activity: None,
        tool: None,
        permission: None,
        edited_files: None,
        research: None,
        subagents: None,
        verifier: None,
    });
}

fn push_runtime_checkpoint(
    state: &WebAppState,
    session_id: &str,
    note: impl Into<String>,
    language: TurnLanguage,
) {
    let note = note.into();
    if note.trim().is_empty() {
        return;
    }
    if let Ok(mut sessions) = lock_stream_runtime(state) {
        if let Some(session) = sessions.get_mut(session_id) {
            session.checkpoints.push(note.clone());
            if session.checkpoints.len() > 12 {
                let drop_count = session.checkpoints.len().saturating_sub(12);
                session.checkpoints.drain(0..drop_count);
            }
        }
    }
    push_runtime_timeline_event(
        state,
        session_id,
        "checkpoint",
        localized_text(language, "检查点", "Checkpoint"),
        note,
        "complete",
        "runtime",
    );
}

fn push_runtime_branch_note(
    state: &WebAppState,
    session_id: &str,
    note: impl Into<String>,
    language: TurnLanguage,
) {
    let note = note.into();
    if note.trim().is_empty() {
        return;
    }
    if let Ok(mut sessions) = lock_stream_runtime(state) {
        if let Some(session) = sessions.get_mut(session_id) {
            session.branch_notes.push(note.clone());
            if session.branch_notes.len() > 10 {
                let drop_count = session.branch_notes.len().saturating_sub(10);
                session.branch_notes.drain(0..drop_count);
            }
        }
    }
    push_runtime_timeline_event(
        state,
        session_id,
        "repair",
        localized_text(language, "修复备注", "Repair note"),
        note,
        "repair",
        "runtime",
    );
}

fn emit_subagent_update(
    tx: &tokio::sync::mpsc::UnboundedSender<StreamEnvelope>,
    state: &WebAppState,
    session_id: &str,
    runtime: &RuntimeSettings,
    persisted_blocks: &[MessageBlock],
    mode: Option<&str>,
    record: &AgentSubagentRecord,
) {
    let web_record = to_web_subagent(record);
    upsert_runtime_subagent(state, session_id, &web_record);
    push_runtime_timeline_event(
        state,
        session_id,
        "subagent",
        record.name.clone(),
        if record.output.trim().is_empty() {
            record.purpose.clone()
        } else {
            tail_string(&record.output, 220)
        },
        record.status.clone(),
        record.name.clone(),
    );
    let _ = tx.send(StreamEnvelope {
        r#type: "subagent".to_string(),
        session_id: Some(session_id.to_string()),
        messages: None,
        delta: None,
        error: None,
        activity: Some(workflow_activity_event(
            "subagent",
            Some(record.name.clone()),
            Some("delegate".to_string()),
            Some(record.status.clone()),
            Some(record.kind.clone()),
            Some(record.name.clone()),
        )),
        tool: None,
        permission: None,
        edited_files: None,
        research: current_research_payload(state, Some(session_id), runtime, persisted_blocks, mode),
        subagents: Some(vec![web_record]),
        verifier: None,
    });
}

fn emit_verifier_update(
    tx: &tokio::sync::mpsc::UnboundedSender<StreamEnvelope>,
    state: &WebAppState,
    session_id: &str,
    runtime: &RuntimeSettings,
    persisted_blocks: &[MessageBlock],
    mode: Option<&str>,
    report: &AgentVerifierReport,
    checkpoints: &[String],
    branch_notes: &[String],
    language: TurnLanguage,
) {
    let web_report = to_web_verifier(report);
    set_runtime_verifier(state, session_id, &web_report, checkpoints, branch_notes);
    push_runtime_timeline_event(
        state,
        session_id,
        "verifier",
        localized_text(language, "验证器", "Verifier"),
        tail_string(&report.summary, 220),
        report.status.clone(),
        "verifier",
    );
    let _ = tx.send(StreamEnvelope {
        r#type: "verifier".to_string(),
        session_id: Some(session_id.to_string()),
        messages: None,
        delta: None,
        error: None,
        activity: Some(workflow_activity_event(
            "verifier",
            Some(report.summary.clone()),
            Some("verify".to_string()),
            Some(report.status.clone()),
            Some(if report.deterministic {
                "hard-verifier".to_string()
            } else {
                "model-verifier".to_string()
            }),
            Some("verifier".to_string()),
        )),
        tool: None,
        permission: None,
        edited_files: None,
        research: current_research_payload(state, Some(session_id), runtime, persisted_blocks, mode),
        subagents: None,
        verifier: Some(web_report),
    });
}

fn push_runtime_tool_event(
    state: &WebAppState,
    session_id: &str,
    tool: &WebToolEvent,
) {
    if let Ok(mut sessions) = lock_stream_runtime(state) {
        if let Some(session) = sessions.get_mut(session_id) {
            if let Some(existing) = session
                .tool_events
                .iter_mut()
                .find(|item| item.call_id == tool.call_id)
            {
                *existing = tool.clone();
            } else {
                session.tool_events.push(tool.clone());
            }
            if session.tool_events.len() > 12 {
                let drop_count = session.tool_events.len().saturating_sub(12);
                session.tool_events.drain(0..drop_count);
            }
        }
    }
}

fn push_runtime_edited_files(
    state: &WebAppState,
    session_id: &str,
    files: &[WebEditedFile],
) {
    if let Ok(mut sessions) = lock_stream_runtime(state) {
        if let Some(session) = sessions.get_mut(session_id) {
            for file in files {
                if let Some(existing) = session.edited_files.iter_mut().find(|item| item.path == file.path) {
                    *existing = file.clone();
                } else {
                    session.edited_files.push(file.clone());
                }
            }
            if session.edited_files.len() > 12 {
                let drop_count = session.edited_files.len().saturating_sub(12);
                session.edited_files.drain(0..drop_count);
            }
        }
    }
}

fn web_edited_file_from_diff(diff: &FileDiff) -> WebEditedFile {
    WebEditedFile {
        path: diff.file_path.clone(),
        added: diff.added,
        removed: diff.removed,
        before_content: diff.before_content.clone(),
        after_content: diff.after_content.clone(),
    }
}

fn refresh_runtime_review_from_filesystem(
    state: &WebAppState,
    session_id: &str,
    runtime: &RuntimeSettings,
) {
    if let Ok(mut sessions) = lock_stream_runtime(state) {
        if let Some(session) = sessions.get_mut(session_id) {
            let valid_paths = session
                .edited_files
                .iter()
                .filter_map(|file| {
                    resolve_workspace_relative_path(
                        state.host.base_dir(),
                        &runtime.workspace_root,
                        &file.path,
                    )
                    .filter(|absolute| absolute.exists() || file.before_content.is_empty())
                    .map(|_| file.path.clone())
                })
                .collect::<Vec<_>>();

            if valid_paths.is_empty() {
                session.edited_files.clear();
                return;
            }

            if let Ok(review) =
                try_build_review_payload(state.host.base_dir(), &runtime.workspace_root, &valid_paths)
            {
                let review_map = review
                    .files
                    .into_iter()
                    .map(|file| (file.path.clone(), file))
                    .collect::<HashMap<_, _>>();
                session.edited_files.retain(|file| review_map.contains_key(&file.path));
                for file in &mut session.edited_files {
                    if let Some(summary) = review_map.get(&file.path) {
                        file.added = summary.additions as usize;
                        file.removed = summary.deletions as usize;
                    }
                }
            }
        }
    }
}

fn line_suggests_artifact(line: &str) -> bool {
    let lowered = line.to_ascii_lowercase();
    [
        "saved",
        "written",
        "generated",
        "exported",
        "rendered",
        "artifact",
        "output",
        "report",
        "plot",
        "figure",
        "image",
        "results",
        "saved to",
        "saved at",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

fn is_path_wrapper_char(ch: char) -> bool {
    matches!(ch, '"' | '\'' | '`' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>')
}

fn is_path_trailing_char(ch: char) -> bool {
    matches!(
        ch,
        ',' | ';' | ':' | '!' | '?' | ')' | ']' | '}' | '"' | '\'' | '`' | '，' | '。' | '；' | '：'
    )
}

fn extract_spaced_output_path_from_line(line: &str) -> Option<String> {
    let separators = [
        "written:",
        "written to:",
        "saved to:",
        "saved results to:",
        "saved result to:",
        "saved plot to:",
        "saved figure to:",
        "saved image to:",
        "saved silhouette plot to:",
        "saved at:",
        "output:",
        "outputs saved to:",
        "all outputs saved to:",
        "artifact:",
        "artifacts:",
    ];
    let lowered = line.to_ascii_lowercase();
    for separator in separators {
        if let Some(index) = lowered.find(separator) {
            let candidate = line[index + separator.len()..].trim();
            if looks_like_output_path(candidate) {
                return Some(candidate.to_string());
            }
        }
    }

    if let Some(index) = lowered.rfind(" to ") {
        let prefix = lowered[..index].trim();
        let candidate = line[index + 4..].trim();
        if !candidate.is_empty()
            && looks_like_output_path(candidate)
            && ["saved", "written", "generated", "exported", "rendered"]
                .iter()
                .any(|needle| prefix.contains(needle))
        {
            return Some(candidate.to_string());
        }
    }
    None
}

fn collect_workspace_paths_from_text(text: &str, out: &mut Vec<String>) {
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        let normalized_line = line.replace('\\', "/");
        if let Some(candidate) = extract_spaced_output_path_from_line(&normalized_line) {
            out.push(candidate);
        }
        if looks_like_output_path(&normalized_line) && !normalized_line.contains(' ') {
            out.push(normalized_line.clone());
        }

        if !line_suggests_artifact(&normalized_line) {
            continue;
        }

        out.extend(extract_required_workspace_paths_from_text(&normalized_line));
        for token in normalized_line.split_whitespace() {
            let candidate = token
                .trim()
                .trim_matches(is_path_wrapper_char)
                .trim_end_matches(is_path_trailing_char);
            if looks_like_output_path(candidate) {
                out.push(candidate.to_string());
            }
        }
    }
}

fn json_path_key_suggests_output(prefix: &str) -> bool {
    let lowered = prefix.to_ascii_lowercase();
    [
        "output",
        "artifact",
        "result",
        "report",
        "plot",
        "figure",
        "image",
        "save",
        "saved",
        "export",
        "generated",
        "render",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

fn collect_json_output_paths(value: &Value, prefix: &str, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let next_prefix = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                collect_json_output_paths(child, &next_prefix, out);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                let next_prefix = if prefix.is_empty() {
                    format!("[{index}]")
                } else {
                    format!("{prefix}[{index}]")
                };
                collect_json_output_paths(child, &next_prefix, out);
            }
        }
        Value::String(text) => {
            let trimmed = text.trim();
            if !trimmed.is_empty() && looks_like_output_path(trimmed) && json_path_key_suggests_output(prefix) {
                out.push(trimmed.to_string());
            }
        }
        _ => {}
    }
}

fn relativize_workspace_file(workspace: &Path, absolute: &Path) -> Option<String> {
    let normalized_absolute = absolute.canonicalize().unwrap_or_else(|_| absolute.to_path_buf());
    let relative = normalized_absolute
        .strip_prefix(workspace)
        .ok()?
        .to_string_lossy()
        .replace('\\', "/");
    if relative.is_empty()
        || relative.starts_with(".git/")
        || relative.starts_with(".tokitai-run/")
    {
        return None;
    }
    sanitize_review_path(&relative).ok()
}

fn tool_output_base_dir(
    base_dir: &Path,
    workspace_root: &str,
    tool_name: &str,
    args: &Value,
) -> Option<PathBuf> {
    if let Some(raw) = extract_tool_path(tool_name, args)
        .or_else(|| args.get("cwd").and_then(Value::as_str).map(|value| value.replace('\\', "/")))
    {
        let resolved = resolve_workspace_relative_path(base_dir, workspace_root, &raw)?;
        if resolved.is_dir() {
            return Some(resolved);
        }
        if let Some(parent) = resolved.parent() {
            return Some(parent.to_path_buf());
        }
    }

    canonical_workspace_dir_from(base_dir, workspace_root).ok()
}

fn normalize_workspace_output_path(
    base_dir: &Path,
    workspace_root: &str,
    raw_path: &str,
    base_hint: Option<&Path>,
) -> Option<String> {
    let mut candidate = raw_path
        .trim()
        .trim_matches(is_path_wrapper_char)
        .trim_end_matches(is_path_trailing_char)
        .trim()
        .to_string();

    if let Some(rest) = candidate.strip_prefix("output file ") {
        candidate = rest.trim().to_string();
    }
    if let Some(rest) = candidate.strip_prefix("file://") {
        candidate = rest.trim().to_string();
    }
    candidate = candidate.replace('\\', "/");
    if let Some(rest) = candidate.strip_prefix("//?/") {
        candidate = rest.trim().to_string();
    }
    if let Some(rest) = candidate.strip_prefix("/?/") {
        candidate = rest.trim().to_string();
    }
    if candidate.is_empty() {
        return None;
    }

    let workspace = canonical_workspace_dir_from(base_dir, workspace_root).ok()?;
    let trimmed_relative = candidate
        .trim_start_matches("./")
        .trim_start_matches(".\\")
        .trim_start_matches('/');
    let absolute = base_hint
        .and_then(|dir| {
            let joined = dir.join(trimmed_relative);
            if joined.exists() && joined.is_file() {
                Some(joined)
            } else {
                None
            }
        })
        .or_else(|| {
            let resolved = resolve_workspace_relative_path(base_dir, workspace_root, &candidate)?;
            if resolved.exists() && resolved.is_file() {
                Some(resolved)
            } else {
                None
            }
        })?;

    relativize_workspace_file(&workspace, &absolute)
}

fn runtime_tool_excluded_paths(
    base_dir: &Path,
    workspace_root: &str,
    tool_name: &str,
    args: &Value,
) -> BTreeSet<String> {
    let mut excluded = BTreeSet::new();
    let base_hint = tool_output_base_dir(base_dir, workspace_root, tool_name, args);
    if matches!(tool_name, "run_python_file" | "read_file" | "write_file" | "edit_file" | "delete_file") {
        if let Some(path) = args.get("path").and_then(Value::as_str) {
            if let Some(normalized) =
                normalize_workspace_output_path(base_dir, workspace_root, path, base_hint.as_deref())
            {
                excluded.insert(normalized.to_ascii_lowercase());
            }
        }
    }
    excluded
}

fn collect_runtime_artifact_paths(
    base_dir: &Path,
    workspace_root: &str,
    tool_name: &str,
    args: &Value,
    raw: &str,
    success: bool,
) -> Vec<String> {
    if !matches!(
        tool_name,
        "run_command"
            | "run_safe_command"
            | "run_python"
            | "run_python_file"
            | "run_r"
            | "run_julia"
            | "terminal_run"
    ) {
        return Vec::new();
    }

    let parsed = parse_tool_result_evidence(tool_name, raw, success);
    if !parsed.success || parsed.timed_out || parsed.exit_code.unwrap_or(0) != 0 {
        return Vec::new();
    }

    let base_hint = tool_output_base_dir(base_dir, workspace_root, tool_name, args);
    let excluded = runtime_tool_excluded_paths(base_dir, workspace_root, tool_name, args);
    let mut raw_candidates = Vec::new();
    collect_workspace_paths_from_text(&parsed.stdout, &mut raw_candidates);
    collect_workspace_paths_from_text(&parsed.stderr, &mut raw_candidates);

    if let Ok(value) = serde_json::from_str::<Value>(raw) {
        collect_json_output_paths(&value, "", &mut raw_candidates);
    }

    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    for candidate in raw_candidates {
        let Some(path) =
            normalize_workspace_output_path(base_dir, workspace_root, &candidate, base_hint.as_deref())
        else {
            continue;
        };
        let lowered = path.to_ascii_lowercase();
        if excluded.contains(&lowered) || !seen.insert(lowered) {
            continue;
        }
        normalized.push(path);
    }
    normalized
}

fn build_workspace_artifact_diff(
    base_dir: &Path,
    workspace_root: &str,
    relative_path: &str,
) -> Option<FileDiff> {
    let absolute = resolve_workspace_relative_path(base_dir, workspace_root, relative_path)?;
    if !absolute.exists() || !absolute.is_file() {
        return None;
    }

    let preview_kind = workspace_preview_kind(relative_path);
    if matches!(preview_kind.as_str(), "text" | "markdown") {
        let bytes = std::fs::read(&absolute).ok()?;
        let preview_limit = 220_000usize;
        let preview_bytes = if bytes.len() > preview_limit {
            &bytes[..preview_limit]
        } else {
            &bytes[..]
        };
        let content = decode_bytes(preview_bytes);
        Some(FileDiff::compute(relative_path, "", &content))
    } else {
        Some(FileDiff {
            file_path: relative_path.to_string(),
            lines: vec![DiffLine::Header(format!("Generated artifact: {}", relative_path))],
            added: 0,
            removed: 0,
            before_content: String::new(),
            after_content: String::new(),
        })
    }
}

fn collect_runtime_artifact_diffs(
    base_dir: &Path,
    workspace_root: &str,
    tool_name: &str,
    args: &Value,
    raw: &str,
    success: bool,
) -> Vec<FileDiff> {
    collect_runtime_artifact_paths(base_dir, workspace_root, tool_name, args, raw, success)
        .into_iter()
        .filter_map(|path| build_workspace_artifact_diff(base_dir, workspace_root, &path))
        .collect()
}

#[derive(Debug, Clone, Default)]
struct VerificationEvidence {
    tool_name: String,
    success: bool,
    exit_code: Option<i64>,
    timed_out: bool,
    stdout: String,
    stderr: String,
    summary: String,
    json_evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
struct WebTimelineEvent {
    kind: String,
    title: String,
    detail: String,
    status: String,
    agent: String,
    ts: String,
}

fn parse_tool_result_evidence(tool_name: &str, raw: &str, success: bool) -> VerificationEvidence {
    let mut evidence = VerificationEvidence {
        tool_name: tool_name.to_string(),
        success,
        summary: tail_string(raw, 220),
        ..VerificationEvidence::default()
    };
    if let Ok(value) = serde_json::from_str::<Value>(raw) {
        if matches!(tool_name, "run_command" | "run_safe_command") {
            evidence.success = value.get("success").and_then(Value::as_bool).unwrap_or(success);
            evidence.exit_code = value.pointer("/data/exit_code").and_then(Value::as_i64);
            evidence.stdout = value
                .pointer("/data/stdout")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            evidence.stderr = value
                .pointer("/data/stderr")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
        } else if matches!(tool_name, "run_python" | "run_python_file" | "run_r" | "run_julia") {
            evidence.success = value
                .pointer("/result/status")
                .and_then(Value::as_str)
                .map(|status| status.eq_ignore_ascii_case("success"))
                .unwrap_or(success);
            evidence.exit_code = value.pointer("/result/exit_code").and_then(Value::as_i64);
            evidence.timed_out = value
                .pointer("/result/timed_out")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            evidence.stdout = value
                .pointer("/result/stdout")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            evidence.stderr = value
                .pointer("/result/stderr")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
                } else if tool_name == "terminal_run" {
                    evidence.success = value.get("success").and_then(Value::as_bool).unwrap_or(success);
                    evidence.stdout = value
                        .pointer("/terminal/buffer")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                } else if tool_name == "terminal_read" {
                    evidence.success = value.get("success").and_then(Value::as_bool).unwrap_or(success);
                    evidence.stdout = value
                        .pointer("/terminal/buffer")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                }
        collect_json_text_evidence(&value, "", &mut evidence.json_evidence);
        if evidence.summary.is_empty() {
            evidence.summary = tail_string(raw, 220);
        }
    }
    evidence
}

fn collect_json_text_evidence(value: &Value, prefix: &str, out: &mut Vec<String>) {
    if out.len() >= 32 {
        return;
    }

    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let next_prefix = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                collect_json_text_evidence(child, &next_prefix, out);
                if out.len() >= 32 {
                    break;
                }
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                let next_prefix = if prefix.is_empty() {
                    format!("[{index}]")
                } else {
                    format!("{prefix}[{index}]")
                };
                collect_json_text_evidence(child, &next_prefix, out);
                if out.len() >= 32 {
                    break;
                }
            }
        }
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return;
            }
            let label = if prefix.is_empty() {
                tail_string(trimmed, 220)
            } else {
                format!("{prefix}: {}", tail_string(trimmed, 200))
            };
            if !out.iter().any(|item| item == &label) {
                out.push(label);
            }
            if looks_like_output_path(trimmed) {
                let artifact_line = format!("output file {}", trimmed.replace('\\', "/"));
                if !out.iter().any(|item| item == &artifact_line) {
                    out.push(artifact_line);
                }
            }
        }
        Value::Number(number) => {
            if prefix.is_empty() {
                return;
            }
            let label = format!("{prefix}: {number}");
            if !out.iter().any(|item| item == &label) {
                out.push(label);
            }
        }
        Value::Bool(flag) => {
            if prefix.is_empty() {
                return;
            }
            let label = format!("{prefix}: {flag}");
            if !out.iter().any(|item| item == &label) {
                out.push(label);
            }
        }
        Value::Null => {}
    }
}

fn looks_like_output_path(input: &str) -> bool {
    let lowered = input.trim().to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let file_suffixes = [
        ".json", ".csv", ".tsv", ".txt", ".md", ".html", ".pdf", ".png", ".jpg", ".jpeg",
        ".svg", ".tex", ".log", ".yaml", ".yml", ".toml",
    ];
    lowered.contains('/') || lowered.contains('\\') || file_suffixes.iter().any(|suffix| lowered.ends_with(suffix))
}

fn verification_target_keywords(target: &str) -> Vec<String> {
    let lowered = target.to_ascii_lowercase();
    let mut keywords = Vec::new();

    let mapping: &[(&[&str], &[&str])] = &[
        (&["scaling", "standardscaler", "standardization", "preprocessing", "z-score"], &["scaler", "standard", "scaled", "z-score", "preprocessing"]),
        (&["converge", "converges", "convergence", "stable clusters", "stability"], &["converged", "iterations", "stable", "reproducibility", "random_state"]),
        (&["adjusted rand", "ari", "silhouette", "nmi", "metric", "metrics", "cluster assignments"], &["adjusted rand", "ari", "silhouette", "nmi", "accuracy", "confusion matrix"]),
        (&["without errors", "runs without errors", "consistent output", "reproducible"], &["done.", "results saved", "reproducibility", "pass", "success"]),
        (&["output file", "json output", "results file", "artifact", "saved figure", "saved plot"], &["results saved", ".json", "output", "artifact", "output file", "saved", ".png", ".csv"]),
    ];

    for (needles, emits) in mapping {
        if needles.iter().any(|needle| lowered.contains(needle)) {
            for emit in *emits {
                keywords.push((*emit).to_string());
            }
        }
    }

    if keywords.is_empty() {
        keywords.extend(
            lowered
                .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-')
                .filter(|part| part.len() >= 4)
                .take(8)
                .map(|part| part.to_string()),
        );
    }

    keywords.sort();
    keywords.dedup();
    keywords
}

fn verification_target_matches(target: &str, evidence_pool: &[String]) -> Vec<String> {
    let lowered_target = target.to_ascii_lowercase();
    let keywords = verification_target_keywords(target);
    let target_paths = extract_required_workspace_paths_from_text(target)
        .into_iter()
        .map(|path| path.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let mut matches = Vec::new();

    for candidate in evidence_pool {
        let lowered = candidate.to_ascii_lowercase();
        if !target_paths.is_empty()
            && !target_paths
                .iter()
                .any(|path| lowered.contains(path.as_str()))
        {
            continue;
        }
        let keyword_hit = keywords.iter().any(|keyword| lowered.contains(keyword));
        let direct_hit = !lowered_target.trim().is_empty() && lowered.contains(&lowered_target);
        if keyword_hit || direct_hit {
            let clipped = tail_string(candidate, 220);
            if !clipped.trim().is_empty() && !matches.iter().any(|item| item == &clipped) {
                matches.push(clipped);
            }
        }
    }

    matches
}

fn current_turn_start_index(messages: &[MessageBlock]) -> usize {
    messages
        .iter()
        .rposition(|message| matches!(message, MessageBlock::User { .. }))
        .map(|index| index + 1)
        .unwrap_or(0)
}

fn current_turn_blocks(messages: &[MessageBlock]) -> &[MessageBlock] {
    &messages[current_turn_start_index(messages)..]
}

fn tool_name_by_call_id(messages: &[MessageBlock], call_id: &str) -> Option<String> {
    messages.iter().rev().find_map(|block| match block {
        MessageBlock::ToolCall {
            call_id: candidate,
            name,
            ..
        } if candidate == call_id => Some(name.clone()),
        _ => None,
    })
}

fn tool_args_by_call_id(messages: &[MessageBlock], call_id: &str) -> Option<Value> {
    messages.iter().rev().find_map(|block| match block {
        MessageBlock::ToolCall {
            call_id: candidate,
            args,
            ..
        } if candidate == call_id => Some(args.clone()),
        _ => None,
    })
}

fn tool_result_identity(messages: &[MessageBlock], call_id: &str) -> (String, Option<String>) {
    let tool_name = tool_name_by_call_id(messages, call_id).unwrap_or_else(|| "tool".to_string());
    let tool_path = tool_args_by_call_id(messages, call_id)
        .and_then(|args| extract_tool_path(&tool_name, &args))
        .map(|path| path.to_ascii_lowercase());
    (tool_name, tool_path)
}

fn fingerprint_bytes(bytes: &[u8]) -> u64 {
    let mut hash = 1469598103934665603u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1099511628211u64);
    }
    hash
}

fn read_workspace_file_fingerprint(path: &Path) -> Option<u64> {
    std::fs::read(path).ok().map(|bytes| fingerprint_bytes(&bytes))
}

fn capture_required_path_snapshots(
    base_dir: &Path,
    runtime: &RuntimeSettings,
    required_paths: &[String],
) -> Vec<RequiredPathSnapshot> {
    let mut snapshots = Vec::new();
    let mut seen = BTreeSet::new();
    for display_path in required_paths {
        if !seen.insert(display_path.to_ascii_lowercase()) {
            continue;
        }
        let Some(absolute_path) =
            resolve_workspace_relative_path(base_dir, &runtime.workspace_root, display_path)
        else {
            continue;
        };
        let existed_before = absolute_path.exists();
        let fingerprint_before = if existed_before && absolute_path.is_file() {
            read_workspace_file_fingerprint(&absolute_path)
        } else {
            None
        };
        snapshots.push(RequiredPathSnapshot {
            display_path: display_path.clone(),
            absolute_path,
            existed_before,
            fingerprint_before,
        });
    }
    snapshots
}

fn required_paths_ready_for_review(
    base_dir: &Path,
    runtime: &RuntimeSettings,
    plan: &AgentWorkflowPlan,
) -> bool {
    if plan.required_paths.is_empty() {
        return true;
    }
    let snapshots = capture_required_path_snapshots(base_dir, runtime, &plan.required_paths);
    if snapshots.is_empty() {
        return false;
    }
    snapshots
        .into_iter()
        .all(|snapshot| snapshot.absolute_path.exists())
}

fn dynamic_turn_round_limit(plan: Option<&AgentWorkflowPlan>, structured_workflow: bool) -> usize {
    let base = if structured_workflow { 180usize } else { 96usize };
    let Some(plan) = plan else {
        return base;
    };

    let step_bonus = plan.steps.len().saturating_sub(if structured_workflow { 4 } else { 3 }) * 12;
    let required_path_bonus = plan.required_paths.len().min(10) * 18;
    let verification_bonus = plan.verification.len().min(12) * 10;
    let workflow_bonus = if plan.workflow_kind.eq_ignore_ascii_case("research") {
        180
    } else {
        0
    };
    let long_plan_bonus = if plan.steps.len() + plan.verification.len() + plan.required_paths.len() >= 12 {
        if structured_workflow { 180 } else { 96 }
    } else {
        0
    };

    let mut limit = base + step_bonus + required_path_bonus + verification_bonus + workflow_bonus + long_plan_bonus;
    if plan.workflow_kind.eq_ignore_ascii_case("research") {
        limit = limit.max(if structured_workflow { 720 } else { 420 });
    }
    limit.clamp(base, if structured_workflow { 1800 } else { 900 })
}

fn verifier_report_has_only_soft_evidence_gaps(report: &AgentVerifierReport) -> bool {
    if report.issues.is_empty() {
        return false;
    }

    let mut saw_soft_gap = false;
    for check in &report.checks {
        if check.status.eq_ignore_ascii_case("failed") {
            return false;
        }
        if check.id.starts_with("required-path-miss-") {
            return false;
        }
        if check.id.starts_with("target-miss-") {
            saw_soft_gap = true;
        }
    }

    for issue in &report.issues {
        let lowered = issue.trim().to_ascii_lowercase();
        if lowered.is_empty() {
            continue;
        }
        let is_soft_gap = lowered.contains("missing verification evidence")
            || lowered.contains("verification stage finished without strong validation evidence")
            || lowered.contains("缺少针对")
            || lowered.contains("验证阶段结束时缺少足够强的验证证据")
            || (issue.contains("楠岃瘉") && issue.contains("璇佹嵁"));
        if !is_soft_gap {
            return false;
        }
        saw_soft_gap = true;
    }

    saw_soft_gap
}

fn required_paths_likely_satisfied_by_evidence(
    plan: &AgentWorkflowPlan,
    messages: &[MessageBlock],
    base_dir: &Path,
    workspace_root: &str,
    required_path_snapshots: &[RequiredPathSnapshot],
) -> bool {
    if plan.required_paths.is_empty() {
        return true;
    }

    let turn_blocks = current_turn_blocks(messages);
    let mut changed_file_lookup = BTreeSet::new();
    let mut successful_tool_paths = BTreeSet::new();

    for block in turn_blocks {
        match block {
            MessageBlock::Diff { diff } => {
                changed_file_lookup.insert(diff.file_path.to_ascii_lowercase());
            }
            MessageBlock::ToolResult {
                call_id,
                result,
                success,
            } => {
                let (tool_name, tool_path) = tool_result_identity(turn_blocks, call_id);
                let parsed = parse_tool_result_evidence(&tool_name, result, *success);
                let passed = parsed.success && parsed.exit_code.unwrap_or(0) == 0 && !parsed.timed_out;
                if passed {
                    if let Some(path) = tool_path {
                        successful_tool_paths.insert(path);
                    }
                }
            }
            _ => {}
        }
    }

    plan.required_paths.iter().all(|required_path| {
        let lowered = required_path.to_ascii_lowercase();
        let snapshot = required_path_snapshots
            .iter()
            .find(|item| item.display_path.eq_ignore_ascii_case(required_path));
        let absolute_path = snapshot
            .map(|item| item.absolute_path.clone())
            .or_else(|| resolve_workspace_relative_path(base_dir, workspace_root, required_path));

        let exists_now = absolute_path.as_ref().is_some_and(|path| path.exists());
        if !exists_now {
            return false;
        }

        let exact_changed = changed_file_lookup.contains(&lowered)
            || absolute_path.as_ref().is_some_and(|path| {
                changed_file_lookup.contains(&display_workspace_path(path).to_ascii_lowercase())
            });
        let tool_hit = successful_tool_paths.iter().any(|path| {
            path.contains(&lowered)
                || absolute_path.as_ref().is_some_and(|resolved| {
                    path.contains(&display_workspace_path(resolved).to_ascii_lowercase())
                })
        });

        exact_changed || tool_hit
    })
}

fn turn_made_real_progress(
    recent_blocks: &[MessageBlock],
    base_dir: &Path,
    workspace_root: &str,
) -> bool {
    recent_blocks.iter().any(|block| match block {
        MessageBlock::Assistant { content } | MessageBlock::AssistantStreaming { content } => {
            let text = sanitize_visible_stream_text(content);
            !text.trim().is_empty()
        }
        MessageBlock::Diff { diff } => {
            !diff.file_path.trim().is_empty() || diff.added > 0 || diff.removed > 0
        }
        MessageBlock::ToolResult {
            call_id,
            result,
            success,
        } => {
            let tool_name = if recent_blocks.is_empty() {
                "tool".to_string()
            } else {
                tool_name_by_call_id(recent_blocks, call_id).unwrap_or_else(|| "tool".to_string())
            };
            let parsed = parse_tool_result_evidence(&tool_name, result, *success);
            if parsed.success && parsed.exit_code.unwrap_or(0) == 0 && !parsed.timed_out {
                return true;
            }
            let lowered_result = result.to_ascii_lowercase();
            let workspace_hint = workspace_root.to_ascii_lowercase();
            let path_hint = display_workspace_path(base_dir).to_ascii_lowercase();
            lowered_result.contains(&workspace_hint) || lowered_result.contains(&path_hint)
        }
        MessageBlock::Subagent { record } => {
            matches!(
                record.status.to_ascii_lowercase().as_str(),
                "pass" | "complete" | "running"
            ) && (!record.output.trim().is_empty() || !record.evidence.is_empty())
        }
        MessageBlock::Verification { report } => {
            !report.summary.trim().is_empty()
                || !report.evidence.is_empty()
                || !report.checks.is_empty()
        }
        _ => false,
    })
}

fn build_hard_verifier_report(
    plan: &AgentWorkflowPlan,
    messages: &[MessageBlock],
    base_dir: &Path,
    workspace_root: &str,
    required_path_snapshots: &[RequiredPathSnapshot],
    language: TurnLanguage,
) -> (AgentVerifierReport, Vec<String>, Vec<String>) {
    let turn_blocks = current_turn_blocks(messages);
    let mut checks = Vec::new();
    let mut issues = Vec::new();
    let mut evidence_lines = Vec::new();
    let mut checkpoints = Vec::new();
    let mut branch_notes = Vec::new();
    let mut diff_count = 0usize;
    let mut has_successful_runtime = false;
    let mut has_failed_runtime = false;
    let mut verification_hits = 0usize;
    let mut changed_files = BTreeSet::new();
    let mut observed_tool_names = BTreeSet::new();
    let mut saw_terminal_runtime = false;
    let mut saw_validation_signal = false;
    let mut saw_test_failure = false;
    let mut saw_missing_required_path = false;
    let mut saw_missing_edit = false;
    let mut saw_missing_runtime = false;
    let mut saw_research_runtime_gap = false;
    let mut saw_soft_evidence_gap = false;
    let mut target_evidence_pool = Vec::new();
    let mut changed_file_lookup = BTreeSet::new();
    let mut successful_tool_targets = BTreeSet::new();
    let plan_has_runtime_step = plan
        .steps
        .iter()
        .any(|step| matches!(step.kind.as_str(), "run" | "verify"));
    let plan_is_file_evidence_friendly = !plan.required_paths.is_empty()
        || plan
            .verification
            .iter()
            .any(|target| !extract_required_workspace_paths_from_text(target).is_empty())
        || plan
            .steps
            .iter()
            .all(|step| matches!(step.kind.as_str(), "inspect" | "edit" | "summarize" | "research" | "verify"));

    for block in turn_blocks {
        if let MessageBlock::ToolResult {
            call_id,
            result,
            success,
        } = block
        {
            let (tool_name, tool_path) = tool_result_identity(turn_blocks, call_id);
            let parsed = parse_tool_result_evidence(&tool_name, result, *success);
            let passed = parsed.success && parsed.exit_code.unwrap_or(0) == 0 && !parsed.timed_out;
            if passed {
                successful_tool_targets.insert((tool_name, tool_path));
            }
        }
    }

    for block in turn_blocks {
        match block {
            MessageBlock::ToolResult {
                call_id,
                result,
                success,
            } => {
                let (tool_name, tool_path) = tool_result_identity(turn_blocks, call_id);
                observed_tool_names.insert(tool_name.clone());
                let parsed = parse_tool_result_evidence(&tool_name, result, *success);
                let passed = parsed.success && parsed.exit_code.unwrap_or(0) == 0 && !parsed.timed_out;
                let is_superseded_failure = !passed
                    && successful_tool_targets.contains(&(tool_name.clone(), tool_path.clone()));
                if matches!(
                    tool_name.as_str(),
                    "run_command"
                        | "run_safe_command"
                        | "run_python"
                        | "run_python_file"
                        | "run_r"
                        | "run_julia"
                        | "terminal_run"
                        | "terminal_read"
                ) {
                    if passed {
                        has_successful_runtime = true;
                    } else if !is_superseded_failure {
                        has_failed_runtime = true;
                    }
                }
                if matches!(tool_name.as_str(), "terminal_run" | "terminal_read") {
                    saw_terminal_runtime = true;
                }
                if passed
                    && matches!(
                        tool_name.as_str(),
                        "write_file" | "edit_file" | "read_file" | "delete_file"
                    )
                {
                    saw_validation_signal = true;
                }
                let tool_corpus = format!(
                    "{}\n{}\n{}",
                    parsed.stdout.to_ascii_lowercase(),
                    parsed.stderr.to_ascii_lowercase(),
                    parsed.summary.to_ascii_lowercase()
                );
                for snippet in [&parsed.stdout, &parsed.stderr, &parsed.summary] {
                    if !snippet.trim().is_empty() {
                        target_evidence_pool.push(snippet.clone());
                    }
                }
                target_evidence_pool.extend(parsed.json_evidence.clone());
                if contains_any(
                    &tool_corpus,
                    &[
                        "hello world",
                        "test passed",
                        "tests passed",
                        "all tests passed",
                        "verification passed",
                        "validated",
                        "accuracy",
                        "f1",
                        "auc",
                        "silhouette",
                        "passed",
                        "success",
                    ],
                ) {
                    saw_validation_signal = true;
                }
                if contains_any(
                    &tool_corpus,
                    &[
                        "test failed",
                        "tests failed",
                        "assertionerror",
                        "traceback",
                        "exception",
                        "error:",
                        "failed",
                    ],
                ) && !passed
                    && !is_superseded_failure
                {
                    saw_test_failure = true;
                }
                if !passed && !is_superseded_failure {
                    let detail = if parsed.timed_out {
                        match language {
                            TurnLanguage::Zh => format!("{} 执行超时", tool_name),
                            TurnLanguage::En => format!("{} timed out", tool_name),
                        }
                    } else if let Some(code) = parsed.exit_code {
                        match language {
                            TurnLanguage::Zh => format!("{} 以退出码 {} 结束", tool_name, code),
                            TurnLanguage::En => format!("{} exited with {}", tool_name, code),
                        }
                    } else {
                        match language {
                            TurnLanguage::Zh => format!("{} 执行失败", tool_name),
                            TurnLanguage::En => format!("{} reported failure", tool_name),
                        }
                    };
                    issues.push(detail.clone());
                    branch_notes.push(detail.clone());
                    checks.push(AgentVerifierCheck {
                        id: format!("tool-{}", call_id),
                        title: tool_name.clone(),
                        status: "failed".to_string(),
                        detail,
                        evidence: vec![
                            tail_string(&parsed.stdout, 180),
                            tail_string(&parsed.stderr, 180),
                            tail_string(&parsed.summary, 180),
                        ]
                        .into_iter()
                        .filter(|item| !item.trim().is_empty())
                        .collect(),
                    });
                } else if is_superseded_failure {
                    checks.push(AgentVerifierCheck {
                        id: format!("tool-{}", call_id),
                        title: tool_name.clone(),
                        status: "skipped".to_string(),
                        detail: localized_text(
                            language,
                            "同一工具目标后续已成功重试，因此更早的失败已被覆盖",
                            "Earlier failure was superseded by a later successful retry for the same tool target",
                        ),
                        evidence: vec![
                            tail_string(&parsed.stderr, 180),
                            tail_string(&parsed.summary, 180),
                        ]
                        .into_iter()
                        .filter(|item| !item.trim().is_empty())
                        .collect(),
                    });
                } else {
                    evidence_lines.push(localized_string(
                        language,
                        format!(
                            "{} 成功{}",
                            tool_name,
                            parsed
                                .exit_code
                                .map(|code| format!("（退出码 {}）", code))
                                .unwrap_or_default()
                        ),
                        format!(
                            "{} ok{}",
                            tool_name,
                            parsed
                                .exit_code
                                .map(|code| format!(" (exit {})", code))
                                .unwrap_or_default()
                        ),
                    ));
                    checks.push(AgentVerifierCheck {
                        id: format!("tool-{}", call_id),
                        title: tool_name.clone(),
                        status: "passed".to_string(),
                        detail: localized_text(
                            language,
                            "运行/工具执行成功",
                            "Runtime/tool execution succeeded",
                        ),
                        evidence: vec![
                            tail_string(&parsed.stdout, 180),
                            tail_string(&parsed.summary, 180),
                        ]
                        .into_iter()
                        .filter(|item| !item.trim().is_empty())
                        .collect(),
                    });
                }
            }
            MessageBlock::Diff { diff } => {
                diff_count += 1;
                changed_files.insert(diff.file_path.clone());
                changed_file_lookup.insert(diff.file_path.to_ascii_lowercase());
                target_evidence_pool.push(format!(
                    "{}",
                    localized_string(
                        language,
                        format!("已编辑 {} (+{} / -{})", diff.file_path, diff.added, diff.removed),
                        format!("edited {} (+{} / -{})", diff.file_path, diff.added, diff.removed)
                    )
                ));
                checkpoints.push(format!(
                    "{} (+{} / -{})",
                    diff.file_path, diff.added, diff.removed
                ));
            }
            _ => {}
        }
    }

    for required_path in &plan.required_paths {
        let lowered = required_path.to_ascii_lowercase();
        let snapshot = required_path_snapshots
            .iter()
            .find(|item| item.display_path.eq_ignore_ascii_case(required_path));
        let absolute_path = snapshot
            .map(|item| item.absolute_path.clone())
            .or_else(|| resolve_workspace_relative_path(base_dir, workspace_root, required_path));
        let exact_changed = changed_file_lookup.contains(&lowered)
            || absolute_path.as_ref().is_some_and(|path| {
                changed_file_lookup.contains(&display_workspace_path(path).to_ascii_lowercase())
            });
        let mut exists_now = false;
        let mut changed_on_disk = false;
        let mut content_excerpt = None;
        if let Some(path) = absolute_path.as_ref() {
            exists_now = path.exists();
            if exists_now && path.is_file() {
                let fingerprint_now = read_workspace_file_fingerprint(path);
                let fingerprint_before = snapshot.and_then(|item| item.fingerprint_before);
                changed_on_disk = match (snapshot, fingerprint_now) {
                    (Some(before), Some(now)) => !before.existed_before || before.fingerprint_before != Some(now),
                    (Some(before), None) => !before.existed_before,
                    (None, Some(_)) => true,
                    (None, None) => false,
                };
                if let Ok(content) = read_text_file(path) {
                    let excerpt = content.lines().take(24).collect::<Vec<_>>().join(" ");
                    if !excerpt.trim().is_empty() {
                        content_excerpt = Some(tail_string(&excerpt, 600));
                    }
                }
            } else if exists_now {
                changed_on_disk = snapshot.map(|item| !item.existed_before).unwrap_or(true);
            }
        }

        if exists_now {
            target_evidence_pool.push(format!("workspace path exists: {}", required_path));
            if let Some(excerpt) = content_excerpt.as_ref() {
                target_evidence_pool.push(format!(
                    "workspace file {} content: {}",
                    required_path, excerpt
                ));
            }
        }

        let tool_path_hit = successful_tool_targets.iter().any(|(_, path)| {
            path.as_ref().is_some_and(|candidate| {
                candidate.contains(&lowered)
                    || absolute_path.as_ref().is_some_and(|resolved| {
                        candidate.contains(&display_workspace_path(resolved).to_ascii_lowercase())
                    })
            })
        });

        if exists_now && (exact_changed || changed_on_disk || tool_path_hit) {
            checks.push(AgentVerifierCheck {
                id: format!("required-path-{}", checks.len() + 1),
                title: required_path.clone(),
                status: "passed".to_string(),
                detail: localized_text(
                    language,
                    "所需工作区路径已存在，并且本轮确实发生了变更",
                    "Required workspace path exists and changed during this turn",
                ),
                evidence: vec![
                    format!("required path satisfied: {}", required_path),
                    absolute_path
                        .as_ref()
                        .map(|path| format!("filesystem path: {}", display_workspace_path(path)))
                        .unwrap_or_default(),
                ]
                .into_iter()
                .filter(|item| !item.trim().is_empty())
                .collect(),
            });
        } else {
            saw_missing_required_path = true;
            let detail = if !exists_now {
                match language {
                    TurnLanguage::Zh => format!("缺少所需工作区路径：{}", required_path),
                    TurnLanguage::En => format!("required workspace path missing: {}", required_path),
                }
            } else {
                match language {
                    TurnLanguage::Zh => format!("所需工作区路径本轮未发生变更：{}", required_path),
                    TurnLanguage::En => format!("required workspace path unchanged this turn: {}", required_path),
                }
            };
            issues.push(detail.clone());
            checks.push(AgentVerifierCheck {
                id: format!("required-path-miss-{}", checks.len() + 1),
                title: required_path.clone(),
                status: "missing".to_string(),
                detail: localized_text(
                    language,
                    "本轮没有创建或更新用户明确指定的精确工作区目标",
                    "The exact user-requested workspace target was not created or updated in this turn",
                ),
                evidence: absolute_path
                    .as_ref()
                    .map(|path| vec![format!("filesystem path: {}", display_workspace_path(path))])
                    .unwrap_or_default(),
            });
        }
    }

    for target in &plan.verification {
        let matched_evidence = verification_target_matches(target, &target_evidence_pool);
        if !matched_evidence.is_empty() {
            verification_hits += 1;
            checks.push(AgentVerifierCheck {
                id: format!("target-{}", verification_hits),
                title: target.clone(),
                status: "passed".to_string(),
                detail: localized_text(
                    language,
                    "验证目标已与执行证据对齐",
                    "Verification target matched execution evidence",
                ),
                evidence: matched_evidence,
            });
        } else if !target.trim().is_empty() {
            saw_soft_evidence_gap = true;
            issues.push(match language {
                TurnLanguage::Zh => format!("缺少针对“{}”的验证证据", target),
                TurnLanguage::En => format!("missing verification evidence for '{}'", target),
            });
            checks.push(AgentVerifierCheck {
                id: format!("target-miss-{}", checks.len() + 1),
                title: target.clone(),
                status: "missing".to_string(),
                detail: localized_text(
                    language,
                    "没有硬证据匹配到这个验证目标",
                    "No hard evidence matched this verification target",
                ),
                evidence: vec![],
            });
        }
    }

    if diff_count == 0 && plan.steps.iter().any(|step| step.kind == "edit") {
        saw_missing_edit = true;
        issues.push(localized_text(
            language,
            "没有观察到计划中的文件编辑。",
            "planned file edits were not observed",
        ));
    }

    if !has_successful_runtime && plan_has_runtime_step && !plan_is_file_evidence_friendly {
        saw_missing_runtime = true;
        issues.push(localized_text(
            language,
            "没有观察到成功的运行或验证命令。",
            "no successful runtime or verification command was observed",
        ));
    }

    if plan.steps.iter().any(|step| step.kind == "verify") && !saw_validation_signal && verification_hits == 0 && !plan_is_file_evidence_friendly {
        saw_soft_evidence_gap = true;
        issues.push(localized_text(
            language,
            "验证阶段结束时缺少足够强的验证证据。",
            "verification stage finished without strong validation evidence",
        ));
    }

    if plan.workflow_kind.eq_ignore_ascii_case("research")
        && !changed_files.is_empty()
        && !has_successful_runtime
        && !saw_terminal_runtime
        && !plan_is_file_evidence_friendly
    {
        saw_research_runtime_gap = true;
        issues.push(localized_text(
            language,
            "研究工作流虽然修改了文件，但没有产出运行证据。",
            "research workflow changed files but did not produce runtime evidence",
        ));
    }

    if saw_test_failure {
        branch_notes.push(localized_text(
            language,
            "检测到失败的测试或运行证据，需要进入修复分支。",
            "detected failing test/runtime evidence that should be repaired",
        ));
    }

    if !changed_files.is_empty() {
        let changed_files_list = changed_files.iter().cloned().collect::<Vec<_>>().join(", ");
        evidence_lines.push(localized_string(
            language,
            format!("已编辑文件：{}", changed_files_list),
            format!("edited files: {}", changed_files_list),
        ));
    }

    if !observed_tool_names.is_empty() {
        let observed_tool_names_list = observed_tool_names
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        evidence_lines.push(localized_string(
            language,
            format!("已观察到的工具：{}", observed_tool_names_list),
            format!("observed tools: {}", observed_tool_names_list),
        ));
    }

    let soft_evidence_only = !issues.is_empty()
        && !has_failed_runtime
        && !saw_test_failure
        && !saw_missing_required_path
        && !saw_missing_edit
        && !saw_missing_runtime
        && !saw_research_runtime_gap
        && saw_soft_evidence_gap
        && (has_successful_runtime
            || !plan.steps.iter().any(|step| matches!(step.kind.as_str(), "run" | "verify"))
            || !plan.required_paths.is_empty());
    if soft_evidence_only {
        branch_notes.push(localized_text(
            language,
            "硬验证器检测到真实落盘与文件系统证据已成立，剩余缺口仅属于软性验证证据，因此直接放行完成。",
            "Hard verifier accepted completion because real filesystem-backed outputs were already present and only soft verification evidence gaps remained.",
        ));
        evidence_lines.push(localized_text(
            language,
            "已基于真实文件落盘与工具证据提前通过验证。",
            "verification accepted early from real file writes and tool evidence",
        ));
        issues.clear();
    }

    let status = if has_failed_runtime || !issues.is_empty() {
        "repair"
    } else {
        "pass"
    };
    let summary = if status == "pass" {
        if diff_count > 0 {
            match language {
                TurnLanguage::Zh => format!(
                    "硬验证器已通过：共检测到 {} 个编辑文件，匹配到 {} 个验证目标。",
                    diff_count, verification_hits
                ),
                TurnLanguage::En => format!(
                    "Hard verifier passed with {} edited files and {} matched verification targets.",
                    diff_count, verification_hits
                ),
            }
        } else {
            match language {
                TurnLanguage::Zh => format!(
                    "硬验证器已通过：匹配到 {} 个验证目标。",
                    verification_hits
                ),
                TurnLanguage::En => format!(
                    "Hard verifier passed with {} matched verification targets.",
                    verification_hits
                ),
            }
        }
    } else {
        match language {
            TurnLanguage::Zh => format!("硬验证器发现 {} 个问题。", issues.len().max(1)),
            TurnLanguage::En => format!("Hard verifier found {} issue(s).", issues.len().max(1)),
        }
    };
    let next_actions = if status == "repair" {
        let mut actions = Vec::new();
        if has_failed_runtime {
            actions.push(localized_text(
                language,
                "修复失败的运行或测试命令，并重新执行。",
                "repair the failing runtime/test command and re-run it",
            ));
        }
        if diff_count == 0 && plan.steps.iter().any(|step| step.kind == "edit") {
            actions.push(localized_text(
                language,
                "在结束前先补上预期的工作区文件编辑。",
                "apply the intended workspace edits before finalizing",
            ));
        }
        if verification_hits < plan.verification.len() {
            actions.push(localized_text(
                language,
                "为每一个计划中的验证目标补充明确证据。",
                "produce explicit verification evidence for each planned target",
            ));
        }
        if !saw_validation_signal && plan.steps.iter().any(|step| step.kind == "verify") && !plan_is_file_evidence_friendly {
            actions.push(localized_text(
                language,
                "执行一个具体的验证或测试步骤，并记录结果。",
                "run a concrete validation or test step and capture the result",
            ));
        }
        if actions.is_empty() {
            actions.push(localized_text(
                language,
                "在结束前补充更强的工具或测试证据。",
                "collect stronger tool or test evidence before finalizing",
            ));
        }
        actions
    } else {
        Vec::new()
    };

    (
        AgentVerifierReport {
            status: status.to_string(),
            summary,
            checks,
            issues,
            evidence: evidence_lines,
            next_actions,
            deterministic: true,
        },
        checkpoints,
        branch_notes,
    )
}

fn extract_json_value<T: for<'de> Deserialize<'de>>(raw: &str) -> Result<T> {
    let json_text = extract_json_object(raw).unwrap_or_else(|| raw.trim().to_string());
    serde_json::from_str(&json_text)
        .or_else(|_| serde_json::from_str(raw.trim()))
        .map_err(|err| anyhow!("failed to parse structured agent output: {}", err))
}

async fn provider_completion(
    provider: Arc<dyn LLMProvider>,
    runtime: &RuntimeSettings,
    messages: Vec<Message>,
) -> Result<String> {
    let response = provider
        .chat(ChatRequest {
            model: runtime.model.clone(),
            messages,
            temperature: 0.2,
            max_tokens: Some((effort_max_tokens(runtime) / 3).max(256)),
            top_p: Some(0.95),
            stop: None,
            stream: false,
            tools: None,
            thinking_mode: provider_thinking_mode(runtime, false),
            reasoning_effort: provider_reasoning_effort(runtime),
        })
        .await?;
    Ok(strip_emoji(&response.content))
}

async fn generate_agent_plan(
    provider: Arc<dyn LLMProvider>,
    runtime: &RuntimeSettings,
    messages: &[MessageBlock],
    user_content: &str,
    mode: Option<&str>,
    language: TurnLanguage,
) -> Result<AgentWorkflowPlan> {
    let mode_name = workflow_mode(mode);
    let required_paths = collect_required_workspace_paths_from_user_content(user_content);
    let conversation = build_conversation(messages, Some(system_prompt_for_mode(mode).as_str()));
    let transcript = conversation
        .iter()
        .rev()
        .take(8)
        .rev()
        .map(|message| format!("{}: {}", message.role, message.content))
        .collect::<Vec<_>>()
        .join("\n");
    let planner_prompt = match language {
        TurnLanguage::Zh => format!(
            "你是面向代码与科研 IDE 的规划子代理。只返回 JSON。\n\n任务模式：{mode_name}\n用户请求：\n{user_content}\n\n最近对话：\n{transcript}\n\n明确要求的工作区路径：\n{required_paths}\n\n输出 schema：\n{{\"workflow_kind\":\"chat|implementation|research\",\"goal\":\"...\",\"summary\":\"...\",\"steps\":[{{\"title\":\"...\",\"purpose\":\"...\",\"owner\":\"main|planner|reviewer|verifier|repairer\",\"kind\":\"inspect|edit|run|verify|summarize|research\"}}],\"delegates\":[{{\"name\":\"planner|reviewer|verifier|repairer\",\"purpose\":\"...\"}}],\"verification\":[\"...\"],\"repair_strategy\":\"...\",\"required_paths\":[\"...\"]}}\n\n规则：\n- 保持 2-6 个步骤。\n- 只要任务可能编辑文件或运行代码，就优先安排 reviewer 和 verifier。\n- 如果只是轻量对话，计划要短。\n- 如果是研究任务，要包含证据收集与验证。\n- 如果用户明确给出了工作区路径，必须保留在 required_paths 中，并围绕这些精确目标设计步骤和验证。\n- 不要把这些路径替换成别的相似文件或目录。\n- 不要输出 markdown 代码块。",
            required_paths = if required_paths.is_empty() {
                "无".to_string()
            } else {
                required_paths.join("\n")
            }
        ),
        TurnLanguage::En => format!(
            "You are the planner subagent for a coding and research IDE agent.\nReturn only JSON.\n\nTask mode: {mode_name}\nUser request:\n{user_content}\n\nRecent conversation:\n{transcript}\n\nExplicit required workspace paths:\n{required_paths}\n\nOutput schema:\n{{\"workflow_kind\":\"chat|implementation|research\",\"goal\":\"...\",\"summary\":\"...\",\"steps\":[{{\"title\":\"...\",\"purpose\":\"...\",\"owner\":\"main|planner|reviewer|verifier|repairer\",\"kind\":\"inspect|edit|run|verify|summarize|research\"}}],\"delegates\":[{{\"name\":\"planner|reviewer|verifier|repairer\",\"purpose\":\"...\"}}],\"verification\":[\"...\"],\"repair_strategy\":\"...\",\"required_paths\":[\"...\"]}}\n\nRules:\n- Keep 2-6 steps.\n- Choose reviewer and verifier delegates whenever the task may edit files or run code.\n- For lightweight chat, keep the plan short.\n- For research, include evidence gathering and verification.\n- If explicit required workspace paths are listed, keep them in required_paths and build steps and verification around those exact targets.\n- Do not replace required paths with semantically similar files in another directory.\n- No markdown fences.",
            required_paths = if required_paths.is_empty() {
                "none".to_string()
            } else {
                required_paths.join("\n")
            }
        ),
    };
    let content = provider_completion(
        provider,
        runtime,
        vec![
            Message::system(&localized_text(
                language,
                "你是严格的规划子代理。只输出 JSON。",
                "You are a strict planning subagent. Output JSON only.",
            )),
            Message::user(&planner_prompt),
        ],
    )
    .await?;
    let mut plan: AgentWorkflowPlan = extract_json_value(&content)?;
    for delegate in &mut plan.delegates {
        if delegate.status.trim().is_empty() {
            delegate.status = "planned".to_string();
        }
        if delegate.input.trim().is_empty() {
            delegate.input = user_content.trim().to_string();
        }
        if delegate.output.trim().is_empty() {
            delegate.output = plan.summary.clone();
        }
    }
    if plan.workflow_kind.trim().is_empty() {
        plan.workflow_kind = mode_name.to_string();
    }
    if plan.goal.trim().is_empty() {
        plan.goal = user_content.trim().to_string();
    }
    if plan.summary.trim().is_empty() {
        plan.summary = plan.goal.clone();
    }
    if plan.steps.is_empty() {
        plan.steps.push(AgentWorkflowStep {
            title: localized_text(language, "执行请求", "Execute request"),
            purpose: localized_text(
                language,
                "按需使用工具和文件编辑来完成用户请求。",
                "Carry out the user request with tools and file edits as needed.",
            ),
            owner: "main".to_string(),
            kind: "execute".to_string(),
        });
    }
    if required_paths.is_empty() {
        plan.required_paths = normalize_required_workspace_paths(plan.required_paths);
    } else {
        plan.required_paths = required_paths.clone();
        plan.verification = plan
            .verification
            .into_iter()
            .filter(|item| verification_item_matches_required_paths(item, &required_paths))
            .collect();
    }
    if !plan.required_paths.is_empty() {
        for path in &plan.required_paths {
            if !plan
                .verification
                .iter()
                .any(|item| item.to_ascii_lowercase().contains(&path.to_ascii_lowercase()))
            {
                plan.verification
                    .push(localized_required_target_text(language, path));
            }
        }
    }
    Ok(plan)
}

async fn review_agent_progress(
    provider: Arc<dyn LLMProvider>,
    runtime: &RuntimeSettings,
    plan: &AgentWorkflowPlan,
    messages: &[MessageBlock],
    user_content: &str,
    language: TurnLanguage,
) -> Result<AgentCritiqueReport> {
    let transcript = messages
        .iter()
        .rev()
        .take(18)
        .rev()
        .map(|block| match block {
            MessageBlock::User { content, .. } => format!("user: {}", content),
            MessageBlock::Assistant { content } => format!("assistant: {}", content),
            MessageBlock::ToolCall { name, status, .. } => {
                format!("tool_call: {} [{}]", name, tool_status_name(status))
            }
            MessageBlock::ToolResult { result, success, .. } => format!(
                "tool_result: {} :: {}",
                if *success { "success" } else { "failure" },
                tail_string(result, 400)
            ),
            MessageBlock::Diff { diff } => format!(
                "diff: {} (+{} -{})",
                diff.file_path, diff.added, diff.removed
            ),
            MessageBlock::Thinking { content, .. } => format!("thinking: {}", content),
            MessageBlock::System { content } => format!("system: {}", content),
            MessageBlock::Error { content } => format!("error: {}", content),
            MessageBlock::AssistantStreaming { content } => format!("assistant_stream: {}", content),
            MessageBlock::Subagent { record } => format!(
                "subagent: {} [{}] :: {}",
                record.name,
                record.status,
                tail_string(&record.output, 240)
            ),
            MessageBlock::Verification { report } => format!(
                "verification: {} :: {}",
                report.status,
                tail_string(&report.summary, 240)
            ),
        })
        .collect::<Vec<_>>()
        .join("\n");
    let prompt = match language {
        TurnLanguage::Zh => format!(
            "你是代码代理的审查/验证子代理。只返回 JSON。\n\n原始用户请求：\n{user_content}\n\n计划摘要：{summary}\n验证目标：{verification}\n\n已观察到的执行轨迹：\n{transcript}\n\n输出 schema：\n{{\"status\":\"pass|repair\",\"summary\":\"...\",\"issues\":[\"...\"],\"evidence\":[\"...\"],\"next_actions\":[\"...\"]}}\n\n规则：\n- 如果存在工具失败、缺少验证，或者任务明显未完成，返回 repair。\n- 只有当轨迹显示目标大概率已经满足时，才返回 pass。\n- 结论要简洁、具体。\n- 不要输出 markdown 代码块。",
            summary = plan.summary,
            verification = if plan.verification.is_empty() {
                "无".to_string()
            } else {
                plan.verification.join("; ")
            }
        ),
        TurnLanguage::En => format!(
            "You are the reviewer/verifier subagent for a coding agent.\nReturn only JSON.\n\nOriginal user request:\n{user_content}\n\nPlan summary: {summary}\nVerification targets: {verification}\n\nObserved execution trace:\n{transcript}\n\nOutput schema:\n{{\"status\":\"pass|repair\",\"summary\":\"...\",\"issues\":[\"...\"],\"evidence\":[\"...\"],\"next_actions\":[\"...\"]}}\n\nRules:\n- Return status=repair when there are tool failures, missing verification, or evidence that the task is incomplete.\n- Return status=pass only when the trace shows the goal is likely satisfied.\n- Be concise and concrete.\n- No markdown fences.",
            summary = plan.summary,
            verification = if plan.verification.is_empty() {
                "none".to_string()
            } else {
                plan.verification.join("; ")
            }
        ),
    };
    let content = provider_completion(
        provider,
        runtime,
        vec![
            Message::system(&localized_text(
                language,
                "你是严格的审查子代理。只输出 JSON。",
                "You are a strict reviewer subagent. Output JSON only.",
            )),
            Message::user(&prompt),
        ],
    )
    .await?;
    let mut report: AgentCritiqueReport = extract_json_value(&content)?;
    if report.status.trim().is_empty() {
        report.status = "pass".to_string();
    }
    Ok(report)
}

async fn run_reviewer_subagent(
    provider: Arc<dyn LLMProvider>,
    runtime: &RuntimeSettings,
    plan: &AgentWorkflowPlan,
    messages: &[MessageBlock],
    user_content: &str,
    language: TurnLanguage,
) -> Result<(AgentCritiqueReport, AgentSubagentRecord)> {
    let report = review_agent_progress(provider, runtime, plan, messages, user_content, language).await?;
    let record = AgentSubagentRecord {
        id: "reviewer".to_string(),
        name: "reviewer".to_string(),
        purpose: localized_text(
            language,
            "检查本轮结果是否完整，以及是否满足用户请求。",
            "Review the turn for completeness and whether the result satisfies the request.",
        ),
        input: tail_string(user_content.trim(), 280),
        output: report.summary.clone(),
        status: report.status.clone(),
        kind: "review".to_string(),
        started_at: Some(web_now_iso()),
        completed_at: Some(web_now_iso()),
        evidence: report.evidence.clone(),
    };
    Ok((report, record))
}

fn analysis_subagent_specs() -> Vec<AnalysisSubagentSpec> {
    vec![
        AnalysisSubagentSpec {
            id: "reviewer",
            name: "reviewer",
            purpose: "Review the turn for completeness and whether the result satisfies the request.",
            kind: "review",
            system_prompt: "You are a strict reviewer subagent. Output JSON only.",
            focus: "Check whether the user request is fully satisfied and whether the response is complete.",
            focus_zh: "检查用户请求是否已经被完整满足，以及结果是否足够完整。",
        },
        AnalysisSubagentSpec {
            id: "critic",
            name: "critic",
            purpose: "Search for hidden gaps, weak assumptions, and places where the result could still be wrong.",
            kind: "critique",
            system_prompt: "You are a strict critic subagent. Output JSON only.",
            focus: "Look for hidden risks, unverified assumptions, or evidence that a repair loop is still needed.",
            focus_zh: "寻找隐藏风险、未验证假设，或仍然需要修复回路的证据。",
        },
        AnalysisSubagentSpec {
            id: "researcher",
            name: "researcher",
            purpose: "Assess whether the execution produced enough evidence, artifacts, and follow-through for the current workflow.",
            kind: "research",
            system_prompt: "You are a strict research audit subagent. Output JSON only.",
            focus: "Check whether the workflow produced enough evidence, artifacts, and next-step readiness for implementation or research.",
            focus_zh: "检查当前工作流是否产出了足够的证据、产物，以及继续推进实施或研究所需的准备度。",
        },
    ]
}

fn build_subagent_prompt(
    spec: &AnalysisSubagentSpec,
    plan: &AgentWorkflowPlan,
    messages: &[MessageBlock],
    user_content: &str,
    language: TurnLanguage,
) -> String {
    let transcript = messages
        .iter()
        .rev()
        .take(18)
        .rev()
        .map(|block| match block {
            MessageBlock::User { content, .. } => format!("user: {}", content),
            MessageBlock::Assistant { content } => format!("assistant: {}", content),
            MessageBlock::ToolCall { name, status, .. } => {
                format!("tool_call: {} [{}]", name, tool_status_name(status))
            }
            MessageBlock::ToolResult { result, success, .. } => format!(
                "tool_result: {} :: {}",
                if *success { "success" } else { "failure" },
                tail_string(result, 400)
            ),
            MessageBlock::Diff { diff } => format!(
                "diff: {} (+{} -{})",
                diff.file_path, diff.added, diff.removed
            ),
            MessageBlock::Thinking { content, .. } => format!("thinking: {}", content),
            MessageBlock::System { content } => format!("system: {}", content),
            MessageBlock::Error { content } => format!("error: {}", content),
            MessageBlock::AssistantStreaming { content } => format!("assistant_stream: {}", content),
            MessageBlock::Subagent { record } => format!(
                "subagent: {} [{}] :: {}",
                record.name,
                record.status,
                tail_string(&record.output, 240)
            ),
            MessageBlock::Verification { report } => format!(
                "verification: {} :: {}",
                report.status,
                tail_string(&report.summary, 240)
            ),
        })
        .collect::<Vec<_>>()
        .join("\n");

    match language {
        TurnLanguage::Zh => format!(
            "你是代码与科研 IDE 代理中的 {name} 子代理。只返回 JSON。\n\n原始用户请求：\n{user_content}\n\n计划摘要：{summary}\n工作流类型：{workflow_kind}\n验证目标：{verification}\n\n你的关注点：\n{focus}\n\n已观察到的执行轨迹：\n{transcript}\n\n输出 schema：\n{{\"status\":\"pass|repair|complete|failed\",\"summary\":\"...\",\"issues\":[\"...\"],\"evidence\":[\"...\"],\"next_actions\":[\"...\"]}}\n\n规则：\n- 如果轨迹显示还需要继续工作或证据不足，返回 repair。\n- 只有当轨迹对你的关注点已经形成强支撑时，才返回 pass 或 complete。\n- 结论要具体、简洁。\n- 不要输出 markdown 代码块。",
            name = spec.name,
            user_content = user_content,
            summary = plan.summary,
            workflow_kind = plan.workflow_kind,
            verification = if plan.verification.is_empty() {
                "无".to_string()
            } else {
                plan.verification.join("; ")
            },
            focus = match language {
                TurnLanguage::Zh => spec.focus_zh,
                TurnLanguage::En => spec.focus,
            },
            transcript = transcript,
        ),
        TurnLanguage::En => format!(
            "You are the {name} subagent for a coding and research IDE agent.\nReturn only JSON.\n\nOriginal user request:\n{user_content}\n\nPlan summary: {summary}\nWorkflow kind: {workflow_kind}\nVerification targets: {verification}\n\nYour focus:\n{focus}\n\nObserved execution trace:\n{transcript}\n\nOutput schema:\n{{\"status\":\"pass|repair|complete|failed\",\"summary\":\"...\",\"issues\":[\"...\"],\"evidence\":[\"...\"],\"next_actions\":[\"...\"]}}\n\nRules:\n- Return repair when the trace suggests more work or stronger evidence is needed.\n- Return pass or complete only when the trace strongly supports success for your focus area.\n- Be concrete and concise.\n- No markdown fences.",
            name = spec.name,
            user_content = user_content,
            summary = plan.summary,
            workflow_kind = plan.workflow_kind,
            verification = if plan.verification.is_empty() {
                "none".to_string()
            } else {
                plan.verification.join("; ")
            },
            focus = match language {
                TurnLanguage::Zh => spec.focus_zh,
                TurnLanguage::En => spec.focus,
            },
            transcript = transcript,
        ),
    }
}

async fn run_specialist_subagent(
    provider: Arc<dyn LLMProvider>,
    runtime: &RuntimeSettings,
    spec: AnalysisSubagentSpec,
    plan: &AgentWorkflowPlan,
    messages: &[MessageBlock],
    user_content: &str,
    language: TurnLanguage,
) -> Result<(AgentCritiqueReport, AgentSubagentRecord)> {
    let prompt = build_subagent_prompt(&spec, plan, messages, user_content, language);
    let content = provider_completion(
        provider,
        runtime,
        vec![
            Message::system(&localized_text(language, "你是严格的子代理。只输出 JSON。", spec.system_prompt)),
            Message::user(&prompt),
        ],
    )
    .await?;
    let mut report: AgentCritiqueReport = extract_json_value(&content)?;
    if report.status.trim().is_empty() {
        report.status = "pass".to_string();
    }
    let record = AgentSubagentRecord {
        id: spec.id.to_string(),
        name: spec.name.to_string(),
        purpose: localized_string(
            language,
            match spec.id {
                "reviewer" => "检查本轮结果是否完整，以及是否满足用户请求。".to_string(),
                "critic" => "寻找隐藏风险、薄弱假设，以及结果仍可能出错的地方。".to_string(),
                "researcher" => "评估当前执行是否为本次工作流产出了足够的证据、产物和后续推进依据。".to_string(),
                _ => spec.purpose.to_string(),
            },
            spec.purpose.to_string(),
        ),
        input: tail_string(user_content.trim(), 280),
        output: report.summary.clone(),
        status: report.status.clone(),
        kind: spec.kind.to_string(),
        started_at: Some(web_now_iso()),
        completed_at: Some(web_now_iso()),
        evidence: report.evidence.clone(),
    };
    Ok((report, record))
}

fn merge_parallel_analysis(
    review_report: AgentCritiqueReport,
    review_record: AgentSubagentRecord,
    verifier_report: AgentVerifierReport,
    mut specialist_reports: Vec<(AgentCritiqueReport, AgentSubagentRecord)>,
    checkpoints: Vec<String>,
    branch_notes: Vec<String>,
    language: TurnLanguage,
) -> ParallelAnalysisResult {
    let mut subagent_records = vec![review_record];
    let mut issues = Vec::new();
    let mut next_actions = Vec::new();
    let mut evidence = Vec::new();

    issues.extend(review_report.issues.clone());
    issues.extend(verifier_report.issues.clone());
    next_actions.extend(review_report.next_actions.clone());
    next_actions.extend(verifier_report.next_actions.clone());
    evidence.extend(review_report.evidence.clone());
    evidence.extend(verifier_report.evidence.clone());

    for (report, record) in specialist_reports.drain(..) {
        issues.extend(report.issues.clone());
        next_actions.extend(report.next_actions.clone());
        evidence.extend(report.evidence.clone());
        subagent_records.push(record);
    }

    subagent_records.sort_by(|left, right| left.id.cmp(&right.id));

    let mut unique_issues = Vec::new();
    let mut seen_issues = BTreeSet::new();
    for issue in issues {
        let normalized = issue.trim().to_ascii_lowercase();
        if normalized.is_empty() || !seen_issues.insert(normalized) {
            continue;
        }
        unique_issues.push(issue);
    }

    let mut unique_actions = Vec::new();
    let mut seen_actions = BTreeSet::new();
    for action in next_actions {
        let normalized = action.trim().to_ascii_lowercase();
        if normalized.is_empty() || !seen_actions.insert(normalized) {
            continue;
        }
        unique_actions.push(action);
    }

    let mut unique_evidence = Vec::new();
    let mut seen_evidence = BTreeSet::new();
    for item in evidence {
        let normalized = item.trim().to_ascii_lowercase();
        if normalized.is_empty() || !seen_evidence.insert(normalized) {
            continue;
        }
        unique_evidence.push(item);
    }

    let verifier_soft_repair = verifier_report.status.eq_ignore_ascii_case("repair")
        && verifier_report_has_only_soft_evidence_gaps(&verifier_report);
    let needs_repair = review_report.status.eq_ignore_ascii_case("repair")
        || (verifier_report.status.eq_ignore_ascii_case("repair") && !verifier_soft_repair)
        || subagent_records
            .iter()
            .any(|record| record.status.eq_ignore_ascii_case("repair") || record.status.eq_ignore_ascii_case("failed"));

    let summary = if verifier_report.status.eq_ignore_ascii_case("repair") {
        verifier_report.summary.clone()
    } else if let Some(record) = subagent_records
        .iter()
        .find(|record| record.status.eq_ignore_ascii_case("repair"))
    {
        if record.output.trim().is_empty() {
            localized_string(
                language,
                format!("{} 请求修复。", record.name),
                format!("{} requested repair.", record.name),
            )
        } else {
            record.output.clone()
        }
    } else {
        review_report.summary.clone()
    };

    ParallelAnalysisResult {
        review_report,
        verifier_report,
        subagent_records,
        checkpoints,
        branch_notes,
        needs_repair,
        summary,
        issues: unique_issues,
        next_actions: unique_actions,
        evidence: unique_evidence,
        hard_failed: needs_repair,
    }
}

async fn run_parallel_analysis_subagents(
    provider: Arc<dyn LLMProvider>,
    runtime: &RuntimeSettings,
    plan: &AgentWorkflowPlan,
    messages: &[MessageBlock],
    user_content: &str,
    base_dir: &Path,
    required_path_snapshots: &[RequiredPathSnapshot],
    language: TurnLanguage,
    mut on_progress: impl FnMut(ParallelAnalysisProgress),
) -> Result<ParallelAnalysisResult> {
    let hard_report = build_hard_verifier_report(
        plan,
        messages,
        base_dir,
        &runtime.workspace_root,
        required_path_snapshots,
        language,
    );
    let verifier_record = AgentSubagentRecord {
        id: "verifier".to_string(),
        name: "verifier".to_string(),
        purpose: localized_text(
            language,
            "使用确定性的工具、运行时、diff 和执行证据验证本轮结果。",
            "Verify the turn using deterministic tool, runtime, diff, and execution evidence.",
        ),
        input: tail_string(user_content.trim(), 280),
        output: hard_report.0.summary.clone(),
        status: hard_report.0.status.clone(),
        kind: "verify".to_string(),
        started_at: Some(web_now_iso()),
        completed_at: Some(web_now_iso()),
        evidence: hard_report.0.evidence.clone(),
    };
    on_progress(ParallelAnalysisProgress {
        verifier_report: Some(hard_report.0.clone()),
        subagent_record: Some(verifier_record.clone()),
        checkpoints: hard_report.1.clone(),
        branch_notes: hard_report.2.clone(),
    });

    let specialist_specs = analysis_subagent_specs();
    let runtime_owned = runtime.clone();
    let plan_owned = plan.clone();
    let messages_owned = messages.to_vec();
    let user_content_owned = user_content.to_string();
    let mut join_set = JoinSet::new();

    {
        let provider = provider.clone();
        let runtime = runtime_owned.clone();
        let plan = plan_owned.clone();
        let messages = messages_owned.clone();
        let user_content = user_content_owned.clone();
        let language = language;
        join_set.spawn(async move {
            run_reviewer_subagent(provider, &runtime, &plan, &messages, &user_content, language).await
        });
    }

    for spec in specialist_specs.into_iter().filter(|spec| spec.id != "reviewer") {
        let provider = provider.clone();
        let runtime = runtime_owned.clone();
        let plan = plan_owned.clone();
        let messages = messages_owned.clone();
        let user_content = user_content_owned.clone();
        let language = language;
        join_set.spawn(async move {
            run_specialist_subagent(provider, &runtime, spec, &plan, &messages, &user_content, language).await
        });
    }

    let mut reviewer_result: Option<(AgentCritiqueReport, AgentSubagentRecord)> = None;
    let mut specialist_results = Vec::new();

    while let Some(joined) = join_set.join_next().await {
        let (report, record) = joined.map_err(|err| {
            anyhow!(
                "{}",
                localized_string(
                    language,
                    format!("分析子代理 join 失败：{}", err),
                    format!("analysis subagent join failed: {}", err),
                )
            )
        })??;
        on_progress(ParallelAnalysisProgress {
            verifier_report: None,
            subagent_record: Some(record.clone()),
            checkpoints: Vec::new(),
            branch_notes: Vec::new(),
        });
        if record.id.eq_ignore_ascii_case("reviewer") {
            reviewer_result = Some((report, record));
        } else {
            specialist_results.push((report, record));
        }
    }

    let reviewer_result = reviewer_result.ok_or_else(|| {
        anyhow!(
            "{}",
            localized_text(
                language,
                "审查子代理未完成执行",
                "reviewer subagent did not complete",
            )
        )
    })?;

    let mut merged = merge_parallel_analysis(
        reviewer_result.0,
        reviewer_result.1,
        hard_report.0,
        specialist_results,
        hard_report.1,
        hard_report.2,
        language,
    );
    merged.subagent_records.push(verifier_record);
    merged.subagent_records.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(merged)
}

async fn stream_provider_turn(
    provider: Arc<dyn LLMProvider>,
    request: ChatRequest,
    state: &WebAppState,
    session_id: &str,
    visible_history: &[WebMessage],
    visible_assistant_prefix: &str,
    has_workspace_edits: bool,
    mode: Option<&str>,
    language: TurnLanguage,
    tx: &tokio::sync::mpsc::UnboundedSender<StreamEnvelope>,
) -> Result<StreamTurnResult> {
    let mut stream = provider.chat_stream(request).await?;
    let mut raw_text = String::new();
    let mut text = String::new();
    let mut emitted_combined = visible_assistant_prefix.to_string();
    let mut finish_reason = None;
    let mut tool_calls = None;
    let mut pseudo_tool_names = Vec::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        if let Some(next_tool_calls) = chunk.tool_calls.clone() {
            tool_calls = Some(next_tool_calls);
        }
        if let Some(reason) = chunk.finish_reason.clone() {
            finish_reason = Some(reason);
        }

        if !chunk.content.is_empty() {
            raw_text = merge_stream_text(&raw_text, &chunk.content);
            let sanitized_text =
                stream_visible_workspace_text(&raw_text, has_workspace_edits, mode, language);
            if sanitized_text == text {
                continue;
            }
            text = sanitized_text;
            let current_text = strip_emoji(&text);
            if let Ok(mut sessions) = lock_stream_runtime(state) {
                if let Some(session) = sessions.get_mut(session_id) {
                    session.partial_text = combine_assistant_segments(visible_assistant_prefix, &current_text);
                }
            }
            let combined = combine_assistant_segments(visible_assistant_prefix, &current_text);
            let delta = if combined.starts_with(&emitted_combined) {
                combined[emitted_combined.len()..].to_string()
            } else {
                current_text.clone()
            };
            emitted_combined = combined.clone();
            if delta.is_empty() {
                continue;
            }
            let _ = tx.send(StreamEnvelope {
                r#type: "assistant_delta".to_string(),
                session_id: Some(session_id.to_string()),
                messages: None,
                delta: Some(delta),
                error: None,
                activity: None,
                tool: None,
                permission: None,
                edited_files: None,
                research: None,
                subagents: None,
                verifier: None,
            });
        }
    }

    if tool_calls.as_ref().is_none_or(|calls| calls.is_empty()) {
        let dsml_tool_calls = extract_dsml_tool_calls(&raw_text);
        if !dsml_tool_calls.is_empty() {
            tool_calls = Some(dsml_tool_calls);
            if !is_tool_call_finish(&finish_reason) {
                finish_reason = Some("tool_calls".to_string());
            }
        }
    }

    if tool_calls.as_ref().is_none_or(|calls| calls.is_empty()) {
        pseudo_tool_names = detect_plaintext_tool_narration(&raw_text);
    }

    Ok(StreamTurnResult {
        text,
        finish_reason,
        tool_calls,
        pseudo_tool_names,
    })
}

fn sanitize_visible_stream_text(input: &str) -> String {
    let trimmed = trim_from_dsml_start(input);
    if trimmed.is_empty() {
        return String::new();
    }
    let cleaned = strip_provider_tool_narration(trimmed);
    cleaned.trim().to_string()
}

fn stream_visible_workspace_text(
    input: &str,
    has_workspace_edits: bool,
    mode: Option<&str>,
    language: TurnLanguage,
) -> String {
    let cleaned = sanitize_visible_stream_text(input);
    if cleaned.is_empty() {
        return cleaned;
    }
    assistant_text_for_workspace_control_channel(&cleaned, has_workspace_edits, mode, language)
}

fn looks_like_large_inline_code(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.contains("```") && trimmed.len() > 320 {
        return true;
    }
    let codey_lines = trimmed
        .lines()
        .filter(|line| {
            let line = line.trim_start();
            line.starts_with("def ")
                || line.starts_with("class ")
                || line.starts_with("function ")
                || line.starts_with("import ")
                || line.starts_with("from ")
                || line.starts_with("const ")
                || line.starts_with("let ")
                || line.starts_with("var ")
                || line.starts_with("pub ")
                || line.starts_with("fn ")
                || line.starts_with("use ")
                || line.starts_with("#include ")
        })
        .count();
    codey_lines >= 6
}

fn workspace_write_notice(language: TurnLanguage) -> String {
    localized_text(
        language,
        "本轮改动已直接写入工作区文件，详细代码不在对话区展开。请查看下方文件变更、实验产物与验证结果。",
        "Changes were written directly into workspace files. See the edited files below for the exact paths, experiment artifacts, and verification results.",
    )
}

fn assistant_text_for_workspace_control_channel(
    text: &str,
    has_workspace_edits: bool,
    mode: Option<&str>,
    language: TurnLanguage,
) -> String {
    let cleaned = strip_emoji(text).trim().to_string();
    if cleaned.is_empty() {
        return cleaned;
    }
    let normalized_mode = workflow_mode(mode);
    if has_workspace_edits && normalized_mode != "chat" && looks_like_large_inline_code(&cleaned) {
        return workspace_write_notice(language);
    }
    cleaned
}

fn strip_provider_tool_narration(input: &str) -> String {
    let normalized = input.replace("\r\n", "\n");
    let markers = [
        "\nTool write_file ",
        "\nTool edit_file ",
        "\nTool read_file ",
        "\nTool list_dir ",
        "\nTool find_files ",
        "\nTool run_python",
        "\nTool run_command ",
        "\nTool run_safe_command ",
        "\nTool terminal_run ",
        "\nArguments:",
        "\nResult summary:",
        "\n{\"operation\":\"write_file\"",
        "\n{\"operation\":\"read_file\"",
        "\n{\"operation\":\"edit_file\"",
        "\n{\"operation\":\"run_python\"",
        "\n{\"operation\":\"run_command\"",
    ];
    let mut cut_at = normalized.len();
    for marker in markers {
        if let Some(index) = normalized.find(marker) {
            cut_at = cut_at.min(index);
        }
    }
    for marker in [
        "Tool write_file ",
        "Tool edit_file ",
        "Tool read_file ",
        "Tool list_dir ",
        "Tool find_files ",
        "Tool run_python",
        "Tool run_command ",
        "Tool run_safe_command ",
        "Tool terminal_run ",
        "Arguments:",
        "Result summary:",
        "{\"operation\":\"write_file\"",
        "{\"operation\":\"read_file\"",
        "{\"operation\":\"edit_file\"",
        "{\"operation\":\"run_python\"",
        "{\"operation\":\"run_command\"",
    ] {
        if normalized.starts_with(marker) {
            cut_at = 0;
            break;
        }
    }
    normalized[..cut_at].to_string()
}

fn summarize_workspace_turn_for_chat(
    text: &str,
    recent_blocks: &[MessageBlock],
    mode: Option<&str>,
    language: TurnLanguage,
) -> Option<String> {
    let normalized_mode = workflow_mode(mode);
    if normalized_mode == "chat" {
        return None;
    }

    let diffs = recent_blocks
        .iter()
        .filter_map(|block| match block {
            MessageBlock::Diff { diff } => Some(diff),
            _ => None,
        })
        .collect::<Vec<_>>();
    if diffs.is_empty() {
        return None;
    }

    let outputs = recent_blocks
        .iter()
        .rev()
        .filter_map(|block| match block {
            MessageBlock::ToolResult { result, success, .. } if *success => {
                summarize_tool_result_for_chat(result)
            }
            _ => None,
        })
        .filter(|line| {
            let lowered = line.to_ascii_lowercase();
            !lowered.contains("??????")
                && !lowered.contains("written")
                && !lowered.contains(r#""operation":"write_file""#)
                && !lowered.contains(r#""status":"success""#)
                && !lowered.contains("read back content:")
                && !lowered.contains("read_file")
        })
        .take(2)
        .collect::<Vec<_>>();
    let output_line = outputs
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join(" | ");

    let verifier_summary = recent_blocks
        .iter()
        .rev()
        .find_map(|block| match block {
            MessageBlock::Verification { report } => {
                let summary = tail_string(report.summary.trim(), 180);
                if summary.is_empty() {
                    None
                } else {
                    Some((report.status.clone(), summary))
                }
            }
            _ => None,
        });

    let file_list = diffs
        .iter()
        .rev()
        .take(3)
        .map(|diff| format!("{} (+{} / -{})", diff.file_path, diff.added, diff.removed))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("; ");

    let original = text.trim();
    let looks_like_code = looks_like_large_inline_code(original);
    let write_notice = workspace_write_notice(language);
    let should_synthesize_summary = original.is_empty()
        || looks_like_code
        || original == write_notice.trim();
    if !should_synthesize_summary {
        return None;
    }
    let mut parts = Vec::new();
    if !original.is_empty() && !looks_like_code {
        parts.push(original.to_string());
    }

    if matches!(language, TurnLanguage::Zh) {
        parts.push(format!("工作区文件已更新：{}。", file_list));
        if !output_line.is_empty() {
            parts.push(format!("运行或实验输出：{}。", output_line));
        }
        if let Some((status, summary)) = verifier_summary {
            parts.push(format!(
                "验证（{}）：{}。",
                localized_status_text(language, &status),
                summary
            ));
        }
    } else {
        parts.push(format!("Workspace files updated: {}.", file_list));
        if !output_line.is_empty() {
            parts.push(format!("Run or experiment outputs: {}.", output_line));
        }
        if let Some((status, summary)) = verifier_summary {
            parts.push(format!(
                "Verification ({}) : {}.",
                localized_status_text(language, &status),
                summary
            ));
        }
    }

    let summary = parts
        .into_iter()
        .filter(|item| !item.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
        .trim()
        .to_string();
    if summary.is_empty() { None } else { Some(summary) }
}

fn final_turn_has_meaningful_assistant_text(
    recent_blocks: &[MessageBlock],
    language: TurnLanguage,
) -> bool {
    let write_notice = workspace_write_notice(language);
    recent_blocks.iter().any(|block| match block {
        MessageBlock::Assistant { content } | MessageBlock::AssistantStreaming { content } => {
            let cleaned = sanitize_visible_stream_text(content);
            let trimmed = cleaned.trim();
            !trimmed.is_empty() && trimmed != write_notice.trim()
        }
        _ => false,
    })
}

fn final_turn_should_collapse_assistant_history(
    recent_blocks: &[MessageBlock],
    language: TurnLanguage,
) -> bool {
    let verifier_passed = recent_blocks.iter().rev().any(|block| match block {
        MessageBlock::Verification { report } => {
            report.status.eq_ignore_ascii_case("pass")
                || report.status.eq_ignore_ascii_case("complete")
        }
        _ => false,
    });
    if !verifier_passed {
        return false;
    }

    let write_notice = workspace_write_notice(language);
    let assistant_texts = recent_blocks
        .iter()
        .filter_map(|block| match block {
            MessageBlock::Assistant { content } | MessageBlock::AssistantStreaming { content } => {
                let cleaned = sanitize_visible_stream_text(content);
                let trimmed = cleaned.trim();
                if trimmed.is_empty() || trimmed == write_notice.trim() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if assistant_texts.len() < 2 {
        return false;
    }

    let combined = assistant_texts.join("\n\n");
    let combined_lower = combined.to_ascii_lowercase();
    let noisy_marker = matches!(language, TurnLanguage::Zh)
        && ["让我", "现在让我", "问题找到了", "有错误", "修复", "读取当前脚本", "重新编写这个脚本"]
            .iter()
            .any(|needle| combined.contains(needle))
        || ["let me", "i need to", "there was an error", "repair", "rewrite this script"]
            .iter()
            .any(|needle| combined_lower.contains(needle));

    noisy_marker || assistant_texts.len() >= 4 || combined.chars().count() >= 700
}

fn ensure_final_turn_assistant_summary(
    messages: &[MessageBlock],
    mode: Option<&str>,
    language: TurnLanguage,
) -> Vec<MessageBlock> {
    let mut merged = messages.to_vec();
    let turn_start = current_turn_start_index(&merged);
    let turn_blocks = &merged[turn_start..];
    if final_turn_should_collapse_assistant_history(turn_blocks, language) {
        let mut collapsed = merged[..turn_start].to_vec();
        collapsed.extend(
            turn_blocks
                .iter()
                .filter(|block| {
                    !matches!(
                        block,
                        MessageBlock::Assistant { .. } | MessageBlock::AssistantStreaming { .. }
                    )
                })
                .cloned(),
        );
        if let Some(summary) = summarize_workspace_turn_for_chat("", turn_blocks, mode, language) {
            collapsed.push(MessageBlock::Assistant { content: summary });
        }
        return collapsed;
    }
    if final_turn_has_meaningful_assistant_text(turn_blocks, language) {
        return merged;
    }

    if let Some(summary) = summarize_workspace_turn_for_chat("", turn_blocks, mode, language) {
        merged.push(MessageBlock::Assistant { content: summary });
    }

    merged
}

fn summarize_tool_result_for_chat(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        if let Some(operation) = value.get("operation").and_then(Value::as_str) {
            if operation == "write_file" || operation == "read_file" {
                return None;
            }
        }
        if let Some(content) = value
            .pointer("/data/content")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            if content.len() <= 120 && !looks_like_large_inline_code(content) {
                return Some(format!("content read: {}", tail_string(content, 120)));
            }
            return None;
        }
        if let Some(message) = value
            .get("message")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(tail_string(message, 140));
        }
        if let Some(output) = value
            .get("output")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(tail_string(output, 140));
        }
    }

    Some(tail_string(trimmed, 140))
}

fn trim_from_dsml_start(input: &str) -> &str {
    if let Some(start) = find_dsml_like_start(input) {
        return &input[..start];
    }
    input
}

fn find_dsml_like_start(input: &str) -> Option<usize> {
    const FULL_MARKERS: &[&str] = &[
        "<\u{FF5C}\u{FF5C}DSML\u{FF5C}\u{FF5C}",
        "</\u{FF5C}\u{FF5C}DSML\u{FF5C}\u{FF5C}",
        "\u{FF5C}\u{FF5C}DSML\u{FF5C}\u{FF5C}",
        "<DSML",
        "</DSML",
    ];
    for marker in FULL_MARKERS {
        if let Some(index) = input.find(marker) {
            return Some(index);
        }
    }

    const PARTIAL_MARKERS: &[&str] = &[
        "<\u{FF5C}",
        "<\u{FF5C}\u{FF5C}",
        "<\u{FF5C}\u{FF5C}D",
        "<\u{FF5C}\u{FF5C}DS",
        "<\u{FF5C}\u{FF5C}DSM",
        "<\u{FF5C}\u{FF5C}DSML",
        "<\u{FF5C}\u{FF5C}DSML\u{FF5C}",
        "</\u{FF5C}",
        "</\u{FF5C}\u{FF5C}",
        "</\u{FF5C}\u{FF5C}D",
        "</\u{FF5C}\u{FF5C}DS",
        "</\u{FF5C}\u{FF5C}DSM",
        "</\u{FF5C}\u{FF5C}DSML",
        "</\u{FF5C}\u{FF5C}DSML\u{FF5C}",
    ];
    for marker in PARTIAL_MARKERS {
        if input.ends_with(marker) {
            return Some(input.len().saturating_sub(marker.len()));
        }
    }

    if input.ends_with('<') || input.ends_with("</") {
        return Some(input.len().saturating_sub(1));
    }

    None
}

fn sanitize_message_block_for_stream_provider(block: MessageBlock) -> Option<MessageBlock> {
    match block {
        MessageBlock::Assistant { content } => {
            let cleaned = sanitize_visible_stream_text(&content);
            if cleaned.is_empty() {
                None
            } else {
                Some(MessageBlock::Assistant { content: cleaned })
            }
        }
        MessageBlock::AssistantStreaming { content } => {
            let cleaned = sanitize_visible_stream_text(&content);
            if cleaned.is_empty() {
                None
            } else {
                Some(MessageBlock::AssistantStreaming { content: cleaned })
            }
        }
        MessageBlock::Thinking { content, collapsed } => {
            let cleaned = sanitize_visible_stream_text(&content);
            if cleaned.is_empty() {
                None
            } else {
                Some(MessageBlock::Thinking {
                    content: cleaned,
                    collapsed,
                })
            }
        }
        MessageBlock::System { content } => {
            let cleaned = sanitize_visible_stream_text(&content);
            if cleaned.is_empty() {
                None
            } else {
                Some(MessageBlock::System { content: cleaned })
            }
        }
        MessageBlock::Error { content } => {
            let cleaned = sanitize_visible_stream_text(&content);
            if cleaned.is_empty() {
                None
            } else {
                Some(MessageBlock::Error { content: cleaned })
            }
        }
        MessageBlock::Subagent { mut record } => {
            record.input = sanitize_visible_stream_text(&record.input);
            record.output = sanitize_visible_stream_text(&record.output);
            record.evidence = record
                .evidence
                .into_iter()
                .map(|item| sanitize_visible_stream_text(&item))
                .filter(|item| !item.is_empty())
                .collect();
            Some(MessageBlock::Subagent { record })
        }
        MessageBlock::Verification { mut report } => {
            report.summary = sanitize_visible_stream_text(&report.summary);
            report.issues = report
                .issues
                .into_iter()
                .map(|item| sanitize_visible_stream_text(&item))
                .filter(|item| !item.is_empty())
                .collect();
            report.next_actions = report
                .next_actions
                .into_iter()
                .map(|item| sanitize_visible_stream_text(&item))
                .filter(|item| !item.is_empty())
                .collect();
            report.evidence = report
                .evidence
                .into_iter()
                .map(|item| sanitize_visible_stream_text(&item))
                .filter(|item| !item.is_empty())
                .collect();
            Some(MessageBlock::Verification { report })
        }
        other => Some(other),
    }
}

fn extract_dsml_tool_calls(input: &str) -> Vec<Value> {
    static DSML_TOOL_BLOCK_RE: OnceLock<Regex> = OnceLock::new();
    static DSML_INVOKE_RE: OnceLock<Regex> = OnceLock::new();
    static DSML_PARAMETER_RE: OnceLock<Regex> = OnceLock::new();

    let block_re = DSML_TOOL_BLOCK_RE.get_or_init(|| {
        Regex::new(r#"(?s)<[^>]*DSML[^>]*tool_calls[^>]*>(.*?)</[^>]*DSML[^>]*tool_calls>"#)
            .expect("valid DSML tool call regex")
    });
    let invoke_re = DSML_INVOKE_RE.get_or_init(|| {
        Regex::new(r#"(?s)<[^>]*DSML[^>]*invoke\b([^>]*)>(.*?)</[^>]*DSML[^>]*invoke>"#)
            .expect("valid DSML invoke regex")
    });
    let parameter_re = DSML_PARAMETER_RE.get_or_init(|| {
        Regex::new(r#"(?s)<[^>]*DSML[^>]*parameter\b([^>]*)>(.*?)</[^>]*DSML[^>]*parameter>"#)
            .expect("valid DSML parameter regex")
    });

    let mut tool_calls = Vec::new();

    for block_caps in block_re.captures_iter(input) {
        let Some(block_body) = block_caps.get(1).map(|value| value.as_str()) else {
            continue;
        };

        for (index, invoke_caps) in invoke_re.captures_iter(block_body).enumerate() {
            let invoke_attrs = parse_dsml_attributes(
                invoke_caps.get(1).map(|value| value.as_str()).unwrap_or_default(),
            );
            let invoke_body = invoke_caps.get(2).map(|value| value.as_str()).unwrap_or_default();
            let invoke_name = invoke_attrs
                .get("name")
                .map(String::as_str)
                .unwrap_or("unknown");

            let mut args = serde_json::Map::new();
            for parameter_caps in parameter_re.captures_iter(invoke_body) {
                let parameter_attrs = parse_dsml_attributes(
                    parameter_caps
                        .get(1)
                        .map(|value| value.as_str())
                        .unwrap_or_default(),
                );
                let Some(parameter_name) = parameter_attrs.get("name").cloned() else {
                    continue;
                };
                let raw_value = parameter_caps
                    .get(2)
                    .map(|value| value.as_str())
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                args.insert(
                    parameter_name,
                    parse_dsml_parameter_value(&parameter_attrs, &raw_value),
                );
            }

            if let Some(tool_call) = normalize_dsml_tool_call(
                invoke_name,
                Value::Object(args),
                invoke_attrs.get("id").cloned(),
                index,
            ) {
                tool_calls.push(tool_call);
            }
        }
    }

    tool_calls
}

fn parse_dsml_attributes(raw: &str) -> BTreeMap<String, String> {
    static DSML_ATTR_RE: OnceLock<Regex> = OnceLock::new();
    let attr_re = DSML_ATTR_RE.get_or_init(|| {
        Regex::new(r#"([A-Za-z_][A-Za-z0-9_-]*)="([^"]*)""#)
            .expect("valid DSML attribute regex")
    });

    let mut attributes = BTreeMap::new();
    for captures in attr_re.captures_iter(raw) {
        let Some(key) = captures.get(1).map(|value| value.as_str().to_string()) else {
            continue;
        };
        let Some(value) = captures.get(2).map(|value| value.as_str().to_string()) else {
            continue;
        };
        attributes.insert(key, value);
    }
    attributes
}

fn parse_dsml_parameter_value(
    attrs: &BTreeMap<String, String>,
    raw_value: &str,
) -> Value {
    let raw = raw_value.trim();
    let as_string = attrs
        .get("string")
        .is_some_and(|value| value.eq_ignore_ascii_case("true"));

    if as_string {
        return Value::String(raw.to_string());
    }

    serde_json::from_str::<Value>(raw).unwrap_or_else(|_| Value::String(raw.to_string()))
}

fn normalize_dsml_tool_call(
    invoke_name: &str,
    args: Value,
    call_id: Option<String>,
    index: usize,
) -> Option<Value> {
    let normalized_name = invoke_name.trim();
    if normalized_name.is_empty() {
        return None;
    }

    let mapped = if normalized_name.eq_ignore_ascii_case("bash")
        || normalized_name.eq_ignore_ascii_case("shell")
    {
        let command = args
            .get("command")
            .and_then(|value| value.as_str())
            .map(adapt_bash_command_for_powershell)
            .unwrap_or_default();
        let wait_ms = args
            .get("timeout")
            .and_then(|value| {
                value
                    .as_u64()
                    .or_else(|| value.as_i64().and_then(|inner| u64::try_from(inner).ok()))
            })
            .unwrap_or(1_200)
            .min(5_000);
        json!({
            "name": "terminal_run",
            "args": {
                "command": command,
                "wait_ms": wait_ms
            }
        })
    } else {
        let normalized_args = normalize_dsml_arguments(normalized_name, args);
        json!({
            "name": normalized_name,
            "args": normalized_args
        })
    };

    let tool_name = mapped.get("name").and_then(Value::as_str)?.to_string();
    let tool_args = mapped.get("args").cloned().unwrap_or_else(|| json!({}));
    Some(json!({
        "id": call_id.unwrap_or_else(|| format!("dsml-{}", index + 1)),
        "type": "function",
        "function": {
            "name": tool_name,
            "arguments": serde_json::to_string(&tool_args).unwrap_or_else(|_| "{}".to_string())
        }
    }))
}

fn normalize_dsml_arguments(tool_name: &str, args: Value) -> Value {
    let mut object = match args {
        Value::Object(map) => map,
        other => return other,
    };

    if matches!(tool_name, "list_dir" | "mkdir" | "create_dir") {
        if object.get("dir").is_none() {
            if let Some(path) = object.get("path").cloned() {
                object.insert("dir".to_string(), path);
            }
        }
    }

    if matches!(tool_name, "write_file" | "read_file" | "edit_file" | "delete_file") {
        if object.get("path").is_none() {
            if let Some(dir) = object.get("dir").cloned() {
                object.insert("path".to_string(), dir);
            }
        }
    }

    Value::Object(object)
}

fn adapt_bash_command_for_powershell(command: &str) -> String {
    let trimmed = command.trim();
    let normalized = normalize_dsml_command_for_windows(trimmed);
    let normalized = normalized.replace("&&", ";");
    let normalized = adapt_powershell_dir_globs(&normalized);

    let adapt_cd_prefix = |raw: &str, prefix: &str| -> Option<String> {
        let remainder = raw.strip_prefix(prefix)?.trim();
        if remainder.is_empty() {
            return None;
        }
        if let Some((path, tail)) = remainder.split_once(';') {
            let path = normalize_powershell_path_arg(path);
            let tail = tail.trim();
            if tail.is_empty() {
                return Some(format!(
                    "Set-Location -LiteralPath {}",
                    powershell_single_quote(&path)
                ));
            }
            return Some(format!(
                "Set-Location -LiteralPath {}; {}",
                powershell_single_quote(&path),
                adapt_bash_command_for_powershell(tail)
            ));
        }
        if let Some((path, tail)) = remainder.split_once('\n') {
            let path = normalize_powershell_path_arg(path);
            let tail = tail.trim();
            if tail.is_empty() {
                return Some(format!(
                    "Set-Location -LiteralPath {}",
                    powershell_single_quote(&path)
                ));
            }
            return Some(format!(
                "Set-Location -LiteralPath {};\n{}",
                powershell_single_quote(&path),
                adapt_bash_command_for_powershell(tail)
            ));
        }
        let path = normalize_powershell_path_arg(remainder);
        Some(format!(
            "Set-Location -LiteralPath {}",
            powershell_single_quote(&path)
        ))
    };

    if let Some(adapted) = adapt_cd_prefix(&normalized, "cd /d ") {
        return adapted;
    }
    if let Some(adapted) = adapt_cd_prefix(&normalized, "cd ") {
        return adapted;
    }

    let adapt_mkdir_prefix = |raw: &str, prefix: &str| -> Option<String> {
        let remainder = raw.strip_prefix(prefix)?.trim();
        if remainder.is_empty() {
            return None;
        }
        let emit = |path: &str| {
            format!(
                "New-Item -ItemType Directory -Force -Path {} | Out-Null",
                powershell_single_quote(&normalize_powershell_path_arg(path))
            )
        };
        if let Some((path, tail)) = remainder.split_once(';') {
            let tail = tail.trim();
            if tail.is_empty() {
                return Some(emit(path));
            }
            return Some(format!(
                "{}; {}",
                emit(path),
                adapt_bash_command_for_powershell(tail)
            ));
        }
        if let Some((path, tail)) = remainder.split_once('\n') {
            let tail = tail.trim();
            if tail.is_empty() {
                return Some(emit(path));
            }
            return Some(format!(
                "{};\n{}",
                emit(path),
                adapt_bash_command_for_powershell(tail)
            ));
        }
        Some(emit(remainder))
    };

    if let Some(adapted) = adapt_mkdir_prefix(&normalized, "mkdir -p ") {
        return adapted;
    }
    if let Some(adapted) = adapt_mkdir_prefix(&normalized, "mkdir ") {
        return adapted;
    }
    if normalized == "pwd" {
        return "Get-Location".to_string();
    }
    if normalized == "ls" {
        return "Get-ChildItem -Force".to_string();
    }
    if let Some(path) = normalized.strip_prefix("ls ") {
        let path = normalize_powershell_path_arg(path);
        return format!("Get-ChildItem -Force {}", powershell_single_quote(&path));
    }
    if let Some(path) = normalized.strip_prefix("cat ") {
        let path = normalize_powershell_path_arg(path);
        return format!("Get-Content {}", powershell_single_quote(&path));
    }
    if let Some(path) = normalized.strip_prefix("touch ") {
        let path = normalize_powershell_path_arg(path);
        return format!(
            "New-Item -ItemType File -Force -Path {} | Out-Null",
            powershell_single_quote(&path)
        );
    }
    normalized
}

fn normalize_powershell_path_arg(raw: &str) -> String {
    let mut value = raw.trim().replace("\\\"", "\"").replace("\\'", "'");
    loop {
        let trimmed = value.trim();
        let next = if trimmed.len() >= 2
            && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
                || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
        {
            trimmed[1..trimmed.len().saturating_sub(1)].to_string()
        } else {
            break;
        };
        value = next;
    }
    value.trim().to_string()
}

fn adapt_powershell_dir_globs(command: &str) -> String {
    let trimmed = command.trim();
    let Some(rest) = trimmed.strip_prefix("dir ") else {
        return trimmed.to_string();
    };

    let tokens: Vec<&str> = rest.split_whitespace().collect();
    if tokens.len() < 2 || !tokens.iter().all(|token| token.contains('*') || token.contains('?')) {
        return trimmed.to_string();
    }

    let includes = tokens
        .iter()
        .map(|token| powershell_single_quote(token))
        .collect::<Vec<_>>()
        .join(", ");
    format!("Get-ChildItem -Force -Include {}", includes)
}

fn normalize_dsml_command_for_windows(command: &str) -> String {
    let mut normalized = command.trim().replace("/workspace/", "./");
    if normalized == "/workspace" {
        normalized = ".".to_string();
    } else if normalized.starts_with("cd /workspace && ") {
        normalized = normalized.replacen("cd /workspace && ", "", 1);
    } else if normalized.starts_with("cd /workspace; ") {
        normalized = normalized.replacen("cd /workspace; ", "", 1);
    } else if normalized.starts_with("cd /workspace\n") {
        normalized = normalized.replacen("cd /workspace\n", "", 1);
    }

    normalized = normalized.replace("python3 ", "python ");
    normalized = normalized.replace("python3.exe ", "python ");
    normalized = normalized.replace("pip3 ", "pip ");
    normalized = normalized.replace("bash -lc ", "");
    normalized
}

fn powershell_single_quote(input: &str) -> String {
    format!("'{}'", input.replace('\'', "''"))
}

async fn assistant_tool_definitions(
    state: &WebAppState,
    runtime: &RuntimeSettings,
) -> Result<Vec<Value>> {
    let assistant_state = state.assistant.clone();
    let assistant_api_url = if runtime.api_url.trim().is_empty() {
        state.assistant_api_url.clone()
    } else {
        runtime.api_url.clone()
    };
    let assistant_api_key = runtime
        .api_key
        .clone()
        .or_else(|| state.assistant_api_key.clone());
    let runtime_for_task = runtime.clone();
    let base_security = state.base_security_config.clone();
    let host_base_dir = state.host.base_dir().to_path_buf();

    tokio::task::spawn_blocking(move || -> Result<Vec<Value>> {
        let _cwd_guard = enter_workspace_dir_from(&host_base_dir, &runtime_for_task.workspace_root)?;
        let mut assistant_slot = lock_assistant_mutex(&assistant_state)?;

        if assistant_slot.is_none() {
            let assistant_config = AssistantConfig::new_with_runtime(
                assistant_api_url,
                assistant_api_key,
                runtime_for_task.model.clone(),
                effort_temperature(&runtime_for_task),
                effort_max_tokens(&runtime_for_task),
            );
            let security_config = runtime_security_config(&base_security, &runtime_for_task);
            *assistant_slot = Some(CliAssistant::new(assistant_config, security_config)?);
        }

        let mut tools = assistant_slot
            .as_ref()
            .map(|assistant| assistant.get_tool_definitions())
            .ok_or_else(|| anyhow!("assistant initialization failed"))?;
        tools.extend(web_terminal_tool_definitions());
        Ok(tools)
    })
    .await
    .map_err(|err| anyhow!("assistant tool definition task failed: {}", err))?
}

fn web_terminal_tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "type": "function",
            "function": {
                "name": "terminal_create",
                "description": "Create a persistent workspace terminal session. Use this before running interactive or multi-step shell commands.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "workspace_root": {
                            "type": "string",
                            "description": "Optional workspace root. Defaults to the current configured workspace."
                        }
                    },
                    "additionalProperties": false
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "terminal_run",
                "description": "Send a command to a persistent terminal and return the current terminal output buffer. Creates a terminal when terminal_id is omitted.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "terminal_id": {
                            "type": "string",
                            "description": "Existing terminal id. Omit to use the active terminal or create one."
                        },
                        "command": {
                            "type": "string",
                            "description": "Command to type into the terminal."
                        },
                        "wait_ms": {
                            "type": "integer",
                            "description": "Milliseconds to wait before reading the buffer. Defaults to 600, max 5000."
                        }
                    },
                    "required": ["command"],
                    "additionalProperties": false
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "terminal_read",
                "description": "Read the current output buffer from a persistent terminal session.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "terminal_id": {
                            "type": "string",
                            "description": "Existing terminal id. Omit to read the active terminal."
                        },
                        "tail": {
                            "type": "integer",
                            "description": "Maximum number of trailing characters to return. Defaults to 12000."
                        }
                    },
                    "additionalProperties": false
                }
            }
        }),
    ]
}

async fn execute_tool_calls(
    state: &WebAppState,
    runtime: &RuntimeSettings,
    session_id: &str,
    tool_calls: &[Value],
    persisted_blocks: &mut Vec<MessageBlock>,
    tx: &tokio::sync::mpsc::UnboundedSender<StreamEnvelope>,
    mode: Option<&str>,
    language: TurnLanguage,
    abort_after_first_workspace_edit: bool,
) -> Result<()> {
    for tool_call in tool_calls {
        let call_id = tool_call
            .get("id")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        let name = tool_call
            .get("function")
            .and_then(|function| function.get("name"))
            .and_then(|value| value.as_str())
            .unwrap_or("unknown")
            .to_string();
        let args = tool_call
            .get("function")
            .and_then(|function| function.get("arguments"))
            .and_then(|value| value.as_str())
            .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
            .unwrap_or_else(|| json!({}));
        let args = bind_tool_args_to_workspace(
            runtime,
            &normalize_web_tool_args(&name, args),
        );
        let tool_target_path = extract_tool_path(&name, &args);
        let risk = default_tool_risk_map()
            .get(&name)
            .cloned()
            .unwrap_or(RiskLevel::Moderate);
        let risk_name = risk_level_name(&risk).to_string();
        let pending_file_snapshot = capture_pending_file_snapshot(state.host.base_dir(), runtime, &name, &args);

        upsert_tool_call_block(
            persisted_blocks,
            &call_id,
            &name,
            &args,
            ToolCallStatus::Pending,
        );
        sync_stream_runtime_messages(state, session_id, persisted_blocks);

        let _ = tx.send(StreamEnvelope {
            r#type: "tool".to_string(),
            session_id: Some(session_id.to_string()),
            messages: None,
            delta: None,
            error: None,
            activity: Some(activity_event(
                "tool_pending",
                Some(localized_string(
                    language,
                    format!("正在准备 {}", name),
                    format!("Preparing {}", name),
                )),
            )),
            tool: Some(WebToolEvent {
                call_id: call_id.clone(),
                name: name.clone(),
                status: "pending".to_string(),
                risk: risk_name.clone(),
                args: Some(args.clone()),
                result: None,
                success: None,
                file_path: extract_tool_path(&name, &args),
            }),
            permission: None,
            edited_files: None,
            research: current_research_payload(state, Some(session_id), runtime, persisted_blocks, mode),
            subagents: None,
            verifier: None,
        });
        push_runtime_tool_event(
            state,
            session_id,
            &WebToolEvent {
                call_id: call_id.clone(),
                name: name.clone(),
                status: "pending".to_string(),
                risk: risk_name.clone(),
                args: Some(args.clone()),
                result: None,
                success: None,
                file_path: extract_tool_path(&name, &args),
            },
        );
        if let Some(summary) = tool_progress_narration(
            language,
            &name,
            "pending",
            tool_target_path.as_deref(),
            None,
        ) {
            emit_assistant_progress_delta(tx, state, session_id, summary.dedupe_key, summary.text);
        }

        if !tool_call_is_allowed(state, runtime, &name, &risk) {
            let approved = wait_for_tool_approval(state, session_id, &call_id, &name, &risk_name, &args, tx).await?;
            if !approved {
                persisted_blocks.push(MessageBlock::ToolResult {
                    call_id: call_id.clone(),
                    result: "Tool call denied by user approval gate.".to_string(),
                    success: false,
                });
                sync_stream_runtime_messages(state, session_id, persisted_blocks);
                let _ = tx.send(StreamEnvelope {
                    r#type: "tool".to_string(),
                    session_id: Some(session_id.to_string()),
                    messages: None,
                    delta: None,
                    error: None,
                    activity: Some(activity_event(
                        "tool_denied",
                        Some(localized_string(
                            language,
                            format!("已拒绝 {}", name),
                            format!("Denied {}", name),
                        )),
                    )),
                    tool: Some(WebToolEvent {
                        call_id: call_id.clone(),
                        name: name.clone(),
                        status: "denied".to_string(),
                        risk: risk_name.clone(),
                        args: Some(args.clone()),
                        result: Some("Denied by user".to_string()),
                        success: Some(false),
                        file_path: extract_tool_path(&name, &args),
                    }),
                    permission: None,
                    edited_files: None,
                    research: current_research_payload(state, Some(session_id), runtime, persisted_blocks, mode),
                    subagents: None,
                    verifier: None,
                });
                push_runtime_tool_event(
                    state,
                    session_id,
                    &WebToolEvent {
                        call_id: call_id.clone(),
                        name: name.clone(),
                        status: "denied".to_string(),
                        risk: risk_name.clone(),
                        args: Some(args.clone()),
                        result: Some("Denied by user".to_string()),
                        success: Some(false),
                        file_path: extract_tool_path(&name, &args),
                    },
                );
                continue;
            }
        }

        if let Err(rate_err) = runtime_security_config(&state.base_security_config, runtime)
            .rate_limiter
            .check(&name)
        {
            persisted_blocks.push(MessageBlock::ToolResult {
                call_id: call_id.clone(),
                result: rate_err.clone(),
                success: false,
            });
            sync_stream_runtime_messages(state, session_id, persisted_blocks);
            let _ = tx.send(StreamEnvelope {
                r#type: "tool".to_string(),
                session_id: Some(session_id.to_string()),
                messages: None,
                delta: None,
                error: None,
                activity: Some(activity_event(
                    "tool_rate_limited",
                    Some(localized_string(
                        language,
                        format!("{} 已触发速率限制", name),
                        format!("Rate limited {}", name),
                    )),
                )),
                tool: Some(WebToolEvent {
                    call_id: call_id.clone(),
                    name: name.clone(),
                    status: "failed".to_string(),
                    risk: risk_name.clone(),
                    args: Some(args.clone()),
                    result: Some(rate_err.clone()),
                    success: Some(false),
                    file_path: extract_tool_path(&name, &args),
                }),
                permission: None,
                edited_files: None,
                research: current_research_payload(state, Some(session_id), runtime, persisted_blocks, mode),
                subagents: None,
                verifier: None,
            });
            push_runtime_tool_event(
                state,
                session_id,
                &WebToolEvent {
                    call_id: call_id.clone(),
                    name: name.clone(),
                    status: "failed".to_string(),
                    risk: risk_name.clone(),
                    args: Some(args.clone()),
                    result: Some(rate_err.clone()),
                    success: Some(false),
                    file_path: extract_tool_path(&name, &args),
                },
            );
            continue;
        }

        upsert_tool_call_block(
            persisted_blocks,
            &call_id,
            &name,
            &args,
            ToolCallStatus::Executing,
        );
        sync_stream_runtime_messages(state, session_id, persisted_blocks);

        let _ = tx.send(StreamEnvelope {
            r#type: "tool".to_string(),
            session_id: Some(session_id.to_string()),
            messages: None,
            delta: None,
            error: None,
            activity: Some(activity_event(
                "tool_executing",
                Some(localized_string(
                    language,
                    format!("正在运行 {}", name),
                    format!("Running {}", name),
                )),
            )),
            tool: Some(WebToolEvent {
                call_id: call_id.clone(),
                name: name.clone(),
                status: "executing".to_string(),
                risk: risk_name.clone(),
                args: Some(args.clone()),
                result: None,
                success: None,
                file_path: extract_tool_path(&name, &args),
            }),
            permission: None,
            edited_files: None,
            research: current_research_payload(state, Some(session_id), runtime, persisted_blocks, mode),
            subagents: None,
            verifier: None,
        });
        push_runtime_tool_event(
            state,
            session_id,
            &WebToolEvent {
                call_id: call_id.clone(),
                name: name.clone(),
                status: "executing".to_string(),
                risk: risk_name.clone(),
                args: Some(args.clone()),
                result: None,
                success: None,
                file_path: extract_tool_path(&name, &args),
            },
        );
        if let Some(summary) = tool_progress_narration(
            language,
            &name,
            "executing",
            tool_target_path.as_deref(),
            None,
        ) {
            emit_assistant_progress_delta(tx, state, session_id, summary.dedupe_key, summary.text);
        }

        match assistant_call_tool(state, runtime, &name, args.clone()).await {
            Ok(result) => {
                let verified_write = detect_web_file_write(
                    state.host.base_dir(),
                    runtime,
                    &name,
                    &args,
                    &result,
                    pending_file_snapshot.as_ref(),
                );
                persisted_blocks.push(MessageBlock::ToolResult {
                    call_id: call_id.clone(),
                    result: result.clone(),
                    success: true,
                });
                sync_stream_runtime_messages(state, session_id, persisted_blocks);
                if let Some(verified_write) = verified_write {
                    let diff = verified_write.diff;
                    upsert_diff_block(persisted_blocks, diff.clone());
                    sync_stream_runtime_messages(state, session_id, persisted_blocks);
                    push_runtime_checkpoint(
                        state,
                        session_id,
                        localized_string(
                            language,
                            format!("已编辑 {} (+{} / -{})", diff.file_path, diff.added, diff.removed),
                            format!("Edited {} (+{} / -{})", diff.file_path, diff.added, diff.removed),
                        ),
                        language,
                    );
                    let _ = tx.send(StreamEnvelope {
                        r#type: "edited_files".to_string(),
                        session_id: Some(session_id.to_string()),
                        messages: None,
                        delta: None,
                        error: None,
                        activity: Some(activity_event(
                            "editing",
                            Some(localized_string(
                                language,
                                format!("正在编辑 {}", diff.file_path),
                                format!("Editing {}", diff.file_path),
                            )),
                        )),
                        tool: Some(WebToolEvent {
                            call_id: call_id.clone(),
                            name: name.clone(),
                            status: "complete".to_string(),
                            risk: risk_name.clone(),
                            args: Some(args.clone()),
                            result: Some(result.clone()),
                            success: Some(true),
                            file_path: Some(diff.file_path.clone()),
                        }),
                        permission: None,
                        edited_files: Some(vec![web_edited_file_from_diff(&diff)]),
                        research: current_research_payload(state, Some(session_id), runtime, persisted_blocks, mode),
                        subagents: None,
                        verifier: None,
                    });
                    push_runtime_tool_event(
                        state,
                        session_id,
                        &WebToolEvent {
                            call_id: call_id.clone(),
                            name: name.clone(),
                            status: "complete".to_string(),
                            risk: risk_name.clone(),
                            args: Some(args.clone()),
                            result: Some(result.clone()),
                            success: Some(true),
                            file_path: Some(diff.file_path.clone()),
                        },
                    );
                    push_runtime_edited_files(
                        state,
                        session_id,
                        &[web_edited_file_from_diff(&diff)],
                    );
                    refresh_runtime_review_from_filesystem(state, session_id, runtime);
                    if let Some(summary) = tool_progress_narration(
                        language,
                        &name,
                        "complete",
                        Some(&diff.file_path),
                        Some(&result),
                    ) {
                        emit_assistant_progress_delta(tx, state, session_id, summary.dedupe_key, summary.text);
                    }
                    if abort_after_first_workspace_edit {
                        return Err(anyhow!(
                            "intentional test stream abort after first workspace edit"
                        ));
                    }
                } else {
                    let runtime_artifact_diffs = collect_runtime_artifact_diffs(
                        state.host.base_dir(),
                        &runtime.workspace_root,
                        &name,
                        &args,
                        &result,
                        true,
                    );
                    if !runtime_artifact_diffs.is_empty() {
                        let mut edited_files = Vec::new();
                        for diff in runtime_artifact_diffs {
                            push_runtime_checkpoint(
                                state,
                                session_id,
                                localized_string(
                                    language,
                                    format!(
                                        "已生成 {} (+{} / -{})",
                                        diff.file_path, diff.added, diff.removed
                                    ),
                                    format!(
                                        "Generated {} (+{} / -{})",
                                        diff.file_path, diff.added, diff.removed
                                    ),
                                ),
                                language,
                            );
                            upsert_diff_block(persisted_blocks, diff.clone());
                            edited_files.push(web_edited_file_from_diff(&diff));
                        }
                        sync_stream_runtime_messages(state, session_id, persisted_blocks);
                        let primary_file_path = edited_files
                            .last()
                            .map(|file| file.path.clone())
                            .or_else(|| extract_tool_path(&name, &args));
                        let _ = tx.send(StreamEnvelope {
                            r#type: "edited_files".to_string(),
                            session_id: Some(session_id.to_string()),
                            messages: None,
                            delta: None,
                            error: None,
                            activity: Some(activity_event(
                                "artifact",
                                Some(localized_string(
                                    language,
                                    format!("已捕获 {} 个产物", edited_files.len()),
                                    format!("Captured {} artifact(s)", edited_files.len()),
                                )),
                            )),
                            tool: Some(WebToolEvent {
                                call_id: call_id.clone(),
                                name: name.clone(),
                                status: "complete".to_string(),
                                risk: risk_name.clone(),
                                args: Some(args.clone()),
                                result: Some(result.clone()),
                                success: Some(true),
                                file_path: primary_file_path.clone(),
                            }),
                            permission: None,
                            edited_files: Some(edited_files.clone()),
                            research: current_research_payload(state, Some(session_id), runtime, persisted_blocks, mode),
                            subagents: None,
                            verifier: None,
                        });
                        push_runtime_tool_event(
                            state,
                            session_id,
                            &WebToolEvent {
                                call_id: call_id.clone(),
                                name: name.clone(),
                                status: "complete".to_string(),
                                risk: risk_name.clone(),
                                args: Some(args.clone()),
                                result: Some(result.clone()),
                                success: Some(true),
                                file_path: primary_file_path,
                            },
                        );
                        push_runtime_edited_files(state, session_id, &edited_files);
                        refresh_runtime_review_from_filesystem(state, session_id, runtime);
                        if let Some(last_file) = edited_files.last() {
                            if let Some(summary) = tool_progress_narration(
                                language,
                                &name,
                                "complete",
                                Some(&last_file.path),
                                Some(&result),
                            ) {
                                emit_assistant_progress_delta(tx, state, session_id, summary.dedupe_key, summary.text);
                            }
                        }
                        if abort_after_first_workspace_edit {
                            return Err(anyhow!(
                                "intentional test stream abort after first workspace artifact"
                            ));
                        }
                    } else {
                    let _ = tx.send(StreamEnvelope {
                        r#type: "tool".to_string(),
                        session_id: Some(session_id.to_string()),
                        messages: None,
                        delta: None,
                        error: None,
                        activity: Some(activity_event(
                            "tool_complete",
                            Some(localized_string(
                                language,
                                format!("已完成 {}", name),
                                format!("Finished {}", name),
                            )),
                        )),
                        tool: Some(WebToolEvent {
                            call_id: call_id.clone(),
                            name: name.clone(),
                            status: "complete".to_string(),
                            risk: risk_name.clone(),
                            args: Some(args.clone()),
                            result: Some(result.clone()),
                            success: Some(true),
                            file_path: extract_tool_path(&name, &args),
                        }),
                        permission: None,
                        edited_files: None,
                        research: current_research_payload(state, Some(session_id), runtime, persisted_blocks, mode),
                        subagents: None,
                        verifier: None,
                    });
                    push_runtime_tool_event(
                        state,
                        session_id,
                        &WebToolEvent {
                            call_id: call_id.clone(),
                            name: name.clone(),
                            status: "complete".to_string(),
                            risk: risk_name.clone(),
                            args: Some(args.clone()),
                            result: Some(result.clone()),
                            success: Some(true),
                            file_path: extract_tool_path(&name, &args),
                        },
                    );
                    if let Some(summary) = tool_progress_narration(
                        language,
                        &name,
                        "complete",
                        tool_target_path.as_deref(),
                        Some(&result),
                    ) {
                        emit_assistant_progress_delta(tx, state, session_id, summary.dedupe_key, summary.text);
                    }
                    }
                }
            }
            Err(err) => {
                let error_message = format!("Error: {}", err);
                persisted_blocks.push(MessageBlock::ToolResult {
                    call_id: call_id.clone(),
                    result: error_message.clone(),
                    success: false,
                });
                sync_stream_runtime_messages(state, session_id, persisted_blocks);
                push_runtime_branch_note(
                    state,
                    session_id,
                    localized_string(
                        language,
                        format!("{} 失败：{}", name, tail_string(&error_message, 160)),
                        format!("{} failed: {}", name, tail_string(&error_message, 160)),
                    ),
                    language,
                );
                let _ = tx.send(StreamEnvelope {
                    r#type: "tool".to_string(),
                    session_id: Some(session_id.to_string()),
                    messages: None,
                    delta: None,
                    error: None,
                    activity: Some(activity_event(
                        "tool_failed",
                        Some(localized_string(
                            language,
                            format!("执行失败 {}", name),
                            format!("Failed {}", name),
                        )),
                    )),
                    tool: Some(WebToolEvent {
                        call_id: call_id.clone(),
                        name: name.clone(),
                        status: "failed".to_string(),
                        risk: risk_name.clone(),
                        args: Some(args.clone()),
                        result: Some(error_message.clone()),
                        success: Some(false),
                        file_path: extract_tool_path(&name, &args),
                    }),
                    permission: None,
                    edited_files: None,
                    research: current_research_payload(state, Some(session_id), runtime, persisted_blocks, mode),
                    subagents: None,
                    verifier: None,
                });
                push_runtime_tool_event(
                    state,
                    session_id,
                    &WebToolEvent {
                        call_id: call_id.clone(),
                        name: name.clone(),
                        status: "failed".to_string(),
                        risk: risk_name.clone(),
                        args: Some(args.clone()),
                        result: Some(error_message.clone()),
                        success: Some(false),
                        file_path: extract_tool_path(&name, &args),
                    },
                );
                if let Some(summary) = tool_progress_narration(
                    language,
                    &name,
                    "failed",
                    tool_target_path.as_deref(),
                    Some(&error_message),
                ) {
                    emit_assistant_progress_delta(tx, state, session_id, summary.dedupe_key, summary.text);
                }
            }
        }
    }

    Ok(())
}

fn normalize_web_tool_args(tool_name: &str, args: Value) -> Value {
    let mut object = match args {
        Value::Object(map) => map,
        other => return other,
    };

    let normalize_path_value = |value: &str| -> String {
        let normalized = value.replace('\\', "/");
        if let Some(stripped) = normalized.strip_prefix("/home/user/workspace/") {
            return stripped.trim_start_matches('/').to_string();
        }
        if let Some(stripped) = normalized.strip_prefix("/workspace/") {
            return stripped.trim_start_matches('/').to_string();
        }
        if normalized == "/workspace" || normalized == "/home/user/workspace" {
            return ".".to_string();
        }
        normalized
    };

    for key in ["path", "dir", "source", "dest", "destination", "workspace_root"] {
        if let Some(next_value) = object
            .get(key)
            .and_then(|value| value.as_str())
            .map(normalize_path_value)
        {
            object.insert(key.to_string(), Value::String(next_value));
        }
    }

    if tool_name == "terminal_run" {
        if let Some(command) = object
            .get("command")
            .and_then(|value| value.as_str())
            .map(adapt_bash_command_for_powershell)
        {
            object.insert("command".to_string(), Value::String(command));
        }
    }

    if matches!(tool_name, "terminal_create") && !object.contains_key("workspace_root")
    {
        object.insert(
            "workspace_root".to_string(),
            Value::String("__CURRENT_WORKSPACE__".to_string()),
        );
    }

    Value::Object(object)
}

async fn assistant_call_tool(
    state: &WebAppState,
    runtime: &RuntimeSettings,
    name: &str,
    args: Value,
) -> Result<String> {
    if is_web_terminal_tool(name) {
        return execute_web_terminal_tool(state, runtime, name, args).await;
    }
    if name == "gather_context" {
        return gather_context_tool_result(state.host.base_dir(), runtime, &args);
    }

    let assistant_state = state.assistant.clone();
    let assistant_api_url = if runtime.api_url.trim().is_empty() {
        state.assistant_api_url.clone()
    } else {
        runtime.api_url.clone()
    };
    let assistant_api_key = runtime
        .api_key
        .clone()
        .or_else(|| state.assistant_api_key.clone());
    let runtime_for_task = runtime.clone();
    let base_security = state.base_security_config.clone();
    let tool_name = name.to_string();
    let host_base_dir = state.host.base_dir().to_path_buf();

    tokio::task::spawn_blocking(move || -> Result<String> {
        let _cwd_guard = enter_workspace_dir_from(&host_base_dir, &runtime_for_task.workspace_root)?;
        let mut assistant_slot = lock_assistant_mutex(&assistant_state)?;

        if assistant_slot.is_none() {
            let assistant_config = AssistantConfig::new_with_runtime(
                assistant_api_url,
                assistant_api_key,
                runtime_for_task.model.clone(),
                effort_temperature(&runtime_for_task),
                effort_max_tokens(&runtime_for_task),
            );
            let security_config = runtime_security_config(&base_security, &runtime_for_task);
            *assistant_slot = Some(CliAssistant::new(assistant_config, security_config)?);
        }

        assistant_slot
            .as_ref()
            .ok_or_else(|| anyhow!("assistant initialization failed"))?
            .call_tool_without_auth(&tool_name, &args)
    })
    .await
    .map_err(|err| anyhow!("assistant tool call task failed: {}", err))?
}

fn gather_context_tool_result(
    base_dir: &Path,
    runtime: &RuntimeSettings,
    args: &Value,
) -> Result<String> {
    let requested_path = args
        .get("path")
        .or_else(|| args.get("dir"))
        .or_else(|| args.get("workspace_root"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(".");
    let absolute_path = resolve_workspace_relative_path(base_dir, &runtime.workspace_root, requested_path)
        .or_else(|| canonical_workspace_dir_from(base_dir, &runtime.workspace_root).ok())
        .ok_or_else(|| anyhow!("failed to resolve context path '{}'", requested_path))?;
    let metadata = fs::metadata(&absolute_path)
        .map_err(|err| anyhow!("failed to inspect context path '{}': {}", absolute_path.display(), err))?;
    let max_entries = args
        .get("max_entries")
        .and_then(|value| value.as_u64())
        .map(|value| value.clamp(1, 50) as usize)
        .unwrap_or(24);
    let max_preview_chars = args
        .get("max_preview_chars")
        .and_then(|value| value.as_u64())
        .map(|value| value.clamp(120, 4000) as usize)
        .unwrap_or(1200);

    let payload = if metadata.is_dir() {
        json!({
            "path": display_workspace_path(&absolute_path),
            "kind": "directory",
            "entries": gather_context_directory_entries(&absolute_path, max_entries)?,
        })
    } else {
        json!({
            "path": display_workspace_path(&absolute_path),
            "kind": "file",
            "preview": gather_context_file_preview(&absolute_path, max_preview_chars)?,
        })
    };

    serde_json::to_string_pretty(&payload).map_err(Into::into)
}

fn gather_context_directory_entries(path: &Path, max_entries: usize) -> Result<Vec<Value>> {
    let mut entries: Vec<_> = fs::read_dir(path)
        .map_err(|err| anyhow!("failed to read context directory '{}': {}", path.display(), err))?
        .filter_map(|entry| entry.ok())
        .collect();
    entries.sort_by_key(|entry| entry.file_name().to_string_lossy().to_lowercase());

    let mut summarized = Vec::new();
    for entry in entries.into_iter().take(max_entries) {
        let file_type = entry
            .file_type()
            .map_err(|err| anyhow!("failed to inspect '{}': {}", entry.path().display(), err))?;
        let kind = if file_type.is_dir() {
            "directory"
        } else if file_type.is_file() {
            "file"
        } else if file_type.is_symlink() {
            "symlink"
        } else {
            "other"
        };
        summarized.push(json!({
            "name": entry.file_name().to_string_lossy().to_string(),
            "kind": kind,
        }));
    }

    Ok(summarized)
}

fn gather_context_file_preview(path: &Path, max_preview_chars: usize) -> Result<String> {
    let content = read_text_file(path)
        .map_err(|err| anyhow!("failed to read context file '{}': {}", path.display(), err))?;
    let total_chars = content.chars().count();
    let preview = content.chars().take(max_preview_chars).collect::<String>();
    if total_chars > max_preview_chars {
        Ok(format!("{}\n...[truncated]", preview))
    } else {
        Ok(preview)
    }
}

fn is_web_terminal_tool(name: &str) -> bool {
    matches!(name, "terminal_create" | "terminal_run" | "terminal_read")
}

async fn execute_web_terminal_tool(
    state: &WebAppState,
    runtime: &RuntimeSettings,
    name: &str,
    args: Value,
) -> Result<String> {
    match name {
        "terminal_create" => {
            let workspace_root = args
                .get("workspace_root")
                .and_then(|value| value.as_str())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(&runtime.workspace_root);
            let terminal_id = create_terminal_session(state, workspace_root).await?;
            let snapshot = terminal_snapshot(state, Some(&terminal_id), Some(12_000))?;
            Ok(serde_json::to_string_pretty(&json!({
                "success": true,
                "terminal_id": terminal_id,
                "terminal": snapshot,
            }))?)
        }
        "terminal_run" => {
            let command = args
                .get("command")
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| anyhow!("terminal_run requires command"))?;
            validate_terminal_command(command)?;

            let mut terminal_id = match args
                .get("terminal_id")
                .and_then(|value| value.as_str())
                .filter(|value| !value.trim().is_empty())
            {
                Some(id) => id.to_string(),
                None => match active_terminal_id(state) {
                    Some(id) => id,
                    None => create_terminal_session(state, &runtime.workspace_root).await?,
                },
            };

            if !terminal_session_exists(state, &terminal_id) {
                terminal_id = create_terminal_session(state, &runtime.workspace_root).await?;
            }

            if let Err(first_error) = write_terminal_input(state, &terminal_id, command).await {
                terminal_id = create_terminal_session(state, &runtime.workspace_root).await?;
                write_terminal_input(state, &terminal_id, command)
                    .await
                    .map_err(|second_error| {
                        anyhow!(
                            "terminal_run failed after terminal recovery: {}; retry error: {}",
                            first_error,
                            second_error
                        )
                    })?;
            }
            let wait_ms = args
                .get("wait_ms")
                .and_then(|value| value.as_u64())
                .unwrap_or(600)
                .min(5_000);
            if wait_ms > 0 {
                tokio::time::sleep(Duration::from_millis(wait_ms)).await;
            }
            let snapshot = terminal_snapshot(state, Some(&terminal_id), Some(16_000))?;
            Ok(serde_json::to_string_pretty(&json!({
                "success": true,
                "terminal_id": terminal_id,
                "command": command,
                "terminal": snapshot,
            }))?)
        }
        "terminal_read" => {
            let tail = args
                .get("tail")
                .and_then(|value| value.as_u64())
                .unwrap_or(12_000)
                .min(64_000) as usize;
            let terminal_id = args
                .get("terminal_id")
                .and_then(|value| value.as_str())
                .filter(|value| !value.trim().is_empty())
                .map(|value| value.to_string())
                .or_else(|| active_terminal_id(state));
            let snapshot = terminal_snapshot(state, terminal_id.as_deref(), Some(tail))?;
            Ok(serde_json::to_string_pretty(&json!({
                "success": true,
                "terminal_id": snapshot.get("id").cloned().unwrap_or(Value::Null),
                "terminal": snapshot,
            }))?)
        }
        other => Err(anyhow!("unsupported web terminal tool: {}", other)),
    }
}

fn active_terminal_id(state: &WebAppState) -> Option<String> {
    state.terminal_runtime.lock().ok().and_then(|mut runtime| {
        prune_terminal_sessions(&mut runtime);
        runtime.active_id.clone()
    })
}

fn validate_terminal_command(command: &str) -> Result<()> {
    if command.len() > 4096 || command.contains('\n') || command.contains('\r') {
        return Err(anyhow!("terminal command is empty, too long, or contains newlines"));
    }
    let normalized = command.trim().to_ascii_lowercase();
    let blocked = [
        "rm -rf /",
        "del /f /s /q",
        "format ",
        "diskpart",
        "shutdown ",
        "restart-computer",
        "stop-computer",
    ];
    if blocked.iter().any(|pattern| normalized.contains(pattern)) {
        return Err(anyhow!("terminal command blocked by safety guard"));
    }
    Ok(())
}

fn terminal_session_exists(state: &WebAppState, terminal_id: &str) -> bool {
    state
        .terminal_runtime
        .lock()
        .ok()
        .map(|mut runtime| {
            prune_terminal_sessions(&mut runtime);
            runtime.sessions.iter().any(|session| session.id == terminal_id)
        })
        .unwrap_or(false)
}

fn terminal_snapshot(
    state: &WebAppState,
    terminal_id: Option<&str>,
    tail: Option<usize>,
) -> Result<Value> {
    let payload = build_terminal_payload(state)?;
    let id = terminal_id
        .map(|value| value.to_string())
        .or(payload.active_id.clone())
        .ok_or_else(|| anyhow!("no active terminal session"))?;
    let session = payload
        .sessions
        .into_iter()
        .find(|session| session.id == id)
        .ok_or_else(|| anyhow!("terminal not found: {}", id))?;
    let buffer = tail_string(&session.buffer, tail.unwrap_or(12_000));
    Ok(json!({
        "id": session.id,
        "title": session.title,
        "cwd": session.cwd,
        "created_at": session.created_at,
        "command": session.command,
        "status": session.status,
        "buffer": buffer,
    }))
}

fn tail_string(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    let skip = input.chars().count().saturating_sub(max_chars);
    input.chars().skip(skip).collect()
}

fn current_research_payload(
    state: &WebAppState,
    session_id: Option<&str>,
    runtime: &RuntimeSettings,
    messages: &[MessageBlock],
    mode: Option<&str>,
) -> Option<WebResearchPayload> {
    let normalized = mode.unwrap_or("chat").trim().to_ascii_lowercase();
    if normalized == "research" || normalized == "spec" {
        let runtime_snapshot = session_id
            .and_then(|id| lock_stream_runtime(state).ok().and_then(|sessions| sessions.get(id).map(clone_stream_runtime_view)));
        Some(build_research_payload(
            runtime,
            messages,
            runtime_snapshot.as_ref(),
            SessionResearchState::Research,
        ))
    } else {
        None
    }
}

fn detect_plaintext_tool_narration(input: &str) -> Vec<String> {
    static PLAIN_TOOL_CALL_RE: OnceLock<Regex> = OnceLock::new();
    let tool_re = PLAIN_TOOL_CALL_RE.get_or_init(|| {
        Regex::new(r#"(?m)^Tool\s+([A-Za-z_][A-Za-z0-9_]*)\s*$"#)
            .expect("valid plaintext tool call regex")
    });

    let mut tool_names = Vec::new();
    for captures in tool_re.captures_iter(input) {
        let Some(tool_name) = captures.get(1).map(|value| value.as_str().trim()) else {
            continue;
        };
        if tool_name.is_empty() {
            continue;
        }
        tool_names.push(normalize_plaintext_tool_name(tool_name));
    }

    tool_names
}

fn normalize_plaintext_tool_name(tool_name: &str) -> String {
    let trimmed = tool_name.trim();
    if trimmed.eq_ignore_ascii_case("bash") || trimmed.eq_ignore_ascii_case("shell") {
        return "terminal_run".to_string();
    }
    trimmed.to_string()
}


fn summarize_tool_args_for_provider(tool_name: &str, args: &Value) -> String {
    match tool_name {
        "write_file" => {
            let path = extract_tool_path(tool_name, args).unwrap_or_else(|| "unknown".to_string());
            let bytes = args
                .get("content")
                .and_then(Value::as_str)
                .map(|value| value.len())
                .unwrap_or(0);
            format!("path={} bytes={}", path, bytes)
        }
        "edit_file" => {
            let path = extract_tool_path(tool_name, args).unwrap_or_else(|| "unknown".to_string());
            let mode = args
                .get("mode")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            format!("path={} mode={}", path, mode)
        }
        "read_file" => {
            let path = extract_tool_path(tool_name, args).unwrap_or_else(|| "unknown".to_string());
            format!("path={}", path)
        }
        _ => tail_string(&serde_json::to_string(args).unwrap_or_default(), 180),
    }
}

fn summarize_tool_result_for_provider_memory(tool_name: &str, raw: &str, success: bool) -> String {
    if let Ok(value) = serde_json::from_str::<Value>(raw) {
        if matches!(tool_name, "write_file" | "edit_file" | "read_file" | "delete_file") {
            let path = value
                .pointer("/data/path")
                .and_then(Value::as_str)
                .or_else(|| value.get("path").and_then(Value::as_str))
                .unwrap_or("unknown");
            let operation = value
                .get("operation")
                .and_then(Value::as_str)
                .unwrap_or(tool_name);
            return format!(
                "{} {} {}",
                operation,
                if success { "ok" } else { "failed" },
                path.replace('\\', "/")
            );
        }
    }
    let parsed = parse_tool_result_evidence(tool_name, raw, success);
    if !parsed.summary.trim().is_empty() {
        return tail_string(&parsed.summary, 220);
    }
    tail_string(raw, 220)
}

fn combine_assistant_segments(existing: &str, next: &str) -> String {
    if existing.is_empty() {
        return next.to_string();
    }
    if next.is_empty() {
        return existing.to_string();
    }

    let mut combined = existing.to_string();
    let needs_break = !combined.ends_with(['\n', ' ', '\t'])
        && !next.starts_with(['\n', ' ', '\t']);
    if needs_break {
        combined.push_str("\n\n");
    }
    combined.push_str(next);
    combined
}

fn merge_stream_text(existing: &str, incoming: &str) -> String {
    if existing.is_empty() {
        return incoming.to_string();
    }
    if incoming.is_empty() {
        return existing.to_string();
    }
    if incoming.starts_with(existing) {
        return incoming.to_string();
    }
    if existing.ends_with(incoming) {
        return existing.to_string();
    }

    let max_overlap = existing.len().min(incoming.len());
    let mut overlap_points = incoming
        .char_indices()
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    overlap_points.push(incoming.len());

    for overlap in overlap_points.into_iter().rev() {
        if overlap == 0 || overlap > max_overlap {
            continue;
        }
        if existing.ends_with(&incoming[..overlap]) {
            let mut merged = existing.to_string();
            merged.push_str(&incoming[overlap..]);
            return merged;
        }
    }

    let mut merged = existing.to_string();
    merged.push_str(incoming);
    merged
}

fn capture_pending_file_snapshot(
    base_dir: &Path,
    runtime: &RuntimeSettings,
    tool_name: &str,
    args: &Value,
) -> Option<PendingFileSnapshot> {
    if tool_name != "write_file" && tool_name != "edit_file" {
        return None;
    }

    let display_path = extract_tool_path(tool_name, args)?;
    let absolute_path = resolve_workspace_relative_path(base_dir, &runtime.workspace_root, &display_path)?;
    let old_content = read_text_file(&absolute_path).unwrap_or_default();

    Some(PendingFileSnapshot {
        display_path,
        absolute_path,
        old_content,
    })
}

fn bind_tool_args_to_workspace(runtime: &RuntimeSettings, args: &Value) -> Value {
    let mut object = match args {
        Value::Object(map) => map.clone(),
        _ => return args.clone(),
    };

    let workspace_root = runtime.workspace_root.trim();
    if workspace_root.is_empty() {
        return Value::Object(object);
    }

    for key in ["workspace_root"] {
        if let Some(value) = object.get(key).and_then(Value::as_str) {
            if value == "__CURRENT_WORKSPACE__" || value.trim().is_empty() {
                object.insert(key.to_string(), Value::String(workspace_root.to_string()));
            }
        }
    }

    Value::Object(object)
}

fn detect_web_file_write(
    base_dir: &Path,
    runtime: &RuntimeSettings,
    tool_name: &str,
    args: &Value,
    result: &str,
    snapshot: Option<&PendingFileSnapshot>,
) -> Option<VerifiedWorkspaceWrite> {
    if tool_name != "write_file" && tool_name != "edit_file" {
        return None;
    }

    let snapshot = snapshot?;
    if serde_json::from_str::<Value>(result)
        .ok()
        .and_then(|value| value.get("status").and_then(Value::as_str).map(str::to_string))
        .map(|status| !status.eq_ignore_ascii_case("success"))
        .unwrap_or(false)
    {
        return None;
    }

    let new_content = read_text_file(&snapshot.absolute_path).ok()?;

    if new_content.is_empty() || snapshot.old_content == new_content {
        return None;
    }

    let absolute_path = resolve_workspace_relative_path(
        base_dir,
        &runtime.workspace_root,
        &snapshot.display_path,
    )?;
    if absolute_path != snapshot.absolute_path || !absolute_path.exists() {
        return None;
    }

    Some(VerifiedWorkspaceWrite {
        absolute_path,
        diff: FileDiff::compute(
        &snapshot.display_path,
        &snapshot.old_content,
        &new_content,
        ),
    })
}

fn extract_tool_path(tool_name: &str, args: &Value) -> Option<String> {
    let key = match tool_name {
        "write_file" | "edit_file" | "read_file" | "delete_file" => "path",
        "list_dir" | "mkdir" | "create_dir" => "dir",
        _ => "path",
    };

    args.get(key)
        .or_else(|| args.get("path"))
        .and_then(|value| value.as_str())
        .map(|value| value.replace('\\', "/"))
}

fn resolve_workspace_relative_path(base_dir: &Path, workspace_root: &str, raw_path: &str) -> Option<PathBuf> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized = trimmed
        .replace('\\', "/")
        .replace("/home/user/workspace/", "")
        .replace("/workspace/", "")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string();
    let candidate = if normalized.is_empty() { trimmed } else { normalized.as_str() };
    let path = PathBuf::from(candidate);
    if path.is_absolute() {
        return Some(path);
    }

    canonical_workspace_dir_from(base_dir, workspace_root)
        .ok()
        .map(|workspace| workspace.join(path))
}

fn tool_call_is_allowed(
    state: &WebAppState,
    runtime: &RuntimeSettings,
    tool_name: &str,
    risk: &RiskLevel,
) -> bool {
    let security = runtime_security_config(&state.base_security_config, runtime);
    if security.auto_approve_tools {
        risk <= &security.max_auto_approve_risk
    } else {
        false
    }
}

async fn wait_for_tool_approval(
    state: &WebAppState,
    session_id: &str,
    call_id: &str,
    name: &str,
    risk: &str,
    args: &Value,
    tx: &tokio::sync::mpsc::UnboundedSender<StreamEnvelope>,
) -> Result<bool> {
    let (approval_tx, approval_rx) = oneshot::channel::<bool>();
    {
        let mut runtime = lock_stream_runtime(state)?;
        let session = runtime
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("missing stream session runtime"))?;
        session
            .pending_approvals
            .insert(
                call_id.to_string(),
                PendingApprovalRuntime {
                    sender: approval_tx,
                    name: name.to_string(),
                    risk: risk.to_string(),
                    args: args.clone(),
                },
            );
    }

    let _ = tx.send(StreamEnvelope {
        r#type: "permission_required".to_string(),
        session_id: Some(session_id.to_string()),
        messages: None,
        delta: None,
        error: None,
        activity: Some(activity_event(
            "permission_required",
            Some(name.to_string()),
        )),
        tool: None,
        permission: Some(WebPermissionRequest {
            call_id: call_id.to_string(),
            name: name.to_string(),
            risk: risk.to_string(),
            reason: format!("Tool '{}' requires manual approval.", name),
            args: args.clone(),
        }),
        edited_files: None,
        research: None,
        subagents: None,
        verifier: None,
    });

    let approved = approval_rx.await.unwrap_or(false);
    Ok(approved)
}

struct StreamTurnResult {
    text: String,
    finish_reason: Option<String>,
    tool_calls: Option<Vec<Value>>,
    pseudo_tool_names: Vec<String>,
}

fn build_bootstrap(state: &WebAppState) -> Result<WebBootstrap> {
    let runtime = {
        let runtime = lock_runtime_settings(state)?;
        runtime.clone()
    };
    let sandbox_bootstrap = initialize_app_sandbox(&state.host.paths)?;
    let mut session_manager = lock_session_manager(state)?;
    let _ = session_manager.refresh_summaries();
    if session_manager.current_id.is_none() {
        restore_session_selection(&mut session_manager, &state.persisted_state_path);
    }
    let current_id = session_manager.current_id.clone();
    let messages = if let Some(ref id) = current_id {
        session_manager.load_messages(id).unwrap_or_default()
    } else {
        Vec::new()
    };
    let active_sessions = {
        let runtime = lock_stream_runtime(state)?;
        runtime
            .iter()
            .map(|(session_id, runtime)| WebActiveSession {
                session_id: session_id.clone(),
                status: "running".to_string(),
                waiting_approval: !runtime.pending_approvals.is_empty(),
            })
            .collect::<Vec<_>>()
    };
    let runtime_snapshots = {
        let runtime = lock_stream_runtime(state)?;
        runtime
            .iter()
            .map(|(session_id, session)| WebSessionRuntimeSnapshot {
                session_id: session_id.clone(),
                partial_text: session.partial_text.clone(),
                progress_updates: session.progress_updates.clone(),
                latest_activity: session.latest_activity.clone(),
                tool_events: session.tool_events.clone(),
                edited_files: session.edited_files.clone(),
                permission: session.pending_approvals.iter().next().map(|(call_id, pending)| WebPermissionRequest {
                    call_id: call_id.clone(),
                    name: pending.name.clone(),
                    risk: pending.risk.clone(),
                    reason: format!("Tool '{}' requires manual approval.", pending.name),
                    args: pending.args.clone(),
                }),
                subagents: session.subagents.clone(),
                verifier: session.verifier.clone(),
                checkpoints: session.checkpoints.clone(),
                branch_notes: session.branch_notes.clone(),
                timeline: session.timeline.clone(),
            })
            .collect::<Vec<_>>()
    };
    let active_research_runtime = current_id.as_ref().and_then(|session_id| {
        lock_stream_runtime(state)
            .ok()
            .and_then(|runtime| runtime.get(session_id).map(clone_stream_runtime_view))
    });
    let current_research_state = infer_session_research_state(&messages);
    let review = current_id
        .as_ref()
        .and_then(|session_id| {
            let sessions = lock_stream_runtime(state).ok()?;
            let session = sessions.get(session_id)?;
            let runtime_paths = session
                .edited_files
                .iter()
                .map(|file| file.path.clone())
                .collect::<Vec<_>>();
            let selected_paths = collect_review_paths_for_current_turn(&messages, &runtime_paths);
            if selected_paths.is_empty() {
                return Some(WebReviewPayload::default());
            }
            try_build_review_payload(state.host.base_dir(), &runtime.workspace_root, &selected_paths).ok()
        })
        .or_else(|| {
            let selected_paths = collect_review_paths_for_current_turn(&messages, &[]);
            if selected_paths.is_empty() {
                Some(WebReviewPayload::default())
            } else {
                try_build_review_payload(
                    state.host.base_dir(),
                    &runtime.workspace_root,
                    &selected_paths,
                )
                .ok()
            }
        })
        .unwrap_or_default();

    Ok(WebBootstrap {
        host: state.host.descriptor(),
        workspace_root: runtime.workspace_root.clone(),
        sandbox: WebSandboxBootstrapPayload {
            initialized: true,
            first_run: sandbox_bootstrap.first_run,
            sandbox_root: sandbox_bootstrap.manifest.sandbox_root,
            downloads_root: sandbox_bootstrap.manifest.downloads_root,
            sessions_root: sandbox_bootstrap.manifest.sessions_root,
        },
        config: runtime_to_payload(&runtime),
        research: build_research_payload(
            &runtime,
            &messages,
            active_research_runtime.as_ref(),
            current_research_state,
        ),
        review,
        git: build_git_payload(state.host.base_dir(), &runtime.workspace_root, false, false).unwrap_or_else(|err| WebGitPayload {
            available: false,
            repository_root: runtime.workspace_root.clone(),
            error: Some(err.to_string()),
            ..WebGitPayload::default()
        }),
        workspace_browser: build_workspace_browser(state.host.base_dir(), &runtime.workspace_root).unwrap_or_else(|_| {
            WebWorkspaceBrowser {
                root_name: basename_for_display(&runtime.workspace_root),
                root_path: runtime.workspace_root.clone(),
                entries: Vec::new(),
            }
        }),
        sessions: session_manager.list_recent(20).to_vec(),
        active_sessions,
        runtime_snapshots,
        current_session_id: current_id.clone(),
        branches: current_id
            .as_ref()
            .and_then(|id| session_manager.index.iter().find(|m| &m.id == id))
            .map(|meta| meta.branches.clone())
            .unwrap_or_else(|| vec![SessionBranch::main()]),
        messages: messages_to_web(&messages),
    })
}


fn build_research_payload(
    runtime: &RuntimeSettings,
    messages: &[MessageBlock],
    session_runtime: Option<&StreamSessionRuntimeView>,
    session_state: SessionResearchState,
) -> WebResearchPayload {
    let topic = infer_research_topic(messages)
        .unwrap_or_else(|| "待确定研究课题".to_string());
    let workflow_kind = infer_research_workflow_kind(&topic, messages);
    let capability = detect_system_capability(runtime);
    let assessment = assess_research_runtime(&workflow_kind, messages, &capability);
    let graph = build_research_graph(&workflow_kind, messages, &assessment);
    let current_node = graph
        .nodes
        .iter()
        .find(|node| node.status == "current" || node.status == "blocked" || node.status == "resumable")
        .or_else(|| graph.nodes.first());
    let next_phase = assessment.next_phase_override.clone().or_else(|| {
        current_node.and_then(|node| {
            graph.edges.iter()
                .find(|edge| edge.from == node.id)
                .and_then(|edge| graph.nodes.iter().find(|candidate| candidate.id == edge.to))
                .map(|node| node.label.clone())
        })
    });
    let phase_index = graph
        .nodes
        .iter()
        .position(|node| current_node.is_some_and(|current| current.id == node.id))
        .map(|index| index + 1)
        .unwrap_or(0);
    let phase_total = graph.nodes.len().max(1);
    let phase = current_node
        .map(|node| node.label.clone())
        .unwrap_or_else(|| "Plan".to_string());

    WebResearchPayload {
        active: session_state == SessionResearchState::Research,
        topic,
        phase,
        phase_index,
        phase_total,
        next_phase,
        workspace: Some(runtime.workspace_root.clone()),
        security_level: if runtime.privacy_mode {
            "Confidential"
        } else {
            "Public"
        }
        .to_string(),
        waiting_approval: runtime.competition_mode,
        competition_mode: runtime.competition_mode,
        workflow_kind: workflow_kind.clone(),
        overall_state: assessment.overall_state,
        rationale: research_workflow_rationale(&workflow_kind).to_string(),
        blocker: assessment.blocker,
        recovery_hint: assessment.recovery_hint,
        resume_points: assessment.resume_points,
        resource_summary: assessment.resource_summary,
        graph,
        review: research_review_checks(&workflow_kind),
        runtime: build_research_runtime_event(messages, session_runtime),
    }
}

fn infer_research_topic(messages: &[MessageBlock]) -> Option<String> {
    messages.iter().rev().find_map(|message| match message {
        MessageBlock::User { content, .. } => {
            let trimmed = content.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.chars().take(96).collect::<String>())
            }
        }
        _ => None,
    })
}

fn infer_research_workflow_kind(topic: &str, messages: &[MessageBlock]) -> String {
    let mut corpus = topic.to_ascii_lowercase();
    for message in messages.iter().rev().take(12) {
        match message {
            MessageBlock::User { content, .. }
            | MessageBlock::Assistant { content }
            | MessageBlock::AssistantStreaming { content } => {
                corpus.push('\n');
                corpus.push_str(&content.to_ascii_lowercase());
                corpus.push('\n');
                corpus.push_str(content);
            }
            MessageBlock::ToolCall { name, .. } => {
                corpus.push('\n');
                corpus.push_str(&name.to_ascii_lowercase());
            }
            _ => {}
        }
    }

    if contains_any(
        &corpus,
        &[
            "sklearn",
            "scikit-learn",
            "iris",
            "logistic regression",
            "linear regression",
            "random forest",
            "decision tree",
            "svm",
            "xgboost",
            "lightgbm",
            "kmeans",
            "classification",
            "classifier",
            "regression",
            "逻辑回归",
            "线性回归",
            "随机森林",
            "决策树",
            "支持向量机",
            "聚类",
            "分类",
            "轻量机器学习",
        ],
    ) && !contains_any(
        &corpus,
        &[
            "deep learning",
            "neural",
            "cnn",
            "transformer",
            "bert",
            "llm",
            "pytorch",
            "tensorflow",
            "cuda",
            "gpu",
            "finetune",
            "fine-tune",
            "微调",
            "深度学习",
        ],
    ) {
        "data_analysis".to_string()
    } else if contains_any(
        &corpus,
        &[
            "deep learning",
            "neural",
            "cnn",
            "transformer",
            "bert",
            "llm",
            "pytorch",
            "tensorflow",
            "epoch",
            "checkpoint",
            "训练",
            "gpu",
            "cuda",
            "深度学习",
            "模型训练",
            "finetune",
            "fine-tune",
            "微调",
        ],
    ) {
        "deep_learning".to_string()
    } else if contains_any(
        &corpus,
        &["proof", "定理", "证明", "lemma", "数学", "推导"],
    ) {
        "theory".to_string()
    } else if contains_any(
        &corpus,
        &[
            "experiment design",
            "protocol",
            "assay",
            "pcr",
            "western blot",
            "cell culture",
            "实验设计",
            "实验方案",
            "生物实验",
            "化学实验",
            "对照组",
        ],
    ) {
        "experimental_design".to_string()
    } else if contains_any(
        &corpus,
        &["literature", "综述", "论文", "review", "citation", "文献"],
    ) {
        "literature_review".to_string()
    } else if contains_any(
        &corpus,
        &["simulation", "monte carlo", "有限元", "仿真", "模拟"],
    ) {
        "simulation".to_string()
    } else if contains_any(
        &corpus,
        &["dataset", "kmeans", "regression", "classification", "数据", "聚类", "统计"],
    ) {
        "data_analysis".to_string()
    } else {
        "adaptive_research".to_string()
    }
}

fn contains_any(input: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| input.contains(&needle.to_ascii_lowercase()))
}

fn system_capability_cache() -> &'static Mutex<Option<CachedSystemCapability>> {
    static CACHE: OnceLock<Mutex<Option<CachedSystemCapability>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

fn system_capability_signature(runtime: &RuntimeSettings) -> String {
    let cuda = runtime
        .toolchains
        .get("cuda")
        .map(|value| value.trim())
        .unwrap_or("");
    let python = runtime
        .toolchains
        .get("python")
        .map(|value| value.trim())
        .unwrap_or("");
    format!(
        "{}|{}|{}",
        runtime.workspace_root.trim(),
        cuda,
        python
    )
}

fn detect_system_capability(runtime: &RuntimeSettings) -> SystemCapability {
    let signature = system_capability_signature(runtime);
    if let Ok(cache) = system_capability_cache().lock() {
        if let Some(entry) = cache.as_ref() {
            if entry.signature == signature && entry.fetched_at.elapsed() < Duration::from_secs(8) {
                return entry.capability.clone();
            }
        }
    }

    let mut capability = SystemCapability {
        cpu_cores: std::thread::available_parallelism().map(|n| n.get() as u32).unwrap_or(1),
        total_memory_mb: None,
        available_memory_mb: None,
        gpu_hint: None,
    };

    let monitor = crate::tools::system::system_monitor::SystemMonitor::default();
    if let Ok(raw) = monitor.get_system_resources() {
        if let Ok(value) = serde_json::from_str::<Value>(&raw) {
            capability.cpu_cores = value
                .pointer("/data/cpu/cores")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .unwrap_or(capability.cpu_cores);
            capability.total_memory_mb = value
                .pointer("/data/memory/total_mb")
                .and_then(|v| v.as_u64());
            capability.available_memory_mb = value
                .pointer("/data/memory/available_mb")
                .and_then(|v| v.as_u64());
        }
    }

    let cuda = runtime
        .toolchains
        .get("cuda")
        .filter(|v| !v.trim().is_empty())
        .cloned();
    let python = runtime
        .toolchains
        .get("python")
        .filter(|v| !v.trim().is_empty())
        .cloned();
    capability.gpu_hint = cuda.or(python.map(|_| "python-runtime".to_string()));

    if let Ok(mut cache) = system_capability_cache().lock() {
        *cache = Some(CachedSystemCapability {
            signature,
            fetched_at: Instant::now(),
            capability: capability.clone(),
        });
    }

    capability
}

fn assess_research_runtime(
    kind: &str,
    messages: &[MessageBlock],
    capability: &SystemCapability,
) -> ResearchRuntimeAssessment {
    let mut assessment = ResearchRuntimeAssessment {
        overall_state: "active".to_string(),
        ..ResearchRuntimeAssessment::default()
    };

    let tool_corpus = messages
        .iter()
        .rev()
        .take(40)
        .filter_map(|message| match message {
            MessageBlock::ToolResult { result, .. } => Some(result.to_ascii_lowercase()),
            MessageBlock::Assistant { content }
            | MessageBlock::AssistantStreaming { content } => Some(content.to_ascii_lowercase()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    if kind == "deep_learning" {
        let mem = capability.available_memory_mb.unwrap_or(0);
        let weak_cpu = capability.cpu_cores > 0 && capability.cpu_cores <= 4;
        let no_gpu_hint = capability.gpu_hint.is_none();
        let oom_or_device = contains_any(
            &tool_corpus,
            &[
                "out of memory",
                "cuda out of memory",
                "not enough memory",
                "no module named torch",
                "no cuda",
                "gpu not available",
                "显存不足",
                "内存不足",
                "找不到cuda",
                "设备不可用",
            ],
        );
        let likely_blocked = oom_or_device || (mem > 0 && mem < 8192 && weak_cpu && no_gpu_hint);

        assessment.resource_summary = Some(format!(
            "CPU {} cores / avail mem {} MB / {}",
            capability.cpu_cores,
            capability.available_memory_mb.unwrap_or(0),
            capability
                .gpu_hint
                .as_deref()
                .unwrap_or("no GPU hint")
        ));

        if likely_blocked {
            assessment.overall_state = "blocked".to_string();
            assessment.blocker = Some(
                "当前机器资源不足以稳定执行深度学习训练，建议保留前序准备结果并迁移到更高性能环境继续。"
                    .to_string(),
            );
            assessment.recovery_hint = Some(
                "保留数据清洗、baseline、训练脚本和 checkpoint 配置；切换到高性能机器后，从“训练与监控”节点继续。"
                    .to_string(),
            );
            assessment.resume_points = vec![
                "从 baseline 配置恢复".to_string(),
                "从最近 checkpoint 或训练配置继续".to_string(),
                "在高性能机器上恢复 train_monitor 节点".to_string(),
            ];
            assessment.current_node_override = Some("train_monitor".to_string());
            assessment.next_phase_override =
                Some("切换到更高性能环境后恢复训练".to_string());
        } else if contains_any(&tool_corpus, &["checkpoint", "resume", "继续训练", "恢复训练"]) {
            assessment.overall_state = "resumable".to_string();
            assessment.recovery_hint = Some(
                "已检测到可恢复训练线索，可从最近 checkpoint 或训练配置继续。".to_string(),
            );
            assessment.resume_points = vec![
                "读取最近 checkpoint".to_string(),
                "复用现有训练脚本与参数".to_string(),
                "从 train_monitor 节点继续".to_string(),
            ];
        }
    }

    assessment
}

fn build_research_graph(
    kind: &str,
    messages: &[MessageBlock],
    assessment: &ResearchRuntimeAssessment,
) -> ResearchGraphPayload {
    let specs = research_workflow_specs(kind);
    let current_index = infer_research_progress_index(kind, messages, specs.len());
    let current_override = assessment.current_node_override.as_deref();
    let nodes = specs
        .iter()
        .enumerate()
        .map(|(index, spec)| ResearchGraphNode {
            id: spec.0.to_string(),
            label: spec.1.to_string(),
            detail: if current_override == Some(spec.0) {
                assessment
                    .recovery_hint
                    .clone()
                    .unwrap_or_else(|| spec.2.to_string())
            } else {
                spec.2.to_string()
            },
            status: if current_override == Some(spec.0) && assessment.overall_state == "blocked" {
                "blocked".to_string()
            } else if current_override == Some(spec.0) && assessment.overall_state == "resumable" {
                "resumable".to_string()
            } else if index < current_index {
                "done".to_string()
            } else if index == current_index {
                "current".to_string()
            } else {
                "pending".to_string()
            },
            lane: spec.3.to_string(),
            x: 24 + ((index as i32 % 2) * 136),
            y: 28 + (index as i32 * 78),
        })
        .collect::<Vec<_>>();

    let mut edges = specs
        .windows(2)
        .map(|pair| ResearchGraphEdge {
            from: pair[0].0.to_string(),
            to: pair[1].0.to_string(),
            label: None,
        })
        .collect::<Vec<_>>();

    if kind == "deep_learning" {
        edges.push(ResearchGraphEdge {
            from: "train_monitor".to_string(),
            to: "debug_or_checkpoint".to_string(),
            label: Some("unstable/loss plateau".to_string()),
        });
        edges.push(ResearchGraphEdge {
            from: "debug_or_checkpoint".to_string(),
            to: "train_monitor".to_string(),
            label: Some("retry".to_string()),
        });
    }

    ResearchGraphPayload { nodes, edges }
}

fn infer_research_progress_index(kind: &str, messages: &[MessageBlock], total: usize) -> usize {
    if total == 0 {
        return 0;
    }

    let mut has_evidence = false;
    let mut has_design = false;
    let mut has_execution = false;
    let mut has_validation = false;
    let mut has_delivery = false;
    let mut has_review = false;

    for message in messages.iter().rev().take(32) {
        match message {
            MessageBlock::ToolCall { name, .. } => {
                let name = name.as_str();
                if matches!(name, "read_file" | "list_dir" | "search_files" | "grep_files" | "terminal_read") {
                    has_evidence = true;
                }
                if matches!(name, "write_file" | "edit_file") {
                    has_design = true;
                }
                if matches!(name, "terminal_run" | "run_python" | "run_command" | "run_safe_command") {
                    has_execution = true;
                }
                if matches!(name, "git_status" | "git_diff" | "run_tests") {
                    has_validation = true;
                }
            }
            MessageBlock::ToolResult { result, success, .. } => {
                let text = result.to_ascii_lowercase();
                if *success {
                    if contains_any(&text, &["accuracy", "loss", "f1", "auc", "val_", "metrics", "结果", "指标", "误差"]) {
                        has_validation = true;
                    }
                    if contains_any(&text, &["saved", "written", "created", "report", "summary", "导出", "保存", "报告", "结论"]) {
                        has_delivery = true;
                    }
                } else {
                    has_review = true;
                }
            }
            MessageBlock::Diff { .. } => {
                has_design = true;
                has_delivery = true;
            }
            MessageBlock::Assistant { content } | MessageBlock::AssistantStreaming { content } => {
                let text = content.to_ascii_lowercase();
                if contains_any(&text, &["validate", "verification", "cross-check", "误差分析", "验证", "核查"]) {
                    has_validation = true;
                }
                if contains_any(&text, &["self-review", "risk", "assumption", "局限", "假设", "风险", "审查"]) {
                    has_review = true;
                }
            }
            _ => {}
        }
    }

    let stage = match kind {
        "deep_learning" => {
            if has_delivery { 5 }
            else if has_validation { 4 }
            else if has_execution && has_review { 3 }
            else if has_execution { 2 }
            else if has_evidence { 1 }
            else { 0 }
        }
        "theory" => {
            if has_delivery { 5 }
            else if has_validation { 4 }
            else if has_review { 3 }
            else if has_design || has_evidence { 2 }
            else if has_evidence { 1 }
            else { 0 }
        }
        "experimental_design" => {
            if has_delivery { 5 }
            else if has_validation { 4 }
            else if has_execution { 3 }
            else if has_design { 2 }
            else if has_evidence { 1 }
            else { 0 }
        }
        _ => {
            if has_delivery { total.saturating_sub(1) }
            else if has_validation { total.saturating_sub(2).min(total.saturating_sub(1)) }
            else if has_execution { 3.min(total.saturating_sub(1)) }
            else if has_design { 2.min(total.saturating_sub(1)) }
            else if has_evidence { 1.min(total.saturating_sub(1)) }
            else { 0 }
        }
    };

    stage.min(total.saturating_sub(1))
}

fn research_workflow_specs(
    kind: &str,
) -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    match kind {
        "deep_learning" => vec![
            ("scope", "界定任务", "明确数据、指标、算力和停止条件", "plan"),
            ("baseline", "建立基线", "先跑最小 baseline，确认流程可复现", "evidence"),
            ("train_monitor", "训练与监控", "记录 epoch、loss、日志和 checkpoint", "execute"),
            ("debug_or_checkpoint", "调参与回退", "检查失败原因、资源与训练配置", "branch"),
            ("evaluate", "评估泛化", "验证集/测试集评估并做误差分析", "validate"),
            ("report", "整理结论", "输出图表、局限和下一步实验", "report"),
        ],
        "theory" => vec![
            ("formalize", "形式化问题", "定义对象、条件和目标命题", "plan"),
            ("known_results", "已有结论", "查找相关定理、反例和边界条件", "evidence"),
            ("proof_path", "证明路径", "拆分引理并检查依赖关系", "reason"),
            ("counterexample", "反例搜索", "尝试构造反例或极端情况", "branch"),
            ("verify", "逐步核查", "检查量词、假设遗漏和推理跳步", "validate"),
            ("writeup", "证明写作", "形成可审阅证明与讨论", "report"),
        ],
        "experimental_design" => vec![
            ("question", "定义实验问题", "明确机制、变量、终点和成功标准", "plan"),
            ("background", "背景证据", "整理先验文献、对照组和可测指标", "evidence"),
            ("protocol", "设计实验方案", "确定材料、步骤、剂量、时间窗和样本量", "reason"),
            ("pilot", "预实验", "先跑最小闭环，检查可操作性与关键风险", "execute"),
            ("analysis", "结果判读", "检查统计方法、偏差来源和可重复性", "validate"),
            ("report", "输出方案与结论", "给出实验卡、风险点和下一步建议", "report"),
        ],
        "literature_review" => vec![
            ("question", "综述问题", "界定检索范围和评价维度", "plan"),
            ("search", "检索证据", "搜索论文、方法、数据集和评估指标", "evidence"),
            ("screen", "筛选质量", "排除弱证据并记录纳入/排除理由", "review"),
            ("synthesize", "综合脉络", "比较方法、共识、冲突和空白", "reason"),
            ("gap", "识别空白", "形成可检验问题或后续实验建议", "branch"),
            ("write", "输出综述", "生成引用、表格和结论", "report"),
        ],
        "simulation" => vec![
            ("model", "建立模型", "明确变量、边界、近似和守恒关系", "plan"),
            ("implementation", "实现仿真", "编写最小可运行模型", "execute"),
            ("sensitivity", "参数扫描", "多组参数、随机种子和稳定性检查", "branch"),
            ("compare", "对照验证", "与解析解、实验或基线比较", "validate"),
            ("uncertainty", "不确定性", "说明误差来源和适用范围", "review"),
            ("report", "报告结果", "输出图表、结论和复现实验说明", "report"),
        ],
        "data_analysis" => vec![
            ("inspect", "理解数据", "读取字段、缺失、分布和任务目标", "plan"),
            ("clean", "清洗与特征", "预处理、标准化和特征构造", "execute"),
            ("baseline", "基线分析", "简单模型、统计检验和可视化", "evidence"),
            ("iterate", "迭代实验", "调整方法并比较指标", "branch"),
            ("validate", "验证结论", "稳健性、误差和泄漏检查", "validate"),
            ("artifact", "产出材料", "脚本、图表、表格和结论摘要", "report"),
        ],
        _ => vec![
            ("clarify", "澄清目标", "明确问题、约束、资源和成功标准", "plan"),
            ("evidence", "收集证据", "读取资料、运行工具或查找背景", "evidence"),
            ("method", "设计方法", "选择可执行路径和验证指标", "reason"),
            ("execute", "执行任务", "分步运行并记录中间结果", "execute"),
            ("self_review", "自我审查", "检查假设、失败模式和证据强度", "review"),
            ("deliver", "交付结论", "输出结果、文件和下一步建议", "report"),
        ],
    }
}

fn research_workflow_rationale(kind: &str) -> &'static str {
    match kind {
        "deep_learning" => "该课题可能包含长时间训练，因此流程需要显式展示训练监控、checkpoint、失败回退和多轮调参。",
        "theory" => "理论问题不应强行套实验流程，需要以形式化、引理、反例和逐步审查为主。",
        "experimental_design" => "实验设计类课题需要把对照、变量、样本量、预实验、偏差控制和结果判读显式化。",
        "literature_review" => "综述类任务以证据检索、筛选标准、综合脉络和空白识别为核心。",
        "simulation" => "仿真类任务需要明确模型假设、参数扫描、对照验证和不确定性边界。",
        "data_analysis" => "数据分析任务应先理解数据，再清洗、建基线、迭代、验证和产出可复现材料。",
        _ => "当前课题类型尚不明确，采用自适应研究流程，并根据工具调用和产出动态调整。",
    }
}

fn research_review_checks(kind: &str) -> Vec<String> {
    let mut checks = vec![
        "目标是否可验证，成功标准是否明确".to_string(),
        "关键假设是否被记录，是否存在反例或失败模式".to_string(),
        "工具输出是否支持当前结论，而不是只靠模型生成".to_string(),
    ];
    if kind == "deep_learning" {
        checks.push("长训练任务是否有日志、checkpoint、资源监控和早停策略".to_string());
    }
    if kind == "experimental_design" {
        checks.push("是否明确了阴阳性对照、重复次数、样本量依据和潜在偏差来源".to_string());
    }
    checks.push("最终产物是否可复现：代码、数据路径、参数和环境说明是否完整".to_string());
    checks
}

#[derive(Debug, Clone)]
struct ReviewStatusEntry {
    path: String,
    status: String,
    untracked: bool,
}

fn build_git_payload(
    base_dir: &Path,
    workspace_root: &str,
    include_diff: bool,
    include_graph: bool,
) -> Result<WebGitPayload> {
    let workspace = canonical_workspace_dir_from(base_dir, workspace_root)?;
    ensure_git_repository(&workspace)?;
    let status = read_git_status(&workspace)?;
    let branches = read_git_branches(&workspace)?;
    let commits = read_git_commits(&workspace)?;
    let staged_diff = if include_diff { read_git_diff(&workspace, true).ok() } else { None };
    let working_diff = if include_diff { read_git_diff(&workspace, false).ok() } else { None };
    let graph = if include_graph { read_git_graph(&workspace).unwrap_or_default() } else { Vec::new() };

    Ok(WebGitPayload {
        available: true,
        repository_root: display_workspace_path(&workspace),
        status: Some(status),
        branches,
        commits,
        graph,
        staged_diff,
        working_diff,
        error: None,
    })
}

fn build_extensions_payload() -> Result<WebExtensionsPayload> {
    let mut items = Vec::new();
    let plugin_roots = discover_plugin_roots();

    for root in plugin_roots {
        if !root.exists() {
            continue;
        }
        for entry in fs::read_dir(&root)
            .map_err(|err| anyhow!("failed to read plugin root '{}': {}", root.display(), err))?
            .filter_map(|entry| entry.ok())
        {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let id = entry.file_name().to_string_lossy().to_string();
            let version = fs::read_dir(&path)
                .ok()
                .and_then(|dirs| {
                    dirs.filter_map(|item| item.ok())
                        .find(|item| item.path().is_dir())
                        .map(|item| item.file_name().to_string_lossy().to_string())
                })
                .unwrap_or_else(|| "unknown".to_string());

            items.push(WebExtensionItem {
                id: id.clone(),
                title: humanize_extension_name(&id),
                source: root
                    .file_name()
                    .map(|value| value.to_string_lossy().to_string())
                    .unwrap_or_else(|| "plugins".to_string()),
                version,
                description: extension_description(&id),
            });
        }
    }

    if let Some(skill_root) = discover_skill_root() {
        for entry in fs::read_dir(&skill_root)
            .map_err(|err| anyhow!("failed to read skills root '{}': {}", skill_root.display(), err))?
            .filter_map(|entry| entry.ok())
        {
            if !entry.path().is_dir() {
                continue;
            }
            let id = entry.file_name().to_string_lossy().to_string();
            items.push(WebExtensionItem {
                id: format!("skill-{}", id),
                title: humanize_extension_name(&id),
                source: "skills".to_string(),
                version: "system".to_string(),
                description: "Installed system skill".to_string(),
            });
        }
    }

    items.sort_by(|left, right| left.title.to_lowercase().cmp(&right.title.to_lowercase()));
    Ok(WebExtensionsPayload { items, error: None })
}

fn discover_plugin_roots() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(codex_home) = std::env::var("CODEX_HOME") {
        candidates.push(PathBuf::from(codex_home).join("plugins").join("cache"));
    }
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".codex").join("plugins").join("cache"));
    }
    if let Some(data_local) = dirs::data_local_dir() {
        candidates.push(data_local.join("Codex").join("plugins").join("cache"));
    }

    let mut roots = Vec::new();
    for root in candidates {
        if !root.exists() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&root) {
            for entry in entries.filter_map(|entry| entry.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    roots.push(path);
                }
            }
        }
    }
    roots
}

fn discover_skill_root() -> Option<PathBuf> {
    if let Ok(codex_home) = std::env::var("CODEX_HOME") {
        let path = PathBuf::from(codex_home).join("skills").join(".system");
        if path.exists() {
            return Some(path);
        }
    }
    if let Some(home) = dirs::home_dir() {
        let path = home.join(".codex").join("skills").join(".system");
        if path.exists() {
            return Some(path);
        }
    }
    if let Some(data_local) = dirs::data_local_dir() {
        let path = data_local.join("Codex").join("skills").join(".system");
        if path.exists() {
            return Some(path);
        }
    }
    None
}

fn humanize_extension_name(id: &str) -> String {
    id.split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn extension_description(id: &str) -> String {
    match id {
        "browser" => "In-app browser control and page automation.".to_string(),
        "chrome" => "Chrome session automation with existing profile state.".to_string(),
        "computer-use" => "Desktop UI automation for Windows apps.".to_string(),
        "latex" => "LaTeX compile, doctor, and runtime tooling.".to_string(),
        "documents" => "Word and document generation workflows.".to_string(),
        "pdf" => "PDF inspection, rendering, and generation.".to_string(),
        "presentations" => "Slide deck creation and editing.".to_string(),
        "spreadsheets" => "Spreadsheet creation, analysis, and export.".to_string(),
        _ => "Installed local plugin or skill.".to_string(),
    }
}

fn build_run_debug_payload(state: &WebAppState, workspace_root: &str) -> Result<WebRunDebugPayload> {
    let workspace = canonical_workspace_dir_from(state.host.base_dir(), workspace_root)?;
    let runtime = lock_runtime_settings(state)?.clone();
    let configs = detect_run_configs(&workspace, &runtime)?;
    let active = {
        let mut runtime = state
            .run_debug_runtime
            .lock()
            .map_err(|_| anyhow!("failed to lock run/debug runtime"))?;
        if let Some(session) = runtime.active.as_ref() {
            if !process_is_running(session.pid) {
                runtime.active = None;
            }
        }
        runtime
            .active
            .as_ref()
            .and_then(|session| hydrate_run_debug_session(session).ok())
    };

    Ok(WebRunDebugPayload {
        configs,
        active,
        error: None,
    })
}

fn build_terminal_payload(state: &WebAppState) -> Result<WebTerminalPayload> {
    let mut runtime = state
        .terminal_runtime
        .lock()
        .map_err(|_| anyhow!("failed to lock terminal runtime"))?;
    prune_terminal_sessions(&mut runtime);

    let sessions = runtime
        .sessions
        .iter()
        .map(|session| WebTerminalSession {
            id: session.id.clone(),
            title: session.title.clone(),
            cwd: session.cwd.clone(),
            created_at: session.created_at.clone(),
            command: session.command.clone(),
            status: session
                .status
                .lock()
                .map(|status| status.clone())
                .unwrap_or_else(|_| "unknown".to_string()),
            buffer: session
                .buffer
                .lock()
                .map(|buffer| decode_bytes(&buffer))
                .unwrap_or_default(),
        })
        .collect();

    Ok(WebTerminalPayload {
        sessions,
        active_id: runtime.active_id.clone(),
        error: None,
    })
}

fn prune_terminal_sessions(runtime: &mut TerminalRuntime) {
    runtime.sessions.retain(|session| {
        let mut keep = true;
        if let Ok(mut child) = session.child.lock() {
            match child.try_wait() {
                Ok(Some(_status)) => {
                    if let Ok(mut status) = session.status.lock() {
                        *status = "exited".to_string();
                    }
                    keep = false;
                }
                Ok(None) => {
                    if let Ok(mut status) = session.status.lock() {
                        *status = "running".to_string();
                    }
                }
                Err(_error) => {}
            }
        }
        keep
    });

    if runtime.active_id.as_ref().is_some_and(|active_id| {
        !runtime.sessions.iter().any(|session| &session.id == active_id)
    }) {
        runtime.active_id = runtime.sessions.last().map(|session| session.id.clone());
    }
}

async fn create_terminal_session(state: &WebAppState, workspace_root: &str) -> Result<String> {
    let workspace = canonical_workspace_dir_from(state.host.base_dir(), workspace_root)?;
    ensure_git_repository(&workspace)?;

    let mut command = Command::new("powershell.exe");
    command
        .current_dir(&workspace)
        .args(["-NoLogo", "-NoExit"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        command.creation_flags(0x08000000);
    }

    let mut child = command
        .spawn()
        .map_err(|err| anyhow!("failed to start terminal: {}", err))?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("terminal stdin unavailable"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("terminal stdout unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow!("terminal stderr unavailable"))?;

    let buffer = Arc::new(Mutex::new(Vec::new()));
    let stdout_buffer = Arc::clone(&buffer);
    let stderr_buffer = Arc::clone(&buffer);

    thread::spawn(move || {
        pipe_terminal_output(stdout, stdout_buffer);
    });
    thread::spawn(move || {
        pipe_terminal_output(stderr, stderr_buffer);
    });

    let mut stdin = stdin;
    stdin
        .write_all(
            b"$OutputEncoding = [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false); [Console]::InputEncoding = [System.Text.UTF8Encoding]::new($false); $env:PYTHONUTF8='1'; $env:PYTHONIOENCODING='utf-8'\r\n",
        )
        .map_err(|err| anyhow!("failed to initialize terminal encoding: {}", err))?;
    stdin
        .flush()
        .map_err(|err| anyhow!("failed to flush terminal initialization: {}", err))?;

    let mut runtime = state
        .terminal_runtime
        .lock()
        .map_err(|_| anyhow!("failed to lock terminal runtime"))?;
    runtime.next_id = runtime.next_id.saturating_add(1);
    let next_number = runtime.next_id;
    let id = format!("terminal-{}", next_number);
    runtime.active_id = Some(id.clone());
    runtime.sessions.push(TerminalSessionRuntime {
        id: id.clone(),
        title: format!("Terminal {}", next_number),
        cwd: workspace.display().to_string(),
        created_at: Local::now().format("%Y-%m-%d %H:%M").to_string(),
        command: "powershell.exe -NoLogo -NoExit".to_string(),
        status: Arc::new(Mutex::new("running".to_string())),
        buffer,
        stdin: Arc::new(Mutex::new(stdin)),
        child: Arc::new(Mutex::new(child)),
    });

    Ok(id)
}

fn pipe_terminal_output<R>(mut reader: R, buffer: Arc<Mutex<Vec<u8>>>)
where
    R: Read,
{
    let mut chunk = [0_u8; 4096];
    loop {
        let read = match reader.read(&mut chunk) {
            Ok(read) => read,
            Err(_) => return,
        };
        if read == 0 {
            return;
        }
        if let Ok(mut content) = buffer.lock() {
            content.extend_from_slice(&chunk[..read]);
            if content.starts_with(&[0xEF, 0xBB, 0xBF]) {
                content.drain(..3);
            }
            if content.len() > 64_000 {
                let drain_to = content.len().saturating_sub(64_000);
                content.drain(..drain_to);
            }
        }
    }
}

async fn write_terminal_input(state: &WebAppState, terminal_id: &str, input: &str) -> Result<()> {
    let stdin = {
        let runtime = state
            .terminal_runtime
            .lock()
            .map_err(|_| anyhow!("failed to lock terminal runtime"))?;
        let session = runtime
            .sessions
            .iter()
            .find(|session| session.id == terminal_id)
            .ok_or_else(|| anyhow!("terminal not found: {}", terminal_id))?;
        Arc::clone(&session.stdin)
    };

    let mut stdin = stdin
        .lock()
        .map_err(|_| anyhow!("failed to lock terminal stdin"))?;
    stdin
        .write_all(format!("{}\r\n", input).as_bytes())
        .map_err(|err| anyhow!("failed to write terminal input: {}", err))?;
    stdin
        .flush()
        .map_err(|err| anyhow!("failed to flush terminal input: {}", err))?;
    Ok(())
}

async fn close_terminal_session(state: &WebAppState, terminal_id: &str) -> Result<()> {
    let session = {
        let mut runtime = state
            .terminal_runtime
            .lock()
            .map_err(|_| anyhow!("failed to lock terminal runtime"))?;
        let index = runtime
            .sessions
            .iter()
            .position(|session| session.id == terminal_id)
            .ok_or_else(|| anyhow!("terminal not found: {}", terminal_id))?;
        let session = runtime.sessions.remove(index);
        runtime.active_id = runtime.sessions.last().map(|item| item.id.clone());
        session
    };

    if let Ok(mut child) = session.child.lock() {
        let _ = child.kill();
    }
    Ok(())
}

fn detect_run_configs(workspace: &Path, runtime: &RuntimeSettings) -> Result<Vec<WebRunConfig>> {
    let mut configs = Vec::new();
    let tools = &runtime.toolchains;
    if workspace.join("Cargo.toml").exists() {
        let cargo = tool_value(tools, "cargo", "cargo");
        configs.push(make_run_config("cargo-web", "Web Workspace", &format!("{} run -- --web", shell_quote(&cargo)), &cargo, "Rust", "rust", "launch", Some("Cargo.toml"), &["cargo"], tools));
        configs.push(make_run_config("cargo-tui", "Terminal UI", &format!("{} run -- --tui", shell_quote(&cargo)), &cargo, "Rust", "rust", "launch", Some("Cargo.toml"), &["cargo"], tools));
    }
    if workspace.join("package.json").exists() {
        let npm = tool_value(tools, "npm", "npm");
        configs.push(make_run_config("npm-dev", "Frontend Dev Server", &format!("{} run dev", shell_quote(&npm)), &npm, "Node", "javascript", "task", Some("package.json"), &["npm"], tools));
        configs.push(make_run_config("npm-start", "Node App", &format!("{} start", shell_quote(&npm)), &npm, "Node", "javascript", "launch", Some("package.json"), &["npm"], tools));
    }
    if workspace.join("pyproject.toml").exists() || workspace.join("requirements.txt").exists() {
        let python = tool_value(tools, "python", "python");
        if workspace.join("main.py").exists() {
            configs.push(make_run_config("python-main", "Python main.py", &format!("{} main.py", shell_quote(&python)), &python, "Python", "python", "launch", Some("main.py"), &["python"], tools));
        }
        if workspace.join("app.py").exists() {
            configs.push(make_run_config("python-app", "Python app.py", &format!("{} app.py", shell_quote(&python)), &python, "Python", "python", "launch", Some("app.py"), &["python"], tools));
        }
        if workspace.join("manage.py").exists() {
            configs.push(make_run_config("python-manage-runserver", "Django runserver", &format!("{} manage.py runserver", shell_quote(&python)), &python, "Python", "python", "launch", Some("manage.py"), &["python"], tools));
        }
    }
    add_multilanguage_run_configs(workspace, tools, &mut configs);
    Ok(configs)
}

fn add_multilanguage_run_configs(
    workspace: &Path,
    tools: &BTreeMap<String, String>,
    configs: &mut Vec<WebRunConfig>,
) {
    for path in find_workspace_files_with_extensions(workspace, &["java"], 16) {
        if let Some(relative) = relative_workspace_path(workspace, &path) {
            let class_name = path.file_stem().and_then(|value| value.to_str()).unwrap_or("Main");
            let class_dir = Path::new(&relative)
                .parent()
                .map(|value| value.to_string_lossy().replace('\\', "/"))
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| ".".to_string());
            let javac = tool_value(tools, "javac", "javac");
            let java = tool_value(tools, "java", "java");
            let command = format!(
                "{} {} && {} -cp {} {}",
                shell_quote(&javac),
                shell_quote(&relative),
                shell_quote(&java),
                shell_quote(&class_dir),
                shell_quote(&class_name),
            );
            configs.push(make_run_config(
                &format!("java-{}", slugify_path(&relative)),
                &format!("Java {}", class_name),
                &command,
                &java,
                "Java",
                "java",
                "launch",
                Some(&relative),
                &["javac", "java"],
                tools,
            ));
        }
    }

    for path in find_workspace_files_with_extensions(workspace, &["c"], 16) {
        if let Some(relative) = relative_workspace_path(workspace, &path) {
            let output = output_name_for(&relative);
            let compiler = tool_value(tools, "c", "gcc");
            let output_path = format!(".tokitai-run/{}", output);
            let command = format!(
                "{} && {} {} -o {} && {}",
                ensure_run_dir_command(),
                shell_quote(&compiler),
                shell_quote(&relative),
                shell_quote(&output_path),
                shell_quote(&output_path),
            );
            configs.push(make_run_config(&format!("c-{}", slugify_path(&relative)), &format!("C {}", relative), &command, &compiler, "C/C++", "c", "launch", Some(&relative), &["c"], tools));
        }
    }

    for path in find_workspace_files_with_extensions(workspace, &["cpp", "cc", "cxx"], 16) {
        if let Some(relative) = relative_workspace_path(workspace, &path) {
            let output = output_name_for(&relative);
            let compiler = tool_value(tools, "cpp", "g++");
            let output_path = format!(".tokitai-run/{}", output);
            let command = format!(
                "{} && {} {} -std=c++17 -o {} && {}",
                ensure_run_dir_command(),
                shell_quote(&compiler),
                shell_quote(&relative),
                shell_quote(&output_path),
                shell_quote(&output_path),
            );
            configs.push(make_run_config(&format!("cpp-{}", slugify_path(&relative)), &format!("C++ {}", relative), &command, &compiler, "C/C++", "cpp", "launch", Some(&relative), &["cpp"], tools));
        }
    }

    if workspace.join("go.mod").exists() {
        let go = tool_value(tools, "go", "go");
        configs.push(make_run_config("go-run-module", "Go module", &format!("{} run .", shell_quote(&go)), &go, "Go", "go", "launch", Some("go.mod"), &["go"], tools));
    }
    for path in find_workspace_files_with_extensions(workspace, &["go"], 16) {
        if let Some(relative) = relative_workspace_path(workspace, &path) {
            let go = tool_value(tools, "go", "go");
            configs.push(make_run_config(&format!("go-{}", slugify_path(&relative)), &format!("Go {}", relative), &format!("{} run {}", shell_quote(&go), shell_quote(&relative)), &go, "Go", "go", "launch", Some(&relative), &["go"], tools));
        }
    }

    let csproj_files = find_workspace_files_with_extensions(workspace, &["csproj"], 8);
    if !csproj_files.is_empty() {
        let project_hint = csproj_files
            .first()
            .and_then(|path| relative_workspace_path(workspace, path));
        let dotnet = tool_value(tools, "dotnet", "dotnet");
        configs.push(make_run_config("dotnet-run", ".NET project", &format!("{} run", shell_quote(&dotnet)), &dotnet, "C#", "csharp", "launch", project_hint.as_deref(), &["dotnet"], tools));
    }
    for path in find_workspace_files_with_extensions(workspace, &["cs"], 8) {
        if let Some(relative) = relative_workspace_path(workspace, &path) {
            let dotnet = tool_value(tools, "dotnet", "dotnet");
            configs.push(make_run_config(&format!("csharp-{}", slugify_path(&relative)), &format!("C# {}", relative), &format!("{} run", shell_quote(&dotnet)), &dotnet, "C#", "csharp", "launch", Some(&relative), &["dotnet"], tools));
        }
    }

    for path in find_workspace_files_with_extensions(workspace, &["jl"], 16) {
        if let Some(relative) = relative_workspace_path(workspace, &path) {
            let julia = tool_value(tools, "julia", "julia");
            configs.push(make_run_config(&format!("julia-{}", slugify_path(&relative)), &format!("Julia {}", relative), &format!("{} {}", shell_quote(&julia), shell_quote(&relative)), &julia, "Julia", "julia", "launch", Some(&relative), &["julia"], tools));
        }
    }

    for path in find_workspace_files_with_extensions(workspace, &["r"], 16) {
        if let Some(relative) = relative_workspace_path(workspace, &path) {
            let rscript = tool_value(tools, "rscript", "Rscript");
            configs.push(make_run_config(&format!("r-{}", slugify_path(&relative)), &format!("R {}", relative), &format!("{} {}", shell_quote(&rscript), shell_quote(&relative)), &rscript, "R", "r", "launch", Some(&relative), &["rscript"], tools));
        }
    }

    for path in find_workspace_files_with_extensions(workspace, &["tex"], 16) {
        if let Some(relative) = relative_workspace_path(workspace, &path) {
            let has_tectonic = tool_is_available(tools, "tectonic");
            let has_pdflatex = tool_is_available(tools, "pdflatex");
            let runtime_executable = if has_tectonic {
                tool_value(tools, "tectonic", "tectonic")
            } else {
                tool_value(tools, "pdflatex", "pdflatex")
            };
            let command = format!("{} {}", shell_quote(&runtime_executable), shell_quote(&relative));
            let mut config = make_run_config(&format!("latex-{}", slugify_path(&relative)), &format!("LaTeX {}", relative), &command, &runtime_executable, "LaTeX", "latex", "task", Some(&relative), &[], tools);
            config.available = has_tectonic || has_pdflatex;
            config.missing = if config.available { Vec::new() } else { vec!["tectonic or pdflatex".to_string()] };
            config.detail = if has_tectonic {
                "Ready: tectonic".to_string()
            } else if has_pdflatex {
                "Ready: pdflatex".to_string()
            } else {
                "Missing: tectonic or pdflatex".to_string()
            };
            configs.push(config);
        }
    }

    for path in find_workspace_files_with_extensions(workspace, &["md", "markdown"], 16) {
        if let Some(relative) = relative_workspace_path(workspace, &path) {
            configs.push(make_run_config(&format!("markdown-preview-{}", slugify_path(&relative)), &format!("Preview {}", relative), &format!("preview-markdown {}", shell_quote(&relative)), "preview-markdown", "Markdown", "markdown", "preview", Some(&relative), &[], tools));
        }
    }
}

fn make_run_config(
    id: &str,
    title: &str,
    command: &str,
    runtime_executable: &str,
    category: &str,
    language: &str,
    task_type: &str,
    file_hint: Option<&str>,
    required_tools: &[&str],
    toolchains: &BTreeMap<String, String>,
) -> WebRunConfig {
    let missing_dependencies = required_tools
        .iter()
        .filter_map(|tool| missing_dependency(toolchains, tool))
        .collect::<Vec<_>>();
    let missing = missing_dependencies
        .iter()
        .map(|item| item.executable.clone())
        .collect::<Vec<_>>();
    WebRunConfig {
        id: id.to_string(),
        title: title.to_string(),
        command: command.to_string(),
        runtime_executable: runtime_executable.to_string(),
        category: category.to_string(),
        file_hint: file_hint.map(|value| value.to_string()),
        language: language.to_string(),
        task_type: task_type.to_string(),
        available: missing.is_empty(),
        missing: missing.clone(),
        missing_dependencies: missing_dependencies.clone(),
        detail: if missing_dependencies.is_empty() {
            "Ready".to_string()
        } else {
            format_missing_dependency_detail(&missing_dependencies)
        },
        task: json!({
            "label": title,
            "type": "shell",
            "command": command,
            "group": if task_type == "launch" { "build" } else { task_type },
            "problemMatcher": [],
            "presentation": {
                "reveal": "always",
                "panel": "dedicated",
            },
        }),
        launch: json!({
            "version": "0.2.0",
            "type": language,
            "request": "launch",
            "name": title,
            "program": file_hint.unwrap_or(""),
            "runtimeExecutable": runtime_executable,
            "command": command,
            "preLaunchTask": if task_type == "launch" { Some(format!("run {}", title)) } else { None::<String> },
        }),
    }
}

fn tool_value(toolchains: &BTreeMap<String, String>, key: &str, fallback: &str) -> String {
    toolchains
        .get(key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| resolve_toolchain_value(key, fallback))
        .unwrap_or_else(|| default_toolchain_command(key))
}

fn tool_is_available(toolchains: &BTreeMap<String, String>, key: &str) -> bool {
    let value = tool_value(toolchains, key, key);
    command_is_available(&value)
}

fn missing_dependency(toolchains: &BTreeMap<String, String>, key: &str) -> Option<WebMissingDependency> {
    if tool_is_available(toolchains, key) {
        return None;
    }
    let configured = tool_value(toolchains, key, key);
    let executable = executable_name_for_hint(&configured, key);
    Some(WebMissingDependency {
        key: key.to_string(),
        executable,
        configured,
    })
}

fn executable_name_for_hint(configured: &str, key: &str) -> String {
    let trimmed = configured.trim();
    if trimmed.is_empty() {
        return default_toolchain_command(key);
    }
    Path::new(trimmed)
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.to_string())
        .unwrap_or_else(|| trimmed.to_string())
}

fn format_missing_dependency_detail(items: &[WebMissingDependency]) -> String {
    let parts = items
        .iter()
        .map(|item| {
            if item.configured.trim().is_empty() || item.configured == item.executable {
                item.executable.clone()
            } else {
                format!("{} (configured as {})", item.executable, item.configured)
            }
        })
        .collect::<Vec<_>>();
    match parts.len() {
        0 => "Ready".to_string(),
        1 => format!("Missing executable: {}", parts[0]),
        _ => format!("Missing executables: {}", parts.join(", ")),
    }
}

fn shell_quote(value: &str) -> String {
    let escaped = value.replace('"', "\"\"");
    format!("\"{}\"", escaped)
}

fn find_workspace_files_with_extensions(workspace: &Path, extensions: &[&str], limit: usize) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let wanted = extensions
        .iter()
        .map(|value| value.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let mut stack = vec![workspace.to_path_buf()];

    while let Some(dir) = stack.pop() {
        if files.len() >= limit {
            break;
        }
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if files.len() >= limit {
                break;
            }
            let path = entry.path();
            let name = path.file_name().and_then(|value| value.to_str()).unwrap_or("");
            if should_skip_workspace_entry(name) {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let extension = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if wanted.contains(&extension) {
                files.push(path);
            }
        }
    }

    files.sort();
    files
}

fn relative_workspace_path(workspace: &Path, path: &Path) -> Option<String> {
    path.strip_prefix(workspace)
        .ok()
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
}

fn slugify_path(path: &str) -> String {
    path.chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch.to_ascii_lowercase() } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn output_name_for(relative: &str) -> String {
    let stem = Path::new(relative)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("program");
    if cfg!(windows) {
        format!("{}.exe", stem)
    } else {
        stem.to_string()
    }
}

fn ensure_run_dir_command() -> &'static str {
    if cfg!(windows) {
        "if not exist .tokitai-run mkdir .tokitai-run"
    } else {
        "mkdir -p .tokitai-run"
    }
}

fn process_is_running(pid: u32) -> bool {
    Command::new("tasklist")
        .args(["/FI", &format!("PID eq {}", pid)])
        .output()
        .map(|output| {
            output.status.success() && decode_bytes(&output.stdout).contains(&pid.to_string())
        })
        .unwrap_or(false)
}

fn run_debug_action(
    state: &WebAppState,
    workspace_root: &str,
    payload: &RunDebugActionRequest,
) -> Result<()> {
    let workspace = canonical_workspace_dir_from(state.host.base_dir(), workspace_root)?;
    let runtime_settings = lock_runtime_settings(state)?.clone();
    let configs = detect_run_configs(&workspace, &runtime_settings)?;
    let mut runtime = state
        .run_debug_runtime
        .lock()
        .map_err(|_| anyhow!("failed to lock run/debug runtime"))?;

    match payload.action.trim() {
        "start" => {
            let config_id = payload
                .config_id
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| anyhow!("start action requires config_id"))?;
            let config = configs
                .iter()
                .find(|config| config.id == config_id)
                .ok_or_else(|| anyhow!("run config not found: {}", config_id))?;
            if !config.available {
                let detail = if config.missing_dependencies.is_empty() {
                    if config.missing.is_empty() {
                        "Missing runtime dependency".to_string()
                    } else {
                        format!("Missing executable: {}", config.missing.join(", "))
                    }
                } else {
                    format_missing_dependency_detail(&config.missing_dependencies)
                };
                return Err(anyhow!("cannot start '{}'; {}", config.title, detail));
            }
            if config.task_type == "preview" {
                return Err(anyhow!("preview tasks are handled inside the editor"));
            }
            if let Some(session) = runtime.active.take() {
                stop_run_debug_session(&session);
            }
            runtime.active = Some(start_run_debug_session(state.host.as_ref(), &workspace, config)?);
            Ok(())
        }
        "stop" => {
            if let Some(session) = runtime.active.take() {
                stop_run_debug_session(&session);
            }
            Ok(())
        }
        "refresh" => Ok(()),
        other => Err(anyhow!("unsupported run/debug action: {}", other)),
    }
}

fn start_run_debug_session(
    host: &WebHostConfig,
    workspace: &Path,
    config: &WebRunConfig,
) -> Result<RunDebugSessionRuntime> {
    let logs_dir = host.workspace_run_debug_dir(workspace);
    fs::create_dir_all(&logs_dir)
        .map_err(|err| anyhow!("failed to create run/debug log dir '{}': {}", logs_dir.display(), err))?;

    let timestamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let stdout_path = logs_dir.join(format!("{}-stdout-{}.log", config.id, timestamp));
    let stderr_path = logs_dir.join(format!("{}-stderr-{}.log", config.id, timestamp));
    let stdout_file = File::create(&stdout_path)
        .map_err(|err| anyhow!("failed to create stdout log '{}': {}", stdout_path.display(), err))?;
    let stderr_file = File::create(&stderr_path)
        .map_err(|err| anyhow!("failed to create stderr log '{}': {}", stderr_path.display(), err))?;

    let mut command = Command::new("cmd");
    command
        .current_dir(workspace)
        .args(["/C", &config.command])
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));

    #[cfg(windows)]
    {
        command.creation_flags(0x08000000);
    }

    let child = command
        .spawn()
        .map_err(|err| anyhow!("failed to start '{}': {}", config.command, err))?;

    Ok(RunDebugSessionRuntime {
        config_id: config.id.clone(),
        title: config.title.clone(),
        pid: child.id(),
        stdout_path,
        stderr_path,
        started_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        command: config.command.clone(),
        cwd: workspace.to_path_buf(),
    })
}

fn stop_run_debug_session(session: &RunDebugSessionRuntime) {
    let _ = Command::new("taskkill")
        .args(["/PID", &session.pid.to_string(), "/T", "/F"])
        .output();
}

fn hydrate_run_debug_session(session: &RunDebugSessionRuntime) -> Result<WebRunSession> {
    Ok(WebRunSession {
        config_id: session.config_id.clone(),
        title: session.title.clone(),
        pid: session.pid,
        started_at: session.started_at.clone(),
        stdout_tail: read_log_tail(&session.stdout_path, 60),
        stderr_tail: read_log_tail(&session.stderr_path, 60),
        command: session.command.clone(),
        cwd: session.cwd.to_string_lossy().to_string(),
    })
}

fn read_log_tail(path: &Path, max_lines: usize) -> String {
    read_text_file(path)
        .map(|content| {
            let lines: Vec<&str> = content.lines().collect();
            let start = lines.len().saturating_sub(max_lines);
            lines[start..].join("\n")
        })
        .unwrap_or_default()
}

fn read_git_status(workspace: &Path) -> Result<WebGitStatus> {
    let head_branch = run_git_capture(
        workspace,
        &["branch", "--show-current"],
        "read current branch",
    )?
    .trim()
    .to_string();
    let branch = if head_branch.is_empty() {
        "detached".to_string()
    } else {
        head_branch
    };

    let upstream_raw =
        run_git_capture_allow_failure(workspace, &["rev-parse", "--abbrev-ref", "@{upstream}"])?;
    let upstream = upstream_raw
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());

    let mut ahead = 0_u32;
    let mut behind = 0_u32;
    if let Some(ref upstream_name) = upstream {
        if let Some(counts) = run_git_capture_allow_failure(
            workspace,
            &[
                "rev-list",
                "--left-right",
                "--count",
                &format!("{}...HEAD", upstream_name),
            ],
        )? {
            let mut parts = counts.split_whitespace();
            behind = parts.next().and_then(|value| value.parse().ok()).unwrap_or(0);
            ahead = parts.next().and_then(|value| value.parse().ok()).unwrap_or(0);
        }
    }

    let porcelain = run_git_capture(
        workspace,
        &["status", "--porcelain=v1", "--untracked-files=all"],
        "read git status",
    )?;
    let mut changed_files = BTreeMap::<String, WebGitChangedFile>::new();
    let mut has_conflicts = false;
    let mut has_staged_changes = false;
    let mut has_unstaged_changes = false;
    let mut has_untracked_files = false;

    for raw_line in porcelain.lines() {
        if raw_line.len() < 3 {
            continue;
        }
        let code = &raw_line[..2];
        let raw_path = raw_line[3..].trim();
        if raw_path.is_empty() {
            continue;
        }

        let (original_path, path) = if code.contains('R') || code.contains('C') {
            let mut split = raw_path.splitn(2, " -> ");
            let old = split.next().map(|value| value.trim().replace('\\', "/"));
            let new = split
                .next()
                .map(|value| value.trim().replace('\\', "/"))
                .unwrap_or_else(|| raw_path.replace('\\', "/"));
            (old.filter(|value| !value.is_empty()), new)
        } else {
            (None, raw_path.replace('\\', "/"))
        };

        let bytes = code.as_bytes();
        let staged_code = bytes.first().copied().unwrap_or(b' ');
        let unstaged_code = bytes.get(1).copied().unwrap_or(b' ');
        let conflicted = matches!(
            code,
            "DD" | "AU" | "UD" | "UA" | "DU" | "AA" | "UU"
        );
        let untracked = code == "??";
        let staged = staged_code != b' ' && staged_code != b'?' && staged_code != b'U';
        let unstaged = unstaged_code != b' ' && unstaged_code != b'?';

        has_conflicts |= conflicted;
        has_staged_changes |= staged || conflicted;
        has_unstaged_changes |= unstaged || conflicted;
        has_untracked_files |= untracked;

        changed_files.insert(
            path.clone(),
            WebGitChangedFile {
                path,
                original_path,
                change_type: classify_git_change_type(code),
                staged,
                unstaged,
                untracked,
                conflicted,
            },
        );
    }

    let change_count = changed_files.len();
    let repository_clean = change_count == 0;
    let mut parts = vec![format!("branch {}", branch)];
    if let Some(ref upstream_name) = upstream {
        if ahead > 0 || behind > 0 {
            parts.push(format!("tracking {} (ahead {}, behind {})", upstream_name, ahead, behind));
        } else {
            parts.push(format!("tracking {}", upstream_name));
        }
    }
    if has_staged_changes {
        parts.push("staged changes".to_string());
    }
    if has_unstaged_changes {
        parts.push("unstaged changes".to_string());
    }
    if has_untracked_files {
        parts.push("untracked files".to_string());
    }
    if has_conflicts {
        parts.push("conflicts".to_string());
    }
    if repository_clean {
        parts.push("working tree clean".to_string());
    } else {
        parts.push(format!("{} file(s) changed", change_count));
    }

    Ok(WebGitStatus {
        branch,
        upstream,
        ahead,
        behind,
        has_conflicts,
        has_staged_changes,
        has_unstaged_changes,
        has_untracked_files,
        repository_clean,
        summary: parts.join(" 路 "),
        changed_files: changed_files.into_values().collect(),
    })
}

fn read_git_branches(workspace: &Path) -> Result<Vec<WebGitBranch>> {
    let output = run_git_capture(
        workspace,
        &[
            "for-each-ref",
            "--sort=-committerdate",
            "--format=%(refname)%x1f%(refname:short)%x1f%(upstream:short)%x1f%(if)%(HEAD)%(then)true%(else)false%(end)%x1f%(committerdate:relative)",
            "refs/heads",
            "refs/remotes",
        ],
        "read git branches",
    )?;

    let mut branches = Vec::new();
    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\u{1f}').collect();
        if parts.len() < 2 {
            continue;
        }
        let refname = parts.first().copied().unwrap_or("").trim().to_string();
        let name = parts.get(1).copied().unwrap_or("").trim().to_string();
        if name.is_empty() {
            continue;
        }
        branches.push(WebGitBranch {
            is_remote: refname.starts_with("refs/remotes/"),
            is_current: parts.get(3).copied().unwrap_or("false") == "true",
            upstream: parts
                .get(2)
                .copied()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| value.to_string()),
            last_updated: parts
                .get(4)
                .copied()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| value.to_string()),
            name,
        });
    }
    Ok(branches)
}

fn read_git_commits(workspace: &Path) -> Result<Vec<WebGitCommit>> {
    let output = run_git_capture(
        workspace,
        &[
            "log",
            "-n",
            "40",
            "--date=relative",
            "--pretty=format:%h%x1f%an%x1f%ae%x1f%s%x1f%cr",
        ],
        "read git history",
    )?;

    let mut commits = Vec::new();
    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\u{1f}').collect();
        if parts.len() < 5 {
            continue;
        }
        commits.push(WebGitCommit {
            hash: parts[0].to_string(),
            author: parts[1].to_string(),
            author_email: parts[2].to_string(),
            message: parts[3].to_string(),
            date: parts[4].to_string(),
        });
    }
    Ok(commits)
}

fn read_git_diff(workspace: &Path, staged: bool) -> Result<String> {
    let mut args = vec!["diff", "--no-ext-diff", "--patch", "--stat=160,120"];
    if staged {
        args.push("--cached");
    }
    args.push("--");
    run_git_capture(workspace, &args, "read git diff")
}

fn read_git_graph(workspace: &Path) -> Result<Vec<WebGitGraphRow>> {
    let output = Command::new("git")
        .current_dir(workspace)
        .args([
            "log",
            "--all",
            "--topo-order",
            "--decorate",
            "--date=relative",
            "--pretty=format:%h%x1f%H%x1f%P%x1f%d%x1f%s%x1f%cr%x1f%an",
            "-n",
            "40",
        ])
        .output()
        .map_err(|err| anyhow!("failed to read git graph: {}", err))?;

    if !output.status.success() {
        let stderr = decode_bytes(&output.stderr).trim().to_string();
        return Err(anyhow!("git log --graph failed: {}", stderr));
    }

    let stdout = decode_bytes(&output.stdout);
    let mut rows = Vec::new();
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\u{1f}').collect();
        if parts.len() < 7 {
            continue;
        }
        let parent_hashes = parts[2]
            .split_whitespace()
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.to_string())
            .collect::<Vec<_>>();
        let refs = parts[3]
            .trim()
            .trim_matches(['(', ')'])
            .split(',')
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
            .collect::<Vec<_>>();
        rows.push(WebGitGraphRow {
            hash: parts[0].to_string(),
            full_hash: parts[1].to_string(),
            parents: parent_hashes,
            refs,
            subject: parts[4].to_string(),
            relative_time: parts[5].to_string(),
            author: parts[6].to_string(),
        });
    }
    Ok(rows)
}

fn run_git_action(base_dir: &Path, workspace_root: &str, request: &GitActionRequest) -> Result<()> {
    let workspace = canonical_workspace_dir_from(base_dir, workspace_root)?;
    ensure_git_repository(&workspace)?;

    match request.action.trim() {
        "fetch" => run_git_command(&workspace, &["fetch", "--all", "--prune"]),
        "pull" => run_git_command(&workspace, &["pull", "--ff-only"]),
        "push" => run_git_command(&workspace, &["push"]),
        "refresh" => Ok(()),
        "checkout" => {
            let branch = request
                .branch
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| anyhow!("checkout action requires a branch"))?;
            run_git_command(&workspace, &["checkout", branch])
        }
        "create_branch" => {
            let branch = request
                .branch
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| anyhow!("create_branch action requires a branch"))?;
            if let Some(reference) = request.reference.as_deref().filter(|value| !value.trim().is_empty()) {
                run_git_command(&workspace, &["branch", branch, reference])
            } else {
                run_git_command(&workspace, &["branch", branch])
            }
        }
        "delete_branch" => {
            let branch = request
                .branch
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| anyhow!("delete_branch action requires a branch"))?;
            run_git_command(&workspace, &["branch", "-D", branch])
        }
        "stage_all" => run_git_command(&workspace, &["add", "-A"]),
        "unstage_all" => run_git_command(&workspace, &["reset", "HEAD", "--", "."]),
        "unstage_paths" => {
            let pathspecs = request.pathspecs.clone().unwrap_or_default();
            if pathspecs.is_empty() {
                return Err(anyhow!("unstage_paths action requires pathspecs"));
            }
            let owned_paths = sanitize_git_pathspecs(pathspecs)?;
            let mut refs = vec!["reset", "HEAD", "--"];
            for path in &owned_paths {
                refs.push(path.as_str());
            }
            run_git_command(&workspace, &refs)
        }
        "discard_paths" => {
            let pathspecs = request.pathspecs.clone().unwrap_or_default();
            if pathspecs.is_empty() {
                return Err(anyhow!("discard_paths action requires pathspecs"));
            }
            let owned_paths = sanitize_git_pathspecs(pathspecs)?;
            let mut tracked = Vec::new();
            let mut untracked = Vec::new();
            for path in &owned_paths {
                if git_path_is_tracked(&workspace, path)? {
                    tracked.push(path.as_str());
                } else {
                    untracked.push(path.as_str());
                }
            }
            if !tracked.is_empty() {
                let mut refs = vec!["restore", "--worktree", "--"];
                refs.extend(tracked);
                run_git_command(&workspace, &refs)?;
            }
            if !untracked.is_empty() {
                let mut refs = vec!["clean", "-fd", "--"];
                refs.extend(untracked);
                run_git_command(&workspace, &refs)?;
            }
            Ok(())
        }
        "commit" => {
            let message = request
                .message
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| anyhow!("commit action requires a message"))?;
            run_git_command(&workspace, &["commit", "-m", message])
        }
        "stage_paths" => {
            let pathspecs = request.pathspecs.clone().unwrap_or_default();
            if pathspecs.is_empty() {
                return Err(anyhow!("stage_paths action requires pathspecs"));
            }
            let owned_paths = sanitize_git_pathspecs(pathspecs)?;
            let mut refs = vec!["add", "--"];
            for path in &owned_paths {
                refs.push(path.as_str());
            }
            run_git_command(&workspace, &refs)
        }
        other => Err(anyhow!("unsupported git action: {}", other)),
    }
}

fn run_git_command(workspace: &Path, args: &[&str]) -> Result<()> {
    let output = Command::new("git")
        .current_dir(workspace)
        .args(args)
        .output()
        .map_err(|err| anyhow!("failed to run git {}: {}", args.join(" "), err))?;

    if !output.status.success() {
        let stderr = decode_bytes(&output.stderr).trim().to_string();
        return Err(anyhow!("git {} failed: {}", args.join(" "), stderr));
    }

    Ok(())
}

fn run_git_capture(workspace: &Path, args: &[&str], context: &str) -> Result<String> {
    let output = Command::new("git")
        .current_dir(workspace)
        .args(args)
        .output()
        .map_err(|err| anyhow!("failed to {}: {}", context, err))?;

    if !output.status.success() {
        let stderr = decode_bytes(&output.stderr).trim().to_string();
        let stdout = decode_bytes(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        return Err(anyhow!("git {} failed: {}", args.join(" "), detail));
    }

    Ok(decode_bytes(&output.stdout).replace('\u{feff}', ""))
}

fn run_git_capture_allow_failure(workspace: &Path, args: &[&str]) -> Result<Option<String>> {
    let output = Command::new("git")
        .current_dir(workspace)
        .args(args)
        .output()
        .map_err(|err| anyhow!("failed to run git {}: {}", args.join(" "), err))?;

    if !output.status.success() {
        return Ok(None);
    }

    Ok(Some(decode_bytes(&output.stdout).replace('\u{feff}', "")))
}

fn sanitize_git_pathspecs(pathspecs: Vec<String>) -> Result<Vec<String>> {
    let mut sanitized = Vec::new();
    for path in pathspecs {
        sanitized.push(sanitize_review_path(&path)?);
    }
    Ok(sanitized)
}

fn git_path_is_tracked(workspace: &Path, path: &str) -> Result<bool> {
    let output = Command::new("git")
        .current_dir(workspace)
        .args(["ls-files", "--error-unmatch", "--", path])
        .output()
        .map_err(|err| anyhow!("failed to inspect git path '{}': {}", path, err))?;
    Ok(output.status.success())
}

fn classify_git_change_type(code: &str) -> String {
    let bytes = code.as_bytes();
    let left = bytes.first().copied().unwrap_or(b' ');
    let right = bytes.get(1).copied().unwrap_or(b' ');

    if code == "??" {
        "untracked".to_string()
    } else if matches!(code, "DD" | "AU" | "UD" | "UA" | "DU" | "AA" | "UU") {
        "conflicted".to_string()
    } else if left == b'D' || right == b'D' {
        "deleted".to_string()
    } else if left == b'A' || right == b'A' {
        "added".to_string()
    } else if left == b'R' || right == b'R' {
        "renamed".to_string()
    } else if left == b'C' || right == b'C' {
        "copied".to_string()
    } else if left == b'M' || right == b'M' || left == b'U' || right == b'U' {
        "modified".to_string()
    } else {
        "changed".to_string()
    }
}

fn try_build_review_payload(base_dir: &Path, workspace_root: &str, selected_paths: &[String]) -> Result<WebReviewPayload> {
    let workspace = canonical_workspace_dir_from(base_dir, workspace_root)?;
    ensure_git_repository(&workspace)?;
    let status_entries = read_review_status_entries(&workspace)?;
    let numstat = read_review_numstat(&workspace)?;
    let selected: BTreeSet<&str> = selected_paths.iter().map(|path| path.as_str()).collect();
    let mut status_map: HashMap<String, ReviewStatusEntry> = status_entries
        .into_iter()
        .map(|entry| (entry.path.clone(), entry))
        .collect();
    let mut files = Vec::new();
    let mut total_additions = 0_u32;
    let mut total_deletions = 0_u32;

    for path in selected_paths {
        let entry = status_map.remove(path).unwrap_or(ReviewStatusEntry {
            path: path.clone(),
            status: "modified".to_string(),
            untracked: false,
        });

        let (additions, deletions) = if let Some((adds, dels)) = numstat.get(&entry.path) {
            (*adds, *dels)
        } else if entry.untracked {
            (count_file_lines(&workspace.join(&entry.path)) as u32, 0)
        } else if selected.contains(entry.path.as_str()) {
            (0, 0)
        } else {
            (0, 0)
        };

        total_additions += additions;
        total_deletions += deletions;
        files.push(WebReviewFileSummary {
            path: entry.path,
            status: entry.status,
            additions,
            deletions,
        });
    }

    Ok(WebReviewPayload {
        available: true,
        total_files: files.len(),
        total_additions,
        total_deletions,
        files,
        error: None,
    })
}

fn collect_review_paths_for_current_turn(
    messages: &[MessageBlock],
    runtime_paths: &[String],
) -> Vec<String> {
    let mut touched_files = extract_recent_review_paths(messages);
    for path in runtime_paths {
        if !touched_files.iter().any(|existing| existing == path) {
            touched_files.push(path.clone());
        }
    }
    touched_files
}

fn build_review_file_detail(base_dir: &Path, workspace_root: &str, path: &str) -> Result<WebReviewFileDetail> {
    let workspace = canonical_workspace_dir_from(base_dir, workspace_root)?;
    ensure_git_repository(&workspace)?;
    let relative_path = sanitize_review_path(path)?;
    let summary = try_build_review_payload(base_dir, workspace_root, &[relative_path.clone()])?
        .files
        .into_iter()
        .find(|file| file.path == relative_path)
        .ok_or_else(|| anyhow!("review file not found: {}", relative_path))?;
    let preview_kind = workspace_preview_kind(&relative_path);
    let mime_type = workspace_mime_type(&relative_path);
    let is_binary = !matches!(preview_kind.as_str(), "text" | "markdown" | "svg");

    let hunks = if is_binary {
        Vec::new()
    } else if summary.status == "untracked" {
        build_untracked_file_hunks(&workspace.join(&relative_path))?
    } else {
        let diff_output = Command::new("git")
            .current_dir(&workspace)
            .args(["diff", "--unified=3", "--no-ext-diff", "HEAD", "--", &relative_path])
            .output()
            .map_err(|err| anyhow!("failed to inspect git diff: {}", err))?;

        if !diff_output.status.success() {
            let stderr = decode_bytes(&diff_output.stderr).trim().to_string();
            return Err(anyhow!(
                "git diff failed for '{}': {}",
                relative_path,
                stderr
            ));
        }

        let diff_text = decode_bytes(&diff_output.stdout);
        parse_unified_diff(&diff_text)
    };

    Ok(WebReviewFileDetail {
        path: summary.path,
        status: summary.status,
        additions: summary.additions,
        deletions: summary.deletions,
        hunks,
        preview_kind,
        mime_type,
        is_binary,
    })
}

fn build_workspace_browser(base_dir: &Path, workspace_root: &str) -> Result<WebWorkspaceBrowser> {
    let workspace = canonical_workspace_dir_from(base_dir, workspace_root)?;
    let entries = read_workspace_entries(&workspace, &workspace, 0)?;

    Ok(WebWorkspaceBrowser {
        root_name: basename_for_display(workspace_root),
        root_path: display_workspace_path(&workspace),
        entries,
    })
}

fn read_workspace_entries(root: &Path, dir: &Path, depth: usize) -> Result<Vec<WebWorkspaceEntry>> {
    if depth > 3 {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    let mut dir_entries: Vec<_> = std::fs::read_dir(dir)
        .map_err(|err| anyhow!("failed to read workspace dir '{}': {}", dir.display(), err))?
        .filter_map(|entry| entry.ok())
        .collect();

    dir_entries.sort_by(|left, right| {
        let left_is_dir = left.file_type().map(|file_type| file_type.is_dir()).unwrap_or(false);
        let right_is_dir = right.file_type().map(|file_type| file_type.is_dir()).unwrap_or(false);
        right_is_dir
            .cmp(&left_is_dir)
            .then_with(|| left.file_name().cmp(&right.file_name()))
    });

    for entry in dir_entries.into_iter().take(80) {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if should_skip_workspace_entry(&name) {
            continue;
        }

        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");

        let file_type = entry.file_type().ok();
        let is_dir = file_type.as_ref().map(|kind| kind.is_dir()).unwrap_or(false);

        let children = if is_dir {
            Some(read_workspace_entries(root, &path, depth + 1)?)
        } else {
            None
        };

        entries.push(WebWorkspaceEntry {
            path: relative,
            name,
            kind: if is_dir { "directory".to_string() } else { "file".to_string() },
            children,
        });
    }

    Ok(entries)
}

fn should_skip_workspace_entry(name: &str) -> bool {
    matches!(
        name,
        ".git" | "target" | "node_modules" | ".next" | "dist" | "build" | "__pycache__"
    )
}

fn build_workspace_file_response(base_dir: &Path, workspace_root: &str, path: &str) -> Result<WebWorkspaceFileResponse> {
    let workspace = canonical_workspace_dir_from(base_dir, workspace_root)?;
    let relative_path = sanitize_review_path(path)?;
    let absolute = workspace.join(&relative_path);

    if !absolute.exists() {
        return Err(anyhow!("workspace file does not exist: {}", relative_path));
    }
    if !absolute.is_file() {
        return Err(anyhow!("workspace path is not a file: {}", relative_path));
    }

    let bytes = std::fs::read(&absolute)
        .map_err(|err| anyhow!("failed to read workspace file '{}': {}", absolute.display(), err))?;

    let preview_kind = workspace_preview_kind(&relative_path);
    let mime_type = workspace_mime_type(&relative_path);
    let is_binary = !matches!(preview_kind.as_str(), "text" | "markdown" | "svg");
    let preview_limit = if matches!(preview_kind.as_str(), "text" | "markdown") {
        1_200_000usize
    } else {
        120_000usize
    };
    let truncated = bytes.len() > preview_limit;
    let preview_bytes = if truncated { &bytes[..preview_limit] } else { &bytes[..] };
    let content = if is_binary {
        String::new()
    } else {
        decode_bytes(preview_bytes)
    };
    let line_count = if is_binary { 0 } else { content.lines().count() };

    Ok(WebWorkspaceFileResponse {
        path: relative_path.clone(),
        name: Path::new(&relative_path)
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or(relative_path.clone()),
        language: file_language_from_path(&relative_path),
        content,
        truncated,
        line_count,
        mime_type,
        preview_kind,
        is_binary,
    })
}

fn workspace_preview_kind(path: &str) -> String {
    match Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "md" | "markdown" => "markdown",
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "ico" | "avif" | "svg" => "image",
        "pdf" => "pdf",
        "mp3" | "wav" | "ogg" | "m4a" | "flac" | "aac" => "audio",
        "mp4" | "webm" | "mov" | "mkv" | "avi" => "video",
        "zip" | "rar" | "7z" | "gz" | "tar" | "exe" | "dll" | "bin" | "wasm" | "class" => "unsupported",
        _ => "text",
    }
    .to_string()
}

fn workspace_mime_type(path: &str) -> String {
    match Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "avif" => "image/avif",
        "svg" => "image/svg+xml; charset=utf-8",
        "pdf" => "application/pdf",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "m4a" => "audio/mp4",
        "flac" => "audio/flac",
        "aac" => "audio/aac",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",
        "md" | "markdown" | "txt" | "rs" | "js" | "jsx" | "ts" | "tsx" | "json" | "toml" | "yml" | "yaml" | "css" | "html" | "py" | "sh" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
    .to_string()
}

fn file_language_from_path(path: &str) -> String {
    match Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "rs" => "rust",
        "js" => "javascript",
        "ts" => "typescript",
        "tsx" => "tsx",
        "jsx" => "jsx",
        "json" => "json",
        "html" => "html",
        "css" => "css",
        "md" => "markdown",
        "py" => "python",
        "toml" => "toml",
        "yml" | "yaml" => "yaml",
        "sh" => "bash",
        "txt" => "text",
        _ => "text",
    }
    .to_string()
}

fn basename_for_display(path: &str) -> String {
    let trimmed = path.trim_end_matches(['\\', '/']);
    Path::new(trimmed)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| trimmed.to_string())
}

fn read_review_status_entries(workspace: &Path) -> Result<Vec<ReviewStatusEntry>> {
    let output = Command::new("git")
        .current_dir(workspace)
        .args(["status", "--porcelain=v1", "--untracked-files=all"])
        .output()
        .map_err(|err| anyhow!("failed to read git status: {}", err))?;

    if !output.status.success() {
        let stderr = decode_bytes(&output.stderr).trim().to_string();
        return Err(anyhow!("git status failed: {}", stderr));
    }

    let stdout = decode_bytes(&output.stdout);
    let mut entries = Vec::new();

    for line in stdout.lines() {
        if line.len() < 4 {
            continue;
        }
        let code = &line[..2];
        let raw_path = line[3..].trim();
        if raw_path.is_empty() {
            continue;
        }
        let path = raw_path
            .split(" -> ")
            .last()
            .unwrap_or(raw_path)
            .trim()
            .to_string();
        entries.push(ReviewStatusEntry {
            path,
            status: review_status_name(code),
            untracked: code == "??",
        });
    }

    Ok(entries)
}

fn read_review_numstat(workspace: &Path) -> Result<std::collections::HashMap<String, (u32, u32)>> {
    let output = Command::new("git")
        .current_dir(workspace)
        .args(["diff", "--numstat", "HEAD", "--"])
        .output()
        .map_err(|err| anyhow!("failed to read git diff stats: {}", err))?;

    if !output.status.success() {
        let stderr = decode_bytes(&output.stderr).trim().to_string();
        if stderr.contains("bad revision 'HEAD'")
            || stderr.contains("ambiguous argument 'HEAD'")
            || stderr.contains("unknown revision or path not in the working tree")
        {
            return Ok(std::collections::HashMap::new());
        }
        return Err(anyhow!("git diff --numstat failed: {}", stderr));
    }

    let stdout = decode_bytes(&output.stdout);
    let mut map = std::collections::HashMap::new();

    for line in stdout.lines() {
        let mut parts = line.splitn(3, '\t');
        let additions = parts.next().unwrap_or_default();
        let deletions = parts.next().unwrap_or_default();
        let path = parts.next().unwrap_or_default().trim();
        if path.is_empty() {
            continue;
        }
        map.insert(
            path.to_string(),
            (
                additions.parse::<u32>().unwrap_or(0),
                deletions.parse::<u32>().unwrap_or(0),
            ),
        );
    }

    Ok(map)
}

fn review_status_name(code: &str) -> String {
    if code == "??" {
        return "untracked".to_string();
    }

    let bytes = code.as_bytes();
    let left = bytes.first().copied().unwrap_or(b' ');
    let right = bytes.get(1).copied().unwrap_or(b' ');

    if left == b'D' || right == b'D' {
        "deleted".to_string()
    } else if left == b'A' || right == b'A' {
        "added".to_string()
    } else if left == b'R' || right == b'R' {
        "renamed".to_string()
    } else if left == b'C' || right == b'C' {
        "copied".to_string()
    } else if left == b'M' || right == b'M' || left == b'U' || right == b'U' {
        "modified".to_string()
    } else {
        "changed".to_string()
    }
}

fn extract_recent_review_paths(messages: &[MessageBlock]) -> Vec<String> {
    let start_index = messages
        .iter()
        .rposition(|message| matches!(message, MessageBlock::User { .. }))
        .map(|index| index + 1)
        .unwrap_or(0);

    let mut pending_tools: HashMap<String, (String, Value)> = HashMap::new();
    let mut touched = Vec::new();
    let mut seen = BTreeSet::new();

    for message in &messages[start_index..] {
        match message {
            MessageBlock::ToolCall {
                name,
                args,
                call_id,
                ..
            } => {
                pending_tools.insert(call_id.clone(), (name.clone(), args.clone()));
            }
            MessageBlock::ToolResult {
                call_id,
                success,
                ..
            } if *success => {
                if let Some((tool_name, args)) = pending_tools.get(call_id) {
                    for path in extract_paths_from_tool_call(tool_name, args) {
                        if seen.insert(path.clone()) {
                            touched.push(path);
                        }
                    }
                }
            }
            MessageBlock::Diff { diff } => {
                if seen.insert(diff.file_path.clone()) {
                    touched.push(diff.file_path.clone());
                }
            }
            _ => {}
        }
    }

    touched
}

fn extract_paths_from_tool_call(tool_name: &str, args: &Value) -> Vec<String> {
    let mut paths = Vec::new();
    match tool_name {
        "write_file" | "edit_file" | "delete_file" | "read_file" => {
            if let Some(path) = args.get("path").and_then(|value| value.as_str()) {
                if let Ok(normalized) = sanitize_review_path(path) {
                    paths.push(normalized);
                }
            }
        }
        "copy_file" => {
            if let Some(path) = args.get("dst").and_then(|value| value.as_str()) {
                if let Ok(normalized) = sanitize_review_path(path) {
                    paths.push(normalized);
                }
            }
        }
        "move_file" => {
            if let Some(path) = args
                .get("dst")
                .or_else(|| args.get("destination"))
                .and_then(|value| value.as_str())
            {
                if let Ok(normalized) = sanitize_review_path(path) {
                    paths.push(normalized);
                }
            }
        }
        _ => {}
    }
    paths
}

fn sanitize_review_path(path: &str) -> Result<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("review path is empty"));
    }

    let candidate = Path::new(trimmed);
    if candidate.is_absolute() {
        return Err(anyhow!("absolute review paths are not allowed"));
    }
    if candidate
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(anyhow!("parent directory traversal is not allowed"));
    }

    Ok(trimmed.replace('\\', "/"))
}

fn extract_required_workspace_paths_from_text(text: &str) -> Vec<String> {
    static PATH_RE: OnceLock<Regex> = OnceLock::new();
    let path_re = PATH_RE.get_or_init(|| {
        Regex::new(
            r#"(?i)(?:^|[\s"'`\(\[\{<,，、。；：])((?:\./)?(?:[\w.-]+/)+[\w.-]+|(?:\./)?[\w.-]+\.[A-Za-z0-9]{1,12})"#,
        )
            .expect("valid workspace path regex")
    });

    let mut paths = Vec::new();
    let mut seen = BTreeSet::new();
    for caps in path_re.captures_iter(text) {
        let Some(raw) = caps.get(1).map(|m| m.as_str()) else {
            continue;
        };
        if let Ok(normalized) = sanitize_review_path(raw) {
            let lowered = normalized.to_ascii_lowercase();
            if lowered.starts_with("http/")
                || lowered.starts_with("https/")
                || lowered.starts_with("www.")
                || lowered == "json"
                || lowered == "yaml"
                || lowered == "toml"
            {
                continue;
            }
            if seen.insert(normalized.clone()) {
                paths.push(normalized);
            }
        }
    }
    paths
}

fn directory_like_required_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/").trim_matches('/').to_string();
    if normalized.is_empty() {
        return false;
    }
    Path::new(&normalized).extension().is_none()
}

fn common_required_directory_base(paths: &[String]) -> Option<String> {
    let mut directories = paths
        .iter()
        .filter(|path| directory_like_required_path(path))
        .map(|path| path.replace('\\', "/").trim_matches('/').to_string())
        .filter(|path| !path.is_empty())
        .collect::<Vec<_>>();
    directories.sort_by_key(|path| std::cmp::Reverse(path.len()));
    directories.into_iter().next()
}

fn normalize_required_workspace_paths(paths: Vec<String>) -> Vec<String> {
    let directory_base = common_required_directory_base(&paths);
    let mut normalized = Vec::new();
    let mut seen = BTreeSet::new();
    for path in paths {
        let candidate = if !path.contains('/') && !path.contains('\\') {
            if let Some(base) = directory_base.as_ref() {
                format!("{}/{}", base.trim_end_matches('/'), path)
            } else {
                path
            }
        } else {
            path
        };
        if let Ok(cleaned) = sanitize_review_path(&candidate) {
            if seen.insert(cleaned.to_ascii_lowercase()) {
                normalized.push(cleaned);
            }
        }
    }
    normalized
        .iter()
        .filter(|path| {
            if !directory_like_required_path(path) {
                return true;
            }
            let prefix = format!("{}/", path.trim_end_matches('/'));
            !normalized.iter().any(|other| {
                !other.eq_ignore_ascii_case(path) && other.to_ascii_lowercase().starts_with(&prefix.to_ascii_lowercase())
            })
        })
        .cloned()
        .collect()
}

fn collect_required_workspace_paths_from_user_content(content: &str) -> Vec<String> {
    normalize_required_workspace_paths(extract_required_workspace_paths_from_text(content))
}

fn collect_latest_required_workspace_paths(messages: &[MessageBlock]) -> Vec<String> {
    for block in messages.iter().rev() {
        if let MessageBlock::User { content, .. } = block {
            return collect_required_workspace_paths_from_user_content(content);
        }
    }
    Vec::new()
}

fn verification_item_matches_required_paths(item: &str, required_paths: &[String]) -> bool {
    if required_paths.is_empty() {
        return true;
    }
    let mentioned_paths = extract_required_workspace_paths_from_text(item);
    if mentioned_paths.is_empty() {
        return true;
    }
    mentioned_paths.iter().all(|mentioned| {
        required_paths
            .iter()
            .any(|required| required.eq_ignore_ascii_case(mentioned))
    })
}

fn count_file_lines(path: &Path) -> usize {
    read_text_file(path)
        .map(|content| content.lines().count())
        .unwrap_or(0)
}

fn build_untracked_file_hunks(path: &Path) -> Result<Vec<WebReviewHunk>> {
    let content = read_text_file(path)
        .map_err(|err| anyhow!("failed to read '{}': {}", path.display(), err))?;
    let mut lines = Vec::new();

    for (index, line) in content.lines().enumerate() {
        lines.push(WebReviewLine {
            kind: "added".to_string(),
            old_number: None,
            new_number: Some(index + 1),
            content: line.to_string(),
        });
    }

    if content.ends_with('\n') && content.lines().count() == 0 {
        lines.push(WebReviewLine {
            kind: "added".to_string(),
            old_number: None,
            new_number: Some(1),
            content: String::new(),
        });
    }

    Ok(vec![WebReviewHunk {
        header: "@@ new file @@".to_string(),
        lines,
    }])
}

fn parse_unified_diff(diff: &str) -> Vec<WebReviewHunk> {
    let mut hunks = Vec::new();
    let mut current_hunk: Option<WebReviewHunk> = None;
    let mut old_line = 0_usize;
    let mut new_line = 0_usize;

    for line in diff.lines() {
        if line.starts_with("@@") {
            if let Some(hunk) = current_hunk.take() {
                hunks.push(hunk);
            }
            let (old_start, new_start) = parse_hunk_positions(line);
            old_line = old_start;
            new_line = new_start;
            current_hunk = Some(WebReviewHunk {
                header: line.to_string(),
                lines: Vec::new(),
            });
            continue;
        }

        let Some(hunk) = current_hunk.as_mut() else {
            continue;
        };

        if line.starts_with('+') && !line.starts_with("+++") {
            hunk.lines.push(WebReviewLine {
                kind: "added".to_string(),
                old_number: None,
                new_number: Some(new_line),
                content: line[1..].to_string(),
            });
            new_line += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            hunk.lines.push(WebReviewLine {
                kind: "removed".to_string(),
                old_number: Some(old_line),
                new_number: None,
                content: line[1..].to_string(),
            });
            old_line += 1;
        } else if line.starts_with(' ') {
            hunk.lines.push(WebReviewLine {
                kind: "context".to_string(),
                old_number: Some(old_line),
                new_number: Some(new_line),
                content: line[1..].to_string(),
            });
            old_line += 1;
            new_line += 1;
        }
    }

    if let Some(hunk) = current_hunk.take() {
        hunks.push(hunk);
    }

    hunks
}

fn parse_hunk_positions(header: &str) -> (usize, usize) {
    let mut old_start = 0_usize;
    let mut new_start = 0_usize;
    let parts: Vec<&str> = header.split_whitespace().collect();

    if let Some(old_part) = parts.get(1) {
        old_start = old_part
            .trim_start_matches('-')
            .split(',')
            .next()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
    }

    if let Some(new_part) = parts.get(2) {
        new_start = new_part
            .trim_start_matches('+')
            .split(',')
            .next()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
    }

    (old_start, new_start)
}

fn initial_runtime_settings(
    config: &Config,
    security: &SecurityConfig,
    assistant_config: &AssistantConfig,
    persisted_state_path: &Path,
    paths: &AppPaths,
) -> RuntimeSettings {
    let persisted_state = load_persisted_web_state(persisted_state_path);
    let model = effective_model_name(config);
    let providers = effective_providers(config);
    let config_workspace_root = default_workspace_root(config, paths);
    let workspace_root = persisted_state
        .as_ref()
        .and_then(|state| state.workspace_root.clone())
        .and_then(|saved| resolve_workspace_root(&saved, &config_workspace_root).ok())
        .unwrap_or(config_workspace_root);
    let toolchains = persisted_state
        .as_ref()
        .and_then(|state| state.toolchains.clone())
        .map(normalize_toolchain_paths)
        .unwrap_or_else(auto_detect_toolchain_paths);
    let api_url = persisted_state
        .as_ref()
        .and_then(|state| state.api_url.clone())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| assistant_config.api_url.clone());
    let model = persisted_state
        .as_ref()
        .and_then(|state| state.model.clone())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(model);
    let reasoning_effort = persisted_state
        .as_ref()
        .and_then(|state| state.reasoning_effort.clone())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "medium".to_string());
    let auto_approve_tools = persisted_state
        .as_ref()
        .and_then(|state| state.auto_approve_tools)
        .unwrap_or(security.auto_approve_tools);
    let max_auto_approve_risk = persisted_state
        .as_ref()
        .and_then(|state| state.max_auto_approve_risk.clone())
        .and_then(|value| parse_risk_level(&value).ok())
        .unwrap_or_else(|| security.max_auto_approve_risk.clone());
    let max_tool_calls_per_minute = persisted_state
        .as_ref()
        .and_then(|state| state.max_tool_calls_per_minute)
        .unwrap_or(security.max_tool_calls_per_minute);
    let burst_limit = persisted_state
        .as_ref()
        .and_then(|state| state.burst_limit)
        .unwrap_or(security.tool_call_burst_limit);
    RuntimeSettings {
        api_url,
        model,
        deep_think: false,
        reasoning_effort,
        competition_mode: false,
        privacy_mode: false,
        api_key: assistant_config.api_key.clone(),
        providers,
        workspace_root,
        auto_approve_tools,
        max_auto_approve_risk,
        max_tool_calls_per_minute,
        burst_limit,
        toolchains,
    }
}

fn extend_security_allowed_roots(security: &mut SecurityConfig, paths: &AppPaths) {
    push_allowed_root(&mut security.allowed_roots, paths.state_dir().to_path_buf());
    push_allowed_root(&mut security.allowed_roots, paths.sessions_dir());
    push_allowed_root(&mut security.allowed_roots, paths.sandbox_dir());
    push_allowed_root(&mut security.allowed_roots, paths.downloads_dir());
}

fn push_allowed_root(roots: &mut Vec<PathBuf>, candidate: PathBuf) {
    let normalized = candidate.canonicalize().unwrap_or(candidate);
    let exists = roots.iter().any(|root| {
        root == &normalized || root.canonicalize().map(|value| value == normalized).unwrap_or(false)
    });
    if !exists {
        roots.push(normalized);
    }
}

fn default_workspace_root(config: &Config, paths: &AppPaths) -> String {
    let sandbox_root = display_workspace_path(&paths.sandbox_dir());
    config
        .user_tools
        .workspace_dir
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .and_then(|value| resolve_workspace_root(value, &sandbox_root).ok())
        .unwrap_or(sandbox_root)
}

fn sanitize_toolchain_paths(mut incoming: BTreeMap<String, String>) -> BTreeMap<String, String> {
    for (key, value) in incoming.iter_mut() {
        let normalized_key = key.trim().to_ascii_lowercase();
        let trimmed = value.trim();
        if normalized_key.is_empty() || trimmed.is_empty() {
            continue;
        }
        if let Some(resolved) = resolve_toolchain_value(&normalized_key, trimmed) {
            *value = resolved;
        } else {
            *value = trimmed.to_string();
        }
    }
    normalize_toolchain_paths(incoming)
}

fn load_persisted_current_session_id(path: &Path) -> Option<String> {
    let state = load_persisted_web_state(path)?;
    state
        .current_session_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn load_persisted_web_state(path: &Path) -> Option<PersistedWebState> {
    let content = ensure_json_text(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn preferred_session_id(session_manager: &SessionManager) -> Option<String> {
    session_manager
        .index
        .iter()
        .find(|meta| meta.message_count > 0)
        .or_else(|| session_manager.index.first())
        .map(|meta| meta.id.clone())
}

fn restore_session_selection(session_manager: &mut SessionManager, persisted_state_path: &Path) {
    if session_manager.current_id.is_some() {
        return;
    }

    if let Some(saved_id) = load_persisted_current_session_id(persisted_state_path) {
        if session_manager.index.iter().any(|meta| meta.id == saved_id) {
            session_manager.current_id = Some(saved_id);
            return;
        }
    }

    session_manager.current_id = preferred_session_id(session_manager);
}

fn persist_web_state(
    state: &WebAppState,
    runtime: &RuntimeSettings,
    current_session_id: Option<String>,
) -> Result<()> {
    if let Some(parent) = state.persisted_state_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let payload = PersistedWebState {
        workspace_root: Some(runtime.workspace_root.clone()),
        current_session_id,
        api_url: Some(runtime.api_url.clone()),
        model: Some(runtime.model.clone()),
        reasoning_effort: Some(runtime.reasoning_effort.clone()),
        auto_approve_tools: Some(runtime.auto_approve_tools),
        max_auto_approve_risk: Some(risk_level_name(&runtime.max_auto_approve_risk).to_string()),
        max_tool_calls_per_minute: Some(runtime.max_tool_calls_per_minute),
        burst_limit: Some(runtime.burst_limit),
        toolchains: Some(runtime.toolchains.clone()),
    };
    let content = serde_json::to_string_pretty(&payload)?;
    fs::write(&state.persisted_state_path, content)?;
    Ok(())
}

fn runtime_to_payload(runtime: &RuntimeSettings) -> WebConfigPayload {
    WebConfigPayload {
        api_url: runtime.api_url.clone(),
        model: runtime.model.clone(),
        deep_think: runtime.deep_think,
        reasoning_effort: runtime.reasoning_effort.clone(),
        competition_mode: runtime.competition_mode,
        privacy_mode: runtime.privacy_mode,
        api_key: runtime.api_key.clone(),
        providers: runtime.providers.clone(),
        workspace_root: runtime.workspace_root.clone(),
        max_tool_calls_per_minute: runtime.max_tool_calls_per_minute,
        burst_limit: runtime.burst_limit,
        auto_approve_tools: runtime.auto_approve_tools,
        max_auto_approve_risk: risk_level_name(&runtime.max_auto_approve_risk).to_string(),
        toolchains: runtime.toolchains.clone(),
    }
}

fn runtime_security_config(base: &SecurityConfig, runtime: &RuntimeSettings) -> SecurityConfig {
    let mut security = base.clone();
    security.auto_approve_tools = runtime.auto_approve_tools;
    security.max_auto_approve_risk = runtime.max_auto_approve_risk.clone();
    let max_tool_calls_per_minute = if runtime.max_tool_calls_per_minute == 0 {
        u32::MAX / 8
    } else {
        runtime.max_tool_calls_per_minute
    };
    let burst_limit = if runtime.burst_limit == 0 {
        u32::MAX / 8
    } else {
        runtime.burst_limit
    };
    security.max_tool_calls_per_minute = max_tool_calls_per_minute;
    security.tool_call_burst_limit = burst_limit;
    security.rate_limiter = Arc::new(RateLimiter::new(
        max_tool_calls_per_minute,
        burst_limit,
    ));

    if let Ok(workspace) = canonical_workspace_dir(&runtime.workspace_root) {
        security.allowed_roots.retain(|path| path != &workspace);
        security.allowed_roots.insert(0, workspace);
    }

    security
}

fn effort_temperature(runtime: &RuntimeSettings) -> f32 {
    match runtime.reasoning_effort.to_ascii_lowercase().as_str() {
        "low" => 0.6,
        "high" => 0.85,
        "max" => 0.9,
        _ => 0.7,
    }
}

fn effort_max_tokens(runtime: &RuntimeSettings) -> usize {
    let model_limit =
        crate::tui::model_config::ModelRegistry::get_max_tokens(&runtime.model, usize::MAX);
    let token_ratio = match runtime.reasoning_effort.to_ascii_lowercase().as_str() {
        "low" => 0.35,
        "high" => 0.8,
        "max" => 1.0,
        _ => 0.55,
    };
    let mut max_tokens = (model_limit as f64 * token_ratio) as usize;
    if runtime.deep_think {
        max_tokens = model_limit;
    }
    max_tokens.max(1)
}

fn ensure_current_session(state: &WebAppState) -> Result<String> {
    let model = {
        let runtime = lock_runtime_settings(state)?;
        runtime.model.clone()
    };
    let (current_id, should_persist) = {
        let mut session_manager = lock_session_manager(state)?;
        let mut changed = false;
        if session_manager.current_id.is_none() {
            restore_session_selection(&mut session_manager, &state.persisted_state_path);
            if session_manager.current_id.is_none() {
                let meta = session_manager.create_session(&model)?;
                session_manager.current_id = Some(meta.id.clone());
            }
            changed = true;
        }
        (
            session_manager
                .current_id
                .clone()
                .ok_or_else(|| anyhow!("missing current session"))?,
            changed,
        )
    };

    if should_persist {
        let runtime = lock_runtime_settings(state)?;
        let _ = persist_web_state(state, &runtime, Some(current_id.clone()));
    }

    Ok(current_id)
}

fn event_bytes(event: StreamEnvelope) -> Bytes {
    let mut value = serde_json::to_value(&event)
        .unwrap_or_else(|_| json!({"type":"error","error":"serialization failed"}));
    normalize_json_strings(&mut value);
    let line = serde_json::to_string(&value).unwrap_or_else(|_| {
        "{\"type\":\"error\",\"error\":\"serialization failed\"}".to_string()
    });
    Bytes::from(format!("{}\n", line))
}

fn effective_model_name(config: &Config) -> String {
    if let Ok(model) = std::env::var("AI_MODEL") {
        if !model.trim().is_empty() {
            return model;
        }
    }

    if let Ok(provider_manager) = ProviderManager::from_env_file(None) {
        return provider_manager.current().model.clone();
    }

    config.ai.model.clone()
}

fn effective_providers(config: &Config) -> Vec<String> {
    if let Ok(provider_manager) = ProviderManager::from_env_file(None) {
        return provider_manager
            .providers()
            .iter()
            .map(|provider| provider.name.clone())
            .collect();
    }

    let providers: Vec<String> = config.ai.providers.keys().cloned().collect();
    if providers.is_empty() {
        vec!["default".to_string()]
    } else {
        providers
    }
}

fn parse_risk_level(value: &str) -> Result<RiskLevel> {
    match value.trim().to_ascii_lowercase().as_str() {
        "safe" => Ok(RiskLevel::Safe),
        "moderate" => Ok(RiskLevel::Moderate),
        "low" => Ok(RiskLevel::Low),
        other => Err(anyhow!("unsupported risk level: {}", other)),
    }
}

fn non_empty_or(value: &str, fallback: &str) -> String {
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

fn resolve_workspace_root(requested: &str, fallback: &str) -> Result<String> {
    if requested.is_empty() {
        return Ok(fallback.to_string());
    }

    let workspace = PathBuf::from(requested);
    if !workspace.exists() {
        return Err(anyhow!("workspace root does not exist: {}", requested));
    }
    if !workspace.is_dir() {
        return Err(anyhow!("workspace root is not a directory: {}", requested));
    }

    let canonical = canonical_workspace_dir(&workspace.to_string_lossy()).unwrap_or(workspace);
    Ok(display_workspace_path(&canonical))
}

fn lock_stream_runtime(
    state: &WebAppState,
) -> Result<std::sync::MutexGuard<'_, HashMap<String, StreamSessionRuntime>>> {
    state
        .stream_runtime
        .lock()
        .map_err(|_| anyhow!("stream runtime lock poisoned"))
}

fn resolve_stream_session_id(state: &WebAppState, session_id: Option<String>) -> Result<String> {
    if let Some(session_id) = session_id.filter(|value| !value.trim().is_empty()) {
        return Ok(session_id);
    }

    let session_manager = lock_session_manager(state)?;
    session_manager
        .current_id
        .clone()
        .ok_or_else(|| anyhow!("missing current session"))
}

fn stop_stream_session(state: &WebAppState, session_id: &str) -> Result<()> {
    let mut runtime = lock_stream_runtime(state)?;
    if let Some(session) = runtime.remove(session_id) {
        session.abort_handle.abort();
        for (_, pending) in session.pending_approvals {
            let _ = pending.sender.send(false);
        }
    }
    Ok(())
}

fn respond_to_tool_permission(
    state: &WebAppState,
    session_id: &str,
    call_id: &str,
    approved: bool,
) -> Result<()> {
    let mut runtime = lock_stream_runtime(state)?;
    let session = runtime
        .get_mut(session_id)
        .ok_or_else(|| anyhow!("session is not waiting for approval"))?;
    let pending = session
        .pending_approvals
        .remove(call_id)
        .ok_or_else(|| anyhow!("tool approval request not found"))?;
    let status = if approved { "approved" } else { "denied" }.to_string();
    let result = if approved {
        "Approved by user".to_string()
    } else {
        "Denied by user".to_string()
    };
    let _ = session.event_tx.send(StreamEnvelope {
        r#type: "tool".to_string(),
        session_id: Some(session_id.to_string()),
        messages: None,
        delta: None,
        error: None,
        activity: Some(activity_event(
            format!("tool_{}", status),
            Some(pending.name.clone()),
        )),
        tool: Some(WebToolEvent {
            call_id: call_id.to_string(),
            name: pending.name.clone(),
            status,
            risk: pending.risk.clone(),
            args: Some(pending.args.clone()),
            result: Some(result),
            success: Some(approved),
            file_path: extract_tool_path(&pending.name, &pending.args),
        }),
        permission: None,
        edited_files: None,
        research: None,
        subagents: None,
        verifier: None,
    });
    pending
        .sender
        .send(approved)
        .map_err(|_| anyhow!("tool approval channel closed"))?;
    Ok(())
}

fn canonical_workspace_dir_from(base_dir: &Path, path: &str) -> Result<PathBuf> {
    let workspace = PathBuf::from(path);
    if workspace.as_os_str().is_empty() {
        return Err(anyhow!("workspace root is empty"));
    }

    workspace
        .canonicalize()
        .or_else(|_| -> Result<PathBuf, std::io::Error> {
            if workspace.is_absolute() {
                Ok(workspace.clone())
            } else {
                Ok(base_dir.join(workspace))
            }
        })
        .map_err(|err| anyhow!("failed to resolve workspace root '{}': {}", path, err))
}

fn canonical_workspace_dir(path: &str) -> Result<PathBuf> {
    let base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    canonical_workspace_dir_from(&base_dir, path)
}

fn display_workspace_path(path: &Path) -> String {
    let raw = path.display().to_string();
    raw.trim_start_matches(r"\\?\").to_string()
}

fn ensure_git_repository(workspace: &Path) -> Result<()> {
    let git_dir = workspace.join(".git");
    if git_dir.exists() {
        return Ok(());
    }

    let output = Command::new("git")
        .current_dir(workspace)
        .args(["init", "-b", "main"])
        .output()
        .or_else(|_| {
            Command::new("git")
                .current_dir(workspace)
                .arg("init")
                .output()
        })
        .map_err(|err| anyhow!("failed to initialize git repository: {}", err))?;

    if !output.status.success() {
        let stderr = decode_bytes(&output.stderr).trim().to_string();
        return Err(anyhow!("git init failed: {}", stderr));
    }

    Ok(())
}

fn enter_workspace_dir_from(base_dir: &Path, path: &str) -> Result<Option<WorkspaceDirGuard>> {
    if path.trim().is_empty() {
        return Ok(None);
    }

    let target = canonical_workspace_dir_from(base_dir, path)?;
    ensure_git_repository(&target)?;
    let previous = std::env::current_dir().map_err(|err| anyhow!("failed to read current dir: {}", err))?;
    std::env::set_current_dir(&target)
        .map_err(|err| anyhow!("failed to enter workspace '{}': {}", target.display(), err))?;

    Ok(Some(WorkspaceDirGuard { previous }))
}

struct WorkspaceDirGuard {
    previous: PathBuf,
}

impl Drop for WorkspaceDirGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(Path::new(&self.previous));
    }
}

fn lock_runtime_settings(
    state: &WebAppState,
) -> Result<std::sync::MutexGuard<'_, RuntimeSettings>> {
    state
        .runtime_settings
        .lock()
        .map_err(|_| anyhow!("runtime settings lock poisoned"))
}

fn lock_session_manager(
    state: &WebAppState,
) -> Result<std::sync::MutexGuard<'_, SessionManager>> {
    state
        .session_manager
        .lock()
        .map_err(|_| anyhow!("session manager lock poisoned"))
}

fn lock_assistant_slot(
    state: &WebAppState,
) -> Result<std::sync::MutexGuard<'_, Option<CliAssistant>>> {
    lock_assistant_mutex(&state.assistant)
}

fn lock_assistant_mutex(
    assistant: &Arc<Mutex<Option<CliAssistant>>>,
) -> Result<std::sync::MutexGuard<'_, Option<CliAssistant>>> {
    match assistant.lock() {
        Ok(guard) => Ok(guard),
        Err(poisoned) => {
            let mut guard = poisoned.into_inner();
            *guard = None;
            Ok(guard)
        }
    }
}

fn panic_payload_to_string(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown panic".to_string()
    }
}

fn messages_to_web(messages: &[MessageBlock]) -> Vec<WebMessage> {
    messages.iter().filter_map(message_to_web).collect()
}

fn message_to_web(message: &MessageBlock) -> Option<WebMessage> {
    match message {
        MessageBlock::User { content, .. } => Some(WebMessage {
            kind: "message".to_string(),
            role: "user".to_string(),
            content: sanitize_visible_stream_text(content),
            call_id: None,
            success: None,
            collapsed: None,
            tool_name: None,
            tool_args: None,
            status: None,
            file_path: None,
            added: None,
            removed: None,
            before_content: None,
            subagent: None,
            verifier: None,
        }),
        MessageBlock::Assistant { content } | MessageBlock::AssistantStreaming { content } => {
            Some(WebMessage {
                kind: "message".to_string(),
                role: "assistant".to_string(),
                content: sanitize_visible_stream_text(content),
                call_id: None,
                success: None,
                collapsed: None,
                tool_name: None,
                tool_args: None,
                    status: None,
                    file_path: None,
                    added: None,
                    removed: None,
                    before_content: None,
                    subagent: None,
                    verifier: None,
                })
        }
        MessageBlock::System { content } => Some(WebMessage {
            kind: "message".to_string(),
            role: "system".to_string(),
            content: sanitize_visible_stream_text(content),
            call_id: None,
            success: None,
            collapsed: None,
            tool_name: None,
            tool_args: None,
            status: None,
            file_path: None,
            added: None,
            removed: None,
            before_content: None,
            subagent: None,
            verifier: None,
        }),
        MessageBlock::Thinking { content, collapsed } => Some(WebMessage {
            kind: "thinking".to_string(),
            role: "assistant".to_string(),
            content: sanitize_visible_stream_text(content),
            call_id: None,
            success: None,
            collapsed: Some(*collapsed),
            tool_name: None,
            tool_args: None,
            status: None,
            file_path: None,
            added: None,
            removed: None,
            before_content: None,
            subagent: None,
            verifier: None,
        }),
        MessageBlock::ToolCall {
            name,
            args,
            call_id,
            status,
        } => Some(WebMessage {
            kind: "tool".to_string(),
            role: "assistant".to_string(),
            content: String::new(),
            call_id: Some(call_id.clone()),
            success: None,
            collapsed: None,
            tool_name: Some(name.clone()),
            tool_args: Some(args.clone()),
            status: Some(tool_status_name(status).to_string()),
            file_path: extract_tool_path(name, args),
            added: None,
            removed: None,
            before_content: None,
            subagent: None,
            verifier: None,
        }),
        MessageBlock::ToolResult {
            call_id,
            result,
            success,
        } => Some(WebMessage {
            kind: "tool_result".to_string(),
            role: "assistant".to_string(),
            content: sanitize_visible_stream_text(result),
            call_id: Some(call_id.clone()),
            success: Some(*success),
            collapsed: None,
            tool_name: None,
            tool_args: None,
            status: Some(if *success { "complete" } else { "failed" }.to_string()),
            file_path: None,
            added: None,
            removed: None,
            before_content: None,
            subagent: None,
            verifier: None,
        }),
        MessageBlock::Error { content } => Some(WebMessage {
            kind: "message".to_string(),
            role: "error".to_string(),
            content: sanitize_visible_stream_text(content),
            call_id: None,
            success: Some(false),
            collapsed: None,
            tool_name: None,
            tool_args: None,
            status: None,
            file_path: None,
            added: None,
            removed: None,
            before_content: None,
            subagent: None,
            verifier: None,
        }),
        MessageBlock::Diff { diff } => Some(WebMessage {
            kind: "diff".to_string(),
            role: "assistant".to_string(),
            content: String::new(),
            call_id: None,
            success: None,
            collapsed: None,
            tool_name: None,
            tool_args: None,
            status: Some("edited".to_string()),
            file_path: Some(diff.file_path.clone()),
            added: Some(diff.added),
            removed: Some(diff.removed),
            before_content: Some(diff.before_content.clone()),
            subagent: None,
            verifier: None,
        }),
        MessageBlock::Subagent { record } => Some(WebMessage {
            kind: "subagent".to_string(),
            role: "assistant".to_string(),
            content: String::new(),
            call_id: None,
            success: None,
            collapsed: None,
            tool_name: None,
            tool_args: None,
            status: Some(record.status.clone()),
            file_path: None,
            added: None,
            removed: None,
            before_content: None,
            subagent: Some(to_web_subagent(record)),
            verifier: None,
        }),
        MessageBlock::Verification { report } => Some(WebMessage {
            kind: "verification".to_string(),
            role: "assistant".to_string(),
            content: report.summary.clone(),
            call_id: None,
            success: Some(report.status.eq_ignore_ascii_case("pass")),
            collapsed: None,
            tool_name: None,
            tool_args: None,
            status: Some(report.status.clone()),
            file_path: None,
            added: None,
            removed: None,
            before_content: None,
            subagent: None,
            verifier: Some(to_web_verifier(report)),
        }),
    }
}

fn risk_level_name(level: &RiskLevel) -> &'static str {
    match level {
        RiskLevel::Safe => "safe",
        RiskLevel::Moderate => "moderate",
        RiskLevel::Low => "low",
    }
}

fn tool_status_name(status: &ToolCallStatus) -> &'static str {
    match status {
        ToolCallStatus::Pending => "pending",
        ToolCallStatus::Approved => "approved",
        ToolCallStatus::Denied(_) => "denied",
        ToolCallStatus::Executing => "executing",
        ToolCallStatus::Complete => "complete",
        ToolCallStatus::Failed(_) => "failed",
    }
}

fn internal_error(err: anyhow::Error) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn user(content: &str) -> MessageBlock {
        MessageBlock::User {
            content: content.to_string(),
            branch_id: "main".to_string(),
        }
    }

    fn test_runtime_settings(workspace_root: String) -> RuntimeSettings {
        RuntimeSettings {
            api_url: String::new(),
            model: "test-model".to_string(),
            deep_think: false,
            reasoning_effort: "medium".to_string(),
            competition_mode: false,
            privacy_mode: false,
            api_key: None,
            providers: Vec::new(),
            workspace_root,
            auto_approve_tools: true,
            max_auto_approve_risk: RiskLevel::Moderate,
            max_tool_calls_per_minute: 120,
            burst_limit: 12,
            toolchains: BTreeMap::new(),
        }
    }

    #[test]
    fn infer_research_kind_matches_deep_learning() {
        let messages = vec![
            user("Help me run a deep learning training experiment and track epoch loss with checkpointing"),
        ];
        let topic = infer_research_topic(&messages).unwrap();
        assert_eq!(infer_research_workflow_kind(&topic, &messages), "deep_learning");
    }

    #[test]
    fn infer_research_kind_matches_experimental_design() {
        let messages = vec![
            user("Design a cell culture and western blot experiment with a control group"),
        ];
        let topic = infer_research_topic(&messages).unwrap();
        assert_eq!(infer_research_workflow_kind(&topic, &messages), "experimental_design");
    }

    #[test]
    fn infer_research_kind_prefers_lightweight_classical_ml() {
        let messages = vec![user(
            "在当前工作区 experiments/ml_probe_5min 中做一个总时长控制在 5 分钟内的轻量机器学习研究：用 sklearn 的 iris 数据集快速验证逻辑回归是否能得到稳定分类效果。",
        )];
        let topic = infer_research_topic(&messages).unwrap();
        assert_eq!(infer_research_workflow_kind(&topic, &messages), "data_analysis");
    }

    #[test]
    fn extract_required_paths_handles_chinese_list_separators() {
        let paths = collect_required_workspace_paths_from_user_content(
            "在当前工作区 experiments/ml_probe_5min 中输出 train_and_eval.py、metrics.md、confusion_matrix.png，并验证文件存在。",
        );
        assert_eq!(
            paths,
            vec![
                "experiments/ml_probe_5min/train_and_eval.py".to_string(),
                "experiments/ml_probe_5min/metrics.md".to_string(),
                "experiments/ml_probe_5min/confusion_matrix.png".to_string(),
            ]
        );
    }

    #[test]
    fn adapt_bash_command_rewrites_cd_and_and_for_powershell() {
        let adapted = adapt_bash_command_for_powershell(
            "cd /d D:\\Project Testing\\experiments\\ml_probe_5min_r2 && python train_and_eval.py",
        );
        assert_eq!(
            adapted,
            "Set-Location -LiteralPath 'D:\\Project Testing\\experiments\\ml_probe_5min_r2'; python train_and_eval.py"
        );
    }

    #[test]
    fn adapt_bash_command_rewrites_dir_glob_list_for_powershell() {
        let adapted = adapt_bash_command_for_powershell("dir *.py *.md *.png");
        assert_eq!(
            adapted,
            "Get-ChildItem -Force -Include '*.py', '*.md', '*.png'"
        );
    }

    #[test]
    fn adapt_bash_command_strips_wrapped_quotes_from_windows_paths() {
        let adapted = adapt_bash_command_for_powershell(
            r#"cd /d "\"D:\Project Testing\experiments\ml_probe_5min_r5\"" && python train_and_eval.py"#,
        );
        assert_eq!(
            adapted,
            "Set-Location -LiteralPath 'D:\\Project Testing\\experiments\\ml_probe_5min_r5'; python train_and_eval.py"
        );
    }

    #[test]
    fn adapt_bash_command_preserves_mkdir_followed_by_cd_and_pwd() {
        let adapted = adapt_bash_command_for_powershell(
            "mkdir experiments/ml_probe_5min_r6 && cd experiments/ml_probe_5min_r6 && pwd",
        );
        assert_eq!(
            adapted,
            "New-Item -ItemType Directory -Force -Path 'experiments/ml_probe_5min_r6' | Out-Null; Set-Location -LiteralPath 'experiments/ml_probe_5min_r6'; Get-Location"
        );
    }

    #[test]
    fn gather_context_tool_result_summarizes_directory() {
        let temp_dir = std::env::temp_dir().join(format!(
            "tokitai_gather_context_dir_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(temp_dir.join("nested")).unwrap();
        fs::write(temp_dir.join("notes.txt"), "hello").unwrap();
        let runtime = test_runtime_settings(temp_dir.to_string_lossy().to_string());

        let result = gather_context_tool_result(Path::new("."), &runtime, &json!({"path": "."})).unwrap();
        let value: Value = serde_json::from_str(&result).unwrap();

        assert_eq!(value["kind"], "directory");
        let entries = value["entries"].as_array().unwrap();
        assert!(entries.iter().any(|entry| entry["name"] == "nested"));
        assert!(entries.iter().any(|entry| entry["name"] == "notes.txt"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn gather_context_tool_result_returns_file_preview() {
        let temp_dir = std::env::temp_dir().join(format!(
            "tokitai_gather_context_file_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("summary.md");
        fs::write(&file_path, "line 1\nline 2\nline 3").unwrap();
        let runtime = test_runtime_settings(temp_dir.to_string_lossy().to_string());

        let result = gather_context_tool_result(
            Path::new("."),
            &runtime,
            &json!({"path": "summary.md", "max_preview_chars": 64}),
        )
        .unwrap();
        let value: Value = serde_json::from_str(&result).unwrap();

        assert_eq!(value["kind"], "file");
        assert!(value["preview"].as_str().unwrap().contains("line 2"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn research_graph_advances_when_execution_and_diff_exist() {
        let messages = vec![
            user("Run a kmeans data analysis experiment"),
            MessageBlock::ToolCall {
                name: "read_file".to_string(),
                args: json!({"path":"data.csv"}),
                call_id: "c1".to_string(),
                status: ToolCallStatus::Complete,
            },
            MessageBlock::ToolCall {
                name: "run_python".to_string(),
                args: json!({"code":"print(\"ok\")"}),
                call_id: "c2".to_string(),
                status: ToolCallStatus::Executing,
            },
            MessageBlock::Diff {
                diff: FileDiff::compute("experiment.py", "", "print(\"ok\")\n"),
            },
        ];
        let graph = build_research_graph(
            "data_analysis",
            &messages,
            &ResearchRuntimeAssessment::default(),
        );
        assert!(!graph.nodes.is_empty());
        let current = graph.nodes.iter().find(|node| node.status == "current").unwrap();
        assert!(matches!(current.id.as_str(), "iterate" | "validate" | "artifact"));
    }

    #[test]
    fn deep_learning_runtime_can_be_marked_blocked_and_resumable() {
        let messages = vec![
            user("Start a deep learning training run on this machine and save checkpoints"),
            MessageBlock::ToolResult {
                call_id: "c1".to_string(),
                result: "CUDA out of memory while training epoch 1".to_string(),
                success: false,
            },
        ];
        let capability = SystemCapability {
            cpu_cores: 4,
            total_memory_mb: Some(8192),
            available_memory_mb: Some(4096),
            gpu_hint: None,
        };
        let assessment = assess_research_runtime("deep_learning", &messages, &capability);
        assert_eq!(assessment.overall_state, "blocked");
        assert!(assessment.recovery_hint.is_some());
        assert!(!assessment.resume_points.is_empty());
    }
}

