//! 核心功能性能基准测试
//!
//! 使用 criterion 框架测试 tokitai 核心功能的性能指标

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::collections::HashMap;
use std::time::Duration;

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

criterion_group!(
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(3))
        .sample_size(100);
    targets = 
        benchmark_noop,
        benchmark_string_creation,
        benchmark_hashmap_insert,
        benchmark_json_serialization,
        benchmark_json_deserialization,
        benchmark_regex_match,
        benchmark_hashmap_sizes,
        benchmark_string_concat,
);

criterion_main!(benches);
