#!/usr/bin/expect -f

set timeout 120

# 从 .env 文件加载环境变量（如果存在）
# 注意：expect 无法直接读取 .env 文件，需要在运行前手动设置
# 建议：运行此脚本前先执行 source .env 或 export 变量

if {[info exists env(AI_API_KEY)] == 0} {
    puts "⚠️  警告：未设置环境变量 AI_API_KEY"
    puts "   请先运行：source .env 或手动 export AI_API_KEY=your_key"
    puts ""
}

set env(AI_API_URL) [expr {[info exists env(AI_API_URL)] ? $env(AI_API_URL) : "https://ollama.com/v1/chat/completions"}]

# 启动程序
spawn cargo run --release

# 等待提示符
expect "👤 你："

# 发送 help 命令
send "help\r"
sleep 2
expect "👤 你："

# 查看目录
send "查看当前目录有哪些文件\r"
sleep 5
expect "👤 你："

# 读取文件
send "读取 README.md 的内容\r"
sleep 5
expect "👤 你："

# 退出
send "exit\r"
expect eof
