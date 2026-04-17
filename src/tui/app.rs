//! TUI 应用主模块
//!
//! 实现完整的终端用户界面

use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};
use tracing::{error, info};

use crate::tui::components::{
    ChatPanel, ChatState, StatusBar, StatusBarState, ToolItem, ToolListPanel, ToolListState,
};
use crate::tui::layout::TuiLayout;

/// TUI 应用状态
pub struct TuiApp {
    /// 是否运行
    pub running: bool,
    /// 工具列表状态
    pub tool_list: ToolListState,
    /// 对话状态
    pub chat: ChatState,
    /// 状态栏状态
    pub status_bar: StatusBarState,
}

impl TuiApp {
    /// 创建新的 TUI 应用
    pub fn new() -> Self {
        let mut tool_list = ToolListState::new();

        // 添加工具示例
        tool_list.add_tool(ToolItem {
            name: "read_file".to_string(),
            description: "读取文件内容".to_string(),
            category: "file".to_string(),
        });
        tool_list.add_tool(ToolItem {
            name: "write_file".to_string(),
            description: "写入文件内容".to_string(),
            category: "file".to_string(),
        });
        tool_list.add_tool(ToolItem {
            name: "search_code".to_string(),
            description: "搜索代码".to_string(),
            category: "code".to_string(),
        });

        let mut chat = ChatState::new();
        chat.add_message(
            "assistant".to_string(),
            "👋 欢迎使用 Tokitai AI 助手！按 Ctrl+Q 退出，按 Ctrl+H 查看帮助。".to_string(),
        );

        let status_bar = StatusBarState {
            model: "qwen3.5:397b".to_string(),
            provider: "Ollama".to_string(),
            tokens_used: 0,
            estimated_cost: 0.0,
            tool_calls: 0,
            avg_latency_ms: 0.0,
            error: None,
        };

        Self {
            running: true,
            tool_list,
            chat,
            status_bar,
        }
    }

    /// 处理输入事件
    pub fn handle_event(&mut self, event: Event) {
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press {
                let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

                match key.code {
                    KeyCode::Char('q') if is_ctrl => {
                        self.running = false;
                    }
                    KeyCode::Char('l') if is_ctrl => {
                        self.chat.clear();
                    }
                    KeyCode::Char('j') => {
                        self.tool_list.select_next();
                    }
                    KeyCode::Char('k') => {
                        self.tool_list.select_previous();
                    }
                    KeyCode::Char('h') if is_ctrl => {
                        self.show_help();
                    }
                    KeyCode::Char(c) => {
                        self.chat.input.push(c);
                    }
                    KeyCode::Backspace => {
                        self.chat.input.pop();
                    }
                    KeyCode::Enter => {
                        if !self.chat.input.is_empty() {
                            // 发送消息
                            let input = std::mem::take(&mut self.chat.input);
                            self.chat.add_message("user".to_string(), input);
                            // TODO: 调用 AI 处理
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// 显示帮助
    fn show_help(&self) {
        println!("╔════════════════════════════════════════════════════════╗");
        println!("║                  Tokitai TUI 快捷键                    ║");
        println!("╠════════════════════════════════════════════════════════╣");
        println!("║  Ctrl+Q  退出                                          ║");
        println!("║  Ctrl+L  清空对话                                      ║");
        println!("║  Ctrl+H  显示帮助                                      ║");
        println!("║  j/k     选择上一个/下一个工具                         ║");
        println!("║  Enter   发送消息                                      ║");
        println!("║  Ctrl+C  中断当前操作                                  ║");
        println!("╚════════════════════════════════════════════════════════╝");
    }

    /// 渲染界面
    pub fn render(&self, frame: &mut ratatui::Frame) {
        let layout = TuiLayout::calculate(frame.size());

        // 渲染工具列表
        ToolListPanel::render(frame, layout.tool_list_area, &self.tool_list);

        // 渲染对话区域
        ChatPanel::render(frame, layout.chat_area, &self.chat);

        // 渲染上下文区域（暂时显示选中工具详情）
        let context_block = ratatui::widgets::Block::default()
            .borders(ratatui::widgets::Borders::ALL)
            .title("工具详情");
        frame.render_widget(context_block, layout.context_area);

        // 渲染状态栏
        StatusBar::render(frame, layout.status_bar_area, &self.status_bar);
    }
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new()
    }
}

/// 运行 TUI 应用
pub fn run_tui() -> Result<()> {
    info!("启动 TUI 应用");

    // 设置终端
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 创建应用
    let mut app = TuiApp::new();

    // 主循环
    let res = run_app(&mut terminal, &mut app);

    // 恢复终端
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        error!("TUI 错误：{:?}", err);
        return Err(err);
    }

    Ok(())
}

/// 应用主循环
fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut TuiApp,
) -> Result<()> {
    while app.running {
        terminal.draw(|frame| app.render(frame))?;

        // 事件轮询
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('c') && is_ctrl {
                    app.running = false;
                } else {
                    app.handle_event(Event::Key(key));
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation() {
        let app = TuiApp::new();
        assert!(app.running);
        assert!(!app.tool_list.tools.is_empty());
        assert!(!app.chat.messages.is_empty());
    }
}
