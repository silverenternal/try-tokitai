use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::warn;

/// 安全的文件操作沙箱
///
/// 限制文件操作在允许的目录内，防止访问敏感文件
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SandboxedFileOps {
    allowed_dirs: Vec<PathBuf>,
    max_file_size: usize,
}

#[allow(dead_code)]
impl SandboxedFileOps {
    /// 创建新的沙箱文件操作
    /// 
    /// # 参数
    /// - `allowed_dirs`: 允许的目录列表
    /// - `max_file_size`: 最大文件大小（字节），默认 10MB
    pub fn new(allowed_dirs: Vec<PathBuf>, max_file_size: Option<usize>) -> Self {
        Self {
            allowed_dirs,
            max_file_size: max_file_size.unwrap_or(10 * 1024 * 1024), // 10MB
        }
    }

    /// 检查路径是否在允许的目录内
    pub fn is_path_allowed(&self, path: &Path) -> bool {
        // 绝对路径检查
        let abs_path = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // 如果文件不存在，检查父目录
                if let Some(parent) = path.parent() {
                    match parent.canonicalize() {
                        Ok(p) => p,
                        Err(_) => return false,
                    }
                } else {
                    return false;
                }
            }
        };

        // 检查是否在任何一个允许的目录内
        self.allowed_dirs.iter().any(|dir| {
            if let Ok(canonical_dir) = dir.canonicalize() {
                abs_path.starts_with(&canonical_dir)
            } else {
                abs_path.starts_with(dir)
            }
        })
    }

    /// 验证路径，如果不允许则返回错误
    pub fn validate_path(&self, path: &Path) -> Result<()> {
        if self.is_path_allowed(path) {
            Ok(())
        } else {
            warn!("尝试访问不允许的路径：{:?}", path);
            anyhow::bail!("访问被拒绝：路径 {:?} 不在允许的目录内", path)
        }
    }

    /// 检查文件大小是否超过限制
    pub fn check_file_size(&self, path: &Path) -> Result<()> {
        let metadata = std::fs::metadata(path)
            .with_context(|| format!("获取文件元数据失败：{:?}", path))?;
        
        let size = metadata.len() as usize;
        if size > self.max_file_size {
            anyhow::bail!(
                "文件过大：{} bytes (最大允许：{} bytes)",
                size,
                self.max_file_size
            );
        }
        Ok(())
    }

    /// 安全的读取文件
    pub fn read_file(&self, path: &Path) -> Result<String> {
        self.validate_path(path)?;
        self.check_file_size(path)?;
        
        std::fs::read_to_string(path)
            .with_context(|| format!("读取文件失败：{:?}", path))
    }

    /// 安全的写入文件
    pub fn write_file(&self, path: &Path, content: &str) -> Result<()> {
        self.validate_path(path)?;
        
        // 检查写入内容大小
        if content.len() > self.max_file_size {
            anyhow::bail!(
                "内容过大：{} bytes (最大允许：{} bytes)",
                content.len(),
                self.max_file_size
            );
        }

        std::fs::write(path, content)
            .with_context(|| format!("写入文件失败：{:?}", path))
    }

    /// 获取允许的最大文件大小
    pub fn max_file_size(&self) -> usize {
        self.max_file_size
    }

    /// 获取允许的目录列表
    pub fn allowed_dirs(&self) -> &[PathBuf] {
        &self.allowed_dirs
    }
}

/// 创建默认的沙箱文件操作（允许当前目录和项目目录）
#[allow(dead_code)]
pub fn create_default_sandbox() -> SandboxedFileOps {
    let mut allowed_dirs = Vec::new();
    
    // 当前目录
    if let Ok(current) = std::env::current_dir() {
        allowed_dirs.push(current);
    }
    
    // 用户主目录
    if let Some(home) = dirs::home_dir() {
        allowed_dirs.push(home);
    }
    
    // 临时目录
    allowed_dirs.push(PathBuf::from("/tmp"));
    
    SandboxedFileOps::new(allowed_dirs, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_allows_current_dir() {
        let sandbox = create_default_sandbox();
        let current = std::env::current_dir().unwrap();
        assert!(sandbox.is_path_allowed(&current));
    }

    #[test]
    fn test_sandbox_blocks_system_dirs() {
        let sandbox = create_default_sandbox();
        // 应该阻止访问系统目录
        assert!(!sandbox.is_path_allowed(Path::new("/etc")));
        assert!(!sandbox.is_path_allowed(Path::new("/root")));
    }

    #[test]
    fn test_sandbox_read_write() {
        let sandbox = create_default_sandbox();
        let test_path = PathBuf::from("/tmp/test_sandbox.txt");
        let content = "Hello, Sandbox!";

        // 测试写入
        assert!(sandbox.write_file(&test_path, content).is_ok());

        // 测试读取
        let read_content = sandbox.read_file(&test_path).unwrap();
        assert_eq!(read_content, content);

        // 清理
        let _ = std::fs::remove_file(&test_path);
    }
}
