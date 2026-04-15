# DOC-001: 性能文档重写规划

**创建日期**: 2026-04-15
**版本**: v0.5.0
**优先级**: P1（v0.6.0 规划项）
**状态**: 规划中

---

## 1. 现有问题分析

### 1.1 规模分级问题

以下位置使用了错误的规模描述（将 100K keys 称为"大规模"）：

| 文件 | 位置 | 错误描述 | 正确分类 |
|------|------|---------|---------|
| `CHANGELOG.md` | v0.5.0 标题 | "大规模数据集性能优化" | 极小规模（100K keys） |
| `CHANGELOG.md` | v0.5.0 完成总结 | "聚焦大规模数据集性能优化" | 极小规模 |
| `CHANGELOG.md` | [Unreleased] v0.5.0 Planning | "大规模数据集性能优化" | 极小规模 |
| `CHANGELOG.md` | Phase 7 标题 | "v0.5.0 Large-Scale Performance" | 极小规模 |
| `CHANGELOG.md` | TEST-002 描述 | "大规模数据集基准测试" | 极小规模基准测试 |
| `doc/filekv/POSITION_AND_STATUS.md` | 演进历程表格 | v0.5.0 "大规模数据集优化" | 极小规模数据集优化 |
| `doc/filekv/POSITION_AND_STATUS.md` | Phase 7 标题 | "v0.5.0 Large-Scale Performance" | 极小规模 |
| `doc/filekv/POSITION_AND_STATUS.md` | TEST-002 描述 | "大规模数据集基准测试" | 极小规模基准测试 |
| `docs/v050_PERFORMANCE_VALIDATION.md` | 执行摘要 | "聚焦大规模数据集性能优化" | 极小规模 |
| `README.md` | 最新特性列表 | "大规模数据集基准测试" | 极小规模 |

**说明**：README.md 已在警告框中正确声明 100K = 极小规模，但 CHANGELOG、POSITION_AND_STATUS、PERFORMANCE_VALIDATION 中仍有大量错误描述。

### 1.2 性能声明问题

以下性能声明缺少对比基准或测试环境说明：

| 文件 | 声明 | 缺少内容 |
|------|------|---------|
| `CHANGELOG.md` | "内存减少 50%+" (POL-005) | 缺少基准值、测试规模、硬件环境 |
| `CHANGELOG.md` | "查找性能提升 2-3x" (POL-005) | 缺少对比基准（vs 什么？） |
| `CHANGELOG.md` | "热缓存读取从 61.92µs 降至 0.229µs (270x)" | 缺少测试环境说明 |
| `docs/v050_PERFORMANCE_VALIDATION.md` | 全文性能数据 | **缺少测试环境说明**（CPU/内存/磁盘/OS/Rust版本） |
| `docs/v050_PERFORMANCE_VALIDATION.md` | "减少 15-25% 的读取延迟" | 缺少测量条件和测试负载 |
| `docs/v050_PERFORMANCE_VALIDATION.md` | "减少 40-50% 的 Bloom Filter 重建开销" | 缺少基准值 |
| `README.md` | "Bloom 负向查询 62.37µs vs RocksDB 247.38µs" | 有测试日期，但无详细环境说明 |
| `README.md` | "全 KV Get 0.229µs vs RocksDB 600.07µs" | 无测试规模说明（多少 keys？） |

### 1.3 放大率缺失

以下位置需要补充放大率测量数据：

| 文件 | 位置 | 缺失内容 |
|------|------|---------|
| `README.md` | 核心特性 | 提到 "Write Amplification Tracking: WAF/RAF/SAF 监控"，但无实际数据 |
| `docs/v050_PERFORMANCE_VALIDATION.md` | 全文 | 完全缺少写/读/空间放大率测量 |
| `doc/filekv/POSITION_AND_STATUS.md` | v0.6.0 规划 | 提到放大率定义，但无测量方法和当前基线 |
| `CHANGELOG.md` | v0.6.0 规划 | 提到放大率测量目标，但无当前值 |

---

## 2. 修正标准

### 2.1 正确的规模分级

所有文档必须统一使用以下分级：

| 级别 | Key 数量 | 数据量 | 英文标签 | 用途 |
|------|---------|--------|---------|------|
| 极小规模 | ≤100K | ≤100MB | `tiny` | 功能验证、CI 测试 |
| 小规模 | 100K ~ 1M | 100MB ~ 1GB | `small` | 开发测试 |
| 中等规模 | 1M ~ 10M | 1GB ~ 10GB | `medium` | 生产验证 |
| 大规模 | 10M ~ 100M | 10GB ~ 100GB | `large` | 生产 benchmark |
| 超大规模 | ≥100M | ≥100GB | `xlarge` | 工业级 benchmark |

### 2.2 放大率定义

统一使用以下定义（对齐工业界标准）：

- **写放大率 (WA, Write Amplification)** = 实际磁盘写入字节数 / 用户逻辑写入字节数
- **读放大率 (RA, Read Amplification)** = 实际磁盘读取字节数 / 用户逻辑读取字节数
- **空间放大率 (SA, Space Amplification)** = 磁盘占用 / 用户逻辑数据

### 2.3 性能声明规范化模板

所有性能报告必须包含以下环境说明：

```markdown
## 测试环境
- CPU: AMD Ryzen 9 8945HS
- 内存: 64GB DDR5
- 磁盘: NVMe SSD
- OS: Linux
- Rust: 1.x
- 测试日期: 2026-04-15

## 性能数据
（注明对比基准和测试规模）
```

每个性能声明必须满足：
1. 明确对比基准（vs 哪个版本？vs 哪个产品？）
2. 明确测试规模（多少 keys？多少线程？）
3. 如果是百分比改善，必须提供绝对值

---

## 3. 修正清单

按优先级排序：

### P0 - 立即修正（影响对外声明准确性）

| # | 文件 | 修改内容 | 修正建议 |
|---|------|---------|---------|
| 1 | `CHANGELOG.md` | v0.5.0 所有"大规模"描述 | 改为"极小规模"，移除"为保持版本连续性"的保留说明 |
| 2 | `CHANGELOG.md` | Phase 7 标题 | 改为 "Phase 7: v0.5.0 Tiny-Scale Performance" |
| 3 | `CHANGELOG.md` | POL-005 性能声明 | 补充基准值（vs HashMap 的内存和查找对比） |
| 4 | `doc/filekv/POSITION_AND_STATUS.md` | 演进历程表格 v0.5.0 | 改为"极小规模数据集优化" |
| 5 | `doc/filekv/POSITION_AND_STATUS.md` | Phase 7 标题 | 改为 "v0.5.0 Tiny-Scale Performance" |
| 6 | `docs/v050_PERFORMANCE_VALIDATION.md` | 执行摘要 | 改为"聚焦极小规模数据集"，补充测试环境说明 |
| 7 | `docs/v050_PERFORMANCE_VALIDATION.md` | 各优化项 | 补充百分比声明的基准值 |

### P1 - 重要修正（提升文档专业性）

| # | 文件 | 修改内容 | 修正建议 |
|---|------|---------|---------|
| 8 | `README.md` | 核心特性中的 WAF/RAF/SAF | 添加实际测量数据或标注"待 v0.6.0 测量" |
| 9 | `README.md` | 性能表格 | 补充各数据点的测试规模说明 |
| 10 | `README.md` | 最新特性列表 | "大规模数据集基准测试"改为"极小规模" |
| 11 | `docs/v050_PERFORMANCE_VALIDATION.md` | 文档开头 | 添加完整测试环境说明 |
| 12 | `docs/v050_PERFORMANCE_VALIDATION.md` | 已知限制部分 | 补充放大率数据缺失的说明 |
| 13 | `doc/filekv/POSITION_AND_STATUS.md` | v0.6.0 规划 | 补充放大率测量方法和当前基线（如已有） |

### P2 - 格式优化（提升一致性）

| # | 文件 | 修改内容 | 修正建议 |
|---|------|---------|---------|
| 14 | 所有文档 | 规模描述用语 | 统一使用"极小规模/小规模/中等规模/大规模/超大规模" |
| 15 | 所有文档 | 英文标签 | 统一使用 tiny/small/medium/large/xlarge |
| 16 | `CHANGELOG.md` | v0.5.0 性能提升描述 | 统一格式："X 从 A 优化到 B（提升 Y%，对比基准 Z）" |

---

## 4. 实施计划

### 阶段 1: 修正 README.md（P0）

**预计时间**: 1h
**修改内容**:
- 修正"大规模数据集基准测试"为"极小规模"
- 性能表格补充测试规模说明
- WAF/RAF/SAF 特性添加"待测量"标注

### 阶段 2: 修正 POSITION_AND_STATUS.md（P0）

**预计时间**: 1h
**修改内容**:
- 修正演进历程表格中 v0.5.0 的描述
- 修正 Phase 7 标题
- 修正所有"大规模"为"极小规模"

### 阶段 3: 重写 PERFORMANCE_VALIDATION.md（P1）

**预计时间**: 2h
**修改内容**:
- 在文档开头添加完整测试环境说明
- 修正"大规模"为"极小规模"
- 为所有百分比声明补充基准值
- 添加放大率数据缺失说明

### 阶段 4: 修正 CHANGELOG.md（P0）

**预计时间**: 1h
**修改内容**:
- 修正 v0.5.0 所有"大规模"描述
- 修正 Phase 7 标题
- 为 POL-005 性能声明补充基准值
- 移除"为保持版本连续性"的错误保留说明

### 阶段 5: 补充放大率测量方法文档（P1）

**预计时间**: 2h
**修改内容**:
- 创建 `docs/v060_amplification_measurement.md` 或在现有文档中补充
- 定义 WA/RA/SA 的测量方法和计算公式
- 记录当前基线（如已有）或标注"待 v0.6.0 测量"

### 阶段 6: 归档冗余文档（P2）

**预计时间**: 1h
**修改内容**:
- 识别与 v050_PERFORMANCE_VALIDATION.md 重复的内容
- 将过时或重复的性能报告归档到 `doc/filekv/archive/`
- 更新相关文档链接

---

## 5. 约束

1. **避免跨文档重复**：同一性能数据只在一个主文档中详细记录，其他文档引用
2. **CHANGELOG 格式**：严格遵循 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) 格式
3. **归档规则**：冗余或过时文档归档到 `doc/filekv/archive/`，保留原始文件名
4. **向后兼容**：修正后不影响已发布的 crates.io 版本语义
5. **测试验证**：所有修正后的文档必须通过链接检查（无断裂链接）

---

## 6. 成功标准

- [ ] 所有文档中 0 处"100K keys = 大规模"的错误描述
- [ ] 所有性能报告包含完整测试环境说明
- [ ] 所有百分比声明附带绝对值和基准
- [ ] 放大率测量方法文档已创建
- [ ] 冗余文档已归档
- [ ] CHANGELOG 格式符合 Keep a Changelog 标准

---

*本文档为 v0.6.0 DOC-001 任务的详细规划，实施前需评审确认。*
