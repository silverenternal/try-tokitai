#!/usr/bin/expect -f

# 设置超时
set timeout 60

# 设置环境变量
set env(AI_API_URL) "https://ollama.com/v1/chat/completions"
set env(AI_API_KEY) "645c36802a434774b0ff2101596e1c2d.Re7mAsiOwiRTGx6UNNk1sv_M"

# 启动程序
spawn cargo run

# 等待提示符
expect "👤 你："

# 发送第一个测试命令
send "你好\r"
expect -re "🤖 AI:.*\n"
expect "👤 你："

# 发送第二个测试命令
send "查看当前目录下有哪些文件\r"
expect -re "🤖 AI:.*\n"
expect "👤 你："

# 退出
send "exit\r"
expect eof
