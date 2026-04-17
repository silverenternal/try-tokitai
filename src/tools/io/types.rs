//! 统一的工具参数结构体
//!
//! 为 AI 提供结构化的参数类型，避免使用 `Option<bool>` 等不清晰的类型
//! 同时提供更好的文档和默认值

use serde::{Deserialize, Serialize};

/// 大小写敏感选项
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaseSensitivity {
    /// 区分大小写
    #[default]
    Sensitive,
    /// 不区分大小写
    Insensitive,
    /// 自动（根据内容判断）
    Auto,
}

#[allow(dead_code)]
impl CaseSensitivity {
    pub fn is_case_sensitive(self) -> bool {
        match self {
            CaseSensitivity::Sensitive => true,
            CaseSensitivity::Insensitive => false,
            CaseSensitivity::Auto => false, // 默认不区分
        }
    }

    pub fn from_bool(case_sensitive: bool) -> Self {
        if case_sensitive {
            CaseSensitivity::Sensitive
        } else {
            CaseSensitivity::Insensitive
        }
    }
}

/// 文件编辑模式
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EditMode {
    /// 追加到文件末尾
    #[default]
    Append,
    /// 插入到文件开头
    Prepend,
    /// 替换指定文本
    Replace,
}

#[allow(dead_code)]
impl EditMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            EditMode::Append => "append",
            EditMode::Prepend => "prepend",
            EditMode::Replace => "replace",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "append" => Some(EditMode::Append),
            "prepend" => Some(EditMode::Prepend),
            "replace" => Some(EditMode::Replace),
            _ => None,
        }
    }
}

/// 文件搜索参数
#[allow(dead_code)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GrepParams {
    /// 搜索模式（文本或正则表达式）
    pub pattern: String,
    /// 文件路径
    pub path: String,
    /// 大小写敏感性（默认：区分大小写）
    #[serde(default)]
    pub case_sensitive: CaseSensitivity,
    /// 最大返回结果数（默认：100）
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    /// 是否使用正则表达式（默认：false）
    #[serde(default)]
    pub use_regex: bool,
}

#[allow(dead_code)]
fn default_max_results() -> usize {
    100
}

#[allow(dead_code)]
impl GrepParams {
    pub fn new(pattern: String, path: String) -> Self {
        Self {
            pattern,
            path,
            case_sensitive: CaseSensitivity::default(),
            max_results: default_max_results(),
            use_regex: false,
        }
    }

    pub fn with_case_sensitive(mut self, case_sensitive: bool) -> Self {
        self.case_sensitive = CaseSensitivity::from_bool(case_sensitive);
        self
    }

    pub fn with_max_results(mut self, max_results: usize) -> Self {
        self.max_results = max_results;
        self
    }

    pub fn with_regex(mut self, use_regex: bool) -> Self {
        self.use_regex = use_regex;
        self
    }
}

/// 文件查找参数
#[allow(dead_code)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FindFilesParams {
    /// 目录路径
    pub directory: String,
    /// 文件名模式（可选）
    #[serde(default)]
    pub pattern: Option<String>,
    /// 文件扩展名（可选，不包含点）
    #[serde(default)]
    pub extension: Option<String>,
    /// 最大返回结果数（默认：100）
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    /// 最大搜索深度（默认：50）
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
}

#[allow(dead_code)]
fn default_max_depth() -> usize {
    50
}

#[allow(dead_code)]
impl FindFilesParams {
    pub fn new(directory: String) -> Self {
        Self {
            directory,
            pattern: None,
            extension: None,
            max_results: default_max_results(),
            max_depth: default_max_depth(),
        }
    }

    pub fn by_extension(mut self, ext: &str) -> Self {
        self.extension = Some(ext.trim_start_matches('.').to_string());
        self
    }

    pub fn by_pattern(mut self, pattern: &str) -> Self {
        self.pattern = Some(pattern.to_string());
        self
    }
}

/// 文件编辑参数
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditFileParams {
    /// 文件路径
    pub path: String,
    /// 编辑模式
    pub mode: EditMode,
    /// 要写入的内容
    pub content: String,
    /// 搜索文本（仅在 replace 模式下需要）
    #[serde(default)]
    pub search: Option<String>,
}

#[allow(dead_code)]
impl EditFileParams {
    pub fn append(path: String, content: String) -> Self {
        Self {
            path,
            mode: EditMode::Append,
            content,
            search: None,
        }
    }

    pub fn prepend(path: String, content: String) -> Self {
        Self {
            path,
            mode: EditMode::Prepend,
            content,
            search: None,
        }
    }

    pub fn replace(path: String, search: String, content: String) -> Self {
        Self {
            path,
            mode: EditMode::Replace,
            content,
            search: Some(search),
        }
    }
}

/// 项目创建参数
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectParams {
    /// 项目名称
    pub name: String,
    /// 目标路径（可选，默认为 sandbox/项目名）
    #[serde(default)]
    pub dest: Option<String>,
    /// 项目结构配置（仅用于 custom 项目）
    #[serde(default)]
    pub structure: Option<String>,
}

#[allow(dead_code)]
impl CreateProjectParams {
    pub fn new(name: String) -> Self {
        Self {
            name,
            dest: None,
            structure: None,
        }
    }

    pub fn with_dest(mut self, dest: &str) -> Self {
        self.dest = Some(dest.to_string());
        self
    }
}

/// 缓存预热参数
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheWarmupParams {
    /// 要预热的文件路径列表
    pub paths: Vec<String>,
}

#[allow(dead_code)]
impl CacheWarmupParams {
    pub fn new(paths: Vec<String>) -> Self {
        Self { paths }
    }

    pub fn single(path: String) -> Self {
        Self { paths: vec![path] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_sensitivity() {
        assert!(CaseSensitivity::Sensitive.is_case_sensitive());
        assert!(!CaseSensitivity::Insensitive.is_case_sensitive());
        assert!(!CaseSensitivity::Auto.is_case_sensitive());
    }

    #[test]
    fn test_edit_mode_from_str() {
        assert_eq!(EditMode::from_str("append"), Some(EditMode::Append));
        assert_eq!(EditMode::from_str("APPEND"), Some(EditMode::Append));
        assert_eq!(EditMode::from_str("invalid"), None);
    }

    #[test]
    fn test_grep_params_default() {
        let params = GrepParams::new("hello".to_string(), "/path".to_string());
        assert_eq!(params.pattern, "hello");
        assert_eq!(params.path, "/path");
        assert_eq!(params.case_sensitive, CaseSensitivity::Sensitive);
        assert_eq!(params.max_results, 100);
        assert!(!params.use_regex);
    }

    #[test]
    fn test_find_files_params_builder() {
        let params = FindFilesParams::new("/src".to_string())
            .by_extension("rs")
            .by_pattern("test");

        assert_eq!(params.directory, "/src");
        assert_eq!(params.extension, Some("rs".to_string()));
        assert_eq!(params.pattern, Some("test".to_string()));
    }

    #[test]
    fn test_edit_file_params_builders() {
        let append = EditFileParams::append("/path".to_string(), "content".to_string());
        assert_eq!(append.mode, EditMode::Append);
        assert!(append.search.is_none());

        let replace =
            EditFileParams::replace("/path".to_string(), "old".to_string(), "new".to_string());
        assert_eq!(replace.mode, EditMode::Replace);
        assert_eq!(replace.search, Some("old".to_string()));
    }
}
