# AI Assistant powered by Tokitai

一个使用 Rust 和 Tokitai 构建的强大 AI 助手，可以让 AI 调用各种工具来完成实际任务。

## 🚀 快速开始

### 一键启动演示

```bash
./demo.sh
```

### 手动启动

```bash
export AI_API_URL="https://ollama.com/v1/chat/completions"
export AI_API_KEY="你的 API key"
cargo run --release
```

---

## 功能特性

### 🛠️ 工具系统

基于 [tokitai](https://github.com/silverenternal/tokitai) 库实现编译时工具定义：

- **文件操作** - 读取/写入文件、列出目录、复制/删除文件
- **系统命令** - 执行 shell 命令、获取环境变量
- **代码分析** - 统计代码行数、查找函数定义、检测编程语言
- **网络搜索** - 搜索网页内容、获取 URL 内容

### 🤖 AI 集成

- 支持 OpenAI 兼容 API（Ollama Cloud、OpenAI、Azure 等）
- 自动工具调用（Function Calling）
- 多轮对话历史记忆

---

## 💬 交互命令

启动后可以使用：

| 命令 | 说明 |
|------|------|
| `help` | 显示可用操作列表 |
| `exit` / `quit` | 退出程序 |
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

### 6. 多步骤任务
```
👤 你：帮我看看 Cargo.toml 的内容，然后统计一下有多少行
```

---

## 项目结构

```
.
├── Cargo.toml              # 项目依赖配置
├── demo.sh                 # 一键启动脚本
├── src/
│   ├── main.rs             # 主程序入口
│   └── tools/              # 工具模块
│       ├── mod.rs          # 模块导出
│       ├── file_ops.rs     # 文件操作工具
│       ├── system.rs       # 系统命令工具
│       ├── code_analysis.rs # 代码分析工具
│       └── web_search.rs   # 网络搜索工具
├── examples/               # 示例代码
└── README.md
```

---

## 环境变量

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| `AI_API_URL` | AI API 地址 | `https://ollama.com/v1/chat/completions` |
| `AI_API_KEY` | API 密钥 | 无 |

---

## 可用模型

在 Ollama Cloud 上可用的模型：
- `qwen3.5:397b` - 通义千问 3.5（397B 参数）
- `qwen3-coder:480b` - 通义千问代码版（480B 参数）
- `deepseek-v3.2` - DeepSeek V3.2
- `gemma3` 系列 - Google Gemma 3

---

## 技术栈

- **Rust** - 系统编程语言
- **tokitai** - AI 工具集成框架
- **reqwest** - HTTP 客户端
- **serde_json** - JSON 处理
- **anyhow** - 错误处理

---

## 许可证

MIT OR Apache-2.0

## 致谢

- [tokitai](https://github.com/silverenternal/tokitai) - 优秀的 AI 工具集成库
