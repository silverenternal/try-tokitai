use crate::text_encoding::read_text_file;
use crate::tools::io::error::IoToolError;
use crate::tools::io::security::SecurePathResolver;
use crate::tools::io::utils::{
    ensure_file_exists, ensure_is_dir, ensure_is_file, ensure_parent_dir_exists,
    validate_single_path,
};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use tokitai::tool;

/// 文件操作工具集
///
/// 提供安全的文件读写操作，所有路径都经过沙箱验证
pub struct FileOperations {
    resolver: SecurePathResolver,
}

impl Default for FileOperations {
    fn default() -> Self {
        Self::new()
    }
}

impl FileOperations {
    pub fn new() -> Self {
        Self {
            resolver: SecurePathResolver::new(),
        }
    }

    #[allow(dead_code)]
    pub fn with_resolver(resolver: SecurePathResolver) -> Self {
        Self { resolver }
    }

    pub(crate) fn resolver(&self) -> &SecurePathResolver {
        &self.resolver
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PatchChange {
    pub path: String,
    pub old_text: String,
    pub new_text: String,
    #[serde(default)]
    pub replace_all: bool,
}

#[tool]
impl FileOperations {
    /// 读取文件内容
    ///
    /// # 安全特性
    /// - 路径经过沙箱验证
    /// - 防止路径遍历攻击
    /// - 检测符号链接循环
    pub fn read_file(&self, path: String) -> Result<Value, Value> {
        // 验证路径并检查文件存在
        let canonical_path =
            validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical_path);
        ensure_file_exists(path_obj).map_err(|e| e.to_value())?;

        // 读取文件
        let content = read_text_file(path_obj).map_err(|e| {
            IoToolError::IoError {
                message: e.to_string(),
                path: Some(canonical_path.clone()),
                operation: "read_file".to_string(),
                suggestion: "请检查文件权限或文件是否被其他进程占用".to_string(),
            }
            .to_value()
        })?;

        Ok(IoToolError::success_response(
            "read_file",
            json!({
                "path": canonical_path,
                "content": content,
                "size_bytes": content.len()
            }),
        ))
    }

    /// 写入文件内容
    ///
    /// # 安全特性
    /// - 路径经过沙箱验证
    /// - 自动创建父目录
    /// - 防止写入系统目录
    pub fn write_file(&self, path: String, content: String) -> Result<Value, Value> {
        // 验证路径
        let canonical_path =
            validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical_path);

        // 创建父目录
        ensure_parent_dir_exists(path_obj).map_err(|e| e.to_value())?;

        // 写入文件
        fs::write(path_obj, &content).map_err(|e| {
            IoToolError::IoError {
                message: e.to_string(),
                path: Some(canonical_path.clone()),
                operation: "write_file".to_string(),
                suggestion: "请检查目录权限或磁盘空间".to_string(),
            }
            .to_value()
        })?;

        Ok(IoToolError::success_response(
            "write_file",
            json!({
                "path": canonical_path,
                "bytes_written": content.len(),
                "message": format!("成功写入文件：{}", canonical_path)
            }),
        ))
    }

    /// 列出目录内容
    pub fn list_dir(&self, path: String) -> Result<Value, Value> {
        // 验证路径并检查是目录
        let canonical_path =
            validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical_path);
        ensure_is_dir(path_obj).map_err(|e| e.to_value())?;

        let entries = fs::read_dir(path_obj).map_err(|e| {
            IoToolError::IoError {
                message: e.to_string(),
                path: Some(canonical_path.clone()),
                operation: "list_dir".to_string(),
                suggestion: "请检查目录权限".to_string(),
            }
            .to_value()
        })?;

        let mut files = Vec::new();
        let mut dirs = Vec::new();

        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let is_dir = e.path().is_dir();
            let entry = json!({
                "name": name,
                "is_dir": is_dir,
                "path": e.path().to_string_lossy().to_string()
            });
            if is_dir {
                dirs.push(entry);
            } else {
                files.push(entry);
            }
        }

        Ok(IoToolError::success_response(
            "list_dir",
            json!({
                "path": canonical_path,
                "directories": dirs,
                "files": files,
                "total_dirs": dirs.len(),
                "total_files": files.len()
            }),
        ))
    }

    /// 删除文件
    pub fn delete_file(&self, path: String) -> Result<Value, Value> {
        // 验证路径并检查是文件
        let canonical_path =
            validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical_path);
        ensure_file_exists(path_obj).map_err(|e| e.to_value())?;
        ensure_is_file(path_obj).map_err(|e| e.to_value())?;

        fs::remove_file(path_obj).map_err(|e| {
            IoToolError::IoError {
                message: e.to_string(),
                path: Some(canonical_path.clone()),
                operation: "delete_file".to_string(),
                suggestion: "请检查文件权限或文件是否被其他进程占用".to_string(),
            }
            .to_value()
        })?;

        Ok(IoToolError::success_response(
            "delete_file",
            json!({
                "path": canonical_path,
                "message": format!("成功删除文件：{}", canonical_path)
            }),
        ))
    }

    /// 复制文件
    pub fn copy_file(&self, src: String, dst: String) -> Result<Value, Value> {
        // 验证两个路径
        let src_path = validate_single_path(&self.resolver, &src).map_err(|e| e.to_value())?;
        let dst_path = validate_single_path(&self.resolver, &dst).map_err(|e| e.to_value())?;

        let src_obj = Path::new(&src_path);
        let dst_obj = Path::new(&dst_path);

        // 检查源文件存在
        ensure_file_exists(src_obj).map_err(|e| e.to_value())?;
        ensure_is_file(src_obj).map_err(|e| e.to_value())?;

        // 创建目标父目录
        ensure_parent_dir_exists(dst_obj).map_err(|e| e.to_value())?;

        let bytes = fs::copy(src_obj, dst_obj).map_err(|e| {
            IoToolError::IoError {
                message: e.to_string(),
                path: Some(format!("{} -> {}", src_path, dst_path)),
                operation: "copy_file".to_string(),
                suggestion: "请检查源文件权限和目标目录空间".to_string(),
            }
            .to_value()
        })?;

        Ok(IoToolError::success_response(
            "copy_file",
            json!({
                "source": src_path,
                "destination": dst_path,
                "bytes_copied": bytes,
                "message": format!("成功复制文件：{} -> {}", src_path, dst_path)
            }),
        ))
    }

    /// 编辑文件 - 在现有文件基础上进行修改
    ///
    /// 支持三种编辑模式：
    /// - `append`: 在文件末尾追加内容
    /// - `prepend`: 在文件开头插入内容
    /// - `replace`: 替换文件中包含的文本（需要精确匹配）
    pub fn edit_file(
        &self,
        path: String,
        mode: String,
        content: String,
        search: Option<String>,
    ) -> Result<Value, Value> {
        // 验证路径并检查文件存在
        let canonical_path =
            validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical_path);
        ensure_file_exists(path_obj).map_err(|e| e.to_value())?;

        // 读取现有内容
        let mut existing = read_text_file(path_obj).map_err(|e| {
            IoToolError::IoError {
                message: e.to_string(),
                path: Some(canonical_path.clone()),
                operation: "read_file (for edit)".to_string(),
                suggestion: "请检查文件权限".to_string(),
            }
            .to_value()
        })?;

        let original_size = existing.len();

        match mode.as_str() {
            "append" => {
                if !existing.ends_with('\n') {
                    existing.push('\n');
                }
                existing.push_str(&content);
            }
            "prepend" => {
                existing = format!("{}\n{}", content, existing);
            }
            "replace" => {
                let search_text = search.ok_or_else(|| {
                    IoToolError::MissingParameter {
                        param_name: "search".to_string(),
                        message: "replace 模式需要提供 search 参数".to_string(),
                        suggestion: "请提供要替换的文本内容".to_string(),
                    }
                    .to_value()
                })?;

                if !existing.contains(&search_text) {
                    let (line, col) = find_closest_match(&existing, &search_text);
                    let context = get_context(&existing, line, 3);
                    return Err(IoToolError::TextNotFound {
                        search_text,
                        closest_line: Some(line + 1),
                        closest_col: Some(col + 1),
                        context: Some(context),
                        suggestion: "提示：原文本必须完全匹配（包括空白字符和换行）".to_string(),
                    }
                    .to_value());
                }
                existing = existing.replace(&search_text, &content);
            }
            _ => {
                return Err(IoToolError::InvalidEditMode {
                    mode,
                    valid_modes: vec![
                        "append".to_string(),
                        "prepend".to_string(),
                        "replace".to_string(),
                    ],
                    suggestion: "支持的模式：append, prepend, replace".to_string(),
                }
                .to_value());
            }
        }

        // 写回文件
        fs::write(path_obj, &existing).map_err(|e| {
            IoToolError::IoError {
                message: e.to_string(),
                path: Some(canonical_path.clone()),
                operation: "write_file (for edit)".to_string(),
                suggestion: "请检查文件权限".to_string(),
            }
            .to_value()
        })?;

        let new_size = existing.len();
        let bytes_changed = (new_size as i64 - original_size as i64).abs();

        Ok(IoToolError::success_response(
            "edit_file",
            json!({
                "path": canonical_path,
                "mode": mode,
                "original_size": original_size,
                "new_size": new_size,
                "bytes_changed": bytes_changed,
                "message": format!("成功编辑文件：{} (模式：{})", canonical_path, mode)
            }),
        ))
    }
}

/// 查找最接近的匹配位置
fn find_closest_match(content: &str, search: &str) -> (usize, usize) {
    let search_lower = search.to_lowercase();
    let mut best_line = 0;
    let mut best_similarity = 0.0;

    for (i, line) in content.lines().enumerate() {
        let similarity = calculate_similarity(&line.to_lowercase(), &search_lower);
        if similarity > best_similarity {
            best_similarity = similarity;
            best_line = i;
        }
    }

    (best_line, 0)
}

/// 计算两个字符串的相似度（简单实现）
fn calculate_similarity(a: &str, b: &str) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let max_len = a.len().max(b.len());
    let distance = levenshtein_distance(a, b);
    1.0 - (distance as f64 / max_len as f64)
}

/// 计算编辑距离
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut dp = vec![vec![0; n + 1]; m + 1];

    // 初始化第一列
    for (i, row) in dp.iter_mut().enumerate().take(m + 1) {
        row[0] = i;
    }
    // 初始化第一行
    for (j, cell) in dp[0].iter_mut().enumerate().take(n + 1) {
        *cell = j;
    }

    // 动态规划计算编辑距离
    for i in 1..=m {
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[m][n]
}

/// 获取指定行的上下文（前后各 radius 行）
fn get_context(content: &str, line: usize, radius: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let start = line.saturating_sub(radius);
    let end = (line + radius + 1).min(lines.len());

    let mut result = String::new();
    for (i, l) in lines.iter().enumerate().take(end).skip(start) {
        let marker = if i == line { ">>> " } else { "    " };
        result.push_str(&format!("{}{}: {}\n", marker, i + 1, l));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// 获取测试临时文件路径（在当前目录下，避免沙箱问题）
    fn get_test_temp_path(name: &str) -> PathBuf {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let test_dir = current_dir.join("target").join("test_tmp");
        let _ = std::fs::create_dir_all(&test_dir);
        test_dir.join(name)
    }

    #[test]
    fn test_read_file_success() {
        let test_file = get_test_temp_path("test_read.txt");
        std::fs::write(&test_file, "hello world").unwrap();

        let ops = FileOperations::new();
        let result = ops.read_file(test_file.to_string_lossy().to_string());

        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["status"], "success");
        assert_eq!(value["data"]["content"], "hello world");

        let _ = std::fs::remove_file(&test_file);
    }

    #[test]
    fn test_read_file_not_found() {
        let ops = FileOperations::with_resolver(SecurePathResolver::new_for_tests());
        // 使用当前目录下的不存在路径，避免沙箱问题
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let nonexistent_path = current_dir
            .join("target")
            .join("test_tmp")
            .join("nonexistent_file.txt");
        let result = ops.read_file(nonexistent_path.to_string_lossy().to_string());

        assert!(result.is_err());
        let err = result.unwrap_err();
        // 可能是 file_not_found 或 path_validation 错误
        assert!(err.get("error").is_some());
    }

    #[test]
    fn test_write_file_success() {
        let test_file = get_test_temp_path("test_write.txt");
        let ops = FileOperations::with_resolver(SecurePathResolver::new_for_tests());
        let result = ops.write_file(
            test_file.to_string_lossy().to_string(),
            "hello world".to_string(),
        );

        assert!(result.is_ok());
        assert!(test_file.exists());
        assert_eq!(std::fs::read_to_string(&test_file).unwrap(), "hello world");

        let _ = std::fs::remove_file(&test_file);
    }

    #[test]
    fn test_edit_file_append() {
        let test_file = get_test_temp_path("test_edit.txt");
        std::fs::write(&test_file, "original").unwrap();

        let ops = FileOperations::new();
        let result = ops.edit_file(
            test_file.to_string_lossy().to_string(),
            "append".to_string(),
            " appended".to_string(),
            None,
        );

        assert!(result.is_ok());
        assert_eq!(
            std::fs::read_to_string(&test_file).unwrap(),
            "original\n appended"
        );

        let _ = std::fs::remove_file(&test_file);
    }

    #[test]
    fn test_edit_file_replace_not_found() {
        let test_file = get_test_temp_path("test_replace.txt");
        std::fs::write(&test_file, "hello world").unwrap();

        let ops = FileOperations::new();
        let result = ops.edit_file(
            test_file.to_string_lossy().to_string(),
            "replace".to_string(),
            "rust".to_string(),
            Some("foo".to_string()),
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err["error"]["code"], "text_not_found");

        let _ = std::fs::remove_file(&test_file);
    }

    #[test]
    fn test_read_file_range() {
        let test_file = get_test_temp_path("test_range.txt");
        std::fs::write(&test_file, "a\nb\nc\nd").unwrap();

        let ops = FileOperations::new();
        let result = ops
            .read_file_range(test_file.to_string_lossy().to_string(), 2, 3)
            .unwrap();

        assert_eq!(result["status"], "success");
        assert_eq!(result["data"]["content"], "b\nc");

        let _ = std::fs::remove_file(&test_file);
    }

    #[test]
    fn test_mkdir_and_rename_path() {
        let base = get_test_temp_path("test_mkdir");
        let _ = std::fs::remove_dir_all(&base);
        let ops = FileOperations::new();

        let dir = base.join("nested");
        let mkdir_result = ops
            .mkdir(dir.to_string_lossy().to_string(), Some(true))
            .unwrap();
        assert_eq!(mkdir_result["status"], "success");
        assert!(dir.exists());

        let src = dir.join("src.txt");
        std::fs::write(&src, "hello").unwrap();
        let dst = dir.join("dst.txt");
        let rename_result = ops
            .rename_path(
                src.to_string_lossy().to_string(),
                dst.to_string_lossy().to_string(),
            )
            .unwrap();
        assert_eq!(rename_result["status"], "success");
        assert!(dst.exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn test_search_and_replace_multi() {
        let test_file = get_test_temp_path("test_multi_replace.txt");
        std::fs::write(&test_file, "foo\nbar\nfoo").unwrap();

        let ops = FileOperations::new();
        let result = ops
            .search_and_replace_multi(
                test_file.to_string_lossy().to_string(),
                vec![PatchChange {
                    path: test_file.to_string_lossy().to_string(),
                    old_text: "foo".to_string(),
                    new_text: "baz".to_string(),
                    replace_all: true,
                }],
            )
            .unwrap();

        assert_eq!(result["status"], "success");
        assert_eq!(
            std::fs::read_to_string(&test_file).unwrap(),
            "baz\nbar\nbaz"
        );

        let _ = std::fs::remove_file(&test_file);
    }

    #[test]
    fn test_apply_patch_basic() {
        let test_file = get_test_temp_path("test_patch.txt");
        std::fs::write(&test_file, "one\ntwo\nthree").unwrap();

        let ops = FileOperations::new();
        let patch = "\
--- a/test_patch.txt
+++ b/test_patch.txt
@@ -1,3 +1,3 @@
 one
-two
+TWO
 three"
            .to_string();
        let result = ops
            .apply_patch(test_file.to_string_lossy().to_string(), patch)
            .unwrap();

        assert_eq!(result["status"], "success");
        assert_eq!(
            std::fs::read_to_string(&test_file).unwrap(),
            "one\nTWO\nthree"
        );

        let _ = std::fs::remove_file(&test_file);
    }
}
