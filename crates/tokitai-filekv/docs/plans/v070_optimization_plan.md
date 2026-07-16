# v0.7.0 优化规划

> **版本**: v0.7.0
> **日期**: 2026-04-14
> **前置版本**: v0.6.0 (GlobalKeyIndex + 批量 WAL + Compaction 优化)
> **架构**: LSM-Tree KV 存储引擎

---

## 1. 当前状态分析

### 1.1 架构优势

| 特性 | 状态 | 说明 |
|------|------|------|
| **Engine 解耦** | 已完成 | ReadEngine / WriteEngine / CompactionEngine / LifecycleManager 职责清晰 |
| **GlobalKeyIndex** | 已完成 | BTreeMap 实现 O(log n) 点查，含 stale segment 追踪 |
| **Compaction** | 已完成 | Streaming Merge Iterator, Leveled Compaction (L0-L3), 异步后台线程 |
| **缓存体系** | 已完成 | UnifiedCacheManager + BlockCache (Sharded Moka) + BloomFilterCache + 自适应 Bloom (INNO-001) + 顺序预取 (INNO-002) |
| **Bloom Filter** | 已完成 | 三层自适应缓存 (L1/L2/L3) + 迁移控制器 + FPR 控制器 |
| **Zone Map** | 已完成 | Block-level pruning + 顺序预取 |
| **WAL** | 已完成 | 批量写入 (batch WAL) + 三种同步模式 (Immediate/Batch/Lazy) |
| **Write Coalescer** | 已完成 | 写合并优化 |
| **压缩** | 已完成 | Zstd 压缩 + 字典训练 (配置化) |
| **Checkpoint** | 已完成 | 增量 Checkpoint 体系 |
| **审计日志** | 已完成 | AuditLogger |
| **Metrics** | 已完成 | Prometheus exporter (feature-gated) |
| **Feature Flags** | 已完成 | FeatureFlagController 运行时控制 |
| **I/O 抽象** | 已完成 | FileKVFileSystem trait + MemFs + FaultInjector |
| **Backpressure** | 已完成 | MemTable 内存限制 + 背压 |
| **预分配** | 已完成 | AdaptivePreallocator |

### 1.2 已知瓶颈

#### 读取路径瓶颈

1. **GlobalKeyIndex 不完整**: v0.6.0 中 GlobalKeyIndex 创建为空的 (`GlobalKeyIndex::new()`)，flush_memtable 和 compaction 后没有主动将 key 写入索引。get() 路径中 GlobalIndex 命中后还有一个 `eprintln!` debug 输出（生产环境应移除）。
2. **BlockCache 全量遍历**: `get_by_key()` 遍历所有 Moka shards 查找 key，最坏情况 O(num_shards)。没有 key -> shard 的直接映射。
3. **冷缓存场景**: 冷缓存 get() 需要遍历所有 L0 段文件 + Bloom 加载 + 稀疏索引查询，延迟高。
4. **Bloom Filter 加载延迟**: 每次 segment 首次访问需要从磁盘加载 Bloom Filter（V2 格式仍需重建），未预热时每个 segment 都有加载开销。

#### 写入路径瓶颈

1. **MemTable flush 阻塞写**: `flush_memtable()` 使用 `Mutex<()>` 锁，flush 期间所有写入被阻塞。
2. **flush_memtable 无排序优化**: 使用 `get_entries()` 而非 `entries_sorted()`，段文件内 key 无序影响后续 compaction 效率。
3. **大 value 写入**: value > 1KB 时，WAL 和 memtable 的内存占用线性增长，无 value 分离存储策略。
4. **Compaction 全局索引更新**: `remove_segments()` + `bulk_insert()` 两步操作之间存在窗口期，全局索引可能短暂不一致。

#### 内存瓶颈

1. **GlobalKeyIndex 内存占用**: 10M keys 场景约 800MB-1.2GB（每 key ~80-120 bytes），无持久化/恢复机制，重启后需重建。
2. **BlockCache 无按 key 查找**: `get_by_key()` 遍历所有 shards，无法针对特定 key 做 LRU 淘汰。
3. **DenseIndex 全内存**: `dense_index_enabled=true` 时每个 entry ~20 bytes，10M keys 约 200MB。
4. **MemTable size 估算偏差**: DashMap 桶、分片结构等底层分配未计入 `size_bytes`，实际内存可能高 10-20%。

#### 并发瓶颈

1. **flush_lock 串行化**: 所有 flush 操作通过 `Mutex<()>` 串行执行。
2. **GlobalIndex write lock**: `insert()` / `remove()` / `batch_update()` 使用 `RwLock::write()`，高并发写入时竞争。
3. **Compaction manifest 同步**: manifest 文件写入是串行的。

### 1.3 缺失功能

| 功能 | 优先级 | 说明 |
|------|--------|------|
| **MVCC / 快照** | P1 | 无多版本并发控制，无隔离级别支持 |
| **事务支持** | P1 | 无 ACID 事务（仅有 batch 原子写入） |
| **压缩算法选择** | P1 | 仅支持 Zstd，缺少 Snappy/LZ4 等更快选项 |
| **备份/恢复工具** | P2 | 有 Checkpoint 但无完整备份恢复 CLI |
| **大 value 优化** | P2 | value 与 key 同存 segment 文件，无单独存储策略 |
| **GlobalKeyIndex 持久化** | P0 | 索引重启后需重建，10M keys 重建耗时长 |
| **BlockCache key 直查** | P0 | 无 key -> shard 映射，需遍历 |

---

## 2. v0.7.0 优先级任务列表

### P0 任务

#### T-001: GlobalKeyIndex 持久化与恢复

| 字段 | 内容 |
|------|------|
| **标题** | GlobalKeyIndex 持久化与启动恢复 |
| **描述** | 将 GlobalKeyIndex 的 BTreeMap 序列化到磁盘文件（index_dir/global_index.bin），启动时优先从磁盘加载，避免 10M keys 场景下全量扫描 segment 重建。采用 bincode 或自定义二进制格式。 |
| **预期收益** | 启动时间从分钟级降至秒级；避免冷启动期间 get() 性能退化 |
| **估计工作量** | 2-3 天 |
| **验收标准** | 1. 支持持久化到 disk 2. 启动时自动加载 3. 加载失败时降级重建 4. 10M keys 场景启动 < 5s |
| **AI 执行提示词** | "实现 GlobalKeyIndex 的持久化与恢复功能。在 src/core/global_index.rs 中添加 `save_to_disk()` 和 `load_from_disk()` 方法，使用 bincode 序列化 BTreeMap。在 FileKV::open() 流程中，优先尝试加载 global_index.bin，失败时降级到 rebuild_from_segments。在 flush_memtable 和 compaction 完成后异步持久化。确保向后兼容。" |

#### T-002: BlockCache key 直查优化

| 字段 | 内容 |
|------|------|
| **标题** | BlockCache key -> shard 直接映射 |
| **描述** | 当前 `get_by_key()` 遍历所有 Moka shards。改为使用 key 的哈希值直接计算 shard 索引，实现 O(1) 查找。同时维护 key -> shard_id 的轻量级映射（或使用一致的哈希策略）。 |
| **预期收益** | BlockCache get_by_key 延迟降低 50-80%（减少遍历开销）；BlockCache 命中率提升 5-10% |
| **估计工作量** | 1-2 天 |
| **验收标准** | 1. get_by_key 不再遍历 shards 2. insert_by_key 和 get_by_key 使用相同 shard 路由 3. 现有测试全部通过 4. benchmark 确认延迟降低 |
| **AI 执行提示词** | "优化 BlockCache 的 get_by_key 方法。当前实现遍历所有 shards 查找 key，改为使用 FxHash 对 key 做哈希取模直接路由到目标 shard。确保 insert_by_key 和 get_by_key 使用相同的哈希路由策略。更新 CACHE-003 的 segment_index 也要跟随 shard 拆分。保持 segment_id:offset 格式的 key 解析逻辑。" |

#### T-003: GlobalKeyIndex 在 flush/compaction 中的正确维护

| 字段 | 内容 |
|------|------|
| **标题** | 修复 GlobalKeyIndex 写入路径集成 |
| **描述** | 当前 GlobalKeyIndex 在 `lib.rs` 中创建后为空，flush_memtable 和 compaction 完成后未将新 key 写入索引。需要在以下路径维护索引：(1) flush_memtable 后添加 segment key 到索引 (2) compaction 后更新 key 位置 (3) delete 后从索引移除 key。同时移除生产环境的 `eprintln!` 调试输出。 |
| **预期收益** | GlobalKeyIndex 真正生效，点查命中率从 ~0% 提升至 >90%（热数据场景） |
| **估计工作量** | 2-3 天 |
| **验收标准** | 1. flush 后新 key 可在 global index 查询到 2. compaction 后 key 位置正确更新 3. delete 后 key 从索引移除 4. 移除所有 eprintln! 5. benchmark 显示点查性能提升 |
| **AI 执行提示词** | "修复 GlobalKeyIndex 在写入路径中的集成。在 WriteEngine::flush_memtable() 中，flush 完成后将新 segment 的 key 批量插入 global index。在 CompactionEngine 中，compaction 完成后调用 global_index.update_after_compaction()。在 WriteEngine::delete() 中调用 global_index.remove()。移除 ReadEngine::get() 中的所有 eprintln! 调试输出。添加单元测试验证三个场景。" |

#### T-004: 混合负载优化 (70% 读 + 30% 写)

| 字段 | 内容 |
|------|------|
| **标题** | 读写混合场景性能优化 |
| **描述** | 针对 70% 读 + 30% 写的混合负载场景，实施以下优化：(1) 读取路径减少锁竞争 (2) 写入路径异步化 (3) Compaction 触发策略针对混合负载调优 (4) 缓存预热策略优化 |
| **预期收益** | 混合负载 QPS 提升 30-50%；p99 读取延迟降低 20-40% |
| **估计工作量** | 3-4 天 |
| **验收标准** | 1. 混合 workload benchmark QPS > 200K ops/sec 2. p99 读取 < 200us 3. 写放大率保持 < 2x |
| **AI 执行提示词** | "优化 FileKV 的混合读写负载。分析 benches/07_professional_benchmark.rs 中的 bench_mixed_workload 测试，识别性能瓶颈。优化方向：(1) MemTable flush 改为异步非阻塞模式 (2) BlockCache 增加并发优化 (3) 调整 CompactionConfig 的 l0_file_count_threshold 和 level_size_multiplier 针对混合负载 (4) 确保 get() 路径充分利用 BlockCache 和 GlobalKeyIndex。运行 benchmark 对比优化前后结果。" |

### P1 任务

#### T-005: 压缩算法扩展 (Snappy / LZ4)

| 字段 | 内容 |
|------|------|
| **标题** | 支持多压缩算法选择 |
| **描述** | 当前仅支持 Zstd。添加 Snappy (低延迟) 和 LZ4 (极高吞吐) 作为可选压缩算法。通过 BlockCompressionConfig.mode 配置。创建 CompressionStrategy trait 支持扩展。 |
| **预期收益** | 不同场景可选择最合适的压缩：延迟敏感用 Snappy/LZ4，存储敏感用 Zstd。压缩延迟降低 30-60%（使用 Snappy 时） |
| **估计工作量** | 2-3 天 |
| **验收标准** | 1. BlockCompressionMode 增加 Snappy / Lz4 变体 2. 通过 config 可配置 3. benchmark 对比各算法性能 4. 向后兼容 |
| **AI 执行提示词** | "扩展压缩算法支持。在 src/compression/ 下创建 strategy.rs，定义 CompressionStrategy trait。实现 ZstdCompressor、SnappyCompressor、Lz4Compressor。修改 BlockCompressionMode 枚举增加 Snappy / Lz4 变体。在 SegmentFile 的写入/读取路径中使用配置的压缩算法。添加 benchmark 对比三种算法的压缩比和性能。添加 snap 和 lz4 crate 依赖。" |

#### T-006: 快照 / 隔离级别支持

| 字段 | 内容 |
|------|------|
| **标题** | MVCC 快照与读隔离 |
| **描述** | 基于现有 seq_num 机制实现 MVCC：(1) 写入时递增全局 seq_num (2) 快照捕获当前 seq_num (3) 读取时只看到 snapshot_seq_num 之前的版本 (4) 实现 Read Committed 和 Repeatable Read 隔离级别 |
| **预期收益** | 支持并发读写隔离，避免脏读和不可重复读；为后续事务支持奠定基础 |
| **估计工作量** | 4-5 天 |
| **验收标准** | 1. Snapshot API: kv.snapshot(seq_num) 返回快照视图 2. Read Committed: 每次读看到已提交数据 3. Repeatable Read: 快照内多次读一致 4. 不影响现有单线程性能 |
| **AI 执行提示词** | "实现 MVCC 快照支持。方案：(1) 在 MemTable 中保留多版本值（使用 seq_num 区分）(2) segment 文件中 value 已隐含版本号（写入时记录）(3) 实现 Snapshot 结构体，捕获 seq_num (4) Snapshot::get() 只返回 seq_num <= snapshot_seq 的版本 (5) 实现 ReadCommitted 和 RepeatableRead 两种隔离级别。先设计数据结构和 API，再实现核心逻辑，最后添加测试。" |

#### T-007: MemTable 异步 flush

| 字段 | 内容 |
|------|------|
| **标题** | 非阻塞 MemTable 刷盘 |
| **描述** | 当前 flush_memtable() 通过 `Mutex<()>` 串行阻塞所有写入。改为异步模式：(1) 交换 MemTable (swap current with empty new) (2) 后台线程 flush old MemTable (3) 写入不阻塞 |
| **预期收益** | 写入延迟降低 40-60%（消除 flush 阻塞）；写入吞吐提升 20-30% |
| **估计工作量** | 3-4 天 |
| **验收标准** | 1. flush 期间 put() 不阻塞 2. 数据一致性保证 (WAL + flush 原子性) 3. 现有测试全部通过 4. benchmark 确认延迟降低 |
| **AI 执行提示词** | "实现 MemTable 异步 flush。当前 WriteEngine::flush_memtable() 使用 flush_lock: Mutex<()> 串行化。改为：(1) 使用 ArcSwap<MemTable> 实现双缓冲 (2) put() 写入当前 MemTable (3) flush 时 atomically swap current with fresh MemTable (4) 后台线程 flush old MemTable 到 segment (5) flush 完成后更新 sparse index 和 global index。确保 WAL 一致性。参考 RocksDB 的 memtable switch 设计。" |

#### T-008: 启动时 Cache 预热优化

| 字段 | 内容 |
|------|------|
| **标题** | 智能 Cache 预热策略 |
| **描述** | 当前 CacheWarmer 简单遍历所有 segment。优化为：(1) 优先预热 GlobalKeyIndex 中最近访问的 key (2) 使用 ZoneMap 信息优先热块高频 range (3) Bloom Filter 启动时并行加载 (4) 预热可配置策略 (全量 / 高频 / 自定义) |
| **预期收益** | 冷启动后首个请求延迟降低 50-70%；Bloom Filter 冷缓存场景消除首次加载开销 |
| **估计工作量** | 2-3 天 |
| **验收标准** | 1. 支持多种预热策略 2. 并行 Bloom 加载 3. 预热进度可观测 4. benchmark 确认冷启动延迟降低 |
| **AI 执行提示词** | "优化 CacheWarmer 的预热策略。当前实现在 src/cache/warmup.rs 中简单遍历。改进：(1) 添加 WarmingStrategy 枚举 (Full / HotKeys / RecentRanges) (2) 支持从 GlobalKeyIndex 获取热 key 列表 (3) Bloom Filter 并行加载 (使用 rayon) (4) 预热进度通过 CacheWarmingStats 暴露 (5) 配置化预热策略。保持向后兼容的默认行为。" |

### P2 任务

#### T-009: 大 Value 优化

| 字段 | 内容 |
|------|------|
| **标题** | 大 Value 分离存储 |
| **描述** | 当 value > 阈值（如 4KB）时，将 value 单独存储到 overflow 文件，segment 文件中仅存储指针。读取时按需加载大 value。 |
| **预期收益** | 大 value 场景写入吞吐提升 30-50%；内存占用降低；compaction 效率提升 |
| **估计工作量** | 3-4 天 |
| **验收标准** | 1. value > 阈值时自动分离存储 2. get() 透明解析 3. compaction 正确处理 overflow 文件 4. 配置化阈值 |
| **AI 执行提示词** | "实现大 value 分离存储。当 value 大小超过 config.large_value_threshold (默认 4KB) 时：(1) 写入时将 value 存到 overflow_dir/value_{segment_id}_{offset}.bin (2) segment 文件中写入 LargeValuePointer { overflow_path, size, checksum } (3) get() 检测到大 value 时从 overflow 文件读取 (4) compaction 时合并 overflow 文件 (5) delete 时清理 overflow 文件。确保对 get/put API 透明。" |

#### T-010: 备份 / 恢复工具

| 字段 | 内容 |
|------|------|
| **标题** | 完整备份与恢复 CLI |
| **描述** | 基于现有 IncrementalCheckpoint 体系，构建完整的备份/恢复工具链：(1) 全量备份 (2) 增量备份 (3) 点对点恢复 (4) 备份校验 |
| **预期收益** | 生产环境可用；支持定时备份和灾难恢复 |
| **估计工作量** | 3-4 天 |
| **验收标准** | 1. CLI 工具支持 backup / restore / verify 命令 2. 支持全量和增量模式 3. 恢复后数据一致性校验 4. 集成测试验证 |
| **AI 执行提示词** | "构建备份恢复 CLI 工具。基于 src/checkpoint/ 中的 IncrementalCheckpointManager：(1) 创建 src/ops/backup_tool.rs (2) 实现 backup() 全量备份 segment + index + bloom 文件 (3) 实现 restore() 从备份恢复 (4) 实现 verify() 校验备份完整性 (5) 提供简单 CLI 接口。使用 tar 或直接文件复制。确保恢复后 FileKV::open() 正常工作。" |

#### T-011: DenseIndex 序列化与按需加载

| 字段 | 内容 |
|------|------|
| **标题** | DenseIndex 持久化 |
| **描述** | DenseIndex 当前仅在内存中，重启后重建。实现序列化到磁盘，启动时按需加载。对于大 segment，支持懒加载（仅加载被访问的 block 的索引）。 |
| **预期收益** | 大 segment 启动时间降低；内存占用降低 50-70%（懒加载场景） |
| **估计工作量** | 2-3 天 |
| **验收标准** | 1. DenseIndex 可序列化到 segment 文件尾部或独立文件 2. 启动时按需加载 3. 支持懒加载模式 4. 不影响已有性能 |
| **AI 执行提示词** | "实现 DenseIndex 的持久化与按需加载。在 src/core/sparse_index.rs 中：(1) 为 DenseIndex 添加 save_to_file() 和 load_from_file() 方法 (2) 在 flush/compaction 写段时序列化 dense index 到 .idx 文件 (3) SegmentFile::open() 时不立即加载，而是在 get_by_key() 时按需加载 (4) 支持配置 eager_load (全量加载) vs lazy_load (按需加载)。" |

---

## 3. 具体优化建议

### 3.1 P0: 读取性能优化

#### BlockCache 命中率提升
- **问题**: 当前 BlockCache `get_by_key()` 遍历所有 shards，且缺乏有效的淘汰策略
- **方案**: 实施 T-002 (key 直查) + 改进 weigher 函数（考虑访问频率） + 预热高频 key
- **预期**: 命中率从当前水平提升 10-15%

#### 预取机制优化
- **问题**: SequentialPrefetcher 当前仅基于顺序访问模式，未利用 ZoneMap 信息
- **方案**: 当 ZoneMap 检测到范围查询时，主动预取相邻 blocks；GlobalKeyIndex 命中后预取同 segment 相邻 key
- **预期**: 范围查询吞吐量提升 30-50%

#### 冷缓存优化
- **问题**: 冷缓存 get() 需要遍历所有 segments + 加载 Bloom Filter
- **方案**: 实施 T-008 (Cache 预热) + 并行 Bloom 加载 + GlobalKeyIndex 直接路由
- **预期**: 冷缓存首个请求延迟降低 50%

### 3.2 P0: 混合负载优化

#### 读写路径解耦
- **问题**: flush 期间写被阻塞
- **方案**: 实施 T-007 (异步 flush) + 双 MemTable 缓冲
- **预期**: 写入延迟降低 40-60%

#### Compaction 触发策略调优
- **问题**: 当前 l0_file_count_threshold=4 在混合负载下可能触发过频繁
- **方案**: 自适应触发：根据读写比例动态调整 threshold（读多时降低 threshold 加快 compaction，写多时提高 threshold 减少 compaction 开销）
- **预期**: 混合负载下 WA 降低 10-20%

### 3.3 P1: 压缩算法支持

#### 架构设计
```
CompressionStrategy (trait)
├── ZstdCompressor (level 1-22, current default at level 3)
├── SnappyCompressor (low latency, ~2-3x compression ratio)
└── Lz4Compressor (highest throughput, ~2x compression ratio)

BlockCompressionConfig {
    mode: BlockCompressionMode::None | Zstd | Snappy | Lz4,
    compression_level: i32,
    min_compress_size: u64,  // blocks smaller than this skip compression
}
```

#### 选择建议
| 场景 | 推荐算法 | 原因 |
|------|----------|------|
| 延迟敏感 | Snappy / LZ4 | 压缩/解压延迟最低 |
| 存储敏感 | Zstd level 3-5 | 压缩比最高 (~3-5x) |
| 混合场景 | LZ4 | 吞吐与延迟平衡 |

### 3.4 P1: 快照 / 隔离级别支持

#### 设计要点
```
Snapshot {
    seq_num: u64,           // 快照捕获的序列号
    isolation_level: IsolationLevel,
}

IsolationLevel {
    ReadCommitted,          // 每次读看到已提交数据
    RepeatableRead,         // 快照内多次读一致
    Serializable,           // (预留) 完全隔离
}
```

#### 实现策略
1. **MemTable**: 保留多版本值（当前 value 指针 + 历史版本列表）
2. **Segment**: 写入时记录 seq_num（已有此机制）
3. **Snapshot::get()**: 读取时过滤 seq_num > snapshot_seq 的版本
4. **版本清理**: Compaction 时清理无引用历史版本

### 3.5 P2: 大 Value 优化

#### 设计要点
```
Segment Entry:
├── small value (<= 4KB): 直接存储在 segment 文件中
└── large value (> 4KB):
    └── LargeValuePointer {
        overflow_file_id: u64,
        offset: u64,
        size: u64,
        checksum: u32,
    }
```

- **写入路径**: 检测 value 大小，超过阈值写入 overflow 文件
- **读取路径**: 检测 pointer 类型，大 value 从 overflow 文件读取
- **Compaction**: 合并 overflow 文件（类似 segment merge）
- **清理**: delete/compaction 时清理无用 overflow 文件

### 3.6 P2: 备份 / 恢复工具

#### 设计要点
```
Backup {
    full_backup: {
        segments/          // 所有 segment 文件
        index/             // sparse index, dense index, global index
        bloom/             // bloom filter files
        metadata.json      // 备份元数据 (时间戳, 版本, 校验和)
    }
    incremental_backup: {
        changes_since_seq/ // 自上次备份以来的变更
    }
}
```

- **全量备份**: 复制所有数据文件 + 生成校验和
- **增量备份**: 基于 seq_num 导出变更
- **恢复**: 复制到目标目录 + 校验完整性
- **CLI**: `filekv backup`, `filekv restore`, `filekv verify`

---

## 4. 性能目标

### 4.1 总体目标

| 指标 | v0.6.0 基线 | v0.7.0 目标 | 说明 |
|------|-------------|-------------|------|
| 10M 顺序写入 | 357K ops/sec | 400K+ ops/sec | +12% 提升 |
| 热缓存点查 | ~62 us | < 20 us | GlobalKeyIndex + BlockCache 优化 |
| 冷缓存点查 | N/A | < 500 us (p99) | 预热 + 并行 Bloom |
| 混合负载 (70R/30W) | N/A | 250K+ ops/sec | 新增场景 |
| 写放大率 (WA) | 1.00x | < 1.5x | 异步 flush 可能增加 |
| 空间放大率 (SA) | 1.24x | < 1.3x | 大 value 分离可能增加 |
| vs RocksDB 差距 | 1.4x-2.8x | < 1.5x | 综合对比 |

### 4.2 读取延迟目标

| 场景 | v0.6.0 | v0.7.0 目标 |
|------|--------|-------------|
| MemTable 命中 | < 1 us | < 1 us (保持) |
| BlockCache 命中 | ~5-10 us | < 3 us (key 直查) |
| GlobalKeyIndex 命中 | N/A (未集成) | < 15 us |
| Bloom 负向 | ~1-2 ms | < 500 us (预热) |
| 磁盘命中 (L0) | ~1-5 ms | < 1 ms (ZoneMap + 预取) |
| 磁盘未命中 | ~5-10 ms | < 2 ms (Bloom 过滤) |

### 4.3 混合负载目标

| 指标 | 目标值 | 测量方法 |
|------|--------|----------|
| 整体 QPS | > 250K ops/sec | bench_mixed_workload (10M ops) |
| 读取 p99 | < 200 us | 延迟分布 |
| 读取 p999 | < 500 us | 延迟分布 |
| 写入 p99 | < 5 ms | 延迟分布 |
| WA | < 2.0x | total_disk_write / user_write |
| SA | < 1.5x | disk_size / logical_size |

### 4.4 内存使用目标

| 组件 | v0.6.0 估算 | v0.7.0 目标 | 说明 |
|------|-------------|-------------|------|
| GlobalKeyIndex (10M) | N/A (空) | < 800MB | 持久化后可选加载 |
| BlockCache | 64MB-4GB (可配置) | 同左 | key 直查优化 |
| MemTable | 64MB (max) | 同左 | 异步 flush |
| DenseIndex (10M) | ~200MB | ~200MB (按需加载可降至 ~50MB) | T-011 |
| Bloom Filters (10M) | ~50-200MB | ~50-200MB | 并行加载 |
| **总计 (10M keys)** | **~300MB-4.5GB** | **~1.1GB-5.2GB** | 含 GlobalKeyIndex |

---

## 5. 执行路线图

### Phase 1: 核心修复 (Week 1-2)
- T-003: GlobalKeyIndex 正确维护
- T-001: GlobalKeyIndex 持久化
- T-002: BlockCache key 直查

### Phase 2: 性能优化 (Week 3-4)
- T-007: MemTable 异步 flush
- T-004: 混合负载优化
- T-008: Cache 预热优化

### Phase 3: 功能扩展 (Week 5-6)
- T-005: 压缩算法扩展
- T-006: 快照 / 隔离级别
- T-009: 大 Value 优化

### Phase 4: 工具与完善 (Week 7)
- T-010: 备份 / 恢复工具
- T-011: DenseIndex 持久化
- 全面 benchmark + 回归测试

---

## 6. 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| GlobalKeyIndex 持久化格式不兼容 | 升级时无法加载旧索引 | 格式版本号 + 自动迁移 |
| 异步 flush 导致数据不一致 | 崩溃恢复失败 | WAL 保证原子性 + 完整性校验 |
| MVCC 增加内存开销 | 10M keys 场景内存超限 | 可配置版本数限制 + compaction 清理 |
| 大 value 分离增加读延迟 | 点查大 value 需额外 I/O | 缓存 overflow 文件元数据 |
| 新压缩算法依赖增加编译时间 | CI/CD 变慢 | feature-gated 按需编译 |

---

## 7. 验收清单

- [ ] 所有 P0 任务完成并通过 review
- [ ] 所有 P1 任务完成并通过 review
- [ ] 443 lib tests + 28 integration tests 全部通过
- [ ] 0 clippy warnings
- [ ] benchmark 结果达到性能目标
- [ ] CHANGELOG.md 更新
- [ ] 文档更新 (README + docs/)
- [ ] RocksDB fair comparison 验证 < 1.5x 差距
