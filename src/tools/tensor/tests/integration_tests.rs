//! Tensor 模块集成测试
//!
//! 测试完整的端到端场景，包括：
//! - 多步骤工作流
//! - 复杂计算场景
//! - 并发操作

use ai_assistant::tools::tensor::{TensorService, TensorTools, Tensor};
use serde_json::json;

// ========== 端到端场景测试 ==========

#[test]
fn test_neural_network_forward_pass() -> anyhow::Result<()> {
    let service = TensorService::new();

    // 模拟一个简单的两层神经网络前向传播
    // 输入：[batch_size=2, input_dim=3]
    let input = service.from_data(
        &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        &[2, 3]
    )?;

    // 权重层 1：[3, 4]
    let w1 = service.from_data(
        &[0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2],
        &[3, 4]
    )?;

    // 权重层 2：[4, 1]
    let w2 = service.from_data(
        &[0.1, 0.2, 0.3, 0.4],
        &[4, 1]
    )?;

    // 层 1: input @ w1
    let layer1 = service.matmul(&input, &w1)?;
    assert_eq!(layer1.dims(), &[2, 4]);

    // 激活：ReLU
    let activated1 = service.relu(&layer1)?;

    // 层 2: activated1 @ w2
    let output = service.matmul(&activated1, &w2)?;
    assert_eq!(output.dims(), &[2, 1]);

    Ok(())
}

#[test]
fn test_batch_normalization_workflow() -> anyhow::Result<()> {
    let service = TensorService::new();

    // 模拟批量归一化流程
    let batch = service.from_data(
        &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
        &[4, 2]
    )?;

    // 计算均值
    let mean = service.mean(&batch, &[0])?;
    assert_eq!(mean.dims(), &[2]);

    // 计算标准差（简化版）
    let centered = service.sub(&batch, &service.reshape(&mean, &[1, 2])?)?;
    let squared = service.mul(&centered, &centered)?;
    let variance = service.mean(&squared, &[0])?;

    // 归一化
    let std = service.sqrt(&variance)?;
    let normalized = service.div(&centered, &service.reshape(&std, &[1, 2])?)?;

    // 验证归一化后的均值接近 0
    let norm_mean = service.mean(&normalized, &[0])?;
    let norm_mean_slice = norm_mean.as_slice().unwrap();
    assert!(norm_mean_slice[0].abs() < 1e-5);
    assert!(norm_mean_slice[1].abs() < 1e-5);

    Ok(())
}

#[test]
fn test_gradient_descent_step() -> anyhow::Result<()> {
    let service = TensorService::new();

    // 模拟梯度下降更新步骤
    let params = service.from_data(&[1.0, 2.0, 3.0, 4.0], &[4])?;
    let gradients = service.from_data(&[0.1, 0.2, 0.3, 0.4], &[4])?;
    let learning_rate = 0.01;

    // 参数更新：params = params - lr * gradients
    let scaled_grad = service.mul_scalar(&gradients, learning_rate)?;
    let new_params = service.sub(&params, &scaled_grad)?;

    let new_params_slice = new_params.as_slice().unwrap();
    assert!((new_params_slice[0] - 0.999).abs() < 0.002);
    assert!((new_params_slice[1] - 1.998).abs() < 0.002);

    Ok(())
}

// ========== 并发操作测试 ==========

#[test]
fn test_concurrent_tensor_operations() {
    use std::thread;
    use std::sync::{Arc, Mutex};

    let service = Arc::new(Mutex::new(TensorService::new()));
    let mut handles = vec![];

    // 创建多个线程并发执行张量操作
    for i in 0..5 {
        let service_clone = Arc::clone(&service);
        let handle = thread::spawn(move || {
            let svc = service_clone.lock().unwrap();
            let tensor = svc.from_data(&[1.0, 2.0, 3.0], &[3]).unwrap();
            let result = svc.mul_scalar(&tensor, (i + 1) as f64).unwrap();
            
            let expected: Vec<f64> = vec![1.0, 2.0, 3.0].iter()
                .map(|&x| x * (i + 1) as f64)
                .collect();
            assert_eq!(result.as_slice().unwrap(), &expected);
        });
        handles.push(handle);
    }

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }
}

// ========== JSON 工具集成测试 ==========

#[test]
fn test_tools_json_workflow() -> anyhow::Result<()> {
    let tools = TensorTools::new();

    // 创建张量
    let zeros_result = tools.zeros(vec![2, 2])?;
    let zeros_shape = zeros_result["shape"].as_array().unwrap();
    assert_eq!(zeros_shape.len(), 2);

    // 使用 JSON 数据进行矩阵乘法
    let a = json!({
        "shape": [2, 2],
        "data": [1.0, 2.0, 3.0, 4.0]
    });
    let b = json!({
        "shape": [2, 2],
        "data": [5.0, 6.0, 7.0, 8.0]
    });

    let result = tools.matmul(a, b)?;
    let obj = result.as_object().unwrap();
    
    assert_eq!(obj["shape"].as_array().unwrap().len(), 2);
    assert_eq!(obj["data"].as_array().unwrap().len(), 4);

    Ok(())
}

// ========== 性能边界测试 ==========

#[test]
fn test_large_matrix_multiplication() -> anyhow::Result<()> {
    let service = TensorService::new();

    // 创建较大的矩阵
    let size = 100;
    let data_a: Vec<f64> = (0..size * size).map(|i| i as f64).collect();
    let data_b: Vec<f64> = (0..size * size).map(|i| (i * 2) as f64).collect();

    let a = service.from_data(&data_a, &[size, size])?;
    let b = service.from_data(&data_b, &[size, size])?;

    let result = service.matmul(&a, &b)?;
    assert_eq!(result.dims(), &[size, size]);

    Ok(())
}

#[test]
fn test_deep_chain_operations() -> anyhow::Result<()> {
    let service = TensorService::new();

    // 创建深度操作链
    let mut tensor = service.from_data(&[1.0, 2.0, 3.0, 4.0], &[2, 2])?;

    for i in 0..10 {
        tensor = service.add_scalar(&tensor, 1.0)?;
        tensor = service.mul_scalar(&tensor, 1.1)?;
    }

    // 验证最终结果
    let final_slice = tensor.as_slice().unwrap();
    assert!(final_slice.iter().all(|&x| x.is_finite()));
    assert!(final_slice.iter().all(|&x| x > 10.0)); // 经过 10 次增长应该大于 10

    Ok(())
}
