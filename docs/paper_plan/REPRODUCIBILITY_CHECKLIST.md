# Reproducibility Package Checklist

> **用途**: 确保研究可复现性，满足顶会要求
> **目标会议**: ACL 2027 / AAAI 2027 / EMNLP 2027
> **状态**: 🟡 准备中

---

## 📦 必需材料 (Required Materials)

### 1. 代码仓库 (Code Repository)

- [ ] **GitHub 仓库公开**
  - 仓库 URL: `https://github.com/silverenternal/tokitai`
  - 许可证：MIT OR Apache-2.0
  - 访问权限：公开 (public)

- [ ] **版本标签 (Release Tag)**
  - Paper A 对应版本：`v0.5.0-paper-a` (待创建)
  - Paper B 对应版本：`v0.5.0-paper-b` (待创建)
  - DOI 归档：通过 Zenodo 获取 DOI (投稿前创建)

- [ ] **代码文档**
  - [ ] README.md (安装、使用、测试说明)
  - [ ] CONTRIBUTING.md (贡献指南)
  - [ ] API 文档 (rustdoc 生成)
  - [ ] 架构文档 (docs/ARCHITECTURE.md)

- [ ] **依赖声明**
  - [ ] Cargo.toml (完整依赖列表)
  - [ ] Cargo.lock (锁定版本，确保可复现)
  - [ ] Rust 版本要求：`rustc 1.75+`

### 2. 实验数据 (Experimental Data)

#### Paper A: Parallel Context Architecture

- [ ] **基准测试任务定义**
  - 文件：`experiments/paper_a/benchmark_tasks.json`
  - 内容：24 个任务的详细描述、输入、预期输出
  - 格式：JSON Schema 验证

- [ ] **原始实验日志**
  - 目录：`experiments/paper_a/logs/`
  - 内容：
    - `user_study_raw.json` (12 名参与者的原始数据)
    - `performance_benchmarks.json` (1000 次操作延迟测量)
    - `branch_operations.log` (所有 fork/checkout/merge/abort 操作)
  - 隐私：匿名化处理 (移除参与者个人信息)

- [ ] **处理后的数据**
  - 目录：`experiments/paper_a/data/`
  - 文件：
    - `task_success_rates.csv` (任务成功率统计)
    - `latency_measurements.csv` (延迟测量汇总)
    - `satisfaction_scores.csv` (满意度评分)

- [ ] **分析脚本**
  - 目录：`experiments/paper_a/analysis/`
  - 脚本：
    - `analyze_success_rates.py` (成功率分析)
    - `analyze_latency.py` (延迟分析)
    - `generate_figures.py` (图表生成)
  - 依赖：`requirements.txt` (Python 包列表)

#### Paper B: HybridGapDetector

- [ ] **30 天进化实验日志**
  - 目录：`experiments/paper_b/logs/`
  - 内容：
    - `daily_evolution_logs.json` (每日进化决策记录)
    - `tool_creation_history.json` (工具创建历史)
    - `api_call_logs.json` (API 调用统计)
  - 隐私：移除敏感信息 (API keys, 用户数据)

- [ ] **缺口检测标注数据**
  - 文件：`experiments/paper_b/data/annotated_gaps.json`
  - 内容：100+ 人工标注的工具缺口
  - 标注者间一致性：Cohen's kappa 系数

- [ ] **成本分析数据**
  - 文件：`experiments/paper_b/data/cost_analysis.csv`
  - 内容：
    - API 调用次数 (按类型分类)
    - 每次调用成本
    - 月度总成本

- [ ] **分析脚本**
  - 目录：`experiments/paper_b/analysis/`
  - 脚本：
    - `analyze_detection_accuracy.py` (检测准确率分析)
    - `analyze_cost_effectiveness.py` (成本效益分析)
    - `plot_evolution_timeline.py` (进化时间线可视化)

### 3. Prompt 模板 (Prompt Templates)

#### Paper A Prompts

- [ ] **AI-Assisted Merge Prompts**
  - 文件：`prompts/paper_a/merge_prompts.json`
  - 内容：
    - `conflict_resolution_prompt`: 冲突解决 Prompt
    - `merge_recommendation_prompt`: 合并建议 Prompt
    - `branch_summary_prompt`: 分支总结 Prompt

- [ ] **Branch Purpose Inference Prompts**
  - 文件：`prompts/paper_a/purpose_inference.json`
  - 内容：
    - `purpose_classification_prompt`: 目的分类 Prompt
    - `branch_labeling_prompt`: 分支标签 Prompt

#### Paper B Prompts

- [ ] **HybridGapDetector Prompts**
  - 文件：`prompts/paper_b/gap_detection.json`
  - 内容：
    - `statistical_filter_prompt`: 统计过滤 Prompt
    - `causal_analysis_prompt`: 因果分析 Prompt (含 Chain-of-Thought)
    - `counterfactual_reasoning_prompt`: 反事实推理 Prompt

- [ ] **Prompt Engineering Components**
  - 文件：`prompts/paper_b/components.json`
  - 内容：
    - `prompt_gap_detector`: 缺口检测器 Prompt
    - `prompt_creator`: 工具创建器 Prompt
    - `prompt_optimizer`: 工具优化器 Prompt
    - `multi_agent_negotiator`: 多智能体协商 Prompt (4 个角色)

- [ ] **Few-Shot Examples**
  - 文件：`prompts/paper_b/few_shot_examples.json`
  - 内容：
    - 因果推理示例 (5 个)
    - 代码生成示例 (3 个)
    - 工具优化示例 (2 个)

### 4. 模型信息 (Model Information)

- [ ] **LLM 模型规格**
  - 文件：`docs/model_specs.md`
  - 内容：
    - 模型名称 (如：GPT-3.5-turbo, Qwen-2.5-72B)
    - 提供商 (OpenAI, Anthropic, etc.)
    - 上下文窗口大小
    - API 版本
    - 温度参数 (temperature)
    - 其他超参数

- [ ] **模型访问信息**
  - [ ] API 端点
  - [ ] 认证方式 (不含真实 API key)
  - [ ] 速率限制
  - [ ] 成本估算

- [ ] **替代模型说明**
  - 文件：`docs/alternative_models.md`
  - 内容：
    - 可替代的开源模型 (如：Llama-2-70B, Mistral)
    - 本地部署方案 (如：Ollama, vLLM)
    - 预期性能差异

### 5. 环境配置 (Environment Setup)

- [ ] **系统要求**
  - 文件：`docs/system_requirements.md`
  - 内容：
    - 操作系统：Linux (Ubuntu 20.04+), macOS (11+), Windows 10+
    - CPU: 4 核 +
    - 内存：8GB+ (推荐 16GB)
    - 存储：10GB 可用空间
    - 网络：需要访问 LLM API

- [ ] **安装脚本**
  - 文件：`scripts/setup.sh` (Linux/macOS)
  - 文件：`scripts/setup.ps1` (Windows)
  - 内容：
    ```bash
    # 安装 Rust
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    
    # 安装依赖
    cargo install --path .
    
    # 运行测试
    cargo test
    
    # 运行基准测试
    cargo bench
    ```

- [ ] **Docker 配置** (可选但推荐)
  - 文件：`Dockerfile.repro`
  - 文件：`docker-compose.yml`
  - 内容：完整容器化环境，包含所有依赖

- [ ] **环境变量模板**
  - 文件：`.env.example`
  - 内容：
    ```bash
    # LLM API Keys (用户需自行填写)
    OPENAI_API_KEY=your_key_here
    ANTHROPIC_API_KEY=your_key_here
    
    # 可选配置
    LOG_LEVEL=info
    EXPERIMENT_MODE=true
    ```

### 6. 运行说明 (Running Instructions)

- [ ] **快速开始指南**
  - 文件：`REPRODUCIBILITY.md`
  - 内容：
    ```markdown
    # 5 分钟快速开始
    1. 克隆仓库：`git clone <repo_url>`
    2. 安装依赖：`cargo install --path .`
    3. 配置 API key: 复制 `.env.example` 到 `.env`
    4. 运行测试：`cargo test`
    5. 运行基准：`cargo bench`
    ```

- [ ] **复现实验步骤**
  - 文件：`experiments/REPRODUCE.md`
  - 内容：
    ```markdown
    ## Paper A 实验复现
    1. 运行基准测试：`cargo bench --bench context_bench`
    2. 运行用户研究脚本：`python experiments/paper_a/analysis/run_study.py`
    3. 生成图表：`python experiments/paper_a/analysis/generate_figures.py`
    
    ## Paper B 实验复现
    1. 运行 30 天模拟：`cargo run --bin simulate_evolution -- --days 30`
    2. 分析准确率：`python experiments/paper_b/analysis/analyze_accuracy.py`
    3. 生成成本报告：`python experiments/paper_b/analysis/cost_report.py`
    ```

- [ ] **预期输出**
  - 文件：`experiments/expected_output.md`
  - 内容：
    - 性能指标范围 (如：Fork Latency 5-10ms)
    - 准确率范围 (如：70-75%)
    - 成本范围 (如：$2-3/month)

### 7. 测试套件 (Test Suite)

- [ ] **单元测试**
  - 命令：`cargo test --lib`
  - 覆盖率目标：>80%
  - 报告生成：`cargo tarpaulin --out html`

- [ ] **集成测试**
  - 命令：`cargo test --test integration`
  - 内容：端到端场景测试

- [ ] **基准测试**
  - 命令：`cargo bench`
  - 内容：性能基准测试
  - 输出：HTML 报告 (`target/criterion/report/index.html`)

- [ ] **复现性测试**
  - 命令：`cargo test --test reproducibility`
  - 内容：验证结果可复现性 (多次运行，统计方差)

---

## 📋 会议特定要求 (Conference-Specific Requirements)

### ACL 2027

- [ ] **Reproducibility Checklist** (预计要求)
  - [ ] 代码公开
  - [ ] 数据公开 (或说明访问限制)
  - [ ] 模型规格完整
  - [ ] 运行说明清晰
  - [ ] 实验脚本提供

- [ ] **Responsible NLP Checklist**
  - [ ] 数据使用声明
  - [ ] 潜在风险讨论
  - [ ] 伦理审查通过 (如适用)

### AAAI 2027

- [ ] **Supplementary Material**
  - [ ] 论文 PDF
  - [ ] 代码仓库链接
  - [ ] 实验数据链接
  - [ ] 匿名版本 (双盲评审期间)

- [ ] **Reproducibility Badge** (如申请)
  - [ ] 代码可运行
  - [ ] 结果可复现
  - [ ] 第三方验证通过

### EMNLP 2027

- [ ] **Ethics Statement**
  - [ ] 数据收集伦理
  - [ ] 参与者知情同意
  - [ ] 隐私保护措施

---

## 🔍 自查清单 (Self-Check Checklist)

### 代码质量

- [ ] 代码通过所有测试 (`cargo test` ✅)
- [ ] 无编译警告 (`cargo clippy` ✅)
- [ ] 代码格式化 (`cargo fmt` ✅)
- [ ] 关键函数有文档注释
- [ ] 复杂逻辑有解释性注释

### 数据完整性

- [ ] 所有原始数据已上传
- [ ] 数据处理脚本可运行
- [ ] 图表可重新生成
- [ ] 统计数据可验证

### 文档完整性

- [ ] README 清晰完整
- [ ] 安装说明经过测试
- [ ] 运行示例可执行
- [ ] 常见问题有解答

### 可访问性

- [ ] 仓库链接有效
- [ ] 数据下载链接有效
- [ ] 无需特殊权限即可访问
- [ ] 大文件使用 Git LFS 或外部存储

---

## 📅 时间表 (Timeline)

| 任务 | 截止日期 | 状态 |
|------|----------|------|
| 代码整理与文档完善 | 2026-06-30 | 🟡 待开始 |
| 实验数据匿名化 | 2026-07-15 | 🟡 待开始 |
| Prompt 模板整理 | 2026-07-15 | 🟡 待开始 |
| 分析脚本测试 | 2026-07-31 | 🟡 待开始 |
| Docker 配置 (可选) | 2026-08-01 | 🟡 待开始 |
| 内部复现测试 | 2026-08-07 | 🟡 待开始 |
| 最终检查 | 2026-08-10 | 🟡 待开始 |
| 提交 supplementary material | 2026-08-15 | 🟡 待开始 |

---

## 🔗 相关资源

- **ACL Reproducibility Guidelines**: https://aclrollingreview.org/reproducibility-checklist/
- **AAAI Supplementary Material**: https://aaai.org/aaai-conference/
- **EMNLP Ethics Guidelines**: https://www.emnlp2023.org/ethics-guidelines
- **Zenodo DOI 归档**: https://zenodo.org/
- **GitHub Archive Program**: https://archiveprogram.github.com/

---

**维护者**: Tokitai Development Team
**最后更新**: 2026-03-27
**下次更新**: 2026-06-30 (代码整理完成后)
