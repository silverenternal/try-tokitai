//! 核心类型和元数据模块
//!
//! 设计原则:
//! 1. AI 可理解：所有类型都有语义化描述
//! 2. 简化数据所有权：直接持有数据，移除全局存储
//! 3. 领域特定错误：明确的错误类型便于 AI 恢复

pub mod tensor;
pub mod error;
pub mod metadata;

pub use tensor::{Tensor, TensorData, Shape, DType, Device};
pub use error::{TensorError, TensorResult};
pub use metadata::{OperationMetadata, OperationCategory};
