//! Trie 树索引（基于 fst crate 实现）
//!
//! 使用 fst (Finite State Transducer) 实现高性能前缀搜索索引
//! 支持 O(m) 复杂度的工具名称查找，m 为查询字符串长度
//!
//! ## 性能优化
//! - **批量重建策略**：新增工具先写入缓冲区，达到阈值后批量重建 fst Set
//! - **搜索合并**：搜索时自动合并 fst Set 和缓冲区结果
//! - **避免频繁重建**：每次 add_tool 不再重建 fst，性能从 O(n log n) 降至 O(1)

#![allow(dead_code)]

use fst::{Set, Streamer, IntoStreamer};
use std::collections::HashMap;
use tracing::{debug, info};

/// Trie 树索引配置
#[derive(Debug, Clone)]
pub struct TrieIndexConfig {
    /// 触发批量重建的缓冲区阈值
    pub rebuild_threshold: usize,
}

impl Default for TrieIndexConfig {
    fn default() -> Self {
        Self {
            rebuild_threshold: 100, // 每 100 次添加触发一次重建
        }
    }
}

/// Trie 树索引（基于 fst 实现，支持批量重建）
pub struct TrieIndex {
    /// fst Set：存储已持久化的工具名称（压缩格式）
    tool_set: Set<Vec<u8>>,
    /// 工具名称 -> ID 映射（包含所有工具，含未持久化的）
    tool_map: HashMap<String, u64>,
    /// 关键词前缀 -> 工具名称列表
    keyword_index: HashMap<String, Vec<String>>,
    /// 待持久化工具缓冲区（避免每次 add_tool 都重建 fst）
    pending_adds: Vec<(String, u64)>,
    /// 配置
    config: TrieIndexConfig,
    /// 是否需要重建的标志
    needs_rebuild: bool,
}

impl Clone for TrieIndex {
    fn clone(&self) -> Self {
        // 从字节重建 Set - 使用 from_bytes 并收集工具名称
        let mut tool_names: Vec<String> = Vec::new();
        let mut stream = self.tool_set.stream();
        while let Some(bytes) = stream.next() {
            if let Ok(word) = std::str::from_utf8(bytes) {
                tool_names.push(word.to_string());
            }
        }
        tool_names.sort();
        let tool_set = Set::from_iter(tool_names.iter()).unwrap();

        Self {
            tool_set,
            tool_map: self.tool_map.clone(),
            keyword_index: self.keyword_index.clone(),
            pending_adds: self.pending_adds.clone(),
            config: self.config.clone(),
            needs_rebuild: self.needs_rebuild,
        }
    }
}

impl std::fmt::Debug for TrieIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrieIndex")
            .field("tool_count", &self.tool_map.len())
            .field("pending_count", &self.pending_adds.len())
            .field("keyword_count", &self.keyword_index.len())
            .field("needs_rebuild", &self.needs_rebuild)
            .finish()
    }
}

impl TrieIndex {
    /// 创建新的空 Trie 索引（使用默认配置）
    pub fn new() -> Self {
        Self::with_config(TrieIndexConfig::default())
    }

    /// 创建新的空 Trie 索引（自定义配置）
    pub fn with_config(config: TrieIndexConfig) -> Self {
        Self {
            tool_set: Set::default(),
            tool_map: HashMap::new(),
            keyword_index: HashMap::new(),
            pending_adds: Vec::new(),
            config,
            needs_rebuild: false,
        }
    }

    /// 从工具列表构建索引
    pub fn build(tools: &[(&str, u64)]) -> Result<Self, String> {
        Self::build_with_config(tools, TrieIndexConfig::default())
    }

    /// 从工具列表构建索引（自定义配置）
    pub fn build_with_config(tools: &[(&str, u64)], config: TrieIndexConfig) -> Result<Self, String> {
        let mut tool_map = HashMap::new();
        let mut keyword_index: HashMap<String, Vec<String>> = HashMap::new();
        let mut tool_names: Vec<String> = Vec::with_capacity(tools.len());

        for (name, id) in tools {
            tool_map.insert(name.to_string(), *id);
            tool_names.push(name.to_string());

            let prefixes = Self::extract_prefixes(name);
            for prefix in prefixes {
                keyword_index
                    .entry(prefix)
                    .or_default()
                    .push(name.to_string());
            }
        }

        tool_names.sort();
        tool_names.dedup();

        let tool_set = Set::from_iter(tool_names.iter())
            .map_err(|e| format!("构建 fst Set 失败：{}", e))?;

        info!(
            "Trie 索引构建完成：{} 个工具，{} 个关键词，fst 大小：{} bytes",
            tools.len(),
            keyword_index.len(),
            tool_set.as_fst().as_bytes().len()
        );

        Ok(Self {
            tool_set,
            tool_map,
            keyword_index,
            pending_adds: Vec::new(),
            config,
            needs_rebuild: false,
        })
    }

    /// 添加工具到索引（O(1) 复杂度，不立即重建 fst）
    pub fn add_tool(&mut self, name: &str, id: u64) {
        // 如果工具已存在，先更新 tool_map
        if self.tool_map.contains_key(name) {
            self.tool_map.insert(name.to_string(), id);
            debug!("更新工具索引：{} (id: {})", name, id);
            return;
        }

        // 添加到 tool_map
        self.tool_map.insert(name.to_string(), id);

        // 添加到关键词索引
        let prefixes = Self::extract_prefixes(name);
        for prefix in prefixes {
            self.keyword_index
                .entry(prefix)
                .or_default()
                .push(name.to_string());
        }

        // 添加到待持久化缓冲区（O(1)）
        self.pending_adds.push((name.to_string(), id));

        // 检查是否需要批量重建
        if self.pending_adds.len() >= self.config.rebuild_threshold {
            self.rebuild_fst();
        } else {
            self.needs_rebuild = true;
        }

        debug!("添加工具到索引：{} (id: {}), 待重建：{}/{}", 
               name, id, self.pending_adds.len(), self.config.rebuild_threshold);
    }

    /// 强制重建 fst Set（用于手动触发或关闭前）
    pub fn rebuild_fst(&mut self) {
        if self.pending_adds.is_empty() {
            return;
        }

        let rebuild_start = std::time::Instant::now();

        // 收集所有工具名称（已持久化 + 待持久化）
        let mut tool_names: Vec<String> = Vec::with_capacity(self.tool_map.len());
        for name in self.tool_map.keys() {
            tool_names.push(name.clone());
        }
        tool_names.sort();

        // 重建 fst Set
        match Set::from_iter(tool_names.iter()) {
            Ok(new_set) => {
                self.tool_set = new_set;
                self.pending_adds.clear();
                self.needs_rebuild = false;

                let elapsed = rebuild_start.elapsed();
                info!("fst Set 重建完成：{} 个工具，耗时 {:?}", 
                      self.tool_map.len(), elapsed);
            }
            Err(e) => {
                debug!("fst Set 重建失败：{}", e);
            }
        }
    }

    /// 移除工具
    pub fn remove_tool(&mut self, name: &str) -> Option<u64> {
        let id = self.tool_map.remove(name);

        if id.is_some() {
            // 从关键词索引移除
            let prefixes = Self::extract_prefixes(name);
            for prefix in prefixes {
                if let Some(tools) = self.keyword_index.get_mut(&prefix) {
                    tools.retain(|t| t != name);
                }
            }

            // 从待持久化缓冲区移除
            self.pending_adds.retain(|(n, _)| n != name);

            // 标记需要重建
            self.needs_rebuild = true;

            debug!("从索引移除工具：{}", name);
        }

        id
    }

    /// 前缀搜索（自动合并 fst Set 和待持久化缓冲区）
    pub fn search_prefix(&self, prefix: &str) -> Vec<String> {
        let mut results: Vec<String> = Vec::new();
        let prefix_lower = prefix.to_lowercase();

        // 1. 从关键词索引查找（最快，直接返回）
        if let Some(tools) = self.keyword_index.get(&prefix_lower) {
            results.extend(tools.clone());
            return results;
        }

        // 2. 使用 fst 进行前缀范围查询（仅在关键词索引未命中时）
        if prefix_lower.is_empty() {
            let mut stream = self.tool_set.stream();
            while let Some(bytes) = stream.next() {
                if let Ok(word) = std::str::from_utf8(bytes) {
                    results.push(word.to_string());
                }
            }
        } else {
            let range_end = prefix_lower.clone() + "\u{7F}";
            let range = self.tool_set.range().ge(prefix_lower.as_str()).lt(range_end.as_str());
            let mut stream = range.into_stream();
            while let Some(bytes) = stream.next() {
                if let Ok(word) = std::str::from_utf8(bytes) {
                    results.push(word.to_string());
                }
            }
        }

        // 3. 添加待持久化工具（去重）
        for (name, _) in &self.pending_adds {
            if name.to_lowercase().starts_with(&prefix_lower) && !results.iter().any(|s| s == name) {
                results.push(name.clone());
            }
        }

        debug!("前缀搜索：'{}' -> {} 个结果", prefix, results.len());
        results
    }

    /// 精确查找
    pub fn get(&self, name: &str) -> Option<u64> {
        self.tool_map.get(name).copied()
    }

    /// 检查工具是否存在
    pub fn contains(&self, name: &str) -> bool {
        self.tool_map.contains_key(name)
    }

    /// 获取所有工具名称
    pub fn get_all_names(&self) -> Vec<String> {
        let mut results: Vec<String> = Vec::new();
        let mut seen: HashMap<String, bool> = HashMap::new();

        // 从 fst Set 获取
        let mut stream = self.tool_set.stream();
        while let Some(bytes) = stream.next() {
            if let Ok(word) = std::str::from_utf8(bytes) {
                let word_str = word.to_string();
                if !seen.contains_key(&word_str) {
                    seen.insert(word_str.clone(), true);
                    results.push(word_str);
                }
            }
        }

        // 添加待持久化工具
        for (name, _) in &self.pending_adds {
            if !seen.contains_key(name) {
                seen.insert(name.clone(), true);
                results.push(name.clone());
            }
        }

        results
    }

    /// 获取索引大小（总工具数）
    pub fn len(&self) -> usize {
        self.tool_map.len()
    }

    /// 检查索引是否为空
    pub fn is_empty(&self) -> bool {
        self.tool_map.is_empty()
    }

    /// 获取待持久化工具数量
    pub fn pending_count(&self) -> usize {
        self.pending_adds.len()
    }

    /// 检查是否需要重建
    pub fn needs_rebuild(&self) -> bool {
        self.needs_rebuild
    }

    /// 获取 fst 内存占用
    pub fn fst_memory_bytes(&self) -> usize {
        self.tool_set.as_fst().as_bytes().len()
    }

    fn extract_prefixes(name: &str) -> Vec<String> {
        let mut prefixes = Vec::new();
        let name_lower = name.to_lowercase();

        let parts: Vec<&str> = name_lower
            .split(['_', '-', ' '])
            .collect();

        for (i, _) in parts.iter().enumerate() {
            if i >= 2 {
                break;
            }
            let prefix = parts[..=i].join("_");
            if !prefix.is_empty() {
                prefixes.push(prefix);
            }
        }

        prefixes.push(name_lower);
        prefixes
    }

    /// 获取索引统计信息
    pub fn stats(&self) -> TrieIndexStats {
        TrieIndexStats {
            tool_count: self.tool_map.len(),
            pending_count: self.pending_adds.len(),
            keyword_count: self.keyword_index.len(),
            fst_memory_bytes: self.fst_memory_bytes(),
            estimated_total_memory: self.estimate_memory_usage(),
            needs_rebuild: self.needs_rebuild,
        }
    }

    fn estimate_memory_usage(&self) -> usize {
        let fst_size = self.fst_memory_bytes();
        let tool_map_size = self.tool_map.len() * (32 + 8);
        let keyword_size = self.keyword_index.len() * (32 + 24);
        let avg_tools = if self.keyword_index.is_empty() {
            0
        } else {
            self.keyword_index.values().map(|v| v.len()).sum::<usize>() / self.keyword_index.len()
        };
        let keyword_values_size = self.keyword_index.len() * avg_tools * 32;
        let pending_size = self.pending_adds.len() * (32 + 8);

        fst_size + tool_map_size + keyword_size + keyword_values_size + pending_size
    }
}

impl Default for TrieIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Trie 索引统计信息
#[derive(Debug, Clone)]
pub struct TrieIndexStats {
    pub tool_count: usize,
    pub pending_count: usize,
    pub keyword_count: usize,
    pub fst_memory_bytes: usize,
    pub estimated_total_memory: usize,
    pub needs_rebuild: bool,
}

// ============================================================================
// BK-Tree 拼写纠错
// ============================================================================

#[derive(Debug, Clone)]
struct BKTreeNode {
    word: String,
    children: HashMap<u32, BKTreeNode>,
}

impl BKTreeNode {
    fn new(word: String) -> Self {
        Self { word, children: HashMap::new() }
    }
}

pub struct BKTree {
    root: Option<BKTreeNode>,
    words: HashMap<String, Vec<String>>,
}

impl BKTree {
    pub fn new() -> Self {
        Self { root: None, words: HashMap::new() }
    }

    pub fn build(words: &[(&str, Vec<String>)]) -> Self {
        let mut tree = Self::new();
        for (word, tools) in words {
            tree.add_word(word, tools.clone());
        }
        tree
    }

    pub fn add_word(&mut self, word: &str, tools: Vec<String>) {
        self.words.insert(word.to_string(), tools);
        match &mut self.root {
            None => self.root = Some(BKTreeNode::new(word.to_string())),
            Some(root) => Self::insert_node(root, word.to_string()),
        }
    }

    fn insert_node(node: &mut BKTreeNode, word: String) {
        let dist = Self::levenshtein_distance(&node.word, &word) as u32;
        if dist == 0 { return; }
        if let Some(child) = node.children.get_mut(&dist) {
            Self::insert_node(child, word);
        } else {
            node.children.insert(dist, BKTreeNode::new(word));
        }
    }

    pub fn query(&self, query: &str, max_distance: u32) -> Vec<String> {
        let mut results = Vec::new();
        if let Some(root) = &self.root {
            Self::query_node(root, query, max_distance, &mut results);
        }
        results
    }

    fn query_node(node: &BKTreeNode, query: &str, max_dist: u32, results: &mut Vec<String>) {
        let dist = Self::levenshtein_distance(&node.word, query) as u32;
        if dist <= max_dist {
            results.push(node.word.clone());
        }
        let min_d = if dist as i32 - max_dist as i32 > 0 { (dist as i32 - max_dist as i32) as u32 } else { 0 };
        let max_d = dist + max_dist;
        for (&d, child) in &node.children {
            if d >= min_d && d <= max_d {
                Self::query_node(child, query, max_dist, results);
            }
        }
    }

    pub fn get_tools(&self, word: &str) -> Option<&Vec<String>> {
        self.words.get(word)
    }

    fn levenshtein_distance(s1: &str, s2: &str) -> usize {
        let s1: Vec<char> = s1.chars().collect();
        let s2: Vec<char> = s2.chars().collect();
        let m = s1.len();
        let n = s2.len();
        if m == 0 { return n; }
        if n == 0 { return m; }
        let mut dp = vec![vec![0; n + 1]; m + 1];
        for i in 0..=m { dp[i][0] = i; }
        for j in 0..=n { dp[0][j] = j; }
        #[allow(clippy::needless_range_loop)]
        for i in 1..=m {
            for j in 1..=n {
                let cost = if s1[i-1] == s2[j-1] { 0 } else { 1 };
                dp[i][j] = (dp[i-1][j] + 1).min(dp[i][j-1] + 1).min(dp[i-1][j-1] + cost);
            }
        }
        dp[m][n]
    }

    pub fn stats(&self) -> BKTreeStats {
        BKTreeStats {
            word_count: self.words.len(),
            max_depth: Self::calc_depth(&self.root),
        }
    }

    fn calc_depth(node: &Option<BKTreeNode>) -> usize {
        match node {
            None => 0,
            Some(n) => 1 + n.children.values().map(|c| Self::calc_depth(&Some(c.clone()))).max().unwrap_or(0),
        }
    }
}

impl Default for BKTree {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Clone)]
pub struct BKTreeStats {
    pub word_count: usize,
    pub max_depth: usize,
}

// ============================================================================
// 混合索引
// ============================================================================

pub struct HybridIndex {
    trie: TrieIndex,
    bk_tree: BKTree,
    tool_map: HashMap<String, u64>,
}

impl HybridIndex {
    pub fn new() -> Self {
        Self { trie: TrieIndex::new(), bk_tree: BKTree::new(), tool_map: HashMap::new() }
    }

    pub fn build(tools: &[(&str, u64)]) -> Result<Self, String> {
        let trie = TrieIndex::build(tools)?;
        let bk_words: Vec<(&str, Vec<String>)> = tools.iter().map(|(n, _)| (*n, vec![n.to_string()])).collect();
        let bk_tree = BKTree::build(&bk_words);
        let mut tool_map = HashMap::new();
        for (name, id) in tools { tool_map.insert(name.to_string(), *id); }
        Ok(Self { trie, bk_tree, tool_map })
    }

    pub fn add_tool(&mut self, name: &str, id: u64) {
        self.trie.add_tool(name, id);
        self.bk_tree.add_word(name, vec![name.to_string()]);
        self.tool_map.insert(name.to_string(), id);
    }

    pub fn search(&self, query: &str, max_distance: u32) -> Vec<String> {
        let mut results = self.trie.search_prefix(query);
        for word in self.bk_tree.query(query, max_distance) {
            if let Some(tools) = self.bk_tree.get_tools(&word) {
                for tool in tools {
                    if !results.iter().any(|r| r == tool) { results.push(tool.clone()); }
                }
            }
        }
        if self.tool_map.contains_key(query) && !results.iter().any(|r| r == query) {
            results.push(query.to_string());
        }
        results
    }

    pub fn get(&self, name: &str) -> Option<u64> { self.tool_map.get(name).copied() }

    pub fn stats(&self) -> HybridIndexStats {
        HybridIndexStats { tool_count: self.tool_map.len(), trie_stats: self.trie.stats(), bk_tree_stats: self.bk_tree.stats() }
    }
}

impl Default for HybridIndex { fn default() -> Self { Self::new() } }

#[derive(Debug, Clone)]
pub struct HybridIndexStats {
    pub tool_count: usize,
    pub trie_stats: TrieIndexStats,
    pub bk_tree_stats: BKTreeStats,
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trie_index_build() {
        let tools = vec![("read_file", 1), ("write_file", 2), ("read_dir", 3)];
        let index = TrieIndex::build(&tools).unwrap();
        assert_eq!(index.len(), 3);
        assert_eq!(index.pending_count(), 0); // 构建后无待持久化
        assert!(index.contains("read_file"));
    }

    #[test]
    fn test_trie_index_prefix_search() {
        let tools = vec![("read_file", 1), ("write_file", 2), ("read_dir", 3)];
        let index = TrieIndex::build(&tools).unwrap();
        let results = index.search_prefix("read");
        assert!(results.iter().any(|s| s == "read_file"));
        assert!(results.iter().any(|s| s == "read_dir"));
    }

    #[test]
    fn test_trie_index_exact() {
        let tools = vec![("read_file", 1), ("write_file", 2)];
        let index = TrieIndex::build(&tools).unwrap();
        assert_eq!(index.get("read_file"), Some(1));
        assert_eq!(index.get("x"), None);
    }

    #[test]
    fn test_trie_index_add_tool_pending() {
        let mut index = TrieIndex::with_config(TrieIndexConfig { rebuild_threshold: 10 });
        index.add_tool("read_file", 1);
        index.add_tool("write_file", 2);
        
        // 工具应该能被搜到（即使在待持久化缓冲区）
        assert!(index.contains("read_file"));
        assert!(index.contains("write_file"));
        assert_eq!(index.pending_count(), 2);
        assert!(index.needs_rebuild());
        
        // 前缀搜索应该包含待持久化工具
        let results = index.search_prefix("read");
        assert!(results.iter().any(|s| s == "read_file"));
    }

    #[test]
    fn test_trie_index_batch_rebuild() {
        let mut index = TrieIndex::with_config(TrieIndexConfig { rebuild_threshold: 5 });
        
        // 添加 5 个工具，应该触发自动重建
        for i in 0..5 {
            index.add_tool(&format!("tool_{}", i), i as u64);
        }
        
        // 重建后待持久化应该为空
        assert_eq!(index.pending_count(), 0);
        assert!(!index.needs_rebuild());
        assert_eq!(index.len(), 5);
    }

    #[test]
    fn test_trie_index_manual_rebuild() {
        let mut index = TrieIndex::with_config(TrieIndexConfig { rebuild_threshold: 100 });
        index.add_tool("read_file", 1);
        index.add_tool("write_file", 2);
        
        // 手动触发重建
        index.rebuild_fst();
        
        assert_eq!(index.pending_count(), 0);
        assert!(!index.needs_rebuild());
    }

    #[test]
    fn test_trie_index_remove_tool() {
        let tools = vec![("read_file", 1), ("write_file", 2)];
        let mut index = TrieIndex::build(&tools).unwrap();
        
        let removed = index.remove_tool("read_file");
        assert_eq!(removed, Some(1));
        assert!(!index.contains("read_file"));
        assert!(index.contains("write_file"));
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn test_bk_tree() {
        let mut tree = BKTree::new();
        tree.add_word("read_file", vec!["read_file".to_string()]);
        let results = tree.query("read_fle", 2);
        assert!(results.iter().any(|s| s == "read_file"));
    }

    #[test]
    fn test_levenshtein() {
        assert_eq!(BKTree::levenshtein_distance("abc", "abc"), 0);
        assert_eq!(BKTree::levenshtein_distance("abc", "abd"), 1);
        assert_eq!(BKTree::levenshtein_distance("", "abc"), 3);
    }

    #[test]
    fn test_trie_index_stats() {
        let mut index = TrieIndex::with_config(TrieIndexConfig { rebuild_threshold: 10 });
        index.add_tool("read_file", 1);
        index.add_tool("write_file", 2);
        
        let stats = index.stats();
        assert_eq!(stats.tool_count, 2);
        assert_eq!(stats.pending_count, 2);
        assert!(stats.needs_rebuild);
    }
}
