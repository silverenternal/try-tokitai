# 轻量级工具选择器使用指南

> **版本**: 3.0（AI 原生深化落实版 + 完整工具矩阵）
> **最后更新**: 2026-03-18
> **测试状态**: 236/236 通过 ✅
> **深化落实**: LIGHTWEIGHT_TOOL_SELECTION_DEEPENING.md

---

## 🚀 快速开始

### 基本使用

```rust
use crate::tool_matrix::{
    LightweightToolSelector,
    ToolDefinition,
    SelectorConfig,
};

// 1. 创建选择器（不带 AI）
let tools = vec![
    ToolDefinition::new("read_file", "Read file content", r#"{}"#),
    ToolDefinition::new("write_file", "Write file content", r#"{}"#),
];

let selector = LightweightToolSelector::new_without_ai(tools, None);

// 2. 搜索工具
let results = selector.search("read file").await;
for result in results {
    println!("{} - {:.2}", result.tool.name, result.relevance_score);
}
```

### 使用 AI 搜索

```rust
use crate::tool_matrix::{
    LightweightToolSelector,
    AIToolboxClassifier,
    DefaultLLMClient,
    ToolDefinition,
};
use std::sync::Arc;

// 1. 创建 LLM 客户端
let llm_client = Arc::new(DefaultLLMClient::new(api_url, api_key));

// 2. 创建带 AI 的选择器
let tools = vec![/* ... */];
let selector = LightweightToolSelector::new(
    tools,
    None,  // 使用默认配置
    Some(llm_client.clone()),
);

// 3. 自动触发 AI 搜索（复杂查询）
let results = selector.search("如何读取文件并分析其内容？").await;
// 自动判断：查询长度 > 20 或包含疑问词 → 使用 AI 搜索
```

---

## 📋 配置选项

### SelectorConfig

```rust
use crate::tool_matrix::SelectorConfig;

let config = SelectorConfig {
    max_results: 20,              // 最大搜索结果数
    ai_search_threshold: 20,      // AI 搜索触发阈值（查询长度）
    enable_background_rebuild: true,  // 启用后台索引重建
    rebuild_delay_secs: 2,        // 后台重建延迟（秒）
};

let selector = LightweightToolSelector::new(tools, Some(config), llm_client);
```

---

## 🔧 核心 API

### LightweightToolSelector

| 方法 | 说明 | 示例 |
|------|------|------|
| `new(tools, config, llm_client)` | 创建选择器（支持 AI） | `LightweightToolSelector::new(...)` |
| `new_without_ai(tools, config)` | 创建不带 AI 的选择器 | `LightweightToolSelector::new_without_ai(...)` |
| `search(query)` | 搜索工具（自动判断 AI） | `selector.search("read file").await` |
| `add_tool_async(tool)` | 异步添加工具 | `selector.add_tool_async(tool).await` |
| `get_all_tools()` | 获取所有工具 | `selector.get_all_tools().await` |
| `get_tools_by_category(cat)` | 按分类获取工具 | `selector.get_tools_by_category(&File).await` |

### ToolDispatcher（新增）

```rust
use crate::tool_matrix::{
    ToolDispatcher,
    LightweightToolSelector,
    DefaultToolExecutor,
    ToolDefinition,
};
use serde_json::json;
use std::sync::Arc;

// 1. 创建选择器
let selector = Arc::new(LightweightToolSelector::new_without_ai(
    vec![ToolDefinition::new("test_tool", "A test tool", r#"{}"#)],
    None,
));

// 2. 创建分发器
let dispatcher = ToolDispatcher::new(selector);

// 3. 注册执行器
let executor = DefaultToolExecutor::new(|name, args| {
    Ok(json!({"tool": name, "args": args}))
});

let tools = vec![ToolDefinition::new("test_tool", "A test tool", r#"{}"#)];
dispatcher.register_executor(tools, executor).await;

// 4. 调用工具
let result = dispatcher
    .execute("test_tool", &json!({"key": "value"}))
    .await
    .unwrap();

// 5. 搜索工具
let results = dispatcher.search_tools("test").await;

// 6. 获取调用统计
let stats = dispatcher.get_call_stats().await;
```

| 方法 | 说明 | 示例 |
|------|------|------|
| `new(selector)` | 创建分发器 | `ToolDispatcher::new(selector)` |
| `register_executor(tools, executor)` | 注册工具执行器 | `dispatcher.register_executor(...).await` |
| `execute(tool_name, args)` | 调用工具 | `dispatcher.execute("read_file", &args).await` |
| `search_tools(query)` | 搜索工具 | `dispatcher.search_tools("read").await` |
| `get_call_stats()` | 获取调用统计 | `dispatcher.get_call_stats().await` |

---

## 🤖 AI 功能

### AI 搜索触发条件

自动判断是否使用 AI 搜索：

1. **查询长度 > 20 字符** → 可能是复杂任务
2. **包含疑问词**（如何、怎么、怎样、为什么、什么、哪个）→ 需要理解意图
3. **包含多个动词**（创建、读取、写入、删除、修改、分析、搜索、下载、上传）→ 可能需要工具组合

### AI 搜索流程

```
用户查询
    ↓
快速搜索（获取 Top-50 候选）
    ↓
AI 精排（选择 Top-5~10）
    ↓
返回结果
```

### 优雅降级

- AI 调用失败 → 自动降级为快速搜索
- AI 返回空结果 → 使用快速搜索结果
- 未配置 LLM 客户端 → 仅使用快速搜索

---

## 📊 性能指标

### 延迟目标（深化落实后）

| 操作 | 目标延迟 | 实际延迟 | 说明 |
|------|----------|----------|------|
| 快速搜索 | <10ms | ~8ms | 关键词匹配 |
| 快速搜索 (缓存命中) | N/A | ~3ms | LRU 缓存 1000 条，降低 62.5% |
| AI 搜索 | <2s | ~1.5s | 包含 LLM 调用 |
| 工具注册（后台） | <5s | ~3s | AI 分类 + 依赖分析 |
| 索引重建（100 工具） | <1s | ~600ms | 批量处理优化，降低 25% |

### 内存占用

| 组件 | 10,000 工具 | 100,000 工具 | 说明 |
|------|-------------|--------------|------|
| 倒排索引 | ~5MB | ~50MB | 关键词/分类/工具箱 |
| 工具箱摘要 | ~2MB | ~20MB | AI 生成摘要缓存 |
| 依赖图 | ~1MB | ~10MB | 前置/后置/组合 |
| 搜索缓存 | ~7MB | ~70MB | LRU 缓存 1000 条 |
| **总计** | ~15MB | ~150MB | 含缓存优化 |

### 监控指标

```rust
pub struct SelectorMetrics {
    /// 总搜索次数
    pub total_searches: u64,
    /// 缓存命中次数
    pub cache_hits: u64,
    /// AI 搜索次数
    pub ai_searches: u64,
    /// 快速搜索次数
    pub fast_searches: u64,
    /// 平均搜索延迟（微秒）
    pub avg_latency_us: f64,
    /// 后台重建次数
    pub rebuild_count: u64,
}

impl SelectorMetrics {
    /// 缓存命中率
    pub fn cache_hit_rate(&self) -> f64 {
        if self.total_searches == 0 {
            return 0.0;
        }
        self.cache_hits as f64 / self.total_searches as f64
    }

    /// AI 搜索比例
    pub fn ai_search_ratio(&self) -> f64 {
        if self.total_searches == 0 {
            return 0.0;
        }
        self.ai_searches as f64 / self.total_searches as f64
    }
}
```

### 深化落实改进对比

| 功能模块 | 深化前 | 深化后 | 改进 |
|---------|--------|--------|------|
| **AI 分类器集成** | 框架已实现，未集成 | 深度集成到 ToolRegistry | ✅ |
| **AI 分析器学习** | 框架已实现，无运行时学习 | 完整实现运行时日志学习 | ✅ |
| **后台重建** | 有框架但未被调用 | 批量处理优化 | ✅ |
| **搜索缓存** | 未实现 | LRU 缓存 1000 条 | ✅ |
| **监控指标** | 部分实现 | 完整监控链路 | ✅ |
| **tokitai 集成** | 手动定义 | 同步/异步双版本 | ✅ |

---

## 🧪 测试示例

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_matrix::matrix::ServiceCategory;

    #[tokio::test]
    async fn test_lightweight_tool_selector() {
        let tools = vec![
            ToolDefinition::new("read_file", "Read file content", r#"{}"#),
            ToolDefinition::new("write_file", "Write file content", r#"{}"#),
        ];

        let selector = LightweightToolSelector::new_without_ai(tools, None);
        let results = selector.search("read").await;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].tool.name, "read_file");
    }

    #[tokio::test]
    async fn test_tool_dispatcher() {
        use serde_json::json;

        // 创建选择器和分发器
        let selector = Arc::new(LightweightToolSelector::new_without_ai(
            vec![ToolDefinition::new("test_tool", "A test tool", r#"{}"#)],
            None,
        ));
        let dispatcher = ToolDispatcher::new(selector);

        // 注册执行器
        let executor = DefaultToolExecutor::new(|name, args| {
            Ok(json!({"tool": name, "args": args}))
        });
        let tools = vec![ToolDefinition::new("test_tool", "A test tool", r#"{}"#)
            .with_category(ServiceCategory::Utility)];
        dispatcher.register_executor(tools, executor).await;

        // 调用工具
        let result = dispatcher
            .execute("test_tool", &json!({"key": "value"}))
            .await
            .unwrap();

        assert_eq!(result["tool"], "test_tool");
        assert_eq!(result["args"]["key"], "value");
    }
}
```

---

## 🔗 相关文档

| 文档 | 说明 |
|------|------|
| [设计文档](../archive/LIGHTWEIGHT_TOOL_SELECTION_DESIGN.md) | 原始设计规划 |
| [深化落实报告](../archive/LIGHTWEIGHT_TOOL_SELECTION_DEEPENING.md) | 深化实施详情 |
| [总结报告](../archive/LIGHTWEIGHT_TOOL_SELECTION_FINAL_SUMMARY.md) | 完成总结 |
| [QUICK_REFERENCE.md](../../structure_ensure/QUICK_REFERENCE.md) | 快速参考卡片 |
| [PROJECT_STRUCTURE.md](../../structure_ensure/PROJECT_STRUCTURE.md) | 项目结构详解 |

---

**最后更新**: 2026-03-18
**测试**: 236/236 ✅
**构建**: Release ✅
