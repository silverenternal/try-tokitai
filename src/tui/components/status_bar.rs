//! Enhanced status bar component
//!
//! Displays model info, token usage, tool call count, and mode indicator.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Status bar state
#[derive(Debug, Clone)]
pub struct StatusBarState {
    /// Current model name
    pub model: String,
    /// Provider name
    pub provider: String,
    /// Total tokens used in current session
    pub tokens_used: usize,
    /// Number of tool calls made
    pub tool_calls: usize,
    /// Current application mode text
    pub mode_text: String,
    /// Whether there's an error
    pub error: Option<String>,
    /// Privacy level (shown as badge)
    pub privacy_level: String,
    /// Whether privacy is enforced
    pub privacy_enforced: bool,
}

impl Default for StatusBarState {
    fn default() -> Self {
        Self {
            model: String::new(),
            provider: String::new(),
            tokens_used: 0,
            tool_calls: 0,
            mode_text: "Ready".to_string(),
            error: None,
            privacy_level: "OFF".to_string(),
            privacy_enforced: false,
        }
    }
}

/// Status bar renderer
pub struct StatusBar;

impl StatusBar {
    /// Render the status bar
    pub fn render(frame: &mut Frame, area: Rect, state: &StatusBarState) {
        let mode_color = if state.error.is_some() {
            Color::Red
        } else if state.mode_text.contains("Streaming") {
            Color::Cyan
        } else if state.mode_text.contains("Tool") {
            Color::Yellow
        } else {
            Color::Green
        };

        let mode = if state.error.is_some() {
            "Error".to_string()
        } else {
            state.mode_text.clone()
        };

        let privacy_color = if state.privacy_enforced {
            Color::Red
        } else {
            Color::DarkGray
        };
        let spans = vec![
            Span::styled(
                format!("{} ", mode),
                Style::default().fg(mode_color),
            ),
            Span::styled(
                format!("🔒{} ", state.privacy_level),
                Style::default().fg(privacy_color),
            ),
            Span::styled(
                format!(
                    "| {} ({}) | {} tokens | {} tools",
                    state.model, state.provider, state.tokens_used, state.tool_calls
                ),
                Style::default().fg(Color::DarkGray),
            ),
        ];

        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(mode_color));

        let paragraph = Paragraph::new(Line::from(spans)).block(block);
        frame.render_widget(paragraph, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_bar_default() {
        let state = StatusBarState::default();
        assert!(state.mode_text == "Ready");
        assert!(state.error.is_none());
    }
}
