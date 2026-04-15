# tokitai-filekv v0.4.0 规划与落地建议

**生成日期**: 2026-04-14
**当前版本**: v0.3.1
**项目状态**: ✅ Phase 0-5 全部完成 | 431 测试通过 | 0 clippy 警告

---

## 📊 项目当前状态

### 测试覆盖
- **Lib 测试**: 431/431 (100%) ✅
- **Doctests**: 15/15 通过，6 忽略 ✅
- **集成测试**: 28/28 通过 ✅
- **高并发测试**: 9 个标记 #[ignore] ⚠️
- **Clippy**: 0 warnings ✅
- **unwrap() 审计**: 生产路径 0 处 ✅

### 已完成任务
✅ 四引擎架构 (ReadEngine/WriteEngine/CompactionEngine/LifecycleManager)
✅ 四层错误体系
✅ 统一缓存管理 (UnifiedCacheManager + rebalance)
✅ Bloom Filter 三层自适应缓存 (L1/L2/L3)
✅ Zone Map 范围剪枝
✅ SequentialPrefetch 顺序预取
✅ 字典压缩训练
✅ WAL 安全增强
✅ CI 覆盖 (default/async-io/full)
✅ 属性测试 (proptest)
✅ 运维手册

---

## 🎯 v0.4.0 核心目标

### 任务概览

| ID | 任务 | 优先级 | 预估工时 | 预期收益 |
|----|------|--------|---------|---------|
| TEST-001 | 解除 9 个高并发 ignored 测试 | P0 | 4h | 测试质量提升 |
| POL-003 | Bloom Filter 序列化优化 | P0 | 12h | 性能提升 50%+ |
| POL-004 | Segment 遍历优化 | P1 | 10h | 性能提升 20%+ |
| PROD-001 | BlockCache 真正动态缩容 | P1 | 20h | 内存管理改进（可选） |

**总预计工时**: 46 小时 (约 2-4 周)

---

## 📝 任务详情与 AI Agent 提示

### TEST-001: 解除 9 个高并发 ignored 测试

**优先级**: P0 (立即执行)
**预估工时**: 4 小时
**文件**: `tests/filekv_integration/high_concurrency.rs`

#### 测试清单

**32 线程测试 (4个)**:
1. `test_32_threads_concurrent_puts` - 并发写入测试
2. `test_32_threads_concurrent_gets` - 并发读取测试（预填充 10K keys）
3. `test_32_threads_mixed_read_write` - 混合读写测试
4. `test_32_threads_cache_stress` - 缓存压力测试

**64 线程测试 (3个)**:
5. `test_64_threads_concurrent_puts` - 高并发写入测试
6. `test_64_threads_concurrent_gets` - 高并发读取测试（预填充 20K keys）
7. `test_64_threads_hot_key_contention` - 热键竞争测试

**缓存压力测试 (2个)**:
8. `test_32_threads_cache_stress` - BlockCache 压力测试
9. `test_32_threads_puts_then_flush_and_reopen` - 持久化验证

#### AI Agent 提示词

```
分析 9 个高并发测试的执行时间瓶颈。这些测试使用 std::thread::spawn 创建 32/64 个线程，
每个线程执行 50-1000 次操作。优化策略：

1. 减少线程数量（32→16，64→32）以降低测试时间
2. 减少每线程操作数（1000→100，5000→500）
3. 对于预填充大量 keys 的测试（如 test_32_threads_concurrent_gets 预填充 10K keys），
   减少到 1K keys
4. 添加 #[timeout(60_000)] 防止挂起
5. 逐个解除 #[ignore] 并验证 cargo test --test filekv_integration 能通过

注意：这些测试验证并发正确性而非性能基准，减少规模不应影响测试价值。

验证命令：
- cargo test --test filekv_integration -- 确认 19 passed, 0 failed
- cargo clippy --features wal -- -D warnings -- 确认 0 warnings
```

#### 验收标准
- ✅ cargo test --test filekv_integration 默认运行并通过所有测试
- ✅ ignored 从 9 降至 0（或仅剩 1-2 个极慢测试如热键竞争）
- ✅ 所有测试在 60 秒内完成

---

### POL-003: Bloom Filter 序列化格式优化

**优先级**: P0 (最高 ROI)
**预估工时**: 12 小时
**预期收益**: Bloom 加载时间降低 50%+，负向查询从 14ms 降至 <100µs

#### 问题分析

当前 Bloom Filter 存储格式为 **keys 列表** (`Vec<String>`)，每次加载需：
1. 反序列化 keys 列表
2. 重新创建 Bloom Filter（插入所有 keys）
3. 计算哈希函数

这导致 Bloom 重复重建占 **40-50%** 性能时间。

#### 优化方案

**新序列化格式**: 存储位数组 (bitset)
```
[magic 4B][version 1B][bit vector bytes][bit count][num_hash_functions]
```

**实现步骤**:
1. 在 `src/bloom/manager.rs` 中实现 `serialize_to_bitset()`/`deserialize_from_bitset()`
2. 更新 segment 写入路径：Bloom Filter 序列化为位数组
3. 更新加载路径：直接从位数组加载，跳过重建步骤
4. 确保向后兼容：旧格式 segment 仍能加载（fallback 到 keys 重建）

#### AI Agent 提示词

```
分析当前 Bloom Filter 的存储格式（在 src/bloom/manager.rs 中如何序列化？是否存储 keys 列表？）。

设计位数组序列化格式：[magic 4B][version 1B][bit vector bytes][bit count][num_hash_functions]

实现步骤：
1. 在 src/bloom/manager.rs 中实现 serialize_to_bitset()/deserialize_from_bitset() 方法
2. 更新 segment 写入路径（src/core/segment.rs）：Bloom Filter 序列化为位数组而非 keys 列表
3. 更新加载路径：直接从位数组加载，跳过重建步骤
4. 运行 benches/adaptive_bloom_bench.rs 验证性能提升
5. 确保向后兼容：旧格式 segment 仍能加载（fallback 到 keys 重建）

参考文档：docs/BLOOM_FORMAT.md 了解当前格式规范

验证命令：
- cargo bench --features benchmarks --bench adaptive_bloom_bench
- cargo test --lib --features wal -- 确认所有 Bloom 测试通过
```

#### 验收标准
- ✅ benches/adaptive_bloom_bench.rs 验证 Bloom 加载时间降低 50%+
- ✅ 负向查询延迟从 14ms 降至 <100µs
- ✅ 向后兼容：旧格式 segment 仍能加载

---

### POL-004: Segment 遍历性能优化

**优先级**: P1
**预估工时**: 10 小时
**预期收益**: get() 延迟降低 20%+

#### 问题分析

当前 `ReadEngine::get()` 使用**线性 segment 遍历**查找 key，占 **25-30%** 性能时间。

#### 优化方案

**方案 A**: 使用 dense index 直接定位
- 检查 `src/core/sparse_index.rs` 中的 SparseIndex 是否可升级
- 如果可行，修改 get() 使用索引而非顺序扫描

**方案 B**: 添加二级索引
- 在 Segment 中添加 `key_hash → block_offset` 映射
- 使用哈希表加速查找

#### AI Agent 提示词

```
分析 ReadEngine::get() 的 segment 遍历路径（在 src/engine/read_engine.rs 中）。

步骤：
1. 检查 SparseIndex（src/core/sparse_index.rs）是否可用于加速查找
2. 如果 dense index 可用，修改 get() 使用索引而非顺序扫描
3. 如果不可用，考虑在 Segment 中添加二级索引（key_hash → block_offset）
4. 运行基准测试验证性能提升

注意：不要破坏现有的 Zone Map 剪枝逻辑。

验证命令：
- cargo bench --features benchmarks --bench file_kv_bench
- cargo test --lib --features wal -- 确认所有测试通过
```

#### 验收标准
- ✅ get() 延迟降低 20%+
- ✅ benches/file_kv_bench.rs 验证数据更新
- ✅ 所有现有测试通过

---

### PROD-001: BlockCache 真正动态缩容（可选）

**优先级**: P1（可选，v0.4.0 或 v0.5.0）
**预估工时**: 20 小时
**状态**: 设计方案已完成

#### 问题分析

当前 BlockCache rebalance 仅 **advisory mode**（Moka capacity 不可变），无法真正缩容。

#### 优化方案

**方案 B**: 多实例 Moka 分片架构（推荐）
- 将 BlockCache 拆分为 4-8 个 Moka 子实例
- 每个实例 max_capacity 独立可调整
- 当需要缩容时，销毁最不活跃的子实例并重建

#### AI Agent 提示词

```
阅读设计文档 docs/plans/PROD-001-blockcache-dynamic-shrink-design.md

推荐实现方案 B（多实例 Moka 分片）：
1. 将 BlockCache 拆分为 4-8 个 Moka 子实例，每个实例 max_capacity 独立可调整
2. 当需要缩容时，销毁最不活跃的子实例并重建
3. 实现 shrink_to_memory()/grow_max_memory() 方法
4. 更新 rebalance 执行引擎（src/cache/rebalance.rs）调用新方法
5. 验证内存使用变化

这是可选任务，可以在 v0.4.0 或 v0.5.0 实现。

验证命令：
- cargo test --lib --features wal
- cargo test --test filekv_integration --features wal
```

#### 验收标准
- ✅ rebalance 执行后 BlockCache 实际内存使用变化
- ✅ 通过 metrics 验证内存缩容效果
- ✅ 所有现有测试通过

---

## 🚀 推荐执行顺序

### 第一阶段：测试质量提升 (1-2 天)
1. **TEST-001** - 解除 9 个高并发 ignored 测试
   - 立即可做，无需额外依赖
   - 提升测试覆盖率和信心

### 第二阶段：核心性能优化 (1-2 周)
2. **POL-003** - Bloom Filter 序列化优化
   - 最高 ROI（性能提升 50%+）
   - 解决已知 14ms 异常慢查询
3. **POL-004** - Segment 遍历优化
   - 性能提升 20%+
   - 依赖 POL-003 完成后的性能基线

### 第三阶段：可选改进 (1 周，可选)
4. **PROD-001** - BlockCache 真正动态缩容
   - 可选任务，可推迟到 v0.5.0
   - 改善内存管理但对性能影响较小

---

## 📋 验证清单

每次任务完成后，运行以下命令验证：

```bash
# 1. Lib 测试（431 个）
cargo test --lib --features wal

# 2. 集成测试（28 个）
cargo test --test filekv_integration --features wal

# 3. Doctests（15 个）
cargo test --doc

# 4. Clippy 检查（0 warnings）
cargo clippy --features wal -- -D warnings

# 5. 基准测试（性能验证）
cargo bench --features benchmarks
```

---

## 📚 关键文件索引

| 文件 | 描述 |
|------|------|
| `todo.json` | 完整任务规划（含 AI Agent 提示词） |
| `README.md` | 快速参考：性能数据、快速开始 |
| `CHANGELOG.md` | 版本历史记录 |
| `doc/filekv/POSITION_AND_STATUS.md` | 项目定位与状态 |
| `doc/filekv/FILEKV_GUIDE.md` | 技术指南（架构、配置、故障排查） |
| `docs/BLOOM_FORMAT.md` | Bloom Filter 格式规范 |
| `docs/plans/PROD-001-blockcache-dynamic-shrink-design.md` | BlockCache 设计方案 |
| `V040_PLANNING_UPDATE_SUMMARY.md` | 本次规划更新总结 |

---

## ⚠️ 重要说明

1. **测试位置更正**: 9 个 ignored 测试全部在 `tests/filekv_integration/high_concurrency.rs`，不在 src/ 中
2. **测试数量更正**: Lib 测试实际为 431 个（非 413 个）
3. **性能数据**: 注明测试日期 (2026-04-14)，可能随优化而变化
4. **代码风格**: Rust 2021 Edition，与现有代码保持一致
5. **向后兼容**: 所有优化需确保现有 API 和数据格式兼容

---

**生成时间**: 2026-04-14T22:50:00Z
**下次更新**: v0.4.0 任务完成后
