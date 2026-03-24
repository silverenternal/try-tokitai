//! 系统工具配置
//!
//! 统一管理所有魔法数字和配置常量

/// 最大输出大小（100KB）
pub const MAX_OUTPUT_SIZE: usize = 100 * 1024;

/// 进程文件输出最大大小（50KB）
#[allow(dead_code)]
pub const MAX_PROCESS_FILES_OUTPUT_SIZE: usize = 50 * 1024;

/// 最大命令长度
pub const MAX_COMMAND_LENGTH: usize = 4096;

/// 最大搜索模式长度
pub const MAX_PATTERN_LENGTH: usize = 256;

/// 进程列表默认限制
#[allow(dead_code)]
pub const DEFAULT_PROCESS_LIMIT: usize = 20;

/// 进程列表最大限制
#[allow(dead_code)]
pub const MAX_PROCESS_LIMIT: usize = 100;

/// 进程文件默认限制
#[allow(dead_code)]
pub const DEFAULT_PROCESS_FILES_LIMIT: usize = 50;

/// 进程文件最大限制
#[allow(dead_code)]
pub const MAX_PROCESS_FILES_LIMIT: usize = 200;

/// 可用命令默认限制
pub const DEFAULT_COMMANDS_LIMIT: usize = 50;

/// 可用命令最大限制
pub const MAX_COMMANDS_LIMIT: usize = 500;

/// 代码搜索默认限制
pub const DEFAULT_CODE_SEARCH_LIMIT: usize = 50;

/// 代码搜索最大限制
pub const MAX_CODE_SEARCH_LIMIT: usize = 200;

/// 敏感环境变量模式
pub const SENSITIVE_ENV_PATTERNS: &[&str] = &[
    "PASSWORD", "PASSWD", "SECRET", "TOKEN", "API_KEY", "APIKEY",
    "PRIVATE_KEY", "PRIVATEKEY", "CREDENTIAL", "CRED",
    "AWS_SECRET", "AZURE_", "GCP_", "DATABASE_URL", "DB_PASS",
    "ENCRYPTION_KEY", "SIGNING_KEY", "AUTH_TOKEN", "ACCESS_TOKEN",
    "REFRESH_TOKEN", "SESSION_KEY",
];

/// 危险命令黑名单
pub const DANGEROUS_COMMANDS: &[&str] = &[
    // 文件删除/破坏
    "rm", "dd", "mkfs", "fdisk", "parted", "shred", "wipe",
    // 权限修改
    "chmod", "chown", "chgrp",
    // 提权
    "sudo", "su", "pkexec", "doas", "runas",
    // 网络（可能泄露数据）
    "wget", "curl", "nc", "netcat", "telnet", "ssh", "scp", "rsync",
    // 进程终止
    "kill", "pkill", "killall", "xkill",
    // 系统控制
    "shutdown", "reboot", "halt", "poweroff", "init", "systemctl", "service",
    // 文件系统
    "mount", "umount", "losetup", "mkswap", "swapon", "swapoff",
    // 防火墙/网络配置
    "iptables", "firewall-cmd", "ufw", "nft", "ip", "ifconfig", "route",
    // 用户管理
    "visudo", "passwd", "useradd", "userdel", "usermod", "groupadd", "groupdel", "groupmod",
    // 设备/内核
    "mkfifo", "mknod", "insmod", "rmmod", "modprobe",
    // NFS
    "exportfs", "nfsstat",
    // 包管理器（可能安装恶意软件）
    "apt", "apt-get", "yum", "dnf", "pacman", "brew", "pip", "npm", "cargo",
    // 其他危险命令
    "crontab", "at", "batch",
];

/// 安全命令白名单
pub const WHITELISTED_COMMANDS: &[&str] = &[
    // 文件操作
    "ls", "cat", "head", "tail", "wc", "file", "stat", "readlink", "realpath",
    "basename", "dirname", "cp", "mv", "mkdir", "rmdir", "touch", "ln",
    // 搜索工具
    "grep", "find", "locate", "which", "whereis", "type",
    // 文本处理
    "awk", "sed", "cut", "sort", "uniq", "tr", "tee", "xargs", "paste",
    "join", "comm", "diff", "patch",
    // 系统信息
    "pwd", "whoami", "id", "uname", "hostname", "date", "time", "cal", "uptime",
    "ps", "pgrep", "top", "free", "df", "du", "lsof", "netstat", "ss",
    // 其他安全命令
    "echo", "printf", "true", "false", "sleep", "yes", "seq", "shuf",
    // 压缩/解压
    "tar", "gzip", "gunzip", "zip", "unzip", "xz", "unxz",
    // 文档
    "man", "info", "whatis", "apropos",
];

/// 危险的 shell 元字符
#[allow(dead_code)]
pub const DANGEROUS_SHELL_CHARS: &[char] = &[
    ';',  // 命令分隔符
    '|',  // 管道
    '&',  // 后台执行/逻辑与
    '$',  // 变量/命令替换
    '`',  // 命令替换
    '\\', // 转义
    '<',  // 重定向输入
    '>',  // 重定向输出
    '(',  // 子 shell
    ')',  // 子 shell
    '{',  // 命令组
    '}',  // 命令组
    '[',  // 字符类
    ']',  // 字符类
    '!',  // 历史扩展
    '\'', // 单引号
    '"',  // 双引号
    '\n', // 换行
    '\r', // 回车
];

/// 工具元数据
pub struct ToolMetadata {
    pub name: &'static str,
    pub description: &'static str,
    pub version: &'static str,
}

#[allow(dead_code)]
pub const PROCESS_MANAGER_METADATA: ToolMetadata = ToolMetadata {
    name: "process_manager",
    description: "进程管理工具 - 查询、监控和管理系统进程",
    version: "1.0.0",
};

#[allow(dead_code)]
pub const SYSTEM_MONITOR_METADATA: ToolMetadata = ToolMetadata {
    name: "system_monitor",
    description: "系统监控工具 - 获取 CPU、内存、磁盘等系统资源信息",
    version: "1.0.0",
};

#[allow(dead_code)]
pub const SYSTEM_COMMANDS_METADATA: ToolMetadata = ToolMetadata {
    name: "system_commands",
    description: "系统命令执行工具 - 安全执行 shell 命令（白名单机制）",
    version: "1.0.0",
};

pub const CODE_ANALYZER_METADATA: ToolMetadata = ToolMetadata {
    name: "code_analyzer",
    description: "代码分析工具 - 统计行数、查找函数、检测语言类型",
    version: "1.0.0",
};
