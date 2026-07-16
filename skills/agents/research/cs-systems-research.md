---
name: cs-systems-research
description: Computer systems research skill for workload definition, throughput-latency tradeoff analysis, infrastructure benchmarking, and reproducible systems evaluation. Use when the task involves databases, distributed systems, operating systems, compilers, storage, or serving performance studies.
---

# CS Systems Research

## 触发条件
- 研究对象是数据库、分布式系统、操作系统、编译器、服务系统或基础设施优化。
- 需要定义 workload、压测协议、资源隔离和性能对比。
- 需要分析 tail latency、吞吐、可扩展性、成本或故障恢复。

## 标准化流程
1. 固定系统边界、硬件环境、软件版本、部署拓扑与背景负载。
2. 明确 workload：请求类型、到达分布、数据规模、并发模式和热冷数据比例。
3. 选取系统领域可接受的对照基线，并写清优化点作用路径。
4. 同时测量吞吐、平均延迟、尾延迟、资源占用与成本。
5. 覆盖稳态、升压、故障注入和恢复阶段，避免只看理想稳态。
6. 汇报时将优化收益分解到子模块、关键路径和资源瓶颈。

## 反模式
- 把缓存预热后的成绩与冷启动成绩混写。
- 只在单节点上验证却声称系统级可扩展。
- 忽略尾延迟、抖动和多租户干扰。
- 不公布硬件、内核、容器或编译参数。

## 验证方法
- 核查实验日志是否记录了环境版本、实例规格和 workload 参数。
- 检查是否有 tail latency 与资源成本的联合分析。
- 复核扩展性 claim 是否至少覆盖两档以上负载或节点规模。
- 验证故障恢复结论是否由故障注入或真实恢复日志支持。
