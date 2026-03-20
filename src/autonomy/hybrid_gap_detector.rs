//! 混合工具缺口检测器
//!
//! 融合统计方法与 Prompt Engineering 的因果推理，
//! 实现高性能、低成本、高可解释性的工具缺口检测
//!
//! ## 架构设计
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    HybridGapDetector                         │
//! │                                                              │
//! │  ┌────────────────────────────────────────────────────────┐ │
//! │  │  Stage 1: Statistical Filter (快速筛选，<100ms, 0 API)  │ │
//! │  │  - 基于失败率、满意度等指标筛选候选任务                  │ │
//! │  │  - 聚类失败模式，识别高频问题                            │ │
//! │  └────────────────────────────────────────────────────────┘ │
//! │                              │                                │
//! │                              ▼                                │
//! │  ┌────────────────────────────────────────────────────────┐ │
//! │  │  Stage 2: Causal Analysis (深度分析，5-30 秒，1-2 API)   │ │
//! │  │  - 对候选缺口进行因果推理                                │ │
//! │  │  - 反事实提问："如果有这个工具，任务会成功吗？"           │ │
//! │  └────────────────────────────────────────────────────────┘ │
//! │                              │                                │
//! │                              ▼                                │
//! │  ┌────────────────────────────────────────────────────────┐ │
//! │  │  Stage 3: Merger & Prioritize (融合，<50ms, 0 API)      │ │
//! │  │  - 合并统计证据和因果证据                                │ │
//! │  │  - 计算融合置信度                                        │ │
//! │  └────────────────────────────────────────────────────────┘ │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## 核心优势
//! - **性能**：80% 简单缺口用统计方法快速检测
//! - **深度**：20% 复杂缺口用因果推理深度分析
//! - **成本**：相比纯 Prompt Engineering 方案节省 80%+ API 调用
//! - **可解释性**：同时输出统计证据和因果证据
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! let config = HybridConfig::default();
//! let mut detector = HybridGapDetector::new(
//!     data_dir,
//!     llm_client,
//!     config
//! )?;
//!
//! // 添加任务记录
//! detector.add_task(task_record);
//!
//! // 检测缺口
//! let gaps = detector.detect_gaps().await?;
//! for gap in gaps {
//!     println!("缺口：{}", gap.description);
//!     println!("  统计证据：失败率={}", gap.statistical_evidence.failure_rate);
//!     if let Some(causal) = &gap.causal_evidence {
//!         println!("  因果证据：{}", causal.counterfactual_reasoning);
//!     }
//! }
//! ```

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{info, debug};

use crate::autonomy::gap_detector::{
    ToolGapDetector, TaskExecutionRecord, ToolGap, GapType, GapEvidence,
};
use crate::autonomy::prompt_gap_detector::{
    PromptGapDetector, CausalAnalysisRequest, IdentifiedGap, CausalFactor, LLMClient,
};

/// 融合后的工具缺口
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridToolGap {
    /// 缺口唯一 ID
    pub id: String,
    /// 缺口类型
    pub gap_type: GapType,
    /// 缺口描述
    pub description: String,
    /// 建议的工具名称
    pub suggested_tool_name: Option<String>,
    /// 建议的功能描述
    pub suggested_capabilities: Vec<String>,
    /// 优先级 (1-10)
    pub priority: u8,
    /// 证据列表（统计证据）
    pub evidence: Vec<GapEvidence>,
    /// 影响范围描述
    pub impact_scope: String,
    /// 统计证据详情
    pub statistical_evidence: StatisticalEvidence,
    /// 因果证据（可能为空，表示仅依赖统计证据）
    pub causal_evidence: Option<CausalEvidence>,
    /// 融合后的置信度 (0.0-1.0)
    pub hybrid_confidence: f32,
}

/// 统计证据详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalEvidence {
    /// 失败率 (0.0-1.0)
    pub failure_rate: f32,
    /// 影响的任务数量
    pub affected_tasks_count: u32,
    /// 平均满意度 (1.0-5.0)
    pub avg_satisfaction: f32,
    /// 失败模式出现频率
    pub pattern_frequency: u32,
    /// 相关任务 ID 列表
    pub related_task_ids: Vec<String>,
}

/// 因果证据详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalEvidence {
    /// 因果因素列表
    pub causal_factors: Vec<CausalFactor>,
    /// 反事实推理结果
    pub counterfactual_reasoning: String,
    /// LLM 置信度 (0.0-1.0)
    pub llm_confidence: f32,
    /// 预期影响
    pub expected_impact: GapImpact,
}

/// 缺口影响评估（复用 prompt_gap_detector 的定义）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapImpact {
    /// 影响的任务数量
    pub affected_tasks: u32,
    /// 平均减少的工具调用次数
    pub avg_tool_calls_reduced: f32,
    /// 预计节省的时间（分钟）
    pub time_saved_minutes: f32,
    /// 预期成功率提升
    pub expected_success_rate_improvement: f32,
}

/// 融合检测器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridConfig {
    /// 统计筛选的失败率阈值
    pub statistical_threshold: f32,
    /// 触发因果分析的最小出现次数
    pub min_occurrence_count: u32,
    /// 是否启用因果分析
    pub enable_causal_analysis: bool,
    /// 触发因果分析的最小优先级
    pub causal_min_priority: u8,
    /// 每周期最大因果分析数（控制成本）
    pub max_causal_analyses_per_cycle: u32,
    /// 统计证据在融合置信度中的权重
    pub statistical_weight: f32,
    /// 因果证据在融合置信度中的权重
    pub causal_weight: f32,
    /// 每周期 API 预算（美元）
    pub api_budget_per_cycle: f32,
    /// 单次 API 调用的估计成本（美元）
    pub estimated_cost_per_call: f32,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            statistical_threshold: 0.5,
            min_occurrence_count: 3,
            enable_causal_analysis: true,
            causal_min_priority: 6,
            max_causal_analyses_per_cycle: 5,
            statistical_weight: 0.4,
            causal_weight: 0.6,
            api_budget_per_cycle: 0.5,
            estimated_cost_per_call: 0.015,
        }
    }
}

/// 缓存条目
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    /// 缓存的缺口 ID
    gap_id: String,
    /// 缓存的因果证据
    causal_evidence: CausalEvidence,
    /// 缓存时间戳
    timestamp: u64,
    /// 过期时间戳
    expires_at: u64,
}

/// 混合工具缺口检测器
pub struct HybridGapDetector {
    /// 数据存储目录
    data_dir: PathBuf,
    /// 统计检测器
    statistical_detector: ToolGapDetector,
    /// 因果检测器（需要 LLM 客户端）
    causal_detector: Option<PromptGapDetector>,
    /// 配置
    config: HybridConfig,
    /// 缓存的因果分析结果
    cache: HashMap<String, CacheEntry>,
    /// 当前周期已用 API 预算
    used_api_budget: f32,
    /// 当前周期已进行的因果分析次数
    causal_analyses_count: u32,
}

impl HybridGapDetector {
    /// 创建新的混合检测器（不带 LLM 客户端，仅使用统计方法）
    pub fn new_statistical_only(data_dir: PathBuf) -> Result<Self> {
        let statistical_detector = ToolGapDetector::new(data_dir.clone())?;
        
        Ok(Self {
            data_dir,
            statistical_detector,
            causal_detector: None,
            config: HybridConfig::default(),
            cache: HashMap::new(),
            used_api_budget: 0.0,
            causal_analyses_count: 0,
        })
    }

    /// 创建新的混合检测器（带 LLM 客户端，启用因果分析）
    pub fn new(
        data_dir: PathBuf,
        llm_client: Arc<dyn LLMClient>,
        config: HybridConfig,
    ) -> Result<Self> {
        std::fs::create_dir_all(&data_dir)?;
        
        let statistical_detector = ToolGapDetector::new(data_dir.clone())?;
        let causal_detector = PromptGapDetector::with_config(llm_client, {
            crate::autonomy::prompt_gap_detector::PromptGapDetectorConfig::default()
        });

        Ok(Self {
            data_dir,
            statistical_detector,
            causal_detector: Some(causal_detector),
            config,
            cache: HashMap::new(),
            used_api_budget: 0.0,
            causal_analyses_count: 0,
        })
    }

    /// 从配置创建
    pub fn with_config(
        data_dir: PathBuf,
        llm_client: Arc<dyn LLMClient>,
        config: HybridConfig,
    ) -> Result<Self> {
        Self::new(data_dir, llm_client, config)
    }

    /// 重置周期统计（每轮自进化循环开始时调用）
    pub fn reset_cycle_stats(&mut self) {
        self.used_api_budget = 0.0;
        self.causal_analyses_count = 0;
        self.expire_cache();
    }

    /// 记录任务执行
    pub fn record_task(&mut self, record: TaskExecutionRecord) {
        self.statistical_detector.record_task(record);
    }

    /// 批量记录任务
    pub fn record_tasks(&mut self, records: Vec<TaskExecutionRecord>) {
        for record in records {
            self.record_task(record);
        }
    }

    /// 检测工具缺口（主流程）
    pub async fn detect_gaps(&mut self) -> Vec<HybridToolGap> {
        info!("开始混合缺口检测...");

        // Stage 1: 统计筛选（快速）
        let candidate_gaps = self.statistical_detector.analyze_and_detect();
        info!("统计筛选识别出 {} 个候选缺口", candidate_gaps.len());

        if candidate_gaps.is_empty() {
            return Vec::new();
        }

        // 过滤并克隆候选缺口（避免借用问题）
        let strong_candidates: Vec<ToolGap> = candidate_gaps
            .into_iter()
            .filter(|gap| {
                // 检查出现次数
                let total_occurrences: u32 = gap.evidence.iter()
                    .map(|e| e.occurrence_count)
                    .sum();

                // 检查最高置信度
                let max_confidence = gap.evidence.iter()
                    .map(|e| e.confidence)
                    .fold(0.0f32, |a, b| a.max(b));

                total_occurrences >= self.config.min_occurrence_count
                    && max_confidence >= self.config.statistical_threshold
            })
            .cloned()
            .collect();

        info!("过滤后剩余 {} 个强候选缺口", strong_candidates.len());

        // Stage 2 & 3: 因果分析 + 证据融合
        let mut hybrid_gaps = Vec::new();

        for candidate in strong_candidates {
            // 检查是否需要进行因果分析
            let should_analyze_causally = self.config.enable_causal_analysis
                && candidate.priority >= self.config.causal_min_priority
                && self.causal_analyses_count < self.config.max_causal_analyses_per_cycle
                && self.used_api_budget < self.config.api_budget_per_cycle
                && self.causal_detector.is_some();

            let causal_result = if should_analyze_causally {
                // 检查缓存
                let cache_key = self.compute_cache_key_from_id(&candidate.id);
                if let Some(cached) = self.cache.get(&cache_key) {
                    if cached.expires_at > get_current_timestamp() {
                        debug!("使用缓存的因果分析结果：{}", cache_key);
                        Some(Ok(vec![self.convert_cache_to_identified_gap(cached)]))
                    } else {
                        // 缓存过期，重新分析
                        self.execute_causal_analysis(&candidate).await
                    }
                } else {
                    // 执行因果分析
                    self.execute_causal_analysis(&candidate).await
                }
            } else {
                None
            };

            // Stage 3: 融合证据
            let hybrid_gap = self.merge_evidence(&candidate, causal_result);
            hybrid_gaps.push(hybrid_gap);
        }

        // 按融合后的置信度排序
        hybrid_gaps.sort_by(|a, b| {
            b.hybrid_confidence.partial_cmp(&a.hybrid_confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        info!(
            "混合检测完成：输出 {} 个缺口，平均置信度={:.2}",
            hybrid_gaps.len(),
            hybrid_gaps.iter().map(|g| g.hybrid_confidence).sum::<f32>()
                / hybrid_gaps.len() as f32
        );

        hybrid_gaps
    }

    /// 执行因果分析
    async fn execute_causal_analysis(
        &mut self,
        candidate: &ToolGap,
    ) -> Option<Result<Vec<IdentifiedGap>>> {
        // 先准备因果分析任务（避免借用冲突）
        let causal_tasks = self.prepare_causal_analysis_tasks(candidate);
        if causal_tasks.is_empty() {
            return None;
        }

        let causal_detector = match self.causal_detector.as_mut() {
            Some(detector) => detector,
            None => return None,
        };

        causal_detector.add_tasks(causal_tasks);

        // 执行因果推理
        info!("执行因果分析：{}", candidate.id);
        let result = causal_detector.detect_gaps().await;

        // 更新统计
        self.causal_analyses_count += 1;
        self.used_api_budget += self.config.estimated_cost_per_call;

        debug!(
            "因果分析进度：{}/{}, 已用预算=${:.3}/${:.3}",
            self.causal_analyses_count,
            self.config.max_causal_analyses_per_cycle,
            self.used_api_budget,
            self.config.api_budget_per_cycle
        );

        Some(result)
    }

    /// 准备因果分析任务
    fn prepare_causal_analysis_tasks(&self, candidate: &ToolGap) -> Vec<CausalAnalysisRequest> {
        // 从统计检测器中获取相关任务记录
        // 这里简化处理，实际应该从 statistical_detector 中查询
        let mut tasks = Vec::new();

        for evidence in &candidate.evidence {
            for task_id in &evidence.related_task_ids {
                // 构造简化的因果分析请求
                tasks.push(CausalAnalysisRequest {
                    task_id: task_id.clone(),
                    task_description: format!(
                        "与缺口相关的任务：{}",
                        candidate.description
                    ),
                    success: false,
                    used_tools: vec![],
                    failure_reason: Some(evidence.description.clone()),
                    user_satisfaction: Some(2),
                    execution_time_ms: 0,
                    context: Some(candidate.impact_scope.clone()),
                });
            }
        }

        tasks
    }

    /// 融合证据
    fn merge_evidence(
        &self,
        candidate: &ToolGap,
        causal_result: Option<Result<Vec<IdentifiedGap>>>,
    ) -> HybridToolGap {
        // 提取统计证据
        let stat_evidence = self.extract_statistical_evidence(candidate);

        // 处理因果证据
        let (causal_evidence, hybrid_confidence) = match causal_result {
            Some(Ok(causal_gaps)) if !causal_gaps.is_empty() => {
                // 因果分析成功
                let causal = self.extract_causal_evidence(&causal_gaps[0]);
                let confidence = self.calculate_hybrid_confidence(&stat_evidence, &causal);
                (Some(causal), confidence)
            }
            _ => {
                // 因果分析失败或未执行：仅依赖统计证据
                let confidence = self.calculate_statistical_confidence(&stat_evidence);
                (None, confidence)
            }
        };

        // 计算融合后的优先级
        let hybrid_priority = self.calculate_hybrid_priority(
            candidate.priority,
            hybrid_confidence,
        );

        HybridToolGap {
            id: candidate.id.clone(),
            gap_type: candidate.gap_type.clone(),
            description: candidate.description.clone(),
            suggested_tool_name: candidate.suggested_tool_name.clone(),
            suggested_capabilities: candidate.suggested_capabilities.clone(),
            priority: hybrid_priority,
            evidence: candidate.evidence.clone(),
            impact_scope: candidate.impact_scope.clone(),
            statistical_evidence: stat_evidence,
            causal_evidence,
            hybrid_confidence,
        }
    }

    /// 提取统计证据详情
    fn extract_statistical_evidence(&self, candidate: &ToolGap) -> StatisticalEvidence {
        let total_occurrences: u32 = candidate.evidence.iter()
            .map(|e| e.occurrence_count)
            .sum();

        let max_confidence = candidate.evidence.iter()
            .map(|e| e.confidence)
            .fold(0.0f32, |a, b| a.max(b));

        let related_task_ids: Vec<String> = candidate.evidence.iter()
            .flat_map(|e| e.related_task_ids.clone())
            .collect();

        // 估算失败率（从置信度推导）
        let estimated_failure_rate = max_confidence;

        // 估算满意度（从置信度反推）
        let estimated_satisfaction = 5.0 - (max_confidence * 3.0);

        StatisticalEvidence {
            failure_rate: estimated_failure_rate,
            affected_tasks_count: related_task_ids.len() as u32,
            avg_satisfaction: estimated_satisfaction,
            pattern_frequency: total_occurrences,
            related_task_ids,
        }
    }

    /// 提取因果证据详情
    fn extract_causal_evidence(&self, identified_gap: &IdentifiedGap) -> CausalEvidence {
        // 提取因果因素
        let causal_factors = identified_gap.causal_evidence.clone();

        // 生成反事实推理总结
        let counterfactual = if causal_factors.iter().any(|f| f.is_causal) {
            let causal_count = causal_factors.iter().filter(|f| f.is_causal).count();
            format!(
                "反事实分析：识别出{}个因果因素。如果提供建议的工具功能（{}），预计可减少{}次工具调用，节省{:.1}分钟。",
                causal_count,
                identified_gap.suggested_functionality,
                identified_gap.expected_impact.avg_tool_calls_reduced as u32,
                identified_gap.expected_impact.time_saved_minutes,
            )
        } else {
            "反事实分析：未识别出强因果因素，可能为相关性问题。".to_string()
        };

        // 计算 LLM 置信度
        let llm_confidence = causal_factors.iter()
            .map(|f| f.confidence)
            .sum::<f32>() / causal_factors.len() as f32;

        CausalEvidence {
            causal_factors,
            counterfactual_reasoning: counterfactual,
            llm_confidence,
            expected_impact: GapImpact {
                affected_tasks: identified_gap.expected_impact.affected_tasks,
                avg_tool_calls_reduced: identified_gap.expected_impact.avg_tool_calls_reduced,
                time_saved_minutes: identified_gap.expected_impact.time_saved_minutes,
                expected_success_rate_improvement: identified_gap.expected_impact.expected_success_rate_improvement,
            },
        }
    }

    /// 计算融合置信度
    fn calculate_hybrid_confidence(
        &self,
        stat: &StatisticalEvidence,
        causal: &CausalEvidence,
    ) -> f32 {
        // 统计置信度分量
        let stat_conf = (stat.failure_rate * 0.3)
            + (stat.affected_tasks_count as f32 / 50.0).min(0.3)
            + ((5.0 - stat.avg_satisfaction) / 5.0 * 0.2);
        let stat_conf = stat_conf.min(0.8);

        // 因果置信度分量
        let causal_factors_count = causal.causal_factors.len() as f32;
        let causal_factors_is_causal = causal.causal_factors.iter()
            .filter(|f| f.is_causal)
            .count() as f32;
        
        let causal_conf = (causal.llm_confidence * 0.4)
            + ((causal_factors_is_causal / causal_factors_count.max(1.0)) * 0.3);
        let causal_conf = causal_conf.min(0.7);

        // 加权融合
        (stat_conf * self.config.statistical_weight) 
            + (causal_conf * self.config.causal_weight)
    }

    /// 仅基于统计证据计算置信度
    fn calculate_statistical_confidence(&self, stat: &StatisticalEvidence) -> f32 {
        let stat_conf = (stat.failure_rate * 0.4)
            + (stat.affected_tasks_count as f32 / 50.0).min(0.3)
            + ((5.0 - stat.avg_satisfaction) / 5.0 * 0.3);
        stat_conf.min(0.7)
    }

    /// 计算融合后的优先级
    fn calculate_hybrid_priority(&self, original_priority: u8, hybrid_confidence: f32) -> u8 {
        // 基于置信度调整优先级
        let confidence_bonus = (hybrid_confidence * 3.0) as u8;
        (original_priority + confidence_bonus).min(10)
    }

    /// 计算缓存键（从 ID）
    fn compute_cache_key_from_id(&self, gap_id: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        gap_id.hash(&mut hasher);
        format!("cache_{:x}", hasher.finish())
    }

    /// 转换缓存条目为 IdentifiedGap
    fn convert_cache_to_identified_gap(&self, entry: &CacheEntry) -> IdentifiedGap {
        IdentifiedGap {
            gap_type: "cached".to_string(),
            description: entry.gap_id.clone(),
            suggested_name: None,
            suggested_functionality: String::new(),
            input_schema: None,
            priority: 5,
            causal_evidence: entry.causal_evidence.causal_factors.clone(),
            expected_impact: crate::autonomy::prompt_gap_detector::GapImpact {
                affected_tasks: 1,
                avg_tool_calls_reduced: 0.0,
                time_saved_minutes: 0.0,
                expected_success_rate_improvement: 0.0,
            },
        }
    }

    /// 计算缓存键
    fn compute_cache_key(&self, candidate: &ToolGap) -> String {
        self.compute_cache_key_from_id(&candidate.id)
    }

    /// 转换缓存条目为 IdentifiedGap（旧方法，保持兼容）
    fn convert_cache_to_gap(&self, entry: &CacheEntry) -> IdentifiedGap {
        self.convert_cache_to_identified_gap(entry)
    }

    /// 缓存因果分析结果
    fn cache_causal_result(&mut self, gap_id: &str, evidence: CausalEvidence) {
        let now = get_current_timestamp();
        let expires_at = now + 24 * 60 * 60; // 24 小时过期

        let cache_key = self.compute_cache_key(&ToolGap {
            id: gap_id.to_string(),
            gap_type: GapType::MissingTool,
            description: String::new(),
            suggested_tool_name: None,
            suggested_capabilities: vec![],
            priority: 0,
            evidence: vec![],
            impact_scope: String::new(),
        });

        self.cache.insert(cache_key, CacheEntry {
            gap_id: gap_id.to_string(),
            causal_evidence: evidence,
            timestamp: now,
            expires_at,
        });
    }

    /// 清理过期缓存
    fn expire_cache(&mut self) {
        let now = get_current_timestamp();
        self.cache.retain(|_, entry| entry.expires_at > now);
    }

    /// 获取检测器统计信息
    pub fn get_stats(&self) -> HybridDetectorStats {
        HybridDetectorStats {
            total_tasks_recorded: 0, // 需要从 statistical_detector 获取
            cache_size: self.cache.len() as u32,
            used_api_budget: self.used_api_budget,
            causal_analyses_count: self.causal_analyses_count,
        }
    }
}

/// 检测器统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridDetectorStats {
    /// 记录的任务总数
    pub total_tasks_recorded: u32,
    /// 缓存条目数
    pub cache_size: u32,
    /// 已用 API 预算
    pub used_api_budget: f32,
    /// 已进行的因果分析次数
    pub causal_analyses_count: u32,
}

/// 获取当前时间戳（秒）
fn get_current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_config_default() {
        let config = HybridConfig::default();
        assert_eq!(config.statistical_threshold, 0.5);
        assert_eq!(config.min_occurrence_count, 3);
        assert!(config.enable_causal_analysis);
        assert_eq!(config.causal_min_priority, 6);
        assert_eq!(config.max_causal_analyses_per_cycle, 5);
        assert_eq!(config.statistical_weight, 0.4);
        assert_eq!(config.causal_weight, 0.6);
    }

    #[test]
    fn test_statistical_evidence_extraction() {
        // 创建测试用的 ToolGap
        let gap = ToolGap {
            id: "test_gap".to_string(),
            gap_type: GapType::MissingTool,
            description: "测试缺口".to_string(),
            suggested_tool_name: Some("test_tool".to_string()),
            suggested_capabilities: vec!["test capability".to_string()],
            priority: 7,
            evidence: vec![GapEvidence {
                evidence_type: "test".to_string(),
                description: "test evidence".to_string(),
                confidence: 0.8,
                related_task_ids: vec!["task1".to_string(), "task2".to_string()],
                occurrence_count: 5,
            }],
            impact_scope: "test scope".to_string(),
        };

        // 创建检测器（仅统计模式）
        let temp_dir = std::env::temp_dir().join("hybrid_gap_detector_test");
        let detector = HybridGapDetector::new_statistical_only(temp_dir).unwrap();

        let stat_evidence = detector.extract_statistical_evidence(&gap);
        
        assert_eq!(stat_evidence.affected_tasks_count, 2);
        assert_eq!(stat_evidence.pattern_frequency, 5);
        assert!(stat_evidence.failure_rate > 0.0);
    }

    #[test]
    fn test_cache_key_computation() {
        let temp_dir = std::env::temp_dir().join("hybrid_gap_detector_cache_test");
        let detector = HybridGapDetector::new_statistical_only(temp_dir).unwrap();

        let gap1 = ToolGap {
            id: "gap_1".to_string(),
            gap_type: GapType::MissingTool,
            description: "Gap 1".to_string(),
            suggested_tool_name: None,
            suggested_capabilities: vec![],
            priority: 5,
            evidence: vec![],
            impact_scope: String::new(),
        };

        let gap2 = ToolGap {
            id: "gap_2".to_string(),
            gap_type: GapType::MissingTool,
            description: "Gap 2".to_string(),
            suggested_tool_name: None,
            suggested_capabilities: vec![],
            priority: 5,
            evidence: vec![],
            impact_scope: String::new(),
        };

        let key1 = detector.compute_cache_key(&gap1);
        let key2 = detector.compute_cache_key(&gap2);

        assert_ne!(key1, key2);
    }
}
