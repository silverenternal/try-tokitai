//! Message block types for rich chat rendering
//!
//! Each message in the conversation is represented as a `MessageBlock`,
//! which carries its visual presentation state alongside the data.

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use super::diff_viewer::FileDiff;

/// Status of a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolCallStatus {
    /// Waiting for user permission
    Pending,
    /// Approved, about to execute
    Approved,
    /// Denied by user
    Denied(String),
    /// Currently executing
    Executing,
    /// Completed successfully
    Complete,
    /// Execution failed
    Failed(String),
}

/// A single message block in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageBlock {
    /// A user message
    User {
        content: String,
        /// Branch id this message belongs to ("main" or fork id)
        #[serde(default)]
        branch_id: String,
    },
    /// A complete assistant response
    Assistant {
        content: String,
    },
    /// A streaming/partial assistant response (renders with cursor)
    AssistantStreaming {
        content: String,
    },
    /// An LLM tool call
    ToolCall {
        name: String,
        args: Value,
        call_id: String,
        status: ToolCallStatus,
    },
    /// A tool execution result
    ToolResult {
        call_id: String,
        result: String,
        success: bool,
    },
    /// A collapsible thinking/reasoning block
    Thinking {
        content: String,
        collapsed: bool,
    },
    /// An error message
    Error {
        content: String,
    },
    /// A system message
    System {
        content: String,
    },
    /// A file diff (code changes: green +, red -)
    Diff {
        diff: FileDiff,
    },
}

impl MessageBlock {
    /// Get the role label for display
    pub fn role_label(&self) -> &str {
        match self {
            MessageBlock::User { .. } => "You",
            MessageBlock::Assistant { .. } | MessageBlock::AssistantStreaming { .. } => "Assistant",
            MessageBlock::ToolCall { .. } => "Tool",
            MessageBlock::ToolResult { .. } => "Result",
            MessageBlock::Thinking { .. } => "Thinking",
            MessageBlock::Error { .. } => "Error",
            MessageBlock::System { .. } => "System",
            MessageBlock::Diff { .. } => "Diff",
        }
    }

    /// Get the color for this block type
    pub fn color(&self) -> Color {
        match self {
            MessageBlock::User { .. } => Color::Cyan,
            MessageBlock::Assistant { .. } | MessageBlock::AssistantStreaming { .. } => {
                Color::Green
            }
            MessageBlock::ToolCall { .. } => Color::Yellow,
            MessageBlock::ToolResult { .. } => Color::Gray,
            MessageBlock::Thinking { .. } => Color::Blue,
            MessageBlock::Error { .. } => Color::Red,
            MessageBlock::System { .. } => Color::DarkGray,
            MessageBlock::Diff { .. } => Color::Green,
        }
    }

    /// Get the content for computing line count
    pub fn content(&self) -> &str {
        match self {
            MessageBlock::User { content, .. }
            | MessageBlock::Assistant { content }
            | MessageBlock::AssistantStreaming { content }
            | MessageBlock::Thinking { content, .. }
            | MessageBlock::Error { content }
            | MessageBlock::System { content } => content,
            MessageBlock::ToolCall { name, .. } => name,
            MessageBlock::ToolResult { result, .. } => result,
            MessageBlock::Diff { diff } => &diff.file_path,
        }
    }

    /// Estimate the number of rendered lines for virtual scrolling
    pub fn line_count(&self, width: u16) -> usize {
        let text = self.content();
        if text.is_empty() {
            return 1;
        }
        // Rough estimate: count lines accounting for wrapping
        let max_chars = width.saturating_sub(4) as usize; // Allow for borders/padding
        if max_chars == 0 {
            return 1;
        }
        let mut lines = 1;
        let mut line_chars = 0;
        for ch in text.chars() {
            if ch == '\n' {
                lines += 1;
                line_chars = 0;
            } else {
                line_chars += 1;
                if line_chars >= max_chars {
                    lines += 1;
                    line_chars = 0;
                }
            }
        }
        // Add header line
        lines + 1
    }

    /// Render this block into styled lines
    pub fn render_lines(&self, width: u16) -> Vec<Line<'_>> {
        let color = self.color();
        let label = self.role_label();
        let max_chars = width.saturating_sub(4) as usize;

        let mut lines = Vec::new();

        // Header line
        match self {
            MessageBlock::ToolCall {
                name,
                args,
                call_id: _,
                status,
            } => {
                let status_icon = match status {
                    ToolCallStatus::Pending => "⏳",
                    ToolCallStatus::Approved => "✓",
                    ToolCallStatus::Denied(_) => "✗",
                    ToolCallStatus::Executing => "⚙",
                    ToolCallStatus::Complete => "✓",
                    ToolCallStatus::Failed(_) => "✗",
                };
                let args_preview = serde_json::to_string(args)
                    .unwrap_or_default();
                let args_short = if args_preview.len() > 60 {
                    format!("{}...", &args_preview[..57])
                } else {
                    args_preview
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{} ", status_icon),
                        Style::default().fg(color),
                    ),
                    Span::styled(
                        format!("Tool: {}", name),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  {}", args_short),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }
            MessageBlock::ToolResult {
                call_id: _,
                result,
                success,
            } => {
                let icon = if *success { "✓" } else { "✗" };
                let result_color = if *success { Color::Green } else { Color::Red };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{} Result", icon),
                        Style::default().fg(result_color).add_modifier(Modifier::BOLD),
                    ),
                ]));
                // Content lines follow below
                for line in wrap_text(result, max_chars) {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", line),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
                return lines;
            }
            MessageBlock::Thinking { collapsed, .. } => {
                let toggle = if *collapsed { "[+]" } else { "[-]" };
                lines.push(Line::from(vec![
                    Span::styled(toggle, Style::default().fg(Color::Blue)),
                    Span::styled(
                        " Thinking...",
                        Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]));
                if *collapsed {
                    return lines;
                }
            }
            _ => {
                lines.push(Line::from(vec![Span::styled(
                    format!("{} {}", label, if matches!(self, MessageBlock::AssistantStreaming { .. }) { "|" } else { "" }),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                )]));
            }
        }

        // Content lines
        let text = match self {
            MessageBlock::ToolCall { .. } => return lines,
            _ => self.content(),
        };

        for line in wrap_text(text, max_chars) {
            lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(Color::White),
            )));
        }

        lines
    }
}

/// Simple word-wrap helper
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    for line in text.lines() {
        if line.len() <= max_width {
            lines.push(line.to_string());
            continue;
        }
        let mut current = String::new();
        for word in line.split_whitespace() {
            if current.len() + word.len() + 1 > max_width {
                if !current.is_empty() {
                    lines.push(current.clone());
                }
                current = word.to_string();
            } else {
                if !current.is_empty() {
                    current.push(' ');
                }
                current.push_str(word);
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_block_line_count() {
        let block = MessageBlock::User {
            content: "Hello".to_string(),
            branch_id: String::new(),
        };
        assert!(block.line_count(80) >= 1);
    }

    #[test]
    fn test_wrap_text() {
        let result = wrap_text("hello world test", 10);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_message_block_colors() {
        assert_eq!(MessageBlock::User { content: String::new(), branch_id: String::new() }.color(), Color::Cyan);
        assert_eq!(MessageBlock::Assistant { content: String::new() }.color(), Color::Green);
        assert_eq!(MessageBlock::Error { content: String::new() }.color(), Color::Red);
    }
}
