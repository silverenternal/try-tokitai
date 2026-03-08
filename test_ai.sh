#!/bin/bash

# 测试 AI 助手的脚本

export AI_API_URL="https://api.olui.ai/v1/chat/completions"
export AI_API_KEY="645c36802a434774b0ff2101596e1c2d.Re7mAsiOwiRTGx6UNNk1sv_M"

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
