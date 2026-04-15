# FileKV v5.0 最终定位

**更新日期**: 2026-04-16  
**版本**: v5.0  
**状态**: 架构确认完成

---

## 🎯 FileKV 的正确定位

### 一句话定义

> **FileKV 是 tokitai-context 的高性能可选存储引擎**（类似 PostgreSQL 的 InnoDB，可选替代 RocksDB）

### 架构图

```
┌─────────────────────────────────────────────┐
│           try-tokitai (AI 助手)              │
├─────────────────────────────────────────────┤
│                                             │
│  ┌──────────────────────────────────┐       │
│  │   tokitai-context                │       │
│  │   - 分支管理 (Git 风格)           │       │
│  │   - 时间旅行                      │       │
│  │   - 上下文合并                    │       │
│  │   - 多 Agent 协作                 │       │
│  │   - 会话管理                      │       │
│  └──────────────┬───────────────────┘       │
│                 │ 使用存储引擎               │
│                 ▼                           │
│  ┌──────────────────────────────────┐       │
│  │   存储引擎层（可选）              │       │
│  │                                  │       │
│  │  选项 A: tokitai-filekv          │       │
│  │  - Rust 原生，零 FFI             │       │
│  │  - LSM-Tree 存储                 │       │
│  │  - 配置简单（4 个预设）          │       │
│  │  - 针对 AI 工作负载优化          │       │
│  │                                  │       │
│  │  选项 B: RocksDB                 │       │
│  │  - 工业级验证                   │       │
│  │  - C++ FFI                       │       │
│  │  - 100+ 参数调优                 │       │
│  └──────────────────────────────────┘       │
│                                             │
└─────────────────────────────────────────────┘
```

---

## ✅ FileKV 的职责（存储引擎）

| 职责 | 描述 |
|------|------|
| **低延迟 KV 操作** | put/get < 50µs |
| **高效 LSM-Tree** | 自动分层、压缩 |
| **缓存策略** | Bloom、Block 缓存优化 |
| **WAL + 崩溃恢复** | 数据持久性保证 |
| **范围查询** | BTreeMap 二级索引 |
| **可观测性** | WA/RA/SA 监控 |

---

## ❌ FileKV 不做的（tokitai-context 负责）

| 功能 | 负责方 | 原因 |
|------|--------|------|
| 分支管理 | tokitai-context | AI 语义，不在存储引擎层 |
| 时间旅行 | tokitai-context | 需要理解上下文语义 |
| 会话隔离 | tokitai-context | 多租户管理 |
| 多 Agent 协作 | tokitai-context | 工作流编排 |
| 上下文合并 | tokitai-context | 需要 AI 语义理解 |

---

## 📊 v0.8.0 已完成优化

| 优化 | 效果 |
|------|------|
| WAL 二进制序列化 | 3-5x 加速 |
| CDict/DDict 预创建 | 10-100x 压缩加速 |
| GlobalKeyIndex 启用 | AHashMap + moka |
| Bloom L2 Arc | O(1) 访问 |
| CLOCK 算法 | 7.4x 并发提升 |
| ZoneMap Arc | 消除 Vec clone |
| Instant 时间戳 | 无系统调用 |
| AHash 分片 | 3-5x 加速 |
| Compaction 锁优化 | AtomicUsize |
| 定时 fsync | 10ms 间隔 |

---

## 🚀 v0.9.0 优化规划

### Phase 1: 存储引擎核心优化（Week 1-2）

| 任务 | 优先级 | 目标 |
|------|--------|------|
| OPT-007: 批量 WAL + 异步 flush | P0 | 写入 < 50µs |
| OPT-003: Compaction 不阻塞 | P0 | P99 < 100µs |
| OPT-002: CustomBloom 集成 | P1 | 加载 < 100µs |
| OPT-005: BlockCache 热点感知 | P1 | 命中率 > 60% |
| OPT-004: DashMap 批量写入 | P1 | 吞吐 > 200K/s |

### Phase 2: 进阶优化（Week 3-4）

| 任务 | 优先级 | 目标 |
|------|--------|------|
| OPT-001: GlobalKeyIndex 范围查询 | P0 | 范围查询 < 1ms |
| OPT-008: WA/RA/SA 监控 | P1 | 实时指标 |

---

## 💡 为什么选择 FileKV（vs RocksDB）

| 维度 | FileKV | RocksDB |
|------|--------|---------|
| **集成** | `Cargo.toml` 一行 | 编译 C++、FFI |
| **配置** | 4 个预设 | 100+ 参数 |
| **语言** | Rust 原生 | C++ |
| **部署** | 单二进制 | 需要链接 |
| **类型安全** | 编译期保证 | 运行期检查 |
| **AI 优化** | 针对 AI 工作负载 | 通用设计 |

---

## 📁 文档结构

```
docs/
├── V5_FINAL_POSITIONING.md          # 本文档
├── BLOOM_FORMAT.md                  # Bloom 格式说明
├── TEST_STRATEGY.md                 # 测试策略
├── DOCUMENT_CONSOLIDATION_REPORT.md # 文档整理报告
├── plans/                           # 设计计划
├── releases/                        # 发布总结
├── benchmarks/                      # Benchmark 结果
├── architecture/                    # 架构文档
├── guides/                          # 使用指南
└── archive/                         # 历史文档归档
    ├── 2026-04-14/                  # v3/v4 定位文档（已弃用）
    └── v050-v070/                   # v0.5-v0.7 历史文档
```

---

## 🎓 经验教训

### 走过的弯路

1. **v3.0**: 追求"超越 RocksDB"的通用性能 → 目标不切实际
2. **v4.0**: 定位为"AI Agent 持久化层" → 重复 tokitai-context 工作

### 正确定位

**v5.0**: tokitai-context 的可选存储引擎 → **专注存储引擎优化**

### 关键洞察

> 不要重复造轮子。  
> tokitai-context 已经做了 AI 语义，  
> FileKV 只需要做好存储引擎。

---

**文档结束** - 详见 `todo.json` v5.0 获取完整优化规划
