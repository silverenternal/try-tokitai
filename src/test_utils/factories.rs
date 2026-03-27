//! 测试数据工厂
//!
//! 提供标准化的测试数据创建函数

use crate::context::ContextBranch;
use crate::orchestrator::{Workflow, Stage, Step, AgentRole};
use crate::provider_config::ProviderConfig;
use std::path::Path;

/// 创建测试用的 ContextBranch
pub fn create_test_branch(id: &str, name: &str, parent: Option<&str>) -> ContextBranch {
    let parent_hash = parent.unwrap_or("0x0000000000000000000000000000000000000000000000000000000000000000");
    ContextBranch::new(
        id,
        name,
        parent_hash,
        Path::new("/tmp/test_branches").to_path_buf(),
    ).unwrap()
}

/// 创建测试用的 Workflow
pub fn create_test_workflow(
    name: &str,
    stage_count: usize,
    steps_per_stage: usize,
) -> Workflow {
    let mut workflow = Workflow::new(
        name.to_string(),
        format!("Test Workflow: {}", name),
        "A test workflow for unit testing".to_string(),
    );

    for i in 0..stage_count {
        let mut stage = Stage::new(
            format!("stage_{}", i),
            format!("Stage {}", i),
            format!("Description for stage {}", i),
        );

        for j in 0..steps_per_stage {
            stage.add_step(Step::new(
                format!("step_{}_{}", i, j),
                format!("Step {}.{}", i, j),
                AgentRole::Executor,
            ));
        }

        workflow.add_stage(stage);
    }

    workflow
}

/// 创建测试用的 Tensor 数据（形状）
pub fn create_test_shape(dimensions: &[usize]) -> Vec<usize> {
    dimensions.to_vec()
}

/// 创建测试用的 Provider 配置
pub fn create_test_provider_config(name: &str, api_key: &str, api_url: &str) -> ProviderConfig {
    ProviderConfig {
        name: name.to_string(),
        api_key: Some(api_key.to_string()),
        api_url: api_url.to_string(),
        model: "default-model".to_string(),
    }
}

/// 创建 Zipf 分布的测试数据（模拟 80-20 规则）
pub fn create_zipf_distribution(size: usize, exponent: f64) -> Vec<usize> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut data = Vec::with_capacity(size);

    for _ in 0..size {
        // 简化版 Zipf 分布
        let rank = (rng.gen::<f64>().powf(-1.0 / (exponent - 1.0)) as usize) % size.max(1);
        data.push(rank.min(size - 1));
    }

    data
}

/// 创建顺序访问模式测试数据
pub fn create_sequential_access_pattern(size: usize) -> Vec<usize> {
    (0..size).collect()
}

/// 创建循环访问模式测试数据
pub fn create_cyclic_access_pattern(cycle: &[usize], repetitions: usize) -> Vec<usize> {
    cycle.iter()
        .cycle()
        .take(cycle.len() * repetitions)
        .copied()
        .collect()
}

/// 创建随机测试数据
pub fn create_random_data(size: usize, min: usize, max: usize) -> Vec<usize> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..size)
        .map(|_| rng.gen_range(min..=max))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_workflow() {
        let workflow = create_test_workflow("test", 2, 3);
        assert_eq!(workflow.stages.len(), 2);
        assert_eq!(workflow.stages[0].steps.len(), 3);
        assert_eq!(workflow.stages[1].steps.len(), 3);
    }

    #[test]
    fn test_create_zipf_distribution() {
        let data = create_zipf_distribution(1000, 2.0);
        assert_eq!(data.len(), 1000);
        // Zipf 分布应该有很多重复的小值
        let unique_count = data.iter().collect::<std::collections::HashSet<_>>().len();
        assert!(unique_count < data.len() / 2);
    }

    #[test]
    fn test_create_sequential_access_pattern() {
        let data = create_sequential_access_pattern(100);
        assert_eq!(data.len(), 100);
        for i in 0..100 {
            assert_eq!(data[i], i);
        }
    }

    #[test]
    fn test_create_cyclic_access_pattern() {
        let cycle = vec![1, 2, 3];
        let data = create_cyclic_access_pattern(&cycle, 5);
        assert_eq!(data.len(), 15);
        for i in 0..5 {
            assert_eq!(data[i * 3..(i + 1) * 3], [1, 2, 3]);
        }
    }
}
