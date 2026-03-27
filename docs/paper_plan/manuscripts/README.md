# 论文手稿文件夹

> **项目**: Self-Evolving Tool Ecosystem for AI Agents
> **核心平台**: tokitai (Rust AI 工具调用框架)
> **创建日期**: 2026-03-27

---

## 📁 文件夹结构

```
manuscripts/
├── README.md                    # 本文件
├── paper_template.tex           # LaTeX 论文模板
├── paper_template.md            # Markdown 写作模板
├── paper_a/                     # 论文 A: Git 分支式上下文管理
│   ├── draft.md                 # 当前草稿
│   ├── figures/                 # 图表文件夹
│   └── references.bib           # 参考文献
├── paper_b/                     # 论文 B: 自进化工具生态系统
│   ├── draft.md                 # 当前草稿
│   ├── figures/                 # 图表文件夹
│   └── references.bib           # 参考文献
└── shared/                      # 共享资源
    ├── figures/                 # 通用图表
    ├── tables/                  # 通用表格
    └── prompts/                 # Prompt 模板集合
```

---

## 📝 论文列表

### 论文 A: Parallel Context Architecture
- **标题**: Parallel Context Architecture: Git-like Branching for AI Agent Memory
- **目标会议**: ACL 2027 (Systems and Infrastructure track)
- **截止日期**: 2027-02-15
- **当前状态**: 初稿 6500 字，等待实验数据
- **核心贡献**:
  1. Context Branch Primitives (fork/checkout/merge/abort)
  2. Copy-on-Write Implementation
  3. AI-Assisted Merge
  4. Comprehensive Evaluation

### 论文 B: Self-Evolving Tool Ecosystem
- **标题**: Self-Evolving Tool Ecosystem: Enabling AI Agents with Proactive Tool Management via Prompt Engineering
- **目标会议**: AAAI 2027
- **截止日期**: 2026-08-15
- **当前状态**: 规划中
- **核心贡献**:
  1. Causal Reasoning Prompt Design
  2. Multi-Agent Negotiation Protocol
  3. Self-Correcting Code Generation
  4. Tool Matrix Architecture

---

## 🎯 写作规范

### 格式要求

| 项目 | 论文 A (ACL) | 论文 B (AAAI) |
|------|--------------|---------------|
| **篇幅** | 8页 (正文) | 7页 (正文) |
| **参考文献** | 不限 | 不限 |
| **附录** | 允许 | 允许 |
| **格式** | ACL LaTeX | AAAI LaTeX |
| **匿名期** | 需要 | 需要 |

### 写作流程

1. **大纲阶段** (1周)
   - 确定论文结构
   - 分配各部分字数
   - 规划图表

2. **初稿阶段** (2-3周)
   - 完成所有章节初稿
   - 插入占位符图表
   - 初步参考文献

3. **修改阶段** (2周)
   - 完善实验章节
   - 优化图表
   - 补充参考文献

4. **润色阶段** (1周)
   - 语言润色
   - 格式检查
   - 最终审稿

---

## 📊 进度跟踪

### 论文 A 进度

| 章节 | 目标字数 | 当前字数 | 状态 | 截止日期 |
|------|----------|----------|------|----------|
| Abstract | 200 | 180 | 🟡 初稿 | 2026-06-30 |
| Introduction | 1500 | 1200 | 🟡 初稿 | 2026-06-30 |
| Related Work | 2000 | 800 | 🔴 待完善 | 2026-07-15 |
| System Design | 2500 | 2500 | 🟡 初稿 | 2026-06-30 |
| Implementation | 2000 | 1500 | 🟡 初稿 | 2026-06-30 |
| AI-Enhanced Features | 1500 | 0 | 🔴 待写 | 2026-07-15 |
| Evaluation | 3000 | 0 | 🔴 待数据 | 2026-08-31 |
| Discussion | 1000 | 0 | 🔴 待写 | 2026-09-15 |
| Conclusion | 500 | 0 | 🔴 待写 | 2026-09-15 |

**总计**: 14000 字 / 6500 字完成 (46%)

### 论文 B 进度

| 章节 | 目标字数 | 当前字数 | 状态 | 截止日期 |
|------|----------|----------|------|----------|
| Abstract | 200 | 0 | 🔴 待写 | - |
| Introduction | 1500 | 0 | 🔴 待写 | - |
| Related Work | 1000 | 0 | 🔴 待写 | - |
| Background | 500 | 0 | 🔴 待写 | - |
| Tool Matrix | 1500 | 0 | 🔴 待写 | - |
| Method | 2500 | 0 | 🔴 待写 | - |
| Implementation | 1000 | 0 | 🔴 待写 | - |
| Experiments | 2500 | 0 | 🔴 待写 | - |
| Discussion | 600 | 0 | 🔴 待写 | - |
| Conclusion | 300 | 0 | 🔴 待写 | - |

**总计**: 11600 字 / 0 字完成 (0%)

---

## 📚 参考文献管理

### 推荐工具
- **Zotero**: 免费，功能强大
- **Mendeley**: 跨平台同步
- **JabRef**: 开源，BibTeX 专用

### 引用格式
- ACL: `\cite{author2024title}`
- AAAI: 同上
- 确保所有引用都有对应的 BibTeX 条目

---

## 🎨 图表制作

### 推荐工具

| 图表类型 | 推荐工具 | 输出格式 |
|----------|----------|----------|
| 架构图 | TikZ / Draw.io | PDF / PNG |
| 流程图 | TikZ / Lucidchart | PDF / PNG |
| 统计图 | Python (matplotlib) | PDF / PNG |
| 表格 | LaTeX / Excel | - |

### 图表规范
- 分辨率: 300 DPI 以上
- 字体: Times New Roman 或 Computer Modern
- 颜色: 可打印 (避免依赖颜色区分)
- 大小: 单栏 3.4英寸，双栏 7英寸

---

## ✅ 提交前检查清单

### 内容检查
- [ ] 所有章节完整
- [ ] 实验数据完整
- [ ] 参考文献完整 (40-60篇)
- [ ] 图表清晰可读
- [ ] 无占位符文本

### 格式检查
- [ ] 符合会议格式要求
- [ ] 页数限制内
- [ ] 引用格式正确
- [ ] 图表编号连续
- [ ] 页边距正确

### 语言检查
- [ ] 无语法错误
- [ ] 无拼写错误
- [ ] 术语一致
- [ ] 缩写已定义
- [ ] 时态正确

### 匿名检查 (如需要)
- [ ] 移除作者信息
- [ ] 移除机构信息
- [ ] 匿名化自引用
- [ ] 检查致谢部分

---

## 📞 联系方式

- **项目负责人**: Tokitai Development Team
- **论文协调**: AI Assistant
- **代码仓库**: https://github.com/tokitai/tokitai

---

**最后更新**: 2026-03-27
