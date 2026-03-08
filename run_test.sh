#!/bin/bash

export AI_API_URL="https://ollama.com/v1/chat/completions"
export AI_API_KEY="645c36802a434774b0ff2101596e1c2d.Re7mAsiOwiRTGx6UNNk1sv_M"

cd /home/hugo/codes/try-tokitai

echo "🧪 测试 AI 助手..."

# 使用 script 命令创建伪终端
script -q -c "cargo run" /dev/stdin <<EOF
你好
查看当前目录下有哪些文件
读取 README.md 文件
exit
EOF
