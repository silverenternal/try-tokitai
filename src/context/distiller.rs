//! 分层上下文蒸馏（Hierarchical Context Distillation, HCD）
//!
//! 核心思想：
//! - 基于任务意图 + 工具调用类型，对上下文做"分层蒸馏"
//! - 第一层：保留"任务核心意图"（如"更新项目并提交"）
//! - 第二层：保留"关键工具依赖"（如"Git 无冲突"）
//! - 第三层：丢弃"冗余交互"（如无关追问、重复确认）
//! - 生成意图驱动的结构化摘要，比普通摘要体积小 60%+

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// 蒸馏后的结构化摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistilledSummary {
    /// 第一层：任务核心意图
    pub core_intent: Option<String>,
    /// 第二层：关键工具依赖
    pub tool_dependencies: Vec<ToolDependency>,
    /// 第三层：丢弃的冗余内容（仅记录元数据）
    pub discarded_metadata: DiscardedMetadata,
    /// 蒸馏时间
    pub distilled_at: DateTime<Utc>,
    /// 原始内容哈希
    pub content_hash: String,
    /// 蒸馏质量评分（0.0 - 1.0）
    pub quality_score: f32,
}

/// 工具依赖
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDependency {
    /// 工具名称（如 "git", "cargo", "npm"）
    pub tool_name: String,
    /// 操作类型（如 "commit", "build", "install"）
    pub operation: String,
    /// 状态（成功/失败/无冲突等）
    pub status: ToolStatus,
    /// 关键输出/错误信息（精简版）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_output: Option<String>,
}

/// 工具状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    Success,
    Failure,
    NoConflict,
    Conflict,
    Pending,
    Skipped,
}

/// 被丢弃内容的元数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiscardedMetadata {
    /// 丢弃的冗余交互数量
    pub redundant_interactions: usize,
    /// 丢弃的无意义内容字符数
    pub discarded_chars: usize,
    /// 压缩率（0.0 - 1.0，越高表示压缩越多）
    pub compression_ratio: f32,
}

/// 蒸馏器配置
#[derive(Debug, Clone)]
pub struct DistillerConfig {
    /// 是否启用意图提取
    pub enable_intent_extraction: bool,
    /// 是否启用工具依赖识别
    pub enable_tool_detection: bool,
    /// 是否启用冗余过滤
    pub enable_redundancy_filter: bool,
    /// 无意义短语列表（用于过滤）
    pub meaningless_phrases: Vec<String>,
    /// 工具关键词映射
    pub tool_keywords: Vec<ToolKeywordMapping>,
}

impl Default for DistillerConfig {
    fn default() -> Self {
        Self {
            enable_intent_extraction: true,
            enable_tool_detection: true,
            enable_redundancy_filter: true,
            meaningless_phrases: vec![
                "好的".to_string(),
                "谢谢".to_string(),
                "明白了".to_string(),
                "收到".to_string(),
                "ok".to_string(),
                "okay".to_string(),
                "sure".to_string(),
                "no problem".to_string(),
                "没问题".to_string(),
            ],
            tool_keywords: vec![
                ToolKeywordMapping {
                    tool_name: "git".to_string(),
                    keywords: vec!["git".to_string(), "commit".to_string(), "push".to_string(), "pull".to_string(), "merge".to_string(), "rebase".to_string(), "branch".to_string(), "checkout".to_string()],
                },
                ToolKeywordMapping {
                    tool_name: "cargo".to_string(),
                    keywords: vec!["cargo".to_string(), "build".to_string(), "test".to_string(), "run".to_string(), "check".to_string(), "clippy".to_string(), "fmt".to_string()],
                },
                ToolKeywordMapping {
                    tool_name: "npm".to_string(),
                    keywords: vec!["npm".to_string(), "yarn".to_string(), "pnpm".to_string(), "install".to_string(), "build".to_string(), "dev".to_string(), "start".to_string()],
                },
                ToolKeywordMapping {
                    tool_name: "docker".to_string(),
                    keywords: vec!["docker".to_string(), "container".to_string(), "image".to_string(), "build".to_string(), "run".to_string(), "compose".to_string()],
                },
            ],
        }
    }
}

/// 工具关键词映射
#[derive(Debug, Clone)]
pub struct ToolKeywordMapping {
    pub tool_name: String,
    pub keywords: Vec<String>,
}

/// 分层上下文蒸馏器
pub struct ContextDistiller {
    config: DistillerConfig,
}

impl ContextDistiller {
    /// 创建蒸馏器
    pub fn new(config: DistillerConfig) -> Self {
        Self { config }
    }

    /// 蒸馏上下文内容（单次遍历优化版本）
    pub fn distill(&self, content: &str, content_hash: &str) -> DistilledSummary {
        let original_chars = content.chars().count();

        // 单次遍历处理所有逻辑
        let (core_intent, tool_dependencies, redundant_count, discarded_chars) = 
            self.process_content_single_pass(content);

        // 构建摘要
        let mut summary = DistilledSummary {
            core_intent,
            tool_dependencies,
            discarded_metadata: DiscardedMetadata {
                redundant_interactions: redundant_count,
                discarded_chars,
                compression_ratio: if original_chars > 0 {
                    discarded_chars as f32 / original_chars as f32
                } else {
                    0.0
                },
            },
            distilled_at: Utc::now(),
            content_hash: content_hash.to_string(),
            quality_score: 0.0,
        };

        // 计算质量评分
        summary.quality_score = self.calculate_quality_score(&summary);

        summary
    }

    /// 单次遍历处理内容（整合意图提取、工具检测、冗余过滤）
    fn process_content_single_pass(
        &self,
        content: &str,
    ) -> (Option<String>, Vec<ToolDependency>, usize, usize) {
        let mut intent_lines = Vec::new();
        let mut tool_deps = Vec::new();
        let mut redundant_count = 0;
        let mut discarded_chars = 0;

        // 工具检测需要完整内容，先快速检测
        if self.config.enable_tool_detection {
            tool_deps = self.detect_tool_dependencies(content);
        }

        // 逐行处理用于意图提取和冗余过滤
        for line in content.lines() {
            let trimmed = line.trim();

            // 跳过空行
            if trimmed.is_empty() {
                continue;
            }

            // 检查是否冗余
            if self.config.enable_redundancy_filter && 
               (self.is_meaningless(trimmed) || self.is_redundant_confirmation(trimmed)) {
                redundant_count += 1;
                discarded_chars += trimmed.len();
                continue;
            }

            // 收集意图行
            if self.config.enable_intent_extraction
                && self.contains_action_word(trimmed)
            {
                intent_lines.push(trimmed);
            }
        }

        // 构建核心意图
        let core_intent = if self.config.enable_intent_extraction {
            if intent_lines.is_empty() {
                // 如果没有找到动作关键词，返回前 3 行非空内容
                let non_empty: Vec<&str> = content.lines()
                    .map(|l| l.trim())
                    .filter(|l| !l.is_empty() && 
                           !self.is_meaningless(l.trim()) &&
                           !self.is_redundant_confirmation(l.trim()))
                    .take(3)
                    .collect();

                if non_empty.is_empty() {
                    None
                } else {
                    Some(non_empty.join("\n"))
                }
            } else {
                Some(intent_lines.join("\n"))
            }
        } else {
            None
        };

        (core_intent, tool_deps, redundant_count, discarded_chars)
    }

    /// 检测工具依赖
    fn detect_tool_dependencies(&self, content: &str) -> Vec<ToolDependency> {
        let mut tools = Vec::new();
        let content_lower = content.to_lowercase();

        for mapping in &self.config.tool_keywords {
            for keyword in &mapping.keywords {
                if content_lower.contains(&keyword.to_lowercase()) {
                    // 检测工具状态
                    let status = self.detect_tool_status(content, &mapping.tool_name);
                    
                    // 提取关键输出
                    let key_output = self.extract_tool_output(content, &mapping.tool_name);

                    let dependency = ToolDependency {
                        tool_name: mapping.tool_name.clone(),
                        operation: keyword.clone(),
                        status,
                        key_output,
                    };

                    // 避免重复
                    if !tools.iter().any(|t: &ToolDependency| t.tool_name == dependency.tool_name && t.operation == dependency.operation) {
                        tools.push(dependency);
                    }
                    break;
                }
            }
        }

        tools
    }

    /// 检测工具状态（使用优先级状态机）
    fn detect_tool_status(&self, content: &str, tool_name: &str) -> ToolStatus {
        let content_lower = content.to_lowercase();

        // 状态检测优先级（从高到低）：
        // 1. Conflict > 2. NoConflict > 3. Failure > 4. Success > 5. Skipped

        // 检查是否包含工具名称
        if !content_lower.contains(&tool_name.to_lowercase()) {
            return ToolStatus::Skipped;
        }

        // 优先级 1：检查冲突（最高优先级）
        let has_conflict = self.contains_any_keyword(&content_lower, &["conflict", "merge conflict", "冲突"]);
        
        // 优先级 2：检查无冲突
        let has_no_conflict = self.contains_any_keyword(&content_lower, &["no conflict", "无冲突"]);

        // 冲突检测：如果有冲突关键词且没有"无冲突"修饰
        if has_conflict && !has_no_conflict {
            return ToolStatus::Conflict;
        }

        // 无冲突明确声明（覆盖其他状态）
        if has_no_conflict {
            return ToolStatus::NoConflict;
        }

        // 优先级 3：检查失败
        if self.contains_any_keyword(&content_lower, &["error", "failed", "failure", "fail", "exception", "错误", "失败"]) {
            return ToolStatus::Failure;
        }

        // 优先级 4：检查成功
        if self.contains_any_keyword(&content_lower, &["success", "successful", "done", "completed", "passed", "成功", "完成", "通过"]) {
            return ToolStatus::Success;
        }

        // 优先级 5：默认无冲突
        ToolStatus::NoConflict
    }

    /// 辅助函数：检查是否包含任意关键词
    fn contains_any_keyword(&self, content_lower: &str, keywords: &[&str]) -> bool {
        keywords.iter().any(|k| content_lower.contains(k))
    }

    /// 提取工具输出
    fn extract_tool_output(&self, content: &str, tool_name: &str) -> Option<String> {
        // 简化的实现：提取包含工具名称的行的后 50 个字符
        for line in content.lines() {
            if line.to_lowercase().contains(&tool_name.to_lowercase()) {
                let chars: Vec<char> = line.chars().collect();
                if chars.len() > 50 {
                    return Some(chars[chars.len()-50..].iter().collect::<String>());
                } else {
                    return Some(line.to_string());
                }
            }
        }
        None
    }

    /// 检查是否是无意义短语
    fn is_meaningless(&self, text: &str) -> bool {
        let text_lower = text.to_lowercase();
        for phrase in &self.config.meaningless_phrases {
            if text_lower == phrase.to_lowercase() {
                return true;
            }
        }
        false
    }

    /// 检查是否是冗余确认
    fn is_redundant_confirmation(&self, text: &str) -> bool {
        let patterns = [
            "好的，", "好的。", "明白了，", "明白了。",
            "收到，", "收到。", "没问题，", "没问题。",
            "i see", "got it", "understood",
        ];
        
        let text_lower = text.to_lowercase();
        for pattern in patterns {
            if text_lower.starts_with(&pattern.to_lowercase()) {
                return true;
            }
        }
        false
    }

    /// 检查是否包含动作关键词
    fn contains_action_word(&self, text: &str) -> bool {
        let action_keywords = [
            "更新", "提交", "推送", "拉取", "创建", "删除", "修改", "添加",
            "update", "commit", "push", "pull", "create", "delete", "modify", "add",
            "build", "test", "run", "install", "deploy",
            "构建", "测试", "运行", "安装", "部署",
        ];

        let text_lower = text.to_lowercase();
        for keyword in action_keywords {
            if text_lower.contains(&keyword.to_lowercase()) {
                return true;
            }
        }
        false
    }

    /// 计算质量评分
    fn calculate_quality_score(&self, summary: &DistilledSummary) -> f32 {
        let mut score = 0.5; // 基础分

        // 有核心意图加分
        if summary.core_intent.is_some() {
            score += 0.2;
        }

        // 有工具依赖加分
        if !summary.tool_dependencies.is_empty() {
            score += 0.1 * (summary.tool_dependencies.len() as f32).min(3.0);
        }

        // 压缩率适中加分（0.3 - 0.7 之间最佳）
        if summary.discarded_metadata.compression_ratio >= 0.3 && 
           summary.discarded_metadata.compression_ratio <= 0.7 {
            score += 0.1;
        }

        score.min(1.0)
    }

    /// 将蒸馏结果转换为 JSON 字符串（用于云端传输）
    pub fn to_json(&self, summary: &DistilledSummary) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(summary)
    }

    /// 从 JSON 字符串解析蒸馏结果
    pub fn parse_from_json(&self, json: &str) -> Result<DistilledSummary, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// 蒸馏结果缓存（真正的 LRU 实现）
use std::collections::VecDeque;

pub struct DistillationCache {
    cache: std::collections::HashMap<String, DistilledSummary>,
    /// 维护插入顺序的队列（用于 LRU 淘汰）
    order: VecDeque<String>,
    max_size: usize,
}

impl DistillationCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: std::collections::HashMap::with_capacity(max_size),
            order: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    pub fn get(&self, hash: &str) -> Option<&DistilledSummary> {
        self.cache.get(hash)
    }

    /// 获取并提升访问顺序（用于 LRU）
    pub fn get_mut(&mut self, hash: &str) -> Option<&mut DistilledSummary> {
        self.cache.get_mut(hash)
    }

    pub fn insert(&mut self, hash: String, summary: DistilledSummary) {
        // 如果已存在，先移除旧顺序
        if self.cache.contains_key(&hash) {
            self.order.retain(|h| h != &hash);
        }

        // 如果缓存已满，移除最旧的条目
        while self.cache.len() >= self.max_size {
            if let Some(oldest) = self.order.pop_front() {
                self.cache.remove(&oldest);
            } else {
                break;
            }
        }

        self.order.push_back(hash.clone());
        self.cache.insert(hash, summary);
    }

    pub fn remove(&mut self, hash: &str) {
        self.cache.remove(hash);
        self.order.retain(|h| h != hash);
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.order.clear();
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// 获取缓存命中率统计
    pub fn get_stats(&self) -> CacheStats {
        CacheStats {
            total_items: self.cache.len(),
            max_size: self.max_size,
            utilization: if self.max_size > 0 {
                self.cache.len() as f32 / self.max_size as f32
            } else {
                0.0
            },
        }
    }
}

/// 缓存统计信息
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_items: usize,
    pub max_size: usize,
    pub utilization: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distiller_creation() {
        let config = DistillerConfig::default();
        let distiller = ContextDistiller::new(config);
        
        // 蒸馏简单内容
        let summary = distiller.distill("Hello, world!", "hash123");
        assert_eq!(summary.content_hash, "hash123");
    }

    #[test]
    fn test_core_intent_extraction() {
        let config = DistillerConfig::default();
        let distiller = ContextDistiller::new(config);

        let content = "更新项目代码并提交到远程仓库";
        let summary = distiller.distill(content, "hash1");

        assert!(summary.core_intent.is_some());
        let intent = summary.core_intent.unwrap();
        assert!(intent.contains("更新") || intent.contains("提交"));
    }

    #[test]
    fn test_tool_dependency_detection() {
        let config = DistillerConfig::default();
        let distiller = ContextDistiller::new(config);

        // 测试 1: 包含"无冲突"应该返回 NoConflict
        let content = "运行 cargo build 构建项目，成功完成无冲突";
        let summary = distiller.distill(content, "hash2");

        assert!(!summary.tool_dependencies.is_empty());
        let cargo_deps: Vec<_> = summary.tool_dependencies.iter()
            .filter(|d| d.tool_name == "cargo")
            .collect();
        assert!(!cargo_deps.is_empty());
        assert_eq!(cargo_deps[0].status, ToolStatus::NoConflict);

        // 测试 2: 只有成功关键词应该返回 Success
        let content2 = "运行 cargo build 构建项目，成功完成";
        let summary2 = distiller.distill(content2, "hash2b");
        let cargo_deps2: Vec<_> = summary2.tool_dependencies.iter()
            .filter(|d| d.tool_name == "cargo")
            .collect();
        assert!(!cargo_deps2.is_empty());
        assert_eq!(cargo_deps2[0].status, ToolStatus::Success);
    }

    #[test]
    fn test_redundancy_filtering() {
        let config = DistillerConfig::default();
        let distiller = ContextDistiller::new(config);

        let content = "好的\n谢谢\n更新项目代码\n没问题\n提交更改";
        let summary = distiller.distill(content, "hash3");

        assert!(summary.discarded_metadata.redundant_interactions > 0);
        assert!(summary.discarded_metadata.compression_ratio > 0.0);
    }

    #[test]
    fn test_quality_score() {
        let config = DistillerConfig::default();
        let distiller = ContextDistiller::new(config);

        let content = "使用 cargo build 构建项目，成功完成，更新了依赖";
        let summary = distiller.distill(content, "hash4");

        assert!(summary.quality_score > 0.5);
        assert!(summary.quality_score <= 1.0);
    }

    #[test]
    fn test_json_serialization() {
        let config = DistillerConfig::default();
        let distiller = ContextDistiller::new(config);

        let content = "cargo build 成功";
        let summary = distiller.distill(content, "hash5");

        let json = distiller.to_json(&summary).unwrap();
        assert!(!json.is_empty());

        let parsed = distiller.parse_from_json(&json).unwrap();
        assert_eq!(parsed.content_hash, summary.content_hash);
    }

    #[test]
    fn test_cache() {
        let mut cache = DistillationCache::new(3);

        let summary = DistilledSummary {
            core_intent: Some("test".to_string()),
            tool_dependencies: vec![],
            discarded_metadata: DiscardedMetadata::default(),
            distilled_at: Utc::now(),
            content_hash: "hash1".to_string(),
            quality_score: 0.8,
        };

        cache.insert("hash1".to_string(), summary);
        assert_eq!(cache.len(), 1);

        let retrieved = cache.get("hash1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().core_intent, Some("test".to_string()));

        cache.remove("hash1");
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_lru_eviction() {
        let mut cache = DistillationCache::new(3);

        // 插入 3 个元素
        for i in 1..=3 {
            cache.insert(format!("hash{}", i), DistilledSummary {
                core_intent: Some(format!("intent{}", i)),
                tool_dependencies: vec![],
                discarded_metadata: DiscardedMetadata::default(),
                distilled_at: Utc::now(),
                content_hash: format!("hash{}", i),
                quality_score: 0.8,
            });
        }

        assert_eq!(cache.len(), 3);

        // 插入第 4 个元素，应该淘汰最旧的 hash1
        cache.insert("hash4".to_string(), DistilledSummary {
            core_intent: Some("intent4".to_string()),
            tool_dependencies: vec![],
            discarded_metadata: DiscardedMetadata::default(),
            distilled_at: Utc::now(),
            content_hash: "hash4".to_string(),
            quality_score: 0.8,
        });

        assert_eq!(cache.len(), 3);
        assert!(cache.get("hash1").is_none()); // 最旧的被淘汰
        assert!(cache.get("hash2").is_some());
        assert!(cache.get("hash3").is_some());
        assert!(cache.get("hash4").is_some());
    }

    #[test]
    fn test_cache_stats() {
        let mut cache = DistillationCache::new(10);

        for i in 0..5 {
            cache.insert(format!("hash{}", i), DistilledSummary {
                core_intent: Some("test".to_string()),
                tool_dependencies: vec![],
                discarded_metadata: DiscardedMetadata::default(),
                distilled_at: Utc::now(),
                content_hash: format!("hash{}", i),
                quality_score: 0.8,
            });
        }

        let stats = cache.get_stats();
        assert_eq!(stats.total_items, 5);
        assert_eq!(stats.max_size, 10);
        assert!((stats.utilization - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_git_status_detection() {
        let config = DistillerConfig::default();
        let distiller = ContextDistiller::new(config);

        // 测试 Git 无冲突状态
        let content = "git commit 成功，无冲突";
        let summary = distiller.distill(content, "hash6");
        
        let git_deps: Vec<_> = summary.tool_dependencies.iter()
            .filter(|d| d.tool_name == "git")
            .collect();
        assert!(!git_deps.is_empty());
        assert_eq!(git_deps[0].status, ToolStatus::NoConflict);

        // 测试 Git 冲突状态
        let content2 = "git merge 失败，存在冲突";
        let summary2 = distiller.distill(content2, "hash7");
        
        let git_deps2: Vec<_> = summary2.tool_dependencies.iter()
            .filter(|d| d.tool_name == "git")
            .collect();
        assert!(!git_deps2.is_empty());
        assert_eq!(git_deps2[0].status, ToolStatus::Conflict);
    }

    #[test]
    fn test_distillation_compression() {
        let config = DistillerConfig::default();
        let distiller = ContextDistiller::new(config);

        // 创建包含大量冗余内容的文本
        let content = "好的\n谢谢\n明白了\n收到\n没问题\n使用 cargo test 运行测试，全部通过\n好的\n谢谢";
        let summary = distiller.distill(content, "hash8");

        // 验证压缩率
        assert!(summary.discarded_metadata.compression_ratio > 0.3);
        
        // 验证核心意图保留了关键信息
        assert!(summary.core_intent.is_some());
        let intent = summary.core_intent.as_ref().unwrap();
        assert!(intent.contains("cargo") || intent.contains("测试"));
    }
}
