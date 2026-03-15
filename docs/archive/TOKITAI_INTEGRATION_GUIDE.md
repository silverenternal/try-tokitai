# tokitai 深度集成指南

> **版本**: 1.0
> **最后更新**: 2026-03-15
> **目标**: 利用 `#[tool]` 宏自动生成元数据，实现工具矩阵深度集成

---

## 📋 概述

tokitai 的 `#[tool]` 宏可以自动生成工具元数据，包括：
- 工具名称和描述
- 输入参数 schema
- 风险等级
- 分类标签

这些信息可以直接用于 `ToolMatrix` 的倒排索引和 AI 分类。

---

## 🔧 使用 `#[tool]` 宏

### 基本用法

```rust
use tokitai::tool;

/// 文件操作工具集
pub struct FileOperations;

#[tool]
impl FileOperations {
    /// 读取文件内容
    pub fn read_file(&self, path: String) -> Result<String, String> {
        // ...
    }

    /// 写入文件内容
    pub fn write_file(&self, path: String, content: String) -> Result<String, String> {
        // ...
    }
}
```

### 宏自动生成的内容

`#[tool]` 宏会自动生成：

1. **ToolDefinition 元数据**
   ```rust
   ToolDefinition {
       name: "read_file".to_string(),
       description: "读取文件内容".to_string(),
       input_schema: r#"{"type":"object","properties":{"path":{"type":"string"}}}"#.to_string(),
       metadata: ServiceMetadata {
           category: ServiceCategory::File,
           risk_level: RiskLevel::Safe,
           // ...
       },
       tags: vec!["file".to_string(), "read".to_string()],
   }
   ```

2. **ToolProvider trait 实现**
   ```rust
   impl ToolProvider for FileOperations {
       fn tool_definitions() -> Vec<ToolDefinition> {
           // 自动生成所有工具的元数据
       }
   }
   ```

---

## 🚀 与 ToolMatrix 集成

### 1. 注册工具到工具箱

```rust
use crate::tool_matrix::{ToolRegistry, ToolBox, ToolSource};
use crate::tools::FileOperations;

// 创建工具箱
let mut file_box = ToolBox::new("file_ops", "File Operations", "File operations tools");

// 从 ToolProvider 注册工具
let registry = ToolRegistry::new();
let _ = registry.register_from_provider::<FileOperations>(Some("file_ops"), ToolSource::Builtin);
```

### 2. 自动元数据提取

`#[tool]` 宏会自动从函数签名提取：

- **参数类型** → JSON Schema
- **返回类型** → 输出类型推断
- **函数文档** → 工具描述
- **函数名** → 工具名称

### 3. 自定义元数据（可选）

如果需要更精细的控制，可以使用宏属性：

```rust
#[tool]
impl FileOperations {
    /// 读取文件内容
    #[tool(
        description = "安全地读取指定路径的文件内容",
        category = "file",
        tags = ["io", "read_only", "safe"],
        risk_level = "safe"
    )]
    pub fn read_file(&self, path: String) -> Result<String, String> {
        // ...
    }
}
```

---

## 📊 元数据自动映射

| 函数特征 | 自动提取的元数据 | 说明 |
|---------|----------------|------|
| 函数名 | `name` | 蛇形转下划线 |
| 文档注释 | `description` | 第一行作为描述 |
| 参数类型 | `input_schema.properties` | 推断 JSON Schema |
| 返回类型 | `output_type` | 推断输出类型 |
| `path: String` | `tags: ["file", "path"]` | 自动添加标签 |
| `url: String` | `tags: ["url", "network"]` | 自动添加标签 |
| `data: Vec<u8>` | `tags: ["binary", "data"]` | 自动添加标签 |

---

## 🔗 与 LightweightToolSelector 集成

### 自动索引构建

```rust
use crate::tool_matrix::tool_selector::LightweightToolSelector;
use crate::tool_matrix::matrix::ToolDefinition;
use crate::tools::FileOperations;
use tokitai::ToolProvider;

// 从 ToolProvider 获取工具定义
let tools = FileOperations::tool_definitions();

// 创建选择器（自动构建倒排索引）
let selector = LightweightToolSelector::new_without_ai(tools, None);

// 搜索工具（自动使用元数据）
let results = selector.search("read file").await;
```

### AI 分类器使用元数据

```rust
use crate::tool_matrix::ai_classifier::{AIToolboxClassifier, DefaultLLMClient};
use std::sync::Arc;

// 创建 LLM 客户端
let llm_client = Arc::new(DefaultLLMClient::new(api_url, api_key));

// 创建分类器
let classifier = AIToolboxClassifier::new(llm_client, toolboxes);

// AI 分类时会使用工具元数据
let tool = &tools[0];  // ToolDefinition
let assignment = classifier.classify_tool(tool).await?;
```

---

## 🎯 最佳实践

### 1. 编写清晰的文档注释

```rust
/// 读取文件内容
/// 
/// 安全地读取指定路径的文件内容，支持 UTF-8 编码。
/// 
/// # 参数
/// - `path`: 文件路径
/// 
/// # 返回
/// 文件内容字符串
pub fn read_file(&self, path: String) -> Result<String, String> {
    // ...
}
```

### 2. 使用描述性参数名

```rust
// ✅ 好
pub fn read_file(&self, file_path: String) -> Result<String, String>

// ❌ 避免
pub fn read_file(&self, p: String) -> Result<String, String>
```

### 3. 保持函数单一职责

```rust
// ✅ 好：每个函数做一件事
pub fn read_file(&self, path: String) -> Result<String, String>
pub fn write_file(&self, path: String, content: String) -> Result<String, String>

// ❌ 避免：多功能混合
pub fn file_operation(&self, op: String, path: String, content: Option<String>) -> Result<String, String>
```

---

## 📝 示例：完整工具集

```rust
use tokitai::tool;
use std::fs;
use std::path::Path;

/// 文件操作工具集
/// 
/// 提供文件读写、目录管理等基础功能
pub struct FileOperations;

#[tool]
impl FileOperations {
    /// 读取文件内容
    /// 
    /// 安全地读取指定路径的文件内容
    pub fn read_file(&self, path: String) -> Result<String, String> {
        if !Path::new(&path).exists() {
            return Err(format!("文件不存在：{}", path));
        }
        fs::read_to_string(&path)
            .map_err(|e| format!("读取文件失败：{}", e))
    }

    /// 写入文件内容
    /// 
    /// 将内容写入指定路径的文件（自动创建父目录）
    pub fn write_file(&self, path: String, content: String) -> Result<String, String> {
        if path.contains("..") {
            return Err("路径包含非法字符".to_string());
        }

        if let Some(parent) = Path::new(&path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("创建目录失败：{}", e))?;
        }

        fs::write(&path, content)
            .map_err(|e| format!("写入文件失败：{}", e))?;
        Ok(format!("成功写入文件：{}", path))
    }

    /// 列出目录内容
    /// 
    /// 返回指定目录下所有文件和子目录的名称列表
    pub fn list_dir(&self, path: String) -> Result<String, String> {
        let entries = fs::read_dir(&path)
            .map_err(|e| format!("列出目录失败：{}", e))?;

        let mut result = Vec::new();
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let is_dir = e.path().is_dir();
            result.push(format!("{}{}", name, if is_dir { "/" } else { "" }));
        }

        Ok(result.join("\n"))
    }
}
```

---

## 🔍 验证元数据

### 检查生成的 ToolDefinition

```rust
use crate::tools::FileOperations;
use tokitai::ToolProvider;

let tools = FileOperations::tool_definitions();

for tool in tools {
    println!("工具名：{}", tool.name);
    println!("描述：{}", tool.description);
    println!("分类：{:?}", tool.metadata.category);
    println!("风险等级：{:?}", tool.risk_level);
    println!("标签：{:?}", tool.tags);
    println!("输入 Schema: {}", tool.input_schema);
    println!("---");
}
```

---

## 🔗 相关文档

- [设计文档](../archive/LIGHTWEIGHT_TOOL_SELECTION_DESIGN.md)
- [使用指南](../structure_ensure/TOOL_SELECTOR_GUIDE.md)
- [tokitai 文档](https://docs.rs/tokitai)

---

**最后更新**: 2026-03-15
**测试状态**: ✅ 已验证
