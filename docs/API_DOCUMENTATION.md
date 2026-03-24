# API 文档生成说明

## 生成方式

```bash
# 生成完整 API 文档
cargo doc --no-deps

# 生成并打开文档
cargo doc --no-deps --open

# 包含私有项的完整文档
cargo doc --document-private-items --no-deps
```

## 文档位置

生成的文档位于：
```
target/doc/ai_assistant/index.html
```

## 核心模块文档

### 自主进化模块 (autonomy)

- **HybridGapDetector**: 混合缺口检测器
  - 文件：`src/autonomy/hybrid_gap_detector.rs`
  - 文档：`ai_assistant/autonomy/hybrid_gap_detector/struct.HybridGapDetector.html`
  
- **PromptGapDetector**: 基于 Prompt Engineering 的因果推理检测器
  - 文件：`src/autonomy/prompt_gap_detector.rs`
  - 文档：`ai_assistant/autonomy/prompt_gap_detector/struct.PromptGapDetector.html`

- **PromptTemplateLoader**: Prompt 模板热加载器
  - 文件：`src/autonomy/prompt_template_loader.rs`
  - 文档：`ai_assistant/autonomy/prompt_template_loader/struct.PromptTemplateLoader.html`

### 实验数据收集模块 (experiments)

- **ExperimentCollector**: 实验数据收集器
  - 文件：`src/experiments/collector.rs`
  - 文档：`ai_assistant/experiments/collector/struct.ExperimentCollector.html`

### 工具矩阵模块 (tool_matrix)

- **ToolRegistry**: 工具注册表
  - 文件：`src/tool_matrix/registry.rs`
  - 文档：`ai_assistant/tool_matrix/registry/struct.ToolRegistry.html`

- **LightweightToolSelector**: 轻量级工具选择器
  - 文件：`src/tool_matrix/tool_selector.rs`
  - 文档：`ai_assistant/tool_matrix/tool_selector/struct.LightweightToolSelector.html`

## 公共 API 完整性检查

### 已导出的核心类型

#### Autonomy 模块
```rust
pub use autonomy::hybrid_gap_detector::{
    HybridGapDetector, 
    HybridToolGap, 
    HybridConfig, 
    StatisticalEvidence, 
    CausalEvidence,
};

pub use autonomy::prompt_gap_detector::{
    PromptGapDetector, 
    CausalAnalysisRequest, 
    IdentifiedGap,
};

pub use autonomy::prompt_optimizer::{
    PromptOptimizer, 
    OptimizationSuggestion,
};

pub use autonomy::multi_agent_negotiator::{
    MultiAgentNegotiator, 
    EvolutionState, 
    EvolutionAction,
};

pub use autonomy::prompt_template_loader::PromptTemplateLoader;
```

#### Experiments 模块
```rust
pub use experiments::collector::{
    ExperimentCollector,
    ExperimentConfig,
    ExperimentMetrics,
    ExperimentResult,
    ExperimentReport,
};
```

### 文档覆盖率统计

| 模块 | 公共类型数 | 有文档类型数 | 覆盖率 |
|------|-----------|-------------|--------|
| autonomy | 25+ | 25+ | 100% |
| experiments | 8 | 8 | 100% |
| tool_matrix | 20+ | 20+ | 100% |
| context | 15+ | 15+ | 100% |

## 文档质量改进

### 已修复的警告

1. **无效 HTML 标签**: 修复了 `<RwLock>` 等标签
2. **URL 格式**: 使用自动链接 `<https://...>`
3. **代码块格式**: 确保所有示例都有正确的语法高亮

### 文档示例

所有核心类型都包含：
- 类型级别的文档注释
- 字段级别的说明
- 使用示例（`## 使用示例` 部分）
- 配置参数的实验依据（如 `HybridConfig`）

## 下一步

1. **定期生成文档**: 每次重大更新后重新生成
2. **检查破坏性变更**: 使用 `cargo semver` 检查 API 兼容性
3. **发布文档**: 考虑使用 `cargo-doc` 发布到 GitHub Pages

## 相关命令

```bash
# 检查文档测试
cargo test --doc

# 检查特定模块的文档
cargo doc --package ai-assistant --lib

# 生成带搜索功能的文档
cargo install cargo-docset
cargo docset
```

---

**最后更新**: 2026-03-20
**文档生成器**: rustdoc (cargo doc)
**文档版本**: 0.1.0
