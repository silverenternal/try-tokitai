use tokitai::tool;
use std::process::{Command, Stdio};
use std::path::Path;

/// 验证搜索模式
fn validate_search_pattern(pattern: &str) -> Result<(), String> {
    const MAX_PATTERN_LENGTH: usize = 1024;
    
    if pattern.len() > MAX_PATTERN_LENGTH {
        return Err(format!(
            "搜索模式过长 ({} > {} 字符)",
            pattern.len(),
            MAX_PATTERN_LENGTH
        ));
    }
    Ok(())
}

/// 进程管理工具集
/// 提供查看和管理系统进程的功能
pub struct ProcessTools;

// PID 最大值限制（Linux 通常为 2^22 = 4194304）
const MAX_PID: u32 = 4194304;

#[tool]
impl ProcessTools {
    /// 列出当前运行的进程
    /// 显示进程名、PID、CPU 使用率等信息
    pub fn list_processes(&self, limit: Option<usize>) -> Result<String, String> {
        let limit = limit.unwrap_or(20).min(100); // 限制最大 100 个

        // 使用 ps 命令获取进程列表
        let output = Command::new("ps")
            .args(["aux", "--sort=-%cpu"])
            .stdin(Stdio::null())
            .output()
            .map_err(|e| format!("执行 ps 命令失败：{}", e))?;

        if !output.status.success() {
            return Err("获取进程列表失败".to_string());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // 保留表头 + limit 个进程
        let header = lines.first().copied().unwrap_or("");
        let processes: Vec<&str> = lines.iter().skip(1).take(limit).copied().collect();

        let mut result = format!("📊 进程列表 (前 {} 个按 CPU 排序)\n\n", limit);
        result.push_str(&format!("{}\n", header));
        result.push_str(&"-".repeat(80));
        result.push('\n');

        for proc in processes {
            result.push_str(&format!("{}\n", proc));
        }

        Ok(result)
    }

    /// 获取进程详细信息
    /// 查看指定 PID 的进程详情
    pub fn get_process_info(&self, pid: u32) -> Result<String, String> {
        validate_pid(pid)?;
        verify_process_exists(pid)?;
        verify_process_ownership(pid)?;
        
        // 使用 ps 获取详细信息
        let output = Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", "pid,ppid,user,%cpu,%mem,vsz,rss,tty,stat,start,time,comm,args"])
            .stdin(Stdio::null())
            .output()
            .map_err(|e| format!("执行 ps 命令失败：{}", e))?;

        if !output.status.success() {
            return Err(format!("未找到进程 PID: {}", pid));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        if lines.len() < 2 {
            return Err(format!("未找到进程 PID: {}", pid));
        }

        let header = lines[0];
        let info = lines[1];

        // 尝试读取 /proc/[pid]/status 获取更多信息（仅限自己的进程）
        let mut extra_info = String::new();
        let status_path = format!("/proc/{}/status", pid);
        if let Ok(status) = std::fs::read_to_string(&status_path) {
            for line in status.lines().take(15) {
                extra_info.push_str(&format!("  {}\n", line));
            }
        }

        Ok(format!(
            "📋 进程信息 (PID: {})\n\n{}\n{}\n\n📁 详细信息:\n{}",
            pid, header, info, extra_info
        ))
    }

    /// 按名称搜索进程
    /// 查找所有匹配的进程
    pub fn search_processes(&self, name: String, limit: Option<usize>) -> Result<String, String> {
        validate_search_pattern(&name)?;
        
        let limit = limit.unwrap_or(20).min(100);

        let output = Command::new("pgrep")
            .args(["-a", "-f", &name])
            .stdin(Stdio::null())
            .output()
            .map_err(|e| format!("执行 pgrep 命令失败：{}", e))?;

        if !output.status.success() {
            return Ok(format!("未找到包含 '{}' 的进程", name));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().take(limit).collect();

        let mut result = format!("🔍 匹配的进程 (搜索：'{}')\n\n", name);
        result.push_str(&format!("找到 {} 个进程\n\n", lines.len()));

        for line in lines {
            result.push_str(&format!("  {}\n", line));
        }

        Ok(result)
    }

    /// 获取系统资源使用情况
    /// CPU、内存、负载等
    pub fn get_system_resources(&self) -> Result<String, String> {
        let mut result = String::from("📊 系统资源使用情况\n\n");

        // CPU 信息
        result.push_str("🔹 CPU 信息:\n");
        if let Ok(output) = Command::new("nproc").stdin(Stdio::null()).output() {
            let cores_str = String::from_utf8_lossy(&output.stdout);
            let cores = cores_str.trim();
            result.push_str(&format!("  CPU 核心数：{}\n", cores));
        }

        // 负载平均值
        if let Ok(loadavg) = std::fs::read_to_string("/proc/loadavg") {
            let parts: Vec<&str> = loadavg.split_whitespace().collect();
            if parts.len() >= 3 {
                result.push_str(&format!(
                    "  系统负载：{} (1 分钟), {} (5 分钟), {} (15 分钟)\n",
                    parts[0], parts[1], parts[2]
                ));
            }
        }

        // 内存信息
        result.push_str("\n🔹 内存信息:\n");
        if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
            for line in meminfo.lines().take(5) {
                result.push_str(&format!("  {}\n", line));
            }
        }

        // 磁盘空间
        result.push_str("\n🔹 磁盘使用:\n");
        if let Ok(output) = Command::new("df").args(["-h", "/"]).stdin(Stdio::null()).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = stdout.lines().collect();
            if lines.len() >= 2 {
                result.push_str(&format!("  {}\n", lines[1]));
            }
        }

        // 运行时间
        if let Ok(output) = Command::new("uptime").stdin(Stdio::null()).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            result.push_str(&format!("\n🔹 运行时间:\n  {}", stdout.trim()));
        }

        Ok(result)
    }

    /// 查看进程的打开文件
    /// 列出进程打开的文件描述符
    pub fn get_process_files(&self, pid: u32) -> Result<String, String> {
        validate_pid(pid)?;
        verify_process_exists(pid)?;
        verify_process_ownership(pid)?;
        
        let fd_path = format!("/proc/{}/fd", pid);

        if !Path::new(&fd_path).exists() {
            return Err(format!("进程 {} 不存在或无权限访问", pid));
        }

        let entries = std::fs::read_dir(&fd_path)
            .map_err(|e| format!("读取文件描述符失败：{}", e))?;

        let mut files = Vec::new();
        const MAX_FILES: usize = 100;

        for e in entries.take(MAX_FILES).flatten() {
            let fd_name = e.file_name().to_string_lossy().to_string();
            if let Ok(link) = std::fs::read_link(e.path()) {
                files.push(format!("  FD {}: {}", fd_name, link.to_string_lossy()));
            }
        }

        let truncated = files.len() >= MAX_FILES;
        
        Ok(format!(
            "📁 进程 {} 的打开文件 (共 {} 个{})\n\n{}",
            pid,
            files.len(),
            if truncated { "，仅显示前 100 个" } else { "" },
            files.join("\n")
        ))
    }

    /// 查看进程的环境变量
    /// 获取指定进程的环境变量（仅限自己的进程）
    pub fn get_process_env(&self, pid: u32) -> Result<String, String> {
        validate_pid(pid)?;
        verify_process_exists(pid)?;
        verify_process_ownership(pid)?;
        
        let env_path = format!("/proc/{}/environ", pid);

        let content = std::fs::read(&env_path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    format!("无权限读取进程 {} 的环境变量", pid)
                } else {
                    format!("读取环境变量失败：{}", e)
                }
            })?;

        let vars: Vec<&str> = content
            .split(|&b| b == 0)
            .filter_map(|s| std::str::from_utf8(s).ok())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(format!(
            "🔧 进程 {} 的环境变量 (共 {} 个)\n\n{}",
            pid,
            vars.len(),
            vars.join("\n")
        ))
    }
}

/// 验证 PID 是否有效
fn validate_pid(pid: u32) -> Result<(), String> {
    if pid == 0 {
        return Err("PID 0 无效（内核调度进程）".to_string());
    }
    
    if pid > MAX_PID {
        return Err(format!("PID {} 超出有效范围 (1-{})", pid, MAX_PID));
    }
    
    Ok(())
}

/// 验证进程是否存在
fn verify_process_exists(pid: u32) -> Result<(), String> {
    if !Path::new(&format!("/proc/{}", pid)).exists() {
        return Err(format!("进程 {} 不存在", pid));
    }
    Ok(())
}

/// 验证进程所有权（仅限自己的进程或 root）
fn verify_process_ownership(pid: u32) -> Result<(), String> {
    use std::os::unix::fs::MetadataExt;
    
    let proc_exe_path = format!("/proc/{}/exe", pid);
    
    if let Ok(metadata) = std::fs::metadata(&proc_exe_path) {
        let current_uid = unsafe { libc::getuid() };
        let process_uid = metadata.uid();
        
        // 如果不是 root 且进程不属于当前用户
        if current_uid != 0 && process_uid != current_uid {
            return Err(format!(
                "无权限访问其他用户的进程 {} (UID: {})",
                pid, process_uid
            ));
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_pid() {
        // PID 0 应该无效
        assert!(validate_pid(0).is_err());
        
        // 正常 PID 应该有效
        assert!(validate_pid(1).is_ok());
        assert!(validate_pid(1000).is_ok());
        assert!(validate_pid(MAX_PID).is_ok());
        
        // 超出范围的 PID 应该无效
        assert!(validate_pid(MAX_PID + 1).is_err());
    }

    #[test]
    fn test_verify_process_exists() {
        // PID 1 (init/systemd) 应该存在
        assert!(verify_process_exists(1).is_ok());
        
        // 非常大的 PID 应该不存在
        assert!(verify_process_exists(99999999).is_err());
    }

    #[test]
    fn test_get_system_resources() {
        let tools = ProcessTools;
        let result = tools.get_system_resources();
        
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("CPU 信息"));
        assert!(output.contains("内存信息"));
    }

    #[test]
    fn test_list_processes() {
        let tools = ProcessTools;
        let result = tools.list_processes(Some(5));
        
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("进程列表"));
        assert!(output.contains("CPU"));
    }

    #[test]
    fn test_search_processes() {
        let tools = ProcessTools;
        // 搜索 bash 或 sh 进程（通常存在）
        let result = tools.search_processes("bash".to_string(), Some(5));
        
        // 可能找到也可能找不到，但不应该出错
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_process_info_for_pid_1() {
        let tools = ProcessTools;
        
        // 尝试获取 PID 1 的信息（可能需要 root 权限）
        let result = tools.get_process_info(1);
        
        // 如果成功，验证输出格式
        if let Ok(output) = result {
            assert!(output.contains("PID: 1"));
        }
    }
}
