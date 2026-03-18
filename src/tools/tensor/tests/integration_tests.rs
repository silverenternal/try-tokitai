//! Tensor 模块集成测试

#[cfg(test)]
mod tests {
    use ai_assistant::tools::tensor::{TensorService, TensorTools, Tensor};
    use serde_json::json;

    // ========== TensorService 测试 ==========

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
        let result = service
            .zeros(&[2, 2])?
            .into(); // 需要正确链式调用

        let zeros = service.zeros(&[2, 2])?;
        let added = service.add_scalar(&zeros, 1.0)?;
        let multiplied = service.mul_scalar(&added, 2.0)?;

        assert_eq!(multiplied.as_slice().unwrap(), &[2.0, 2.0, 2.0, 2.0]);

        Ok(())
    }

    // ========== TensorTools 测试 ==========

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
}
