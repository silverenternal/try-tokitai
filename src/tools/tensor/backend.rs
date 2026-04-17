//! 简化的张量后端
//!
//! 设计原则:
//! 1. 单一 trait：移除过度设计的接口拆分
//! 2. 使用 ndarray 作为主要后端
//! 3. 性能优化：使用 ndarray 内置方法

use crate::tools::tensor::core::{DType, Device, Shape, Tensor, TensorError, TensorResult};
use ndarray::ArrayD;

/// 张量后端 trait
///
/// 所有张量操作的统一接口
pub trait TensorBackend: Send + Sync {
    /// 获取后端名称
    fn name(&self) -> &str;

    // ========== 创建操作 ==========

    /// 创建零张量
    fn zeros(&self, shape: &[usize]) -> TensorResult<Tensor>;

    /// 创建一张量
    fn ones(&self, shape: &[usize]) -> TensorResult<Tensor>;

    /// 创建随机张量（标准正态分布）
    fn randn(&self, shape: &[usize]) -> TensorResult<Tensor>;

    /// 从数据创建张量
    fn from_data(&self, data: &[f64], shape: &[usize]) -> TensorResult<Tensor>;

    /// 创建范围张量
    fn arange(&self, start: f64, end: f64, step: f64) -> TensorResult<Tensor>;

    // ========== 算术操作 ==========

    /// 逐元素加法
    fn add(&self, a: &Tensor, b: &Tensor) -> TensorResult<Tensor>;

    /// 逐元素减法
    fn sub(&self, a: &Tensor, b: &Tensor) -> TensorResult<Tensor>;

    /// 逐元素乘法
    fn mul(&self, a: &Tensor, b: &Tensor) -> TensorResult<Tensor>;

    /// 逐元素除法
    fn div(&self, a: &Tensor, b: &Tensor) -> TensorResult<Tensor>;

    /// 标量加法
    fn add_scalar(&self, tensor: &Tensor, value: f64) -> TensorResult<Tensor>;

    /// 标量乘法
    fn mul_scalar(&self, tensor: &Tensor, value: f64) -> TensorResult<Tensor>;

    // ========== 矩阵操作 ==========

    /// 矩阵乘法
    fn matmul(&self, a: &Tensor, b: &Tensor) -> TensorResult<Tensor>;

    /// 转置（2D）
    fn transpose(&self, tensor: &Tensor) -> TensorResult<Tensor>;

    /// 重塑形状
    fn reshape(&self, tensor: &Tensor, shape: &[usize]) -> TensorResult<Tensor>;

    // ========== 归约操作 ==========

    /// 求和
    fn sum(&self, tensor: &Tensor, dims: &[usize]) -> TensorResult<Tensor>;

    /// 平均值
    fn mean(&self, tensor: &Tensor, dims: &[usize]) -> TensorResult<Tensor>;

    /// 最大值
    fn max(&self, tensor: &Tensor, dims: &[usize]) -> TensorResult<Tensor>;

    /// 最小值
    fn min(&self, tensor: &Tensor, dims: &[usize]) -> TensorResult<Tensor>;

    /// Argmax
    fn argmax(&self, tensor: &Tensor, dim: usize) -> TensorResult<Tensor>;

    // ========== 索引与切片 ==========

    /// 切片
    fn slice(&self, tensor: &Tensor, ranges: &[(usize, usize)]) -> TensorResult<Tensor>;

    /// 拼接
    fn cat(&self, tensors: &[&Tensor], dim: usize) -> TensorResult<Tensor>;

    /// 堆叠
    fn stack(&self, tensors: &[&Tensor], dim: usize) -> TensorResult<Tensor>;

    // ========== 广播与变形 ==========

    /// 广播到目标形状
    fn broadcast(&self, tensor: &Tensor, shape: &[usize]) -> TensorResult<Tensor>;

    /// 扩展维度
    fn unsqueeze(&self, tensor: &Tensor, dim: usize) -> TensorResult<Tensor>;

    /// 压缩维度
    fn squeeze(&self, tensor: &Tensor, dim: Option<usize>) -> TensorResult<Tensor>;
}

// ========== NdArray 后端实现 ==========

/// NdArray 后端
///
/// 使用 ndarray 作为底层存储
#[derive(Clone, Default)]
pub struct NdArrayBackend;

impl NdArrayBackend {
    pub fn new() -> Self {
        Self
    }

    /// 辅助函数：检查两个形状是否可以广播
    fn broadcast_shapes(a: &[usize], b: &[usize]) -> Result<Vec<usize>, TensorError> {
        let (a, b) = if a.len() >= b.len() { (a, b) } else { (b, a) };

        let mut result = a.to_vec();
        let offset = a.len() - b.len();

        for (i, &b_dim) in b.iter().enumerate() {
            let a_dim = result[i + offset];
            if a_dim == b_dim || b_dim == 1 {
                result[i + offset] = a_dim.max(b_dim);
            } else if a_dim == 1 {
                result[i + offset] = b_dim;
            } else {
                return Err(TensorError::broadcast_error(format!(
                    "Cannot broadcast shapes {:?} and {:?}",
                    a, b
                )));
            }
        }

        Ok(result)
    }

    /// 辅助函数：广播数组到目标形状
    fn broadcast_array(
        arr: &ArrayD<f64>,
        target_shape: &[usize],
    ) -> Result<ArrayD<f64>, TensorError> {
        let current_shape = arr.shape();

        // 如果形状相同，直接返回
        if current_shape == target_shape {
            return Ok(arr.clone());
        }

        // 标量广播
        if arr.len() == 1 {
            let value = arr[&ndarray::IxDyn(&[0])];
            return Ok(ArrayD::from_elem(target_shape.to_vec(), value));
        }

        // 简化实现：使用 ndarray 的广播功能
        let target_dim = ndarray::IxDyn::from(target_shape.to_vec());

        arr.broadcast(target_dim)
            .map(|view| view.to_owned())
            .ok_or_else(|| {
                TensorError::broadcast_error(format!(
                    "Cannot broadcast shape {:?} to {:?}",
                    current_shape, target_shape
                ))
            })
    }
}

impl TensorBackend for NdArrayBackend {
    fn name(&self) -> &str {
        "NdArray"
    }

    fn zeros(&self, shape: &[usize]) -> TensorResult<Tensor> {
        Ok(Tensor::zeros(shape))
    }

    fn ones(&self, shape: &[usize]) -> TensorResult<Tensor> {
        Ok(Tensor::ones(shape))
    }

    fn randn(&self, shape: &[usize]) -> TensorResult<Tensor> {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let numel: usize = shape.iter().product();
        let data: Vec<f64> = (0..numel)
            .map(|_| {
                // Box-Muller 变换生成标准正态分布
                let u1: f64 = rng.gen_range(1e-10..1.0);
                let u2: f64 = rng.gen_range(0.0..1.0);
                (2.0 * u1.ln() * std::f64::consts::PI).sqrt()
                    * (2.0 * std::f64::consts::PI * u2).cos()
            })
            .collect();

        Tensor::from_data(&data, shape)
    }

    fn from_data(&self, data: &[f64], shape: &[usize]) -> TensorResult<Tensor> {
        Tensor::from_data(data, shape)
    }

    fn arange(&self, start: f64, end: f64, step: f64) -> TensorResult<Tensor> {
        let mut values = Vec::new();
        let mut current = start;
        while current < end {
            values.push(current);
            current += step;
        }

        let shape = vec![values.len()];
        Tensor::from_data(&values, &shape)
    }

    fn add(&self, a: &Tensor, b: &Tensor) -> TensorResult<Tensor> {
        let a_arr = a
            .as_array()
            .ok_or_else(|| TensorError::other("Cannot get array from tensor a"))?;
        let b_arr = b
            .as_array()
            .ok_or_else(|| TensorError::other("Cannot get array from tensor b"))?;

        // 计算广播后的形状
        let target_shape = Self::broadcast_shapes(&a.dims(), &b.dims())?;

        // 广播到相同形状
        let a_broadcast = Self::broadcast_array(a_arr, &target_shape)?;
        let b_broadcast = Self::broadcast_array(b_arr, &target_shape)?;

        Ok(Tensor::from_array(a_broadcast + b_broadcast))
    }

    fn sub(&self, a: &Tensor, b: &Tensor) -> TensorResult<Tensor> {
        let a_arr = a
            .as_array()
            .ok_or_else(|| TensorError::other("Cannot get array from tensor a"))?;
        let b_arr = b
            .as_array()
            .ok_or_else(|| TensorError::other("Cannot get array from tensor b"))?;

        let target_shape = Self::broadcast_shapes(&a.dims(), &b.dims())?;
        let a_broadcast = Self::broadcast_array(a_arr, &target_shape)?;
        let b_broadcast = Self::broadcast_array(b_arr, &target_shape)?;

        Ok(Tensor::from_array(a_broadcast - b_broadcast))
    }

    fn mul(&self, a: &Tensor, b: &Tensor) -> TensorResult<Tensor> {
        let a_arr = a
            .as_array()
            .ok_or_else(|| TensorError::other("Cannot get array from tensor a"))?;
        let b_arr = b
            .as_array()
            .ok_or_else(|| TensorError::other("Cannot get array from tensor b"))?;

        let target_shape = Self::broadcast_shapes(&a.dims(), &b.dims())?;
        let a_broadcast = Self::broadcast_array(a_arr, &target_shape)?;
        let b_broadcast = Self::broadcast_array(b_arr, &target_shape)?;

        Ok(Tensor::from_array(a_broadcast * b_broadcast))
    }

    fn div(&self, a: &Tensor, b: &Tensor) -> TensorResult<Tensor> {
        let a_arr = a
            .as_array()
            .ok_or_else(|| TensorError::other("Cannot get array from tensor a"))?;
        let b_arr = b
            .as_array()
            .ok_or_else(|| TensorError::other("Cannot get array from tensor b"))?;

        // 检查除零
        if b_arr.iter().any(|&x| x == 0.0) {
            return Err(TensorError::DivisionByZero {
                message: "Division by zero detected".to_string(),
            });
        }

        let target_shape = Self::broadcast_shapes(&a.dims(), &b.dims())?;
        let a_broadcast = Self::broadcast_array(a_arr, &target_shape)?;
        let b_broadcast = Self::broadcast_array(b_arr, &target_shape)?;

        Ok(Tensor::from_array(a_broadcast / b_broadcast))
    }

    fn add_scalar(&self, tensor: &Tensor, value: f64) -> TensorResult<Tensor> {
        let arr = tensor
            .as_array()
            .ok_or_else(|| TensorError::other("Cannot get array from tensor"))?;
        Ok(Tensor::from_array(arr.clone() + value))
    }

    fn mul_scalar(&self, tensor: &Tensor, value: f64) -> TensorResult<Tensor> {
        let arr = tensor
            .as_array()
            .ok_or_else(|| TensorError::other("Cannot get array from tensor"))?;
        Ok(Tensor::from_array(arr.clone() * value))
    }

    fn matmul(&self, a: &Tensor, b: &Tensor) -> TensorResult<Tensor> {
        let a_arr = a
            .as_array()
            .ok_or_else(|| TensorError::other("Cannot get array from tensor a"))?;
        let b_arr = b
            .as_array()
            .ok_or_else(|| TensorError::other("Cannot get array from tensor b"))?;

        if a_arr.ndim() != 2 || b_arr.ndim() != 2 {
            return Err(TensorError::InvalidDimension {
                dim: a_arr.ndim() as i32,
                message: "matmul requires 2D tensors".to_string(),
            });
        }

        let shape_a = a_arr.shape();
        let shape_b = b_arr.shape();
        let (m, k) = (shape_a[0], shape_a[1]);
        let (k2, n) = (shape_b[0], shape_b[1]);

        if k != k2 {
            return Err(TensorError::ShapeMismatch {
                message: format!("matmul shape mismatch: ({}, {}) x ({}, {})", m, k, k2, n),
            });
        }

        // 使用 ndarray 的 dot 方法（性能优化）
        let a_view = a_arr.view().into_dimensionality::<ndarray::Ix2>().unwrap();
        let b_view = b_arr.view().into_dimensionality::<ndarray::Ix2>().unwrap();

        let result = a_view.dot(&b_view);
        Ok(Tensor::from_array(result.into_dyn()))
    }

    fn transpose(&self, tensor: &Tensor) -> TensorResult<Tensor> {
        tensor.transpose()
    }

    fn reshape(&self, tensor: &Tensor, shape: &[usize]) -> TensorResult<Tensor> {
        tensor.reshape(shape)
    }

    fn sum(&self, tensor: &Tensor, dims: &[usize]) -> TensorResult<Tensor> {
        let arr = tensor
            .as_array()
            .ok_or_else(|| TensorError::other("Cannot get array from tensor"))?;

        if dims.is_empty() {
            // 对所有元素求和
            let sum: f64 = arr.iter().sum();
            return Ok(Tensor::from_array(ArrayD::from_elem(vec![], sum)));
        }

        let mut result = arr.clone();
        // 按维度降序求和，避免维度索引变化
        let mut sorted_dims: Vec<_> = dims.iter().copied().collect();
        sorted_dims.sort_by(|a, b| b.cmp(a));

        for dim in sorted_dims {
            result = result.sum_axis(ndarray::Axis(dim));
        }

        Ok(Tensor::from_array(result))
    }

    fn mean(&self, tensor: &Tensor, dims: &[usize]) -> TensorResult<Tensor> {
        let arr = tensor
            .as_array()
            .ok_or_else(|| TensorError::other("Cannot get array from tensor"))?;

        // 计算参与平均的元素数量
        let numel: usize = if dims.is_empty() {
            arr.len()
        } else {
            dims.iter().map(|&d| arr.shape()[d]).product()
        };

        let sum = self.sum(tensor, dims)?;
        let sum_arr = sum
            .as_array()
            .ok_or_else(|| TensorError::other("Cannot get sum array"))?;

        Ok(Tensor::from_array(sum_arr.clone() / (numel as f64)))
    }

    fn max(&self, tensor: &Tensor, dims: &[usize]) -> TensorResult<Tensor> {
        let arr = tensor
            .as_array()
            .ok_or_else(|| TensorError::other("Cannot get array from tensor"))?;

        if dims.is_empty() {
            let max = arr.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            return Ok(Tensor::from_array(ArrayD::from_elem(vec![], max)));
        }

        let mut result = arr.clone();
        let mut sorted_dims: Vec<_> = dims.iter().copied().collect();
        sorted_dims.sort_by(|a, b| b.cmp(a));

        for dim in sorted_dims {
            result = result.map_axis(ndarray::Axis(dim), |axis| {
                axis.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
            });
        }

        Ok(Tensor::from_array(result))
    }

    fn min(&self, tensor: &Tensor, dims: &[usize]) -> TensorResult<Tensor> {
        let arr = tensor
            .as_array()
            .ok_or_else(|| TensorError::other("Cannot get array from tensor"))?;

        if dims.is_empty() {
            let min = arr.iter().cloned().fold(f64::INFINITY, f64::min);
            return Ok(Tensor::from_array(ArrayD::from_elem(vec![], min)));
        }

        let mut result = arr.clone();
        let mut sorted_dims: Vec<_> = dims.iter().copied().collect();
        sorted_dims.sort_by(|a, b| b.cmp(a));

        for dim in sorted_dims {
            result = result.map_axis(ndarray::Axis(dim), |axis| {
                axis.iter().cloned().fold(f64::INFINITY, f64::min)
            });
        }

        Ok(Tensor::from_array(result))
    }

    fn argmax(&self, tensor: &Tensor, dim: usize) -> TensorResult<Tensor> {
        let arr = tensor
            .as_array()
            .ok_or_else(|| TensorError::other("Cannot get array from tensor"))?;

        if dim >= arr.ndim() {
            return Err(TensorError::InvalidDimension {
                dim: dim as i32,
                message: format!(
                    "dim {} out of range for tensor with {} dimensions",
                    dim,
                    arr.ndim()
                ),
            });
        }

        let result = arr.map_axis(ndarray::Axis(dim), |axis| {
            let (max_idx, _) = axis
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .unwrap();
            max_idx as f64
        });

        Ok(Tensor::from_array(result))
    }

    fn slice(&self, tensor: &Tensor, ranges: &[(usize, usize)]) -> TensorResult<Tensor> {
        let arr = tensor
            .as_array()
            .ok_or_else(|| TensorError::other("Cannot get array from tensor"))?;

        if ranges.len() > arr.ndim() {
            return Err(TensorError::InvalidDimension {
                dim: ranges.len() as i32,
                message: format!(
                    "Too many slice ranges for tensor with {} dimensions",
                    arr.ndim()
                ),
            });
        }

        let mut result = arr.clone();
        for (dim, (start, end)) in ranges.iter().enumerate() {
            if *end > result.shape()[dim] {
                return Err(TensorError::IndexOutOfBounds {
                    message: format!(
                        "Slice end {} exceeds dimension {} size {}",
                        end,
                        dim,
                        result.shape()[dim]
                    ),
                });
            }

            let ndim = result.ndim();
            let indices: Vec<_> = (0..ndim)
                .map(|d| {
                    if d == dim {
                        ndarray::Slice::from(*start..*end)
                    } else {
                        ndarray::Slice::new(0, None, 1)
                    }
                })
                .collect();

            result = result
                .slice_each_axis(|ax| indices[ax.axis.index()])
                .to_owned();
        }

        Ok(Tensor::from_array(result))
    }

    fn cat(&self, tensors: &[&Tensor], dim: usize) -> TensorResult<Tensor> {
        if tensors.is_empty() {
            return Err(TensorError::ShapeMismatch {
                message: "Cannot concatenate empty tensor list".to_string(),
            });
        }

        let arrays: Result<Vec<_>, _> = tensors
            .iter()
            .map(|t| {
                t.as_array()
                    .ok_or_else(|| TensorError::other("Cannot get array from tensor"))
            })
            .collect();
        let arrays = arrays?;

        // 检查维度一致性
        let ndim = arrays[0].ndim();
        for (i, arr) in arrays.iter().enumerate() {
            if arr.ndim() != ndim {
                return Err(TensorError::ShapeMismatch {
                    message: format!("Tensor {} has rank {} but expected {}", i, arr.ndim(), ndim),
                });
            }
        }

        // 转换为 ArrayView 切片
        let views: Vec<_> = arrays.iter().map(|a| a.view()).collect();

        let result = ndarray::concatenate(ndarray::Axis(dim), &views)
            .map_err(|e| TensorError::ShapeMismatch {
                message: format!("Concatenation failed: {}", e),
            })?
            .to_owned();

        Ok(Tensor::from_array(result))
    }

    fn stack(&self, tensors: &[&Tensor], dim: usize) -> TensorResult<Tensor> {
        if tensors.is_empty() {
            return Err(TensorError::ShapeMismatch {
                message: "Cannot stack empty tensor list".to_string(),
            });
        }

        let arrays: Result<Vec<_>, _> = tensors
            .iter()
            .map(|t| {
                t.as_array()
                    .ok_or_else(|| TensorError::other("Cannot get array from tensor"))
            })
            .collect();
        let arrays = arrays?;

        // 检查形状一致性
        let first_shape = arrays[0].shape();
        for (i, arr) in arrays.iter().enumerate() {
            if arr.shape() != first_shape {
                return Err(TensorError::ShapeMismatch {
                    message: format!(
                        "Tensor {} has shape {:?} but expected {:?}",
                        i,
                        arr.shape(),
                        first_shape
                    ),
                });
            }
        }

        let views: Vec<_> = arrays.iter().map(|a| a.view()).collect();

        let result = ndarray::stack(ndarray::Axis(dim), &views)
            .map_err(|e| TensorError::ShapeMismatch {
                message: format!("Stacking failed: {}", e),
            })?
            .to_owned();

        Ok(Tensor::from_array(result))
    }

    fn broadcast(&self, tensor: &Tensor, shape: &[usize]) -> TensorResult<Tensor> {
        let arr = tensor
            .as_array()
            .ok_or_else(|| TensorError::other("Cannot get array from tensor"))?;
        let broadcasted = Self::broadcast_array(arr, shape)?;
        Ok(Tensor::from_array(broadcasted))
    }

    fn unsqueeze(&self, tensor: &Tensor, dim: usize) -> TensorResult<Tensor> {
        let arr = tensor
            .as_array()
            .ok_or_else(|| TensorError::other("Cannot get array from tensor"))?;

        if dim > arr.ndim() {
            return Err(TensorError::InvalidDimension {
                dim: dim as i32,
                message: format!(
                    "dim {} out of range for tensor with {} dimensions",
                    dim,
                    arr.ndim()
                ),
            });
        }

        let mut new_shape = arr.shape().to_vec();
        new_shape.insert(dim, 1);

        let result = arr.clone().into_shape_with_order(new_shape).map_err(|e| {
            TensorError::ShapeMismatch {
                message: format!("Failed to unsqueeze: {}", e),
            }
        })?;

        Ok(Tensor::from_array(result))
    }

    fn squeeze(&self, tensor: &Tensor, dim: Option<usize>) -> TensorResult<Tensor> {
        let arr = tensor
            .as_array()
            .ok_or_else(|| TensorError::other("Cannot get array from tensor"))?;

        let mut new_shape = Vec::new();
        for (i, &d) in arr.shape().iter().enumerate() {
            if let Some(dim_idx) = dim {
                if i != dim_idx || d != 1 {
                    new_shape.push(d);
                }
            } else if d != 1 {
                new_shape.push(d);
            }
        }

        let result = arr.clone().into_shape_with_order(new_shape).map_err(|e| {
            TensorError::ShapeMismatch {
                message: format!("Failed to squeeze: {}", e),
            }
        })?;

        Ok(Tensor::from_array(result))
    }
}

// ========== 测试 ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zeros() {
        let backend = NdArrayBackend::new();
        let tensor = backend.zeros(&[2, 3]).unwrap();
        assert_eq!(tensor.dims(), &[2, 3]);
        assert!(tensor.as_slice().unwrap().iter().all(|&x| x == 0.0));
    }

    #[test]
    fn test_add() {
        let backend = NdArrayBackend::new();
        let a = backend.from_data(&[1.0, 2.0, 3.0], &[3]).unwrap();
        let b = backend.from_data(&[4.0, 5.0, 6.0], &[3]).unwrap();
        let result = backend.add(&a, &b).unwrap();
        assert_eq!(result.as_slice().unwrap(), &[5.0, 7.0, 9.0]);
    }

    #[test]
    fn test_matmul() {
        let backend = NdArrayBackend::new();
        let a = backend
            .from_data(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[2, 3])
            .unwrap();
        let b = backend
            .from_data(&[7.0, 8.0, 9.0, 10.0, 11.0, 12.0], &[3, 2])
            .unwrap();
        let result = backend.matmul(&a, &b).unwrap();
        assert_eq!(result.dims(), &[2, 2]);
        assert_eq!(result.as_slice().unwrap(), &[58.0, 64.0, 139.0, 154.0]);
    }

    #[test]
    fn test_broadcast_add() {
        let backend = NdArrayBackend::new();
        let a = backend.from_data(&[1.0, 2.0, 3.0], &[3, 1]).unwrap();
        let b = backend.from_data(&[1.0, 2.0, 3.0, 4.0], &[4]).unwrap();
        let result = backend.add(&a, &b).unwrap();
        assert_eq!(result.dims(), &[3, 4]);
    }

    #[test]
    fn test_sum() {
        let backend = NdArrayBackend::new();
        let tensor = backend.from_data(&[1.0, 2.0, 3.0, 4.0], &[2, 2]).unwrap();
        let result = backend.sum(&tensor, &[0]).unwrap();
        assert_eq!(result.as_slice().unwrap(), &[4.0, 6.0]);
    }

    #[test]
    fn test_transpose() {
        let backend = NdArrayBackend::new();
        let tensor = backend.from_data(&[1.0, 2.0, 3.0, 4.0], &[2, 2]).unwrap();
        let result = backend.transpose(&tensor).unwrap();
        assert_eq!(result.as_slice().unwrap(), &[1.0, 3.0, 2.0, 4.0]);
    }
}
