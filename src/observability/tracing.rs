//! 全链路追踪系统
//!
//! 实现从用户输入到工具执行的完整追踪
//!
//! # 设计原则
//! - 为每个请求生成唯一 trace_id
//! - 记录所有关键事件（工具调用、状态转换、决策点）
//! - 支持按 trace_id 查询完整执行链
//! - JSONL 文件存储，无需数据库

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

/// 追踪错误类型
#[derive(Error, Debug)]
pub enum TracingError {
    #[error("文件操作失败：{0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON 处理失败：{0}")]
    JsonError(#[from] serde_json::Error),
}

/// 追踪 Span 类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpanType {
    /// 用户请求
    UserRequest,
    /// 意图识别
    IntentClassification,
    /// 工具选择
    ToolSelection,
    /// 工具执行
    ToolExecution,
    /// 响应生成
    ResponseGeneration,
    /// 状态转换
    StateTransition,
    /// 自主迭代
    AutonomousIteration,
    /// 代码审查
    CodeReview,
    /// Git 操作
    GitOperation,
}

impl std::fmt::Display for SpanType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpanType::UserRequest => write!(f, "用户请求"),
            SpanType::IntentClassification => write!(f, "意图识别"),
            SpanType::ToolSelection => write!(f, "工具选择"),
            SpanType::ToolExecution => write!(f, "工具执行"),
            SpanType::ResponseGeneration => write!(f, "响应生成"),
            SpanType::StateTransition => write!(f, "状态转换"),
            SpanType::AutonomousIteration => write!(f, "自主迭代"),
            SpanType::CodeReview => write!(f, "代码审查"),
            SpanType::GitOperation => write!(f, "Git 操作"),
        }
    }
}

/// 追踪 Span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSpan {
    /// Span ID
    pub span_id: String,
    /// 父 Span ID
    pub parent_span_id: Option<String>,
    /// Trace ID
    pub trace_id: String,
    /// Span 类型
    pub span_type: SpanType,
    /// Span 名称
    pub name: String,
    /// 开始时间戳（毫秒）
    pub start_time_ms: i64,
    /// 结束时间戳（毫秒）
    pub end_time_ms: Option<i64>,
    /// 持续时间（毫秒）
    pub duration_ms: Option<i64>,
    /// 属性
    pub attributes: HashMap<String, String>,
    /// 事件列表
    pub events: Vec<TraceEvent>,
    /// 状态（ok/error）
    pub status: SpanStatus,
    /// 错误信息
    pub error_message: Option<String>,
}

impl TraceSpan {
    /// 创建新的 Span
    pub fn new(trace_id: String, span_type: SpanType, name: String) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            span_id: Uuid::new_v4().to_string()[..16].to_string(),
            parent_span_id: None,
            trace_id,
            span_type,
            name,
            start_time_ms: now,
            end_time_ms: None,
            duration_ms: None,
            attributes: HashMap::new(),
            events: vec![],
            status: SpanStatus::Ok,
            error_message: None,
        }
    }

    /// 设置父 Span
    pub fn with_parent(mut self, parent_span_id: String) -> Self {
        self.parent_span_id = Some(parent_span_id);
        self
    }

    /// 添加属性
    pub fn with_attribute(mut self, key: String, value: String) -> Self {
        self.attributes.insert(key, value);
        self
    }

    /// 开始计时
    pub fn start(&mut self) {
        self.start_time_ms = chrono::Utc::now().timestamp_millis();
    }

    /// 结束 Span
    pub fn end(&mut self) {
        let end_time = chrono::Utc::now().timestamp_millis();
        self.end_time_ms = Some(end_time);
        self.duration_ms = Some(end_time - self.start_time_ms);
    }

    /// 标记为错误
    pub fn with_error(mut self, error_message: String) -> Self {
        self.status = SpanStatus::Error;
        self.error_message = Some(error_message);
        self.end();
        self
    }

    /// 添加事件
    pub fn add_event(&mut self, event: TraceEvent) {
        self.events.push(event);
    }
}

/// Span 状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SpanStatus {
    Ok,
    Error,
}

/// 追踪事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    /// 事件时间戳（毫秒）
    pub timestamp_ms: i64,
    /// 事件名称
    pub name: String,
    /// 事件属性
    pub attributes: HashMap<String, String>,
}

impl TraceEvent {
    /// 创建新事件
    pub fn new(name: String) -> Self {
        Self {
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            name,
            attributes: HashMap::new(),
        }
    }

    /// 添加属性
    pub fn with_attribute(mut self, key: String, value: String) -> Self {
        self.attributes.insert(key, value);
        self
    }
}

/// 追踪上下文（用于线程间传递）
#[derive(Debug, Clone)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
}

impl TraceContext {
    /// 创建新的追踪上下文
    pub fn new(trace_id: String, span_id: String) -> Self {
        Self { trace_id, span_id }
    }

    /// 生成新的 trace_id
    pub fn generate_trace_id() -> String {
        Uuid::new_v4().to_string()[..16].to_string()
    }
}

/// 追踪记录器
pub struct TracingRecorder {
    /// 存储目录
    storage_dir: PathBuf,
    /// 当前活跃的 Spans
    active_spans: HashMap<String, TraceSpan>,
    /// 已完成的 Spans
    completed_spans: Vec<TraceSpan>,
    /// 是否启用控制台输出
    console_output: bool,
}

impl TracingRecorder {
    /// 创建新的追踪记录器
    pub fn new(storage_dir: PathBuf, console_output: bool) -> Result<Self, TracingError> {
        fs::create_dir_all(&storage_dir)?;

        Ok(Self {
            storage_dir,
            active_spans: HashMap::new(),
            completed_spans: vec![],
            console_output,
        })
    }

    /// 开始新的追踪
    pub fn start_trace(&mut self, span_type: SpanType, name: String) -> TraceContext {
        let trace_id = TraceContext::generate_trace_id();
        let span = TraceSpan::new(trace_id.clone(), span_type, name);
        let span_id = span.span_id.clone();

        if self.console_output {
            println!("🔍 [TRACE] 开始：{} ({})", span.name, span.span_type);
        }

        self.active_spans.insert(span_id.clone(), span);

        TraceContext::new(trace_id, span_id)
    }

    /// 开始子 Span
    pub fn start_child_span(&mut self, parent: &TraceContext, span_type: SpanType, name: String) -> TraceContext {
        let span = TraceSpan::new(parent.trace_id.clone(), span_type, name)
            .with_parent(parent.span_id.clone());
        let span_id = span.span_id.clone();

        if self.console_output {
            println!("  ↳ [SPAN] 开始：{}", span.name);
        }

        self.active_spans.insert(span_id.clone(), span);

        TraceContext::new(parent.trace_id.clone(), span_id)
    }

    /// 结束 Span
    pub fn end_span(&mut self, context: &TraceContext) {
        if let Some(mut span) = self.active_spans.remove(&context.span_id) {
            span.end();

            if self.console_output {
                if let Some(duration) = span.duration_ms {
                    println!("  ✓ [SPAN] 完成：{} ({}ms)", span.name, duration);
                } else {
                    println!("  ✓ [SPAN] 完成：{}", span.name);
                }
            }

            // 写入文件
            self.write_span(&span).ok();

            self.completed_spans.push(span);
        }
    }

    /// 结束 Span 并标记错误
    pub fn end_span_with_error(&mut self, context: &TraceContext, error_message: String) {
        if let Some(mut span) = self.active_spans.remove(&context.span_id) {
            span.status = SpanStatus::Error;
            span.error_message = Some(error_message.clone());
            span.end();

            if self.console_output {
                println!("  ✗ [SPAN] 错误：{} - {}", span.name, error_message);
            }

            // 写入文件
            self.write_span(&span).ok();

            self.completed_spans.push(span);
        }
    }

    /// 添加事件到 Span
    pub fn add_event(&mut self, context: &TraceContext, event: TraceEvent) {
        if let Some(span) = self.active_spans.get_mut(&context.span_id) {
            span.add_event(event);
        }
    }

    /// 添加属性到 Span
    pub fn add_attribute(&mut self, context: &TraceContext, key: String, value: String) {
        if let Some(span) = self.active_spans.get_mut(&context.span_id) {
            span.attributes.insert(key, value);
        }
    }

    /// 获取追踪摘要
    pub fn get_trace_summary(&self, trace_id: &str) -> Option<TraceSummary> {
        let spans: Vec<&TraceSpan> = self.completed_spans
            .iter()
            .filter(|s| s.trace_id == trace_id)
            .collect();

        if spans.is_empty() {
            return None;
        }

        let total_duration: i64 = spans.iter()
            .filter_map(|s| s.duration_ms)
            .sum();

        let error_count = spans.iter()
            .filter(|s| s.status == SpanStatus::Error)
            .count();

        Some(TraceSummary {
            trace_id: trace_id.to_string(),
            span_count: spans.len(),
            total_duration_ms: total_duration,
            error_count,
            start_time: spans.iter().map(|s| s.start_time_ms).min().unwrap_or(0),
        })
    }

    /// 写入 Span 到文件
    fn write_span(&self, span: &TraceSpan) -> Result<(), TracingError> {
        let date = chrono::DateTime::from_timestamp(span.start_time_ms / 1000, 0)
            .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap())
            .format("%Y-%m-%d")
            .to_string();

        let file_path = self.storage_dir.join(format!("trace_{}.jsonl", date));

        let json = serde_json::to_string(span)?;
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;

        writeln!(file, "{}", json)?;

        Ok(())
    }

    /// 获取已完成的 Spans 数量
    pub fn completed_span_count(&self) -> usize {
        self.completed_spans.len()
    }

    /// 清理旧的追踪文件（保留最近 N 天）
    pub fn cleanup_old_traces(&self, keep_days: u32) -> Result<usize, TracingError> {
        let cutoff = chrono::Utc::now()
            .checked_sub_days(chrono::Days::new(keep_days as u64))
            .unwrap()
            .timestamp();

        let mut deleted = 0;

        for entry in fs::read_dir(&self.storage_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("trace_") && name.ends_with(".jsonl") {
                    // 从文件名解析日期
                    if let Some(date_str) = name.strip_prefix("trace_").and_then(|s| s.strip_suffix(".jsonl")) {
                        if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                            let timestamp = date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
                            if timestamp < cutoff {
                                fs::remove_file(&path)?;
                                deleted += 1;
                            }
                        }
                    }
                }
            }
        }

        Ok(deleted)
    }
}

/// 追踪摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSummary {
    pub trace_id: String,
    pub span_count: usize,
    pub total_duration_ms: i64,
    pub error_count: usize,
    pub start_time: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_tracing_recorder() {
        let temp_dir = TempDir::new().unwrap();
        let mut recorder = TracingRecorder::new(temp_dir.path().to_path_buf(), false).unwrap();

        // 开始追踪
        let trace_ctx = recorder.start_trace(SpanType::UserRequest, "用户请求".to_string());
        
        // 开始子 Span
        let child_ctx = recorder.start_child_span(&trace_ctx, SpanType::ToolExecution, "工具执行".to_string());
        
        // 添加事件
        recorder.add_event(&child_ctx, TraceEvent::new("工具调用开始".to_string()));
        
        // 结束子 Span
        recorder.end_span(&child_ctx);
        
        // 结束追踪
        recorder.end_span(&trace_ctx);

        // 验证
        assert_eq!(recorder.completed_span_count(), 2);
        
        let summary = recorder.get_trace_summary(&trace_ctx.trace_id).unwrap();
        assert_eq!(summary.span_count, 2);
    }

    #[test]
    fn test_trace_context() {
        let trace_id = TraceContext::generate_trace_id();
        assert_eq!(trace_id.len(), 16);
    }
}
