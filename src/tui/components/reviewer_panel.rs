//! Reviewer feedback panel for human-in-the-loop checkpoints.

use crate::tui::research_pipeline::ReviewerFeedbackEntry;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct ReviewerPanel;

impl ReviewerPanel {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        current_run_id: Option<&str>,
        entries: &[ReviewerFeedbackEntry],
    ) {
        if area.height == 0 {
            return;
        }

        let unresolved = entries.iter().filter(|entry| !entry.resolved).count();
        let title = match current_run_id {
            Some(run_id) if !run_id.trim().is_empty() => {
                format!(
                    " Reviewer Feedback | run {} | {} open ",
                    run_id.trim(),
                    unresolved
                )
            }
            _ => format!(" Reviewer Feedback | {} open ", unresolved),
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines = Vec::new();
        if entries.is_empty() {
            lines.push(Line::from(Span::styled(
                "No reviewer feedback yet. Use /reviewer-add reviewer|score|comment|run_id(optional).",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for (idx, entry) in entries.iter().rev().take(inner.height as usize).enumerate() {
                let item_no = entries.len().saturating_sub(idx);
                let status = if entry.resolved { "[done]" } else { "[open]" };
                let status_color = if entry.resolved {
                    Color::Green
                } else {
                    Color::Yellow
                };
                let score = entry
                    .score
                    .map(|score| format!(" score={}", score))
                    .unwrap_or_default();
                let run = if entry.linked_run_id.trim().is_empty() {
                    String::new()
                } else {
                    format!(" run={}", entry.linked_run_id.trim())
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("{} ", status), Style::default().fg(status_color)),
                    Span::styled(
                        format!("#{} {}", item_no, entry.reviewer),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{}{}", score, run),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
                if !entry.comment.trim().is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", entry.comment.trim()),
                        Style::default().fg(Color::Gray),
                    )));
                }
            }
        }

        frame.render_widget(Paragraph::new(lines), inner);
    }
}
