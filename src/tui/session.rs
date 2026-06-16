//! Session persistence — Claude Code style conversation history
//!
//! Stores conversation messages as JSON files under `.tokitai/sessions/`.
//! On startup, shows a session picker so the user can resume a previous
//! conversation or start a new one.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::components::message_block::MessageBlock;

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

// ============================================================================
// SessionManager
// ============================================================================

pub struct SessionManager {
    /// `.tokitai/sessions/`
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
        let sessions_dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".tokitai")
            .join("sessions");

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
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<SessionMeta>>(&s).ok())
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
        let id = uuid_v4();
        let now = now_iso();

        let sid = uuid_v4();
        let now = now_iso();

        let meta = SessionMeta {
            id: sid.clone(),
            title: "New conversation".to_string(),
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

    /// Save the full message list for the current session.
    pub fn save_messages(&mut self, messages: &[MessageBlock]) -> Result<()> {
        let Some(ref id) = self.current_id else {
            return Ok(());
        };

        // Build file payload
        let meta = self
            .index
            .iter()
            .find(|m| &m.id == id)
            .cloned()
            .unwrap_or_else(|| SessionMeta {
                id: id.clone(),
                title: "Unknown".to_string(),
                summary: String::new(),
                created_at: now_iso(),
                updated_at: now_iso(),
                message_count: 0,
                model: String::new(),
                branches: default_branches(),
            });

        // Auto-derive title from first user message
        let title = auto_title(messages).unwrap_or_else(|| "New conversation".to_string());
        let mut updated_meta = meta;
        updated_meta.title = title;
        updated_meta.message_count = messages.len();
        updated_meta.updated_at = now_iso();

        let file = SessionFile {
            meta: updated_meta.clone(),
            messages: messages.to_vec(),
        };

        let path = self.sessions_dir.join(format!("{}.json", id));
        let json = serde_json::to_string_pretty(&file)?;
        std::fs::write(&path, json)?;

        // Update in-memory index
        if let Some(entry) = self.index.iter_mut().find(|m| &m.id == id) {
            *entry = updated_meta;
        }
        self.save_index()?;

        Ok(())
    }

    /// Load the message list for a given session id.
    /// Returns an empty vec if the file doesn't exist (session with 0 messages).
    pub fn load_messages(&self, id: &str) -> Result<Vec<MessageBlock>> {
        let path = self.sessions_dir.join(format!("{}.json", id));
        if !path.exists() {
            return Ok(Vec::new());
        }
        let json = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read session file: {}", id))?;
        let file: SessionFile = serde_json::from_str(&json)
            .with_context(|| format!("Failed to parse session file: {}", id))?;
        Ok(file.messages)
    }

    /// Delete a session (both from disk and index).
    pub fn delete_session(&mut self, id: &str) -> Result<()> {
        let path = self.sessions_dir.join(format!("{}.json", id));
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        self.index.retain(|m| m.id != id);
        self.save_index()?;
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

/// Extract a title from the first exchange (user question + assistant reply).
/// Uses the first user message as the primary topic, supplemented by the first
/// assistant message to give context about what was accomplished.
fn auto_title(messages: &[MessageBlock]) -> Option<String> {
    let user_msg = messages.iter().find_map(|m| {
        if let MessageBlock::User { content, .. } = m {
            Some(content.as_str())
        } else {
            None
        }
    })?;

    // Look for the first meaningful assistant response
    let assistant_msg = messages.iter().find_map(|m| match m {
        MessageBlock::Assistant { content } if !content.is_empty() => Some(content.as_str()),
        _ => None,
    });

    // Title: first user message (capped at 60 chars)
    let user_title: String = user_msg.chars().take(60).collect();
    let user_title = if user_title.len() < user_msg.len() {
        format!("{}...", user_title.trim())
    } else {
        user_title.trim().to_string()
    };

    // If there's an assistant response, enrich the title with its first line
    if let Some(assistant) = assistant_msg {
        let first_line = assistant.lines().next().unwrap_or("").trim();
        // Only use assistant context if it's meaningful (not just "Sure!", "Okay", etc.)
        let skip_prefixes = ["Sure", "Okay", "OK", "Let me", "I'll", "Here", "Certainly"];
        let meaningful =
            first_line.len() > 15 && !skip_prefixes.iter().any(|p| first_line.starts_with(p));

        if meaningful && !user_title.contains(first_line) {
            let context: String = first_line.chars().take(80).collect();
            return Some(format!("{} — {}", user_title, context.trim()));
        }
    }

    Some(user_title)
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
            _ => {}
        }
    }

    let request = ChatRequest {
        model: provider.default_model().to_string(),
        messages: vec![
            Message::system("Summarize the following conversation in one line (max 80 chars). Focus on what was asked and what was accomplished. Output ONLY the summary text, no prefix."),
            Message::user(&conv_text),
        ],
        temperature: 0.3,
        max_tokens: Some(120),
        top_p: None,
        stop: None,
        stream: false,
        tools: None,
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
        if let Some(entry) = self.index.iter_mut().find(|m| &m.id == id) {
            entry.title = title.to_string();
        }
        self.save_index()?;
        Ok(())
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
