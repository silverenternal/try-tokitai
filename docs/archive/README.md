# 文档归档说明

> **归档日期**: 2026-03-20
> **归档目的**: 整理历史技术报告，保留核心参考价值，移除冗余文档

---

## 📁 归档原则

### 保留标准
- ✅ **设计文档**: 系统架构、核心模块设计（论文 Method 章节素材）
- ✅ **实现报告**: 关键功能实现细节（HybridGapDetector、Prompt Engineering 系统）
- ✅ **实验设计**: 实验框架、基准任务定义（论文 Experiments 章节素材）
- ✅ **性能报告**: 有真实数据的性能分析

### 移除标准
- ❌ **重复报告**: 内容高度相似的多个版本
- ❌ **过程文档**: 临时性的实施记录、周报
- ❌ **过时文档**: 已被新文档替代的旧版本
- ❌ **无数据宣称**: 没有实验数据支撑的性能宣称

---

## 📂 归档结构

```
docs/archive/
├── README.md                     # 本文档（归档索引）
│
├── 01_architecture/              # 架构设计
│   ├── ARCHITECTURE_IMPROVEMENT_PLAN.json   # 架构改进计划
│   └── SERVICE_ARCHITECTURE_IMPLEMENTATION.md - 服务化架构实施
│
├── 02_tool_matrix/               # 工具矩阵
│   ├── TOOL_MATRIX_INTEGRATION.md
│   ├── TOOL_SELECTOR_IMPLEMENTATION.md
│   ├── TOOL_SELECTOR_SUMMARY.md
│   ├── LIGHTWEIGHT_TOOL_SELECTION_DESIGN.md
│   ├── LIGHTWEIGHT_TOOL_SELECTION_DEEPENING.md
│   └── LIGHTWEIGHT_TOOL_SELECTION_FINAL_SUMMARY.md
│
├── 03_autonomy/                  # 自主进化
│   ├── AUTONOMOUS_EVOLUTION_GUIDE.md
│   ├── AUTONOMOUS_EVOLUTION_INTEGRATION_REPORT.md
│   └── AUTONOMY_TOOL_MATRIX_INTEGRATION.md
│
├── 04_context/                   # 上下文存储
│   ├── CONTEXT_STORAGE.md
│   ├── CONTEXT_FEATURES.md
│   └── CONTEXT_REFACTOR_REPORT.md
│
├── 05_network/                   # 网络优化
│   ├── NETWORK_OPTIMIZATION_REPORT.md
│   ├── NETWORK_OPTIMIZATION_FINAL_REPORT.md
│   ├── NETWORK_TOOLS_GUIDE.md
│   └── NETWORK_SEARCH_SKILLS.md
│
├── 06_integration/               # 模块集成
│   ├── INTEGRATION_PLAN.md
│   ├── MODULE_INTEGRATION_REPORT.md
│   ├── MODULE_IMPROVEMENT_REPORT.md
│   └── NEW_FEATURES.md
│
├── 07_quality/                   # 质量改进
│   ├── CODE_REVIEW_REPORT.md
│   ├── PRIORITY_FIX_REPORT.md
│   ├── TEST_OPTIMIZATION_REPORT.md
│   └── GOOGLE_DEPENDENCY_REMOVAL_REPORT.md
│
├── 08_benchmarks/                # 基准测试
│   └── BENCHMARK_REPORT.md
│
└── 99_deprecated/                # 已废弃（待删除）
    ├── MessageBoard.md
    └── IMPLEMENTATION_STATUS_REPORT.md
```

---

## 🔄 文档更新流程

### 新增文档
1. 判断文档类型（设计/实现/实验/性能）
2. 放入对应分类目录
3. 更新本文档索引

### 归档旧文档
1. 标记为"已归档"
2. 移动到 `archive/` 对应分类
3. 在原文档位置留下重定向说明

### 删除文档
1. 确认无参考价值
2. 确认无引用依赖
3. 移动到 `deprecated/`（保留 30 天后删除）

---

## 📊 文档统计

| 分类 | 文档数 | 总页数（估计） | 核心价值 |
|------|--------|---------------|---------|
| 架构设计 | 2 | 30 | ⭐⭐⭐⭐⭐ |
| 工具矩阵 | 6 | 80 | ⭐⭐⭐⭐ |
| 自主进化 | 3 | 40 | ⭐⭐⭐⭐⭐ |
| 上下文存储 | 3 | 30 | ⭐⭐⭐ |
| 网络优化 | 4 | 50 | ⭐⭐⭐ |
| 模块集成 | 4 | 50 | ⭐⭐⭐ |
| 质量改进 | 4 | 40 | ⭐⭐ |
| 基准测试 | 1 | 20 | ⭐⭐⭐⭐ |
| 已废弃 | 2 | 20 | ⭐ |

**总计**: 29 份文档，约 360 页

---

## 🎯 论文写作参考

### 可直接引用的文档

| 论文章节 | 参考文档 | 内容 |
|---------|---------|------|
| **Introduction** | SERVICES.md | 双轨服务架构、研究动机 |
| **Related Work** | （需补充） | 工具学习、自进化系统相关工作 |
| **Method** | paper_plan/MECHANISMS.md | Prompt Engineering 核心机制 |
| **Method** | HYBRID_GAP_DETECTOR_IMPLEMENTATION.md | HybridGapDetector 设计 |
| **Implementation** | 各实现报告 | 系统实现细节 |
| **Experiments** | experiments/scripts/ | 实验框架设计 |
| **Experiments** | （待生成） | 实验结果数据 |

### 需补充的文档

- [ ] **Related Work 综述**: 工具学习、自进化系统、Prompt Engineering 相关工作
- [ ] **实验结果报告**: 30 天实验数据、对比分析、消融实验
- [ ] **用户研究**: 用户满意度调查、定性反馈

---

## 📝 文档维护清单

### 高优先级（论文核心）
- [x] SERVICES.md - 服务架构说明
- [x] paper_plan/MECHANISMS.md - 核心机制设计
- [x] HYBRID_GAP_DETECTOR_IMPLEMENTATION.md - HybridGapDetector 实现
- [ ] experiments/results/ - 实验结果（待生成）

### 中优先级（实现参考）
- [x] TOOL_MATRIX 系列文档
- [x] AUTONOMOUS_EVOLUTION 系列文档
- [x] NETWORK_OPTIMIZATION 系列文档

### 低优先级（历史参考）
- [ ] 归档到 archive/ 的文档
- [ ] deprecated/ 中的文档（30 天后删除）

---

**最后更新**: 2026-03-20
**维护人**: AI Assistant
**下次审查**: 2026-04-20（实验完成后）
