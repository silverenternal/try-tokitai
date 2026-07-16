//! 工具选择器性能测试
//!
//! 测试 TrieIndex 集成后的搜索性能
//!
//! 运行方式：
//! ```bash
//! cargo run --example perf_test --release
//! ```

use std::time::Instant;

fn main() {
    println!("=== 工具选择器性能测试 ===\n");

    // 创建测试数据集（使用更多样化的工具名称）
    let test_sizes = [100, 1000, 5000, 10000];
    let tool_prefixes = [
        "file_", "net_", "sys_", "git_", "data_", "code_", "http_", "db_", "ai_", "test_",
    ];

    for &size in &test_sizes {
        println!("--- 工具数量：{} ---", size);

        // 创建工具定义（使用多样化前缀）
        let tools: Vec<ToolDef> = (0..size)
            .map(|i| {
                let prefix = tool_prefixes[i % tool_prefixes.len()];
                ToolDef {
                    name: format!("{}op_{}", prefix, i),
                    description: format!("{} operation tool {} for processing tasks", prefix, i),
                }
            })
            .collect();

        // 创建索引
        let mut index = ToolIndex::new();
        let start = Instant::now();
        for tool in &tools {
            index.add_tool(tool.clone());
        }
        let build_time = start.elapsed();
        println!("索引构建时间：{:?}", build_time);

        // 测试前缀搜索
        let mut total_search_time = Duration::ZERO;
        let iterations = 100;

        for _ in 0..iterations {
            let start = Instant::now();
            let _results = index.search("file_", 50);
            total_search_time += start.elapsed();
        }

        let avg_search_time = total_search_time / iterations;
        let max_search_time = total_search_time * 2 / iterations; // 估算

        println!(
            "平均搜索延迟：{:?} ({} 次迭代)",
            avg_search_time, iterations
        );
        println!("目标：<10ms");
        println!(
            "结果：{}\n",
            if avg_search_time < Duration::from_millis(10) {
                "✅ 通过"
            } else {
                "❌ 失败"
            }
        );
    }

    // 详细对比测试
    println!("\n=== 详细性能对比 (5000 工具) ===");

    let size = 5000;
    let tools: Vec<ToolDef> = (0..size)
        .map(|i| {
            let prefix = tool_prefixes[i % tool_prefixes.len()];
            ToolDef {
                name: format!("{}op_{}", prefix, i),
                description: format!("{} operation tool {} for processing tasks", prefix, i),
            }
        })
        .collect();

    let mut index = ToolIndex::new();
    for tool in &tools {
        index.add_tool(tool.clone());
    }

    // 测试不同查询类型
    let queries = [
        ("前缀查询", "file_"),
        ("包含查询", "op"),
        ("语义查询", "read write"),
        ("精确匹配", "file_op_123"),
    ];

    for (query_name, query) in queries {
        let mut total_time = Duration::ZERO;
        let iterations = 50;

        for _ in 0..iterations {
            let start = Instant::now();
            let _results = index.search(query, 50);
            total_time += start.elapsed();
        }

        let avg_time = total_time / iterations;
        println!("{} ({}): {:?}", query_name, query, avg_time);
    }
}

use std::collections::HashMap;
use std::time::Duration;

#[derive(Clone)]
struct ToolDef {
    name: String,
    description: String,
}

struct ToolIndex {
    tools: HashMap<String, ToolDef>,
    keyword_index: HashMap<String, Vec<String>>,
    trie_index: TrieIndex,
}

impl ToolIndex {
    fn new() -> Self {
        Self {
            tools: HashMap::new(),
            keyword_index: HashMap::new(),
            trie_index: TrieIndex::new(),
        }
    }

    fn add_tool(&mut self, tool: ToolDef) {
        let name = tool.name.clone();

        // 提取关键词
        let keywords: Vec<String> = name.split('_').map(|s| s.to_lowercase()).collect();

        for keyword in &keywords {
            self.keyword_index
                .entry(keyword.clone())
                .or_insert_with(Vec::new)
                .push(name.clone());
        }

        // 添加到 Trie 索引
        self.trie_index.add_tool(&name, self.tools.len() as u64);

        self.tools.insert(name.clone(), tool);
    }

    fn search(&self, query: &str, max_results: usize) -> Vec<String> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();
        let mut seen = HashMap::new();

        // 1. Trie 树前缀搜索（最快）
        let trie_results = self.trie_index.search_prefix(&query_lower);
        for tool_name in trie_results {
            if !seen.contains_key(&tool_name) {
                results.push(tool_name.clone());
                seen.insert(tool_name.clone(), ());
                if results.len() >= max_results {
                    return results;
                }
            }
        }

        // 2. 关键词匹配
        for (keyword, tools) in &self.keyword_index {
            if keyword.contains(&query_lower) || query_lower.contains(keyword) {
                for tool_name in tools {
                    if !seen.contains_key(tool_name) {
                        results.push(tool_name.clone());
                        seen.insert(tool_name.clone(), ());
                        if results.len() >= max_results {
                            return results;
                        }
                    }
                }
            }
        }

        results
    }
}

struct TrieIndex {
    tool_map: HashMap<String, u64>,
    keyword_index: HashMap<String, Vec<String>>,
}

impl TrieIndex {
    fn new() -> Self {
        Self {
            tool_map: HashMap::new(),
            keyword_index: HashMap::new(),
        }
    }

    fn add_tool(&mut self, name: &str, id: u64) {
        self.tool_map.insert(name.to_string(), id);

        // 构建前缀索引
        let prefixes = Self::extract_prefixes(name);
        for prefix in prefixes {
            self.keyword_index
                .entry(prefix)
                .or_insert_with(Vec::new)
                .push(name.to_string());
        }
    }

    fn search_prefix(&self, prefix: &str) -> Vec<String> {
        let prefix_lower = prefix.to_lowercase();
        let mut results = Vec::new();

        // 直接查找前缀
        if let Some(tools) = self.keyword_index.get(&prefix_lower) {
            results.extend(tools.clone());
        }

        // 也查找包含前缀的关键词
        for (keyword, tools) in &self.keyword_index {
            if keyword.starts_with(&prefix_lower) && !results.contains(&tools[0]) {
                results.extend(tools.clone());
            }
        }

        results
    }

    fn extract_prefixes(name: &str) -> Vec<String> {
        let mut prefixes = Vec::new();
        let name_lower = name.to_lowercase();

        // 提取不同长度的前缀
        for i in 1..=name_lower.len().min(10) {
            prefixes.push(name_lower[..i].to_string());
        }

        // 提取下划线分隔的部分
        let parts: Vec<&str> = name_lower.split('_').collect();
        for (i, part) in parts.iter().enumerate() {
            let prefix = parts[..=i].join("_");
            if !prefix.is_empty() {
                prefixes.push(prefix);
            }
        }

        prefixes
    }
}
