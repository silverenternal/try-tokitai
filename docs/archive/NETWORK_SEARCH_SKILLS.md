# 网络搜索工具 Skills

## 概述

本 Skills 文件描述了 `WebSearchTools` 提供的网络搜索工具集，支持多种搜索引擎。

## 工具列表

### 0. `edit_file` - 文件编辑工具 🔧

**功能**: 在现有文件基础上进行修改，支持三种编辑模式

**参数**:
- `path` (String): 文件路径
- `mode` (String): 编辑模式 (`append`, `prepend`, `replace`)
- `content` (String): 要添加/替换的内容
- `search` (Option<String>): 要替换的原文本（仅 `replace` 模式需要）

**支持的模式**:
| 模式 | 说明 | 用法 |
|------|------|------|
| `append` | 在文件末尾追加内容 | `edit_file(path, "append", content, None)` |
| `prepend` | 在文件开头插入内容 | `edit_file(path, "prepend", content, None)` |
| `replace` | 替换文件中包含的文本 | `edit_file(path, "replace", new_content, Some(search_text))` |

**replace 模式使用技巧**:
1. 先使用 `read_file` 读取文件内容
2. 复制要替换的原文本（**包括空白字符**）
3. 使用原文本作为 `search` 参数
4. 使用新文本作为 `content` 参数

**使用示例**:
```json
// 追加内容
{
  "name": "edit_file",
  "arguments": {
    "path": "src/main.rs",
    "mode": "append",
    "content": "\n// 新增注释\n// 这是追加的内容"
  }
}

// 替换文本
{
  "name": "edit_file",
  "arguments": {
    "path": "src/main.rs",
    "mode": "replace",
    "content": "fn calculate() -> i32 { 42 }",
    "search": "fn calculate() -> i32 { let x = 5; 42 }"
  }
}
```

**错误提示**:
- 如果 `replace` 模式未找到匹配的文本，会显示：
  - 最接近的位置（行号、列号）
  - 上下文内容（前后各 3 行）
  - 提示：原文本必须完全匹配（包括空白字符）

**注意事项**:
- `replace` 模式需要**精确匹配**原文本
- 空白字符（空格、制表符、换行）也必须匹配
- 建议先读取文件内容，确认原文本后再替换

---

### 1. `search_web` - 通用网页搜索

**功能**: 使用默认搜索引擎进行网页搜索

**参数**:
- `query` (String): 搜索关键词
- `limit` (Option<usize>): 返回结果数量（默认 5，最大 20）

**返回**: JSON 格式的搜索结果列表

**使用示例**:
```json
{
  "name": "search_web",
  "arguments": {
    "query": "Rust 编程语言教程",
    "limit": 10
  }
}
```

**返回示例**:
```json
{
  "query": "Rust 编程语言教程",
  "total": 10,
  "results": [
    {
      "title": "Rust 编程语言入门教程",
      "url": "https://example.com/rust-tutorial",
      "snippet": "本文介绍 Rust 编程语言的基础知识...",
      "engine": "google"
    }
  ]
}
```

---

### 2. `search_with_searxng` - SearXNG 隐私搜索 🔒

**功能**: 使用 SearXNG 元搜索引擎进行隐私保护的搜索

**特性**:
- 🛡️ **隐私优先**: 不追踪用户搜索历史
- 🔄 **多引擎聚合**: 同时搜索 Bing、DuckDuckGo 等多个引擎
- ⚙️ **可配置**: 支持自定义 SearXNG 实例

**参数**:
- `query` (String): 搜索关键词
- `limit` (Option<usize>): 返回结果数量（默认 5，最大 20）
- `searxng_url` (Option<String>): SearXNG 实例 URL（可选）
  - 如果未提供，使用环境变量 `SEARXNG_URL`
  - 如果环境变量也未设置，使用默认实例 `https://searx.be`

**返回**: JSON 格式的搜索结果列表

**使用示例**:

使用默认 SearXNG 实例:
```json
{
  "name": "search_with_searxng",
  "arguments": {
    "query": "Rust async programming",
    "limit": 10
  }
}
```

使用自定义 SearXNG 实例:
```json
{
  "name": "search_with_searxng",
  "arguments": {
    "query": "Rust async programming",
    "limit": 10,
    "searxng_url": "https://searx.example.org"
  }
}
```

**推荐配置**:
在 `.env` 文件中配置:
```bash
SEARXNG_URL=https://searx.be
```

**注意事项**:
- 公共 SearXNG 实例可能有速率限制
- 建议自建 SearXNG 实例以获得更好的稳定性

---

### 3. `search_with_duckduckgo` - DuckDuckGo 搜索 🦆

**功能**: 使用 DuckDuckGo 搜索引擎进行搜索

**特性**:
- 🛡️ **隐私保护**: 不追踪用户，不记录搜索历史
- 🔄 **自动重试**: 失败时指数退避重试（最多 3 次）
- ⚡ **开箱即用**: 无需配置，直接使用

**参数**:
- `query` (String): 搜索关键词
- `limit` (Option<usize>): 返回结果数量（默认 5，最大 20）

**返回**: JSON 格式的搜索结果列表

**使用示例**:
```json
{
  "name": "search_with_duckduckgo",
  "arguments": {
    "query": "Rust memory safety",
    "limit": 10
  }
}
```

**重试机制**:
- 第 1 次失败：等待 300ms 后重试
- 第 2 次失败：等待 600ms 后重试
- 第 3 次失败：等待 900ms 后重试
- 3 次全部失败：返回错误

**注意事项**:
- DuckDuckGo 有反爬虫机制，频繁请求可能触发 503 错误
- 自动重试机制可处理临时性错误

---

## 搜索引擎选择建议

| 场景 | 推荐工具 | 理由 |
|------|----------|------|
| 日常搜索 | `search_web` | 使用默认引擎，简单快捷 |
| 隐私敏感 | `search_with_searxng` | 隐私优先，多引擎聚合 |
| 无需配置 | `search_with_duckduckgo` | 开箱即用，自动重试 |
| 中文搜索 | `search_with_searxng` | 可配置支持中文的引擎 |
| 技术搜索 | `search_with_duckduckgo` | 技术内容质量高 |

---

## 错误处理

所有搜索工具在失败时返回详细的错误信息：

```json
{
  "error": "搜索失败：网络请求失败。建议：1) 检查网络连接 2) 稍后重试"
}
```

**常见错误及解决方案**:

| 错误 | 原因 | 解决方案 |
|------|------|----------|
| 网络请求失败 | 网络连接问题 | 检查网络连接 |
| 未找到搜索结果 | 查询无匹配内容 | 尝试不同的关键词 |
| 搜索超时 | 服务器响应慢 | 稍后重试 |
| 503 Service Unavailable | 触发速率限制 | 等待后重试 |

---

## 最佳实践

1. **合理设置 limit**: 默认 5 条结果通常足够，避免请求过多数据
2. **使用缓存**: 相同查询会返回缓存结果，提高响应速度
3. **错误重试**: 对于临时性错误，可以手动重试 1-2 次
4. **隐私保护**: 敏感查询建议使用 `search_with_searxng` 或 `search_with_duckduckgo`

---

## 环境变量配置

在 `.env` 文件中配置:

```bash
# SearXNG 实例 URL（可选）
SEARXNG_URL=https://searx.be

# 最大重试次数（可选，默认 3）
MAX_SEARCH_RETRIES=3
```

---

*最后更新：2026 年 3 月 14 日*
