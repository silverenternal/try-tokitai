//! TUI 布局模块
//!
//! 定义三面板布局：左侧工具列表，中间对话区，右侧上下文/状态

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// TUI 布局结构
#[derive(Debug, Clone)]
pub struct TuiLayout {
    /// 整个区域
    pub area: Rect,
    /// 左侧工具列表区域
    pub tool_list_area: Rect,
    /// 中间对话区域
    pub chat_area: Rect,
    /// 右侧上下文/状态区域
    pub context_area: Rect,
    /// 底部状态栏
    pub status_bar_area: Rect,
}

impl TuiLayout {
    /// 计算布局
    pub fn calculate(area: Rect) -> Self {
        // 垂直布局：主区域 + 状态栏
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(1), // 状态栏
            ])
            .split(area);

        // 水平布局：工具列表 + 对话区 + 上下文
        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20), // 工具列表
                Constraint::Percentage(60), // 对话区
                Constraint::Percentage(20), // 上下文
            ])
            .split(main_chunks[0]);

        Self {
            area,
            tool_list_area: horizontal_chunks[0],
            chat_area: horizontal_chunks[1],
            context_area: horizontal_chunks[2],
            status_bar_area: main_chunks[1],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_calculation() {
        let area = Rect::new(0, 0, 100, 50);
        let layout = TuiLayout::calculate(area);

        assert_eq!(layout.area, area);
        assert_eq!(layout.tool_list_area.width, 20);
        assert_eq!(layout.chat_area.width, 60);
        assert_eq!(layout.context_area.width, 20);
        assert_eq!(layout.status_bar_area.height, 1);
    }
}
