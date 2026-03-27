//! TensorTools 测试
//!
//! 测试 TensorTools 的 JSON 接口，包括：
//! - JSON 输入/输出格式
//! - 参数验证
//! - 工具注册

use ai_assistant::tools::tensor::TensorTools;
use serde_json::json;

#[test]
fn test_tools_zeros() -> anyhow::Result<()> {
    let tools = TensorTools::new();
    let result = tools.zeros(vec![2, 3])?;

    let obj = result.as_object().unwrap();
    assert_eq!(obj["shape"], json!([2, 3]));
    assert_eq!(obj["data"].as_array().unwrap().len(), 6);

    Ok(())
}

#[test]
fn test_tools_from_data() -> anyhow::Result<()> {
    let tools = TensorTools::new();
    let result = tools.from_data(vec![1.0, 2.0, 3.0, 4.0], vec![2, 2])?;

    let obj = result.as_object().unwrap();
    assert_eq!(obj["shape"], json!([2, 2]));
    assert_eq!(obj["data"].as_array().unwrap().len(), 4);

    Ok(())
}

#[test]
fn test_tools_matmul() -> anyhow::Result<()> {
    let tools = TensorTools::new();

    let a = json!({
        "shape": [2, 3],
        "data": [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
    });
    let b = json!({
        "shape": [3, 2],
        "data": [7.0, 8.0, 9.0, 10.0, 11.0, 12.0]
    });

    let result = tools.matmul(a, b)?;
    let obj = result.as_object().unwrap();
    assert_eq!(obj["shape"], json!([2, 2]));

    let data = obj["data"].as_array().unwrap();
    assert_eq!(data[0].as_f64().unwrap(), 58.0);
    assert_eq!(data[1].as_f64().unwrap(), 64.0);

    Ok(())
}

#[test]
fn test_tools_relu() -> anyhow::Result<()> {
    let tools = TensorTools::new();

    let input = json!({
        "shape": [5],
        "data": [-2.0, -1.0, 0.0, 1.0, 2.0]
    });

    let result = tools.relu(input)?;
    let obj = result.as_object().unwrap();
    let data = obj["data"].as_array().unwrap();

    assert_eq!(data[0].as_f64().unwrap(), 0.0);
    assert_eq!(data[3].as_f64().unwrap(), 1.0);
    assert_eq!(data[4].as_f64().unwrap(), 2.0);

    Ok(())
}

#[test]
fn test_tools_reshape() -> anyhow::Result<()> {
    let tools = TensorTools::new();

    let tensor = json!({
        "shape": [2, 2],
        "data": [1.0, 2.0, 3.0, 4.0]
    });

    let result = tools.reshape(tensor, vec![4])?;
    let obj = result.as_object().unwrap();
    assert_eq!(obj["shape"], json!([4]));

    Ok(())
}

#[test]
fn test_tools_sum() -> anyhow::Result<()> {
    let tools = TensorTools::new();

    let tensor = json!({
        "shape": [2, 2],
        "data": [1.0, 2.0, 3.0, 4.0]
    });

    // 沿第 0 维求和
    let result = tools.sum(tensor.clone(), Some(vec![0]))?;
    let obj = result.as_object().unwrap();
    let data = obj["data"].as_array().unwrap();
    assert_eq!(data[0].as_f64().unwrap(), 4.0);
    assert_eq!(data[1].as_f64().unwrap(), 6.0);

    // 对所有元素求和
    let result = tools.sum(tensor, None)?;
    let obj = result.as_object().unwrap();
    let data = obj["data"].as_array().unwrap();
    assert_eq!(data[0].as_f64().unwrap(), 10.0);

    Ok(())
}

#[test]
fn test_tools_layer_norm() -> anyhow::Result<()> {
    let tools = TensorTools::new();

    let input = json!({
        "shape": [1, 4],
        "data": [1.0, 2.0, 3.0, 4.0]
    });

    let result = tools.layer_norm(input, 4, Some(1e-5))?;
    let obj = result.as_object().unwrap();

    // 验证形状
    assert_eq!(obj["shape"], json!([1, 4]));

    // 验证归一化后的均值接近 0
    let data = obj["data"].as_array().unwrap();
    let mean: f64 = data.iter().map(|v| v.as_f64().unwrap()).sum::<f64>() / 4.0;
    assert!(mean.abs() < 1e-5);

    Ok(())
}

#[test]
fn test_tools_backend_name() -> anyhow::Result<()> {
    let tools = TensorTools::new();
    let result = tools.backend_name()?;
    assert_eq!(result.as_str().unwrap(), "NdArray");

    Ok(())
}

// ========== 错误处理测试 ==========

#[test]
fn test_tools_invalid_shape() -> anyhow::Result<()> {
    let tools = TensorTools::new();

    // 元素数量不匹配
    let result = tools.from_data(vec![1.0, 2.0], vec![2, 2]);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_tools_matmul_shape_mismatch() -> anyhow::Result<()> {
    let tools = TensorTools::new();

    let a = json!({
        "shape": [2, 3],
        "data": [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
    });
    let b = json!({
        "shape": [2, 3],  // 应该是 [3, 2]
        "data": [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
    });

    let result = tools.matmul(a, b);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_tools_reshape_invalid() -> anyhow::Result<()> {
    let tools = TensorTools::new();

    let tensor = json!({
        "shape": [2, 2],
        "data": [1.0, 2.0, 3.0, 4.0]
    });

    // 元素数量不匹配
    let result = tools.reshape(tensor, vec![3, 3]);
    assert!(result.is_err());

    Ok(())
}

// ========== 边界条件测试 ==========

#[test]
fn test_tools_empty_tensor() -> anyhow::Result<()> {
    let tools = TensorTools::new();
    let result = tools.zeros(vec![0, 0])?;

    let obj = result.as_object().unwrap();
    assert_eq!(obj["shape"], json!([0, 0]));
    assert!(obj["data"].as_array().unwrap().is_empty());

    Ok(())
}

#[test]
fn test_tools_single_element() -> anyhow::Result<()> {
    let tools = TensorTools::new();
    let result = tools.from_data(vec![42.0], vec![1])?;

    let obj = result.as_object().unwrap();
    assert_eq!(obj["shape"], json!([1]));
    assert_eq!(obj["data"].as_array().unwrap().len(), 1);

    Ok(())
}
