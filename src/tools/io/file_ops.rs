use tokitai::tool;
use std::fs;
use std::path::Path;

/// 文件操作工具集
pub struct FileOperations;

#[tool]
impl FileOperations {
    /// 读取文件内容
    pub fn read_file(&self, path: String) -> Result<String, String> {
        if !Path::new(&path).exists() {
            return Err(format!("文件不存在：{}", path));
        }
        fs::read_to_string(&path)
            .map_err(|e| format!("读取文件失败：{}", e))
    }

    /// 写入文件内容
    pub fn write_file(&self, path: String, content: String) -> Result<String, String> {
        // 安全检查：防止路径遍历攻击
        if path.contains("..") {
            return Err("路径包含非法字符".to_string());
        }
        
        // 创建父目录（如果不存在）
        if let Some(parent) = Path::new(&path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("创建目录失败：{}", e))?;
        }
        
        fs::write(&path, content)
            .map_err(|e| format!("写入文件失败：{}", e))?;
        Ok(format!("成功写入文件：{}", path))
    }

    /// 列出目录内容
    pub fn list_dir(&self, path: String) -> Result<String, String> {
        let entries = fs::read_dir(&path)
            .map_err(|e| format!("列出目录失败：{}", e))?;

        let mut result = Vec::new();
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let is_dir = e.path().is_dir();
            result.push(format!("{}{}", name, if is_dir { "/" } else { "" }));
        }

        Ok(result.join("\n"))
    }

    /// 删除文件
    pub fn delete_file(&self, path: String) -> Result<String, String> {
        fs::remove_file(&path)
            .map_err(|e| format!("删除文件失败：{}", e))?;
        Ok(format!("成功删除文件：{}", path))
    }

    /// 复制文件
    pub fn copy_file(&self, src: String, dst: String) -> Result<String, String> {
        fs::copy(&src, &dst)
            .map_err(|e| format!("复制文件失败：{}", e))?;
        Ok(format!("成功复制文件：{} -> {}", src, dst))
    }

    /// 编辑文件 - 在现有文件基础上进行修改
    ///
    /// 支持三种编辑模式：
    /// - `append`: 在文件末尾追加内容
    /// - `prepend`: 在文件开头插入内容
    /// - `replace`: 替换文件中包含的文本（需要精确匹配）
    pub fn edit_file(&self, path: String, mode: String, content: String, search: Option<String>) -> Result<String, String> {
        // 安全检查
        if path.contains("..") {
            return Err("路径包含非法字符".to_string());
        }

        let path_obj = Path::new(&path);

        // 检查文件是否存在
        if !path_obj.exists() {
            return Err(format!("文件不存在：{}", path));
        }

        // 读取现有内容
        let mut existing = fs::read_to_string(path_obj)
            .map_err(|e| format!("读取文件失败：{}", e))?;

        match mode.as_str() {
            "append" => {
                // 在末尾追加
                if !existing.ends_with('\n') {
                    existing.push('\n');
                }
                existing.push_str(&content);
            }
            "prepend" => {
                // 在开头插入
                existing = format!("{}\n{}", content, existing);
            }
            "replace" => {
                // 替换指定文本
                let search_text = search.ok_or("replace 模式需要提供 search 参数".to_string())?;
                if !existing.contains(&search_text) {
                    // 提供详细的错误提示和上下文
                    let (line, col) = find_closest_match(&existing, &search_text);
                    let context = get_context(&existing, line, 3);
                    return Err(format!(
                        "未找到要替换的文本\n\
                         提示：原文本必须完全匹配（包括空白字符）\n\
                         最接近的位置：第 {} 行，第 {} 列\n\
                         上下文:\n\
                         {}",
                        line + 1,
                        col + 1,
                        context
                    ));
                }
                existing = existing.replace(&search_text, &content);
            }
            _ => {
                return Err(format!("不支持的编辑模式：{} (支持：append, prepend, replace)", mode));
            }
        }

        // 写回文件
        fs::write(path_obj, &existing)
            .map_err(|e| format!("写入文件失败：{}", e))?;

        Ok(format!("成功编辑文件：{} (模式：{})", path, mode))
    }
}

/// 查找最接近的匹配位置
/// TODO: 可以实现更智能的差异定位（当前返回第一行第一列）
#[allow(dead_code)]
fn find_closest_match(_content: &str, _search: &str) -> (usize, usize) {
    // 简单实现：返回第一行的第一列
    // TODO: 可以实现更智能的差异定位
    (0, 0)
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
