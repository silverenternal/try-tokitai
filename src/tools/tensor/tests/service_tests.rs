//! TensorService API 测试
//!
//! 测试 TensorService 的公共 API，包括：
//! - 创建操作
//! - 算术运算
//! - 矩阵运算
//! - 归一化操作
//! - 错误处理

use ai_assistant::tools::tensor::TensorService;

#[test]
fn test_service_creation_ops() -> anyhow::Result<()> {
    let service = TensorService::new();

    // zeros
    let zeros = service.zeros(&[2, 3])?;
    assert_eq!(zeros.dims(), &[2, 3]);
    assert!(zeros.as_slice().unwrap().iter().all(|&x| x == 0.0));

    // ones
    let ones = service.ones(&[2, 3])?;
    assert!(ones.as_slice().unwrap().iter().all(|&x| x == 1.0));

    // randn
    let randn = service.randn(&[2, 3])?;
    assert_eq!(randn.dims(), &[2, 3]);

    // from_data
    let data = service.from_data(&[1.0, 2.0, 3.0, 4.0], &[2, 2])?;
    assert_eq!(data.as_slice().unwrap(), &[1.0, 2.0, 3.0, 4.0]);

    Ok(())
}

#[test]
fn test_service_arithmetic_ops() -> anyhow::Result<()> {
    let service = TensorService::new();

    let a = service.from_data(&[1.0, 2.0, 3.0], &[3])?;
    let b = service.from_data(&[4.0, 5.0, 6.0], &[3])?;

    // add
    let sum = service.add(&a, &b)?;
    assert_eq!(sum.as_slice().unwrap(), &[5.0, 7.0, 9.0]);

    // sub
    let diff = service.sub(&a, &b)?;
    assert_eq!(diff.as_slice().unwrap(), &[-3.0, -3.0, -3.0]);

    // mul
    let prod = service.mul(&a, &b)?;
    assert_eq!(prod.as_slice().unwrap(), &[4.0, 10.0, 18.0]);

    // div
    let quot = service.div(&b, &a)?;
    assert!(quot.as_slice().unwrap()[0].is_finite());

    // mul_scalar
    let scaled = service.mul_scalar(&a, 2.0)?;
    assert_eq!(scaled.as_slice().unwrap(), &[2.0, 4.0, 6.0]);

    Ok(())
}

#[test]
fn test_service_matmul() -> anyhow::Result<()> {
    let service = TensorService::new();

    let a = service.from_data(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[2, 3])?;
    let b = service.from_data(&[7.0, 8.0, 9.0, 10.0, 11.0, 12.0], &[3, 2])?;

    let result = service.matmul(&a, &b)?;
    assert_eq!(result.dims(), &[2, 2]);
    assert_eq!(result.as_slice().unwrap(), &[58.0, 64.0, 139.0, 154.0]);

    Ok(())
}

#[test]
fn test_service_reduction_ops() -> anyhow::Result<()> {
    let service = TensorService::new();

    let tensor = service.from_data(&[1.0, 2.0, 3.0, 4.0], &[2, 2])?;

    // sum
    let sum = service.sum(&tensor, &[0])?;
    assert_eq!(sum.as_slice().unwrap(), &[4.0, 6.0]);

    // mean
    let mean = service.mean(&tensor, &[0])?;
    assert_eq!(mean.as_slice().unwrap(), &[2.0, 3.0]);

    // max
    let max = service.max(&tensor, &[0])?;
    assert_eq!(max.as_slice().unwrap(), &[3.0, 4.0]);

    // min
    let min = service.min(&tensor, &[0])?;
    assert_eq!(min.as_slice().unwrap(), &[1.0, 2.0]);

    Ok(())
}

#[test]
fn test_service_activation_ops() -> anyhow::Result<()> {
    let service = TensorService::new();

    let input = service.from_data(&[-2.0, -1.0, 0.0, 1.0, 2.0], &[5])?;

    // relu
    let relu = service.relu(&input)?;
    assert_eq!(relu.as_slice().unwrap(), &[0.0, 0.0, 0.0, 1.0, 2.0]);

    // sigmoid
    let sigmoid = service.sigmoid(&input)?;
    let sig_slice = sigmoid.as_slice().unwrap();
    assert!(sig_slice[2] > 0.49 && sig_slice[2] < 0.51); // sigmoid(0) ≈ 0.5

    Ok(())
}

#[test]
fn test_service_chain_operations() -> anyhow::Result<()> {
    let service = TensorService::new();

    // 链式调用：zeros -> add_scalar -> mul_scalar
    let zeros = service.zeros(&[2, 2])?;
    let added = service.add_scalar(&zeros, 1.0)?;
    let multiplied = service.mul_scalar(&added, 2.0)?;

    assert_eq!(multiplied.as_slice().unwrap(), &[2.0, 2.0, 2.0, 2.0]);

    Ok(())
}

// ========== 错误处理测试 ==========

#[test]
fn test_matmul_shape_mismatch() -> anyhow::Result<()> {
    let service = TensorService::new();

    let a = service.from_data(&[1.0, 2.0, 3.0], &[3])?;
    let b = service.from_data(&[1.0, 2.0, 3.0], &[3])?;

    // 1D 张量不能 matmul
    let result = service.matmul(&a, &b);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_reshape_element_mismatch() -> anyhow::Result<()> {
    let service = TensorService::new();

    let tensor = service.from_data(&[1.0, 2.0, 3.0, 4.0], &[2, 2])?;

    // 元素数量不匹配
    let result = service.reshape(&tensor, &[3, 3]);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_division_by_zero() -> anyhow::Result<()> {
    let service = TensorService::new();

    let a = service.from_data(&[1.0, 2.0, 3.0], &[3])?;
    let b = service.from_data(&[1.0, 0.0, 2.0], &[3])?;

    let result = service.div(&a, &b);
    assert!(result.is_err());

    Ok(())
}

// ========== 边界条件测试 ==========

#[test]
fn test_empty_tensor() -> anyhow::Result<()> {
    let service = TensorService::new();

    let empty = service.zeros(&[0, 0])?;
    assert_eq!(empty.dims(), &[0, 0]);
    assert_eq!(empty.as_slice().unwrap().len(), 0);

    Ok(())
}

#[test]
fn test_single_element() -> anyhow::Result<()> {
    let service = TensorService::new();

    let single = service.from_data(&[42.0], &[1])?;
    assert_eq!(single.dims(), &[1]);
    assert_eq!(single.as_slice().unwrap(), &[42.0]);

    Ok(())
}

#[test]
fn test_large_tensor() -> anyhow::Result<()> {
    let service = TensorService::new();

    let data: Vec<f64> = (0..10000).map(|i| i as f64).collect();
    let tensor = service.from_data(&data, &[100, 100])?;
    
    assert_eq!(tensor.dims(), &[100, 100]);
    assert_eq!(tensor.as_slice().unwrap().len(), 10000);

    Ok(())
}

#[test]
fn test_reshape_identity() -> anyhow::Result<()> {
    let service = TensorService::new();

    let original = service.from_data(&[1.0, 2.0, 3.0, 4.0], &[2, 2])?;
    let reshaped = service.reshape(&original, &[2, 2])?;

    assert_eq!(original.as_slice().unwrap(), reshaped.as_slice().unwrap());

    Ok(())
}

#[test]
fn test_transpose_2d() -> anyhow::Result<()> {
    let service = TensorService::new();

    let tensor = service.from_data(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[2, 3])?;
    let transposed = service.transpose(&tensor)?;

    assert_eq!(transposed.dims(), &[3, 2]);
    assert_eq!(transposed.as_slice().unwrap(), &[1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);

    Ok(())
}
