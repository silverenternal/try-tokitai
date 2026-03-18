//! 张量微服务模块
//!
//! 设计原则:
//! 1. tokitai 工具集成：使用 #[tool] 宏注册
//! 2. AI 可理解：操作有完整的元数据描述
//! 3. 同步操作：移除不必要的 async

pub mod service;
pub mod tools;

pub use service::TensorService;
pub use tools::TensorTools;
