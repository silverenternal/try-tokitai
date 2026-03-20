# Changelog

All notable changes to tokitai will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### 📚 文档整理

- 移动已完成的计划性文档到 `docs/archive/`
  - `ARCHITECTURE_IMPROVEMENT_PLAN.json` - 架构改进计划（已归档）
  - `IMPLEMENTATION_STATUS_REPORT.md` - 实施状态报告（已归档）
- 更新所有文档中的归档链接
- 更新 `structure_ensure/` 目录下的文档导航
- 更新主 `README.md` 技术报告链接

---

## [2.0.0] - 2026-03-18

### 🎉 AI 原生工具选择器 + 完整工具矩阵

本次发布引入了 AI 原生工具选择器系统和完整的工具矩阵 (IMP-001~004)，大幅提升了工具搜索性能和 AI 自主管理能力。

### ✨ 新增功能

#### AI 原生工具选择器
- **LightweightToolSelector** - 轻量级工具选择器
  - 快速搜索 <10ms
  - AI 搜索 <2s（含 LLM 调用）
  - LRU 缓存命中后 ~3ms（降低 62.5%）
  - 后台异步索引重建 ~600ms（降低 25%）
  - 完整监控指标（SelectorMetrics）

- **ToolIndex** - 倒排索引
  - 关键词索引
  - 分类索引
  - 工具箱索引

- **AIToolboxClassifier** - AI 工具箱分类器
  - AI 自主管理工具箱
  - 自动分类工具
  - 创建新工具箱

- **AIDependencyAnalyzer** - AI 依赖关系分析器
  - 静态分析依赖
  - 运行时日志学习
  - 智能工具推荐

- **ToolDispatcher** - 工具调用分发器
  - 统一工具调用入口
  - 调用统计收集
  - 执行器注册

#### 完整工具矩阵 (IMP-001~004)
- **IMP-001: 规则分类器**
  - `rule_classifier.rs` - 规则分类器核心
  - `HierarchicalClassifier` - 分层分类器
  - L1 精确缓存 (~0.1ms)
  - L2 模糊缓存 (~1ms)
  - L3 规则分类 (~5ms)
  - L4 LLM 分类 (~1.5s)
  - `from_tool_tags()` - 从工具标签自动构建规则
  - `merge_from_tool_tags()` - 合并工具标签规则

- **IMP-002: 工具生成器**
  - `tool_generator.rs` - 工具生成核心
  - `generate_with_tokitai_macro()` - 使用 tokitai 宏生成
  - Tera 模板引擎集成
  - JSON Schema 参数解析
  - 代码模板和测试模板

- **IMP-003: Trie 索引**
  - `trie_index.rs` - Trie 树索引
  - `BKTree` - BK-Tree 拼写纠正
  - `HybridIndex` - 混合索引
  - 搜索优化

- **IMP-004: 动态注册表**
  - `dynamic_registry.rs` - 动态工具注册表
  - `DynamicToolBuilder` - 动态工具构建器
  - 热加载支持
  - 运行时添加/移除工具

#### 双轨服务架构
- **CLI AI 助手模式** - 面向用户
  - 交互式对话
  - 用户驱动
  - 即时响应

- **项目自更新服务模式** - 面向项目自身
  - AI 自主驱动
  - Planner-Executor-Reviewer 迭代循环
  - 自主代码审查
  - 自主 Git 提交（可选）

#### 集成模块
- **IntegratedModules** - 统一管理
  - dialogue 模块集成
  - observability 模块集成
  - prompt_engineering 模块集成
  - 共享状态管理 (`Arc<RwLock>`)
  - 统一生命周期管理
  - 优雅降级

### 📊 项目规模

| 指标 | 旧值 | 新值 | 变化 |
|------|------|------|------|
| 代码行数 | ~26,600 | ~27,500 | +900 行 |
| 源文件数 | 78 | 99 | +21 个 |
| tool_matrix 文件 | 10 | 15 | +5 个 |
| tool_matrix 行数 | 3,362 | 4,200 | +838 行 |

### 🧪 测试

- 新增 tool_matrix 测试 11 个
- 总测试数：236 个
- 通过率：100% ✅

### 📚 文档

- 新增 `structure_ensure/` 目录
- 更新 `README.md` 为完整项目说明
- 更新 `FEATURE_SPEC.md` 添加 IMP-001~004 说明
- 更新 `TECHNICAL_SPEC.md` 添加工具矩阵详情
- 更新 `QUICKSTART.md` 添加双轨服务说明

---

## [1.0.0] - 2026-03-14

### 🎉 首个稳定发行版

这是 tokitai 的第一个稳定发行版，包含完整的 AI 助手功能和生产级安全特性。

### ✨ 新增功能

#### 监控命令（Phase 2）
- **`/health` 命令** - 系统健康检查
  - AI API 连接检查
  - Git 仓库状态检查
  - 文件权限检查
  - 磁盘空间检查
  - 环境变量配置检查

- **`/stats` 命令** - 自主进化统计
  - 迭代次数统计（总次数/成功/失败）
  - 成功率计算
  - 平均迭代时长
  - 文件修改次数统计
  - 事件类型分布

- **`/optimize` 命令** - 缓存清理
  - 文件缓存清理
  - 临时文件清理
  - 上下文缓存清理
  - HTTP 连接池回收提示

#### 工具增强
- **`edit_file` 工具** - 文件编辑支持
  - `append` 模式：文件末尾追加
  - `prepend` 模式：文件开头插入
  - `replace` 模式：替换指定文本

- **`--project-path` 参数** - 指定自主进化目标
  - `-p` 简写支持
  - 沙箱项目隔离测试
  - 主项目保护

#### 核心功能
- **纯文件上下文存储系统**
  - 三层存储架构（瞬时/短期/长期）
  - SHA256 哈希去重
  - 符号链接索引
  - 增量日志
  - 自动裁剪机制

- **任务分解引擎**
  - `task_decomposer.rs` 完整实现
  - 支持复杂任务分解为可执行子任务
  - 优先级排序和依赖分析
  - 增量执行（暂停/恢复/回滚）

- **迭代循环系统**
  - Research/Develop/Critic 三重 Agent 循环
  - 状态机自动流转
  - 检查点保存和错误恢复
  - 任务进度追踪

#### 安全增强
- **合规报告生成器**
  - `compliance_report.rs` 完整实现
  - 定期生成安全报告
  - 合规性评分
  - 改进建议

- **增量确认功能**
  - pause/resume 实现
  - 上下文快照保存
  - 等待用户审查代码变更

- **项目习惯记忆**
  - `.tokitai/project_conventions.md` 自动记忆
  - 项目约定持久化
  - 自动应用历史约定

#### 工具系统
- **50+ 工具支持**
  - 文件操作：5 个工具
  - 系统命令：4 个工具
  - 代码分析：4 个工具
  - 网络搜索：3 个工具
  - 文件下载：3 个工具
  - Git 操作：4 个工具
  - HTTP 客户端：4 个工具
  - JSON 处理：6 个工具
  - 文件搜索：5 个工具
  - 进程管理：6 个工具
  - 网络工具：6 个工具

- **工具链支持**
  - `download_and_analyze` - 下载并分析
  - `git_status_check` - Git 状态检查
  - `code_review` - 代码审查

#### 身份系统
- **7 种预定义身份**
  - `assistant` - 只读工具
  - `developer` - 开发工具
  - `researcher` - 搜索工具
  - `analyst` - 分析工具
  - `operator` - 系统工具
  - `auditor` - 审计工具
  - `admin` - 全部工具

- **身份感知权限**
  - 每种身份有不同的工具白名单
  - 速率限制独立配置
  - 身份切换审计

### 🔒 安全特性

- **文件沙箱**
  - 仅允许访问项目目录内文件
  - 阻止敏感路径访问（/etc, /root, .ssh）
  - 路径遍历检测

- **命令沙箱**
  - 40+ 命令白名单
  - 20+ 危险模式检测
  - 命令参数验证

- **网络沙箱**
  - 20+ 白名单主机
  - 9 个内网 IP 段阻止（SSRF 防护）
  - 重定向检查

- **敏感文件过滤器**
  - 70+ 敏感模式匹配
  - 10+ 敏感目录阻止

- **预提交检查**
  - 敏感文件检查
  - 代码格式化
  - Clippy 检查
  - 测试运行

### 📊 性能优化

- **缓存优化**
  - 重复查询延迟 <10ms（100x 提升）
  - 全局 HTTP 连接池复用
  - 零连接开销

- **异步优化**
  - 纯异步线程模型
  - 无线程阻塞
  - 实时延迟监控

- **传输优化**
  - 云端传输量减少 60%
  - 摘要 + 哈希替代完整上下文

### 📚 文档

- **用户指南** (USER_GUIDE.md)
  - 快速入门
  - 配置指南
  - 使用指南
  - TUI 界面使用
  - 工具系统详解
  - 多模型支持
  - 故障排除
  - 最佳实践

- **性能基准报告** (docs/BENCHMARK_REPORT.md)
  - criterion 框架基准测试
  - 详细性能指标
  - 优化建议

- **阶段 1 完成报告** (docs/PHASE1_COMPLETION_REPORT.md)
  - Phase 1 任务完成情况
  - 测试结果
  - 验收标准达成

### 🧪 测试

- **161 个测试全部通过**
  - 单元测试
  - 集成测试
  - 基准测试

### 🛠️ 技术栈

- Rust 1.75+
- tokio 1.50 - 异步运行时
- reqwest 0.12 - HTTP 客户端
- moka 0.12 - 高性能缓存
- serde_json 1.0 - JSON 处理
- regex 1.10 - 正则表达式
- chrono 0.4 - 日期时间

### 📦 依赖更新

- tokitai 0.4.0
- tokitai-core 0.4.0

### 🔜 未来计划

v1.0.0 发布后的可选增强功能：
- 语义索引（jieba-rs 中文分词）
- 云同步优化
- 代码知识图谱
- 架构模式识别
- 自定义身份支持
- 工具模板生成
- 外部工具集成（插件系统）

---

## [0.4.0] - 2026-03-12

### 新增
- 网络工具集：ping、端口检查、端口扫描、路由追踪、公网 IP
- 上下文存储系统重构
- 性能优化：缓存命中率提升

### 优化
- 重构工具模块结构
- 改进错误处理

---

## [0.3.0] - 2026-03-11

### 新增
- HTTP 客户端工具集
- JSON 处理工具集
- 文件搜索工具集
- 进程管理工具集
- TUI 界面基础功能
- 下载工具（arxiv 支持）
- Git 操作工具集

---

## [0.2.0] - 2026-03-11

### 新增
- 多模型支持（OpenAI/Anthropic/Gemini/Qwen）
- Qwen OAuth 登录
- 新手配置指南

---

## [0.1.0] - 2026-03-08

### 新增
- 初始版本
- 基础 AI 助手功能
- 文件操作工具
- 系统命令工具
- 代码分析工具

---

## 版本说明

### 语义化版本

tokitai 遵循语义化版本 2.0.0：

- **主版本号 (Major)**: 不兼容的 API 变更
- **次版本号 (Minor)**: 向后兼容的功能新增
- **修订号 (Patch)**: 向后兼容的问题修复

### 发布周期

- **主版本**: 每季度发布
- **次版本**: 每月发布
- **修订版**: 按需发布

### 支持政策

- **v1.0.x**: 长期支持 (LTS)，12 个月安全更新
- **v0.x.x**: 不支持，建议升级到 v1.0

---

## 相关链接

- [GitHub Releases](https://github.com/silverenternal/tokitai/releases)
- [项目文档](docs/README.md)
- [用户指南](USER_GUIDE.md)
- [性能基准报告](docs/BENCHMARK_REPORT.md)
