#!/usr/bin/expect -f

set timeout 120

# 设置环境变量
set env(AI_API_URL) "https://ollama.com/v1/chat/completions"
set env(AI_API_KEY) "645c36802a434774b0ff2101596e1c2d.Re7mAsiOwiRTGx6UNNk1sv_M"

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
