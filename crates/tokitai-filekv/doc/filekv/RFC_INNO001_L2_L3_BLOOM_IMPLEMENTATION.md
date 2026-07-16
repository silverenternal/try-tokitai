# RFC: INNO-001 L2/L3 自适应 Bloom 缓存完整实现

## 1. 背景

### 1.1 当前状态
- L1 工作正常 (DashMap 存储 BloomFilter)
- L2 decompress() 永远返回错误
- L2CacheEntry::new() 使用 dummy data
- L3 完全没有磁盘 I/O 实现
- 实际工作率: 33% (只有 L1)

### 1.2 根本原因
- `bloom = "0.3"` crate 不暴露内部 bit vector
- 无法从 bits 重建 BloomFilter (没有 from_bits 构造函数)
- 因此无法实现 bit 级别的压缩/解压

## 2. 设计方案

### 2.1 方案选择: 混合方案

**L2 (温点缓存)**:
- 存储: 序列化 BloomFilter 的 keys + zstd 压缩
- 元数据: num_bits, num_hashes, original_fpr
- 重建: 解压 -> 反序列化 keys -> 重建 BloomFilter::with_rate(fpr, num_keys) -> 重新插入所有 keys
- 预期延迟: ~500ns (解压 + 重建)
- 内存占用: 压缩后约原始大小的 20-40%

**L3 (冷点缓存)**:
- 存储: 完整 bloom 文件 (keys 格式) 到磁盘
- 文件格式: [magic: u32][version: u32][num_bits: u32][num_hashes: u32][fpr: f32][num_keys: u64][keys...]
- 重建: 读取文件 -> 重建 BloomFilter
- 预期延迟: ~10us (磁盘 I/O)
- 磁盘占用: 每个 bloom 文件约几 KB 到几十 KB

### 2.2 L2 存储格式

```
L2CompressedEntry:
  [header: L2Header]
  [compressed_keys: Vec<u8>]  // zstd 压缩后的 keys

L2Header:
  magic: u32 (0x4C32424C = "L2BL")
  version: u32 (1)
  num_bits: u32
  num_hashes: u32
  original_fpr: f32
  num_keys: u64
  compressed_size: u32
  checksum: u32 (CRC32C)
```

### 2.3 L3 磁盘文件格式

```
文件路径: {l3_index_dir}/bloom_{segment_id}.bloom

文件格式:
  [magic: u32 (0x424C4F4F = "BLOO")]
  [version: u32 (2)]  // 升级到 version 2
  [num_bits: u32]
  [num_hashes: u32]
  [original_fpr: f32]
  [num_keys: u64]
  [keys...]  // 每个 key: [key_len: u32][key_bytes: key_len]
  [checksum: u32 (CRC32C)]
```

## 3. 实施计划

### 3.1 修改 adaptive_bloom_cache.rs

- 重新实现 L2CacheEntry::new() - 存储压缩的 keys
- 重新实现 L2CacheEntry::decompress() - 解压并重建 BloomFilter
- 实现 L3 磁盘读写: load_from_disk(), save_to_disk()
- 激活 insert_l2(), evict_l2(), migrate_l1_to_l2()
- 修复 L2 命中路径 (当前 fall through 到 L3)

### 3.2 修改 bloom.rs

- 升级 bloom 文件格式到 version 2
- 添加 num_bits, num_hashes, fpr 元数据
- 添加 checksum 验证

### 3.3 修改 read_engine.rs

- 将 bloom_filter_cache 字段类型从 Arc<BloomFilterCache> 改为 Arc<AdaptiveBloomCache>
- 确保 feature flag 控制生效

### 3.4 激活 MigrationController

- 在 ReadEngine 或后台线程中消费 MigrationDecision
- 实现自动迁移逻辑

## 4. 性能预期

| 层 | 延迟 | 命中率 | 内存/磁盘 |
|----|------|--------|----------|
| L1 | <100ns | 热数据 ~80% | 内存 ~100KB/segment |
| L2 | ~500ns | 温数据 ~15% | 内存 ~20-40KB/segment (压缩) |
| L3 | ~10us | 冷数据 ~4% | 磁盘 ~50-200KB/segment |
| Disk | ~100us | ~1% | 磁盘 ~50-200KB/segment |

## 5. 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| L2 重建开销大 | 延迟增加 | 基准测试验证,若不达标则简化 L2 |
| zstd 依赖 | 增加 crate 大小 | 可替换为 gzip 或 lz4 |
| 磁盘 I/O 慢 | L3 延迟高 | 使用异步 I/O (future) |
