//! 查询增强器
//!
//! 增强工具选择器的查询理解能力，支持：
//! - 同义词映射
//! - 意图识别
//! - 工具别名
//! - 拼写纠错
//!
//! ## 设计原则
//! - 轻量化：不引入重型 NLP 库

#![allow(dead_code)]
//! - 可配置：通过 JSON 文件加载配置
//! - 高性能：查询扩展在毫秒级完成

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::{debug, info, warn};

/// 同义词配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynonymsConfig {
    /// 同义词映射：主词 -> 同义词列表
    #[serde(flatten)]
    pub synonyms: HashMap<String, Vec<String>>,
}

/// 意图模式配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentPatternsConfig {
    /// 意图类型 -> 模式配置
    #[serde(flatten)]
    pub intents: HashMap<String, IntentPattern>,
}

/// 单个意图模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentPattern {
    /// 意图描述
    pub description: String,
    /// 正则模式列表
    pub patterns: Vec<String>,
    /// 处理动作
    pub action: String,
}

/// 查询增强器
pub struct QueryEnhancer {
    /// 同义词映射（反向索引：同义词 -> 主词）
    synonym_map: HashMap<String, Vec<String>>,
    /// 意图模式（编译后的正则）
    intent_patterns: HashMap<String, Vec<regex::Regex>>,
    /// 工具别名映射
    tool_aliases: HashMap<String, String>,
    /// 常见拼写错误映射
    spelling_corrections: HashMap<String, String>,
}

/// 增强后的查询
#[derive(Debug, Clone)]
pub struct EnhancedQuery {
    /// 原始查询
    pub original: String,
    /// 标准化后的查询
    pub normalized: String,
    /// 识别的意图类型
    pub intent: Option<String>,
    /// 同义词扩展列表
    pub synonym_expansions: Vec<String>,
    /// 拼写纠错标记
    pub corrected: bool,
}

/// 意图识别结果
#[derive(Debug, Clone)]
pub struct IntentRecognition {
    /// 意图类型
    pub intent_type: String,
    /// 置信度 (0-1)
    pub confidence: f32,
    /// 提取的目标内容
    pub extracted: Option<String>,
    /// 处理动作
    pub action: String,
}

impl QueryEnhancer {
    /// 创建新的查询增强器
    pub fn new() -> Self {
        Self {
            synonym_map: HashMap::new(),
            intent_patterns: HashMap::new(),
            tool_aliases: HashMap::new(),
            spelling_corrections: HashMap::new(),
        }
    }

    /// 从文件加载同义词配置
    pub fn load_synonyms<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        let content =
            fs::read_to_string(path.as_ref()).map_err(|e| format!("读取同义词文件失败：{}", e))?;

        let config: SynonymsConfig =
            serde_json::from_str(&content).map_err(|e| format!("解析同义词配置失败：{}", e))?;

        // 构建反向索引：每个同义词 -> 主词
        let synonyms_count = config.synonyms.len();
        for (main_term, synonyms) in config.synonyms {
            for synonym in synonyms {
                self.synonym_map
                    .entry(synonym.to_lowercase())
                    .or_default()
                    .push(main_term.clone());
            }
            // 主词本身也映射到自己
            self.synonym_map
                .entry(main_term.to_lowercase())
                .or_default()
                .push(main_term.clone());
        }

        info!("加载同义词配置：{} 个主词", synonyms_count);

        Ok(())
    }

    /// 从文件加载意图模式配置
    pub fn load_intent_patterns<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| format!("读取意图模式文件失败：{}", e))?;

        let config: IntentPatternsConfig =
            serde_json::from_str(&content).map_err(|e| format!("解析意图模式配置失败：{}", e))?;

        // 编译正则表达式
        let intents_count = config.intents.len();
        for (intent_type, pattern_config) in config.intents {
            let mut compiled_patterns = Vec::new();
            for pattern_str in &pattern_config.patterns {
                match regex::Regex::new(pattern_str) {
                    Ok(regex) => compiled_patterns.push(regex),
                    Err(e) => {
                        warn!(
                            "编译意图 {} 的正则表达式失败：{} - {}",
                            intent_type, pattern_str, e
                        );
                    }
                }
            }
            self.intent_patterns.insert(intent_type, compiled_patterns);
        }

        info!("加载意图模式配置：{} 个意图类型", intents_count);

        Ok(())
    }

    /// 从默认配置加载
    pub fn from_default_config() -> Result<Self, String> {
        let mut enhancer = Self::new();

        // 加载默认同义词
        let synonyms_json = include_str!("../../config/tool_synonyms.json");
        let config: SynonymsConfig = serde_json::from_str(synonyms_json)
            .map_err(|e| format!("解析默认同义词配置失败：{}", e))?;

        for (main_term, synonyms) in config.synonyms {
            for synonym in synonyms {
                enhancer
                    .synonym_map
                    .entry(synonym.to_lowercase())
                    .or_default()
                    .push(main_term.clone());
            }
            enhancer
                .synonym_map
                .entry(main_term.to_lowercase())
                .or_default()
                .push(main_term.clone());
        }

        // 加载默认意图模式
        let intents_json = include_str!("../../config/intent_patterns.json");
        let intent_config: IntentPatternsConfig = serde_json::from_str(intents_json)
            .map_err(|e| format!("解析默认意图配置失败：{}", e))?;

        for (intent_type, pattern_config) in intent_config.intents {
            let mut compiled_patterns = Vec::new();
            for pattern_str in &pattern_config.patterns {
                if let Ok(regex) = regex::Regex::new(pattern_str) {
                    compiled_patterns.push(regex);
                }
            }
            enhancer
                .intent_patterns
                .insert(intent_type, compiled_patterns);
        }

        // 内置拼写纠错
        enhancer.spelling_corrections = Self::build_default_spelling_corrections();

        // 内置工具别名
        enhancer.tool_aliases = Self::build_default_tool_aliases();

        Ok(enhancer)
    }

    /// 增强查询
    pub fn enhance(&self, query: &str) -> EnhancedQuery {
        let original = query.to_string();

        // 1. 拼写纠错
        let (corrected_query, corrected) = self.correct_spelling(query);

        // 2. 意图识别
        let intent = self.recognize_intent(&corrected_query);

        // 3. 同义词扩展
        let synonym_expansions = self.expand_synonyms(&corrected_query);

        // 4. 标准化（去除疑问词等）
        let normalized = self.normalize_query(&corrected_query, intent.as_ref());

        debug!(
            "查询增强：原始=\"{}\" -> 标准化=\"{}\", 意图={:?}, 扩展={} 个",
            original,
            normalized,
            intent.as_ref().map(|i| &i.intent_type),
            synonym_expansions.len()
        );

        EnhancedQuery {
            original,
            normalized,
            intent: intent.map(|i| i.intent_type),
            synonym_expansions,
            corrected,
        }
    }

    /// 拼写纠错
    fn correct_spelling(&self, query: &str) -> (String, bool) {
        let query_lower = query.to_lowercase();

        // 检查常见拼写错误
        if let Some(correction) = self.spelling_corrections.get(&query_lower) {
            debug!("拼写纠错：{} -> {}", query_lower, correction);
            return (correction.clone(), true);
        }

        // 检查子串拼写错误
        let mut corrected = query_lower.clone();
        let mut has_correction = false;

        for (wrong, right) in &self.spelling_corrections {
            if corrected.contains(wrong) {
                corrected = corrected.replace(wrong, right);
                has_correction = true;
            }
        }

        (corrected, has_correction)
    }

    /// 意图识别
    fn recognize_intent(&self, query: &str) -> Option<IntentRecognition> {
        for (intent_type, patterns) in &self.intent_patterns {
            for (idx, pattern) in patterns.iter().enumerate() {
                if let Some(captures) = pattern.captures(query) {
                    // 提取匹配内容
                    let extracted = if captures.len() > 1 {
                        Some(
                            captures
                                .get(1)
                                .map(|m| m.as_str())
                                .unwrap_or("")
                                .to_string(),
                        )
                    } else {
                        None
                    };

                    // 置信度：第一个模式匹配最高
                    let confidence = 1.0 - (idx as f32 * 0.1);

                    debug!("意图识别：{} (置信度：{:.2})", intent_type, confidence);

                    return Some(IntentRecognition {
                        intent_type: intent_type.clone(),
                        confidence,
                        extracted,
                        action: intent_type.clone(), // 使用意图类型作为 action
                    });
                }
            }
        }

        None
    }

    /// 同义词扩展
    pub fn expand_synonyms(&self, query: &str) -> Vec<String> {
        let query_lower = query.to_lowercase();
        let mut expansions = Vec::new();

        // 分词（使用中文分词 + 空格分割）
        let words = self.tokenize_query(&query_lower);

        for word in words {
            if let Some(main_terms) = self.synonym_map.get(&word) {
                for term in main_terms {
                    if term.to_lowercase() != word {
                        expansions.push(term.clone());
                    }
                }
            }
        }

        expansions
    }

    /// 分词（支持中文和英文）
    fn tokenize_query(&self, query: &str) -> Vec<String> {
        // 首先尝试使用 jieba 分词（如果有中文）
        if !query.is_ascii() {
            // 使用 jieba 分词
            use jieba_rs::Jieba;
            use std::sync::OnceLock;

            static JIEBA: OnceLock<Jieba> = OnceLock::new();
            let jieba = JIEBA.get_or_init(Jieba::new);

            return jieba
                .tokenize(query, jieba_rs::TokenizeMode::Search, false)
                .iter()
                .map(|t| t.word.to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        // 纯英文查询使用简单分词
        query
            .split(|c: char| c.is_whitespace() || c == '_' || c == '-' || c == '/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    }

    /// 标准化查询（去除疑问词等）
    fn normalize_query(&self, query: &str, intent: Option<&IntentRecognition>) -> String {
        let mut normalized = query.to_string();

        if let Some(intent_rec) = intent {
            match intent_rec.action.as_str() {
                "remove_question_words" => {
                    // 去除疑问词
                    let question_words = [
                        "怎么",
                        "如何",
                        "怎样",
                        "为什么",
                        "什么",
                        "哪个",
                        "how to",
                        "what is",
                    ];
                    for qw in question_words {
                        normalized = normalized.replace(qw, "");
                    }
                }
                "extract_target" | "extract_action" => {
                    // 使用提取的内容
                    if let Some(extracted) = &intent_rec.extracted {
                        normalized = extracted.clone();
                    }
                }
                _ => {}
            }
        }

        // 清理多余空格
        normalized.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// 添加工具别名
    pub fn add_tool_alias(&mut self, alias: &str, tool_name: &str) {
        self.tool_aliases
            .insert(alias.to_lowercase(), tool_name.to_string());
    }

    /// 解析工具别名
    pub fn resolve_alias(&self, query: &str) -> Option<String> {
        self.tool_aliases.get(&query.to_lowercase()).cloned()
    }

    /// 构建默认拼写纠错映射
    fn build_default_spelling_corrections() -> HashMap<String, String> {
        let mut corrections = HashMap::new();
        corrections.insert("read_fle".to_string(), "read_file".to_string());
        corrections.insert("wrtie_file".to_string(), "write_file".to_string());
        corrections.insert("delet_file".to_string(), "delete_file".to_string());
        corrections.insert("cop_file".to_string(), "copy_file".to_string());
        corrections.insert("git_comit".to_string(), "git_commit".to_string());
        corrections.insert("git_brach".to_string(), "git_branch".to_string());
        corrections.insert("htp_get".to_string(), "http_get".to_string());
        corrections.insert("jsson_format".to_string(), "json_format".to_string());
        corrections
    }

    /// 构建默认工具别名映射
    fn build_default_tool_aliases() -> HashMap<String, String> {
        let mut aliases = HashMap::new();
        aliases.insert("读取文件".to_string(), "read_file".to_string());
        aliases.insert("写入文件".to_string(), "write_file".to_string());
        aliases.insert("删除文件".to_string(), "delete_file".to_string());
        aliases.insert("复制文件".to_string(), "copy_file".to_string());
        aliases.insert("查看 git 状态".to_string(), "git_status".to_string());
        aliases.insert("查看 git 日志".to_string(), "git_log".to_string());
        aliases.insert("http 请求".to_string(), "http_get".to_string());
        aliases.insert("格式化 json".to_string(), "json_format".to_string());
        aliases
    }
}

impl Default for QueryEnhancer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_enhancer_synonyms() {
        let mut enhancer = QueryEnhancer::new();

        // 手动添加同义词
        enhancer.synonym_map.insert(
            "读取".to_lowercase(),
            vec!["read".to_string(), "读取".to_string()],
        );
        enhancer.synonym_map.insert(
            "文件".to_lowercase(),
            vec!["file".to_string(), "文件".to_string()],
        );

        let expansions = enhancer.expand_synonyms("读取文件");
        assert!(expansions.contains(&"read".to_string()));
        assert!(expansions.contains(&"file".to_string()));
    }

    #[test]
    fn test_query_enhancer_spelling() {
        let mut enhancer = QueryEnhancer::new();

        // 添加拼写纠错映射
        enhancer
            .spelling_corrections
            .insert("read_fle".to_string(), "read_file".to_string());

        let (corrected, has_correction) = enhancer.correct_spelling("read_fle");
        assert!(has_correction);
        assert_eq!(corrected, "read_file");
    }

    #[test]
    fn test_query_enhancer_intent() {
        let mut enhancer = QueryEnhancer::new();

        // 添加 how_to 意图模式
        let how_to_pattern = regex::Regex::new("怎么.*").unwrap();
        enhancer
            .intent_patterns
            .insert("how_to".to_string(), vec![how_to_pattern]);

        let intent = enhancer.recognize_intent("怎么读取文件");
        assert!(intent.is_some());
        assert_eq!(intent.unwrap().intent_type, "how_to");
    }

    #[test]
    fn test_query_enhancer_full() {
        let enhancer = QueryEnhancer::from_default_config().unwrap();

        let enhanced = enhancer.enhance("怎么读取文件");
        assert!(enhanced.intent.is_some());
        // 注意：同义词扩展可能为空，因为默认配置可能不包含"读取"的同义词
        // 这里只检查意图识别
    }
}
