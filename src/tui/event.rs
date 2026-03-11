//! 事件处理模块 - 只负责转换原始事件，不操作 App 状态

use crossterm::event::{KeyCode, KeyModifiers};

/// 应用事件枚举 - 解耦事件源和事件处理
#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    /// 退出应用
    Quit,
    /// 发送消息
    SendMessage,
    /// 向上滚动（查看更早消息）
    ScrollUp,
    /// 向下滚动（查看更新消息）
    ScrollDown,
    /// 清除历史
    ClearHistory,
    /// 清空缓存
    ClearCache,
    /// 输入历史 - 上一条
    InputHistoryPrev,
    /// 输入历史 - 下一条
    InputHistoryNext,
    /// 删除到行首 (Ctrl+U)
    DeleteToStart,
    /// 删除到行尾 (Ctrl+K)
    DeleteToEnd,
    /// 删除前一个单词 (Ctrl+W)
    DeleteWordBackward,
    /// 光标到行首 (Ctrl+A / Home)
    GoToStart,
    /// 光标到行尾 (Ctrl+E / End)
    GoToEnd,
    /// 其他输入（由 tui-input 处理）
    Other,
}

/// 处理按键事件，返回 AppEvent
pub fn handle_key_event(code: KeyCode, modifiers: KeyModifiers) -> AppEvent {
    // Ctrl 组合键
    if modifiers.contains(KeyModifiers::CONTROL) {
        match code {
            KeyCode::Char('c') | KeyCode::Char('q') => return AppEvent::Quit,
            KeyCode::Char('l') => return AppEvent::ClearHistory,
            KeyCode::Char('r') => return AppEvent::ClearCache,  // 新增：清空缓存
            KeyCode::Char('u') => return AppEvent::DeleteToStart,
            KeyCode::Char('k') => return AppEvent::DeleteToEnd,
            KeyCode::Char('w') => return AppEvent::DeleteWordBackward,
            KeyCode::Char('a') => return AppEvent::GoToStart,
            KeyCode::Char('e') => return AppEvent::GoToEnd,
            KeyCode::Char('p') => return AppEvent::InputHistoryPrev,
            KeyCode::Char('n') => return AppEvent::InputHistoryNext,
            _ => return AppEvent::Other,
        }
    }

    // 普通按键
    match code {
        KeyCode::Enter => AppEvent::SendMessage,
        KeyCode::Up => AppEvent::InputHistoryPrev,
        KeyCode::Down => AppEvent::InputHistoryNext,
        KeyCode::PageUp => AppEvent::ScrollUp,
        KeyCode::PageDown => AppEvent::ScrollDown,
        KeyCode::Home => AppEvent::GoToStart,
        KeyCode::End => AppEvent::GoToEnd,
        KeyCode::Esc => AppEvent::Quit,
        _ => AppEvent::Other,
    }
}
