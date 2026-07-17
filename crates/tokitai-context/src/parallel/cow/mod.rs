//! Copy-on-Write (COW) 机制
//!
//! 实现高效的分支继承机制：
//! - 使用文件系统符号链接（symlink）实现 O(1) 复杂度的 fork
//! - 写入时自动复制文件，保证分支间隔离
//! - 支持 Linux/macOS 原生 symlink，Windows 使用 junction points 或降级为实际复制

use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use tracing::{info, warn, debug};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// COW 管理器配置
#[derive(Debug, Clone)]
pub struct CowConfig {
    /// 是否启用符号链接（false 则使用实际复制）
    pub use_symlinks: bool,
    /// Windows 平台是否使用 junction points（否则使用实际复制）
    pub use_junction_on_windows: bool,
    /// 写入时复制的缓冲区大小（字节）
    pub copy_buffer_size: usize,
}

impl Default for CowConfig {
    fn default() -> Self {
        Self {
            use_symlinks: true,
            use_junction_on_windows: true,
            copy_buffer_size: 8192,
        }
    }
}

/// 符号链接元数据
#[derive(Debug, Clone)]
pub struct SymlinkMetadata {
    /// 源文件路径
    pub source_path: PathBuf,
    /// 链接创建时间
    pub created_at: std::time::SystemTime,
    /// 是否已被写入（触发 COW）
    pub has_been_written: bool,
}

/// Copy-on-Write 管理器
pub struct CowManager {
    config: CowConfig,
    /// 跟踪所有符号链接及其元数据
    symlinks: Arc<RwLock<HashMap<PathBuf, SymlinkMetadata>>>,
    /// 平台类型
    platform: Platform,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Platform {
    Linux,
    MacOS,
    Windows,
    Unknown,
}

impl Platform {
    fn current() -> Self {
        #[cfg(target_os = "linux")]
        return Self::Linux;

        #[cfg(target_os = "macos")]
        return Self::MacOS;

        #[cfg(target_os = "windows")]
        return Self::Windows;

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        return Self::Unknown;
    }

    fn supports_symlinks(&self) -> bool {
        matches!(self, Self::Linux | Self::MacOS)
    }

    /// Check if platform supports junction points (Windows only)
    fn supports_junction(&self) -> bool {
        matches!(self, Self::Windows)
    }

    /// Get the recommended fallback_strategy for this platform
    fn fallback_strategy(&self) -> FallbackStrategy {
        match self {
            Self::Linux | Self::MacOS => FallbackStrategy::Copy,
            Self::Windows => FallbackStrategy::JunctionThenCopy,
            Self::Unknown => FallbackStrategy::Copy,
        }
    }
}

/// Fallback strategy for platforms without symlink support
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackStrategy {
    /// Just copy files (slow but works everywhere)
    Copy,
    /// Try junction first, then copy (Windows)
    JunctionThenCopy,
    /// Use hard links if possible
    HardLink,
}

impl CowManager {
    /// 创建 COW 管理器
    pub fn new(config: CowConfig) -> Self {
        let platform = Platform::current();

        info!(
            "Creating CowManager: platform={:?}, use_symlinks={}",
            platform, config.use_symlinks
        );

        Self {
            config,
            symlinks: Arc::new(RwLock::new(HashMap::new())),
            platform,
        }
    }

    /// 从默认配置创建管理器
    pub fn with_defaults() -> Self {
        Self::new(CowConfig::default())
    }

    /// Create a fork by copying files (fallback for Windows or when symlinks fail)
    /// 
    /// This method is slower than symlink-based fork but works on all platforms
    /// and filesystems, including network drives and filesystems that don't
    /// support symbolic links.
    /// 
    /// # Arguments
    /// * `source_dir` - Source branch directory
    /// * `target_dir` - Target branch directory
    /// * `layer_name` - Layer name (short-term, long-term)
    /// 
    /// # Returns
    /// * `Result<usize>` - Number of files copied
    #[tracing::instrument(skip_all, fields(source = %source_dir.display(), target = %target_dir.display()))]
    pub fn fork_with_copy(
        &self,
        source_dir: &Path,
        target_dir: &Path,
        layer_name: &str,
    ) -> Result<usize> {
        let source_layer = source_dir.join(layer_name);
        let target_layer = target_dir.join(layer_name);

        if !source_layer.exists() {
            debug!("Source layer does not exist: {:?}", source_layer);
            return Ok(0);
        }

        // Create target layer directory
        std::fs::create_dir_all(&target_layer)
            .with_context(|| format!("Failed to create target layer directory: {:?}", target_layer))?;

        let mut copy_count = 0;

        // Walk through all files in source directory
        for entry in walkdir::WalkDir::new(&source_layer)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let source_path = entry.path();
            let relative_path = source_path.strip_prefix(&source_layer)?;
            let target_path = target_layer.join(relative_path);

            if source_path.is_dir() {
                // Create subdirectory
                std::fs::create_dir_all(&target_path)?;
            } else if source_path.is_file() {
                // Copy file
                match std::fs::copy(source_path, &target_path) {
                    Ok(_) => {
                        copy_count += 1;
                        debug!("Copied file: {:?} -> {:?}", source_path, target_path);
                    }
                    Err(e) => {
                        warn!("Failed to copy file {:?}: {}", target_path, e);
                    }
                }
            }
        }

        info!(
            "Fork with copy completed: {} -> {} (layer: {}), {} files copied",
            source_dir.display(),
            target_dir.display(),
            layer_name,
            copy_count
        );

        Ok(copy_count)
    }

    /// Fork with platform-optimized strategy
    /// 
    /// Automatically chooses the best fork strategy based on platform:
    /// - Linux/macOS: Symlinks (O(1))
    /// - Windows: Junction points, falls back to copy
    /// - Other: Copy
    /// 
    /// # Arguments
    /// * `source_dir` - Source branch directory
    /// * `target_dir` - Target branch directory
    /// * `layer_name` - Layer name (short-term, long-term)
    /// 
    /// # Returns
    /// * `Result<usize>` - Number of links/files created
    #[tracing::instrument(skip_all, fields(source = %source_dir.display(), target = %target_dir.display()))]
    pub fn fork_optimized(
        &self,
        source_dir: &Path,
        target_dir: &Path,
        layer_name: &str,
    ) -> Result<usize> {
        if self.config.use_symlinks && self.platform.supports_symlinks() {
            // Use symlinks on Linux/macOS
            self.fork_with_symlinks(source_dir, target_dir, layer_name)
        } else if self.platform.supports_junction() && self.config.use_junction_on_windows {
            // Try junction on Windows
            self.fork_with_symlinks(source_dir, target_dir, layer_name)
                .or_else(|e| {
                    warn!("Junction failed: {}. Falling back to copy", e);
                    self.fork_with_copy(source_dir, target_dir, layer_name)
                })
        } else {
            // Fallback to copy
            debug!("Using copy fallback for unsupported platform");
            self.fork_with_copy(source_dir, target_dir, layer_name)
        }
    }

    /// 创建分支的符号链接继承
    ///
    /// # Arguments
    /// * `source_dir` - 源分支目录
    /// * `target_dir` - 目标分支目录
    /// * `layer_name` - 层名称（short-term, long-term）
    ///
    /// # Returns
    /// * `Result<usize>` - 创建的符号链接数量
    pub fn fork_with_symlinks(
        &self,
        source_dir: &Path,
        target_dir: &Path,
        layer_name: &str,
    ) -> Result<usize> {
        let source_layer = source_dir.join(layer_name);
        let target_layer = target_dir.join(layer_name);

        if !source_layer.exists() {
            debug!("Source layer does not exist: {:?}", source_layer);
            return Ok(0);
        }

        // 创建目标层目录
        std::fs::create_dir_all(&target_layer)
            .with_context(|| format!("Failed to create target layer directory: {:?}", target_layer))?;

        let mut symlink_count = 0;

        // 遍历源目录的所有文件和子目录
        for entry in walkdir::WalkDir::new(&source_layer)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let source_path = entry.path();
            let file_name = entry.file_name();
            let target_path = target_layer.join(file_name);

            if source_path.is_dir() {
                // 对于目录，递归创建符号链接
                let sublayer_name = file_name.to_string_lossy();
                let sub_count = self.fork_with_symlinks(
                    source_dir,
                    target_dir,
                    &format!("{}/{}", layer_name, sublayer_name),
                )?;
                symlink_count += sub_count;
            } else if source_path.is_file() {
                // 为文件创建符号链接
                match self.create_symlink(source_path, &target_path) {
                    Ok(_) => {
                        symlink_count += 1;
                        debug!(
                            "Created symlink: {:?} -> {:?}",
                            target_path, source_path
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to create symlink for {:?}: {}. Falling back to copy.",
                            target_path, e
                        );
                        // 降级为实际复制
                        match std::fs::copy(source_path, &target_path) {
                            Ok(_) => {
                                symlink_count += 1;
                                debug!("Copied file: {:?} -> {:?}", source_path, target_path);
                            }
                            Err(copy_err) => {
                                warn!(
                                    "Failed to copy file {:?}: {}",
                                    target_path, copy_err
                                );
                            }
                        }
                    }
                }
            }
        }

        info!(
            "Fork completed: {} -> {} (layer: {}), {} symlinks created",
            source_dir.display(),
            target_dir.display(),
            layer_name,
            symlink_count
        );

        Ok(symlink_count)
    }

    /// 创建符号链接（跨平台）
    fn create_symlink(&self, source: &Path, target: &Path) -> Result<()> {
        // 检查是否已存在
        if target.exists() {
            return Ok(());
        }

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(source, target)
                .with_context(|| format!("Failed to create symlink: {:?} -> {:?}", target, source))?;
        }

        #[cfg(windows)]
        {
            if self.config.use_junction_on_windows {
                // Windows 使用 junction points（需要目录）
                if source.is_dir() {
                    std::os::windows::fs::symlink_dir(source, target)
                        .with_context(|| format!("Failed to create dir symlink: {:?} -> {:?}", target, source))?;
                } else {
                    std::os::windows::fs::symlink_file(source, target)
                        .with_context(|| format!("Failed to create file symlink: {:?} -> {:?}", target, source))?;
                }
            } else {
                // 降级为实际复制
                std::fs::copy(source, target)
                    .with_context(|| format!("Failed to copy file: {:?} -> {:?}", source, target))?;
            }
        }

        // 记录符号链接元数据
        let metadata = SymlinkMetadata {
            source_path: source.to_path_buf(),
            created_at: std::time::SystemTime::now(),
            has_been_written: false,
        };

        if let Some(mut symlinks) = self.symlinks.try_write() {
            symlinks.insert(target.to_path_buf(), metadata);
        }

        Ok(())
    }

    /// 写入时复制（Copy-on-Write）
    ///
    /// 当检测到文件是符号链接且需要写入时，自动复制源文件
    ///
    /// # Arguments
    /// * `file_path` - 要写入的文件路径
    ///
    /// # Returns
    /// * `Result<bool>` - 是否触发了 COW 操作
    pub fn copy_on_write(&self, file_path: &Path) -> Result<bool> {
        // 检查是否是符号链接
        if !self.is_symlink(file_path)? {
            return Ok(false);
        }

        // 获取符号链接的源文件
        let source_path = self.read_symlink_target(file_path)?;

        // 复制源文件到目标位置
        debug!("COW triggered: {:?} -> {:?}", source_path, file_path);

        // 先删除符号链接
        std::fs::remove_file(file_path)
            .with_context(|| format!("Failed to remove symlink: {:?}", file_path))?;

        // 复制实际内容
        let bytes_copied = std::fs::copy(&source_path, file_path)
            .with_context(|| format!("Failed to copy file for COW: {:?} -> {:?}", source_path, file_path))?;

        // 更新元数据
        if let Some(mut symlinks) = self.symlinks.try_write() {
            if let Some(meta) = symlinks.get_mut(file_path) {
                meta.has_been_written = true;
            }
        }

        debug!(
            "COW completed: {:?} ({} bytes copied)",
            file_path, bytes_copied
        );

        Ok(true)
    }

    /// 检查文件是否是符号链接
    pub fn is_symlink(&self, path: &Path) -> Result<bool> {
        let metadata = std::fs::symlink_metadata(path)
            .with_context(|| format!("Failed to get metadata: {:?}", path))?;

        Ok(metadata.file_type().is_symlink())
    }

    /// 读取符号链接的目标
    pub fn read_symlink_target(&self, path: &Path) -> Result<PathBuf> {
        #[cfg(unix)]
        {
            std::fs::read_link(path)
                .with_context(|| format!("Failed to read symlink: {:?}", path))
        }

        #[cfg(windows)]
        {
            std::fs::read_link(path)
                .with_context(|| format!("Failed to read symlink: {:?}", path))
        }
    }

    /// 准备写入文件（如果需要，触发 COW）
    ///
    /// # Usage
    /// ```ignore
    /// cow_manager.prepare_for_write(&file_path)?;
    /// std::fs::write(&file_path, content)?;
    /// ```
    pub fn prepare_for_write(&self, file_path: &Path) -> Result<bool> {
        if file_path.exists() {
            self.copy_on_write(file_path)
        } else {
            Ok(false)
        }
    }

    /// 获取统计信息
    pub fn stats(&self) -> CowStats {
        let symlinks = self.symlinks.read();
        CowStats {
            total_symlinks: symlinks.len(),
            written_symlinks: symlinks.values().filter(|m| m.has_been_written).count(),
        }
    }

    /// 清理已写入的符号链接（可选的垃圾回收）
    pub fn cleanup_written_symlinks(&self) -> Result<usize> {
        let mut cleaned = 0;
        let mut symlinks = self.symlinks.write();

        let to_remove: Vec<PathBuf> = symlinks
            .iter()
            .filter(|(_, meta)| meta.has_been_written)
            .map(|(path, _)| path.clone())
            .collect();

        for path in to_remove {
            symlinks.remove(&path);
            cleaned += 1;
        }

        Ok(cleaned)
    }
}

/// COW 统计信息
#[derive(Debug, Clone)]
pub struct CowStats {
    pub total_symlinks: usize,
    pub written_symlinks: usize,
}

impl std::fmt::Display for CowStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "COW Statistics:")?;
        writeln!(f, "  Total symlinks: {}", self.total_symlinks)?;
        writeln!(f, "  Written symlinks (COW triggered): {}", self.written_symlinks)?;
        if self.total_symlinks > 0 {
            let ratio = self.written_symlinks as f64 / self.total_symlinks as f64 * 100.0;
            writeln!(f, "  COW ratio: {:.2}%", ratio)?;
        }
        Ok(())
    }
}

/// 分支克隆器 - 使用 COW 机制高效克隆分支
pub struct BranchCloner {
    cow_manager: Arc<CowManager>,
}

impl BranchCloner {
    /// 创建分支克隆器
    pub fn new(cow_manager: Arc<CowManager>) -> Self {
        Self { cow_manager }
    }

    /// 克隆分支的存储层
    ///
    /// # Arguments
    /// * `source_branch_dir` - 源分支目录
    /// * `target_branch_dir` - 目标分支目录
    /// * `layers` - 要克隆的层（["short-term", "long-term"]）
    ///
    /// # Returns
    /// * `Result<usize>` - 克隆的文件/符号链接数量
    pub fn clone_layers(
        &self,
        source_branch_dir: &Path,
        target_branch_dir: &Path,
        layers: &[&str],
    ) -> Result<usize> {
        let mut total_count = 0;

        for &layer in layers {
            let count = self.cow_manager.fork_with_symlinks(
                source_branch_dir,
                target_branch_dir,
                layer,
            )?;
            total_count += count;
        }

        Ok(total_count)
    }

    /// 使用层列表进行 fork
    ///
    /// # Arguments
    /// * `source_branch_dir` - 源分支目录
    /// * `target_branch_dir` - 目标分支目录
    /// * `layers` - 要克隆的层
    ///
    /// # Returns
    /// * `Result<ForkResult>` - fork 结果
    pub fn fork_with_layers(
        &self,
        source_branch_dir: &Path,
        target_branch_dir: &Path,
        layers: &[&str],
    ) -> Result<ForkResult> {
        let start_time = std::time::Instant::now();

        let symlinks_created = self.clone_layers(
            source_branch_dir,
            target_branch_dir,
            layers,
        )?;

        let duration = start_time.elapsed();

        Ok(ForkResult {
            source_branch: source_branch_dir.file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            target_branch: target_branch_dir.file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            symlinks_created,
            duration_ms: duration.as_millis() as u64,
        })
    }

    /// 高效 fork 操作
    ///
    /// # Arguments
    /// * `source_branch` - 源分支
    /// * `target_branch` - 目标分支
    ///
    /// # Returns
    /// * `Result<ForkResult>` - fork 结果
    pub fn fork(
        &self,
        source_branch: &crate::branch::ContextBranch,
        target_branch: &crate::branch::ContextBranch,
    ) -> Result<ForkResult> {
        let start_time = std::time::Instant::now();

        // 使用 COW 机制克隆短期层和长期层
        let layers = ["short-term", "long-term"];
        let symlink_count = self.clone_layers(
            &source_branch.branch_dir,
            &target_branch.branch_dir,
            &layers,
        )?;

        let duration = start_time.elapsed();

        Ok(ForkResult {
            source_branch: source_branch.branch_id.clone(),
            target_branch: target_branch.branch_id.clone(),
            symlinks_created: symlink_count,
            duration_ms: duration.as_millis() as u64,
        })
    }
}

/// Fork 操作结果
#[derive(Debug, Clone)]
pub struct ForkResult {
    pub source_branch: String,
    pub target_branch: String,
    pub symlinks_created: usize,
    pub duration_ms: u64,
}

impl std::fmt::Display for ForkResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Fork Result:")?;
        writeln!(f, "  Source: {}", self.source_branch)?;
        writeln!(f, "  Target: {}", self.target_branch)?;
        writeln!(f, "  Symlinks created: {}", self.symlinks_created)?;
        writeln!(f, "  Duration: {}ms", self.duration_ms)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::branch::ContextBranch;

    #[test]
    fn test_cow_manager_creation() {
        let cow = CowManager::with_defaults();
        let stats = cow.stats();
        assert_eq!(stats.total_symlinks, 0);
    }

    #[test]
    fn test_fork_with_symlinks() {
        let temp_dir = TempDir::new().unwrap();
        let cow = CowManager::with_defaults();

        // 创建源分支目录结构
        let source_dir = temp_dir.path().join("source");
        let source_layer = source_dir.join("short-term");
        std::fs::create_dir_all(&source_layer).unwrap();

        // 添加一些测试文件
        std::fs::write(source_layer.join("file1.txt"), "content1").unwrap();
        std::fs::write(source_layer.join("file2.txt"), "content2").unwrap();

        // 创建目标分支目录
        let target_dir = temp_dir.path().join("target");
        std::fs::create_dir_all(&target_dir).unwrap();

        // 执行 fork
        let count = cow.fork_with_symlinks(&source_dir, &target_dir, "short-term").unwrap();

        assert_eq!(count, 2);

        // 验证目标目录存在
        let target_layer = target_dir.join("short-term");
        assert!(target_layer.exists());
        assert!(target_layer.join("file1.txt").exists());
        assert!(target_layer.join("file2.txt").exists());
    }

    #[test]
    fn test_branch_cloner() {
        let temp_dir = TempDir::new().unwrap();
        let cow = Arc::new(CowManager::with_defaults());
        let cloner = BranchCloner::new(cow);

        // 创建源分支
        let source_dir = temp_dir.path().join("source");
        let source_branch = ContextBranch::new("source", "source", "main", source_dir.clone()).unwrap();

        // 添加测试文件
        std::fs::write(source_branch.short_term_dir.join("test.txt"), "test content").unwrap();
        std::fs::write(source_branch.long_term_dir.join("config.json"), "{}").unwrap();

        // 创建目标分支
        let target_dir = temp_dir.path().join("target");
        let target_branch = ContextBranch::new("target", "target", "main", target_dir).unwrap();

        // 执行 fork
        let result = cloner.fork_with_layers(
            &source_branch.branch_dir,
            &target_branch.branch_dir,
            &["short-term", "long-term"],
        ).unwrap();

        assert!(result.symlinks_created > 0);
        assert!(result.duration_ms < 100); // 应该在 100ms 内完成

        println!("{}", result);
    }

    #[test]
    #[cfg(unix)] // 仅在 Unix 系统上测试符号链接
    fn test_copy_on_write() {
        let temp_dir = TempDir::new().unwrap();
        let cow = CowManager::with_defaults();

        // 创建源文件
        let source_file = temp_dir.path().join("source.txt");
        std::fs::write(&source_file, "original content").unwrap();

        // 创建符号链接
        let link_file = temp_dir.path().join("link.txt");
        std::os::unix::fs::symlink(&source_file, &link_file).unwrap();

        // 验证是符号链接
        assert!(cow.is_symlink(&link_file).unwrap());

        // 准备写入（触发 COW）
        let cow_triggered = cow.prepare_for_write(&link_file).unwrap();
        assert!(cow_triggered);

        // 验证不再是符号链接
        assert!(!cow.is_symlink(&link_file).unwrap());

        // 写入新内容
        std::fs::write(&link_file, "modified content").unwrap();

        // 验证源文件未受影响
        let source_content = std::fs::read_to_string(&source_file).unwrap();
        assert_eq!(source_content, "original content");

        // 验证目标文件已修改
        let link_content = std::fs::read_to_string(&link_file).unwrap();
        assert_eq!(link_content, "modified content");
    }
}
