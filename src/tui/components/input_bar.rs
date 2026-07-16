//! Rich input bar component
//!
//! Provides a full-featured text input with cursor movement, history navigation,
//! and keyboard shortcuts similar to readline/Claude Code input.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// State for the input bar
#[derive(Debug, Clone)]
pub struct InputBarState {
    /// Current text buffer
    pub buffer: String,
    /// Cursor position (byte index)
    pub cursor: usize,
    /// Input history (most recent last)
    pub history: Vec<String>,
    /// Current position in history browsing (None = not browsing)
    pub history_pos: Option<usize>,
    /// Saved buffer while browsing history
    saved_buffer: String,
}

impl InputBarState {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            cursor: 0,
            history: Vec::new(),
            history_pos: None,
            saved_buffer: String::new(),
        }
    }

    /// Handle a key event. Returns Some(text) when Enter is pressed and input is non-empty.
    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<String> {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);

        match key.code {
            // Submit
            KeyCode::Enter if !ctrl => {
                let text = self.buffer.clone();
                if !text.is_empty() {
                    self.history.push(text.clone());
                    self.buffer.clear();
                    self.cursor = 0;
                    self.history_pos = None;
                    return Some(text);
                }
            }

            // Backspace
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    let byte_pos = self.byte_index(self.cursor - 1);
                    self.buffer.remove(byte_pos);
                    self.cursor -= 1;
                }
            }

            // Delete
            KeyCode::Delete => {
                if self.cursor < self.char_count() {
                    let byte_pos = self.byte_index(self.cursor);
                    self.buffer.remove(byte_pos);
                }
            }

            // Cursor movement
            KeyCode::Left if ctrl => {
                // Move to previous word
                self.cursor = self.prev_word_boundary(self.cursor);
            }
            KeyCode::Right if ctrl => {
                self.cursor = self.next_word_boundary(self.cursor);
            }
            KeyCode::Left => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor < self.char_count() {
                    self.cursor += 1;
                }
            }

            // Home/End
            KeyCode::Home | KeyCode::Char('a') if ctrl => {
                self.cursor = 0;
            }
            KeyCode::End | KeyCode::Char('e') if ctrl => {
                self.cursor = self.char_count();
            }

            // Kill to end of line
            KeyCode::Char('k') if ctrl => {
                if self.cursor < self.char_count() {
                    let byte_pos = self.byte_index(self.cursor);
                    self.buffer.truncate(byte_pos);
                }
            }

            // Kill to start of line
            KeyCode::Char('u') if ctrl => {
                if self.cursor > 0 {
                    let byte_pos = self.byte_index(self.cursor);
                    self.buffer = self.buffer[byte_pos..].to_string();
                    self.cursor = 0;
                }
            }

            // Delete word backward
            KeyCode::Char('w') if ctrl => {
                let target = self.prev_word_boundary(self.cursor);
                if target < self.cursor {
                    let start_byte = self.byte_index(target);
                    let end_byte = self.byte_index(self.cursor);
                    self.buffer.drain(start_byte..end_byte);
                    self.cursor = target;
                }
            }

            // History navigation
            KeyCode::Up if !ctrl => {
                if !self.history.is_empty() {
                    if self.history_pos.is_none() {
                        self.saved_buffer = self.buffer.clone();
                        self.history_pos = Some(self.history.len().saturating_sub(1));
                    } else if let Some(pos) = self.history_pos {
                        if pos > 0 {
                            self.history_pos = Some(pos - 1);
                        }
                    }
                    if let Some(pos) = self.history_pos {
                        self.buffer = self.history[pos].clone();
                        self.cursor = self.char_count();
                    }
                }
            }
            KeyCode::Down if !ctrl => {
                if let Some(pos) = self.history_pos {
                    if pos + 1 < self.history.len() {
                        self.history_pos = Some(pos + 1);
                        self.buffer = self.history[pos + 1].clone();
                    } else {
                        self.history_pos = None;
                        self.buffer = self.saved_buffer.clone();
                    }
                    self.cursor = self.char_count();
                }
            }

            // Clear
            KeyCode::Esc => {
                self.buffer.clear();
                self.cursor = 0;
                self.history_pos = None;
            }

            // Character input
            KeyCode::Char(c) if !ctrl || alt => {
                let byte_pos = self.byte_index(self.cursor);
                self.buffer.insert_str(byte_pos, &c.to_string());
                self.cursor += 1;
            }

            _ => {}
        }

        None
    }

    /// Get byte index from character index
    fn byte_index(&self, char_idx: usize) -> usize {
        self.buffer
            .char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or(self.buffer.len())
    }

    /// Get character count
    fn char_count(&self) -> usize {
        self.buffer.chars().count()
    }

    /// Find previous word boundary
    fn prev_word_boundary(&self, from: usize) -> usize {
        if from == 0 {
            return 0;
        }
        let chars: Vec<char> = self.buffer.chars().collect();
        let mut pos = from.saturating_sub(1);

        // Skip whitespace
        while pos > 0 && chars[pos].is_whitespace() {
            pos -= 1;
        }
        // Skip word characters
        while pos > 0 && !chars[pos].is_whitespace() {
            pos -= 1;
        }
        // If we stopped at whitespace, move past it
        if pos > 0 && chars[pos].is_whitespace() {
            pos += 1;
        }

        pos
    }

    /// Find next word boundary
    fn next_word_boundary(&self, from: usize) -> usize {
        let chars: Vec<char> = self.buffer.chars().collect();
        let len = chars.len();
        let mut pos = from;

        // Skip word characters
        while pos < len && !chars[pos].is_whitespace() {
            pos += 1;
        }
        // Skip whitespace
        while pos < len && chars[pos].is_whitespace() {
            pos += 1;
        }

        pos
    }
}

impl Default for InputBarState {
    fn default() -> Self {
        Self::new()
    }
}

/// Input bar renderer
pub struct InputBar;

impl InputBar {
    /// Render the input bar at the bottom of the screen
    pub fn render(frame: &mut Frame, area: Rect, state: &InputBarState) {
        // Build styled content showing the prompt, buffer, and cursor
        let mut spans = vec![Span::styled("> ", Style::default().fg(Color::Green))];

        let chars: Vec<char> = state.buffer.chars().collect();
        for (i, ch) in chars.iter().enumerate() {
            if i == state.cursor {
                spans.push(Span::styled(
                    ch.to_string(),
                    Style::default().fg(Color::Black).bg(Color::White),
                ));
            } else {
                spans.push(Span::styled(
                    ch.to_string(),
                    Style::default().fg(Color::White),
                ));
            }
        }

        // Cursor at end
        if state.cursor >= chars.len() {
            spans.push(Span::styled(
                " ",
                Style::default().fg(Color::Black).bg(Color::White),
            ));
        }

        // Show suggestion hint if browsing history
        if let Some(pos) = state.history_pos {
            spans.push(Span::styled(
                format!("  [history {}/{}]", pos + 1, state.history.len()),
                Style::default().fg(Color::DarkGray),
            ));
        }

        let paragraph = Paragraph::new(Line::from(spans)).block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

        frame.render_widget(paragraph, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_typing() {
        let mut state = InputBarState::new();
        // Simulate typing "hi"
        let result = state.handle_key(&KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
        assert!(result.is_none());
        let result = state.handle_key(&KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        assert!(result.is_none());
        assert_eq!(state.buffer, "hi");
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn test_input_submit() {
        let mut state = InputBarState::new();
        state.buffer = "test".to_string();
        state.cursor = 4;
        let result = state.handle_key(&KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(result, Some("test".to_string()));
        assert!(state.buffer.is_empty());
        assert_eq!(state.history, vec!["test"]);
    }

    #[test]
    fn test_backspace() {
        let mut state = InputBarState::new();
        state.buffer = "ab".to_string();
        state.cursor = 2;
        state.handle_key(&KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(state.buffer, "a");
        assert_eq!(state.cursor, 1);
    }

    #[test]
    fn test_ctrl_a_home() {
        let mut state = InputBarState::new();
        state.buffer = "hello".to_string();
        state.cursor = 5;
        state.handle_key(&KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn test_ctrl_e_end() {
        let mut state = InputBarState::new();
        state.buffer = "hello".to_string();
        state.cursor = 0;
        state.handle_key(&KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL));
        assert_eq!(state.cursor, 5);
    }
}
