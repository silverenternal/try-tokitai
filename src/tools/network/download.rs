use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tokitai::tool;
use urlencoding::encode;

/// 下载工具集 - 支持下载 PDF 论文等文件
pub struct DownloadTools;

/// 获取默认下载目录路径（跨平台支持）
fn get_default_download_dir() -> PathBuf {
    // 优先检查环境变量
    if let Ok(download_dir) = std::env::var("DOWNLOAD_DIR") {
        let path = PathBuf::from(download_dir);
        if path.exists() {
            return path;
        }
    }

    // 使用用户主目录下的 Downloads 文件夹
    if let Some(home_dir) = dirs::download_dir() {
        return home_dir;
    }

    // 回退到主目录
    if let Some(home_dir) = dirs::home_dir() {
        return home_dir.join("Downloads");
    }

    // 最后回退到当前目录
    PathBuf::from("./downloads")
}

/// 确保下载目录存在
fn ensure_download_dir(path: &Path) -> Result<PathBuf, String> {
    if !path.exists() {
        fs::create_dir_all(path)
            .map_err(|e| format!("创建下载目录失败：{}", e))?;
    }
    Ok(path.to_path_buf())
}

/// 从 URL 提取文件名
fn extract_filename_from_url(url: &str) -> Option<String> {
    // 尝试从 URL 路径部分提取文件名
    if let Some(filename) = url.split('/').next_back() {
        if !filename.is_empty() && filename.contains('.') {
            return Some(filename.to_string());
        }
    }

    // 尝试从查询参数提取
    if let Some(query) = url.split('?').nth(1) {
        for param in query.split('&') {
            if let Some(value) = param.split('=').nth(1) {
                if value.contains(".pdf") {
                    return Some(value.to_string());
                }
            }
        }
    }

    None
}

/// 生成安全的文件名（防止路径遍历攻击）
fn sanitize_filename(filename: &str) -> String {
    // 移除所有路径分隔符和危险字符
    filename
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
            c
        } else {
            '_'
        })
        .collect()
}

/// 验证下载路径是否安全（防止路径遍历）
fn validate_download_path(base_dir: &std::path::Path, full_path: &std::path::Path) -> Result<(), String> {
    // 确保最终路径在基础目录内
    let canonical_base = base_dir.canonicalize()
        .map_err(|e| format!("规范化基础目录失败：{}", e))?;
    
    // 如果文件已存在，规范化比较
    if full_path.exists() {
        let canonical_full = full_path.canonicalize()
            .map_err(|e| format!("规范化完整路径失败：{}", e))?;
        
        if !canonical_full.starts_with(&canonical_base) {
            return Err("路径遍历攻击检测：文件不在允许的目录内".to_string());
        }
    } else {
        // 文件不存在，检查父目录
        if let Some(parent) = full_path.parent() {
            if parent.exists() {
                let canonical_parent = parent.canonicalize()
                    .map_err(|e| format!("规范化父目录失败：{}", e))?;
                
                if !canonical_parent.starts_with(&canonical_base) {
                    return Err("路径遍历攻击检测：父目录不在允许的范围内".to_string());
                }
            }
        }
    }
    
    Ok(())
}

#[tool]
impl DownloadTools {
    /// 下载 PDF 文件到指定目录
    ///
    /// # 参数
    /// - `url`: PDF 文件的 URL 地址
    /// - `filename`: 可选的文件名，不提供则自动从 URL 提取
    /// - `directory`: 可选的保存目录，不提供则使用默认下载目录
    ///
    /// # 返回
    /// 返回保存的文件路径
    #[tool(default_filename = "null", default_directory = "null")]
    pub fn download_pdf(
        &self,
        url: String,
        filename: Option<String>,
        directory: Option<String>,
    ) -> Result<String, String> {
        // 确定保存目录（处理 AI 可能传入 "null" 字符串的情况）
        let download_dir = match directory.as_deref() {
            Some(dir) if dir != "null" && dir != "None" && !dir.is_empty() => PathBuf::from(dir),
            _ => get_default_download_dir(),
        };

        ensure_download_dir(&download_dir)?;

        // 确定文件名
        let final_filename = match filename.as_deref() {
            Some(name) if name != "null" && name != "None" && !name.is_empty() => sanitize_filename(name),
            _ => {
                extract_filename_from_url(&url)
                    .map(|n| sanitize_filename(&n))
                    .unwrap_or_else(|| format!("download_{}.pdf", chrono::Local::now().timestamp()))
            }
        };

        // 确保文件名以 .pdf 结尾
        let final_filename = if final_filename.to_lowercase().ends_with(".pdf") {
            final_filename
        } else {
            format!("{}.pdf", final_filename)
        };

        let file_path = download_dir.join(&final_filename);

        // 验证下载路径（防止路径遍历攻击）
        validate_download_path(&download_dir, &file_path)
            .map_err(|e| format!("安全验证失败：{}", e))?;

        // 下载文件
        let response = ureq::get(&url)
            .set("User-Agent", "Mozilla/5.0 (compatible; AI Assistant)")
            .call()
            .map_err(|e| format!("下载请求失败：{}", e))?;

        let mut bytes = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut bytes)
            .map_err(|e| format!("读取响应失败：{}", e))?;

        // 写入文件
        let mut file = File::create(&file_path)
            .map_err(|e| format!("创建文件失败：{}", e))?;

        file.write_all(&bytes)
            .map_err(|e| format!("写入文件失败：{}", e))?;

        Ok(format!(
            "✅ PDF 已下载到：{}",
            file_path.display()
        ))
    }

    /// 下载任意文件到指定目录
    ///
    /// # 参数
    /// - `url`: 文件的 URL 地址
    /// - `filename`: 可选的文件名
    /// - `directory`: 可选的保存目录
    ///
    /// # 返回
    /// 返回保存的文件路径
    #[tool(default_filename = "null", default_directory = "null")]
    pub fn download_file(
        &self,
        url: String,
        filename: Option<String>,
        directory: Option<String>,
    ) -> Result<String, String> {
        // 确定保存目录（处理 AI 可能传入 "null" 字符串的情况）
        let download_dir = match directory.as_deref() {
            Some(dir) if dir != "null" && dir != "None" && !dir.is_empty() => PathBuf::from(dir),
            _ => get_default_download_dir(),
        };

        ensure_download_dir(&download_dir)?;

        // 确定文件名
        let final_filename = match filename.as_deref() {
            Some(name) if name != "null" && name != "None" && !name.is_empty() => sanitize_filename(name),
            _ => {
                extract_filename_from_url(&url)
                    .map(|n| sanitize_filename(&n))
                    .unwrap_or_else(|| format!("download_{}", chrono::Local::now().timestamp()))
            }
        };

        let file_path = download_dir.join(&final_filename);

        // 下载文件
        let response = ureq::get(&url)
            .set("User-Agent", "Mozilla/5.0 (compatible; AI Assistant)")
            .call()
            .map_err(|e| format!("下载请求失败：{}", e))?;

        let mut bytes = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut bytes)
            .map_err(|e| format!("读取响应失败：{}", e))?;

        // 写入文件
        let mut file = File::create(&file_path)
            .map_err(|e| format!("创建文件失败：{}", e))?;

        file.write_all(&bytes)
            .map_err(|e| format!("写入文件失败：{}", e))?;

        Ok(format!(
            "✅ 文件已下载到：{}",
            file_path.display()
        ))
    }

    /// 获取当前默认下载目录
    /// 
    /// # 返回
    /// 返回默认下载目录的路径
    pub fn get_download_dir(&self) -> Result<String, String> {
        let download_dir = get_default_download_dir();
        ensure_download_dir(&download_dir)?;
        Ok(format!(
            "默认下载目录：{}",
            download_dir.display()
        ))
    }

    /// 搜索 arXiv 论文
    ///
    /// # 参数
    /// - `query`: 搜索关键词
    /// - `limit`: 返回结果数量限制（默认 5）
    ///
    /// # 返回
    /// 返回论文列表，包含标题、作者、摘要和 PDF 链接
    pub fn search_arxiv(
        &self,
        query: String,
        limit: Option<usize>,
    ) -> Result<String, String> {
        let limit = limit.unwrap_or(5);
        let encoded_query = encode(&query);
        
        // 使用 arXiv API
        let url = format!(
            "http://export.arxiv.org/api/query?search_query=all:{}&start=0&max_results={}",
            encoded_query,
            limit
        );

        let response = ureq::get(&url)
            .set("User-Agent", "Mozilla/5.0 (compatible; AI Assistant)")
            .call()
            .map_err(|e| format!("搜索请求失败：{}", e))?;

        let body = response
            .into_string()
            .map_err(|e| format!("读取响应失败：{}", e))?;

        Ok(parse_arxiv_response(&body))
    }

    /// 从 arXiv 下载论文 PDF
    ///
    /// # 参数
    /// - `arxiv_id`: arXiv 论文 ID（如 2301.00001）
    /// - `directory`: 可选的保存目录
    ///
    /// # 返回
    /// 返回保存的文件路径
    pub fn download_arxiv_paper(
        &self,
        arxiv_id: String,
        directory: Option<String>,
    ) -> Result<String, String> {
        // 清理 arxiv_id
        let arxiv_id = arxiv_id.trim();
        let pdf_url = format!("https://arxiv.org/pdf/{}.pdf", arxiv_id);
        let filename = format!("arxiv_{}.pdf", arxiv_id.replace("/", "_"));

        self.download_pdf(pdf_url, Some(filename), directory)
    }
}

/// 解析 arXiv API 响应
fn parse_arxiv_response(xml: &str) -> String {
    let mut results = Vec::new();

    // arXiv 返回的是 Atom XML 格式，使用简单的文本提取
    let mut current_entry = String::new();
    let mut in_entry = false;

    for line in xml.lines() {
        let line = line.trim();
        
        if line.contains("<entry>") {
            in_entry = true;
            current_entry.clear();
        }
        
        if in_entry {
            current_entry.push_str(line);
        }
        
        if line.contains("</entry>") {
            in_entry = false;
            
            // 提取信息
            if let Some(title) = extract_xml_tag(&current_entry, "title") {
                if let Some(id) = extract_xml_tag(&current_entry, "id") {
                    let arxiv_id = id
                        .split("/abs/")
                        .last()
                        .unwrap_or(&id)
                        .split("/pdf/")
                        .last()
                        .unwrap_or(&id);
                    
                    let summary = extract_xml_tag(&current_entry, "summary")
                        .map(|s| {
                            s.split_whitespace()
                                .take(30)
                                .collect::<Vec<_>>()
                                .join(" ")
                        })
                        .unwrap_or_else(|| "无摘要".to_string());

                    let authors: Vec<String> = extract_xml_tags(&current_entry, "name")
                        .into_iter()
                        .collect();

                    let mut entry = format!("📄 {}", trim_whitespace(&title));
                    entry.push_str(&format!("\n[ARXIV_ID: {}]", arxiv_id));
                    entry.push_str(&format!("\n[PDF_URL: https://arxiv.org/pdf/{}.pdf]", arxiv_id));
                    if !authors.is_empty() {
                        entry.push_str(&format!("\n[AUTHORS: {}]", authors.join(", ")));
                    }
                    entry.push_str(&format!("\n[SUMMARY: {}...]", trim_whitespace(&summary)));

                    results.push(entry);
                }
            }
        }
    }

    if results.is_empty() {
        "未找到论文".to_string()
    } else {
        format!("找到 {} 篇论文：\n\n{}", results.len(), results.join("\n\n"))
    }
}

/// 提取 XML 标签内容
fn extract_xml_tag(content: &str, tag: &str) -> Option<String> {
    let open_tag = format!("<{}>", tag);
    let close_tag = format!("</{}>", tag);
    
    if let Some(start) = content.find(&open_tag) {
        let start = start + open_tag.len();
        if let Some(end) = content[start..].find(&close_tag) {
            return Some(content[start..start + end].to_string());
        }
    }
    None
}

/// 提取所有匹配的 XML 标签内容
fn extract_xml_tags(content: &str, tag: &str) -> Vec<String> {
    let mut results = Vec::new();
    let open_tag = format!("<{}>", tag);
    let close_tag = format!("</{}>", tag);
    
    let mut search_start = 0;
    while let Some(start) = content[search_start..].find(&open_tag) {
        let abs_start = search_start + start + open_tag.len();
        if let Some(end) = content[abs_start..].find(&close_tag) {
            results.push(content[abs_start..abs_start + end].to_string());
            search_start = abs_start + end + close_tag.len();
        } else {
            break;
        }
    }
    results
}

/// 去除首尾空白并压缩中间空白
fn trim_whitespace(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_default_download_dir() {
        let dir = get_default_download_dir();
        assert!(dir.exists() || dir.parent().map(|p| p.exists()).unwrap_or(true));
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("test.pdf"), "test.pdf");
        assert_eq!(sanitize_filename("test/file.pdf"), "test_file.pdf");
        assert_eq!(sanitize_filename("test<>file.pdf"), "test__file.pdf");
    }

    #[test]
    fn test_extract_filename_from_url() {
        assert_eq!(
            extract_filename_from_url("https://example.com/paper.pdf"),
            Some("paper.pdf".to_string())
        );
        assert_eq!(
            extract_filename_from_url("https://arxiv.org/pdf/2301.00001.pdf"),
            Some("2301.00001.pdf".to_string())
        );
    }

    #[test]
    fn test_extract_xml_tag() {
        let xml = "<root><title>Test Title</title></root>";
        assert_eq!(
            extract_xml_tag(xml, "title"),
            Some("Test Title".to_string())
        );
    }

    #[test]
    fn test_trim_whitespace() {
        assert_eq!(trim_whitespace("  hello   world  "), "hello world");
        assert_eq!(trim_whitespace("single"), "single");
        assert_eq!(trim_whitespace(""), "");
    }
}
