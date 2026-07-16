//! 张量句柄模块 - 重构版
//!
//! 设计原则:
//! 1. 数据所有权清晰：TensorHandle 通过 Arc 共享数据
//! 2. 统一 ID 管理：所有 ID 来自全局 TensorStore
//! 3. 简化数据类型：移除冗余的 dtype/device 字段
//! 4. 易于 AI 理解：API 简洁，语义明确

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;
use parking_lot::RwLock;

// ========== 全局张量存储 ==========

/// 全局唯一的张量存储器
/// 
/// 所有 TensorHandle 的数据都存储在这里，通过 ID 引用
pub struct GlobalTensorStore {
    store: RwLock<HashMap<usize, Arc<TensorData>>>,
    next_id: AtomicUsize,
}

impl GlobalTensorStore {
    /// 创建新的张量存储器
    pub fn new() -> Self {
        Self {
            store: RwLock::new(HashMap::new()),
            next_id: AtomicUsize::new(1),
        }
    }

    /// 获取全局单例
    pub fn instance() -> &'static Self {
        static INSTANCE: OnceLock<GlobalTensorStore> = OnceLock::new();
        INSTANCE.get_or_init(|| GlobalTensorStore::new())
    }

    /// 分配新的张量 ID
    pub fn alloc_id(&self) -> usize {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// 存储张量数据
    pub fn insert(&self, id: usize, data: Arc<TensorData>) {
        self.store.write().insert(id, data);
    }

    /// 获取张量数据
    pub fn get(&self, id: usize) -> Option<Arc<TensorData>> {
        self.store.read().get(&id).cloned()
    }

    /// 移除张量数据
    pub fn remove(&self, id: usize) -> Option<Arc<TensorData>> {
        self.store.write().remove(&id)
    }

    /// 清空所有张量
    pub fn clear(&self) {
        self.store.write().clear();
        self.next_id.store(1, Ordering::Relaxed);
    }

    /// 获取存储的张量数量
    pub fn len(&self) -> usize {
        self.store.read().len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.store.read().is_empty()
    }
}

// ========== 核心数据类型 ==========

/// 张量数据类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TensorDType {
    F32,
    F64,
    F16,
    I32,
    I64,
    U8,
}

impl TensorDType {
    pub fn element_size(&self) -> usize {
        match self {
            TensorDType::F32 => 4,
            TensorDType::F64 => 8,
            TensorDType::F16 => 2,
            TensorDType::I32 => 4,
            TensorDType::I64 => 8,
            TensorDType::U8 => 1,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            TensorDType::F32 => "f32",
            TensorDType::F64 => "f64",
            TensorDType::F16 => "f16",
            TensorDType::I32 => "i32",
            TensorDType::I64 => "i64",
            TensorDType::U8 => "u8",
        }
    }
}

impl std::fmt::Display for TensorDType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 设备类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TensorDevice {
    #[default]
    Cpu,
}

impl std::fmt::Display for TensorDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cpu")
    }
}

/// 张量形状
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TensorShape {
    dims: Vec<usize>,
}

impl TensorShape {
    pub fn new(dims: Vec<usize>) -> Self {
        Self { dims }
    }

    pub fn dims(&self) -> &[usize] {
        &self.dims
    }

    pub fn numel(&self) -> usize {
        self.dims.iter().copied().product()
    }

    pub fn rank(&self) -> usize {
        self.dims.len()
    }

    pub fn is_scalar(&self) -> bool {
        self.dims.is_empty() || self.dims == [1]
    }

    /// 检查形状是否兼容（元素数量相同）
    pub fn is_compatible(&self, other: &TensorShape) -> bool {
        self.numel() == other.numel()
    }
}

/// 张量数据内部表示
/// 
/// 所有数据都存储在这里，TensorHandle 通过 Arc 引用
#[derive(Debug, Clone)]
pub enum TensorData {
    /// NdArray 后端数据（主要后端）
    NdArray {
        data: ndarray::ArrayD<f64>,
        dtype: TensorDType,
    },
    /// 原始 f64 数据（简化后端）
    Raw {
        data: Vec<f64>,
        shape: TensorShape,
        dtype: TensorDType,
    },
}

impl TensorData {
    /// 获取数据类型
    pub fn dtype(&self) -> TensorDType {
        match self {
            TensorData::NdArray { dtype, .. } => *dtype,
            TensorData::Raw { dtype, .. } => *dtype,
        }
    }

    /// 获取形状
    pub fn shape(&self) -> TensorShape {
        match self {
            TensorData::NdArray { data, .. } => TensorShape::new(data.shape().to_vec()),
            TensorData::Raw { shape, .. } => shape.clone(),
        }
    }

    /// 获取数据切片
    pub fn as_slice(&self) -> Option<&[f64]> {
        match self {
            TensorData::NdArray { data, .. } => data.as_slice(),
            TensorData::Raw { data, .. } => Some(data),
        }
    }

    /// 获取元素数量
    pub fn numel(&self) -> usize {
        self.shape().numel()
    }
}

// ========== 张量句柄 ==========

/// 张量句柄
/// 
/// 这是 AI 可操作的核心类型，所有张量操作都返回 TensorHandle
/// 
/// # 设计说明
/// - 通过 ID 引用 GlobalTensorStore 中的数据
/// - 轻量级可克隆（内部使用 Arc）
/// - 包含完整的元数据（dtype, device, shape）
#[derive(Debug, Clone)]
pub struct TensorHandle {
    /// 张量唯一 ID（引用 GlobalTensorStore）
    pub id: usize,
    /// 数据类型
    pub dtype: TensorDType,
    /// 设备类型
    pub device: TensorDevice,
    /// 形状
    pub shape: TensorShape,
    /// 数据引用（指向 GlobalTensorStore）
    data: Arc<TensorData>,
}

impl TensorHandle {
    /// 创建新的张量句柄
    /// 
    /// # 参数
    /// * `id` - 张量 ID（应来自 GlobalTensorStore::alloc_id）
    /// * `dtype` - 数据类型
    /// * `device` - 设备类型
    /// * `shape` - 形状
    /// * `data` - 张量数据
    /// 
    /// # 返回值
    /// 新的 TensorHandle
    pub fn new(
        id: usize,
        dtype: TensorDType,
        device: TensorDevice,
        shape: TensorShape,
        data: TensorData,
    ) -> Self {
        let data = Arc::new(data);
        Self { id, dtype, device, shape, data }
    }

    /// 从 GlobalTensorStore 创建句柄
    /// 
    /// # 参数
    /// * `id` - 张量 ID
    /// 
    /// # 返回值
    /// 如果 ID 存在则返回 TensorHandle，否则返回 None
    pub fn from_store(id: usize) -> Option<Self> {
        let data = GlobalTensorStore::instance().get(id)?;
        let shape = data.shape();
        Some(Self {
            id,
            dtype: data.dtype(),
            device: TensorDevice::Cpu,
            shape,
            data,
        })
    }

    /// 获取数据引用
    pub fn data(&self) -> &Arc<TensorData> {
        &self.data
    }

    /// 获取数据切片
    pub fn as_slice(&self) -> Option<&[f64]> {
        self.data.as_slice()
    }

    /// 获取元素数量
    pub fn numel(&self) -> usize {
        self.shape.numel()
    }

    /// 获取秩
    pub fn rank(&self) -> usize {
        self.shape.rank()
    }

    /// 获取维度
    pub fn dims(&self) -> &[usize] {
        self.shape.dims()
    }

    /// 检查是否为空张量
    pub fn is_empty(&self) -> bool {
        self.numel() == 0
    }

    /// 存储到 GlobalTensorStore
    pub fn store(&self) {
        GlobalTensorStore::instance().insert(self.id, self.data.clone());
    }

    /// 从存储中移除
    pub fn remove_from_store(&self) -> Option<Arc<TensorData>> {
        GlobalTensorStore::instance().remove(self.id)
    }
}

// ========== 张量构建器 ==========

/// 张量构建器，支持链式调用
pub struct TensorBuilder {
    dtype: TensorDType,
    device: TensorDevice,
    shape: TensorShape,
}

impl TensorBuilder {
    pub fn new(shape: Vec<usize>) -> Self {
        Self {
            dtype: TensorDType::F64,
            device: TensorDevice::Cpu,
            shape: TensorShape::new(shape),
        }
    }

    pub fn dtype(mut self, dtype: TensorDType) -> Self {
        self.dtype = dtype;
        self
    }

    pub fn device(mut self, device: TensorDevice) -> Self {
        self.device = device;
        self
    }

    pub fn build_shape(self) -> TensorShape {
        self.shape
    }

    pub fn dtype_value(&self) -> TensorDType {
        self.dtype
    }

    pub fn device_value(&self) -> TensorDevice {
        self.device
    }
}

// ========== 工具函数 ==========

/// 创建新的 TensorHandle 并自动存储
/// 
/// 这是推荐的使用方式，避免手动管理 ID
pub fn create_tensor_handle(
    dtype: TensorDType,
    device: TensorDevice,
    shape: TensorShape,
    data: TensorData,
) -> TensorHandle {
    let store = GlobalTensorStore::instance();
    let id = store.alloc_id();
    let handle = TensorHandle::new(id, dtype, device, shape, data);
    handle.store();
    handle
}

// ========== 测试 ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tensor_shape() {
        let shape = TensorShape::new(vec![2, 3, 4]);
        assert_eq!(shape.dims(), &[2, 3, 4]);
        assert_eq!(shape.numel(), 24);
        assert_eq!(shape.rank(), 3);
        assert!(!shape.is_scalar());
    }

    #[test]
    fn test_tensor_dtype() {
        assert_eq!(TensorDType::F32.element_size(), 4);
        assert_eq!(TensorDType::F64.element_size(), 8);
        assert_eq!(TensorDType::F16.element_size(), 2);
        assert_eq!(TensorDType::F64.as_str(), "f64");
    }

    #[test]
    fn test_global_tensor_store() {
        let store = GlobalTensorStore::instance();
        
        // 分配 ID
        let id1 = store.alloc_id();
        let id2 = store.alloc_id();
        assert_eq!(id1 + 1, id2);

        // 存储数据
        let data = Arc::new(TensorData::Raw {
            data: vec![1.0, 2.0, 3.0],
            shape: TensorShape::new(vec![3]),
            dtype: TensorDType::F64,
        });
        store.insert(id1, data.clone());

        // 获取数据
        let retrieved = store.get(id1);
        assert!(retrieved.is_some());
        assert!(Arc::ptr_eq(&data, &retrieved.unwrap()));

        // 移除数据
        store.remove(id1);
        assert!(store.get(id1).is_none());
    }

    #[test]
    fn test_tensor_handle_creation() {
        let shape = TensorShape::new(vec![2, 3]);
        let data = TensorData::Raw {
            data: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
            shape: shape.clone(),
            dtype: TensorDType::F64,
        };

        let handle = create_tensor_handle(
            TensorDType::F64,
            TensorDevice::Cpu,
            shape,
            data,
        );

        assert_eq!(handle.dims(), &[2, 3]);
        assert_eq!(handle.numel(), 6);
        assert_eq!(handle.dtype, TensorDType::F64);
        assert!(!handle.is_empty());
    }

    #[test]
    fn test_tensor_handle_from_store() {
        let shape = TensorShape::new(vec![2]);
        let data = TensorData::Raw {
            data: vec![1.0, 2.0],
            shape: shape.clone(),
            dtype: TensorDType::F64,
        };

        let handle = create_tensor_handle(
            TensorDType::F64,
            TensorDevice::Cpu,
            shape,
            data,
        );

        // 从 store 重新获取
        let retrieved = TensorHandle::from_store(handle.id);
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, handle.id);
        assert_eq!(retrieved.dims(), handle.dims());
    }

    #[test]
    fn test_tensor_data_methods() {
        let data = TensorData::Raw {
            data: vec![1.0, 2.0, 3.0, 4.0],
            shape: TensorShape::new(vec![2, 2]),
            dtype: TensorDType::F64,
        };

        assert_eq!(data.dtype(), TensorDType::F64);
        assert_eq!(data.shape().dims(), &[2, 2]);
        assert_eq!(data.numel(), 4);
        assert_eq!(data.as_slice(), Some(&[1.0, 2.0, 3.0, 4.0][..]));
    }

    #[test]
    fn test_empty_tensor() {
        let shape = TensorShape::new(vec![]);
        let data = TensorData::Raw {
            data: vec![],
            shape: shape.clone(),
            dtype: TensorDType::F64,
        };

        let handle = create_tensor_handle(
            TensorDType::F64,
            TensorDevice::Cpu,
            shape,
            data,
        );

        assert!(handle.is_empty());
        assert_eq!(handle.numel(), 0);
    }
}
