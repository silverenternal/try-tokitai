//! 路径解析模块 - 支持 @ 语法快速引用文件路径
//! 
//! 功能：从用户输入中提取 @path 语法，自动读取文件内容并替换到输入中
//! 
//! 示例：
//! - `@README.md 的内容是什么` -> 自动读取 README.md 内容并附加到问题中
//! - `分析 @src/main.rs 的结构` -> 自动读取 src/main.rs 内容并附加到问题中
//! - `@./file.txt @./config.toml 比较这两个文件` -> 支持多个文件

use std::path::Path;
use regex::Regex;
use anyhow::{Context, Result};

/// 解析用户输入中的 @path 语法，返回处理后的输入和读取的文件内容
/// 
/// # Arguments
/// * `input` - 用户原始输入
/// 
/// # Returns
/// * `(String, Vec<String>)` - 处理后的输入和读取到的文件内容列表
pub fn resolve_paths(input: &str) -> Result<(String, Vec<String>)> {
    // 匹配 @path 模式，path 可以包含字母、数字、下划线、点、斜杠、连字符
    let re = Regex::new(r"@([^\s,]+)")?;
    
    let mut file_contents = Vec::new();
    let mut processed_input = input.to_string();
    let mut replacements: Vec<(String, String)> = Vec::new();
    
    for cap in re.captures_iter(input) {
        let full_match = cap.get(0).unwrap().as_str();
        let path_str = cap.get(1).unwrap().as_str();
        
        // 尝试读取文件内容
        match read_file_content(path_str) {
            Ok(content) => {
                file_contents.push(content.clone());
                replacements.push((full_match.to_string(), content));
            }
            Err(e) => {
                // 如果文件读取失败，保留原始 @path 并添加错误提示
                let error_msg = format!("[文件读取失败：{} - {}]", path_str, e);
                replacements.push((full_match.to_string(), error_msg));
            }
        }
    }
    
    // 执行替换
    for (pattern, replacement) in replacements {
        processed_input = processed_input.replace(&pattern, &replacement);
    }
    
    Ok((processed_input, file_contents))
}

/// 读取文件内容
fn read_file_content(path_str: &str) -> Result<String> {
    let path = Path::new(path_str);
    
    // 安全检查：不允许路径遍历攻击
    if path.parent().is_some() {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let current_dir = std::env::current_dir()?;
        
        // 确保文件在当前目录或其子目录下
        if !canonical.starts_with(&current_dir) {
            // 允许绝对路径但必须在允许的范围内
            // 这里简单检查是否尝试访问系统敏感目录
            let path_str = canonical.to_string_lossy();
            if path_str.starts_with("/etc") || 
               path_str.starts_with("/root") || 
               path_str.starts_with("/proc") ||
               path_str.starts_with("/sys") {
                anyhow::bail!("访问受限目录");
            }
        }
    }
    
    // 检查文件大小
    if let Ok(metadata) = std::fs::metadata(path) {
        const MAX_FILE_SIZE: usize = 1024 * 1024; // 1MB 限制
        if metadata.len() > MAX_FILE_SIZE as u64 {
            anyhow::bail!("文件过大（最大 1MB）");
        }
    }
    
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("无法读取文件：{}", path_str))?;
    
    // 格式化文件内容为 AI 友好的格式
    Ok(format!("\n[文件内容：{}]\n{}\n[/文件内容]\n", path_str, content))
}

/// 检查输入中是否包含 @path 语法（@ 后面跟着非空白字符）
pub fn contains_path_reference(input: &str) -> bool {
    // 使用简单的正则表达式检查 @ 后面是否有非空白字符
    let re = Regex::new(r"@[^\s]").unwrap();
    re.is_match(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    
    #[test]
    fn test_resolve_single_path() {
        // 创建测试文件
        let test_file = "/tmp/test_path_resolver.txt";
        let test_content = "Hello, World!";
        let _ = fs::write(test_file, test_content);
        
        let input = format!("读取 @{}", test_file);
        let (processed, contents) = resolve_paths(&input).unwrap();
        
        assert!(processed.contains(test_content));
        assert_eq!(contents.len(), 1);
        
        // 清理
        let _ = fs::remove_file(test_file);
    }
    
    #[test]
    fn test_resolve_multiple_paths() {
        // 创建测试文件
        let test_file1 = "/tmp/test_file1.txt";
        let test_file2 = "/tmp/test_file2.txt";
        let _ = fs::write(test_file1, "Content 1");
        let _ = fs::write(test_file2, "Content 2");
        
        let input = format!("@{} 和 @{} 的内容", test_file1, test_file2);
        let (processed, contents) = resolve_paths(&input).unwrap();
        
        assert!(processed.contains("Content 1"));
        assert!(processed.contains("Content 2"));
        assert_eq!(contents.len(), 2);
        
        // 清理
        let _ = fs::remove_file(test_file1);
        let _ = fs::remove_file(test_file2);
    }
    
    #[test]
    fn test_resolve_nonexistent_path() {
        let input = "读取 @/nonexistent/file.txt";
        let (processed, contents) = resolve_paths(&input).unwrap();
        
        assert!(processed.contains("文件读取失败"));
        assert!(contents.is_empty());
    }
    
    #[test]
    fn test_no_path_reference() {
        let input = "你好，世界";
        let (processed, contents) = resolve_paths(&input).unwrap();
        
        assert_eq!(processed, input);
        assert!(contents.is_empty());
    }
    
    #[test]
    fn test_contains_path_reference() {
        // 应该匹配：@后面跟非空白字符
        assert!(contains_path_reference("@file.txt"));
        assert!(contains_path_reference("@./src/main.rs 的内容"));
        assert!(contains_path_reference("@/home/user/file.txt"));
        
        // 不应该匹配：没有@或@后面是空白
        assert!(!contains_path_reference("没有 at 符号"));
        assert!(!contains_path_reference("hello @ world"));
        assert!(!contains_path_reference("@ "));
        assert!(!contains_path_reference("text@"));  // @在末尾
    }
}
