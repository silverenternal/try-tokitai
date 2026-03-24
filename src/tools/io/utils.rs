//! 路径验证宏和工具函数
//!
//! 提供统一的路径验证逻辑，消除重复代码

use crate::tools::io::error::{IoToolError, IoResult};
use crate::tools::io::security::SecurePathResolver;
use std::path::Path;

/// 验证单个路径
///
/// 用法：
/// ```rust
/// let canonical_path = validate_path!(resolver, path)?;
/// ```
#[macro_export]
macro_rules! validate_path {
    ($resolver:expr, $path:expr) => {
        $crate::tools::io::utils::validate_single_path($resolver, $path)
    };
}

/// 验证两个路径（用于复制/移动等操作）
///
/// 用法：
/// ```rust
/// let (src, dst) = validate_paths!(resolver, src_path, dst_path)?;
/// ```
#[macro_export]
macro_rules! validate_paths {
    ($resolver:expr, $src:expr, $dst:expr) => {{
        let src = $crate::tools::io::utils::validate_single_path($resolver, $src)?;
        let dst = $crate::tools::io::utils::validate_single_path($resolver, $dst)?;
        Ok((src, dst))
    }};
}

/// 内部函数：验证单个路径并返回规范化路径
pub fn validate_single_path(resolver: &SecurePathResolver, path: &str) -> IoResult<String> {
    resolver.resolve(path).into_result(path)
}

/// 检查文件是否存在
pub fn ensure_file_exists(path: &Path) -> IoResult<()> {
    if !path.exists() {
        return Err(IoToolError::FileNotFound {
            path: path.to_string_lossy().to_string(),
            suggestion: "请检查路径是否正确，或使用 find_files 搜索类似文件".to_string(),
        });
    }
    Ok(())
}

/// 检查目录是否存在
#[allow(dead_code)]
pub fn ensure_dir_exists(path: &Path) -> IoResult<()> {
    if !path.exists() {
        return Err(IoToolError::DirNotFound {
            path: path.to_string_lossy().to_string(),
            suggestion: "请检查目录路径是否正确".to_string(),
        });
    }
    Ok(())
}

/// 确保路径是文件（不是目录）
pub fn ensure_is_file(path: &Path) -> IoResult<()> {
    if path.exists() && !path.is_file() {
        return Err(IoToolError::NotAFile {
            path: path.to_string_lossy().to_string(),
            suggestion: "如果要操作目录，请使用相应的方法".to_string(),
        });
    }
    Ok(())
}

/// 确保路径是目录（不是文件）
pub fn ensure_is_dir(path: &Path) -> IoResult<()> {
    if path.exists() && !path.is_dir() {
        return Err(IoToolError::NotADirectory {
            path: path.to_string_lossy().to_string(),
            suggestion: "请提供有效的目录路径".to_string(),
        });
    }
    Ok(())
}

/// 确保文件扩展名匹配
pub fn ensure_extension(path: &Path, expected: &str) -> IoResult<()> {
    let actual = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(IoToolError::InvalidFileType {
            path: path.to_string_lossy().to_string(),
            expected_extension: expected.to_string(),
            actual_extension: Some(actual.to_string()),
            suggestion: format!("请提供.{} 扩展名的文件", expected),
        });
    }
    Ok(())
}

/// 创建父目录（如果不存在）
pub fn ensure_parent_dir_exists(path: &Path) -> IoResult<()> {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| IoToolError::DirCreationFailed {
                path: parent.to_string_lossy().to_string(),
                message: e.to_string(),
                suggestion: "请检查父目录的权限设置".to_string(),
            })?;
        }
    }
    Ok(())
}

/// 检查路径是否不存在（用于创建新文件/目录）
#[allow(dead_code)]
pub fn ensure_path_not_exists(path: &Path) -> IoResult<()> {
    if path.exists() {
        return Err(IoToolError::AlreadyExists {
            path: path.to_string_lossy().to_string(),
            suggestion: "请使用不同的名称或删除现有文件/目录".to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_ensure_file_exists() {
        let tmpdir = tempdir().unwrap();
        let file = tmpdir.path().join("test.txt");
        fs::write(&file, "content").unwrap();

        assert!(ensure_file_exists(&file).is_ok());
        assert!(ensure_file_exists(&tmpdir.path().join("nonexistent.txt")).is_err());
    }

    #[test]
    fn test_ensure_extension() {
        let tmpdir = tempdir().unwrap();
        let file = tmpdir.path().join("test.txt");
        fs::write(&file, "content").unwrap();

        assert!(ensure_extension(&file, "txt").is_ok());
        assert!(ensure_extension(&file, "pdf").is_err());
    }
}
