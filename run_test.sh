#!/bin/bash

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

echo "🧪 测试 AI 助手..."

# 使用 script 命令创建伪终端
script -q -c "cargo run" /dev/stdin <<EOF
你好
查看当前目录下有哪些文件
读取 README.md 文件
exit
EOF
