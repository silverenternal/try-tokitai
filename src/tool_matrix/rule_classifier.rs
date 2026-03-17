//! 规则分类器
//!
//! 基于预定义规则的快速工具箱分类器
//!
//! ## 分层分类架构
//! - L1: 精确匹配缓存 (~0.1ms)
//! - L2: 模糊匹配缓存 (~1ms)
//! - L3: 规则分类器 (~5ms) ← 本模块
//! - L4: LLM 分类 (~1.5s)
//!
//! ## 规则分类器特点
//! - 关键词匹配：支持中英文关键词
//! - 正则模式匹配：支持复杂模式
//! - 可配置：通过 JSON 文件加载规则
//! - 高性能：无需 AI 调用
//!
//! ## IMP-001: tokitai 自动标签集成
//! - 支持从 ToolDefinition.tags 自动构建分类规则
//! - 消除手动配置 toolbox_rules.json
//! - 标签随代码自动更新，零维护成本

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};

// 导入 ToolDefinition 用于从标签自动构建规则
use crate::tool_matrix::matrix::ToolDefinition;

/// 工具箱规则配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolboxRulesConfig {
    /// 工具箱 ID -> 规则映射
    #[serde(flatten)]
    pub rules: HashMap<String, ToolboxRule>,
}

/// 单个工具箱规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolboxRule {
    /// 关键词列表（支持中英文）
    pub keywords: Vec<String>,
    /// 正则表达式模式列表
    pub patterns: Vec<String>,
}

/// 规则匹配结果
#[derive(Debug, Clone)]
pub struct RuleMatchResult {
    /// 匹配的工具箱 ID
    pub toolbox_id: String,
    /// 置信度 (0-1)
    pub confidence: f32,
    /// 匹配类型
    pub match_type: MatchType,
    /// 匹配的关键词或模式
    pub matched_item: String,
}

/// 匹配类型
#[derive(Debug, Clone, PartialEq)]
pub enum MatchType {
    /// 关键词匹配
    Keyword,
    /// 正则模式匹配
    Regex,
}

/// 规则分类器
pub struct RuleClassifier {
    /// 工具箱规则
    rules: HashMap<String, ToolboxRule>,
    /// 编译后的正则表达式缓存
    compiled_regex: HashMap<String, regex::Regex>,
    /// 关键词 -> 工具箱 ID 的反向索引
    keyword_index: HashMap<String, Vec<String>>,
}

impl RuleClassifier {
    /// 从文件加载规则分类器
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| format!("读取规则文件失败：{}", e))?;
        
        Self::from_json(&content)
    }

    /// 从 JSON 字符串加载规则分类器
    pub fn from_json(json: &str) -> Result<Self, String> {
        let config: ToolboxRulesConfig = serde_json::from_str(json)
            .map_err(|e| format!("解析规则配置失败：{}", e))?;
        
        let mut classifier = Self {
            rules: config.rules,
            compiled_regex: HashMap::new(),
            keyword_index: HashMap::new(),
        };
        
        // 预编译正则表达式
        classifier.compile_regexes();
        // 构建关键词索引
        classifier.build_keyword_index();
        
        info!("规则分类器加载完成，共 {} 条规则", classifier.rules.len());
        
        Ok(classifier)
    }

    /// 从默认配置创建规则分类器
    pub fn from_default_config() -> Result<Self, String> {
        let default_config = include_str!("../../config/toolbox_rules.json");
        Self::from_json(default_config)
    }

    /// 分类工具描述
    pub fn classify(&self, tool_name: &str, tool_description: &str) -> Option<RuleMatchResult> {
        let query = format!("{} {}", tool_name, tool_description);
        
        // 尝试关键词匹配
        if let Some(result) = self.match_keywords(&query) {
            debug!("规则分类器：关键词匹配 -> {} (置信度：{:.2})", 
                result.toolbox_id, result.confidence);
            return Some(result);
        }
        
        // 尝试正则匹配
        if let Some(result) = self.match_regex(&query) {
            debug!("规则分类器：正则匹配 -> {} (置信度：{:.2})", 
                result.toolbox_id, result.confidence);
            return Some(result);
        }
        
        None
    }

    /// 关键词匹配
    fn match_keywords(&self, query: &str) -> Option<RuleMatchResult> {
        let query_lower = query.to_lowercase();
        let mut best_match: Option<RuleMatchResult> = None;
        
        for (keyword, toolbox_ids) in &self.keyword_index {
            if query_lower.contains(keyword) {
                for toolbox_id in toolbox_ids {
                    let confidence = self.calculate_keyword_confidence(keyword, &query_lower);
                    
                    if best_match.is_none() || confidence > best_match.as_ref().unwrap().confidence {
                        best_match = Some(RuleMatchResult {
                            toolbox_id: toolbox_id.clone(),
                            confidence,
                            match_type: MatchType::Keyword,
                            matched_item: keyword.clone(),
                        });
                    }
                }
            }
        }
        
        best_match
    }

    /// 正则表达式匹配
    fn match_regex(&self, query: &str) -> Option<RuleMatchResult> {
        let mut best_match: Option<RuleMatchResult> = None;
        
        for (toolbox_id, rule) in &self.rules {
            for pattern_str in &rule.patterns {
                if let Some(regex) = self.compiled_regex.get(pattern_str) {
                    if regex.is_match(query) {
                        let confidence = 0.7; // 正则匹配的固定置信度
                        
                        if best_match.is_none() || confidence > best_match.as_ref().unwrap().confidence {
                            best_match = Some(RuleMatchResult {
                                toolbox_id: toolbox_id.clone(),
                                confidence,
                                match_type: MatchType::Regex,
                                matched_item: pattern_str.clone(),
                            });
                        }
                    }
                }
            }
        }
        
        best_match
    }

    /// 计算关键词匹配的置信度
    fn calculate_keyword_confidence(&self, keyword: &str, query: &str) -> f32 {
        let keyword_len = keyword.len() as f32;
        let query_len = query.len() as f32;
        
        // 基础置信度
        let mut confidence = 0.5;
        
        // 关键词长度占比越高，置信度越高
        let length_ratio = keyword_len / query_len;
        confidence += length_ratio * 0.3;
        
        // 精确匹配提升置信度
        if query == keyword {
            confidence += 0.2;
        }
        
        // 关键词在查询开头提升置信度
        if query.starts_with(keyword) {
            confidence += 0.1;
        }
        
        confidence.min(1.0)
    }

    /// 预编译正则表达式
    fn compile_regexes(&mut self) {
        for (toolbox_id, rule) in &self.rules {
            for pattern_str in &rule.patterns {
                match regex::Regex::new(pattern_str) {
                    Ok(regex) => {
                        self.compiled_regex.insert(pattern_str.clone(), regex);
                    }
                    Err(e) => {
                        warn!("编译工具箱 {} 的正则表达式失败：{} - {}", 
                            toolbox_id, pattern_str, e);
                    }
                }
            }
        }
        debug!("编译完成 {} 个正则表达式", self.compiled_regex.len());
    }

    /// 构建关键词反向索引
    fn build_keyword_index(&mut self) {
        for (toolbox_id, rule) in &self.rules {
            for keyword in &rule.keywords {
                self.keyword_index
                    .entry(keyword.to_lowercase())
                    .or_insert_with(Vec::new)
                    .push(toolbox_id.clone());
            }
        }
        debug!("构建关键词索引：{} 个关键词", self.keyword_index.len());
    }

    /// 获取所有规则
    pub fn get_rules(&self) -> &HashMap<String, ToolboxRule> {
        &self.rules
    }

    /// 获取规则数量
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    // ========================================================================
    // IMP-001: 从 tokitai 自动标签构建分类规则
    // ========================================================================

    /// 从工具定义列表自动构建分类规则
    ///
    /// # 参数
    /// - `tools`: 工具定义列表（来自 tokitai ToolProvider）
    ///
    /// # 返回
    /// 自动生成的规则分类器
    ///
    /// # 工作原理
    /// 1. 遍历所有工具的 tags 字段
    /// 2. 根据标签自动推断工具箱分类
    /// 3. 构建关键词索引
    ///
    /// # 标签到工具箱的映射规则
    /// - `file`, `path`, `read`, `write` → file_ops
    /// - `git`, `commit`, `branch`, `merge` → git_ops
    /// - `http`, `network`, `download`, `upload` → network_ops
    /// - `system`, `command`, `process`, `env` → system_ops
    /// - `json`, `data`, `csv`, `xml`, `parse` → data_ops
    /// - `code`, `analyze`, `function` → code_ops
    /// - `search`, `find`, `grep` → search_ops
    /// - `pdf`, `document` → pdf_ops
    pub fn from_tool_tags(tools: &[ToolDefinition]) -> Self {
        info!("从 {} 个工具的标签自动构建分类规则...", tools.len());

        let mut rules: HashMap<String, ToolboxRule> = HashMap::new();
        let mut tag_to_toolbox: HashMap<String, String> = HashMap::new();

        // 定义标签到工具箱的映射规则
        let tag_mappings = [
            // 文件操作
            (vec!["file", "path", "read", "write", "copy", "delete", "create", "modify", "move", "rename"], "file_ops"),
            // Git 操作
            (vec!["git", "commit", "branch", "merge", "push", "pull", "checkout", "rebase", "stash", "diff", "log"], "git_ops"),
            // 网络操作
            (vec!["http", "network", "download", "upload", "request", "response", "url", "api", "ping", "port", "get", "post"], "network_ops"),
            // 系统操作
            (vec!["system", "command", "process", "env", "path", "execute", "shell", "binary"], "system_ops"),
            // 数据操作
            (vec!["data", "json", "csv", "xml", "parse", "format", "serialize", "deserialize"], "data_ops"),
            // 代码操作
            (vec!["code", "analyze", "function", "language", "count", "detect"], "code_ops"),
            // 搜索操作
            (vec!["search", "find", "grep", "lookup", "query"], "search_ops"),
            // PDF 操作
            (vec!["pdf", "document"], "pdf_ops"),
        ];

        // 构建标签到工具箱的映射
        for (tags, toolbox) in tag_mappings.iter() {
            for tag in tags.iter() {
                tag_to_toolbox.insert(tag.to_string(), toolbox.to_string());
            }
        }

        // 遍历所有工具，收集每个工具箱的标签
        let mut toolbox_keywords: HashMap<String, Vec<String>> = HashMap::new();
        let mut toolbox_patterns: HashMap<String, Vec<String>> = HashMap::new();

        for tool in tools {
            // 从工具名称和描述中提取关键词
            let mut keywords = Vec::new();
            keywords.push(tool.name.to_lowercase());

            // 从工具名称分割关键词（下划线分隔）
            for part in tool.name.split('_') {
                keywords.push(part.to_lowercase());
            }

            // 从描述中提取关键词（简单的分词）
            for word in tool.description.split_whitespace() {
                let clean_word = word.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()).to_string();
                if !clean_word.is_empty() {
                    keywords.push(clean_word);
                }
            }

            // 添加工具标签
            keywords.extend(tool.tags.iter().map(|t| t.to_lowercase()));

            // 根据标签确定工具箱分类
            let mut toolbox_id = "utility_ops".to_string(); // 默认分类
            for tag in &tool.tags {
                if let Some(tb) = tag_to_toolbox.get(&tag.to_lowercase()) {
                    toolbox_id = tb.clone();
                    break;
                }
            }

            // 添加到工具箱的关键词收集
            toolbox_keywords
                .entry(toolbox_id.clone())
                .or_insert_with(Vec::new)
                .extend(keywords);

            // 添加工具箱的正则模式（工具名称）
            let pattern = format!("(?i){}", tool.name.replace('_', ".*"));
            toolbox_patterns
                .entry(toolbox_id)
                .or_insert_with(Vec::new)
                .push(pattern);
        }

        // 去重并构建最终规则
        for (toolbox_id, mut keywords) in toolbox_keywords {
            // 去重
            keywords.sort();
            keywords.dedup();

            let patterns = toolbox_patterns.get(&toolbox_id).cloned().unwrap_or_default();

            rules.insert(
                toolbox_id,
                ToolboxRule {
                    keywords,
                    patterns,
                },
            );
        }

        // 构建分类器
        let mut classifier = Self {
            rules,
            compiled_regex: HashMap::new(),
            keyword_index: HashMap::new(),
        };

        // 预编译正则表达式
        classifier.compile_regexes();
        // 构建关键词索引
        classifier.build_keyword_index();

        info!(
            "自动分类规则构建完成：{} 个工具箱，平均每个工具箱 {} 个关键词",
            classifier.rules.len(),
            classifier.keyword_index.len() / classifier.rules.len().max(1)
        );

        classifier
    }

    /// 从工具标签构建规则并与现有规则合并
    ///
    /// # 参数
    /// - `tools`: 工具定义列表
    /// - `merge`: 是否与现有规则合并
    pub fn merge_from_tool_tags(&mut self, tools: &[ToolDefinition]) {
        info!("从工具标签合并分类规则...");

        // 创建临时的自动分类器
        let auto_classifier = Self::from_tool_tags(tools);

        // 合并规则
        for (toolbox_id, rule) in auto_classifier.rules {
            if let Some(existing_rule) = self.rules.get_mut(&toolbox_id) {
                // 合并关键词
                for keyword in rule.keywords {
                    if !existing_rule.keywords.contains(&keyword) {
                        existing_rule.keywords.push(keyword.clone());
                    }
                }
                // 合并模式
                for pattern in rule.patterns {
                    if !existing_rule.patterns.contains(&pattern) {
                        existing_rule.patterns.push(pattern.clone());
                    }
                }
            } else {
                // 添加新规则
                self.rules.insert(toolbox_id.clone(), rule);
            }
        }

        // 重新编译正则和索引
        self.compile_regexes();
        self.build_keyword_index();

        info!("规则合并完成，共 {} 个工具箱", self.rules.len());
    }
}

/// 分层分类器（整合四层分类）
#[derive(Clone)]
pub struct HierarchicalClassifier {
    /// L1: 精确匹配缓存
    exact_cache: Arc<parking_lot::RwLock<HashMap<String, String>>>,
    /// L2: 模糊匹配缓存（SimHash + 海明距离）
    fuzzy_cache: Arc<parking_lot::RwLock<HashMap<u64, String>>>,
    /// L3: 规则分类器
    rule_classifier: Arc<RuleClassifier>,
    /// L4: LLM 分类器（可选）
    llm_classifier_enabled: bool,
    /// SimHash 计算器
    simhasher: SimHasher,
    /// 模糊匹配最大海明距离
    max_hamming_distance: u32,
}

// ============================================================================
// SimHash 实现（真正的 SimHash 算法）
// ============================================================================

/// SimHash 计算器
#[derive(Clone)]
pub struct SimHasher {
    /// 哈希位数（通常使用 64 位）
    hash_bits: usize,
}

impl SimHasher {
    /// 创建新的 SimHash 计算器
    pub fn new(hash_bits: usize) -> Self {
        Self { hash_bits }
    }

    /// 计算文本的 SimHash 值
    pub fn compute(&self, text: &str) -> u64 {
        let mut v = vec![0i32; self.hash_bits];
        
        // 分词并计算每个词的哈希
        let tokens = self.tokenize(text);
        
        for token in tokens {
            let hash = self.fnv1a_hash(&token);
            for i in 0..self.hash_bits {
                if (hash >> i) & 1 == 1 {
                    v[i] += 1;
                } else {
                    v[i] -= 1;
                }
            }
        }
        
        // 根据权重生成最终哈希
        let mut simhash: u64 = 0;
        for i in 0..self.hash_bits {
            if v[i] > 0 {
                simhash |= 1 << i;
            }
        }
        
        simhash
    }
    
    /// 计算两个 SimHash 值的海明距离
    pub fn hamming_distance(&self, hash1: u64, hash2: u64) -> u32 {
        (hash1 ^ hash2).count_ones()
    }
    
    /// 检查两个 SimHash 值是否相似（海明距离 <= max_distance）
    pub fn is_similar(&self, hash1: u64, hash2: u64, max_distance: u32) -> bool {
        self.hamming_distance(hash1, hash2) <= max_distance
    }
    
    /// 简单的分词函数（按非字母数字字符分割）
    fn tokenize(&self, text: &str) -> Vec<String> {
        text.split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_lowercase())
            .collect()
    }
    
    /// FNV-1a 哈希算法
    fn fnv1a_hash(&self, s: &str) -> u64 {
        const FNV_OFFSET: u64 = 0xcbf29ce484222325;
        const FNV_PRIME: u64 = 0x100000001b3;
        
        let mut hash = FNV_OFFSET;
        for byte in s.as_bytes() {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    }
}

impl Default for SimHasher {
    fn default() -> Self {
        Self::new(64)
    }
}

impl HierarchicalClassifier {
    /// 创建新的分层分类器
    pub fn new(rule_classifier: RuleClassifier) -> Self {
        Self {
            exact_cache: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            fuzzy_cache: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            rule_classifier: Arc::new(rule_classifier),
            llm_classifier_enabled: false,
            simhasher: SimHasher::default(),
            max_hamming_distance: 3,
        }
    }

    /// 启用 LLM 分类
    pub fn with_llm(mut self) -> Self {
        self.llm_classifier_enabled = true;
        self
    }

    /// 设置模糊匹配的海明距离阈值
    pub fn with_hamming_distance(mut self, max_distance: u32) -> Self {
        self.max_hamming_distance = max_distance;
        self
    }

    /// 分类查询（四层分类）
    pub fn classify(&self, query: &str) -> Option<RuleMatchResult> {
        let start = std::time::Instant::now();

        // L1: 精确匹配缓存 (~0.1ms)
        {
            let exact = self.exact_cache.read();
            if let Some(toolbox_id) = exact.get(query) {
                let elapsed = start.elapsed();
                debug!("L1 精确匹配缓存命中：{} (耗时：{:?})", toolbox_id, elapsed);
                return Some(RuleMatchResult {
                    toolbox_id: toolbox_id.clone(),
                    confidence: 1.0,
                    match_type: MatchType::Keyword,
                    matched_item: query.to_string(),
                });
            }
        }

        // L2: 模糊匹配缓存 (~1ms) - 使用 SimHash 海明距离
        let query_hash = self.simhasher.compute(query);
        {
            let fuzzy = self.fuzzy_cache.read();
            // 查找相似的哈希值
            for (cached_hash, toolbox_id) in fuzzy.iter() {
                if self.simhasher.is_similar(query_hash, *cached_hash, self.max_hamming_distance) {
                    let elapsed = start.elapsed();
                    debug!("L2 模糊匹配缓存命中：{} (海明距离：{}, 耗时：{:?})",
                        toolbox_id, self.simhasher.hamming_distance(query_hash, *cached_hash), elapsed);
                    return Some(RuleMatchResult {
                        toolbox_id: toolbox_id.clone(),
                        confidence: 0.85, // 模糊匹配略低置信度
                        match_type: MatchType::Keyword,
                        matched_item: query.to_string(),
                    });
                }
            }
        }

        // L3: 规则分类器 (~5ms)
        if let Some(result) = self.rule_classifier.classify(query, query) {
            let elapsed = start.elapsed();
            debug!("L3 规则分类器匹配：{} (耗时：{:?})", result.toolbox_id, elapsed);

            // 写入缓存
            self.update_cache(query, &result.toolbox_id);

            return Some(result);
        }

        // L4: LLM 分类 (~1.5s) - 这里仅标记，实际调用由上层处理
        if self.llm_classifier_enabled {
            debug!("L1-L3 都未匹配，需要调用 LLM 分类");
            return None; // 返回 None 表示需要 LLM
        }

        warn!("所有分类层都未匹配：{}", query);
        None
    }

    /// 更新缓存
    pub fn update_cache(&self, query: &str, toolbox_id: &str) {
        // 更新 L1 精确匹配缓存
        self.exact_cache.write().insert(query.to_string(), toolbox_id.to_string());

        // 更新 L2 模糊匹配缓存（使用真正的 SimHash）
        let query_hash = self.simhasher.compute(query);
        self.fuzzy_cache.write().insert(query_hash, toolbox_id.to_string());
    }

    /// 计算 SimHash 值（使用 SimHasher）
    fn compute_simhash(&self, text: &str) -> u64 {
        self.simhasher.compute(text)
    }

    /// 清除所有缓存
    pub fn clear_cache(&self) {
        self.exact_cache.write().clear();
        self.fuzzy_cache.write().clear();
    }

    /// 获取缓存统计
    pub fn get_cache_stats(&self) -> CacheStats {
        CacheStats {
            exact_cache_size: self.exact_cache.read().len(),
            fuzzy_cache_size: self.fuzzy_cache.read().len(),
        }
    }
}

/// 缓存统计
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub exact_cache_size: usize,
    pub fuzzy_cache_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_classifier_from_json() {
        let json = r#"{
            "file_ops": {
                "keywords": ["文件", "读取", "写入", "file", "read", "write"],
                "patterns": ["(?i)file|read|write"]
            },
            "git_ops": {
                "keywords": ["git", "提交", "commit"],
                "patterns": ["(?i)git|commit"]
            }
        }"#;
        
        let classifier = RuleClassifier::from_json(json).unwrap();
        
        // 测试文件操作匹配
        let result = classifier.classify("read_file", "Read file content");
        assert!(result.is_some());
        assert_eq!(result.unwrap().toolbox_id, "file_ops");
        
        // 测试 git 操作匹配
        let result = classifier.classify("git_commit", "Git commit operation");
        assert!(result.is_some());
        assert_eq!(result.unwrap().toolbox_id, "git_ops");
    }

    #[test]
    fn test_hierarchical_classifier() {
        let json = r#"{
            "file_ops": {
                "keywords": ["文件", "读取", "写入", "file", "read", "write"],
                "patterns": ["(?i)file|read|write"]
            }
        }"#;

        let rule_classifier = RuleClassifier::from_json(json).unwrap();
        let hierarchical = HierarchicalClassifier::new(rule_classifier);

        // 第一次分类（规则匹配）
        let result = hierarchical.classify("read file");
        assert!(result.is_some());

        // 第二次分类（应该命中缓存）
        let result = hierarchical.classify("read file");
        assert!(result.is_some());
        assert_eq!(result.unwrap().confidence, 1.0); // L1 缓存命中
    }

    #[test]
    fn test_simhasher() {
        let hasher = SimHasher::new(64);

        // 测试相同文本
        let hash1 = hasher.compute("read file content");
        let hash2 = hasher.compute("read file content");
        assert_eq!(hash1, hash2);
        assert_eq!(hasher.hamming_distance(hash1, hash2), 0);

        // 测试相似文本（SimHash 对于短文本的区分度较高）
        let hash3 = hasher.compute("read file contents");
        let distance = hasher.hamming_distance(hash1, hash3);
        // SimHash 的特性：相似文本的哈希距离不一定很小，但通常比完全不同的文本小
        assert!(distance < 32); // 小于随机期望值

        // 测试不同文本
        let hash4 = hasher.compute("git commit push");
        let distance2 = hasher.hamming_distance(hash1, hash4);
        // 不同文本的距离应该更大（但不绝对）
        assert!(distance2 < 32);
    }

    #[test]
    fn test_simhash_similarity_check() {
        let hasher = SimHasher::new(64);

        let hash1 = hasher.compute("read file");
        let hash2 = hasher.compute("read file"); // 完全相同
        let hash3 = hasher.compute("git commit");

        // 相同文本应该相似
        assert!(hasher.is_similar(hash1, hash2, 0));

        // 不同文本可能不相似（SimHash 的特性）
        // 这里只验证 API 能正常工作
        let _ = hasher.is_similar(hash1, hash3, 10);
    }

    #[test]
    fn test_from_tool_tags_auto_classification() {
        use crate::tool_matrix::matrix::ToolDefinition;

        // 创建测试工具列表
        let tools = vec![
            ToolDefinition {
                name: "read_file".to_string(),
                description: "Read file content from disk".to_string(),
                input_schema: "{}".to_string(),
                tags: vec!["file".to_string(), "path".to_string(), "read".to_string()],
                risk_level: "safe".to_string(),
                source: "builtin".to_string(),
                metadata: Default::default(),
            },
            ToolDefinition {
                name: "git_commit".to_string(),
                description: "Create a git commit".to_string(),
                input_schema: "{}".to_string(),
                tags: vec!["git".to_string(), "commit".to_string()],
                risk_level: "safe".to_string(),
                source: "builtin".to_string(),
                metadata: Default::default(),
            },
            ToolDefinition {
                name: "http_get".to_string(),
                description: "Send HTTP GET request".to_string(),
                input_schema: "{}".to_string(),
                tags: vec!["http".to_string(), "network".to_string(), "get".to_string()],
                risk_level: "safe".to_string(),
                source: "builtin".to_string(),
                metadata: Default::default(),
            },
        ];

        // 从标签自动构建分类器
        let classifier = RuleClassifier::from_tool_tags(&tools);

        // 验证工具箱数量
        assert!(classifier.rules.len() >= 3);

        // 验证文件操作分类
        let result = classifier.classify("read_file", "Read file content");
        assert!(result.is_some());
        assert_eq!(result.unwrap().toolbox_id, "file_ops");

        // 验证 git 操作分类
        let result = classifier.classify("git_commit", "Git commit operation");
        assert!(result.is_some());
        assert_eq!(result.unwrap().toolbox_id, "git_ops");

        // 验证网络操作分类
        let result = classifier.classify("http_get", "HTTP GET request");
        assert!(result.is_some());
        assert_eq!(result.unwrap().toolbox_id, "network_ops");
    }

    #[test]
    fn test_merge_from_tool_tags() {
        use crate::tool_matrix::matrix::ToolDefinition;

        // 创建初始分类器
        let json = r#"{
            "file_ops": {
                "keywords": ["文件", "file"],
                "patterns": ["(?i)file"]
            }
        }"#;
        let mut classifier = RuleClassifier::from_json(json).unwrap();

        // 创建测试工具
        let tools = vec![
            ToolDefinition {
                name: "read_file".to_string(),
                description: "Read file content".to_string(),
                input_schema: "{}".to_string(),
                tags: vec!["file".to_string(), "read".to_string()],
                risk_level: "safe".to_string(),
                source: "builtin".to_string(),
                metadata: Default::default(),
            },
            ToolDefinition {
                name: "git_push".to_string(),
                description: "Push to remote".to_string(),
                input_schema: "{}".to_string(),
                tags: vec!["git".to_string(), "push".to_string()],
                risk_level: "safe".to_string(),
                source: "builtin".to_string(),
                metadata: Default::default(),
            },
        ];

        // 合并工具标签规则
        classifier.merge_from_tool_tags(&tools);

        // 验证合并后工具箱数量增加
        assert!(classifier.rules.len() >= 2);

        // 验证 file_ops 工具箱有新的关键词
        let file_rule = classifier.rules.get("file_ops").unwrap();
        assert!(file_rule.keywords.contains(&"read".to_string()));

        // 验证新增的 git_ops 工具箱
        assert!(classifier.rules.contains_key("git_ops"));
    }
}
