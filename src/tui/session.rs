//! Session persistence — Claude Code style conversation history
//!
//! Stores conversation messages as JSON files under `.atlas/sessions/`.
//! On startup, shows a session picker so the user can resume a previous
//! conversation or start a new one.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::components::message_block::MessageBlock;
use crate::app_paths::AppPaths;
use crate::text_encoding::{normalize_text_for_display, read_text_file};

// ============================================================================
// SessionBranch
// ============================================================================

/// One branch in a conversation session (like a git branch)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionBranch {
    pub id: String,
    pub name: String,
    /// Parent branch id ("" for main)
    #[serde(default)]
    pub parent_id: String,
    /// Index of the first User message in this branch (fork point)
    pub fork_msg_index: usize,
    /// Branch this branch was explicitly merged into, if any.
    #[serde(default)]
    pub merged_into: Option<String>,
    /// Color palette index (0-5)
    pub color_idx: usize,
}

impl SessionBranch {
    pub fn main() -> Self {
        Self {
            id: "main".to_string(),
            name: "main".to_string(),
            parent_id: String::new(),
            fork_msg_index: 0,
            merged_into: None,
            color_idx: 0,
        }
    }
}

/// 6-color palette for branches
pub const BRANCH_COLORS: [&str; 6] = ["cyan", "green", "yellow", "magenta", "blue", "red"];

// ============================================================================
// SessionMeta
// ============================================================================

/// Lightweight metadata for one session — stored in `index.json`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: String,
    /// Auto-derived or user-set display title
    pub title: String,
    /// Whether the title was explicitly customized by the user.
    #[serde(default)]
    pub custom_title: bool,
    /// AI-generated summary (populated by /summarize command)
    #[serde(default)]
    pub summary: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: usize,
    pub model: String,
    /// Branch tracking for this session
    #[serde(default = "default_branches")]
    pub branches: Vec<SessionBranch>,
}

fn default_branches() -> Vec<SessionBranch> {
    vec![SessionBranch::main()]
}

// ============================================================================
// SessionFile — the on-disk format for a single session
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionFile {
    meta: SessionMeta,
    messages: Vec<MessageBlock>,
}

fn sanitize_message_block(block: &mut MessageBlock) {
    match block {
        MessageBlock::User { content, branch_id } => {
            *content = normalize_text_for_display(content);
            *branch_id = normalize_text_for_display(branch_id);
        }
        MessageBlock::Assistant { content }
        | MessageBlock::AssistantStreaming { content }
        | MessageBlock::Thinking { content, .. }
        | MessageBlock::Error { content }
        | MessageBlock::System { content } => {
            *content = normalize_text_for_display(content);
        }
        MessageBlock::AssistantChoices { title, options } => {
            *title = normalize_text_for_display(title);
            for value in options {
                *value = normalize_text_for_display(value);
            }
        }
        MessageBlock::ToolCall { name, status, .. } => {
            *name = normalize_text_for_display(name);
            match status {
                super::components::message_block::ToolCallStatus::Denied(reason)
                | super::components::message_block::ToolCallStatus::Failed(reason) => {
                    *reason = normalize_text_for_display(reason);
                }
                _ => {}
            }
        }
        MessageBlock::ToolResult { result, .. } => {
            *result = normalize_text_for_display(result);
        }
        MessageBlock::Diff { diff } => {
            diff.file_path = normalize_text_for_display(&diff.file_path);
            diff.before_content = normalize_text_for_display(&diff.before_content);
            diff.after_content = normalize_text_for_display(&diff.after_content);
            for line in &mut diff.lines {
                match line {
                    super::components::diff_viewer::DiffLine::Add(text)
                    | super::components::diff_viewer::DiffLine::Remove(text)
                    | super::components::diff_viewer::DiffLine::Context(text)
                    | super::components::diff_viewer::DiffLine::Header(text) => {
                        *text = normalize_text_for_display(text);
                    }
                }
            }
        }
        MessageBlock::Subagent { record } => {
            record.id = normalize_text_for_display(&record.id);
            record.name = normalize_text_for_display(&record.name);
            record.purpose = normalize_text_for_display(&record.purpose);
            record.input = normalize_text_for_display(&record.input);
            record.output = normalize_text_for_display(&record.output);
            record.status = normalize_text_for_display(&record.status);
            record.kind = normalize_text_for_display(&record.kind);
            if let Some(value) = &mut record.started_at {
                *value = normalize_text_for_display(value);
            }
            if let Some(value) = &mut record.completed_at {
                *value = normalize_text_for_display(value);
            }
            for value in &mut record.evidence {
                *value = normalize_text_for_display(value);
            }
        }
        MessageBlock::Verification { report } => {
            report.status = normalize_text_for_display(&report.status);
            report.summary = normalize_text_for_display(&report.summary);
            for issue in &mut report.issues {
                *issue = normalize_text_for_display(issue);
            }
            for evidence in &mut report.evidence {
                *evidence = normalize_text_for_display(evidence);
            }
            for action in &mut report.next_actions {
                *action = normalize_text_for_display(action);
            }
            for check in &mut report.checks {
                check.id = normalize_text_for_display(&check.id);
                check.title = normalize_text_for_display(&check.title);
                check.status = normalize_text_for_display(&check.status);
                check.detail = normalize_text_for_display(&check.detail);
                for evidence in &mut check.evidence {
                    *evidence = normalize_text_for_display(evidence);
                }
            }
        }
    }
}

fn sanitize_session_meta(meta: &mut SessionMeta) {
    meta.id = normalize_text_for_display(&meta.id);
    meta.title = normalize_text_for_display(&meta.title);
    meta.summary = normalize_text_for_display(&meta.summary);
    meta.created_at = normalize_text_for_display(&meta.created_at);
    meta.updated_at = normalize_text_for_display(&meta.updated_at);
    meta.model = normalize_text_for_display(&meta.model);
    for branch in &mut meta.branches {
        branch.id = normalize_text_for_display(&branch.id);
        branch.name = normalize_text_for_display(&branch.name);
        branch.parent_id = normalize_text_for_display(&branch.parent_id);
        if let Some(value) = &mut branch.merged_into {
            *value = normalize_text_for_display(value);
        }
    }
}

fn sanitize_session_file(file: &mut SessionFile) {
    sanitize_session_meta(&mut file.meta);
    for block in &mut file.messages {
        sanitize_message_block(block);
    }
}

// ============================================================================
// SessionManager
// ============================================================================

pub struct SessionManager {
    /// `.atlas/sessions/`
    sessions_dir: PathBuf,
    /// In-memory copy of the index (sorted newest-first)
    pub index: Vec<SessionMeta>,
    /// The currently open session id (None until user picks or creates)
    pub current_id: Option<String>,
}

impl SessionManager {
    // ------------------------------------------------------------------
    // Constructor
    // ------------------------------------------------------------------

    pub fn new() -> Result<Self> {
        let sessions_dir =
            AppPaths::for_local_dev(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
                .sessions_dir();

        Self::from_sessions_dir(sessions_dir)
    }

    pub fn from_sessions_dir(sessions_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&sessions_dir)
            .with_context(|| "Failed to create sessions directory")?;

        let index = Self::load_index(&sessions_dir);

        Ok(Self {
            sessions_dir,
            index,
            current_id: None,
        })
    }

    // ------------------------------------------------------------------
    // Index I/O
    // ------------------------------------------------------------------

    fn index_path(sessions_dir: &Path) -> PathBuf {
        sessions_dir.join("index.json")
    }

    fn load_index(sessions_dir: &Path) -> Vec<SessionMeta> {
        let path = Self::index_path(sessions_dir);
        if !path.exists() {
            return Vec::new();
        }
        read_text_file(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<SessionMeta>>(&s).ok())
            .map(|mut metas| {
                for meta in &mut metas {
                    sanitize_session_meta(meta);
                }
                metas
            })
            .unwrap_or_default()
    }

    fn save_index(&self) -> Result<()> {
        let path = Self::index_path(&self.sessions_dir);
        let json = serde_json::to_string_pretty(&self.index)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Session lifecycle
    // ------------------------------------------------------------------

    /// Create a brand-new session and add it to the index.
    pub fn create_session(&mut self, model: &str) -> Result<&SessionMeta> {
        let sid = uuid_v4();
        let now = now_iso();

        let meta = SessionMeta {
            id: sid.clone(),
            title: "New conversation".to_string(),
            custom_title: false,
            summary: String::new(),
            created_at: now.clone(),
            updated_at: now,
            message_count: 0,
            model: model.to_string(),
            branches: default_branches(),
        };

        self.index.insert(0, meta.clone());
        self.save_index()?;

        // Also create an empty session file so load_messages never fails
        let file = SessionFile {
            meta,
            messages: Vec::new(),
        };
        let path = self.sessions_dir.join(format!("{}.json", sid));
        let json = serde_json::to_string_pretty(&file)?;
        std::fs::write(&path, json)?;

        // Return a reference to the just-inserted entry
        Ok(&self.index[0])
    }

    /// Save the full message list for a specific session id.
    pub fn save_messages_for(&mut self, id: &str, messages: &[MessageBlock]) -> Result<()> {
        let id = id.trim();
        if id.is_empty() {
            return Ok(());
        }

        // Build file payload
        let meta = self
            .index
            .iter()
            .find(|m| m.id == id)
            .cloned()
            .unwrap_or_else(|| SessionMeta {
                id: id.to_string(),
                title: "Unknown".to_string(),
                custom_title: false,
                summary: String::new(),
                created_at: now_iso(),
                updated_at: now_iso(),
                message_count: 0,
                model: String::new(),
                branches: default_branches(),
            });

        // Auto-derive title and summary from the persisted conversation state.
        let title = auto_title(messages).unwrap_or_else(|| "New conversation".to_string());
        let summary = auto_summary(messages).unwrap_or_default();
        let mut updated_meta = meta;
        if !updated_meta.custom_title {
            updated_meta.title = title;
        }
        updated_meta.summary = summary;
        updated_meta.message_count = messages.len();
        updated_meta.updated_at = now_iso();

        let mut file = SessionFile {
            meta: updated_meta.clone(),
            messages: messages.to_vec(),
        };
        sanitize_session_file(&mut file);

        let path = self.sessions_dir.join(format!("{}.json", id));
        let json = serde_json::to_string_pretty(&file)?;
        std::fs::write(&path, json)?;

        // Update in-memory index
        if let Some(entry) = self.index.iter_mut().find(|m| m.id == id) {
            *entry = updated_meta;
        } else {
            self.index.insert(0, updated_meta);
        }
        self.index
            .sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        self.save_index()?;

        Ok(())
    }

    /// Record the model that most recently handled this session without
    /// changing its conversation history. A running turn captures its model
    /// before this value is updated, so model changes apply on turn boundaries.
    pub fn update_model_for(&mut self, id: &str, model: &str) -> Result<()> {
        let id = id.trim();
        let model = model.trim();
        if id.is_empty() || model.is_empty() {
            return Ok(());
        }

        let Some(entry) = self.index.iter_mut().find(|meta| meta.id == id) else {
            return Ok(());
        };
        if entry.model == model {
            return Ok(());
        }

        entry.model = model.to_string();
        let path = self.sessions_dir.join(format!("{}.json", id));
        if path.exists() {
            let json = read_text_file(&path)
                .with_context(|| format!("Failed to read session file: {}", id))?;
            let mut file: SessionFile = serde_json::from_str(&json)
                .with_context(|| format!("Failed to parse session file: {}", id))?;
            file.meta.model = model.to_string();
            std::fs::write(&path, serde_json::to_string_pretty(&file)?)?;
        }
        self.save_index()?;
        Ok(())
    }

    pub fn refresh_summaries(&mut self) -> Result<bool> {
        let mut changed = false;

        for meta in &mut self.index {
            let path = self.sessions_dir.join(format!("{}.json", meta.id));
            let content = match read_text_file(&path) {
                Ok(content) => content,
                Err(_) => continue,
            };
            let mut file: SessionFile = match serde_json::from_str(&content) {
                Ok(file) => file,
                Err(_) => continue,
            };
            sanitize_session_file(&mut file);

            let next_title =
                auto_title(&file.messages).unwrap_or_else(|| "New conversation".to_string());
            let next_summary = auto_summary(&file.messages).unwrap_or_default();

            let mut file_changed = false;
            if !file.meta.custom_title && file.meta.title != next_title {
                file.meta.title = next_title.clone();
                meta.title = next_title;
                file_changed = true;
            }
            if file.meta.summary != next_summary || meta.summary != next_summary {
                file.meta.summary = next_summary.clone();
                meta.summary = next_summary;
                file_changed = true;
            }

            if file_changed {
                let json = serde_json::to_string_pretty(&file)?;
                std::fs::write(&path, json)?;
                changed = true;
            }
        }

        if changed {
            self.index
                .sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
            self.save_index()?;
        }

        Ok(changed)
    }

    /// Save the full message list for the current session.
    pub fn save_messages(&mut self, messages: &[MessageBlock]) -> Result<()> {
        let Some(ref id) = self.current_id else {
            return Ok(());
        };
        let id = id.clone();
        self.save_messages_for(&id, messages)
    }

    /// Load the message list for a given session id.
    /// Returns an empty vec if the file doesn't exist (session with 0 messages).
    pub fn load_messages(&self, id: &str) -> Result<Vec<MessageBlock>> {
        let path = self.sessions_dir.join(format!("{}.json", id));
        if !path.exists() {
            return Ok(Vec::new());
        }
        let json = read_text_file(&path)
            .with_context(|| format!("Failed to read session file: {}", id))?;
        let mut file: SessionFile = serde_json::from_str(&json)
            .with_context(|| format!("Failed to parse session file: {}", id))?;
        sanitize_session_file(&mut file);
        Ok(file.messages)
    }

    /// Delete a session (both from disk and index).
    pub fn delete_session(&mut self, id: &str) -> Result<()> {
        let path = self.sessions_dir.join(format!("{}.json", id));
        if path.exists() {
            if let Err(e) = std::fs::remove_file(&path) {
                tracing::warn!("Failed to delete session file {}: {}", path.display(), e);
            }
        }
        self.index.retain(|m| m.id != id);
        if self.current_id.as_deref() == Some(id) {
            self.current_id = self.index.first().map(|meta| meta.id.clone());
        }
        if let Err(e) = self.save_index() {
            tracing::warn!("Failed to save index after deleting session {}: {}", id, e);
        }
        Ok(())
    }

    /// Get recent sessions (newest first).
    pub fn list_recent(&self, limit: usize) -> &[SessionMeta] {
        let end = self.index.len().min(limit);
        &self.index[..end]
    }

    /// Set the current session and load its messages.
    pub fn resume_session(&mut self, id: &str) -> Result<Vec<MessageBlock>> {
        // Verify session exists in index
        if !self.index.iter().any(|m| m.id == id) {
            anyhow::bail!("Session not found: {}", id);
        }
        self.current_id = Some(id.to_string());
        self.load_messages(id)
    }

    /// Set a custom title for a specific session.
    pub fn rename_session(&mut self, id: &str, title: &str) -> Result<()> {
        let title = title.trim();
        if title.is_empty() {
            anyhow::bail!("Session title cannot be empty");
        }

        let Some(entry) = self.index.iter_mut().find(|m| m.id == id) else {
            anyhow::bail!("Session not found: {}", id);
        };

        entry.title = title.to_string();
        entry.custom_title = true;
        entry.updated_at = now_iso();

        let path = self.sessions_dir.join(format!("{}.json", id));
        if path.exists() {
            let json = read_text_file(&path)
                .with_context(|| format!("Failed to read session file: {}", id))?;
            let mut file: SessionFile = serde_json::from_str(&json)
                .with_context(|| format!("Failed to parse session file: {}", id))?;
            file.meta.title = entry.title.clone();
            file.meta.custom_title = true;
            file.meta.updated_at = entry.updated_at.clone();
            std::fs::write(&path, serde_json::to_string_pretty(&file)?)?;
        }

        self.save_index()?;
        Ok(())
    }

    /// Switch to a new session (save current first).
    pub fn switch_to_new(&mut self, messages: &[MessageBlock], model: &str) -> Result<()> {
        // Save current session first
        if self.current_id.is_some() {
            self.save_messages(messages)?;
        }
        // Create new
        let meta = self.create_session(model)?;
        self.current_id = Some(meta.id.clone());
        Ok(())
    }

    /// Check if the current session has messages worth saving.
    pub fn has_current_session(&self) -> bool {
        self.current_id.is_some()
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn compact_message_text(raw: &str, max_chars: usize) -> Option<String> {
    let cleaned = raw
        .replace("[AGENT]", "")
        .replace('\r', " ")
        .replace('\n', " ");
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");

    if cleaned.is_empty() {
        return None;
    }

    let clipped: String = cleaned.chars().take(max_chars).collect();
    Some(if cleaned.chars().count() > max_chars {
        format!("{}...", clipped.trim_end())
    } else {
        clipped.trim().to_string()
    })
}

fn looks_like_corrupted_text(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return false;
    }

    if trimmed.contains('\u{fffd}') {
        return true;
    }

    let total_chars = trimmed.chars().count();
    let question_like = trimmed
        .chars()
        .filter(|ch| matches!(ch, '?' | '？'))
        .count();
    if question_like >= 4 && (question_like as f32 / total_chars as f32) > 0.18 {
        return true;
    }

    const MOJIBAKE_MARKERS: [&str; 12] = [
        "鈥", "銆", "锛", "鍙", "鏂", "寮", "缁", "鐮", "姝", "闂", "璇", "閿",
    ];
    MOJIBAKE_MARKERS
        .iter()
        .any(|marker| trimmed.contains(marker))
}

fn is_low_value_summary_text(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() || looks_like_corrupted_text(trimmed) {
        return true;
    }
    if trimmed.starts_with('[') && trimmed.contains("编码损坏") {
        return true;
    }

    const GENERIC_FALLBACKS: [&str; 14] = [
        "无法理解您发送的内容",
        "无法正常显示的内容",
        "您的输入仍然显示为无法识别的字符",
        "乱码字符",
        "请重新描述您的需求",
        "重新发送一条",
        "历史消息已因旧编码损坏而清洗",
        "原文无法恢复",
        "cannot understand your message",
        "unable to understand your message",
        "corrupted encoding",
        "garbled characters",
        "please resend",
        "unreadable content",
    ];

    GENERIC_FALLBACKS
        .iter()
        .any(|needle| trimmed.contains(needle))
}

/// Extract a short title from the first user message.
fn auto_title(messages: &[MessageBlock]) -> Option<String> {
    let user_msg = messages
        .iter()
        .find_map(|m| {
            if let MessageBlock::User { content, .. } = m {
                if is_low_value_summary_text(content) {
                    None
                } else {
                    Some(content.as_str())
                }
            } else {
                None
            }
        })
        .or_else(|| {
            messages.iter().find_map(|m| {
                if let MessageBlock::User { content, .. } = m {
                    Some(content.as_str())
                } else {
                    None
                }
            })
        })?;

    compact_message_text(user_msg, 28).or_else(|| Some("New conversation".to_string()))
}

/// Extract a short rolling summary from the latest visible conversation message.
fn auto_summary(messages: &[MessageBlock]) -> Option<String> {
    let preferred = messages.iter().rev().find_map(|message| match message {
        MessageBlock::Assistant { content }
        | MessageBlock::AssistantChoices { title: content, .. }
        | MessageBlock::AssistantStreaming { content }
        | MessageBlock::Error { content }
        | MessageBlock::System { content } => {
            if is_low_value_summary_text(content) {
                None
            } else {
                compact_message_text(content, 42)
            }
        }
        _ => None,
    });

    if preferred.is_some() {
        return preferred;
    }

    messages.iter().rev().find_map(|message| match message {
        MessageBlock::User { content, .. } => {
            if is_low_value_summary_text(content) {
                None
            } else {
                compact_message_text(content, 42)
            }
        }
        MessageBlock::AssistantChoices { title, options } => {
            let combined = format!("{} {}", title, options.join(" "));
            if is_low_value_summary_text(&combined) {
                None
            } else {
                compact_message_text(&combined, 42)
            }
        }
        _ => None,
    })
}

/// Generate an AI summary for the conversation (requires LLM provider).
/// Returns the summary text, or an error if the LLM call fails.
pub async fn generate_ai_summary(
    messages: &[MessageBlock],
    provider: &std::sync::Arc<dyn crate::llm::LLMProvider>,
) -> Result<String> {
    use crate::llm::{ChatRequest, Message};

    // Build a conversation summary prompt
    let mut conv_text = String::new();
    for msg in messages.iter().take(20) {
        // Take first 20 messages for context
        match msg {
            MessageBlock::User { content, .. } => {
                conv_text.push_str(&format!("User: {}\n", content));
            }
            MessageBlock::Assistant { content } => {
                let short: String = content.chars().take(200).collect();
                conv_text.push_str(&format!("Assistant: {}\n", short));
            }
            MessageBlock::AssistantChoices { title, options } => {
                let joined = options.join(" | ");
                conv_text.push_str(&format!("Assistant choices: {} => {}\n", title, joined));
            }
            _ => {}
        }
    }

    let request = ChatRequest {
        model: provider.default_model().to_string(),
        messages: vec![
            Message::system("Summarize the following conversation in one line (max 80 chars). Focus on what was asked and what was accomplished. Output ONLY the summary text, no prefix."),
            Message::user(&conv_text),
        ],
        multimodal_content: None,
        temperature: 0.3,
        max_tokens: Some(120),
        top_p: None,
        stop: None,
        stream: false,
        tools: None,
        thinking_mode: None,
        reasoning_effort: None,
    };

    let response = provider.chat(request).await?;
    let summary = response.content.trim().to_string();
    Ok(summary)
}

impl SessionManager {
    /// Set a custom title for the current session
    pub fn set_title(&mut self, title: &str) -> Result<()> {
        let Some(ref id) = self.current_id else {
            anyhow::bail!("No active session");
        };
        let id = id.clone();
        self.rename_session(&id, title)
    }

    /// Set the AI-generated summary for the current session
    pub fn set_summary(&mut self, summary: &str) -> Result<()> {
        let Some(ref id) = self.current_id else {
            anyhow::bail!("No active session");
        };
        if let Some(entry) = self.index.iter_mut().find(|m| &m.id == id) {
            entry.summary = summary.to_string();
        }
        self.save_index()?;
        Ok(())
    }

    /// Fork the current session at the given node index.
    /// Creates a new branch and returns its id.
    pub fn fork_at_node(
        &mut self,
        at_user_msg_index: usize,
        parent_branch_id: &str,
    ) -> Result<String> {
        let Some(ref id) = self.current_id else {
            anyhow::bail!("No active session");
        };
        let id = id.clone();

        // Find next available color
        let used_colors: Vec<usize> = self
            .index
            .iter()
            .find(|m| m.id == id)
            .map(|m| m.branches.iter().map(|b| b.color_idx).collect())
            .unwrap_or_default();
        let color_idx = (0..6).find(|c| !used_colors.contains(c)).unwrap_or(0);

        let base_id = format!("fork-{}", at_user_msg_index + 1);
        let mut fork_id = base_id.clone();
        let mut suffix = 2usize;
        while self
            .index
            .iter()
            .find(|m| m.id == id)
            .map(|m| m.branches.iter().any(|b| b.id == fork_id))
            .unwrap_or(false)
        {
            fork_id = format!("{}-{}", base_id, suffix);
            suffix += 1;
        }

        let parent_id = if parent_branch_id.is_empty() {
            "main".to_string()
        } else {
            parent_branch_id.to_string()
        };

        let branch = SessionBranch {
            id: fork_id.clone(),
            name: fork_id.clone(),
            parent_id,
            fork_msg_index: at_user_msg_index + 1,
            merged_into: None,
            color_idx,
        };

        if let Some(entry) = self.index.iter_mut().find(|m| m.id == id) {
            entry.branches.push(branch);
        }
        self.save_index()?;

        Ok(fork_id)
    }

    /// Mark a branch as explicitly merged into another branch.
    pub fn mark_branch_merged(&mut self, branch_id: &str, target_branch_id: &str) -> Result<()> {
        let Some(ref id) = self.current_id else {
            anyhow::bail!("No active session");
        };
        let Some(entry) = self.index.iter_mut().find(|m| &m.id == id) else {
            anyhow::bail!("No active session metadata");
        };
        let Some(branch) = entry.branches.iter_mut().find(|b| b.id == branch_id) else {
            anyhow::bail!("Branch not found: {}", branch_id);
        };

        branch.merged_into = Some(target_branch_id.to_string());
        self.save_index()?;
        Ok(())
    }

    /// List branches for a specific session id.
    pub fn branches_for_session(&self, session_id: &str) -> Vec<SessionBranch> {
        self.index
            .iter()
            .find(|m| m.id == session_id)
            .map(|m| m.branches.clone())
            .unwrap_or_else(default_branches)
    }

    /// Get the branch info for a given branch id
    pub fn get_branch(&self, branch_id: &str) -> Option<SessionBranch> {
        let session_id = self.current_id.as_ref()?;
        self.index
            .iter()
            .find(|m| &m.id == session_id)
            .and_then(|m| m.branches.iter().find(|b| b.id == branch_id).cloned())
    }

    /// List all branches for the current session
    pub fn list_branches(&self) -> Vec<&SessionBranch> {
        let Some(ref id) = self.current_id else {
            return vec![];
        };
        self.index
            .iter()
            .find(|m| &m.id == id)
            .map(|m| m.branches.iter().collect())
            .unwrap_or_default()
    }
}

/// Generate a simple UUID v4 (no external crate needed)
fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:016x}", now)
}

fn now_iso() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}
