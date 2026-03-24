//! 代码分析工具
//!
//! 提供代码文件分析功能
//!
//! ## 功能
//! - 代码行数统计
//! - 函数定义查找
//! - 语言检测
//! - 代码搜索
//!
//! ## 返回格式
//! 所有方法同时支持人类可读格式和 JSON 格式

use tokitai::tool;
use serde_json::json;
use std::path::Path;

use super::config;
use super::error::CodeAnalysisError;

/// 代码分析工具集
///
/// ## 示例
/// ```rust,ignore
/// let tools = CodeAnalyzer::default();
/// let stats = tools.count_lines("src/main.rs".to_string())?;
/// let functions = tools.find_functions("src/lib.rs".to_string())?;
/// ```
pub struct CodeAnalyzer;

impl Default for CodeAnalyzer {
    fn default() -> Self {
        Self
    }
}

/// 文件语言类型
#[derive(Debug, Clone, PartialEq)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    C,
    Cpp,
    Shell,
    Toml,
    Json,
    Yaml,
    Markdown,
    Unknown,
}

impl Language {
    pub fn from_extension(ext: &str) -> Self {
        match ext {
            "rs" => Language::Rust,
            "py" => Language::Python,
            "js" => Language::JavaScript,
            "ts" => Language::TypeScript,
            "tsx" => Language::TypeScript,
            "jsx" => Language::JavaScript,
            "go" => Language::Go,
            "java" => Language::Java,
            "c" => Language::C,
            "cpp" | "cc" | "cxx" => Language::Cpp,
            "h" | "hpp" => Language::Cpp,
            "sh" | "bash" | "zsh" => Language::Shell,
            "toml" => Language::Toml,
            "json" => Language::Json,
            "yaml" | "yml" => Language::Yaml,
            "md" | "markdown" => Language::Markdown,
            _ => Language::Unknown,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Language::Rust => "Rust",
            Language::Python => "Python",
            Language::JavaScript => "JavaScript",
            Language::TypeScript => "TypeScript",
            Language::Go => "Go",
            Language::Java => "Java",
            Language::C => "C",
            Language::Cpp => "C++",
            Language::Shell => "Shell",
            Language::Toml => "TOML",
            Language::Json => "JSON",
            Language::Yaml => "YAML",
            Language::Markdown => "Markdown",
            Language::Unknown => "Unknown",
        }
    }
}

#[tool]
impl CodeAnalyzer {
    /// 统计代码行数
    ///
    /// 统计文件的总行数、非空行、注释行、代码行
    ///
    /// ## 参数
    /// - `path`: 文件路径
    ///
    /// ## 返回
    /// JSON 格式：
    /// ```json
    /// {
    ///   "success": true,
    ///   "data": {
    ///     "path": "src/main.rs",
    ///     "total_lines": 100,
    ///     "non_empty_lines": 80,
    ///     "comment_lines": 20,
    ///     "code_lines": 60
    ///   }
    /// }
    /// ```
    ///
    /// ## 错误
    /// - `CodeAnalysisError::FileReadFailed`: 读取文件失败
    /// - `CodeAnalysisError::FileNotFound`: 文件不存在
    pub fn count_lines(&self, path: String) -> Result<String, String> {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    CodeAnalysisError::FileNotFound(path.clone())
                } else {
                    CodeAnalysisError::FileReadFailed(format!("读取文件失败：{}", e))
                }
            })
            .map_err(|e| e.to_string())?;

        let total_lines = content.lines().count();
        let non_empty_lines = content.lines()
            .filter(|l| !l.trim().is_empty())
            .count();

        // 检测语言以确定注释风格
        let lang = detect_language_from_path(&path);
        let comment_lines = count_comment_lines(&content, &lang);

        let code_lines = non_empty_lines - comment_lines;

        Ok(json!({
            "success": true,
            "data": {
                "path": path,
                "total_lines": total_lines,
                "non_empty_lines": non_empty_lines,
                "comment_lines": comment_lines,
                "code_lines": code_lines,
            }
        }).to_string())
    }

    /// 查找代码中的函数定义
    ///
    /// 使用简单模式匹配查找函数定义（不支持完整 AST 解析）
    ///
    /// ## 参数
    /// - `path`: 文件路径
    ///
    /// ## 返回
    /// JSON 格式：
    /// ```json
    /// {
    ///   "success": true,
    ///   "data": {
    ///     "path": "src/lib.rs",
    ///     "count": 5,
    ///     "functions": [
    ///       {"line": 10, "name": "main", "content": "pub fn main() {"}
    ///     ]
    ///   }
    /// }
    /// ```
    ///
    /// ## 限制
    /// - 使用正则模式匹配，可能有误报/漏报
    /// - 不支持复杂语言特性（宏、泛型等）
    pub fn find_functions(&self, path: String) -> Result<String, String> {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    CodeAnalysisError::FileNotFound(path.clone())
                } else {
                    CodeAnalysisError::FileReadFailed(format!("读取文件失败：{}", e))
                }
            })
            .map_err(|e| e.to_string())?;

        let lang = detect_language_from_path(&path);
        let mut functions = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            if let Some(func_info) = try_match_function(line, &lang) {
                functions.push(json!({
                    "line": line_num + 1,
                    "name": func_info.name,
                    "content": line.trim(),
                }));
            }
        }

        let (count, message) = if functions.is_empty() {
            (0, Some("未找到函数定义"))
        } else {
            (functions.len(), None)
        };

        Ok(json!({
            "success": true,
            "data": {
                "path": path,
                "count": count,
                "functions": functions,
            },
            "message": message
        }).to_string())
    }

    /// 检测文件类型（编程语言）
    ///
    /// 根据文件扩展名推测编程语言
    ///
    /// ## 参数
    /// - `path`: 文件路径
    ///
    /// ## 返回
    /// JSON 格式：
    /// ```json
    /// {
    ///   "success": true,
    ///   "data": {
    ///     "path": "src/main.rs",
    ///     "extension": "rs",
    ///     "language": "Rust"
    ///   }
    /// }
    /// ```
    pub fn detect_language(&self, path: String) -> Result<String, String> {
        let ext = Path::new(&path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let lang = Language::from_extension(ext);

        Ok(json!({
            "success": true,
            "data": {
                "path": path,
                "extension": ext,
                "language": lang.name(),
            }
        }).to_string())
    }

    /// 搜索代码中的关键词
    ///
    /// 不区分大小写搜索
    ///
    /// ## 参数
    /// - `path`: 文件路径
    /// - `pattern`: 搜索关键词
    /// - `limit`: 最大返回结果数，默认 50
    ///
    /// ## 返回
    /// JSON 格式：
    /// ```json
    /// {
    ///   "success": true,
    ///   "data": {
    ///     "path": "src/main.rs",
    ///     "pattern": "error",
    ///     "count": 5,
    ///     "limit_reached": false,
    ///     "matches": [
    ///       {"line": 10, "content": "if error { ... }"}
    ///     ]
    ///   }
    /// }
    /// ```
    ///
    /// ## 安全
    /// - 搜索模式长度限制为 256 字符
    /// - 不支持正则表达式（纯文本匹配）
    pub fn search_code(&self, path: String, pattern: String, limit: Option<usize>) -> Result<String, String> {
        // 验证搜索模式
        if pattern.is_empty() {
            return Err("搜索关键词不能为空".to_string());
        }
        if pattern.len() > config::MAX_PATTERN_LENGTH {
            return Err(format!("搜索模式过长 ({} > {} 字符)", pattern.len(), config::MAX_PATTERN_LENGTH));
        }

        let limit = limit.unwrap_or(config::DEFAULT_CODE_SEARCH_LIMIT).min(config::MAX_CODE_SEARCH_LIMIT);

        let content = std::fs::read_to_string(&path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    CodeAnalysisError::FileNotFound(path.clone())
                } else {
                    CodeAnalysisError::FileReadFailed(format!("读取文件失败：{}", e))
                }
            })
            .map_err(|e| e.to_string())?;

        let pattern_lower = pattern.to_lowercase();
        let mut matches = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            if line.to_lowercase().contains(&pattern_lower) {
                matches.push(json!({
                    "line": line_num + 1,
                    "content": line.trim(),
                }));

                if matches.len() >= limit {
                    break;
                }
            }
        }

        let (count, message) = if matches.is_empty() {
            (0, Some("未找到匹配的内容"))
        } else {
            (matches.len(), None)
        };

        Ok(json!({
            "success": true,
            "data": {
                "path": path,
                "pattern": pattern,
                "count": count,
                "limit_reached": matches.len() >= limit,
                "matches": matches,
            },
            "message": message
        }).to_string())
    }

    /// 获取文件基本信息
    ///
    /// 返回文件大小、行数、语言等基本信息
    pub fn get_file_info(&self, path: String) -> Result<String, String> {
        let metadata = std::fs::metadata(&path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    CodeAnalysisError::FileNotFound(path.clone())
                } else {
                    CodeAnalysisError::FileReadFailed(format!("获取文件信息失败：{}", e))
                }
            })
            .map_err(|e| e.to_string())?;

        let content = std::fs::read_to_string(&path)
            .map_err(|e| CodeAnalysisError::FileReadFailed(format!("读取文件失败：{}", e)))
            .map_err(|e| e.to_string())?;

        let lang = detect_language_from_path(&path);
        let total_lines = content.lines().count();
        let non_empty_lines = content.lines()
            .filter(|l| !l.trim().is_empty())
            .count();
        let comment_lines = count_comment_lines(&content, &lang);

        Ok(json!({
            "success": true,
            "data": {
                "path": path,
                "size_bytes": metadata.len(),
                "total_lines": total_lines,
                "non_empty_lines": non_empty_lines,
                "comment_lines": comment_lines,
                "code_lines": non_empty_lines - comment_lines,
                "language": lang.name(),
            }
        }).to_string())
    }

    /// 获取工具元数据
    pub fn get_metadata(&self) -> Result<String, String> {
        Ok(json!({
            "success": true,
            "data": {
                "name": config::CODE_ANALYZER_METADATA.name,
                "description": config::CODE_ANALYZER_METADATA.description,
                "version": config::CODE_ANALYZER_METADATA.version,
            }
        }).to_string())
    }
}

/// 检测文件路径对应的语言
fn detect_language_from_path(path: &str) -> Language {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    Language::from_extension(ext)
}

/// 统计注释行数
fn count_comment_lines(content: &str, lang: &Language) -> usize {
    let mut count = 0;
    let mut in_block_comment = false;

    for line in content.lines() {
        let trimmed = line.trim();

        match lang {
            Language::Rust | Language::C | Language::Cpp | Language::Go | Language::Java |
            Language::JavaScript | Language::TypeScript => {
                // 支持 // 和 /* */ 注释
                if in_block_comment {
                    count += 1;
                    if trimmed.contains("*/") {
                        in_block_comment = false;
                    }
                } else if trimmed.starts_with("//") {
                    count += 1;
                } else if trimmed.starts_with("/*") {
                    count += 1;
                    if !trimmed.contains("*/") {
                        in_block_comment = true;
                    }
                }
            }
            Language::Python | Language::Shell | Language::Yaml => {
                // # 注释
                if trimmed.starts_with('#') {
                    count += 1;
                }
            }
            Language::Toml => {
                if trimmed.starts_with('#') {
                    count += 1;
                }
            }
            _ => {}
        }
    }

    count
}

/// 尝试匹配函数定义
fn try_match_function(line: &str, lang: &Language) -> Option<FunctionInfo> {
    let trimmed = line.trim();

    match lang {
        Language::Rust | Language::C | Language::Cpp | Language::Go => {
            // 匹配 fn/function 关键字，忽略 pub/privat 等修饰符
            let without_modifiers = trimmed
                .strip_prefix("pub ")
                .or_else(|| trimmed.strip_prefix("private "))
                .or_else(|| trimmed.strip_prefix("protected "))
                .or_else(|| trimmed.strip_prefix("static "))
                .unwrap_or(trimmed);
            
            if without_modifiers.starts_with("fn ") || without_modifiers.starts_with("function ") {
                let name = extract_function_name(without_modifiers, &["fn ", "function ", "pub fn "])?;
                return Some(FunctionInfo { name });
            }
        }
        Language::Python => {
            if trimmed.starts_with("def ") {
                let name = extract_function_name(trimmed, &["def "])?;
                return Some(FunctionInfo { name });
            }
        }
        Language::JavaScript | Language::TypeScript => {
            // 匹配 function 关键字或箭头函数
            if trimmed.starts_with("function ") || trimmed.starts_with("async function ") {
                let name = extract_function_name(trimmed, &["function ", "async function "])?;
                return Some(FunctionInfo { name });
            }
            // 匹配 const/let foo = () =>
            if (trimmed.starts_with("const ") || trimmed.starts_with("let ") || trimmed.starts_with("var "))
                && trimmed.contains("=>") {
                    let name = extract_function_name(trimmed, &["const ", "let ", "var "])?;
                    let name = name.split('=').next()?.trim().to_string();
                    return Some(FunctionInfo { name });
                }
        }
        Language::Java => {
            // Java 没有明显的关键字，需要更复杂的匹配
            if trimmed.contains("(") && trimmed.contains(")") && trimmed.contains("{")
                && (trimmed.starts_with("public ") || trimmed.starts_with("private ") ||
                   trimmed.starts_with("protected ") || trimmed.starts_with("static ")) {
                    let name = extract_java_method_name(trimmed)?;
                    return Some(FunctionInfo { name });
                }
        }
        _ => {}
    }

    None
}

/// 函数信息
struct FunctionInfo {
    name: String,
}

/// 提取函数名
fn extract_function_name(line: &str, prefixes: &[&str]) -> Option<String> {
    let mut start = 0;
    for prefix in prefixes {
        if line.starts_with(prefix) {
            start = prefix.len();
            break;
        }
    }

    // 找到函数名（到 ( 为止）
    let rest = &line[start..];
    let name_end = rest.find('(')?;
    let name = rest[..name_end].trim().to_string();

    // 移除泛型参数（如果有）
    let name = name.split('<').next().unwrap_or(&name).to_string();

    Some(name)
}

/// 提取 Java 方法名
fn extract_java_method_name(line: &str) -> Option<String> {
    // 简化处理：找到第一个 ( 前的单词
    let paren_pos = line.find('(')?;
    let before_paren = &line[..paren_pos];

    // 找到最后一个空格后的内容（方法名）
    before_paren.split_whitespace().last().map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_extension() {
        assert_eq!(Language::from_extension("rs"), Language::Rust);
        assert_eq!(Language::from_extension("py"), Language::Python);
        assert_eq!(Language::from_extension("js"), Language::JavaScript);
        assert_eq!(Language::from_extension("unknown"), Language::Unknown);
    }

    #[test]
    fn test_count_comment_lines_rust() {
        let content = r#"
// 单行注释
fn main() {
    /* 块注释 */
    let x = 1;
}
"#;
        let count = count_comment_lines(content, &Language::Rust);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_count_comment_lines_python() {
        let content = r#"
# 注释
def main():
    x = 1
"#;
        let count = count_comment_lines(content, &Language::Python);
        assert_eq!(count, 1);
    }

    #[test]
    fn test_extract_function_name() {
        assert_eq!(
            extract_function_name("fn main() {", &["fn "]),
            Some("main".to_string())
        );
        assert_eq!(
            extract_function_name("pub fn hello() {", &["fn ", "pub fn "]),
            Some("hello".to_string())
        );
        assert_eq!(
            extract_function_name("def greet(self):", &["def "]),
            Some("greet".to_string())
        );
    }

    #[test]
    fn test_try_match_function_rust() {
        let result = try_match_function("pub fn main() {", &Language::Rust);
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "main");
    }

    #[test]
    fn test_try_match_function_python() {
        let result = try_match_function("def hello():", &Language::Python);
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "hello");
    }

    #[test]
    fn test_code_analyzer_creation() {
        let analyzer = CodeAnalyzer::default();
        assert!(true);
    }

    #[test]
    fn test_detect_language() {
        let analyzer = CodeAnalyzer::default();
        let result = analyzer.detect_language("src/main.rs".to_string());
        assert!(result.is_ok());
        let output: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output["success"], true);
        assert_eq!(output["data"]["language"], "Rust");
    }

    #[test]
    fn test_search_code_not_found() {
        let analyzer = CodeAnalyzer::default();
        let temp_path = std::env::temp_dir().join("test_search.rs");
        std::fs::write(&temp_path, "fn main() {}").unwrap();

        let result = analyzer.search_code(
            temp_path.to_string_lossy().to_string(),
            "nonexistent_xyz".to_string(),
            Some(10)
        );

        assert!(result.is_ok());
        let output: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output["success"], true);
        assert_eq!(output["data"]["count"], 0);
    }

    #[test]
    fn test_get_metadata() {
        let analyzer = CodeAnalyzer::default();
        let result = analyzer.get_metadata();

        assert!(result.is_ok());
        let output: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output["success"], true);
        assert_eq!(output["data"]["name"], "code_analyzer");
    }
}
