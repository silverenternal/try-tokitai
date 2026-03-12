use tokitai::tool;
use serde_json::{json, Value};
use std::fs;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use regex::Regex;

/// 文件搜索工具集
/// 提供类似 grep 的文件内容搜索功能
pub struct FileSearchTools;

// 最大搜索深度限制
const MAX_SEARCH_DEPTH: usize = 50;

#[tool]
impl FileSearchTools {
    /// 在文件中搜索文本（支持正则表达式）
    /// 类似 grep 命令，支持大小写选项
    pub fn grep(
        &self,
        pattern: String,
        path: String,
        case_sensitive: Option<bool>,
        max_results: Option<usize>,
        use_regex: Option<bool>,
    ) -> Result<Value, String> {
        validate_path_length(&path)?;
        validate_pattern_length(&pattern)?;
        
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("读取文件失败：{}", e))?;

        let max_results = max_results.unwrap_or(100);
        let case_sensitive = case_sensitive.unwrap_or(true);
        let use_regex = use_regex.unwrap_or(false);

        // 编译正则表达式（如果需要）
        let regex = if use_regex {
            let re_pattern = if case_sensitive {
                pattern.clone()
            } else {
                format!("(?i){}", pattern)
            };
            Some(Regex::new(&re_pattern)
                .map_err(|e| format!("无效的正则表达式：{}", e))?)
        } else {
            None
        };

        let mut results = Vec::new();
        let mut total_matches = 0;

        for (line_num, line) in content.lines().enumerate() {
            let matched = if let Some(ref re) = regex {
                re.is_match(line)
            } else if case_sensitive {
                line.contains(&pattern)
            } else {
                line.to_lowercase().contains(&pattern.to_lowercase())
            };

            if matched {
                total_matches += 1;
                if results.len() < max_results {
                    results.push(json!({
                        "line": line_num + 1,
                        "content": line.trim()
                    }));
                }
            }
        }

        Ok(json!({
            "file": path,
            "pattern": pattern,
            "total_matches": total_matches,
            "results": results,
            "truncated": total_matches > max_results,
            "use_regex": use_regex
        }))
    }

    /// 在目录中递归搜索文件
    /// 按文件名或扩展名搜索文件
    pub fn find_files(
        &self,
        directory: String,
        pattern: Option<String>,
        extension: Option<String>,
        max_results: Option<usize>,
        max_depth: Option<usize>,
    ) -> Result<Value, String> {
        validate_path_length(&directory)?;
        
        let max_results = max_results.unwrap_or(100);
        let max_depth = max_depth.unwrap_or(MAX_SEARCH_DEPTH);
        
        let mut visited = HashSet::new();
        let mut results = Vec::new();

        search_directory_recursive(
            &directory,
            &pattern,
            &extension,
            &mut results,
            max_results,
            0,
            max_depth,
            &mut visited,
        )?;

        Ok(json!({
            "directory": directory,
            "pattern": pattern,
            "extension": extension,
            "results": results,
            "total": results.len()
        }))
    }

    /// 统计目录中各类文件的数量
    /// 按扩展名统计文件分布
    pub fn count_file_types(&self, directory: String, max_depth: Option<usize>) -> Result<Value, String> {
        validate_path_length(&directory)?;
        
        let mut stats = std::collections::HashMap::new();
        let mut total_files = 0;
        let mut total_dirs = 0;
        let mut total_size = 0u64;
        let max_depth = max_depth.unwrap_or(MAX_SEARCH_DEPTH);
        
        let mut visited = HashSet::new();
        count_files_recursive(
            &directory,
            &mut stats,
            &mut total_files,
            &mut total_dirs,
            &mut total_size,
            0,
            max_depth,
            &mut visited,
        )?;

        let mut file_types: Vec<Value> = stats.iter()
            .map(|(ext, count)| {
                json!({
                    "extension": ext,
                    "count": count
                })
            })
            .collect();

        file_types.sort_by(|a, b| {
            b.get("count").and_then(|c| c.as_u64())
                .unwrap_or(0)
                .cmp(&a.get("count").and_then(|c| c.as_u64()).unwrap_or(0))
        });

        Ok(json!({
            "directory": directory,
            "total_files": total_files,
            "total_dirs": total_dirs,
            "total_size_bytes": total_size,
            "file_types": file_types
        }))
    }

    /// 查找大文件
    /// 找出超过指定大小的文件
    pub fn find_large_files(
        &self,
        directory: String,
        min_size_mb: Option<f64>,
        max_results: Option<usize>,
        max_depth: Option<usize>,
    ) -> Result<Value, String> {
        validate_path_length(&directory)?;
        
        let min_size_bytes = ((min_size_mb.unwrap_or(10.0) * 1024.0 * 1024.0) as u64).max(1);
        let max_results = max_results.unwrap_or(50);
        let max_depth = max_depth.unwrap_or(MAX_SEARCH_DEPTH);
        let mut results = Vec::new();
        let mut visited = HashSet::new();

        find_large_files_recursive(
            &directory,
            min_size_bytes,
            &mut results,
            max_results,
            0,
            max_depth,
            &mut visited,
        )?;

        // 按大小排序
        results.sort_by(|a, b| {
            b.get("size_bytes").and_then(|s| s.as_u64())
                .unwrap_or(0)
                .cmp(&a.get("size_bytes").and_then(|s| s.as_u64()).unwrap_or(0))
        });

        Ok(json!({
            "directory": directory,
            "min_size_mb": min_size_mb.unwrap_or(10.0),
            "results": results
        }))
    }

    /// 获取文件详细信息
    /// 包括大小、修改时间、权限等
    pub fn get_file_info(&self, path: String) -> Result<Value, String> {
        validate_path_length(&path)?;
        
        let metadata = fs::metadata(&path)
            .map_err(|e| format!("获取文件信息失败：{}", e))?;

        let file_type = if metadata.is_file() {
            "file"
        } else if metadata.is_dir() {
            "directory"
        } else {
            "other"
        };

        let modified = metadata.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let created = metadata.created()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let accessed = metadata.accessed()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        Ok(json!({
            "path": path,
            "type": file_type,
            "size_bytes": metadata.len(),
            "size_human": format_size(metadata.len()),
            "modified_timestamp": modified,
            "created_timestamp": created,
            "accessed_timestamp": accessed,
            "is_readable": !metadata.permissions().readonly(),
            "is_writable": !metadata.permissions().readonly()
        }))
    }
}

/// 验证路径长度
fn validate_path_length(path: &str) -> Result<(), String> {
    const MAX_PATH_LENGTH: usize = 4096;
    
    if path.len() > MAX_PATH_LENGTH {
        return Err(format!(
            "路径过长 ({} > {} 字符)",
            path.len(),
            MAX_PATH_LENGTH
        ));
    }
    Ok(())
}

/// 验证搜索模式长度
fn validate_pattern_length(pattern: &str) -> Result<(), String> {
    const MAX_PATTERN_LENGTH: usize = 4096;
    
    if pattern.len() > MAX_PATTERN_LENGTH {
        return Err(format!(
            "搜索模式过长 ({} > {} 字符)",
            pattern.len(),
            MAX_PATTERN_LENGTH
        ));
    }
    Ok(())
}

/// 规范化路径（解析符号链接）
fn canonicalize_path(path: &Path) -> Option<PathBuf> {
    path.canonicalize().ok()
}

/// 递归搜索目录（带符号链接检测和深度限制）
#[allow(clippy::too_many_arguments)]
fn search_directory_recursive(
    dir: &str,
    pattern: &Option<String>,
    extension: &Option<String>,
    results: &mut Vec<Value>,
    max_results: usize,
    current_depth: usize,
    max_depth: usize,
    visited: &mut HashSet<PathBuf>,
) -> Result<(), String> {
    if current_depth > max_depth {
        return Ok(()); // 达到深度限制，停止递归
    }

    let path = Path::new(dir);
    
    // 规范化路径以检测符号链接循环
    if let Some(real_path) = canonicalize_path(path) {
        if !visited.insert(real_path.clone()) {
            return Ok(()); // 已访问过，跳过（防止符号链接循环）
        }
    } else {
        return Ok(()); // 无法规范化路径，跳过
    }

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            // 权限不足或其他错误，跳过该目录
            eprintln!("警告：无法读取目录 {}: {}", dir, e);
            return Ok(());
        }
    };

    for entry in entries {
        if results.len() >= max_results {
            break;
        }

        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue, // 跳过无法读取的条目
        };
        
        let entry_path = entry.path();
        let name = entry_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        let matches = match (pattern, extension) {
            (Some(p), None) => name.contains(p),
            (None, Some(ext)) => entry_path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e == ext.trim_start_matches('.'))
                .unwrap_or(false),
            (Some(p), Some(ext)) => {
                name.contains(p) && entry_path.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e == ext.trim_start_matches('.'))
                    .unwrap_or(false)
            }
            (None, None) => true,
        };

        if matches {
            results.push(json!({
                "path": entry_path.to_string_lossy().to_string(),
                "name": name,
                "is_dir": entry_path.is_dir()
            }));
        }

        if entry_path.is_dir() {
            // 忽略错误，继续搜索其他目录
            let _ = search_directory_recursive(
                &entry_path.to_string_lossy(),
                pattern,
                extension,
                results,
                max_results,
                current_depth + 1,
                max_depth,
                visited,
            );
        }
    }

    Ok(())
}

/// 递归统计文件（带符号链接检测和深度限制）
#[allow(clippy::too_many_arguments)]
fn count_files_recursive(
    dir: &str,
    stats: &mut std::collections::HashMap<String, u64>,
    total_files: &mut u64,
    total_dirs: &mut u64,
    total_size: &mut u64,
    current_depth: usize,
    max_depth: usize,
    visited: &mut HashSet<PathBuf>,
) -> Result<(), String> {
    if current_depth > max_depth {
        return Ok(());
    }

    let path = Path::new(dir);
    
    // 规范化路径以检测符号链接循环
    if let Some(real_path) = canonicalize_path(path) {
        if !visited.insert(real_path.clone()) {
            return Ok(()); // 已访问过，跳过
        }
    } else {
        return Ok(());
    }

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(()), // 跳过无法读取的目录
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        
        let entry_path = entry.path();

        if entry_path.is_dir() {
            *total_dirs += 1;
            let _ = count_files_recursive(
                &entry_path.to_string_lossy(),
                stats,
                total_files,
                total_dirs,
                total_size,
                current_depth + 1,
                max_depth,
                visited,
            );
        } else if entry_path.is_file() {
            *total_files += 1;

            if let Ok(metadata) = fs::metadata(&entry_path) {
                *total_size += metadata.len();
            }

            let ext = entry_path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("无扩展名")
                .to_string();

            *stats.entry(ext).or_insert(0) += 1;
        }
    }

    Ok(())
}

/// 递归查找大文件（带符号链接检测和深度限制）
fn find_large_files_recursive(
    dir: &str,
    min_size: u64,
    results: &mut Vec<Value>,
    max_results: usize,
    current_depth: usize,
    max_depth: usize,
    visited: &mut HashSet<PathBuf>,
) -> Result<(), String> {
    if current_depth > max_depth {
        return Ok(());
    }

    let path = Path::new(dir);
    
    // 规范化路径以检测符号链接循环
    if let Some(real_path) = canonicalize_path(path) {
        if !visited.insert(real_path.clone()) {
            return Ok(()); // 已访问过，跳过
        }
    } else {
        return Ok(());
    }

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };

    for entry in entries {
        if results.len() >= max_results {
            break;
        }

        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        
        let entry_path = entry.path();

        if entry_path.is_dir() {
            let _ = find_large_files_recursive(
                &entry_path.to_string_lossy(),
                min_size,
                results,
                max_results,
                current_depth + 1,
                max_depth,
                visited,
            );
        } else if entry_path.is_file() {
            if let Ok(metadata) = fs::metadata(&entry_path) {
                let size = metadata.len();
                if size >= min_size {
                    results.push(json!({
                        "path": entry_path.to_string_lossy().to_string(),
                        "size_bytes": size,
                        "size_human": format_size(size)
                    }));
                }
            }
        }
    }

    Ok(())
}

/// 格式化文件大小
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_path_length() {
        let long_path = "/home/".to_string() + &"a".repeat(5000);
        assert!(validate_path_length(&long_path).is_err());
        
        let short_path = "/home/user/file.txt";
        assert!(validate_path_length(short_path).is_ok());
    }

    #[test]
    fn test_validate_pattern_length() {
        let long_pattern = "a".repeat(5000);
        assert!(validate_pattern_length(&long_pattern).is_err());
        
        let short_pattern = "hello";
        assert!(validate_pattern_length(short_pattern).is_ok());
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(100), "100 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1048576), "1.00 MB");
        assert_eq!(format_size(1073741824), "1.00 GB");
    }

    #[test]
    fn test_grep_basic() {
        let tools = FileSearchTools;
        
        // 创建一个临时测试文件
        let test_file = "/tmp/test_grep.txt";
        fs::write(test_file, "hello world\nfoo bar\nhello rust\n").unwrap();
        
        let result = tools.grep(
            "hello".to_string(),
            test_file.to_string(),
            Some(true),
            Some(10),
            None,
        ).unwrap();
        
        assert_eq!(result.get("total_matches").unwrap(), 2);
        
        // 清理
        let _ = fs::remove_file(test_file);
    }

    #[test]
    fn test_grep_regex() {
        let tools = FileSearchTools;
        
        let test_file = "/tmp/test_grep_regex.txt";
        fs::write(test_file, "abc123\nfoo456\nbar789\n").unwrap();
        
        let result = tools.grep(
            r"\d+".to_string(),
            test_file.to_string(),
            None,
            Some(10),
            Some(true),
        ).unwrap();
        
        assert_eq!(result.get("total_matches").unwrap(), 3);
        assert_eq!(result.get("use_regex").unwrap(), true);
        
        // 清理
        let _ = fs::remove_file(test_file);
    }

    #[test]
    fn test_grep_case_insensitive() {
        let tools = FileSearchTools;
        
        let test_file = "/tmp/test_grep_case.txt";
        fs::write(test_file, "HELLO\nhello\nHello\n").unwrap();
        
        let result = tools.grep(
            "hello".to_string(),
            test_file.to_string(),
            Some(false),
            Some(10),
            None,
        ).unwrap();
        
        assert_eq!(result.get("total_matches").unwrap(), 3);
        
        // 清理
        let _ = fs::remove_file(test_file);
    }
}
