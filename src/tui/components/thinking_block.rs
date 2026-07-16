//! Collapsible thinking block rendering

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render a collapsible thinking/reasoning block
pub struct ThinkingBlock;

impl ThinkingBlock {
    /// Render a thinking block.
    /// If collapsed, shows a single summary line with toggle hint.
    /// If expanded, shows full content with dimmed styling.
    pub fn render(frame: &mut Frame, area: Rect, content: &str, collapsed: bool, _index: usize) {
        let toggle = if collapsed { "[+]" } else { "[-]" };

        if collapsed {
            let line = Line::from(vec![
                Span::styled(format!("{} ", toggle), Style::default().fg(Color::Blue)),
                Span::styled(
                    "Thinking...",
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::ITALIC),
                ),
            ]);

            let block = Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(Color::Blue));
            frame.render_widget(Paragraph::new(line).block(block), area);
        } else {
            let mut lines = vec![
                Line::from(vec![
                    Span::styled(
                        format!("{} Thinking ", toggle),
                        Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("(click to collapse)", Style::default().fg(Color::DarkGray)),
                ]),
                Line::from(""),
            ];

            for line in content.lines() {
                lines.push(Line::from(Span::styled(
                    line,
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::ITALIC),
                )));
            }

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue))
                .title(" Thinking ");

            frame.render_widget(Paragraph::new(lines).block(block), area);
        }
    }
}
