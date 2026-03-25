//! 状态栏组件
//! 
//! 显示模型信息、token 使用量、工具调用统计等

use ratatui::{
    Frame,
    layout::Rect,
    style::{Style, Color},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// 状态信息
#[derive(Debug, Clone, Default)]
pub struct StatusBarState {
    /// 当前模型名称
    pub model: String,
    /// 提供商
    pub provider: String,
    /// Token 使用量
    pub tokens_used: usize,
    /// 预估成本
    pub estimated_cost: f64,
    /// 工具调用次数
    pub tool_calls: usize,
    /// 平均响应延迟 (ms)
    pub avg_latency_ms: f64,
    /// 错误信息（如果有）
    pub error: Option<String>,
}

/// 状态栏组件
pub struct StatusBar;

impl StatusBar {
    /// 渲染状态栏
    pub fn render(frame: &mut Frame, area: Rect, state: &StatusBarState) {
        let status_text = if let Some(ref error) = state.error {
            format!(
                " ❌ {} | 🤖 {} ({}) | 📊 Tokens: {} | 💰 ${:.4} | 🛠️  Tools: {} | ⏱️  {}ms",
                error,
                state.model,
                state.provider,
                state.tokens_used,
                state.estimated_cost,
                state.tool_calls,
                state.avg_latency_ms as i64
            )
        } else {
            format!(
                " 🤖 {} ({}) | 📊 Tokens: {} | 💰 ${:.4} | 🛠️  Tools: {} | ⏱️  {}ms",
                state.model,
                state.provider,
                state.tokens_used,
                state.estimated_cost,
                state.tool_calls,
                state.avg_latency_ms as i64
            )
        };
        
        let status_color = if state.error.is_some() {
            Color::Red
        } else {
            Color::Green
        };
        
        let paragraph = Paragraph::new(Line::from(vec![
            Span::styled(status_text, Style::default().fg(status_color))
        ]))
        .block(Block::default().borders(Borders::ALL).title("状态"));
        
        frame.render_widget(paragraph, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_status_bar_state_default() {
        let state = StatusBarState::default();
        assert!(state.model.is_empty());
        assert!(state.provider.is_empty());
        assert_eq!(state.tokens_used, 0);
        assert_eq!(state.tool_calls, 0);
    }
}
