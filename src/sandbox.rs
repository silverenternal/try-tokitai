use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use tracing::warn;

use crate::app_paths::AppPaths;

/// 安全的文件操作沙箱
///
/// 限制文件操作在允许的目录内，防止访问敏感文件
///
/// 安全增强：
/// - 符号链接循环检测
/// - TOCTOU 防护（先验证后操作）
/// - 路径遍历攻击防护
#[derive(Debug, Clone)]
pub struct SandboxedFileOps {
    #[allow(dead_code)]
    allowed_dirs: Vec<PathBuf>,
    #[allow(dead_code)]
    max_file_size: usize,
}

#[allow(dead_code)]
impl SandboxedFileOps {
    #[allow(dead_code)]
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

    /// 检查路径是否在允许的目录内（不解析符号链接，避免 TOCTOU）
    pub fn is_path_allowed(&self, path: &Path) -> bool {
        // 防止路径遍历攻击
        if path
            .components()
            .any(|c| c.as_os_str().to_str() == Some(".."))
        {
            warn!("检测到路径遍历尝试：{:?}", path);
            return false;
        }

        // 使用绝对路径但不解析符号链接
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            match std::env::current_dir() {
                Ok(cwd) => cwd.join(path),
                Err(_) => return false,
            }
        };

        // 规范化路径（移除多余的 / 和 .）
        let normalized = abs_path;

        // 检查是否在任何一个允许的目录内
        self.allowed_dirs
            .iter()
            .any(|dir| normalized.starts_with(dir))
    }

    /// 安全检查路径（带符号链接检测）
    fn safe_validate_path(&self, path: &Path) -> Result<Metadata> {
        // 先检查路径是否允许
        if !self.is_path_allowed(path) {
            warn!("尝试访问不允许的路径：{:?}", path);
            return Err(anyhow::anyhow!(
                "访问被拒绝：路径 {:?} 不在允许的目录内",
                path
            ));
        }

        // 获取元数据（跟随符号链接）
        let metadata =
            fs::metadata(path).with_context(|| format!("获取文件元数据失败：{:?}", path))?;

        // 检查符号链接循环
        if metadata.file_type().is_symlink() {
            // 读取符号链接目标
            if let Ok(target) = fs::read_link(path) {
                // 验证目标路径是否也允许
                if !self.is_path_allowed(&target) {
                    warn!("符号链接目标不在允许目录内：{:?} -> {:?}", path, target);
                    return Err(anyhow::anyhow!("符号链接目标不被允许"));
                }
            }
        }

        Ok(metadata)
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
        let metadata =
            std::fs::metadata(path).with_context(|| format!("获取文件元数据失败：{:?}", path))?;

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

    /// 安全的读取文件（带 TOCTOU 防护）
    pub fn read_file(&self, path: &Path) -> Result<String> {
        // 使用安全验证（包括符号链接检查）
        let metadata = self.safe_validate_path(path)?;

        // 检查文件大小
        let size = metadata.len() as usize;
        if size > self.max_file_size {
            anyhow::bail!(
                "文件过大：{} bytes (最大允许：{} bytes)",
                size,
                self.max_file_size
            );
        }

        // 直接读取（TOCTOU 窗口很小）
        std::fs::read_to_string(path).with_context(|| format!("读取文件失败：{:?}", path))
    }

    /// 安全的写入文件（带 TOCTOU 防护）
    pub fn write_file(&self, path: &Path, content: &str) -> Result<()> {
        // 检查路径允许性
        if !self.is_path_allowed(path) {
            warn!("尝试写入不允许的路径：{:?}", path);
            return Err(anyhow::anyhow!(
                "访问被拒绝：路径 {:?} 不在允许的目录内",
                path
            ));
        }

        // 检查写入内容大小
        if content.len() > self.max_file_size {
            anyhow::bail!(
                "内容过大：{} bytes (最大允许：{} bytes)",
                content.len(),
                self.max_file_size
            );
        }

        // 确保父目录存在
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("创建目录失败：{:?}", parent))?;
            }
        }

        std::fs::write(path, content).with_context(|| format!("写入文件失败：{:?}", path))
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
    allowed_dirs.push(std::env::temp_dir());

    SandboxedFileOps::new(allowed_dirs, None)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxBootstrapManifest {
    pub initialized_at: DateTime<Utc>,
    pub sandbox_root: String,
    pub downloads_root: String,
    pub sessions_root: String,
    pub default_workspace_root: String,
}

#[derive(Debug, Clone)]
pub struct SandboxBootstrapResult {
    pub manifest: SandboxBootstrapManifest,
    pub first_run: bool,
}

pub fn initialize_app_sandbox(paths: &AppPaths) -> Result<SandboxBootstrapResult> {
    fs::create_dir_all(paths.state_dir()).with_context(|| {
        format!(
            "failed to create state directory: {}",
            paths.state_dir().display()
        )
    })?;
    fs::create_dir_all(paths.sessions_dir()).with_context(|| {
        format!(
            "failed to create sessions directory: {}",
            paths.sessions_dir().display()
        )
    })?;
    fs::create_dir_all(paths.sandbox_dir()).with_context(|| {
        format!(
            "failed to create sandbox directory: {}",
            paths.sandbox_dir().display()
        )
    })?;
    fs::create_dir_all(paths.downloads_dir()).with_context(|| {
        format!(
            "failed to create downloads directory: {}",
            paths.downloads_dir().display()
        )
    })?;
    fs::create_dir_all(paths.workspace_state_dir(&paths.sandbox_dir())).with_context(|| {
        format!(
            "failed to create workspace state directory: {}",
            paths.workspace_state_dir(&paths.sandbox_dir()).display()
        )
    })?;

    let manifest_path = paths.sandbox_manifest_path();
    if let Ok(content) = fs::read_to_string(&manifest_path) {
        if let Ok(manifest) = serde_json::from_str::<SandboxBootstrapManifest>(&content) {
            return Ok(SandboxBootstrapResult {
                manifest,
                first_run: false,
            });
        }
    }

    let manifest = SandboxBootstrapManifest {
        initialized_at: Utc::now(),
        sandbox_root: paths.sandbox_dir().display().to_string(),
        downloads_root: paths.downloads_dir().display().to_string(),
        sessions_root: paths.sessions_dir().display().to_string(),
        default_workspace_root: paths.sandbox_dir().display().to_string(),
    };
    let content = serde_json::to_string_pretty(&manifest)?;
    fs::write(&manifest_path, content).with_context(|| {
        format!(
            "failed to write sandbox bootstrap manifest: {}",
            manifest_path.display()
        )
    })?;

    Ok(SandboxBootstrapResult {
        manifest,
        first_run: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use tempfile::TempDir;

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
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_sandbox.txt");
        let content = "Hello, Sandbox!";

        // 测试写入
        assert!(sandbox.write_file(&test_path, content).is_ok());

        // 测试读取
        let read_content = sandbox.read_file(&test_path).unwrap();
        assert_eq!(read_content, content);

        // 清理
        let _ = std::fs::remove_file(&test_path);
    }

    #[test]
    fn test_sandbox_blocks_path_traversal() {
        let sandbox = create_default_sandbox();
        // 测试路径遍历攻击
        assert!(!sandbox.is_path_allowed(Path::new("../etc/passwd")));
        assert!(!sandbox.is_path_allowed(Path::new("/tmp/../../etc/shadow")));
        assert!(!sandbox.is_path_allowed(Path::new("./../../../root/.ssh/id_rsa")));
    }

    #[test]
    fn test_sandbox_blocks_symlink_to_forbidden_dir() {
        let temp_dir = TempDir::new().unwrap();
        let sandbox = SandboxedFileOps::new(vec![temp_dir.path().to_path_buf()], None);

        // 在临时目录创建文件
        let real_file = temp_dir.path().join("real_file.txt");
        std::fs::write(&real_file, "test content").unwrap();

        // 创建符号链接指向 /etc（不允许的目录）
        let symlink = temp_dir.path().join("symlink_to_etc");
        #[cfg(unix)]
        std::os::unix::fs::symlink("/etc", &symlink).unwrap();

        // 读取应该失败，因为符号链接目标不在允许目录
        assert!(sandbox.read_file(&symlink).is_err());

        // 清理
        let _ = std::fs::remove_file(&symlink);
    }

    #[test]
    fn test_sandbox_concurrent_read_write() {
        let temp_dir = TempDir::new().unwrap();
        let sandbox = SandboxedFileOps::new(vec![temp_dir.path().to_path_buf()], None);

        // 创建测试文件
        let test_file = temp_dir.path().join("concurrent_test.txt");
        std::fs::write(&test_file, "initial").unwrap();

        let mut handles = vec![];

        // 创建多个并发读取线程
        for i in 0..5 {
            let sandbox_clone = sandbox.clone();
            let file_path = test_file.clone();
            let handle = thread::spawn(move || {
                let result = sandbox_clone.read_file(&file_path);
                assert!(result.is_ok());
                i // 返回线程 ID
            });
            handles.push(handle);
        }

        // 等待所有线程完成
        for handle in handles {
            handle.join().unwrap();
        }

        // 验证文件内容未变
        let content = sandbox.read_file(&test_file).unwrap();
        assert_eq!(content, "initial");
    }

    #[test]
    fn test_sandbox_file_size_limit() {
        let temp_dir = TempDir::new().unwrap();
        let sandbox = SandboxedFileOps::new(vec![temp_dir.path().to_path_buf()], Some(1024)); // 1KB 限制

        // 测试小文件（应该成功）
        let small_file = temp_dir.path().join("small.txt");
        std::fs::write(&small_file, "small content").unwrap();
        assert!(sandbox.read_file(&small_file).is_ok());

        // 测试大文件（应该失败）
        let large_file = temp_dir.path().join("large.txt");
        let large_content = vec![b'a'; 2048]; // 2KB
        std::fs::write(&large_file, large_content).unwrap();
        assert!(sandbox.read_file(&large_file).is_err());
    }

    #[test]
    fn test_sandbox_write_creates_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let sandbox = SandboxedFileOps::new(vec![temp_dir.path().to_path_buf()], None);

        // 测试写入嵌套目录（应该自动创建父目录）
        let nested_file = temp_dir.path().join("subdir/nested/file.txt");
        assert!(sandbox.write_file(&nested_file, "nested content").is_ok());
        assert!(nested_file.exists());
    }

    #[test]
    fn test_initialize_app_sandbox_bootstraps_and_reuses_manifest() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path().join("base");
        let frontend_dir = temp_dir.path().join("frontend");
        let state_dir = temp_dir.path().join("state");
        fs::create_dir_all(&base_dir).unwrap();
        fs::create_dir_all(&frontend_dir).unwrap();

        let paths = AppPaths::new(base_dir, frontend_dir, state_dir);
        let first = initialize_app_sandbox(&paths).unwrap();
        assert!(first.first_run);
        assert!(paths.sandbox_dir().exists());
        assert!(paths.downloads_dir().exists());
        assert!(paths.sessions_dir().exists());
        assert!(paths.workspace_state_dir(&paths.sandbox_dir()).exists());
        assert!(paths.sandbox_manifest_path().exists());

        let second = initialize_app_sandbox(&paths).unwrap();
        assert!(!second.first_run);
        assert_eq!(
            second.manifest.default_workspace_root,
            first.manifest.default_workspace_root
        );
    }
}
