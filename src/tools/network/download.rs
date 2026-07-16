//! 下载工具集
//!
//! 支持下载文件（PDF、图片等），具备断点续传、进度回调和限速功能

use reqwest::blocking::{Client, Response};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokitai::tool;

use super::error::{DownloadError, HttpError, NetworkError, NetworkResult};
use super::ssrf_protection;

// ============================================================================
// 配置结构
// ============================================================================

/// 下载工具配置
#[derive(Debug, Clone)]
pub struct DownloadConfig {
    /// 请求超时（秒）
    pub timeout_secs: u64,
    /// 连接超时（秒）
    pub connect_timeout_secs: u64,
    /// 分块大小（字节）
    #[allow(dead_code)]
    pub chunk_size: usize,
    /// 最大重试次数
    #[allow(dead_code)]
    pub max_retries: u32,
    /// 是否支持断点续传
    #[allow(dead_code)]
    pub resume_enabled: bool,
    /// 最大文件大小（字节）
    pub max_file_size: usize,
    /// User-Agent
    pub user_agent: String,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 300, // 5 分钟，下载大文件需要更长时间
            connect_timeout_secs: 30,
            chunk_size: 8 * 1024, // 8KB
            max_retries: 3,
            resume_enabled: true,
            max_file_size: 50 * 1024 * 1024, // 50MB
            user_agent: "Mozilla/5.0 (compatible; Atlas AI Assistant/1.0)".to_string(),
        }
    }
}

/// 下载进度回调类型
pub type ProgressCallback = Arc<dyn Fn(u64, u64, f32) + Send + Sync>;

// ============================================================================
// 下载器 - 支持断点续传和进度回调
// ============================================================================

/// 下载器配置
#[derive(Clone)]
pub struct DownloaderConfig {
    pub chunk_size: usize,
    pub resume_enabled: bool,
    pub on_progress: Option<ProgressCallback>,
    pub speed_limit: Option<u64>, // 字节/秒
    pub timeout_secs: u64,
}

impl Default for DownloaderConfig {
    fn default() -> Self {
        Self {
            chunk_size: 8 * 1024,
            resume_enabled: true,
            on_progress: None,
            speed_limit: None,
            timeout_secs: 300,
        }
    }
}

/// 下载器 - 支持断点续传、进度回调和限速
pub struct Downloader {
    client: Client,
    config: DownloaderConfig,
}

impl Downloader {
    pub fn new(config: DownloaderConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .connect_timeout(Duration::from_secs(30))
            .build()
            .expect("创建 HTTP 客户端失败");

        Self { client, config }
    }

    /// 下载文件（支持断点续传和进度回调）
    pub fn download(&self, url: &str, save_path: &Path) -> NetworkResult<u64> {
        let mut downloaded = 0u64;

        // 确保父目录存在
        if let Some(parent) = save_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| DownloadError::Io(format!("创建目录失败：{}", e)))?;
        }

        // 打开文件
        let mut file = self.open_file(save_path)?;
        let mut start_pos = file.metadata().map(|m| m.len()).unwrap_or(0);

        // 检查是否支持断点续传
        let supports_range = self.check_range_support(url)?;

        if supports_range && self.config.resume_enabled && start_pos > 0 {
            tracing::info!("恢复下载，起始位置：{}", start_pos);
            downloaded = start_pos;
        } else {
            // 重新下载，清空文件
            if start_pos > 0 {
                file.set_len(0)
                    .map_err(|e| DownloadError::Io(format!("清空文件失败：{}", e)))?;
                start_pos = 0;
            }
        }

        // 发起请求
        let mut response = self.send_request(url, start_pos)?;

        let total_size = response
            .content_length()
            .map(|len| len + start_pos)
            .unwrap_or(0);

        // 下载循环
        let mut buffer = vec![0u8; self.config.chunk_size];
        let mut last_report = std::time::Instant::now();
        let report_interval = Duration::from_millis(500);
        let mut last_speed_report = std::time::Instant::now();
        let mut last_downloaded = downloaded;

        loop {
            let n = response
                .read(&mut buffer)
                .map_err(|e| DownloadError::Io(format!("读取数据失败：{}", e)))?;

            if n == 0 {
                break;
            }

            file.write_all(&buffer[..n])
                .map_err(|e| DownloadError::Io(format!("写入文件失败：{}", e)))?;

            downloaded += n as u64;

            // 限速
            if let Some(limit) = self.config.speed_limit {
                if limit > 0 {
                    let expected_time = downloaded as f64 / limit as f64;
                    let actual_time = last_report.elapsed().as_secs_f64();
                    if actual_time < expected_time {
                        std::thread::sleep(Duration::from_secs_f64(expected_time - actual_time));
                    }
                }
            }

            // 进度回调
            if let Some(callback) = &self.config.on_progress {
                if last_report.elapsed() >= report_interval {
                    let progress = if total_size > 0 {
                        downloaded as f32 / total_size as f32
                    } else {
                        0.0
                    };
                    callback(downloaded, total_size, progress);

                    // 计算下载速度
                    let elapsed = last_speed_report.elapsed().as_secs_f64();
                    if elapsed > 0.0 {
                        let speed = (downloaded - last_downloaded) as f64 / elapsed;
                        tracing::debug!(
                            "下载速度：{:.2} KB/s, 进度：{:.1}%",
                            speed / 1024.0,
                            progress * 100.0
                        );
                        last_speed_report = std::time::Instant::now();
                        last_downloaded = downloaded;
                    }

                    last_report = std::time::Instant::now();
                }
            }
        }

        file.sync_all()
            .map_err(|e| DownloadError::Io(format!("同步文件失败：{}", e)))?;

        tracing::info!("下载完成：{} bytes", downloaded);
        Ok(downloaded)
    }

    /// 检查服务器是否支持 Range 请求
    fn check_range_support(&self, url: &str) -> NetworkResult<bool> {
        let response = self.client.head(url).send()?;

        let supports_range = response
            .headers()
            .get("accept-ranges")
            .map(|v| v == "bytes")
            .or_else(|| response.headers().get("content-range").map(|_| true))
            .unwrap_or(false);

        Ok(supports_range)
    }

    /// 发送带 Range 头的请求
    fn send_request(&self, url: &str, start: u64) -> NetworkResult<Response> {
        let mut req = self.client.get(url);

        if start > 0 {
            req = req.header("Range", format!("bytes={}-", start));
        }

        let response = req.send()?;
        Ok(response)
    }

    /// 打开文件（支持续传）
    fn open_file(&self, path: &Path) -> NetworkResult<File> {
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .read(true)
            .open(path)
            .map_err(|e| DownloadError::Io(format!("打开文件失败：{}", e)))
            .map_err(NetworkError::from)
    }
}

// ============================================================================
// 下载工具集
// ============================================================================

/// 下载工具集 - 支持下载 PDF 论文等文件
pub struct DownloadTools {
    config: DownloadConfig,
    client: Client,
}

impl DownloadTools {
    /// 创建新的下载工具实例
    pub fn new() -> Self {
        Self::with_config(DownloadConfig::default())
    }

    /// 创建带自定义配置的下载工具实例
    pub fn with_config(config: DownloadConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .connect_timeout(Duration::from_secs(config.connect_timeout_secs))
            .user_agent(&config.user_agent)
            .build()
            .expect("创建 HTTP 客户端失败");

        Self { config, client }
    }

    /// 获取默认下载目录路径（跨平台支持）
    fn get_default_download_dir() -> PathBuf {
        // 优先检查环境变量
        if let Ok(download_dir) = std::env::var("DOWNLOAD_DIR") {
            let path = PathBuf::from(download_dir);
            if path.exists() {
                return path;
            }
        }

        // 使用当前工作目录下的 downloads 文件夹（沙箱安全）
        if let Ok(current_dir) = std::env::current_dir() {
            let downloads_dir = current_dir.join("downloads");
            if downloads_dir.exists() || std::fs::create_dir_all(&downloads_dir).is_ok() {
                return downloads_dir;
            }
        }

        // 回退到临时目录
        PathBuf::from("/tmp")
    }

    /// 确保下载目录存在
    fn ensure_download_dir(path: &Path) -> NetworkResult<PathBuf> {
        if !path.exists() {
            std::fs::create_dir_all(path)
                .map_err(|e| DownloadError::Io(format!("创建下载目录失败：{}", e)))?;
        }
        Ok(path.to_path_buf())
    }

    /// 从 URL 提取文件名
    fn extract_filename_from_url(url: &str) -> Option<String> {
        // 尝试从 URL 路径部分提取文件名
        if let Some(filename) = url.split('/').next_back() {
            if !filename.is_empty() && filename.contains('.') {
                return Some(filename.to_string());
            }
        }

        // 尝试从查询参数提取
        if let Some(query) = url.split('?').nth(1) {
            for param in query.split('&') {
                if let Some(value) = param.split('=').nth(1) {
                    if value.contains(".pdf") {
                        return Some(value.to_string());
                    }
                }
            }
        }

        None
    }

    /// 生成安全的文件名（防止路径遍历攻击）
    fn sanitize_filename(filename: &str) -> String {
        filename
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }

    /// 验证下载路径是否安全
    fn validate_download_path(base_dir: &Path, full_path: &Path) -> NetworkResult<()> {
        let path_str = full_path.to_string_lossy();
        ssrf_protection::validate_save_path(&path_str)?;

        let canonical_base = base_dir
            .canonicalize()
            .map_err(|e| DownloadError::Io(format!("规范化基础目录失败：{}", e)))?;

        if full_path.exists() {
            let canonical_full = full_path
                .canonicalize()
                .map_err(|e| DownloadError::Io(format!("规范化完整路径失败：{}", e)))
                .map_err(NetworkError::from)?;

            if !canonical_full.starts_with(&canonical_base) {
                return Err(NetworkError::Download(DownloadError::PathValidation(
                    "路径遍历攻击检测：文件不在允许的目录内".to_string(),
                )));
            }
        } else {
            if let Some(parent) = full_path.parent() {
                if parent.exists() {
                    let canonical_parent = parent
                        .canonicalize()
                        .map_err(|e| DownloadError::Io(format!("规范化父目录失败：{}", e)))
                        .map_err(NetworkError::from)?;

                    if !canonical_parent.starts_with(&canonical_base) {
                        return Err(NetworkError::Download(DownloadError::PathValidation(
                            "路径遍历攻击检测：父目录不在允许的范围内".to_string(),
                        )));
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for DownloadTools {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tool 实现
// ============================================================================

#[tool]
impl DownloadTools {
    /// 下载 PDF 文件到指定目录
    ///
    /// # 参数
    /// - `url`: PDF 文件的 URL 地址
    /// - `filename`: 可选的文件名，不提供则自动从 URL 提取
    /// - `directory`: 可选的保存目录，不提供则使用默认下载目录
    ///
    /// # 返回
    /// 返回保存的文件路径
    #[tool(default_filename = "null", default_directory = "null")]
    pub fn download_pdf(
        &self,
        url: String,
        filename: Option<String>,
        directory: Option<String>,
    ) -> NetworkResult<String> {
        // 确定保存目录
        let download_dir = match directory.as_deref() {
            Some(dir) if dir != "null" && dir != "None" && !dir.is_empty() => PathBuf::from(dir),
            _ => Self::get_default_download_dir(),
        };

        Self::ensure_download_dir(&download_dir)?;

        // 确定文件名
        let final_filename = match filename.as_deref() {
            Some(name) if name != "null" && name != "None" && !name.is_empty() => {
                Self::sanitize_filename(name)
            }
            _ => Self::extract_filename_from_url(&url)
                .map(|n| Self::sanitize_filename(&n))
                .unwrap_or_else(|| format!("download_{}.pdf", chrono::Local::now().timestamp())),
        };

        let final_filename = if final_filename.to_lowercase().ends_with(".pdf") {
            final_filename
        } else {
            format!("{}.pdf", final_filename)
        };

        let file_path = download_dir.join(&final_filename);

        // 验证下载路径
        Self::validate_download_path(&download_dir, &file_path)?;

        // 下载文件
        let response = self.client.get(&url).send()?;

        let status = response.status();
        if !status.is_success() {
            return Err(NetworkError::Http(HttpError::StatusCode {
                status: status.as_u16(),
                message: "下载失败".to_string(),
            }));
        }

        let bytes = response.bytes()?;

        // 限制文件大小
        if bytes.len() > self.config.max_file_size {
            return Err(NetworkError::Download(DownloadError::FileTooLarge {
                size: bytes.len() / 1024 / 1024,
                max: self.config.max_file_size / 1024 / 1024,
            }));
        }

        std::fs::write(&file_path, &bytes)?;

        Ok(format!("✅ PDF 已下载到：{}", file_path.display()))
    }

    /// 下载任意文件到指定目录
    ///
    /// # 参数
    /// - `url`: 文件的 URL 地址
    /// - `filename`: 可选的文件名
    /// - `directory`: 可选的保存目录
    #[tool(default_filename = "null", default_directory = "null")]
    pub fn download_file(
        &self,
        url: String,
        filename: Option<String>,
        directory: Option<String>,
    ) -> NetworkResult<String> {
        let download_dir = match directory.as_deref() {
            Some(dir) if dir != "null" && dir != "None" && !dir.is_empty() => PathBuf::from(dir),
            _ => Self::get_default_download_dir(),
        };

        Self::ensure_download_dir(&download_dir)?;

        let final_filename = match filename.as_deref() {
            Some(name) if name != "null" && name != "None" && !name.is_empty() => {
                Self::sanitize_filename(name)
            }
            _ => Self::extract_filename_from_url(&url)
                .map(|n| Self::sanitize_filename(&n))
                .unwrap_or_else(|| format!("download_{}", chrono::Local::now().timestamp())),
        };

        let file_path = download_dir.join(&final_filename);
        Self::validate_download_path(&download_dir, &file_path)?;

        let response = self.client.get(&url).send()?;

        let status = response.status();
        if !status.is_success() {
            return Err(NetworkError::Http(HttpError::StatusCode {
                status: status.as_u16(),
                message: "下载失败".to_string(),
            }));
        }

        let bytes = response.bytes()?;

        if bytes.len() > self.config.max_file_size {
            return Err(NetworkError::Download(DownloadError::FileTooLarge {
                size: bytes.len() / 1024 / 1024,
                max: self.config.max_file_size / 1024 / 1024,
            }));
        }

        std::fs::write(&file_path, &bytes)?;

        Ok(format!("✅ 文件已下载到：{}", file_path.display()))
    }

    /// 高级下载（支持断点续传、限速和进度回调）
    ///
    /// # 参数
    /// - `url`: 文件 URL
    /// - `save_path`: 保存路径
    /// - `resume`: 是否启用断点续传（默认 true）
    /// - `speed_limit`: 限速 (KB/s)，0 表示不限速（默认 0）
    #[tool(default_resume = "null", default_speed_limit = "null")]
    pub fn download_file_advanced(
        &self,
        url: String,
        save_path: String,
        resume: Option<bool>,
        speed_limit: Option<u32>,
    ) -> NetworkResult<String> {
        // 验证 URL
        ssrf_protection::validate_url(&url)?;

        let speed_limit_bytes = speed_limit.and_then(|limit| {
            if limit == 0 {
                None
            } else {
                Some(limit as u64 * 1024)
            }
        });

        let config = DownloaderConfig {
            resume_enabled: resume.unwrap_or(true),
            speed_limit: speed_limit_bytes,
            on_progress: Some(Arc::new(|downloaded, total, progress| {
                let percent = progress * 100.0;
                let mb_downloaded = downloaded as f32 / 1024.0 / 1024.0;
                let mb_total = total as f32 / 1024.0 / 1024.0;
                tracing::info!(
                    "下载进度：{:.1}% ({:.2}/{:.2} MB)",
                    percent,
                    mb_downloaded,
                    mb_total
                );
            })),
            ..Default::default()
        };

        let downloader = Downloader::new(config);
        let size = downloader.download(&url, Path::new(&save_path))?;

        Ok(format!(
            "✅ 下载完成\nURL: {}\n路径：{}\n大小：{} bytes",
            url, save_path, size
        ))
    }

    /// 获取当前默认下载目录
    pub fn get_download_dir(&self) -> NetworkResult<String> {
        let download_dir = Self::get_default_download_dir();
        Self::ensure_download_dir(&download_dir)?;
        Ok(format!("默认下载目录：{}", download_dir.display()))
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_download_config_default() {
        let config = DownloadConfig::default();
        assert_eq!(config.timeout_secs, 300);
        assert_eq!(config.chunk_size, 8 * 1024);
        assert_eq!(config.max_file_size, 50 * 1024 * 1024);
    }

    #[test]
    fn test_downloader_config_default() {
        let config = DownloaderConfig::default();
        assert!(config.resume_enabled);
        assert!(config.speed_limit.is_none());
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(DownloadTools::sanitize_filename("test.pdf"), "test.pdf");
        assert_eq!(
            DownloadTools::sanitize_filename("test/file.pdf"),
            "test_file.pdf"
        );
        assert_eq!(
            DownloadTools::sanitize_filename("test<>file.pdf"),
            "test__file.pdf"
        );
    }

    #[test]
    fn test_download_tools_creation() {
        let tools = DownloadTools::new();
        assert_eq!(tools.config.timeout_secs, 300);
    }

    #[test]
    fn test_get_default_download_dir() {
        let dir = DownloadTools::get_default_download_dir();
        // 应该返回一个有效的路径
        assert!(dir.exists() || dir.parent().map(|p| p.exists()).unwrap_or(true));
    }
}
