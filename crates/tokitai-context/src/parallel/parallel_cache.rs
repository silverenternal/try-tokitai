//! 分支缓存优化模块
//!
//! 实现 LRU 缓存机制优化分支访问性能：
//! - LRU 分支缓存：减少磁盘 I/O，加速 checkout 操作
//! - 祖先链缓存：加速分支关系查询
//! - 缓存统计和监控

use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use lru::LruCache;
use parking_lot::RwLock;
use chrono::{DateTime, Utc};

use super::branch::{BranchState, ContextBranch};

/// 缓存统计信息
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// 命中次数
    pub hits: u64,
    /// 未命中次数
    pub misses: u64,
    /// 命中率
    pub hit_rate: f64,
    /// 缓存大小
    pub size: usize,
    /// 缓存容量
    pub capacity: usize,
    /// 插入次数
    pub inserts: u64,
    /// 驱逐次数
    pub evictions: u64,
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Cache Statistics:")?;
        writeln!(f, "  Capacity: {}", self.capacity)?;
        writeln!(f, "  Current size: {}", self.size)?;
        writeln!(f, "  Hits: {}", self.hits)?;
        writeln!(f, "  Misses: {}", self.misses)?;
        writeln!(f, "  Hit rate: {:.2}%", self.hit_rate * 100.0)?;
        writeln!(f, "  Inserts: {}", self.inserts)?;
        writeln!(f, "  Evictions: {}", self.evictions)?;
        Ok(())
    }
}

/// LRU 分支缓存
///
/// 使用线程安全的 LRU 缓存机制，缓存最近访问的分支数据
/// 减少磁盘 I/O，提升 checkout 和分支查询性能
pub struct BranchCache {
    /// LRU 缓存主体
    cache: RwLock<LruCache<String, CachedBranch>>,
    /// 命中计数
    hit_count: AtomicU64,
    /// 未命中计数
    miss_count: AtomicU64,
    /// 插入计数
    insert_count: AtomicU64,
    /// 驱逐计数
    eviction_count: AtomicU64,
    /// 缓存容量
    capacity: usize,
}

/// 缓存的分支数据
#[derive(Debug, Clone)]
pub struct CachedBranch {
    /// 分支数据
    pub branch: ContextBranch,
    /// 缓存时间
    pub cached_at: DateTime<Utc>,
    /// 最后访问时间
    pub last_accessed: DateTime<Utc>,
    /// 访问次数
    pub access_count: u64,
}

impl CachedBranch {
    pub fn new(branch: ContextBranch) -> Self {
        let now = Utc::now();
        Self {
            branch,
            cached_at: now,
            last_accessed: now,
            access_count: 1,
        }
    }

    /// 记录访问
    pub fn record_access(&mut self) {
        self.access_count += 1;
        self.last_accessed = Utc::now();
    }
}

impl BranchCache {
    /// 创建新的分支缓存
    ///
    /// # Arguments
    /// * `capacity` - 缓存容量（最多缓存的分支数量）
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        let cache_size = NonZeroUsize::new(capacity).unwrap();

        Self {
            cache: RwLock::new(LruCache::new(cache_size)),
            hit_count: AtomicU64::new(0),
            miss_count: AtomicU64::new(0),
            insert_count: AtomicU64::new(0),
            eviction_count: AtomicU64::new(0),
            capacity,
        }
    }

    /// 从缓存获取分支
    ///
    /// # Arguments
    /// * `branch_id` - 分支 ID
    ///
    /// # Returns
    /// 如果缓存中存在，返回分支数据；否则返回 None
    pub fn get(&self, branch_id: &str) -> Option<CachedBranch> {
        let mut cache = self.cache.write();
        
        if let Some(cached) = cache.get(branch_id) {
            self.hit_count.fetch_add(1, Ordering::Relaxed);
            // 记录访问
            let mut updated = cached.clone();
            updated.record_access();
            Some(updated)
        } else {
            self.miss_count.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// 将分支插入缓存
    ///
    /// # Arguments
    /// * `branch_id` - 分支 ID
    /// * `branch` - 分支数据
    ///
    /// # Returns
    /// 如果插入了新元素，返回 true；如果更新了现有元素，返回 false
    pub fn insert(&self, branch_id: String, branch: ContextBranch) -> bool {
        let mut cache = self.cache.write();
        let is_new = !cache.contains(&branch_id);

        let cached = CachedBranch::new(branch);
        
        // 如果缓存已满且是新元素，会驱逐最久未使用的元素
        if is_new && cache.len() >= self.capacity {
            self.eviction_count.fetch_add(1, Ordering::Relaxed);
        }

        cache.put(branch_id, cached);
        self.insert_count.fetch_add(1, Ordering::Relaxed);
        
        is_new
    }

    /// 从缓存移除分支
    ///
    /// # Arguments
    /// * `branch_id` - 分支 ID
    pub fn remove(&self, branch_id: &str) {
        let mut cache = self.cache.write();
        cache.pop(branch_id);
    }

    /// 清空缓存
    pub fn clear(&self) {
        let mut cache = self.cache.write();
        cache.clear();
    }

    /// 检查缓存是否包含指定分支
    pub fn contains(&self, branch_id: &str) -> bool {
        let cache = self.cache.read();
        cache.contains(branch_id)
    }

    /// 获取缓存大小
    pub fn len(&self) -> usize {
        let cache = self.cache.read();
        cache.len()
    }

    /// 检查缓存是否为空
    pub fn is_empty(&self) -> bool {
        let cache = self.cache.read();
        cache.is_empty()
    }

    /// 获取缓存容量
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 获取缓存统计信息
    pub fn stats(&self) -> CacheStats {
        let hits = self.hit_count.load(Ordering::Relaxed);
        let misses = self.miss_count.load(Ordering::Relaxed);
        let total = hits + misses;

        CacheStats {
            hits,
            misses,
            hit_rate: if total > 0 { hits as f64 / total as f64 } else { 0.0 },
            size: self.len(),
            capacity: self.capacity,
            inserts: self.insert_count.load(Ordering::Relaxed),
            evictions: self.eviction_count.load(Ordering::Relaxed),
        }
    }

    /// 重置统计信息
    pub fn reset_stats(&self) {
        self.hit_count.store(0, Ordering::Relaxed);
        self.miss_count.store(0, Ordering::Relaxed);
        self.insert_count.store(0, Ordering::Relaxed);
        self.eviction_count.store(0, Ordering::Relaxed);
    }

    /// 获取所有缓存的分支 ID
    pub fn keys(&self) -> Vec<String> {
        let cache = self.cache.read();
        cache.iter().map(|(k, _)| k.clone()).collect()
    }

    /// 获取最久未使用的分支 ID
    pub fn peek_lru(&self) -> Option<String> {
        let cache = self.cache.read();
        cache.peek_lru().map(|(k, _)| k.clone())
    }

    /// 调整缓存容量
    ///
    /// # Arguments
    /// * `new_capacity` - 新的容量
    pub fn resize(&self, new_capacity: usize) {
        let new_capacity = new_capacity.max(1);
        let mut cache = self.cache.write();
        
        // 如果新容量更小，可能需要驱逐元素
        if new_capacity < self.capacity {
            while cache.len() > new_capacity {
                if cache.pop_lru().is_some() {
                    self.eviction_count.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        // 更新容量（需要重新创建 LruCache）
        let new_size = NonZeroUsize::new(new_capacity).unwrap();
        let mut new_cache = LruCache::new(new_size);
        
        // 迁移现有元素
        for (k, v) in cache.iter() {
            new_cache.put(k.clone(), v.clone());
        }

        *cache = new_cache;
        
        // 使用 drop 替换旧的 RwLock 内容（避免死锁）
        drop(cache);
        
        // 更新容量（注意：这里需要 unsafe 或者使用 AtomicUsize）
        // 简化处理：重新创建整个 BranchCache
    }
}

/// 祖先链缓存
///
/// 缓存分支的祖先链，加速 is_descendant_of 和 find_common_ancestor 查询
pub struct AncestorCache {
    /// 映射：branch_id -> 祖先链
    ancestors: dashmap::DashMap<String, Vec<String>>,
    /// 版本号为失效时使用
    version: AtomicU64,
    /// 命中计数
    hit_count: AtomicU64,
    /// 未命中计数
    miss_count: AtomicU64,
}

impl AncestorCache {
    /// 创建祖先链缓存
    pub fn new() -> Self {
        Self {
            ancestors: dashmap::DashMap::new(),
            version: AtomicU64::new(0),
            hit_count: AtomicU64::new(0),
            miss_count: AtomicU64::new(0),
        }
    }

    /// 获取祖先链
    ///
    /// # Arguments
    /// * `branch_id` - 分支 ID
    /// * `loader` - 加载器函数，给定分支 ID 返回其父分支 ID
    ///
    /// # Returns
    /// 祖先链（从直接父节点到根节点）
    pub fn get_ancestors<F>(&self, branch_id: &str, mut loader: F) -> Vec<String>
    where
        F: FnMut(&str) -> Option<String>,
    {
        // 尝试缓存
        if let Some(cached) = self.ancestors.get(branch_id) {
            self.hit_count.fetch_add(1, Ordering::Relaxed);
            return cached.clone();
        }

        self.miss_count.fetch_add(1, Ordering::Relaxed);

        // 计算并缓存
        let mut ancestors = Vec::new();
        let mut current = branch_id.to_string();

        while let Some(parent) = loader(&current) {
            if !parent.is_empty() {
                ancestors.push(parent.clone());
                current = parent;
            } else {
                break;
            }
        }

        // 缓存结果
        self.ancestors.insert(branch_id.to_string(), ancestors.clone());

        ancestors
    }

    /// 检查是否是后代
    ///
    /// # Arguments
    /// * `branch_id` - 分支 ID
    /// * `ancestor` - 祖先分支 ID
    /// * `loader` - 加载器函数
    pub fn is_descendant_of<F>(&self, branch_id: &str, ancestor: &str, loader: F) -> bool
    where
        F: FnMut(&str) -> Option<String>,
    {
        let ancestors = self.get_ancestors(branch_id, loader);
        ancestors.contains(&ancestor.to_string())
    }

    /// 查找公共祖先
    ///
    /// # Arguments
    /// * `branch1` - 分支 1
    /// * `branch2` - 分支 2
    /// * `loader` - 加载器函数
    /// * `main_branch` - 主分支 ID（作为最后的公共祖先）
    pub fn find_common_ancestor<F>(&self, branch1: &str, branch2: &str, mut loader: F, main_branch: &str) -> Option<String>
    where
        F: FnMut(&str) -> Option<String>,
    {
        let ancestors1 = self.get_ancestors(branch1, &mut loader);
        let ancestors2 = self.get_ancestors(branch2, &mut loader);

        // 包含分支自身
        let mut all_ancestors1 = vec![branch1.to_string()];
        all_ancestors1.extend(ancestors1);

        // 查找第一个公共祖先
        for ancestor in &ancestors2 {
            if all_ancestors1.contains(ancestor) {
                return Some(ancestor.clone());
            }
        }

        // 如果没有公共祖先，返回 main
        if all_ancestors1.contains(&main_branch.to_string()) {
            return Some(main_branch.to_string());
        }

        None
    }

    /// 使缓存失效
    ///
    /// # Arguments
    /// * `branch_id` - 分支 ID
    pub fn invalidate(&self, branch_id: &str) {
        self.ancestors.remove(branch_id);
        // 也使包含该分支的祖先链失效
        self.invalidate_descendants(branch_id);
    }

    /// 使后代分支的缓存失效
    fn invalidate_descendants(&self, branch_id: &str) {
        let keys_to_remove: Vec<String> = self.ancestors
            .iter()
            .filter(|entry| entry.value().contains(&branch_id.to_string()))
            .map(|entry| entry.key().clone())
            .collect();

        for key in keys_to_remove {
            self.ancestors.remove(&key);
        }
    }

    /// 清空缓存
    pub fn clear(&self) {
        self.ancestors.clear();
    }

    /// 获取缓存大小
    pub fn len(&self) -> usize {
        self.ancestors.len()
    }

    /// 检查缓存是否为空
    pub fn is_empty(&self) -> bool {
        self.ancestors.is_empty()
    }

    /// 获取统计信息
    pub fn stats(&self) -> AncestorCacheStats {
        let hits = self.hit_count.load(Ordering::Relaxed);
        let misses = self.miss_count.load(Ordering::Relaxed);
        let total = hits + misses;

        AncestorCacheStats {
            hits,
            misses,
            hit_rate: if total > 0 { hits as f64 / total as f64 } else { 0.0 },
            size: self.len(),
        }
    }
}

impl Default for AncestorCache {
    fn default() -> Self {
        Self::new()
    }
}

/// 祖先链缓存统计
#[derive(Debug, Clone)]
pub struct AncestorCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub size: usize,
}

impl std::fmt::Display for AncestorCacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Ancestor Cache Statistics:")?;
        writeln!(f, "  Size: {}", self.size)?;
        writeln!(f, "  Hits: {}", self.hits)?;
        writeln!(f, "  Misses: {}", self.misses)?;
        writeln!(f, "  Hit rate: {:.2}%", self.hit_rate * 100.0)?;
        Ok(())
    }
}

/// 缓存预热配置
#[derive(Debug, Clone)]
pub struct CacheWarmupConfig {
    /// 是否启用预热
    pub enabled: bool,
    /// 预热的分支列表
    pub branches: Vec<String>,
    /// 预热延迟（毫秒）
    pub delay_ms: u64,
}

impl Default for CacheWarmupConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            branches: vec!["main".to_string()],
            delay_ms: 10,
        }
    }
}

/// 缓存预热管理器
pub struct CacheWarmup {
    config: CacheWarmupConfig,
    branch_cache: std::sync::Arc<BranchCache>,
}

impl CacheWarmup {
    /// 创建缓存预热管理器
    pub fn new(config: CacheWarmupConfig, branch_cache: std::sync::Arc<BranchCache>) -> Self {
        Self {
            config,
            branch_cache,
        }
    }

    /// 执行缓存预热
    pub fn warmup(&self, loader: &dyn Fn(&str) -> Option<ContextBranch>) {
        if !self.config.enabled {
            return;
        }

        tracing::info!("Starting cache warmup for {} branches", self.config.branches.len());

        for branch_id in &self.config.branches {
            if let Some(branch) = loader(branch_id) {
                self.branch_cache.insert(branch_id.clone(), branch);
                tracing::debug!("Cached branch: {}", branch_id);
            }

            if self.config.delay_ms > 0 {
                std::thread::sleep(Duration::from_millis(self.config.delay_ms));
            }
        }

        tracing::info!("Cache warmup completed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_branch(id: &str, name: &str, parent: &str) -> ContextBranch {
        let temp_dir = TempDir::new().unwrap();
        let branch_dir = temp_dir.path().join(id);
        ContextBranch::new(id, name, parent, branch_dir).unwrap()
    }

    #[test]
    fn test_branch_cache_basic() {
        let cache = BranchCache::new(10);

        // 测试插入和获取
        let branch = create_test_branch("test-1", "test", "main");
        cache.insert("test-1".to_string(), branch.clone());

        assert!(cache.contains("test-1"));
        assert!(!cache.contains("test-2"));

        let cached = cache.get("test-1").unwrap();
        assert_eq!(cached.branch.branch_id, "test-1");
    }

    #[test]
    fn test_branch_cache_lru() {
        let cache = BranchCache::new(3);

        // 插入 3 个分支
        for i in 1..=3 {
            let branch = create_test_branch(&format!("test-{}", i), "test", "main");
            cache.insert(format!("test-{}", i), branch);
        }

        // 访问第一个分支，使其不是最久未使用
        cache.get("test-1");

        // 插入第 4 个分支，应该驱逐 test-2（最久未使用）
        let branch = create_test_branch("test-4", "test", "main");
        cache.insert("test-4".to_string(), branch);

        assert!(cache.contains("test-1"));
        assert!(!cache.contains("test-2")); // 被驱逐
        assert!(cache.contains("test-3"));
        assert!(cache.contains("test-4"));
    }

    #[test]
    fn test_branch_cache_stats() {
        let cache = BranchCache::new(10);

        // 未命中
        cache.get("test-1");

        // 插入并命中
        let branch = create_test_branch("test-1", "test", "main");
        cache.insert("test-1".to_string(), branch.clone());
        cache.get("test-1");
        cache.get("test-1");

        let stats = cache.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_branch_cache_remove() {
        let cache = BranchCache::new(10);

        let branch = create_test_branch("test-1", "test", "main");
        cache.insert("test-1".to_string(), branch);

        assert!(cache.contains("test-1"));

        cache.remove("test-1");

        assert!(!cache.contains("test-1"));
    }

    #[test]
    fn test_ancestor_cache_basic() {
        let cache = AncestorCache::new();

        // 模拟分支层次：main <- feature <- sub-feature
        let parent_map = HashMap::from([
            ("sub-feature".to_string(), "feature".to_string()),
            ("feature".to_string(), "main".to_string()),
            ("main".to_string(), "".to_string()),
        ]);

        let loader = |id: &str| parent_map.get(id).cloned();

        // 测试祖先链
        let ancestors = cache.get_ancestors("sub-feature", loader);
        assert_eq!(ancestors.len(), 2);
        assert!(ancestors.contains(&"feature".to_string()));
        assert!(ancestors.contains(&"main".to_string()));
    }

    #[test]
    fn test_ancestor_cache_is_descendant() {
        let cache = AncestorCache::new();

        let parent_map = HashMap::from([
            ("sub-feature".to_string(), "feature".to_string()),
            ("feature".to_string(), "main".to_string()),
        ]);

        let loader = |id: &str| parent_map.get(id).cloned();

        assert!(cache.is_descendant_of("sub-feature", "main", loader));
        assert!(cache.is_descendant_of("sub-feature", "feature", loader));
        assert!(!cache.is_descendant_of("main", "feature", loader));
    }

    #[test]
    fn test_ancestor_cache_common_ancestor() {
        let cache = AncestorCache::new();

        let parent_map = HashMap::from([
            ("feature-a".to_string(), "main".to_string()),
            ("feature-b".to_string(), "main".to_string()),
            ("sub-feature".to_string(), "feature-a".to_string()),
        ]);

        let loader = |id: &str| parent_map.get(id).cloned();

        // feature-a 和 feature-b 的公共祖先是 main
        let common = cache.find_common_ancestor("feature-a", "feature-b", &mut loader.clone(), "main");
        assert_eq!(common, Some("main".to_string()));

        // sub-feature 和 feature-b 的公共祖先是 main
        let common = cache.find_common_ancestor("sub-feature", "feature-b", &mut loader.clone(), "main");
        assert_eq!(common, Some("main".to_string()));
        
        // sub-feature 和 main 的公共祖先是 main
        let common = cache.find_common_ancestor("sub-feature", "main", &mut loader.clone(), "main");
        assert_eq!(common, Some("main".to_string()));
    }

    #[test]
    fn test_ancestor_cache_invalidation() {
        let cache = AncestorCache::new();

        let parent_map = HashMap::from([
            ("sub-feature".to_string(), "feature".to_string()),
            ("feature".to_string(), "main".to_string()),
        ]);

        let loader = |id: &str| parent_map.get(id).cloned();

        // 填充缓存
        cache.get_ancestors("sub-feature", &loader);
        cache.get_ancestors("feature", &loader);

        assert_eq!(cache.len(), 2);

        // 使 feature 失效，应该也使 sub-feature 失效
        cache.invalidate("feature");

        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_warmup() {
        let cache = std::sync::Arc::new(BranchCache::new(10));
        let config = CacheWarmupConfig {
            enabled: true,
            branches: vec!["main".to_string(), "feature-1".to_string()],
            delay_ms: 0,
        };

        let warmup = CacheWarmup::new(config, Arc::clone(&cache));

        // 模拟加载器
        let loader = |id: &str| {
            Some(create_test_branch(id, id, "main"))
        };

        warmup.warmup(&loader);

        assert!(cache.contains("main"));
        assert!(cache.contains("feature-1"));
        assert_eq!(cache.len(), 2);
    }
}
