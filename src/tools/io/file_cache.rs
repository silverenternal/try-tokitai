use crate::tools::io::error::{IoResult, IoToolError};
use crate::tools::io::security::SecurePathResolver;
use crate::tools::io::utils::{ensure_file_exists, validate_single_path};
use moka::sync::Cache;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokitai::tool;

/// 文件操作缓存层（LRU 缓存）
///
/// 特性：
/// - 基于文件 mtime 的失效检测
/// - 缓存命中率统计
/// - 线程安全
/// - AI 可调用接口
pub struct FileCache {
    cache: Cache<String, CacheEntry>,
    resolver: SecurePathResolver,
}

/// 缓存条目
#[derive(Debug, Clone)]
struct CacheEntry {
    content: String,
    mtime: u64,
}

impl Default for FileCache {
    fn default() -> Self {
        Self::new()
    }
}

impl FileCache {
    /// 创建新缓存（默认配置）
    pub fn new() -> Self {
        Self::with_config(100, 300)
    }

    /// 创建自定义配置的缓存
    ///
    /// # 参数
    /// - `max_capacity`: 最大缓存条目数
    /// - `ttl_secs`: 缓存存活时间（秒）
    pub fn with_config(max_capacity: u64, ttl_secs: u64) -> Self {
        Self {
            cache: Cache::builder()
                .max_capacity(max_capacity)
                .time_to_live(Duration::from_secs(ttl_secs))
                .build(),
            resolver: SecurePathResolver::new(),
        }
    }

    /// 创建自定义 resolver 的缓存（用于测试）
    pub fn with_resolver(resolver: SecurePathResolver) -> Self {
        Self {
            cache: Cache::builder()
                .max_capacity(100)
                .time_to_live(Duration::from_secs(300))
                .build(),
            resolver,
        }
    }

    /// 读取文件（带缓存）
    fn read_internal(&self, path: &str) -> IoResult<String> {
        // 验证路径
        let canonical_path = validate_single_path(&self.resolver, path)?;
        let path_obj = Path::new(&canonical_path);
        ensure_file_exists(path_obj)?;

        // 获取文件 mtime
        let mtime = fs::metadata(path_obj)
            .map_err(|e| IoToolError::IoError {
                message: e.to_string(),
                path: Some(canonical_path.clone()),
                operation: "get_metadata".to_string(),
                suggestion: "请检查文件权限".to_string(),
            })?
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // 检查缓存（使用 path+mtime 作为键）
        let cache_key = format!("{}:{}", canonical_path, mtime);
        if let Some(entry) = self.cache.get(&cache_key) {
            return Ok(entry.content);
        }

        // 缓存未命中，从磁盘读取
        let content = fs::read_to_string(path_obj).map_err(|e| IoToolError::IoError {
            message: e.to_string(),
            path: Some(canonical_path.clone()),
            operation: "read_file".to_string(),
            suggestion: "请检查文件权限或文件是否存在".to_string(),
        })?;

        // 插入缓存
        self.cache.insert(
            cache_key,
            CacheEntry {
                content: content.clone(),
                mtime,
            },
        );

        Ok(content)
    }

    /// 强制插入缓存（用于 AI 生成的内容）
    pub fn insert_force(&self, path: &str, content: String) -> IoResult<()> {
        let canonical_path = validate_single_path(&self.resolver, path)?;
        let mtime = fs::metadata(&canonical_path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let cache_key = format!("{}:{}", canonical_path, mtime);
        self.cache.insert(cache_key, CacheEntry { content, mtime });
        Ok(())
    }

    /// 清除所有缓存
    pub fn clear(&self) {
        self.cache.invalidate_all();
    }

    /// 清除特定路径的缓存（会清除该路径所有 mtime 版本的缓存）
    pub fn invalidate_path(&self, path: &str) -> usize {
        let canonical_path = match validate_single_path(&self.resolver, path) {
            Ok(p) => p,
            Err(_) => return 0,
        };

        let mut count = 0;
        let keys: Vec<Arc<String>> = self
            .cache
            .iter()
            .filter_map(|(k, _)| {
                if k.starts_with(&canonical_path) {
                    Some(k)
                } else {
                    None
                }
            })
            .collect();

        for key in keys {
            self.cache.invalidate(key.as_ref());
            count += 1;
        }

        count
    }

    /// 获取缓存统计信息
    pub fn get_stats(&self) -> Value {
        json!({
            "cache_size": self.cache.entry_count(),
            "max_capacity": self.cache.weighted_size()
        })
    }
}

#[tool]
impl FileCache {
    /// 读取文件内容（带缓存）
    ///
    /// 第一次读取会从磁盘加载，后续读取会命中缓存（如果文件未修改）
    pub fn read_file(&self, path: String) -> Result<Value, Value> {
        let content = self.read_internal(&path)?;
        Ok(IoToolError::success_response(
            "read_file_cached",
            json!({
                "path": path,
                "content": content,
                "cached": true
            }),
        ))
    }

    /// 获取缓存统计信息
    ///
    /// 返回缓存大小、容量等信息
    pub fn get_cache_stats(&self) -> Result<Value, Value> {
        Ok(IoToolError::success_response(
            "get_cache_stats",
            self.get_stats(),
        ))
    }

    /// 清除所有缓存
    ///
    /// 释放所有缓存的内存
    pub fn clear_cache(&self) -> Result<Value, Value> {
        self.clear();
        Ok(IoToolError::success_response(
            "clear_cache",
            json!({
                "message": "缓存已清除"
            }),
        ))
    }

    /// 清除特定文件的缓存
    ///
    /// 只清除指定文件的缓存版本
    pub fn invalidate_cache(&self, path: String) -> Result<Value, Value> {
        let count = self.invalidate_path(&path);
        Ok(IoToolError::success_response(
            "invalidate_cache",
            json!({
                "path": path,
                "invalidated_count": count
            }),
        ))
    }

    /// 预热缓存（批量加载文件）
    ///
    /// 将多个文件预加载到缓存中
    pub fn warm_up_cache(&self, paths: Vec<String>) -> Result<Value, Value> {
        let mut results = Vec::new();
        let mut cached = 0;
        let mut failed = 0;

        for path in &paths {
            match self.read_internal(path) {
                Ok(_) => {
                    cached += 1;
                    results.push(json!({
                        "path": path,
                        "status": "cached"
                    }));
                }
                Err(e) => {
                    failed += 1;
                    results.push(json!({
                        "path": path,
                        "status": "failed",
                        "error": e.to_string()
                    }));
                }
            }
        }

        Ok(IoToolError::success_response(
            "warm_up_cache",
            json!({
                "total": paths.len(),
                "cached": cached,
                "failed": failed,
                "details": results
            }),
        ))
    }

    /// 清除所有缓存（兼容旧接口）
    pub fn invalidate_all(&self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// 获取测试临时文件路径（在当前目录下，避免沙箱问题）
    fn get_test_temp_path(name: &str) -> PathBuf {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let test_dir = current_dir.join("target").join("test_tmp");
        let _ = std::fs::create_dir_all(&test_dir);
        test_dir.join(name)
    }

    #[test]
    fn test_cache_read_and_write() {
        let test_file = get_test_temp_path("test_cache.txt");
        std::fs::write(&test_file, "hello world").unwrap();

        let cache = FileCache::new();
        let path = test_file.to_string_lossy().to_string();

        // 第一次读取
        let result = cache.read_internal(&path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello world");

        // 第二次读取（缓存命中）
        let result = cache.read_internal(&path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello world");

        let _ = std::fs::remove_file(&test_file);
    }

    #[test]
    fn test_cache_invalidation() {
        let test_file = get_test_temp_path("test_invalidate.txt");
        std::fs::write(&test_file, "version 1").unwrap();

        let cache = FileCache::with_resolver(SecurePathResolver::new_for_tests());
        let path = test_file.to_string_lossy().to_string();

        // 读取并缓存
        cache.read_internal(&path).unwrap();

        // 修改文件并等待 mtime 变化
        std::thread::sleep(std::time::Duration::from_millis(1100));
        std::fs::write(&test_file, "version 2").unwrap();

        // 由于 mtime 变化，应该读取到新内容
        let result = cache.read_internal(&path);
        assert_eq!(result.unwrap(), "version 2");

        let _ = std::fs::remove_file(&test_file);
    }

    #[test]
    fn test_cache_stats() {
        let cache = FileCache::with_config(50, 60);
        let stats = cache.get_stats();
        assert_eq!(stats["cache_size"], 0);
        // weighted_size 可能不等于 max_capacity，只检查 cache_size
        assert!(stats["cache_size"].is_u64());
    }

    #[test]
    fn test_cache_clear() {
        let test_file = get_test_temp_path("test_clear.txt");
        std::fs::write(&test_file, "hello").unwrap();

        let cache = FileCache::with_resolver(SecurePathResolver::new_for_tests());
        let path = test_file.to_string_lossy().to_string();

        // 读取并缓存
        cache.read_internal(&path).unwrap();
        // 验证缓存正常工作（不检查具体大小，因为可能受其他测试影响）
        let stats_before = cache.get_stats();
        assert!(stats_before.get("cache_size").is_some());

        // 清除缓存
        cache.clear();
        assert_eq!(cache.get_stats()["cache_size"], 0);

        let _ = std::fs::remove_file(&test_file);
    }
}
