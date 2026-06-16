//! Tool call and tool result block rendering

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use serde_json::Value;

use super::message_block::ToolCallStatus;

/// Render a tool call block
pub struct ToolCallBlock;

impl ToolCallBlock {
    /// Render tool call with name, arguments, and status
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        name: &str,
        args: &Value,
        call_id: &str,
        status: &ToolCallStatus,
    ) {
        let (status_icon, status_color) = match status {
            ToolCallStatus::Pending => ("⏳ Pending", Color::Yellow),
            ToolCallStatus::Approved => ("✓ Approved", Color::Green),
            ToolCallStatus::Denied(reason) => return Self::render_denied(frame, area, name, reason),
            ToolCallStatus::Executing => ("⚙ Executing...", Color::Cyan),
            ToolCallStatus::Complete => ("✓ Done", Color::Green),
            ToolCallStatus::Failed(err) => return Self::render_failed(frame, area, name, err),
        };

        let args_str = serde_json::to_string_pretty(args).unwrap_or_default();
        let args_preview: String = if args_str.len() > 200 {
            format!("{}...", &args_str[..197])
        } else {
            args_str
        };

        let mut lines = vec![
            Line::from(vec![
                Span::styled(
                    format!("{} ", status_icon),
                    Style::default().fg(status_color),
                ),
                Span::styled(
                    format!("Tool: {}", name),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  ({})", call_id),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Arguments:",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        for line in args_preview.lines() {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(Color::White),
            )));
        }

        let block = Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(status_color))
            .title(" Tool Call ")
            .title_style(Style::default().fg(status_color));

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }

    fn render_denied(frame: &mut Frame, area: Rect, name: &str, reason: &str) {
        let text = vec![
            Line::from(vec![
                Span::styled("✗ Denied: ", Style::default().fg(Color::Red)),
                Span::styled(name, Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(Span::styled(
                format!("  Reason: {}", reason),
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let block = Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(Color::Red));
        frame.render_widget(Paragraph::new(text).block(block), area);
    }

    fn render_failed(frame: &mut Frame, area: Rect, name: &str, error: &str) {
        let text = vec![
            Line::from(vec![
                Span::styled("✗ Failed: ", Style::default().fg(Color::Red)),
                Span::styled(name, Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(Span::styled(
                format!("  Error: {}", error),
                Style::default().fg(Color::Red),
            )),
        ];
        let block = Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(Color::Red));
        frame.render_widget(Paragraph::new(text).block(block), area);
    }
}

/// Render a tool result block
pub struct ToolResultBlock;

impl ToolResultBlock {
    /// Render a collapsible tool result
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        result: &str,
        success: bool,
        call_id: &str,
    ) {
        let color = if success { Color::Green } else { Color::Red };
        let icon = if success { "✓" } else { "✗" };

        let result_preview: String = if result.len() > 500 {
            format!("{}...\n\n[Result truncated, {} total chars]", &result[..497], result.len())
        } else {
            result.to_string()
        };

        let mut lines = vec![
            Line::from(vec![
                Span::styled(
                    format!("{} Result ", icon),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("({})", call_id),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
            Line::from(""),
        ];

        for line in result_preview.lines() {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(Color::Gray),
            )));
        }

        let block = Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(color));

        frame.render_widget(Paragraph::new(lines).block(block), area);
    }
}
