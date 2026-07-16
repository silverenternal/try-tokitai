#!/bin/bash

# Atlas AI Assistant demo script
# 自动配置环境变量并启动交互式会话

set -e

# 获取脚本所在目录（跨平台兼容）
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# 从 .env 文件加载环境变量（如果存在）
if [ -f ".env" ]; then
    export $(cat .env | grep -v "^#" | xargs)
fi

export AI_API_URL="${AI_API_URL:-https://ollama.com/v1/chat/completions}"
export AI_MODEL="${AI_MODEL:-qwen3.5:397b}"

# 检查 API key 配置（支持多供应商模式）
if [ -z "$AI_API_KEY" ] && [ -z "$PROVIDERS" ]; then
    echo "⚠️  警告：未配置 API Key"
    echo "   单供应商模式：在 .env 中设置 AI_API_KEY"
    echo "   多供应商模式：在 .env 中设置 PROVIDERS=ollama,zazaz 和 PROVIDER_XXX_API_KEY"
    echo ""
fi

# 切换到项目目录
cd "$SCRIPT_DIR"

# 检查是否启用自主模式
USE_AUTONOMOUS=false
for arg in "$@"; do
    if [ "$arg" = "--autonomous" ] || [ "$arg" = "-a" ]; then
        USE_AUTONOMOUS=true
        break
    fi
done

# 简洁启动信息
echo "========================================"
echo "  🔥 Atlas AI Assistant"
echo "========================================"
echo ""

if [ "$USE_AUTONOMOUS" = true ]; then
    echo "🚀 启动模式：项目自更新服务（自主进化）"
    echo "💡 按 Ctrl+C 停止"
else
    echo "🚀 启动模式：CLI AI 助手（交互式对话）"
    echo "💡 输入 'help' 查看功能，'quit' 退出"
fi

echo ""
echo "========================================"
echo ""

# 启动程序（使用 quiet 模式减少 cargo 输出干扰）
if [ "$USE_AUTONOMOUS" = true ]; then
    exec cargo run --quiet --release -- --autonomous
else
    exec cargo run --quiet --release
fi
