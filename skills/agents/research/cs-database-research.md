---
name: cs-database-research
description: Database research skill for query workload design, storage-engine evaluation, indexing tradeoff analysis, and reproducible database benchmarking. Use when the task involves OLTP, OLAP, indexing, query optimization, storage layouts, transaction processing, or database systems papers.
---

# CS Database Research

## 触发条件
- 研究对象是数据库内核、查询优化、索引、事务、存储引擎、向量检索或 HTAP 系统。
- 需要定义 workload、数据分布、并发事务、查询模板或 cost model。
- 需要比较数据库方案的吞吐、延迟、放大、空间成本与恢复表现。

## 标准化流程
1. 明确数据库问题边界，区分执行层、优化器、索引、存储和事务层。
2. 固定 workload 类型、数据规模、查询模板、更新比例、冷热分布和事务混合比例。
3. 对比时同时报告吞吐、平均延迟、尾延迟、写放大、空间放大和恢复成本。
4. 将收益分解到关键组件，例如索引命中、计划选择、缓存行为或 IO 路径。
5. 覆盖不同基数、倾斜分布、并发级别和资源约束，而不是只测单点最优设置。
6. 写作时把数据库结论绑定到具体 workload 场景，避免把局部收益写成通用结论。

## 反模式
- 只在单一查询模板上验证，却宣称通用数据库优化。
- 只报平均 latency，不报尾延迟、放大和恢复代价。
- 使用非代表性、过度缓存或过度预热的 workload 做不公平比较。
- 忽略数据倾斜、并发冲突和事务隔离级别的影响。

## 验证方法
- 检查 workload 是否写清了数据规模、查询模板、更新比例和并发配置。
- 复核结果是否至少包含吞吐、延迟和一种放大或成本指标。
- 抽查核心结论是否能追溯到具体 workload 与图表。
- 审核是否覆盖了倾斜分布、并发或恢复场景中的至少一种压力测试。
