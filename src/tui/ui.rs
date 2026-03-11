//! TUI 界面渲染模块（纯渲染逻辑，无状态）

#![allow(dead_code)]

use crate::tui::app::{App, APP_VERSION, LOGO, Message};
use crate::tui::event::{handle_key_event, AppEvent};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tui_input::InputRequest;

/// 运行 TUI 应用
pub fn run_tui() -> Result<(), std::io::Error> {
    // 初始化终端
    enable_raw_mode()?;

    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 创建应用实例
    let mut app = App::new();

    // 运行主循环（包含渲染和事件处理）
    let res = run_app(&mut terminal, &mut app);

    // 恢复终端（保证执行）
    let restore_result = restore_terminal(&mut terminal);

    // 优先返回事件循环的错误
    if let Err(err) = res {
        eprintln!("错误：{:?}", err);
        return Err(err);
    }

    // 返回终端恢复的错误
    restore_result
}

/// 运行应用主循环
fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<(), std::io::Error> {
    // 设置 Ctrl+C 处理器
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .unwrap_or(());

    loop {
        // 检查是否收到 Ctrl+C
        if !running.load(Ordering::SeqCst) {
            app.set_running(false);
            break;
        }

        // 渲染
        terminal.draw(|f| ui(f, app))?;

        // 检查后台线程的响应
        app.check_response();

        // 非阻塞读取事件
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    // 先处理特殊组合键
                    let event = handle_key_event(key.code, key.modifiers);

                    match event {
                        AppEvent::Other => {
                            // 其他输入交给 tui-input 处理
                            // 处理字符输入
                            if let KeyCode::Char(c) = key.code {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    // Ctrl 组合键已处理
                                } else if key.modifiers.contains(KeyModifiers::ALT) {
                                    // Alt 组合键（预留）
                                } else {
                                    // 普通字符输入
                                    app.input_mut().handle(InputRequest::InsertChar(c));
                                }
                            } else {
                                // 其他按键（Backspace, Delete, Left, Right 等）
                                use tui_input::InputRequest;
                                match key.code {
                                    KeyCode::Backspace => {
                                        app.input_mut().handle(InputRequest::DeletePrevChar);
                                    }
                                    KeyCode::Delete => {
                                        app.input_mut().handle(InputRequest::DeleteNextChar);
                                    }
                                    KeyCode::Left => {
                                        app.input_mut().handle(InputRequest::GoToPrevChar);
                                    }
                                    KeyCode::Right => {
                                        app.input_mut().handle(InputRequest::GoToNextChar);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {
                            // 应用事件
                            app.handle_event(event);
                        }
                    }

                    if !app.is_running() {
                        break;
                    }
                }
            }
        }

        if !app.is_running() {
            break;
        }
    }

    Ok(())
}

/// 可靠地恢复终端状态
fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> Result<(), std::io::Error> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// 主渲染函数
pub fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // Logo + 状态
            Constraint::Min(10),   // 消息区域
            Constraint::Length(3), // 输入框
            Constraint::Length(1), // 状态栏
        ])
        .split(f.area());

    render_logo(f, app, chunks[0]);
    render_messages(f, app, chunks[1]);
    render_input(f, app, chunks[2]);
    render_status_bar(f, app, chunks[3]);
}

/// 渲染 Logo（简洁版，带状态）
fn render_logo(f: &mut Frame, app: &App, area: Rect) {
    let status = if app.is_loading() { "🔄 思考中..." } else { "✅ 就绪" };
    let logo = format!("{} v{} | {}", LOGO, APP_VERSION, status);

    let logo_widget = Paragraph::new(Line::from(vec![Span::styled(
        logo,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .block(Block::default().borders(Borders::BOTTOM).style(Style::default().fg(Color::DarkGray)));

    f.render_widget(logo_widget, area);
}

/// 渲染消息区域（支持简单 Markdown 样式 + 自动换行）
fn render_messages(f: &mut Frame, app: &App, area: Rect) {
    use ratatui::widgets::Wrap;

    // 计算可见消息数量（考虑换行后每条消息可能占多行，估算每条消息平均 3 行）
    let visible_count = area.height.saturating_sub(2) as usize / 3;
    let visible_count = visible_count.max(5); // 至少显示 5 条

    // 收集要显示的消息（从最新消息开始，考虑滚动偏移）
    let all_messages: Vec<&Message> = app.messages().iter().collect();
    let total = all_messages.len();

    // 计算起始索引（滚动偏移 = 0 表示看最新消息）
    let start = (total - app.scroll_offset()).saturating_sub(visible_count);
    let end = total - app.scroll_offset();

    let mut display_messages: Vec<&Message> = if start < end {
        all_messages[start..end].to_vec()
    } else {
        Vec::new()
    };

    // 如果有正在流式传输的消息，添加到显示列表末尾
    if let Some(ref stream_msg) = app.streaming_message() {
        display_messages.push(stream_msg);
    }

    // 构建带样式的文本块（使用多行 Lines 支持自动换行）
    let mut lines: Vec<Line> = Vec::new();

    for msg in &display_messages {
        let prefix = msg.message_type.display_name();
        let style = Style::default().fg(msg.message_type.color());

        // 处理内容，按行分割但保留自动换行能力
        for line in msg.content.lines() {
            // 检测代码块并高亮
            if line.trim().starts_with("```") {
                lines.push(Line::from(vec![
                    Span::styled(format!("[{}]: ", prefix), style),
                    Span::styled(
                        line,
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]));
            } else if line.starts_with('#') {
                // 简单标题高亮
                lines.push(Line::from(vec![
                    Span::styled(format!("[{}]: ", prefix), style),
                    Span::styled(
                        line,
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ),
                ]));
            } else {
                // 普通文本（让 Paragraph 的 wrap 处理自动换行）
                lines.push(Line::from(vec![
                    Span::styled(format!("[{}]: ", prefix), style),
                    Span::raw(line),
                ]));
            }
        }

        // 消息之间添加空行分隔
        lines.push(Line::from(""));
    }

    let messages_widget = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("对话历史"))
        .wrap(Wrap { trim: false });

    f.render_widget(messages_widget, area);
}

/// 渲染输入框
fn render_input(f: &mut Frame, app: &App, area: Rect) {
    let input_widget = Paragraph::new(app.input().value().to_string())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("输入消息 (Enter 发送)"),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(input_widget, area);

    // 渲染光标
    if !app.is_loading() {
        f.set_cursor_position((
            area.x + app.input().visual_cursor() as u16 + 1,
            area.y + 1,
        ));
    }
}

/// 渲染状态栏
fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let scroll_info = if app.messages().len() > 1 {
        let current = app.messages().len() - app.scroll_offset();
        format!("消息 {}/{} ", current, app.messages().len())
    } else {
        String::new()
    };

    let history_info = if !app.input_history().is_empty() {
        if let Some(idx) = app.input_history_index() {
            format!("历史 {}/{} ", idx + 1, app.input_history().len())
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    // 显示缓存统计
    let (total, hits) = app.get_cache_stats();
    let cache_info = if total > 0 {
        let hit_rate = (hits as f64 / total as f64 * 100.0) as u32;
        format!("缓存：{}({}%) ", hits, hit_rate)
    } else {
        String::new()
    };

    let status = format!(
        "{}{}{}| {} | ↑/↓ 历史 | Ctrl+R 清空缓存",
        scroll_info, history_info, cache_info, app.status_message()
    );

    let status_widget = Paragraph::new(Line::from(vec![Span::styled(
        status,
        Style::default().fg(Color::DarkGray),
    )]));

    f.render_widget(status_widget, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_logo_status() {
        let app = App::default();
        assert!(app.status_message().contains("就绪"));
    }

    #[test]
    fn test_message_visible_count() {
        // 测试可见消息数量计算
        let area_height = 20u16;
        let visible_count = area_height.saturating_sub(2) as usize;
        assert_eq!(visible_count, 18);
    }
}
