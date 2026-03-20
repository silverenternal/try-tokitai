# 核心机制设计详解（Prompt Engineering 版本）

> 本文档详细描述自进化系统的四个核心机制的实现细节
> **更新**：采用 Prompt Engineering 方法，无需训练模型

---

## 设计哲学转变（2026-03-20）

### 原方案：训练专用模型

```
问题：需要 GPU、训练时间长、维护成本高
方案：训练 PPO 模型学习进化策略、训练代码生成模型
硬件：RTX 3090/A100
时间：12-20 周
成本：$500-2000
```

### 新方案：Prompt Engineering

```
洞察：现代 LLM（Qwen3.5/4.0、GPT-4）已具备推理能力
方案：精心设计的 Prompt 激发 LLM 已有能力
硬件：无需 GPU，仅需 API 调用
时间：8 周
成本：<$150
```

---

## 1. PromptGapDetector（基于 Prompt 的缺口检测器）

### 设计目标

使用 Prompt Engineering 替代复杂的因果图学习，直接让 LLM 进行因果推理。

### 核心 Prompt 模板

```rust
/// 因果分析 Prompt 模板
pub const CAUSAL_ANALYSIS_PROMPT: &str = r#"
你是因果推断专家。请分析以下任务失败的根本原因。

## 任务历史
{task_history}

## 分析步骤
请按以下步骤进行推理（Chain-of-Thought）：

1. **列出所有可能的失败因素**
   - 工具缺失
   - 工具功能不足
   - 工具使用错误
   - 外部因素

2. **对每个因素进行因果判断**
   - 这是相关性还是因果性？
   - 如果消除这个因素，任务会成功吗？

3. **识别真正的工具缺口**
   - 缺少的工具是什么？
   - 如果有这个工具，任务会成功吗？（反事实推理）
   - 这个工具的建议功能是什么？

4. **输出 JSON 格式报告**
{{
    "causal_factors": [...],
    "identified_gaps": [...],
    "confidence": 0.0-1.0
}}
"#;
```

### Few-Shot 示例

```rust
/// Few-Shot 示例库
pub const FEW_SHOT_EXAMPLES: &[CausalExample] = &[
    CausalExample {
        task: "批量下载 100 张图片并压缩",
        failure: "手动逐个下载，耗时 30 分钟，用户满意度 2/5",
        used_tools: ["download_file", "zip_files"],
        causal_analysis: r#"
1. 可能因素：
   - 缺少批量下载工具 → 因果性（如果有批量下载，任务会快 10 倍）
   - 网络速度慢 → 相关性（即使快也需手动操作 100 次）

2. 真正的工具缺口：
   - batch_download: 根据 URL 模式批量下载文件

3. 预期影响：
   - 工具调用从 200 次减少到 2 次
   - 执行时间从 30 分钟减少到 3 分钟
"#,
        output: json!({
            "gap_type": "missing_tool",
            "suggested_name": "batch_download",
            "priority": 9
        })
    },
    // ... 更多示例
];
```

### 实现代码

```rust
pub struct PromptGapDetector {
    llm_client: Arc<dyn LLMClient>,
    task_history: Arc<RwLock<Vec<TaskRecord>>>,
    few_shot_examples: Vec<CausalExample>,
    validator: GapValidator,
}

impl PromptGapDetector {
    pub async fn detect_gaps(&self) -> Result<Vec<ToolGap>> {
        // 1. 收集失败任务
        let failed_tasks = self.collect_failed_tasks();
        
        // 2. 构建 Prompt
        let prompt = self.build_causal_prompt(&failed_tasks);
        
        // 3. LLM 推理（带 JSON Schema 约束）
        let response = self.llm_client
            .chat_with_schema(&prompt, &CAUSAL_SCHEMA)
            .await?;
        
        // 4. 解析结果
        let analysis: CausalAnalysis = serde_json::from_str(&response)?;
        
        // 5. 验证合理性
        let validated_gaps = self.validator.validate(analysis.identified_gaps)?;
        
        Ok(validated_gaps)
    }
    
    fn build_causal_prompt(&self, tasks: &[TaskRecord]) -> String {
        // 插入 Few-Shot 示例
        let examples = self.few_shot_examples
            .iter()
            .take(3)
            .map(|e| e.format())
            .collect::<Vec<_>>()
            .join("\n\n");
        
        // 插入任务历史
        let history = tasks.iter()
            .map(|t| format!("- {}: {}", t.id, t.description))
            .collect::<Vec<_>>()
            .join("\n");
        
        format!(CAUSAL_ANALYSIS_PROMPT, 
            examples = examples,
            history = history
        )
    }
}
```

### 输出格式

```json
{
    "gap_type": "missing_tool",
    "description": "缺少批量下载文件的工具",
    "suggested_name": "batch_download",
    "suggested_functionality": "根据 URL 模式批量下载多个文件",
    "input_schema": {
        "type": "object",
        "properties": {
            "url_pattern": {"type": "string", "description": "URL 模式，如 http://example.com/img_{001-100}.jpg"},
            "output_dir": {"type": "string"}
        },
        "required": ["url_pattern"]
    },
    "priority": 0.85,
    "causal_evidence": [
        {
            "factor": "缺少批量下载工具",
            "is_causal": true,
            "confidence": 0.92,
            "reasoning": "如果有 batch_download，工具调用从 200 次减少到 2 次"
        }
    ],
    "expected_impact": {
        "affected_tasks": 15,
        "avg_tool_calls_reduced": 45.0,
        "time_saved_minutes": 27
    }
}
```

---

## 2. PromptOptimizer（基于 Prompt 的工具优化器）

### 设计目标

使用 Prompt + Few-Shot 学习分析工具使用模式，决定合并/废弃/改进。

### 核心 Prompt 模板

```rust
pub const OPTIMIZER_PROMPT: &str = r#"
你是工具库优化专家。请分析以下工具的健康状态。

## 工具库状态
{tool_stats}

## 分析维度

1. **使用率分析**
   - 找出使用率最低的工具（<P25）
   - 分析原因：功能冗余？功能太窄？命名不清？

2. **失败率分析**
   - 找出失败率最高的工具（>30%）
   - 分析原因：输入验证不足？错误处理不当？

3. **冗余分析**
   - 找出功能重叠的工具
   - 建议合并或废弃

## 输出格式
{{
    "optimizations": [
        {{
            "type": "merge|deprecate|improve|rename",
            "affected_tools": [...],
            "rationale": "...",
            "expected_benefit": "..."
        }}
    ]
}}
"#;
```

### 实现代码

```rust
pub struct PromptOptimizer {
    llm_client: Arc<dyn LLMClient>,
    tool_registry: Arc<RwLock<ToolRegistry>>,
    history: Vec<OptimizationDecision>,
    validator: OptimizationValidator,
}

impl PromptOptimizer {
    pub async fn optimize_tools(&self) -> Result<Vec<OptimizationSuggestion>> {
        // 1. 收集工具统计
        let stats = self.collect_tool_stats();
        
        // 2. 构建 Prompt
        let prompt = self.build_optimizer_prompt(&stats);
        
        // 3. LLM 推理
        let response = self.llm_client
            .chat_with_schema(&prompt, &OPTIMIZER_SCHEMA)
            .await?;
        
        // 4. 解析结果
        let suggestions: OptimizationResult = serde_json::from_str(&response)?;
        
        // 5. 验证合理性（规则检查）
        let validated = self.validator.validate(suggestions.optimizations)?;
        
        Ok(validated)
    }
    
    fn build_optimizer_prompt(&self, stats: &ToolStats) -> String {
        // 插入历史成功决策作为 Few-Shot
        let history = self.history.iter()
            .take(2)
            .map(|d| d.format())
            .collect::<Vec<_>>()
            .join("\n\n");
        
        // 插入工具统计
        let tool_list = stats.tools.iter()
            .map(|t| format!("- {}: 使用{}次，失败率{:.1}%", t.name, t.usage_count, t.failure_rate * 100))
            .collect::<Vec<_>>()
            .join("\n");
        
        format!(OPTIMIZER_PROMPT, tool_stats = tool_list, history = history)
    }
}
```

---

## 3. PromptCreator（基于 Prompt 的工具创建器）

### 设计目标

使用 Few-Shot Prompt + 自修正循环生成可编译的 Rust 工具代码。

### 核心流程

```
1. 检索相似工具代码作为示例
   ↓
2. Few-Shot Prompt 生成初始代码
   ↓
3. cargo check 验证
   ↓
4. 如有错误 → 将错误反馈给 LLM 修正
   ↓
5. 重复直到编译通过或达到最大迭代次数
```

### 代码生成 Prompt

```rust
pub const CODEGEN_PROMPT: &str = r#"
你是 Rust 工具开发专家。请根据以下要求创建新工具。

## 工具需求
{tool_spec}

## 参考示例（相似工具的实现）
{retrieved_examples}

## 代码要求
1. 遵循 tokitai 的#[tool] 宏规范
2. 实现 ExternalTool trait
3. 包含完整的错误处理
4. 包含单元测试

## 输出格式
直接输出 Rust 代码（不含 Markdown 标记）
"#;
```

### 自修正循环实现

```rust
pub struct PromptCreator {
    llm_client: Arc<dyn LLMClient>,
    example_db: ToolExampleDatabase,
    compiler: RustCompiler,
}

impl PromptCreator {
    pub async fn create_tool(&self, gap: &ToolGap) -> Result<GeneratedCode> {
        // 1. 检索相似工具示例
        let examples = self.example_db.retrieve_similar(gap, k=3)?;
        
        // 2. 构建 Few-Shot Prompt
        let prompt = self.build_codegen_prompt(gap, &examples);
        
        // 3. 生成初始代码
        let mut code = self.llm_client.chat(&prompt).await?;
        
        // 4. 自修正循环
        for attempt in 0..5 {
            match self.compiler.check(&code).await {
                Ok(()) => {
                    // 编译通过
                    return Ok(GeneratedCode { code, compiled: true });
                }
                Err(errors) => {
                    // 编译失败，将错误反馈给 LLM 修正
                    let fix_prompt = self.build_fix_prompt(&code, &errors);
                    code = self.llm_client.chat(&fix_prompt).await?;
                }
            }
        }
        
        Err("Failed to generate compilable code after 5 attempts".into())
    }
    
    fn build_fix_prompt(&self, code: &str, errors: &[CompilerError]) -> String {
        format!(
            r#"
请修正以下 Rust 代码的编译错误：

## 原始代码
{}

## 编译错误
{}

## 修正要求
1. 修复所有编译错误
2. 保持原有功能不变
3. 输出完整修正后的代码（不是 diff）
"#,
            code,
            errors.iter().map(|e| e.message).collect::<Vec<_>>().join("\n")
        )
    }
}
```

### 预期性能

| 指标 | 目标 |
|------|------|
| 编译通过率 | >80% |
| 功能正确率（通过测试） | >70% |
| 平均生成时间 | <30 秒/工具 |
| 平均修正次数 | 1-2 次 |

---

## 4. MultiAgentNegotiator（多智能体协商器）

### 设计目标

使用多个 LLM 实例扮演不同角色，通过结构化对话达成进化决策共识。

### 智能体角色定义

```rust
/// 智能体角色 Prompt
pub const AGENT_ROLES: &[(&str, &str)] = &[
    ("Creator", r#"
你是工具创建者。你的目标是发现新工具机会，扩展工具库能力。
你倾向于创建新工具，但需要考虑：
- 工具库的整体健康度
- 避免冗余
- 优先级排序
"#),
    ("Optimizer", r#"
你是工具优化者。你的目标是改进现有工具。
你认为应该优先改进而非新建，除非有明显缺口。
你关注：
- 工具使用率
- 工具失败率
- 用户满意度
"#),
    ("Eliminator", r#"
你是工具淘汰者。你的目标是移除冗余工具，保持工具库精简。
你倾向于：
- 合并功能重叠的工具
- 废弃低使用率工具
- 简化接口
"#),
    ("Planner", r#"
你是系统规划者。你的目标是整体工具库健康。
你协调各方意见，做出最终决策。
你考虑：
- 短期效果 vs 长期影响
- 各智能体的论据质量
- 系统资源限制
"#),
];
```

### 协商协议

```rust
pub struct MultiAgentNegotiator {
    creator: LLMClient,
    optimizer: LLMClient,
    eliminator: LLMClient,
    planner: LLMClient,
}

impl MultiAgentNegotiator {
    pub async fn negotiate(&self, state: &EvolutionState) -> Result<EvolutionAction> {
        // Round 1: 独立分析
        let creator_proposal = self.creator.analyze(state).await?;
        let optimizer_proposal = self.optimizer.analyze(state).await?;
        let eliminator_proposal = self.eliminator.analyze(state).await?;
        
        // Round 2: 互相评论
        let critiques = self.collect_critiques(&[
            &creator_proposal,
            &optimizer_proposal,
            &eliminator_proposal,
        ]).await?;
        
        // Round 3: Planner 决策
        let decision = self.planner.decide(
            &[&creator_proposal, &optimizer_proposal, &eliminator_proposal],
            &critiques
        ).await?;
        
        // Round 4: 投票确认
        let votes = self.collect_votes(&decision).await?;
        
        if votes.approval_rate > 0.6 {
            Ok(decision)
        } else {
            // 未达成共识，重新协商
            self.renegotiate(state).await
        }
    }
}
```

### 协商示例

```
=== Round 1: 独立分析 ===

[Creator] 建议：创建 batch_download 工具
  理由：15 个任务因缺少批量下载失败，预期节省 27 分钟/任务

[Optimizer] 建议：改进 download_file 支持批量
  理由：download_file 使用率低（5 次），扩展功能比新建更经济

[Eliminator] 建议：合并 download_file 和 http_get
  理由：两者功能重叠 80%，用户混淆

=== Round 2: 互相评论 ===

[Creator 评论 Optimizer] download_file 设计为单文件下载，扩展会破坏单一职责

[Optimizer 评论 Creator] 新工具增加维护成本，现有工具扩展更可持续

[Eliminator 评论 两者] 先合并冗余工具，再考虑新功能

=== Round 3: Planner 决策 ===

[Planner] 决定：
1. 暂缓合并 download_file 和 http_get（使用场景不同）
2. 创建独立的 batch_download 工具
3. batch_download 内部复用 download_file 的实现

理由：
- Creator 的证据充分（15 个失败任务）
- Optimizer 的担忧通过复用解决
- Eliminator 的合并建议证据不足

=== Round 4: 投票 ===

[Creator] ✅ 同意
[Optimizer] ✅ 同意（接受复用方案）
[Eliminator] ⚠️ 中立

结果：通过率 67% → 决策生效
```

---

## 5. SelfImprovementLoop（自主改进循环）

### 完整流程（Prompt Engineering 版本）

```rust
pub struct SelfImprovementLoop {
    gap_detector: Arc<PromptGapDetector>,
    optimizer: Arc<PromptOptimizer>,
    creator: Arc<PromptCreator>,
    negotiator: Arc<MultiAgentNegotiator>,
    reflection_interval: Duration,
}

impl SelfImprovementLoop {
    pub async fn run_iteration(&self) -> Result<IterationReport> {
        let mut report = IterationReport::new();

        // 1. 系统反思（Prompt-based）
        tracing::info!("开始系统反思...");
        let reflection = self.reflector.reflect().await?;
        report.reflection = reflection.clone();

        // 2. 发现工具缺口（Prompt-based）
        tracing::info!("发现工具缺口...");
        let gaps = self.gap_detector.detect_gaps().await?;
        report.gaps = gaps.clone();

        // 3. 优化建议（Prompt-based）
        tracing::info!("优化建议...");
        let optimizations = self.optimizer.optimize_tools().await?;
        report.optimizations = optimizations.clone();

        // 4. 多智能体协商决定优先级
        tracing::info!("多智能体协商...");
        let priorities = self.negotiator
            .negotiate(&EvolutionState {
                gaps,
                optimizations,
                reflection,
            })
            .await?;
        report.priorities = priorities.clone();

        // 5. 执行改进
        tracing::info!("执行改进...");
        for action in priorities {
            match action.action_type {
                ActionType::CreateTool => {
                    let tool = self.creator.create_tool(&action.gap).await?;
                    report.created_tools.push(tool);
                }
                ActionType::OptimizeTool => {
                    self.optimizer.execute_optimizations(vec![action.optimization]).await?;
                    report.optimized_tools.push(action.optimization.tool_name);
                }
                ActionType::DeprecateTool => {
                    self.optimizer.deprecate_tool(&action.tool_name).await?;
                    report.deprecated_tools.push(action.tool_name);
                }
            }
        }

        // 6. 保存报告
        self.save_report(&report).await?;

        Ok(report)
    }

    pub async fn run(&self) {
        loop {
            match self.run_iteration().await {
                Ok(report) => {
                    tracing::info!(
                        "迭代完成：创建{}个工具，优化{}个工具，废弃{}个工具",
                        report.created_tools.len(),
                        report.optimized_tools.len(),
                        report.deprecated_tools.len()
                    );
                }
                Err(e) => {
                    tracing::error!("迭代失败：{}", e);
                }
            }

            tokio::time::sleep(self.reflection_interval).await;
        }
    }
}
```

---

## 6. 理论贡献（Prompt Engineering 版本）

### 贡献 1：因果推理 Prompt 设计模式

```
问题：如何让 LLM 进行可靠的因果推理？
方案：Chain-of-Thought + 反事实提问 + JSON Schema 约束
创新：首次应用于工具进化场景
```

### 贡献 2：多智能体协商协议

```
问题：如何避免单 LLM 决策的偏见？
方案：多角色 LLM 通过结构化对话达成共识
创新：新的多 LLM 协作范式
```

### 贡献 3：自修正代码生成

```
问题：如何提高 LLM 生成代码的编译通过率？
方案：编译错误反馈循环
创新：结合编译器反馈的 Prompt 迭代
```

---

## 7. 实验设计（Prompt Engineering 版本）

### 对比实验

| 实验组 | 说明 |
|--------|------|
| **Control** | 无自进化（原始 tokitai） |
| **Ours-Full** | 完整 Prompt Engineering 系统 |
| **Ours-Single** | 单 LLM（无多智能体协商） |
| **Ours-NoCoT** | 无 Chain-of-Thought |
| **Ours-NoFix** | 无自修正循环 |

### 评估指标

| 指标 | 目标 |
|------|------|
| 缺口检测准确率 | >75% |
| 工具创建编译通过率 | >80% |
| 任务完成率提升 | +15-20% |
| 平均工具调用减少 | -30% |
| API 成本/月 | <$50 |

---

**文档维护者**：AI Assistant  
**最后更新**：2026-03-20  
**方法**：Prompt Engineering（无需训练）
