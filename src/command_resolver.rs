//! 命令解析模块 - 解析系统可用命令
//!
//! 功能：
//! - 扫描系统 PATH 中的可执行命令
//! - 提供命令自动补全建议
//! - 命令安全性检查
//!
//! 安全机制：
//! - 命令白名单优先（推荐）
//! - 命令黑名单阻止
//! - Shell 注入防护

use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, warn};

/// 命令执行模式
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum CommandMode {
    /// 白名单模式：只允许白名单中的命令（最安全）
    Whitelist,
    /// 黑名单模式：只阻止黑名单中的命令（默认）
    Blacklist,
}

/// 命令解析器
pub struct CommandResolver {
    /// 系统 PATH 中的命令缓存
    available_commands: HashSet<String>,
    /// 命令白名单（允许执行的命令）
    whitelist: HashSet<String>,
    /// 命令黑名单（禁止执行的命令）
    blacklist: HashSet<String>,
    /// 执行模式
    #[allow(dead_code)]
    mode: CommandMode,
}

impl CommandResolver {
    /// 创建新的命令解析器（默认黑名单模式）
    pub fn new() -> Self {
        Self::with_mode(CommandMode::Blacklist)
    }

    /// 创建指定模式的命令解析器
    pub fn with_mode(mode: CommandMode) -> Self {
        let mut resolver = Self {
            available_commands: HashSet::new(),
            whitelist: HashSet::new(),
            blacklist: HashSet::new(),
            mode,
        };

        // 初始化黑名单
        resolver.init_blacklist();

        // 初始化默认白名单（安全命令）
        resolver.init_default_whitelist();

        // 扫描可用命令
        resolver.scan_available_commands();

        resolver
    }

    /// 初始化默认白名单（只读安全命令）
    fn init_default_whitelist(&mut self) {
        let safe_commands = vec![
            "ls", "dir", "pwd", "whoami", "hostname", "echo", "cat", "head", "tail", "less",
            "more", "grep", "egrep", "fgrep", "find", "which", "whereis", "man", "uname", "uptime",
            "date", "cal", "df", "du", "free", "top", "ps", "git", "cargo", "rustc", "npm", "node",
            "python", "python3",
        ];
        for cmd in safe_commands {
            self.whitelist.insert(cmd.to_string());
        }
    }

    /// 初始化命令黑名单
    fn init_blacklist(&mut self) {
        // 危险命令黑名单
        let dangerous_commands = vec![
            "rm",
            "dd",
            "mkfs",
            "fdisk",
            "parted",
            "chmod",
            "chown",
            "chgrp",
            "sudo",
            "su",
            "pkexec",
            "doas",
            "wget",
            "curl",
            "nc",
            "netcat",
            "ssh",
            "scp",
            "rsync",
            "kill",
            "pkill",
            "killall",
            "shutdown",
            "reboot",
            "halt",
            "poweroff",
            "mount",
            "umount",
            "iptables",
            "firewall-cmd",
            "ufw",
            "visudo",
            "passwd",
            "useradd",
            "userdel",
            "usermod",
            "groupadd",
            "groupdel",
            "groupmod",
        ];

        for cmd in dangerous_commands {
            self.blacklist.insert(cmd.to_string());
        }
    }

    /// 扫描系统 PATH 中的可用命令
    pub fn scan_available_commands(&mut self) {
        self.available_commands.clear();

        // 获取 PATH 环境变量
        let paths = match env::var_os("PATH") {
            Some(val) => val,
            None => {
                warn!("PATH 环境变量未设置");
                return;
            }
        };

        // 遍历 PATH 中的每个目录
        for path in env::split_paths(&paths) {
            if !path.exists() {
                continue;
            }

            // 读取目录内容
            let entries = match std::fs::read_dir(&path) {
                Ok(entries) => entries,
                Err(e) => {
                    debug!("无法读取目录 {:?}: {}", path, e);
                    continue;
                }
            };

            for entry in entries.flatten() {
                let file_path = entry.path();

                // 检查是否是可执行文件
                if self.is_executable_file(&file_path) {
                    if let Some(file_name) = file_path.file_name() {
                        if let Some(name_str) = file_name.to_str() {
                            // 跳过隐藏文件
                            if !name_str.starts_with('.') {
                                self.available_commands.insert(name_str.to_string());
                            }
                        }
                    }
                }
            }
        }

        debug!("扫描到 {} 个可用命令", self.available_commands.len());
    }

    /// 检查文件是否是可执行文件
    fn is_executable_file(&self, path: &Path) -> bool {
        // 必须是文件
        if !path.is_file() {
            return false;
        }

        // 检查是否有执行权限（Unix 系统）
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(path) {
                let mode = metadata.permissions().mode();
                // 检查是否有执行权限位
                if mode & 0o111 != 0 {
                    return true;
                }
            }
            false
        }

        #[cfg(windows)]
        {
            // Windows 检查文件扩展名
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                return ext_str == "exe"
                    || ext_str == "bat"
                    || ext_str == "cmd"
                    || ext_str == "com";
            }
            false
        }
    }

    /// 检查命令是否在白名单中
    #[allow(dead_code)]
    pub fn is_whitelisted(&self, command: &str) -> bool {
        self.whitelist.contains(command)
    }

    /// 检查命令是否在黑名单中
    #[allow(dead_code)]
    pub fn is_blacklisted(&self, command: &str) -> bool {
        self.blacklist.contains(command)
    }

    /// 添加到白名单
    #[allow(dead_code)]
    pub fn add_to_whitelist(&mut self, command: String) {
        self.whitelist.insert(command);
    }

    /// 从白名单移除
    #[allow(dead_code)]
    pub fn remove_from_whitelist(&mut self, command: &str) {
        self.whitelist.remove(command);
    }

    /// 检查命令是否可用
    #[allow(dead_code)]
    pub fn is_command_available(&self, command: &str) -> bool {
        self.available_commands.contains(command)
    }

    /// 获取所有可用命令
    #[allow(dead_code)]
    pub fn get_available_commands(&self) -> Vec<String> {
        let mut commands: Vec<String> = self.available_commands.iter().cloned().collect();
        commands.sort();
        commands
    }

    /// 获取安全的命令列表（排除黑名单）
    #[allow(dead_code)]
    pub fn get_safe_commands(&self) -> Vec<String> {
        self.available_commands
            .iter()
            .filter(|cmd| !self.blacklist.contains(*cmd))
            .cloned()
            .collect()
    }

    /// 命令自动补全
    #[allow(dead_code)]
    pub fn autocomplete(&self, prefix: &str) -> Vec<String> {
        self.available_commands
            .iter()
            .filter(|cmd| cmd.starts_with(prefix))
            .take(20) // 最多返回 20 个建议
            .cloned()
            .collect()
    }

    /// 查找命令的完整路径
    #[allow(dead_code)]
    pub fn find_command_path(&self, command: &str) -> Option<PathBuf> {
        let paths = env::var_os("PATH")?;

        for path in env::split_paths(&paths) {
            let full_path = path.join(command);
            if self.is_executable_file(&full_path) {
                return Some(full_path);
            }
        }

        None
    }

    /// 验证命令是否安全可执行（带 Shell 注入防护）
    #[allow(dead_code)]
    pub fn is_safe_to_execute(&self, command: &str) -> Result<bool, String> {
        // Shell 注入防护：检查危险字符
        if Self::contains_shell_injection_chars(command) {
            return Err(format!("命令包含危险字符：{}", command));
        }

        // 根据模式检查
        match self.mode {
            CommandMode::Whitelist => {
                // 白名单模式：只允许白名单中的命令
                if !self.whitelist.contains(command) {
                    return Err(format!("命令 '{}' 不在白名单中", command));
                }
            }
            CommandMode::Blacklist => {
                // 黑名单模式：只阻止黑名单中的命令
                if self.blacklist.contains(command) {
                    return Ok(false);
                }
            }
        }

        // 检查是否可用
        if !self.is_command_available(command) {
            return Err(format!("命令 '{}' 不存在", command));
        }

        Ok(true)
    }

    /// 检查命令是否包含 Shell 注入字符
    #[allow(dead_code)]
    fn contains_shell_injection_chars(command: &str) -> bool {
        let dangerous_chars = [
            ';', '|', '&', '$', '`', '(', ')', '{', '}', '<', '>', '\\', '\n', '\r',
        ];
        command.chars().any(|c| dangerous_chars.contains(&c))
    }

    /// 设置命令执行模式
    #[allow(dead_code)]
    pub fn set_mode(&mut self, mode: CommandMode) {
        self.mode = mode;
    }

    /// 获取当前执行模式
    #[allow(dead_code)]
    pub fn get_mode(&self) -> &CommandMode {
        &self.mode
    }

    /// 获取命令帮助信息
    #[allow(dead_code)]
    pub fn get_command_help(&self, command: &str) -> Option<String> {
        // 尝试 --help
        let output = Command::new(command).arg("--help").output().ok()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // 只返回前几行
            return Some(stdout.lines().take(10).collect::<Vec<_>>().join("\n"));
        }

        // 尝试 -h
        let output = Command::new(command).arg("-h").output().ok()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Some(stdout.lines().take(10).collect::<Vec<_>>().join("\n"));
        }

        None
    }

    /// 获取命令统计信息
    #[allow(dead_code)]
    pub fn get_stats(&self) -> CommandStats {
        CommandStats {
            total_commands: self.available_commands.len(),
            safe_commands: self
                .available_commands
                .iter()
                .filter(|cmd| !self.blacklist.contains(*cmd))
                .count(),
            blacklisted_commands: self.blacklist.len(),
            whitelisted_commands: self.whitelist.len(),
        }
    }
}

/// 命令统计信息
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CommandStats {
    pub total_commands: usize,
    pub safe_commands: usize,
    pub blacklisted_commands: usize,
    pub whitelisted_commands: usize,
}

impl Default for CommandResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolver_creation() {
        let resolver = CommandResolver::new();
        assert!(!resolver.available_commands.is_empty());
    }

    #[test]
    fn test_blacklist_check() {
        let resolver = CommandResolver::new();
        assert!(resolver.is_blacklisted("rm"));
        assert!(resolver.is_blacklisted("sudo"));
        assert!(!resolver.is_blacklisted("ls"));
    }

    #[test]
    fn test_command_availability() {
        let resolver = CommandResolver::new();
        // 这些命令通常在大多数系统中存在
        let common_commands = ["ls", "echo", "cat", "pwd"];
        for cmd in common_commands {
            // 不assert，因为某些系统可能没有这些命令
            let _ = resolver.is_command_available(cmd);
        }
    }

    #[test]
    fn test_autocomplete() {
        let resolver = CommandResolver::new();
        let suggestions = resolver.autocomplete("l");
        // 应该包含 ls, ln 等命令
        assert!(
            !suggestions.is_empty()
                || resolver
                    .available_commands
                    .iter()
                    .all(|c| !c.starts_with('l'))
        );
    }

    #[test]
    fn test_stats() {
        let resolver = CommandResolver::new();
        let stats = resolver.get_stats();
        assert!(stats.total_commands > 0);
        assert!(stats.safe_commands <= stats.total_commands);
    }
}
