//! 操作元数据模块
//!
//! 设计原则:
//! 1. AI 可理解：每个操作都有语义化描述
//! 2. 支持工具发现：AI 可以通过元数据找到合适的操作
//! 3. 结构化文档：操作描述、参数说明、示例

use serde::{Deserialize, Serialize};

/// 操作类别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationCategory {
    /// 创建操作
    Creation,
    /// 算术操作
    Arithmetic,
    /// 矩阵操作
    Matrix,
    /// 归约操作
    Reduction,
    /// 索引与切片
    Index,
    /// 广播与变形
    Broadcast,
    /// 神经网络层
    NeuralNetwork,
    /// 激活函数
    Activation,
}

impl std::fmt::Display for OperationCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationCategory::Creation => write!(f, "creation"),
            OperationCategory::Arithmetic => write!(f, "arithmetic"),
            OperationCategory::Matrix => write!(f, "matrix"),
            OperationCategory::Reduction => write!(f, "reduction"),
            OperationCategory::Index => write!(f, "index"),
            OperationCategory::Broadcast => write!(f, "broadcast"),
            OperationCategory::NeuralNetwork => write!(f, "neural_network"),
            OperationCategory::Activation => write!(f, "activation"),
        }
    }
}

/// 操作元数据
///
/// 这是 AI 理解张量操作的关键，提供：
/// - 操作的语义描述
/// - 参数的详细说明
/// - 使用示例
/// - 常见错误和修复建议
#[derive(Debug, Clone, Serialize)]
pub struct OperationMetadata {
    /// 操作名称（与工具名对应）
    pub name: &'static str,
    /// 操作的简短描述
    pub description: &'static str,
    /// 操作类别
    pub category: OperationCategory,
    /// 详细文档
    pub documentation: &'static str,
    /// 参数说明
    #[serde(skip)]
    pub parameters: &'static [ParameterMetadata],
    /// 返回值说明
    pub returns: &'static str,
    /// 使用示例
    #[serde(skip)]
    pub examples: &'static [Example],
    /// 常见错误
    #[serde(skip)]
    pub common_errors: &'static [CommonError],
    /// 相关操作
    #[serde(skip)]
    pub related_operations: &'static [&'static str],
    /// 是否支持原地操作
    pub is_inplace: bool,
    /// 是否支持广播
    pub supports_broadcasting: bool,
    /// 计算复杂度（大 O 表示）
    pub complexity: Option<&'static str>,
}

/// 参数元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterMetadata {
    /// 参数名称
    pub name: &'static str,
    /// 参数类型描述
    pub param_type: &'static str,
    /// 参数描述
    pub description: &'static str,
    /// 是否必需
    pub required: bool,
    /// 默认值（如果有）
    pub default: Option<&'static str>,
}

/// 使用示例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example {
    /// 示例描述
    pub description: &'static str,
    /// 示例代码
    pub code: &'static str,
}

/// 常见错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonError {
    /// 错误描述
    pub error: &'static str,
    /// 修复建议
    pub suggestion: &'static str,
}

// ========== 预定义操作元数据 ==========

/// zeros 操作元数据
pub const ZEROS_META: OperationMetadata = OperationMetadata {
    name: "zeros",
    description: "创建一个所有元素为零的张量",
    category: OperationCategory::Creation,
    documentation: "创建一个指定形状的张量，所有元素初始化为 0.0。常用于初始化掩码或占位符。",
    parameters: &[ParameterMetadata {
        name: "shape",
        param_type: "Vec<usize>",
        description: "张量的形状，如 [2, 3] 表示 2 行 3 列",
        required: true,
        default: None,
    }],
    returns: "Result<Tensor> - 创建的零张量",
    examples: &[
        Example {
            description: "创建 2x3 的零张量",
            code: "let tensor = service.zeros(&[2, 3])?;",
        },
        Example {
            description: "创建 1D 零张量",
            code: "let tensor = service.zeros(&[5])?;",
        },
    ],
    common_errors: &[CommonError {
        error: "shape 包含 0 维度",
        suggestion: "确保所有维度都大于 0，或使用 empty shape 创建标量",
    }],
    related_operations: &["ones", "randn", "full"],
    is_inplace: false,
    supports_broadcasting: false,
    complexity: Some("O(n)"),
};

/// ones 操作元数据
pub const ONES_META: OperationMetadata = OperationMetadata {
    name: "ones",
    description: "创建一个所有元素为 1 的张量",
    category: OperationCategory::Creation,
    documentation: "创建一个指定形状的张量，所有元素初始化为 1.0。常用于初始化乘法单位元或掩码。",
    parameters: &[ParameterMetadata {
        name: "shape",
        param_type: "Vec<usize>",
        description: "张量的形状",
        required: true,
        default: None,
    }],
    returns: "Result<Tensor> - 创建的一张量",
    examples: &[Example {
        description: "创建 2x2 的一张量",
        code: "let tensor = service.ones(&[2, 2])?;",
    }],
    common_errors: &[],
    related_operations: &["zeros", "randn", "full"],
    is_inplace: false,
    supports_broadcasting: false,
    complexity: Some("O(n)"),
};

/// add 操作元数据
pub const ADD_META: OperationMetadata = OperationMetadata {
    name: "add",
    description: "逐元素加法运算",
    category: OperationCategory::Arithmetic,
    documentation: "对两个张量进行逐元素相加。支持广播机制，允许不同形状的张量相加。",
    parameters: &[
        ParameterMetadata {
            name: "a",
            param_type: "&Tensor",
            description: "第一个操作数",
            required: true,
            default: None,
        },
        ParameterMetadata {
            name: "b",
            param_type: "&Tensor",
            description: "第二个操作数",
            required: true,
            default: None,
        },
    ],
    returns: "Result<Tensor> - 相加结果",
    examples: &[
        Example {
            description: "两个相同形状的张量相加",
            code: "let result = service.add(&a, &b)?;",
        },
        Example {
            description: "广播加法：张量与标量",
            code: "let result = service.add(&tensor, &scalar)?;",
        },
    ],
    common_errors: &[CommonError {
        error: "形状不匹配且无法广播",
        suggestion: "检查两个张量的形状是否兼容，或使用 reshape 调整形状",
    }],
    related_operations: &["sub", "mul", "div", "add_scalar"],
    is_inplace: false,
    supports_broadcasting: true,
    complexity: Some("O(n)"),
};

/// matmul 操作元数据
pub const MATMUL_META: OperationMetadata = OperationMetadata {
    name: "matmul",
    description: "矩阵乘法运算",
    category: OperationCategory::Matrix,
    documentation: "执行两个矩阵（或批量矩阵）的乘法运算。对于 2D 张量，执行标准矩阵乘法；\
                   对于高维张量，执行批量矩阵乘法。",
    parameters: &[
        ParameterMetadata {
            name: "a",
            param_type: "&Tensor",
            description: "第一个矩阵，形状为 (m, k)",
            required: true,
            default: None,
        },
        ParameterMetadata {
            name: "b",
            param_type: "&Tensor",
            description: "第二个矩阵，形状为 (k, n)",
            required: true,
            default: None,
        },
    ],
    returns: "Result<Tensor> - 矩阵乘积，形状为 (m, n)",
    examples: &[
        Example {
            description: "2x3 乘以 3x2",
            code: "let a = service.from_data(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[2, 3])?;\nlet b = service.from_data(&[7.0, 8.0, 9.0, 10.0, 11.0, 12.0], &[3, 2])?;\nlet result = service.matmul(&a, &b)?; // 形状：[2, 2]",
        },
    ],
    common_errors: &[
        CommonError {
            error: "矩阵维度不匹配：(m, k) x (j, n)，k != j",
            suggestion: "确保第一个矩阵的列数等于第二个矩阵的行数，或转置其中一个矩阵",
        },
    ],
    related_operations: &["transpose", "dot", "bmm"],
    is_inplace: false,
    supports_broadcasting: false,
    complexity: Some("O(m*n*k)"),
};

/// sum 操作元数据
pub const SUM_META: OperationMetadata = OperationMetadata {
    name: "sum",
    description: "沿指定维度求和",
    category: OperationCategory::Reduction,
    documentation: "沿指定的维度对张量元素求和。可以减少张量的维度或保持维度（keepdim）。",
    parameters: &[
        ParameterMetadata {
            name: "tensor",
            param_type: "&Tensor",
            description: "输入张量",
            required: true,
            default: None,
        },
        ParameterMetadata {
            name: "dims",
            param_type: "Vec<usize>",
            description: "要求和的维度列表，空列表表示对所有元素求和",
            required: false,
            default: Some("[]"),
        },
    ],
    returns: "Result<Tensor> - 求和结果",
    examples: &[
        Example {
            description: "对所有元素求和",
            code: "let result = service.sum(&tensor, &[])?;",
        },
        Example {
            description: "沿第 0 维求和",
            code: "let result = service.sum(&tensor, &[0])?;",
        },
    ],
    common_errors: &[CommonError {
        error: "维度索引越界",
        suggestion: "确保所有维度索引在 [0, rank) 范围内",
    }],
    related_operations: &["mean", "max", "min", "prod"],
    is_inplace: false,
    supports_broadcasting: false,
    complexity: Some("O(n)"),
};

/// reshape 操作元数据
pub const RESHAPE_META: OperationMetadata = OperationMetadata {
    name: "reshape",
    description: "改变张量的形状",
    category: OperationCategory::Broadcast,
    documentation: "重新解释张量的形状，不改变底层数据。新形状的元素总数必须与原形状相同。",
    parameters: &[
        ParameterMetadata {
            name: "tensor",
            param_type: "&Tensor",
            description: "输入张量",
            required: true,
            default: None,
        },
        ParameterMetadata {
            name: "shape",
            param_type: "Vec<usize>",
            description: "目标形状",
            required: true,
            default: None,
        },
    ],
    returns: "Result<Tensor> - 重塑后的张量",
    examples: &[
        Example {
            description: "2x2 展平为 4",
            code: "let result = service.reshape(&tensor, &[4])?;",
        },
        Example {
            description: "4 重塑为 2x2",
            code: "let result = service.reshape(&tensor, &[2, 2])?;",
        },
    ],
    common_errors: &[CommonError {
        error: "元素数量不匹配",
        suggestion: "确保新形状的元素总数与原张量相同",
    }],
    related_operations: &["view", "flatten", "squeeze", "unsqueeze"],
    is_inplace: false,
    supports_broadcasting: false,
    complexity: Some("O(1)"),
};

/// transpose 操作元数据
pub const TRANSPOSE_META: OperationMetadata = OperationMetadata {
    name: "transpose",
    description: "转置张量（交换行列）",
    category: OperationCategory::Broadcast,
    documentation: "对于 2D 张量，交换行和列。对于高维张量，可以指定要交换的维度。",
    parameters: &[ParameterMetadata {
        name: "tensor",
        param_type: "&Tensor",
        description: "输入张量",
        required: true,
        default: None,
    }],
    returns: "Result<Tensor> - 转置后的张量",
    examples: &[Example {
        description: "2x3 转置为 3x2",
        code: "let result = service.transpose(&tensor)?;",
    }],
    common_errors: &[CommonError {
        error: "仅支持 2D 张量",
        suggestion: "对于高维张量，使用 permute 指定维度排列",
    }],
    related_operations: &["permute", "swap_axes", "t"],
    is_inplace: false,
    supports_broadcasting: false,
    complexity: Some("O(n)"),
};

/// relu 操作元数据
pub const RELU_META: OperationMetadata = OperationMetadata {
    name: "relu",
    description: "ReLU 激活函数：max(0, x)",
    category: OperationCategory::Activation,
    documentation: "应用修正线性单元激活函数。将所有负值设为 0，正值保持不变。\
                   这是深度学习中最常用的激活函数。",
    parameters: &[
        ParameterMetadata {
            name: "input",
            param_type: "&Tensor",
            description: "输入张量",
            required: true,
            default: None,
        },
    ],
    returns: "Result<Tensor> - 激活后的张量",
    examples: &[
        Example {
            description: "应用 ReLU",
            code: "let input = service.from_data(&[-1.0, 0.0, 1.0, 2.0], &[4])?;\nlet output = service.relu(&input)?; // [0.0, 0.0, 1.0, 2.0]",
        },
    ],
    common_errors: &[],
    related_operations: &["gelu", "sigmoid", "tanh", "leaky_relu"],
    is_inplace: false,
    supports_broadcasting: false,
    complexity: Some("O(n)"),
};

/// linear 操作元数据
pub const LINEAR_META: OperationMetadata = OperationMetadata {
    name: "linear",
    description: "全连接层（线性变换）：y = xW^T + b",
    category: OperationCategory::NeuralNetwork,
    documentation: "应用线性变换：输出 = 输入 × 权重转置 + 偏置。\
                   这是神经网络中最基础的层类型。",
    parameters: &[
        ParameterMetadata {
            name: "input",
            param_type: "&Tensor",
            description: "输入张量，形状为 (batch_size, in_features)",
            required: true,
            default: None,
        },
        ParameterMetadata {
            name: "weight",
            param_type: "&Tensor",
            description: "权重矩阵，形状为 (out_features, in_features)",
            required: true,
            default: None,
        },
        ParameterMetadata {
            name: "bias",
            param_type: "Option<&Tensor>",
            description: "偏置向量，形状为 (out_features)",
            required: false,
            default: Some("None"),
        },
    ],
    returns: "Result<Tensor> - 线性变换结果，形状为 (batch_size, out_features)",
    examples: &[
        Example {
            description: "全连接层前向传播",
            code: "let input = service.from_data(&[1.0, 2.0, 3.0, 4.0], &[2, 2])?;\nlet weight = service.randn(&[3, 2])?;\nlet output = service.linear(&input, &weight, None)?;",
        },
    ],
    common_errors: &[
        CommonError {
            error: "输入特征数与权重不匹配",
            suggestion: "确保输入的最后一个维度等于权重的第二个维度",
        },
    ],
    related_operations: &["matmul", "add", "conv2d"],
    is_inplace: false,
    supports_broadcasting: false,
    complexity: Some("O(batch * in * out)"),
};

/// 获取所有预定义操作的元数据
pub fn get_all_operation_metadata() -> &'static [OperationMetadata] {
    &[
        ZEROS_META,
        ONES_META,
        ADD_META,
        MATMUL_META,
        SUM_META,
        RESHAPE_META,
        TRANSPOSE_META,
        RELU_META,
        LINEAR_META,
    ]
}

/// 根据名称查找操作元数据
pub fn get_operation_metadata(name: &str) -> Option<&'static OperationMetadata> {
    get_all_operation_metadata().iter().find(|m| m.name == name)
}

// ========== 测试 ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_access() {
        assert_eq!(ZEROS_META.name, "zeros");
        assert_eq!(ZEROS_META.category, OperationCategory::Creation);
    }

    #[test]
    fn test_get_metadata() {
        let meta = get_operation_metadata("matmul");
        assert!(meta.is_some());
        assert_eq!(meta.unwrap().category, OperationCategory::Matrix);

        let meta = get_operation_metadata("nonexistent");
        assert!(meta.is_none());
    }

    #[test]
    fn test_serialize_metadata() {
        let json = serde_json::to_string_pretty(&ZEROS_META).unwrap();
        assert!(json.contains("zeros"));
        assert!(json.contains("creation"));
    }
}
