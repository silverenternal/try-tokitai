//! 核心功能性能基准测试
//!
//! 使用 criterion 框架测试 tokitai 核心功能的性能指标

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::collections::HashMap;
use std::time::Duration;

// ============================================================================
// 基础性能测试
// ============================================================================

/// 基准测试：空操作开销
fn benchmark_noop(c: &mut Criterion) {
    c.bench_function("空操作开销", |b| {
        b.iter(|| {
            black_box(());
        })
    });
}

/// 基准测试：字符串创建
fn benchmark_string_creation(c: &mut Criterion) {
    c.bench_function("字符串创建", |b| {
        b.iter(|| {
            black_box(String::from("test_string_for_benchmark"))
        })
    });
}

/// 基准测试：HashMap 插入
fn benchmark_hashmap_insert(c: &mut Criterion) {
    c.bench_function("HashMap 插入 (100 项)", |b| {
        b.iter(|| {
            let mut map = HashMap::new();
            for i in 0..100 {
                map.insert(black_box(i), black_box(i * 2));
            }
            black_box(map);
        })
    });
}

/// 基准测试：JSON 序列化
fn benchmark_json_serialization(c: &mut Criterion) {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct TestData {
        name: String,
        value: u32,
        items: Vec<String>,
    }

    let data = TestData {
        name: "test".to_string(),
        value: 42,
        items: vec!["item1".to_string(), "item2".to_string()],
    };

    c.bench_function("JSON 序列化", |b| {
        b.iter(|| {
            black_box(serde_json::to_string(&data).unwrap())
        })
    });
}

/// 基准测试：JSON 反序列化
fn benchmark_json_deserialization(c: &mut Criterion) {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct TestData {
        name: String,
        value: u32,
        items: Vec<String>,
    }

    let json = r#"{"name":"test","value":42,"items":["item1","item2"]}"#;

    c.bench_function("JSON 反序列化", |b| {
        b.iter(|| {
            black_box(serde_json::from_str::<TestData>(json).unwrap())
        })
    });
}

/// 基准测试：正则表达式匹配
fn benchmark_regex_match(c: &mut Criterion) {
    let re = regex::Regex::new(r"^\w+@\w+\.\w+$").unwrap();

    c.bench_function("正则表达式匹配", |b| {
        b.iter(|| {
            black_box(re.is_match("test@example.com"))
        })
    });
}

/// 基准测试：不同大小的 HashMap 性能
fn benchmark_hashmap_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("HashMap 不同大小");

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut map = HashMap::new();
                for i in 0..size {
                    map.insert(black_box(i), black_box(i * 2));
                }
                black_box(map);
            })
        });
    }
    group.finish();
}

/// 基准测试：字符串连接性能
fn benchmark_string_concat(c: &mut Criterion) {
    let mut group = c.benchmark_group("字符串连接");

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut s = String::new();
                for i in 0..size {
                    s.push_str(&format!("{}", i));
                }
                black_box(s);
            })
        });
    }
    group.finish();
}

// ============================================================================
// 轻量级工具选择器性能测试
// ============================================================================

/// 基准测试：ToolIndex 创建
fn benchmark_tool_index_creation(c: &mut Criterion) {
    use ai_assistant::tool_matrix::tool_selector::ToolIndex;
    use ai_assistant::tool_matrix::matrix::ToolDefinition;

    c.bench_function("ToolIndex 创建 (100 工具)", |b| {
        b.iter(|| {
            let mut index = ToolIndex::new();
            for i in 0..100 {
                let tool = ToolDefinition::new(
                    &format!("tool_{}", i),
                    &format!("Description for tool {}", i),
                    r#"{}"#,
                );
                index.add_tool(tool);
            }
            black_box(index);
        })
    });
}

/// 基准测试：ToolIndex 搜索（小数据集）
fn benchmark_tool_index_search_small(c: &mut Criterion) {
    use ai_assistant::tool_matrix::tool_selector::ToolIndex;
    use ai_assistant::tool_matrix::matrix::ToolDefinition;

    // 创建包含 100 个工具的索引
    let mut index = ToolIndex::new();
    for i in 0..100 {
        let tool = ToolDefinition::new(
            &format!("tool_{}", i),
            &format!("Description for tool {}", i),
            r#"{}"#,
        );
        index.add_tool(tool);
    }

    c.bench_function("ToolIndex 搜索 (100 工具)", |b| {
        b.iter(|| {
            let results = index.search(black_box("tool"), black_box(10));
            black_box(results);
        })
    });
}

/// 基准测试：ToolIndex 搜索（中数据集）
fn benchmark_tool_index_search_medium(c: &mut Criterion) {
    use ai_assistant::tool_matrix::tool_selector::ToolIndex;
    use ai_assistant::tool_matrix::matrix::ToolDefinition;

    // 创建包含 1000 个工具的索引
    let mut index = ToolIndex::new();
    for i in 0..1000 {
        let tool = ToolDefinition::new(
            &format!("tool_{}", i),
            &format!("Description for tool {} - some longer text for better search", i),
            r#"{}"#,
        );
        index.add_tool(tool);
    }

    c.bench_function("ToolIndex 搜索 (1000 工具)", |b| {
        b.iter(|| {
            let results = index.search(black_box("tool"), black_box(10));
            black_box(results);
        })
    });
}

/// 基准测试：ToolIndex 搜索（大数据集）
fn benchmark_tool_index_search_large(c: &mut Criterion) {
    use ai_assistant::tool_matrix::tool_selector::ToolIndex;
    use ai_assistant::tool_matrix::matrix::ToolDefinition;

    // 创建包含 10000 个工具的索引
    let mut index = ToolIndex::new();
    for i in 0..10000 {
        let tool = ToolDefinition::new(
            &format!("tool_{}", i),
            &format!("Description for tool {} - some longer text for better search and matching", i),
            r#"{}"#,
        );
        index.add_tool(tool);
    }

    c.bench_function("ToolIndex 搜索 (10000 工具)", |b| {
        b.iter(|| {
            let results = index.search(black_box("tool"), black_box(10));
            black_box(results);
        })
    });
}

/// 基准测试：LightweightToolSelector 快速搜索
fn benchmark_lightweight_selector_fast_search(c: &mut Criterion) {
    use ai_assistant::tool_matrix::tool_selector::LightweightToolSelector;
    use ai_assistant::tool_matrix::matrix::ToolDefinition;

    // 创建包含 1000 个工具的选择器
    let tools: Vec<ToolDefinition> = (0..1000)
        .map(|i| {
            ToolDefinition::new(
                &format!("tool_{}", i),
                &format!("Description for tool {} - some longer text for better search", i),
                r#"{}"#,
            )
        })
        .collect();

    let selector = LightweightToolSelector::new_without_ai(tools, None);

    // 使用 tokio runtime 运行异步测试
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("LightweightSelector 快速搜索 (1000 工具)", |b| {
        b.to_async(&rt).iter(|| async {
            let results = black_box(selector.search(black_box("tool")).await);
            black_box(results);
        })
    });
}

/// 基准测试：不同工具数量的搜索延迟
fn benchmark_search_latency_by_size(c: &mut Criterion) {
    use ai_assistant::tool_matrix::tool_selector::LightweightToolSelector;
    use ai_assistant::tool_matrix::matrix::ToolDefinition;

    let mut group = c.benchmark_group("搜索延迟对比");

    for &size in [100, 1000, 10000].iter() {
        let tools: Vec<ToolDefinition> = (0..size)
            .map(|i| {
                ToolDefinition::new(
                    &format!("tool_{}", i),
                    &format!("Description for tool {} - some longer text for better search and matching", i),
                    r#"{}"#,
                )
            })
            .collect();

        let selector = LightweightToolSelector::new_without_ai(tools, None);
        let rt = tokio::runtime::Runtime::new().unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &_| {
            b.to_async(&rt).iter(|| async {
                let results = black_box(selector.search(black_box("tool")).await);
                black_box(results);
            })
        });
    }
    group.finish();
}

/// 基准测试：验证 <10ms 延迟目标
fn benchmark_verify_10ms_target(c: &mut Criterion) {
    use ai_assistant::tool_matrix::tool_selector::LightweightToolSelector;
    use ai_assistant::tool_matrix::matrix::ToolDefinition;
    use std::time::Instant;

    // 创建包含 10000 个工具的索引（目标规模）
    let tools: Vec<ToolDefinition> = (0..10000)
        .map(|i| {
            ToolDefinition::new(
                &format!("tool_{}", i),
                &format!("Description for tool {} - some longer text for better search and matching", i),
                r#"{}"#,
            )
        })
        .collect();

    let selector = LightweightToolSelector::new_without_ai(tools, None);
    let rt = tokio::runtime::Runtime::new().unwrap();

    // 测量搜索延迟
    let mut latencies = Vec::new();
    for _ in 0..100 {
        let start = Instant::now();
        rt.block_on(async {
            let _ = selector.search("tool").await;
        });
        latencies.push(start.elapsed());
    }

    let avg_latency = latencies.iter().sum::<std::time::Duration>() / latencies.len() as u32;
    let max_latency = latencies.iter().max().unwrap();

    println!("\n=== 10ms 延迟目标验证 ===");
    println!("工具数量：10,000");
    println!("平均延迟：{:?}", avg_latency);
    println!("最大延迟：{:?}", max_latency);
    println!("目标：<10ms");
    println!("结果：{}", if avg_latency < std::time::Duration::from_millis(10) { "✅ 通过" } else { "❌ 失败" });

    // 基准测试（仅用于报告）
    c.bench_function("10ms 目标验证 (10000 工具)", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = selector.search("tool").await;
        })
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(3))
        .sample_size(100);
    targets =
        // 基础测试
        benchmark_noop,
        benchmark_string_creation,
        benchmark_hashmap_insert,
        benchmark_json_serialization,
        benchmark_json_deserialization,
        benchmark_regex_match,
        benchmark_hashmap_sizes,
        benchmark_string_concat,
        // 工具选择器测试
        benchmark_tool_index_creation,
        benchmark_tool_index_search_small,
        benchmark_tool_index_search_medium,
        benchmark_tool_index_search_large,
        benchmark_lightweight_selector_fast_search,
        benchmark_search_latency_by_size,
        benchmark_verify_10ms_target,
);

criterion_main!(benches);
