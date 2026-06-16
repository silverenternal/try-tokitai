//! TUI layout module
//!
//! Single-panel chat layout: chat + suggestions(optional) + thinking(optional) + input + status

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// TUI layout for single-panel chat interface
#[derive(Debug, Clone)]
pub struct TuiLayout {
    pub chat_area: Rect,
    pub thinking_area: Rect,
    pub suggestions_area: Rect,
    pub input_area: Rect,
    pub status_bar_area: Rect,
}

impl TuiLayout {
    pub fn calculate(area: Rect, input_h: u16, thinking_h: u16, suggestions_h: u16) -> Self {
        let mut constraints = vec![Constraint::Min(1)]; // chat
        if suggestions_h > 0 { constraints.push(Constraint::Length(suggestions_h)); }
        if thinking_h > 0 { constraints.push(Constraint::Length(thinking_h)); }
        constraints.push(Constraint::Length(input_h));
        constraints.push(Constraint::Length(1));

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        let mut idx = 0;
        let chat_area = chunks[idx]; idx += 1;
        let suggestions_area = if suggestions_h > 0 { let a = chunks[idx]; idx += 1; a } else { Rect::default() };
        let thinking_area = if thinking_h > 0 { let a = chunks[idx]; idx += 1; a } else { Rect::default() };
        let input_area = chunks[idx]; idx += 1;
        let status_bar_area = chunks[idx];

        Self { chat_area, thinking_area, suggestions_area, input_area, status_bar_area }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_basic() {
        let layout = TuiLayout::calculate(Rect::new(0, 0, 100, 40), 3, 0, 0);
        assert_eq!(layout.chat_area.height, 36);
        assert_eq!(layout.input_area.height, 3);
        assert_eq!(layout.status_bar_area.height, 1);
    }

    #[test]
    fn test_layout_with_suggestions() {
        let layout = TuiLayout::calculate(Rect::new(0, 0, 100, 40), 3, 0, 3);
        assert_eq!(layout.suggestions_area.height, 3);
        assert_eq!(layout.chat_area.height, 33);
    }
}
