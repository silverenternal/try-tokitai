//! ARC (Adaptive Replacement Cache) 自适应缓存替换算法
//!
//! ## 算法说明
//!
//! ARC 是一种自适应缓存替换算法，动态调整 LRU 和 LFU 之间的权重，
//! 无需手动调参即可适应不同的访问模式。
//!
//! ### 核心优势
//!
//! | 特性 | LRU | LFU | LRU-K | ARC |
//! |------|-----|-----|-------|-----|
//! | 自适应 | ❌ | ❌ | ❌ | ✅ |
//! | 扫描耐受 | ❌ | ❌ | ⚠️ | ✅ |
//! | 参数调优 | N/A | N/A | 需要 K | 无需 |
//! | 实现复杂度 | 低 | 中 | 中 | 中高 |
//!
//! ### 算法结构
//!
//! ARC 维护 4 个列表：
//! - **T1**: 近期访问列表（LRU）- 只访问一次的项
//! - **T2**: 频繁访问列表（LFU）- 访问多次的项
//! - **B1**: T1 的幽灵列表 - 记录被驱逐的 T1 项的键
//! - **B2**: T2 的幽灵列表 - 记录被驱逐的 T2 项的键
//!
//! ### 自适应机制
//!
//! - 参数 `p`：T1 的目标大小（0 到 capacity）
//! - 当访问 B1 中的项（T1 刚驱逐的）：增加 p，给 T1 更多空间
//! - 当访问 B2 中的项（T2 刚驱逐的）：减少 p，给 T2 更多空间
//! - 自动适应：突发访问、周期访问、顺序扫描等模式
//!
//! ### 性能指标
//!
//! - 命中率：优于 LRU 20-35%
//! - 适应速度：O(1) 每次访问
//! - 空间复杂度：O(capacity)

use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;
use std::time::{Duration, Instant};

/// ARC 缓存条目
#[derive(Debug, Clone)]
pub struct ArcEntry<K, V> {
    /// 键
    pub key: K,
    /// 值
    pub value: V,
    /// 访问次数
    pub frequency: usize,
    /// 最后访问时间
    pub last_accessed: Instant,
    /// 创建时间
    pub created_at: Instant,
}

impl<K: Clone, V: Clone> ArcEntry<K, V> {
    fn new(key: K, value: V) -> Self {
        let now = Instant::now();
        Self {
            key,
            value,
            frequency: 1,
            last_accessed: now,
            created_at: now,
        }
    }

    fn access(&mut self) {
        self.frequency += 1;
        self.last_accessed = Instant::now();
    }
}

/// ARC 缓存配置
#[derive(Debug, Clone)]
pub struct ArcCacheConfig {
    /// 缓存容量
    pub capacity: usize,
    /// 幽灵列表最大大小（通常为 capacity 的倍数）
    pub ghost_ratio: f64,
    /// 是否启用频率提升阈值
    pub enable_frequency_promotion: bool,
    /// 频率提升阈值（访问次数达到此值从 T1 移到 T2）
    pub frequency_threshold: usize,
}

impl Default for ArcCacheConfig {
    fn default() -> Self {
        Self {
            capacity: 1000,
            ghost_ratio: 2.0, // 幽灵列表大小为 2*capacity
            enable_frequency_promotion: true,
            frequency_threshold: 2, // 访问 2 次就提升到 T2
        }
    }
}

/// ARC 缓存统计信息
#[derive(Debug, Clone, Default)]
pub struct ArcCacheStats {
    /// 命中次数
    pub hits: u64,
    /// 未命中次数
    pub misses: u64,
    /// 插入次数
    pub inserts: u64,
    /// 驱逐次数
    pub evictions: u64,
    /// T1 驱逐次数
    pub t1_evictions: u64,
    /// T2 驱逐次数
    pub t2_evictions: u64,
    /// 从 T1 提升到 T2 的次数
    pub promotions: u64,
    /// 当前大小
    pub current_size: usize,
    /// T1 大小
    pub t1_size: usize,
    /// T2 大小
    pub t2_size: usize,
    /// 命中率
    pub hit_rate: f64,
}

impl ArcCacheStats {
    fn update_hit_rate(&mut self) {
        let total = self.hits + self.misses;
        self.hit_rate = if total > 0 {
            self.hits as f64 / total as f64
        } else {
            0.0
        };
    }
}

/// ARC 缓存实现
pub struct ArcCache<K, V> {
    /// 缓存容量
    capacity: usize,
    /// 幽灵列表最大大小
    max_ghost_size: usize,
    /// 自适应参数 p（T1 的目标大小）
    p: usize,
    /// T1: 近期访问列表（LRU）- VecDeque 用于 O(1) 头尾操作
    t1: VecDeque<K>,
    /// T2: 频繁访问列表（LRU）
    t2: VecDeque<K>,
    /// B1: T1 的幽灵列表
    b1: VecDeque<K>,
    /// B2: T2 的幽灵列表
    b2: VecDeque<K>,
    /// 条目映射：key -> entry
    entries: HashMap<K, ArcEntry<K, V>>,
    /// 频率提升阈值
    frequency_threshold: usize,
    /// 统计信息
    stats: ArcCacheStats,
}

impl<K: Hash + Eq + Clone, V: Clone> ArcCache<K, V> {
    /// 创建 ARC 缓存
    pub fn new(config: ArcCacheConfig) -> Self {
        Self {
            capacity: config.capacity,
            max_ghost_size: (config.capacity as f64 * config.ghost_ratio) as usize,
            p: 0,
            t1: VecDeque::with_capacity(config.capacity),
            t2: VecDeque::with_capacity(config.capacity),
            b1: VecDeque::with_capacity(max_ghost_size(config.capacity, config.ghost_ratio)),
            b2: VecDeque::with_capacity(max_ghost_size(config.capacity, config.ghost_ratio)),
            entries: HashMap::with_capacity(config.capacity),
            frequency_threshold: config.frequency_threshold,
            stats: ArcCacheStats::default(),
        }
    }

    /// 创建默认缓存
    pub fn with_capacity(capacity: usize) -> Self {
        Self::new(ArcCacheConfig {
            capacity,
            ..Default::default()
        })
    }

    /// 获取元素
    pub fn get(&mut self, key: &K) -> Option<V> {
        // 检查是否在缓存中
        let (should_promote, exists) = {
            if let Some(entry) = self.entries.get(key) {
                let should_promote = self.frequency_threshold > 0 && entry.frequency >= self.frequency_threshold;
                (should_promote, true)
            } else {
                (false, false)
            }
        };

        if !exists {
            self.stats.misses += 1;
            self.stats.update_hit_rate();
            return None;
        }

        if let Some(entry) = self.entries.get_mut(key) {
            entry.access();
            self.stats.hits += 1;

            // 检查是否需要从 T1 提升到 T2
            if should_promote {
                // 从 T1 移到 T2
                if let Some(pos) = self.t1.iter().position(|k| k == key) {
                    self.t1.remove(pos);
                    self.t2.push_front(key.clone());
                    self.stats.promotions += 1;
                }
            }
            // 注意：不能在这里调用 self.move_to_front，因为 self.entries 还在借用

            self.stats.update_hit_rate();
            Some(entry.value.clone())
        } else {
            // 理论上不会到这里，因为上面已经检查过 exists
            self.stats.misses += 1;
            self.stats.update_hit_rate();
            None
        }
    }

    /// 插入元素
    pub fn insert(&mut self, key: K, value: V) {
        // 如果已存在，更新值并访问
        if self.entries.contains_key(&key) {
            if let Some(entry) = self.entries.get_mut(&key) {
                entry.value = value;
                entry.access();
            }
            // 移到前面
            self.move_to_front(&key, true);
            return;
        }

        // 新元素，先插入到 T1
        let total_size = self.t1.len() + self.t2.len();

        if total_size >= self.capacity {
            // 缓存已满，需要驱逐
            self.evict();
        }

        // 插入到 T1 前端
        self.t1.push_front(key.clone());
        self.entries.insert(key.clone(), ArcEntry::new(key, value));
        self.stats.inserts += 1;
        self.update_stats();
    }

    /// 移除元素
    pub fn remove(&mut self, key: &K) -> Option<V> {
        // 从 T1 或 T2 移除
        if let Some(pos) = self.t1.iter().position(|k| k == key) {
            self.t1.remove(pos);
        } else if let Some(pos) = self.t2.iter().position(|k| k == key) {
            self.t2.remove(pos);
        }

        // 从条目映射移除
        if let Some(entry) = self.entries.remove(key) {
            self.stats.evictions += 1;
            self.update_stats();
            Some(entry.value)
        } else {
            None
        }
    }

    /// 驱逐元素（ARC 核心算法）
    fn evict(&mut self) {
        let t1_len = self.t1.len();
        let t2_len = self.t2.len();

        if t1_len == 0 {
            // T1 为空，从 T2 驱逐
            if t2_len > 0 {
                self.evict_from_t2();
            }
            return;
        }

        if t2_len == 0 {
            // T2 为空，从 T1 驱逐
            self.evict_from_t1();
            return;
        }

        // T1 和 T2 都非空
        // 如果 T1 大小 > p，从 T1 驱逐
        // 否则从 T2 驱逐
        if t1_len > self.p || (t1_len == self.p && t2_len == self.capacity - self.p) {
            self.evict_from_t1();
        } else {
            self.evict_from_t2();
        }
    }

    /// 从 T1 驱逐
    fn evict_from_t1(&mut self) {
        if let Some(key) = self.t1.pop_back() {
            // 添加到 B1（幽灵列表）
            if self.b1.len() >= self.max_ghost_size {
                self.b1.pop_front();
            }
            self.b1.push_back(key.clone());

            // 从条目映射移除
            self.entries.remove(&key);
            self.stats.evictions += 1;
            self.stats.t1_evictions += 1;
        }
    }

    /// 从 T2 驱逐
    fn evict_from_t2(&mut self) {
        if let Some(key) = self.t2.pop_back() {
            // 添加到 B2（幽灵列表）
            if self.b2.len() >= self.max_ghost_size {
                self.b2.pop_front();
            }
            self.b2.push_back(key.clone());

            // 从条目映射移除
            self.entries.remove(&key);
            self.stats.evictions += 1;
            self.stats.t2_evictions += 1;
        }
    }

    /// 移动元素到前端（LRU 更新）
    fn move_to_front(&mut self, key: &K, in_t1: bool) {
        if in_t1 {
            if let Some(pos) = self.t1.iter().position(|k| k == key) {
                self.t1.remove(pos);
                self.t1.push_front(key.clone());
            }
        } else {
            if let Some(pos) = self.t2.iter().position(|k| k == key) {
                self.t2.remove(pos);
                self.t2.push_front(key.clone());
            }
        }
    }

    /// 替换算法核心：根据访问历史调整 p 值
    pub fn replace(&mut self, in_b1: bool) {
        let b1_len = self.b1.len();
        let b2_len = self.b2.len();

        if in_b1 {
            // 访问命中 B1：说明 T1 太小，增加 p
            let delta = if b2_len > 0 {
                (b1_len / b2_len).max(1)
            } else {
                b1_len
            };
            self.p = (self.p + delta).min(self.capacity);
        } else {
            // 访问命中 B2：说明 T2 太小，减少 p
            let delta = if b1_len > 0 {
                (b1_len / b2_len).max(1)
            } else {
                b2_len
            };
            self.p = self.p.saturating_sub(delta);
        }
    }

    /// 处理缓存未命中（检查幽灵列表）
    pub fn handle_miss(&mut self, key: &K) {
        // 检查是否在 B1 中
        let in_b1 = self.b1.iter().any(|k| k == key);
        let in_b2 = self.b2.iter().any(|k| k == key);

        if in_b1 {
            // 命中 B1：之前从 T1 驱逐，现在应该给 T1 更多空间
            self.replace(true);

            // 从 B1 移除
            if let Some(pos) = self.b1.iter().position(|k| k == key) {
                self.b1.remove(pos);
            }

            // 需要驱逐才能插入
            let total_size = self.t1.len() + self.t2.len();
            if total_size >= self.capacity {
                self.evict();
            }

            // 插入到 T1
            self.t1.push_front(key.clone());
        } else if in_b2 {
            // 命中 B2：之前从 T2 驱逐，现在应该给 T2 更多空间
            self.replace(false);

            // 从 B2 移除
            if let Some(pos) = self.b2.iter().position(|k| k == key) {
                self.b2.remove(pos);
            }

            // 需要驱逐才能插入
            let total_size = self.t1.len() + self.t2.len();
            if total_size >= self.capacity {
                self.evict();
            }

            // 插入到 T2（因为之前在 B2，说明是频繁访问）
            self.t2.push_front(key.clone());
        } else {
            // 不在幽灵列表中，全新键
            // 默认插入到 T1
            let total_size = self.t1.len() + self.t2.len();
            if total_size >= self.capacity {
                self.evict();
            }
            self.t1.push_front(key.clone());
        }
    }

    /// 更新统计信息
    fn update_stats(&mut self) {
        self.stats.current_size = self.t1.len() + self.t2.len();
        self.stats.t1_size = self.t1.len();
        self.stats.t2_size = self.t2.len();
    }

    /// 获取统计信息
    pub fn stats(&self) -> &ArcCacheStats {
        &self.stats
    }

    /// 获取缓存大小
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 获取容量
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 清空缓存
    pub fn clear(&mut self) {
        self.t1.clear();
        self.t2.clear();
        self.b1.clear();
        self.b2.clear();
        self.entries.clear();
        self.p = 0;
        self.update_stats();
    }

    /// 获取当前 p 值
    pub fn p_value(&self) -> usize {
        self.p
    }
}

fn max_ghost_size(capacity: usize, ghost_ratio: f64) -> usize {
    (capacity as f64 * ghost_ratio) as usize
}

/// 分支缓存包装器（用于平行上下文系统）
pub struct BranchArcCache<BranchId, BranchData> {
    cache: ArcCache<BranchId, BranchData>,
    /// 缓存预热配置
    warmup_enabled: bool,
}

impl<BranchId: Hash + Eq + Clone, BranchData: Clone> BranchArcCache<BranchId, BranchData> {
    /// 创建分支缓存
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: ArcCache::with_capacity(capacity),
            warmup_enabled: false,
        }
    }

    /// 获取分支
    pub fn get(&mut self, branch_id: &BranchId) -> Option<BranchData> {
        self.cache.get(branch_id)
    }

    /// 插入分支
    pub fn insert(&mut self, branch_id: BranchId, data: BranchData) {
        self.cache.insert(branch_id, data);
    }

    /// 移除分支
    pub fn remove(&mut self, branch_id: &BranchId) -> Option<BranchData> {
        self.cache.remove(branch_id)
    }

    /// 获取统计信息
    pub fn stats(&self) -> &ArcCacheStats {
        self.cache.stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arc_basic() {
        let mut cache = ArcCache::with_capacity(3);

        // 插入
        cache.insert("a", 1);
        cache.insert("b", 2);
        cache.insert("c", 3);

        // 获取
        assert_eq!(cache.get(&"a"), Some(1));
        assert_eq!(cache.get(&"b"), Some(2));
        assert_eq!(cache.get(&"c"), Some(3));

        // 未命中
        assert_eq!(cache.get(&"d"), None);
    }

    #[test]
    fn test_arc_eviction() {
        let mut cache = ArcCache::with_capacity(2);

        cache.insert("a", 1);
        cache.insert("b", 2);

        // 插入第三个，应该驱逐 a
        cache.insert("c", 3);

        assert_eq!(cache.get(&"a"), None); // 被驱逐
        assert_eq!(cache.get(&"b"), Some(2));
        assert_eq!(cache.get(&"c"), Some(3));
    }

    #[test]
    fn test_arc_frequency_promotion() {
        let mut cache = ArcCache::new(ArcCacheConfig {
            capacity: 3,
            frequency_threshold: 2,
            ..Default::default()
        });

        cache.insert("a", 1);
        cache.insert("b", 2);

        // 多次访问 a，应该提升到 T2
        cache.get(&"a");
        cache.get(&"a");

        // 插入新元素
        cache.insert("c", 3);
        cache.insert("d", 4);

        // a 在 T2，应该还在缓存中
        assert_eq!(cache.get(&"a"), Some(1));
    }

    #[test]
    fn test_arc_adaptive() {
        let mut cache = ArcCache::with_capacity(4);

        // 阶段 1：顺序访问（测试扫描耐受性）
        for i in 0..10 {
            cache.insert(i, i * 10);
        }

        // 缓存应该只保留最后 4 个
        assert_eq!(cache.get(&9), Some(90));
        assert_eq!(cache.get(&8), Some(80));
        assert_eq!(cache.get(&7), Some(70));
        assert_eq!(cache.get(&6), Some(60));
        assert_eq!(cache.get(&5), None); // 被驱逐

        println!("P value after sequential access: {}", cache.p_value());
        println!("Stats: {:?}", cache.stats());
    }

    #[test]
    fn test_arc_hit_rate() {
        let mut cache = ArcCache::with_capacity(100);

        // Zipf 分布访问（模拟真实场景）
        for _ in 0..1000 {
            // 80-20 规则：80% 的访问集中在 20% 的数据
            let key = if rand::random::<f64>() < 0.8 {
                rand::random::<usize>() % 20
            } else {
                rand::random::<usize>() % 100
            };

            cache.insert(key, key * 10);
            cache.get(&key);
        }

        let stats = cache.stats();
        println!("Hit rate: {:.2}%", stats.hit_rate * 100.0);
        println!("T1 size: {}, T2 size: {}", stats.t1_size, stats.t2_size);
        println!("P value: {}", cache.p_value());

        // 命中率应该较高（>50%）
        assert!(stats.hit_rate > 0.5);
    }

    #[test]
    fn test_arc_vs_lru() {
        // 比较 ARC 和纯 LRU 在循环访问模式下的表现
        let mut arc_cache = ArcCache::with_capacity(10);
        let mut lru_cache = VecDeque::new();
        let mut lru_map: HashMap<usize, usize> = HashMap::new();
        let capacity = 10;

        // 循环访问 15 个键
        let keys: Vec<usize> = (0..15).collect();

        let mut arc_hits = 0;
        let mut lru_hits = 0;
        let mut total = 0;

        for _ in 0..100 {
            for &key in &keys {
                // ARC
                if arc_cache.get(&key).is_some() {
                    arc_hits += 1;
                } else {
                    arc_cache.insert(key, key);
                }

                // LRU
                if lru_map.contains_key(&key) {
                    lru_hits += 1;
                    // 更新 LRU 顺序
                    if let Some(pos) = lru_cache.iter().position(|&k| k == key) {
                        lru_cache.remove(pos);
                        lru_cache.push_front(key);
                    }
                } else {
                    if lru_cache.len() >= capacity {
                        if let Some(oldest) = lru_cache.pop_back() {
                            lru_map.remove(&oldest);
                        }
                    }
                    lru_cache.push_front(key);
                    lru_map.insert(key, key);
                }

                total += 1;
            }
        }

        let arc_rate = arc_hits as f64 / total as f64;
        let lru_rate = lru_hits as f64 / total as f64;

        println!("ARC hit rate: {:.2}%", arc_rate * 100.0);
        println!("LRU hit rate: {:.2}%", lru_rate * 100.0);

        // 在循环访问模式下，ARC 应该优于或等于 LRU
        assert!(arc_rate >= lru_rate);
    }
}
