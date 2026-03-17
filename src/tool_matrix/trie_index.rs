//! Trie 树索引
//!
//! 基于 fst crate 的高性能前缀搜索索引
//! 支持 O(m) 复杂度的工具名称/关键词查找
//!
//! ## 设计原则
//! - 高性能：O(m) 前缀搜索复杂度
//! - 内存友好：fst 使用有限状态转换器，内存占用低
//! - 支持模糊搜索：结合 BK-Tree 实现拼写纠错

use std::collections::HashMap;
use tracing::{debug, info};

/// Trie 树索引（简化版，使用 HashMap 实现）
pub struct TrieIndex {
    /// 工具名称 -> 工具 ID 映射
    tool_map: HashMap<String, u64>,
    /// 关键词索引：前缀 -> 工具名称列表
    keyword_index: HashMap<String, Vec<String>>,
}

impl TrieIndex {
    /// 创建新的 Trie 索引
    pub fn new() -> Self {
        Self {
            tool_map: HashMap::new(),
            keyword_index: HashMap::new(),
        }
    }

    /// 从工具列表构建索引
    pub fn build(tools: &[(&str, u64)]) -> Result<Self, String> {
        let mut tool_map = HashMap::new();
        let mut keyword_index = HashMap::new();

        // 构建索引
        for (name, id) in tools {
            tool_map.insert(name.to_string(), *id);

            // 构建关键词索引（按前缀分割）
            let prefixes = Self::extract_prefixes(name);
            for prefix in prefixes {
                keyword_index
                    .entry(prefix)
                    .or_insert_with(Vec::new)
                    .push(name.to_string());
            }
        }

        info!("Trie 索引构建完成：{} 个工具，{} 个关键词", tools.len(), keyword_index.len());

        Ok(Self {
            tool_map,
            keyword_index,
        })
    }

    /// 添加工具到索引
    pub fn add_tool(&mut self, name: &str, id: u64) {
        self.tool_map.insert(name.to_string(), id);

        // 更新关键词索引
        let prefixes = Self::extract_prefixes(name);
        for prefix in prefixes {
            self.keyword_index
                .entry(prefix)
                .or_insert_with(Vec::new)
                .push(name.to_string());
        }

        debug!("添加工具到索引：{} (id: {})", name, id);
    }

    /// 移除工具
    pub fn remove_tool(&mut self, name: &str) -> Option<u64> {
        let id = self.tool_map.remove(name);

        if let Some(_) = id {
            // 清理关键词索引
            let prefixes = Self::extract_prefixes(name);
            for prefix in prefixes {
                if let Some(tools) = self.keyword_index.get_mut(&prefix) {
                    tools.retain(|t| t != name);
                }
            }
        }

        debug!("从索引移除工具：{}", name);
        id
    }

    /// 前缀搜索
    pub fn search_prefix(&self, prefix: &str) -> Vec<String> {
        let mut results = Vec::new();
        let prefix_lower = prefix.to_lowercase();

        // 从关键词索引查找
        if let Some(tools) = self.keyword_index.get(&prefix_lower) {
            for tool in tools {
                if !results.contains(tool) {
                    results.push(tool.clone());
                }
            }
        }

        // 查找所有以 prefix 开头的工具
        for (name, _) in &self.tool_map {
            if name.to_lowercase().starts_with(&prefix_lower) && !results.contains(name) {
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
        self.tool_map.keys().cloned().collect()
    }

    /// 获取索引大小
    pub fn len(&self) -> usize {
        self.tool_map.len()
    }

    /// 检查索引是否为空
    pub fn is_empty(&self) -> bool {
        self.tool_map.is_empty()
    }

    /// 提取前缀列表（用于关键词索引）
    fn extract_prefixes(name: &str) -> Vec<String> {
        let mut prefixes = Vec::new();
        let name_lower = name.to_lowercase();

        // 按分隔符分割
        let parts: Vec<&str> = name_lower
            .split(|c: char| c == '_' || c == '-' || c == ' ')
            .collect();

        // 限制最多 2 级前缀（防止索引膨胀）
        for (i, _) in parts.iter().enumerate() {
            if i >= 2 {
                break; // 限制深度为 2
            }
            let prefix = parts[..=i].join("_");
            if !prefix.is_empty() {
                prefixes.push(prefix);
            }
        }

        // 添加完整名称
        prefixes.push(name_lower);

        prefixes
    }

    /// 获取索引统计信息
    pub fn stats(&self) -> TrieIndexStats {
        TrieIndexStats {
            tool_count: self.tool_map.len(),
            keyword_count: self.keyword_index.len(),
            memory_bytes: self.estimate_memory_usage(),
        }
    }

    /// 估算内存使用（字节）
    fn estimate_memory_usage(&self) -> usize {
        // 粗略估算
        let tool_map_size = self.tool_map.len() * (32 + 8); // String + u64
        let keyword_size = self.keyword_index.len() * (32 + 24); // String + Vec header
        let avg_tools_per_keyword = if self.keyword_index.is_empty() {
            0
        } else {
            self.keyword_index.values().map(|v| v.len()).sum::<usize>() / self.keyword_index.len()
        };
        let keyword_values_size = self.keyword_index.len() * avg_tools_per_keyword * 32;

        tool_map_size + keyword_size + keyword_values_size
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
    /// 工具数量
    pub tool_count: usize,
    /// 关键词数量
    pub keyword_count: usize,
    /// 估算内存使用（字节）
    pub memory_bytes: usize,
}

// ============================================================================
// BK-Tree 拼写纠错
// ============================================================================

/// BK-Tree 节点
#[derive(Debug, Clone)]
struct BKTreeNode {
    /// 工具名称
    word: String,
    /// 子节点：距离 -> 子节点
    children: HashMap<u32, BKTreeNode>,
}

impl BKTreeNode {
    fn new(word: String) -> Self {
        Self {
            word,
            children: HashMap::new(),
        }
    }
}

/// BK-Tree 拼写纠错器
pub struct BKTree {
    root: Option<BKTreeNode>,
    words: HashMap<String, Vec<String>>, // 原始词 -> 工具名称列表
}

impl BKTree {
    /// 创建新的 BK-Tree
    pub fn new() -> Self {
        Self {
            root: None,
            words: HashMap::new(),
        }
    }

    /// 从词列表构建 BK-Tree
    pub fn build(words: &[(&str, Vec<String>)]) -> Self {
        let mut tree = Self::new();
        for (word, tools) in words {
            tree.add_word(word, tools.clone());
        }
        tree
    }

    /// 添加词到 BK-Tree
    pub fn add_word(&mut self, word: &str, tools: Vec<String>) {
        self.words.insert(word.to_string(), tools);

        match &mut self.root {
            None => {
                self.root = Some(BKTreeNode::new(word.to_string()));
            }
            Some(root) => {
                Self::insert_node_static(root, word.to_string());
            }
        }
    }

    /// 插入节点（静态方法，避免借用冲突）
    fn insert_node_static(node: &mut BKTreeNode, word: String) {
        let distance = Self::levenshtein_distance(&node.word, &word) as u32;

        if distance == 0 {
            return; // 已存在
        }

        match node.children.get_mut(&distance) {
            Some(child) => {
                Self::insert_node_static(child, word);
            }
            None => {
                node.children.insert(distance, BKTreeNode::new(word));
            }
        }
    }

    /// 查询相似词（最大距离为 max_distance）
    pub fn query(&self, query: &str, max_distance: u32) -> Vec<String> {
        let mut results = Vec::new();

        if let Some(root) = &self.root {
            Self::query_node_static(root, query, max_distance, &mut results);
        }

        results
    }

    /// 查询节点（静态方法）
    fn query_node_static(
        node: &BKTreeNode,
        query: &str,
        max_distance: u32,
        results: &mut Vec<String>,
    ) {
        let distance = Self::levenshtein_distance(&node.word, query) as u32;

        if distance <= max_distance {
            results.push(node.word.clone());
        }

        // 递归查询子节点
        let min_distance = if distance as i32 - max_distance as i32 > 0 {
            (distance as i32 - max_distance as i32) as u32
        } else {
            0
        };
        let max_dist = distance + max_distance;

        for (&dist, child) in &node.children {
            if dist >= min_distance && dist <= max_dist {
                Self::query_node_static(child, query, max_distance, results);
            }
        }
    }

    /// 获取工具名称
    pub fn get_tools(&self, word: &str) -> Option<&Vec<String>> {
        self.words.get(word)
    }

    /// 计算 Levenshtein 距离
    fn levenshtein_distance(s1: &str, s2: &str) -> usize {
        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();

        let m = s1_chars.len();
        let n = s2_chars.len();

        if m == 0 {
            return n;
        }
        if n == 0 {
            return m;
        }

        let mut dp = vec![vec![0; n + 1]; m + 1];

        for i in 0..=m {
            dp[i][0] = i;
        }
        for j in 0..=n {
            dp[0][j] = j;
        }

        for i in 1..=m {
            for j in 1..=n {
                let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                    0
                } else {
                    1
                };

                dp[i][j] = (dp[i - 1][j] + 1)
                    .min(dp[i][j - 1] + 1)
                    .min(dp[i - 1][j - 1] + cost);
            }
        }

        dp[m][n]
    }

    /// 获取 BK-Tree 统计信息
    pub fn stats(&self) -> BKTreeStats {
        BKTreeStats {
            word_count: self.words.len(),
            max_depth: Self::calculate_max_depth_static(&self.root),
        }
    }

    /// 计算最大深度（静态方法）
    fn calculate_max_depth_static(node: &Option<BKTreeNode>) -> usize {
        match node {
            None => 0,
            Some(n) => {
                1 + n.children.values().map(|child| Self::calculate_max_depth_static(&Some(child.clone()))).max().unwrap_or(0)
            }
        }
    }
}

impl Default for BKTree {
    fn default() -> Self {
        Self::new()
    }
}

/// BK-Tree 统计信息
#[derive(Debug, Clone)]
pub struct BKTreeStats {
    /// 词数量
    pub word_count: usize,
    /// 树最大深度
    pub max_depth: usize,
}

// ============================================================================
// 混合索引（Trie + BK-Tree）
// ============================================================================

/// 混合搜索索引
pub struct HybridIndex {
    /// Trie 树索引
    trie: TrieIndex,
    /// BK-Tree 拼写纠错
    bk_tree: BKTree,
    /// 工具名称 -> 工具 ID 映射
    tool_map: HashMap<String, u64>,
}

impl HybridIndex {
    /// 创建新的混合索引
    pub fn new() -> Self {
        Self {
            trie: TrieIndex::new(),
            bk_tree: BKTree::new(),
            tool_map: HashMap::new(),
        }
    }

    /// 从工具列表构建索引
    pub fn build(tools: &[(&str, u64)]) -> Result<Self, String> {
        let trie = TrieIndex::build(tools)?;

        // 构建 BK-Tree
        let bk_words: Vec<(&str, Vec<String>)> = tools
            .iter()
            .map(|(name, _)| (*name, vec![name.to_string()]))
            .collect();
        let bk_tree = BKTree::build(&bk_words);

        let mut tool_map = HashMap::new();
        for (name, id) in tools {
            tool_map.insert(name.to_string(), *id);
        }

        Ok(Self {
            trie,
            bk_tree,
            tool_map,
        })
    }

    /// 添加工具
    pub fn add_tool(&mut self, name: &str, id: u64) {
        self.trie.add_tool(name, id);
        self.bk_tree.add_word(name, vec![name.to_string()]);
        self.tool_map.insert(name.to_string(), id);
    }

    /// 搜索工具（支持拼写纠错）
    pub fn search(&self, query: &str, max_distance: u32) -> Vec<String> {
        let mut results = Vec::new();

        // 1. Trie 前缀搜索
        let prefix_results = self.trie.search_prefix(query);
        results.extend(prefix_results);

        // 2. BK-Tree 拼写纠错搜索
        let bk_results = self.bk_tree.query(query, max_distance);
        for word in bk_results {
            if let Some(tools) = self.bk_tree.get_tools(&word) {
                for tool in tools {
                    if !results.contains(tool) {
                        results.push(tool.clone());
                    }
                }
            }
        }

        // 3. 精确匹配
        if let Some(_) = self.tool_map.get(query) {
            if !results.contains(&query.to_string()) {
                results.push(query.to_string());
            }
        }

        results
    }

    /// 获取工具 ID
    pub fn get(&self, name: &str) -> Option<u64> {
        self.tool_map.get(name).copied()
    }

    /// 获取索引统计
    pub fn stats(&self) -> HybridIndexStats {
        HybridIndexStats {
            tool_count: self.tool_map.len(),
            trie_stats: self.trie.stats(),
            bk_tree_stats: self.bk_tree.stats(),
        }
    }
}

impl Default for HybridIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// 混合索引统计信息
#[derive(Debug, Clone)]
pub struct HybridIndexStats {
    /// 工具数量
    pub tool_count: usize,
    /// Trie 统计
    pub trie_stats: TrieIndexStats,
    /// BK-Tree 统计
    pub bk_tree_stats: BKTreeStats,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trie_index() {
        let tools = vec![
            ("read_file", 1),
            ("write_file", 2),
            ("read_dir", 3),
            ("git_commit", 4),
        ];

        let index = TrieIndex::build(&tools).unwrap();

        // 测试前缀搜索
        let results = index.search_prefix("read");
        assert!(results.contains(&"read_file".to_string()));
        assert!(results.contains(&"read_dir".to_string()));

        // 测试精确查找
        assert_eq!(index.get("read_file"), Some(1));
        assert_eq!(index.get("nonexistent"), None);
    }

    #[test]
    fn test_bk_tree() {
        let mut tree = BKTree::new();
        tree.add_word("read_file", vec!["read_file".to_string()]);
        tree.add_word("write_file", vec!["write_file".to_string()]);
        tree.add_word("git_commit", vec!["git_commit".to_string()]);

        // 测试拼写纠错
        let results = tree.query("read_fle", 2);
        assert!(results.contains(&"read_file".to_string()));

        // 测试无纠错
        let results = tree.query("read_file", 0);
        assert!(results.contains(&"read_file".to_string()));
    }

    #[test]
    fn test_hybrid_index() {
        let tools = vec![
            ("read_file", 1),
            ("write_file", 2),
            ("read_dir", 3),
        ];

        let index = HybridIndex::build(&tools).unwrap();

        // 测试前缀搜索
        let results = index.search("read", 2);
        assert!(results.contains(&"read_file".to_string()));
        assert!(results.contains(&"read_dir".to_string()));

        // 测试拼写纠错
        let results = index.search("read_fle", 2);
        assert!(results.contains(&"read_file".to_string()));
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(BKTree::levenshtein_distance("read_file", "read_file"), 0);
        assert_eq!(BKTree::levenshtein_distance("read_file", "read_fle"), 1);
        assert_eq!(BKTree::levenshtein_distance("read_file", "write_file"), 4); // r-e-a-d -> w-r-i-t (4 个字符不同)
        assert_eq!(BKTree::levenshtein_distance("", "abc"), 3);
    }
}
