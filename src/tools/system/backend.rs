//! 平台后端抽象 trait
//!
//! 消除平台条件编译重复代码，提供统一的进程/系统信息接口
//!
//! ## 架构说明
//! - Trait 提供默认实现，减少平台间重复代码
//! - 平台特定逻辑在各自后端实现中覆盖
//! - 使用编译期选择后端，零运行时开销

use crate::tools::system::error::{ProcessError, SystemInfoError, ToErrorString};
use std::process::Command;
use std::io;

/// 进程信息结构体
///
/// 封装进程相关信息，提供访问器方法
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pid: u32,
    ppid: u32,
    user: String,
    cpu_percent: f32,
    mem_percent: f32,
    vsz: u64,
    rss: u64,
    tty: String,
    stat: String,
    start: String,
    time: String,
    comm: String,
    args: String,
}

#[allow(clippy::too_many_arguments)]
impl ProcessInfo {
    pub fn new(
        pid: u32,
        ppid: u32,
        user: String,
        cpu_percent: f32,
        mem_percent: f32,
        vsz: u64,
        rss: u64,
        tty: String,
        stat: String,
        start: String,
        time: String,
        comm: String,
        args: String,
    ) -> Self {
        Self {
            pid,
            ppid,
            user,
            cpu_percent,
            mem_percent,
            vsz,
            rss,
            tty,
            stat,
            start,
            time,
            comm,
            args,
        }
    }

    // 访问器方法
    #[allow(dead_code)]
    pub fn pid(&self) -> u32 { self.pid }
    #[allow(dead_code)]
    pub fn ppid(&self) -> u32 { self.ppid }
    #[allow(dead_code)]
    pub fn user(&self) -> &str { &self.user }
    #[allow(dead_code)]
    pub fn cpu_percent(&self) -> f32 { self.cpu_percent }
    #[allow(dead_code)]
    pub fn mem_percent(&self) -> f32 { self.mem_percent }
    #[allow(dead_code)]
    pub fn vsz(&self) -> u64 { self.vsz }
    #[allow(dead_code)]
    pub fn rss(&self) -> u64 { self.rss }
    #[allow(dead_code)]
    pub fn tty(&self) -> &str { &self.tty }
    #[allow(dead_code)]
    pub fn stat(&self) -> &str { &self.stat }
    #[allow(dead_code)]
    pub fn start(&self) -> &str { &self.start }
    #[allow(dead_code)]
    pub fn time(&self) -> &str { &self.time }
    #[allow(dead_code)]
    pub fn comm(&self) -> &str { &self.comm }
    #[allow(dead_code)]
    pub fn args(&self) -> &str { &self.args }

    /// 转换为 JSON 友好的结构
    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "pid": self.pid,
            "ppid": self.ppid,
            "user": self.user,
            "cpu_percent": self.cpu_percent,
            "mem_percent": self.mem_percent,
            "vsz": self.vsz,
            "rss": self.rss,
            "tty": self.tty,
            "stat": self.stat,
            "start": self.start,
            "time": self.time,
            "comm": self.comm,
            "args": self.args,
        })
    }

    /// 转换为简化版 JSON（用于列表显示）
    pub fn to_summary_json(&self) -> serde_json::Value {
        serde_json::json!({
            "pid": self.pid,
            "comm": self.comm,
            "cpu_percent": self.cpu_percent,
            "mem_percent": self.mem_percent,
            "args": self.args,
        })
    }
}

/// 系统资源信息结构体
#[derive(Debug, Clone)]
pub struct SystemResourceInfo {
    pub cpu_cores: u32,
    pub load_avg_1m: Option<f64>,
    pub load_avg_5m: Option<f64>,
    pub load_avg_15m: Option<f64>,
    pub mem_total_kb: u64,
    pub mem_free_kb: u64,
    pub mem_available_kb: Option<u64>,
    pub disk_usage: DiskUsageInfo,
    pub uptime_secs: Option<u64>,
}

/// 磁盘使用信息
#[derive(Debug, Clone)]
pub struct DiskUsageInfo {
    pub total_gb: f64,
    pub used_gb: f64,
    pub available_gb: f64,
    pub usage_percent: f32,
}

/// 平台后端 trait
///
/// 提供默认实现减少重复代码，平台特定逻辑由实现方覆盖
pub trait ProcessBackend: Send + Sync {
    /// 列出进程（按 CPU 使用率排序）
    ///
    /// 默认实现：执行 `ps aux` 并解析输出
    fn list_processes(&self, limit: usize) -> Result<Vec<ProcessInfo>, ProcessError> {
        let output = self.run_ps_command(&["aux"])?;
        let mut processes = parse_ps_output(&output, usize::MAX)?;
        
        // 按 CPU 使用率降序排序
        processes.sort_by(|a, b| {
            b.cpu_percent().partial_cmp(&a.cpu_percent())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        
        processes.truncate(limit);
        Ok(processes)
    }

    /// 获取进程详细信息
    ///
    /// 默认实现：单次 ps 调用获取所有信息，避免 TOCTOU
    fn get_process_info(&self, pid: u32) -> Result<ProcessInfo, ProcessError> {
        let output = self.run_ps_command(&[
            "-p", &pid.to_string(),
            "-o", "pid,ppid,user,%cpu,%mem,vsz,rss,tty,stat,start,time,comm,args"
        ])?;

        let lines: Vec<&str> = output.lines().collect();
        if lines.len() < 2 {
            return Err(ProcessError::NotFound(pid));
        }

        parse_process_line(lines[1])
            .map_err(|e| ProcessError::ParseFailed(format!("解析进程信息失败：{}", e)))
    }

    /// 搜索进程（按名称）
    ///
    /// 默认实现：使用 pgrep -a -f
    fn search_processes(&self, name: &str, limit: usize) -> Result<Vec<ProcessInfo>, ProcessError> {
        let output = Command::new("pgrep")
            .args(["-a", "-f", name])
            .output()
            .map_err(|e| ProcessError::CommandFailed(format!("执行 pgrep 命令失败：{}", e)))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| ProcessError::ParseFailed(e.to_error_string()))?;

        // pgrep 输出格式：PID COMMAND
        let mut processes = Vec::new();
        for line in stdout.lines().take(limit) {
            if let Some((pid_str, cmd)) = line.split_once(' ') {
                if let Ok(pid) = pid_str.parse::<u32>() {
                    processes.push(ProcessInfo::new(
                        pid,
                        0,
                        String::new(),
                        0.0,
                        0.0,
                        0,
                        0,
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        cmd.to_string(),
                    ));
                }
            }
        }

        Ok(processes)
    }

    /// 获取进程打开的文件描述符
    fn get_process_files(&self, pid: u32, limit: usize) -> Result<Vec<String>, ProcessError>;

    /// 获取进程环境变量
    fn get_process_env(&self, pid: u32) -> Result<Vec<String>, ProcessError>;

    /// 获取系统资源使用情况
    fn get_system_resources(&self) -> Result<SystemResourceInfo, SystemInfoError>;

    /// 检查进程是否存在（用于快速验证）
    #[allow(dead_code)]
    fn process_exists(&self, pid: u32) -> bool;

    /// 检查进程所有权（用于权限验证）
    #[allow(dead_code)]
    fn check_process_ownership(&self, pid: u32) -> Result<(), ProcessError>;

    /// 运行 ps 命令的辅助方法（内部使用）
    fn run_ps_command(&self, args: &[&str]) -> Result<String, ProcessError> {
        let output = Command::new("ps")
            .args(args)
            .output()
            .map_err(|e| ProcessError::CommandFailed(format!("执行 ps 命令失败：{}", e)))?;

        if !output.status.success() {
            return Err(ProcessError::CommandFailed("获取进程列表失败".to_string()));
        }

        String::from_utf8(output.stdout)
            .map_err(|e| ProcessError::ParseFailed(e.to_error_string()))
    }
}

/// Linux 平台后端实现
#[allow(dead_code)]
pub struct LinuxBackend;

impl ProcessBackend for LinuxBackend {
    fn get_process_files(&self, pid: u32, limit: usize) -> Result<Vec<String>, ProcessError> {
        use std::fs;
        use std::path::Path;

        let fd_path = format!("/proc/{}/fd", pid);

        if !Path::new(&fd_path).exists() {
            return Err(ProcessError::NotFound(pid));
        }

        let entries = fs::read_dir(&fd_path)
            .map_err(|e| ProcessError::CommandFailed(format!("读取文件描述符失败：{}", e)))?;

        let mut files = Vec::new();
        for e in entries.take(limit).flatten() {
            let fd_name: String = e.file_name().to_string_lossy().to_string();
            if let Ok(link) = fs::read_link(e.path()) {
                files.push(format!("{}: {}", fd_name, link.to_string_lossy()));
            }
        }

        Ok(files)
    }

    fn get_process_env(&self, pid: u32) -> Result<Vec<String>, ProcessError> {
        use std::fs;

        let env_path = format!("/proc/{}/environ", pid);
        let content = fs::read(&env_path)
            .map_err(|e| {
                if e.kind() == io::ErrorKind::PermissionDenied {
                    ProcessError::PermissionDenied(pid, "无权限读取环境变量".to_string())
                } else {
                    ProcessError::CommandFailed(format!("读取环境变量失败：{}", e))
                }
            })?;

        let vars: Vec<&str> = content
            .split(|&b| b == 0)
            .filter_map(|s| std::str::from_utf8(s).ok())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(vars.iter().map(|s| s.to_string()).collect())
    }

    fn get_system_resources(&self) -> Result<SystemResourceInfo, SystemInfoError> {
        use std::fs;

        // CPU 核心数
        let cpu_cores = fs::read_to_string("/proc/cpuinfo")
            .map(|c| c.matches("processor").count() as u32)
            .unwrap_or(0);

        // 负载平均值
        let (load_1m, load_5m, load_15m) = fs::read_to_string("/proc/loadavg")
            .map(|c| {
                let parts: Vec<&str> = c.split_whitespace().collect();
                if parts.len() >= 3 {
                    (
                        parts[0].parse().ok(),
                        parts[1].parse().ok(),
                        parts[2].parse().ok(),
                    )
                } else {
                    (None, None, None)
                }
            })
            .unwrap_or((None, None, None));

        // 内存信息
        let (mem_total, mem_free, mem_available) = fs::read_to_string("/proc/meminfo")
            .map(|c| {
                let mut total = 0u64;
                let mut free = 0u64;
                let mut available: Option<u64> = None;

                for line in c.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let value = parts[1].parse::<u64>().unwrap_or(0);
                        match parts[0] {
                            "MemTotal:" => total = value,
                            "MemFree:" => free = value,
                            "MemAvailable:" => available = Some(value),
                            _ => {}
                        }
                    }
                }

                (total, free, available)
            })
            .unwrap_or((0, 0, None));

        // 磁盘使用
        let disk_usage = get_disk_usage("/")?;

        // 运行时间
        let uptime_secs = fs::read_to_string("/proc/uptime")
            .ok()
            .and_then(|c| {
                let first = c.split_whitespace().next()?.to_string();
                first.parse::<f64>().ok().map(|s| s as u64)
            });

        Ok(SystemResourceInfo {
            cpu_cores,
            load_avg_1m: load_1m,
            load_avg_5m: load_5m,
            load_avg_15m: load_15m,
            mem_total_kb: mem_total,
            mem_free_kb: mem_free,
            mem_available_kb: mem_available,
            disk_usage,
            uptime_secs,
        })
    }

    fn process_exists(&self, pid: u32) -> bool {
        std::path::Path::new(&format!("/proc/{}", pid)).exists()
    }

    fn check_process_ownership(&self, pid: u32) -> Result<(), ProcessError> {
        use std::os::unix::fs::MetadataExt;
        use std::fs;

        let proc_exe_path = format!("/proc/{}/exe", pid);

        if let Ok(metadata) = fs::metadata(&proc_exe_path) {
            let current_uid = unsafe { libc::getuid() };
            let process_uid = metadata.uid();

            if current_uid != 0 && process_uid != current_uid {
                return Err(ProcessError::PermissionDenied(
                    pid,
                    format!("进程属于 UID {}，当前用户 UID {}", process_uid, current_uid)
                ));
            }
        }

        Ok(())
    }
}

/// macOS 平台后端实现
pub struct MacOSBackend;

impl ProcessBackend for MacOSBackend {
    // macOS 需要覆盖 list_processes 因为 ps aux 不支持 --sort
    fn list_processes(&self, limit: usize) -> Result<Vec<ProcessInfo>, ProcessError> {
        let output = self.run_ps_command(&["aux"])?;
        let mut processes = parse_ps_output(&output, usize::MAX)?;

        // 按 CPU 使用率降序排序
        processes.sort_by(|a, b| {
            b.cpu_percent().partial_cmp(&a.cpu_percent())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        processes.truncate(limit);
        Ok(processes)
    }

    fn get_process_files(&self, pid: u32, limit: usize) -> Result<Vec<String>, ProcessError> {
        let output = Command::new("lsof")
            .args(["-p", &pid.to_string()])
            .output()
            .map_err(|e| ProcessError::CommandFailed(format!("执行 lsof 命令失败：{}", e)))?;

        if !output.status.success() {
            return Err(ProcessError::NotFound(pid));
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| ProcessError::ParseFailed(e.to_error_string()))?;

        let lines: Vec<&str> = stdout.lines().skip(1).take(limit).collect();
        Ok(lines.iter().map(|s| s.to_string()).collect())
    }

    fn get_process_env(&self, pid: u32) -> Result<Vec<String>, ProcessError> {
        let output = Command::new("sh")
            .args(["-c", &format!("ps eww {} | head -1", pid)])
            .output()
            .map_err(|e| ProcessError::CommandFailed(format!("获取环境变量失败：{}", e)))?;

        if !output.status.success() {
            return Err(ProcessError::PermissionDenied(pid, "无法获取环境变量".to_string()));
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| ProcessError::ParseFailed(e.to_error_string()))?;

        // 解析环境变量（格式为 VAR=value）
        let vars: Vec<&str> = stdout
            .split_whitespace()
            .filter(|s| s.contains('='))
            .collect();

        Ok(vars.iter().map(|s| s.to_string()).collect())
    }

    fn get_system_resources(&self) -> Result<SystemResourceInfo, SystemInfoError> {
        // CPU 核心数
        let cpu_cores = Command::new("sysctl")
            .arg("-n")
            .arg("hw.ncpu")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().parse().unwrap_or(0))
            .unwrap_or(0);

        // 负载平均值（macOS 没有 /proc/loadavg，使用 sysctl）
        let (load_1m, load_5m, load_15m) = Command::new("sysctl")
            .arg("vm.loadavg")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| {
                // 输出格式：{ vm.loadavg: { 1m: 1.23 5m: 1.45 15m: 1.67 } }
                let parts: Vec<&str> = s.split_whitespace().collect();
                if parts.len() >= 8 {
                    let load_1m = parts[2].trim_end_matches(',').parse::<f64>().ok();
                    let load_5m = parts[4].trim_end_matches(',').parse::<f64>().ok();
                    let load_15m = parts[6].trim_end_matches('}').parse::<f64>().ok();
                    (load_1m, load_5m, load_15m)
                } else {
                    (None, None, None)
                }
            })
            .unwrap_or((None, None, None));

        // 内存信息
        let (mem_total, mem_free, mem_available) = get_macos_memory_info()
            .unwrap_or((0, 0, None));

        // 磁盘使用
        let disk_usage = get_disk_usage("/")?;

        // 运行时间
        let uptime_secs = get_macos_uptime();

        Ok(SystemResourceInfo {
            cpu_cores,
            load_avg_1m: load_1m,
            load_avg_5m: load_5m,
            load_avg_15m: load_15m,
            mem_total_kb: mem_total,
            mem_free_kb: mem_free,
            mem_available_kb: mem_available,
            disk_usage,
            uptime_secs,
        })
    }

    fn process_exists(&self, pid: u32) -> bool {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn check_process_ownership(&self, pid: u32) -> Result<(), ProcessError> {
        let output = Command::new("ps")
            .args(["-o", "uid=", "-p", &pid.to_string()])
            .output()
            .map_err(|e| ProcessError::CommandFailed(format!("获取进程用户失败：{}", e)))?;

        if !output.status.success() {
            return Err(ProcessError::NotFound(pid));
        }

        let process_uid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let current_uid = unsafe { libc::getuid() };

        if let Ok(process_uid) = process_uid_str.parse::<u32>() {
            if current_uid != 0 && process_uid != current_uid {
                return Err(ProcessError::PermissionDenied(
                    pid,
                    format!("进程属于 UID {}，当前用户 UID {}", process_uid, current_uid)
                ));
            }
        } else {
            return Err(ProcessError::ParseFailed(format!("无法解析进程 UID: {}", process_uid_str)));
        }

        Ok(())
    }
}

/// 获取磁盘使用信息
fn get_disk_usage(mount_point: &str) -> Result<DiskUsageInfo, SystemInfoError> {
    let output = Command::new("df")
        .args(["-h", mount_point])
        .output()
        .map_err(|e| SystemInfoError::InfoFetchFailed(format!("执行 df 命令失败：{}", e)))?;

    if !output.status.success() {
        return Err(SystemInfoError::InfoFetchFailed("获取磁盘信息失败".to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    if lines.len() < 2 {
        return Err(SystemInfoError::ParseFailed("磁盘信息格式异常".to_string()));
    }

    // 解析 df 输出：Filesystem Size Used Avail Capacity Mounted
    let parts: Vec<&str> = lines[1].split_whitespace().collect();
    if parts.len() < 5 {
        return Err(SystemInfoError::ParseFailed("磁盘信息列数不足".to_string()));
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

    let total_gb = parse_size(parts[1]);
    let used_gb = parse_size(parts[2]);
    let available_gb = parse_size(parts[3]);

    let usage_percent = parts[4]
        .trim_end_matches('%')
        .parse::<f32>()
        .unwrap_or(0.0);

    Ok(DiskUsageInfo {
        total_gb,
        used_gb,
        available_gb,
        usage_percent,
    })
}

/// 获取 macOS 内存信息
fn get_macos_memory_info() -> Option<(u64, u64, Option<u64>)> {
    use std::process::Command;

    // 总内存
    let total = Command::new("sysctl")
        .arg("-n")
        .arg("hw.memsize")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(|b| b / 1024)?; // 转换为 KB

    // 空闲内存（通过 vm_stat 计算）
    let free = Command::new("vm_stat")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| {
            let page_size = 4096u64; // macOS 默认页大小
            for line in s.lines() {
                if line.starts_with("Pages free") {
                    if let Some(val) = line.split(':').nth(1) {
                        if let Ok(pages) = val.trim().trim_end_matches('.').parse::<u64>() {
                            return Some((pages * page_size) / 1024);
                        }
                    }
                }
            }
            None
        })
        .unwrap_or(0);

    Some((total, free, None))
}

/// 获取 macOS 运行时间
fn get_macos_uptime() -> Option<u64> {
    Command::new("uptime")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| parse_uptime(&s))
}

/// 解析 ps aux 输出
fn parse_ps_output(output: &str, limit: usize) -> Result<Vec<ProcessInfo>, ProcessError> {
    let lines: Vec<&str> = output.lines().collect();
    let mut processes = Vec::new();

    // 跳过表头
    for line in lines.iter().skip(1).take(limit) {
        if let Ok(info) = parse_process_line(line) {
            processes.push(info);
        }
    }

    Ok(processes)
}

/// 解析单行 ps 输出
fn parse_process_line(line: &str) -> Result<ProcessInfo, String> {
    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.len() < 11 {
        return Err(format!("ps 输出列数不足：{}", parts.len()));
    }

    Ok(ProcessInfo::new(
        parts[0].parse().map_err(|_| "PID 解析失败")?,
        parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
        parts.get(2).map(|s| s.to_string()).unwrap_or_default(),
        parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.0),
        parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.0),
        parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(0),
        parts.get(6).and_then(|s| s.parse().ok()).unwrap_or(0),
        parts.get(7).map(|s| s.to_string()).unwrap_or_default(),
        parts.get(8).map(|s| s.to_string()).unwrap_or_default(),
        parts.get(9).map(|s| s.to_string()).unwrap_or_default(),
        parts.get(10).map(|s| s.to_string()).unwrap_or_default(),
        parts.get(11).map(|s| s.to_string()).unwrap_or_default(),
        parts.iter().skip(11).copied().collect::<Vec<_>>().join(" "),
    ))
}

/// 解析 uptime 输出获取运行时间（秒）
fn parse_uptime(uptime_output: &str) -> Option<u64> {
    // 示例：12:34  up 1 day,  2:34,  1 user,  load averages: 1.23 1.45 1.67
    // 或： 12:34  up  1:23,  1 user,  load averages: 1.23 1.45 1.67
    // 或： 12:34  up 2 days,  3:45,  1 user,  load averages: 1.23
    for part in uptime_output.split(',') {
        let part = part.trim();
        // 查找 "up" 关键字
        if let Some(up_idx) = part.find("up") {
            // 获取 "up" 之后的部分
            let time_part = part[up_idx + 2..].trim();
            // 尝试解析 "X days" 或 "X day" 或 "HH:MM"
            if time_part.contains("day") || time_part.contains("days") {
                // 提取天数
                let days_str = time_part.split_whitespace().next()?;
                let days: u64 = days_str.parse().ok()?;
                return Some(days * 24 * 3600);
            } else {
                // HH:MM 格式
                if let Some(secs) = parse_time_string(time_part) {
                    return Some(secs);
                }
            }
        }
    }
    None
}

/// 解析时间字符串（HH:MM 或 H:MM 格式）
fn parse_time_string(time_str: &str) -> Option<u64> {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() == 2 {
        let hours: u64 = parts[0].parse().ok()?;
        let mins: u64 = parts[1].parse().ok()?;
        return Some(hours * 3600 + mins * 60);
    }
    None
}

/// 根据当前平台创建后端实例
pub fn create_backend() -> Box<dyn ProcessBackend> {
    #[cfg(target_os = "linux")]
    {
        Box::new(LinuxBackend)
    }

    #[cfg(target_os = "macos")]
    {
        Box::new(MacOSBackend)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        panic!("不支持的操作系统")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_info_accessors() {
        let info = ProcessInfo::new(
            1234, 1, "user".to_string(),
            1.5, 2.0, 1000000, 500000,
            "ttys000".to_string(), "S".to_string(),
            "10:00".to_string(), "0:01".to_string(),
            "bash".to_string(), "/bin/bash".to_string(),
        );

        assert_eq!(info.pid(), 1234);
        assert_eq!(info.ppid(), 1);
        assert_eq!(info.user(), "user");
        assert!((info.cpu_percent() - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_process_info_to_json() {
        let info = ProcessInfo::new(
            1234, 1, "user".to_string(),
            1.5, 2.0, 1000000, 500000,
            "ttys000".to_string(), "S".to_string(),
            "10:00".to_string(), "0:01".to_string(),
            "bash".to_string(), "/bin/bash".to_string(),
        );

        let json = info.to_json_value();
        assert_eq!(json["pid"], 1234);
        assert_eq!(json["comm"], "bash");
    }

    #[test]
    fn test_parse_process_line() {
        let line = "  1234     1 user   1.5  2.0 1000000 500000 ttys000  S    10:00   0:01 bash /bin/bash";
        let result = parse_process_line(line);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.pid(), 1234);
        assert_eq!(info.comm(), "bash");
    }

    #[test]
    fn test_parse_uptime_days() {
        let uptime = "12:34  up 2 days,  3:45,  1 user,  load averages: 1.23";
        assert_eq!(parse_uptime(uptime), Some(2 * 24 * 3600));
    }

    #[test]
    fn test_parse_uptime_hours() {
        let uptime = "12:34  up  1:23,  1 user,  load averages: 1.23";
        assert_eq!(parse_uptime(uptime), Some(1 * 3600 + 23 * 60));
    }
}
