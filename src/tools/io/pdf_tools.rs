use crate::tools::io::error::IoToolError;
use crate::tools::io::security::SecurePathResolver;
use crate::tools::io::utils::{ensure_extension, ensure_file_exists, validate_single_path};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use tokitai::tool;

/// PDF 阅读工具 - 支持读取 PDF 文件内容
///
/// 使用 lopdf 库解析 PDF 文件，无需 API key
pub struct PdfTools {
    resolver: SecurePathResolver,
}

impl Default for PdfTools {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfTools {
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

#[tool]
impl PdfTools {
    /// 读取 PDF 文件并提取文本内容
    ///
    /// # 参数
    /// - `path`: PDF 文件的路径
    ///
    /// # 返回
    /// 返回提取的文本内容
    ///
    /// # 示例
    /// ```
    /// read_pdf(path="/path/to/file.pdf")
    /// ```
    pub fn read_pdf(&self, path: String) -> Result<Value, Value> {
        // 验证路径
        let canonical_path = validate_single_path(&self.resolver, &path)?;
        let path_obj = Path::new(&canonical_path);

        // 检查文件存在
        ensure_file_exists(path_obj)?;

        // 检查文件扩展名
        ensure_extension(path_obj, "pdf")?;

        // 获取文件大小
        let file_size =
            fs::metadata(path_obj)
                .map(|m| m.len())
                .map_err(|e| IoToolError::IoError {
                    message: e.to_string(),
                    path: Some(canonical_path.clone()),
                    operation: "get_metadata".to_string(),
                    suggestion: "请检查文件权限".to_string(),
                })?;

        // 使用 lopdf 加载 PDF
        let doc = lopdf::Document::load(path_obj).map_err(|e| {
            IoToolError::PdfLoadFailed {
                path: canonical_path.clone(),
                message: e.to_string(),
                file_size: Some(file_size),
                suggestion: "文件可能已损坏、加密或不是有效的 PDF 格式".to_string(),
            }
            .to_value()
        })?;

        let page_count = doc.get_pages().len();
        let mut text = String::new();
        let mut failed_pages = Vec::new();

        // 遍历所有页面对象提取文本
        for page_num in 1..=page_count as u32 {
            match doc.extract_text(&[page_num]) {
                Ok(page_text) => {
                    if !page_text.trim().is_empty() {
                        text.push_str(&format!("=== 第 {} 页 ===\n", page_num));
                        text.push_str(&page_text);
                        text.push('\n');
                    }
                }
                Err(e) => {
                    failed_pages.push(json!({
                        "page": page_num,
                        "error": e.to_string()
                    }));
                }
            }
        }

        let extracted_text = if text.trim().is_empty() {
            "PDF 文件中未提取到文本内容".to_string()
        } else {
            text
        };

        Ok(IoToolError::success_response(
            "read_pdf",
            json!({
                "path": canonical_path,
                "content": extracted_text,
                "page_count": page_count,
                "file_size_bytes": file_size,
                "failed_pages": failed_pages,
                "message": if failed_pages.is_empty() {
                    format!("成功从 {} 页 PDF 中提取文本", page_count)
                } else {
                    format!("从 {} 页 PDF 中提取文本，{} 页提取失败", page_count, failed_pages.len())
                }
            }),
        ))
    }

    /// 获取 PDF 文件的基本信息
    ///
    /// # 参数
    /// - `path`: PDF 文件的路径
    ///
    /// # 返回
    /// 返回 PDF 的元数据信息（页数等）
    ///
    /// # 示例
    /// ```
    /// get_pdf_info(path="/path/to/file.pdf")
    /// ```
    pub fn get_pdf_info(&self, path: String) -> Result<Value, Value> {
        // 验证路径
        let canonical_path = validate_single_path(&self.resolver, &path)?;
        let path_obj = Path::new(&canonical_path);

        ensure_file_exists(path_obj)?;

        let doc = lopdf::Document::load(path_obj).map_err(|e| {
            IoToolError::PdfLoadFailed {
                path: canonical_path.clone(),
                message: e.to_string(),
                file_size: None,
                suggestion: "文件可能已损坏或加密".to_string(),
            }
            .to_value()
        })?;

        let page_count = doc.get_pages().len();
        let file_size = fs::metadata(path_obj).map(|m| m.len()).unwrap_or(0);

        Ok(IoToolError::success_response(
            "get_pdf_info",
            json!({
                "path": canonical_path,
                "page_count": page_count,
                "file_size_bytes": file_size,
                "file_size_human": format_size(file_size),
                "is_encrypted": doc.is_encrypted(),
                "pdf_version": format!("{:?}", doc.version)
            }),
        ))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_tools_not_found() {
        let tools = PdfTools::with_resolver(SecurePathResolver::new_for_tests());
        // 使用当前目录下的不存在路径
        let current_dir = std::env::current_dir().unwrap();
        let nonexistent_path = current_dir
            .join("target")
            .join("test_tmp")
            .join("nonexistent.pdf");
        let result = tools.read_pdf(nonexistent_path.to_string_lossy().to_string());

        assert!(result.is_err());
        // 可能是 file_not_found 或 path_validation 错误
        let err = result.unwrap_err();
        assert!(err.get("error").is_some());
    }

    #[test]
    fn test_pdf_tools_invalid_type() {
        let tools = PdfTools::new();
        let result = tools.read_pdf("file.txt".to_string());

        // 文件不存在会先返回 file_not_found
        assert!(result.is_err());
    }

    #[test]
    fn test_pdf_info_not_found() {
        let tools = PdfTools::with_resolver(SecurePathResolver::new_for_tests());
        // 使用当前目录下的不存在路径
        let current_dir = std::env::current_dir().unwrap();
        let nonexistent_path = current_dir
            .join("target")
            .join("test_tmp")
            .join("nonexistent.pdf");
        let result = tools.get_pdf_info(nonexistent_path.to_string_lossy().to_string());

        assert!(result.is_err());
        // 可能是 file_not_found 或 path_validation 错误
        let err = result.unwrap_err();
        assert!(err.get("error").is_some());
    }
}
