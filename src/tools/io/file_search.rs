use crate::tools::io::error::IoToolError;
use crate::tools::io::security::SecurePathResolver;
use crate::tools::io::utils::{ensure_is_dir, ensure_is_file, validate_single_path};
use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tokitai::tool;

/// 文件搜索工具集
/// 提供类似 grep 的文件内容搜索功能
pub struct FileSearchTools {
    resolver: SecurePathResolver,
}

impl Default for FileSearchTools {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSearchTools {
    pub fn new() -> Self {
        Self {
            resolver: SecurePathResolver::new(),
        }
    }

    #[allow(dead_code)]
    pub fn with_resolver(resolver: SecurePathResolver) -> Self {
        Self { resolver }
    }
}

// 最大搜索深度限制
const MAX_SEARCH_DEPTH: usize = 50;
const MAX_PATTERN_LENGTH: usize = 4096;

#[tool]
impl FileSearchTools {
    /// 兼容旧名称的内容搜索入口
    pub fn search_content(
        &self,
        pattern: String,
        path: String,
        case_sensitive: Option<bool>,
        max_results: Option<usize>,
        use_regex: Option<bool>,
    ) -> Result<Value, Value> {
        self.grep(pattern, path, case_sensitive, max_results, use_regex)
    }

    /// 在文件中搜索文本（支持正则表达式）
    /// 类似 grep 命令，支持大小写选项
    pub fn grep(
        &self,
        pattern: String,
        path: String,
        case_sensitive: Option<bool>,
        max_results: Option<usize>,
        use_regex: Option<bool>,
    ) -> Result<Value, Value> {
        // 验证路径
        let canonical_path =
            validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;

        // 验证模式长度
        if pattern.len() > MAX_PATTERN_LENGTH {
            return Err(IoToolError::PatternTooLong {
                length: pattern.len(),
                max_length: MAX_PATTERN_LENGTH,
                suggestion: "请缩短搜索模式或使用更精确的正则表达式".to_string(),
            }
            .to_value());
        }

        let max_results = max_results.unwrap_or(100);
        let case_sensitive = case_sensitive.unwrap_or(true);
        let use_regex = use_regex.unwrap_or(false);
        let canonical_path_ref = Path::new(&canonical_path);

        // 编译正则表达式（如果需要）
        let regex = if use_regex {
            let re_pattern = if case_sensitive {
                pattern.clone()
            } else {
                format!("(?i){}", pattern)
            };
            Some(Regex::new(&re_pattern).map_err(|e| {
                IoToolError::InvalidRegex {
                    pattern: pattern.clone(),
                    message: e.to_string(),
                    suggestion: "请检查正则表达式语法是否正确".to_string(),
                }
                .to_value()
            })?)
        } else {
            None
        };

        if canonical_path_ref.is_dir() {
            let mut results = Vec::new();
            let mut total_matches = 0usize;
            let mut files_scanned = 0usize;
            let mut files_with_matches = 0usize;
            let mut skipped_files = Vec::new();
            let mut skipped_count = 0usize;
            let walker = DirectoryWalker::new(canonical_path_ref, MAX_SEARCH_DEPTH);

            for entry in walker {
                let file_path = entry.path();
                if !file_path.is_file() {
                    continue;
                }
                files_scanned += 1;

                let content = match fs::read_to_string(file_path) {
                    Ok(content) => content,
                    Err(_) => {
                        skipped_count += 1;
                        if skipped_files.len() < 10 {
                            skipped_files.push(file_path.to_string_lossy().to_string());
                        }
                        continue;
                    }
                };

                let matched_in_file = collect_grep_matches(
                    &content,
                    file_path.to_string_lossy().as_ref(),
                    &pattern,
                    case_sensitive,
                    regex.as_ref(),
                    max_results,
                    &mut total_matches,
                    &mut results,
                );
                if matched_in_file > 0 {
                    files_with_matches += 1;
                }
            }

            return Ok(IoToolError::success_response(
                "grep",
                json!({
                    "directory": canonical_path,
                    "pattern": pattern,
                    "total_matches": total_matches,
                    "results": results,
                    "truncated": total_matches > max_results,
                    "use_regex": use_regex,
                    "files_scanned": files_scanned,
                    "files_with_matches": files_with_matches,
                    "skipped_files": skipped_files,
                    "skipped_count": skipped_count
                }),
            ));
        }

        ensure_is_file(canonical_path_ref).map_err(|e| e.to_value())?;

        let content = fs::read_to_string(canonical_path_ref).map_err(|e| {
            IoToolError::IoError {
                message: e.to_string(),
                path: Some(canonical_path.clone()),
                operation: "read_file (for grep)".to_string(),
                suggestion: "请检查文件权限或文件是否存在".to_string(),
            }
            .to_value()
        })?;

        let mut results = Vec::new();
        let mut total_matches = 0usize;
        collect_grep_matches(
            &content,
            &canonical_path,
            &pattern,
            case_sensitive,
            regex.as_ref(),
            max_results,
            &mut total_matches,
            &mut results,
        );

        Ok(IoToolError::success_response(
            "grep",
            json!({
                "file": canonical_path,
                "pattern": pattern,
                "total_matches": total_matches,
                "results": results,
                "truncated": total_matches > max_results,
                "use_regex": use_regex
            }),
        ))
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
    ) -> Result<Value, Value> {
        // 验证路径
        let canonical_dir =
            validate_single_path(&self.resolver, &directory).map_err(|e| e.to_value())?;
        ensure_is_dir(Path::new(&canonical_dir)).map_err(|e| e.to_value())?;

        let max_results = max_results.unwrap_or(100);
        let max_depth = max_depth.unwrap_or(MAX_SEARCH_DEPTH);

        let mut results = Vec::new();
        let walker = DirectoryWalker::new(Path::new(&canonical_dir), max_depth);
        let mut truncated = false;

        for entry in walker {
            let name = entry.file_name().to_string();

            // 检查是否匹配
            let matches = match (&pattern, &extension) {
                (Some(p), None) => name.contains(p),
                (None, Some(ext)) => entry
                    .path()
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e == ext.trim_start_matches('.'))
                    .unwrap_or(false),
                (Some(p), Some(ext)) => {
                    name.contains(p)
                        && entry
                            .path()
                            .extension()
                            .and_then(|e| e.to_str())
                            .map(|e| e == ext.trim_start_matches('.'))
                            .unwrap_or(false)
                }
                (None, None) => true,
            };

            if matches {
                results.push(json!({
                    "path": entry.path().to_string_lossy().to_string(),
                    "name": name,
                    "is_dir": entry.path().is_dir(),
                    "depth": entry.depth
                }));
                if results.len() >= max_results {
                    truncated = true;
                    break;
                }
            }
        }

        Ok(IoToolError::success_response(
            "find_files",
            json!({
                "directory": canonical_dir,
                "pattern": pattern,
                "extension": extension,
                "results": results,
                "total": results.len(),
                "truncated": truncated
            }),
        ))
    }

    /// 以树状结构概览目录，适合先理解项目结构再决定读哪个文件
    pub fn tree_dir(
        &self,
        directory: String,
        max_depth: Option<usize>,
        max_entries: Option<usize>,
    ) -> Result<Value, Value> {
        let canonical_dir =
            validate_single_path(&self.resolver, &directory).map_err(|e| e.to_value())?;
        ensure_is_dir(Path::new(&canonical_dir)).map_err(|e| e.to_value())?;

        let max_depth = max_depth.unwrap_or(3).min(MAX_SEARCH_DEPTH);
        let max_entries = max_entries.unwrap_or(200).clamp(1, 1000);

        fn build_tree(path: &Path, depth: usize, max_depth: usize, remaining: &mut usize) -> Value {
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .map(|value| value.to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string());

            if *remaining == 0 {
                return json!({
                    "name": name,
                    "path": path.to_string_lossy().to_string(),
                    "kind": if path.is_dir() { "directory" } else { "file" },
                    "truncated": true,
                });
            }
            *remaining = remaining.saturating_sub(1);

            if !path.is_dir() || depth >= max_depth {
                return json!({
                    "name": name,
                    "path": path.to_string_lossy().to_string(),
                    "kind": if path.is_dir() { "directory" } else { "file" },
                });
            }

            let mut children = Vec::new();
            if let Ok(entries) = fs::read_dir(path) {
                let mut items = entries.flatten().collect::<Vec<_>>();
                items.sort_by_key(|entry| entry.file_name().to_string_lossy().to_lowercase());
                for entry in items {
                    if *remaining == 0 {
                        break;
                    }
                    children.push(build_tree(&entry.path(), depth + 1, max_depth, remaining));
                }
            }

            json!({
                "name": name,
                "path": path.to_string_lossy().to_string(),
                "kind": "directory",
                "children": children,
            })
        }

        let mut remaining = max_entries;
        let tree = build_tree(Path::new(&canonical_dir), 0, max_depth, &mut remaining);

        Ok(IoToolError::success_response(
            "tree_dir",
            json!({
                "directory": canonical_dir,
                "max_depth": max_depth,
                "max_entries": max_entries,
                "truncated": remaining == 0,
                "tree": tree,
            }),
        ))
    }

    /// 统计目录中各类文件的数量
    /// 按扩展名统计文件分布
    pub fn count_file_types(
        &self,
        directory: String,
        max_depth: Option<usize>,
    ) -> Result<Value, Value> {
        // 验证路径
        let canonical_dir =
            validate_single_path(&self.resolver, &directory).map_err(|e| e.to_value())?;
        ensure_is_dir(Path::new(&canonical_dir)).map_err(|e| e.to_value())?;

        let max_depth = max_depth.unwrap_or(MAX_SEARCH_DEPTH);

        let mut stats = std::collections::HashMap::new();
        let mut total_files = 0;
        let mut total_dirs = 0;
        let mut total_size = 0u64;

        let walker = DirectoryWalker::new(Path::new(&canonical_dir), max_depth);

        for entry in walker {
            let path = entry.path();
            if path.is_dir() {
                total_dirs += 1;
            } else if path.is_file() {
                total_files += 1;

                if let Ok(metadata) = fs::metadata(path) {
                    total_size += metadata.len();
                }

                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("无扩展名")
                    .to_string();
                *stats.entry(ext).or_insert(0u64) += 1;
            }
        }

        let mut file_types: Vec<Value> = stats
            .iter()
            .map(|(ext, count)| {
                json!({
                    "extension": ext,
                    "count": count
                })
            })
            .collect();

        file_types.sort_by(|a, b| {
            b.get("count")
                .and_then(|c| c.as_u64())
                .unwrap_or(0)
                .cmp(&a.get("count").and_then(|c| c.as_u64()).unwrap_or(0))
        });

        Ok(IoToolError::success_response(
            "count_file_types",
            json!({
                "directory": canonical_dir,
                "total_files": total_files,
                "total_dirs": total_dirs,
                "total_size_bytes": total_size,
                "file_types": file_types
            }),
        ))
    }

    /// 查找大文件
    /// 找出超过指定大小的文件
    pub fn find_large_files(
        &self,
        directory: String,
        min_size_mb: Option<f64>,
        max_results: Option<usize>,
        max_depth: Option<usize>,
    ) -> Result<Value, Value> {
        // 验证路径
        let canonical_dir =
            validate_single_path(&self.resolver, &directory).map_err(|e| e.to_value())?;
        ensure_is_dir(Path::new(&canonical_dir)).map_err(|e| e.to_value())?;

        let min_size_bytes = ((min_size_mb.unwrap_or(10.0) * 1024.0 * 1024.0) as u64).max(1);
        let max_results = max_results.unwrap_or(50);
        let max_depth = max_depth.unwrap_or(MAX_SEARCH_DEPTH);

        let mut results = Vec::new();
        let walker = DirectoryWalker::new(Path::new(&canonical_dir), max_depth);

        for entry in walker {
            let path = entry.path();
            if path.is_file() {
                if let Ok(metadata) = fs::metadata(path) {
                    let size = metadata.len();
                    if size >= min_size_bytes {
                        results.push(json!({
                            "path": path.to_string_lossy().to_string(),
                            "size_bytes": size,
                            "size_human": format_size(size),
                            "depth": entry.depth
                        }));
                    }
                }
            }
        }

        // 按大小排序
        results.sort_by(|a, b| {
            b.get("size_bytes")
                .and_then(|s| s.as_u64())
                .unwrap_or(0)
                .cmp(&a.get("size_bytes").and_then(|s| s.as_u64()).unwrap_or(0))
        });

        let total_results = results.len();
        let truncated = total_results > max_results;
        results.truncate(max_results);

        Ok(IoToolError::success_response(
            "find_large_files",
            json!({
                "directory": canonical_dir,
                "min_size_mb": min_size_mb.unwrap_or(10.0),
                "max_results": max_results,
                "returned": results.len(),
                "truncated": truncated,
                "results": results
            }),
        ))
    }

    /// 获取文件详细信息
    /// 包括大小、修改时间、权限等
    pub fn get_file_info(&self, path: String) -> Result<Value, Value> {
        // 验证路径
        let canonical_path =
            validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;

        let metadata = fs::metadata(&canonical_path).map_err(|e| {
            IoToolError::IoError {
                message: e.to_string(),
                path: Some(canonical_path.clone()),
                operation: "get_file_info".to_string(),
                suggestion: "请检查文件是否存在及权限设置".to_string(),
            }
            .to_value()
        })?;

        let file_type = if metadata.is_file() {
            "file"
        } else if metadata.is_dir() {
            "directory"
        } else {
            "other"
        };

        let timestamp_to_string = |opt: Result<std::time::SystemTime, _>| {
            opt.ok()
                .and_then(|t: std::time::SystemTime| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d: std::time::Duration| d.as_secs().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        };

        Ok(IoToolError::success_response(
            "get_file_info",
            json!({
                "path": canonical_path,
                "type": file_type,
                "size_bytes": metadata.len(),
                "size_human": format_size(metadata.len()),
                "modified_timestamp": timestamp_to_string(metadata.modified()),
                "created_timestamp": timestamp_to_string(metadata.created()),
                "accessed_timestamp": timestamp_to_string(metadata.accessed()),
                "is_readable": !metadata.permissions().readonly(),
                "is_writable": !metadata.permissions().readonly()
            }),
        ))
    }
}

/// 目录遍历条目
pub struct DirEntry {
    path: PathBuf,
    depth: usize,
}

impl DirEntry {
    fn new(path: PathBuf, depth: usize) -> Self {
        Self { path, depth }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn depth(&self) -> usize {
        self.depth
    }

    pub fn file_name(&self) -> &str {
        self.path.file_name().and_then(|n| n.to_str()).unwrap_or("")
    }
}

/// 目录迭代器 - 统一的目录遍历逻辑
pub struct DirectoryWalker {
    stack: Vec<DirEntry>,
    max_depth: usize,
    visited: HashSet<PathBuf>,
}

impl DirectoryWalker {
    pub fn new(root: &Path, max_depth: usize) -> Self {
        let mut visited = HashSet::new();

        // 尝试规范化根路径
        if let Ok(canonical) = root.canonicalize() {
            visited.insert(canonical);
        } else {
            visited.insert(root.to_path_buf());
        }

        Self {
            stack: vec![DirEntry::new(root.to_path_buf(), 0)],
            max_depth,
            visited,
        }
    }

    fn canonicalize_safe(path: &Path) -> Option<PathBuf> {
        path.canonicalize().ok()
    }
}

impl Iterator for DirectoryWalker {
    type Item = DirEntry;

    #[allow(clippy::never_loop)]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(entry) = self.stack.pop() {
            let path = entry.path();
            let depth = entry.depth();

            // 如果是目录且未达到深度限制，遍历其内容
            if path.is_dir() && depth < self.max_depth {
                if let Ok(entries) = fs::read_dir(path) {
                    for e in entries.flatten() {
                        let child_path = e.path();

                        // 检查是否已访问（符号链接循环检测）
                        if let Some(canonical) = Self::canonicalize_safe(&child_path) {
                            if self.visited.contains(&canonical) {
                                continue;
                            }
                            self.visited.insert(canonical);
                        }

                        self.stack.push(DirEntry::new(child_path, depth + 1));
                    }
                }
            }

            return Some(entry);
        }
        None
    }
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

fn line_matches_pattern(
    line: &str,
    pattern: &str,
    case_sensitive: bool,
    regex: Option<&Regex>,
) -> bool {
    if let Some(re) = regex {
        re.is_match(line)
    } else if case_sensitive {
        line.contains(pattern)
    } else {
        line.to_lowercase().contains(&pattern.to_lowercase())
    }
}

fn collect_grep_matches(
    content: &str,
    file_path: &str,
    pattern: &str,
    case_sensitive: bool,
    regex: Option<&Regex>,
    max_results: usize,
    total_matches: &mut usize,
    results: &mut Vec<Value>,
) -> usize {
    let mut matched_in_file = 0usize;
    for (line_num, line) in content.lines().enumerate() {
        if !line_matches_pattern(line, pattern, case_sensitive, regex) {
            continue;
        }
        *total_matches += 1;
        matched_in_file += 1;
        if results.len() < max_results {
            results.push(json!({
                "file": file_path,
                "line": line_num + 1,
                "content": line.trim()
            }));
        }
    }
    matched_in_file
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// 获取测试临时目录路径（在当前目录下，避免沙箱问题）
    fn get_test_temp_dir(name: &str) -> PathBuf {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let test_dir = current_dir.join("target").join("test_tmp").join(name);
        let _ = std::fs::create_dir_all(&test_dir);
        test_dir
    }

    #[test]
    fn test_grep_basic() {
        let test_dir = get_test_temp_dir("grep_test");
        let test_file = test_dir.join("test.txt");
        std::fs::write(&test_file, "hello world\nfoo bar\nhello rust\n").unwrap();

        let tools = FileSearchTools::new();
        let result = tools
            .grep(
                "hello".to_string(),
                test_file.to_string_lossy().to_string(),
                Some(true),
                Some(10),
                None,
            )
            .unwrap();

        assert_eq!(result["status"], "success");
        assert_eq!(result["data"]["total_matches"], 2);

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_grep_regex() {
        let test_dir = get_test_temp_dir("grep_regex_test");
        let test_file = test_dir.join("test.txt");
        std::fs::write(&test_file, "abc123\nfoo456\nbar789\n").unwrap();

        let tools = FileSearchTools::new();
        let result = tools
            .grep(
                r"\d+".to_string(),
                test_file.to_string_lossy().to_string(),
                None,
                Some(10),
                Some(true),
            )
            .unwrap();

        assert_eq!(result["data"]["use_regex"], true);
        assert_eq!(result["data"]["total_matches"], 3);

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_grep_directory_recursive() {
        let test_dir = get_test_temp_dir("grep_directory_test");
        let nested = test_dir.join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(test_dir.join("file1.txt"), "alpha\nbeta\n").unwrap();
        std::fs::write(nested.join("file2.txt"), "gamma alpha\n").unwrap();

        let tools = FileSearchTools::new();
        let result = tools
            .grep(
                "alpha".to_string(),
                test_dir.to_string_lossy().to_string(),
                Some(true),
                Some(10),
                None,
            )
            .unwrap();

        assert_eq!(result["status"], "success");
        assert_eq!(result["data"]["total_matches"], 2);
        assert_eq!(result["data"]["files_with_matches"], 2);
        assert_eq!(result["data"]["skipped_count"], 0);

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_find_files() {
        let test_dir = get_test_temp_dir("find_files_test");
        std::fs::write(test_dir.join("file1.rs"), "content").unwrap();
        std::fs::write(test_dir.join("file2.rs"), "content").unwrap();
        std::fs::write(test_dir.join("file.txt"), "content").unwrap();

        let tools = FileSearchTools::new();
        let result = tools
            .find_files(
                test_dir.to_string_lossy().to_string(),
                None,
                Some("rs".to_string()),
                Some(10),
                None,
            )
            .unwrap();

        assert_eq!(result["status"], "success");
        assert_eq!(result["data"]["total"], 2);

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_find_files_honors_max_results() {
        let test_dir = get_test_temp_dir("find_files_limit_test");
        std::fs::write(test_dir.join("file1.rs"), "content").unwrap();
        std::fs::write(test_dir.join("file2.rs"), "content").unwrap();
        std::fs::write(test_dir.join("file3.rs"), "content").unwrap();

        let tools = FileSearchTools::new();
        let result = tools
            .find_files(
                test_dir.to_string_lossy().to_string(),
                None,
                Some("rs".to_string()),
                Some(1),
                None,
            )
            .unwrap();

        assert_eq!(result["status"], "success");
        assert_eq!(result["data"]["total"], 1);
        assert_eq!(result["data"]["truncated"], true);

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_find_large_files_honors_max_results() {
        let test_dir = get_test_temp_dir("large_files_limit_test");
        let large_a = test_dir.join("a.bin");
        let large_b = test_dir.join("b.bin");
        let large_c = test_dir.join("c.bin");
        std::fs::write(&large_a, vec![0u8; 1024 * 1024 + 1]).unwrap();
        std::fs::write(&large_b, vec![0u8; 1024 * 1024 + 2]).unwrap();
        std::fs::write(&large_c, vec![0u8; 1024 * 1024 + 3]).unwrap();

        let tools = FileSearchTools::new();
        let result = tools
            .find_large_files(
                test_dir.to_string_lossy().to_string(),
                Some(1.0),
                Some(2),
                Some(4),
            )
            .unwrap();

        assert_eq!(result["status"], "success");
        assert_eq!(result["data"]["returned"], 2);
        assert_eq!(result["data"]["truncated"], true);
        assert_eq!(result["data"]["results"].as_array().unwrap().len(), 2);

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_get_file_info() {
        let test_dir = get_test_temp_dir("file_info_test");
        let test_file = test_dir.join("test.txt");
        std::fs::write(&test_file, "hello").unwrap();

        let tools = FileSearchTools::new();
        let result = tools
            .get_file_info(test_file.to_string_lossy().to_string())
            .unwrap();

        assert_eq!(result["status"], "success");
        assert_eq!(result["data"]["type"], "file");
        assert!(result["data"]["size_bytes"].as_u64().unwrap() > 0);

        let _ = std::fs::remove_dir_all(&test_dir);
    }
}
