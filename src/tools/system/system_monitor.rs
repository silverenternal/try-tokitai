//! 系统监控工具
//!
//! 提供系统资源监控和信息查询功能
//!
//! ## 功能
//! - CPU、内存、磁盘使用情况
//! - 系统负载和运行时间
//! - 系统基本信息（OS、架构、主机名等）
//!
//! ## 性能
//! - 典型延迟：10-50ms
//! - 所有操作均为只读

use tokitai::tool;
use serde_json::json;
use std::sync::Arc;

use super::backend::{ProcessBackend, create_backend};
use super::config;

/// 系统监控工具集
///
/// 提供系统资源使用情况查询功能
///
/// ## 示例
/// ```rust,ignore
/// let monitor = SystemMonitor::default();
/// let resources = monitor.get_system_resources()?;
/// let info = monitor.get_system_info()?;
/// ```
pub struct SystemMonitor {
    backend: Arc<Box<dyn ProcessBackend>>,
}

impl Default for SystemMonitor {
    fn default() -> Self {
        Self {
            backend: Arc::new(create_backend()),
        }
    }
}

impl SystemMonitor {
    /// 创建新的系统监控器
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建带有自定义后端的监控器（用于测试）
    #[allow(dead_code)]
    pub fn with_backend(backend: Box<dyn ProcessBackend>) -> Self {
        Self {
            backend: Arc::new(backend),
        }
    }
}

#[tool]
impl SystemMonitor {
    /// 获取系统资源使用情况
    ///
    /// 返回 CPU、内存、磁盘、负载等系统资源信息
    ///
    /// ## 返回
    /// JSON 格式：
    /// ```json
    /// {
    ///   "success": true,
    ///   "data": {
    ///     "cpu": {"cores": 8},
    ///     "load_avg": {"1m": 1.5, "5m": 1.3, "15m": 1.1},
    ///     "memory": {"total_mb": 16384, "free_mb": 8192, "available_mb": 12288},
    ///     "disk": {"total_gb": 512, "used_gb": 256, "available_gb": 256, "usage_percent": 50},
    ///     "uptime_secs": 86400
    ///   }
    /// }
    /// ```
    ///
    /// ## 错误
    /// - `SystemInfoError::InfoFetchFailed`: 获取系统信息失败
    /// - `SystemInfoError::ParseFailed`: 解析系统信息失败
    ///
    /// ## 性能
    /// - 典型延迟：10-30ms
    pub fn get_system_resources(&self) -> Result<String, String> {
        let info = self.backend.get_system_resources()
            .map_err(|e| e.to_string())?;

        Ok(json!({
            "success": true,
            "data": {
                "cpu": {
                    "cores": info.cpu_cores,
                },
                "load_avg": {
                    "1m": info.load_avg_1m,
                    "5m": info.load_avg_5m,
                    "15m": info.load_avg_15m,
                },
                "memory": {
                    "total_kb": info.mem_total_kb,
                    "free_kb": info.mem_free_kb,
                    "available_kb": info.mem_available_kb,
                    "total_mb": info.mem_total_kb / 1024,
                    "free_mb": info.mem_free_kb / 1024,
                    "available_mb": info.mem_available_kb.map(|v| v / 1024),
                },
                "disk": {
                    "total_gb": info.disk_usage.total_gb,
                    "used_gb": info.disk_usage.used_gb,
                    "available_gb": info.disk_usage.available_gb,
                    "usage_percent": info.disk_usage.usage_percent,
                },
                "uptime_secs": info.uptime_secs,
            }
        }).to_string())
    }

    /// 获取系统基本信息
    ///
    /// 返回操作系统、架构、主机名、用户等基本信息
    ///
    /// ## 返回
    /// JSON 格式：
    /// ```json
    /// {
    ///   "success": true,
    ///   "data": {
    ///     "os": "linux",
    ///     "arch": "x86_64",
    ///     "family": "unix",
    ///     "hostname": "myhost",
    ///     "user": "user"
    ///   }
    /// }
    /// ```
    ///
    /// ## 错误
    /// - 获取主机名失败（罕见）
    ///
    /// ## 性能
    /// - 典型延迟：<5ms
    pub fn get_system_info(&self) -> Result<String, String> {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        let family = std::env::consts::FAMILY;

        // 获取主机名
        let hostname = std::process::Command::new("hostname")
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
            "success": true,
            "data": {
                "os": os,
                "arch": arch,
                "family": family,
                "hostname": hostname,
                "user": user,
            }
        }).to_string())
    }

    /// 获取 CPU 核心数
    ///
    /// 快速返回系统 CPU 核心数量
    ///
    /// ## 返回
    /// JSON 格式：`{"success": true, "data": {"cores": N}}`
    ///
    /// ## 性能
    /// - 典型延迟：<1ms
    pub fn get_cpu_cores(&self) -> Result<String, String> {
        let cores = std::thread::available_parallelism()
            .map(|p| p.get() as u32)
            .unwrap_or(1);

        Ok(json!({
            "success": true,
            "data": {
                "cores": cores
            }
        }).to_string())
    }

    /// 获取系统负载平均值
    ///
    /// 返回 1/5/15 分钟的系统负载平均值
    ///
    /// ## 返回
    /// JSON 格式的负载平均值
    ///
    /// ## 说明
    /// - 负载平均值表示系统繁忙程度
    /// - 值 > CPU 核心数表示系统过载
    /// - Linux/macOS 专用，Windows 不支持
    pub fn get_load_average(&self) -> Result<String, String> {
        #[cfg(unix)]
        {
            use libc::getloadavg;

            let mut loadavg = [0.0f64; 3];
            let result = unsafe { getloadavg(loadavg.as_mut_ptr(), 3) };

            if result >= 3 {
                return Ok(json!({
                    "success": true,
                    "data": {
                        "1m": loadavg[0],
                        "5m": loadavg[1],
                        "15m": loadavg[2],
                    }
                }).to_string());
            }
        }

        // 回退到读取 /proc/loadavg 或使用其他方法
        self.get_load_average_fallback()
    }

    /// 获取内存使用情况
    ///
    /// 返回系统内存总量、已用、可用信息
    ///
    /// ## 返回
    /// JSON 格式的内存信息（单位：MB）
    pub fn get_memory_info(&self) -> Result<String, String> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;

            let meminfo = fs::read_to_string("/proc/meminfo")
                .map_err(|e| format!("读取内存信息失败：{}", e))?;

            let mut total = 0u64;
            let mut free = 0u64;
            let mut available: Option<u64> = None;
            let mut buffers: Option<u64> = None;
            let mut cached: Option<u64> = None;

            for line in meminfo.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let value = parts[1].parse::<u64>().unwrap_or(0);
                    match parts[0] {
                        "MemTotal:" => total = value,
                        "MemFree:" => free = value,
                        "MemAvailable:" => available = Some(value),
                        "Buffers:" => buffers = Some(value),
                        "Cached:" => cached = Some(value),
                        _ => {}
                    }
                }
            }

            Ok(json!({
                "success": true,
                "data": {
                    "total_mb": total / 1024,
                    "free_mb": free / 1024,
                    "available_mb": available.map(|v| v / 1024),
                    "buffers_mb": buffers.map(|v| v / 1024),
                    "cached_mb": cached.map(|v| v / 1024),
                    "used_mb": (total - free) / 1024,
                }
            }).to_string())
        }

        #[cfg(target_os = "macos")]
        {
            let total = std::process::Command::new("sysctl")
                .arg("-n")
                .arg("hw.memsize")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .and_then(|s| s.trim().parse::<u64>().ok())
                .map(|b| b / (1024 * 1024))
                .unwrap_or(0);

            Ok(json!({
                "success": true,
                "data": {
                    "total_mb": total,
                    "note": "macOS 详细内存信息需要额外计算"
                }
            }).to_string())
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Err("当前平台不支持内存查询".to_string())
        }
    }

    /// 获取磁盘使用情况
    ///
    /// 返回指定挂载点的磁盘使用信息
    ///
    /// ## 参数
    /// - `mount_point`: 挂载点路径，默认为 "/"
    ///
    /// ## 返回
    /// JSON 格式的磁盘信息（单位：GB）
    pub fn get_disk_usage(&self, mount_point: Option<String>) -> Result<String, String> {
        let mount = mount_point.unwrap_or_else(|| "/".to_string());

        // 验证路径合法性
        if !mount.starts_with('/') {
            return Err("挂载点必须是绝对路径".to_string());
        }

        let output = std::process::Command::new("df")
            .args(["-h", &mount])
            .output()
            .map_err(|e| format!("执行 df 命令失败：{}", e))?;

        if !output.status.success() {
            return Err(format!("无法获取挂载点 '{}' 的信息", mount));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        if lines.len() < 2 {
            return Err("磁盘信息格式异常".to_string());
        }

        let parts: Vec<&str> = lines[1].split_whitespace().collect();
        if parts.len() < 5 {
            return Err("磁盘信息列数不足".to_string());
        }

        let parse_size = |s: &str| -> f64 {
            let s = s.trim();
            if let Some(num) = s.strip_suffix('G') {
                num.parse().unwrap_or(0.0)
            } else if let Some(num) = s.strip_suffix('M') {
                num.parse::<f64>().unwrap_or(0.0) / 1024.0
            } else if let Some(num) = s.strip_suffix('K') {
                num.parse::<f64>().unwrap_or(0.0) / (1024.0 * 1024.0)
            } else {
                s.parse().unwrap_or(0.0)
            }
        };

        Ok(json!({
            "success": true,
            "data": {
                "mount_point": mount,
                "total_gb": parse_size(parts[1]),
                "used_gb": parse_size(parts[2]),
                "available_gb": parse_size(parts[3]),
                "usage_percent": parts[4].trim_end_matches('%').parse::<f32>().unwrap_or(0.0),
            }
        }).to_string())
    }

    /// 获取系统运行时间
    ///
    /// 返回系统自启动以来的运行时间
    ///
    /// ## 返回
    /// JSON 格式的运行时间（秒和人类可读格式）
    pub fn get_uptime(&self) -> Result<String, String> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;

            let uptime = fs::read_to_string("/proc/uptime")
                .map_err(|e| format!("读取运行时间失败：{}", e))?;

            let secs = uptime.split_whitespace()
                .next()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);

            Ok(json!({
                "success": true,
                "data": {
                    "secs": secs as u64,
                    "human": format_duration(secs as u64),
                }
            }).to_string())
        }

        #[cfg(target_os = "macos")]
        {
            let output = std::process::Command::new("uptime")
                .output()
                .map_err(|e| format!("执行 uptime 命令失败：{}", e))?;

            let uptime_str = String::from_utf8_lossy(&output.stdout);

            Ok(json!({
                "success": true,
                "data": {
                    "raw": uptime_str.trim().to_string(),
                    "note": "macOS 精确运行时间需要解析 boottime 结构"
                }
            }).to_string())
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Err("当前平台不支持运行时间查询".to_string())
        }
    }

    /// 列出可用命令（前 N 个）
    ///
    /// 扫描 PATH 环境变量中的可执行文件
    ///
    /// ## 参数
    /// - `limit`: 返回的最大命令数，默认 50，最大 500
    ///
    /// ## 返回
    /// JSON 格式的命令列表
    ///
    /// ## 性能
    /// - 典型延迟：50-200ms（取决于 PATH 中的目录数）
    pub fn list_available_commands(&self, limit: Option<usize>) -> Result<String, String> {
        let limit = limit.unwrap_or(config::DEFAULT_COMMANDS_LIMIT).min(config::MAX_COMMANDS_LIMIT);

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
            "success": true,
            "data": {
                "commands": commands,
                "total": commands.len(),
                "limit": limit,
            }
        }).to_string())
    }

    /// 检查命令是否可用
    ///
    /// 使用 which 命令检查指定命令是否在 PATH 中
    ///
    /// ## 参数
    /// - `command`: 命令名称
    ///
    /// ## 返回
    /// JSON 格式：`{"success": true, "data": {"available": bool, "path": "..."}}`
    pub fn check_command(&self, command: String) -> Result<String, String> {
        // 验证命令名（防止注入）
        if command.is_empty() || command.len() > config::MAX_PATTERN_LENGTH {
            return Err("无效的命令名称".to_string());
        }

        if command.contains(['/', '\\', ' ', ';', '|']) {
            return Err("命令名称包含非法字符".to_string());
        }

        let output = std::process::Command::new("which")
            .arg(&command)
            .output()
            .map_err(|e| format!("检查命令失败：{}", e))?;

        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(json!({
                "success": true,
                "data": {
                    "available": true,
                    "path": path,
                    "command": command,
                }
            }).to_string())
        } else {
            Ok(json!({
                "success": true,
                "data": {
                    "available": false,
                    "command": command,
                    "message": "命令未找到",
                }
            }).to_string())
        }
    }

    /// 获取工具元数据
    ///
    /// 返回工具的名称、描述、版本等信息
    pub fn get_metadata(&self) -> Result<String, String> {
        Ok(json!({
            "success": true,
            "data": {
                "name": config::SYSTEM_MONITOR_METADATA.name,
                "description": config::SYSTEM_MONITOR_METADATA.description,
                "version": config::SYSTEM_MONITOR_METADATA.version,
            }
        }).to_string())
    }
}

/// 格式化运行时间为人类可读格式
#[allow(dead_code)]
fn format_duration(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    let remaining_secs = secs % 60;

    if days > 0 {
        format!("{} 天 {} 小时 {} 分钟 {} 秒", days, hours, mins, remaining_secs)
    } else if hours > 0 {
        format!("{} 小时 {} 分钟 {} 秒", hours, mins, remaining_secs)
    } else if mins > 0 {
        format!("{} 分钟 {} 秒", mins, remaining_secs)
    } else {
        format!("{} 秒", remaining_secs)
    }
}

/// 检查文件是否可执行（Unix）
#[cfg(unix)]
fn is_executable(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = std::fs::metadata(path) {
        let mode = metadata.permissions().mode();
        return mode & 0o111 != 0;
    }
    false
}

/// 检查文件是否可执行（Windows）
#[cfg(windows)]
fn is_executable(path: &std::path::Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        return ext_str == "exe" || ext_str == "bat" || ext_str == "cmd" || ext_str == "com";
    }
    false
}

/// 获取负载平均值的回退实现
impl SystemMonitor {
    fn get_load_average_fallback(&self) -> Result<String, String> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;

            let loadavg = fs::read_to_string("/proc/loadavg")
                .map_err(|e| format!("读取负载信息失败：{}", e))?;

            let parts: Vec<&str> = loadavg.split_whitespace().collect();
            if parts.len() >= 3 {
                return Ok(json!({
                    "success": true,
                    "data": {
                        "1m": parts[0].parse::<f64>().unwrap_or(0.0),
                        "5m": parts[1].parse::<f64>().unwrap_or(0.0),
                        "15m": parts[2].parse::<f64>().unwrap_or(0.0),
                    }
                }).to_string());
            }
        }

        Err("无法获取负载平均值".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_monitor_creation() {
        let monitor = SystemMonitor::new();
        assert!(true);
    }

    #[test]
    fn test_get_system_resources() {
        let monitor = SystemMonitor::new();
        let result = monitor.get_system_resources();

        assert!(result.is_ok());
        let output: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output["success"], true);
        assert!(output["data"]["cpu"].is_object());
        assert!(output["data"]["memory"].is_object());
        assert!(output["data"]["disk"].is_object());
    }

    #[test]
    fn test_get_system_info() {
        let monitor = SystemMonitor::new();
        let result = monitor.get_system_info();

        assert!(result.is_ok());
        let output: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output["success"], true);
        assert!(output["data"]["os"].is_string());
        assert!(output["data"]["arch"].is_string());
    }

    #[test]
    fn test_get_cpu_cores() {
        let monitor = SystemMonitor::new();
        let result = monitor.get_cpu_cores();

        assert!(result.is_ok());
        let output: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output["success"], true);
        assert!(output["data"]["cores"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0), "0 秒");
        assert_eq!(format_duration(60), "1 分钟 0 秒");
        assert_eq!(format_duration(3661), "1 小时 1 分钟 1 秒");
        assert_eq!(format_duration(90061), "1 天 1 小时 1 分钟 1 秒");
    }

    #[test]
    fn test_check_command() {
        let monitor = SystemMonitor::new();

        // 测试存在的命令
        let result = monitor.check_command("ls".to_string());
        assert!(result.is_ok());
        let output: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output["success"], true);

        // 测试不存在的命令
        let result = monitor.check_command("nonexistent_command_xyz".to_string());
        assert!(result.is_ok());
        let output: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output["data"]["available"], false);
    }

    #[test]
    fn test_check_command_invalid() {
        let monitor = SystemMonitor::new();

        // 测试空命令
        assert!(monitor.check_command("".to_string()).is_err());

        // 测试包含非法字符的命令
        assert!(monitor.check_command("ls /; rm -rf".to_string()).is_err());
        assert!(monitor.check_command("cat /etc/passwd".to_string()).is_err());
    }

    #[test]
    fn test_get_metadata() {
        let monitor = SystemMonitor::new();
        let result = monitor.get_metadata();

        assert!(result.is_ok());
        let output: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output["success"], true);
        assert_eq!(output["data"]["name"], "system_monitor");
    }
}
