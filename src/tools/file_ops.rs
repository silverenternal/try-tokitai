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
        for entry in entries {
            if let Ok(e) = entry {
                let name = e.file_name().to_string_lossy().to_string();
                let is_dir = e.path().is_dir();
                result.push(format!("{}{}", name, if is_dir { "/" } else { "" }));
            }
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
}
