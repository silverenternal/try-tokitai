//! 自主迭代循环模块
//!
//! 实现 AI 自主的任务分解、规划、执行和审查能力
//!
//! # 架构说明
//!
//! ```text
//! autonomy/
//! ├── task_decomposer.rs          # 任务分解引擎（DAG 依赖分析）
//! ├── iteration_tracker.rs        # 迭代状态追踪器（事件溯源）
//! ├── git_workflow.rs             # 自主 Git 工作流
//! ├── git_workflow_tools.rs       # Git 工作流工具包装器（tokitai ToolProvider）
//! ├── self_improvement_loop.rs    # 自进化闭环系统（PEND-008）
//! ├── gap_detector.rs             # 工具缺口检测器（基于统计）
//! ├── tool_optimizer.rs           # 工具优化器（基于统计）
//! ├── system_reflector.rs         # 系统反思器
//! ├── tool_creator.rs             # 工具创建器（基于模板）
//! ├── prompt_gap_detector.rs      # 基于 Prompt Engineering 的因果推理缺口检测器 ⭐
//! ├── prompt_optimizer.rs         # 基于 Prompt Engineering 的工具优化器 ⭐
//! ├── multi_agent_negotiator.rs   # 多智能体协商器 ⭐
//! └── agents/
//!     ├── mod.rs                  # Agent 系统导出
//!     ├── planner.rs              # 规划 Agent
//!     ├── executor.rs             # 执行 Agent（集成工具矩阵）
//!     └── reviewer.rs             # 审查 Agent
//! ```
//!
//! # 设计原则
//! - 纯文件存储，零数据库依赖
//! - 事件溯源，支持回放
//! - 状态机驱动，支持暂停/恢复
//! - **工具矩阵集成**：通过 tokitai ToolProvider 统一调度
//! - **自进化能力**：从任务历史中发现工具缺口并自动创造
//!
//! # Prompt Engineering 版本（论文计划落实）
//!
//! 根据 `docs/paper_plan/` 目录下的规划，本模块实现了基于 Prompt Engineering 的自进化算法：
//!
//! - **PromptGapDetector**: 使用 Chain-of-Thought + 反事实推理识别真正的因果缺口
//! - **PromptOptimizer**: 使用 Few-Shot Learning 分析工具使用模式
//! - **MultiAgentNegotiator**: 4 个 LLM 智能体通过结构化对话达成共识
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! // 使用 Prompt Engineering 版本的缺口检测
//! let detector = PromptGapDetector::new(llm_client);
//! detector.add_task(task_record);
//! let gaps = detector.detect_gaps().await?;
//!
//! // 使用多智能体协商器
//! let negotiator = MultiAgentNegotiator::new(llm_client);
//! let decision = negotiator.negotiate(&evolution_state).await?;
//! ```

pub mod task_decomposer;
pub mod iteration_tracker;
pub mod git_workflow;
pub mod git_workflow_tools;
pub mod agents;
pub mod self_improvement_loop;
pub mod gap_detector;
pub mod tool_optimizer;
pub mod system_reflector;
pub mod tool_creator;

// Prompt Engineering 版本（论文计划落实）
pub mod prompt_gap_detector;
pub mod prompt_optimizer;
pub mod multi_agent_negotiator;

// Prompt 模板热加载
pub mod prompt_template_loader;

// 混合检测器（融合统计与 Prompt Engineering）
pub mod hybrid_gap_detector;

pub use git_workflow::GitWorkflow;
pub use git_workflow_tools::GitWorkflowTools;
pub use agents::AgentCoordinator;

// 导出 Prompt Engineering 组件（供外部使用）
#[allow(unused_imports)]
pub use prompt_gap_detector::{PromptGapDetector, CausalAnalysisRequest, IdentifiedGap};
#[allow(unused_imports)]
pub use prompt_optimizer::{PromptOptimizer, OptimizationSuggestion, ToolMetrics as OptimizerToolMetrics};
#[allow(unused_imports)]
pub use multi_agent_negotiator::{MultiAgentNegotiator, EvolutionState, EvolutionAction, NegotiationDecision};

// 导出混合检测器组件（供外部使用）
#[allow(unused_imports)]
pub use hybrid_gap_detector::{HybridGapDetector, HybridToolGap, HybridConfig, StatisticalEvidence, CausalEvidence};

// 导出 Prompt 模板加载器
#[allow(unused_imports)]
pub use prompt_template_loader::PromptTemplateLoader;
