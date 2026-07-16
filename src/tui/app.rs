//! TUI application core
//!
//! State machine, event loop, input handling, and LLM orchestration.
//! The render loop is synchronous on the main thread; LLM calls are
//! spawned as tokio tasks and communicate via mpsc channels.

use anyhow::Result;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tracing::info;

use crate::llm::ChatRequest;
use crate::llm::LLMProvider;
use crate::tui::commands::CommandRegistry;
use crate::tui::components::chat_panel::ChatPanel;
use crate::tui::components::input_bar::{InputBar, InputBarState};
use crate::tui::components::message_block::MessageBlock;
use crate::tui::components::permission_dialog::{
    PendingToolCall, PermissionAction, PermissionDialog,
};
use crate::tui::components::status_bar::{StatusBar, StatusBarState};
use crate::tui::event::AppEvent;
use crate::tui::layout::TuiLayout;
use crate::tui::streaming::{build_conversation, is_tool_call_finish, start_llm_stream};

/// Application mode / state
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    /// Pre-chat configuration screen
    Config,
    /// Pick a previous session or start new
    SessionPicker,
    /// Waiting for user input
    Idle,
    /// Streaming LLM response
    Streaming,
    /// Waiting for user to approve/reject tool calls
    WaitingForPermission,
    /// Executing approved tools
    ExecutingTool,
    /// Git-graph view of conversation history
    GraphView,
    /// Error state
    Error(String),
}

/// Main TUI application state
pub struct TuiApp {
    /// Whether the app is running
    pub running: bool,
    /// Current mode
    pub mode: AppMode,
    /// All messages in the conversation
    pub messages: Vec<MessageBlock>,
    /// Input bar state
    pub input: InputBarState,
    /// Status bar state
    pub status_bar: StatusBarState,
    /// Current scroll position in chat
    pub scroll_position: usize,
    /// Whether to auto-scroll to bottom on render
    pub auto_scroll: bool,
    /// LLM provider
    pub provider: Arc<dyn LLMProvider>,
    /// Tool definitions for the API (OpenAI format)
    pub tool_definitions: Option<Vec<serde_json::Value>>,
    /// Slash command registry
    pub commands: CommandRegistry,
    /// Channel receiver for async events from LLM stream
    stream_rx: UnboundedReceiver<AppEvent>,
    /// Channel sender (cloned for background tasks)
    stream_tx: UnboundedSender<AppEvent>,
    /// Abort handle for the current LLM stream (for Ctrl+C)
    abort_handle: Option<tokio::task::AbortHandle>,
    /// Auto-approve tool calls without prompting
    pub auto_approve_tools: bool,
    /// Pending tool calls from the latest assistant response
    pending_tool_calls: Vec<PendingToolCall>,
    /// Active agent/skill system prompt
    pub active_agent: crate::tui::agent_loader::ActiveAgent,
    /// Agent loader
    pub agent_loader: crate::tui::agent_loader::AgentLoader,
    /// Research pipeline for AI Scientist mode
    pub research: crate::tui::research_pipeline::ResearchPipeline,
    /// Privacy guard for research security
    pub privacy: crate::tui::privacy_guard::PrivacyGuard,
    /// Real tool executor: (tool_name, args) -> Result<result_string, error_string>
    pub tool_executor:
        Option<Arc<dyn Fn(&str, &serde_json::Value) -> Result<String, String> + Send + Sync>>,
    /// Config screen state
    pub config_state: crate::tui::components::ConfigScreenState,
    /// Frame counter for animation (breathing dot)
    pub frame_count: u64,
    /// Current status word during streaming
    pub status_word: String,
    /// Max tokens for LLM requests
    pub max_tokens: usize,
    /// Temperature for LLM requests
    pub temperature: f32,
    /// Security configuration
    pub security_config: crate::security::SecurityConfig,
    /// Session manager (persistent conversation history)
    pub session_manager: crate::tui::session::SessionManager,
    /// Session picker: currently highlighted index
    session_picker_idx: usize,
    /// Session picker: whether we're showing the picker
    session_picker_visible: bool,
    /// Ctrl+C pending flag — first press warns, second quits
    ctrl_c_pending: bool,
    /// When true, the next completed Assistant message is saved as session summary
    pub pending_summarize: bool,
    /// GraphView: session id being previewed
    graph_session_id: Option<String>,
    /// GraphView: currently highlighted node index
    pub graph_selected: usize,
    /// GraphView: total node count (set by render)
    graph_total: usize,
    /// GraphView: loaded messages for preview
    graph_messages: Vec<MessageBlock>,
    /// GraphView: branch metadata for the previewed session
    graph_branches: Vec<crate::tui::session::SessionBranch>,
    /// Current branch id for this session (for MessageBlock::User tagging)
    pub current_branch_id: String,
}

/// Thinking/reasoning depth — represents a fraction of the model's max output
#[derive(Debug, Clone, PartialEq)]
pub enum ThinkingLevel {
    /// 25% of model's max output
    Low,
    /// 50% of model's max output
    Medium,
    /// 100% of model's max output
    High,
    /// Custom absolute value
    Custom(usize, f32),
}

impl ThinkingLevel {
    pub fn label(&self) -> &str {
        match self {
            ThinkingLevel::Low => "Low",
            ThinkingLevel::Medium => "Medium",
            ThinkingLevel::High => "High",
            ThinkingLevel::Custom(_, _) => "Custom",
        }
    }

    /// Fraction of model's max_output_tokens (0.0 - 1.0)
    pub fn fraction(&self) -> f64 {
        match self {
            ThinkingLevel::Low => 0.25,
            ThinkingLevel::Medium => 0.50,
            ThinkingLevel::High => 1.0,
            ThinkingLevel::Custom(_, _) => 1.0,
        }
    }

    /// Compute actual max_tokens based on the model's API limit
    pub fn max_tokens_for(&self, model_name: &str) -> usize {
        match self {
            ThinkingLevel::Custom(t, _) => *t,
            _ => {
                let model_limit =
                    crate::tui::model_config::ModelRegistry::get_max_tokens(model_name, usize::MAX);
                let fraction = self.fraction();
                ((model_limit as f64) * fraction).max(256.0) as usize
            }
        }
    }

    pub fn temperature(&self) -> f32 {
        match self {
            ThinkingLevel::Low => 0.3,
            ThinkingLevel::Medium => 0.7,
            ThinkingLevel::High => 0.9,
            ThinkingLevel::Custom(_, t) => *t,
        }
    }
}

impl TuiApp {
    /// Create a new TUI application
    pub fn new(
        provider: Arc<dyn LLMProvider>,
        tool_definitions: Option<Vec<serde_json::Value>>,
        tool_executor: Option<
            Arc<dyn Fn(&str, &serde_json::Value) -> Result<String, String> + Send + Sync>,
        >,
        security_config: crate::security::SecurityConfig,
    ) -> Self {
        let (tx, rx) = unbounded_channel();
        let model = provider.default_model().to_string();
        let provider_name = provider.name().to_string();

        let api_key_preview = std::env::var("AI_API_KEY")
            .map(|k| {
                if k.len() > 12 {
                    format!("{}...{}", &k[..8], &k[k.len() - 4..])
                } else {
                    k
                }
            })
            .unwrap_or_else(|_| "none".to_string());

        let mut status_bar = StatusBarState::default();
        status_bar.model = model.clone();
        status_bar.provider = provider_name.clone();

        let config_state = crate::tui::components::ConfigScreenState::new(
            model.clone(),
            provider_name,
            api_key_preview,
        );

        let app = Self {
            running: true,
            mode: AppMode::Config,
            messages: Vec::new(),
            input: InputBarState::new(),
            status_bar,
            scroll_position: 0,
            auto_scroll: true,
            provider,
            tool_definitions,
            commands: CommandRegistry::new(),
            stream_rx: rx,
            stream_tx: tx,
            abort_handle: None,
            auto_approve_tools: false,
            pending_tool_calls: Vec::new(),
            active_agent: crate::tui::agent_loader::ActiveAgent::none(),
            agent_loader: crate::tui::agent_loader::AgentLoader::new(),
            research: crate::tui::research_pipeline::ResearchPipeline::new(),
            privacy: crate::tui::privacy_guard::PrivacyGuard::new(),
            tool_executor,
            config_state,
            frame_count: 0,
            status_word: String::new(),
            max_tokens: (crate::tui::model_config::ModelRegistry::get_max_tokens(&model, usize::MAX)
                as f64
                * 0.5) as usize,
            temperature: 0.7,
            security_config,
            session_manager: crate::tui::session::SessionManager::new().unwrap(),
            session_picker_idx: 0,
            session_picker_visible: false,
            ctrl_c_pending: false,
            pending_summarize: false,
            graph_session_id: None,
            graph_selected: 0,
            graph_total: 0,
            graph_messages: Vec::new(),
            graph_branches: Vec::new(),
            current_branch_id: "main".to_string(),
        };

        app
    }

    /// Add a message to the conversation
    pub fn add_message(&mut self, block: MessageBlock) {
        // Skip transient streaming blocks from persistence (thinking blocks ARE persisted)
        let persistable = !matches!(&block, MessageBlock::AssistantStreaming { .. });
        self.messages.push(block);
        self.auto_scroll = true; // Auto-scroll to bottom on new message

        if persistable && self.session_manager.has_current_session() {
            let _ = self.session_manager.save_messages(&self.messages);
        }
    }

    /// Clear all messages
    pub fn clear_messages(&mut self) {
        self.messages.clear();
        self.scroll_position = 0;
        self.auto_scroll = true;
        self.add_message(MessageBlock::System {
            content: "Conversation cleared.".to_string(),
        });
    }

    /// Handle a key event
    pub fn handle_key_event(
        &mut self,
        key: &crossterm::event::KeyEvent,
        rt: &tokio::runtime::Runtime,
    ) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        // ── Global Ctrl+C: two-step quit (like Claude Code) ──
        if ctrl && key.code == KeyCode::Char('c') {
            match self.mode {
                AppMode::Streaming | AppMode::ExecutingTool => {
                    // Cancel the current operation, save any partial text
                    self.abort_current_stream();
                    let partial = crate::tui::streaming::get_streaming_text(&self.messages);
                    self.remove_streaming_blocks();
                    if !partial.is_empty() {
                        self.add_message(MessageBlock::Assistant { content: partial });
                    }
                    self.add_message(MessageBlock::System {
                        content: "Cancelled. Press Ctrl+C again to exit.".to_string(),
                    });
                    self.mode = AppMode::Idle;
                    self.status_bar.mode_text = "Ready".to_string();
                    self.status_bar.error = None;
                    self.ctrl_c_pending = true;
                    return;
                }
                AppMode::Idle
                | AppMode::SessionPicker
                | AppMode::GraphView
                | AppMode::WaitingForPermission
                | AppMode::Error(_) => {
                    if self.ctrl_c_pending {
                        self.running = false;
                        return;
                    }
                    // First Ctrl+C when idle: go back to config screen
                    self.ctrl_c_pending = false;
                    self.mode = AppMode::Config;
                    self.status_bar.mode_text = "Config".to_string();
                    return;
                }
                AppMode::Config => {
                    // In config screen, Ctrl+C just exits
                    self.running = false;
                    return;
                }
            }
        }

        // Any other key clears the Ctrl+C pending flag
        self.ctrl_c_pending = false;

        match self.mode.clone() {
            AppMode::Config => self.handle_key_config(key),
            AppMode::SessionPicker => self.handle_key_session_picker(key),
            AppMode::GraphView => self.handle_key_graph_view(key),
            AppMode::Idle => self.handle_key_idle(key, rt),
            AppMode::Streaming => self.handle_key_streaming(key),
            AppMode::WaitingForPermission => self.handle_key_permission(key, rt),
            AppMode::ExecutingTool => {
                // Non-Ctrl+C key — ignore during tool execution
            }
            AppMode::Error(_) => {
                // Any key dismisses error, go back to idle
                self.mode = AppMode::Idle;
                self.status_bar.error = None;
                self.status_bar.mode_text = "Ready".to_string();
            }
        }
    }

    /// Handle key in Config mode
    fn handle_key_config(&mut self, key: &crossterm::event::KeyEvent) {
        // Handle key editing mode
        if self.config_state.editing_key {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.config_state.editing_key = false;
                }
                KeyCode::Backspace => {
                    self.config_state.pop_key_char();
                }
                KeyCode::Char(c) => {
                    self.config_state.push_key_char(c);
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.config_state.select_prev(),
            KeyCode::Down | KeyCode::Char('j') => self.config_state.select_next(),
            KeyCode::Left => {
                if self.config_state.selected_field
                    == crate::tui::components::ConfigField::ModelSelect
                {
                    self.config_state.prev_model();
                } else {
                    self.toggle_config_field();
                }
            }
            KeyCode::Right => {
                if self.config_state.selected_field
                    == crate::tui::components::ConfigField::ModelSelect
                {
                    self.config_state.next_model();
                } else {
                    self.toggle_config_field();
                }
            }
            KeyCode::Enter => match self.config_state.selected_field {
                crate::tui::components::ConfigField::KeyInput => {
                    self.config_state.editing_key = true;
                }
                crate::tui::components::ConfigField::Start => {
                    self.apply_config();
                }
                _ => {
                    self.toggle_config_field();
                }
            },
            KeyCode::Char('q') => self.running = false,
            _ => {}
        }
    }

    fn toggle_config_field(&mut self) {
        use crate::tui::components::ConfigField;
        match self.config_state.selected_field {
            ConfigField::DeepThink => self.config_state.deep_think = !self.config_state.deep_think,
            ConfigField::Competition => {
                self.config_state.competition_mode = !self.config_state.competition_mode
            }
            ConfigField::Privacy => {
                self.config_state.privacy_mode = !self.config_state.privacy_mode
            }
            ConfigField::ToolPermission => {
                self.config_state.security_level = self.config_state.security_level.next()
            }
            _ => {}
        }
    }

    /// Handle key in SessionPicker mode
    fn handle_key_session_picker(&mut self, key: &crossterm::event::KeyEvent) {
        let sessions = &self.session_manager.index;
        let max_idx = sessions.len(); // sessions.len() is the "New Conversation" option

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.session_picker_idx > 0 {
                    self.session_picker_idx -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.session_picker_idx < max_idx {
                    self.session_picker_idx += 1;
                }
            }
            KeyCode::Enter => {
                if self.session_picker_idx < sessions.len() {
                    let id = sessions[self.session_picker_idx].id.clone();
                    let model = sessions[self.session_picker_idx].model.clone();
                    match self.session_manager.load_messages(&id) {
                        Ok(msgs) => {
                            if msgs.is_empty() {
                                // Empty session: go straight to chat
                                self.session_manager.current_id = Some(id);
                                self.session_picker_visible = false;
                                self.mode = AppMode::Idle;
                                self.status_bar.model = model;
                                self.add_message(MessageBlock::System {
                                    content: "New conversation started.".to_string(),
                                });
                            } else {
                                self.graph_session_id = Some(id);
                                self.graph_selected = 0;
                                self.graph_messages = msgs;
                                self.graph_branches = self.session_manager.branches_for_session(
                                    self.graph_session_id.as_deref().unwrap_or(""),
                                );
                                self.mode = AppMode::GraphView;
                            }
                        }
                        Err(e) => {
                            self.mode = AppMode::Error(format!("Failed to load session: {}", e));
                        }
                    }
                } else {
                    // "New Conversation" selected
                    self.session_picker_visible = false;
                    self.mode = AppMode::Idle;
                    let model = self.status_bar.model.clone();
                    let _ = self.session_manager.create_session(&model);
                    self.add_message(MessageBlock::System {
                        content: format!(
                            "Ready - Deep Think: {}, {} tokens (temp {})",
                            if self.config_state.deep_think {
                                "ON"
                            } else {
                                "OFF"
                            },
                            self.max_tokens,
                            self.temperature,
                        ),
                    });
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if self.session_picker_idx < sessions.len() {
                    let id = sessions[self.session_picker_idx].id.clone();
                    let title = sessions[self.session_picker_idx].title.clone();
                    // Confirm: remove the session
                    if let Err(e) = self.session_manager.delete_session(&id) {
                        self.mode = AppMode::Error(format!("Delete failed: {}", e));
                    } else {
                        // Reset selection if needed
                        if self.session_picker_idx >= self.session_manager.index.len() {
                            self.session_picker_idx =
                                self.session_manager.index.len().saturating_sub(1);
                        }
                        // If all sessions deleted, go to idle
                        if self.session_manager.index.is_empty() {
                            self.session_picker_visible = false;
                            self.mode = AppMode::Idle;
                            let model = self.status_bar.model.clone();
                            let _ = self.session_manager.create_session(&model);
                        }
                    }
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                // Shortcut: new conversation
                self.session_picker_visible = false;
                self.mode = AppMode::Idle;
                let model = self.status_bar.model.clone();
                let _ = self.session_manager.create_session(&model);
                self.add_message(MessageBlock::System {
                    content: format!(
                        "Ready - Deep Think: {}, {} tokens (temp {})",
                        if self.config_state.deep_think {
                            "ON"
                        } else {
                            "OFF"
                        },
                        self.max_tokens,
                        self.temperature,
                    ),
                });
            }
            KeyCode::Char('q') => self.running = false,
            _ => {}
        }
    }

    /// Handle key in GraphView mode
    fn handle_key_graph_view(&mut self, key: &crossterm::event::KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                if let Some(ref id) = self.graph_session_id.clone() {
                    let id = id.clone();
                    match self.session_manager.resume_session(&id) {
                        Ok(msgs) => {
                            if let Some(meta) =
                                self.session_manager.index.iter().find(|m| m.id == id)
                            {
                                self.status_bar.model = meta.model.clone();
                            }

                            // Check if selected node is the LAST on its own branch
                            let user_nodes: Vec<(usize, String)> = msgs
                                .iter()
                                .enumerate()
                                .filter_map(|(msg_i, m)| {
                                    if let MessageBlock::User { branch_id, .. } = m {
                                        Some((
                                            msg_i,
                                            if branch_id.is_empty() {
                                                "main".to_string()
                                            } else {
                                                branch_id.clone()
                                            },
                                        ))
                                    } else {
                                        None
                                    }
                                })
                                .collect();

                            let current_branch = user_nodes
                                .get(self.graph_selected)
                                .map(|(_, bid)| {
                                    if bid.is_empty() {
                                        "main".to_string()
                                    } else {
                                        bid.clone()
                                    }
                                })
                                .unwrap_or_else(|| "main".to_string());
                            // Find the last NODE INDEX (not msg index) on this branch
                            let last_node_on_branch = user_nodes
                                .iter()
                                .enumerate()
                                .filter(|(_, (_, bid))| bid == &current_branch)
                                .map(|(node_i, _)| node_i)
                                .max()
                                .unwrap_or(self.graph_selected);
                            let is_last_on_branch = self.graph_selected >= last_node_on_branch;

                            if user_nodes.is_empty() || is_last_on_branch {
                                // Last node on its branch: continue on same branch
                                self.messages = msgs;
                                self.current_branch_id = current_branch.clone();
                                self.graph_session_id = None;
                                self.graph_messages.clear();
                                self.graph_branches.clear();
                                self.mode = AppMode::Idle;
                                self.add_message(MessageBlock::System {
                                    content: "Resumed conversation.".to_string(),
                                });
                            } else {
                                // Non-last node: fork new branch, keep all messages
                                let fork_id = self
                                    .session_manager
                                    .fork_at_node(self.graph_selected, &current_branch)
                                    .unwrap_or_else(|_| {
                                        format!("fork-{}", self.graph_selected + 1)
                                    });
                                self.current_branch_id = fork_id.clone();

                                // Keep full history; new messages tagged with fork branch_id
                                self.messages = msgs;
                                self.graph_session_id = None;
                                self.graph_messages.clear();
                                self.graph_branches.clear();
                                self.mode = AppMode::Idle;
                                self.add_message(MessageBlock::System {
                                    content: format!(
                                        "Forked from question #{} on '{}' as branch '{}'. All messages preserved. \
                                         New replies go to '{}'. Use /branches to list.",
                                        self.graph_selected + 1, current_branch, fork_id, fork_id,
                                    ),
                                });
                            }
                        }
                        Err(e) => {
                            self.mode = AppMode::Error(format!("Load failed: {}", e));
                        }
                    }
                }
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                // Back to session picker
                self.graph_session_id = None;
                self.graph_messages.clear();
                self.graph_branches.clear();
                self.graph_selected = 0;
                self.mode = AppMode::SessionPicker;
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if let Some(ref id) = self.graph_session_id.clone() {
                    let id = id.clone();
                    if let Err(e) = self.session_manager.delete_session(&id) {
                        self.mode = AppMode::Error(format!("Delete failed: {}", e));
                    } else {
                        self.graph_session_id = None;
                        self.graph_messages.clear();
                        self.graph_branches.clear();
                        self.graph_selected = 0;
                        self.session_picker_idx = self
                            .session_picker_idx
                            .min(self.session_manager.index.len().saturating_sub(1));
                        if self.session_manager.index.is_empty() {
                            self.session_picker_visible = false;
                            self.mode = AppMode::Idle;
                            let model = self.status_bar.model.clone();
                            let _ = self.session_manager.create_session(&model);
                        } else {
                            self.mode = AppMode::SessionPicker;
                        }
                    }
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.graph_selected = self.graph_selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.graph_selected = self
                    .graph_selected
                    .saturating_add(1)
                    .min(self.graph_total.saturating_sub(1));
            }
            KeyCode::PageUp => {
                self.graph_selected = self.graph_selected.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.graph_selected =
                    (self.graph_selected + 10).min(self.graph_total.saturating_sub(1));
            }
            _ => {}
        }
    }

    /// Apply the config settings and start chatting
    fn apply_config(&mut self) {
        let model = self.config_state.model_name.clone();
        let model_limit =
            crate::tui::model_config::ModelRegistry::get_max_tokens(&model, usize::MAX);
        if self.config_state.deep_think {
            self.max_tokens = model_limit;
            self.temperature = 0.9;
        } else {
            self.max_tokens = (model_limit as f64 * 0.5) as usize;
            self.temperature = 0.7;
        }
        self.research.competition_mode = self.config_state.competition_mode;
        self.privacy.enforced = self.config_state.privacy_mode;

        // Sync security config from config screen
        // Security Level drives both auto_approve and max_risk:
        //   Strict: auto_approve=false (all tools ask for confirmation)
        //   Standard: auto_approve=true, max_risk=Safe (auto safe, dialog for moderate+)
        //   Permissive: auto_approve=true, max_risk=Moderate (auto safe+moderate, dialog for dangerous)
        let level = self.config_state.security_level;
        self.auto_approve_tools = level.auto_approve_enabled();
        self.security_config.auto_approve_tools = level.auto_approve_enabled();
        self.security_config.max_auto_approve_risk = level.max_auto_risk();

        // Build the provider URL from the selected model's known API
        let info = self.config_state.selected_model_info();
        let api_url = info
            .map(|i| match i.provider {
                "deepseek" => "https://api.deepseek.com/v1/chat/completions",
                "openai" => "https://api.openai.com/v1/chat/completions",
                "moonshot" => "https://api.moonshot.cn/v1/chat/completions",
                "qwen" => "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions",
                "zhipu" => "https://open.bigmodel.cn/api/paas/v4/chat/completions",
                "anthropic" => "https://api.anthropic.com/v1/messages",
                "ollama" => "http://localhost:11434/v1/chat/completions",
                _ => "https://api.deepseek.com/v1/chat/completions",
            })
            .unwrap_or("https://api.deepseek.com/v1/chat/completions");

        // API key: custom > env > empty
        let api_key = if !self.config_state.custom_key.is_empty() {
            self.config_state.custom_key.clone()
        } else {
            std::env::var("AI_API_KEY").unwrap_or_default()
        };

        // Create provider with selected model's URL
        self.provider = Arc::new(crate::llm::providers::OpenAIProvider::with_base_url(
            api_key,
            api_url.to_string(),
            Some(model.clone()),
        ));

        self.status_bar.model = model.clone();
        self.status_bar.provider = info
            .map(|i| i.provider.to_string())
            .unwrap_or_else(|| "custom".to_string());

        // Show session picker if history exists, otherwise jump straight to chat
        if self.session_manager.index.is_empty() {
            self.mode = AppMode::Idle;
            let _ = self.session_manager.create_session(&model);
        } else {
            self.session_picker_visible = true;
            self.session_picker_idx = 0;
            self.mode = AppMode::SessionPicker;
        }
    }

    /// Handle key in Idle mode
    fn handle_key_idle(&mut self, key: &crossterm::event::KeyEvent, rt: &tokio::runtime::Runtime) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            KeyCode::Char('q') if ctrl => {
                self.running = false;
            }
            KeyCode::PageUp => {
                self.auto_scroll = false;
                self.scroll_position = self.scroll_position.saturating_sub(3);
            }
            KeyCode::PageDown => {
                self.scroll_position = self.scroll_position.saturating_add(3);
                // Re-enable auto-scroll if scrolled to bottom
                // (handled implicitly by render logic)
            }
            _ => {
                // Try input handling
                if let Some(text) = self.input.handle_key(key) {
                    // Check for slash command
                    if let Some(cmd_name) = self.commands.match_command(&text) {
                        let cmd_name = cmd_name.to_string();
                        let parts: Vec<&str> = text[1..].splitn(2, ' ').collect();
                        let args = parts.get(1).unwrap_or(&"");
                        let result = CommandRegistry::execute(&cmd_name, self, args);
                        match result {
                            crate::tui::commands::CommandResult::Quit => {
                                self.running = false;
                            }
                            crate::tui::commands::CommandResult::Message(msg) => {
                                self.add_message(MessageBlock::System { content: msg });
                            }
                            crate::tui::commands::CommandResult::SendMessage(msg) => {
                                self.send_user_message(&msg, rt);
                            }
                            crate::tui::commands::CommandResult::Handled => {}
                        }
                    } else {
                        self.send_user_message(&text, rt);
                    }
                }
            }
        }
    }

    /// Handle key during streaming
    fn handle_key_streaming(&mut self, key: &crossterm::event::KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            KeyCode::PageUp => {
                self.auto_scroll = false;
                self.scroll_position = self.scroll_position.saturating_sub(3);
            }
            KeyCode::PageDown => {
                self.scroll_position = self.scroll_position.saturating_add(3);
            }
            _ => {}
        }
    }

    /// Handle key during permission dialog
    fn handle_key_permission(
        &mut self,
        key: &crossterm::event::KeyEvent,
        rt: &tokio::runtime::Runtime,
    ) {
        match PermissionDialog::handle_key(key) {
            PermissionAction::ApproveAll => {
                let pending = std::mem::take(&mut self.pending_tool_calls);
                self.execute_tools(pending, rt);
            }
            PermissionAction::DenyAll => {
                let call_ids: Vec<String> = self
                    .pending_tool_calls
                    .iter()
                    .map(|tc| tc.call_id.clone())
                    .collect();
                let tc_names: Vec<(String, String, serde_json::Value)> = self
                    .pending_tool_calls
                    .iter()
                    .map(|tc| (tc.call_id.clone(), tc.name.clone(), tc.args.clone()))
                    .collect();
                for (call_id, name, args) in &tc_names {
                    if let Some(block) = self.find_tool_call_block_mut(call_id) {
                        *block = MessageBlock::ToolCall {
                            name: name.clone(),
                            args: args.clone(),
                            call_id: call_id.clone(),
                            status: crate::tui::components::message_block::ToolCallStatus::Denied(
                                "User denied".to_string(),
                            ),
                        };
                    }
                }
                self.pending_tool_calls.clear();
                self.mode = AppMode::Idle;
                self.status_bar.mode_text = "Ready".to_string();
            }
            PermissionAction::Approve(_) => {
                let pending = std::mem::take(&mut self.pending_tool_calls);
                self.execute_tools(pending, rt);
            }
            PermissionAction::Deny(_) => {
                if let Some(tc) = self.pending_tool_calls.first().cloned() {
                    if let Some(block) = self.find_tool_call_block_mut(&tc.call_id) {
                        *block = MessageBlock::ToolCall {
                            name: tc.name.clone(),
                            args: tc.args.clone(),
                            call_id: tc.call_id.clone(),
                            status: crate::tui::components::message_block::ToolCallStatus::Denied(
                                "User denied".to_string(),
                            ),
                        };
                    }
                    self.pending_tool_calls.remove(0);
                }
                if self.pending_tool_calls.is_empty() {
                    self.mode = AppMode::Idle;
                    self.status_bar.mode_text = "Ready".to_string();
                }
            }
            PermissionAction::None => {}
        }
    }

    /// Send a user message to the LLM
    fn send_user_message(&mut self, text: &str, rt: &tokio::runtime::Runtime) {
        self.add_message(MessageBlock::User {
            content: text.to_string(),
            branch_id: self.current_branch_id.clone(),
        });
        self.start_llm_stream(rt);
    }

    /// Rotate status word based on frame count and context
    fn update_status_word(&mut self) {
        if self.mode != AppMode::Streaming && self.mode != AppMode::ExecutingTool {
            return;
        }
        let words = if self.research.active {
            match self.research.phase {
                crate::tui::research_pipeline::ResearchPhase::LiteratureReview => {
                    &["Searching", "Reading", "Analyzing", "Extracting"][..]
                }
                crate::tui::research_pipeline::ResearchPhase::HypothesisGeneration => {
                    &["Reasoning", "Synthesizing", "Generating", "Formulating"][..]
                }
                crate::tui::research_pipeline::ResearchPhase::ExperimentDesign => {
                    &["Designing", "Planning", "Structuring", "Engineering"][..]
                }
                crate::tui::research_pipeline::ResearchPhase::Execution => {
                    &["Executing", "Running", "Computing", "Processing"][..]
                }
                crate::tui::research_pipeline::ResearchPhase::Validation => {
                    &["Validating", "Testing", "Evaluating", "Measuring"][..]
                }
                crate::tui::research_pipeline::ResearchPhase::PaperWriting => {
                    &["Writing", "Composing", "Drafting", "Structuring"][..]
                }
                _ => &["Thinking", "Processing", "Working"][..],
            }
        } else if self.mode == AppMode::ExecutingTool {
            &["Executing", "Running", "Calling"][..]
        } else {
            &["Thinking", "Analyzing", "Reasoning", "Computing"][..]
        };
        let idx = (self.frame_count as usize / 60) % words.len();
        self.status_word = words[idx].to_string();
    }

    fn sync_reviewer_feedback_status(&mut self) {
        self.status_bar.reviewer_open_items = self.research.unresolved_feedback_count();
        if self.research.phase == crate::tui::research_pipeline::ResearchPhase::Review
            && !self.research.reviewer_feedback.is_empty()
        {
            self.research.show_reviewer_panel();
        }
    }

    /// Start streaming from the LLM
    fn start_llm_stream(&mut self, rt: &tokio::runtime::Runtime) {
        // Update privacy level for current research phase
        if self.research.active {
            self.privacy.update_for_phase(&self.research.phase);
        }
        self.status_bar.privacy_enforced = self.privacy.enforced;
        self.status_bar.privacy_level = if self.privacy.enforced {
            self.privacy.level.label().to_string()
        } else {
            "OFF".to_string()
        };

        // Privacy check: block if confidential content would go to cloud
        let is_cloud = !self.privacy.local_model_available || self.privacy.level.allows_cloud();
        match self.privacy.is_safe_to_send(is_cloud) {
            crate::tui::privacy_guard::SafetyVerdict::Blocked { reason } => {
                self.add_message(MessageBlock::System {
                    content: format!(
                        "Privacy blocked: {}\n\n\
                         Configure a local model (Ollama) to proceed with confidential phases.\n\
                         Use /privacy to check status.",
                        reason
                    ),
                });
                return;
            }
            crate::tui::privacy_guard::SafetyVerdict::Warning { ref message } => {
                self.add_message(MessageBlock::System {
                    content: format!("Privacy warning: {}", message),
                });
            }
            crate::tui::privacy_guard::SafetyVerdict::Allowed => {}
        }

        self.mode = AppMode::Streaming;
        self.status_bar.mode_text = "Streaming...".to_string();
        self.status_bar.error = None;

        // Add a streaming placeholder
        self.add_message(MessageBlock::AssistantStreaming {
            content: String::new(),
        });

        // Determine system prompt priority: Research Pipeline > Active Agent > auto-matched research skill > Default
        let agent_prompt = if self.research.active {
            let rp = self.research.system_prompt();
            if !rp.is_empty() {
                Some(rp)
            } else {
                None
            }
        } else if self.active_agent.is_active {
            Some(self.active_agent.prompt.clone())
        } else {
            let user_text = self
                .messages
                .iter()
                .rev()
                .find_map(|block| {
                    if let MessageBlock::User { content, .. } = block {
                        Some(content.as_str())
                    } else {
                        None
                    }
                })
                .unwrap_or("");
            let matched = self
                .agent_loader
                .auto_match_research_agents(user_text, Some("agent"));
            if matched.is_empty() {
                None
            } else {
                Some(
                    matched
                        .iter()
                        .map(|agent| agent.prompt.as_str())
                        .collect::<Vec<_>>()
                        .join("\n\n"),
                )
            }
        };
        let agent_prompt_ref = agent_prompt.as_deref();
        let messages = build_conversation(
            &self.messages[..self.messages.len().saturating_sub(1)],
            agent_prompt_ref,
        );

        // Don't send empty tools array — filter out None/empty
        let tools = self.tool_definitions.as_ref().and_then(|td| {
            if td.is_empty() {
                None
            } else {
                Some(td.clone())
            }
        });

        let model_name = self.provider.default_model().to_string();

        let request = ChatRequest {
            model: model_name,
            messages,
            multimodal_content: None,
            temperature: self.temperature,
            max_tokens: Some(self.max_tokens),
            top_p: None,
            stop: None,
            stream: true,
            tools,
            thinking_mode: None,
            reasoning_effort: None,
        };

        let provider = Arc::clone(&self.provider);
        let tx = self.stream_tx.clone();
        let handle = start_llm_stream(provider, request, tx, rt);
        self.abort_handle = Some(handle);
    }

    /// Handle an app event from the streaming channel
    pub fn handle_app_event(&mut self, event: AppEvent, rt: &tokio::runtime::Runtime) {
        match event {
            AppEvent::StreamChunk {
                content,
                tool_calls,
                finish_reason,
                usage,
            } => {
                // Update the streaming block
                self.update_streaming_block(&content);

                // Update token usage if present
                if let Some(u) = usage {
                    self.status_bar.tokens_used = u.total_tokens;
                }

                // Check if streaming is complete
                if is_tool_call_finish(&finish_reason)
                    || (finish_reason.is_some() && tool_calls.is_some())
                {
                    self.finish_stream_with_tools(tool_calls, rt);
                } else if finish_reason.is_some() {
                    self.finish_stream(rt);
                }
            }
            AppEvent::StreamComplete => {
                // If no tool calls were detected during stream, finish as text
                if self.mode == AppMode::Streaming {
                    self.finish_stream(rt);
                }
            }
            AppEvent::StreamError(_err) => {
                self.remove_streaming_blocks();
                self.mode = AppMode::Idle;
                self.status_bar.mode_text = "Error".to_string();
                self.status_bar.error = Some("stream".to_string());
                self.abort_handle = None;
            }
            AppEvent::ToolResult {
                call_id,
                result,
                success,
            } => {
                // Update the ToolCall block status (keep it for API conversation history)
                // and add a separate ToolResult block for display
                if let Some(block) = self.find_tool_call_block_mut(&call_id) {
                    let status = if success {
                        crate::tui::components::message_block::ToolCallStatus::Complete
                    } else {
                        crate::tui::components::message_block::ToolCallStatus::Failed(
                            result.clone(),
                        )
                    };
                    if let MessageBlock::ToolCall {
                        name,
                        args,
                        call_id: id,
                        ..
                    } = block
                    {
                        *block = MessageBlock::ToolCall {
                            name: name.clone(),
                            args: args.clone(),
                            call_id: id.clone(),
                            status,
                        };
                    }
                }
                // Add the tool result as a separate block (preserves tool_calls in history)
                self.add_message(MessageBlock::ToolResult {
                    call_id: call_id.clone(),
                    result: result.clone(),
                    success,
                });

                // Auto-detect file write and show diff
                if success {
                    if let Some(tc) = self
                        .pending_tool_calls
                        .iter()
                        .find(|t| t.call_id == call_id)
                    {
                        if let Some(diff) = crate::tui::components::diff_viewer::detect_file_write(
                            &tc.name, &tc.args, &result,
                        ) {
                            self.add_message(MessageBlock::Diff { diff });
                        }
                    }
                }

                self.pending_tool_calls.retain(|tc| tc.call_id != call_id);

                if self.pending_tool_calls.is_empty() {
                    // All tools executed, stream again to get final LLM response
                    self.start_llm_stream(rt);
                }
            }
        }
    }

    /// Update the streaming block with new content
    fn update_streaming_block(&mut self, delta: &str) {
        if let Some(MessageBlock::AssistantStreaming { content }) = self.messages.last_mut() {
            content.push_str(delta);
        }
        self.auto_scroll = true;
    }

    /// Finish streaming as a text response (no tool calls)
    fn finish_stream(&mut self, rt: &tokio::runtime::Runtime) {
        let content = self.extract_streaming_content();
        self.remove_streaming_blocks();
        if !content.is_empty() {
            self.add_message(MessageBlock::Assistant {
                content: content.clone(),
            });
            // If this was a /summarize response, save it as session summary
            if self.pending_summarize {
                self.pending_summarize = false;
                let summary: String = content.chars().take(200).collect();
                if let Err(e) = self.session_manager.set_summary(&summary) {
                    self.add_message(MessageBlock::System {
                        content: format!("Failed to save summary: {}", e),
                    });
                } else {
                    self.add_message(MessageBlock::System {
                        content: format!("Summary saved: {}", summary),
                    });
                }
            }
            // Auto-advance research pipeline if active
            if self.research.active {
                self.research.record(content);
                if self.research.competition_mode {
                    // Competition mode: pause for human checkpoints
                    self.research.waiting_approval = true;
                    self.research.show_reviewer_panel();
                    let phase = self.research.phase.label().to_string();
                    let next_phase = self.research.phase.next();
                    self.add_message(MessageBlock::System {
                        content: format!(
                            "[CHECKPOINT] Phase **{}** complete.\n\n\
                             Next: **{}**\n\n\
                             Type `/approve` to continue or `/stop` to end.",
                            phase,
                            next_phase.label(),
                        ),
                    });
                } else {
                    // Autonomous mode: auto-advance
                    self.research.advance();
                    if self.research.phase == crate::tui::research_pipeline::ResearchPhase::Complete
                    {
                        let ctx = self.research.full_context();
                        self.research.stop();
                        self.add_message(MessageBlock::System {
                            content: format!("Research pipeline complete!\n\n{}", ctx),
                        });
                    } else {
                        let phase = self.research.phase.label().to_string();
                        self.add_message(MessageBlock::System {
                            content: format!("-> Auto-advancing to: **{}**", phase),
                        });
                        let instructions = self.research.system_prompt();
                        self.send_user_message(&format!("Continue research. {}", instructions), rt);
                        return;
                    }
                }
            }
        } else {
            self.add_message(MessageBlock::System {
                content: "[No response received]".to_string(),
            });
        }
        self.mode = AppMode::Idle;
        self.status_bar.mode_text = "Ready".to_string();
        self.status_bar.tool_calls += 1;
        self.abort_handle = None;
    }

    /// Extract accumulated streaming text content
    fn extract_streaming_content(&self) -> String {
        for msg in self.messages.iter().rev() {
            if let MessageBlock::AssistantStreaming { content } = msg {
                return content.clone();
            }
        }
        String::new()
    }

    /// Finish streaming with tool calls detected
    fn finish_stream_with_tools(
        &mut self,
        tool_calls: Option<Vec<serde_json::Value>>,
        rt: &tokio::runtime::Runtime,
    ) {
        // Extract text content (before tool calls) then remove streaming block
        let text_before_tools = self.extract_streaming_content();
        self.remove_streaming_blocks();
        if !text_before_tools.is_empty() {
            self.add_message(MessageBlock::Assistant {
                content: text_before_tools.clone(),
            });
            // Check if this was a /summarize response (text before tool calls)
            if self.pending_summarize {
                self.pending_summarize = false;
                let summary: String = text_before_tools.chars().take(200).collect();
                if let Err(e) = self.session_manager.set_summary(&summary) {
                    self.add_message(MessageBlock::System {
                        content: format!("Failed to save summary: {}", e),
                    });
                } else {
                    self.add_message(MessageBlock::System {
                        content: format!("Summary saved: {}", summary),
                    });
                }
            }
        }

        let mut pending = Vec::new();

        if let Some(tc_list) = tool_calls {
            for tc in &tc_list {
                let id = tc
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let func = tc.get("function").unwrap_or(&serde_json::Value::Null);
                let name = func
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let args = func
                    .get("arguments")
                    .and_then(|v| v.as_str())
                    .unwrap_or("{}")
                    .to_string();
                let args_value: serde_json::Value =
                    serde_json::from_str(&args).unwrap_or(serde_json::json!({}));

                let call_id = if id.is_empty() {
                    format!("call_{}", uuid::Uuid::new_v4())
                } else {
                    id.clone()
                };

                pending.push(PendingToolCall {
                    call_id: call_id.clone(),
                    name: name.clone(),
                    args: args_value.clone(),
                });

                self.add_message(MessageBlock::ToolCall {
                    name,
                    args: args_value,
                    call_id,
                    status: crate::tui::components::message_block::ToolCallStatus::Pending,
                });
            }
        }

        if pending.is_empty() {
            self.finish_stream(rt);
            return;
        }

        self.pending_tool_calls = pending;

        // Decide: auto-execute or show permission dialog?
        // Check risk level of each tool against the configured max_auto_approve_risk
        let needs_confirmation = if self.auto_approve_tools {
            // Auto-approve is on: check if any tool exceeds the risk threshold
            self.pending_tool_calls.iter().any(|tc| {
                let risk = crate::security::default_tool_risk_map()
                    .get(&tc.name)
                    .cloned()
                    .unwrap_or(crate::tool_matrix::matrix::RiskLevel::Moderate);
                risk > self.security_config.max_auto_approve_risk
            })
        } else {
            // Strict mode: always confirm
            true
        };

        if needs_confirmation {
            self.mode = AppMode::WaitingForPermission;
            self.status_bar.mode_text = "Permission required".to_string();
        } else {
            let pending = std::mem::take(&mut self.pending_tool_calls);
            self.status_bar.mode_text = "Executing tools...".to_string();
            self.mode = AppMode::ExecutingTool;
            self.pending_tool_calls = pending;
        }
    }

    /// Execute tools (called from the event loop with runtime access)
    pub fn execute_pending_tools_if_needed(&mut self, rt: &tokio::runtime::Runtime) {
        if self.mode == AppMode::ExecutingTool && !self.pending_tool_calls.is_empty() {
            let pending = std::mem::take(&mut self.pending_tool_calls);
            self.execute_tools(pending, rt);
        }
    }

    /// Execute a list of tool calls
    fn execute_tools(&mut self, pending: Vec<PendingToolCall>, rt: &tokio::runtime::Runtime) {
        self.mode = AppMode::ExecutingTool;
        if let Some(tc) = pending.first() {
            self.status_word = format!("Executing {}", tc.name);
        }
        self.status_bar.mode_text = "Executing tools...".to_string();

        if pending.is_empty() {
            self.mode = AppMode::Idle;
            self.status_bar.mode_text = "Ready".to_string();
            return;
        }

        // Update all pending blocks to Approved
        for tc in &pending {
            if let Some(block) = self.find_tool_call_block_mut(&tc.call_id) {
                *block = MessageBlock::ToolCall {
                    name: tc.name.clone(),
                    args: tc.args.clone(),
                    call_id: tc.call_id.clone(),
                    status: crate::tui::components::message_block::ToolCallStatus::Approved,
                };
            }
        }

        let tx = self.stream_tx.clone();
        let executor = self.tool_executor.clone();

        // Spawn tool execution as background tasks so the render loop
        // continues to animate the breathing indicator while tools run.
        for tc in &pending {
            let call_id = tc.call_id.clone();
            let name = tc.name.clone();
            let args = tc.args.clone();
            let tx = tx.clone();

            // Rate limit check before execution
            if let Err(rate_err) = self.security_config.rate_limiter.check(&name) {
                if let Some(block) = self.find_tool_call_block_mut(&call_id) {
                    if let MessageBlock::ToolCall {
                        name: n,
                        args: a,
                        call_id: id,
                        ..
                    } = block
                    {
                        *block = MessageBlock::ToolCall {
                            name: n.clone(),
                            args: a.clone(),
                            call_id: id.clone(),
                            status: crate::tui::components::message_block::ToolCallStatus::Denied(
                                format!("Rate limit: {}", rate_err),
                            ),
                        };
                    }
                }
                let _ = tx.send(crate::tui::event::AppEvent::ToolResult {
                    call_id: call_id.clone(),
                    result: format!("Rate limited: {}", rate_err),
                    success: false,
                });
                continue;
            }

            // Update to Executing status (visible immediately on next render)
            if let Some(block) = self.find_tool_call_block_mut(&call_id) {
                if let MessageBlock::ToolCall {
                    name: n,
                    args: a,
                    call_id: id,
                    ..
                } = block
                {
                    *block = MessageBlock::ToolCall {
                        name: n.clone(),
                        args: a.clone(),
                        call_id: id.clone(),
                        status: crate::tui::components::message_block::ToolCallStatus::Executing,
                    };
                }
            }

            let exec = executor.clone();
            rt.spawn(async move {
                let (result, success) = if let Some(ref exec) = exec {
                    match exec(&name, &args) {
                        Ok(output) => (output, true),
                        Err(e) => (format!("Tool error: {}", e), false),
                    }
                } else {
                    (
                        format!(
                            "No tool executor. Tool '{}' called with args: {}",
                            name,
                            serde_json::to_string(&args).unwrap_or_default()
                        ),
                        false,
                    )
                };

                let _ = tx.send(AppEvent::ToolResult {
                    call_id: call_id.clone(),
                    result,
                    success,
                });
            });

            self.status_bar.tool_calls += 1;
        }
    }

    /// Render the session picker UI
    fn render_session_picker(&self, frame: &mut ratatui::Frame) {
        use ratatui::{
            layout::{Alignment, Rect},
            style::{Color, Modifier, Style},
            text::{Line, Span},
            widgets::{Block, Borders, Paragraph},
        };

        let area = frame.size();
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Atlas AI - Sessions ")
            .title_alignment(Alignment::Center);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let x = inner.x + 2;
        let mut y = inner.y + 1;
        let w = inner.width.saturating_sub(4);

        // Header
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "Recent Conversations",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ))),
            Rect::new(x, y, w, 1),
        );
        y += 2;

        let sessions = &self.session_manager.index;

        if sessions.is_empty() {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    "  No previous conversations. Starting fresh...",
                    Style::default().fg(Color::DarkGray),
                )),
                Rect::new(x, y, w, 1),
            );
            return;
        }

        let list_start = (self.session_picker_idx / 6) * 6;
        let list_end = (list_start + 6).min(sessions.len());

        for i in list_start..list_end {
            let session = &sessions[i];
            let is_selected = i == self.session_picker_idx;

            let cursor = if is_selected { ">" } else { " " };
            let highlight = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };

            let info = format!(
                "{}  {}  {}{} messages",
                session.updated_at,
                session.model,
                session.message_count,
                if session.message_count == 0 { "+" } else { "" },
            );

            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(
                        format!("{} ", cursor),
                        if is_selected {
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        },
                    ),
                    Span::styled(&session.title, highlight),
                ])),
                Rect::new(x, y, w, 1),
            );
            y += 1;

            // Show AI summary if available, otherwise show meta info
            if !session.summary.is_empty() {
                let summary_display: String = session.summary.chars().take(w as usize).collect();
                frame.render_widget(
                    Paragraph::new(Span::styled(
                        format!("   {}", summary_display),
                        if is_selected {
                            Style::default().fg(Color::Gray)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        },
                    )),
                    Rect::new(x, y, w, 1),
                );
                y += 1;
            }

            frame.render_widget(
                Paragraph::new(Span::styled(
                    format!("   {}", info),
                    Style::default().fg(Color::DarkGray),
                )),
                Rect::new(x, y, w, 1),
            );
            y += 2;
        }

        // "New Conversation" option
        let new_idx = sessions.len();
        let is_new_selected = self.session_picker_idx == new_idx;
        let cursor = if is_new_selected { ">" } else { " " };
        let highlight = if is_new_selected {
            Style::default().fg(Color::Black).bg(Color::Green)
        } else {
            Style::default().fg(Color::Green)
        };

        y += 1;
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!("{} ", cursor),
                    if is_new_selected {
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
                Span::styled("[N] New Conversation", highlight),
            ])),
            Rect::new(x, y, w, 1),
        );

        // Help bar
        let help_y = inner.y + inner.height.saturating_sub(2);
        frame.render_widget(
            Paragraph::new(Span::styled(
                " j/k or Up/Down select  Enter graph  N new  D delete  Q quit",
                Style::default().fg(Color::DarkGray),
            )),
            Rect::new(inner.x + 1, help_y, inner.width.saturating_sub(2), 1),
        );
    }

    /// Find a tool call block by call_id and return a mutable reference
    fn find_tool_call_block_mut(&mut self, call_id: &str) -> Option<&mut MessageBlock> {
        self.messages
            .iter_mut()
            .rev()
            .find(|m| matches!(m, MessageBlock::ToolCall { call_id: id, .. } if id == call_id))
    }

    /// Remove all streaming blocks from messages
    fn remove_streaming_blocks(&mut self) {
        self.messages
            .retain(|m| !matches!(m, MessageBlock::AssistantStreaming { .. }));
    }

    /// Abort the current LLM stream
    fn abort_current_stream(&mut self) {
        if let Some(handle) = self.abort_handle.take() {
            handle.abort();
        }
    }

    /// Handle paste event
    pub fn handle_paste(&mut self, text: &str) {
        for ch in text.chars() {
            if ch == '\n' || ch == '\r' {
                continue; // Skip newlines in paste
            }
            let byte_pos = self.input.byte_pos(self.input.cursor);
            self.input.buffer.insert_str(byte_pos, &ch.to_string());
            self.input.cursor += 1;
        }
    }

    /// Render the entire UI
    pub fn render(&mut self, frame: &mut ratatui::Frame) {
        self.frame_count = self.frame_count.wrapping_add(1);
        self.update_status_word();
        self.sync_reviewer_feedback_status();

        if std::env::var_os("ATLAS_TUI_MINIMAL").is_some() {
            use ratatui::{
                layout::Rect,
                style::{Color, Style},
                widgets::{Block, Borders, Paragraph},
            };

            let area = frame.size();
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Atlas AI ");
            frame.render_widget(block, area);
            let inner = Rect::new(
                area.x.saturating_add(2),
                area.y.saturating_add(2),
                area.width.saturating_sub(4),
                1,
            );
            frame.render_widget(Paragraph::new("Minimal TUI render OK"), inner);
            return;
        }

        if self.mode == AppMode::Config {
            crate::tui::components::ConfigScreen::render(
                frame,
                frame.size(),
                &self.config_state,
                self.frame_count,
            );
            return;
        }

        // Session Picker
        if self.mode == AppMode::SessionPicker {
            self.render_session_picker(frame);
            return;
        }

        // Graph View
        if self.mode == AppMode::GraphView {
            self.graph_total = crate::tui::components::conversation_graph::render_graph(
                frame,
                frame.size(),
                &self.graph_messages,
                &self.graph_branches,
                self.graph_selected,
            );
            return;
        }

        let thinking_on = self.mode == AppMode::Streaming || self.mode == AppMode::ExecutingTool;
        let thinking_h: u16 = if thinking_on { 1 } else { 0 };
        let suggestions_on = self.input.buffer.starts_with('/') && !self.input.buffer.contains(' ');
        let suggestions_h: u16 = if suggestions_on { 3 } else { 0 };
        let review_h: u16 = if self.research.reviewer_panel_visible {
            7
        } else {
            0
        };
        let layout = TuiLayout::calculate(frame.size(), 3, thinking_h, suggestions_h, review_h);

        // Compute effective scroll offset
        let scroll = if self.auto_scroll {
            usize::MAX
        } else {
            self.scroll_position
        };

        // Chat panel (full area, no thinking split)
        ChatPanel::render(
            frame,
            layout.chat_area,
            &self.messages,
            scroll,
            self.frame_count,
            &self.status_word,
        );

        // Thinking bar between chat and input
        if thinking_on {
            render_thinking_bar(
                frame,
                layout.thinking_area,
                self.frame_count,
                &self.status_word,
            );
        }

        // Command suggestions above input when typing /
        if suggestions_on {
            render_suggestions(
                frame,
                layout.suggestions_area,
                &self.input.buffer,
                &self.commands,
            );
        }

        if self.research.reviewer_panel_visible {
            crate::tui::components::ReviewerPanel::render(
                frame,
                layout.review_area,
                self.research.current_run_id.as_deref(),
                &self.research.reviewer_feedback,
            );
        }

        // Permission bar at bottom of chat area
        if self.mode == AppMode::WaitingForPermission && !self.pending_tool_calls.is_empty() {
            PermissionDialog::render(frame, layout.chat_area, &self.pending_tool_calls);
        }

        // Input bar
        InputBar::render(frame, layout.input_area, &self.input);

        // Status bar
        StatusBar::render(frame, layout.status_bar_area, &self.status_bar);
    }
}

/// Render a polished thinking indicator.
///
/// Shows a breathing dot (pulsing size + opacity) and a status word
/// on a single line, optionally with a subtle shimmer trail.
fn render_thinking_bar(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    frame_count: u64,
    status_word: &str,
) {
    use ratatui::style::Color;

    let word = if status_word.is_empty() {
        "Thinking"
    } else {
        status_word
    };

    // 80-frame breathing cycle (~1.3s) with two sub-cycles for the dot
    let cycle = (frame_count % 80) as f32 / 80.0;

    // Breathing intensity with asymmetric feel
    let intensity = if cycle < 0.45 {
        smoothstep(cycle / 0.45)
    } else if cycle < 0.55 {
        1.0
    } else if cycle < 0.95 {
        1.0 - smoothstep((cycle - 0.55) / 0.40)
    } else {
        0.0
    };

    // Dot animation: pick from a set of Unicode circle variants based on intensity
    let dots = [".", "o", "o", "O", "O", "O", "*", "*"];
    let dot_idx = ((intensity * 7.0) as usize).min(7);
    let dot = dots[dot_idx];

    // Color: cyan at peak, dim when resting
    let bg = (intensity * 80.0) as u8;
    let fg_r = (bg as u16 + 80).min(255) as u8;
    let fg_g = (bg as u16 + 200).min(255) as u8;
    let fg_b = (bg as u16 + 220).min(255) as u8;
    let dot_color = Color::Rgb(fg_r, fg_g, fg_b);

    // Shimmer trail: 3 tiny dots after the main dot, fading right
    let shimmer: String = (0..3)
        .map(|i| {
            let fade = (intensity * 0.6) - (i as f32 * 0.20);
            if fade > 0.0 {
                "."
            } else {
                " "
            }
        })
        .collect();

    let line = ratatui::text::Line::from(vec![
        ratatui::text::Span::styled("  ", ratatui::style::Style::default()),
        ratatui::text::Span::styled(
            format!("{} ", dot),
            ratatui::style::Style::default().fg(dot_color),
        ),
        ratatui::text::Span::styled(
            shimmer,
            ratatui::style::Style::default().fg(Color::Rgb(
                (intensity * 80.0) as u8,
                (intensity * 140.0) as u8,
                (intensity * 180.0) as u8,
            )),
        ),
        ratatui::text::Span::styled(
            format!("  {}...", word),
            ratatui::style::Style::default().fg(Color::Cyan),
        ),
    ]);
    frame.render_widget(ratatui::widgets::Paragraph::new(line), area);
}

/// Smoothstep interpolation for eased animation curves
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Render slash command suggestions
fn render_suggestions(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    input: &str,
    registry: &crate::tui::commands::CommandRegistry,
) {
    use ratatui::style::{Color, Style};
    use ratatui::text::{Line, Span};

    let prefix: &str = if input.len() > 1 { &input[1..] } else { "" };
    let completions = registry.completions();
    let matches: Vec<&(&str, &str)> = if prefix.is_empty() {
        completions.iter().collect()
    } else {
        completions
            .iter()
            .filter(|(name, _)| name.starts_with(prefix))
            .collect()
    };

    let mut lines: Vec<Line> = Vec::new();
    for (name, desc) in matches.iter().take(12) {
        lines.push(Line::from(vec![
            Span::styled(format!(" /{}", name), Style::default().fg(Color::Yellow)),
            Span::styled(format!(" - {}", desc), Style::default().fg(Color::DarkGray)),
        ]));
    }

    if !lines.is_empty() {
        frame.render_widget(ratatui::widgets::Paragraph::new(lines), area);
    }
}

// Helper: byte position from char index (needed in InputBarState and handle_paste)
impl InputBarState {
    pub fn byte_pos(&self, char_idx: usize) -> usize {
        self.buffer
            .char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or(self.buffer.len())
    }
}

/// Run the TUI application
pub fn run_tui(
    provider: Arc<dyn LLMProvider>,
    tool_definitions: Option<Vec<serde_json::Value>>,
    tool_executor: Option<
        Arc<dyn Fn(&str, &serde_json::Value) -> Result<String, String> + Send + Sync>,
    >,
    security_config: crate::security::SecurityConfig,
) -> Result<()> {
    info!("Starting Claude Code-style TUI");

    // Set up panic hook to restore terminal before crashing
    let old_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        let _ = crossterm::terminal::enable_raw_mode(); // try to restore cursor
        old_hook(info);
    }));

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create tokio runtime for async LLM tasks
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_time()
        .enable_io()
        .build()?;

    // Create app
    let mut app = TuiApp::new(provider, tool_definitions, tool_executor, security_config);

    // Main loop wrapped in catch_unwind to prevent crashes from killing the process
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_main_loop(&mut terminal, &mut app, &rt)
    }));
    let result = match result {
        Ok(r) => r,
        Err(panic_info) => {
            let msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "Unknown panic".to_string()
            };
            // Print error to console after terminal is restored
            disable_raw_mode().ok();
            execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).ok();
            eprintln!("TUI crashed: {}", msg);
            anyhow::bail!("TUI panic: {}", msg)
        }
    };

    // Restore terminal
    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    );
    let _ = terminal.show_cursor();

    // Shutdown runtime
    rt.shutdown_timeout(Duration::from_secs(1));

    result
}

/// Main event/render loop
fn run_main_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut TuiApp,
    rt: &tokio::runtime::Runtime,
) -> Result<()> {
    while app.running {
        // 1. RENDER
        terminal.draw(|frame| app.render(frame))?;

        // 2. CHECK IF TOOLS NEED EXECUTING (auto-approve path)
        app.execute_pending_tools_if_needed(rt);

        // 3. DRAIN STREAM CHANNEL (non-blocking)
        while let Ok(event) = app.stream_rx.try_recv() {
            app.handle_app_event(event, rt);
        }

        // 4. POLL INPUT (50ms timeout for responsive streaming display)
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat {
                        app.handle_key_event(&key, rt);
                    }
                }
                Event::Resize(_, _) => {
                    // Layout auto-recalculates on next render
                }
                Event::Paste(text) => {
                    app.handle_paste(&text);
                }
                Event::Mouse(mouse) => {
                    if let event::MouseEventKind::ScrollDown = mouse.kind {
                        app.scroll_position = app.scroll_position.saturating_add(3);
                    } else if let event::MouseEventKind::ScrollUp = mouse.kind {
                        app.auto_scroll = false;
                        app.scroll_position = app.scroll_position.saturating_sub(3);
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}
