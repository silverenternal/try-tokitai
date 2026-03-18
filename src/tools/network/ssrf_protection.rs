//! SSRF 防护统一模块
//!
//! 提供统一的 URL 和 IP 地址安全检查，防止服务器端请求伪造攻击
//! 所有网络工具都应通过此模块进行安全验证

use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use parking_lot::RwLock;
use url::Url;

use super::error::{NetworkResult, SsrfError};

// ============================================================================
// 配置结构
// ============================================================================

/// SSRF 防护配置
#[derive(Debug, Clone)]
pub struct SsrfConfig {
    /// 允许的最大 URL 长度
    pub max_url_length: usize,
    /// 允许的最大路径长度
    pub max_path_length: usize,
    /// 禁止访问的内网域名列表
    pub blocked_domains: Vec<String>,
    /// 禁止访问的敏感目录列表
    pub sensitive_paths: Vec<String>,
    /// 是否允许访问回环地址（127.0.0.1）
    pub allow_loopback: bool,
    /// 是否允许访问链路本地地址
    pub allow_link_local: bool,
    /// DNS 重绑定保护：是否解析域名并检查 IP
    pub enable_dns_rebinding_protection: bool,
}

impl Default for SsrfConfig {
    fn default() -> Self {
        Self {
            max_url_length: 4096,
            max_path_length: 1024,
            blocked_domains: vec![
                "localhost".to_string(),
                "localhost.localdomain".to_string(),
                "internal".to_string(),
                "intranet".to_string(),
                "metric".to_string(),  // 防止访问云服务商元数据
            ],
            sensitive_paths: vec![
                "/etc".to_string(),
                "/root".to_string(),
                "/home".to_string(),
                "/var".to_string(),
                "/usr".to_string(),
                "/bin".to_string(),
                "/sbin".to_string(),
                "/proc".to_string(),
                "/sys".to_string(),
            ],
            allow_loopback: false,
            allow_link_local: false,
            enable_dns_rebinding_protection: true,
        }
    }
}

/// 运行时 SSRF 配置（支持热更新）
#[derive(Debug, Clone)]
pub struct RuntimeSsrfConfig {
    pub config: Arc<SsrfConfig>,
    /// 动态黑名单（运行时添加）
    pub dynamic_blocked_ips: Arc<RwLock<Vec<IpAddr>>>,
    /// 动态白名单（运行时添加，优先级高于黑名单）
    pub dynamic_allowed_ips: Arc<RwLock<Vec<IpAddr>>>,
}

impl Default for RuntimeSsrfConfig {
    fn default() -> Self {
        Self {
            config: Arc::new(SsrfConfig::default()),
            dynamic_blocked_ips: Arc::new(RwLock::new(Vec::new())),
            dynamic_allowed_ips: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

// 手动实现 Deserialize 用于 tool 宏
impl<'de> serde::Deserialize<'de> for RuntimeSsrfConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // 忽略反序列化的值，返回默认配置
        let _ = serde_json::Value::deserialize(deserializer)?;
        Ok(RuntimeSsrfConfig::default())
    }
}

impl RuntimeSsrfConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(config: SsrfConfig) -> Self {
        Self {
            config: Arc::new(config),
            ..Default::default()
        }
    }

    /// 动态添加 blocked IP
    pub fn block_ip(&self, ip: IpAddr) {
        self.dynamic_blocked_ips.write().push(ip);
    }

    /// 动态添加 allowed IP
    pub fn allow_ip(&self, ip: IpAddr) {
        self.dynamic_allowed_ips.write().push(ip);
    }

    /// 清空动态列表
    pub fn clear_dynamic_rules(&self) {
        self.dynamic_blocked_ips.write().clear();
        self.dynamic_allowed_ips.write().clear();
    }
}

// ============================================================================
// URL 安全检查
// ============================================================================

/// 验证 URL 是否安全（SSRF 防护）
pub fn validate_url(url: &str) -> NetworkResult<()> {
    validate_url_with_config(url, &RuntimeSsrfConfig::default())
}

/// 验证 URL 是否安全（带配置）
pub fn validate_url_with_config(url: &str, runtime_config: &RuntimeSsrfConfig) -> NetworkResult<()> {
    let config = &runtime_config.config;

    // 检查 URL 长度
    if url.len() > config.max_url_length {
        return Err(SsrfError::UrlTooLong(url.len(), config.max_url_length).into());
    }

    // 解析 URL
    let parsed = Url::parse(url).map_err(|e| SsrfError::InvalidUrl(e.to_string()))?;

    // 检查协议
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(SsrfError::UnsupportedScheme(scheme.to_string()).into());
    }

    // 检查主机名
    let host = parsed.host_str().ok_or(SsrfError::MissingHostname)?;

    // 检查禁止的域名
    let host_lower = host.to_lowercase();
    for blocked in &config.blocked_domains {
        if host_lower == *blocked || host_lower.ends_with(&format!(".{}", blocked)) {
            return Err(SsrfError::BlockedDomain(host.to_string()).into());
        }
    }

    // DNS 重绑定保护：解析域名并检查 IP
    if config.enable_dns_rebinding_protection {
        if let Err(e) = validate_host_resolution(host, runtime_config) {
            return Err(e);
        }
    }

    Ok(())
}

/// 验证主机名解析后的 IP 地址
fn validate_host_resolution(host: &str, runtime_config: &RuntimeSsrfConfig) -> NetworkResult<()> {
    // 尝试解析为 SocketAddr
    let addrs: Vec<SocketAddr> = format!("{}:80", host)
        .to_socket_addrs()
        .map_err(|e| SsrfError::DnsResolution(e.to_string()))?
        .collect();

    if addrs.is_empty() {
        return Err(SsrfError::DnsResolution("DNS 解析未返回任何地址".to_string()).into());
    }

    // 检查所有解析出的 IP 地址
    for addr in addrs {
        check_ip_safety_internal(&addr.ip(), runtime_config)?;
    }

    Ok(())
}

// ============================================================================
// IP 安全检查
// ============================================================================

/// 检查 IP 地址是否安全（非内网地址）
pub fn check_ip_safety(ip: &IpAddr) -> NetworkResult<()> {
    check_ip_safety_internal(ip, &RuntimeSsrfConfig::default())
}

/// 检查 IP 地址是否安全（带配置）
pub fn check_ip_safety_with_config(ip: &IpAddr, runtime_config: &RuntimeSsrfConfig) -> NetworkResult<()> {
    check_ip_safety_internal(ip, runtime_config)
}

/// 内部 IP 安全检查实现
fn check_ip_safety_internal(ip: &IpAddr, runtime_config: &RuntimeSsrfConfig) -> NetworkResult<()> {
    let config = &runtime_config.config;

    // 检查动态白名单（优先级最高）
    if runtime_config.dynamic_allowed_ips.read().contains(ip) {
        return Ok(());
    }

    // 检查动态黑名单
    if runtime_config.dynamic_blocked_ips.read().contains(ip) {
        return Err(SsrfError::PrivateNetwork(format!("{} (动态黑名单)", ip)).into());
    }

    match ip {
        IpAddr::V4(ip4) => {
            // 检查回环地址
            if ip4.is_loopback() {
                if !config.allow_loopback {
                    return Err(SsrfError::PrivateNetwork(ip.to_string()).into());
                }
                return Ok(());
            }

            // 检查链路本地地址
            if ip4.is_link_local() {
                if !config.allow_link_local {
                    return Err(SsrfError::PrivateNetwork(ip.to_string()).into());
                }
                return Ok(());
            }

            // 检查未指定地址
            if ip4.is_unspecified() {
                return Err(SsrfError::PrivateNetwork(ip.to_string()).into());
            }

            // 检查私有地址（10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16）
            if ip4.is_private() {
                return Err(SsrfError::PrivateNetwork(ip.to_string()).into());
            }

            // 检查链路本地多播地址
            if ip4.is_multicast() {
                return Err(SsrfError::PrivateNetwork(ip.to_string()).into());
            }

            // 检查文档专用地址（TEST-NET）
            let octets = ip4.octets();
            if octets[0] == 192 && octets[1] == 0 && octets[2] == 2 {
                return Err(SsrfError::PrivateNetwork(ip.to_string()).into());
            }

            // 检查 100.64.0.0/10 (Carrier-grade NAT)
            if octets[0] == 100 && (octets[1] >= 64 && octets[1] <= 127) {
                return Err(SsrfError::PrivateNetwork(ip.to_string()).into());
            }
        }
        IpAddr::V6(ip6) => {
            // 检查回环地址
            if ip6.is_loopback() {
                if !config.allow_loopback {
                    return Err(SsrfError::PrivateNetwork(ip.to_string()).into());
                }
                return Ok(());
            }

            // 检查唯一本地地址
            if ip6.is_unique_local() {
                return Err(SsrfError::PrivateNetwork(ip.to_string()).into());
            }

            // 检查未指定地址
            if ip6.is_unspecified() {
                return Err(SsrfError::PrivateNetwork(ip.to_string()).into());
            }

            // 检查链路本地地址
            if ip6.is_unicast_link_local() {
                if !config.allow_link_local {
                    return Err(SsrfError::PrivateNetwork(ip.to_string()).into());
                }
                return Ok(());
            }
        }
    }

    Ok(())
}

// ============================================================================
// 路径安全检查
// ============================================================================

/// 验证保存路径是否安全
pub fn validate_save_path(path: &str) -> NetworkResult<()> {
    validate_save_path_with_config(path, &RuntimeSsrfConfig::default())
}

/// 验证保存路径是否安全（带配置）
pub fn validate_save_path_with_config(path: &str, runtime_config: &RuntimeSsrfConfig) -> NetworkResult<()> {
    let config = &runtime_config.config;

    // 检查路径长度
    if path.len() > config.max_path_length {
        return Err(SsrfError::PathTooLong(path.len(), config.max_path_length).into());
    }

    let path_buf = std::path::PathBuf::from(path);

    // 规范化路径（解析符号链接和相对路径）
    let absolute_path = if path_buf.exists() {
        path_buf.canonicalize().map_err(|e| {
            SsrfError::InvalidUrl(format!("规范化路径失败：{}", e))
        })?
    } else {
        // 路径不存在，尝试规范化
        path_buf.clone()
    };

    // 获取当前工作目录
    let cwd = std::env::current_dir().map_err(|e| {
        SsrfError::InvalidUrl(format!("获取当前目录失败：{}", e))
    })?;

    // 检查路径是否在当前目录或其子目录下
    if !absolute_path.starts_with(&cwd) {
        return Err(SsrfError::OutOfCwd(path.to_string()).into());
    }

    // 检查是否尝试访问敏感目录
    let path_str = absolute_path.to_string_lossy();
    for sensitive in &config.sensitive_paths {
        if path_str.starts_with(sensitive.as_str()) {
            return Err(SsrfError::SensitivePath(sensitive.clone()).into());
        }
    }

    Ok(())
}

// ============================================================================
// 便捷函数
// ============================================================================

/// 快速检查 URL 是否安全（返回布尔值）
pub fn is_url_safe(url: &str) -> bool {
    validate_url(url).is_ok()
}

/// 快速检查 IP 是否安全（返回布尔值）
pub fn is_ip_safe(ip: &IpAddr) -> bool {
    check_ip_safety(ip).is_ok()
}

/// 快速检查路径是否安全（返回布尔值）
pub fn is_path_safe(path: &str) -> bool {
    validate_save_path(path).is_ok()
}

/// 验证 URL 并返回解析后的 URL 对象
pub fn parse_and_validate_url(url: &str) -> NetworkResult<Url> {
    validate_url(url)?;
    Ok(Url::parse(url).map_err(|e| SsrfError::InvalidUrl(e.to_string()))?)
}

/// 验证 URL 并返回安全的 reqwest URL
pub fn to_reqwest_url(url: &str) -> NetworkResult<reqwest::Url> {
    validate_url(url)?;
    Ok(reqwest::Url::parse(url).map_err(|e| SsrfError::InvalidUrl(e.to_string()))?)
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::network::error::NetworkError;

    #[test]
    fn test_validate_url_valid() {
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("http://api.github.com/users").is_ok());
        assert!(validate_url("https://www.google.com/search?q=rust").is_ok());
    }

    #[test]
    fn test_validate_url_invalid_scheme() {
        assert!(matches!(
            validate_url("file:///etc/passwd"),
            Err(NetworkError::Ssrf(SsrfError::UnsupportedScheme(_)))
        ));
        assert!(matches!(
            validate_url("ftp://example.com"),
            Err(NetworkError::Ssrf(SsrfError::UnsupportedScheme(_)))
        ));
    }

    #[test]
    fn test_validate_url_localhost() {
        assert!(matches!(
            validate_url("http://localhost:8080"),
            Err(NetworkError::Ssrf(SsrfError::BlockedDomain(_)))
        ));
        assert!(matches!(
            validate_url("http://localhost.localdomain"),
            Err(NetworkError::Ssrf(SsrfError::BlockedDomain(_)))
        ));
    }

    #[test]
    fn test_validate_url_private_ip() {
        assert!(matches!(
            validate_url("http://127.0.0.1:8080"),
            Err(NetworkError::Ssrf(SsrfError::PrivateNetwork(_)))
        ));
        assert!(matches!(
            validate_url("http://192.168.1.1"),
            Err(NetworkError::Ssrf(SsrfError::PrivateNetwork(_)))
        ));
        assert!(matches!(
            validate_url("http://10.0.0.1"),
            Err(NetworkError::Ssrf(SsrfError::PrivateNetwork(_)))
        ));
        assert!(matches!(
            validate_url("http://172.16.0.1"),
            Err(NetworkError::Ssrf(SsrfError::PrivateNetwork(_)))
        ));
    }

    #[test]
    fn test_validate_url_length() {
        let long_url = "https://example.com/".to_string() + &"a".repeat(5000);
        assert!(matches!(
            validate_url(&long_url),
            Err(NetworkError::Ssrf(SsrfError::UrlTooLong(_, _)))
        ));
    }

    #[test]
    fn test_check_ip_safety_public() {
        // 公网 IP 应该安全
        assert!(is_ip_safe(&IpAddr::V4(std::net::Ipv4Addr::new(8, 8, 8, 8))));
        assert!(is_ip_safe(&IpAddr::V4(std::net::Ipv4Addr::new(1, 1, 1, 1))));
        assert!(is_ip_safe(&IpAddr::V6(std::net::Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1))));
    }

    #[test]
    fn test_check_ip_safety_private() {
        // 内网 IP 应该不安全
        assert!(!is_ip_safe(&IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))));
        assert!(!is_ip_safe(&IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 1))));
        assert!(!is_ip_safe(&IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1))));
        assert!(!is_ip_safe(&IpAddr::V4(std::net::Ipv4Addr::new(172, 16, 0, 1))));
    }

    #[test]
    fn test_runtime_config_dynamic_rules() {
        let config = RuntimeSsrfConfig::new();
        
        // 动态添加黑名单
        let blocked_ip = IpAddr::V4(std::net::Ipv4Addr::new(1, 2, 3, 4));
        config.block_ip(blocked_ip);
        
        // 该 IP 应该被拒绝
        assert!(check_ip_safety_with_config(&blocked_ip, &config).is_err());
        
        // 动态添加白名单（优先级更高）
        config.allow_ip(blocked_ip);
        
        // 现在应该允许
        assert!(check_ip_safety_with_config(&blocked_ip, &config).is_ok());
        
        // 清空规则
        config.clear_dynamic_rules();
    }

    #[test]
    fn test_custom_config_allow_loopback() {
        let config = SsrfConfig {
            allow_loopback: true,
            ..Default::default()
        };
        let runtime_config = RuntimeSsrfConfig::with_config(config);

        // 允许回环
        assert!(check_ip_safety_with_config(
            &IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
            &runtime_config
        ).is_ok());
    }

    #[test]
    fn test_carrier_grade_nat() {
        // 100.64.0.0/10 应该被拒绝
        assert!(matches!(
            check_ip_safety(&IpAddr::V4(std::net::Ipv4Addr::new(100, 64, 0, 1))),
            Err(NetworkError::Ssrf(SsrfError::PrivateNetwork(_)))
        ));
    }

    #[test]
    fn test_documentation_range() {
        // 192.0.2.0/24 (TEST-NET) 应该被拒绝
        assert!(matches!(
            check_ip_safety(&IpAddr::V4(std::net::Ipv4Addr::new(192, 0, 2, 1))),
            Err(NetworkError::Ssrf(SsrfError::PrivateNetwork(_)))
        ));
    }
}
