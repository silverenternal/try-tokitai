# v0.9.0 大规模性能优化规划

**创建日期**: 2026-04-16
**目标版本**: v0.9.0
**状态**: 规划中
**最后更新**: 2026-04-16

---

## 概述

v0.9.0 聚焦大规模数据集性能优化，目标将 FileKV 与 RocksDB 的性能差距缩小到可接受范围：

| 场景 | 当前 (v0.8.0) | v0.9.0 目标 |
|------|--------------|-------------|
| 100K keys 写入 | 161x slower | 50x slower |
| 1M keys 写入 | 200x slower | 30x slower |
| 10M keys 写入 | TBD | <10x slower |

---

## v0.8.0 已完成优化 (10/10)

所有 v0.8.0 优化已全部完成，为 v0.9.0 大规模优化奠定基础：

1. ✅ **WAL 二进制序列化** (3-5x 加速)
2. ✅ **CDict/DDict 预创建** (10-100x 压缩加速)
3. ✅ **GlobalKeyIndex 真正启用** (直接 segment 定位)
4. ✅ **Bloom L2 缓存 Arc 重构** (O(1) 访问)
5. ✅ **BloomFilterCache CLOCK 算法** (7.4x 并发提升)
6. ✅ **ZoneMap Arc 包装** (消除 Vec clone)
7. ✅ **Instant 时间戳** (无系统调用)
8. ✅ **AHash 分片** (3-5x 加速)
9. ✅ **Compaction 锁优化** (AtomicUsize)
10. ✅ **定时 fsync** (10ms 间隔)

---

## v0.9.0 优化阶段

### Phase 1: 立即可见的性能优化 (Week 1-2, 2026-04-16 ~ 2026-04-30)

**目标**: 100K keys 场景从 161x 缩小到 50x 以内

#### OPT-001: GlobalKeyIndex 内存优化 + 覆盖率提升
- **优先级**: P0
- **负责人**: global_index 模块负责人
- **描述**: GlobalKeyIndex 已在 v0.6.0+ 启用，但内存开销大 (~100 bytes/key) 且可能 stale
- **行动项**:
  - key 用 Arc<str> 替代 Vec<u8>
  - KeyLocation 用 packed struct
  - query_cache 扩容到 500K-1M，TTL 缩短到 60s
  - 确保 flush/compaction 时立即更新索引
- **验收标准**: 10M keys 索引内存 < 500MB，读取吞吐提升 >3x
- **截止日期**: 2026-04-22

#### OPT-002: CustomBloom 集成到 AdaptiveBloomCache 主路径
- **优先级**: P0
- **负责人**: bloom_cache 模块负责人
- **描述**: CustomBloom (custom_bloom.rs) 已实现 V3 bitset 持久化，但未集成到主路径
- **行动项**:
  - 使用 CustomBloom 替代 ::bloom::BloomFilter
  - V1/V2 → V3 自动迁移
  - 确保 backward compatibility
- **验收标准**: Bloom 加载时间 < 100µs，负向查询 < 10µs
- **截止日期**: 2026-04-25

#### OPT-003: Compaction 触发策略优化
- **优先级**: P1
- **负责人**: compaction 模块负责人
- **描述**: 更激进的 L0 compaction 触发策略，避免 segments 堆积
- **行动项**:
  - L0 file count threshold 降到 2-3
  - write-amplification-aware compaction
  - 后台 compaction 线程数增加到 min(4, num_cpus/2)
- **验收标准**: 稳态 L0 segments <= 3，WA < 3x
- **截止日期**: 2026-04-25

#### OPT-004: DashMap 分片优化 + MemTable 内存布局
- **优先级**: P1
- **负责人**: write_engine 模块负责人
- **描述**: 优化 MemTable DashMap 分片策略，减少高并发锁竞争
- **行动项**:
  - DashMap 分片数从 num_cpus*2 提升到 num_cpus*4
  - batch insert 优化
  - MemTable per-entry overhead < 50 bytes
- **验收标准**: 32 线程并发写入吞吐 > 500K entries/s
- **截止日期**: 2026-04-25

#### OPT-005: BlockCache 淘汰策略优化
- **优先级**: P1
- **负责人**: block_cache 模块负责人
- **描述**: 针对大规模随机访问优化 Moka 缓存参数
- **行动项**:
  - Moka 配置优化
  - admission policy
  - SequentialPrefetcher 更积极使用
- **验收标准**: 10M keys 随机读缓存命中率 > 30%
- **截止日期**: 2026-04-25

---

### Phase 2: 架构级优化 (Week 3-4, 2026-04-30 ~ 2026-05-14)

**目标**: 1M keys 场景从 200x 缩小到 30x 以内

#### OPT-006: LSM-Tree 结构优化 - Size-Tiered Compaction
- **优先级**: P0
- **描述**: L0 使用 STCS，L1+ 保持 Leveled
- **验收标准**: 1M keys 场景 L0 segments <= 10，compaction 并行度 >= 2

#### OPT-007: BlockCache 多级缓存架构
- **优先级**: P1
- **描述**: L1 (内存热点) + L2 (mmap 温点) + L3 (磁盘冷点)
- **验收标准**: 1M keys 场景整体缓存命中率 > 50%

#### OPT-008: 批量 WAL + 异步 MemTable flush
- **优先级**: P1
- **描述**: 真正的批量 WAL 写入和异步 MemTable flush
- **验收标准**: 单线程写入吞吐 > 200K entries/s，WA < 3x

#### OPT-009: Segment 文件格式优化
- **优先级**: P2
- **描述**: block-level Bloom Filter, key range 元数据
- **验收标准**: 单次 segment 扫描时间 < 100µs

---

### Phase 3: 高级优化 (Week 5-8, 2026-05-14 ~ 2026-06-11)

**目标**: 10M keys 场景达到生产级性能（与 RocksDB 差距 < 10x）

#### OPT-010: 全局有序索引重构
- **优先级**: P0
- **描述**: SkipList/Trie 替代 BTreeMap
- **验收标准**: 10M keys 索引内存 < 800MB，查找延迟 P99 < 5µs

#### OPT-011: io_uring 异步 I/O 支持
- **优先级**: P1
- **描述**: Linux io_uring 异步 I/O
- **验收标准**: 随机读 I/O 延迟降低 > 30%

#### OPT-012: WA/RA/SA 实时监控体系
- **优先级**: P1
- **描述**: 完整的放大率监控
- **验收标准**: Prometheus 导出 10+ 放大率相关指标

#### OPT-013: RocksDB 对齐 Benchmark 套件
- **优先级**: P1
- **描述**: 10M+ keys 专业 benchmark
- **验收标准**: 10M keys benchmark 可重复运行（3 次偏差 < 5%）

---

## AI Agent Coder 提示词

每个优化任务的详细执行指导见 `todo.json` 中的 `ai_agent_coder_prompts` 部分。

### 通用指导原则

1. 开始任何优化前，先用 Explore agent 全面了解相关代码模块
2. 每次修改后必须运行相关测试验证正确性
3. 性能优化必须附带 benchmark 对比数据（优化前后）
4. 优先实现最小可行优化（MVP），再迭代增强
5. 保持向后兼容性，任何破坏性变更必须有 fallback

---

## 风险与回滚

### 高风险项

| 风险 | 缓解措施 | 回滚计划 |
|------|---------|---------|
| CustomBloom 正确性 bug | 保留 ::bloom::BloomFilter 作为 fallback | feature flag 切换回旧实现 |
| 过度 compaction 增加 WA | WA 监控，> 3x 时自动降低频率 | 调高 threshold 回退 |
| GlobalKeyIndex 内存仍然过高 | AHashMap 替代 BTreeMap | 保持 BTreeMap，优化布局 |

---

## 进度跟踪

| 任务 | 状态 | 截止日期 | 实际完成 |
|------|------|---------|---------|
| OPT-001 | OPEN | 2026-04-22 | - |
| OPT-002 | OPEN | 2026-04-25 | - |
| OPT-003 | OPEN | 2026-04-25 | - |
| OPT-004 | OPEN | 2026-04-25 | - |
| OPT-005 | OPEN | 2026-04-25 | - |
| OPT-006 | PLANNED | 2026-05-07 | - |
| OPT-007 | PLANNED | 2026-05-14 | - |
| OPT-008 | PLANNED | 2026-05-14 | - |
| OPT-009 | PLANNED | 2026-05-14 | - |
| OPT-010 | PLANNED | 2026-05-28 | - |
| OPT-011 | PLANNED | 2026-06-04 | - |
| OPT-012 | PLANNED | 2026-06-04 | - |
| OPT-013 | PLANNED | 2026-06-11 | - |

---

## 参考文档

- `todo.json` - 完整优化规划与 AI Agent Coder 提示词
- `CHANGELOG.md` - 版本历史
- `README.md` - 项目概览
- `doc/filekv/POSITION_AND_STATUS.md` - 项目定位与状态
