//! 网络诊断工具集
//!
//! 提供网络诊断和连接测试功能（Ping、端口扫描、路由追踪等）

use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;
use tokitai::tool;

use super::error::{NetworkResult, NetworkToolError};
use super::ssrf_protection;

// ============================================================================
// 配置结构
// ============================================================================

/// 网络工具配置
#[derive(Debug, Clone)]
pub struct NetworkToolsConfig {
    /// 默认超时时间（秒）
    pub default_timeout_secs: u64,
    /// 端口扫描延迟（毫秒）
    pub port_scan_delay_ms: u64,
    /// 是否允许扫描 localhost
    pub allow_localhost_scan: bool,
    /// User-Agent（用于公网 IP 查询）
    pub user_agent: String,
}

impl Default for NetworkToolsConfig {
    fn default() -> Self {
        Self {
            default_timeout_secs: 5,
            port_scan_delay_ms: 100,
            allow_localhost_scan: true,
            user_agent: "Tokitai AI Assistant/1.0".to_string(),
        }
    }
}

// ============================================================================
// 网络工具集
// ============================================================================

/// 网络工具集
/// 提供网络诊断和连接测试功能
pub struct NetworkTools {
    config: NetworkToolsConfig,
    client: reqwest::blocking::Client,
}

impl NetworkTools {
    /// 创建新的网络工具实例
    pub fn new() -> Self {
        Self::with_config(NetworkToolsConfig::default())
    }

    /// 创建带自定义配置的网络工具实例
    pub fn with_config(config: NetworkToolsConfig) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(config.default_timeout_secs))
            .user_agent(&config.user_agent)
            .build()
            .expect("创建 HTTP 客户端失败");

        Self { config, client }
    }

    /// 验证主机名长度
    fn validate_host(&self, host: &str) -> NetworkResult<()> {
        const MAX_HOST_LENGTH: usize = 256;

        if host.len() > MAX_HOST_LENGTH {
            return Err(NetworkToolError::InvalidHostname(format!(
                "主机名过长 ({} > {} 字符)",
                host.len(),
                MAX_HOST_LENGTH
            ))
            .into());
        }
        Ok(())
    }

    /// 检查目标是否安全（使用统一的 SSRF 防护模块）
    fn is_safe_target(&self, host: &str) -> NetworkResult<()> {
        // 允许 localhost 用于本地测试
        if self.config.allow_localhost_scan
            && (host == "localhost" || host == "127.0.0.1" || host == "::1")
        {
            return Ok(());
        }

        // 尝试解析 IP 地址并使用统一的 SSRF 检查
        if let Ok(ip_addr) = host.parse::<std::net::IpAddr>() {
            return ssrf_protection::check_ip_safety(&ip_addr);
        }

        // 对于域名，尝试解析并检查所有 IP
        if let Ok(addrs) = host.to_socket_addrs() {
            for addr in addrs {
                ssrf_protection::check_ip_safety(&addr.ip())?;
            }
        }

        Ok(())
    }
}

impl Default for NetworkTools {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tool 实现
// ============================================================================

#[tool]
impl NetworkTools {
    /// Ping 主机（测试连通性）
    ///
    /// # 参数
    /// - `host`: 目标主机名或 IP
    /// - `count`: Ping 次数（默认 4，最大 10）
    ///
    /// # 返回
    /// 返回 Ping 测试结果，包括成功率和响应时间
    #[tool(default_count = "null")]
    pub fn ping_host(&self, host: String, count: Option<u32>) -> NetworkResult<String> {
        self.validate_host(&host)?;
        self.is_safe_target(&host)?;

        let count = count.unwrap_or(4).min(10);

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
                results.push(format!(
                    "  请求 {}: 成功 (耗时 {:.2} ms)",
                    i,
                    elapsed.as_secs_f64() * 1000.0
                ));
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
    ///
    /// # 参数
    /// - `host`: 目标主机名或 IP
    /// - `port`: 端口号
    /// - `timeout_secs`: 超时时间（秒）
    ///
    /// # 返回
    /// 返回端口状态和响应时间
    #[tool(default_timeout_secs = "null")]
    pub fn check_tcp_port(
        &self,
        host: String,
        port: u16,
        timeout_secs: Option<u64>,
    ) -> NetworkResult<String> {
        self.validate_host(&host)?;
        self.is_safe_target(&host)?;

        let timeout = Duration::from_secs(timeout_secs.unwrap_or(5).min(30));

        let start = std::time::Instant::now();
        let result = check_tcp_connect(&host, port, timeout);
        let elapsed = start.elapsed();

        match result {
            Ok(_) => Ok(format!(
                "✅ 端口开放\n\n主机：{}\n端口：{}\n响应时间：{:.2} ms\n状态：可连接",
                host,
                port,
                elapsed.as_secs_f64() * 1000.0
            )),
            Err(e) => Ok(format!(
                "❌ 端口关闭或不可达\n\n主机：{}\n端口：{}\n超时时间：{} 秒\n错误：{}",
                host,
                port,
                timeout_secs.unwrap_or(5),
                e
            )),
        }
    }

    /// 扫描常用端口
    ///
    /// # 参数
    /// - `host`: 目标主机名或 IP
    /// - `timeout_secs`: 每个端口的超时时间（秒）
    ///
    /// # 返回
    /// 返回开放端口列表
    #[tool(default_timeout_secs = "null")]
    pub fn scan_common_ports(
        &self,
        host: String,
        timeout_secs: Option<u64>,
    ) -> NetworkResult<String> {
        self.validate_host(&host)?;
        self.is_safe_target(&host)?;

        let timeout = Duration::from_secs(timeout_secs.unwrap_or(2).min(10));

        // 常用端口列表
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

        let mut result = "⚠️ 警告：端口扫描可能违反目标网络的使用政策\n\n".to_string();
        result.push_str(&format!("🔍 端口扫描结果：{}\n\n", host));

        for (port, service) in &common_ports {
            if check_tcp_connect(&host, *port, timeout).is_ok() {
                open_ports.push((*port, *service));
            } else {
                closed_ports.push((*port, *service));
            }

            // 速率限制
            std::thread::sleep(Duration::from_millis(self.config.port_scan_delay_ms));
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
    ///
    /// # 返回
    /// 返回本地 IP 地址、接口、DNS 等信息
    pub fn get_local_network_info(&self) -> NetworkResult<String> {
        let mut result = String::from("🌐 本地网络信息\n\n");

        // 获取主机名
        if let Ok(hostname) = std::env::var("HOSTNAME") {
            result.push_str(&format!("主机名：{}\n", hostname));
        }

        // IP 地址信息
        result.push_str("\n🔹 网络接口:\n");

        // 尝试获取 IP 地址（跨平台）
        if let Ok(addrs) = get_local_ip_addresses() {
            for addr in addrs {
                result.push_str(&format!("  IP: {}\n", addr));
            }
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

        Ok(result)
    }

    /// 追踪路由（简化版 traceroute）
    ///
    /// # 参数
    /// - `host`: 目标主机名或 IP
    /// - `max_hops`: 最大跳数（默认 15，最大 30）
    ///
    /// # 返回
    /// 返回路由追踪结果
    #[tool(default_max_hops = "null")]
    pub fn trace_route(&self, host: String, max_hops: Option<u32>) -> NetworkResult<String> {
        self.validate_host(&host)?;
        self.is_safe_target(&host)?;

        let max_hops = max_hops.unwrap_or(15).min(30);

        let mut result = format!("🛣️ 路由追踪：{}\n\n", host);

        // 使用 tracepath 命令（如果可用）
        if let Ok(output) = std::process::Command::new("tracepath")
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

    /// 获取公网 IP 地址
    ///
    /// # 返回
    /// 返回公网 IP 地址和查询服务信息
    pub fn get_public_ip(&self) -> NetworkResult<String> {
        let services = [
            "https://api.ipify.org",
            "https://ifconfig.me/ip",
            "https://icanhazip.com",
        ];

        for service in &services {
            if let Ok(ip) = self.query_public_ip(service) {
                return Ok(format!(
                    "🌍 公网 IP 地址\n\nIP: {}\n查询服务：{}\n\n⚠️ 注意：查询公网 IP 会将您的请求发送到第三方服务",
                    ip.trim(),
                    service
                ));
            }
        }

        Err(NetworkToolError::DnsResolution(
            "无法从任何服务获取公网 IP (所有服务均不可用)".to_string(),
        )
        .into())
    }

    /// 检查 UDP 端口
    ///
    /// # 参数
    /// - `host`: 目标主机名或 IP
    /// - `port`: 端口号
    /// - `timeout_secs`: 超时时间（秒）
    /// - `payload`: 可选的探测数据
    ///
    /// # 返回
    /// 返回 UDP 端口测试结果
    #[tool(default_timeout_secs = "null", default_payload = "null")]
    pub fn check_udp_port(
        &self,
        host: String,
        port: u16,
        timeout_secs: Option<u64>,
        payload: Option<String>,
    ) -> NetworkResult<String> {
        self.validate_host(&host)?;
        self.is_safe_target(&host)?;

        let timeout = Duration::from_secs(timeout_secs.unwrap_or(5).min(30));

        // 创建 UDP socket
        let socket = std::net::UdpSocket::bind("0.0.0.0:0").map_err(|e| {
            NetworkToolError::PermissionDenied(format!("创建 UDP socket 失败：{}", e))
        })?;

        socket
            .set_read_timeout(Some(timeout))
            .map_err(|e| NetworkToolError::PermissionDenied(format!("设置超时失败：{}", e)))?;

        // 解析目标地址
        let addr = format!("{}:{}", host, port)
            .to_socket_addrs()
            .map_err(|e| NetworkToolError::DnsResolution(format!("解析主机地址失败：{}", e)))?
            .next()
            .ok_or_else(|| NetworkToolError::InvalidHostname("无法解析主机地址".to_string()))?;

        // 发送探测包
        let payload_bytes = payload.as_ref().map(|s| s.as_bytes()).unwrap_or(&[0u8; 1]);

        let send_result = socket.send_to(payload_bytes, addr);

        match send_result {
            Ok(_) => {
                socket
                    .connect(addr)
                    .map_err(|e| NetworkToolError::ConnectionRefused {
                        host: host.clone(),
                        port,
                    })?;

                let mut buf = [0u8; 1024];
                socket.set_nonblocking(false).map_err(|e| {
                    NetworkToolError::PermissionDenied(format!("设置非阻塞失败：{}", e))
                })?;

                match socket.recv(&mut buf) {
                    Ok(len) => Ok(format!(
                        "✅ UDP 端口测试\n\n主机：{}\n端口：{}\n状态：收到响应 ({} bytes)\n\n注意：UDP 是无连接协议，此测试仅供参考",
                        host, port, len
                    )),
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                    {
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
            Err(e) => {
                Err(NetworkToolError::PermissionDenied(format!("发送探测包失败：{}", e)).into())
            }
        }
    }
}

impl NetworkTools {
    /// 查询公网 IP
    fn query_public_ip(&self, url: &str) -> NetworkResult<String> {
        let response = self.client.get(url).send()?;
        let ip = response.text()?;
        Ok(ip)
    }
}

// ============================================================================
// 工具函数
// ============================================================================

/// 检查 TCP 连接
fn check_tcp_connect(host: &str, port: u16, timeout: Duration) -> Result<(), String> {
    let addr = format!("{}:{}", host, port)
        .to_socket_addrs()
        .map_err(|e| e.to_string())?
        .next()
        .ok_or_else(|| "无法解析主机地址".to_string())?;

    TcpStream::connect_timeout(&addr, timeout).map_err(|e| e.to_string())?;
    Ok(())
}

/// 获取本地 IP 地址列表
fn get_local_ip_addresses() -> Result<Vec<String>, String> {
    let mut addresses = Vec::new();

    // 尝试使用 get_if_addrs crate（如果可用）
    // 这里使用简单的回退方案
    if let Ok(output) = std::process::Command::new("hostname").arg("-I").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for ip in stdout.split_whitespace() {
            addresses.push(ip.to_string());
        }
    }

    // 如果没有找到，添加 localhost
    if addresses.is_empty() {
        addresses.push("127.0.0.1".to_string());
    }

    Ok(addresses)
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_tools_config_default() {
        let config = NetworkToolsConfig::default();
        assert_eq!(config.default_timeout_secs, 5);
        assert_eq!(config.port_scan_delay_ms, 100);
        assert!(config.allow_localhost_scan);
    }

    #[test]
    fn test_network_tools_creation() {
        let tools = NetworkTools::new();
        assert!(tools.config.allow_localhost_scan);
    }

    #[test]
    fn test_is_safe_target_localhost() {
        let tools = NetworkTools::new();
        assert!(tools.is_safe_target("localhost").is_ok());
        assert!(tools.is_safe_target("127.0.0.1").is_ok());
    }

    #[test]
    fn test_is_safe_target_private_ip() {
        let tools = NetworkTools::new();
        assert!(tools.is_safe_target("192.168.1.1").is_err());
        assert!(tools.is_safe_target("10.0.0.1").is_err());
        assert!(tools.is_safe_target("172.16.0.1").is_err());
    }

    #[test]
    fn test_validate_host_length() {
        let tools = NetworkTools::new();
        let long_host = "a".repeat(300);
        assert!(tools.validate_host(&long_host).is_err());

        let short_host = "example.com";
        assert!(tools.validate_host(short_host).is_ok());
    }

    #[test]
    fn test_get_local_network_info() {
        let tools = NetworkTools::new();
        let result = tools.get_local_network_info();

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("本地网络信息"));
    }
}
