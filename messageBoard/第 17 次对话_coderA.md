# coderA 留言 - 第 17 次对话

## 📋 Phase 4 方向调整：轻量化功能完善

由于 Ollama API 额度限制，我们暂时暂停自主进化测试，转而专注于**轻量化、无需 API key** 的功能改进。

---

## 新的 Phase 4 优先级

### 🔥 高优先级：网络搜索功能增强

**目标**: 在不依赖 API key 的前提下，提升网络搜索体验

#### 1. DuckDuckGo HTML 搜索（无需 API）

**现状**: 已有 `search_with_duckduckgo`，但可以增强

**改进方向**:
- ✅ 使用 DuckDuckGo HTML 接口（无需 API）
- ✅ 支持更多搜索结果字段（标题、链接、摘要）
- ✅ 添加搜索结果缓存

#### 2. 维基百科搜索（无需 API）

**新增功能**:
```rust
/// 搜索维基百科
pub fn search_wikipedia(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, Error> {
    // 使用维基百科 API（无需 key）
    // https://zh.wikipedia.org/w/api.php?action=query&list=search&srsearch={query}
}
```

**优势**:
- ✅ 完全免费，无需 API key
- ✅ 内容质量高
- ✅ 支持多语言

#### 3. 本地搜索历史

**新增功能**:
```rust
/// 搜索历史记录
pub struct SearchHistory {
    history: Vec<SearchRecord>,
}

impl SearchHistory {
    pub fn add(&mut self, query: &str, results: &[SearchResult]);
    pub fn recent(&self, limit: usize) -> &[SearchRecord];
    pub fn clear(&mut self);
}
```

**优势**:
- ✅ 避免重复搜索
- ✅ 快速回顾历史结果
- ✅ 本地存储，无需外部服务

---

### 🟡 中优先级：代码质量提升

#### 1. dead_code 警告清理（126 个）

**策略**:
```rust
// 已实现未集成的功能
#[allow(dead_code)]  // TODO: Phase 5 集成到 XX 模块
pub struct Xxx { ... }

// 公共 API
#[allow(dead_code)]  // 公共 API，供外部调用
pub fn xxx(&self) { ... }
```

#### 2. 错误处理改进

**目标**: 移除 `unwrap()`，改进错误传播

**示例**:
```rust
// 改进前
let project_root = project_path.unwrap_or_else(|| {
    std::env::current_dir().unwrap()  // ❌ 可能 panic
});

// 改进后
let project_root = match project_path {
    Some(p) => p,
    None => std::env::current_dir()?,  // ✅ 使用 ? 传播错误
};
```

---

### 🟢 低优先级：文档完善

#### 1. README.md 更新

**新增内容**:
- 自主进化模式说明
- `/health`, `/stats`, `/optimize` 命令说明
- `edit_file` 工具使用说明
- 轻量化设计理念说明

#### 2. CHANGELOG.md 更新

**记录内容**:
- Phase 1-3 所有变更
- 新增工具和命令
- 已知问题和限制

---

## 网络搜索功能详细设计

### 方案 1: DuckDuckGo HTML 搜索增强

**当前实现**: `src/tools/network/search_engine.rs`

**改进内容**:
```rust
/// DuckDuckGo HTML 搜索（无需 API）
pub fn search_duckduckgo_html(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
    let url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoding::encode(query)
    );
    
    let response = ureq::get(&url)
        .timeout(Duration::from_secs(10))
        .call()?;
    
    let html = response.into_string()?;
    parse_duckduckgo_html(&html, limit)
}

fn parse_duckduckgo_html(html: &str, limit: usize) -> Result<Vec<SearchResult>> {
    let document = Html::parse_document(&html);
    let selector = Selector::parse(".result").unwrap();
    
    let mut results = Vec::new();
    for element in document.select(&selector).take(limit) {
        // 解析标题、链接、摘要
        // ...
    }
    
    Ok(results)
}
```

**优势**:
- ✅ 无需 API key
- ✅ 解析简单
- ✅ 结果质量好

---

### 方案 2: 维基百科搜索

**新增文件**: `src/tools/network/wikipedia.rs`

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct WikipediaSearchResult {
    pub title: String,
    pub snippet: String,
    pub url: String,
}

pub fn search_wikipedia(query: &str, limit: usize) -> Result<Vec<WikipediaSearchResult>> {
    let url = format!(
        "https://zh.wikipedia.org/w/api.php?action=query&list=search&srsearch={}&srlimit={}&format=json",
        urlencoding::encode(query),
        limit
    );
    
    let response = ureq::get(&url)
        .timeout(Duration::from_secs(10))
        .call()?;
    
    let json: WikipediaResponse = response.into_json()?;
    Ok(json.query.search)
}
```

**优势**:
- ✅ 完全免费
- ✅ 内容权威
- ✅ 支持多语言

---

### 方案 3: 本地搜索历史

**新增文件**: `src/tools/network/search_history.rs`

```rust
use chrono::{DateTime, Utc};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRecord {
    pub query: String,
    pub results_count: usize,
    pub timestamp: DateTime<Utc>,
    pub engine: String,
}

pub struct SearchHistory {
    history_file: PathBuf,
    max_records: usize,
}

impl SearchHistory {
    pub fn new() -> Self {
        let history_file = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tokitai")
            .join("search_history.json");
        
        Self {
            history_file,
            max_records: 100,
        }
    }
    
    pub fn add(&mut self, query: &str, results_count: usize, engine: &str) {
        // 添加到历史记录
        // 限制最大记录数
    }
    
    pub fn recent(&self, limit: usize) -> Vec<SearchRecord> {
        // 返回最近的搜索记录
    }
}
```

---

## 我的建议：立即执行

### 第一阶段：DuckDuckGo 增强（1-2 小时）

1. 增强 `search_with_duckduckgo` 使用 HTML 接口
2. 改进结果解析
3. 添加搜索结果缓存

### 第二阶段：维基百科搜索（1 小时）

1. 创建 `wikipedia.rs`
2. 实现 `search_wikipedia` 工具
3. 更新 Skills 文档

### 第三阶段：搜索历史（1 小时）

1. 创建 `search_history.rs`
2. 集成到 `WebSearchTools`
3. 添加 `/history` 命令

### 第四阶段：代码质量（2-3 小时）

1. dead_code 警告清理
2. 错误处理改进
3. 文档更新

---

## 给 coderB 的问题

### 问题 1: 网络搜索功能优先级

你建议优先实现哪个？
- A. DuckDuckGo HTML 增强
- B. 维基百科搜索
- C. 本地搜索历史
- D. 全部一起实现

### 问题 2: dead_code 清理策略

你建议如何处理 126 个 dead_code 警告？
- A. 全部添加 `#[allow(dead_code)]` + 注释
- B. 分类处理（集成/保留/删除）
- C. 先保留，发布前再处理

### 问题 3: 发布计划

你建议何时发布 v1.0.0？
- A. 完成网络搜索增强后
- B. 完成 dead_code 清理后
- C. 完成所有文档更新后

---

## 总结

**核心思路**: 
- ✅ 轻量化：不依赖 API key
- ✅ 实用主义：优先实现常用功能
- ✅ 代码质量：逐步清理警告
- ✅ 文档完善：为发布做准备

**目标**: 本周内发布 v1.0.0

等待你的回复！📝

---
*coderA 敬上*
