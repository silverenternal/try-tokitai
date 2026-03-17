# 🚀 快速启动指南

> **最后更新**: 2026-03-18
> **测试状态**: 236/236 通过 ✅
> **构建状态**: Release ✅

## 一键启动演示

```bash
./demo.sh
```

或者手动启动：

```bash
# 复制环境变量模板
cp .env.example .env

# 编辑 .env 文件，填入你的 API key
# AI_API_KEY=your_api_key_here

# 加载环境变量并启动
source .env
cargo run --release
```

---

## 🎯 双轨服务模式

本项目支持两种运行模式：

| 模式 | 启动命令 | 说明 |
|------|----------|------|
| **CLI AI 助手** | `cargo run --release` | 交互式对话，响应用户查询 |
| **项目自更新** | `cargo run --release -- --autonomous` | AI 自主进化，持续改进代码 |

详细说明：[structure_ensure/SERVICES.md](../structure_ensure/SERVICES.md)

---

## 💬 交互式命令

启动后可以使用以下命令：

| 命令 | 说明 |
|------|------|
| `help` | 显示可用操作列表 |
| `exit` 或 `quit` | 退出程序 |
| 任意自然语言 | 与 AI 对话 |

---

## 📋 演示示例

### 1. 查看帮助
```
👤 你：help
```

### 2. 查看目录
```
👤 你：当前目录有哪些文件
```

### 3. 读取文件
```
👤 你：读取 README.md 的内容
```

### 4. 执行命令
```
👤 你：运行 cargo --version
```

### 5. 分析代码
```
👤 你：分析 src/main.rs 的结构
```

### 6. 创建文件
```
👤 你：创建 test.txt，写入 Hello Tokitai
```

### 7. 多步骤任务
```
👤 你：帮我看看 Cargo.toml 的内容，然后统计一下有多少行
```

---

## 🎯 演示要点

1. **工具调用自动化** - AI 自主决定调用哪些工具
2. **多轮对话记忆** - 上下文连续，可以追问
3. **错误处理** - 工具执行失败时有友好提示
4. **自然语言交互** - 不需要记命令，直接说需求

---

## ⚙️ 配置说明

### 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `AI_API_URL` | AI API 地址 | `https://ollama.com/v1/chat/completions` |
| `AI_API_KEY` | API 密钥 | 无 |
| `AI_MODEL` | 模型名称 | `qwen3.5:397b` |

### 更换模型

编辑 `.env` 文件：
```bash
AI_MODEL=qwen3.5:397b  # 通义千问 3.5（推荐）
# 或
AI_MODEL=qwen3-coder:480b  # 代码专用
# 或
AI_MODEL=deepseek-v3.2  # DeepSeek
```

---

## 🐛 故障排除

### 问题：API 请求失败

**原因**：API key 无效或网络问题

**解决**：
```bash
# 检查 API key 是否正确
echo $AI_API_KEY

# 测试 API 连接
curl -H "Authorization: Bearer $AI_API_KEY" $AI_API_URL
```

### 问题：工具执行失败

**原因**：文件路径不存在或权限问题

**解决**：检查路径是否正确，确保有读写权限

### 问题：编译失败

**解决**：
```bash
# 清理后重新编译
cargo clean
cargo build --release
```

### 问题：自主模式无法启动

**解决**：
```bash
# 确保项目路径正确
cargo run --release -- --autonomous --project-path /path/to/project
```

---

## 📚 相关文档

| 文档 | 说明 |
|------|------|
| [USER_GUIDE.md](USER_GUIDE.md) | 完整用户指南 |
| [DEMO.md](DEMO.md) | 演示指南 |
| [structure_ensure/SERVICES.md](../structure_ensure/SERVICES.md) | 服务双轨架构 |
| [structure_ensure/QUICK_REFERENCE.md](../structure_ensure/QUICK_REFERENCE.md) | 快速参考卡片 |

---

**最后更新**: 2026-03-18
