//! 上下文日志模块
//! 
//! 实现增量日志系统，记录所有上下文变更，支持回溯和审计。

use std::path::{Path, PathBuf};
use std::fs::{File, OpenOptions};
use std::io::{Write, BufRead, BufReader};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

/// 日志操作类型
#[derive(Debug, Clone)]
pub enum LogOperation {
    Add,
    Retrieve,
    Delete,
    Trim,
    Update,
}

impl std::fmt::Display for LogOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogOperation::Add => write!(f, "ADD"),
            LogOperation::Retrieve => write!(f, "RETRIEVE"),
            LogOperation::Delete => write!(f, "DELETE"),
            LogOperation::Trim => write!(f, "TRIM"),
            LogOperation::Update => write!(f, "UPDATE"),
        }
    }
}

/// 日志条目
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub hash: String,
    pub operation: LogOperation,
    pub details: Option<String>,
}

impl LogEntry {
    /// 格式化为日志行
    pub fn to_log_line(&self) -> String {
        let details = self.details.as_deref().unwrap_or("");
        format!(
            "{} | {} | {} | {} | {}",
            self.timestamp.to_rfc3339(),
            self.session_id,
            self.hash,
            self.operation,
            details
        )
    }

    /// 从日志行解析
    pub fn from_log_line(line: &str) -> Option<Self> {
        let parts: Vec<&str> = line.split(" | ").collect();
        
        if parts.len() >= 4 {
            let timestamp = DateTime::parse_from_rfc3339(parts[0]).ok()?;
            let session_id = parts[1].to_string();
            let hash = parts[2].to_string();
            let operation = match parts[3] {
                "ADD" => LogOperation::Add,
                "RETRIEVE" => LogOperation::Retrieve,
                "DELETE" => LogOperation::Delete,
                "TRIM" => LogOperation::Trim,
                "UPDATE" => LogOperation::Update,
                _ => return None,
            };
            let details = parts.get(4).map(|s| s.to_string());

            Some(Self {
                timestamp: timestamp.with_timezone(&Utc),
                session_id,
                hash,
                operation,
                details,
            })
        } else {
            None
        }
    }
}

/// 上下文日志管理器
pub struct ContextLogger {
    log_file: PathBuf,
    file: Option<File>,
}

impl ContextLogger {
    /// 创建日志管理器
    pub fn new<P: AsRef<Path>>(log_dir: P) -> Result<Self> {
        let log_dir = log_dir.as_ref();
        std::fs::create_dir_all(log_dir)
            .with_context(|| format!("Failed to create log directory: {:?}", log_dir))?;

        let log_file = log_dir.join("context_append.log");

        Ok(Self {
            log_file,
            file: None,
        })
    }

    /// 打开日志文件（追加模式）
    fn open_file(&mut self) -> Result<&File> {
        if self.file.is_none() {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.log_file)
                .with_context(|| format!("Failed to open log file: {:?}", self.log_file))?;
            self.file = Some(file);
        }
        Ok(self.file.as_ref().unwrap())
    }

    /// 记录日志
    pub fn log(&mut self, entry: &LogEntry) -> Result<()> {
        let file = self.open_file()?;
        let mut file = file.try_clone()?;
        
        let line = entry.to_log_line();
        writeln!(file, "{}", line)
            .with_context(|| format!("Failed to write log entry: {:?}", self.log_file))?;
        
        // 刷新缓冲区，确保立即写入
        file.sync_all()?;
        
        Ok(())
    }

    /// 便捷方法：记录添加操作
    pub fn log_add(&mut self, session_id: &str, hash: &str, details: Option<&str>) -> Result<()> {
        self.log(&LogEntry {
            timestamp: Utc::now(),
            session_id: session_id.to_string(),
            hash: hash.to_string(),
            operation: LogOperation::Add,
            details: details.map(|s| s.to_string()),
        })
    }

    /// 便捷方法：记录检索操作
    pub fn log_retrieve(&mut self, session_id: &str, hash: &str) -> Result<()> {
        self.log(&LogEntry {
            timestamp: Utc::now(),
            session_id: session_id.to_string(),
            hash: hash.to_string(),
            operation: LogOperation::Retrieve,
            details: None,
        })
    }

    /// 便捷方法：记录删除操作
    pub fn log_delete(&mut self, session_id: &str, hash: &str) -> Result<()> {
        self.log(&LogEntry {
            timestamp: Utc::now(),
            session_id: session_id.to_string(),
            hash: hash.to_string(),
            operation: LogOperation::Delete,
            details: None,
        })
    }

    /// 便捷方法：记录裁剪操作
    pub fn log_trim(&mut self, session_id: &str, deleted_hashes: &[&str]) -> Result<()> {
        let details = deleted_hashes.join(",");
        self.log(&LogEntry {
            timestamp: Utc::now(),
            session_id: session_id.to_string(),
            hash: "-".to_string(),
            operation: LogOperation::Trim,
            details: Some(details),
        })
    }

    /// 读取所有日志
    pub fn read_all(&self) -> Result<Vec<LogEntry>> {
        if !self.log_file.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.log_file)
            .with_context(|| format!("Failed to open log file: {:?}", self.log_file))?;
        
        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if let Some(entry) = LogEntry::from_log_line(&line) {
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    /// 按会话 ID 过滤日志
    pub fn filter_by_session(&self, session_id: &str) -> Result<Vec<LogEntry>> {
        let all = self.read_all()?;
        Ok(all.into_iter()
            .filter(|e| e.session_id == session_id)
            .collect())
    }

    /// 按操作类型过滤日志
    pub fn filter_by_operation(&self, operation: &LogOperation) -> Result<Vec<LogEntry>> {
        let all = self.read_all()?;
        Ok(all.into_iter()
            .filter(|e| matches!((&e.operation, operation),
                (LogOperation::Add, LogOperation::Add)
                | (LogOperation::Retrieve, LogOperation::Retrieve)
                | (LogOperation::Delete, LogOperation::Delete)
                | (LogOperation::Trim, LogOperation::Trim)
                | (LogOperation::Update, LogOperation::Update)
            ))
            .collect())
    }

    /// 按时间范围过滤日志
    pub fn filter_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<LogEntry>> {
        let all = self.read_all()?;
        Ok(all.into_iter()
            .filter(|e| e.timestamp >= start && e.timestamp <= end)
            .collect())
    }

    /// 清空日志
    pub fn clear(&mut self) -> Result<()> {
        if self.log_file.exists() {
            std::fs::remove_file(&self.log_file)?;
        }
        self.file = None;
        Ok(())
    }

    /// 获取日志文件路径
    pub fn log_file_path(&self) -> &Path {
        &self.log_file
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_log_entry_formatting() {
        let entry = LogEntry {
            timestamp: Utc::now(),
            session_id: "sess_123".to_string(),
            hash: "abc123".to_string(),
            operation: LogOperation::Add,
            details: Some("test details".to_string()),
        };

        let line = entry.to_log_line();
        assert!(line.contains("sess_123"));
        assert!(line.contains("abc123"));
        assert!(line.contains("ADD"));
        assert!(line.contains("test details"));

        let parsed = LogEntry::from_log_line(&line).unwrap();
        assert_eq!(parsed.session_id, entry.session_id);
        assert_eq!(parsed.hash, entry.hash);
        assert!(matches!(parsed.operation, LogOperation::Add));
    }

    #[test]
    fn test_context_logger() {
        let temp_dir = TempDir::new().unwrap();
        let mut logger = ContextLogger::new(temp_dir.path()).unwrap();

        // 记录操作
        logger.log_add("sess_1", "hash1", Some("content1")).unwrap();
        logger.log_retrieve("sess_1", "hash1").unwrap();
        logger.log_add("sess_2", "hash2", Some("content2")).unwrap();
        logger.log_delete("sess_1", "hash1").unwrap();

        // 读取所有日志
        let entries = logger.read_all().unwrap();
        assert_eq!(entries.len(), 4);

        // 按会话过滤
        let sess1_entries = logger.filter_by_session("sess_1").unwrap();
        assert_eq!(sess1_entries.len(), 3);

        // 按操作类型过滤
        let add_entries = logger.filter_by_operation(&LogOperation::Add).unwrap();
        assert_eq!(add_entries.len(), 2);
    }

    #[test]
    fn test_log_trim() {
        let temp_dir = TempDir::new().unwrap();
        let mut logger = ContextLogger::new(temp_dir.path()).unwrap();

        logger.log_trim("sess_1", &["hash1", "hash2", "hash3"]).unwrap();

        let entries = logger.filter_by_operation(&LogOperation::Trim).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].details, Some("hash1,hash2,hash3".to_string()));
    }
}
