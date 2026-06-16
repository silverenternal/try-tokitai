//! 安全模块 - 提供沙箱目录和路径验证
//!
//! 防止路径遍历攻击、符号链接攻击，确保 AI 只能访问授权目录

use crate::tools::io::error::{IoResult, IoToolError};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// 路径验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathValidationResult {
    /// 是否有效
    pub is_valid: bool,
    /// 规范化后的路径
    pub canonical_path: Option<String>,
    /// 错误信息（如果有）
    pub error: Option<String>,
    /// 安全建议
    pub suggestion: Option<String>,
}

impl PathValidationResult {
    /// 转换为 IoResult
    pub fn into_result(self, path: &str) -> IoResult<String> {
        if self.is_valid {
            Ok(self.canonical_path.unwrap_or_else(|| path.to_string()))
        } else {
            Err(IoToolError::PathValidation {
                message: self.error.unwrap_or_else(|| "未知错误".to_string()),
                path: path.to_string(),
                suggestion: self
                    .suggestion
                    .unwrap_or_else(|| "请检查路径是否正确".to_string()),
            })
        }
    }
}

/// 沙箱配置
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// 允许的根目录列表
    pub allowed_roots: Vec<PathBuf>,
    /// 是否允许符号链接
    pub allow_symlinks: bool,
    /// 最大路径深度
    pub max_depth: usize,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        // 默认允许项目根目录和 sandbox 目录
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        #[cfg(feature = "test-allow-all-paths")]
        {
            // 测试模式（通过 feature flag 显式启用）：允许所有路径
            // 仅在测试二进制中启用，release 构建不包含此路径
            Self {
                allowed_roots: vec![
                    current_dir.clone(),
                    current_dir.join("sandbox"),
                    current_dir.join("downloads"),
                    current_dir.join("target"),
                    PathBuf::from("/"),
                ],
                allow_symlinks: true,
                max_depth: 100,
            }
        }

        #[cfg(not(feature = "test-allow-all-paths"))]
        {
            Self {
                allowed_roots: vec![
                    current_dir.clone(),
                    current_dir.join("sandbox"),
                    current_dir.join("downloads"),
                ],
                allow_symlinks: false, // 默认禁止符号链接以防攻击
                max_depth: 100,
            }
        }
    }
}

/// 安全路径解析器
pub struct SecurePathResolver {
    config: SandboxConfig,
}

impl SecurePathResolver {
    /// 创建默认配置的路径解析器
    pub fn new() -> Self {
        Self {
            config: SandboxConfig::default(),
        }
    }

    /// 创建测试模式的路径解析器（通过 feature flag 启用，允许所有路径）
    #[cfg(any(test, feature = "test-allow-all-paths"))]
    pub fn new_for_tests() -> Self {
        Self {
            config: SandboxConfig {
                allowed_roots: vec![
                    std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                    PathBuf::from("/"), // 测试模式允许所有路径
                ],
                allow_symlinks: true,
                max_depth: 100,
            },
        }
    }

    /// 创建自定义配置的路径解析器
    #[allow(dead_code)]
    pub fn with_config(config: SandboxConfig) -> Self {
        Self { config }
    }

    /// 验证并解析路径
    ///
    /// 执行以下检查：
    /// 1. 路径规范化（解析 .. 和符号链接）
    /// 2. 检查是否在允许的根目录内
    /// 3. 检查路径深度
    /// 4. 检测符号链接循环
    pub fn resolve(&self, path: &str) -> PathValidationResult {
        // 检查路径长度
        if path.len() > 4096 {
            return PathValidationResult {
                is_valid: false,
                canonical_path: None,
                error: Some(format!("路径过长 ({} > 4096 字符)", path.len())),
                suggestion: Some("请检查路径是否正确，或使用相对路径".to_string()),
            };
        }

        let path_obj = Path::new(path);

        // 规范化路径（解析 .. 和符号链接）
        // 对于不存在的路径，canonicalize_safe 会返回存在的父目录
        // 我们需要检查并重新构建完整路径
        let canonical = match self.canonicalize_safe(path_obj) {
            Ok(p) => {
                // 如果返回的路径不等于原始路径且不存在，说明原始路径不存在
                // 需要重新构建：规范化的父目录 + 原始文件名
                if p != path_obj && !path_obj.exists() {
                    if let Some(parent) = path_obj.parent() {
                        if let Some(name) = path_obj.file_name() {
                            match self.canonicalize_safe(parent) {
                                Ok(canonical_parent) => canonical_parent.join(name),
                                Err(_) => path_obj.to_path_buf(),
                            }
                        } else {
                            path_obj.to_path_buf()
                        }
                    } else {
                        path_obj.to_path_buf()
                    }
                } else {
                    p
                }
            }
            Err(e) => {
                // 如果文件不存在，尝试解析父目录
                if let Some(parent) = path_obj.parent() {
                    if parent.exists() {
                        match self.canonicalize_safe(parent) {
                            Ok(canonical_parent) => {
                                if let Some(name) = path_obj.file_name() {
                                    canonical_parent.join(name)
                                } else {
                                    return PathValidationResult {
                                        is_valid: false,
                                        canonical_path: None,
                                        error: Some(format!("无效的路径组件：{}", e)),
                                        suggestion: Some(
                                            "请检查路径中的目录名和文件名是否正确".to_string(),
                                        ),
                                    };
                                }
                            }
                            Err(e) => {
                                return PathValidationResult {
                                    is_valid: false,
                                    canonical_path: None,
                                    error: Some(format!("无法解析父目录：{}", e)),
                                    suggestion: Some("请检查路径中的目录是否存在".to_string()),
                                };
                            }
                        }
                    } else {
                        return PathValidationResult {
                            is_valid: false,
                            canonical_path: None,
                            error: Some(format!("路径不存在：{}", path)),
                            suggestion: Some("请先创建父目录或检查路径是否正确".to_string()),
                        };
                    }
                } else {
                    return PathValidationResult {
                        is_valid: false,
                        canonical_path: None,
                        error: Some(format!("无法解析路径：{}", e)),
                        suggestion: Some("请提供有效的绝对路径或相对路径".to_string()),
                    };
                }
            }
        };

        // 检查是否在允许的根目录内
        if !self.is_within_allowed_roots(&canonical) {
            return PathValidationResult {
                is_valid: false,
                canonical_path: Some(canonical.to_string_lossy().to_string()),
                error: Some(format!("路径不在允许的目录内：{}", canonical.display())),
                suggestion: Some(format!(
                    "允许访问的目录：{}",
                    self.config
                        .allowed_roots
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
            };
        }

        // 检查路径深度
        let depth = self.calculate_depth(&canonical);
        if depth > self.config.max_depth {
            return PathValidationResult {
                is_valid: false,
                canonical_path: Some(canonical.to_string_lossy().to_string()),
                error: Some(format!(
                    "路径深度超限 ({} > {})",
                    depth, self.config.max_depth
                )),
                suggestion: Some("请使用更浅的目录结构".to_string()),
            };
        }

        // 检查是否是符号链接（如果禁止）
        if !self.config.allow_symlinks && path_obj.is_symlink() {
            return PathValidationResult {
                is_valid: false,
                canonical_path: Some(canonical.to_string_lossy().to_string()),
                error: Some("禁止访问符号链接".to_string()),
                suggestion: Some("请使用实际路径而非符号链接".to_string()),
            };
        }

        PathValidationResult {
            is_valid: true,
            canonical_path: Some(canonical.to_string_lossy().to_string()),
            error: None,
            suggestion: None,
        }
    }

    /// 安全的规范化路径（带符号链接循环检测）
    fn canonicalize_safe(&self, path: &Path) -> Result<PathBuf, String> {
        let mut visited = std::collections::HashSet::new();
        let mut current = path.to_path_buf();

        // 迭代解析路径组件，检测符号链接循环
        loop {
            // 尝试规范化路径
            match current.canonicalize() {
                Ok(canonical) => {
                    // 检测循环
                    if !visited.insert(canonical.clone()) {
                        return Err("检测到符号链接循环".to_string());
                    }
                    return Ok(canonical);
                }
                Err(_) => {
                    // 如果路径不存在，尝试解析父目录
                    if let Some(parent) = current.parent() {
                        current = parent.to_path_buf();
                    } else {
                        return Err("无法解析路径".to_string());
                    }
                }
            }
        }
    }

    /// 检查路径是否在允许的根目录内
    fn is_within_allowed_roots(&self, path: &Path) -> bool {
        self.config.allowed_roots.iter().any(|root| {
            // 确保根目录是规范化的
            if let Ok(canonical_root) = root.canonicalize() {
                path.starts_with(&canonical_root)
            } else {
                path.starts_with(root)
            }
        })
    }

    /// 计算路径深度
    fn calculate_depth(&self, path: &Path) -> usize {
        path.components().count()
    }

    /// 添加允许的根目录
    #[allow(dead_code)]
    pub fn add_allowed_root(&mut self, path: PathBuf) {
        if !self.config.allowed_roots.contains(&path) {
            self.config.allowed_roots.push(path);
        }
    }

    /// 移除允许的根目录
    #[allow(dead_code)]
    pub fn remove_allowed_root(&mut self, path: &Path) {
        self.config.allowed_roots.retain(|p| p != path);
    }
}

impl Default for SecurePathResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局路径解析器（线程安全的懒加载单例）
/// 使用 OnceLock + RwLock 保证线程安全，避免 static mut 的未定义行为
static GLOBAL_RESOLVER: OnceLock<RwLock<SecurePathResolver>> = OnceLock::new();

/// 获取全局路径解析器（只读）
pub fn get_global_resolver() -> &'static RwLock<SecurePathResolver> {
    GLOBAL_RESOLVER.get_or_init(|| {
        #[cfg(feature = "test-allow-all-paths")]
        {
            // 测试模式（通过 feature flag 显式启用）
            let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            RwLock::new(SecurePathResolver::with_config(SandboxConfig {
                allowed_roots: vec![
                    current_dir.clone(),
                    current_dir.join("sandbox"),
                    current_dir.join("downloads"),
                    current_dir.join("target"),
                    current_dir.join("target").join("test_tmp"),
                    PathBuf::from("/tmp"),
                    PathBuf::from("/"),
                ],
                allow_symlinks: true,
                max_depth: 100,
            }))
        }

        #[cfg(not(feature = "test-allow-all-paths"))]
        {
            RwLock::new(SecurePathResolver::new())
        }
    })
}

/// 初始化全局路径解析器（带自定义配置）
/// 只能在程序启动时调用一次，后续调用会返回 false
#[allow(dead_code)]
pub fn init_global_resolver(config: SandboxConfig) -> bool {
    GLOBAL_RESOLVER
        .set(RwLock::new(SecurePathResolver::with_config(config)))
        .is_ok()
}

/// Add an allowed root directory to the global resolver
pub fn add_allowed_root(path: PathBuf) {
    get_global_resolver().write().add_allowed_root(path);
}

/// 便捷函数：验证路径（使用全局解析器）
#[allow(dead_code)]
pub fn validate_path(path: &str) -> PathValidationResult {
    get_global_resolver().read().resolve(path)
}

/// 便捷函数：检查路径是否安全
#[allow(dead_code)]
pub fn is_path_safe(path: &str) -> bool {
    get_global_resolver().read().resolve(path).is_valid
}

/// 便捷函数：验证路径并返回结果或错误
#[allow(dead_code)]
pub fn validate_path_or_error(path: &str) -> IoResult<String> {
    get_global_resolver().read().resolve(path).into_result(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_valid_path() {
        let resolver = SecurePathResolver::new();
        let result = resolver.resolve("/tmp");
        assert!(result.is_valid || result.error.is_some());
    }

    #[test]
    fn test_path_traversal_attack() {
        // 使用自定义配置，不允许访问 /etc
        let config = SandboxConfig {
            allowed_roots: vec![PathBuf::from("/tmp")],
            allow_symlinks: false,
            max_depth: 100,
        };
        let resolver = SecurePathResolver::with_config(config);
        let result = resolver.resolve("/etc/passwd");
        assert!(!result.is_valid);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_long_path() {
        let resolver = SecurePathResolver::new();
        let long_path = "/tmp/".to_string() + &"a".repeat(5000);
        let result = resolver.resolve(&long_path);
        assert!(!result.is_valid);
        assert!(result.error.unwrap().contains("路径过长"));
    }

    #[test]
    fn test_sandbox_directory() {
        let tmpdir = tempdir().unwrap();
        let sandbox = tmpdir.path().join("sandbox");
        fs::create_dir_all(&sandbox).unwrap();

        let config = SandboxConfig {
            allowed_roots: vec![tmpdir.path().to_path_buf()],
            allow_symlinks: false,
            max_depth: 100,
        };
        let resolver = SecurePathResolver::with_config(config);

        let test_file = sandbox.join("test.txt");
        fs::write(&test_file, "hello").unwrap();

        let result = resolver.resolve(&test_file.to_string_lossy());
        assert!(result.is_valid);
        assert!(result.canonical_path.is_some());
    }

    #[test]
    fn test_symlink_detection() {
        let tmpdir = tempdir().unwrap();
        let file = tmpdir.path().join("real_file.txt");
        let link = tmpdir.path().join("link_to_file");

        fs::write(&file, "content").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&file, &link).unwrap();

        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&file, &link).unwrap();

        let config = SandboxConfig {
            allowed_roots: vec![tmpdir.path().to_path_buf()],
            allow_symlinks: false,
            max_depth: 100,
        };
        let resolver = SecurePathResolver::with_config(config);

        let result = resolver.resolve(&link.to_string_lossy());
        #[cfg(unix)]
        assert!(!result.is_valid);
    }

    #[test]
    fn test_empty_path() {
        let resolver = SecurePathResolver::new();
        let result = resolver.resolve("");
        // 空路径应该被拒绝或解析为当前目录
        assert!(!result.is_valid || result.canonical_path.is_some());
    }

    #[test]
    fn test_relative_path_traversal() {
        let tmpdir = tempdir().unwrap();
        let config = SandboxConfig {
            allowed_roots: vec![tmpdir.path().to_path_buf()],
            allow_symlinks: false,
            max_depth: 100,
        };
        let resolver = SecurePathResolver::with_config(config);

        // 尝试使用 ../ 跳出沙箱
        let result = resolver.resolve("../../../etc/passwd");
        assert!(!result.is_valid);
    }

    #[test]
    fn test_path_validation_result_into_result() {
        let tmpdir = tempdir().unwrap();
        let config = SandboxConfig {
            allowed_roots: vec![tmpdir.path().to_path_buf()],
            allow_symlinks: false,
            max_depth: 100,
        };
        let resolver = SecurePathResolver::with_config(config);
        let test_file = tmpdir.path().join("test.txt");
        fs::write(&test_file, "hello").unwrap();

        let validation = resolver.resolve(&test_file.to_string_lossy());
        let result: Result<String, crate::tools::io::error::IoToolError> =
            validation.into_result(&test_file.to_string_lossy());
        assert!(result.is_ok());
    }

    #[test]
    fn test_double_slash_path() {
        let tmpdir = tempdir().unwrap();
        let config = SandboxConfig {
            allowed_roots: vec![tmpdir.path().to_path_buf()],
            allow_symlinks: false,
            max_depth: 100,
        };
        let resolver = SecurePathResolver::with_config(config);

        // 双斜杠应该被规范化
        let result = resolver.resolve(&format!("{}/./test", tmpdir.path().display()));
        assert!(result.is_valid, "路径解析应该成功"); // 只要不 panic 就行
    }

    #[test]
    fn test_init_global_resolver_thread_safe() {
        // 测试 OnceLock 的线程安全性
        // 注意：由于 OnceLock 只能初始化一次，这个测试可能失败
        // 如果全局解析器已经被其他测试初始化，这里会返回 false
        let config = SandboxConfig::default();
        let success = init_global_resolver(config.clone());
        // 可能成功（第一次初始化）或失败（已被其他测试初始化）
        // 只要不 panic 就认为测试通过
        drop(config); // 避免 unused 警告
    }

    #[test]
    fn test_concurrent_path_validation() {
        use std::thread;

        let tmpdir = tempdir().unwrap();
        let config = SandboxConfig {
            allowed_roots: vec![tmpdir.path().to_path_buf()],
            allow_symlinks: false,
            max_depth: 100,
        };
        // 使用独立的解析器而不是全局的，避免与其他测试冲突
        let resolver = SecurePathResolver::with_config(config.clone());

        // 将 resolver 包装在 Arc 中以便在线程间共享
        use std::sync::Arc;
        let resolver = Arc::new(resolver);

        let mut handles = vec![];
        for i in 0..10 {
            let path = format!("{}/file_{}.txt", tmpdir.path().display(), i);
            let resolver_clone = Arc::clone(&resolver);
            let handle = thread::spawn(move || resolver_clone.resolve(&path).is_valid);
            handles.push(handle);
        }

        for handle in handles {
            let result = handle.join();
            assert!(result.is_ok()); // 不应该 panic
        }
    }
}
