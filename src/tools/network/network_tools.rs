use once_cell::sync::Lazy;
use tokitai::tool;
use std::net::{TcpStream, UdpSocket, ToSocketAddrs};
use std::time::Duration;
use std::process::Command;

/// 网络工具集
/// 提供网络诊断和连接测试功能
pub struct NetworkTools;

// 复用 HTTP Client 用于查询公网 IP
static HTTP_CLIENT: Lazy<reqwest::blocking::Client> = Lazy::new(|| {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("创建 HTTP 客户端失败")
});

// 端口扫描速率限制（毫秒）
const PORT_SCAN_DELAY_MS: u64 = 100;

// 禁止扫描的内网地址段（防止滥用）
fn is_safe_target(host: &str) -> Result<(), String> {
    use std::net::IpAddr;
    
    // 允许 localhost 用于本地测试
    if host == "localhost" || host == "127.0.0.1" || host == "::1" {
        return Ok(());
    }
    
    // 尝试解析 IP 地址
    if let Ok(ip_addr) = host.parse::<IpAddr>() {
        return check_ip_safety(&ip_addr);
    }
    
    // 对于域名，尝试解析并检查
    if let Ok(addrs) = host.to_socket_addrs() {
        for addr in addrs {
            check_ip_safety(&addr.ip())?;
        }
    }
    
    Ok(())
}

// 检查 IP 地址是否安全
fn check_ip_safety(ip: &std::net::IpAddr) -> Result<(), String> {
    use std::net::IpAddr::{V4, V6};
    
    match ip {
        V4(ip4) => {
            // 允许回环地址
            if ip4.is_loopback() {
                return Ok(());
            }
            
            // 禁止扫描私有地址（防止内部网络扫描）
            if ip4.is_private() || ip4.is_link_local() || ip4.is_unspecified() {
                return Err(format!(
                    "禁止扫描内网地址：{} (安全限制)",
                    ip
                ));
            }
            
            // 禁止扫描 10.0.0.0/8
            let octets = ip4.octets();
            if octets[0] == 10 {
                return Err(format!(
                    "禁止扫描内网地址：{} (安全限制)",
                    ip
                ));
            }
        }
        V6(ip6) => {
            if ip6.is_loopback() {
                return Ok(());
            }
            
            if ip6.is_unique_local() || ip6.is_unspecified() {
                return Err(format!(
                    "禁止扫描内网地址：{} (安全限制)",
                    ip
                ));
            }
        }
    }
    Ok(())
}

/// 验证主机名长度
fn validate_host_length(host: &str) -> Result<(), String> {
    const MAX_HOST_LENGTH: usize = 256;
    
    if host.len() > MAX_HOST_LENGTH {
        return Err(format!(
            "主机名过长 ({} > {} 字符)",
            host.len(),
            MAX_HOST_LENGTH
        ));
    }
    Ok(())
}

#[tool]
impl NetworkTools {
    /// Ping 主机（测试连通性）
    /// 检查主机是否可达
    pub fn ping_host(&self, host: String, count: Option<u32>) -> Result<String, String> {
        validate_host_length(&host)?;
        is_safe_target(&host)?;
        
        let count = count.unwrap_or(4).min(10); // 限制最大 ping 次数

        let mut results = Vec::new();
        let mut success_count = 0;

        for i in 1..=count {
            let start = std::time::Instant::now();

            // 尝试 TCP 连接到常见端口
            let reachable = check_tcp_connect(&host, 80, Duration::from_secs(2)).is_ok()
                || check_tcp_connect(&host, 443, Duration::from_secs(2)).is_ok()
                || check_tcp_connect(&host, 22, Duration::from_secs(2)).is_ok();

            let elapsed = start.elapsed();

            if reachable {
                success_count += 1;
                results.push(format!("  请求 {}: 成功 (耗时 {:.2} ms)", i, elapsed.as_secs_f64() * 1000.0));
            } else {
                results.push(format!("  请求 {}: 超时", i));
            }
            
            // 添加速率限制
            if i < count {
                std::thread::sleep(Duration::from_millis(200));
            }
        }

        Ok(format!(
            "🏓 Ping 测试结果：{}\n\n{}\n\n成功率：{}/{} ({:.0}%)",
            host,
            results.join("\n"),
            success_count,
            count,
            (success_count as f64 / count as f64) * 100.0
        ))
    }

    /// 检查 TCP 端口是否开放
    /// 测试指定主机的 TCP 端口连通性
    pub fn check_tcp_port(&self, host: String, port: u16, timeout_secs: Option<u64>) -> Result<String, String> {
        validate_host_length(&host)?;
        is_safe_target(&host)?;
        
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(5).min(30));

        let start = std::time::Instant::now();
        let result = check_tcp_connect(&host, port, timeout);
        let elapsed = start.elapsed();

        match result {
            Ok(_) => Ok(format!(
                "✅ 端口开放\n\n主机：{}\n端口：{}\n响应时间：{:.2} ms\n状态：可连接",
                host, port, elapsed.as_secs_f64() * 1000.0
            )),
            Err(e) => Ok(format!(
                "❌ 端口关闭或不可达\n\n主机：{}\n端口：{}\n超时时间：{} 秒\n错误：{}",
                host, port, timeout_secs.unwrap_or(5), e
            )),
        }
    }

    /// 扫描常用端口
    /// 快速扫描主机的常用开放端口
    pub fn scan_common_ports(&self, host: String, timeout_secs: Option<u64>) -> Result<String, String> {
        validate_host_length(&host)?;
        is_safe_target(&host)?;
        
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(2).min(10));

        // 常用端口列表（仅限本地测试和授权扫描）
        let common_ports = [
            (21, "FTP"),
            (22, "SSH"),
            (23, "Telnet"),
            (25, "SMTP"),
            (53, "DNS"),
            (80, "HTTP"),
            (110, "POP3"),
            (143, "IMAP"),
            (443, "HTTPS"),
            (993, "IMAPS"),
            (995, "POP3S"),
            (3306, "MySQL"),
            (5432, "PostgreSQL"),
            (6379, "Redis"),
            (8080, "HTTP-Alt"),
            (8443, "HTTPS-Alt"),
            (27017, "MongoDB"),
        ];

        let mut open_ports = Vec::new();
        let mut closed_ports = Vec::new();

        // 添加警告信息
        let mut result = "⚠️ 警告：端口扫描可能违反目标网络的使用政策\n\n".to_string();
        result.push_str(&format!("🔍 端口扫描结果：{}\n\n", host));

        for (port, service) in &common_ports {
            if check_tcp_connect(&host, *port, timeout).is_ok() {
                open_ports.push((*port, *service));
            } else {
                closed_ports.push((*port, *service));
            }
            
            // 速率限制：每个端口之间延迟
            std::thread::sleep(Duration::from_millis(PORT_SCAN_DELAY_MS));
        }

        if !open_ports.is_empty() {
            result.push_str("✅ 开放端口:\n");
            for (port, service) in &open_ports {
                result.push_str(&format!("  端口 {:>5} - {}\n", port, service));
            }
            result.push('\n');
        } else {
            result.push_str("❌ 未发现开放端口\n\n");
        }

        result.push_str(&format!("扫描端口数：{}\n", common_ports.len()));
        result.push_str(&format!("开放端口数：{}\n", open_ports.len()));
        result.push_str(&format!("关闭端口数：{}\n", closed_ports.len()));

        Ok(result)
    }

    /// 获取本地网络信息
    /// 显示本地 IP 地址、接口等信息
    pub fn get_local_network_info(&self) -> Result<String, String> {
        let mut result = String::from("🌐 本地网络信息\n\n");

        // 获取主机名
        if let Ok(hostname) = std::env::var("HOSTNAME") {
            result.push_str(&format!("主机名：{}\n", hostname));
        } else if let Ok(hostname) = Command::new("hostname").stdin(std::process::Stdio::null()).output() {
            result.push_str(&format!(
                "主机名：{}\n",
                String::from_utf8_lossy(&hostname.stdout).trim()
            ));
        }

        // 获取 IP 地址（通过 /proc/net/route 或 ip 命令）
        result.push_str("\n🔹 网络接口:\n");

        if let Ok(output) = Command::new("ip").args(["addr", "show"]).stdin(std::process::Stdio::null()).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            result.push_str(&format!("{}\n", stdout));
        } else if let Ok(output) = Command::new("ifconfig").stdin(std::process::Stdio::null()).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            result.push_str(&format!("{}\n", stdout));
        }

        // DNS 信息
        result.push_str("\n🔹 DNS 配置:\n");
        if let Ok(resolv_conf) = std::fs::read_to_string("/etc/resolv.conf") {
            for line in resolv_conf.lines() {
                if line.trim().starts_with("nameserver") {
                    result.push_str(&format!("  {}\n", line));
                }
            }
        }

        // 路由信息
        result.push_str("\n🔹 默认路由:\n");
        if let Ok(output) = Command::new("ip").args(["route", "show", "default"]).stdin(std::process::Stdio::null()).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            result.push_str(&format!("  {}", stdout));
        }

        Ok(result)
    }

    /// 追踪路由（简化版 traceroute）
    /// 显示到目标主机的路由路径
    pub fn trace_route(&self, host: String, max_hops: Option<u32>) -> Result<String, String> {
        validate_host_length(&host)?;
        is_safe_target(&host)?;
        
        let max_hops = max_hops.unwrap_or(15).min(30);

        let mut result = format!("🛣️ 路由追踪：{}\n\n", host);

        // 使用 tracepath 命令（如果可用）
        if let Ok(output) = Command::new("tracepath")
            .arg("-m")
            .arg(max_hops.to_string())
            .arg(&host)
            .stdin(std::process::Stdio::null())
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            result.push_str(&stdout);
        } else {
            result.push_str("⚠️ tracepath 命令不可用，尝试基础连接测试...\n\n");

            // 降级方案：简单测试
            result.push_str(&format!("目标主机：{}\n", host));

            if check_tcp_connect(&host, 80, Duration::from_secs(5)).is_ok() {
                result.push_str("状态：✅ 可达 (HTTP 端口)\n");
            } else if check_tcp_connect(&host, 443, Duration::from_secs(5)).is_ok() {
                result.push_str("状态：✅ 可达 (HTTPS 端口)\n");
            } else {
                result.push_str("状态：❌ 无法连接常见端口\n");
            }
        }

        Ok(result)
    }

    /// 检查 UDP 端口
    /// 测试 UDP 端口连通性（通过发送探测包）
    pub fn check_udp_port(&self, host: String, port: u16, timeout_secs: Option<u64>, payload: Option<String>) -> Result<String, String> {
        validate_host_length(&host)?;
        is_safe_target(&host)?;
        
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(5).min(30));

        // 创建 UDP socket
        let socket = UdpSocket::bind("0.0.0.0:0")
            .map_err(|e| format!("创建 UDP socket 失败：{}", e))?;
        
        socket.set_read_timeout(Some(timeout))
            .map_err(|e| format!("设置超时失败：{}", e))?;

        // 解析目标地址
        let addr = format!("{}:{}", host, port)
            .to_socket_addrs()
            .map_err(|e| format!("解析主机地址失败：{}", e))?
            .next()
            .ok_or_else(|| "无法解析主机地址".to_string())?;

        // 发送探测包（默认发送空包，或自定义 payload）
        let payload_bytes = payload
            .as_ref()
            .map(|s| s.as_bytes())
            .unwrap_or(&[0u8; 1]);
        
        let send_result = socket.send_to(payload_bytes, addr);
        
        match send_result {
            Ok(_) => {
                // 尝试接收响应（UDP 是无连接的，可能收到 ICMP Port Unreachable）
                socket.connect(addr)
                    .map_err(|e| format!("连接失败：{}", e))?;
                
                let mut buf = [0u8; 1024];
                socket.set_nonblocking(false)
                    .map_err(|e| format!("设置非阻塞失败：{}", e))?;
                
                match socket.recv(&mut buf) {
                    Ok(len) => Ok(format!(
                        "✅ UDP 端口测试\n\n主机：{}\n端口：{}\n状态：收到响应 ({} bytes)\n\n注意：UDP 是无连接协议，此测试仅供参考",
                        host, port, len
                    )),
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock 
                           || e.kind() == std::io::ErrorKind::TimedOut => {
                        Ok(format!(
                            "⚠️ UDP 端口测试\n\n主机：{}\n端口：{}\n状态：超时（无响应）\n\n注意：UDP 是无连接协议，端口可能开放但无响应",
                            host, port
                        ))
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => {
                        Ok(format!(
                            "❌ UDP 端口测试\n\n主机：{}\n端口：{}\n状态：端口关闭（收到 ICMP Port Unreachable）",
                            host, port
                        ))
                    }
                    Err(e) => Ok(format!(
                        "⚠️ UDP 端口测试\n\n主机：{}\n端口：{}\n错误：{}\n\n注意：UDP 是无连接协议，此测试仅供参考",
                        host, port, e
                    )),
                }
            }
            Err(e) => Err(format!("发送探测包失败：{}", e)),
        }
    }

    /// 获取公网 IP 地址
    /// 通过外部服务查询公网 IP
    pub fn get_public_ip(&self) -> Result<String, String> {
        let services = [
            "https://api.ipify.org",
            "https://ifconfig.me/ip",
            "https://icanhazip.com",
        ];

        for service in &services {
            if let Ok(ip) = query_public_ip(service) {
                return Ok(format!(
                    "🌍 公网 IP 地址\n\nIP: {}\n查询服务：{}\n\n⚠️ 注意：查询公网 IP 会将您的请求发送到第三方服务",
                    ip.trim(),
                    service
                ));
            }
        }

        Err("无法从任何服务获取公网 IP (所有服务均不可用)".to_string())
    }
}

/// 检查 TCP 连接
fn check_tcp_connect(host: &str, port: u16, timeout: Duration) -> Result<(), String> {
    let addr = format!("{}:{}", host, port)
        .to_socket_addrs()
        .map_err(|e| e.to_string())?
        .next()
        .ok_or_else(|| "无法解析主机地址".to_string())?;

    TcpStream::connect_timeout(&addr, timeout)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 查询公网 IP
fn query_public_ip(url: &str) -> Result<String, String> {
    let client = &*HTTP_CLIENT;

    let response = client.get(url).send()
        .map_err(|e| e.to_string())?;

    let ip = response.text()
        .map_err(|e| e.to_string())?;

    Ok(ip)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_safe_target_localhost() {
        // localhost 应该允许
        assert!(is_safe_target("localhost").is_ok());
        assert!(is_safe_target("127.0.0.1").is_ok());
    }

    #[test]
    fn test_is_safe_target_private_ip() {
        // 私有地址应该被拒绝
        assert!(is_safe_target("192.168.1.1").is_err());
        assert!(is_safe_target("10.0.0.1").is_err());
        assert!(is_safe_target("172.16.0.1").is_err());
    }

    #[test]
    fn test_validate_host_length() {
        let long_host = "a".repeat(300);
        assert!(validate_host_length(&long_host).is_err());
        
        let short_host = "example.com";
        assert!(validate_host_length(short_host).is_ok());
    }

    #[test]
    fn test_check_tcp_connect_localhost() {
        // 本地回环应该可以连接（即使端口关闭也不会报错解析失败）
        let result = check_tcp_connect("127.0.0.1", 1, Duration::from_millis(100));
        // 连接应该失败（端口 1 通常关闭），但不应该解析失败
        assert!(result.is_err());
        assert!(!result.unwrap_err().contains("解析"));
    }

    #[test]
    fn test_get_local_network_info() {
        let tools = NetworkTools;
        let result = tools.get_local_network_info();
        
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("本地网络信息"));
        assert!(output.contains("网络接口") || output.contains("DNS"));
    }

    #[test]
    fn test_check_udp_port_syntax() {
        let tools = NetworkTools;
        
        // 测试语法正确性（不保证成功）
        let result = tools.check_udp_port(
            "127.0.0.1".to_string(),
            53,
            Some(2),
            Some("test".to_string()),
        );

        // 验证方法不 panic（网络请求可能成功或失败）
        let _ = result;
    }
}
