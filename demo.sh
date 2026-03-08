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

echo "========================================"
echo "  🤖 AI Assistant - Tokitai Demo"
echo "========================================"
echo ""
echo "API URL: $AI_API_URL"
echo "Model: qwen3.5:397b"
echo ""
echo "可用命令:"
echo "  - 直接输入问题与 AI 对话"
echo "  - 输入 'help' 查看示例命令"
echo "  - 输入 'exit' 或 'quit' 退出"
echo ""
echo "========================================"
echo ""

# 直接执行，确保环境变量传递
exec cargo run --release
