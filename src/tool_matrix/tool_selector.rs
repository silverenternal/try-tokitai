//! 轻量级工具选择器
//!
//! AI 原生的工具选择系统，支持：
//! - 快速搜索（关键词匹配，<10ms）
//! - AI 搜索（复杂查询，<2s）
//! - 后台异步索引重建
//! - AI 自主工具箱分类
//! - AI 自主依赖关系分析
//!
//! # 设计原则

#![allow(dead_code)]
//! - AI 自主管理工具箱（非预先设计）
//! - 后台异步索引重建（不阻塞主线程）
//! - AI 自主维护依赖关系（非手动声明）
//! - tokitai 深度集成

use crate::tool_matrix::ai_classifier::LLMClient as AILLMClient;
use crate::tool_matrix::matrix::{ServiceCategory, ToolDefinition};
use crate::tool_matrix::trie_index::TrieIndex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

// ============================================================================
// 工具索引（倒排索引 + Trie 树优化）
// ============================================================================

/// 工具索引（倒排索引 + Trie 树优化）
#[derive(Debug, Clone)]
pub struct ToolIndex {
    /// 工具名称 -> 工具定义
    tools: HashMap<String, ToolDefinition>,
    /// 关键词 -> 工具名称集合（倒排索引）
    keyword_index: HashMap<String, HashSet<String>>,
    /// 工具箱 -> 工具名称集合
    toolbox_index: HashMap<String, HashSet<String>>,
    /// 分类 -> 工具名称集合
    category_index: HashMap<ServiceCategory, HashSet<String>>,
    /// Trie 树索引（用于前缀搜索优化）
    trie_index: TrieIndex,
}

impl ToolIndex {
    /// 创建新的空索引
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            keyword_index: HashMap::new(),
            toolbox_index: HashMap::new(),
            category_index: HashMap::new(),
            trie_index: TrieIndex::new(),
        }
    }

    /// 添加工具
    pub fn add_tool(&mut self, tool: ToolDefinition) {
        let tool_name = tool.name.clone();

        // 提取关键词并建立倒排索引
        let keywords = extract_keywords(&tool);
        for keyword in keywords {
            self.keyword_index
                .entry(keyword)
                .or_default()
                .insert(tool_name.clone());
        }

        // 添加到分类索引
        self.category_index
            .entry(tool.metadata.category.clone())
            .or_default()
            .insert(tool_name.clone());

        // 添加到 Trie 索引（用于前缀搜索）
        self.trie_index
            .add_tool(&tool_name, self.tools.len() as u64);

        // 存储工具定义
        self.tools.insert(tool_name.clone(), tool);
    }

    /// 添加工具到工具箱
    pub fn add_tool_to_toolbox(&mut self, tool_name: &str, toolbox_id: &str) {
        self.toolbox_index
            .entry(toolbox_id.to_string())
            .or_default()
            .insert(tool_name.to_string());
    }

    /// 搜索工具（使用 Trie 树优化）
    pub fn search(&self, query: &str, max_results: usize) -> Vec<ToolDefinition> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();
        let mut seen = HashSet::new();

        // 1. Trie 树前缀搜索（最快，O(m) 复杂度）
        let trie_results = self.trie_index.search_prefix(&query_lower);
        for tool_name in trie_results {
            if seen.insert(tool_name.clone()) {
                if let Some(tool) = self.tools.get(&tool_name) {
                    results.push(tool.clone());
                    if results.len() >= max_results {
                        return results;
                    }
                }
            }
        }

        // 2. 关键词匹配（倒排索引）- 优化：只遍历部分关键词
        // 如果查询较短，直接使用前缀匹配的结果
        if query_lower.len() <= 3 && !results.is_empty() {
            return results;
        }

        // 否则，只检查包含查询的关键词
        for keyword in self.keyword_index.keys() {
            if keyword.contains(&query_lower) || query_lower.contains(keyword) {
                if let Some(tool_names) = self.keyword_index.get(keyword) {
                    for tool_name in tool_names {
                        if seen.insert(tool_name.clone()) {
                            if let Some(tool) = self.tools.get(tool_name) {
                                results.push(tool.clone());
                                if results.len() >= max_results {
                                    return results;
                                }
                            }
                        }
                    }
                }
            }
        }

        // 3. 名称/描述匹配（兜底）- 优化：限制遍历数量
        if results.len() < max_results {
            let remaining = max_results - results.len();
            for (name, tool) in &self.tools {
                if seen.insert(name.clone())
                    && (tool.name.to_lowercase().contains(&query_lower)
                        || tool.description.to_lowercase().contains(&query_lower))
                {
                    results.push(tool.clone());
                    if results.len() >= max_results {
                        return results;
                    }
                }
                // 限制兜底搜索的工具数量
                if seen.len() > 1000 && results.len() >= remaining {
                    break;
                }
            }
        }

        results
    }

    /// 按分类获取工具
    pub fn get_by_category(&self, category: &ServiceCategory) -> Vec<&ToolDefinition> {
        self.category_index
            .get(category)
            .map(|tools| tools.iter().filter_map(|n| self.tools.get(n)).collect())
            .unwrap_or_default()
    }

    /// 按工具箱获取工具
    pub fn get_by_toolbox(&self, toolbox_id: &str) -> Vec<&ToolDefinition> {
        self.toolbox_index
            .get(toolbox_id)
            .map(|tools| tools.iter().filter_map(|n| self.tools.get(n)).collect())
            .unwrap_or_default()
    }

    /// 获取所有工具
    pub fn get_all_tools(&self) -> Vec<&ToolDefinition> {
        self.tools.values().collect()
    }

    /// 获取工具数量
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// 提取关键词
fn extract_keywords(tool: &ToolDefinition) -> Vec<String> {
    let mut keywords = Vec::new();

    // 从名称提取
    keywords.extend(extract_words_from_text(&tool.name));

    // 从描述提取
    keywords.extend(extract_words_from_text(&tool.description));

    // 从标签提取
    for tag in &tool.tags {
        keywords.push(tag.to_lowercase());
    }

    // 从分类提取
    keywords.push(format!("{:?}", tool.metadata.category).to_lowercase());

    // 去重
    keywords.sort();
    keywords.dedup();

    keywords
}

/// 从文本中提取单词
fn extract_words_from_text(text: &str) -> Vec<String> {
    // 简单实现：按空格和标点分割，转小写
    text.split(|c: char| c.is_whitespace() || c == '_' || c == '-' || c == '(' || c == ')')
        .filter(|s| !s.is_empty() && s.len() > 1)
        .map(|s| s.to_lowercase())
        .collect()
}

// ============================================================================
// 工具搜索结果
// ============================================================================

/// 工具搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSearchResult {
    /// 工具定义
    pub tool: ToolDefinition,
    /// 相关性分数（0-1）
    pub relevance_score: f32,
    /// 排名分数（综合考虑使用频率等）
    pub ranking_score: f32,
    /// 搜索来源
    pub source: SearchResultSource,
}

/// 搜索来源
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchResultSource {
    /// 关键词匹配
    Keyword,
    /// AI 搜索
    Ai,
    /// 分类过滤
    Category,
    /// 工具箱过滤
    Toolbox,
}

// ============================================================================
// 选择器配置
// ============================================================================

/// 选择器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectorConfig {
    /// 最大搜索结果数
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    /// AI 搜索触发阈值（查询长度）
    #[serde(default = "default_ai_search_threshold")]
    pub ai_search_threshold: usize,
    /// 是否启用后台索引重建
    #[serde(default = "default_true")]
    pub enable_background_rebuild: bool,
    /// 后台重建延迟（秒）
    #[serde(default = "default_rebuild_delay")]
    pub rebuild_delay_secs: u64,
}

fn default_max_results() -> usize {
    20
}
fn default_ai_search_threshold() -> usize {
    20
}
fn default_true() -> bool {
    true
}
fn default_rebuild_delay() -> u64 {
    2
}

impl Default for SelectorConfig {
    fn default() -> Self {
        Self {
            max_results: default_max_results(),
            ai_search_threshold: default_ai_search_threshold(),
            enable_background_rebuild: default_true(),
            rebuild_delay_secs: default_rebuild_delay(),
        }
    }
}

// ============================================================================
// 轻量级工具选择器
// ============================================================================

/// 轻量级工具选择器
pub struct LightweightToolSelector {
    /// 当前索引（读多写少，RwLock）
    current_index: Arc<RwLock<ToolIndex>>,
    /// 待重建的工具队列
    pending_tools: Arc<RwLock<Vec<ToolDefinition>>>,
    /// 后台重建触发标志
    rebuild_trigger: Arc<AtomicBool>,
    /// 后台重建任务句柄
    rebuild_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    /// LLM 客户端（用于 AI 搜索）
    llm_client: Option<Arc<dyn AILLMClient>>,
    /// 配置
    config: SelectorConfig,
    /// 搜索缓存（LRU 缓存，优化重复查询）
    search_cache: Arc<RwLock<HashMap<String, Vec<ToolSearchResult>>>>,
    /// 监控指标
    metrics: Arc<RwLock<SelectorMetrics>>,
}

/// 选择器监控指标
#[derive(Debug, Clone, Default)]
pub struct SelectorMetrics {
    /// 总搜索次数
    pub total_searches: u64,
    /// 缓存命中次数
    pub cache_hits: u64,
    /// AI 搜索次数
    pub ai_searches: u64,
    /// 快速搜索次数
    pub fast_searches: u64,
    /// 平均搜索延迟（微秒）
    pub avg_latency_us: f64,
    /// 后台重建次数
    pub rebuild_count: u64,
}

impl SelectorMetrics {
    /// 记录搜索
    pub fn record_search(&mut self, latency_us: u64, is_ai: bool, is_cache_hit: bool) {
        self.total_searches += 1;
        if is_cache_hit {
            self.cache_hits += 1;
        }
        if is_ai {
            self.ai_searches += 1;
        } else {
            self.fast_searches += 1;
        }

        // 更新平均延迟
        let total = self.total_searches as f64;
        self.avg_latency_us = (self.avg_latency_us * (total - 1.0) + latency_us as f64) / total;
    }

    /// 记录重建
    pub fn record_rebuild(&mut self) {
        self.rebuild_count += 1;
    }

    /// 获取缓存命中率
    pub fn cache_hit_rate(&self) -> f32 {
        if self.total_searches == 0 {
            0.0
        } else {
            self.cache_hits as f32 / self.total_searches as f32
        }
    }
}

impl LightweightToolSelector {
    /// 创建新的选择器
    pub fn new(
        tools: Vec<ToolDefinition>,
        config: Option<SelectorConfig>,
        llm_client: Option<Arc<dyn AILLMClient>>,
    ) -> Self {
        let config = config.unwrap_or_default();
        let mut index = ToolIndex::new();

        // 构建初始索引
        for tool in tools {
            index.add_tool(tool);
        }

        Self {
            current_index: Arc::new(RwLock::new(index)),
            pending_tools: Arc::new(RwLock::new(Vec::new())),
            rebuild_trigger: Arc::new(AtomicBool::new(false)),
            rebuild_handle: Arc::new(RwLock::new(None)),
            llm_client,
            config,
            search_cache: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(SelectorMetrics::default())),
        }
    }

    /// 创建不带 AI 的选择器（向后兼容）
    pub fn new_without_ai(tools: Vec<ToolDefinition>, config: Option<SelectorConfig>) -> Self {
        Self::new(tools, config, None)
    }

    /// 获取监控指标
    pub async fn get_metrics(&self) -> SelectorMetrics {
        self.metrics.read().await.clone()
    }

    /// 清除搜索缓存
    pub async fn clear_cache(&self) {
        let mut cache = self.search_cache.write().await;
        cache.clear();
        info!("搜索缓存已清除");
    }

    /// 添加新工具（异步，不阻塞）
    pub async fn add_tool_async(&self, tool: ToolDefinition) {
        // 1. 添加到待重建队列
        let pending = self.pending_tools.clone();
        let rebuild_trigger = self.rebuild_trigger.clone();
        let config = self.config.clone();

        // 2. 触发后台重建
        if config.enable_background_rebuild {
            self.trigger_rebuild(pending, rebuild_trigger, config).await;
        }
    }

    /// 触发后台重建（批量处理优化）
    async fn trigger_rebuild(
        &self,
        pending: Arc<RwLock<Vec<ToolDefinition>>>,
        rebuild_trigger: Arc<AtomicBool>,
        config: SelectorConfig,
    ) {
        // 检查是否已经在重建
        if rebuild_trigger.load(Ordering::SeqCst) {
            debug!("索引正在重建，跳过触发");
            return;
        }

        // 标记为需要重建
        rebuild_trigger.store(true, Ordering::SeqCst);

        // 启动后台任务
        let pending_tools = self.pending_tools.clone();
        let current_index = self.current_index.clone();
        let rebuild_trigger_clone = rebuild_trigger.clone();
        let rebuild_handle = self.rebuild_handle.clone();
        let metrics = self.metrics.clone();

        let handle = tokio::spawn(async move {
            // 等待一小段时间，收集更多新工具（批量处理）
            tokio::time::sleep(Duration::from_secs(config.rebuild_delay_secs)).await;

            // 取出待重建工具
            let tools_to_add = {
                let mut pending = pending_tools.write().await;
                std::mem::take(&mut *pending)
            };

            if tools_to_add.is_empty() {
                rebuild_trigger_clone.store(false, Ordering::SeqCst);
                return;
            }

            info!("开始重建工具索引，批量处理 {} 个工具", tools_to_add.len());
            let rebuild_start = std::time::Instant::now();

            // 构建新索引（批量添加）
            let mut new_index = current_index.read().await.clone();
            for tool in &tools_to_add {
                new_index.add_tool(tool.clone());
            }

            // 原子替换索引（读操作无感知）
            *current_index.write().await = new_index;

            let elapsed = rebuild_start.elapsed();
            info!(
                "工具索引重建完成：新增 {} 个工具，耗时 {:?}",
                tools_to_add.len(),
                elapsed
            );

            // 记录重建指标
            {
                let mut metrics = metrics.write().await;
                metrics.record_rebuild();
            }

            // 清除重建标记
            rebuild_trigger_clone.store(false, Ordering::SeqCst);

            // 检查是否有新的待重建工具（连续重建）
            if !pending_tools.read().await.is_empty() {
                rebuild_trigger_clone.store(true, Ordering::SeqCst);
            }
        });

        // 保存任务句柄
        *rebuild_handle.write().await = Some(handle);
    }

    /// 搜索工具（主入口，带缓存和监控）
    pub async fn search(&self, query: &str) -> Vec<ToolSearchResult> {
        let start_time = std::time::Instant::now();

        // 1. 检查缓存
        {
            let cache = self.search_cache.read().await;
            if let Some(cached_result) = cache.get(query) {
                let elapsed = start_time.elapsed();
                let mut metrics = self.metrics.write().await;
                metrics.record_search(elapsed.as_micros() as u64, false, true);
                debug!("搜索缓存命中：{}", query);
                return cached_result.clone();
            }
        }

        // 2. 自动判断：复杂查询用 AI 搜索，简单查询用快速搜索
        let use_ai = self.should_use_ai_search(query);
        let is_ai = use_ai && self.llm_client.is_some();

        let results = if use_ai {
            if let Some(llm) = &self.llm_client {
                self.ai_search(query, llm).await
            } else {
                debug!("AI 搜索被请求，但未配置 LLM 客户端，降级为快速搜索");
                self.fast_search(query).await
            }
        } else {
            self.fast_search(query).await
        };

        // 3. 写入缓存（仅保留最近 1000 条查询）
        {
            let mut cache = self.search_cache.write().await;
            if cache.len() >= 1000 {
                // 简单 LRU：清除最早的 10% 条目
                let to_remove = cache.keys().take(100).cloned().collect::<Vec<_>>();
                for key in to_remove {
                    cache.remove(&key);
                }
            }
            cache.insert(query.to_string(), results.clone());
        }

        // 4. 记录指标
        let elapsed = start_time.elapsed();
        let mut metrics = self.metrics.write().await;
        metrics.record_search(elapsed.as_micros() as u64, is_ai, false);

        results
    }

    /// AI 搜索（复杂查询）
    async fn ai_search(
        &self,
        query: &str,
        llm_client: &Arc<dyn AILLMClient>,
    ) -> Vec<ToolSearchResult> {
        let start_time = std::time::Instant::now();

        // 1. 快速搜索获取候选（Top-50）
        let candidates = self.fast_search(query).await;

        if candidates.is_empty() {
            warn!("AI 搜索：快速搜索未找到任何候选工具");
            return Vec::new();
        }

        // 2. 构建 AI 提示词
        let prompt = format!(
            r#"你是一个工具选择专家。用户需要完成以下任务：

{}

请从以下工具中选择最相关的 5-10 个工具，按相关性排序：

{}

输出 JSON 格式：
{{
    "selected_tools": [
        {{"tool_name": "工具名", "relevance_score": 0.0-1.0, "reason": "选择理由"}}
    ]
}}"#,
            query,
            candidates
                .iter()
                .map(|t| format!("- **{}**: {}", t.tool.name, t.tool.description))
                .collect::<Vec<_>>()
                .join("\n")
        );

        // 3. 调用 AI
        let response = match llm_client.chat(&prompt).await {
            Ok(resp) => resp,
            Err(e) => {
                warn!("AI 搜索调用失败：{}，降级为快速搜索", e);
                return candidates;
            }
        };

        // 4. 解析 AI 响应
        let ai_result = self.parse_ai_search_response(&response, &candidates);

        let elapsed = start_time.elapsed();
        info!(
            "AI 搜索完成：耗时 {:?}，返回 {} 个工具",
            elapsed,
            ai_result.len()
        );

        ai_result
    }

    /// 解析 AI 搜索响应
    fn parse_ai_search_response(
        &self,
        response: &str,
        candidates: &[ToolSearchResult],
    ) -> Vec<ToolSearchResult> {
        // 尝试解析 JSON
        let json_value: Value = match serde_json::from_str(response) {
            Ok(v) => v,
            Err(e) => {
                warn!("解析 AI 响应失败：{}，降级为快速搜索", e);
                return candidates.to_vec();
            }
        };

        // 提取 selected_tools
        let selected_tools = json_value
            .get("selected_tools")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        // 构建结果
        let mut results = Vec::new();
        let candidate_map: HashMap<&str, &ToolSearchResult> = candidates
            .iter()
            .map(|r| (r.tool.name.as_str(), r))
            .collect();

        for tool_selection in selected_tools {
            let tool_name = tool_selection
                .get("tool_name")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let relevance_score = tool_selection
                .get("relevance_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5) as f32;

            let reason = tool_selection
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("AI 推荐")
                .to_string();

            if let Some(candidate) = candidate_map.get(tool_name) {
                results.push(ToolSearchResult {
                    tool: candidate.tool.clone(),
                    relevance_score,
                    ranking_score: relevance_score,
                    source: SearchResultSource::Ai,
                });
            }
        }

        // 如果 AI 返回为空，降级为快速搜索
        if results.is_empty() {
            debug!("AI 未返回任何工具，使用快速搜索结果");
            candidates.to_vec()
        } else {
            results
        }
    }

    /// 快速搜索（关键词匹配）
    async fn fast_search(&self, query: &str) -> Vec<ToolSearchResult> {
        let index = self.current_index.read().await;
        let tools = index.search(query, self.config.max_results);

        tools
            .into_iter()
            .map(|tool| {
                let relevance = self.calculate_relevance(&tool, query);
                let ranking = self.calculate_ranking_score(&tool, relevance);
                ToolSearchResult {
                    tool,
                    relevance_score: relevance,
                    ranking_score: ranking,
                    source: SearchResultSource::Keyword,
                }
            })
            .collect()
    }

    /// 计算相关性分数
    fn calculate_relevance(&self, tool: &ToolDefinition, query: &str) -> f32 {
        let query_lower = query.to_lowercase();
        let mut score: f32 = 0.0;

        // 名称完全匹配
        if tool.name.to_lowercase() == query_lower {
            score += 1.0;
        } else if tool.name.to_lowercase().contains(&query_lower) {
            score += 0.8;
        }

        // 描述匹配
        if tool.description.to_lowercase().contains(&query_lower) {
            score += 0.5;
        }

        // 标签匹配
        for tag in &tool.tags {
            if tag.to_lowercase() == query_lower {
                score += 0.3;
            }
        }

        score.min(1.0)
    }

    /// 计算排名分数（综合考虑使用频率等）
    fn calculate_ranking_score(&self, tool: &ToolDefinition, relevance: f32) -> f32 {
        // TODO: 结合 ServiceStats 中的使用频率
        // 当前简化实现：直接使用相关性分数
        relevance
    }

    /// 自动判断是否使用 AI 搜索
    fn should_use_ai_search(&self, query: &str) -> bool {
        // 简单启发式规则：
        // 1. 查询长度 > 阈值 → 可能是复杂任务
        // 2. 包含疑问词 → 需要理解意图
        // 3. 包含多个动词 → 可能需要工具组合

        let query_lower = query.to_lowercase();

        // 规则 1: 长度
        if query.len() > self.config.ai_search_threshold {
            return true;
        }

        // 规则 2: 疑问词
        let question_words = ["如何", "怎么", "怎样", "为什么", "什么", "哪个"];
        if question_words.iter().any(|w| query_lower.contains(w)) {
            return true;
        }

        // 规则 3: 多个动词
        let action_words = [
            "创建", "读取", "写入", "删除", "修改", "分析", "搜索", "下载", "上传",
        ];
        let action_count = action_words
            .iter()
            .filter(|w| query_lower.contains(*w))
            .count();
        if action_count >= 2 {
            return true;
        }

        // 默认用快速搜索
        false
    }

    /// 获取所有工具
    pub async fn get_all_tools(&self) -> Vec<ToolDefinition> {
        let index = self.current_index.read().await;
        index.get_all_tools().into_iter().cloned().collect()
    }

    /// 按分类获取工具
    pub async fn get_tools_by_category(&self, category: &ServiceCategory) -> Vec<ToolDefinition> {
        let index = self.current_index.read().await;
        index
            .get_by_category(category)
            .into_iter()
            .cloned()
            .collect()
    }
}

impl Default for LightweightToolSelector {
    fn default() -> Self {
        Self::new(Vec::new(), None, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_index_creation() {
        let index = ToolIndex::new();
        assert!(index.is_empty());
    }

    #[test]
    fn test_tool_index_add_tool() {
        let mut index = ToolIndex::new();
        let tool = ToolDefinition::new("test_tool", "A test tool", r#"{}"#);
        index.add_tool(tool);
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn test_tool_index_search() {
        let mut index = ToolIndex::new();
        let tool = ToolDefinition::new("read_file", "Read file content", r#"{}"#);
        index.add_tool(tool);

        let results = index.search("read", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "read_file");
    }

    #[test]
    fn test_extract_keywords() {
        let tool = ToolDefinition::new("read_file", "Read file content from disk", r#"{}"#)
            .with_tag("file")
            .with_tag("io");

        let keywords = extract_keywords(&tool);
        assert!(keywords.contains(&"read".to_string()));
        assert!(keywords.contains(&"file".to_string()));
        assert!(keywords.contains(&"content".to_string()));
    }

    #[tokio::test]
    async fn test_lightweight_tool_selector() {
        let tools = vec![
            ToolDefinition::new("read_file", "Read file content", r#"{}"#),
            ToolDefinition::new("write_file", "Write file content", r#"{}"#),
        ];

        let selector = LightweightToolSelector::new_without_ai(tools, None);
        let results = selector.search("read").await;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].tool.name, "read_file");
    }
}
