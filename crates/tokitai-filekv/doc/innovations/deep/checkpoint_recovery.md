# 检查点与恢复机制创新深度调研

> 本文档详细分析 tokitai-filekv 的检查点 (Checkpoint) 与恢复机制,包含快照隔离、WAL 恢复、崩溃恢复和性能数据。

---

## 目录

- [1. 检查点与恢复总览](#1-检查点与恢复总览)
- [2. Checkpoint 创建流程](#2-checkpoint-创建流程)
- [3. CheckpointMetadata 数据结构](#3-checkpointmetadata-数据结构)
- [4. WAL 恢复和重放逻辑](#4-wal-恢复和重放逻辑)
- [5. 崩溃恢复场景](#5-崩溃恢复场景)
- [6. 快照隔离和一致性保证](#6-快照隔离和一致性保证)
- [7. RTO 和 RPO 分析](#7-rto-和-rpo-分析)
- [8. 测试用例和场景](#8-测试用例和场景)
- [9. 性能影响数据](#9-性能影响数据)
- [10. 关键文件索引](#10-关键文件索引)

---

## 1. 检查点与恢复总览

### 1.1 设计目标

tokitai-filekv 的检查点与恢复机制设计目标:

1. **零数据丢失**: RPO = 0 (理论上)
2. **快速恢复**: RTO < 10s (1M keys)
3. **一致性保证**: 快照隔离
4. **自动化**: 后台自动 checkpoint

### 1.2 核心架构

```
数据保护层次:
  ├── MemTable (内存,最新数据)
  ├── WAL (持久化,重放日志)
  ├── Segments (持久化,不可变数据)
  └── Checkpoint (快照,一致性视图)
```

---

## 2. Checkpoint 创建流程

### 2.1 7 步创建流程

**文件**: `src/checkpoint/manager.rs`

```
create_full_checkpoint():
  ├── 1. 触发 memtable flush (将内存数据刷到 segment)
  ├── 2. 获取当前所有活跃 segment 列表
  ├── 3. 读取 WAL 当前序列号 (LSN)
  ├── 4. 构建 CheckpointMetadata
  ├── 5. 将 checkpoint 元数据持久化到 checkpoints/ 目录
  │     └── 写入 JSON: checkpoints/{cp_id}/metadata.json
  ├── 6. 复制/硬链接 segment 文件到 checkpoint 目录
  └── 7. 清理过期 checkpoint (保留最近 N 个)
```

### 2.2 入口函数

**文件**: `src/checkpoint/filekv_impl.rs`

```rust
impl FileKV {
    pub async fn create_full_checkpoint(&self) -> Result<()> {
        self.checkpoint_manager.create_checkpoint().await
    }
}
```

### 2.3 完全 vs 增量 Checkpoint

| 特性 | 完全 Checkpoint | 增量 Checkpoint |
|------|----------------|----------------|
| 当前实现 | ✅ 已实现 | ❌ 未实现 |
| 磁盘占用 | 大 (复制所有 segment) | 小 (只记录变更) |
| 创建速度 | 慢 | 快 |
| 恢复速度 | 快 (直接加载) | 慢 (需要重放 WAL) |
| 实现复杂度 | 低 | 高 |

**为什么没有增量 Checkpoint**:
- 项目专注于 LSM-Tree 架构,segment 本身就是不可变的
- 每次 checkpoint 前 flush memtable,确保数据落入 segment
- 增量 checkpoint 在这种架构下收益有限

---

## 3. CheckpointMetadata 数据结构

### 3.1 定义

**文件**: `src/checkpoint/types.rs`

```rust
pub struct CheckpointMetadata {
    pub id: String,              // 单调递增 ID (如 "cp_20250315_123456")
    pub timestamp: u64,          // 创建时间戳
    pub segment_ids: Vec<String>, // 快照包含的段文件 ID 列表
    pub wal_sequence_number: u64, // 检查点时刻的 WAL 序列号
    pub manifest_version: u64,   // 对应的压缩清单版本
    pub total_keys: u64,         // 总键数
    pub total_size_bytes: u64,   // 总大小
}
```

### 3.2 示例

```json
{
  "id": "cp_20250315_123456",
  "timestamp": 1710486896,
  "segment_ids": ["seg_001", "seg_002", "seg_003"],
  "wal_sequence_number": 12345,
  "manifest_version": 42,
  "total_keys": 50000,
  "total_size_bytes": 104857600
}
```

### 3.3 CheckpointStats

```rust
pub struct CheckpointStats {
    pub total_checkpoints: u64,
    pub last_checkpoint_time: u64,
}
```

---

## 4. WAL 恢复和重放逻辑

### 4.1 WAL 二进制格式

**文件**: `src/core/wal.rs`

**当前版本格式**:
```
| seq:u64 | op_type:u8 | session:u16+len | hash:u16+len | layer:u16+len | payload:u32+len | checksum:u32 |
```

**字段说明**:
- `seq`: 64 位序列号 (单调递增)
- `op_type`: PUT=1, DELETE=2
- `session/layer/hash`: 审计元数据
- `payload`: 实际 KV 数据
- `checksum`: CRC32 校验和

### 4.2 恢复流程

```
WAL 恢复:
  ├── 1. 扫描 wal_dir/ 下所有 WAL 文件
  ├── 2. 按文件名排序 (确保按创建顺序处理)
  ├── 3. 逐个读取 WAL 条目:
  │     ├── a. 读取序列号,校验连续性
  │     ├── b. 验证 checksum (CRC32)
  │     ├── c. 根据 op_type 执行 PUT 或 DELETE
  │     └── d. 将条目插入 MemTable
  ├── 4. 恢复完成后,记录最新的序列号
  └── 5. 截断/清理已恢复的 WAL 文件
```

### 4.3 序列号连续性校验

```rust
// 恢复时检查序列号是否连续
let mut expected_seq = start_seq;
for entry in wal_entries {
    if entry.seq != expected_seq {
        warn!("WAL sequence gap: expected {}, got {}", expected_seq, entry.seq);
    }
    expected_seq = entry.seq + 1;
}
```

### 4.4 WAL Channel 异步架构

**文件**: `src/core/wal_channel.rs`

```
put_buffered() → channel send → 后台线程批量写入 WAL → 批量插入 MemTable
```

**优化**:
- MemTable 插入在 WAL 刷盘后批量执行
- 非每次写入单独插入
- 减少锁竞争

---

## 5. 崩溃恢复场景

### 5.1 场景 A: 正常关闭后重启

**状态**:
- WAL 可能为空 (已 flush 并清理)
- MemTable 从磁盘 segment 重建

**恢复流程**:
```
open() → 加载 segments → 重建索引 → 就绪
```

**数据丢失**: 无

### 5.2 场景 B: 写入中崩溃 (WAL 已写入,MemTable 未持久化)

**状态**:
- WAL 包含未持久化的数据
- MemTable 为空或部分数据

**恢复流程**:
```
open() → 扫描 WAL → 重放条目 → 重建 MemTable → 就绪
```

**数据丢失**: 仅丢失正在写入 WAL 的条目

### 5.3 场景 C: Checkpoint 中崩溃

**状态**:
- Checkpoint 元数据可能未完成写入
- 已有 segment 不受影响

**恢复流程**:
```
open() → 加载最近有效 checkpoint → 重放后续 WAL → 就绪
```

**数据丢失**: 无 (checkpoint 原子写入)

### 5.4 场景 D: Compaction 中崩溃

**状态**:
- CompactionManifest 保护新旧段文件清单
- 未完成的 compaction 数据

**恢复流程**:
```
open() → 加载 manifest → 检测未完成 compaction → 清理临时文件 → 就绪
```

**数据丢失**: 无 (manifest 保护)

---

## 6. 快照隔离和一致性保证

### 6.1 三层隔离

**层级 1: MemTable 隔离**
- MemTable 使用 DashMap (并发哈希表)
- 提供线程安全访问

**层级 2: Segment 不可变性**
- Segment 一旦写入即为不可变 (immutable)
- 写入通过 SegmentWriter 追加,读取通过 mmap 只读访问
- checkpoint 时刻的 segment 快照天然一致

**层级 3: WAL 原子性**
- WAL 条目带有序列号 (sequence number),保证有序重放
- CRC32 校验和检测损坏

### 6.2 写入路径一致性

```
put() → WriteCoalescer → WAL Channel → MemTable → (异步) → Segment Flush
```

- WAL 先于 MemTable 写入 (Write-Ahead 语义)
- WAL 通过 channel 异步写入,保证顺序
- WriteCoalescer 合并写入,减少竞争

### 6.3 Checkpoint 时刻一致性

1. Checkpoint 创建前强制 flush memtable
2. 记录当前 WAL 序列号 (LSN)
3. 记录当前 segment 列表 (快照时刻的完整数据状态)
4. 元数据和 segment 列表作为原子单元持久化

---

## 7. RTO 和 RPO 分析

### 7.1 RTO (Recovery Time Objective)

**理论分析**:

恢复时间 = MemTable 重建时间 + WAL 重放时间

| 场景 | 估计 RTO |
|------|---------|
| 小数据集 (<100K keys) | <1 秒 |
| 中等数据集 (1M keys) | 数秒 |
| 大数据集 (10M+ keys) | 数十秒 |

**影响因素**:
- WAL 条目数量 (需要重放的条目数)
- MemTable 插入速度
- 磁盘 I/O 性能

**优化措施**:
- 定期 checkpoint 减少需要重放的 WAL 量
- WAL 二进制序列化 (比 JSON 快 3-5x)
- 批量 MemTable 插入

### 7.2 RPO (Recovery Point Objective)

**RPO = 0** (理论零数据丢失)

**原因**:
- WAL 在数据写入 MemTable 之前持久化
- 每次 `put()` 操作先写入 WAL
- 崩溃后通过 WAL 重放可恢复到崩溃前最后一笔操作

**实际 RPO**:
- 如果 WAL 启用: RPO ≈ 0 (仅丢失正在写入 WAL 的条目)
- 如果 WAL 禁用: RPO = 上次 flush 到崩溃之间的所有写入

### 7.3 WAL 同步模式对比

| 模式 | RPO | 性能 | 说明 |
|------|-----|------|------|
| Immediate | 0 | 最低 | 每次 fsync |
| Batch | ~10ms | 中等 | 批量 fsync |
| Lazy | 不确定 | 最高 | OS 缓冲 |

---

## 8. 测试用例和场景

### 8.1 集成测试

**文件**: `tests/filekv_integration/checkpoint.rs`

| 测试 | 验证内容 |
|------|---------|
| `test_checkpoint_creation_basic` | 基本 checkpoint 创建 |
| `test_checkpoint_recovery_after_crash` | 写入→flush→checkpoint→重启→验证数据 |
| `test_multiple_checkpoints` | 多轮写入+checkpoint,验证 ID 单调递增 |
| `test_checkpoint_stats_tracking` | Checkpoint 统计记录 |

### 8.2 WAL 恢复测试

**文件**: `src/tests/wal_recovery.rs`

**测试覆盖**:
1. 基本 WAL 重放 (写入后崩溃恢复)
2. 序列号连续性校验
3. Checksum 损坏检测
4. 多 WAL 文件恢复
5. 不完整条目处理

### 8.3 Manifest 崩溃测试

**文件**: `src/compaction/manifest_crash_tests.rs`

**测试场景**:
- 文件截断
- 内容损坏
- 并发写入冲突

### 8.4 故障注入测试

**文件**: `src/io/fault_inject.rs`

- I/O 写入失败注入
- I/O 读取失败注入
- 延迟注入

### 8.5 稳定性测试

**文件**: `tests/stability_24h.rs`

24 小时稳定性测试:
- 持续写入/读取/删除
- 配置 `enable_wal: false` (测试无 WAL 模式)
- 验证长时间运行无内存泄漏

---

## 9. 性能影响数据

### 9.1 WAL 写入性能

| 操作 | 值大小 | FileKV 性能 |
|------|--------|-----------|
| Write (WAL, 64B) | 64 bytes | **1.57 µs/entry** (637K ops/sec) |
| Write (WAL, 1KB) | 1 KB | **3.92 µs/entry** (255K ops/sec) |
| Write (WAL, 4KB) | 4 KB | **10.91 µs/entry** (92K ops/sec) |
| Write (no WAL, 64B) | 64 bytes | **1.17 µs/entry** (854K ops/sec) |

### 9.2 WAL 优化效果

| 优化 | 加速比 |
|------|--------|
| 二进制序列化 (vs JSON) | 3-5x |
| CDict 预创建 | 10-100x |
| 批量 WAL + 定时 fsync | 显著减少 fsync 开销 |
| Channel 异步写入 | 非阻塞 put() |

### 9.3 Checkpoint 性能影响

- **创建开销**: 主要来源于 memtable flush (将内存数据刷到磁盘)
- **磁盘占用**: 完全 checkpoint 需要复制所有 segment 文件
- **恢复加速**: 有 checkpoint 时,只需重放 checkpoint 之后的 WAL 条目

### 9.4 写放大 (WA)

- **写放大: 1.00x** (完美)
- 批量 WAL 优化使写入路径无额外放大
- 相比 RocksDB (~1.0-1.5x) 相当

---

## 10. 关键文件索引

| 文件路径 | 职责 |
|---------|------|
| `src/checkpoint/mod.rs` | Checkpoint 模块入口 |
| `src/checkpoint/manager.rs` | CheckpointManager 核心实现 |
| `src/checkpoint/types.rs` | CheckpointMetadata, CheckpointStats 类型 |
| `src/checkpoint/filekv_impl.rs` | FileKV checkpoint 方法扩展 |
| `src/checkpoint/tests.rs` | Checkpoint 单元测试 |
| `src/core/wal.rs` | WAL 管理器 (写入、恢复、序列化) |
| `src/core/wal_channel.rs` | 异步 WAL channel |
| `src/core/wal_batcher.rs` | WAL 批量写入 |
| `src/core/write_coalescer.rs` | 写入合并 |
| `src/core/memtable.rs` | MemTable 实现 |
| `src/core/flush.rs` | MemTable flush 逻辑 |
| `src/compaction/manifest.rs` | CompactionManifest (段文件清单) |
| `src/compaction/manifest_crash_tests.rs` | Manifest 崩溃测试 |
| `src/tests/wal_recovery.rs` | WAL 恢复测试 |
| `tests/filekv_integration/checkpoint.rs` | Checkpoint 集成测试 |
| `tests/stability_24h.rs` | 24h 稳定性测试 |

---

## 总结

tokitai-filekv 的检查点与恢复机制通过以下创新实现:

1. **WAL 架构成熟**: 二进制序列化、异步 channel、批量写入、定时 fsync
2. **恢复路径完整**: WAL 重放 + checkpoint 快照双重保护
3. **一致性保证强**: 序列号连续性校验、CRC32 checksum、manifest 保护
4. **测试覆盖广**: 集成测试、崩溃测试、故障注入、稳定性测试
5. **性能优秀**: WAL 写入 1.57µs/entry,写放大 1.00x

这些设计使 tokitai-filekv 达到生产级数据可靠性标准。
