# FileKV 定位说明文档

**最后更新**: 2026-04-13 (三次复审 - 代码验证)
**状态**: 学术研究原型 (Academic Research Prototype)

---

## 📋 目录

1. [项目定位](#项目定位)
2. [设计目标](#设计目标)
3. [使用场景](#使用场景)
4. [非设计目标](#非设计目标)
5. [当前已知限制](#当前已知限制)
6. [生产就绪路线图](#生产就绪路线图)

---

## 项目定位

**FileKV 是一个正在向实验性生产引擎转型的 LSM-Tree KV 存储引擎**。核心架构清晰（六阶段重构、四引擎拆分），代码质量达到生产级标准（四层错误体系、完整指标体系、崩溃安全机制），但仍有已知限制需解决，正在向生产就绪方向持续演进。

### 核心定位

| 维度 | 定位 |
|------|------|
| **目标** | 实验性生产引擎 (Experimental Production-Ready) - 转型中 |
| **用户** | 开发者、DBA、系统架构师、研究人员 |
| **场景** | 开发/测试环境、小规模部署、算法验证、评估验证 |
| **可靠性** | 代码质量生产级，核心 API 已稳定，但需在实际环境验证，已知限制明确 |

### 项目演进历程

| 阶段 | 版本 | 定位 | 状态 |
|------|------|------|------|
| v0.0.x | 初始原型 | 功能验证 | ✅ 已完成 |
| v0.1.0-v0.1.6 | 六阶段重构 | 架构完善 | ✅ 已完成 |
| v0.1.7 | 代码质量清理 | Critical/Major 问题修复 | ✅ 已完成 |
| **v0.2.0** | **实验性生产引擎** | **剩余问题修复、文档对齐、API 稳定** | **🎯 目标版本** |
| v0.3.0 | 生产就绪候选 | 大规模并发测试、性能优化 | 📋 规划中 |
| v1.0.0 | 稳定版 | 生产就绪 | 📋 远期规划 |

---

## 设计目标

### ✅ 已实现的目标

1. **性能验证**
   - 验证 LSM-Tree 架构的写优化特性
   - 展示 Bloom Filter 对读取性能的优化
   - 证明 BlockCache 对热点数据的有效性

2. **功能演示**
   - 完整的 LSM-Tree 实现 (MemTable + Segment + Compaction)
   - 崩溃恢复机制 (WAL)
   - 多层缓存架构 (BlockCache + BloomFilterCache)

3. **学术创新**
   - Compressed Bloom Filter (RLE + Huffman)
   - Adaptive Bloom Filter Cache
   - Range Scan 优化 (Zone Map, Pruner)

### 📊 性能数据 (学术用途)

```
单条写入 (无 WAL):  92.5ns
单条写入 (有 WAL):  1.68µs
批量写入 (1000 项):  0.26µs/项
热读取 (缓存命中):   111ns
冷读取 (Bloom 阴性): 1.39µs
Bloom Filter QPS:    1.15B/s (纯内存 contains() 操作)
```

**注意**: 以上数据来自受控实验环境，实际生产环境可能有显著差异。

---

## 使用场景

### ✅ 推荐使用场景

1. **开发与测试环境**
   - 应用程序开发与集成测试
   - 性能基准测试与对比评估
   - CI/CD 管道自动化测试

2. **小规模生产部署**
   - 内部工具与监控系统
   - 非关键业务数据存储
   - 配置管理与元数据

3. **技术评估与选型**
   - 存储引擎特性验证
   - 性能特征验证
   - API 适配测试

4. **学习与研究**
   - LSM-Tree 算法研究
   - Rust 系统编程实践
   - 存储架构探索

### ⚠️ 需评估后使用

1. **中等负载生产环境**
   - 需充分性能测试
   - 需评估已知限制
   - 建议有备用方案

2. **业务关键数据**
   - 需评估 ACID 保证
   - 需制定恢复策略
   - 建议定期备份

### ❌ 暂不建议使用

1. **大规模超负载**
   - TB 级数据存储（性能待优化）
   - 极高并发场景（32+ 线程待验证）

2. **金融/医疗等关键系统**
   - 需要完整 ACID 保证
   - 需要企业级可靠性特性

---

## 非设计目标

FileKV **暂不追求**以下企业级特性（但路线图包含）：

### 1. 完整的 ACID 保证

- ✅ 原子性：批量写入通过 `WalManager::log_batch` 实现 (WAL 级别原子性)
- ⚠️ 一致性：基础事务隔离（无 MVCC，计划中）
- ✅ 持久性：WAL 支持 `fsync` 模式，可配置
- ⚠️ 隔离性：快照隔离（规划 v0.3.0）

### 2. 企业级可靠性

- ❌ 自动故障转移
- ❌ 数据复制（分布式支持）
- ✅ 崩溃恢复 - Compaction Manifest 有 crash recovery
- ⚠️ 时间点恢复 (PITR) - Incremental Checkpoint 已实现

### 3. 运维工具

- ✅ 监控指标导出 - Prometheus Metrics 完整实现 (`metrics` feature)
- ✅ 基础性能诊断 - 统计指标完整
- ❌ 自动化运维脚本
- ❌ 配置热更新

### 4. 数据完整性

- ✅ 校验和验证 (Bloom Filter 文件有 CRC32 验证)
- ✅ Compaction Manifest 崩溃安全
- ✅ WAL 恢复机制完整

---

## 当前已知限制

### 代码质量现状

| 维度 | FileKV (当前) | 说明 |
|------|--------------|------|
| **架构设计** | ✅ 生产级 | 四引擎拆分（Read/Write/Compaction/Lifecycle） |
| **错误体系** | ✅ 生产级 | 四层错误体系（Fatal/Transient/Expected/Domain） |
| **指标体系** | ✅ 完整 | Prometheus 指标覆盖所有关键路径 |
| **测试覆盖** | ⚠️ 良好 | 核心功能有测试，大规模并发测试待补充 |
| **代码审计** | ✅ 完成 | Critical/Major 问题全部解决，Minor 审计完成 |
| **错误处理** | 部分 unwrap() | 零 unwrap() | 生产代码不应 panic |
| **测试覆盖** | ~85% | ~95%+ | 边界条件测试不足 |
| **文档完整** | 良好 | 优秀 | API 文档详细，运维文档缺失 |
| **代码审查** | 有限 | 严格 | 缺少多轮审查 |

### 功能差距

| 功能 | FileKV | RocksDB | 优先级 |
|------|--------|---------|--------|
| **WAL Batch Write** | ✅ 已实现 (`WalManager::log_batch`) | ✅ | P2 |
| **Compaction 异步化** | ✅ 已实现 (`run_compaction_thread_async`) | ✅ | P2 |
| **Snapshot 隔离** | ❌ | ✅ | P1 |
| **列族 (Column Family)** | ❌ 未实现 | ✅ 完整 | P2 |
| **在线 Compaction** | ✅ 已实现 (后台线程异步执行) | ✅ | P1 |
| **自适应 Compaction** | ❌ | ✅ | P2 |
| **Block Cache 限流** | ❌ | ✅ | P2 |
| **Prometheus Metrics** | ✅ 已实现 (`metrics` feature) | ✅ | P1 |
| **Incremental Checkpoint** | ✅ 已实现 (`IncrementalCheckpointManager`) | ✅ (PITR) | P2 |

### 性能差距

| 指标 | FileKV | RocksDB | 说明 |
|------|--------|---------|------|
| **写放大** | ~1.5-2.0 (估算) | 1.1-1.5 | 通过 Compaction 优化可降低 |
| **读放大** | ~1.2-1.5 (估算) | 1.0-1.2 | Bloom Filter + Zone Map 优化 |
| **空间放大** | ~1.2 | ~1.1 | 待优化 |
| **并发写入** | 有限测试 | 充分优化 | WAL 锁瓶颈，batch write 缓解 |

---

## 已知限制

### 1. 数据持久化

- ⚠️ WAL 可能丢失最近写入 (操作系统缓存未 flush)
- ⚠️ Segment 文件无原子写入保证
- ⚠️ 崩溃恢复可能丢失部分数据

### 2. 并发控制

- ⚠️ WAL 锁可能成为瓶颈 (批量写入通过 `log_batch` 减少锁竞争)
- ❌ 无读写隔离
- ⚠️ Compaction 异步化已实现，但极端情况下仍可能影响写入

### 3. 内存管理

- ⚠️ BlockCache 无严格内存限制
- ⚠️ MemTable 超限可能 panic
- ⚠️ mmap 使用未优化

### 4. 错误恢复

- ⚠️ 部分错误静默处理
- ⚠️ 恢复策略有限 (WAL 恢复、Compaction Manifest crash recovery 已实现)
- ⚠️ 无自动修复 (Incremental Checkpoint 提供基础恢复能力)

---

## 未来改进方向

### P0 (生产级必需，论文项目可选)

1. **移除所有 unwrap()**
   - 当前：222 处 unwrap()
   - 目标：0 处 (生产代码)
   - 状态：核心算法已修复

2. **完成 TODO 功能**
   - Segment 遍历 ✅
   - Zone Map 加载 ✅ (Zone Map 集成到 Segment)
   - Range Pruner ✅ (已实现 range scan pruning)

3. **完善错误处理**
   - WAL 恢复错误上报 ✅
   - 统一错误类型 ⏳
   - 错误码系统 ⏳

### P1 (论文加分项)

4. **性能优化**
   - WAL Batch Write ✅ 已实现
   - Compaction 异步化 ✅ 已实现
   - 并发控制优化

5. **可观测性**
   - Metrics 导出 ✅ 已实现 (`metrics` feature, PrometheusExporter)
   - Tracing 集成 ✅ 已集成 (tracing crate)
   - 性能分析工具

### P2 (未来探索)

6. **高级特性**
   - 列族完整实现
   - 事务支持
   - 分布式扩展

---

## 总结

### FileKV 是什么

✅ **实验性生产引擎（转型中）** - 代码质量达到生产级标准
✅ **核心 API 已稳定** - FileKV/FileKVConfig 签名和语义冻结
✅ **开发/测试环境可用** - 适合小规模部署和评估验证
✅ **完整错误体系** - Fatal/Transient/Expected/Domain 四层分类
✅ **完整指标体系** - Prometheus 覆盖关键路径
✅ **技术学习参考** - LSM-Tree 架构设计示例

### FileKV 暂不是什么

❌ **成熟商业数据库** - 缺乏多年生产验证
❌ **RocksDB 直接替代品** - 100K keys 场景慢 240x，需评估使用
❌ **分布式存储** - 无复制和分布式支持

### 使用建议

**如果你是**：
- 👨‍💻 **开发者**：寻找开发/测试用 KV 存储
- 🔍 **架构师**：评估 LSM-Tree 特性是否适合你的场景
- 📊 **运维**：需要简单可监控的存储方案
- 🎓 **学习者**：研究 Rust 系统编程和存储引擎设计
- 🧪 **研究人员**：验证存储引擎创新算法

**FileKV 适合你！**

**如果你需要**：
- 💰 **成熟生产数据库**：RocksDB/LevelDB 有更多验证
- 🔒 **企业级高可用**：考虑商业分布式数据库
- 📈 **超大规模部署**：等待 v0.3.0+ 版本验证

**FileKV 暂不适合你！**

---

## 参考资源

- **源码**: https://github.com/silverenternal/tokitai-context
- **文档**: [filekv/FILEKV_GUIDE.md](filekv/FILEKV_GUIDE.md)
- **架构**: [../ARCHITECTURE.md](../ARCHITECTURE.md)
- **性能**: [PERFORMANCE_BENCHMARK_REPORT.md](PERFORMANCE_BENCHMARK_REPORT.md)
- **专利**: [../patent_disclosure_zone_map.md](../patent_disclosure_zone_map.md)

---

## 联系方式

**作者**: [Silverenternal]  
**邮箱**: [请联系作者]  
**问题反馈**: GitHub Issues

---

*本文档最后更新：2026-04-13*
