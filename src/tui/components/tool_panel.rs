//! 工具列表面板
//! 
//! 显示可用工具列表，支持选择和查看工具详情

use ratatui::{
    Frame,
    layout::Rect,
    style::{Style, Color},
    widgets::{Block, Borders, List, ListItem},
};

/// 工具项
#[derive(Debug, Clone)]
pub struct ToolItem {
    pub name: String,
    pub description: String,
    pub category: String,
}

/// 工具列表状态
#[derive(Debug, Default)]
pub struct ToolListState {
    /// 所有工具
    pub tools: Vec<ToolItem>,
    /// 当前选中的工具索引
    pub selected_index: usize,
    /// 滚动偏移
    pub scroll_offset: usize,
}

impl ToolListState {
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
        }
    }
    
    /// 添加工具
    pub fn add_tool(&mut self, tool: ToolItem) {
        self.tools.push(tool);
    }
    
    /// 选择上一个工具
    pub fn select_previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }
    
    /// 选择下一个工具
    pub fn select_next(&mut self) {
        if self.selected_index < self.tools.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }
    
    /// 获取选中的工具
    pub fn selected_tool(&self) -> Option<&ToolItem> {
        self.tools.get(self.selected_index)
    }
}

/// 工具列表面板
pub struct ToolListPanel;

impl ToolListPanel {
    /// 渲染工具列表
    pub fn render(frame: &mut Frame, area: Rect, state: &ToolListState) {
        let items: Vec<ListItem> = state.tools
            .iter()
            .map(|tool| {
                ListItem::new(tool.name.as_str())
                    .style(Style::default().fg(Color::White))
            })
            .collect();
        
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("工具列表 (j/k 选择)"))
            .highlight_style(Style::default().fg(Color::Yellow).bg(Color::DarkGray));
        
        frame.render_widget(list, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tool_list_state() {
        let mut state = ToolListState::new();
        
        state.add_tool(ToolItem {
            name: "read_file".to_string(),
            description: "读取文件内容".to_string(),
            category: "file".to_string(),
        });
        
        state.add_tool(ToolItem {
            name: "write_file".to_string(),
            description: "写入文件内容".to_string(),
            category: "file".to_string(),
        });
        
        assert_eq!(state.tools.len(), 2);
        
        state.select_next();
        assert_eq!(state.selected_index, 1);
        
        state.select_previous();
        assert_eq!(state.selected_index, 0);
    }
}
