# Tensor 模块 - AI 可操作的张量计算微服务

> **重构完成**: ✅ 架构重构完成
> **tokitai 集成**: ✅ 使用 #[tool] 宏注册
> **AI 可理解**: ✅ 完整的操作元数据

---

## 📌 概述

Tensor 模块是一个**LLM 驱动的张量计算微服务**，通过 tokitai 库将张量操作封装为 AI 可理解和调用的工具。

### 核心设计理念

1. **AI 可操作**: 所有操作都有语义化元数据，AI 知道每个操作是做什么的
2. **简化架构**: 移除 GlobalTensorStore，Tensor 直接持有数据
3. **tokitai 集成**: 使用 `#[tool]` 宏注册，AI 可以通过工具调用执行张量操作
4. **领域特定错误**: 错误类型带修复建议，AI 可以自主恢复
5. **性能优化**: 使用 ndarray 内置方法（dot, broadcast, concatenate 等）

---

## 🏗️ 架构设计

```
┌─────────────────────────────────────────────────────────────┐
│                    工具层 (Tool Layer)                      │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ TensorTools (tokitai #[tool] 集成)                  │    │
│  │ - zeros, ones, randn, from_data                     │    │
│  │ - add, sub, mul, div, matmul                        │    │
│  │ - sum, mean, max, min, argmax                       │    │
│  │ - relu, gelu, sigmoid, layer_norm                   │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                    服务层 (Service Layer)                   │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ TensorService                                        │    │
│  │ - 所有张量操作的统一入口                              │    │
│  │ - 支持链式调用                                        │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                    后端层 (Backend Layer)                   │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ TensorBackend trait + NdArrayBackend                 │    │
│  │ - 单一 trait，移除过度设计的接口拆分                  │    │
│  │ - 使用 ndarray 内置方法优化性能                       │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                    核心层 (Core Layer)                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ Tensor      │  │ TensorError │  │ OperationMetadata   │  │
│  │ 直接持有数据 │  │ 领域特定错误 │  │ AI 可理解的元数据     │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## 🚀 快速开始

### 使用 TensorService（编程方式）

```rust
use ai_assistant::tools::tensor::{TensorService, Tensor};

fn main() -> anyhow::Result<()> {
    let service = TensorService::new();

    // 创建张量
    let a = service.from_data(&[1.0, 2.0, 3.0, 4.0], &[2, 2])?;
    let b = service.from_data(&[5.0, 6.0, 7.0, 8.0], &[2, 2])?;

    // 矩阵乘法
    let result = service.matmul(&a, &b)?;
    println!("Matmul result: {:?}", result.as_slice());
    // 输出：Matmul result: Some([19.0, 22.0, 43.0, 50.0])

    // 链式调用
    let zeros = service.zeros(&[2, 2])?;
    let result = service
        .add_scalar(&zeros, 1.0)?
        .mul_scalar(&2.0)?;

    Ok(())
}
```

### 使用 TensorTools（AI 工具调用）

```rust
use ai_assistant::tools::tensor::TensorTools;
use tokitai::ToolProvider;
use serde_json::json;

fn main() -> anyhow::Result<()> {
    let tools = TensorTools::new();

    // 获取工具定义（发送给 AI）
    let definitions = TensorTools::tool_definitions();
    println!("Available tools: {:?}", definitions);

    // AI 调用工具
    let result = tools.zeros(vec![2, 3])?;
    println!("Zeros tensor: {}", result);

    // 矩阵乘法
    let a = json!({
        "shape": [2, 3],
        "data": [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
    });
    let b = json!({
        "shape": [3, 2],
        "data": [7.0, 8.0, 9.0, 10.0, 11.0, 12.0]
    });
    let result = tools.matmul(a, b)?;
    println!("Matmul result: {}", result);

    Ok(())
}
```

---

## 📊 支持的操作

### 创建操作

| 工具名 | 说明 | 参数 |
|--------|------|------|
| `zeros` | 创建零张量 | `shape: Vec<usize>` |
| `ones` | 创建一张量 | `shape: Vec<usize>` |
| `randn` | 创建随机张量（标准正态分布） | `shape: Vec<usize>` |
| `from_data` | 从数据创建张量 | `data: Vec<f64>`, `shape: Vec<usize>` |

### 算术操作

| 工具名 | 说明 | 参数 |
|--------|------|------|
| `add` | 逐元素加法（支持广播） | `a: Tensor`, `b: Tensor` |
| `sub` | 逐元素减法（支持广播） | `a: Tensor`, `b: Tensor` |
| `mul` | 逐元素乘法（支持广播） | `a: Tensor`, `b: Tensor` |
| `div` | 逐元素除法（支持广播） | `a: Tensor`, `b: Tensor` |
| `mul_scalar` | 标量乘法 | `tensor: Tensor`, `scalar: f64` |

### 矩阵操作

| 工具名 | 说明 | 参数 |
|--------|------|------|
| `matmul` | 矩阵乘法 | `a: Tensor`, `b: Tensor` |
| `transpose` | 转置（2D） | `tensor: Tensor` |
| `reshape` | 重塑形状 | `tensor: Tensor`, `shape: Vec<usize>` |

### 归约操作

| 工具名 | 说明 | 参数 |
|--------|------|------|
| `sum` | 沿指定维度求和 | `tensor: Tensor`, `dims: Vec<usize>` |
| `mean` | 沿指定维度求平均 | `tensor: Tensor`, `dims: Vec<usize>` |
| `max` | 沿指定维度求最大值 | `tensor: Tensor`, `dims: Vec<usize>` |
| `min` | 沿指定维度求最小值 | `tensor: Tensor`, `dims: Vec<usize>` |

### 激活函数

| 工具名 | 说明 | 参数 |
|--------|------|------|
| `relu` | ReLU 激活：max(0, x) | `input: Tensor` |
| `gelu` | GELU 激活（近似） | `input: Tensor` |
| `sigmoid` | Sigmoid 激活 | `input: Tensor` |
| `layer_norm` | LayerNorm 归一化 | `input: Tensor`, `normalized_shape: usize`, `eps: f64` |

---

## 🔧 与 tokitai 集成

### 工具注册

```rust
use ai_assistant::tools::tensor::TensorTools;
use tokitai::ToolProvider;

// 获取工具定义
let tools = TensorTools::tool_definitions();

// 转换为 JSON 发送给 AI
let tools_json = serde_json::to_string_pretty(&tools)?;
```

### 工具调用

```rust
use ai_assistant::tools::tensor::TensorTools;
use tokitai::ToolProvider;
use serde_json::json;

let tools = TensorTools::new();

// 调用工具
let call_request = json!({
    "name": "zeros",
    "arguments": {"shape": [2, 3]}
});

let result = tools.call_tool("zeros", &call_request["arguments"])?;
```

---

## 📝 操作元数据

每个操作都有完整的元数据，AI 可以理解：

```rust
use ai_assistant::tools::tensor::core::{get_operation_metadata, OperationCategory};

let meta = get_operation_metadata("matmul").unwrap();

println!("名称：{}", meta.name);
println!("描述：{}", meta.description);
println!("类别：{}", meta.category);  // OperationCategory::Matrix
println!("文档：{}", meta.documentation);
println!("参数：{:?}", meta.parameters);
println!("示例：{:?}", meta.examples);
println!("常见错误：{:?}", meta.common_errors);
println!("修复建议：{:?}", meta.common_errors.iter().map(|e| e.suggestion).collect::<Vec<_>>());
```

---

## 🧪 测试

```bash
# 运行所有 tensor 测试
cargo test --features tensor tensor::

# 运行特定测试
cargo test --features tensor test_zeros
cargo test --features tensor test_matmul
cargo test --features tensor test_relu
```

---

## 📈 性能优化

### 使用 ndarray 内置方法

- `matmul`: 使用 `ndarray::Array2::dot()` 而非手动三重循环
- `broadcast`: 使用 `ndarray::ArrayD::broadcast()` 而非手动复制
- `concatenate`: 使用 `ndarray::concatenate()` 而非手动拼接
- `sum_axis`: 使用 `ndarray::ArrayD::sum_axis()` 而非手动迭代

### 基准测试

```bash
# 运行基准测试（需要启用 benchmark 功能）
cargo bench --features tensor
```

---

## 🔧 配置选项

### Cargo.toml

```toml
[dependencies]
# 启用 tensor 功能
ai-assistant = { path = ".", features = ["tensor"] }
```

### 编译选项

```bash
# 启用 tensor 功能编译
cargo build --features tensor

# Release 编译
cargo build --release --features tensor
```

---

## 📚 文件结构

```
src/tools/tensor/
├── mod.rs                      # 模块导出
├── core/
│   ├── mod.rs                  # 核心模块导出
│   ├── tensor.rs               # Tensor 类型（直接持有数据）
│   ├── error.rs                # TensorError 领域特定错误
│   └── metadata.rs             # OperationMetadata 操作元数据
├── backend/
│   └── backend.rs              # TensorBackend trait + NdArrayBackend
└── service/
    ├── mod.rs                  # 服务模块导出
    ├── service.rs              # TensorService 张量服务
    └── tools.rs                # TensorTools tokitai 集成
```

---

## 🆚 与旧版对比

| 特性 | 旧版 | 新版 |
|------|------|------|
| 数据存储 | GlobalTensorStore（全局单例） | Tensor 直接持有（Arc） |
| 后端抽象 | 6 个 trait（过度设计） | 1 个 trait（简化） |
| 错误处理 | anyhow::Error | TensorError（领域特定） |
| AI 理解 | 无元数据 | 完整 OperationMetadata |
| tokitai 集成 | 无 | #[tool] 宏注册 |
| 性能 | 手动循环 | ndarray 内置方法 |
| Hook 机制 | 未完成 | 已移除（简化） |

---

**最后更新**: 2026-03-18
**状态**: 重构完成 ✅
**下一步**: 编写集成测试和示例
