//! Tensor 工具集 - tokitai 集成
//!
//! 设计原则:
//! 1. 使用 tokitai #[tool] 宏注册
//! 2. AI 可理解的参数和返回值
//! 3. 完整的文档注释

use tokitai::{tool, tool_desc};
use serde_json::{Value, json};
use crate::tools::tensor::core::Tensor;
use crate::tools::tensor::service::TensorService;

/// Tensor 计算工具集
///
/// 提供张量创建、算术运算、矩阵运算、归约操作等功能
/// 所有操作都支持链式调用，返回 Tensor 对象
#[tool]
pub struct TensorTools {
    service: TensorService,
}

impl TensorTools {
    /// 创建新的 Tensor 工具集
    pub fn new() -> Self {
        Self {
            service: TensorService::new(),
        }
    }

    /// 创建零张量
    ///
    /// 创建一个所有元素为零的张量，常用于初始化掩码或占位符
    #[tool_desc("shape - 张量的形状，如 [2, 3] 表示 2 行 3 列")]
    pub fn zeros(&self, shape: Vec<usize>) -> Result<Value, tokitai::ToolError> {
        let tensor = self.service.zeros(&shape)
            .map_err(|e| tokitai::ToolError::validation_error(format!("Failed to create zeros tensor: {}", e)))?;
        
        Ok(json!({
            "shape": tensor.dims(),
            "dtype": "f64",
            "device": "cpu",
            "data": tensor.as_slice().unwrap_or(&[]),
        }))
    }

    /// 创建一张量
    ///
    /// 创建一个所有元素为 1 的张量，常用于初始化乘法单位元
    #[tool_desc("shape - 张量的形状")]
    pub fn ones(&self, shape: Vec<usize>) -> Result<Value, tokitai::ToolError> {
        let tensor = self.service.ones(&shape)
            .map_err(|e| tokitai::ToolError::validation_error(format!("Failed to create ones tensor: {}", e)))?;
        
        Ok(json!({
            "shape": tensor.dims(),
            "dtype": "f64",
            "device": "cpu",
            "data": tensor.as_slice().unwrap_or(&[]),
        }))
    }

    /// 创建随机张量
    ///
    /// 创建服从标准正态分布（均值 0，方差 1）的随机张量，常用于权重初始化
    #[tool_desc("shape - 张量的形状")]
    pub fn randn(&self, shape: Vec<usize>) -> Result<Value, tokitai::ToolError> {
        let tensor = self.service.randn(&shape)
            .map_err(|e| tokitai::ToolError::validation_error(format!("Failed to create randn tensor: {}", e)))?;
        
        Ok(json!({
            "shape": tensor.dims(),
            "dtype": "f64",
            "device": "cpu",
            "data": tensor.as_slice().unwrap_or(&[]),
        }))
    }

    /// 从数据创建张量
    ///
    /// 从给定的 f64 数据创建张量，需要指定形状
    #[tool_desc("data - 数据列表")]
    #[tool_desc("shape - 张量的形状，元素数量必须与 data 长度匹配")]
    pub fn from_data(&self, data: Vec<f64>, shape: Vec<usize>) -> Result<Value, tokitai::ToolError> {
        let tensor = self.service.from_data(&data, &shape)
            .map_err(|e| tokitai::ToolError::validation_error(format!("Failed to create tensor from data: {}", e)))?;
        
        Ok(json!({
            "shape": tensor.dims(),
            "dtype": "f64",
            "device": "cpu",
            "data": tensor.as_slice().unwrap_or(&[]),
        }))
    }

    /// 张量加法
    ///
    /// 逐元素相加，支持广播机制
    #[tool_desc("a - 第一个张量（JSON 格式）")]
    #[tool_desc("b - 第二个张量（JSON 格式）")]
    pub fn add(&self, a: Value, b: Value) -> Result<Value, tokitai::ToolError> {
        let a_tensor = self._value_to_tensor(&a)?;
        let b_tensor = self._value_to_tensor(&b)?;
        
        let result = self.service.add(&a_tensor, &b_tensor)
            .map_err(|e| tokitai::ToolError::validation_error(format!("Addition failed: {}", e)))?;
        
        Ok(json!({
            "shape": result.dims(),
            "dtype": "f64",
            "device": "cpu",
            "data": result.as_slice().unwrap_or(&[]),
        }))
    }

    /// 张量减法
    ///
    /// 逐元素相减，支持广播机制
    #[tool_desc("a - 被减数张量")]
    #[tool_desc("b - 减数张量")]
    pub fn sub(&self, a: Value, b: Value) -> Result<Value, tokitai::ToolError> {
        let a_tensor = self._value_to_tensor(&a)?;
        let b_tensor = self._value_to_tensor(&b)?;
        
        let result = self.service.sub(&a_tensor, &b_tensor)
            .map_err(|e| tokitai::ToolError::validation_error(format!("Subtraction failed: {}", e)))?;
        
        Ok(json!({
            "shape": result.dims(),
            "dtype": "f64",
            "device": "cpu",
            "data": result.as_slice().unwrap_or(&[]),
        }))
    }

    /// 张量乘法
    ///
    /// 逐元素相乘，支持广播机制
    #[tool_desc("a - 第一个张量")]
    #[tool_desc("b - 第二个张量")]
    pub fn mul(&self, a: Value, b: Value) -> Result<Value, tokitai::ToolError> {
        let a_tensor = self._value_to_tensor(&a)?;
        let b_tensor = self._value_to_tensor(&b)?;
        
        let result = self.service.mul(&a_tensor, &b_tensor)
            .map_err(|e| tokitai::ToolError::validation_error(format!("Multiplication failed: {}", e)))?;
        
        Ok(json!({
            "shape": result.dims(),
            "dtype": "f64",
            "device": "cpu",
            "data": result.as_slice().unwrap_or(&[]),
        }))
    }

    /// 张量除法
    ///
    /// 逐元素相除，支持广播机制。注意：除数不能为零
    #[tool_desc("a - 被除数张量")]
    #[tool_desc("b - 除数张量")]
    pub fn div(&self, a: Value, b: Value) -> Result<Value, tokitai::ToolError> {
        let a_tensor = self._value_to_tensor(&a)?;
        let b_tensor = self._value_to_tensor(&b)?;
        
        let result = self.service.div(&a_tensor, &b_tensor)
            .map_err(|e| tokitai::ToolError::validation_error(format!("Division failed: {}", e)))?;
        
        Ok(json!({
            "shape": result.dims(),
            "dtype": "f64",
            "device": "cpu",
            "data": result.as_slice().unwrap_or(&[]),
        }))
    }

    /// 标量乘法
    ///
    /// 张量乘以标量值
    #[tool_desc("tensor - 输入张量")]
    #[tool_desc("scalar - 标量值")]
    pub fn mul_scalar(&self, tensor: Value, scalar: f64) -> Result<Value, tokitai::ToolError> {
        let t = self._value_to_tensor(&tensor)?;
        
        let result = self.service.mul_scalar(&t, scalar)
            .map_err(|e| tokitai::ToolError::validation_error(format!("Scalar multiplication failed: {}", e)))?;
        
        Ok(json!({
            "shape": result.dims(),
            "dtype": "f64",
            "device": "cpu",
            "data": result.as_slice().unwrap_or(&[]),
        }))
    }

    /// 矩阵乘法
    ///
    /// 执行两个矩阵的乘法运算。第一个矩阵的列数必须等于第二个矩阵的行数
    #[tool_desc("a - 第一个矩阵，形状为 (m, k)")]
    #[tool_desc("b - 第二个矩阵，形状为 (k, n)")]
    pub fn matmul(&self, a: Value, b: Value) -> Result<Value, tokitai::ToolError> {
        let a_tensor = self._value_to_tensor(&a)?;
        let b_tensor = self._value_to_tensor(&b)?;
        
        let result = self.service.matmul(&a_tensor, &b_tensor)
            .map_err(|e| tokitai::ToolError::validation_error(format!("Matrix multiplication failed: {}", e)))?;
        
        Ok(json!({
            "shape": result.dims(),
            "dtype": "f64",
            "device": "cpu",
            "data": result.as_slice().unwrap_or(&[]),
        }))
    }

    /// 转置
    ///
    /// 交换矩阵的行和列（仅支持 2D 张量）
    #[tool_desc("tensor - 输入张量（2D）")]
    pub fn transpose(&self, tensor: Value) -> Result<Value, tokitai::ToolError> {
        let t = self._value_to_tensor(&tensor)?;
        
        let result = self.service.transpose(&t)
            .map_err(|e| tokitai::ToolError::validation_error(format!("Transpose failed: {}", e)))?;
        
        Ok(json!({
            "shape": result.dims(),
            "dtype": "f64",
            "device": "cpu",
            "data": result.as_slice().unwrap_or(&[]),
        }))
    }

    /// 重塑形状
    ///
    /// 改变张量的形状，不改变底层数据。新形状的元素总数必须与原形状相同
    #[tool_desc("tensor - 输入张量")]
    #[tool_desc("shape - 目标形状")]
    pub fn reshape(&self, tensor: Value, shape: Vec<usize>) -> Result<Value, tokitai::ToolError> {
        let t = self._value_to_tensor(&tensor)?;
        
        let result = self.service.reshape(&t, &shape)
            .map_err(|e| tokitai::ToolError::validation_error(format!("Reshape failed: {}", e)))?;
        
        Ok(json!({
            "shape": result.dims(),
            "dtype": "f64",
            "device": "cpu",
            "data": result.as_slice().unwrap_or(&[]),
        }))
    }

    /// 求和
    ///
    /// 沿指定维度对张量元素求和。空 dims 表示对所有元素求和
    #[tool_desc("tensor - 输入张量")]
    #[tool_desc("dims - 要求和的维度列表，空列表表示对所有元素求和")]
    pub fn sum(&self, tensor: Value, dims: Vec<usize>) -> Result<Value, tokitai::ToolError> {
        let t = self._value_to_tensor(&tensor)?;
        
        let result = self.service.sum(&t, &dims)
            .map_err(|e| tokitai::ToolError::validation_error(format!("Sum failed: {}", e)))?;
        
        Ok(json!({
            "shape": result.dims(),
            "dtype": "f64",
            "device": "cpu",
            "data": result.as_slice().unwrap_or(&[]),
        }))
    }

    /// 平均值
    ///
    /// 沿指定维度计算张量元素的平均值
    #[tool_desc("tensor - 输入张量")]
    #[tool_desc("dims - 求平均的维度列表，空列表表示对所有元素求平均")]
    pub fn mean(&self, tensor: Value, dims: Vec<usize>) -> Result<Value, tokitai::ToolError> {
        let t = self._value_to_tensor(&tensor)?;
        
        let result = self.service.mean(&t, &dims)
            .map_err(|e| tokitai::ToolError::validation_error(format!("Mean failed: {}", e)))?;
        
        Ok(json!({
            "shape": result.dims(),
            "dtype": "f64",
            "device": "cpu",
            "data": result.as_slice().unwrap_or(&[]),
        }))
    }

    /// ReLU 激活函数
    ///
    /// 应用修正线性单元：max(0, x)。将所有负值设为 0，正值保持不变
    #[tool_desc("input - 输入张量")]
    pub fn relu(&self, input: Value) -> Result<Value, tokitai::ToolError> {
        let t = self._value_to_tensor(&input)?;
        
        let result = self.service.relu(&t)
            .map_err(|e| tokitai::ToolError::validation_error(format!("ReLU failed: {}", e)))?;
        
        Ok(json!({
            "shape": result.dims(),
            "dtype": "f64",
            "device": "cpu",
            "data": result.as_slice().unwrap_or(&[]),
        }))
    }

    /// GELU 激活函数
    ///
    /// 应用高斯误差线性单元激活函数，常用于 Transformer 模型
    #[tool_desc("input - 输入张量")]
    pub fn gelu(&self, input: Value) -> Result<Value, tokitai::ToolError> {
        let t = self._value_to_tensor(&input)?;
        
        let result = self.service.gelu(&t)
            .map_err(|e| tokitai::ToolError::validation_error(format!("GELU failed: {}", e)))?;
        
        Ok(json!({
            "shape": result.dims(),
            "dtype": "f64",
            "device": "cpu",
            "data": result.as_slice().unwrap_or(&[]),
        }))
    }

    /// Sigmoid 激活函数
    ///
    /// 应用 Sigmoid 函数：1 / (1 + exp(-x))。输出范围 (0, 1)
    #[tool_desc("input - 输入张量")]
    pub fn sigmoid(&self, input: Value) -> Result<Value, tokitai::ToolError> {
        let t = self._value_to_tensor(&input)?;
        
        let result = self.service.sigmoid(&t)
            .map_err(|e| tokitai::ToolError::validation_error(format!("Sigmoid failed: {}", e)))?;
        
        Ok(json!({
            "shape": result.dims(),
            "dtype": "f64",
            "device": "cpu",
            "data": result.as_slice().unwrap_or(&[]),
        }))
    }

    /// LayerNorm 层归一化
    ///
    /// 对最后一个维度进行层归一化，常用于 Transformer 模型
    #[tool_desc("input - 输入张量")]
    #[tool_desc("normalized_shape - 归一化的维度大小")]
    #[tool_desc("eps - 数值稳定性常数")]
    pub fn layer_norm(&self, input: Value, normalized_shape: usize, eps: f64) -> Result<Value, tokitai::ToolError> {
        let t = self._value_to_tensor(&input)?;
        
        let result = self.service.layer_norm(&t, normalized_shape, eps)
            .map_err(|e| tokitai::ToolError::validation_error(format!("LayerNorm failed: {}", e)))?;
        
        Ok(json!({
            "shape": result.dims(),
            "dtype": "f64",
            "device": "cpu",
            "data": result.as_slice().unwrap_or(&[]),
        }))
    }

    /// 获取后端名称
    ///
    /// 返回当前使用的后端名称
    pub fn backend_name(&self) -> Result<Value, tokitai::ToolError> {
        Ok(json!({
            "backend": self.service.backend_name()
        }))
    }
}

impl Default for TensorTools {
    fn default() -> Self {
        Self::new()
    }
}

impl TensorTools {
    /// 从 JSON Value 解析 Tensor
    fn _value_to_tensor(&self, value: &Value) -> Result<Tensor, tokitai::ToolError> {
        let obj = value.as_object()
            .ok_or_else(|| tokitai::ToolError::validation_error("Expected tensor JSON object".to_string()))?;
        
        let data = obj.get("data")
            .and_then(|v| v.as_array())
            .ok_or_else(|| tokitai::ToolError::validation_error("Missing tensor data".to_string()))?;
        
        let shape = obj.get("shape")
            .and_then(|v| v.as_array())
            .ok_or_else(|| tokitai::ToolError::validation_error("Missing tensor shape".to_string()))?;
        
        let data_vec: Vec<f64> = data.iter()
            .filter_map(|v| v.as_f64())
            .collect();
        
        let shape_vec: Vec<usize> = shape.iter()
            .filter_map(|v| v.as_u64())
            .map(|v| v as usize)
            .collect();
        
        self.service.from_data(&data_vec, &shape_vec)
            .map_err(|e| tokitai::ToolError::validation_error(format!("Invalid tensor: {}", e)))
    }
}

// ========== 测试 ==========

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_zeros() {
        let tools = TensorTools::new();
        let result = tools.zeros(vec![2, 3]).unwrap();
        
        let obj = result.as_object().unwrap();
        assert_eq!(obj["shape"], json!([2, 3]));
        assert_eq!(obj["data"].as_array().unwrap().len(), 6);
    }

    #[test]
    fn test_from_data() {
        let tools = TensorTools::new();
        let result = tools.from_data(vec![1.0, 2.0, 3.0, 4.0], vec![2, 2]).unwrap();
        
        let obj = result.as_object().unwrap();
        assert_eq!(obj["shape"], json!([2, 2]));
    }

    #[test]
    fn test_matmul() {
        let tools = TensorTools::new();
        
        let a = json!({
            "shape": [2, 3],
            "data": [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
        });
        let b = json!({
            "shape": [3, 2],
            "data": [7.0, 8.0, 9.0, 10.0, 11.0, 12.0]
        });
        
        let result = tools.matmul(a, b).unwrap();
        let obj = result.as_object().unwrap();
        assert_eq!(obj["shape"], json!([2, 2]));
    }

    #[test]
    fn test_relu() {
        let tools = TensorTools::new();
        
        let input = json!({
            "shape": [5],
            "data": [-2.0, -1.0, 0.0, 1.0, 2.0]
        });
        
        let result = tools.relu(input).unwrap();
        let obj = result.as_object().unwrap();
        let data = obj["data"].as_array().unwrap();
        
        assert_eq!(data[0].as_f64().unwrap(), 0.0);
        assert_eq!(data[3].as_f64().unwrap(), 1.0);
    }

    #[test]
    fn test_reshape() {
        let tools = TensorTools::new();
        
        let tensor = json!({
            "shape": [2, 2],
            "data": [1.0, 2.0, 3.0, 4.0]
        });
        
        let result = tools.reshape(tensor, vec![4]).unwrap();
        let obj = result.as_object().unwrap();
        assert_eq!(obj["shape"], json!([4]));
    }
}
