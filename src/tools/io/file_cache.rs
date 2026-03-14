use moka::sync::Cache;
use std::time::Duration;
use std::fs;

/// 文件操作缓存层（LRU 缓存）
#[allow(dead_code)]
pub struct FileCache {
    cache: Cache<String, String>,
}

impl FileCache {
    pub fn new() -> Self {
        Self {
            cache: Cache::builder()
                .max_capacity(50)
                .time_to_live(Duration::from_secs(300))
                .build(),
        }
    }

    /// 读取文件（带缓存）
    pub fn read(&self, path: &str) -> Option<String> {
        // 使用 mtime 作为缓存键的一部分
        let mtime = self.get_mtime(path).unwrap_or(0);
        let cache_key = format!("{}:{}", path, mtime);
        self.cache.get(&cache_key)
    }

    /// 插入文件内容到缓存
    pub fn insert(&self, path: &str, content: String) {
        let mtime = self.get_mtime(path).unwrap_or(0);
        let cache_key = format!("{}:{}", path, mtime);
        self.cache.insert(cache_key, content);
    }

    /// 清除所有缓存
    pub fn invalidate_all(&self) {
        self.cache.invalidate_all();
    }

    /// 清除特定路径的缓存
    /// 注意：由于缓存键包含 mtime，我们无法精确匹配，需要清除所有
    /// TODO: 可以维护一个路径到缓存键的映射
    #[allow(dead_code)]
    pub fn invalidate_path(&self, _path: &str) {
        // 由于缓存键包含 mtime，我们无法精确匹配，需要清除所有
        // 或者可以维护一个路径到缓存键的映射
        self.cache.invalidate_all();
    }

    fn get_mtime(&self, path: &str) -> Option<u64> {
        fs::metadata(path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
    }
}
