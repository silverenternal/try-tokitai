//! 核心功能性能基准测试
//!
//! 使用 criterion 框架测试基础性能指标
//!
//! 运行方式：
//! ```bash
//! cargo bench --bench core_benchmarks
//! ```
//!
//! 注意：工具选择器性能测试请使用 `cargo run --example perf_test --release`
//!
//! ## 测试覆盖
//! - 基础操作开销（空操作、字符串创建、HashMap、JSON）
//! - 工具索引性能（创建、搜索）
//! - 上下文哈希链性能
//! - 规则分类器性能

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::HashMap;
use std::time::Duration;

// ============================================================================
// 基础性能测试
// ============================================================================

/// 基准测试：空操作开销
fn benchmark_noop(c: &mut Criterion) {
    c.bench_function("noop_overhead", |b| {
        b.iter(|| {
            black_box(());
        })
    });
}

/// 基准测试：字符串创建
fn benchmark_string_creation(c: &mut Criterion) {
    c.bench_function("string_creation", |b| {
        b.iter(|| black_box(String::from("test_string_for_benchmark")))
    });
}

/// 基准测试：HashMap 插入
fn benchmark_hashmap_insert(c: &mut Criterion) {
    c.bench_function("hashmap_insert_100", |b| {
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

    c.bench_function("json_serialization", |b| {
        b.iter(|| black_box(serde_json::to_string(&data).unwrap()))
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

    c.bench_function("json_deserialization", |b| {
        b.iter(|| black_box(serde_json::from_str::<TestData>(json).unwrap()))
    });
}

/// 基准测试：正则表达式匹配
fn benchmark_regex_match(c: &mut Criterion) {
    let re = regex::Regex::new(r"^\w+@\w+\.\w+$").unwrap();

    c.bench_function("regex_match", |b| {
        b.iter(|| black_box(re.is_match("test@example.com")))
    });
}

/// 基准测试：不同大小的 HashMap 性能
fn benchmark_hashmap_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("hashmap_sizes");

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
    let mut group = c.benchmark_group("string_concat");

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
// 简化版工具索引性能测试（使用本地定义，避免依赖问题）
// ============================================================================

#[derive(Clone)]
struct ToolDef {
    name: String,
    description: String,
}

impl ToolDef {
    fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
        }
    }
}

/// 基准测试：工具索引创建性能
fn benchmark_tool_index_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("tool_index_creation");

    for &size in [100, 1000, 5000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let mut index: HashMap<String, ToolDef> = HashMap::new();
                for i in 0..size {
                    let tool = ToolDef::new(
                        &format!("tool_{}", i),
                        &format!("Description for tool {} with keywords", i),
                    );
                    index.insert(tool.name.clone(), tool);
                }
                black_box(index);
            })
        });
    }
    group.finish();
}

/// 基准测试：工具搜索性能（简化版 HashMap 实现）
fn benchmark_tool_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("tool_search");
    group.measurement_time(Duration::from_secs(15));

    // 预创建索引
    let tools: Vec<ToolDef> = (0..5000)
        .map(|i| {
            ToolDef::new(
                &format!("file_op_{}", i),
                &format!("File operation tool {} for reading and writing", i),
            )
        })
        .collect();

    let index: HashMap<String, &ToolDef> = tools.iter().map(|t| (t.name.clone(), t)).collect();

    // 测试前缀搜索
    group.bench_function("prefix_search_file", |b| {
        b.iter(|| {
            let results: Vec<_> = index
                .iter()
                .filter(|(name, _)| name.starts_with("file_"))
                .take(50)
                .collect();
            black_box(results);
        })
    });

    // 测试包含搜索
    group.bench_function("contains_search_op", |b| {
        b.iter(|| {
            let results: Vec<_> = index
                .iter()
                .filter(|(name, _)| name.contains("op"))
                .take(50)
                .collect();
            black_box(results);
        })
    });

    group.finish();
}

// ============================================================================
// 上下文哈希链性能测试
// ============================================================================

/// 基准测试：上下文哈希链追加性能
fn benchmark_hash_chain_append(c: &mut Criterion) {
    use sha2::{Digest, Sha256};

    let mut group = c.benchmark_group("hash_chain_append");

    // 模拟增量哈希链追加
    for &size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let mut current_hash = String::from(
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                );

                for i in 0..size {
                    let content_hash = format!("content_{}", i);
                    let mut hasher = Sha256::new();
                    hasher.update(current_hash.as_bytes());
                    hasher.update(content_hash.as_bytes());
                    let result = hasher.finalize();
                    current_hash = format!("0x{}", hex::encode(result));
                }

                black_box(&current_hash);
            })
        });
    }
    group.finish();
}

/// 基准测试：上下文哈希链验证性能
fn benchmark_hash_chain_verify(c: &mut Criterion) {
    use sha2::{Digest, Sha256};

    let mut group = c.benchmark_group("hash_chain_verify");

    // 预先生成哈希链
    let mut chain: Vec<(String, String)> = Vec::new();
    let mut current_hash =
        String::from("0x0000000000000000000000000000000000000000000000000000000000000000");

    for i in 0..100 {
        let content_hash = format!("content_{}", i);
        let mut hasher = Sha256::new();
        hasher.update(current_hash.as_bytes());
        hasher.update(content_hash.as_bytes());
        let result = hasher.finalize();
        current_hash = format!("0x{}", hex::encode(result));
        chain.push((current_hash.clone(), content_hash));
    }

    group.bench_function("verify_100_nodes", |b| {
        b.iter(|| {
            let mut prev_hash =
                "0x0000000000000000000000000000000000000000000000000000000000000000";
            let mut valid = true;

            for (node_hash, content_hash) in &chain {
                let mut hasher = Sha256::new();
                hasher.update(prev_hash.as_bytes());
                hasher.update(content_hash.as_bytes());
                let result = hasher.finalize();
                let expected_hash = format!("0x{}", hex::encode(result));

                if &expected_hash != node_hash {
                    valid = false;
                    break;
                }
                prev_hash = node_hash;
            }

            black_box(valid);
        })
    });
    group.finish();
}

// ============================================================================
// 规则分类器性能测试（模拟分层缓存）
// ============================================================================

/// 基准测试：分层缓存查找性能
fn benchmark_layered_cache_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("layered_cache_lookup");

    // 模拟 L1 缓存（精确匹配，最快）
    let l1_cache: HashMap<String, Vec<String>> = (0..100)
        .map(|i| (format!("exact_{}", i), vec![format!("tool_{}", i)]))
        .collect();

    // 模拟 L2 缓存（模糊匹配）
    let l2_cache: HashMap<String, Vec<String>> = (0..50)
        .map(|i| (format!("fuzzy_{}", i), vec![format!("tool_{}", i)]))
        .collect();

    // 测试 L1 缓存命中
    group.bench_function("l1_cache_hit", |b| {
        b.iter(|| {
            let query = "exact_50";
            let result = l1_cache.get(query);
            black_box(result);
        })
    });

    // 测试 L2 缓存命中
    group.bench_function("l2_cache_hit", |b| {
        b.iter(|| {
            let query = "fuzzy_25";
            let result = l2_cache.get(query);
            black_box(result);
        })
    });

    // 测试缓存未命中
    group.bench_function("cache_miss", |b| {
        b.iter(|| {
            let query = "unknown_query";
            let l1_result = l1_cache.get(query);
            let l2_result = if l1_result.is_none() {
                l2_cache.get(query)
            } else {
                None
            };
            black_box((l1_result, l2_result));
        })
    });

    group.finish();
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
        // 工具索引测试
        benchmark_tool_index_creation,
        benchmark_tool_search,
        // 上下文哈希链测试
        benchmark_hash_chain_append,
        benchmark_hash_chain_verify,
        // 规则分类器测试
        benchmark_layered_cache_lookup,
);

criterion_main!(benches);
