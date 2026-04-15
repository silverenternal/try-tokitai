# 异步 I/O 集成设计文档

**日期**: 2026-04-12
**状态**: 待实施
**目标**: 将 AsyncWriter 集成到生产写入路径，同时提供同步和异步 API

---

## 1. 架构设计

### 1.1 双 API 表面

```rust
// 同步 API（保持向后兼容）
impl FileKV {
    pub fn put(&self, key: &str, value: &[u8]) -> anyhow::Result<()>
    pub fn delete(&self, key: &str) -> anyhow::Result<()>
}

// 异步 API（新增）
impl FileKV {
    pub async fn put_async(&self, key: &str, value: &[u8]) -> anyhow::Result<()>
    pub async fn delete_async(&self, key: &str) -> anyhow::Result<()>
    pub async fn flush_async(&self) -> anyhow::Result<()>
}
```

### 1.2 写入路径

```
put() [同步]          → WriteEngine::put() → put_buffered() → WAL 同步写入
                                                  ↓
                                           flush_memtable() → segment 同步写入

put_async() [异步]    → WriteEngine::put_async() → put_buffered_async() → WAL 异步写入
                                                        ↓
                                                 flush_memtable_async() → segment 异步写入 (AsyncWriter)
```

### 1.3 同步桥接

```rust
// 同步方法内部使用 block_on 桥接异步
pub fn put(&self, key: &str, value: &[u8]) -> anyhow::Result<()> {
    if self.config.async_io_enabled {
        // 使用 tokio runtime block_on
        tokio::runtime::Handle::current()
            .block_on(self.put_async(key, value))
    } else {
        // 原有同步路径
        self.write_engine.put(key, value)
    }
}
```

---

## 2. 核心组件

### 2.1 AsyncWriter 改造

**当前问题**:
- 所有方法返回 `impl Future<Output = Result<AsyncWriteResult, AsyncIoError>>`
- 没有同步桥接方法

**改造内容**:
```rust
impl AsyncWriter {
    // 新增同步桥接方法
    pub fn write_segment_sync(&self, ...) -> Result<AsyncWriteResult, AsyncIoError> {
        self.runtime.block_on(self.write_segment(...))
    }
    
    pub fn write_wal_sync(&self, ...) -> Result<AsyncWriteResult, AsyncIoError> {
        self.runtime.block_on(self.write_wal(...))
    }
}
```

### 2.2 WriteEngine 改造

**新增异步方法**:
```rust
impl WriteEngine {
    // 异步写入路径
    pub async fn put_async(&self, key: &str, value: &[u8]) -> anyhow::Result<()> {
        self.put_buffered_async(key, value).await
    }
    
    async fn put_buffered_async(&self, key: &str, value: &[u8]) -> anyhow::Result<()> {
        // 1. 插入 memtable（同步）
        // 2. WAL 异步写入（AsyncWriter）
        // 3. Coalescer 处理（可能需要异步化）
    }
    
    pub async fn flush_memtable_async(&self) -> anyhow::Result<()> {
        // 使用 AsyncWriter 写入 segment 文件
    }
}
```

### 2.3 FileKV 改造

**新增公开异步 API**:
```rust
impl FileKV {
    #[cfg(feature = "async-io")]
    pub async fn put_async(&self, key: &str, value: &[u8]) -> anyhow::Result<()> {
        self.write_engine.put_async(key, value).await
    }
    
    #[cfg(feature = "async-io")]
    pub async fn delete_async(&self, key: &str) -> anyhow::Result<()> {
        // 异步删除实现
    }
    
    #[cfg(feature = "async-io")]
    pub async fn flush_async(&self) -> anyhow::Result<()> {
        self.write_engine.flush_memtable_async().await
    }
}
```

---

## 3. 实施步骤

### Phase 1: AsyncWriter 同步桥接 (1-2 小时)
1. 在 AsyncWriter 添加 `runtime: Arc<tokio::runtime::Runtime>` 字段
2. 添加 `write_segment_sync()`, `write_wal_sync()` 等方法
3. 编写同步桥接的单元测试

### Phase 2: WriteEngine 异步路径 (2-3 小时)
1. 添加 `put_async()`, `put_buffered_async()` 方法
2. 添加 `flush_memtable_async()` 方法
3. WAL 写入切换到 AsyncWriter
4. segment 写入切换到 AsyncWriter

### Phase 3: FileKV 公开 API (1 小时)
1. 添加 `put_async()`, `delete_async()`, `flush_async()`
2. 修改同步 `put()` 使用桥接（当 async-io 启用时）
3. 更新文档注释

### Phase 4: 测试验证 (2-3 小时)
1. 异步 API 的集成测试
2. 同步桥接的性能测试
3. 并发压力测试
4. 更新 cargo test --doc

### Phase 5: 文档更新 (1 小时)
1. README 添加异步 API 示例
2. CHANGELOG.md 记录变更
3. 更新 FILEKV_GUIDE.md

---

## 4. 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| block_on() 死锁 | 高 | 使用 `Handle::current().block_on()` 而非创建新 runtime |
| 性能退化 | 中 | 提供 benchmark 对比，文档说明同步桥接的性能损失 |
| API 表面增大 | 低 | 保持向后兼容，异步 API 仅当 feature 启用时可见 |
| 测试复杂度 | 中 | 异步测试独立 feature gate，不影响默认测试 |

---

## 5. 成功标准

- [ ] `cargo check --all-features` 零错误零警告
- [ ] `cargo test --all-features` 包含异步 API 测试
- [ ] `cargo clippy --all-features` 零警告
- [ ] 异步 put_async() 性能优于同步 put() 至少 10%（高并发场景）
- [ ] 同步 put() 性能退化不超过 5%
- [ ] 文档完整，包含异步 API 示例

---

## 6. 后续优化

- 后台 compaction 使用异步 I/O（天然适合异步）
- 后台 flush 使用异步 I/O
- 可选：添加批量异步 API `put_batch_async()`
