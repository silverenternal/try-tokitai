# v0.4.0 规划执行摘要

**日期**: 2026-04-14
**版本**: v0.3.1 (v0.4.0 规划中)
**状态**: Phase 0-5 全部完成 ✅

---

## 📊 当前项目健康度

| 指标 | 状态 | 详情 |
|------|------|------|
| **Lib 测试** | ✅ 413/413 (100%) | 0 失败，8 ignored |
| **集成测试** | ✅ 28/28 (100%) | 6 个模块 |
| **Doctests** | ✅ 15/15 (100%) | 6 ignored（预期） |
| **Clippy** | ✅ 0 warnings | 零警告 |
| **生产路径 unwrap()** | ✅ 0 处 | 全部消除 |
| **CI 覆盖** | ✅ default/async-io/full | 三维度 |

---

## 🎯 v0.4.0 核心目标（3 大任务）

### 1. TEST-001: 解除 8 个 ignored 测试 (P0, 4h)

**当前 ignored 分布**:
- 3 compaction: `src/engine/tests.rs:459,507` + 1 未定位
- 3 stability: `src/tests/stability.rs`
- 2 bloom: `src/bloom/adaptive.rs:2759`, `src/bloom/migration.rs:795`

**优化方案**:
- 减少数据规模（entries 1000→100, segments 5→2）
- 缩短运行时间（60s→10s）
- 添加 `#[timeout(30_000)]` 防挂起

**验收标准**: `cargo test --lib` 默认运行并通过（ignored 8→0）

---

### 2. POL-003: Bloom Filter 序列化优化 (P0, 12h, 最高 ROI)

**问题**: Bloom 重复重建占 40-50% 时间，负向查询异常慢 (14ms)

**根因**: 当前存储 keys 列表 (`Vec<String>`)，每次加载需重建 Bloom Filter

**方案**: 存储位数组 (bitset)
```
序列化格式: [magic 4B][bit vector bytes][bit count][num_hash_functions]
```

**关键步骤**:
1. 分析当前 segment 中 Bloom 存储格式（`src/core/segment.rs`）
2. 实现 `serialize_to_bitset()`/`deserialize_from_bitset()` 
3. 更新写入路径：Bloom → 位数组
4. 更新加载路径：位数组 → 直接使用（跳过重建）
5. 向后兼容：旧格式 fallback 到 keys 重建

**验收标准**:
- Bloom 加载时间降低 50%+
- 负向查询从 14ms 降至 <100µs
- `benches/adaptive_bloom_bench.rs` 验证

---

### 3. POL-004: Segment 遍历优化 (P1, 10h)

**问题**: 线性 segment 遍历占 25-30% 性能时间

**方案**:
- (a) 使用 dense index 直接定位
- (b) 添加二级索引 (key_hash → segment_offset)

**验收标准**: `get()` 延迟降低 20%+

---

### 4. PROD-001: BlockCache 动态缩容 (P1, 20h, 可选)

**状态**: 设计方案完成（多实例 Moka 分片）

**方案**: 拆分为 4-8 个 Moka 子实例，每个可独立销毁重建

**验收**: rebalance 后实际内存使用变化

---

## 📁 关键文件路径

| 组件 | 文件路径 |
|------|---------|
| **Bloom Filter** | `src/bloom/adaptive.rs`, `src/bloom/filter_cache.rs` |
| **Segment** | `src/core/segment.rs` |
| **Read Engine** | `src/engine/read_engine.rs` |
| **BlockCache** | `src/cache/block_cache.rs` |
| **Ignored Tests** | `src/engine/tests.rs`, `src/tests/stability.rs`, `src/bloom/adaptive.rs`, `src/bloom/migration.rs` |
| **Bloom Bench** | `benches/adaptive_bloom_bench.rs` |
| **KV Bench** | `benches/file_kv_bench.rs` |

---

## 🚀 推荐执行顺序

1. **TEST-001** (4h) - 立即可做，提升测试质量
2. **POL-003** (12h) - 最高 ROI，性能提升 50%+
3. **POL-004** (10h) - 性能提升 20%+
4. **PROD-001** (20h) - 可选，v0.4.0 或 v0.5.0

**总预计工时**: 46 小时（2-4 周）

---

## 📝 文档更新清单

| 文档 | 状态 | 更新内容 |
|------|------|---------|
| `todo.json` | ✅ 已更新 | v0.4.0 规划，Phase 0-5 完成标记 |
| `CHANGELOG.md` | ✅ 已更新 | 添加 v0.4.0 Planning 章节 |
| `README.md` | ✅ 已更新 | Phase 0-5 完成，v0.4.0 规划中 |
| `doc/filekv/README.md` | ✅ 已更新 | 测试状态，v0.4.0 规划 |
| `doc/filekv/POSITION_AND_STATUS.md` | ✅ 已更新 | v0.4.0 规划章节，质量指标 |
| `doc/filekv/FILEKV_GUIDE.md` | ✅ 已更新 | 版本号更新 |

---

## ✅ Phase 0-5 完成情况

### 已完成任务清单

| Phase | 任务 | 状态 |
|-------|------|------|
| **Phase 0** | 性能基准更新 (PERF-001~004) | ✅ |
| **Phase 1** | 关键功能修复 (FIX-001~004) | ✅ |
| **Phase 2** | 测试管线增强 (TEST-001~005) | ✅ 部分提前完成 |
| **Phase 3** | 文档诚实化 (DOC-UPD-007~009) | ✅ |
| **Phase 4** | 锦上添花 (POL-001~002) | ✅ |
| **Phase 5** | 生产就绪 (PROD-001~004) | ✅ |

### 提前完成的任务（原规划在 Phase 2/4/5）

- ✅ **tests/ 集成测试目录**: 28 个测试，6 个模块（原 TEST-003）
- ✅ **CI async-io feature 测试矩阵**: 三维度覆盖（原 TEST-002）
- ✅ **unwrap() 审计**: 生产路径 0 处（原 POL-001）
- ✅ **属性测试**: `src/tests/property_tests.rs`（原 POL-002）
- ✅ **高并发测试**: 9 个测试（原 PROD-002）
- ✅ **稳定性测试框架**: 脚本 + 示例（原 PROD-003）
- ✅ **运维文档**: OPERATIONS_MANUAL.md（原 PROD-004）

---

## 🎓 AI Agent Coder 执行提示

### 通用原则
- 每次修改后运行 `cargo test --lib` 验证
- 每次修改后运行 `cargo clippy --features wal -- -D warnings` 验证
- 代码风格：Rust 2021 Edition
- 使用现有公共 API，避免直接访问私有字段

### 测试执行
```bash
# 运行所有 lib tests
cargo test --features wal --lib

# 运行 ignored 测试
cargo test --features wal --lib -- --ignored

# 运行集成测试
cargo test --features wal --test '*'

# 使用 nextest 加速
cargo nextest run --lib --test-threads 4
```

### 基准测试执行
```bash
# Bloom 性能
cargo bench --features benchmarks --bench adaptive_bloom_bench

# KV 性能
cargo bench --features benchmarks --bench file_kv_bench
```

---

## 📞 下一步

1. **立即可做**: TEST-001 解除 ignored 测试（4h）
2. **高优先级**: POL-003 Bloom 序列化优化（12h，50%+ 性能提升）
3. **中优先级**: POL-004 Segment 遍历优化（10h，20%+ 性能提升）
4. **可选**: PROD-001 BlockCache 动态缩容（20h）

详细 AI Agent Coder 提示词请查看 `todo.json` 中每个任务的 `ai_prompt_hint` 字段。
