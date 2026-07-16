# PERF-007: 10M keys 写入性能优化方案

> **作者**: LSM-Tree 写入性能优化专家
> **日期**: 2026-04-14
> **状态**: 设计阶段
> **目标**: 将 tokitai-filekv 的 10M keys 写入性能优化至比 RocksDB 慢 <10x（当前 161x）

---

## 1. 当前写入路径分析

### 1.1 单次 put() 完整流程

从代码分析，`put()` 调用经历以下完整路径：

```
put(key, value)
  │
  ├─ 1. 背压检查: memtable.should_apply_backpressure()
  │     └─ 若触发，先 flush_memtable()
  │
  ├─ 2. put_buffered(key, value)
  │     │
  │     ├─ 2a. WAL 写入 (src/core/wal.rs:211)
  │     │     ├─ 计算 XXH3 checksum (hasher.write(value))
  │     │     ├─ 构建 payload (len_bytes + hash_bytes + value)
  │     │     ├─ wal_guard.log_with_payload(op, payload)
  │     │     │     ├─ 序列化 WalEntry 为 JSON (serde_json::to_string)
  │     │     │     ├─ 写入 write_buffer (Vec<u8>)
  │     │     │     └─ apply_sync_policy() (根据 sync_mode 决定 fsync)
  │     │     └─ 更新 stats (atomic fetch_add)
  │     │
  │     ├─ 2b. MemTable 插入 (src/core/memtable.rs:149)
  │     │     ├─ seq_num.fetch_add()
  │     │     ├─ Bytes::copy_from_slice(value)  ← 分配新 Bytes
  │     │     ├─ DashMap::insert(key, entry)    ← key.to_string() 分配
  │     │     ├─ 计算 size delta (原子 fetch_add/sub)
  │     │     └─ entry_count.fetch_add()
  │     │
  │     ├─ 2c. Write Coalescer (src/core/write_coalescer.rs:95)
  │     │     ├─ write_coalescer.add(key.to_string(), value.to_vec())  ← 又一次分配
  │     │     └─ 若触发 flush，返回 batch 调用 flush_batch_to_wal_and_memtable()
  │     │
  │     ├─ 2d. 刷盘检查: memtable.should_flush()
  │     │     └─ 若触发，调用 flush_memtable()
  │     │
  │     └─ 2e. Compaction 检查: compaction_manager.record_write()
  │           └─ 若触发，调用 maybe_run_compaction()
  │
  └─ 3. 审计日志 (若启用)
```

### 1.2 瓶颈识别

| # | 瓶颈点 | 代码位置 | 预估影响 | 说明 |
|---|--------|---------|---------|------|
| B1 | **单次 WAL 序列化** | `wal.rs:211-232` | 高 | 每次 `put()` 都调用 `serde_json::to_string()` 序列化整个 WalEntry，JSON 序列化开销显著 |
| B2 | **WAL 多次 fsync** | `wal.rs:248-268` | 高 | `Immediate` 模式下每次写入都 fsync；`Batch` 模式虽然减少 fsync，但仍逐条序列化 |
| B3 | **内存分配泛滥** | `memtable.rs:149-170` | 中高 | 每次 `put()`: `String` 分配 (key.to_string()) + `Bytes::copy_from_slice` + `Vec<u8>` (write coalescer) = 至少 3 次堆分配 |
| B4 | **Write Coalescer 重复分配** | `write_coalescer.rs:95-130` | 中 | `add()` 再次 `key.to_string()` 和 `value.to_vec()`，与 WAL/MemTable 重复 |
| B5 | **MemTable 全排序刷盘** | `write_engine.rs:540` | 中 | `entries_sorted()` 每次刷盘都 O(n log n) 排序，即使数据本已有序 |
| B6 | **Compaction 写放大** | `compaction/mod.rs:500-700` | 高 | L0 段使用 size-tiered 合并，但 leveled 策略下 L0→L1 仍需重写全部数据 |
| B7 | **Atomic 操作频繁** | 全路径 | 低中 | 每次 `put()` 调用 5+ 个 atomic fetch_add (stats, size, seq, count) |
| B8 | **Checksum 重复计算** | `write_engine.rs:227-232` + `wal.rs` | 低 | put_buffered 计算一次 XXH3，WAL 内又可能计算一次 |

---

## 2. 优化方案

### 优化 1：批量 WAL 写入（优先级 P0）

**当前问题**：
- `put_buffered()` 在 `src/engine/write_engine.rs:224-245` 中，**每次** `put()` 都单独调用 `wal_guard.log_with_payload()`
- 即使 Write Coalescer 会合并批次，单条写入仍然逐条序列化 WAL Entry
- `serde_json::to_string()` 对每个 WalEntry 独立序列化，无法利用批量编码优势

**优化方案**：引入 `WalBatcher` 合并 N 次 `put()` 为单次批量 WAL 写入

**实现要点**：

```rust
pub struct WalBatcher {
    buffer: Vec<(String, Vec<u8>)>,
    max_batch_size: usize,        // 批次大小阈值 (如 1024 条)
    max_batch_time: Duration,     // 时间窗口 (如 10ms)
    last_flush: Instant,
}

impl WalBatcher {
    /// 添加写入，返回是否达到批次阈值
    pub fn add(&mut self, key: &str, value: &[u8]) -> bool {
        self.buffer.push((key.to_string(), value.to_vec()));
        self.buffer.len() >= self.max_batch_size
            || self.last_flush.elapsed() >= self.max_batch_time
    }

    /// 将批次写入 WAL (单次 fsync)
    pub fn flush(&mut self, wal: &mut WalManager) -> Result<()> {
        if self.buffer.is_empty() { return Ok(()); }
        wal.log_batch(&self.buffer)?;  // 单次批量序列化 + 单次 fsync
        self.buffer.clear();
        self.last_flush = Instant::now();
        Ok(())
    }
}
```

关键改进：
- 在 `put()` 路径上累积写入，不立即写 WAL
- 达到批次阈值时，调用 `wal.log_batch()` 一次性写入所有条目
- `log_batch()` 已在 `wal.rs:401-450` 存在，需优化其序列化格式（使用 bincode 替代 JSON）

**预期收益**：
- syscall 减少 50%+ (N 次 write → 1 次 write)
- fsync 减少 N 倍 (N 次 fsync → 1 次 fsync)
- WAL 序列化开销降低 60%+ (批量 bincode vs 逐条 JSON)

**风险**：
- 需要确保 fsync 时机以保证持久性
- 崩溃恢复可能丢失批次内数据（需在 shutdown 时强制 flush）
- 缓解措施：提供 `Durability::Immediate` 绕过批处理用于关键写入

---

### 优化 2：Compaction 策略优化（优先级 P0）

**当前策略**：
- `src/compaction/mod.rs` 实现了 leveled compaction 框架
- L0 触发：`l0_file_count_threshold` (默认 4 个文件)
- L1+ 触发：基于 level budget (`target_segment_size * level_multiplier^(level-1)`)
- 但实际合并时使用 **streaming merge iterator**，所有段都参与合并
- 输出到 `max_input_level + 1`，导致 L0→L1 写放大

**优化方案**：混合 Compaction 策略 (Size-Tiered L0 + Leveled L1+)

**实现要点**：

```
L0 (memtable flushes)     → 使用 size-tiered 合并
L1 (compacted, sorted)    → 使用 leveled 合并
L2/L3 (compacted, sorted) → 使用 leveled 合并
```

1. **L0 Size-Tiered 合并**：
   - 当 L0 文件数 >= `l0_file_count_threshold` 时，合并所有 L0 文件
   - 合并时保持 L0 文件的有序特性，输出到 L1
   - 减少写放大：避免 L0 文件与 L1 文件的重叠合并

2. **L1+ Leveled 合并**：
   - 保持当前 leveled 策略
   - L1 文件与 L1 文件合并（而非 L0 与 L1 合并）
   - 利用 L1 已排序特性，减少合并范围

3. **关键修改** (`src/compaction/mod.rs:230-280` `select_leveled_segments`):

```rust
fn select_leveled_segments(...) -> (Vec<u64>, u8) {
    // L0: 使用 size-tiered (合并所有 L0 文件)
    if let Some(l0_segs) = levels.get(&0) {
        if l0_segs.len() >= config.l0_file_count_threshold {
            // 合并所有 L0，输出到 L1
            return (l0_segs.clone(), 0);
        }
    }

    // L1+: 使用 leveled (同层合并)
    for (level, seg_ids) in &levels {
        if *level == 0 { continue; }
        let total_size: u64 = seg_ids.iter()...sum();
        let level_budget = ...;

        if total_size > level_budget {
            // 同层合并：L1→L1, L2→L2
            // 而非当前 L0+L1→L2
            return (seg_ids.clone(), *level);
        }
    }
}
```

4. **Write Engine 修改** (`src/engine/write_engine.rs:193-210`):
   - L0 段标记为 `level: 0`
   - Compaction 输出保持 level 连续性

**预期收益**：
- L0→L1 写放大减少 2x+ (避免 L0 与 L1 重叠合并)
- Compaction 延迟降低 30-50%
- 读性能保持 (L1+ 仍为 leveled，保证 O(log N) 查找)

**风险**：
- L0 文件数可能短暂增加
- 需要调整 `l0_file_count_threshold` 参数
- 缓解措施：添加 L0 文件大小上限检查

---

### 优化 3：内存分配优化（优先级 P1）

**问题**：
- 每次 `put()` 至少 3 次堆分配：
  1. `key.to_string()` (write_engine.rs:224)
  2. `Bytes::copy_from_slice(value)` (memtable.rs:151)
  3. `value.to_vec()` (write_coalescer.rs:95)
- 对于 10M keys，这些分配累积成显著开销

**方案**：对象池 + 零拷贝路径

1. **String 对象池**：

```rust
pub struct KeyPool {
    pool: crossbeam_queue::ArrayQueue<String>,
}

impl KeyPool {
    pub fn acquire(&self, key: &str) -> String {
        if let Some(mut s) = self.pool.pop() {
            s.clear();
            s.push_str(key);
            s
        } else {
            key.to_string()
        }
    }

    pub fn release(&self, s: String) {
        if s.capacity() <= 256 { // 限制池大小
            let _ = self.pool.push(s);
        }
    }
}
```

2. **Bytes 零拷贝路径**：
   - 对于小 value (< 256 bytes)，使用 `Bytes::from_static()` 避免复制
   - 对于大 value，复用预分配缓冲区

3. **Write Coalescer 优化**：
   - 直接引用 MemTable 的 `Bytes`，避免 `value.to_vec()` 复制

**预期收益**：
- 减少分配开销 10-20%
- 减少 GC 压力 (更少的 Vec<String> 创建/销毁)

**风险**：
- 对象池增加内存占用
- 需要谨慎管理对象生命周期
- 缓解措施：池大小可配置，支持禁用

---

### 优化 4：MemTable 刷盘优化（优先级 P1）

**问题**：
- `write_engine.rs:540-600`: `flush_memtable()` 每次刷盘：
  1. `entries_sorted()` 全排序 (O(n log n))
  2. 逐条写入 segment 文件
  3. 每次 append 后 `flush()` (write_engine.rs:593)
- 频繁刷盘导致 I/O 放大

**方案**：

1. **增大刷盘阈值（可配置）**：
   ```rust
   // 当前默认 4MB，可提升至 16-32MB
   flush_threshold_bytes: 16 * 1024 * 1024,
   ```

2. **批量写入 segment**：
   ```rust
   // 当前：逐条 append + flush
   // 优化：批量写入后单次 flush
   let batch_size = 1024; // 每 1024 条 flush 一次
   for (i, (key, entry)) in entries.iter().enumerate() {
       segment.append(key, value)?;
       if i % batch_size == 0 {
           writer.flush()?;
       }
   }
   ```

3. **后台异步刷盘**：
   ```rust
   pub struct AsyncFlusher {
       tx: mpsc::Sender<Vec<(String, MemTableEntry)>>,
       handle: JoinHandle<()>,
   }
   // MemTable 达到阈值时，将数据发送后台线程刷盘
   // put() 不阻塞，继续接收新写入
   ```

**预期收益**：
- 刷盘延迟降低 30%+
- I/O 次数减少 2-3x
- put() 延迟降低 (异步刷盘不阻塞)

**风险**：
- 异步刷盘增加崩溃数据丢失窗口
- 需要 WAL 保证恢复
- 缓解措施：确保 WAL 在 put() 时已写入

---

## 3. 实施优先级

| 优化项 | 优先级 | 预期收益 | 实现难度 | 风险 | 预估工期 |
|--------|--------|---------|---------|------|---------|
| 批量 WAL | P0 | syscall 减少 50%+, fsync 减少 Nx | 中 | 低 (WAL 持久性必须保证) | 2-3 天 |
| Compaction 优化 | P0 | L0→L1 写放大减少 2x+ | 高 | 中 (需调整参数) | 4-5 天 |
| 刷盘优化 | P1 | 刷盘延迟降低 30%+ | 中 | 中 (异步刷盘需谨慎) | 3-4 天 |
| 内存分配 | P1 | 分配开销减少 10-20% | 低 | 低 | 2-3 天 |

**推荐实施顺序**：
1. 批量 WAL (最大收益，最低风险)
2. 刷盘优化 (配合批量 WAL，进一步减少 I/O)
3. Compaction 优化 (架构性改动，需充分测试)
4. 内存分配优化 (收益较小，最后实施)

---

## 4. 总体目标

| 指标 | 当前 | 目标 | 备注 |
|------|------|------|------|
| 10M keys 写入 (vs RocksDB) | 161x 慢 | <10x 慢 | 主要优化批量写入 |
| 批量写入吞吐 | 基准 | 5x+ 提升 | 批量 WAL + 刷盘优化 |
| Compaction 写放大 | 未知 | <3x | L0 size-tiered 减少放大 |
| 单次 put() P99 延迟 | 未知 | <500μs | 内存分配 + WAL 优化 |
| MemTable 刷盘延迟 | 未知 | <100ms | 批量刷盘优化 |

---

## 5. 风险评估

### 5.1 数据安全性风险

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| 批量 WAL 丢失批次内数据 | 崩溃时未 fsync 的批次丢失 | 1. shutdown 时强制 flush<br>2. 提供 `Durability::Immediate` 绕过<br>3. 定期 fsync (可配置间隔) |
| 异步刷盘崩溃恢复 | MemTable 数据未刷盘 | WAL 保证恢复，MemTable 数据可重建 |
| Compaction 崩溃中断 | 中间状态数据丢失 | 已有的 `CompactionManifest` 机制保证 crash-safe |

### 5.2 一致性风险

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| 批量 WAL 部分写入 | 批次内部分条目未写入 | 使用 `log_batch()` 原子写入 (当前已支持) |
| 并发写入排序 | 多线程 put() 批次交错 | 批次内部排序，确保 segment 有序 |

### 5.3 向后兼容性

- 所有优化通过配置开关控制，默认启用
- WAL 格式向后兼容 (bincode vs JSON 需提供迁移路径)
- 现有数据文件无需迁移

### 5.4 性能回归风险

- Compaction 策略调整可能导致短期读性能下降 (L0 文件数增加)
- 缓解措施：逐步调整 `l0_file_count_threshold`，监控读延迟

---

## 6. 监控与度量

实施优化后，需添加以下度量指标：

| 指标 | 说明 | 告警阈值 |
|------|------|---------|
| `wal_batch_size` | WAL 批次大小分布 | P50 < 10 条目 |
| `wal_flush_interval` | WAL fsync 间隔 | P99 > 100ms |
| `memtable_flush_duration` | 刷盘耗时 | P99 > 500ms |
| `compaction_write_amplication` | 写放大倍数 | >5x |
| `allocation_rate` | 每秒堆分配量 | 显著上升 |
| `batch_write_latency` | 批量写入延迟 | P99 > 10ms |

---

## 附录 A: 相关代码索引

| 文件 | 行号 | 说明 |
|------|------|------|
| `src/engine/write_engine.rs` | 217-290 | `put_buffered()` 主路径 |
| `src/engine/write_engine.rs` | 292-310 | `put()` 入口 |
| `src/engine/write_engine.rs` | 498-650 | `flush_memtable()` |
| `src/core/wal.rs` | 205-235 | `log_with_payload()` |
| `src/core/wal.rs` | 401-450 | `log_batch()` |
| `src/core/memtable.rs` | 145-175 | `insert()` |
| `src/core/memtable.rs` | 177-220 | `insert_batch()` |
| `src/core/write_coalescer.rs` | 90-140 | `add()` |
| `src/compaction/mod.rs` | 230-280 | `select_leveled_segments()` |
| `src/compaction/mod.rs` | 500-700 | `execute_streaming_compaction()` |
| `src/core/segment.rs` | 575-620 | `append()` |

## 附录 B: 参考文献

- RocksDB Write Buffer 设计: https://github.com/facebook/rocksdb/wiki/Write-Buffer
- LevelDB Compaction 策略: https://github.com/google/leveldb/blob/main/doc/impl.md
- LSM-Tree 写优化论文: https://www.cs.umb.edu/~poneil/lsmtree.pdf
