#!/bin/bash
# 自动化演示测试脚本

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
echo "  AI Assistant 演示测试"
echo "========================================"
echo ""

# 使用 printf 发送命令
{
    printf "help\n"
    sleep 2
    printf "查看当前目录有哪些文件\n"
    sleep 8
    printf "读取 README.md 的内容\n"
    sleep 8
    printf "exit\n"
} | cargo run --release 2>&1

echo ""
echo "========================================"
echo "  演示完成"
echo "========================================"
