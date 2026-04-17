//! 张量服务实现
//!
//! 设计原则:
//! 1. 简化设计：移除不必要的状态管理
//! 2. 基于 NdArrayBackend
//! 3. 支持链式调用

use crate::tools::tensor::backend::{NdArrayBackend, TensorBackend};
use crate::tools::tensor::core::{Tensor, TensorError, TensorResult};

/// 张量服务
///
/// 提供所有张量操作的统一入口
/// 所有操作都返回 Tensor，支持链式调用
pub struct TensorService {
    backend: NdArrayBackend,
}

impl TensorService {
    /// 创建新的张量服务
    pub fn new() -> Self {
        Self {
            backend: NdArrayBackend::new(),
        }
    }

    /// 获取后端名称
    pub fn backend_name(&self) -> &str {
        self.backend.name()
    }

    // ========== 创建操作 ==========

    /// 创建零张量
    pub fn zeros(&self, shape: &[usize]) -> TensorResult<Tensor> {
        self.backend.zeros(shape)
    }

    /// 创建一张量
    pub fn ones(&self, shape: &[usize]) -> TensorResult<Tensor> {
        self.backend.ones(shape)
    }

    /// 创建随机张量（标准正态分布）
    pub fn randn(&self, shape: &[usize]) -> TensorResult<Tensor> {
        self.backend.randn(shape)
    }

    /// 从数据创建张量
    pub fn from_data(&self, data: &[f64], shape: &[usize]) -> TensorResult<Tensor> {
        self.backend.from_data(data, shape)
    }

    /// 创建范围张量
    pub fn arange(&self, start: f64, end: f64, step: f64) -> TensorResult<Tensor> {
        self.backend.arange(start, end, step)
    }

    // ========== 算术操作 ==========

    /// 逐元素加法
    pub fn add(&self, a: &Tensor, b: &Tensor) -> TensorResult<Tensor> {
        self.backend.add(a, b)
    }

    /// 逐元素减法
    pub fn sub(&self, a: &Tensor, b: &Tensor) -> TensorResult<Tensor> {
        self.backend.sub(a, b)
    }

    /// 逐元素乘法
    pub fn mul(&self, a: &Tensor, b: &Tensor) -> TensorResult<Tensor> {
        self.backend.mul(a, b)
    }

    /// 逐元素除法
    pub fn div(&self, a: &Tensor, b: &Tensor) -> TensorResult<Tensor> {
        self.backend.div(a, b)
    }

    /// 标量加法
    pub fn add_scalar(&self, tensor: &Tensor, value: f64) -> TensorResult<Tensor> {
        self.backend.add_scalar(tensor, value)
    }

    /// 标量乘法
    pub fn mul_scalar(&self, tensor: &Tensor, value: f64) -> TensorResult<Tensor> {
        self.backend.mul_scalar(tensor, value)
    }

    // ========== 矩阵操作 ==========

    /// 矩阵乘法
    pub fn matmul(&self, a: &Tensor, b: &Tensor) -> TensorResult<Tensor> {
        self.backend.matmul(a, b)
    }

    /// 转置
    pub fn transpose(&self, tensor: &Tensor) -> TensorResult<Tensor> {
        self.backend.transpose(tensor)
    }

    /// 重塑形状
    pub fn reshape(&self, tensor: &Tensor, shape: &[usize]) -> TensorResult<Tensor> {
        self.backend.reshape(tensor, shape)
    }

    // ========== 归约操作 ==========

    /// 求和
    pub fn sum(&self, tensor: &Tensor, dims: &[usize]) -> TensorResult<Tensor> {
        self.backend.sum(tensor, dims)
    }

    /// 平均值
    pub fn mean(&self, tensor: &Tensor, dims: &[usize]) -> TensorResult<Tensor> {
        self.backend.mean(tensor, dims)
    }

    /// 最大值
    pub fn max(&self, tensor: &Tensor, dims: &[usize]) -> TensorResult<Tensor> {
        self.backend.max(tensor, dims)
    }

    /// 最小值
    pub fn min(&self, tensor: &Tensor, dims: &[usize]) -> TensorResult<Tensor> {
        self.backend.min(tensor, dims)
    }

    /// Argmax
    pub fn argmax(&self, tensor: &Tensor, dim: usize) -> TensorResult<Tensor> {
        self.backend.argmax(tensor, dim)
    }

    // ========== 索引与切片 ==========

    /// 切片
    pub fn slice(&self, tensor: &Tensor, ranges: &[(usize, usize)]) -> TensorResult<Tensor> {
        self.backend.slice(tensor, ranges)
    }

    /// 拼接
    pub fn cat(&self, tensors: &[&Tensor], dim: usize) -> TensorResult<Tensor> {
        self.backend.cat(tensors, dim)
    }

    /// 堆叠
    pub fn stack(&self, tensors: &[&Tensor], dim: usize) -> TensorResult<Tensor> {
        self.backend.stack(tensors, dim)
    }

    // ========== 广播与变形 ==========

    /// 广播到目标形状
    pub fn broadcast(&self, tensor: &Tensor, shape: &[usize]) -> TensorResult<Tensor> {
        self.backend.broadcast(tensor, shape)
    }

    /// 扩展维度
    pub fn unsqueeze(&self, tensor: &Tensor, dim: usize) -> TensorResult<Tensor> {
        self.backend.unsqueeze(tensor, dim)
    }

    /// 压缩维度
    pub fn squeeze(&self, tensor: &Tensor, dim: Option<usize>) -> TensorResult<Tensor> {
        self.backend.squeeze(tensor, dim)
    }

    // ========== 神经网络操作 ==========

    /// ReLU 激活
    pub fn relu(&self, input: &Tensor) -> TensorResult<Tensor> {
        let data = input
            .as_slice()
            .ok_or_else(|| TensorError::other("Cannot get tensor data"))?;
        let output: Vec<f64> = data.iter().map(|&x| x.max(0.0)).collect();
        Tensor::from_data(&output, &input.dims())
    }

    /// GELU 激活（近似）
    pub fn gelu(&self, input: &Tensor) -> TensorResult<Tensor> {
        const SQRT_2_PI: f64 = 0.7978845608028654;
        const COEF: f64 = 0.044715;

        let data = input
            .as_slice()
            .ok_or_else(|| TensorError::other("Cannot get tensor data"))?;
        let output: Vec<f64> = data
            .iter()
            .map(|&x| {
                let x3 = x * x * x;
                let inner = SQRT_2_PI * (x + COEF * x3);
                0.5 * x * (1.0 + inner.tanh())
            })
            .collect();
        Tensor::from_data(&output, &input.dims())
    }

    /// Sigmoid 激活
    pub fn sigmoid(&self, input: &Tensor) -> TensorResult<Tensor> {
        let data = input
            .as_slice()
            .ok_or_else(|| TensorError::other("Cannot get tensor data"))?;
        let output: Vec<f64> = data.iter().map(|&x| 1.0 / (1.0 + (-x).exp())).collect();
        Tensor::from_data(&output, &input.dims())
    }

    /// 全连接层（线性变换）
    pub fn linear(
        &self,
        input: &Tensor,
        weight: &Tensor,
        bias: Option<&Tensor>,
    ) -> TensorResult<Tensor> {
        // output = input @ weight.T + bias
        let weight_t = self.transpose(weight)?;
        let mut output = self.matmul(input, &weight_t)?;

        if let Some(b) = bias {
            output = self.add(&output, b)?;
        }

        Ok(output)
    }

    /// LayerNorm
    pub fn layer_norm(
        &self,
        input: &Tensor,
        normalized_shape: usize,
        eps: f64,
    ) -> TensorResult<Tensor> {
        let data = input
            .as_slice()
            .ok_or_else(|| TensorError::other("Cannot get tensor data"))?;
        let n = normalized_shape;

        if data.len() % n != 0 {
            return Err(TensorError::ShapeMismatch {
                message: format!(
                    "Input size {} is not divisible by normalized_shape {}",
                    data.len(),
                    n
                ),
            });
        }

        let batch_size = data.len() / n;
        let mut output_data = Vec::with_capacity(data.len());

        for b in 0..batch_size {
            let slice = &data[b * n..(b + 1) * n];
            let mean = slice.iter().sum::<f64>() / n as f64;
            let variance = slice.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / n as f64;
            let std = (variance + eps).sqrt();

            for &x in slice {
                output_data.push((x - mean) / std);
            }
        }

        Tensor::from_data(&output_data, &input.dims())
    }
}

impl Default for TensorService {
    fn default() -> Self {
        Self::new()
    }
}

// ========== Clone 实现 ==========

impl Clone for TensorService {
    fn clone(&self) -> Self {
        Self::new()
    }
}

// ========== 测试 ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zeros() {
        let service = TensorService::new();
        let tensor = service.zeros(&[2, 3]).unwrap();
        assert_eq!(tensor.dims(), &[2, 3]);
    }

    #[test]
    fn test_matmul() {
        let service = TensorService::new();
        let a = service
            .from_data(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[2, 3])
            .unwrap();
        let b = service
            .from_data(&[7.0, 8.0, 9.0, 10.0, 11.0, 12.0], &[3, 2])
            .unwrap();
        let result = service.matmul(&a, &b).unwrap();
        assert_eq!(result.dims(), &[2, 2]);
        assert_eq!(result.as_slice().unwrap(), &[58.0, 64.0, 139.0, 154.0]);
    }

    #[test]
    fn test_relu() {
        let service = TensorService::new();
        let input = service
            .from_data(&[-2.0, -1.0, 0.0, 1.0, 2.0], &[5])
            .unwrap();
        let output = service.relu(&input).unwrap();
        assert_eq!(output.as_slice().unwrap(), &[0.0, 0.0, 0.0, 1.0, 2.0]);
    }

    #[test]
    fn test_chain_operations() {
        let service = TensorService::new();
        let zeros = service.zeros(&[2, 2]).unwrap();
        let added = service.add_scalar(&zeros, 1.0).unwrap();
        let multiplied = service.mul_scalar(&added, 2.0).unwrap();
        assert_eq!(multiplied.as_slice().unwrap(), &[2.0, 2.0, 2.0, 2.0]);
    }

    #[test]
    fn test_layer_norm() {
        let service = TensorService::new();
        let input = service.from_data(&[1.0, 2.0, 3.0, 4.0], &[1, 4]).unwrap();
        let output = service.layer_norm(&input, 4, 1e-5).unwrap();
        assert_eq!(output.dims(), &[1, 4]);
    }
}
