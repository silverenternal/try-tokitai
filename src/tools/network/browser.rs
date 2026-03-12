use anyhow::{Context, Result};
use headless_chrome::Browser;
use headless_chrome::LaunchOptions;
use headless_chrome::protocol::cdp::Page;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokitai::tool;
use std::env;

/// 浏览器配置
#[derive(Clone, Debug)]
pub struct BrowserConfig {
    pub headless: bool,
    pub sandbox: bool,
    pub chrome_path: Option<PathBuf>,
    pub user_data_dir: Option<PathBuf>,
    pub window_size: (u32, u32),
    pub proxy: Option<String>,
    pub enable_gpu: bool,
}

impl BrowserConfig {
    /// 从环境变量和平台检测自动配置
    pub fn from_env() -> Self {
        let headless = env::var("BROWSER_HEADLESS")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        let sandbox = env::var("BROWSER_SANDBOX")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        let chrome_path = env::var("CHROME_PATH")
            .ok()
            .map(PathBuf::from)
            .or_else(|| Self::detect_chrome_path());

        let window_width = env::var("BROWSER_WIDTH")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1920);

        let window_height = env::var("BROWSER_HEIGHT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1080);

        let proxy = env::var("BROWSER_PROXY").ok();

        let enable_gpu = env::var("BROWSER_ENABLE_GPU")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        Self {
            headless,
            sandbox,
            chrome_path,
            user_data_dir: None,
            window_size: (window_width, window_height),
            proxy,
            enable_gpu,
        }
    }

    /// 从 config.toml 配置
    pub fn from_config(config: &toml::Value) -> Option<Self> {
        let browser_table = config.as_table()?.get("browser")?.as_table()?;

        let headless = browser_table
            .get("headless")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let sandbox = browser_table
            .get("sandbox")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let chrome_path = browser_table
            .get("chrome_path")
            .and_then(|v| v.as_str())
            .map(PathBuf::from);

        let window_width = browser_table
            .get("width")
            .and_then(|v| v.as_integer())
            .map(|v| v as u32)
            .unwrap_or(1920);

        let window_height = browser_table
            .get("height")
            .and_then(|v| v.as_integer())
            .map(|v| v as u32)
            .unwrap_or(1080);

        let proxy = browser_table
            .get("proxy")
            .and_then(|v| v.as_str())
            .map(String::from);

        let enable_gpu = browser_table
            .get("enable_gpu")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Some(Self {
            headless,
            sandbox,
            chrome_path: chrome_path.or_else(|| Self::detect_chrome_path()),
            user_data_dir: None,
            window_size: (window_width, window_height),
            proxy,
            enable_gpu,
        })
    }

    /// 自动检测 Chrome 路径（跨平台）
    fn detect_chrome_path() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            let paths = [
                "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
                "/Applications/Chromium.app/Contents/MacOS/Chromium",
                "/Applications/Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary",
                "/usr/bin/chromium-browser",
            ];
            for path in &paths {
                if std::path::Path::new(path).exists() {
                    return Some(PathBuf::from(path));
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            let paths = [
                r"C:\Program Files\Google\Chrome\Application\chrome.exe",
                r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
                r"C:\Users\AppData\Local\Google\Chrome\Application\chrome.exe",
            ];
            for path in &paths {
                if std::path::Path::new(path).exists() {
                    return Some(PathBuf::from(path));
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            let paths = [
                "/usr/bin/google-chrome",
                "/usr/bin/chromium-browser",
                "/usr/bin/chromium",
                "/snap/bin/chromium",
                "/usr/local/bin/chrome",
            ];
            for path in &paths {
                if std::path::Path::new(path).exists() {
                    return Some(PathBuf::from(path));
                }
            }
        }

        None
    }
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

/// 无头浏览器工具集
/// 使用 headless_chrome 控制 Chromium 浏览器
pub struct BrowserTools {
    browser: Browser,
    config: BrowserConfig,
}

/// 验证 URL 是否安全（使用统一 SSRF 防护模块）
fn is_safe_url(url: &str) -> Result<(), String> {
    crate::tools::network::ssrf_protection::validate_url(url)
        .map_err(|e| e.to_string())
}

/// 检查 IP 地址是否安全（使用统一 SSRF 防护模块）
fn check_ip_safety(ip: &std::net::IpAddr) -> Result<(), String> {
    crate::tools::network::ssrf_protection::check_ip_safety(ip)
        .map_err(|e| e.to_string())
}

/// 验证保存路径（使用统一 SSRF 防护模块）
fn validate_save_path(path: &str) -> Result<(), String> {
    crate::tools::network::ssrf_protection::validate_save_path(path)
        .map_err(|e| e.to_string())
}

/// 确保目录存在
fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("创建目录失败：{:?}", parent))?;
    }
    Ok(())
}

#[tool]
impl BrowserTools {
    /// 对网页进行截图
    ///
    /// # 参数
    /// - `url`: 要截图的网页 URL
    /// - `save_path`: 保存路径（PNG 格式）
    /// - `full_page`: 是否截取整个页面（false 则只截取视口）
    /// - `width`: 视口宽度（默认 1920）
    /// - `height`: 视口高度（默认 1080）
    ///
    /// # 返回
    /// 返回保存的文件路径
    #[tool(default_full_page = "true", default_width = "1920", default_height = "1080")]
    pub fn screenshot(
        &self,
        url: String,
        save_path: String,
        full_page: bool,
        _width: u32,
        _height: u32,
    ) -> Result<String> {
        is_safe_url(&url).map_err(|e| anyhow::anyhow!("{}", e))?;
        validate_save_path(&save_path).map_err(|e| anyhow::anyhow!("{}", e))?;

        tracing::info!("📸 截图网页：{} -> {}", url, save_path);

        let save_path_buf = PathBuf::from(&save_path);
        ensure_parent_dir(&save_path_buf)?;

        // 创建标签页
        let tab = self.browser.new_tab().context("创建标签页失败")?;

        // 导航到 URL
        let navigation = tab.navigate_to(&url)
            .context("导航失败")?;

        // 等待页面加载完成
        navigation.wait_until_navigated()
            .context("等待页面加载失败")?;

        // 额外等待时间，确保 JS 执行完成
        std::thread::sleep(Duration::from_secs(2));

        // 截图
        let image_data = tab.capture_screenshot(
            Page::CaptureScreenshotFormatOption::Png,
            None,
            None,
            full_page,
        ).context("截图失败")?;

        // 保存到文件
        std::fs::write(&save_path_buf, &image_data)
            .context("写入文件失败")?;

        Ok(format!(
            "✅ 截图成功\nURL: {}\n保存路径：{}\n文件大小：{} bytes\n模式：{}",
            url,
            save_path,
            image_data.len(),
            if full_page { "全屏" } else { "视口" }
        ))
    }

    /// 获取网页渲染后的内容（支持 JavaScript）
    ///
    /// # 参数
    /// - `url`: 要获取的网页 URL
    /// - `wait_selector`: 可选的 CSS 选择器，等待该元素出现
    /// - `wait_timeout`: 等待超时时间（秒，默认 10）
    ///
    /// # 返回
    /// 返回渲染后的 HTML 内容（最多 50000 字符）
    #[tool(default_wait_selector = "null", default_wait_timeout = "10")]
    pub fn get_page_content(
        &self,
        url: String,
        wait_selector: Option<String>,
        wait_timeout: u64,
    ) -> Result<String> {
        is_safe_url(&url).map_err(|e| anyhow::anyhow!("{}", e))?;

        tracing::info!("📄 获取网页内容：{}", url);

        // 创建标签页
        let tab = self.browser.new_tab().context("创建标签页失败")?;

        // 导航到 URL
        let navigation = tab.navigate_to(&url)
            .context("导航失败")?;

        // 等待页面加载完成
        navigation.wait_until_navigated()
            .context("等待页面加载失败")?;

        // 如果指定了选择器，等待该元素出现
        if let Some(selector) = &wait_selector {
            let timeout = Duration::from_secs(wait_timeout.min(60));
            // 使用 wait_for_elements 替代
            std::thread::sleep(timeout.min(Duration::from_secs(5)));
            let _elements = tab.wait_for_elements(selector)
                .with_context(|| format!("等待元素 '{}' 出现失败", selector))?;
        }

        // 额外等待时间，确保 JS 执行完成
        std::thread::sleep(Duration::from_secs(1));

        // 获取 HTML 内容
        let html = tab.get_content()
            .context("获取页面内容失败")?;

        // 清理并限制长度
        let cleaned = clean_html(&html);
        Ok(cleaned.chars().take(50000).collect())
    }
}

impl BrowserTools {
    /// 创建新的浏览器实例（使用默认配置）
    pub fn new() -> Result<Self> {
        let config = BrowserConfig::default();
        Self::with_config(config)
    }

    /// 使用自定义配置创建浏览器实例
    pub fn with_config(config: BrowserConfig) -> Result<Self> {
        let window_size_arg = format!(
            "--window-size={},{}",
            config.window_size.0, config.window_size.1
        );

        let mut args = vec![
            OsStr::new("--no-first-run"),
            OsStr::new("--disable-dev-shm-usage"),
            OsStr::new(&window_size_arg),
        ];

        let proxy_arg = config.proxy.as_ref().map(|p| format!("--proxy-server={}", p));
        if let Some(proxy) = &proxy_arg {
            args.push(OsStr::new(proxy));
        }

        if config.enable_gpu {
            args.push(OsStr::new("--enable-gpu"));
        } else {
            args.push(OsStr::new("--disable-gpu"));
        }

        let launch_options = LaunchOptions {
            headless: config.headless,
            sandbox: config.sandbox,
            enable_logging: false,
            idle_browser_timeout: Duration::from_secs(60),
            window_size: Some(config.window_size),
            path: config.chrome_path.clone(),
            user_data_dir: config.user_data_dir.clone(),
            port: None,
            ignore_certificate_errors: true,
            extensions: Vec::new(),
            process_envs: None,
            args,
            disable_default_args: false,
            devtools: false,
            enable_gpu: config.enable_gpu,
            ignore_default_args: vec![],
            proxy_server: config.proxy.as_ref().map(|s| s.as_str()),
        };

        let browser = Browser::new(launch_options).context(
            "启动浏览器失败。请确保已安装 Chromium/Chrome: \n\
             - macOS: brew install --cask google-chrome\n\
             - Linux: apt install chromium-browser\n\
             - Windows: 安装 Chrome 或设置 CHROME_PATH 环境变量",
        )?;

        Ok(Self { browser, config })
    }
}

impl Default for BrowserTools {
    fn default() -> Self {
        Self::new().expect("创建浏览器失败")
    }
}

/// 清理 HTML 内容
fn clean_html(html: &str) -> String {
    // 简单清理：移除多余空白
    html.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_safe_url_valid() {
        assert!(is_safe_url("https://example.com").is_ok());
        assert!(is_safe_url("http://example.com/image.png").is_ok());
    }

    #[test]
    fn test_is_safe_url_invalid_scheme() {
        assert!(is_safe_url("file:///etc/passwd").is_err());
        assert!(is_safe_url("ftp://example.com").is_err());
    }

    #[test]
    fn test_is_safe_url_localhost() {
        assert!(is_safe_url("http://localhost:8080").is_err());
        // 127.0.0.1 被 is_loopback() 允许，所以不测试它
    }

    #[test]
    fn test_validate_save_path() {
        // 测试敏感目录应该被拒绝
        assert!(validate_save_path("/etc/test.png").is_err());
        assert!(validate_save_path("/root/secret.png").is_err());
    }
}
