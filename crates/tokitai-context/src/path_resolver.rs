//! 路径解析器模块
//!
//! 扩展 @ 语法支持：
//! - @path/to/file.md  - 引用单个文件
//! - @dir/  - 引用整个目录
//! - @dir/pattern*  - 引用通配符匹配的文件
//! - @tag:标签  - 引用所有标签为指定值的文件

use std::path::Path;
use regex::Regex;

use super::knowledge_index::KnowledgeIndex;
use crate::error::{ContextResult, ContextError};

/// 解析输入中的 @ 引用
/// 返回处理后的输入和被引用的文件内容
pub fn resolve_paths(input: &str, knowledge: &KnowledgeIndex) -> ContextResult<(String, Vec<String>)> {
    let mut result = input.to_string();
    let mut contents = Vec::new();

    // 1. 处理 @tag:标签 引用
    let tag_re = Regex::new(r"@tag:([^\s,]+)")
        .map_err(|e| ContextError::OperationFailed(format!("Invalid regex pattern for tag: {}", e)))?;
    for cap in tag_re.captures_iter(input) {
        if let Some(full_match) = cap.get(0) {
            let tag = cap.get(1).unwrap().as_str();
            let files = knowledge.find_by_tag(tag);
            
            let mut replacement = String::new();
            for node in files {
                if let Some(ref content) = node.content {
                    replacement.push_str(&format!("\n\n--- 来自 {} ---\n{}\n", node.path.display(), content));
                }
            }
            
            if !replacement.is_empty() {
                contents.push(replacement);
                result = result.replace(full_match.as_str(), "[已加载相关知识]");
            }
        }
    }

    // 2. 处理 @dir/ 目录引用
    let dir_re = Regex::new(r"@([^\s,]+/)")
        .map_err(|e| ContextError::OperationFailed(format!("Invalid regex pattern for directory: {}", e)))?;
    for cap in dir_re.captures_iter(input) {
        if let Some(full_match) = cap.get(0) {
            let dir_path = cap.get(1).unwrap().as_str().trim_end_matches('/');
            let files = knowledge.find_by_directory(dir_path);
            
            let mut replacement = String::new();
            for node in files {
                if let Some(ref content) = node.content {
                    replacement.push_str(&format!("\n\n--- 来自 {} ---\n{}\n", node.path.display(), content));
                }
            }
            
            if !replacement.is_empty() {
                contents.push(replacement);
                result = result.replace(full_match.as_str(), "[已加载目录内容]");
            }
        }
    }

    // 3. 处理 @path/pattern* 通配符引用
    let wildcard_re = Regex::new(r"@([^\s,]+[*])")
        .map_err(|e| ContextError::OperationFailed(format!("Invalid regex pattern for wildcard: {}", e)))?;
    for cap in wildcard_re.captures_iter(input) {
        if let Some(full_match) = cap.get(0) {
            let pattern = cap.get(1).unwrap().as_str();
            let files = knowledge.find_by_wildcard(pattern);
            
            let mut replacement = String::new();
            for node in files {
                if let Some(ref content) = node.content {
                    replacement.push_str(&format!("\n\n--- 来自 {} ---\n{}\n", node.path.display(), content));
                }
            }
            
            if !replacement.is_empty() {
                contents.push(replacement);
                result = result.replace(full_match.as_str(), "[已加载匹配文件]");
            }
        }
    }

    // 4. 处理普通的 @path/to/file.md 引用（简化版，检查文件是否存在）
    let file_re = Regex::new(r"@([^\s,]+\.(md|txt|rs|toml|json))")
        .map_err(|e| ContextError::OperationFailed(format!("Invalid regex pattern for file: {}", e)))?;
    for cap in file_re.captures_iter(input) {
        if let Some(full_match) = cap.get(0) {
            let file_path = cap.get(1).unwrap().as_str();
            
            // 尝试在知识索引中查找
            if let Some(node) = knowledge.get(file_path) {
                if let Some(ref content) = node.content {
                    contents.push(format!("\n\n--- 来自 {} ---\n{}\n", node.path.display(), content));
                    result = result.replace(full_match.as_str(), "[已加载文件]");
                }
            } else {
                // 尝试直接读取文件
                if Path::new(file_path).exists() {
                    if let Ok(content) = std::fs::read_to_string(file_path) {
                        contents.push(format!("\n\n--- 来自 {} ---\n{}\n", file_path, content));
                        result = result.replace(full_match.as_str(), "[已加载文件]");
                    }
                }
            }
        }
    }

    Ok((result, contents))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_tag_reference() {
        let temp_dir = TempDir::new().unwrap();
        
        let db_dir = temp_dir.path().join("数据库");
        std::fs::create_dir_all(&db_dir).unwrap();
        
        let mysql_file = db_dir.join("MySQL 索引优化.md");
        std::fs::write(&mysql_file, "# MySQL 索引优化\n\n内容...").unwrap();

        let knowledge = KnowledgeIndex::from_directory(temp_dir.path()).unwrap();
        
        let (result, contents) = resolve_paths("@tag:数据库 相关的内容有哪些？", &knowledge).unwrap();
        
        assert!(result.contains("[已加载相关知识]"));
        assert!(!contents.is_empty());
    }

    #[test]
    fn test_resolve_directory_reference() {
        let temp_dir = TempDir::new().unwrap();
        
        let db_dir = temp_dir.path().join("数据库");
        std::fs::create_dir_all(&db_dir).unwrap();
        
        let mysql_file = db_dir.join("MySQL 索引优化.md");
        std::fs::write(&mysql_file, "# MySQL 索引优化\n\n内容...").unwrap();

        let knowledge = KnowledgeIndex::from_directory(temp_dir.path()).unwrap();
        
        let (result, contents) = resolve_paths("@数据库/ 里的内容有哪些？", &knowledge).unwrap();
        
        assert!(result.contains("[已加载目录内容]"));
        assert!(!contents.is_empty());
    }
}
