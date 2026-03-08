#!/bin/bash
# 自动化演示测试脚本

export AI_API_URL="https://ollama.com/v1/chat/completions"
export AI_API_KEY="645c36802a434774b0ff2101596e1c2d.Re7mAsiOwiRTGx6UNNk1sv_M"

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
