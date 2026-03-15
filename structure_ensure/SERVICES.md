# Tokitai 服务架构说明

> **版本**: 2.0.0
> **最后更新**: 2026-03-15
>
> 本文档清晰区分 Tokitai 项目的两种服务定位：**CLI AI 助手（面向用户）**和**项目自更新服务（面向项目自身）**

---

## 🎯 服务双轨架构

Tokitai 采用**双轨服务架构**，两种服务共享底层能力但定位和使用场景完全不同：

```
┌─────────────────────────────────────────────────────────────────┐
│                        Tokitai 双轨服务                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────┐    ┌─────────────────────────────┐│
│  │   CLI AI 助手            │    │   项目自更新服务             ││
│  │   (面向用户)            │    │   (面向项目自身)            ││
│  │                         │    │                             ││
│  │  📱 交互式对话          │    │  🤖 自主进化循环            ││
│  │  👤 用户驱动            │    │  🧠 AI 驱动                 ││
│  │  ⚡ 即时响应            │    │  🔄 迭代执行                ││
│  │  🛠️ 完成任务            │    │  📈 持续改进                ││
│  └─────────────────────────┘    └─────────────────────────────┘│
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    共享底层能力                              ││
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐  ││
│  │  │ToolMatrix│ │ Context  │ │Orchestrator│ │IntegratedModules││
│  │  │服务注册表 │ │ 上下文存储│ │ 编排调度  │ │ 集成模块      │  ││
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────────┘  ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 📱 服务一：CLI AI 助手（面向用户）

### 定位
**个人 AI 助手**，帮助用户完成日常开发任务和查询需求

### 启动方式
```bash
# 交互模式
cargo run --release

# 单次查询
cargo run --release -- "查看当前目录有哪些文件"
```

### 核心特点

| 特点 | 说明 |
|------|------|
| **用户驱动** | 等待用户输入，按需响应 |
| **即时响应** | 单次请求-响应模式 |
| **交互式** | 支持多轮对话，保持上下文 |
| **工具丰富** | 63+ 工具，覆盖文件/网络/Git/数据处理 |
| **安全沙箱** | 路径验证、命令黑名单、SSRF 防护 |

### 典型使用场景

1. **文件操作**
   ```
   👤 读取 README.md 的内容
   👤 创建 test.txt，写入 Hello World
   👤 在 src 目录搜索 .rs 文件
   ```

2. **代码分析**
   ```
   👤 分析 @src/main.rs 的结构
   👤 统计 main.rs 有多少行代码
   👤 这个函数的作用是什么？
   ```

3. **网络请求**
   ```
   👤 GET 请求 https://api.github.com
   👤 下载 https://example.com/file.pdf
   👤 搜索关于 transformer 的 arXiv 论文
   ```

4. **Git 操作**
   ```
   👤 查看 git 状态
   👤 查看最近的提交记录
   👤 当前分支是什么
   ```

5. **数据处理**
   ```
   👤 格式化这段 JSON
   👤 查询 JSON 中的 user.name 字段
   👤 提取 document.pdf 的文本
   ```

### 核心能力

```rust
pub struct AiAssistant {
    // 工具实例
    file_ops: FileOperations,
    system_tools: SystemTools,
    code_tools: CodeTools,
    web_search: WebSearchTools,
    git_ops: GitOperations,
    // ... 63+ 工具

    // 工具矩阵（服务注册表）
    tool_registry: ToolRegistry,
    tool_selector: ToolSelector,
    lightweight_selector: Arc<LightweightToolSelector>,
    tool_dispatcher: Arc<ToolDispatcher>,

    // 集成模块
    integrated_modules: IntegratedModules,  // dialogue/observability/prompt

    // 编排器
    orchestrator: Orchestrator,  // 角色切换、工作流引擎

    // ... 其他字段
}

impl AiAssistant {
    /// 创建 CLI 助手（非自主模式）
    pub fn new(api_url: String, api_key: Option<String>, model: String) -> Self;

    /// 聊天并处理工具调用
    pub fn chat_and_handle_tools(&mut self, messages: &mut Vec<Value>, input: &str) -> Result<String>;
}
```

### 编排器命令

```bash
/role <name>       # 切换角色（planner/executor/reviewer/researcher）
/optimize          # 优化上下文
/context           # 显示上下文状态
/roles             # 显示角色信息
/workflow list     # 列出可用工作流
/workflow start    # 启动工作流
/toolbox           # 显示工具箱状态
/help              # 显示所有命令
```

### 服务边界

- ✅ **响应用户查询**
- ✅ **执行用户指定的工具调用**
- ✅ **保持对话上下文**
- ✅ **提供建议和指导**
- ❌ **不主动修改项目代码**
- ❌ **不自主发起 Git 操作**
- ❌ **不自主推送代码**

---

## 🤖 服务二：项目自更新服务（面向项目自身）

### 定位
**自主进化系统**，AI 自主发现项目改进点并实施，持续优化项目

### 启动方式
```bash
# 自主进化模式（默认当前目录）
cargo run --release -- --autonomous

# 指定项目路径
cargo run --release -- --autonomous --project-path ./sandbox/test-project
```

### 核心特点

| 特点 | 说明 |
|------|------|
| **AI 驱动** | AI 自主分析项目，发现改进点 |
| **迭代循环** | Planner → Executor → Reviewer 循环 |
| **自主执行** | 无需用户干预，自动完成任务 |
| **Git 集成** | 自动生成提交并推送（可选） |
| **持续改进** | 每次迭代优化项目 |

### 工作流程

```
┌─────────────────────────────────────────────────────────────────┐
│                     自主进化迭代循环                             │
│                                                                 │
│  1. 分析项目状态                                                 │
│     └─→ 读取项目结构、代码质量、测试覆盖率                       │
│                                                                 │
│  2. 发现改进点                                                   │
│     └─→ 识别代码异味、缺失功能、性能瓶颈                         │
│                                                                 │
│  3. 制定改进计划                                                 │
│     └─→ Planner Agent 生成任务列表（DAG 依赖分析）               │
│                                                                 │
│  4. 执行改进任务                                                 │
│     └─→ Executor Agent 按计划执行（工具矩阵调度）                │
│                                                                 │
│  5. 审查代码变更                                                 │
│     └─→ Reviewer Agent 代码审查（本地 fmt/clippy/test）          │
│                                                                 │
│  6. 提交并推送（可选）                                           │
│     └─→ Git 工作流自动提交变更                                   │
│                                                                 │
│  7. 继续下一轮迭代                                               │
│     └─→ 回到步骤 1，持续改进                                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 典型使用场景

1. **代码质量改进**
   ```
   - 自动修复 Clippy 警告
   - 添加缺失的单元测试
   - 重构复杂函数
   - 添加文档注释
   ```

2. **功能扩展**
   ```
   - 添加新的工具函数
   - 实现缺失的 API 端点
   - 集成新的第三方库
   - 扩展配置文件支持
   ```

3. **性能优化**
   ```
   - 识别性能瓶颈
   - 优化算法复杂度
   - 添加缓存机制
   - 减少内存分配
   ```

4. **技术债务清理**
   ```
   - 删除废弃代码
   - 更新过时的依赖
   - 统一代码风格
   - 修复类型安全问题
   ```

### 核心能力

```rust
pub struct AiAssistant {
    // ... 共享字段（与 CLI 助手相同）

    // 自主进化专属字段
    coordinator: Option<Arc<RwLock<AgentCoordinator>>>,  // 多 Agent 协调器
    git_workflow: Option<GitWorkflow>,                    // Git 工作流
    autonomous_mode: bool,                                // 自主模式标志
}

impl AiAssistant {
    /// 创建自主模式助手
    pub fn new_autonomous(
        api_url: String,
        api_key: Option<String>,
        model: String,
        project_path: PathBuf,
    ) -> Result<Self>;

    /// 运行自主进化
    pub fn run_autonomous_evolution(&self) -> Result<()>;

    /// 执行单次进化迭代
    fn execute_evolution_iteration(
        &self,
        coordinator: &AgentCoordinator,
        goal: &str,
    ) -> Result<()>;
}
```

### Agent 协作架构

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  Planner     │ ──▶ │  Executor    │ ──▶ │  Reviewer    │
│  规划 Agent   │     │  执行 Agent   │     │  审查 Agent   │
└──────────────┘     └──────────────┘     └──────────────┘
       ▲                                        │
       │                                        ▼
       │                              ┌──────────────┐
       │                              │  Git Workflow │
       │                              │  提交/推送    │
       │                              └──────────────┘
       │                                        │
       └────────────────────────────────────────┘
                        迭代循环
```

### 服务边界

- ✅ **自主分析项目状态**
- ✅ **自主发现改进点**
- ✅ **自主制定并执行计划**
- ✅ **自主代码审查**
- ✅ **自主 Git 提交（可选）**
- ❌ **不响应用户交互**
- ❌ **不处理外部查询**
- ❌ **不提供服务接口**

---

## 🔄 服务对比

| 维度 | CLI AI 助手 | 项目自更新服务 |
|------|------------|---------------|
| **服务对象** | 用户（开发者） | 项目自身 |
| **驱动方式** | 用户输入驱动 | AI 自主驱动 |
| **交互模式** | 交互式对话 | 自主迭代循环 |
| **响应模式** | 即时响应 | 批量执行 |
| **执行时长** | 秒级（单次任务） | 分钟级（多轮迭代） |
| **Git 操作** | 仅查询状态 | 可自动提交推送 |
| **代码修改** | 用户明确指令 | 自主决定修改 |
| **使用频率** | 按需使用 | 定期/持续运行 |
| **典型场景** | 查询、分析、临时任务 | 代码改进、技术债务清理 |

---

## 🏗️ 共享底层能力

两种服务共享以下核心模块：

### 1. ToolMatrix（工具矩阵/服务注册表）
- **职责**: 工具注册、分类、选择、调用分发
- **核心组件**:
  - `ToolRegistry`: 工具注册表（AI 分类/依赖分析）
  - `LightweightToolSelector`: 轻量级工具选择器（<10ms 搜索）
  - `AIToolboxClassifier`: AI 工具箱分类器
  - `AIDependencyAnalyzer`: AI 依赖关系分析器
  - `ToolDispatcher`: 工具调用分发器

### 2. Context Storage（上下文存储）
- **职责**: 对话上下文、项目状态存储
- **核心特性**:
  - 三层存储架构（瞬时/短期/长期）
  - 增量哈希链（ICHC）
  - 上下文蒸馏（HCD）
  - 语义索引（LSFI）

### 3. Orchestrator（编排调度）
- **职责**: 角色切换、工作流引擎
- **核心组件**:
  - `RoleSwitcher`: 角色切换（planner/executor/reviewer）
  - `WorkflowEngine`: 声明式工作流执行
  - `WorkflowLoader`: TOML 工作流加载器

### 4. IntegratedModules（集成模块）
- **职责**: 统一管理 dialogue/observability/prompt_engineering
- **核心特性**:
  - 共享状态管理（`Arc<RwLock>`）
  - 统一生命周期管理
  - 与 autonomy 模块状态同步

### 5. Autonomy Agents（自主进化 Agent）
- **职责**: 多 Agent 协作系统
- **核心组件**:
  - `PlannerAgent`: 规划 Agent
  - `ExecutorAgent`: 执行 Agent
  - `ReviewerAgent`: 审查 Agent
  - `AgentCoordinator`: 协调器
  - `TaskDecomposer`: 任务分解引擎（DAG）
  - `IterationTracker`: 迭代追踪器（事件溯源）
  - `GitWorkflow`: 自主 Git 工作流

---

## 📋 使用建议

### 选择 CLI AI 助手
当你需要：
- ✅ 快速查询项目信息
- ✅ 分析代码结构
- ✅ 执行临时任务（文件操作、网络请求）
- ✅ 获取建议和指导
- ✅ 多轮对话讨论问题

### 选择项目自更新服务
当你需要：
- ✅ 持续改进代码质量
- ✅ 自动修复技术问题
- ✅ 清理技术债务
- ✅ 添加常规功能
- ✅ 定期维护项目

### 组合使用
```bash
# 1. 先用 CLI 助手了解项目
cargo run --release
👤 分析当前项目的结构

# 2. 启动自主进化服务进行改进
cargo run --release -- --autonomous

# 3. 再用 CLI 助手检查改进结果
cargo run --release
👤 查看最近的 Git 提交记录
```

---

## 🔒 安全考虑

### CLI AI 助手
- **沙箱隔离**: 路径验证、命令黑名单
- **用户确认**: 危险操作需要用户确认
- **SSRF 防护**: 内网 IP 过滤
- **审计日志**: 所有工具调用记录到 tracing

### 项目自更新服务
- **本地审查**: 自动执行 fmt/clippy/test
- **回滚机制**: 失败时自动回滚
- **Git 隔离**: 在独立分支操作（可选）
- **推送确认**: 可配置为仅提交不推送

---

## 📊 性能指标

| 指标 | CLI AI 助手 | 项目自更新服务 |
|------|------------|---------------|
| **首次响应延迟** | <2s | N/A |
| **工具搜索延迟** | <10ms（缓存命中 ~3ms） | <10ms |
| **单次迭代时长** | N/A | 5-30 分钟 |
| **内存占用** | ~50MB | ~80MB（含 Agent 状态） |
| **并发能力** | 10 请求/秒 | 1 迭代/5 分钟 |

---

## 🚀 未来规划

### CLI AI 助手
- [ ] 支持语音交互
- [ ] 增强代码理解能力
- [ ] 支持多项目上下文
- [ ] 集成更多开发工具

### 项目自更新服务
- [ ] 支持自定义改进策略
- [ ] 增强代码审查能力
- [ ] 支持 PR 自动创建
- [ ] 集成 CI/CD 检查

---

**最后更新**: 2026-03-15
**测试状态**: 236/236 通过 ✅
**构建状态**: Release 成功
