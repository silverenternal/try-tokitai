//! 上下文优化器
//!
//! 负责压缩和管理上下文历史，减少 token 开销
//!
//! ## 核心功能
//! - 上下文摘要：将长对话压缩为简洁摘要
//! - 关键信息提取：保留重要事实和决策
//! - 滑动窗口：保留最近的 N 轮对话
//! - 优先级排序：根据重要性决定保留哪些内容

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// 上下文消息类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    /// 用户输入
    User,
    /// 助手回复
    Assistant,
    /// 系统消息
    System,
    /// 工具调用
    ToolCall,
    /// 工具结果
    ToolResult,
}

/// 单个上下文消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMessage {
    /// 消息类型
    pub msg_type: MessageType,
    /// 消息内容
    pub content: String,
    /// 时间戳（秒）
    pub timestamp: u64,
    /// 估计的 token 数量
    pub token_count: usize,
    /// 重要性评分（1-10）
    pub importance: u8,
    /// 是否已被摘要
    pub is_summarized: bool,
}

impl ContextMessage {
    pub fn new(msg_type: MessageType, content: String) -> Self {
        let token_count = Self::estimate_tokens(&content);
        Self {
            msg_type,
            content,
            timestamp: 0,
            token_count,
            importance: 5,
            is_summarized: false,
        }
    }

    pub fn with_importance(msg_type: MessageType, content: String, importance: u8) -> Self {
        let mut msg = Self::new(msg_type, content);
        msg.importance = importance.min(10);
        msg
    }

    /// 估算 token 数量（简化版本：按字符数估算）
    fn estimate_tokens(content: &str) -> usize {
        // 英文：约 4 字符/token，中文：约 1.5 字符/token
        let chinese_chars = content.chars().filter(|c| c.is_ascii()).count();
        let other_chars = content.len() - chinese_chars;
        (chinese_chars / 4) + (other_chars / 2) + 1
    }
}

/// 摘要记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryRecord {
    /// 原始消息数量
    pub original_count: usize,
    /// 摘要内容
    pub summary: String,
    /// 摘要时间
    pub timestamp: u64,
    /// 节省的 token 数
    pub tokens_saved: usize,
}

/// 上下文优化策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationStrategy {
    /// 滑动窗口：只保留最近 N 条消息
    SlidingWindow { window_size: usize },
    /// 重要性过滤：只保留重要性 >= 阈值的消息
    ImportanceFilter { threshold: u8 },
    /// 混合模式：结合滑动窗口和重要性过滤
    Hybrid {
        window_size: usize,
        threshold: u8,
        /// 始终保留的重要消息数量
        keep_important_count: usize,
    },
    /// 摘要压缩：定期生成摘要
    Summarization {
        /// 触发摘要的消息数量阈值
        message_threshold: usize,
        /// 触发摘要的 token 阈值
        token_threshold: usize,
    },
}

/// 上下文优化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizerConfig {
    /// 最大上下文长度（token 数）
    pub max_context_tokens: usize,
    /// 最大消息数量
    pub max_messages: usize,
    /// 优化策略
    pub strategy: OptimizationStrategy,
    /// 是否启用自动优化
    pub auto_optimize: bool,
    /// 是否保留系统消息
    pub keep_system_messages: bool,
    /// 是否保留工具调用历史
    pub keep_tool_history: bool,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: 8000,
            max_messages: 50,
            strategy: OptimizationStrategy::Hybrid {
                window_size: 20,
                threshold: 6,
                keep_important_count: 5,
            },
            auto_optimize: true,
            keep_system_messages: true,
            keep_tool_history: false,
        }
    }
}

/// 上下文优化器
pub struct ContextOptimizer {
    /// 上下文消息队列
    messages: VecDeque<ContextMessage>,
    /// 配置
    config: OptimizerConfig,
    /// 当前 token 总数
    current_tokens: usize,
    /// 摘要历史
    summaries: Vec<SummaryRecord>,
    /// 优化统计
    stats: OptimizationStats,
}

/// 优化统计信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OptimizationStats {
    /// 优化次数
    pub optimization_count: u32,
    /// 累计节省的 token 数
    pub total_tokens_saved: usize,
    /// 被丢弃的消息数
    pub discarded_messages: usize,
    /// 生成的摘要数
    pub generated_summaries: usize,
}

impl ContextOptimizer {
    /// 创建新的上下文优化器
    pub fn new() -> Self {
        Self::with_config(OptimizerConfig::default())
    }

    /// 使用指定配置创建优化器
    pub fn with_config(config: OptimizerConfig) -> Self {
        Self {
            messages: VecDeque::with_capacity(config.max_messages),
            config,
            current_tokens: 0,
            summaries: Vec::new(),
            stats: OptimizationStats::default(),
        }
    }

    /// 添加消息到上下文
    pub fn add_message(&mut self, msg: ContextMessage) {
        self.current_tokens += msg.token_count;
        self.messages.push_back(msg);

        // 检查是否需要优化
        if self.config.auto_optimize {
            self.maybe_optimize();
        }
    }

    /// 添加用户消息
    pub fn add_user_message(&mut self, content: String) {
        self.add_message(ContextMessage::new(MessageType::User, content));
    }

    /// 添加助手消息
    pub fn add_assistant_message(&mut self, content: String) {
        self.add_message(ContextMessage::new(MessageType::Assistant, content));
    }

    /// 添加系统消息
    #[allow(dead_code)]
    pub fn add_system_message(&mut self, content: String) {
        if self.config.keep_system_messages {
            let mut msg = ContextMessage::new(MessageType::System, content);
            msg.importance = 10; // 系统消息总是重要的
            self.add_message(msg);
        }
    }

    /// 添加工具调用
    #[allow(dead_code)]
    pub fn add_tool_call(&mut self, tool_name: String, args: String) {
        if self.config.keep_tool_history {
            let content = format!("调用工具：{}({})", tool_name, args);
            self.add_message(ContextMessage::new(MessageType::ToolCall, content));
        }
    }

    /// 添加工具结果
    #[allow(dead_code)]
    pub fn add_tool_result(&mut self, tool_name: String, result: String) {
        if self.config.keep_tool_history {
            let content = format!("工具 {} 返回：{}", tool_name, result);
            self.add_message(ContextMessage::new(MessageType::ToolResult, content));
        }
    }

    /// 检查是否需要优化
    fn maybe_optimize(&mut self) {
        let needs_optimization = match &self.config.strategy {
            OptimizationStrategy::SlidingWindow { window_size } => {
                self.messages.len() > *window_size
            }
            OptimizationStrategy::ImportanceFilter { .. } => {
                self.current_tokens > self.config.max_context_tokens
            }
            OptimizationStrategy::Hybrid { window_size, .. } => {
                self.messages.len() > *window_size
                    || self.current_tokens > self.config.max_context_tokens
            }
            OptimizationStrategy::Summarization {
                message_threshold,
                token_threshold,
            } => {
                self.messages.len() > *message_threshold
                    || self.current_tokens > *token_threshold
            }
        };

        if needs_optimization {
            self.optimize();
        }
    }

    /// 执行优化
    pub fn optimize(&mut self) -> OptimizationResult {
        let original_tokens = self.current_tokens;
        let original_count = self.messages.len();

        match &self.config.strategy {
            OptimizationStrategy::SlidingWindow { window_size } => {
                self.apply_sliding_window(*window_size);
            }
            OptimizationStrategy::ImportanceFilter { threshold } => {
                self.apply_importance_filter(*threshold);
            }
            OptimizationStrategy::Hybrid {
                window_size,
                threshold,
                keep_important_count,
            } => {
                self.apply_hybrid_strategy(*window_size, *threshold, *keep_important_count);
            }
            OptimizationStrategy::Summarization { .. } => {
                self.apply_summarization();
            }
        }

        let tokens_saved = original_tokens - self.current_tokens;
        let discarded = original_count - self.messages.len();

        self.stats.optimization_count += 1;
        self.stats.total_tokens_saved += tokens_saved;
        self.stats.discarded_messages += discarded;

        OptimizationResult {
            original_token_count: original_tokens,
            new_token_count: self.current_tokens,
            tokens_saved,
            messages_discarded: discarded,
        }
    }

    /// 应用滑动窗口策略
    fn apply_sliding_window(&mut self, window_size: usize) {
        while self.messages.len() > window_size {
            if let Some(msg) = self.messages.pop_front() {
                self.current_tokens -= msg.token_count;
                self.stats.discarded_messages += 1;
            }
        }
    }

    /// 应用重要性过滤策略
    fn apply_importance_filter(&mut self, threshold: u8) {
        let mut retained = VecDeque::new();
        let mut retained_tokens = 0;

        // 总是保留系统消息
        let mut new_messages = VecDeque::new();
        while let Some(msg) = self.messages.pop_front() {
            if msg.msg_type == MessageType::System {
                retained_tokens += msg.token_count;
                new_messages.push_back(msg);
            } else if msg.importance >= threshold {
                retained_tokens += msg.token_count;
                retained.push_back(msg);
            } else {
                self.stats.discarded_messages += 1;
                self.stats.total_tokens_saved += msg.token_count;
            }
        }

        // 按重要性排序，保留最重要的
        let mut retained_vec: Vec<ContextMessage> = retained.drain(..).collect();
        retained_vec.sort_by(|a, b| b.importance.cmp(&a.importance));

        // 添加回消息队列，直到达到 token 限制
        for msg in retained_vec {
            if retained_tokens + msg.token_count <= self.config.max_context_tokens {
                retained_tokens += msg.token_count;
                new_messages.push_back(msg);
            } else {
                self.stats.discarded_messages += 1;
                self.stats.total_tokens_saved += msg.token_count;
            }
        }

        self.messages = new_messages;
        self.current_tokens = retained_tokens;
    }

    /// 应用混合策略
    fn apply_hybrid_strategy(
        &mut self,
        window_size: usize,
        threshold: u8,
        keep_important_count: usize,
    ) {
        // 分离重要和普通消息
        let mut important: Vec<ContextMessage> = Vec::new();
        let mut normal: Vec<ContextMessage> = Vec::new();

        while let Some(msg) = self.messages.pop_front() {
            if msg.importance >= threshold || msg.msg_type == MessageType::System {
                important.push(msg);
            } else {
                normal.push(msg);
            }
        }

        // 按重要性排序重要消息
        important.sort_by(|a, b| b.importance.cmp(&a.importance));

        // 保留最重要的几条
        let kept_important: Vec<ContextMessage> = important
            .into_iter()
            .take(keep_important_count)
            .collect();

        // 普通消息应用滑动窗口
        let kept_normal: Vec<ContextMessage> = normal
            .into_iter()
            .rev()
            .take(window_size)
            .rev()
            .collect();

        // 重建消息队列
        let mut new_tokens = 0;
        let mut new_messages = VecDeque::new();

        for msg in kept_important.into_iter().chain(kept_normal) {
            new_tokens += msg.token_count;
            new_messages.push_back(msg);
        }

        self.messages = new_messages;
        self.current_tokens = new_tokens;
    }

    /// 应用摘要压缩策略
    fn apply_summarization(&mut self) {
        // 简化实现：将旧消息替换为摘要
        if self.messages.len() < 4 {
            return;
        }

        // 取前一半消息生成摘要
        let split_idx = self.messages.len() / 2;
        let to_summarize: Vec<ContextMessage> = self.messages.drain(0..split_idx).collect();

        // 计算被移除的 token
        let removed_tokens: usize = to_summarize.iter().map(|m| m.token_count).sum();
        self.stats.discarded_messages += to_summarize.len();

        // 生成摘要（简化实现：只是拼接关键信息）
        let summary_text = self.generate_summary(&to_summarize);
        let summary_tokens = ContextMessage::estimate_tokens(&summary_text);
        let summary_text_clone = summary_text.clone();

        // 创建摘要消息
        let summary_msg = ContextMessage {
            msg_type: MessageType::System,
            content: summary_text,
            timestamp: 0,
            token_count: summary_tokens,
            importance: 8,
            is_summarized: true,
        };

        self.current_tokens = self.current_tokens - removed_tokens + summary_tokens;
        self.messages.push_front(summary_msg);

        self.stats.generated_summaries += 1;
        self.stats.total_tokens_saved += removed_tokens - summary_tokens;

        self.summaries.push(SummaryRecord {
            original_count: to_summarize.len(),
            summary: summary_text_clone,
            timestamp: 0,
            tokens_saved: removed_tokens - summary_tokens,
        });
    }

    /// 生成摘要（简化版本）
    fn generate_summary(&self, messages: &[ContextMessage]) -> String {
        let mut summary = String::from("【对话摘要】\n");

        // 提取关键信息
        let user_messages: Vec<&ContextMessage> = messages
            .iter()
            .filter(|m| m.msg_type == MessageType::User)
            .collect();
        let assistant_messages: Vec<&ContextMessage> = messages
            .iter()
            .filter(|m| m.msg_type == MessageType::Assistant)
            .collect();

        summary.push_str(&format!(
            "用户提出了 {} 个问题/请求，助手回复了 {} 次。\n",
            user_messages.len(),
            assistant_messages.len()
        ));

        // 保留最后一条用户消息的关键内容
        if let Some(last_user) = user_messages.last() {
            let preview = if last_user.content.len() > 50 {
                format!("{}...", &last_user.content[..50])
            } else {
                last_user.content.clone()
            };
            summary.push_str(&format!("最近请求：{}\n", preview));
        }

        summary
    }

    /// 获取当前上下文消息
    #[allow(dead_code)]
    pub fn get_messages(&self) -> &VecDeque<ContextMessage> {
        &self.messages
    }

    /// 获取当前 token 数
    pub fn current_tokens(&self) -> usize {
        self.current_tokens
    }

    /// 获取消息数量
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// 获取优化统计
    pub fn get_stats(&self) -> &OptimizationStats {
        &self.stats
    }

    /// 清空上下文
    pub fn clear(&mut self) {
        self.messages.clear();
        self.current_tokens = 0;
    }

    /// 设置重要性标记
    #[allow(dead_code)]
    pub fn set_importance(&mut self, index: usize, importance: u8) {
        if let Some(msg) = self.messages.get_mut(index) {
            msg.importance = importance.min(10);
        }
    }

    /// 标记最后一条消息为高重要性
    #[allow(dead_code)]
    pub fn mark_last_as_important(&mut self) {
        if let Some(msg) = self.messages.back_mut() {
            msg.importance = 9;
        }
    }
}

/// 优化结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    /// 原始 token 数
    pub original_token_count: usize,
    /// 优化后 token 数
    pub new_token_count: usize,
    /// 节省的 token 数
    pub tokens_saved: usize,
    /// 丢弃的消息数
    pub messages_discarded: usize,
}

impl Default for ContextOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_messages() {
        let mut optimizer = ContextOptimizer::new();

        optimizer.add_user_message("你好".to_string());
        optimizer.add_assistant_message("你好！有什么可以帮助你的吗？".to_string());

        assert_eq!(optimizer.message_count(), 2);
        assert!(optimizer.current_tokens() > 0);
    }

    #[test]
    fn test_sliding_window() {
        let config = OptimizerConfig {
            max_messages: 10,
            strategy: OptimizationStrategy::SlidingWindow { window_size: 5 },
            ..Default::default()
        };

        let mut optimizer = ContextOptimizer::with_config(config);

        // 添加超过窗口大小的消息
        for i in 0..10 {
            optimizer.add_user_message(format!("消息 {}", i));
        }

        // 应该只保留最后 5 条
        assert!(optimizer.message_count() <= 5);
    }

    #[test]
    fn test_importance_filter() {
        let config = OptimizerConfig {
            max_context_tokens: 1000,
            strategy: OptimizationStrategy::ImportanceFilter { threshold: 7 },
            ..Default::default()
        };

        let mut optimizer = ContextOptimizer::with_config(config);

        // 添加低重要性消息
        optimizer.add_message(ContextMessage::with_importance(
            MessageType::User,
            "普通消息".to_string(),
            3,
        ));

        // 添加高重要性消息
        optimizer.add_message(ContextMessage::with_importance(
            MessageType::User,
            "重要消息".to_string(),
            8,
        ));

        optimizer.optimize();

        // 低重要性消息应该被丢弃
        assert_eq!(optimizer.message_count(), 1);
    }

    #[test]
    fn test_token_estimation() {
        let optimizer = ContextOptimizer::new();

        let short_msg = ContextMessage::new(MessageType::User, "短消息".to_string());
        let long_msg = ContextMessage::new(
            MessageType::User,
            "这是一条更长的消息，包含更多的字符和内容".to_string(),
        );

        assert!(long_msg.token_count > short_msg.token_count);
    }
}
