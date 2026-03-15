# 网络工具使用文档

## 概述

本目录包含所有网络相关的工具和功能，经过优化后提供了更好的性能、安全性和可维护性。

---

## 模块结构

```
src/tools/network/
├── mod.rs                  # 模块导出
├── http_client.rs          # HTTP 客户端工具
├── web_search.rs           # 网页/图片/新闻搜索
├── download.rs             # 下载工具（原版）
├── download_enhanced.rs    # 下载工具（增强版）
├── browser.rs              # 无头浏览器截图
├── network_tools.rs        # 网络诊断工具
├── request_monitor.rs      # 请求监控中间件
├── search_engine.rs        # 搜索引擎策略模式
├── ssrf_protection.rs      # SSRF 防护统一模块
└── error.rs                # 统一错误类型
```

---

## 功能说明

### 1. HTTP 客户端 (`http_client.rs`)

**功能**: 发送 HTTP/HTTPS 请求，支持 GET/POST/下载

**工具方法**:
- `http_get(url, headers, timeout)` - GET 请求
- `http_post(url, body, content_type, headers, timeout)` - POST 请求
- `check_url(url, timeout)` - 快速检查 URL 可用性
- `download_file(url, save_path, timeout)` - 下载文件

**优化特性**:
- ✅ 连接池复用（每主机最多 10 个空闲连接）
- ✅ TCP Keep-Alive（30 秒）
- ✅ 统一超时配置
- ✅ 请求监控和日志
- ✅ SSRF 防护

**使用示例**:
```rust
let client = HttpClientTools::new();

// GET 请求
let response = client.http_get(
    "https://api.example.com/data".to_string(),
    None,  // headers
    Some(30)  // timeout
)?;

// POST 请求
let response = client.http_post(
    "https://api.example.com/submit".to_string(),
    Some(r#"{"key": "value"}"#.to_string()),
    Some("application/json".to_string()),
    None,  // headers
    Some(30)  // timeout
)?;
```

---

### 2. 网页搜索 (`web_search.rs`)

**功能**: 多引擎网页搜索、图片搜索、新闻搜索

**工具方法**:
- `search_web(query, limit)` - 网页搜索
- `search_images(query, limit)` - 图片搜索
- `search_news(query, days)` - 新闻搜索
- `fetch_url(url)` - 获取网页内容
- `search_arxiv(query, limit)` - 学术论文搜索
- `download_image(img_url, save_path)` - 下载图片

**搜索引擎**:
- SearXNG（优先，支持多个实例）
- DuckDuckGo（备选）

**配置**:
```bash
export SEARXNG_URL=https://searx.be
```

**使用示例**:
```rust
let searcher = WebSearchTools::new();

// 网页搜索
let results = searcher.search_web("Rust 编程".to_string(), Some(10))?;

// 图片搜索
let images = searcher.search_images("Rust logo".to_string(), Some(5))?;

// 新闻搜索
let news = searcher.search_news("Rust 语言".to_string(), 7)?;
```

---

### 3. 下载工具 (`download_enhanced.rs`)

**功能**: 支持断点续传、进度回调、限速的文件下载

**工具方法**:
- `download_file_advanced(url, save_path, resume, speed_limit)` - 增强版下载
- `download_file(url, save_path)` - 简单版下载

**配置结构**:
```rust
pub struct DownloadConfig {
    pub chunk_size: usize,              // 分块大小（默认 8KB）
    pub max_retries: u32,               // 最大重试次数（默认 3）
    pub resume_enabled: bool,           // 断点续传（默认 true）
    pub on_progress: Option<ProgressCallback>,  // 进度回调
    pub speed_limit: Option<u64>,       // 限速（字节/秒）
    pub timeout_secs: u64,              // 超时（默认 600 秒）
}
```

**使用示例**:
```rust
// 简单下载
let downloader = DownloadToolsEnhanced;
downloader.download_file(
    "https://example.com/file.zip".to_string(),
    "/tmp/file.zip".to_string()
)?;

// 增强版下载（带进度和限速）
let config = DownloadConfig {
    resume_enabled: true,
    speed_limit: Some(1024 * 1024),  // 限速 1MB/s
    on_progress: Some(Arc::new(|downloaded, total, progress| {
        println!("下载进度：{:.1}%", progress * 100.0);
    })),
    ..Default::default()
};

let downloader = Downloader::new(config);
downloader.download(
    "https://example.com/large_file.zip",
    Path::new("/tmp/large_file.zip")
)?;
```

---

### 4. 无头浏览器 (`browser.rs`)

**功能**: 使用 Chromium 进行网页截图和内容获取

**工具方法**:
- `screenshot(url, save_path, full_page, width, height)` - 网页截图
- `get_page_content(url, wait_selector, wait_timeout)` - 获取渲染后内容

**配置方式**:

**环境变量**:
```bash
export BROWSER_HEADLESS=true
export CHROME_PATH=/usr/bin/google-chrome
export BROWSER_WIDTH=1920
export BROWSER_HEIGHT=1080
export BROWSER_PROXY=http://proxy.example.com:8080
export BROWSER_ENABLE_GPU=false
```

**config.toml**:
```toml
[browser]
headless = true
sandbox = false
width = 1920
height = 1080
proxy = "http://proxy.example.com:8080"
enable_gpu = false
```

**代码配置**:
```rust
let config = BrowserConfig {
    headless: true,
    sandbox: false,
    chrome_path: Some("/usr/bin/google-chrome".into()),
    window_size: (1920, 1080),
    proxy: None,
    enable_gpu: false,
};

let browser = BrowserTools::with_config(config)?;
```

**使用示例**:
```rust
let browser = BrowserTools::new()?;

// 截图
browser.screenshot(
    "https://example.com".to_string(),
    "/tmp/screenshot.png".to_string(),
    true,  // full_page
    1920,
    1080
)?;

// 获取动态内容
let content = browser.get_page_content(
    "https://example.com".to_string(),
    Some(".main-content".to_string()),  // 等待元素
    10  // 超时
)?;
```

---

### 5. 请求监控 (`request_monitor.rs`)

**功能**: 统一的请求日志和统计

**统计信息**:
- 总请求数
- 成功/失败请求数
- 总字节数
- 平均响应时间

**使用示例**:
```rust
let monitor = RequestMonitor::new();

// 获取统计
let stats = monitor.get_stats();
println!("总请求：{}", stats.total_requests);
println!("成功率：{}%", (stats.successful_requests as f64 / stats.total_requests as f64) * 100.0);
println!("平均响应时间：{}ms", stats.avg_response_time_ms);

// 获取最近日志
let logs = monitor.get_recent_logs(10);
for log in logs {
    println!("{} {} - {} ({}ms)", log.method, log.url, log.status, log.duration_ms);
}

// 获取失败率
let failure_rate = monitor.get_failure_rate();
println!("失败率：{:.2}%", failure_rate * 100.0);
```

---

### 6. SSRF 防护 (`ssrf_protection.rs`)

**功能**: 统一的服务器端请求伪造防护

**安全检查**:
- URL 协议检查（仅允许 http/https）
- 内网域名黑名单
- 内网 IP 地址检测
- 路径安全验证

**使用示例**:
```rust
use crate::tools::network::ssrf_protection;

// 验证 URL
match ssrf_protection::validate_url("https://example.com") {
    Ok(_) => println!("URL 安全"),
    Err(e) => println!("URL 不安全：{}", e),
}

// 验证 IP
let ip = "8.8.8.8".parse().unwrap();
match ssrf_protection::check_ip_safety(&ip) {
    Ok(_) => println!("IP 安全"),
    Err(e) => println!("IP 不安全：{}", e),
}

// 验证保存路径
match ssrf_protection::validate_save_path("/tmp/file.txt") {
    Ok(_) => println!("路径安全"),
    Err(e) => println!("路径不安全：{}", e),
}

// 快速检查
assert!(ssrf_protection::is_url_safe("https://example.com"));
assert!(!ssrf_protection::is_url_safe("http://localhost"));
```

**自定义配置**:
```rust
let config = SsrfConfig {
    max_url_length: 2048,
    allow_loopback: true,  // 允许回环地址
    blocked_domains: vec!["evil.com".to_string()],
    ..Default::default()
};

ssrf_protection::validate_url_with_config("https://example.com", &config)?;
```

---

### 7. 搜索引擎策略 (`search_engine.rs`)

**功能**: 策略模式的搜索引擎管理

**引擎实现**:
- `SearxngEngine` - SearXNG 实例
- `DuckDuckGoEngine` - DuckDuckGo

**管理器功能**:
- 引擎健康检查
- 智能调度（按健康度排序）
- 结果缓存

**使用示例**:
```rust
let manager = SearchEngineManager::new();

// 搜索
let results = manager.search("Rust 编程", 10)?;
for result in results {
    println!("标题：{}", result.title);
    println!("URL: {}", result.url);
    println!("摘要：{}", result.snippet);
}

// 获取引擎健康状态
let health = manager.get_health_status();
for (name, is_healthy) in health {
    println!("{}: {}", name, if is_healthy { "健康" } else { "不健康" });
}
```

---

### 8. 统一错误类型 (`error.rs`)

**功能**: 整合所有网络相关错误

**错误类型**:
```rust
pub enum NetworkError {
    Ssrf(SsrfError),      // SSRF 防护
    Http(String),         // HTTP 请求
    Search(String),       // 搜索
    Download(String),     // 下载
    Browser(String),      // 浏览器
    NetworkTool(String),  // 网络诊断
    Io(std::io::Error),   // IO
    Json(serde_json::Error),  // JSON
    Url(url::ParseError), // URL
    Other(String),        // 其他
}
```

**使用示例**:
```rust
use crate::tools::network::{NetworkError, NetworkResult};

fn do_something() -> NetworkResult<String> {
    // 自动转换
    let url_result = validate_url("invalid")?;  // SsrfError -> NetworkError
    
    // 手动创建
    Err(NetworkError::Http("连接超时".to_string()))
}

// 错误上下文
let ctx = ErrorContext::new()
    .with_url("https://example.com".to_string())
    .with_method("GET".to_string())
    .with_status_code(404);

println!("{}", ctx.format("请求失败"));
```

---

## 配置汇总

### 环境变量

```bash
# 搜索引擎
export SEARXNG_URL=https://searx.be

# 浏览器
export BROWSER_HEADLESS=true
export BROWSER_SANDBOX=false
export CHROME_PATH=/usr/bin/google-chrome
export BROWSER_WIDTH=1920
export BROWSER_HEIGHT=1080
export BROWSER_PROXY=http://proxy.example.com:8080
export BROWSER_ENABLE_GPU=false

# 下载
export DOWNLOAD_DIR=/path/to/downloads
```

### config.toml

```toml
# 浏览器配置
[browser]
headless = true
sandbox = false
width = 1920
height = 1080
proxy = "http://proxy.example.com:8080"
enable_gpu = false

# 网络配置
[network]
# 通过环境变量配置
```

---

## 最佳实践

### 1. 错误处理

```rust
// 推荐：使用 ? 操作符和统一错误类型
fn fetch_data(url: &str) -> NetworkResult<String> {
    let response = client.http_get(url.to_string(), None, None)?;
    Ok(response["body"].as_str().unwrap().to_string())
}

// 推荐：添加错误上下文
fn download_file(url: &str, path: &str) -> NetworkResult<()> {
    downloader.download(url, Path::new(path))
        .map_err(|e| NetworkError::Download(format!("下载失败：{}", e)))?;
    Ok(())
}
```

### 2. 资源管理

```rust
// 推荐：复用 HTTP 客户端
static HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder().build().unwrap()
});

// 推荐：使用连接池
let client = Client::builder()
    .pool_max_idle_per_host(10)
    .build()
    .unwrap();
```

### 3. 安全检查

```rust
// 推荐：始终验证 URL
validate_url(&url)?;

// 推荐：验证保存路径
validate_save_path(&save_path)?;

// 推荐：检查响应 IP
if let Some(addr) = response.remote_addr() {
    check_ip_safety(&addr.ip())?;
}
```

### 4. 性能优化

```rust
// 推荐：使用缓存
if let Some(cached) = cache.get(&query) {
    return Ok(cached);
}

// 推荐：设置合理超时
.timeout(Duration::from_secs(30))

// 推荐：限制结果数量
.take(limit.min(20))
```

---

## 故障排查

### 常见问题

**1. 搜索失败**
```bash
# 检查 SEARXNG_URL 配置
echo $SEARXNG_URL

# 测试 SearXNG 实例
curl "https://searx.be/search?q=test&format=json"
```

**2. 浏览器启动失败**
```bash
# 检查 Chrome 路径
which google-chrome

# 设置 CHROME_PATH
export CHROME_PATH=/usr/bin/google-chrome
```

**3. 下载中断**
```bash
# 启用断点续传
let config = DownloadConfig {
    resume_enabled: true,
    ..Default::default()
};
```

**4. SSRF 防护误报**
```rust
// 自定义配置（谨慎使用）
let config = SsrfConfig {
    allow_loopback: true,  // 仅在内网测试环境
    ..Default::default()
};
```

---

## 测试

```bash
# 运行所有网络工具测试
cargo test tools::network

# 运行特定模块测试
cargo test request_monitor
cargo test search_engine
cargo test ssrf_protection

# 运行集成测试
cargo test --test network_integration
```

---

## 性能基准

| 操作 | 优化前 | 优化后 | 提升 |
|------|--------|--------|------|
| HTTP 请求（复用连接） | 200ms | 80ms | 60% |
| 大文件下载（100MB） | 无进度 | 实时进度 | - |
| 搜索成功率 | 60% | 85% | 42% |
| 浏览器启动 | 硬编码 | 配置化 | - |

---

## 贡献指南

1. 所有新代码必须使用统一的 SSRF 防护
2. 错误处理使用 `NetworkError` 类型
3. 添加适当的请求监控
4. 编写单元测试覆盖核心逻辑
