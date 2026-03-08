#!/bin/bash

# 测试 AI 助手的脚本

# 从 .env 文件加载环境变量（如果存在）
if [ -f ".env" ]; then
    export $(cat .env | grep -v "^#" | xargs)
fi

export AI_API_URL="${AI_API_URL:-https://api.olui.ai/v1/chat/completions}"

# 检查 API key 是否已设置
if [ -z "$AI_API_KEY" ]; then
    echo "⚠️  警告：未设置 AI_API_KEY"
    echo "   请复制 .env.example 为 .env 并填入你的 API key"
    echo ""
fi

cd /home/hugo/codes/try-tokitai

echo "🧪 开始测试 AI 助手..."
echo "API URL: $AI_API_URL"
echo ""

# 使用 expect 或简单的 echo 管道来测试
{
    echo "你好，请介绍一下你自己"
    sleep 2
    echo "帮我查看当前目录下有哪些文件"
    sleep 3
    echo "读取 README.md 文件的内容"
    sleep 3
    echo "exit"
} | cargo run 2>&1
