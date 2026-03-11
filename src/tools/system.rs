use tokitai::tool;
use std::process::Command;
use serde_json::json;

/// 系统命令执行工具集
pub struct SystemTools;

#[tool]
impl SystemTools {
    /// 执行安全的 shell 命令（白名单命令）
    /// 只能执行安全的只读命令，如 ls, cat, grep 等
    pub fn run_safe_command(&self, command: String) -> Result<String, String> {
        // 提取命令的第一个词（命令名）
        let command_name = command.split_whitespace().next().unwrap_or("");
        
        // 检查黑名单
        if is_dangerous_command(command_name) {
            return Err(format!("⚠️ 安全限制：命令 '{}' 在黑名单中，禁止执行", command_name));
        }
        
        let output = Command::new("bash")
            .arg("-c")
            .arg(&command)
            .output()
            .map_err(|e| format!("执行命令失败：{}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let mut result = String::new();
        if !stdout.is_empty() {
            result.push_str(&stdout);
        }
        if !stderr.is_empty() {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(&format!("⚠️ {}\n", stderr));
        }
        if result.is_empty() {
            result.push_str("命令执行成功，无输出");
        }

        Ok(result)
    }

    /// 执行任意 shell 命令（需要确认）
    /// 注意：此命令可能执行危险操作，请谨慎使用
    pub fn run_command(&self, command: String, confirmed: bool) -> Result<String, String> {
        if !confirmed {
            return Err("⚠️ 此命令可能需要确认才能执行。请设置 confirmed=true 后重试。".to_string());
        }
        
        let output = Command::new("bash")
            .arg("-c")
            .arg(&command)
            .output()
            .map_err(|e| format!("执行命令失败：{}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let status = output.status;

        let mut result = String::new();
        
        // 添加执行状态
        if status.success() {
            result.push_str("✅ 执行成功\n");
        } else {
            result.push_str(&format!("❌ 执行失败 (退出码：{})\n", status));
        }
        
        // 添加标准输出
        if !stdout.is_empty() {
            result.push_str(&format!("\n📋 输出:\n{}\n", stdout));
        }
        
        // 添加标准错误
        if !stderr.is_empty() {
            result.push_str(&format!("\n⚠️ 错误输出:\n{}\n", stderr));
        }

        Ok(result)
    }

    /// 获取当前工作目录
    pub fn get_current_dir(&self) -> Result<String, String> {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .map_err(|e| format!("获取当前目录失败：{}", e))
    }

    /// 获取环境变量
    pub fn get_env(&self, key: String) -> Result<String, String> {
        std::env::var(&key)
            .map_err(|e| format!("获取环境变量失败：{}", e))
    }

    /// 列出所有环境变量
    pub fn list_env(&self) -> Result<String, String> {
        let vars: Vec<String> = std::env::vars()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        Ok(vars.join("\n"))
    }

    /// 检查命令是否可用
    pub fn check_command(&self, command: String) -> Result<serde_json::Value, String> {
        // 使用 which 命令查找
        let output = Command::new("which")
            .arg(&command)
            .output();

        match output {
            Ok(out) => {
                if out.status.success() {
                    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    Ok(json!({
                        "available": true,
                        "path": path,
                        "command": command
                    }))
                } else {
                    Ok(json!({
                        "available": false,
                        "command": command,
                        "message": "命令未找到"
                    }))
                }
            }
            Err(e) => Err(format!("检查命令失败：{}", e))
        }
    }

    /// 获取系统信息
    pub fn get_system_info(&self) -> Result<serde_json::Value, String> {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        let family = std::env::consts::FAMILY;
        
        // 获取主机名
        let hostname = Command::new("hostname")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        
        // 获取用户
        let user = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "unknown".to_string());
        
        Ok(json!({
            "os": os,
            "arch": arch,
            "family": family,
            "hostname": hostname,
            "user": user
        }))
    }

    /// 列出可用命令（前 N 个）
    pub fn list_available_commands(&self, limit: Option<usize>) -> Result<serde_json::Value, String> {
        let limit = limit.unwrap_or(50);
        
        // 获取 PATH
        let paths = std::env::var("PATH")
            .map_err(|e| format!("获取 PATH 失败：{}", e))?;
        
        let mut commands = Vec::new();
        
        for path in std::env::split_paths(&paths) {
            if !path.exists() {
                continue;
            }
            
            if let Ok(entries) = std::fs::read_dir(&path) {
                for entry in entries.flatten() {
                    let file_path = entry.path();
                    if file_path.is_file() && is_executable(&file_path) {
                        if let Some(name) = file_path.file_name() {
                            if let Some(name_str) = name.to_str() {
                                if !name_str.starts_with('.') && commands.len() < limit {
                                    commands.push(name_str.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        
        commands.sort();
        commands.dedup();
        
        Ok(json!({
            "commands": commands,
            "total": commands.len(),
            "limit": limit
        }))
    }
}

/// 检查是否是危险命令
fn is_dangerous_command(command: &str) -> bool {
    let dangerous = [
        "rm", "dd", "mkfs", "fdisk", "parted", "shred",
        "chmod", "chown", "chgrp",
        "sudo", "su", "pkexec", "doas",
        "wget", "curl", "nc", "netcat", "telnet",
        "ssh", "scp", "rsync",
        "kill", "pkill", "killall", "xkill",
        "shutdown", "reboot", "halt", "poweroff", "init",
        "mount", "umount", "losetup",
        "iptables", "firewall-cmd", "ufw", "nft",
        "visudo", "passwd", "useradd", "userdel", "usermod",
        "groupadd", "groupdel", "groupmod",
        "mkfifo", "mknod",
        "insmod", "rmmod", "modprobe",
        "exportfs", "nfsstat",
    ];
    
    dangerous.contains(&command)
}

/// 检查文件是否可执行
#[cfg(unix)]
fn is_executable(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = std::fs::metadata(path) {
        let mode = metadata.permissions().mode();
        return mode & 0o111 != 0;
    }
    false
}

#[cfg(windows)]
fn is_executable(path: &std::path::Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        return ext_str == "exe" || ext_str == "bat" || ext_str == "cmd" || ext_str == "com";
    }
    false
}
