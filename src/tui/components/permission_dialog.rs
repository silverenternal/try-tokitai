//! Permission dialog for tool call confirmation
//!
//! Renders an inline prompt at the bottom of the chat area (above the input bar),
//! mimicking Claude Code's inline permission UI.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use serde_json::Value;

/// A pending tool call awaiting user permission
#[derive(Debug, Clone)]
pub struct PendingToolCall {
    pub call_id: String,
    pub name: String,
    pub args: Value,
}

/// User action on the permission dialog
pub enum PermissionAction {
    Approve(usize),
    Deny(usize),
    ApproveAll,
    DenyAll,
    None,
}

/// Permission dialog renderer
pub struct PermissionDialog;

impl PermissionDialog {
    /// Render the permission bar at the bottom of the chat area.
    /// Returns the number of rows consumed (for layout adjustment).
    pub fn render(frame: &mut Frame, chat_area: Rect, pending: &[PendingToolCall]) -> u16 {
        if pending.is_empty() {
            return 0;
        }

        let tool = &pending[0];
        let args_str = serde_json::to_string_pretty(&tool.args).unwrap_or_default();
        let args_short = if args_str.chars().count() > 80 {
            let truncated: String = args_str.chars().take(77).collect();
            format!("{}...", truncated)
        } else {
            args_str
        };

        let count_note = if pending.len() > 1 {
            format!(" (+{} more)", pending.len() - 1)
        } else {
            String::new()
        };

        // Calculate how many lines we need
        let arg_lines: Vec<&str> = args_short.lines().collect();
        let needed_height = 4 + arg_lines.len() as u16; // title + args + spacer + options

        // Place at bottom of chat area
        let y = chat_area.y + chat_area.height.saturating_sub(needed_height);
        let area = Rect::new(chat_area.x, y, chat_area.width, needed_height);

        let mut lines = vec![Line::from(vec![
            Span::styled(
                " Tool call",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(": ", Style::default().fg(Color::White)),
            Span::styled(
                &tool.name,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(&count_note, Style::default().fg(Color::DarkGray)),
        ])];

        for arg_line in &arg_lines {
            lines.push(Line::from(Span::styled(
                format!("   {}", arg_line),
                Style::default().fg(Color::Gray),
            )));
        }
        lines.push(Line::from(""));

        // Options line — clearly visible
        lines.push(Line::from(vec![
            Span::styled(" y", Style::default().fg(Color::Black).bg(Color::Green)),
            Span::styled(" approve  ", Style::default().fg(Color::Green)),
            Span::styled(" n", Style::default().fg(Color::Black).bg(Color::Red)),
            Span::styled(" deny  ", Style::default().fg(Color::Red)),
            Span::styled(" a", Style::default().fg(Color::Black).bg(Color::Green)),
            Span::styled(" approve all  ", Style::default().fg(Color::Green)),
            Span::styled(" d", Style::default().fg(Color::Black).bg(Color::Red)),
            Span::styled(" deny all", Style::default().fg(Color::Red)),
        ]));

        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::Yellow));

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);

        needed_height
    }

    /// Handle a key event when the permission dialog is active.
    pub fn handle_key(key: &crossterm::event::KeyEvent) -> PermissionAction {
        use crossterm::event::{KeyCode, KeyModifiers};

        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            KeyCode::Char('y') if !ctrl => PermissionAction::Approve(0),
            KeyCode::Char('Y') => PermissionAction::Approve(0),
            KeyCode::Char('n') if !ctrl => PermissionAction::Deny(0),
            KeyCode::Char('N') => PermissionAction::Deny(0),
            KeyCode::Char('a') if !ctrl => PermissionAction::ApproveAll,
            KeyCode::Char('A') => PermissionAction::ApproveAll,
            KeyCode::Char('d') if !ctrl => PermissionAction::DenyAll,
            KeyCode::Char('D') => PermissionAction::DenyAll,
            _ => PermissionAction::None,
        }
    }
}
