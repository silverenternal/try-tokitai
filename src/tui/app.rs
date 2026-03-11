//! TUI 应用状态和业务逻辑（优化版：流式响应 + 缓存 + 连接池）

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use dirs::home_dir;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tui_input::{Input, InputRequest};

use super::api_client::{ApiConfig, StreamEvent};
use super::assistant::Assistant;
use crate::path_resolver;

/// ========== 配置常量 ==========

/// 应用版本
pub const APP_VERSION: &str = "0.2.0";

/// 简洁 Logo
pub const LOGO: &str = "🔥 Tokitai AI Assistant";

/// 消息历史最大条目数
const MAX_MESSAGES: usize = 100;

/// 输入历史最大条目数
const MAX_INPUT_HISTORY: usize = 50;

/// 自动滚动阈值：当消息超过此数量时自动滚动到底部
const AUTO_SCROLL_THRESHOLD: usize = 25;

/// ========== 错误类型 ==========
#[derive(Error, Debug)]
pub enum TuiError {
    #[error("终端初始化失败：{0}")]
    TerminalInit(#[from] std::io::Error),

    #[error("API 请求失败：{0}")]
    ApiRequest(String),

    #[error("网络超时")]
    Timeout,

    #[error("认证失败：请检查 AI_API_KEY")]
    AuthFailed,

    #[error("消息持久化失败：{0}")]
    PersistFailed(#[from] serde_json::Error),

    #[error("通道错误：{0}")]
    ChannelError(String),

    #[error("模型响应格式错误")]
    InvalidResponse,
}

/// 消息类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    User,
    Assistant,
    System,
    Error,
}

impl MessageType {
    /// 获取显示名称（统一中文风格）
    pub fn display_name(&self) -> &'static str {
        match self {
            MessageType::User => "你",
            MessageType::Assistant => "助手",
            MessageType::System => "系统",
            MessageType::Error => "错误",
        }
    }

    /// 获取显示颜色
    pub fn color(&self) -> Color {
        match self {
            MessageType::User => Color::Cyan,
            MessageType::Assistant => Color::Green,
            MessageType::System => Color::Yellow,
            MessageType::Error => Color::Red,
        }
    }
}

/// 消息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub content: String,
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            message_type: MessageType::User,
            timestamp: Utc::now(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            message_type: MessageType::Assistant,
            timestamp: Utc::now(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            message_type: MessageType::System,
            timestamp: Utc::now(),
        }
    }

    pub fn error(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            message_type: MessageType::Error,
            timestamp: Utc::now(),
        }
    }

    /// 追加内容（用于流式响应）
    pub fn append(&mut self, content: &str) {
        self.content.push_str(content);
        self.timestamp = Utc::now();
    }
}

/// 消息历史持久化结构
#[derive(Debug, Serialize, Deserialize)]
struct MessageHistory {
    messages: Vec<Message>,
    saved_at: DateTime<Utc>,
}

/// 应用状态
pub struct App {
    running: bool,
    messages: VecDeque<Message>,
    input: Input,
    /// 自动滚动标志：true 表示自动滚动到底部，false 表示用户手动控制了滚动
    auto_scroll: bool,
    /// 用户自定义滚动偏移（仅在 auto_scroll=false 时使用）
    user_scroll_offset: usize,
    loading: bool,
    status_message: String,
    max_messages: usize,
    input_history: VecDeque<String>,
    input_history_index: Option<usize>,
    input_buffer: String,

    // 流式响应接收器
    stream_rx: Option<std::sync::mpsc::Receiver<StreamEvent>>,

    // 当前正在构建的助手消息（流式累积）
    streaming_message: Option<Message>,

    // AI 助手（整合 tokitai 工具调用）
    assistant: Assistant,

    history_file: Option<PathBuf>,
}

impl App {
    // ========== Getter/Setter 方法 ==========

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn set_running(&mut self, running: bool) {
        self.running = running;
    }

    pub fn messages(&self) -> &VecDeque<Message> {
        &self.messages
    }

    pub fn input(&self) -> &Input {
        &self.input
    }

    pub fn input_mut(&mut self) -> &mut Input {
        &mut self.input
    }

    /// 获取当前滚动偏移（事件驱动：自动滚动时返回 0）
    pub fn scroll_offset(&self) -> usize {
        if self.auto_scroll {
            0 // 自动滚动时始终看最新消息
        } else {
            self.user_scroll_offset
        }
    }

    /// 检查是否需要自动滚动（当消息超过阈值时强制回滚到底部）
    fn check_auto_scroll(&mut self) {
        // 当消息超过阈值时，强制自动滚动到底部（忽略用户之前的手动滚动）
        if self.messages.len() >= AUTO_SCROLL_THRESHOLD {
            self.auto_scroll = true;
            self.user_scroll_offset = 0;
        }
    }

    /// 添加消息到历史（带自动回滚）
    pub fn add_message(&mut self, message: Message) {
        if self.messages.len() >= self.max_messages {
            self.messages.pop_front();
        }
        self.messages.push_back(message);
        // 检查是否需要自动滚动（事件驱动：超过阈值时自动回滚）
        self.check_auto_scroll();
        // 自动保存历史
        self.save_history();
    }

    pub fn is_loading(&self) -> bool {
        self.loading
    }

    pub fn status_message(&self) -> &str {
        &self.status_message
    }

    pub fn set_status_message(&mut self, msg: String) {
        self.status_message = msg;
    }

    pub fn streaming_message(&self) -> Option<&Message> {
        self.streaming_message.as_ref()
    }

    pub fn input_history(&self) -> &VecDeque<String> {
        &self.input_history
    }

    pub fn input_history_index(&self) -> Option<usize> {
        self.input_history_index
    }

    // ========== 内部方法 ==========

    /// 设置历史文件路径
    fn set_history_file(&mut self, path: PathBuf) {
        self.history_file = Some(path);
    }
}

impl Default for App {
    fn default() -> Self {
        let mut messages = VecDeque::with_capacity(MAX_MESSAGES);
        messages.push_back(Message::system(
            "欢迎使用 Tokitai AI 助手！\n\
             快捷键：Enter 发送 | ↑/↓ 历史 | PgUp/PgDn 滚动 | Ctrl+L 清除 | Ctrl+C/Q 退出\n\
             更多：Ctrl+A/E 行首尾 | Ctrl+U/K 删除 | Ctrl+W 删词 | Ctrl+R 清空缓存",
        ));

        Self {
            running: true,
            messages,
            input: Input::default(),
            auto_scroll: true, // 默认启用自动滚动
            user_scroll_offset: 0,
            loading: false,
            status_message: String::from("就绪"),
            max_messages: MAX_MESSAGES,
            input_history: VecDeque::new(),
            input_history_index: None,
            input_buffer: String::new(),
            stream_rx: None,
            streaming_message: None,
            assistant: Assistant::new(ApiConfig::default()),
            history_file: None,
        }
    }
}

impl App {
    /// 创建新实例并尝试加载历史
    pub fn new() -> Self {
        let mut app = Self::default();
        // 使用跨平台路径
        if let Some(path) = get_history_file_path() {
            app.set_history_file(path.clone());
            app.load_history(&path);
        }
        app
    }

    /// 设置历史文件路径
    pub fn with_history_file(mut self, path: &PathBuf) -> Self {
        self.set_history_file(path.clone());
        self.load_history(path);
        self
    }

    /// 处理应用事件
    pub fn handle_event(&mut self, event: crate::tui::event::AppEvent) {
        use crate::tui::event::AppEvent;

        match event {
            AppEvent::Quit => {
                self.set_running(false);
            }
            AppEvent::SendMessage => {
                self.handle_input();
            }
            AppEvent::ScrollUp => {
                self.scroll_up();
            }
            AppEvent::ScrollDown => {
                self.scroll_down();
            }
            AppEvent::ClearHistory => {
                self.clear_history();
            }
            AppEvent::InputHistoryPrev => {
                self.input_history_prev();
            }
            AppEvent::InputHistoryNext => {
                self.input_history_next();
            }
            AppEvent::DeleteToStart => {
                self.input_mut().handle(InputRequest::DeleteLine);
            }
            AppEvent::DeleteToEnd => {
                self.input_mut().handle(InputRequest::DeleteTillEnd);
            }
            AppEvent::DeleteWordBackward => {
                self.input_mut().handle(InputRequest::DeletePrevWord);
            }
            AppEvent::GoToStart => {
                self.input_mut().handle(InputRequest::GoToStart);
            }
            AppEvent::GoToEnd => {
                self.input_mut().handle(InputRequest::GoToEnd);
            }
            AppEvent::ClearCache => {
                // 清空缓存（如果需要可以在 Assistant 中添加方法）
                self.set_status_message("缓存功能暂不支持".to_string());
            }
            AppEvent::Other => {
                // 其他输入由 tui-input 在 ui.rs 中处理
            }
        }
    }

    /// 处理用户输入（流式版本，支持工具调用）
    pub fn handle_input(&mut self) {
        if self.input().value().trim().is_empty() || self.is_loading() {
            return;
        }

        let user_input = self.input().value().to_string();

        // 处理 @path 语法
        let (processed_input, file_contents) = match path_resolver::resolve_paths(&user_input) {
            Ok(result) => result,
            Err(e) => {
                self.add_message(Message::error(format!("路径解析失败：{}", e)));
                return;
            }
        };

        // 添加到输入历史
        self.input_history.push_back(user_input.clone());
        if self.input_history.len() > MAX_INPUT_HISTORY {
            self.input_history.pop_front();
        }
        self.input_history_index = None;

        // 添加用户消息（使用处理后的输入）
        self.add_message(Message::user(&processed_input));

        // 如果有文件内容被加载，添加系统提示
        if !file_contents.is_empty() {
            self.add_message(Message::system(format!("📎 已加载 {} 个文件内容", file_contents.len())));
        }

        // 清空输入框
        self.input_mut().reset();
        self.input_buffer.clear();

        self.loading = true;
        self.set_status_message("正在思考...".to_string());

        // 创建 channel 用于接收流式响应
        let (tx, rx) = std::sync::mpsc::channel::<StreamEvent>();
        self.stream_rx = Some(rx);

        // 构建消息历史（转换为 AI API 格式）
        let messages: Vec<Value> = self
            .messages()
            .iter()
            .filter(|m| m.message_type != MessageType::System && m.message_type != MessageType::Error)
            .map(|m| {
                json!({
                    "role": if m.message_type == MessageType::User { "user" } else { "assistant" },
                    "content": m.content
                })
            })
            .collect();

        // 使用 AI 助手发起流式请求（支持工具调用）
        if let Err(e) = self.assistant.chat_stream(&messages, tx) {
            self.add_message(Message::error(format!("请求失败：{}", e)));
            self.loading = false;
        }
    }

    /// 检查并处理流式响应（优化版：批量处理事件，提高响应速度）
    pub fn check_response(&mut self) {
        // 没有流式接收器时直接返回
        let stream_rx = match &mut self.stream_rx {
            Some(rx) => rx,
            None => return,
        };

        // 批量处理所有可用事件（提高响应速度）
        let mut processed_count = 0;
        const MAX_EVENTS_PER_FRAME: usize = 100; // 每帧最多处理 100 个事件

        // 跟踪是否需要更新状态
        let is_first_chunk = self.streaming_message.is_none();
        let mut has_new_content = false;
        let mut done = false;
        let mut error_msg: Option<String> = None;

        // 累积的文本块（在循环外处理，避免借用冲突）
        let mut accumulated_text: Vec<String> = Vec::new();

        while processed_count < MAX_EVENTS_PER_FRAME {
            match stream_rx.try_recv() {
                Ok(StreamEvent::Text(chunk)) => {
                    processed_count += 1;
                    has_new_content = true;
                    accumulated_text.push(chunk);
                }
                Ok(StreamEvent::Done) => {
                    done = true;
                    break;
                }
                Ok(StreamEvent::Error(e)) => {
                    error_msg = Some(e);
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // 还没有数据，退出循环
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    error_msg = Some("响应通道意外断开".to_string());
                    break;
                }
            }
        }

        // 在循环外处理文本累积（避免借用冲突）
        if has_new_content {
            for chunk in accumulated_text {
                if self.streaming_message.is_none() {
                    self.streaming_message = Some(Message::assistant(chunk));
                } else if let Some(ref mut msg) = self.streaming_message {
                    msg.append(&chunk);
                }
            }
            // 检查自动滚动
            self.check_auto_scroll();
            if is_first_chunk {
                self.set_status_message("正在输入...".to_string());
            }
        }

        if done {
            // 流式完成，添加完整消息
            if let Some(msg) = self.streaming_message.take() {
                self.add_message(msg);
            }
            self.loading = false;
            self.set_status_message("就绪".to_string());
            self.stream_rx = None;
        }

        if let Some(e) = error_msg {
            self.add_message(Message::error(format!("错误：{}", e)));
            self.loading = false;
            self.streaming_message = None;
            self.stream_rx = None;
            self.set_status_message("发生错误".to_string());
        }
    }

    /// 滚动消息（向上 = 看更早的消息）
    pub fn scroll_up(&mut self) {
        // 用户手动滚动时，禁用自动滚动
        self.auto_scroll = false;
        let max_scroll = self.messages.len().saturating_sub(1);
        if self.user_scroll_offset < max_scroll {
            self.user_scroll_offset += 1;
        }
    }

    /// 滚动消息（向下 = 看更新的消息）
    pub fn scroll_down(&mut self) {
        if self.user_scroll_offset > 0 {
            self.user_scroll_offset -= 1;
        }
        // 滚动到底部时，恢复自动滚动
        if self.user_scroll_offset == 0 {
            self.auto_scroll = true;
        }
    }

    /// 滚动到底部
    pub fn scroll_to_bottom(&mut self) {
        self.user_scroll_offset = 0;
        self.auto_scroll = true;
    }

    /// 清除历史
    pub fn clear_history(&mut self) {
        self.messages.clear();
        self.messages.push_back(Message::system("历史记录已清除"));
        self.set_status_message("历史已清除".to_string());
        self.save_history();
    }

    /// 输入历史 - 上一条
    pub fn input_history_prev(&mut self) {
        if self.input_history.is_empty() {
            return;
        }

        // 第一次按 ↑，保存当前输入
        if self.input_history_index.is_none() {
            self.input_buffer = self.input().value().to_string();
            self.input_history_index = Some(self.input_history.len());
        }

        if let Some(idx) = self.input_history_index {
            if idx > 0 {
                let new_idx = idx - 1;
                self.input_history_index = Some(new_idx);
                let value = self.input_history.get(new_idx).cloned().unwrap_or_default();
                self.set_input_value(value);
            }
        }
    }

    /// 输入历史 - 下一条
    pub fn input_history_next(&mut self) {
        if self.input_history.is_empty() {
            return;
        }

        if let Some(idx) = self.input_history_index {
            if idx < self.input_history.len().saturating_sub(1) {
                let new_idx = idx + 1;
                self.input_history_index = Some(new_idx);
                let value = self.input_history.get(new_idx).cloned().unwrap_or_default();
                self.set_input_value(value);
            } else {
                // 回到当前输入
                self.input_history_index = None;
                self.set_input_value(self.input_buffer.clone());
            }
        }
    }

    /// 设置输入值（辅助方法）
    fn set_input_value(&mut self, value: String) {
        self.input_mut().handle(InputRequest::DeleteLine);
        for c in value.chars() {
            self.input_mut().handle(InputRequest::InsertChar(c));
        }
    }

    /// 保存历史到文件
    fn save_history(&self) {
        if let Some(path) = &self.history_file {
            let history = MessageHistory {
                messages: self.messages.iter().cloned().collect(),
                saved_at: Utc::now(),
            };

            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }

            match serde_json::to_string_pretty(&history) {
                Ok(json) => {
                    let _ = fs::write(path, json);
                }
                Err(e) => eprintln!("保存历史失败：{}", e),
            }
        }
    }

    /// 从文件加载历史（带损坏处理）
    fn load_history(&mut self, path: &Path) {
        match fs::read_to_string(path) {
            Ok(content) => {
                match serde_json::from_str::<MessageHistory>(&content) {
                    Ok(history) => {
                        let today = Utc::now().date_naive();
                        if history.saved_at.date_naive() == today {
                            self.messages = history.messages.into();
                            self.status_message = String::from("已恢复今日历史");
                        }
                    }
                    Err(e) => {
                        let backup_path = format!(
                            "{}.corrupted.{}",
                            path.display(),
                            Utc::now().timestamp()
                        );
                        if let Err(rename_err) = fs::rename(path, &backup_path) {
                            eprintln!("备份损坏文件失败：{}", rename_err);
                        }
                        self.add_message(Message::system(format!(
                            "历史文件损坏，已备份到：{} (错误：{})",
                            backup_path, e
                        )));
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // 文件不存在，正常
            }
            Err(e) => {
                eprintln!("读取历史文件失败：{}", e);
            }
        }
    }

    /// 获取缓存统计
    pub fn get_cache_stats(&self) -> (u64, u64) {
        // TODO: 在 Assistant 中添加缓存统计功能
        (0, 0)
    }
}

/// 获取跨平台历史文件路径
fn get_history_file_path() -> Option<PathBuf> {
    let home = home_dir()?;

    let config_dir = if cfg!(windows) {
        home.join("AppData").join("Local").join("tokitai")
    } else if cfg!(target_os = "macos") {
        home.join("Library").join("Application Support").join("tokitai")
    } else {
        home.join(".config").join("tokitai")
    };

    Some(config_dir.join("history.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_type_display() {
        assert_eq!(MessageType::User.display_name(), "你");
        assert_eq!(MessageType::Assistant.display_name(), "助手");
        assert_eq!(MessageType::System.display_name(), "系统");
        assert_eq!(MessageType::Error.display_name(), "错误");
    }

    #[test]
    fn test_scroll_boundary() {
        let mut app = App::default();
        assert_eq!(app.messages.len(), 1);

        // 禁用自动滚动以便测试手动滚动
        app.auto_scroll = false;

        app.add_message(Message::user("test1"));
        app.add_message(Message::user("test2"));
        app.add_message(Message::user("test3"));

        assert_eq!(app.user_scroll_offset, 0);

        app.scroll_up();
        assert_eq!(app.user_scroll_offset, 1);

        app.scroll_up();
        assert_eq!(app.user_scroll_offset, 2);

        app.scroll_up();
        assert_eq!(app.user_scroll_offset, 3);

        app.scroll_up();
        assert_eq!(app.user_scroll_offset, 3);

        app.scroll_down();
        assert_eq!(app.user_scroll_offset, 2);

        // 滚动到底部时恢复自动滚动
        app.scroll_to_bottom();
        assert!(app.auto_scroll);
        assert_eq!(app.user_scroll_offset, 0);
    }

    #[test]
    fn test_input_history_navigation() {
        let mut app = App::default();

        app.input_history.push_back("hello".to_string());
        app.input_history.push_back("world".to_string());

        app.input_history_prev();
        assert_eq!(app.input_history_index, Some(1));
        assert_eq!(app.input.value(), "world");

        app.input_history_prev();
        assert_eq!(app.input_history_index, Some(0));
        assert_eq!(app.input.value(), "hello");

        app.input_history_prev();
        assert_eq!(app.input_history_index, Some(0));
        assert_eq!(app.input.value(), "hello");

        app.input_history_next();
        assert_eq!(app.input_history_index, Some(1));
        assert_eq!(app.input.value(), "world");

        app.input_history_next();
        assert_eq!(app.input_history_index, None);
    }

    #[test]
    fn test_get_history_file_path() {
        let path = get_history_file_path();
        assert!(path.is_some());
        let p = path.unwrap();
        assert!(p.ends_with("history.json"));
    }

    #[test]
    fn test_message_append() {
        let mut msg = Message::assistant("Hello");
        msg.append(" World");
        assert_eq!(msg.content, "Hello World");
    }
}
