use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::mpsc::UnboundedReceiver;

pub const BRIDGE_PROTOCOL_V1: &str = "atlas-host-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostCommand {
    BootstrapLoad,
    SettingsUpdate,
    WorkspacePick,
    WorkspaceFileOpen,
    WorkspaceFileSave,
    WorkspaceFileUndo,
    WorkspaceFileComplete,
    WorkspaceReviewFile,
    WorkspaceIndexState,
    WorkspaceIndexUpdate,
    WorkspaceIndexSearch,
    VisualizationCatalog,
    VisualizationSnapshot,
    TasksState,
    TasksEnqueue,
    TasksStart,
    TasksCancel,
    TasksLog,
    ReviewerFeedbackState,
    ReviewerFeedbackAdd,
    ReviewerFeedbackResolve,
    ResearchPaperWorkflowRun,
    SearchHealth,
    SearchWeb,
    SearchPapers,
    SearchModels,
    SearchTracking,
    SearchBenchmarks,
    SearchGitHub,
    SearchGitHubPreview,
    SearchDatasets,
    SearchDatasetManifest,
    BrowserOpen,
    ChatSend,
    PromptOptimize,
    ChatStream,
    ChatStop,
    ScheduleManage,
    NativeRequest,
    ToolApprovalApprove,
    ToolApprovalDeny,
    GitState,
    GitAction,
    RunDebugState,
    RunDebugAction,
    TerminalsState,
    TerminalsCreate,
    TerminalsInput,
    TerminalsClose,
    SessionsCreate,
    SessionsSelect,
    SessionsRename,
    SessionsDelete,
}

impl HostCommand {
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "bootstrap.load" => Self::BootstrapLoad,
            "settings.update" => Self::SettingsUpdate,
            "workspace.pick" => Self::WorkspacePick,
            "workspace.file.open" => Self::WorkspaceFileOpen,
            "workspace.file.save" => Self::WorkspaceFileSave,
            "workspace.file.undo" => Self::WorkspaceFileUndo,
            "workspace.file.complete" => Self::WorkspaceFileComplete,
            "workspace.review.file" => Self::WorkspaceReviewFile,
            "workspace.index.state" => Self::WorkspaceIndexState,
            "workspace.index.update" => Self::WorkspaceIndexUpdate,
            "workspace.index.search" => Self::WorkspaceIndexSearch,
            "visualization.catalog" => Self::VisualizationCatalog,
            "visualization.snapshot" => Self::VisualizationSnapshot,
            "tasks.state" => Self::TasksState,
            "tasks.enqueue" => Self::TasksEnqueue,
            "tasks.start" => Self::TasksStart,
            "tasks.cancel" => Self::TasksCancel,
            "tasks.log" => Self::TasksLog,
            "reviewer_feedback.state" => Self::ReviewerFeedbackState,
            "reviewer_feedback.add" => Self::ReviewerFeedbackAdd,
            "reviewer_feedback.resolve" => Self::ReviewerFeedbackResolve,
            "research.paper_workflow.run" => Self::ResearchPaperWorkflowRun,
            "search.health" => Self::SearchHealth,
            "search.web" => Self::SearchWeb,
            "search.papers" => Self::SearchPapers,
            "search.models" => Self::SearchModels,
            "search.tracking" => Self::SearchTracking,
            "search.benchmarks" => Self::SearchBenchmarks,
            "search.github" => Self::SearchGitHub,
            "search.github_preview" => Self::SearchGitHubPreview,
            "search.datasets" => Self::SearchDatasets,
            "search.dataset_manifest" => Self::SearchDatasetManifest,
            "browser.open" => Self::BrowserOpen,
            "chat.send" => Self::ChatSend,
            "prompt.optimize" => Self::PromptOptimize,
            "chat.stream" => Self::ChatStream,
            "chat.stop" => Self::ChatStop,
            "schedule.manage" => Self::ScheduleManage,
            "native.request" => Self::NativeRequest,
            "tool.approval.approve" => Self::ToolApprovalApprove,
            "tool.approval.deny" => Self::ToolApprovalDeny,
            "git.state" => Self::GitState,
            "git.action" => Self::GitAction,
            "run_debug.state" => Self::RunDebugState,
            "run_debug.action" => Self::RunDebugAction,
            "terminals.state" => Self::TerminalsState,
            "terminals.create" => Self::TerminalsCreate,
            "terminals.input" => Self::TerminalsInput,
            "terminals.close" => Self::TerminalsClose,
            "sessions.create" => Self::SessionsCreate,
            "sessions.select" => Self::SessionsSelect,
            "sessions.rename" => Self::SessionsRename,
            "sessions.delete" => Self::SessionsDelete,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::BootstrapLoad => "bootstrap.load",
            Self::SettingsUpdate => "settings.update",
            Self::WorkspacePick => "workspace.pick",
            Self::WorkspaceFileOpen => "workspace.file.open",
            Self::WorkspaceFileSave => "workspace.file.save",
            Self::WorkspaceFileUndo => "workspace.file.undo",
            Self::WorkspaceFileComplete => "workspace.file.complete",
            Self::WorkspaceReviewFile => "workspace.review.file",
            Self::WorkspaceIndexState => "workspace.index.state",
            Self::WorkspaceIndexUpdate => "workspace.index.update",
            Self::WorkspaceIndexSearch => "workspace.index.search",
            Self::VisualizationCatalog => "visualization.catalog",
            Self::VisualizationSnapshot => "visualization.snapshot",
            Self::TasksState => "tasks.state",
            Self::TasksEnqueue => "tasks.enqueue",
            Self::TasksStart => "tasks.start",
            Self::TasksCancel => "tasks.cancel",
            Self::TasksLog => "tasks.log",
            Self::ReviewerFeedbackState => "reviewer_feedback.state",
            Self::ReviewerFeedbackAdd => "reviewer_feedback.add",
            Self::ReviewerFeedbackResolve => "reviewer_feedback.resolve",
            Self::ResearchPaperWorkflowRun => "research.paper_workflow.run",
            Self::SearchHealth => "search.health",
            Self::SearchWeb => "search.web",
            Self::SearchPapers => "search.papers",
            Self::SearchModels => "search.models",
            Self::SearchTracking => "search.tracking",
            Self::SearchBenchmarks => "search.benchmarks",
            Self::SearchGitHub => "search.github",
            Self::SearchGitHubPreview => "search.github_preview",
            Self::SearchDatasets => "search.datasets",
            Self::SearchDatasetManifest => "search.dataset_manifest",
            Self::BrowserOpen => "browser.open",
            Self::ChatSend => "chat.send",
            Self::PromptOptimize => "prompt.optimize",
            Self::ChatStream => "chat.stream",
            Self::ChatStop => "chat.stop",
            Self::ScheduleManage => "schedule.manage",
            Self::NativeRequest => "native.request",
            Self::ToolApprovalApprove => "tool.approval.approve",
            Self::ToolApprovalDeny => "tool.approval.deny",
            Self::GitState => "git.state",
            Self::GitAction => "git.action",
            Self::RunDebugState => "run_debug.state",
            Self::RunDebugAction => "run_debug.action",
            Self::TerminalsState => "terminals.state",
            Self::TerminalsCreate => "terminals.create",
            Self::TerminalsInput => "terminals.input",
            Self::TerminalsClose => "terminals.close",
            Self::SessionsCreate => "sessions.create",
            Self::SessionsSelect => "sessions.select",
            Self::SessionsRename => "sessions.rename",
            Self::SessionsDelete => "sessions.delete",
        }
    }

    pub fn is_stream(self) -> bool {
        matches!(self, Self::ChatStream)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostBridgeResponse {
    pub ok: bool,
    pub status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
}

impl HostBridgeResponse {
    pub fn success<T: Serialize>(payload: T) -> Self {
        Self {
            ok: true,
            status: 200,
            data: Some(json!(payload)),
            error: None,
            protocol: Some(BRIDGE_PROTOCOL_V1.to_string()),
        }
    }

    pub fn error(status: u16, message: impl Into<String>) -> Self {
        Self {
            ok: false,
            status,
            data: None,
            error: Some(message.into()),
            protocol: Some(BRIDGE_PROTOCOL_V1.to_string()),
        }
    }
}

#[derive(Debug)]
pub struct HostBridgeStream {
    pub command: HostCommand,
    pub session_id: Option<String>,
    pub receiver: UnboundedReceiver<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HostMode {
    Web,
    Desktop,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HostTransport {
    Http,
    Bridge,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostCapabilities {
    pub supports_streaming: bool,
    pub supports_file_dialog: bool,
    pub supports_terminal: bool,
    pub supports_terminal_pty: bool,
    pub supports_native_menu: bool,
}

impl HostCapabilities {
    pub fn web_default() -> Self {
        Self {
            supports_streaming: true,
            supports_file_dialog: true,
            supports_terminal: true,
            supports_terminal_pty: false,
            supports_native_menu: false,
        }
    }

    pub fn desktop_default() -> Self {
        Self {
            supports_streaming: true,
            supports_file_dialog: true,
            supports_terminal: true,
            supports_terminal_pty: true,
            supports_native_menu: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostDescriptor {
    pub mode: HostMode,
    pub transport: HostTransport,
    #[serde(flatten)]
    pub capabilities: HostCapabilities,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge_protocol: Option<String>,
}

impl HostDescriptor {
    pub fn web_http(capabilities: HostCapabilities) -> Self {
        Self {
            mode: HostMode::Web,
            transport: HostTransport::Http,
            capabilities,
            bridge_protocol: None,
        }
    }

    pub fn desktop_bridge(capabilities: HostCapabilities) -> Self {
        Self {
            mode: HostMode::Desktop,
            transport: HostTransport::Bridge,
            capabilities,
            bridge_protocol: Some(BRIDGE_PROTOCOL_V1.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{HostCommand, BRIDGE_PROTOCOL_V1};

    #[test]
    fn parses_known_bridge_commands() {
        assert_eq!(
            HostCommand::parse("workspace.file.save"),
            Some(HostCommand::WorkspaceFileSave)
        );
        assert_eq!(
            HostCommand::parse("chat.stream"),
            Some(HostCommand::ChatStream)
        );
        assert_eq!(HostCommand::parse("missing.command"), None);
    }

    #[test]
    fn uses_stable_protocol_name() {
        assert_eq!(BRIDGE_PROTOCOL_V1, "atlas-host-v1");
        assert_eq!(HostCommand::ChatSend.as_str(), "chat.send");
        assert_eq!(
            HostCommand::parse("prompt.optimize"),
            Some(HostCommand::PromptOptimize)
        );
    }
}
