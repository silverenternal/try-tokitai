# FileKV 运维手册

**版本**: v0.4.0 规划
**最后更新**: 2026-04-14
**目标读者**: 运维工程师、SRE、DBA

---

## 📋 目录

1. [概述](#1-概述)
2. [部署指南](#2-部署指南)
3. [备份与恢复](#3-备份与恢复)
4. [监控指标解读](#4-监控指标解读)
5. [故障排查指南](#5-故障排查指南)
6. [容量规划](#6-容量规划)
7. [日常运维操作](#7-日常运维操作)
8. [升级与迁移](#8-升级与迁移)
9. [应急响应](#9-应急响应)

---

## 1. 概述

### 1.1 FileKV 运维定位

FileKV 是一个**嵌入式 KV 存储引擎**，不是独立服务进程。运维工作主要集中在：

- **宿主应用**的运维（FileKV 作为库被应用使用）
- **数据目录**的管理（segments、WAL、index、checkpoints）
- **性能监控**和**容量规划**
- **备份恢复**操作

### 1.2 数据目录结构

```
data/
├── segments/          # Segment 文件 (segment_0.log, segment_1.log, ...)
├── wal/               # Write-Ahead Log (wal_0.log, wal_1.log, ...)
├── index/             # 稀疏索引、Bloom Filter、Zone Map
│   ├── sparse/        # 稀疏索引文件
│   ├── zone_map/      # Zone Map 索引
│   ├── l3_bloom/      # L3 Bloom Filter
│   └── bloom/         # 主 Bloom Filter
└── checkpoints/       # 检查点快照
```

### 1.3 配置文件示例

```toml
# filekv_config.toml (应用层配置)
[storage.filekv]
base_dir = "/data/filekv"
wal_enabled = true
wal_sync_mode = "Batch"           # Immediate | Batch | Lazy
enable_bloom = true
enable_zone_map = true
cache_max_memory_bytes = 268435456  # 256MB
segment_max_size_bytes = 67108864   # 64MB
compaction_enabled = true
compaction_level_max_bytes = [67108864, 671088640, 6710886400]  # L0/L1/L2
```

---

## 2. 部署指南

### 2.1 系统要求

| 项目 | 要求 |
|------|------|
| **操作系统** | Linux (x86_64/aarch64), macOS (开发) |
| **Rust 版本** | 1.75+ (Edition 2021) |
| **内存** | 最低 64MB，推荐 256MB+ |
| **磁盘** | SSD 推荐，HDD 可用但性能降低 |
| **文件系统** | ext4, xfs, btrfs (避免 NFS) |

### 2.2 安装步骤

```bash
# 1. 安装 Rust (如未安装)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. 在宿主项目中添加依赖
cargo add tokitai-filekv

# 3. 或启用完整功能
cargo add tokitai-filekv --features full
```

### 2.3 初始化

```rust
use tokitai_filekv::{FileKV, FileKVConfig};

fn init_filekv(data_dir: &str) -> anyhow::Result<FileKV> {
    let mut config = FileKVConfig::balanced();
    config.segment_dir = std::path::PathBuf::from(data_dir).join("segments");
    config.wal_dir = std::path::PathBuf::from(data_dir).join("wal");
    config.index_dir = std::path::PathBuf::from(data_dir).join("index");
    config.checkpoint_dir = std::path::PathBuf::from(data_dir).join("checkpoints");

    // 确保目录存在 (FileKV::open 会自动创建，但显式创建更好)
    std::fs::create_dir_all(&config.segment_dir)?;
    std::fs::create_dir_all(&config.wal_dir)?;
    std::fs::create_dir_all(&config.index_dir)?;
    std::fs::create_dir_all(&config.checkpoint_dir)?;

    FileKV::open(config)
}
```

### 2.4 验证部署

```rust
fn verify_filekv(kv: &FileKV) -> anyhow::Result<()> {
    // 写入测试数据
    kv.put("__health_check__", b"ok")?;

    // 读取验证
    let value = kv.get("__health_check__")?;
    assert_eq!(value.as_deref(), Some(b"ok".as_ref()));

    // 清理
    kv.delete("__health_check__")?;

    // 检查统计
    let stats = kv.get_stats();
    println!("FileKV initialized: {} segments, {} bytes",
             stats.segment_count, stats.total_size_bytes);

    Ok(())
}
```

---

## 3. 备份与恢复

### 3.1 备份策略

FileKV 提供两种备份方式：

| 方式 | 适用场景 | RPO | RTO |
|------|----------|-----|-----|
| **Checkpoint** | 在线热备份 | ~分钟级 | 秒级 |
| **文件快照** | 离线冷备份 | 取决于频率 | 分钟级 |

### 3.2 Checkpoint 备份

```rust
use tokitai_filekv::IncrementalCheckpoint;

fn create_checkpoint(kv: &FileKV, checkpoint_dir: &str) -> anyhow::Result<()> {
    // 创建检查点 (需要在应用层调用)
    // 注意: 当前版本 checkpoint 需要手动触发
    // 建议: 在低峰期或写入间隙创建

    // 示例: 创建 checkpoint
    // let cp = IncrementalCheckpoint::new(checkpoint_dir)?;
    // cp.create(kv)?;

    println!("Checkpoint created at: {}", checkpoint_dir);
    Ok(())
}
```

**操作步骤**:

```bash
# 1. 暂停写入 (如果可能)
# 2. 创建 checkpoint
# 3. 备份整个数据目录
tar czf filekv_backup_$(date +%Y%m%d_%H%M%S).tar.gz \
    --exclude='*.tmp' \
    /data/filekv/

# 4. 传输到备份存储
scp filekv_backup_*.tar.gz backup-server:/backups/filekv/
```

### 3.3 文件级备份

```bash
#!/bin/bash
# filekv_backup.sh - 自动化备份脚本

BACKUP_DIR="/backups/filekv"
DATA_DIR="/data/filekv"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RETENTION_DAYS=30

mkdir -p "$BACKUP_DIR"

# 创建一致性快照 (建议先 flush)
echo "[$(date)] Starting backup..."

# Option 1: 如果应用支持，先 flush
# curl -X POST http://app:8080/admin/filekv/flush

# 备份数据目录
tar czf "$BACKUP_DIR/filekv_${TIMESTAMP}.tar.gz" \
    -C "$(dirname $DATA_DIR)" \
    "$(basename $DATA_DIR)"

# 清理旧备份
find "$BACKUP_DIR" -name "filekv_*.tar.gz" -mtime +$RETENTION_DAYS -delete

echo "[$(date)] Backup complete: filekv_${TIMESTAMP}.tar.gz"
```

### 3.4 恢复操作

```bash
#!/bin/bash
# filekv_restore.sh - 恢复脚本

BACKUP_FILE="$1"
RESTORE_DIR="/data/filekv_restored"

if [[ -z "$BACKUP_FILE" ]]; then
    echo "Usage: $0 <backup_file>"
    exit 1
fi

if [[ ! -f "$BACKUP_FILE" ]]; then
    echo "ERROR: Backup file not found: $BACKUP_FILE"
    exit 1
fi

echo "[$(date)] Restoring from: $BACKUP_FILE"

# 停止应用
# systemctl stop myapp

# 备份当前数据 (以防万一)
if [[ -d /data/filekv ]]; then
    mv /data/filekv /data/filekv_pre_restore_$(date +%s)
fi

# 恢复数据
mkdir -p "$RESTORE_DIR"
tar xzf "$BACKUP_FILE" -C "$RESTORE_DIR"
mv "$RESTORE_DIR/data/filekv" /data/filekv

# 验证恢复
# 启动应用并运行健康检查
# systemctl start myapp
# curl http://app:8080/health

echo "[$(date)] Restore complete"
```

### 3.5 WAL 恢复

FileKV 在启动时自动执行 WAL 恢复：

```rust
// 恢复过程是自动的 - FileKV::open() 会检测 WAL 并恢复
let kv = FileKV::open(config)?;  // 自动恢复未持久化的 WAL 条目
```

**注意事项**:

- WAL 恢复仅在**非正常关闭**后触发
- 正常关闭 (flush + 清理 WAL) 后不需要恢复
- Lazy WAL 模式不保证崩溃安全，可能丢失最后几条写入

---

## 4. 监控指标解读

### 4.1 获取统计信息

```rust
let stats = kv.get_stats();
println!("Segment count: {}", stats.segment_count);
println!("Total size: {} bytes", stats.total_size_bytes);
println!("Write count: {}", stats.write_count);
println!("Read count: {}", stats.read_count);
```

### 4.2 核心指标清单

#### 写入指标

| 指标 | 含义 | 正常范围 | 异常处理 |
|------|------|----------|----------|
| `write_count` | 总写入次数 | 持续增长 | 检查写入频率是否异常 |
| `write_latency_avg` | 平均写入延迟 | <10µs (WAL) | 检查磁盘 I/O |
| `segment_count` | Segment 文件数 | 5-50 | 过多表示 compaction 跟不上 |
| `total_size_bytes` | 总磁盘占用 | 符合预期 | 过大需手动 compaction |

#### 读取指标

| 指标 | 含义 | 正常范围 | 异常处理 |
|------|------|----------|----------|
| `read_count` | 总读取次数 | 持续增长 | - |
| `read_latency_avg` | 平均读取延迟 | <100µs (热缓存) | 检查缓存命中率 |
| `cache_hit_rate` | 缓存命中率 | >70% | 调大 cache 或优化工作负载 |
| `bloom_hit_rate` | Bloom Filter 命中率 | >90% | FPR 过高，调整配置 |

#### Compaction 指标

| 指标 | 含义 | 正常范围 | 异常处理 |
|------|------|----------|----------|
| `compaction_count` | 压缩次数 | 依负载而定 | 频繁压缩需检查阈值 |
| `compaction_duration` | 单次压缩耗时 | <30s | 过长影响读性能 |
| `tombstones_cleaned` | 墓碑清理数 | 持续增长 | - |

#### 内存指标

| 指标 | 含义 | 正常范围 | 异常处理 |
|------|------|----------|----------|
| `memtable_entries` | MemTable 条目数 | <100K | 过多需降低 flush 阈值 |
| `block_cache_items` | BlockCache 条目数 | <配置值 | 持续上升可能泄漏 |
| `block_cache_memory` | BlockCache 内存 | <max_memory | 超过需调大或排查 |
| `bloom_cache_memory` | Bloom 缓存内存 | <配置值 | - |

### 4.3 Prometheus 指标 (启用 `metrics` feature)

```rust
// 启用 Prometheus 指标
#[cfg(feature = "metrics")]
{
    let exporter = tokitai_filekv::PrometheusExporter::new()?;
    exporter.start_http_server("0.0.0.0:9090")?;
}
```

| 指标名称 | 类型 | 标签 | 描述 |
|----------|------|------|------|
| `filekv_get_duration_seconds` | Histogram | `cache_hit` | Get 操作延迟分布 |
| `filekv_put_duration_seconds` | Histogram | - | Put 操作延迟分布 |
| `filekv_cache_hits_total` | Counter | `cache_type` | 缓存命中总数 |
| `filekv_cache_misses_total` | Counter | `cache_type` | 缓存未命中总数 |
| `filekv_compactions_total` | Counter | `level` | Compaction 次数 |
| `filekv_segments_total` | Gauge | - | Segment 文件数 |
| `filekv_memory_bytes` | Gauge | `component` | 各组件内存占用 |

### 4.4 健康检查脚本

```bash
#!/bin/bash
# filekv_health_check.sh

ENDPOINT="http://localhost:9090/metrics"  # Prometheus endpoint

# 检查指标
response=$(curl -s "$ENDPOINT")

# 提取关键指标
segments=$(echo "$response" | grep "filekv_segments_total" | awk '{print $2}')
memory=$(echo "$response" | grep 'filekv_memory_bytes{component="memtable"}' | awk '{print $2}')

echo "Segments: $segments"
echo "MemTable memory: $memory bytes"

# 告警阈值
if [[ $segments -gt 100 ]]; then
    echo "WARNING: Too many segments ($segments) - compaction may be lagging"
fi

# 检查最近是否有 compaction
last_compaction=$(echo "$response" | grep "filekv_compactions_total" | awk '{print $2}')
if [[ -z "$last_compaction" || "$last_compaction" == "0" ]]; then
    echo "WARNING: No compaction has run"
fi
```

---

## 5. 故障排查指南

### 5.1 常见问题

#### 问题 1: 写入延迟突然增加

**症状**: `put()` 延迟从 ~2µs 增加到 >100µs

**排查步骤**:

```bash
# 1. 检查磁盘 I/O
iostat -x 1

# 2. 检查 segment 数量
ls -la /data/filekv/segments/ | wc -l

# 3. 检查是否卡在 flush
# 查看应用日志中的 flush 相关消息
grep -i "flush\|memtable" /var/log/app.log | tail -20
```

**可能原因**:
- 磁盘 I/O 饱和
- Segment 数量过多导致遍历开销
- Compaction 正在运行，占用 I/O

**解决方案**:
- 检查磁盘是否有足够 IOPS
- 手动触发 compaction (如支持)
- 调整 `segment_max_size_bytes` 减少 segment 数量

#### 问题 2: 内存占用持续增长

**症状**: 进程 RSS 随时间线性增长

**排查步骤**:

```bash
# 1. 获取 FileKV 内存统计
# 在应用代码中打印
# let stats = kv.get_stats();

# 2. 检查文件描述符
ls /proc/<pid>/fd | wc -l

# 3. 检查 MMap 映射
cat /proc/<pid>/maps | grep filekv | wc -l
```

**可能原因**:
- BlockCache 未正确淘汰 (FIX-002 已修复字节级 weigher)
- MemTable 未及时 flush
- MMap 未释放

**解决方案**:
- 调小 `cache_max_memory_bytes`
- 降低 `memtable_max_entries`
- 启用更积极的 flush 策略

#### 问题 3: Bloom Filter 负向查询异常慢

**症状**: 不存在的 key 查询延迟 >1ms

**已知原因**: Bloom Filter 重复重建占 40-50% 时间 (规划于 v0.4.0 修复)

**临时方案**:
- 增加 `bloom_cache_ratio`
- 减少 segment 数量 (加快 Bloom 加载)

#### 问题 4: 启动时恢复失败

**症状**: `FileKV::open()` 报错 "WAL recovery failed"

**排查步骤**:

```bash
# 1. 检查 WAL 文件完整性
xxd /data/filekv/wal/wal_latest.log | head -20

# 2. 检查是否有未完成的 WAL 条目
# WAL 条目格式: [magic][length][data][checksum]

# 3. 临时方案: 禁用 WAL 恢复
# (会丢失未持久化的数据)
config.enable_wal = false;  // 仅在调试时使用
```

### 5.2 日志级别配置

```rust
// 使用 tracing 配置日志级别
use tracing::Level;

// 调试模式: 详细日志
tracing_subscriber::fmt()
    .with_max_level(Level::DEBUG)
    .with_target(true)
    .init();

// 生产模式: 仅警告
tracing_subscriber::fmt()
    .with_max_level(Level::WARN)
    .compact()
    .init();
```

**日志模式参考**:

| 级别 | 内容 | 适用场景 |
|------|------|----------|
| `TRACE` | 每次缓存命中/未命中 | 深度调试 |
| `DEBUG` | Compaction 决策、rebalance 决策 | 开发调试 |
| `INFO` | 启动、恢复、checkpoint | 正常运行 |
| `WARN` | 数据完整性问题、性能警告 | 生产监控 |
| `ERROR` | 不可恢复的错误 | 故障告警 |

### 5.3 诊断工具

```rust
// 在应用中添加诊断端点
fn diagnostic_info(kv: &FileKV) -> serde_json::Value {
    let stats = kv.get_stats();

    serde_json::json!({
        "segments": stats.segment_count,
        "total_size_bytes": stats.total_size_bytes,
        "write_count": stats.write_count,
        "read_count": stats.read_count,
        "memtable_entries": stats.memtable_entries,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })
}
```

---

## 6. 容量规划

### 6.1 容量估算公式

```
总磁盘占用 ≈ 原始数据大小 × (1 + 写入放大系数) × 压缩率

其中:
- 写入放大系数 (WAF) ≈ 1.5-3.0 (取决于 compaction 策略)
- 压缩率 ≈ 0.4-0.8 (取决于数据可压缩性)

内存占用 ≈ MemTable + BlockCache + BloomFilter + SparseIndex
         ≈ 64MB + cache_max_memory + bloom_cache + index_size
```

### 6.2 配置预设对照表

| 预设 | 内存 | 适用数据量 | 写入放大 | 读延迟 (P99) |
|------|------|------------|----------|--------------|
| Conservative | ~64MB | <1GB | ~1.5x | ~200µs |
| Balanced | ~256MB | <10GB | ~2.0x | ~100µs |
| Performance | ~1GB | <50GB | ~2.5x | ~50µs |
| Extreme | ~4GB | <100GB | ~3.0x | ~30µs |

### 6.3 扩展建议

| 指标 | 阈值 | 操作 |
|------|------|------|
| Segment 数量 >50 | 调大 `segment_max_size_bytes` |
| Segment 数量 >100 | 检查 compaction 是否正常 |
| BlockCache 命中率 <50% | 调大 `cache_max_memory_bytes` |
| Bloom FPR >1% | 调整 `bloom_fpr` 或增加缓存层 |
| WAL 文件 >10 个 | 检查 WAL 轮转配置 |
| 内存占用 > 预期 150% | 检查是否有内存泄漏 |

### 6.4 生命周期成本

```
每日写入量: 1M entries
平均 entry 大小: 100B
每日原始数据: 100MB
写入放大: 2.0x
每日磁盘占用: 200MB
压缩率: 0.6
实际每日占用: 120MB

月增长: 3.6GB
年增长: 43.8GB

推荐初始磁盘: 100GB (SSD)
推荐 cache 内存: 256MB (Balanced 配置)
```

---

## 7. 日常运维操作

### 7.1 每日检查清单

- [ ] 检查 segment 数量是否正常 (5-50)
- [ ] 检查磁盘占用增长是否符合预期
- [ ] 检查缓存命中率 (>70%)
- [ ] 检查 compaction 是否正常运行
- [ ] 检查错误日志 (无 ERROR 级别)

### 7.2 每周检查清单

- [ ] 运行完整性检查 (checkpoint + 恢复测试)
- [ ] 分析性能趋势 (延迟、吞吐量)
- [ ] 检查内存增长趋势
- [ ] 清理过期备份 (>30 天)

### 7.3 每月检查清单

- [ ] 容量规划评估 (磁盘使用预测)
- [ ] 配置优化评估 (缓存大小、compaction 阈值)
- [ ] 灾难恢复演练
- [ ] 版本更新评估 (新版本 FileKV)

### 7.4 手动操作

#### 手动 Flush

```rust
// 强制将 MemTable 刷到磁盘
kv.flush_memtable()?;
```

#### 手动 Compaction

```rust
// 触发同步 compaction (会阻塞调用线程)
kv.run_compaction()?;

// 异步 compaction (后台运行)
// 注意: 需要 compaction_engine 支持
```

#### 手动 Checkpoint

```rust
// 创建 checkpoint
let checkpoint_dir = "/data/filekv/checkpoints/manual";
// 当前版本需要手动实现 checkpoint 逻辑
// 推荐使用 IncrementalCheckpoint 类型
```

---

## 8. 升级与迁移

### 8.1 版本升级流程

```bash
# 1. 创建完整备份
./scripts/backup.sh full

# 2. 验证备份完整性
./scripts/backup.sh verify

# 3. 升级 Cargo.toml 中的版本
# [dependencies]
# tokitai-filekv = "0.4"

# 4. 重新编译应用
cargo build --release

# 5. 重启应用 (FileKV 会自动打开现有数据)
# systemctl restart myapp

# 6. 运行健康检查
curl http://app:8080/health

# 7. 保留旧版本回滚窗口 (7 天)
```

### 8.2 兼容性矩阵

| 从版本 | 到版本 | 数据兼容 | 操作 |
|--------|--------|----------|------|
| 0.1.x | 0.2.x | ✅ 兼容 | 直接升级 |
| 0.2.x | 0.3.x | ✅ 兼容 | 直接升级 |
| 0.3.x | 0.4.x | ✅ 兼容 | 直接升级 (规划中) |
| 0.x.x | 1.0.0 | ✅ 兼容 (目标) | 直接升级 (规划中) |

### 8.3 迁移到其他存储

```bash
# 导出所有 KV 数据 (伪代码)
for segment in /data/filekv/segments/segment_*.log; do
    # 解析 segment 格式并导出
    parse_segment "$segment" >> export.csv
done

# 导入到目标存储
# (根据目标存储的导入工具)
```

---

## 9. 应急响应

### 9.1 故障等级定义

| 等级 | 定义 | 响应时间 | 处理流程 |
|------|------|----------|----------|
| P0 | 数据丢失/损坏 | 15 分钟 | 立即停机，恢复备份 |
| P1 | 写入/读取完全不可用 | 30 分钟 | 切换备用，排查原因 |
| P2 | 性能严重降级 | 2 小时 | 限流，扩容，优化配置 |
| P3 | 轻微性能下降 | 24 小时 | 分析指标，调整配置 |

### 9.2 P0 响应流程: 数据损坏

```bash
# 1. 立即停止写入
# systemctl stop myapp

# 2. 备份当前状态 (用于事后分析)
cp -r /data/filekv /data/filekv_corrupted_$(date +%s)

# 3. 从最新备份恢复
./scripts/filekv_restore.sh /backups/filekv/latest.tar.gz

# 4. 验证恢复
# 启动应用并运行健康检查

# 5. 分析损坏原因
# 检查日志、磁盘 SMART 数据、内核日志
dmesg | grep -i "error\|io\|disk"
smartctl -a /dev/sda
```

### 9.3 P1 响应流程: 服务不可用

```bash
# 1. 检查磁盘空间
df -h /data

# 2. 检查文件描述符
ulimit -n
ls /proc/<pid>/fd | wc -l

# 3. 检查 OOM Killer
dmesg | grep -i "oom\|kill"

# 4. 尝试重启
# systemctl restart myapp

# 5. 如果启动失败，尝试禁用 WAL 恢复
# (会丢失最后几条写入)
```

### 9.4 紧急联系人

| 角色 | 联系方式 | 升级条件 |
|------|----------|----------|
| 一线运维 | oncall@company.com | 所有故障 |
| 存储工程师 | storage-team@company.com | P0/P1 |
| 架构师 | architect@company.com | P0 且 30 分钟未解决 |

---

## 附录 A: 命令速查表

| 操作 | 命令 |
|------|------|
| 检查 segment 数量 | `ls /data/filekv/segments/segment_*.log \| wc -l` |
| 检查 WAL 文件 | `ls -lt /data/filekv/wal/` |
| 查看索引大小 | `du -sh /data/filekv/index/` |
| 检查进程内存 | `ps aux \| grep <app> \| awk '{print $6}'` |
| 检查文件描述符 | `ls /proc/<pid>/fd \| wc -l` |
| 检查磁盘 I/O | `iostat -x 1` |
| 检查 OOM | `dmesg \| grep -i oom` |
| 运行稳定性测试 | `./scripts/run_stability_tests.sh --quick` |
| 运行高并发测试 | `cargo test --test filekv_integration --release -- --ignored` |

## 附录 B: 配置文件模板

```toml
# FileKV 生产配置模板 (Balanced 预设)
# 适用于: <10GB 数据，QPS <1000

[storage.filekv]
# 数据目录
segment_dir = "/data/filekv/segments"
wal_dir = "/data/filekv/wal"
index_dir = "/data/filekv/index"
checkpoint_dir = "/data/filekv/checkpoints"

# WAL 配置
wal_enabled = true
wal_sync_mode = "Batch"           # Immediate | Batch | Lazy
wal_max_size_bytes = 67108864     # 64MB
wal_max_files = 10

# 内存配置
cache_max_memory_bytes = 268435456  # 256MB
memtable_max_entries = 100000

# Segment 配置
segment_max_size_bytes = 67108864   # 64MB
block_size = 4096                   # 4KB

# Compaction 配置
compaction_enabled = true
compaction_max_concurrent = 2
compaction_level_thresholds = [67108864, 671088640, 6710886400]  # L0: 64MB, L1: 640MB, L2: 6.4GB

# 功能开关
enable_bloom = true
enable_zone_map = true
enable_adaptive_bloom_cache = true
enable_sequential_prefetch = true
```

## 附录 C: 相关文档

| 文档 | 链接 |
|------|------|
| 用户指南 | [FILEKV_GUIDE.md](FILEKV_GUIDE.md) |
| 项目状态 | [POSITION_AND_STATUS.md](POSITION_AND_STATUS.md) |
| 性能基准报告 | [../PERFORMANCE_BENCHMARK_REPORT.md](../PERFORMANCE_BENCHMARK_REPORT.md) |
| RocksDB 公平对比 | [rocksdb_fair_comparison_2026_04_08.md](rocksdb_fair_comparison_2026_04_08.md) |
| 测试策略 | [../../docs/TEST_STRATEGY.md](../../docs/TEST_STRATEGY.md) |
| BlockCache 设计 | [../../docs/plans/PROD-001-blockcache-dynamic-shrink-design.md](../../docs/plans/PROD-001-blockcache-dynamic-shrink-design.md) |
