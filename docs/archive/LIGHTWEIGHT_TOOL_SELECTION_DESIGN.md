# 轻量级工具选择器设计文档（AI 原生版）

> **设计目标**：充分发挥 AI 智能和 tokitai 优势，实现自主进化的工具选择系统  
> **核心原则**：尽可能减少人工干预，让 AI 自主管理工具索引、分类和依赖关系  
> **最后更新**：2026-03-15  
> **参考论文**：arXiv:2602.23368, arXiv:2512.17052

---

## 📋 问题定义

### 当前挑战

当工具数量达到 10,000+ 且由 AI 自主创造时，工具选择面临以下挑战：

| 问题 | 传统方案 | 问题 |
|------|----------|------|
| **语义搜索** | 向量嵌入 + 相似度匹配 | 嵌入计算延迟高（~50-100ms/工具），需要额外模型 |
| **全量检索** | 遍历所有工具描述 | O(n) 复杂度，10,000 工具 = 10,000 次比较 |
| **静态分类** | 手动分类 + 层级导航 | 无法适应 AI 动态创造新工具 |
| **人工维护** | 开发者编写分类和依赖 | 与自主进化理念相悖 |

### 设计哲学转变

| 旧思维（工程师中心） | 新思维（AI 原生） |
|---------------------|------------------|
| 人工设计分类体系 | AI 自主发现和创建工具箱 |
| 手动维护依赖关系 | AI 分析工具语义自动推断 |
| 关键词分词匹配 | AI 理解查询意图 |
| 静态索引结构 | 后台异步重建，动态演化 |

### 延迟要求

| 场景 | 目标延迟 | 说明 |
|------|----------|------|
| 快速搜索（日常浏览） | <10ms | 关键词匹配，零 AI 调用 |
| AI 搜索（Executor 用） | <2s | 包含一次 LLM 调用 |
| 工具注册（后台处理） | <5s | AI 生成摘要 + 分配工具箱 + 分析依赖 |

---

## 🎯 核心设计原则

### 1. **AI 自主管理工具箱**（AI-Native Toolbox Management）

**核心理念**：工具箱不是预先设计的，而是 AI 在创造工具过程中自然演化的。

```rust
use tokitai::tool;
use crate::tool_matrix::matrix::{ToolDefinition, ServiceMetadata, ServiceCategory};

/// AI 工具分类器 - 自主维护工具箱体系
pub struct AIToolboxClassifier {
    llm_client: Arc<dyn LLMClient>,
    tool_registry: Arc<RwLock<ToolRegistry>>,
}

impl AIToolboxClassifier {
    /// 为新工具选择或创建工具箱
    pub async fn classify_tool(&self, tool: &ToolDefinition) -> Result<ToolboxAssignment> {
        // 获取现有工具箱摘要
        let toolboxes = self.tool_registry.read().get_all_toolboxes();
        let toolbox_summaries = self.get_toolbox_summaries(&toolboxes).await?;
        
        // AI 判断：放入现有工具箱 or 创建新的
        let prompt = format!(
            r#"你是一个工具分类专家。请为新工具选择最合适的工具箱。

## 新工具
- **名称**: {}
- **描述**: {}
- **类别**: {:?}
- **标签**: {:?}

## 现有工具箱
{}

## 任务
1. 判断新工具应该放入哪个工具箱
2. 如果现有工具箱都不合适，建议创建新工具箱
3. 给出理由

## 输出格式（JSON）
{{
    "action": "add_to_existing" | "create_new",
    "toolbox_id": "现有工具箱 ID（如果放入现有）",
    "new_toolbox": {{
        "name": "新工具箱名称",
        "description": "新工具箱简介",
        "use_cases": ["使用场景 1", "使用场景 2"]
    }}（如果创建新的）,
    "confidence": 0.0-1.0,
    "reason": "分类理由"
}}"#,
            tool.name,
            tool.description,
            tool.metadata.category,
            tool.tags,
            toolbox_summaries.iter()
                .map(|tb| format!("- **{}**: {}", tb.name, tb.description))
                .collect::<Vec<_>>()
                .join("\n")
        );
        
        let response = self.llm_client.chat(&prompt).await?;
        let assignment: ToolboxAssignment = serde_json::from_str(&response)?;
        
        // 如果 AI 建议创建新工具箱，自动执行
        if assignment.action == ToolboxAction::CreateNew {
            if let Some(new_tb) = &assignment.new_toolbox {
                self.create_new_toolbox(new_tb).await?;
            }
        }
        
        Ok(assignment)
    }
    
    /// 获取工具箱摘要（缓存）
    async fn get_toolbox_summaries(&self, toolboxes: &[ToolBox]) -> Result<Vec<ToolboxSummary>> {
        // 从缓存读取，如果不存在则 AI 生成
        let mut summaries = Vec::new();
        for toolbox in toolboxes {
            let summary = self.get_or_generate_toolbox_summary(toolbox).await?;
            summaries.push(summary);
        }
        Ok(summaries)
    }
    
    /// 获取或生成工具箱摘要
    async fn get_or_generate_toolbox_summary(&self, toolbox: &ToolBox) -> Result<ToolboxSummary> {
        // 1. 尝试从缓存读取
        if let Some(cached) = self.summary_cache.read().get(&toolbox.id) {
            return Ok(cached.clone());
        }
        
        // 2. AI 生成摘要
        let tools = toolbox.get_all_tools().cloned().collect::<Vec<_>>();
        let summary = self.generate_toolbox_summary(&toolbox.name, &tools).await?;
        
        // 3. 写入缓存
        self.summary_cache.write().insert(toolbox.id.clone(), summary.clone());
        
        Ok(summary)
    }
    
    /// AI 生成工具箱摘要
    async fn generate_toolbox_summary(&self, toolbox_name: &str, tools: &[ToolDefinition]) -> Result<ToolboxSummary> {
        let prompt = format!(
            r#"你是一个工具分类专家。请为以下工具箱生成简介。

## 工具箱名称
{}

## 包含工具
{}

## 任务
1. 生成工具箱简介（50 字以内）
2. 列出典型使用场景（3-5 个）
3. 提取关键词（5-10 个，用于搜索）

## 输出格式（JSON）
{{
    "description": "工具箱简介",
    "use_cases": ["场景 1", "场景 2", "场景 3"],
    "keywords": ["关键词 1", "关键词 2"]
}}"#,
            toolbox_name,
            tools.iter()
                .map(|t| format!("- **{}**: {}", t.name, t.description))
                .collect::<Vec<_>>()
                .join("\n")
        );
        
        let response = self.llm_client.chat(&prompt).await?;
        let summary: ToolboxSummary = serde_json::from_str(&response)?;
        
        Ok(summary)
    }
}

/// 工具箱分配结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolboxAssignment {
    pub action: ToolboxAction,
    pub toolbox_id: Option<String>,
    pub new_toolbox: Option<NewToolbox>,
    pub confidence: f32,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolboxAction {
    AddToExisting,
    CreateNew,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewToolbox {
    pub name: String,
    pub description: String,
    pub use_cases: Vec<String>,
}
```

**与 tokitai 集成**：

```rust
// 在 AiAssistant::new_autonomous 中
pub fn new_autonomous(...) -> Result<Self, String> {
    // ... 现有代码 ...
    
    // 创建 AI 工具箱分类器
    let ai_classifier = AIToolboxClassifier::new(
        Arc::new(tokitai::LLMClient::new(api_url, api_key)),
        Arc::new(RwLock::new(tool_registry.clone())),
    );
    
    // 注册到工具选择器
    let tool_selector = LightweightToolSelector::new(
        all_tools,
        Some(ai_classifier),
    );
    
    Self {
        // ...
        tool_selector,
        ai_classifier: Some(ai_classifier),
    }
}
```

---

### 2. **后台异步索引重建**（Background Async Index Rebuild）

**核心理念**：新工具注册不阻塞主线程，后台批量重建索引。

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;

/// 轻量级工具选择器
pub struct LightweightToolSelector {
    // 当前使用的索引（读多写少，用 RwLock）
    current_index: Arc<RwLock<ToolIndex>>,
    
    // 待重建的工具队列
    pending_tools: Arc<RwLock<Vec<ToolDefinition>>>,
    
    // 后台重建控制
    rebuild_trigger: Arc<AtomicBool>,
    rebuild_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    
    // AI 分类器（可选，用于自主分类）
    ai_classifier: Option<AIToolboxClassifier>,
    
    // 配置
    config: SelectorConfig,
}

impl LightweightToolSelector {
    /// 创建新的选择器
    pub fn new(
        tools: Vec<ToolDefinition>,
        ai_classifier: Option<AIToolboxClassifier>,
    ) -> Self {
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
            ai_classifier,
            config: SelectorConfig::default(),
        }
    }
    
    /// 添加新工具（异步，不阻塞）
    pub fn add_tool_async(&self, tool: ToolDefinition) {
        // 1. 添加到待重建队列
        self.pending_tools.write().push(tool);
        
        // 2. 触发后台重建（如果还没在重建）
        self.trigger_rebuild();
    }
    
    /// 触发后台重建
    fn trigger_rebuild(&self) {
        // 检查是否已经在重建
        if self.rebuild_trigger.load(Ordering::SeqCst) {
            return;  // 已经在重建，跳过
        }
        
        // 标记为需要重建
        self.rebuild_trigger.store(true, Ordering::SeqCst);
        
        // 启动后台任务
        let pending = self.pending_tools.clone();
        let current_index = self.current_index.clone();
        let ai_classifier = self.ai_classifier.clone();
        let rebuild_trigger = self.rebuild_trigger.clone();
        let rebuild_handle = self.rebuild_handle.clone();
        
        let handle = tokio::spawn(async move {
            // 等待一小段时间，收集更多新工具
            tokio::time::sleep(Duration::from_secs(2)).await;
            
            // 取出待重建工具
            let tools_to_add = {
                let mut pending = pending.write();
                std::mem::take(&mut *pending)
            };
            
            if tools_to_add.is_empty() {
                rebuild_trigger.store(false, Ordering::SeqCst);
                return;
            }
            
            // AI 分类（如果有分类器）
            let mut classified_tools = Vec::new();
            if let Some(classifier) = &ai_classifier {
                for tool in tools_to_add {
                    match classifier.classify_tool(&tool).await {
                        Ok(assignment) => {
                            // 根据 AI 的分类结果，将工具分配到工具箱
                            classified_tools.push((tool, assignment));
                        }
                        Err(e) => {
                            tracing::warn!("AI 分类失败，使用默认分类：{}", e);
                            classified_tools.push((tool, ToolboxAssignment::default()));
                        }
                    }
                }
            } else {
                classified_tools = tools_to_add.into_iter()
                    .map(|t| (t, ToolboxAssignment::default()))
                    .collect();
            }
            
            // 构建新索引
            let mut new_index = current_index.read().clone();
            for (tool, assignment) in classified_tools {
                new_index.add_tool_with_assignment(tool, assignment);
            }
            
            // 原子替换索引（读操作无感知）
            *current_index.write() = new_index;
            
            tracing::info!("工具索引重建完成，新增 {} 个工具", tools_to_add.len());
            
            // 清除重建标记
            rebuild_trigger.store(false, Ordering::SeqCst);
            
            // 检查是否有新的待重建工具
            if !pending.read().is_empty() {
                rebuild_trigger.store(true, Ordering::SeqCst);
            }
        });
        
        // 保存任务句柄
        *rebuild_handle.write() = Some(handle);
    }
    
    /// 搜索工具（主入口）
    pub fn search(&self, query: &str) -> Vec<ToolSearchResult> {
        // 自动判断：复杂查询用 AI 搜索，简单查询用快速搜索
        let use_ai = self.should_use_ai_search(query);
        
        if use_ai {
            self.ai_search(query)
        } else {
            self.fast_search(query)
        }
    }
    
    /// 自动判断是否使用 AI 搜索
    fn should_use_ai_search(&self, query: &str) -> bool {
        // 简单启发式规则：
        // 1. 查询长度 > 20 字符 → 可能是复杂任务
        // 2. 包含疑问词（如何、怎么、为什么）→ 需要理解意图
        // 3. 包含多个动词 → 可能需要工具组合
        
        let query_lower = query.to_lowercase();
        
        // 规则 1: 长度
        if query.len() > 20 {
            return true;
        }
        
        // 规则 2: 疑问词
        let question_words = ["如何", "怎么", "怎样", "为什么", "什么", "哪个"];
        if question_words.iter().any(|w| query_lower.contains(w)) {
            return true;
        }
        
        // 规则 3: 多个动词（简单检测）
        let action_words = ["创建", "读取", "写入", "删除", "修改", "分析", "搜索", "下载", "上传"];
        let action_count = action_words.iter().filter(|w| query_lower.contains(*w)).count();
        if action_count >= 2 {
            return true;
        }
        
        // 默认用快速搜索
        false
    }
    
    /// 快速搜索（关键词匹配）
    fn fast_search(&self, query: &str) -> Vec<ToolSearchResult> {
        let index = self.current_index.read();
        index.search(query, self.config.max_results)
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
    
    /// AI 搜索（复杂查询）
    fn ai_search(&self, query: &str) -> Vec<ToolSearchResult> {
        // 1. 快速搜索获取候选（Top-50）
        let candidates = self.fast_search(query);
        
        // 2. AI 从候选中选择最相关的
        let prompt = format!(
            r#"你是一个工具选择专家。用户需要完成以下任务：

{}

请从以下工具中选择最合适的 5-10 个工具，按相关性排序：

{}

输出 JSON 格式：
{{
    "selected_tools": [
        {{"tool_name": "工具名", "relevance_score": 0.0-1.0, "reason": "选择理由"}}
    ]
}}"#,
            query,
            candidates.iter()
                .map(|t| format!("- **{}**: {}", t.name, t.description))
                .collect::<Vec<_>>()
                .join("\n")
        );
        
        // 3. 调用 AI（异步）
        let rt = tokio::runtime::Handle::current();
        let response = rt.block_on(async {
            // 这里需要访问 LLM 客户端，可以通过 ai_classifier 或者独立的 client
            // 简化实现：返回空结果，实际应该调用 AI
            String::new()
        });
        
        // 4. 解析 AI 响应
        if response.is_empty() {
            // AI 调用失败，降级为快速搜索
            return candidates.into_iter().take(10).collect();
        }
        
        // 解析并返回
        self.parse_ai_search_response(&response, &candidates)
    }
}
```

---

### 3. **AI 自主维护依赖关系**（AI-Maintained Dependency Graph）

**核心理念**：依赖关系不是手动声明的，而是 AI 分析工具语义自动推断的。

```rust
/// AI 依赖关系分析器
pub struct AIDependencyAnalyzer {
    llm_client: Arc<dyn LLMClient>,
    dependency_graph: Arc<RwLock<ToolDependencyGraph>>,
}

impl AIDependencyAnalyzer {
    /// 分析新工具的依赖关系
    pub async fn analyze_dependencies(
        &self,
        tool: &ToolDefinition,
        all_tools: &[ToolDefinition],
    ) -> Result<DependencyAnalysis> {
        let prompt = format!(
            r#"你是一个工具依赖分析专家。请分析以下工具的依赖关系。

## 新工具
- **名称**: {}
- **描述**: {}
- **输入类型**: {}
- **输出类型**: {}
- **风险等级**: {}

## 现有工具列表
{}

## 任务
1. **前置依赖**: 执行这个工具前，通常需要先调用哪些工具？
   （例如：处理文件前需要先读取文件）

2. **后置依赖**: 哪些工具可能会依赖这个工具的输出？
   （例如：写入文件后可能需要验证文件内容）

3. **工具组合**: 这个工具经常和哪些工具一起使用？

## 输出格式（JSON）
{{
    "prerequisites": [
        {{"tool_name": "工具名", "reason": "依赖理由", "confidence": 0.0-1.0}}
    ],
    "dependents": [
        {{"tool_name": "工具名", "reason": "依赖理由", "confidence": 0.0-1.0}}
    ],
    "combinations": [
        {{"tool_name": "工具名", "scenario": "使用场景"}}
    ]
}}"#,
            tool.name,
            tool.description,
            extract_input_types(&tool.input_schema),
            extract_output_type(&tool.input_schema),
            tool.risk_level,
            all_tools.iter()
                .map(|t| format!("- **{}**: {}", t.name, t.description))
                .collect::<Vec<_>>()
                .join("\n")
        );
        
        let response = self.llm_client.chat(&prompt).await?;
        let analysis: DependencyAnalysis = serde_json::from_str(&response)?;
        
        // 更新依赖图
        self.update_dependency_graph(tool, &analysis).await?;
        
        Ok(analysis)
    }
    
    /// 更新依赖图
    async fn update_dependency_graph(
        &self,
        tool: &ToolDefinition,
        analysis: &DependencyAnalysis,
    ) -> Result<()> {
        let mut graph = self.dependency_graph.write();
        
        // 添加前置依赖
        for prereq in &analysis.prerequisites {
            graph.add_dependency(
                prereq.tool_name.clone(),
                tool.name.clone(),
                prereq.confidence,
            );
        }
        
        // 添加后置依赖
        for dependent in &analysis.dependents {
            graph.add_dependency(
                tool.name.clone(),
                dependent.tool_name.clone(),
                dependent.confidence,
            );
        }
        
        // 添加工具组合关系
        for combo in &analysis.combinations {
            graph.add_co_occurrence(
                tool.name.clone(),
                combo.tool_name.clone(),
                0.8,  // 组合关系权重
            );
        }
        
        tracing::info!("工具依赖关系已更新：{}", tool.name);
        
        Ok(())
    }
    
    /// 从运行时日志学习（补充 AI 分析）
    pub fn learn_from_runtime_logs(&self, logs: &[ToolCallSequence]) {
        let mut graph = self.dependency_graph.write();
        
        for seq in logs {
            // 分析工具调用序列，发现共现关系
            for i in 0..seq.tools.len() {
                for j in (i+1)..seq.tools.len() {
                    // 时间窗口内的工具调用视为共现
                    if seq.timestamps[j] - seq.timestamps[i] < 30000 {  // 30 秒内
                        graph.add_co_occurrence(
                            seq.tools[i].clone(),
                            seq.tools[j].clone(),
                            0.5,  // 运行时学习的权重较低
                        );
                    }
                }
            }
        }
    }
}

/// 依赖分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyAnalysis {
    pub prerequisites: Vec<DependencyRelation>,
    pub dependents: Vec<DependencyRelation>,
    pub combinations: Vec<ToolCombination>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyRelation {
    pub tool_name: String,
    pub reason: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCombination {
    pub tool_name: String,
    pub scenario: String,
}
```

**与 ExecutorAgent 集成**：

```rust
impl ExecutorAgent {
    /// 执行计划步骤（智能工具推荐）
    pub fn execute_step(
        &mut self,
        record_id: &str,
        step_id: String,
        tool_name: String,
        args: Value,
    ) -> Result<(), ExecutorError> {
        let start_time = chrono::Utc::now().timestamp();

        // 记录步骤开始
        self.record_step_start(record_id, step_id.clone())?;

        // 调用工具
        let result = self.call_tool(&tool_name, &args);

        // 如果成功，推荐下一步可能需要的工具
        if result.is_ok() {
            let recommended = self.recommend_next_tools(&tool_name);
            tracing::info!("推荐后续工具：{:?}", recommended);
        }

        let duration = (chrono::Utc::now().timestamp() - start_time) as u64;

        match result {
            Ok(output) => {
                self.record_step_complete(record_id, step_id, output, duration)?;
                Ok(())
            }
            Err(e) => {
                self.record_step_failed(record_id, step_id, e.to_string())?;
                Err(e)
            }
        }
    }
    
    /// 推荐后续工具（基于依赖图）
    fn recommend_next_tools(&self, current_tool: &str) -> Vec<String> {
        let graph = self.tool_registry.read();
        graph.get_dependency_graph().recommend_next_tools(&[current_tool.to_string()], 3)
    }
}
```

---

### 4. **tokitai 深度集成**

#### 4.1 利用 `#[tool]` 宏自动生成元数据

```rust
use tokitai::tool;
use crate::tool_matrix::matrix::{ToolDefinition, ServiceMetadata, ServiceCategory};

/// 文件操作工具集
#[tool]
pub struct FileOperations;

#[tool]
impl FileOperations {
    /// 读取文件内容
    #[tool(description = "读取指定路径的文件内容", category = "file")]
    pub fn read_file(&self, path: String) -> Result<String, String> {
        // ...
    }
    
    /// 写入文件
    #[tool(description = "将内容写入指定路径的文件", category = "file")]
    pub fn write_file(&self, path: String, content: String) -> Result<(), String> {
        // ...
    }
}

// tokitai 的 #[tool] 宏自动生成：
impl ToolProvider for FileOperations {
    fn tool_definitions() -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "read_file".to_string(),
                description: "读取指定路径的文件内容".to_string(),
                input_schema: r#"{"type": "object", "properties": {"path": {"type": "string"}}}"#.to_string(),
                metadata: ServiceMetadata {
                    category: ServiceCategory::File,
                    // ... 其他元数据可以从宏属性自动生成
                    ..Default::default()
                },
                ..Default::default()
            },
            // ...
        ]
    }
}
```

#### 4.2 利用 tokitai 的 ToolExecutor 统一调用

```rust
use tokitai::{ToolProvider, ToolExecutor};

/// 工具调用分发器
pub struct ToolDispatcher {
    registry: Arc<RwLock<ToolRegistry>>,
    executors: Arc<RwLock<HashMap<String, Box<dyn ToolExecutor>>>>,
}

impl ToolDispatcher {
    /// 注册工具执行器
    pub fn register_executor<T: ToolProvider + ToolExecutor + 'static>(
        &mut self,
        toolbox_id: &str,
    ) {
        let tools = T::tool_definitions();
        let executor = Box::new(T::new());  // 假设 ToolExecutor 有 new 方法
        
        for tool in tools {
            self.executors.write().insert(tool.name.clone(), executor.clone());
        }
    }
    
    /// 调用工具
    pub fn execute(&self, tool_name: &str, args: &Value) -> Result<Value, String> {
        let executors = self.executors.read();
        let executor = executors.get(tool_name)
            .ok_or_else(|| format!("工具未找到：{}", tool_name))?;
        
        executor.execute(tool_name, args)
    }
}
```

---

## 🏗️ 完整架构

```
┌─────────────────────────────────────────────────────────────────┐
│                         AiAssistant                              │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                   ToolRegistry                            │   │
│  │  - 工具注册表（tokitai ToolProvider）                      │   │
│  │  - 工具箱管理（AI 自主分类）                                │   │
│  └──────────────────────────────────────────────────────────┘   │
│                              │                                   │
│                              ▼                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              LightweightToolSelector                      │   │
│  │  ┌────────────────────────────────────────────────────┐   │   │
│  │  │              ToolIndex                              │   │   │
│  │  │  - 倒排索引（关键词匹配）                            │   │   │
│  │  │  - 后台异步重建                                      │   │   │
│  │  └────────────────────────────────────────────────────┘   │   │
│  │  ┌────────────────────────────────────────────────────┐   │   │
│  │  │              AIToolboxClassifier                    │   │   │
│  │  │  - AI 自主分类工具到工具箱                            │   │   │
│  │  │  - AI 生成工具箱摘要                                  │   │   │
│  │  └────────────────────────────────────────────────────┘   │   │
│  │  ┌────────────────────────────────────────────────────┐   │   │
│  │  │              AIDependencyAnalyzer                   │   │   │
│  │  │  - AI 分析工具依赖关系                                │   │   │
│  │  │  - 运行时日志学习                                   │   │   │
│  │  └────────────────────────────────────────────────────┘   │   │
│  └──────────────────────────────────────────────────────────┘   │
│                              │                                   │
│                              ▼                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              AgentCoordinator                             │   │
│  │  - Planner: 制定计划                                       │   │
│  │  - Executor: 执行任务（智能工具推荐）                       │   │
│  │  - Reviewer: 审查结果                                      │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

---

## 📊 性能预期

### 延迟基准

| 操作 | 目标延迟 | 预期延迟 |
|------|----------|----------|
| 快速搜索 | <10ms | 5-8ms |
| AI 搜索 | <2s | 1-1.5s（含 LLM 调用） |
| 工具注册（后台） | <5s | 2-3s（AI 分类 + 依赖分析） |
| 索引重建（100 工具） | <1s | 500-800ms |

### 内存占用

| 组件 | 10,000 工具 | 100,000 工具 |
|------|-------------|--------------|
| 倒排索引 | ~5MB | ~50MB |
| 工具箱摘要 | ~2MB | ~20MB |
| 依赖图 | ~1MB | ~10MB |
| **总计** | ~8MB | ~80MB |

---

## 🚀 实施计划

### 阶段 1：核心索引实现（1 周）

- [ ] 实现 `ToolIndex`（倒排索引）
- [ ] 实现后台异步重建机制
- [ ] 集成到 `ToolRegistry`

### 阶段 2：AI 分类器实现（1 周）

- [ ] 实现 `AIToolboxClassifier`
- [ ] AI 生成工具箱摘要
- [ ] AI 分配工具到工具箱
- [ ] 自动创建新工具箱

### 阶段 3：AI 依赖分析器实现（1 周）

- [ ] 实现 `AIDependencyAnalyzer`
- [ ] AI 分析工具依赖关系
- [ ] 运行时日志学习
- [ ] 集成到 `ExecutorAgent`

### 阶段 4：tokitai 深度集成（1 周）

- [ ] 利用 `#[tool]` 宏自动生成元数据
- [ ] 实现 `ToolDispatcher` 统一调用
- [ ] 优化 AI 搜索性能

---

## 📚 参考论文

1. **arXiv:2602.23368** - *Keyword search is all you need* (2026.02)
2. **arXiv:2512.17052** - *DTDR: Dynamic Tool Dependency Retrieval for LLM Agents* (2025.12)

---

## ✅ 验收标准

- [ ] 10,000 工具快速搜索延迟 <10ms
- [ ] AI 搜索自动触发（复杂查询）
- [ ] 新工具自动分类到工具箱（AI 自主）
- [ ] 新工具依赖关系自动分析（AI 自主）
- [ ] 后台索引重建不阻塞主线程
- [ ] 内存占用 <50MB（10,000 工具）

---

**作者**：AI Assistant  
**审核状态**：待审核  
**实施优先级**：高
