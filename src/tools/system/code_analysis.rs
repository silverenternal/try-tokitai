use tokitai::tool;

/// 代码分析工具集
pub struct CodeTools;

#[tool]
impl CodeTools {
    /// 统计代码行数
    pub fn count_lines(&self, path: String) -> Result<String, String> {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("读取文件失败：{}", e))?;
        
        let total_lines = content.lines().count();
        let non_empty_lines = content.lines().filter(|l| !l.trim().is_empty()).count();
        let comment_lines = content.lines()
            .filter(|l| l.trim().starts_with("//") || l.trim().starts_with('#'))
            .count();
        
        Ok(format!(
            "总行数：{}\n非空行：{}\n注释行：{}\n代码行：{}",
            total_lines,
            non_empty_lines,
            comment_lines,
            non_empty_lines - comment_lines
        ))
    }

    /// 查找代码中的函数定义
    pub fn find_functions(&self, path: String) -> Result<String, String> {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("读取文件失败：{}", e))?;
        
        let mut functions = Vec::new();
        for (line_num, line) in content.lines().enumerate() {
            if line.contains("fn ") || line.contains("function ") || line.contains("def ") {
                functions.push(format!("第 {} 行：{}", line_num + 1, line.trim()));
            }
        }
        
        if functions.is_empty() {
            Ok("未找到函数定义".to_string())
        } else {
            Ok(functions.join("\n"))
        }
    }

    /// 检测文件类型
    pub fn detect_language(&self, path: String) -> Result<String, String> {
        let ext = std::path::Path::new(&path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        
        let language = match ext {
            "rs" => "Rust",
            "py" => "Python",
            "js" | "ts" => "JavaScript/TypeScript",
            "go" => "Go",
            "rb" => "Ruby",
            "java" => "Java",
            "c" | "cpp" | "h" | "hpp" => "C/C++",
            "sh" => "Shell",
            "toml" => "TOML",
            "json" => "JSON",
            "yaml" | "yml" => "YAML",
            "md" => "Markdown",
            _ => "未知",
        };
        
        Ok(format!("文件扩展名：.{}\n推测语言：{}", ext, language))
    }

    /// 搜索代码中的关键词
    pub fn search_code(&self, path: String, pattern: String) -> Result<String, String> {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("读取文件失败：{}", e))?;
        
        let mut matches = Vec::new();
        for (line_num, line) in content.lines().enumerate() {
            if line.to_lowercase().contains(&pattern.to_lowercase()) {
                matches.push(format!("第 {} 行：{}", line_num + 1, line.trim()));
            }
        }
        
        if matches.is_empty() {
            Ok(format!("未找到匹配 '{}' 的内容", pattern))
        } else {
            Ok(format!("找到 {} 处匹配:\n{}", matches.len(), matches.join("\n")))
        }
    }
}
