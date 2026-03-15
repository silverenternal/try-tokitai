# 核心机制设计详解

> 本文档详细描述自进化系统的四个核心机制的实现细节

---

## 1. ToolGapDetector（工具缺口检测器）

### 设计目标

从历史任务中自动发现"缺少什么工具"，而非等待用户提出需求。

### 输入数据

```rust
struct TaskRecord {
    id: String,
    description: String,
    status: TaskStatus,  // Success, Failed, Partial
    user_satisfaction: f32,  // 0.0-5.0
    tool_call_count: usize,
    tool_sequence: Vec<String>,  // 调用的工具序列
    error_message: Option<String>,
}
```

### 检测策略

#### 策略 1：从失败任务学习

```rust
fn detect_from_failures(&self, failed_tasks: &[TaskRecord]) -> Vec<ToolGap> {
    // 失败任务分类
    let categories = failed_tasks.iter()
        .map(|t| {
            if t.error_message.contains("工具未找到") {
                FailureCategory::MissingTool
            } else if t.error_message.contains("参数错误") {
                FailureCategory::WrongParameters
            } else if t.error_message.contains("权限不足") {
                FailureCategory::PermissionDenied
            } else {
                FailureCategory::Other
            }
        })
        .collect();
    
    // AI 分析 MissingTool 类型的失败
    let prompt = format!(
        r#"分析以下失败的任务，找出缺少的工具。

失败任务：
{}

请描述：
1. 用户想完成什么任务
2. 为什么失败（缺少什么工具）
3. 需要什么新工具

输出 JSON 格式。"#,
        failed_tasks.iter()
            .map(|t| format!("- {}: {}", t.id, t.description))
            .join("\n")
    );
    
    // ...
}
```

#### 策略 2：从低效任务学习

```rust
fn detect_from_inefficiency(&self, inefficient_tasks: &[TaskRecord]) -> Vec<ToolGap> {
    // 找出工具调用次数 > 5 的任务
    let tasks = inefficient_tasks.iter()
        .filter(|t| t.tool_call_count > 5)
        .collect();
    
    // AI 分析：是否可以创造一个新工具来简化？
    let prompt = format!(
        r#"分析以下低效的任务，找出可以简化的模式。

低效任务：
{}

工具调用序列：
{}

请分析：
1. 哪些工具调用是重复的？
2. 哪些工具可以组合成一个新工具？
3. 新工具应该有什么功能？

输出 JSON 格式。"#,
        tasks.iter().map(|t| t.description).join("\n"),
        tasks.iter().map(|t| t.tool_sequence.join(" → ")).join("\n")
    );
    
    // ...
}
```

#### 策略 3：从用户反馈学习

```rust
fn detect_from_feedback(&self, low_satisfaction_tasks: &[TaskRecord]) -> Vec<ToolGap> {
    // 用户满意度 < 3.0 的任务
    let tasks = low_satisfaction_tasks.iter()
        .filter(|t| t.user_satisfaction < 3.0)
        .collect();
    
    // AI 分析：用户为什么不满意？
    // ...
}
```

### 输出格式

```json
{
    "gap_type": "missing_tool",
    "description": "缺少批量重命名文件的工具",
    "suggested_name": "batch_rename_files",
    "suggested_functionality": "根据模式批量重命名多个文件",
    "input_schema": {
        "type": "object",
        "properties": {
            "directory": {"type": "string"},
            "pattern": {"type": "string"},
            "replacement": {"type": "string"}
        }
    },
    "priority": 0.85,
    "affected_tasks": ["task_123", "task_456"],
    "expected_impact": {
        "tasks_improved": 15,
        "avg_tool_calls_reduced": 4.5
    }
}
```

---

## 2. ToolOptimizer（工具优化器）

### 设计目标

自主分析现有工具的问题，决定合并/废弃/改进。

### 分析维度

#### 维度 1：使用率分析

```rust
fn analyze_usage(&self, tools: &[ToolDefinition]) -> Vec<ToolInsight> {
    // 计算使用率分位数
    let usage_counts: Vec<usize> = tools.iter()
        .map(|t| t.usage_count)
        .collect();
    
    let p25 = percentile(&usage_counts, 25.0);
    let p75 = percentile(&usage_counts, 75.0);
    
    // 找出低使用率工具（< P25）
    let underused = tools.iter()
        .filter(|t| t.usage_count < p25)
        .collect();
    
    // AI 分析原因
    let prompt = format!(
        r#"分析以下低使用率工具的原因。

工具列表：
{}

可能原因：
1. 功能冗余（有其他工具能完成相同功能）
2. 功能太窄（使用场景太少）
3. 功能太复杂（用户不知道怎么用）
4. 命名不清（用户找不到这个工具）
5. 已被更好的工具替代

对每个工具，判断原因并给出建议。"#,
        underused.iter()
            .map(|t| format!("- {}: 使用{}次，{}", t.name, t.usage_count, t.description))
            .join("\n")
    );
    
    // ...
}
```

#### 维度 2：失败率分析

```rust
fn analyze_failures(&self, tools: &[ToolDefinition]) -> Vec<ToolInsight> {
    // 找出高失败率工具（> 30%）
    let high_failure = tools.iter()
        .filter(|t| t.failure_rate > 0.3)
        .collect();
    
    // AI 分析根本原因
    let prompt = format!(
        r#"分析以下高失败率工具的根本原因。

工具列表：
{}

可能原因：
1. 输入验证不足
2. 错误处理不当
3. 依赖外部服务不稳定
4. 边界条件未处理
5. 文档不清导致误用

对每个工具，判断原因并给出改进建议。"#,
        high_failure.iter()
            .map(|t| format!("- {}: 失败率{:.1}%, {}", t.name, t.failure_rate * 100, t.description))
            .join("\n")
    );
    
    // ...
}
```

#### 维度 3：工具冗余分析

```rust
fn analyze_redundancy(&self, tools: &[ToolDefinition]) -> Vec<ToolMergeSuggestion> {
    // 基于功能描述相似度找出冗余工具
    let prompt = format!(
        r#"分析以下工具，找出功能重复的工具。

工具列表：
{}

请找出：
1. 功能完全相同的工具（可以合并）
2. 功能高度重叠的工具（建议保留一个）
3. 功能有包含关系的工具（建议用通用工具替代特化工具）

输出 JSON 格式。"#,
        tools.iter()
            .map(|t| format!("- {}: {}", t.name, t.description))
            .join("\n")
    );
    
    // ...
}
```

### 优化动作

```rust
enum OptimizationAction {
    /// 合并工具
    Merge {
        source_tools: Vec<String>,
        target_tool: String,
    },
    
    /// 废弃工具
    Deprecate {
        tool: String,
        replacement: Option<String>,  // 替代工具
    },
    
    /// 重命名工具
    Rename {
        tool: String,
        new_name: String,
    },
    
    /// 扩展功能
    Extend {
        tool: String,
        new_features: Vec<String>,
    },
    
    /// 简化接口
    Simplify {
        tool: String,
        simplifications: Vec<String>,
    },
    
    /// 改进文档
    ImproveDocs {
        tool: String,
        improvements: Vec<String>,
    },
}
```

---

## 3. SystemReflector（系统反思器）

### 设计目标

定期生成系统"体检报告"，发现系统性问题（而非单个工具的问题）。

### 反思维度

#### 维度 1：工具库覆盖分析

```rust
fn analyze_coverage(&self, tools: &[ToolDefinition]) -> CoverageAnalysis {
    // 定义工具领域本体
    let domains = vec![
        "文件操作",
        "系统管理",
        "网络通信",
        "数据处理",
        "代码分析",
        "版本控制",
        "数据库",
        "云服务",
        // ...
    ];
    
    // AI 评估每个领域的覆盖情况
    let prompt = format!(
        r#"评估工具库在各领域的覆盖情况。

当前工具库：
{}

领域列表：{}

请评估：
1. 每个领域的工具数量
2. 覆盖是否充分（1-5 分）
3. 找出覆盖不足的领域
4. 找出覆盖过度的领域（可能冗余）

输出 JSON 格式。"#,
        tools.iter()
            .map(|t| format!("- {}: {}", t.name, t.description))
            .join("\n"),
        domains.join(", ")
    );
    
    CoverageAnalysis {
        domain_scores: /* ... */,
        undercovered_areas: /* ... */,
        overcovered_areas: /* ... */,
        balance_score: /* ... */,
    }
}
```

#### 维度 2：工具库演化趋势

```rust
fn analyze_trend(&self, history: &[ToolRecord]) -> EvolutionTrend {
    // 按时间分析工具库变化
    let timeline = history.iter()
        .group_by_date()
        .map(|(date, tools)| {
            (date, ToolSnapshot {
                total_count: tools.len(),
                new_tools: tools.iter().filter(|t| t.created_on == date).count(),
                deprecated_tools: tools.iter().filter(|t| t.deprecated_on == date).count(),
                avg_usage: tools.iter().map(|t| t.usage_count).mean(),
                avg_failure_rate: tools.iter().map(|t| t.failure_rate).mean(),
            })
        })
        .collect();
    
    // AI 分析趋势
    let prompt = format!(
        r#"分析工具库的演化趋势。

时间线数据：
{}

请分析：
1. 工具库规模增长趋势（增长/稳定/萎缩）
2. 工具质量变化趋势（提升/稳定/下降）
3. 工具使用模式的变化
4. 预测未来发展趋势

输出 JSON 格式。"#,
        timeline.iter()
            .map(|(d, s)| format!("- {}: {}工具，平均使用{}次", d, s.total_count, s.avg_usage))
            .join("\n")
    );
    
    EvolutionTrend {
        growth_trend: Trend::Increasing,
        quality_trend: Trend::Improving,
        predictions: vec!["..."],
    }
}
```

#### 维度 3：系统性问题发现

```rust
fn find_systemic_issues(&self, tools: &[ToolDefinition], history: &[TaskRecord]) -> Vec<SystemicIssue> {
    // 系统性问题类型
    let issue_types = vec![
        "工具链断裂",      // 缺少连接多个工具的工具
        "重复造轮子",      // 多个工具实现相似功能
        "依赖关系混乱",    // 工具依赖不清晰
        "工具命名混乱",    // 命名不一致，用户难以查找
        "文档质量参差",    // 部分工具文档缺失
        "错误处理不统一",  // 错误格式不一致
    ];
    
    // AI 诊断系统性问题
    let prompt = format!(
        r#"诊断工具库的系统性问题。

工具库数据：
- 工具总数：{}
- 平均使用率：{:.1}%
- 平均失败率：{:.1}%
- 工具箱数量：{}

任务历史数据：
- 总任务数：{}
- 平均工具调用次数：{:.1}
- 用户满意度：{:.1}/5.0

可能的问题类型：{}

请找出存在的系统性问题，按严重程度排序。"#,
        tools.len(),
        self.get_average_usage(),
        self.get_average_failure_rate(),
        self.get_toolbox_count(),
        history.len(),
        self.get_average_tool_calls(),
        self.get_average_satisfaction(),
        issue_types.join(", ")
    );
    
    // ...
}
```

### 反思报告格式

```json
{
    "timestamp": "2026-03-15T10:00:00Z",
    "coverage_analysis": {
        "coverage_score": 0.75,
        "domain_scores": {
            "文件操作": 0.9,
            "系统管理": 0.8,
            "网络通信": 0.7,
            "数据处理": 0.6,
            "数据库": 0.3
        },
        "undercovered_areas": ["数据库", "云服务"],
        "overcovered_areas": ["文件操作"],
        "balance_score": 0.65
    },
    "evolution_trend": {
        "growth_trend": "increasing",
        "quality_trend": "improving",
        "predictions": ["数据库工具需求增长", "文件工具趋于饱和"]
    },
    "systemic_issues": [
        {
            "issue_type": "工具链断裂",
            "description": "下载文件后缺乏自动解压工具",
            "severity": "medium",
            "evidence": "15% 的下载任务需要手动解压",
            "recommendation": "创建 auto_extract 工具"
        }
    ],
    "strategic_recommendations": [
        {
            "category": "priority",
            "recommendation": "优先发展数据库工具",
            "rationale": "30% 的任务涉及数据持久化，但只有 2 个数据库工具",
            "timeframe": "未来 2 周"
        }
    ]
}
```

---

## 4. ToolCreator（工具创造器）

### 设计目标

根据缺口建议，AI 自主创造新工具并自动注册到 tokitai。

### 创造流程

```rust
impl ToolCreator {
    /// 创造新工具
    pub async fn create_tool(&self, gap: &ToolGap) -> Result<ToolDefinition> {
        // 1. AI 设计工具接口
        let design = self.design_tool(gap).await?;
        
        // 2. AI 生成工具实现代码
        let code = self.generate_implementation(&design).await?;
        
        // 3. 编译代码（安全检查）
        self.compile_and_verify(&code).await?;
        
        // 4. 注册到 tokitai ToolRegistry
        let tool = self.register_tool(&design, &code).await?;
        
        // 5. 生成工具文档
        self.generate_documentation(&tool).await?;
        
        Ok(tool)
    }
    
    /// AI 设计工具接口
    async fn design_tool(&self, gap: &ToolGap) -> Result<ToolDesign> {
        let prompt = format!(
            r#"设计一个新工具来填补以下缺口。

缺口描述：
{}

建议功能：
{}

请设计：
1. 工具名称（snake_case 格式）
2. 工具描述（简洁清晰）
3. 输入参数（JSON Schema 格式）
4. 返回类型
5. 风险等级（low/medium/high）
6. 工具类别

输出 JSON 格式。"#,
            gap.description,
            gap.suggested_functionality
        );
        
        let response = self.llm_client.chat(&prompt).await?;
        let design: ToolDesign = serde_json::from_str(&response)?;
        
        Ok(design)
    }
    
    /// AI 生成工具实现
    async fn generate_implementation(&self, design: &ToolDesign) -> Result<String> {
        let prompt = format!(
            r#"实现以下工具。

工具设计：
- 名称：{}
- 描述：{}
- 输入：{}
- 返回：{}

请使用 Rust 实现，遵循 tokitai 的#[tool] 宏规范。

输出代码（不含 Markdown 标记）。"#,
            design.name,
            design.description,
            design.input_schema,
            design.output_type
        );
        
        let code = self.llm_client.chat(&prompt).await?;
        Ok(code)
    }
    
    /// 注册到 tokitai
    async fn register_tool(&self, design: &ToolDesign, code: &str) -> Result<ToolDefinition> {
        // 动态加载工具代码（需要 Rust 运行时编译支持）
        // 或者生成代码后重新编译整个项目
        
        // 简化方案：生成工具定义，手动添加到项目
        let tool = ToolDefinition {
            name: design.name.clone(),
            description: design.description.clone(),
            input_schema: design.input_schema.clone(),
            metadata: ServiceMetadata {
                category: design.category.clone(),
                risk_level: design.risk_level.clone(),
                ..Default::default()
            },
            ..Default::default()
        };
        
        // 注册到 tokitai ToolRegistry
        self.tool_registry.write().add_tool(tool.clone());
        
        Ok(tool)
    }
}
```

---

## 5. 自主改进循环（Self-Improvement Loop）

### 完整流程

```rust
impl SelfImprovementLoop {
    /// 单次迭代
    pub async fn run_iteration(&self) -> Result<IterationReport> {
        let mut report = IterationReport::new();
        
        // 1. 系统反思
        tracing::info!("开始系统反思...");
        let reflection = self.reflector.reflect().await?;
        report.reflection = reflection.clone();
        
        // 2. 发现工具缺口
        tracing::info!("发现工具缺口...");
        let gaps = self.gap_detector.detect_gaps().await?;
        report.gaps = gaps.clone();
        
        // 3. 优化工具
        tracing::info!("优化工具...");
        let optimizations = self.optimizer.optimize_tools().await?;
        report.optimizations = optimizations.clone();
        
        // 4. AI 决定优先级
        tracing::info!("决定优先级...");
        let priorities = self.prioritize_actions(&gaps, &optimizations, &reflection).await?;
        
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
    
    /// 持续运行
    pub async fn run(&self) {
        loop {
            match self.run_iteration().await {
                Ok(report) => {
                    tracing::info!("迭代完成：创建{}个工具，优化{}个工具，废弃{}个工具",
                        report.created_tools.len(),
                        report.optimized_tools.len(),
                        report.deprecated_tools.len());
                }
                Err(e) => {
                    tracing::error!("迭代失败：{}", e);
                }
            }
            
            // 等待下一个周期
            tokio::time::sleep(self.reflection_interval).await;
        }
    }
}
```

---

## 6. 与 tokitai 的集成

### 新工具自动注册

```rust
// 在 AiAssistant::new_autonomous 中
pub fn new_autonomous(...) -> Result<Self, String> {
    let tool_registry = ToolRegistry::new();
    
    // 创建自进化组件
    let gap_detector = Arc::new(ToolGapDetector::new(...));
    let optimizer = Arc::new(ToolOptimizer::new(...));
    let reflector = Arc::new(SystemReflector::new(...));
    let creator = Arc::new(ToolCreator::new(tool_registry.clone(), ...));
    
    // 创建自主改进循环
    let improvement_loop = Arc::new(SelfImprovementLoop::new(
        gap_detector,
        optimizer,
        reflector,
        creator,
        Duration::from_secs(86400),  // 每天一次
    ));
    
    // 后台启动改进循环
    tokio::spawn({
        let loop_clone = improvement_loop.clone();
        async move {
            loop_clone.run().await;
        }
    });
    
    Ok(Self {
        tool_registry,
        improvement_loop,
        // ...
    })
}
```

---

**文档维护者**：AI Assistant  
**最后更新**：2026-03-15
