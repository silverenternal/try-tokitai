# PROD-001: BlockCache 真正动态缩容设计方案

**状态**: 草案 (Draft)
**版本**: v0.4.0 规划
**创建日期**: 2026-04-14
**作者**: Phase 5 规划

---

## 1. 问题陈述

### 1.1 当前状况

当前 `BlockCache` 使用 Moka sync `Cache` 作为底层实现。Moka 的 `max_capacity` 在构造时固定，运行时不可变。这导致：

- `rebalance.rs` 的 `apply_block_shrink/grow` 只能调用 `apply_eviction_pressure()`（即 `run_pending_tasks()`），这仅能加速待决驱逐处理
- 无法真正将 BlockCache 的内存上限从 64MB 降低到 32MB（例如）
- 在 UnifiedCacheManager 的 rebalance 决策中，BlockCache 是"半响应"的：能驱逐但不能缩容

### 1.2 影响

| 场景 | 当前行为 | 期望行为 |
|------|----------|----------|
| Bloom 命中率 >80%, Block <30% | BlockCache 施加驱逐压力，但不缩容 | BlockCache 真正缩容，释放内存给 Bloom |
| 工作负载突变（写入密集 -> 读取密集） | BlockCache 保持原有上限 | BlockCache 动态扩容 |
| 内存紧张场景 | 无法主动释放 BlockCache 内存 | 可缩容到 min_budget |

---

## 2. 方案评估

### 方案 A: 替换 Moka 为自定义 LRU

**思路**: 完全弃用 Moka，实现一个线程安全的 LRU 缓存。

#### 优点
- 完全控制容量，可随时缩容/扩容
- 可定制 weigher 行为
- 无第三方依赖限制

#### 缺点
- **工作量大**: 需要实现并发安全的 LRU、淘汰策略、统计收集
- **性能风险**: Moka 的 Wineclock 算法经过大量优化，自研可能性能不如
- **维护负担**: 新增一个需要长期维护的核心组件
- **功能缺失**: 失去 Moka 的高级特性（expiration, async, eviction listener 等）

#### 技术设计

```rust
// 核心数据结构
struct CustomLruCache {
    max_bytes: AtomicU64,           // 运行时可变的容量上限
    current_bytes: AtomicU64,       // 当前使用字节数
    map: DashMap<String, CacheEntry>,  // 并发哈希表
    lru_list: Mutex<LruList>,       // LRU 双向链表
    stats: CacheStatsInner,
}

struct CacheEntry {
    key: String,
    value: Bytes,
    weight: u32,
    lru_node: *mut LruNode,         // 指向 LRU 链表节点
}

// 动态缩容核心方法
impl CustomLruCache {
    fn shrink_to(&self, target_bytes: u64) -> usize {
        let mut evicted = 0;
        while self.current_bytes.load(Relaxed) > target_bytes {
            if let Some(entry) = self.evict_lru() {
                self.current_bytes.fetch_sub(entry.weight, Relaxed);
                evicted += 1;
            } else {
                break; // No more entries to evict
            }
        }
        evicted
    }

    fn grow_to(&self, new_max: u64) {
        self.max_bytes.store(new_max, Relaxed);
    }
}
```

#### 预估工作量: **40-60 小时**

---

### 方案 B: 多实例 Moka，每个实例独立销毁重建 (推荐)

**思路**: 将 BlockCache 拆分为多个 Moka 子实例，每个子实例有独立容量。需要缩容时，销毁部分子实例并重建。

#### 优点
- **复用 Moka**: 保留 Wineclock 算法和高并发性能
- **增量实现**: 在现有架构上扩展，非推倒重来
- **风险可控**: 可灰度切换，出问题回退到 advisory mode

#### 缺点
- **复杂度**: 多实例管理增加代码复杂度
- **碎片化**: 子实例可能导致缓存碎片化（命中率轻微下降）
- **销毁延迟**: Moka 实例销毁需要等待后台线程完成

#### 技术设计

```
当前架构 (单实例):
  BlockCache
    └── Moka Cache (capacity: 64MB, 不可变)

目标架构 (多实例):
  BlockCache
    ├── Moka Shard-0 (capacity: 16MB)
    ├── Moka Shard-1 (capacity: 16MB)
    ├── Moka Shard-2 (capacity: 16MB)
    └── Moka Shard-3 (capacity: 16MB)
    Total: 64MB (4 shards x 16MB)

缩容操作 (64MB -> 32MB):
  BlockCache
    ├── Moka Shard-0 (capacity: 16MB)  ← 保留
    ├── Moka Shard-1 (capacity: 16MB)  ← 保留
    ├── Moka Shard-2 (drain & drop)    ← 销毁
    └── Moka Shard-3 (drain & drop)    ← 销毁
    Total: 32MB

扩容操作 (32MB -> 64MB):
  BlockCache
    ├── Moka Shard-0 (capacity: 16MB)  ← 已有
    ├── Moka Shard-1 (capacity: 16MB)  ← 已有
    ├── Moka Shard-2 (new: 16MB)       ← 新建
    └── Moka Shard-3 (new: 16MB)       ← 新建
    Total: 64MB
```

```rust
// 核心数据结构
struct BlockCache {
    shards: RwLock<Vec<Arc<MokaShard>>>,  // 可变数量的分片
    shard_size_bytes: u64,                 // 每个分片的大小
    stats: Arc<CacheStatsInner>,
    segment_index: RwLock<HashMap<u64, HashSet<String>>>,
}

struct MokaShard {
    cache: Cache<String, Bytes>,
    id: usize,
}

impl BlockCache {
    /// 缩容：销毁多余分片
    fn shrink_to(&self, target_bytes: u64) -> usize {
        let mut shards = self.shards.write();
        let current_total = shards.len() as u64 * self.shard_size_bytes;
        let target_shards = (target_bytes + self.shard_size_bytes - 1) / self.shard_size_bytes;

        let mut evicted = 0;
        while shards.len() > target_shards as usize {
            // 取出最后一个分片，排空后销毁
            if let Some(shard) = shards.pop() {
                evicted += shard.cache.weighted_size() as usize;
                // 注意: drop 会等待 Moka 后台线程完成
                drop(shard);
            }
        }
        evicted
    }

    /// 扩容：创建新分片
    fn grow_to(&self, target_bytes: u64) {
        let mut shards = self.shards.write();
        let target_shards = (target_bytes + self.shard_size_bytes - 1) / self.shard_size_bytes;

        while shards.len() < target_shards as usize {
            let id = shards.len();
            let shard = self.create_shard(id);
            shards.push(Arc::new(shard));
        }
    }

    /// Get 操作：遍历所有分片查找
    fn get_by_key(&self, key: &str) -> Option<Bytes> {
        let shards = self.shards.read();
        // 使用一致性哈希或简单的轮询查找
        // 这里简化为遍历所有分片
        for shard in shards.iter() {
            if let Some(value) = shard.cache.get(key) {
                return Some(value);
            }
        }
        None
    }
}
```

#### 预估工作量: **20-30 小时**

---

### 方案 C: 混合方案 - 自定义 LRU + Moka 共存

**思路**: 保留 Moka 用于热数据，自定义 LRU 用于温数据。动态缩容仅针对自定义 LRU 部分。

#### 优点
- 两全其美：Moka 热数据高性能 + 自定义 LRU 可缩容
- 渐进式：可以先实现方案 B，再演进到方案 C

#### 缺点
- 最复杂的设计，维护两套缓存逻辑
- 需要额外的迁移逻辑

#### 预估工作量: **50-70 小时**

---

## 3. 推荐方案: 方案 B（多实例 Moka）

### 3.1 推荐理由

1. **工作量可控**: 20-30 小时，比方案 A/C 少 50%
2. **风险最低**: 保留 Moka 核心能力，仅增加分片管理层
3. **可回退**: 如果分片管理出现问题，可回退到单实例 + advisory mode
4. **渐进式**: 后续可按需升级到方案 C

### 3.2 实现阶段

#### Phase B1: 基础分片架构 (8-10h)
- [ ] 设计 `MokaShard` 结构
- [ ] 实现分片创建/销毁
- [ ] 实现 get/insert 跨分片操作
- [ ] 单元测试：基本 CRUD

#### Phase B2: 动态缩容/扩容 (8-10h)
- [ ] 实现 `shrink_to()` 方法
- [ ] 实现 `grow_to()` 方法
- [ ] 实现分片间数据迁移（可选优化）
- [ ] 单元测试：缩容驱逐、扩容新建

#### Phase B3: 接入 rebalance (4-6h)
- [ ] 修改 `UnifiedCacheManager.apply_block_shrink/grow` 调用新方法
- [ ] 集成测试：rebalance 触发真实缩容
- [ ] 指标：记录缩容事件和驱逐数量

#### Phase B4: 性能验证与优化 (4-6h)
- [ ] 基准测试：单分片 vs 多分片性能对比
- [ ] 优化：减少 get 时的分片遍历开销
- [ ] 文档：更新 API 文档和运维手册

### 3.3 关键设计决策

| 决策点 | 选择 | 理由 |
|--------|------|------|
| 分片大小 | 16MB (固定) | 平衡碎片化和管理开销 |
| 分片数量 | 4 (默认) | 64MB / 16MB = 4 分片 |
| Get 策略 | 遍历所有分片 | 简单可靠，分片数少时性能可接受 |
| Insert 策略 | 一致性哈希分配 | 减少遍历，均匀分布 |
| 销毁等待 | 阻塞等待 Moka 后台线程 | 保证内存真正释放 |

### 3.4 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| Moka 实例销毁延迟 | 缩容后内存未立即释放 | 调用 `cache.invalidate_all()` + `run_pending_tasks()` 后再 drop |
| 多分片 get 性能下降 | 命中率轻微下降（~5%） | 使用一致性哈希减少查找范围；分片数量少时影响可忽略 |
| 分片间数据倾斜 | 某些分片过热 | 一致性哈希 + 定期重平衡 |
| 并发安全 | 分片列表读写竞争 | RwLock 读多写少，竞争极低 |

---

## 4. 与现有代码的兼容

### 4.1 公共 API 不变

```rust
// 现有 API 保持不变
impl BlockCache {
    pub fn get(&self, segment_id: u64, offset: u64) -> Option<Bytes>;
    pub fn get_by_key(&self, key: &str) -> Option<Bytes>;
    pub fn insert(&self, key: String, value: Bytes);
    pub fn put(&self, segment_id: u64, offset: u64, value: Bytes);
    pub fn invalidate_by_segment(&self, segment_id: u64);
    pub fn stats(&self) -> CacheStats;
    pub fn memory_usage(&self) -> u64;
    // 新增
    pub fn shrink_to(&self, target_bytes: u64) -> usize;
    pub fn grow_to(&self, target_bytes: u64);
}
```

### 4.2 rebalance.rs 集成

```rust
// 修改 UnifiedCacheManager 中的执行方法
impl UnifiedCacheManager {
    fn apply_block_shrink(&self, bytes: u64) {
        let target = current_memory.saturating_sub(bytes);
        let evicted = self.block_cache.shrink_to(target);
        tracing::info!(evicted, bytes_freed = bytes, "BlockCache shrunk");
    }

    fn apply_block_grow(&self, bytes: u64) {
        let target = current_memory.saturating_add(bytes);
        self.block_cache.grow_to(target);
        tracing::info!(new_capacity = target, "BlockCache grown");
    }
}
```

---

## 5. 测试计划

| 测试类别 | 测试用例 | 预期 |
|----------|----------|------|
| 功能测试 | 创建多分片 BlockCache | 所有分片正常创建 |
| 功能测试 | shrink_to 缩容 | 多余分片被销毁，内存释放 |
| 功能测试 | grow_to 扩容 | 新分片被创建，容量增加 |
| 功能测试 | 缩容后 get/insert | 操作正常，数据不丢失 |
| 功能测试 | 扩容后 get/insert | 操作正常 |
| 并发测试 | 32 线程并发 get/insert + 缩容 | 无 panic，数据一致 |
| 性能测试 | 单分片 vs 4 分片 QPS | 性能下降 <10% |
| 集成测试 | rebalance 触发缩容 | 实际内存减少，Bloom 内存增加 |

---

## 6. 后续演进

- **v0.4.0**: 完成方案 B 实现，BlockCache 支持真正动态缩容
- **v0.5.0**: 评估是否需要方案 C（热/温双层），进一步优化内存效率
- **v1.0.0**: 稳定分片架构，提供配置选项（分片数量、分片大小）

---

## 7. 参考文档

- [Moka GitHub](https://github.com/moka-rs/moka) - Wineclock 算法文档
- [CHANGELOG.md v0.3.0](../../CHANGELOG.md) - FIX-003 描述了当前 advisory mode 限制
- [POSITION_AND_STATUS.md](../../doc/filekv/POSITION_AND_STATUS.md) - PROD-001 在路线图中的位置
- `src/cache/block_cache.rs` - 当前 BlockCache 实现
- `src/cache/rebalance.rs` - 当前 rebalance 决策逻辑
