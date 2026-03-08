#!/bin/bash

# AI Assistant 演示脚本
# 自动配置环境变量并启动交互式会话

set -e

export AI_API_URL="https://ollama.com/v1/chat/completions"
export AI_API_KEY="645c36802a434774b0ff2101596e1c2d.Re7mAsiOwiRTGx6UNNk1sv_M"

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
