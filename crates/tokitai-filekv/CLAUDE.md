# tokitai-filekv 项目开发指南

## 项目定位
- **是什么**：实验性生产级 LSM-Tree KV 存储引擎，纯 Rust 实现
- **生态位**：`try-tokitai` 仓库中 `crates/tokitai-filekv` 子 crate，为 `tokitai-context` 提供持久化存储（可选 feature）
- **当前版本**：0.5.0，API 不稳定（0.x）
- **性能定位**：小规模嵌入式 KV 场景（≤100K keys），大规模性能仍落后 RocksDB

## 快速命令

```bash
# 开发常用（通过 just）
just check          # cargo check --all-features
just lint           # cargo clippy --all-features --all-targets -- -D warnings
just fmt            # cargo fmt --all
just test           # cargo nextest run --all-features
just bench          # cargo bench --features benchmarks
just clean-data     # 清理 segments/ index/ wal/ checkpoints/

# cargo 别名（.cargo/config.toml）
cargo x             # clippy --all-features --all-targets -- -D warnings
cargo ba            # build --all-features
cargo ca            # check --all-features
cargo precommit     # fmt + clippy + check
```

## 架构速览

```
FileKV (薄门面)
├── ReadEngine     # get() / range() / Bloom 缓存 / Zone Map 剪枝
├── WriteEngine    # put() / delete() / WAL / MemTable flush
├── CompactionEngine # Leveled compaction (L0/L1/L2)
└── LifecycleManager # open() / recover() / checkpoint / 指标
```

## 关键模块路径

| 模块 | 路径 |
|---|---|
| 主入口 | `src/lib.rs` |
| 核心类型 | `src/core/types.rs` `src/core/config.rs` |
| MemTable | `src/core/memtable.rs` |
| Segment | `src/core/segment.rs` |
| WAL | `src/core/wal.rs` |
| 缓存 | `src/cache/block_cache.rs` `src/cache/budget.rs` |
| Bloom | `src/bloom/adaptive.rs` `src/bloom/fpr_controller.rs` |
| 压缩 | `src/compaction/mod.rs` `src/compaction/manifest.rs` |
| 查询 | `src/query/scan.rs` `src/query/pruner.rs` |
| 引擎 | `src/engine/read_engine.rs` `src/engine/write_engine.rs` |
| I/O 抽象 | `src/io/mod.rs` (StdFs / MemFs / FaultInjector) |

## Feature Flags

- `wal` (默认) - Write-Ahead Log
- `mimalloc` - mimalloc 分配器
- `benchmarks` - 性能基准测试
- `rocksdb-compare` - RocksDB 对比基准
- `metrics` - Prometheus 指标导出
- `async-io` - 异步 I/O
- `full` - 全部启用

## 测试规则

- **运行测试**：`just test`（使用 cargo-nextest，并行执行）
- **单模块测试**：`just test-one module_name`
- **Doc tests**：`just test-docs`
- **所有测试**：`--all-features` 是必须的，确保所有 feature 路径覆盖
- **当前测试状态**：593 lib + 28 integration + 15 doctests (100% 通过，0 ignored)

## 代码风格

- 行宽：120 字符
- 缩进：4 空格
- Import 风格：`StdExternalCrate` 分组 + `Crate` 粒度（需 nightly，当前 stable 用 `reorder_imports`）
- 错误处理：使用 `anyhow::Result` + 自定义 `FileKVError` 四层体系
- 禁止在生产路径使用 `unwrap()`（CI 审计）

## 开发注意事项

1. **修改后必跑**：`just precommit`（fmt + clippy + check），要求 0 警告
2. **API 变更**：确保 doctest 更新，`just test-docs`
3. **性能相关改动**：跑 benchmark `just bench` 对比
4. **不要创建 .vscode/** 目录，使用 Neovim
5. **大量文档在 `doc/filekv/` 目录**，修改前查阅相关文档
6. **bloom crate** 位于 `crates/bloom/`，通过 patch 覆盖 crates.io 版本

## 版本历史参考

- v0.5.0: 当前版本，Round 1-9 全部完成 (Phase 1-4)，593 测试通过
- v0.8.0: WAL 二进制 + CDict + GlobalKeyIndex + ZoneMap Arc + CLOCK 算法
- v0.9.0 (已合并到 v0.5.0): GlobalKeyIndex 覆盖率 + CustomBloom + STCS + io_uring

## 焚诀工作流 (FenJue)

项目使用 **焚诀** 双端迭代工作流进行维护：
- `/fenjue` — 启动完整工作流（开发端 + 审查端，5-7 轮迭代）
- `/fenjue continue` — 继续上一轮的 todo.json
- `/fenjue dev` — 仅开发端（执行 todo.json 中的任务）
- `/fenjue review` — 仅审查端（对比代码与文档，更新 todo.json）
- `/fenjue status` — 查看 todo.json 当前状态

**核心规则**：
1. 以 `todo.json` 为唯一真实计划，所有开发围绕它进行
2. 不砍文档 — 代码追平文档
3. context 超过 10% 时 `/compress`
4. 使用子 Agent 分工执行 todo.json 任务
