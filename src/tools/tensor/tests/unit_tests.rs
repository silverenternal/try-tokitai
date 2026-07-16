//! Tensor 核心数据结构单元测试
//!
//! 测试 Tensor 核心数据结构和基础操作

use ai_assistant::tools::tensor::Tensor;

#[test]
fn test_tensor_zeros() {
    let tensor = Tensor::zeros(&[2, 3]);
    assert_eq!(tensor.dims(), &[2, 3]);
    assert_eq!(tensor.numel(), 6);
    assert!(tensor.as_slice().unwrap().iter().all(|&x| x == 0.0));
}

#[test]
fn test_tensor_ones() {
    let tensor = Tensor::ones(&[2, 3]);
    assert_eq!(tensor.dims(), &[2, 3]);
    assert!(tensor.as_slice().unwrap().iter().all(|&x| x == 1.0));
}

#[test]
fn test_tensor_from_data() {
    let data = vec![1.0, 2.0, 3.0, 4.0];
    let tensor = Tensor::from_data(&data, &[2, 2]);
    
    assert_eq!(tensor.dims(), &[2, 2]);
    assert_eq!(tensor.as_slice().unwrap(), &data);
}

#[test]
fn test_tensor_reshape() {
    let data = vec![1.0, 2.0, 3.0, 4.0];
    let tensor = Tensor::from_data(&data, &[2, 2]);
    
    let reshaped = tensor.reshape(&[4]).unwrap();
    assert_eq!(reshaped.dims(), &[4]);
    assert_eq!(reshaped.as_slice().unwrap(), &data);
}

#[test]
fn test_tensor_transpose() {
    let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let tensor = Tensor::from_data(&data, &[2, 3]);
    
    let transposed = tensor.transpose().unwrap();
    assert_eq!(transposed.dims(), &[3, 2]);
    assert_eq!(transposed.as_slice().unwrap(), &[1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);
}

#[test]
fn test_tensor_arithmetic() {
    let a = Tensor::from_data(&[1.0, 2.0, 3.0], &[3]);
    let b = Tensor::from_data(&[4.0, 5.0, 6.0], &[3]);
    
    let sum = a.add(&b).unwrap();
    assert_eq!(sum.as_slice().unwrap(), &[5.0, 7.0, 9.0]);
    
    let diff = a.sub(&b).unwrap();
    assert_eq!(diff.as_slice().unwrap(), &[-3.0, -3.0, -3.0]);
    
    let prod = a.mul(&b).unwrap();
    assert_eq!(prod.as_slice().unwrap(), &[4.0, 10.0, 18.0]);
}

#[test]
fn test_tensor_scalar_ops() {
    let tensor = Tensor::from_data(&[1.0, 2.0, 3.0], &[3]);
    
    let scaled = tensor.mul_scalar(2.0).unwrap();
    assert_eq!(scaled.as_slice().unwrap(), &[2.0, 4.0, 6.0]);
    
    let added = tensor.add_scalar(1.0).unwrap();
    assert_eq!(added.as_slice().unwrap(), &[2.0, 3.0, 4.0]);
}

#[test]
fn test_tensor_matmul() {
    let a = Tensor::from_data(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[2, 3]);
    let b = Tensor::from_data(&[7.0, 8.0, 9.0, 10.0, 11.0, 12.0], &[3, 2]);
    
    let result = a.matmul(&b).unwrap();
    assert_eq!(result.dims(), &[2, 2]);
    assert_eq!(result.as_slice().unwrap(), &[58.0, 64.0, 139.0, 154.0]);
}

#[test]
fn test_tensor_reduction() {
    let data = vec![1.0, 2.0, 3.0, 4.0];
    let tensor = Tensor::from_data(&data, &[2, 2]);
    
    let sum = tensor.sum(&[0]).unwrap();
    assert_eq!(sum.as_slice().unwrap(), &[4.0, 6.0]);
    
    let mean = tensor.mean(&[0]).unwrap();
    assert_eq!(mean.as_slice().unwrap(), &[2.0, 3.0]);
}

#[test]
fn test_tensor_relu() {
    let data = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
    let tensor = Tensor::from_data(&data, &[5]);
    
    let relu = tensor.relu().unwrap();
    assert_eq!(relu.as_slice().unwrap(), &[0.0, 0.0, 0.0, 1.0, 2.0]);
}

#[test]
fn test_tensor_sigmoid() {
    let tensor = Tensor::from_data(&[0.0], &[1]);
    let sigmoid = tensor.sigmoid().unwrap();
    
    let sig_value = sigmoid.as_slice().unwrap()[0];
    assert!(sig_value > 0.49 && sig_value < 0.51); // sigmoid(0) = 0.5
}

#[test]
fn test_tensor_clone() {
    let tensor = Tensor::from_data(&[1.0, 2.0, 3.0], &[3]);
    let cloned = tensor.clone();
    
    assert_eq!(tensor.dims(), cloned.dims());
    assert_eq!(tensor.as_slice().unwrap(), cloned.as_slice().unwrap());
}

#[test]
fn test_tensor_eq() {
    let a = Tensor::from_data(&[1.0, 2.0, 3.0], &[3]);
    let b = Tensor::from_data(&[1.0, 2.0, 3.0], &[3]);
    let c = Tensor::from_data(&[1.0, 2.0, 4.0], &[3]);
    
    assert_eq!(a, b);
    assert_ne!(a, c);
}

// ========== 错误处理测试 ==========

#[test]
fn test_tensor_reshape_invalid() {
    let data = vec![1.0, 2.0, 3.0, 4.0];
    let tensor = Tensor::from_data(&data, &[2, 2]);
    
    let result = tensor.reshape(&[3, 3]);
    assert!(result.is_err());
}

#[test]
fn test_tensor_matmul_shape_mismatch() {
    let a = Tensor::from_data(&[1.0, 2.0, 3.0], &[3]);
    let b = Tensor::from_data(&[1.0, 2.0, 3.0], &[3]);
    
    let result = a.matmul(&b);
    assert!(result.is_err());
}

#[test]
fn test_tensor_transpose_invalid() {
    let tensor = Tensor::from_data(&[1.0, 2.0, 3.0], &[3]);
    
    // 1D tensor cannot be transposed
    let result = tensor.transpose();
    assert!(result.is_err());
}

// ========== 边界条件测试 ==========

#[test]
fn test_tensor_empty() {
    let tensor = Tensor::zeros(&[0, 0]);
    assert_eq!(tensor.dims(), &[0, 0]);
    assert_eq!(tensor.numel(), 0);
}

#[test]
fn test_tensor_single_element() {
    let tensor = Tensor::from_data(&[42.0], &[1]);
    assert_eq!(tensor.dims(), &[1]);
    assert_eq!(tensor.numel(), 1);
    assert_eq!(tensor.as_slice().unwrap(), &[42.0]);
}

#[test]
fn test_tensor_large() {
    let data: Vec<f64> = (0..10000).map(|i| i as f64).collect();
    let tensor = Tensor::from_data(&data, &[100, 100]);
    
    assert_eq!(tensor.dims(), &[100, 100]);
    assert_eq!(tensor.numel(), 10000);
}

#[test]
fn test_tensor_reshape_identity() {
    let data = vec![1.0, 2.0, 3.0, 4.0];
    let tensor = Tensor::from_data(&data, &[2, 2]);
    
    let reshaped = tensor.reshape(&[2, 2]).unwrap();
    assert_eq!(tensor.as_slice().unwrap(), reshaped.as_slice().unwrap());
}
