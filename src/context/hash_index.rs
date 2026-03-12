//! 哈希索引模块
//!
//! 使用符号链接实现哈希值到文件路径的映射，支持快速检索。

use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

/// 哈希索引管理器
/// 
/// 在 Unix 系统上使用符号链接，在 Windows 上使用文本文件存储路径映射
pub struct HashIndex {
    index_dir: PathBuf,
}

impl HashIndex {
    /// 创建哈希索引管理器
    pub fn new<P: AsRef<Path>>(index_dir: P) -> Result<Self> {
        let index_dir = index_dir.as_ref().to_path_buf();
        
        if !index_dir.exists() {
            std::fs::create_dir_all(&index_dir)
                .with_context(|| format!("Failed to create hash index directory: {:?}", index_dir))?;
        }
        
        Ok(Self { index_dir })
    }

    /// 获取哈希对应的索引文件路径
    fn hash_path(&self, hash: &str) -> PathBuf {
        self.index_dir.join(hash)
    }

    /// 添加哈希映射（创建符号链接或路径文件）
    #[cfg(unix)]
    pub fn add(&self, hash: &str, target_path: &Path) -> Result<()> {
        let link_path = self.hash_path(hash);
        
        // 如果已存在，先删除
        if link_path.exists() || link_path.is_symlink() {
            let _ = std::fs::remove_file(&link_path);
        }

        // 创建相对路径符号链接
        let relative_target = pathdiff::diff_paths(target_path, &self.index_dir)
            .unwrap_or_else(|| target_path.to_path_buf());
        
        std::os::unix::fs::symlink(&relative_target, &link_path)
            .with_context(|| format!("Failed to create symlink for hash: {}", hash))?;
        
        Ok(())
    }

    /// 添加哈希映射（Windows 版本：使用文本文件存储路径）
    #[cfg(windows)]
    pub fn add(&self, hash: &str, target_path: &Path) -> Result<()> {
        let link_path = self.hash_path(hash);
        
        // 写入路径到文本文件
        let mut file = std::fs::File::create(&link_path)
            .with_context(|| format!("Failed to create hash index file: {:?}", link_path))?;
        
        let path_str = target_path.to_string_lossy();
        file.write_all(path_str.as_bytes())
            .with_context(|| format!("Failed to write path to hash index file: {:?}", link_path))?;
        
        Ok(())
    }

    /// 通过哈希值获取目标文件路径
    pub fn get_path(&self, hash: &str) -> Result<PathBuf> {
        let link_path = self.hash_path(hash);
        
        #[cfg(unix)]
        {
            // 读取符号链接目标
            std::fs::read_link(&link_path)
                .with_context(|| format!("Failed to read symlink for hash: {}", hash))
                .map(|path| {
                    // 如果是相对路径，转换为绝对路径
                    if path.is_relative() {
                        self.index_dir.join(path)
                    } else {
                        path
                    }
                })
        }
        
        #[cfg(windows)]
        {
            // 从文本文件读取路径
            let mut file = std::fs::File::open(&link_path)
                .with_context(|| format!("Failed to open hash index file: {:?}", link_path))?;
            
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .with_context(|| format!("Failed to read hash index file: {:?}", link_path))?;
            
            Ok(PathBuf::from(contents))
        }
    }

    /// 检查哈希是否存在
    pub fn contains(&self, hash: &str) -> bool {
        let link_path = self.hash_path(hash);
        link_path.exists() || link_path.is_symlink()
    }

    /// 移除哈希映射
    pub fn remove(&self, hash: &str) -> Result<()> {
        let link_path = self.hash_path(hash);
        if link_path.exists() || link_path.is_symlink() {
            std::fs::remove_file(&link_path)
                .with_context(|| format!("Failed to remove hash index: {}", hash))?;
        }
        Ok(())
    }

    /// 列出所有哈希值
    pub fn list_hashes(&self) -> Result<Vec<String>> {
        let mut hashes = Vec::new();
        
        for entry in std::fs::read_dir(&self.index_dir)
            .with_context(|| format!("Failed to read hash index directory: {:?}", self.index_dir))? 
        {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                hashes.push(name.to_string());
            }
        }
        
        Ok(hashes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_hash_index_add_and_get() {
        let temp_dir = TempDir::new().unwrap();
        let index_dir = temp_dir.path().join("hashes");
        let hash_index = HashIndex::new(&index_dir).unwrap();

        // 创建目标文件
        let target_file = temp_dir.path().join("target.txt");
        std::fs::write(&target_file, "test content").unwrap();

        // 添加哈希映射
        let hash = "abc123";
        hash_index.add(hash, &target_file).unwrap();

        // 获取路径
        let retrieved_path = hash_index.get_path(hash).unwrap();
        assert!(retrieved_path.exists());
        assert_eq!(retrieved_path.canonicalize().unwrap(), target_file.canonicalize().unwrap());
    }

    #[test]
    fn test_hash_index_contains() {
        let temp_dir = TempDir::new().unwrap();
        let index_dir = temp_dir.path().join("hashes");
        let hash_index = HashIndex::new(&index_dir).unwrap();

        let target_file = temp_dir.path().join("target.txt");
        std::fs::write(&target_file, "test content").unwrap();

        let hash = "abc123";
        hash_index.add(hash, &target_file).unwrap();

        assert!(hash_index.contains(hash));
        assert!(!hash_index.contains("nonexistent"));
    }

    #[test]
    fn test_hash_index_remove() {
        let temp_dir = TempDir::new().unwrap();
        let index_dir = temp_dir.path().join("hashes");
        let hash_index = HashIndex::new(&index_dir).unwrap();

        let target_file = temp_dir.path().join("target.txt");
        std::fs::write(&target_file, "test content").unwrap();

        let hash = "abc123";
        hash_index.add(hash, &target_file).unwrap();
        hash_index.remove(hash).unwrap();

        assert!(!hash_index.contains(hash));
    }

    #[test]
    fn test_hash_index_list() {
        let temp_dir = TempDir::new().unwrap();
        let index_dir = temp_dir.path().join("hashes");
        let hash_index = HashIndex::new(&index_dir).unwrap();

        let target_file = temp_dir.path().join("target.txt");
        std::fs::write(&target_file, "test content").unwrap();

        hash_index.add("hash1", &target_file).unwrap();
        hash_index.add("hash2", &target_file).unwrap();
        hash_index.add("hash3", &target_file).unwrap();

        let hashes = hash_index.list_hashes().unwrap();
        assert_eq!(hashes.len(), 3);
        assert!(hashes.contains(&"hash1".to_string()));
        assert!(hashes.contains(&"hash2".to_string()));
        assert!(hashes.contains(&"hash3".to_string()));
    }
}
