//! Tensor 计算模块 - 重构版
//!
//! ## 设计原则
//!
//! 1. **AI 可操作**: 所有类型和操作都有语义化元数据
//! 2. **简化架构**: 移除 GlobalTensorStore，直接持有数据
//! 3. **tokitai 集成**: 使用 #[tool] 宏注册工具
//! 4. **领域特定错误**: 明确的错误类型便于 AI 恢复
//! 5. **性能优化**: 使用 ndarray 内置方法
//!
//! ## 架构设计
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    工具层 (Tool Layer)                      │
//! │  ┌─────────────────────────────────────────────────────┐    │
//! │  │ TensorTools (tokitai #[tool] 集成)                  │    │
//! │  │ - 所有方法都暴露为 AI 可调用的工具                     │    │
//! │  │ - JSON 序列化/反序列化支持                            │    │
//! │  └─────────────────────────────────────────────────────┘    │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    服务层 (Service Layer)                   │
//! │  ┌─────────────────────────────────────────────────────┐    │
//! │  │ TensorService                                        │    │
//! │  │ - 所有张量操作的统一入口                              │    │
//! │  │ - 支持链式调用                                        │    │
//! │  └─────────────────────────────────────────────────────┘    │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    后端层 (Backend Layer)                   │
//! │  ┌─────────────────────────────────────────────────────┐    │
//! │  │ TensorBackend trait + NdArrayBackend                 │    │
//! │  │ - 单一 trait，移除过度设计的接口拆分                  │    │
//! │  │ - 使用 ndarray 内置方法优化性能                       │    │
//! │  └─────────────────────────────────────────────────────┘    │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    核心层 (Core Layer)                      │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
//! │  │ Tensor      │  │ TensorError │  │ OperationMetadata   │  │
//! │  │ 直接持有数据 │  │ 领域特定错误 │  │ AI 可理解的元数据     │  │
//! │  └─────────────┘  └─────────────┘  └─────────────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## 使用示例
//!
//! ### 直接使用 TensorService
//!
//! ```rust,no_run
//! use crate::tools::tensor::TensorService;
//!
//! fn main() -> anyhow::Result<()> {
//!     let service = TensorService::new();
//!
//!     // 创建张量
//!     let a = service.from_data(&[1.0, 2.0, 3.0, 4.0], &[2, 2])?;
//!     let b = service.from_data(&[5.0, 6.0, 7.0, 8.0], &[2, 2])?;
//!
//!     // 矩阵乘法
//!     let result = service.matmul(&a, &b)?;
//!     println!("Result: {:?}", result.as_slice());
//!
//!     // 链式调用
//!     let zeros = service.zeros(&[2, 2])?;
//!     let result = service.mul_scalar(&zeros, 2.0)?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ### 使用 tokitai 工具
//!
//! ```rust,no_run
//! use crate::tools::tensor::TensorTools;
//! use tokitai::ToolProvider;
//!
//! fn main() -> anyhow::Result<()> {
//!     let tools = TensorTools::new();
//!
//!     // 获取工具定义（发送给 AI）
//!     let definitions = TensorTools::tool_definitions();
//!
//!     // 调用工具
//!     let result = tools.zeros(vec![2, 3])?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## 核心类型
//!
//! | 类型 | 说明 |
//! |------|------|
//! | `Tensor` | 张量类型，直接持有数据（Arc<ArrayD<f64>>） |
//! | `TensorError` | 领域特定错误类型，带修复建议 |
//! | `OperationMetadata` | 操作元数据，AI 理解的语义信息 |
//! | `TensorService` | 张量服务，所有操作的统一入口 |
//! | `TensorTools` | tokitai 工具集成 |
//!
//! ## 支持的操作
//!
//! ### 创建操作
//! - `zeros` - 创建零张量
//! - `ones` - 创建一张量
//! - `randn` - 创建随机张量（标准正态分布）
//! - `from_data` - 从数据创建张量
//! - `arange` - 创建范围张量
//!
//! ### 算术操作
//! - `add` - 逐元素加法（支持广播）
//! - `sub` - 逐元素减法（支持广播）
//! - `mul` - 逐元素乘法（支持广播）
//! - `div` - 逐元素除法（支持广播）
//! - `add_scalar` - 标量加法
//! - `mul_scalar` - 标量乘法
//!
//! ### 矩阵操作
//! - `matmul` - 矩阵乘法（使用 ndarray dot 优化）
//! - `transpose` - 转置（2D）
//! - `reshape` - 重塑形状
//!
//! ### 归约操作
//! - `sum` - 沿指定维度求和
//! - `mean` - 沿指定维度求平均
//! - `max` - 沿指定维度求最大值
//! - `min` - 沿指定维度求最小值
//! - `argmax` - 沿指定维度求 argmax
//!
//! ### 索引与切片
//! - `slice` - 切片
//! - `cat` - 拼接
//! - `stack` - 堆叠
//!
//! ### 广播与变形
//! - `broadcast` - 广播到目标形状
//! - `unsqueeze` - 扩展维度
//! - `squeeze` - 压缩维度
//!
//! ### 神经网络操作
//! - `relu` - ReLU 激活函数
//! - `gelu` - GELU 激活函数
//! - `sigmoid` - Sigmoid 激活函数
//! - `layer_norm` - LayerNorm 归一化
//! - `linear` - 全连接层（线性变换）

pub mod backend;
pub mod core;
pub mod service;

// Re-export core types
pub use core::{Tensor, TensorData, TensorError, TensorResult};

pub use backend::{NdArrayBackend, TensorBackend};

pub use service::{TensorService, TensorTools};

/// 模块版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 模块名称
pub const MODULE_NAME: &str = "tensor";
