//! 简化的张量类型 - 直接持有数据，无全局存储
//!
//! 设计原则:
//! 1. 数据所有权清晰：Tensor 直接持有 Arc<ArrayD<f64>>
//! 2. 轻量级可克隆：内部使用 Arc 共享数据
//! 3. AI 友好：提供完整的元数据和序列化支持

use ndarray::ArrayD;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::error::{TensorError, TensorResult};

/// 张量数据类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DType {
    F64,
    F32,
    I64,
    I32,
}

impl DType {
    pub fn element_size(&self) -> usize {
        match self {
            DType::F64 => 8,
            DType::F32 => 4,
            DType::I64 => 8,
            DType::I32 => 4,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            DType::F64 => "f64",
            DType::F32 => "f32",
            DType::I64 => "i64",
            DType::I32 => "i32",
        }
    }
}

impl std::fmt::Display for DType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 设备类型（当前仅支持 CPU）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Device {
    #[default]
    Cpu,
}

impl std::fmt::Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cpu")
    }
}

/// 张量形状
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Shape {
    dims: Vec<usize>,
}

impl Shape {
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
    pub fn is_compatible(&self, other: &Shape) -> bool {
        self.numel() == other.numel()
    }

    /// 检查是否可以广播到目标形状
    pub fn can_broadcast_to(&self, target: &Shape) -> bool {
        let src = self.dims();
        let tgt = target.dims();

        // 目标形状维度不能少于源形状
        if tgt.len() < src.len() {
            return false;
        }

        // 从后往前检查每个维度
        for (i, &s) in src.iter().enumerate() {
            let offset = tgt.len() - src.len();
            let t = tgt[i + offset];

            // 维度必须相同或源维度为 1
            if s != t && s != 1 {
                return false;
            }
        }

        true
    }
}

impl From<Vec<usize>> for Shape {
    fn from(dims: Vec<usize>) -> Self {
        Self::new(dims)
    }
}

impl From<&[usize]> for Shape {
    fn from(dims: &[usize]) -> Self {
        Self::new(dims.to_vec())
    }
}

/// 内部数据表示
#[derive(Debug, Clone)]
pub enum TensorData {
    /// 动态维度数组（主要存储）
    Array(ArrayD<f64>),
}

impl TensorData {
    pub fn shape(&self) -> Shape {
        match self {
            TensorData::Array(arr) => Shape::new(arr.shape().to_vec()),
        }
    }

    pub fn as_slice(&self) -> Option<&[f64]> {
        match self {
            TensorData::Array(arr) => arr.as_slice(),
        }
    }

    pub fn as_slice_mut(&mut self) -> Option<&mut [f64]> {
        match self {
            TensorData::Array(arr) => arr.as_slice_mut(),
        }
    }

    pub fn numel(&self) -> usize {
        self.shape().numel()
    }
}

/// 张量类型
///
/// 这是 AI 可操作的核心类型，所有张量操作都返回 Tensor
///
/// # 设计说明
/// - 直接持有数据（Arc<ArrayD<f64>>），无全局存储依赖
/// - 轻量级可克隆（内部使用 Arc）
/// - 包含完整的元数据（dtype, device, shape）
#[derive(Debug, Clone)]
pub struct Tensor {
    data: Arc<TensorData>,
    dtype: DType,
    device: Device,
}

impl Tensor {
    /// 从 ArrayD 创建 Tensor
    pub fn from_array(array: ArrayD<f64>) -> Self {
        Self {
            data: Arc::new(TensorData::Array(array)),
            dtype: DType::F64,
            device: Device::Cpu,
        }
    }

    /// 创建零张量
    pub fn zeros(shape: &[usize]) -> Self {
        Self::from_array(ArrayD::zeros(shape.to_vec()))
    }

    /// 创建一张量
    pub fn ones(shape: &[usize]) -> Self {
        Self::from_array(ArrayD::from_elem(shape.to_vec(), 1.0f64))
    }

    /// 从数据切片创建张量
    pub fn from_data(data: &[f64], shape: &[usize]) -> TensorResult<Self> {
        let array = ArrayD::from_shape_vec(shape.to_vec(), data.to_vec()).map_err(|e| {
            TensorError::ShapeMismatch {
                message: format!("Failed to create array from data: {}", e),
            }
        })?;
        Ok(Self::from_array(array))
    }

    /// 获取数据切片
    pub fn as_slice(&self) -> Option<&[f64]> {
        self.data.as_slice()
    }

    /// 获取形状
    pub fn shape(&self) -> Shape {
        self.data.shape()
    }

    /// 获取维度
    pub fn dims(&self) -> Vec<usize> {
        self.data.shape().dims().to_vec()
    }

    /// 获取元素数量
    pub fn numel(&self) -> usize {
        self.data.numel()
    }

    /// 获取秩
    pub fn rank(&self) -> usize {
        self.data.shape().rank()
    }

    /// 获取数据类型
    pub fn dtype(&self) -> DType {
        self.dtype
    }

    /// 获取设备
    pub fn device(&self) -> Device {
        self.device
    }

    /// 检查是否为空张量
    pub fn is_empty(&self) -> bool {
        self.numel() == 0
    }

    /// 检查是否为标量
    pub fn is_scalar(&self) -> bool {
        self.data.shape().is_scalar()
    }

    /// 重塑形状
    pub fn reshape(&self, new_shape: &[usize]) -> TensorResult<Self> {
        let numel: usize = new_shape.iter().product();
        if numel != self.numel() {
            return Err(TensorError::ShapeMismatch {
                message: format!(
                    "Cannot reshape tensor with {} elements to shape {:?}",
                    self.numel(),
                    new_shape
                ),
            });
        }

        match &*self.data {
            TensorData::Array(arr) => {
                let array = arr
                    .clone()
                    .into_shape_with_order(new_shape.to_vec())
                    .map_err(|e| TensorError::ShapeMismatch {
                        message: format!("Failed to reshape: {}", e),
                    })?;
                Ok(Self::from_array(array))
            }
        }
    }

    /// 转置（仅支持 2D）
    pub fn transpose(&self) -> TensorResult<Self> {
        if self.rank() != 2 {
            return Err(TensorError::InvalidDimension {
                dim: self.rank() as i32,
                message: "transpose currently supports 2D tensors only".to_string(),
            });
        }

        match &*self.data {
            TensorData::Array(arr) => {
                let shape = arr.shape();
                let (m, n) = (shape[0], shape[1]);

                let mut result = ArrayD::zeros(vec![n, m]);
                for i in 0..m {
                    for j in 0..n {
                        result[[j, i]] = arr[[i, j]];
                    }
                }
                Ok(Self::from_array(result))
            }
        }
    }

    /// 获取内部数组引用
    pub fn as_array(&self) -> Option<&ArrayD<f64>> {
        match &*self.data {
            TensorData::Array(arr) => Some(arr),
        }
    }
}

// ========== 序列化支持 ==========

impl Serialize for Tensor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("Tensor", 3)?;
        state.serialize_field("shape", &self.shape().dims())?;
        state.serialize_field("dtype", &self.dtype.as_str())?;

        // 序列化数据
        let data = self.as_slice().unwrap_or(&[]);
        state.serialize_field("data", &data)?;

        state.end()
    }
}

// ========== 测试 ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zeros() {
        let tensor = Tensor::zeros(&[2, 3]);
        assert_eq!(tensor.dims(), &[2, 3]);
        assert_eq!(tensor.numel(), 6);
        assert!(tensor.as_slice().unwrap().iter().all(|&x| x == 0.0));
    }

    #[test]
    fn test_ones() {
        let tensor = Tensor::ones(&[2, 3]);
        assert!(tensor.as_slice().unwrap().iter().all(|&x| x == 1.0));
    }

    #[test]
    fn test_from_data() {
        let tensor = Tensor::from_data(&[1.0, 2.0, 3.0, 4.0], &[2, 2]).unwrap();
        assert_eq!(tensor.dims(), &[2, 2]);
        assert_eq!(tensor.as_slice().unwrap(), &[1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_reshape() {
        let tensor = Tensor::from_data(&[1.0, 2.0, 3.0, 4.0], &[2, 2]).unwrap();
        let reshaped = tensor.reshape(&[4]).unwrap();
        assert_eq!(reshaped.dims(), &[4]);
    }

    #[test]
    fn test_transpose() {
        let tensor = Tensor::from_data(&[1.0, 2.0, 3.0, 4.0], &[2, 2]).unwrap();
        let transposed = tensor.transpose().unwrap();
        assert_eq!(transposed.dims(), &[2, 2]);
        assert_eq!(transposed.as_slice().unwrap(), &[1.0, 3.0, 2.0, 4.0]);
    }

    #[test]
    fn test_shape_broadcast() {
        let shape1 = Shape::new(vec![3, 1]);
        let shape2 = Shape::new(vec![3, 4]);
        assert!(shape1.can_broadcast_to(&shape2));

        let shape3 = Shape::new(vec![3, 2]);
        assert!(!shape3.can_broadcast_to(&shape2));
    }
}
