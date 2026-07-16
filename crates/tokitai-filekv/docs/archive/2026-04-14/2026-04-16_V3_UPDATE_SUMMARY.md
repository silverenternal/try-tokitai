# Tokitai-FileKV v0.9.0 性能优化规划 - v3.0 更新总结

**更新日期**: 2026-04-16  
**文档版本**: v3.0  
**状态**: 规划阶段，待实施

---

## 一、本次更新概览

### 1.1 核心变更

| 变更项 | v2.0 | v3.0 | 说明 |
|--------|------|------|------|
| **todo.json 版本** | v2.0 | v3.0 | 新增详细 AI Agent Coder 提示词 |
| **v0.8.0 完成状态** | 8/10（错误） | **10/10（正确）** | 发现 CLOCK 算法和 ZoneMap Arc 已完成 |
| **AI Agent 提示词** | 5 个任务（粗略） | **8 个任务（详细）** | 包含具体代码位置、修改步骤、测试验证 |
| **RCA 状态** | 3 个 PARTIALLY_RESOLVED | **3 个 PARTIALLY_RESOLVED** | 保持不变，与代码实际一致 |
| **性能目标** | 10M 场景 < 10x | **v0.9.0: 100K 50x, 1M 30x; v1.0: 10M < 10x** | 分阶段目标更明确 |

### 1.2 关键发现

通过深入阅读项目源码，发现以下与之前文档不一致的关键事实：

1. **GlobalKeyIndex 已使用 AHashMap<Arc<str>>**（非 BTreeMap<Vec<u8>>），内存布局已优化
2. **CLOCK 算法已完成**（`adaptive.rs` 有 ShardedClockCache，`filter_cache.rs` 也有独立实现）
3. **ZoneMap 已使用 Arc<Vec<ZoneMapEntry>>**（非 Vec clone）
4. **CustomBloom V3 已实现但未集成**（`custom_bloom.rs` 已完成，但 `adaptive.rs` 仍用 `::bloom::BloomFilter`）
5. **moka query_cache 已配置**（500K capacity, 60s TTL）

---

## 二、v0.9.0 优化任务详细规划

### Phase 1: 立即可见的性能优化（Week 1-2, 截止 2026-04-30）

#### OPT-001: GlobalKeyIndex 覆盖率提升

**当前状态**: GlobalKeyIndex 已使用 `AHashMap<Arc<str>, KeyLocation>` + moka query_cache (500K, 60s TTL)  
**存在问题**: `range()` 方法仍遍历全部 entries（HashMap 无序），O(n) 时间复杂度  
**优化方案**: 添加 `BTreeMap<Arc<str>, KeyLocation>` 作为二级索引用于范围查询  
**代码位置**: `src/core/global_index.rs` - `range()` 方法（line ~200）  
**验收标准**: 100K keys 范围查询 P99 < 100µs，内存开销增加 < 20%  
**AI Agent 提示词**: 见 `todo.json` → `ai_agent_coder_prompts` → `OPT-001_GlobalKeyIndex_覆盖率提升`

#### OPT-002: CustomBloom 集成到 AdaptiveBloomCache

**当前状态**: CustomBloom 已实现 V3 bitset 持久化（XXH3 + 双哈希技术）  
**存在问题**: AdaptiveBloomCache 和 BloomFilterCache 仍使用 `::bloom::BloomFilter`  
**优化方案**: 添加枚举 `BloomFilterWrapper { Bloom(::bloom::BloomFilter), Custom(CustomBloom) }`  
**代码位置**: 
- `src/bloom/custom_bloom.rs` - CustomBloom 实现（已完成）
- `src/bloom/adaptive.rs` - L1/L2 缓存（需修改）
- `src/bloom/filter_cache.rs` - BloomFilterCache（需修改）

**验收标准**: Bloom 加载时间 < 100µs（V3 格式直接加载 bitset），负向查询 < 10µs  
**AI Agent 提示词**: 见 `todo.json` → `ai_agent_coder_prompts` → `OPT-002_CustomBloom_集成到_AdaptiveBloomCache`

#### OPT-003: Compaction 触发策略优化

**当前状态**: 依赖采样触发（`CompactionManager.record_write()`），threshold 可能过高  
**存在问题**: L0 segments 可能堆积到 50+ 个  
**优化方案**: 
- 降低 L0 file count threshold 到 2-3
- 实现 WA-aware compaction（WA > 3x 强制触发）
- 增加 compaction 线程数到 min(4, num_cpus/2)
- 实现优先级队列（L0 > L1 > L2）

**代码位置**: `src/compaction/mod.rs`, `src/engine/compaction_engine.rs`  
**验收标准**: 稳态 L0 segments <= 3，写放大率 WA < 3x  
**AI Agent 提示词**: 见 `todo.json` → `ai_agent_coder_prompts` → `OPT-003_Compaction_触发策略优化`

#### OPT-004: DashMap 分片优化与批量写入

**当前状态**: DashMap 分片数默认 num_cpus*2，每次 put 独立插入  
**存在问题**: 高负载下锁竞争明显，批量写入效率低  
**优化方案**: 
- 分片数改为 num_cpus*4
- 实现 batch insert（收集 1ms 内写入，单次锁定批量插入）
- 优化 MemTableEntry 内存布局

**代码位置**: `src/core/memtable.rs`, `src/engine/write_engine.rs` - `put_buffered()`  
**验收标准**: 32 线程并发写入吞吐 > 500K entries/s  
**AI Agent 提示词**: 见 `todo.json` → `ai_agent_coder_prompts` → `OPT-004_DashMap_分片优化与批量写入`

#### OPT-005: BlockCache 淘汰策略优化

**当前状态**: Moka 分片缓存，1GB 容量，大规模随机访问命中率低  
**存在问题**: 10M keys 场景缓存命中率 < 20%  
**优化方案**: 
- 增加分片数到 64-128
- 实现 admission policy（Count-Min Sketch 追踪频率）
- 调高 readahead_multiplier 到 4-8
- 实现缓存预热

**代码位置**: `src/cache/block_cache.rs`, `src/cache/sequential_prefetcher.rs`  
**验收标准**: 10M keys 随机读缓存命中率 > 30%  
**AI Agent 提示词**: 见 `todo.json` → `ai_agent_coder_prompts` → `OPT-005_BlockCache_淘汰策略优化`

### Phase 2: 架构级优化（Week 3-4, 截止 2026-05-14）

#### OPT-006: Size-Tiered Compaction

**目标**: L0 使用 STCS 合并相似大小 segments，L1+ 保持 Leveled  
**验收标准**: 1M keys 场景 L0 segments <= 10，WA < 4x  
**AI Agent 提示词**: 见 `todo.json` → `ai_agent_coder_prompts` → `OPT-006_SizeTiered_Compaction`

#### OPT-007: 批量 WAL + 异步 MemTable Flush

**目标**: 批量 WAL 写入（收集 1-5ms 内写入）+ 异步 MemTable flush（multi-MemTable）  
**验收标准**: 单线程写入吞吐 > 200K entries/s，批量 WAL 延迟 < 5ms  
**AI Agent 提示词**: 见 `todo.json` → `ai_agent_coder_prompts` → `OPT-007_批量WAL_异步MemTable_flush`

#### OPT-008: 写放大监控体系

**目标**: 实现 WA/RA/SA 实时监控，Prometheus 导出  
**验收标准**: 指标实时更新（延迟 < 1s），导出 3 个放大率指标  
**AI Agent 提示词**: 见 `todo.json` → `ai_agent_coder_prompts` → `OPT-008_写放大监控体系`

---

## 三、AI Agent Coder 提示词设计原则

### 3.1 结构化提示词格式

每个任务的提示词包含以下部分：

1. **context**: 当前实现状态和存在的问题
2. **task**: 优化任务描述
3. **specific_code_locations**: 
   - `files`: 涉及的文件列表
   - `key_methods`: 需要修改的方法/函数
   - `integration_points`: 与其他模块的集成点
4. **steps**: 详细执行步骤（包含具体的文件读取、代码修改、测试验证命令）
5. **acceptance_test**: 验收标准（量化指标）
6. **risk_mitigation**: 风险缓解策略（fallback 方案、feature flag）

### 3.2 执行步骤特点

- **具体到文件和方法**: 明确指出需要读取的文件和修改的方法
- **包含测试命令**: 每个步骤后都有对应的 `cargo test` 或 `cargo bench` 命令
- **方案选择**: 提供多个方案供 AI Agent 选择，并给出推荐
- **量化指标**: 验收标准包含具体的性能指标

### 3.3 通用指导原则

```
开始任何优化前，先用 agent tool (Explore) 全面了解相关代码模块
每次修改后必须运行相关测试验证正确性（cargo test --lib <module>）
性能优化必须附带 benchmark 对比数据（优化前后），提升 <5% 视为无效
优先实现最小可行优化（MVP），再迭代增强
保持向后兼容性，任何破坏性变更必须有 fallback 或 feature flag
```

---

## 四、文档整理建议

### 4.1 核心文档（保留）

| 文档 | 路径 | 说明 |
|------|------|------|
| **用户指南** | `doc/filekv/FILEKV_GUIDE.md` | 架构详解、配置指南 |
| **项目定位** | `doc/filekv/POSITION_AND_STATUS.md` | 项目状态、路线图 |
| **操作手册** | `doc/filekv/OPERATIONS_MANUAL.md` | 运维操作指南 |
| **RocksDB 对比** | `doc/filekv/rocksdb_fair_comparison_2026_04_08.md` | 公平对比方法论 |
| **优化规划** | `todo.json` | v0.9.0 完整优化规划与 AI Agent 提示词 |
| **CHANGELOG** | `CHANGELOG.md` | 变更历史 |
| **README** | `README.md` | 项目概览 |

### 4.2 归档文档（移至 docs/archive/）

以下文档为历史阶段总结，已过期，建议归档：

- `v050_PERFORMANCE_VALIDATION.md` - v0.5.0 性能验证
- `v060_EXECUTION_PROGRESS.md` - v0.6.0 执行进度
- `v060_EXECUTION_SUMMARY.md` - v0.6.0 执行总结
- `V060_PERFORMANCE_REPORT.md` - v0.6.0 性能报告
- `v060_STATUS_REPORT.md` - v0.6.0 状态报告
- `v070_P0_EXECUTION_SUMMARY.md` - v0.7.0 P0 执行总结
- `v070_T002_COMPLETION.md` - v0.7.0 T002 完成报告
- `v070_T003_COMPLETION.md` - v0.7.0 T003 完成报告
- `v070_T004_COMPLETION.md` - v0.7.0 T004 完成报告
- `2026-04-16_UPDATE_SUMMARY.md` - 本次更新总结（可保留或归档）

### 4.3 保留文档（docs/ 根目录）

- `BLOOM_FORMAT.md` - Bloom Filter 格式说明（仍有效）
- `TEST_STRATEGY.md` - 测试策略（仍有效）
- `v090_PERFORMANCE_OPTIMIZATION_PLAN.md` - v0.9.0 优化规划（人类可读版）
- `DOCUMENT_CONSOLIDATION_REPORT.md` - 文档整理报告（已过期，可归档）

---

## 五、下一步行动

### 5.1 立即可执行

1. **实施 Phase 1 优化**（OPT-001 ~ OPT-005）:
   - 使用 `todo.json` 中的 AI Agent Coder 提示词
   - 每个任务独立可执行，包含详细步骤和测试验证
   - 预计 2 周内完成

2. **整理文档**:
   - 将历史阶段总结移至 `docs/archive/`
   - 保持核心文档清晰可见

### 5.2 后续规划

3. **实施 Phase 2 优化**（OPT-006 ~ OPT-008）: Week 3-4
4. **实施 Phase 3 优化**（高级优化）: Week 5-8
5. **发布 v0.9.0**: 包含 Phase 1 + Phase 2 优化
6. **发布 v1.0**: 包含 Phase 3 优化，10M 场景与 RocksDB 差距 < 10x

---

## 六、关键指标跟踪

| 指标 | 当前（v0.8.0） | v0.9.0 目标 | v1.0 目标 |
|------|----------------|-------------|-----------|
| **100K keys 写入** | 101ms (161x) | < 20ms (32x) | < 10ms (16x) |
| **1M keys 写入** | 1.27s (200x) | < 100ms (16x) | < 50ms (8x) |
| **10M keys 写入** | N/A | N/A | < 1s (16x) |
| **L0 segments** | 可能 50+ | <= 3 | <= 5 |
| **Bloom 缓存命中率** | 可能 < 50% | > 80% | > 90% |
| **BlockCache 命中率** | < 20% | > 30% | > 50% |
| **写放大率 WA** | 可能 > 5x | < 3x | < 2x |

---

## 七、风险与缓解

| 风险 | 影响 | 缓解策略 |
|------|------|----------|
| **CustomBloom 正确性** | 可能引入 FPR bug | 保留 `::bloom::BloomFilter` 作为 fallback，自动化对比测试 |
| **异步 flush 崩溃恢复** | 数据丢失风险 | 保留同步 flush 作为 fallback，完善 WAL 重放逻辑 |
| **STCS 空间放大** | 磁盘使用增加 | compaction_strategy 可配置，空间放大率高时回退到 Leveled |
| **过度 compaction** | 写放大率上升 | WA-aware 触发策略，WA > 3x 时抑制 compaction |
| **BTreeMap 内存开销** | GlobalKeyIndex 内存增加 | BTreeMap 作为可选 feature flag，默认保持 HashMap-only |

---

**文档结束** - 详细内容请参阅 `todo.json` 中的 AI Agent Coder 提示词
