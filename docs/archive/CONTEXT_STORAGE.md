# 纯文件上下文存储系统

基于"文件即数据库"理念实现的轻量级上下文存储系统，使用分层文件目录 + 哈希符号链接 + 增量日志，实现本地完整存储 + 云端最小传输的核心逻辑。

## 核心特性

- **无数据库依赖**：全程基于文件系统，无需安装/配置 SQLite
- **三层存储架构**：瞬时层、短期层、长期层，按需存储
- **哈希去重**：SHA256 内容哈希，自动去重节省空间
- **符号链接索引**：快速哈希→路径映射，检索高效
- **增量日志**：所有变更可追溯、可审计
- **mmap 优化**：大文件内存映射，减少 IO 开销
- **自动裁剪**：短期层自动保留最近 N 轮，超出自动删除

## 目录结构

```
.context/
├── sessions/          # 会话级目录
│   └── sess_xxx/      # 单个会话目录
│       ├── transient/ # 瞬时层：单轮临时文件
│       ├── short-term/# 短期层：最近 N 轮
│       └── long-term/ # 长期层：项目习惯/规则
│           ├── git_rules/
│           ├── tool_configs/
│           └── task_patterns/
├── hashes/            # 哈希索引目录（符号链接）
└── logs/              # 增量日志
    └── context_append.log
```

## 使用示例

### 基本使用

```rust
use crate::context::{
    FileContextServiceImpl, FileContextConfig,
    FileContextService, ContentType,
};

// 创建服务实例
let config = FileContextConfig {
    max_short_term_rounds: 10,  // 短期层保留 10 轮
    enable_mmap: true,          // 启用 mmap 优化
    enable_logging: true,       // 启用日志
};

let mut service = FileContextServiceImpl::new("./.context", config)?;

// 添加内容到短期层
let hash = service.add(
    "session_123",
    b"这是上下文内容",
    ContentType::ShortTerm,
)?;
println!("内容哈希：{}", hash);

// 通过哈希获取内容
let content = service.get_by_hash(&hash)?;
println!("内容：{:?}", String::from_utf8_lossy(&content));

// 获取摘要
if let Some(summary) = service.get_summary(&hash)? {
    println!("摘要：{}", summary);
}

// 为云端裁剪内容（只返回摘要 + 哈希）
let cloud_items = service.trim_for_cloud("session_123")?;
for item in cloud_items {
    println!("云端项：hash={}, summary={}", item.hash, item.summary);
}

// 删除内容
service.delete("session_123", &hash)?;

// 清理整个会话（删除瞬时层，移除会话目录）
service.cleanup_session("session_123")?;
```

### 三层存储说明

#### 瞬时层（Transient）

单轮临时文件，会话结束自动删除，适合存储临时对话、中间结果。

```rust
let hash = service.add(
    "session_123",
    b"临时对话内容",
    ContentType::Transient,
)?;
```

#### 短期层（ShortTerm）

最近 N 轮对话，自动裁剪，适合存储当前会话的主要上下文。

```rust
let hash = service.add(
    "session_123",
    b"重要对话内容",
    ContentType::ShortTerm,
)?;
```

#### 长期层（LongTerm）

项目习惯、规则配置，按关键词分类，适合存储持久化知识。

```rust
let metadata = ContentMetadata {
    id: uuid::Uuid::new_v4().to_string(),
    hash: hash.clone(),
    created_at: chrono::Utc::now(),
    updated_at: chrono::Utc::now(),
    content_type: ContentType::LongTerm,
    tags: vec!["git_rules".to_string()], // 分类标签
    summary: Some("Git 提交规范".to_string()),
};

// 长期层通常直接使用 Layer API
let long_term = LongTermLayer::new("./.context/sessions/sess_xxx/long-term")?;
long_term.store(b"Git 提交必须包含 JIRA 编号", &metadata)?;

// 按关键词搜索
let results = long_term.search_by_keyword("git")?;
for path in results {
    println!("找到规则文件：{:?}", path);
}
```

### 哈希索引

```rust
use crate::context::HashIndex;

let index = HashIndex::new("./.context/hashes")?;

// 添加映射
let content_path = PathBuf::from("./.context/sessions/sess_xxx/short-term/abc123_content.bin");
index.add("abc123", &content_path)?;

// 获取路径
let path = index.get_path("abc123")?;
println!("内容路径：{:?}", path);

// 检查是否存在
if index.contains("abc123") {
    println!("哈希存在");
}

// 列出所有哈希
let hashes = index.list_hashes()?;
for hash in hashes {
    println!("哈希：{}", hash);
}
```

### 增量日志

```rust
use crate::context::{ContextLogger, LogEntry, LogOperation};
use chrono::Utc;

let mut logger = ContextLogger::new("./.context/logs")?;

// 记录操作
logger.log_add("session_123", "abc123", Some("添加内容"))?;
logger.log_retrieve("session_123", "abc123")?;
logger.log_delete("session_123", "abc123")?;
logger.log_trim("session_123", &["hash1", "hash2"])?;

// 读取所有日志
let entries = logger.read_all()?;
for entry in entries {
    println!(
        "{} | {} | {} | {} | {:?}",
        entry.timestamp, entry.session_id, entry.hash, entry.operation, entry.details
    );
}

// 按会话过滤
let session_entries = logger.filter_by_session("session_123")?;

// 按操作类型过滤
let add_entries = logger.filter_by_operation(&LogOperation::Add)?;

// 按时间范围过滤
let start = Utc::now() - chrono::Duration::hours(1);
let end = Utc::now();
let range_entries = logger.filter_by_time_range(start, end)?;
```

## 配置说明

### FileContextConfig

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `max_short_term_rounds` | `usize` | 10 | 短期层最大保留轮数 |
| `enable_mmap` | `bool` | true | 是否启用 mmap 优化 |
| `enable_logging` | `bool` | true | 是否启用日志 |

## 性能优化

### mmap 内存映射

对于大文件（>1MB），使用 `memmap2` 库进行内存映射，减少磁盘 IO：

```rust
// 内部自动使用 mmap（如果启用）
let content = service.get_by_hash(&hash)?;
```

### 哈希去重

相同内容只存储一次，通过 SHA256 哈希自动识别：

```rust
// 两次添加相同内容，只会存储一次
let hash1 = service.add("sess1", b"same content", ContentType::ShortTerm)?;
let hash2 = service.add("sess1", b"same content", ContentType::ShortTerm)?;
assert_eq!(hash1, hash2);
```

### 热点缓存

会话级目录缓存在内存 HashMap 中，避免重复遍历：

```rust
// 首次访问自动创建并缓存
service.add("session_123", content, layer)?;

// 后续访问直接使用缓存
service.get_by_hash(&hash)?;
```

## 自动裁剪

短期层自动保留最近 N 轮（由 `max_short_term_rounds` 配置），超出自动删除：

```rust
let mut config = FileContextConfig::default();
config.max_short_term_rounds = 5; // 只保留 5 轮

let mut service = FileContextServiceImpl::new("./.context", config)?;

// 添加 10 个项目
for i in 0..10 {
    service.add("sess1", format!("content{}", i).as_bytes(), ContentType::ShortTerm)?;
}

// 裁剪到 5 个
let cloud_items = service.trim_for_cloud("sess1")?;
assert!(cloud_items.len() <= 5);
```

## 与 AI 调用集成

```rust
// 伪代码示例：在 AI 请求代理中使用

async fn send_to_ai_with_context(
    service: &mut FileContextServiceImpl,
    session: &str,
    user_message: &str,
) -> Result<String> {
    // 1. 将用户消息添加到短期层
    let hash = service.add(session, user_message.as_bytes(), ContentType::ShortTerm)?;
    
    // 2. 获取云端上下文（摘要 + 哈希）
    let cloud_items = service.trim_for_cloud(session)?;
    
    // 3. 构建 AI 请求（只发送摘要，不发送原文）
    let mut context_text = String::new();
    for item in cloud_items {
        context_text.push_str(&format!("Context [{}]: {}\n", item.hash, item.summary));
    }
    
    let ai_request = format!("{}\n\n{}", context_text, user_message);
    
    // 4. 发送 AI 请求
    let ai_response = call_ai_api(&ai_request).await?;
    
    // 5. 将 AI 响应添加到短期层
    service.add(session, ai_response.as_bytes(), ContentType::ShortTerm)?;
    
    Ok(ai_response)
}
```

## 安全考虑

- **文件权限**：建议设置 `.context/` 目录权限为 `0700`（仅所有者可访问）
- **符号链接**：Unix 系统使用符号链接，Windows 使用文本文件存储路径映射
- **路径验证**：所有文件操作限制在 `.context/` 目录内，防止路径遍历攻击

## 故障排查

### 日志文件

查看 `.context/logs/context_append.log` 了解所有上下文变更历史。

### 常见问题

1. **符号链接失败（Windows）**：Windows 默认使用文本文件存储路径映射，性能稍低但功能相同。

2. **mmap 失败**：如果 mmap 失败，自动回退到普通文件读取。

3. **磁盘空间不足**：定期调用 `trim_for_cloud()` 清理短期层，或使用 `cleanup_session()` 清理整个会话。

## 未来扩展

- [ ] 压缩长期层文件
- [ ] 支持远程存储后端（S3、OSS 等）
- [ ] 增量备份到云端
- [ ] 全文检索支持
- [ ] 多会话并发锁
