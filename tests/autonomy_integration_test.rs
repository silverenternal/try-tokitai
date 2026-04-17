//! 自主进化模块集成测试
//!
//! 测试自主进化核心功能：
//! - 工具缺口检测 (Gap Detector)
//! - 混合缺口检测器 (HybridGapDetector)
//! - 工具优化器 (ToolOptimizer)
//! - Prompt 优化器 (PromptOptimizer)

#[cfg(test)]
mod tests {
    use ai_assistant::autonomy::gap_detector::{
        TaskExecutionRecord, ToolGapDetector, ToolUsageStats,
    };
    use ai_assistant::autonomy::tool_optimizer::{OptimizationType, ToolMetrics, ToolOptimizer};
    use tempfile::TempDir;

    // ========== ToolGapDetector 测试 ==========

    #[test]
    fn test_gap_detector_basic_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let mut detector = ToolGapDetector::new(temp_dir.path().to_path_buf()).unwrap();

        // 记录成功任务
        detector.record_task(TaskExecutionRecord {
            task_id: "success_1".to_string(),
            task_description: "成功完成任务".to_string(),
            success: true,
            used_tools: vec!["read_file".to_string(), "write_file".to_string()],
            execution_time_ms: 150,
            failure_reason: None,
            user_satisfaction: Some(5),
        });

        // 记录失败任务
        detector.record_task(TaskExecutionRecord {
            task_id: "fail_1".to_string(),
            task_description: "下载文件失败".to_string(),
            success: false,
            used_tools: vec![],
            execution_time_ms: 50,
            failure_reason: Some("缺少批量下载工具".to_string()),
            user_satisfaction: Some(1),
        });

        // 执行检测
        let gaps = detector.analyze_and_detect();

        // 验证结果
        assert!(!gaps.is_empty(), "应该检测到至少一个缺口");

        let gap = &gaps[0];
        assert!(!gap.id.is_empty());
        assert!(!gap.description.is_empty());
        assert!(gap.priority >= 1 && gap.priority <= 10);
    }

    #[test]
    fn test_gap_detector_pattern_recognition() {
        let temp_dir = TempDir::new().unwrap();
        let mut detector = ToolGapDetector::new(temp_dir.path().to_path_buf()).unwrap();

        // 模拟相似的失败模式（5 次）
        for i in 0..5 {
            detector.record_task(TaskExecutionRecord {
                task_id: format!("batch_fail_{}", i),
                task_description: "批量操作失败".to_string(),
                success: false,
                used_tools: vec!["single_download".to_string()],
                execution_time_ms: 100,
                failure_reason: Some("需要批量操作但只有单个操作工具".to_string()),
                user_satisfaction: Some(2),
            });
        }

        let gaps = detector.analyze_and_detect();

        // 应该识别出模式并生成缺口
        assert!(!gaps.is_empty());

        // 验证缺口信息
        let gap = gaps
            .iter()
            .find(|g| g.description.contains("批量") || g.description.contains("batch"))
            .expect("应该检测到批量操作相关的缺口");

        assert!(gap.priority >= 5, "重复出现的缺口应该有较高优先级");
    }

    #[test]
    fn test_gap_detector_tool_stats() {
        let temp_dir = TempDir::new().unwrap();
        let mut detector = ToolGapDetector::new(temp_dir.path().to_path_buf()).unwrap();

        // 记录工具使用
        for i in 0..10 {
            detector.record_task(TaskExecutionRecord {
                task_id: format!("task_{}", i),
                task_description: "文件操作".to_string(),
                success: i < 8, // 80% 成功率
                used_tools: vec!["read_file".to_string()],
                execution_time_ms: 50 + (i * 10) as u64,
                failure_reason: if i >= 8 {
                    Some("失败".to_string())
                } else {
                    None
                },
                user_satisfaction: Some(if i < 8 { 4 } else { 2 }),
            });
        }

        let stats = detector.get_tool_stats();
        assert!(stats.contains_key("read_file"));

        let read_file_stats = stats.get("read_file").unwrap();
        assert_eq!(read_file_stats.usage_count, 10);
        assert!((read_file_stats.success_rate - 0.8).abs() < 0.01);
    }

    // ========== ToolOptimizer 测试 ==========

    #[test]
    fn test_optimizer_health_calculation() {
        let temp_dir = TempDir::new().unwrap();
        let mut optimizer = ToolOptimizer::new(temp_dir.path().to_path_buf()).unwrap();

        // 添加健康工具
        optimizer.update_metrics(ToolMetrics {
            tool_name: "healthy_tool".to_string(),
            total_calls: 1000,
            success_count: 990,
            failure_count: 10,
            avg_execution_time_ms: 20.0,
            last_used_timestamp: 0,
            avg_satisfaction: 4.8,
            tags: vec!["file".to_string()],
            dependencies: vec![],
        });

        // 添加不健康工具
        optimizer.update_metrics(ToolMetrics {
            tool_name: "unhealthy_tool".to_string(),
            total_calls: 100,
            success_count: 50,
            failure_count: 50,
            avg_execution_time_ms: 500.0,
            last_used_timestamp: 0,
            avg_satisfaction: 2.0,
            tags: vec!["network".to_string()],
            dependencies: vec![],
        });

        optimizer.calculate_health_scores();

        let healthy_score = optimizer
            .health_scores
            .get("healthy_tool")
            .unwrap()
            .health_score;
        let unhealthy_score = optimizer
            .health_scores
            .get("unhealthy_tool")
            .unwrap()
            .health_score;

        assert!(healthy_score > 0.8, "健康工具应该有高分数");
        assert!(unhealthy_score < 0.5, "不健康工具应该有低分数");
        assert!(
            healthy_score > unhealthy_score,
            "健康工具分数应该高于不健康工具"
        );
    }

    #[test]
    fn test_optimizer_redundancy_detection() {
        let temp_dir = TempDir::new().unwrap();
        let mut optimizer = ToolOptimizer::new(temp_dir.path().to_path_buf()).unwrap();

        // 添加两个功能相似的工具
        optimizer.update_metrics(ToolMetrics {
            tool_name: "read_text_file".to_string(),
            total_calls: 500,
            success_count: 490,
            failure_count: 10,
            avg_execution_time_ms: 25.0,
            last_used_timestamp: 0,
            avg_satisfaction: 4.5,
            tags: vec!["file".to_string(), "read".to_string(), "text".to_string()],
            dependencies: vec![],
        });

        optimizer.update_metrics(ToolMetrics {
            tool_name: "text_file_reader".to_string(),
            total_calls: 50,
            success_count: 48,
            failure_count: 2,
            avg_execution_time_ms: 30.0,
            last_used_timestamp: 0,
            avg_satisfaction: 4.0,
            tags: vec!["file".to_string(), "read".to_string(), "text".to_string()],
            dependencies: vec![],
        });

        let suggestions = optimizer.analyze_and_optimize();

        // 应该检测到冗余并建议合并
        let merge_suggestion = suggestions
            .iter()
            .find(|s| s.optimization_type == OptimizationType::Merge);

        assert!(merge_suggestion.is_some(), "应该检测到冗余工具并建议合并");

        if let Some(suggestion) = merge_suggestion {
            assert!(suggestion.affected_tools.len() >= 2);
            assert!(suggestion.priority >= 5);
        }
    }

    #[test]
    fn test_optimizer_low_usage_detection() {
        let temp_dir = TempDir::new().unwrap();
        let mut optimizer = ToolOptimizer::new(temp_dir.path().to_path_buf()).unwrap();

        // 添加低使用率工具
        optimizer.update_metrics(ToolMetrics {
            tool_name: "rarely_used_tool".to_string(),
            total_calls: 2,
            success_count: 2,
            failure_count: 0,
            avg_execution_time_ms: 100.0,
            last_used_timestamp: 0,
            avg_satisfaction: 3.0,
            tags: vec!["utility".to_string()],
            dependencies: vec![],
        });

        // 添加常用工具作为对比
        optimizer.update_metrics(ToolMetrics {
            tool_name: "frequently_used_tool".to_string(),
            total_calls: 1000,
            success_count: 990,
            failure_count: 10,
            avg_execution_time_ms: 20.0,
            last_used_timestamp: 0,
            avg_satisfaction: 4.8,
            tags: vec!["core".to_string()],
            dependencies: vec![],
        });

        let suggestions = optimizer.analyze_and_optimize();

        // 应该建议废弃或改进低使用率工具
        let deprecate_suggestion = suggestions
            .iter()
            .find(|s| s.optimization_type == OptimizationType::Deprecate);

        assert!(deprecate_suggestion.is_some(), "应该建议废弃低使用率工具");

        if let Some(suggestion) = deprecate_suggestion {
            assert!(suggestion
                .affected_tools
                .contains(&"rarely_used_tool".to_string()));
        }
    }

    // ========== 集成场景测试 ==========

    #[test]
    fn test_gap_detector_and_optimizer_integration() {
        let temp_dir = TempDir::new().unwrap();

        // 创建缺口检测器
        let mut detector = ToolGapDetector::new(temp_dir.path().to_path_buf()).unwrap();

        // 模拟真实使用场景：某些工具频繁失败
        for i in 0..20 {
            let success = i < 10; // 50% 失败率
            detector.record_task(TaskExecutionRecord {
                task_id: format!("task_{}", i),
                task_description: "网络请求".to_string(),
                success,
                used_tools: vec!["http_get".to_string()],
                execution_time_ms: if success { 200 } else { 5000 },
                failure_reason: if !success {
                    Some("超时或连接失败".to_string())
                } else {
                    None
                },
                user_satisfaction: Some(if success { 4 } else { 1 }),
            });
        }

        // 检测缺口
        let gaps = detector.analyze_and_detect();
        assert!(!gaps.is_empty());

        // 创建优化器并导入数据
        let mut optimizer = ToolOptimizer::new(temp_dir.path().to_path_buf()).unwrap();

        // 基于缺口检测的结果更新优化器指标
        optimizer.update_metrics(ToolMetrics {
            tool_name: "http_get".to_string(),
            total_calls: 20,
            success_count: 10,
            failure_count: 10,
            avg_execution_time_ms: 2600.0,
            last_used_timestamp: 0,
            avg_satisfaction: 2.5,
            tags: vec!["network".to_string(), "http".to_string()],
            dependencies: vec![],
        });

        let suggestions = optimizer.analyze_and_optimize();

        // 应该生成改进建议
        assert!(!suggestions.is_empty());

        // 验证建议质量
        let improve_suggestion = suggestions
            .iter()
            .find(|s| s.optimization_type == OptimizationType::Improve);

        assert!(improve_suggestion.is_some(), "应该建议改进低可靠性的工具");
    }

    #[test]
    fn test_concurrent_task_recording() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let temp_dir = TempDir::new().unwrap();
        let detector: Arc<Mutex<ToolGapDetector>> = Arc::new(Mutex::new(
            ToolGapDetector::new(temp_dir.path().to_path_buf()).unwrap(),
        ));

        // 多线程记录任务
        let mut handles = vec![];
        for i in 0..10 {
            let detector_clone = Arc::clone(&detector);
            let handle = thread::spawn(move || {
                let mut det = detector_clone.lock().unwrap();
                det.record_task(TaskExecutionRecord {
                    task_id: format!("thread_{}_task", i),
                    task_description: "并发任务".to_string(),
                    success: true,
                    used_tools: vec![format!("tool_{}", i)],
                    execution_time_ms: 100,
                    failure_reason: None,
                    user_satisfaction: Some(5),
                });
            });
            handles.push(handle);
        }

        // 等待所有线程完成
        for handle in handles {
            handle.join().unwrap();
        }

        // 验证所有任务都被记录
        let det = detector.lock().unwrap();
        assert_eq!(det.get_task_records().len(), 10);
    }
}
