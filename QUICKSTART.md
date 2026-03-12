# 🚀 快速启动指南

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

### TUI 界面模式（推荐）

```bash
# 使用 TUI 界面（现代化终端 UI）
./demo.sh --tui

# 或手动启动
cargo run --release -- --tui
# 或
cargo run --release -- -t
```

**TUI 优势**：
- ✨ 低延迟：缓存响应 <10ms，流式首字节延迟降低 60-70%
- 📊 性能监控：实时显示请求数、缓存命中率、平均延迟
- 🎨 现代化 UI：消息历史滚动、流式响应显示
- ⌨️ 快捷键：PageUp/PageDown 快速滚动，Ctrl+L 清除历史

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

### 更换模型

编辑 `src/main.rs` 中的模型名称：
```rust
let request_body = json!({
    "model": "qwen3.5:397b",  // 修改这里
    ...
});
```

可用模型：
- `qwen3.5:397b` - 通义千问（推荐）
- `qwen3-coder:480b` - 代码专用
- `deepseek-v3.2` - DeepSeek

---

## 🐛 故障排除

### 问题：API 请求失败

**原因**：API key 无效或网络问题

**解决**：
```bash
# 检查 API key 是否正确
echo $AI_API_KEY

# 测试 API 连接
cargo run --example test_api
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
