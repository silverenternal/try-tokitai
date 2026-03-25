//! 对话面板
//! 
//! 显示用户和 AI 的对话历史

use ratatui::{
    Frame,
    layout::Rect,
    style::{Style, Color},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

/// 对话消息
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// 对话状态
#[derive(Debug, Default)]
pub struct ChatState {
    /// 对话历史
    pub messages: Vec<ChatMessage>,
    /// 当前输入
    pub input: String,
    /// 滚动位置
    pub scroll_position: usize,
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            scroll_position: 0,
        }
    }
    
    /// 添加消息
    pub fn add_message(&mut self, role: String, content: String) {
        self.messages.push(ChatMessage { role, content });
        // 自动滚动到底部
        self.scroll_position = self.messages.len();
    }
    
    /// 清空对话
    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

/// 对话面板
pub struct ChatPanel;

impl ChatPanel {
    /// 渲染对话区域
    pub fn render(frame: &mut Frame, area: Rect, state: &ChatState) {
        // 构建对话内容
        let mut lines = Vec::new();
        
        for msg in &state.messages {
            let (prefix_color, content_color) = if msg.role == "user" {
                (Color::Cyan, Color::White)
            } else {
                (Color::Green, Color::White)
            };
            
            let prefix = if msg.role == "user" { "👤 你：" } else { "🤖 AI：" };
            lines.push(Line::from(Span::styled(prefix, Style::default().fg(prefix_color))));
            
            // 包装长文本
            let wrapped_content = wrap_text(&msg.content, area.width as usize - 2);
            for line in wrapped_content {
                lines.push(Line::from(Span::styled(line, Style::default().fg(content_color))));
            }
            
            lines.push(Line::from("")); // 空行分隔
        }
        
        // 显示当前输入
        if !state.input.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("> {}", state.input),
                Style::default().fg(Color::Yellow),
            )));
        }
        
        let paragraph = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("对话 (Ctrl+L 清空)"))
            .scroll((state.scroll_position as u16, 0));
        
        frame.render_widget(paragraph, area);
        
        // 渲染滚动条
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        
        let mut scrollbar_state = ScrollbarState::new(state.messages.len())
            .position(state.scroll_position);
        
        frame.render_stateful_widget(
            scrollbar,
            area,
            &mut scrollbar_state,
        );
    }
}

/// 文本换行
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();
    
    for word in text.split_whitespace() {
        if current_line.len() + word.len() + 1 > max_width {
            lines.push(current_line);
            current_line = word.to_string();
        } else {
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }
    }
    
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_chat_state() {
        let mut state = ChatState::new();
        
        state.add_message("user".to_string(), "Hello".to_string());
        state.add_message("assistant".to_string(), "Hi there!".to_string());
        
        assert_eq!(state.messages.len(), 2);
    }
    
    #[test]
    fn test_wrap_text() {
        let text = "This is a long line that should be wrapped";
        let lines = wrap_text(text, 15);
        
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0], "This is a");
    }
}
