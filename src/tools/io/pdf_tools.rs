use std::path::Path;
use tokitai::tool;

/// PDF 阅读工具 - 支持读取 PDF 文件内容
///
/// 使用 lopdf 库解析 PDF 文件，无需 API key
pub struct PdfTools;

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
    pub fn read_pdf(&self, path: String) -> Result<String, String> {
        let path_obj = Path::new(&path);

        // 检查文件是否存在
        if !path_obj.exists() {
            return Err(format!("文件不存在：{}", path));
        }

        // 检查文件扩展名
        if !path_obj.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("pdf")) {
            return Err("不是 PDF 文件".to_string());
        }

        // 使用 lopdf 加载 PDF
        let doc = lopdf::Document::load(path_obj)
            .map_err(|e| format!("加载 PDF 失败：{}", e))?;

        let mut text = String::new();

        // 遍历所有页面对象提取文本
        for page_num in 1..=doc.get_pages().len() as u32 {
            match doc.extract_text(&[page_num]) {
                Ok(page_text) => {
                    if !page_text.trim().is_empty() {
                        text.push_str(&format!("=== 第 {} 页 ===\n", page_num));
                        text.push_str(&page_text);
                        text.push('\n');
                    }
                }
                Err(_) => continue,
            }
        }

        if text.trim().is_empty() {
            Ok("PDF 文件中未提取到文本内容".to_string())
        } else {
            Ok(text)
        }
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
    pub fn get_pdf_info(&self, path: String) -> Result<String, String> {
        let path_obj = Path::new(&path);

        if !path_obj.exists() {
            return Err(format!("文件不存在：{}", path));
        }

        let doc = lopdf::Document::load(path_obj)
            .map_err(|e| format!("加载 PDF 失败：{}", e))?;

        let mut info = String::new();

        // 页数
        let page_count = doc.get_pages().len();
        info.push_str(&format!("页数：{}\n", page_count));

        // 文件大小
        if let Ok(metadata) = std::fs::metadata(path_obj) {
            let size = metadata.len();
            if size < 1024 {
                info.push_str(&format!("文件大小：{} bytes\n", size));
            } else if size < 1024 * 1024 {
                info.push_str(&format!("文件大小：{:.2} KB\n", size as f64 / 1024.0));
            } else if size < 1024 * 1024 * 1024 {
                info.push_str(&format!("文件大小：{:.2} MB\n", size as f64 / (1024.0 * 1024.0)));
            } else {
                info.push_str(&format!("文件大小：{:.2} GB\n", size as f64 / (1024.0 * 1024.0 * 1024.0)));
            }
        }

        Ok(info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_tools_methods() {
        let tools = PdfTools;
        // 测试方法返回预期错误（因为需要实际 PDF 文件）
        let result = tools.read_pdf("nonexistent.pdf".to_string());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("不存在") || err.contains("No such file") || err.contains("无法打开"));
    }
}
