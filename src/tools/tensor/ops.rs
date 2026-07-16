//! 张量操作服务 - 重构版
//!
//! 设计原则:
//! 1. 同步操作：移除不必要的 async
//! 2. 组合后端 trait：按需使用
//! 3. 统一使用 GlobalTensorStore

use anyhow::Result;
use crate::tools::tensor::backend::{
    TensorBackend, CreationBackend, ArithmeticBackend,
};
use crate::tools::tensor::tensor_handle::TensorHandle;

/// 张量操作服务
///
/// 封装张量操作，提供统一的 API
/// 所有操作都返回 TensorHandle，支持链式调用
pub struct TensorOps<B: TensorBackend> {
    backend: B,
}

impl<B: TensorBackend> TensorOps<B> {
    /// 创建新的张量操作服务
    pub fn new(backend: B) -> Self {
        Self { backend }
    }

    /// 获取后端引用
    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// 获取后端名称
    pub fn backend_name(&self) -> &str {
        self.backend.backend_name()
    }

    // ========== 创建操作 ==========

    /// 创建零张量
    pub fn zeros(&self, shape: &[usize]) -> Result<TensorHandle> {
        self.backend.zeros(shape)
    }

    /// 创建一张量
    pub fn ones(&self, shape: &[usize]) -> Result<TensorHandle> {
        self.backend.ones(shape)
    }

    /// 创建随机张量
    pub fn randn(&self, shape: &[usize]) -> Result<TensorHandle> {
        self.backend.randn(shape)
    }

    /// 从数据创建张量
    pub fn from_data(&self, data: &[f64], shape: &[usize]) -> Result<TensorHandle> {
        self.backend.from_data(data, shape)
    }

    /// 创建范围张量
    pub fn arange(&self, start: f64, end: f64, step: f64) -> Result<TensorHandle> {
        self.backend.arange(start, end, step)
    }

    // ========== 算术操作 ==========

    /// 加法
    pub fn add(&self, a: &TensorHandle, b: &TensorHandle) -> Result<TensorHandle> {
        self.backend.add(a, b)
    }

    /// 减法
    pub fn sub(&self, a: &TensorHandle, b: &TensorHandle) -> Result<TensorHandle> {
        self.backend.sub(a, b)
    }

    /// 乘法
    pub fn mul(&self, a: &TensorHandle, b: &TensorHandle) -> Result<TensorHandle> {
        self.backend.mul(a, b)
    }

    /// 除法
    pub fn div(&self, a: &TensorHandle, b: &TensorHandle) -> Result<TensorHandle> {
        self.backend.div(a, b)
    }

    /// 标量加法
    pub fn add_scalar(&self, tensor: &TensorHandle, value: f64) -> Result<TensorHandle> {
        self.backend.add_scalar(tensor, value)
    }

    /// 标量乘法
    pub fn mul_scalar(&self, tensor: &TensorHandle, value: f64) -> Result<TensorHandle> {
        self.backend.mul_scalar(tensor, value)
    }

    // ========== 矩阵操作 ==========

    /// 矩阵乘法
    pub fn matmul(&self, a: &TensorHandle, b: &TensorHandle) -> Result<TensorHandle> {
        self.backend.matmul(a, b)
    }

    /// 转置
    pub fn transpose(&self, tensor: &TensorHandle, axes: Option<&[usize]>) -> Result<TensorHandle> {
        self.backend.transpose(tensor, axes)
    }

    /// 重塑
    pub fn reshape(&self, tensor: &TensorHandle, shape: &[usize]) -> Result<TensorHandle> {
        self.backend.reshape(tensor, shape)
    }

    // ========== 归约操作 ==========

    /// 求和
    pub fn sum(&self, tensor: &TensorHandle, dims: &[usize]) -> Result<TensorHandle> {
        self.backend.sum(tensor, dims)
    }

    /// 平均值
    pub fn mean(&self, tensor: &TensorHandle, dims: &[usize]) -> Result<TensorHandle> {
        self.backend.mean(tensor, dims)
    }

    /// 最大值
    pub fn max(&self, tensor: &TensorHandle, dims: &[usize]) -> Result<TensorHandle> {
        self.backend.max(tensor, dims)
    }

    /// 最小值
    pub fn min(&self, tensor: &TensorHandle, dims: &[usize]) -> Result<TensorHandle> {
        self.backend.min(tensor, dims)
    }

    /// Argmax
    pub fn argmax(&self, tensor: &TensorHandle, dim: usize) -> Result<TensorHandle> {
        self.backend.argmax(tensor, dim)
    }

    // ========== 索引与切片 ==========

    /// 切片
    pub fn slice(&self, tensor: &TensorHandle, ranges: &[(usize, usize)]) -> Result<TensorHandle> {
        self.backend.slice(tensor, ranges)
    }

    /// 拼接
    pub fn cat(&self, tensors: &[&TensorHandle], dim: usize) -> Result<TensorHandle> {
        self.backend.cat(tensors, dim)
    }

    /// 堆叠
    pub fn stack(&self, tensors: &[&TensorHandle], dim: usize) -> Result<TensorHandle> {
        self.backend.stack(tensors, dim)
    }

    // ========== 广播与扩展 ==========

    /// 广播
    pub fn broadcast(&self, tensor: &TensorHandle, shape: &[usize]) -> Result<TensorHandle> {
        self.backend.broadcast(tensor, shape)
    }

    /// 扩展维度
    pub fn unsqueeze(&self, tensor: &TensorHandle, dim: usize) -> Result<TensorHandle> {
        self.backend.unsqueeze(tensor, dim)
    }

    /// 压缩维度
    pub fn squeeze(&self, tensor: &TensorHandle, dim: Option<usize>) -> Result<TensorHandle> {
        self.backend.squeeze(tensor, dim)
    }
}

impl<B: TensorBackend + Default> Default for TensorOps<B> {
    fn default() -> Self {
        Self::new(B::default())
    }
}

// ========== 便捷函数 ==========

/// 创建零张量（使用默认后端）
pub fn zeros(shape: &[usize]) -> Result<TensorHandle> {
    use crate::tools::tensor::backend::NdArrayBackend;
    let ops = TensorOps::<NdArrayBackend>::default();
    ops.zeros(shape)
}

/// 创建一张量（使用默认后端）
pub fn ones(shape: &[usize]) -> Result<TensorHandle> {
    use crate::tools::tensor::backend::NdArrayBackend;
    let ops = TensorOps::<NdArrayBackend>::default();
    ops.ones(shape)
}

/// 矩阵乘法（使用默认后端）
pub fn matmul(a: &TensorHandle, b: &TensorHandle) -> Result<TensorHandle> {
    use crate::tools::tensor::backend::NdArrayBackend;
    let ops = TensorOps::<NdArrayBackend>::default();
    ops.matmul(a, b)
}

// ========== 测试 ==========

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::tensor::backend::NdArrayBackend;

    #[test]
    fn test_zeros() {
        let ops = TensorOps::<NdArrayBackend>::default();
        let tensor = ops.zeros(&[2, 3]).unwrap();
        assert_eq!(tensor.dims(), &[2, 3]);
        assert_eq!(tensor.numel(), 6);
    }

    #[test]
    fn test_ones() {
        let ops = TensorOps::<NdArrayBackend>::default();
        let tensor = ops.ones(&[2, 3]).unwrap();
        let slice = tensor.as_slice().unwrap();
        assert!(slice.iter().all(|&x| x == 1.0));
    }

    #[test]
    fn test_add() {
        let ops = TensorOps::<NdArrayBackend>::default();
        let a = ops.from_data(&[1.0, 2.0, 3.0], &[3]).unwrap();
        let b = ops.from_data(&[4.0, 5.0, 6.0], &[3]).unwrap();
        let result = ops.add(&a, &b).unwrap();
        let slice = result.as_slice().unwrap();
        assert_eq!(slice, &[5.0, 7.0, 9.0]);
    }

    #[test]
    fn test_mul_scalar() {
        let ops = TensorOps::<NdArrayBackend>::default();
        let tensor = ops.from_data(&[1.0, 2.0, 3.0], &[3]).unwrap();
        let result = ops.mul_scalar(&tensor, 2.0).unwrap();
        let slice = result.as_slice().unwrap();
        assert_eq!(slice, &[2.0, 4.0, 6.0]);
    }

    #[test]
    fn test_sum() {
        let ops = TensorOps::<NdArrayBackend>::default();
        let tensor = ops.from_data(&[1.0, 2.0, 3.0, 4.0], &[2, 2]).unwrap();
        let result = ops.sum(&tensor, &[0]).unwrap();
        let slice = result.as_slice().unwrap();
        assert_eq!(slice, &[4.0, 6.0]);
    }

    #[test]
    fn test_matmul() {
        let ops = TensorOps::<NdArrayBackend>::default();
        let a = ops.from_data(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[2, 3]).unwrap();
        let b = ops.from_data(&[7.0, 8.0, 9.0, 10.0, 11.0, 12.0], &[3, 2]).unwrap();
        let result = ops.matmul(&a, &b).unwrap();
        assert_eq!(result.dims(), &[2, 2]);
    }

    #[test]
    fn test_chain_operations() {
        let ops = TensorOps::<NdArrayBackend>::default();
        let zeros = ops.zeros(&[2, 2]).unwrap();
        let added = ops.add_scalar(&zeros, 1.0).unwrap();
        let multiplied = ops.mul_scalar(&added, 2.0).unwrap();
        let slice = multiplied.as_slice().unwrap();
        assert_eq!(slice, &[2.0, 2.0, 2.0, 2.0]);
    }
}
