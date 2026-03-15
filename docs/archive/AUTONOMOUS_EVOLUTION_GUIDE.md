# 自主进化功能使用指南

**版本**: 1.0.0  
**最后更新**: 2026-03-14

---

## 📋 概述

自主进化功能允许 AI 在后台自动发现项目改进点、执行改进任务、本地审查通过后自动推送到 GitHub。

### 核心特性

- ✅ **自主发现改进点** - AI 分析项目现状，识别需要改进的地方
- ✅ **自主规划任务** - 生成具体的改进计划和步骤
- ✅ **自主执行任务** - 调用工具执行代码修改
- ✅ **本地审查** - 自动运行 `cargo fmt`、`cargo clippy`、`cargo test`
- ✅ **自动推送** - 审查通过后自动提交并推送到 GitHub
- ✅ **失败回滚** - 审查未通过时自动回滚变更

---

## 🚀 启动方式

### 1. 准备工作

确保已配置环境变量：

```bash
# 设置 API Key（必填）
export AI_API_KEY="your-api-key"

# 设置 API URL（可选，默认使用 Ollama Cloud）
export AI_API_URL="https://ollama.com/v1/chat/completions"

# 设置模型（可选）
export AI_MODEL="qwen3.5:397b"
```

### 2. 启动自主进化模式

```bash
cargo run --release -- --autonomous
```

或使用简写：

```bash
cargo run --release -- -a
```

---

## 🔄 工作流程

自主进化系统按以下流程运行：

```
┌─────────────────────────────────────────────────────────┐
│  1. 分析项目现状                                         │
│     - 检查 Git 状态                                      │
│     - 扫描项目文件结构                                   │
│     - 查找 TODO/FIXME 注释                              │
└─────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────┐
│  2. 生成改进计划                                         │
│     - AI 分析项目现状                                    │
│     - 制定具体改进步骤                                   │
│     - 评估风险和优先级                                   │
└─────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────┐
│  3. 执行改进任务                                         │
│     - 读取相关文件                                       │
│     - 修改代码                                           │
│     - 运行命令验证                                       │
└─────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────┐
│  4. 本地审查                                             │
│     - cargo fmt --check（代码格式）                     │
│     - cargo clippy -- -D warnings（代码质量）           │
│     - cargo test --quiet（测试通过）                    │
└─────────────────────────────────────────────────────────┘
                          ↓
            ┌─────────────┴─────────────┐
            ↓                           ↓
    ┌───────────────┐           ┌───────────────┐
    │ 审查通过 ✅    │           │ 审查失败 ❌    │
    └───────────────┘           └───────────────┘
            ↓                           ↓
┌───────────────────────────┐   ┌───────────────────────┐
│  5. 推送到 GitHub          │   │  回滚变更             │
│     - git add .           │   │     - git checkout -- │
│     - git commit -m "..." │   │                       │
│     - git push            │   │                       │
└───────────────────────────┘   └───────────────────────┘
```

---

## 📊 自主进化目标

系统默认执行以下进化目标（按顺序）：

| 序号 | 目标 | 说明 |
|------|------|------|
| 1 | 改进代码质量 | 检查并修复代码中的潜在问题 |
| 2 | 优化性能 | 分析并优化慢查询和低效代码 |
| 3 | 增强错误处理 | 改进错误提示和日志 |
| 4 | 完善文档 | 检查并更新 README 和注释 |
| 5 | 清理技术债务 | 移除未使用的代码和依赖 |

---

## 🛠️ 配置

### 存储目录

自主进化数据存储在 `.tokitai/autonomy/` 目录下：

```
.tokitai/
└── autonomy/
    ├── planner/      # 规划 Agent 数据
    ├── executor/     # 执行 Agent 数据
    ├── reviewer/     # 审查 Agent 数据
    ├── tracker/      # 迭代追踪数据
    └── git/          # Git 工作流数据
```

### 自定义进化目标

修改 `src/main.rs` 中的 `evolution_goals` 数组：

```rust
let evolution_goals = vec![
    "你的自定义目标 1".to_string(),
    "你的自定义目标 2".to_string(),
    // ...
];
```

---

## 📝 输出示例

```
🤖 AI Assistant powered by Tokitai
=====================================
🔄 自主进化模式
模型：qwen3.5:397b (Ollama Cloud)


🤖 启动自主进化系统...
   - AI 将自主发现项目改进点
   - 本地审查通过后将自动推送到 GitHub
   - 按 Ctrl+C 停止自主模式


📋 自主进化目标：改进代码质量：检查并修复代码中的潜在问题
   🔍 分析项目现状...
   📝 生成改进计划...
   🔧 执行改进任务...
   🧪 本地审查...
      - 运行 cargo fmt...
      - 运行 cargo clippy...
      ⚠️  Clippy 发现警告
      - 运行 cargo test...
      ✅ 审查通过
   🚀 推送到 GitHub...
      - git add .
      - git commit -m 'fix: 修复潜在的类型转换问题'
      - git push
✅ 进化完成并已推送到 GitHub


📋 自主进化目标：优化性能：分析并优化慢查询和低效代码
   ...
```

---

## ⚠️ 注意事项

### 1. Git 配置

确保已配置 Git 用户信息：

```bash
git config --global user.name "Your Name"
git config --global user.email "your.email@example.com"
```

### 2. GitHub 推送权限

确保有推送权限：

```bash
# 使用 SSH 密钥
git remote set-url origin git@github.com:username/repo.git

# 或使用 Personal Access Token
git remote set-url origin https://github.com/username/repo.git
```

### 3. 停止自主进化

按 `Ctrl+C` 随时停止自主进化模式。

### 4. 审查失败处理

审查失败时，系统会自动回滚变更，不会影响现有代码。

---

## 🔧 故障排除

### Q: 推送失败 "Permission denied"

**A**: 检查 Git 远程仓库配置和认证信息：

```bash
git remote -v
git config --global credential.helper store
git push  # 手动推送一次，保存凭证
```

### Q: 审查总是失败

**A**: 检查项目是否有预先存在的警告：

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

### Q: AI 无法理解项目结构

**A**: 确保项目有清晰的目录结构和命名规范。

---

## 📈 最佳实践

1. **定期运行** - 建议每天或每周运行一次自主进化
2. **代码审查** - 即使自动推送，也应定期审查 AI 提交的代码
3. **分支保护** - 在 GitHub 上设置分支保护，要求 PR 审查
4. **测试覆盖** - 保持足够的测试覆盖率，确保 AI 修改不会破坏功能

---

## 🔮 未来计划

- [ ] 支持自定义审查规则
- [ ] 支持多分支进化
- [ ] 支持 Pull Request 自动创建
- [ ] 支持进化进度可视化
- [ ] 支持用户干预和审查点

---

## 📚 相关文档

- [AI_AUTONOMY_EVOLUTION_PLAN.json](./DUAL_LOOP_IMPROVEMENT_PLAN.json) - 自主进化计划
- [DUAL_LOOP_IMPLEMENTATION_REPORT.md](./DUAL_LOOP_IMPLEMENTATION_REPORT.md) - 双循环实现报告
- [USER_GUIDE.md](../USER_GUIDE.md) - 用户指南
