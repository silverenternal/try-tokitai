//! 可观测性工具集
//!
//! 将 observability 模块封装为 tokitai ToolProvider

use tokitai::tool;
use tokitai::Value;
use super::tracing::{TracingRecorder, TraceSpan, SpanStatus};
use std::sync::Arc;
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use chrono::{DateTime, Local};

/// 可观测性工具集
#[tool]
pub struct ObservabilityTools {
    recorder: Arc<RwLock<TracingRecorder>>,
    storage_dir: PathBuf,
}

impl ObservabilityTools {
    /// 创建新的可观测性工具集
    pub fn new(log_dir: &str) -> Result<Self, String> {
        let storage_dir = PathBuf::from(log_dir);
        fs::create_dir_all(&storage_dir)
            .map_err(|e| format!("创建追踪目录失败：{}", e))?;
        
        let recorder = TracingRecorder::new(storage_dir.clone(), false)
            .map_err(|e| format!("创建追踪记录器失败：{}", e))?;
        
        Ok(Self {
            recorder: Arc::new(RwLock::new(recorder)),
            storage_dir,
        })
    }

    /// 使用共享记录器创建工具集
    pub fn with_shared_recorder(
        recorder: Arc<RwLock<TracingRecorder>>,
        storage_dir: PathBuf,
    ) -> Self {
        Self { recorder, storage_dir }
    }

    /// 获取共享记录器的引用
    pub fn get_recorder(&self) -> Arc<RwLock<TracingRecorder>> {
        self.recorder.clone()
    }

    /// 从 JSONL 文件加载追踪记录
    fn load_traces_from_file(&self, file_path: &Path) -> Result<Vec<TraceSpan>, String> {
        if !file_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(file_path)
            .map_err(|e| format!("打开追踪文件失败：{}", e))?;
        let reader = BufReader::new(file);
        
        let mut spans = Vec::new();
        for line in reader.lines() {
            let line = line.map_err(|e| format!("读取追踪文件失败：{}", e))?;
            if line.trim().is_empty() {
                continue;
            }
            
            let span: TraceSpan = serde_json::from_str(&line)
                .map_err(|e| format!("解析追踪记录失败：{}", e))?;
            spans.push(span);
        }

        Ok(spans)
    }

    /// 获取所有追踪文件
    fn get_trace_files(&self) -> Result<Vec<PathBuf>, String> {
        let mut files = Vec::new();
        
        if !self.storage_dir.exists() {
            return Ok(files);
        }

        for entry in fs::read_dir(&self.storage_dir)
            .map_err(|e| format!("读取追踪目录失败：{}", e))?
        {
            let entry = entry.map_err(|e| format!("读取目录条目失败：{}", e))?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                files.push(path);
            }
        }

        Ok(files)
    }

    /// 按 trace_id 查询追踪
    fn query_by_trace_id(&self, trace_id: &str) -> Result<Vec<TraceSpan>, String> {
        let mut all_spans = Vec::new();
        
        for file_path in self.get_trace_files()? {
            let spans = self.load_traces_from_file(&file_path)?;
            let filtered: Vec<_> = spans
                .into_iter()
                .filter(|s| s.trace_id == trace_id)
                .collect();
            all_spans.extend(filtered);
        }

        all_spans.sort_by_key(|s| s.start_time_ms);
        Ok(all_spans)
    }

    /// 获取所有追踪记录
    fn get_all_traces(&self) -> Result<Vec<TraceSpan>, String> {
        let mut all_spans = Vec::new();
        
        for file_path in self.get_trace_files()? {
            let spans = self.load_traces_from_file(&file_path)?;
            all_spans.extend(spans);
        }

        Ok(all_spans)
    }
}

impl Default for ObservabilityTools {
    fn default() -> Self {
        Self::new(".tokitai/traces").unwrap_or_else(|_| {
            Self::new("target/traces").unwrap()
        })
    }
}

#[tool]
impl ObservabilityTools {
    /// 获取最近的追踪记录
    #[tool(description = "获取最近的执行记录，用于调试和审计")]
    pub fn get_recent_traces(&self, limit: Option<usize>) -> Result<Value, String> {
        let limit = limit.unwrap_or(10);
        let mut all_spans = self.get_all_traces()?;
        
        all_spans.sort_by_key(|s| -(s.start_time_ms));
        let traces: Vec<Value> = all_spans
            .into_iter()
            .take(limit)
            .map(|span| {
                serde_json::json!({
                    "span_id": &span.span_id,
                    "trace_id": &span.trace_id,
                    "name": &span.name,
                    "type": format!("{:?}", span.span_type),
                    "start_time": DateTime::from_timestamp(span.start_time_ms / 1000, 0)
                        .map(|t| t.to_rfc3339()),
                    "duration_ms": span.duration_ms,
                    "status": if span.status == SpanStatus::Error { "error" } else { "ok" },
                    "error_message": span.error_message,
                })
            })
            .collect();

        Ok(serde_json::json!(traces))
    }

    /// 获取统计信息
    #[tool(description = "获取追踪统计信息，包括总追踪数、平均执行时长、错误率等")]
    pub fn get_stats(&self) -> Result<Value, String> {
        let all_spans = self.get_all_traces()?;
        let total_spans = all_spans.len();
        
        let mut trace_groups: std::collections::HashMap<String, Vec<&TraceSpan>> = 
            std::collections::HashMap::new();
        for span in &all_spans {
            trace_groups.entry(span.trace_id.clone())
                .or_insert_with(Vec::new)
                .push(span);
        }
        let unique_traces = trace_groups.len();

        let mut total_duration = 0i64;
        let mut error_count = 0usize;
        let mut span_type_counts: std::collections::HashMap<String, usize> = 
            std::collections::HashMap::new();

        for span in &all_spans {
            if let Some(duration) = span.duration_ms {
                total_duration += duration;
            }
            
            if span.status == SpanStatus::Error {
                error_count += 1;
            }

            let type_name = format!("{:?}", span.span_type);
            *span_type_counts.entry(type_name).or_insert(0) += 1;
        }

        let error_rate = if total_spans == 0 {
            0.0
        } else {
            (error_count as f64 / total_spans as f64) * 100.0
        };
        
        let avg_duration_ms = if total_spans == 0 {
            None
        } else {
            Some(total_duration / total_spans as i64)
        };

        let earliest_trace = all_spans.iter()
            .map(|s| s.start_time_ms)
            .min()
            .and_then(|t| DateTime::from_timestamp(t / 1000, 0));
        
        let latest_trace = all_spans.iter()
            .map(|s| s.start_time_ms)
            .max()
            .and_then(|t| DateTime::from_timestamp(t / 1000, 0));

        Ok(serde_json::json!({
            "unique_traces": unique_traces,
            "total_spans": total_spans,
            "error_count": error_count,
            "error_rate": format!("{:.2}%", error_rate),
            "avg_duration_ms": avg_duration_ms,
            "span_type_distribution": span_type_counts,
            "earliest_trace": earliest_trace.map(|t| t.to_rfc3339()),
            "latest_trace": latest_trace.map(|t| t.to_rfc3339()),
        }))
    }

    /// 查询指定 trace_id 的完整执行链
    #[tool(description = "根据 trace_id 查询完整执行链")]
    pub fn query_trace(&self, trace_id: String) -> Result<Value, String> {
        let spans = self.query_by_trace_id(&trace_id)?;
        
        if spans.is_empty() {
            return Ok(serde_json::json!({
                "error": "未找到指定的追踪记录",
                "trace_id": trace_id
            }));
        }

        let total_duration: i64 = spans.iter()
            .filter_map(|s| s.duration_ms)
            .sum();
        
        let error_count = spans.iter()
            .filter(|s| s.status == SpanStatus::Error)
            .count();

        Ok(serde_json::json!({
            "trace_id": trace_id,
            "total_spans": spans.len(),
            "total_duration_ms": total_duration,
            "error_count": error_count,
        }))
    }

    /// 查询错误追踪
    #[tool(description = "查询错误的追踪记录")]
    pub fn query_errors(&self, limit: Option<usize>) -> Result<Value, String> {
        let mut all_spans = self.get_all_traces()?;
        let mut errors: Vec<_> = all_spans
            .drain(..)
            .filter(|s| s.status == SpanStatus::Error)
            .collect();
        
        errors.sort_by_key(|s| -(s.start_time_ms));
        
        if let Some(limit) = limit {
            errors.truncate(limit);
        }
        
        let result: Vec<Value> = errors
            .iter()
            .map(|span| {
                serde_json::json!({
                    "span_id": &span.span_id,
                    "trace_id": &span.trace_id,
                    "name": &span.name,
                    "type": format!("{:?}", span.span_type),
                    "error_message": span.error_message,
                    "start_time": DateTime::from_timestamp(span.start_time_ms / 1000, 0)
                        .map(|t| t.to_rfc3339()),
                })
            })
            .collect();

        Ok(serde_json::json!(result))
    }

    /// 导出追踪数据
    #[tool(description = "导出追踪数据为 JSON 文件")]
    pub fn export_traces(&self, output_path: String, trace_id: Option<String>) -> Result<Value, String> {
        let spans = if let Some(tid) = trace_id.as_deref() {
            self.query_by_trace_id(tid)?
        } else {
            self.get_all_traces()?
        };

        let mut file = File::create(&output_path)
            .map_err(|e| format!("创建输出文件失败：{}", e))?;
        
        let mut count = 0;
        for span in &spans {
            let json = serde_json::to_string(span)
                .map_err(|e| format!("序列化追踪数据失败：{}", e))?;
            writeln!(file, "{}", json)
                .map_err(|e| format!("写入文件失败：{}", e))?;
            count += 1;
        }

        Ok(serde_json::json!({
            "message": "追踪数据已导出",
            "output_path": output_path,
            "exported_count": count,
        }))
    }

    /// 清理旧的追踪文件
    #[tool(description = "清理超过指定天数的追踪文件")]
    pub fn cleanup_old_traces(&self, keep_days: Option<u32>) -> Result<Value, String> {
        let keep_days = keep_days.unwrap_or(7);
        let cutoff = Local::now()
            .checked_sub_signed(chrono::Duration::days(keep_days as i64))
            .unwrap();
        
        let cutoff_ms = cutoff.timestamp_millis();
        let mut deleted = 0;

        for file_path in self.get_trace_files()? {
            let spans = self.load_traces_from_file(&file_path)?;
            let has_recent = spans.iter().any(|s| s.start_time_ms > cutoff_ms);
            
            if !has_recent {
                fs::remove_file(&file_path)
                    .map_err(|e| format!("删除追踪文件失败：{}", e))?;
                deleted += 1;
            }
        }

        Ok(serde_json::json!({
            "message": format!("已清理 {} 天前的追踪文件", keep_days),
            "deleted_files": deleted,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_get_recent_traces_empty() {
        let dir = tempdir().unwrap();
        let tools = ObservabilityTools::new(dir.path().to_str().unwrap()).unwrap();
        let traces = tools.get_recent_traces(Some(10)).unwrap();
        assert!(traces.is_array());
    }

    #[test]
    fn test_get_stats_empty() {
        let dir = tempdir().unwrap();
        let tools = ObservabilityTools::new(dir.path().to_str().unwrap()).unwrap();
        let stats = tools.get_stats().unwrap();
        
        assert!(stats.is_object());
        assert_eq!(stats["unique_traces"], 0);
        assert_eq!(stats["total_spans"], 0);
    }
}
