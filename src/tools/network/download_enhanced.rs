use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use reqwest::blocking::{Client, Response};
use tokitai::tool;

/// 下载进度回调类型
/// 参数：已下载字节数、总字节数、进度 (0.0-1.0)
pub type ProgressCallback = Arc<dyn Fn(u64, u64, f32) + Send + Sync>;

/// 下载配置
#[derive(Clone)]
pub struct DownloadConfig {
    /// 分块大小（默认 8KB）
    pub chunk_size: usize,
    /// 最大重试次数（默认 3）
    pub max_retries: u32,
    /// 是否支持断点续传（默认 true）
    pub resume_enabled: bool,
    /// 进度回调
    pub on_progress: Option<ProgressCallback>,
    /// 限速（字节/秒，None 表示不限速）
    pub speed_limit: Option<u64>,
    /// 请求超时（秒，默认 600）
    pub timeout_secs: u64,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            chunk_size: 8 * 1024,
            max_retries: 3,
            resume_enabled: true,
            on_progress: None,
            speed_limit: None,
            timeout_secs: 600,
        }
    }
}

/// 下载器 - 支持断点续传、进度回调和限速
pub struct Downloader {
    client: Client,
    config: DownloadConfig,
}

impl Downloader {
    pub fn new(config: DownloadConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .connect_timeout(Duration::from_secs(30))
            .build()
            .expect("创建 HTTP 客户端失败");

        Self { client, config }
    }

    /// 下载文件（支持断点续传和进度回调）
    pub fn download(&self, url: &str, save_path: &Path) -> Result<u64, String> {
        let mut downloaded = 0u64;

        // 确保父目录存在
        if let Some(parent) = save_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("创建目录失败：{}", e))?;
        }

        // 打开文件
        let mut file = self.open_file(save_path)?;
        let mut start_pos = file
            .metadata()
            .map(|m| m.len())
            .unwrap_or(0);

        // 检查是否支持断点续传
        let supports_range = self.check_range_support(url)?;

        if supports_range && self.config.resume_enabled && start_pos > 0 {
            tracing::info!("恢复下载，起始位置：{}", start_pos);
            downloaded = start_pos;
        } else {
            // 重新下载，清空文件
            if start_pos > 0 {
                file.set_len(0).map_err(|e| e.to_string())?;
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
        let report_interval = Duration::from_millis(500); // 每 500ms 报告一次
        let mut last_speed_report = std::time::Instant::now();
        let mut last_downloaded = downloaded;

        loop {
            let n = response
                .read(&mut buffer)
                .map_err(|e| format!("读取数据失败：{}", e))?;

            if n == 0 {
                break; // 下载完成
            }

            file.write_all(&buffer[..n])
                .map_err(|e| format!("写入文件失败：{}", e))?;

            downloaded += n as u64;

            // 限速
            if let Some(limit) = self.config.speed_limit {
                if limit > 0 {
                    let expected_time = downloaded as f64 / limit as f64;
                    let actual_time = last_report.elapsed().as_secs_f64();
                    if actual_time < expected_time {
                        std::thread::sleep(Duration::from_secs_f64(
                            expected_time - actual_time,
                        ));
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

        file.sync_all().map_err(|e| e.to_string())?;

        tracing::info!("下载完成：{} bytes", downloaded);
        Ok(downloaded)
    }

    /// 检查服务器是否支持 Range 请求
    fn check_range_support(&self, url: &str) -> Result<bool, String> {
        let response = self
            .client
            .head(url)
            .send()
            .map_err(|e| e.to_string())?;

        // 检查 Accept-Ranges 头
        response
            .headers()
            .get("accept-ranges")
            .map(|v| v == "bytes")
            .or_else(|| {
                // 或者检查 Content-Range 头
                response
                    .headers()
                    .get("content-range")
                    .map(|_| true)
            })
            .unwrap_or(false)
            .then(|| true)
            .ok_or_else(|| "服务器不支持断点续传".to_string())
    }

    /// 发送带 Range 头的请求
    fn send_request(&self, url: &str, start: u64) -> Result<Response, String> {
        let mut req = self.client.get(url);

        if start > 0 {
            req = req.header("Range", format!("bytes={}-", start));
        }

        req.send().map_err(|e| format!("发送请求失败：{}", e))
    }

    /// 打开文件（支持续传）
    fn open_file(&self, path: &Path) -> Result<File, String> {
        OpenOptions::new()
            .write(true)
            .create(true)
            .read(true) // 需要读取以支持续传
            .open(path)
            .map_err(|e| format!("打开文件失败：{}", e))
    }
}

/// 下载工具集 - 增强版
pub struct DownloadToolsEnhanced;

#[tool]
impl DownloadToolsEnhanced {
    /// 下载文件（增强版，支持进度和断点续传）
    ///
    /// # 参数
    /// - `url`: 文件 URL
    /// - `save_path`: 保存路径
    /// - `resume`: 是否启用断点续传（默认 true）
    /// - `speed_limit`: 限速 (KB/s)，0 表示不限速（默认 0）
    pub fn download_file_advanced(
        &self,
        url: String,
        save_path: String,
        resume: Option<bool>,
        speed_limit: Option<u32>,
    ) -> Result<String, String> {
        let speed_limit_bytes = speed_limit.map(|limit| {
            if limit == 0 {
                None
            } else {
                Some(limit as u64 * 1024)
            }
        }).flatten();

        let config = DownloadConfig {
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

    /// 下载文件（简单版，兼容原有接口）
    pub fn download_file(
        &self,
        url: String,
        save_path: String,
    ) -> Result<String, String> {
        let config = DownloadConfig::default();
        let downloader = Downloader::new(config);
        let size = downloader.download(&url, Path::new(&save_path))?;

        Ok(format!(
            "✅ 下载完成\nURL: {}\n路径：{}\n大小：{} bytes",
            url, save_path, size
        ))
    }
}

impl Default for DownloadToolsEnhanced {
    fn default() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_download_config_default() {
        let config = DownloadConfig::default();
        assert_eq!(config.chunk_size, 8 * 1024);
        assert_eq!(config.max_retries, 3);
        assert!(config.resume_enabled);
        assert!(config.speed_limit.is_none());
    }

    #[test]
    fn test_download_config_clone() {
        let config = DownloadConfig::default();
        let cloned = config.clone();
        assert_eq!(config.chunk_size, cloned.chunk_size);
    }
}
