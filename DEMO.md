# AI Assistant 演示指南

## 📋 演示准备

### 1. 环境检查

```bash
# 确认项目可以编译
cargo build --release

# 确认 API 配置
export AI_API_URL="https://ollama.com/v1/chat/completions"
export AI_API_KEY="你的 API key"
```

### 2. 演示脚本（5 分钟版本）

---

## 🎬 演示流程

### 第一幕：基础对话 (30 秒)

```
👤 输入：你好，介绍一下你自己

预期输出：
AI 会介绍自己是一个可以调用工具的助手
```

**演示要点**：
- 展示 AI 有基础对话能力
- 提到可以调用工具

---

### 第二幕：查看目录 (1 分钟)

```
👤 输入：帮我看看当前目录下有哪些文件

预期输出：
🔧 执行工具：get_current_dir
✅ 工具执行成功
🔧 执行工具：list_dir
✅ 工具执行成功

AI: 当前目录有以下文件...
```

**演示要点**：
- 展示 AI 可以**自主决定调用多个工具**
- 先获取当前目录路径，再列出内容
- 工具调用过程对用户透明

---

### 第三幕：读取文件 (1 分钟)

```
👤 输入：我想看看 README.md 的内容

预期输出：
🔧 执行工具：read_file
✅ 工具执行成功

AI: 已读取 README.md，内容是...
```

**演示要点**：
- 展示文件读取能力
- AI 会总结文件内容，不是简单输出

---

### 第四幕：代码分析 (1 分钟)

```
👤 输入：分析一下 src/main.rs 这个文件

预期输出：
AI 可能调用：
- detect_language: 检测编程语言
- count_lines: 统计代码行数
- find_functions: 查找函数定义
```

**演示要点**：
- 展示代码分析能力
- AI 会根据需求选择合适工具

---

### 第五幕：执行命令 (1 分钟)

```
👤 输入：帮我运行 cargo --version

预期输出：
🔧 执行工具：run_command
✅ 工具执行成功

AI: 命令执行结果：cargo 1.x.x...
```

**演示要点**：
- 展示系统命令执行能力
- 可以集成到任何工作流

---

### 第六幕：组合任务 (1 分钟)

```
👤 输入：创建一个测试文件，写入"Hello Tokitai"，然后读取它

预期输出：
🔧 执行工具：write_file
✅ 工具执行成功
🔧 执行工具：read_file  
✅ 工具执行成功

AI: 已成功创建并读取文件，内容是...
```

**演示要点**：
- 展示多步骤任务
- AI 可以规划工具调用顺序

---

## 🎯 演示技巧

### 1. 突出 tokitai 的价值

```
"大家注意，这个 AI 助手的所有工具都是用 Rust 的 tokitai 库定义的。
tokitai 是一个编译时工具定义框架，由我开发。

它的核心优势：
- 只需一个 #[tool] 宏
- 类型安全，错误在编译期发现
- 零运行时依赖（可选）
- 兼容任何 AI 供应商
"
```

### 2. 展示代码结构

```bash
# 打开项目结构
tree -L 2 src/

# 展示工具定义有多简单
cat src/tools/file_ops.rs | head -20
```

### 3. 对比传统方式

```
传统方式需要：
1. 手写 JSON Schema
2. 手动解析参数
3. 运行时验证类型

使用 tokitai：
1. 写普通 Rust 函数
2. 加 #[tool] 宏
3. 完成！
```

---

## 🚀 快速演示命令

### 方式 1：交互式（推荐）

```bash
export AI_API_URL="https://ollama.com/v1/chat/completions"
export AI_API_KEY="你的 API key"
cargo run --release
```

### 方式 2：自动化演示

```bash
# 使用提供的演示脚本
./demo.sh
```

### 方式 3：API 测试（展示底层能力）

```bash
# 测试 API 连接
cargo run --example test_api

# 测试完整对话流程
cargo run --example test_chat
```

---

## 💡 常见问题解答

### Q1: tokitai 和 MCP 有什么区别？

**A**: 
- tokitai 是 Rust 库，专注编译时工具定义
- MCP 是网络协议，专注跨进程通信
- tokitai 可以作為 MCP Server 的底层实现

### Q2: 支持哪些 AI 模型？

**A**: 任何支持 Function Calling 的模型：
- Ollama (qwen3.5:397b, 等)
- OpenAI (GPT-4, GPT-3.5)
- Azure OpenAI
- 其他兼容 API

### Q3: 如何添加新工具？

**A**: 三步：
1. 在 `src/tools/` 新建文件
2. 用 `#[tool]` 宏标记 impl
3. 在 `mod.rs` 导出，在 `main.rs` 注册

### Q4: 性能如何？

**A**: 
- 工具定义在编译时生成，零运行时开销
- 工具执行是直接的 Rust 函数调用
- 唯一网络开销是 AI API 调用

---

## 📊 演示数据

### 工具数量
- 文件操作：5 个工具
- 系统命令：4 个工具
- 代码分析：4 个工具
- 网络搜索：2 个工具
- **总计：15 个工具**

### 代码量
- 工具定义：~200 行
- 主程序：~350 行
- 总计：~550 行 Rust 代码

### 依赖
- tokitai: 0.3.3
- reqwest: HTTP 客户端
- serde_json: JSON 处理
- anyhow: 错误处理

---

## 🎬 演示结尾

```
"这就是使用 tokitai 构建的 AI 助手。
tokitai 让 AI 工具集成变得简单、类型安全、高性能。

项目代码已开源，欢迎大家试用和贡献！
"
```

---

## 📝 演示检查清单

- [ ] 环境配置完成
- [ ] API key 有效
- [ ] 项目可以编译
- [ ] 测试运行通过
- [ ] 演示脚本熟悉
- [ ] 备用方案准备（录屏/截图）
