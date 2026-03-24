//! HybridGapDetector 性能基准测试
//!
//! 测试混合缺口检测器的关键性能指标：
//! - 统计筛选延迟
//! - 融合置信度计算延迟
//! - 缓存操作性能
//! - 端到端检测延迟
//!
//! 运行方式：
//! ```bash
//! cargo bench --bench hybrid_gap_detector_bench
//! ```

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::path::PathBuf;
use std::time::Duration;

// 导入被测试模块
use ai_assistant::autonomy::hybrid_gap_detector::{
    HybridGapDetector, StatisticalEvidence, CausalEvidence, GapImpact, CacheEntry,
};
use ai_assistant::autonomy::gap_detector::{TaskExecutionRecord, ToolGap, GapType, GapEvidence};

/// 基准测试：统计证据置信度计算
fn bench_statistical_confidence_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("statistical_confidence");
    
    let detector = HybridGapDetector::new_statistical_only(
        PathBuf::from("/tmp/hybrid_bench_test")
    ).unwrap();

    // 测试不同失败率场景
    for failure_rate in [0.1, 0.3, 0.5, 0.7, 0.9].iter() {
        let stat_evidence = StatisticalEvidence {
            failure_rate: *failure_rate,
            affected_tasks_count: 10,
            avg_satisfaction: 3.0,
            pattern_frequency: 5,
            related_task_ids: vec!["task1".to_string()],
        };

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("failure_rate_{:.1}", failure_rate)),
            &stat_evidence,
            |b, evidence| {
                b.iter(|| {
                    black_box(detector.calculate_statistical_confidence(black_box(evidence)))
                })
            },
        );
    }
    
    group.finish();
}

/// 基准测试：融合置信度计算
fn bench_hybrid_confidence_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("hybrid_confidence");
    
    // 测试不同权重配置
    let test_cases = vec![
        (0.4, 0.6, "default_weights"),
        (0.5, 0.5, "equal_weights"),
        (0.3, 0.7, "causal_heavy"),
        (0.6, 0.4, "statistical_heavy"),
    ];

    for (stat_weight, causal_weight, name) in test_cases {
        let stat_confidence = 0.7;
        let causal_confidence = 0.8;

        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &(stat_weight, causal_weight, stat_confidence, causal_confidence),
            |b, inputs| {
                b.iter(|| {
                    let (stat_w, causal_w, stat_c, causal_c) = black_box(inputs);
                    let hybrid = stat_c * stat_w + causal_c * causal_w;
                    black_box(hybrid)
                })
            },
        );
    }
    
    group.finish();
}

/// 基准测试：缓存插入性能
fn bench_cache_insertion(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_insertion");
    
    let mut detector = HybridGapDetector::new_statistical_only(
        PathBuf::from("/tmp/hybrid_cache_bench")
    ).unwrap();

    group.bench_function("cache_insert_single", |b| {
        b.iter(|| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            detector.cache.insert(
                black_box(format!("test_key_{}", black_box(1))),
                black_box(CacheEntry {
                    gap_id: black_box("test_gap".to_string()),
                    causal_evidence: CausalEvidence {
                        causal_factors: vec![],
                        counterfactual_reasoning: String::new(),
                        llm_confidence: 0.8,
                        expected_impact: GapImpact {
                            affected_tasks: 0,
                            avg_tool_calls_reduced: 0.0,
                            time_saved_minutes: 0.0,
                            expected_success_rate_improvement: 0.0,
                        },
                    },
                    timestamp: now,
                    expires_at: now + 3600,
                }),
            );
        })
    });
    
    group.finish();
}

/// 基准测试：缓存查找性能
fn bench_cache_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_lookup");
    
    let mut detector = HybridGapDetector::new_statistical_only(
        PathBuf::from("/tmp/hybrid_lookup_bench")
    ).unwrap();

    // 预填充缓存
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    for i in 0..100 {
        detector.cache.insert(
            format!("key_{}", i),
            CacheEntry {
                gap_id: format!("gap_{}", i),
                causal_evidence: CausalEvidence {
                    causal_factors: vec![],
                    counterfactual_reasoning: String::new(),
                    llm_confidence: 0.8,
                    expected_impact: GapImpact {
                        affected_tasks: 0,
                        avg_tool_calls_reduced: 0.0,
                        time_saved_minutes: 0.0,
                        expected_success_rate_improvement: 0.0,
                    },
                },
                timestamp: now,
                expires_at: now + 3600,
            },
        );
    }

    // 测试查找性能
    group.bench_function("cache_lookup_hit", |b| {
        b.iter(|| {
            let key = black_box("key_50");
            black_box(detector.cache.get(key))
        })
    });

    group.bench_function("cache_lookup_miss", |b| {
        b.iter(|| {
            let key = black_box("nonexistent_key");
            black_box(detector.cache.get(key))
        })
    });
    
    group.finish();
}

/// 基准测试：任务记录性能
fn bench_task_recording(c: &mut Criterion) {
    let mut group = c.benchmark_group("task_recording");
    
    let mut detector = HybridGapDetector::new_statistical_only(
        PathBuf::from("/tmp/hybrid_record_bench")
    ).unwrap();

    let task_record = TaskExecutionRecord {
        task_id: "bench_task".to_string(),
        task_description: "基准测试任务".to_string(),
        success: false,
        used_tools: vec!["read_file".to_string(), "write_file".to_string()],
        execution_time_ms: 100,
        failure_reason: Some("功能不足".to_string()),
        user_satisfaction: Some(2),
    };

    group.bench_function("record_single_task", |b| {
        b.iter(|| {
            detector.record_task(black_box(task_record.clone()))
        })
    });
    
    group.finish();
}

/// 基准测试：优先级计算
fn bench_priority_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("priority_calculation");
    
    let detector = HybridGapDetector::new_statistical_only(
        PathBuf::from("/tmp/hybrid_priority_bench")
    ).unwrap();

    let test_cases = vec![
        (5, 0.3, "low_priority_low_confidence"),
        (5, 0.7, "low_priority_high_confidence"),
        (8, 0.3, "high_priority_low_confidence"),
        (8, 0.7, "high_priority_high_confidence"),
    ];

    for (priority, confidence, name) in test_cases {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &(priority, confidence),
            |b, inputs| {
                b.iter(|| {
                    let (p, c) = black_box(inputs);
                    black_box(detector.calculate_hybrid_priority(*p, *c))
                })
            },
        );
    }
    
    group.finish();
}

/// 基准测试：统计证据提取
fn bench_statistical_evidence_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("evidence_extraction");
    
    let detector = HybridGapDetector::new_statistical_only(
        PathBuf::from("/tmp/hybrid_evidence_bench")
    ).unwrap();

    // 创建测试缺口
    let gap = ToolGap {
        id: "test_gap".to_string(),
        gap_type: GapType::MissingTool,
        description: "测试缺口".to_string(),
        suggested_tool_name: Some("test_tool".to_string()),
        suggested_capabilities: vec!["capability1".to_string()],
        priority: 7,
        evidence: vec![
            GapEvidence {
                evidence_type: "statistical".to_string(),
                description: "高失败率".to_string(),
                confidence: 0.8,
                related_task_ids: vec!["task1".to_string(), "task2".to_string()],
                occurrence_count: 5,
            },
        ],
        impact_scope: "test scope".to_string(),
    };

    group.bench_function("extract_statistical_evidence", |b| {
        b.iter(|| {
            black_box(detector.extract_statistical_evidence(black_box(&gap)))
        })
    });
    
    group.finish();
}

// ============================================================================
// 配置
// ============================================================================

criterion_group!(
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(3))
        .sample_size(100);
    targets = 
        bench_statistical_confidence_calculation,
        bench_hybrid_confidence_calculation,
        bench_cache_insertion,
        bench_cache_lookup,
        bench_task_recording,
        bench_priority_calculation,
        bench_statistical_evidence_extraction,
);

criterion_main!(benches);
