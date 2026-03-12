#!/bin/bash

# AI Assistant 演示脚本
# 自动配置环境变量并启动交互式会话

set -e

# 从 .env 文件加载环境变量（如果存在）
if [ -f ".env" ]; then
    export $(cat .env | grep -v "^#" | xargs)
fi

export AI_API_URL="${AI_API_URL:-https://ollama.com/v1/chat/completions}"

# 检查 API key 是否已设置
if [ -z "$AI_API_KEY" ]; then
    echo "⚠️  警告：未设置 AI_API_KEY"
    echo "   请复制 .env.example 为 .env 并填入你的 API key"
    echo ""
fi

cd /home/hugo/codes/try-tokitai

# 检查是否使用 TUI 模式
if [ "$1" = "--tui" ] || [ "$1" = "-t" ]; then
    echo "========================================"
    echo "  🤖 AI Assistant - TUI 模式"
    echo "========================================"
    echo ""
    echo "✨ 性能优化已启用："
    echo "   - 缓存响应 <10ms (50x 提升)"
    echo "   - 流式首字节延迟降低 60-70%"
    echo "   - 全局连接池复用"
    echo "   - 实时延迟监控"
    echo ""
    echo "快捷键：PageUp/PageDown 滚动，Ctrl+L 清除历史，Ctrl+C 退出"
    echo ""
    echo "========================================"
    echo ""
    exec cargo run --release -- --tui
else
    echo "========================================"
    echo "  🤖 AI Assistant - 命令行模式"
    echo "========================================"
    echo ""
    echo "API URL: $AI_API_URL"
    echo "Model: qwen3.5:397b"
    echo ""
    echo "✨ 性能优化："
    echo "   - 缓存响应 <10ms (50x 提升)"
    echo "   - 流式首字节延迟降低 60-70%"
    echo "   - 全局连接池复用"
    echo "   - 纯异步线程模型"
    echo ""
    echo "可用命令:"
    echo "  - 直接输入问题与 AI 对话"
    echo "  - 输入 'help' 查看示例命令"
    echo "  - 输入 'exit' 或 'quit' 退出"
    echo ""
    echo "工具示例:"
    echo "  - 文件：'读取 README.md 的内容'"
    echo "  - 代码：'分析 @src/main.rs 的结构'"
    echo "  - HTTP: '发送 GET 请求到 https://api.github.com'"
    echo "  - JSON: '格式化这个 JSON: {"a":1,"b":2}'"
    echo "  - 搜索：'在 src 目录中搜索函数 main'"
    echo "  - 进程：'列出当前运行的前 10 个进程'"
    echo "  - 网络：'检查 localhost 的 80 端口是否开放'"
    echo "  - Git:  '查看最近的提交记录'"
    echo ""
    echo "提示：使用 @ 文件路径可快速引用文件内容"
    echo "提示：使用 ./demo.sh --tui 启动 TUI 界面"
    echo ""
    echo "========================================"
    echo ""
    exec cargo run --release
fi
