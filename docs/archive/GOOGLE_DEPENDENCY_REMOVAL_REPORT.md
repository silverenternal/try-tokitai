# 移除 Google 依赖报告

## 执行摘要

**完成时间**: 2026-03-14  
**目标**: 移除项目对 Google/Chrome 浏览器的依赖，保持轻量化  
**状态**: ✅ 完成

---

## 移除的依赖

### 1. Rust 库依赖

| 库 | 版本 | 用途 | 移除原因 |
|-----|------|------|----------|
| `headless_chrome` | 1.0 | 无头浏览器控制 | 重量级依赖，需要 Chrome/Chromium 二进制文件 |

**修改文件**: `Cargo.toml`

### 2. 源代码文件

| 文件 | 操作 | 原因 |
|------|------|------|
| `src/tools/network/browser.rs` | 删除 | 包含 screenshot 和 get_page_content 功能，依赖 headless_chrome |
| `src/tools/network/mod.rs` | 更新 | 移除 browser 模块导出 |
| `src/tools/mod.rs` | 更新 | 移除 BrowserTools 导出 |

### 3. 搜索引擎配置

| 文件 | 变更 |
|------|------|
| `config.toml.example` | engines 从 `["google", "bing", "duckduckgo"]` 改为 `["bing", "duckduckgo"]` |
| `src/tools/network/search_engine.rs` | SearXNG URL 从 `engines=google,bing,duckduckgo` 改为 `engines=bing,duckduckgo` |
| `src/tools/network/web_search.rs` | 移除 google_news、google_images、google 引擎引用 |

---

## 修改的文件清单

### 核心代码
- ✅ `Cargo.toml` - 移除 headless_chrome 依赖
- ✅ `src/tools/network/mod.rs` - 移除 browser 模块
- ✅ `src/tools/mod.rs` - 移除 BrowserTools 导出
- ✅ `src/main.rs` - 移除 AiAssistant 中的 browser_tools 字段

### 搜索引擎
- ✅ `src/tools/network/search_engine.rs` - 移除 google 引擎
- ✅ `src/tools/network/web_search.rs` - 移除 google_news、google_images

### 配置
- ✅ `config.toml.example` - 移除 google 引擎配置

### 测试脚本
- ✅ `test_image_features.sh` - 移除 chromium 检测逻辑

### 删除的文件
- ✅ `src/tools/network/browser.rs` (434 行)

---

## 功能影响分析

### 移除的功能

| 功能 | 工具名称 | 影响 | 替代方案 |
|------|----------|------|----------|
| 网页截图 | `screenshot()` | ❌ 移除 | 无需替代（非核心功能） |
| 获取渲染后内容 | `get_page_content()` | ❌ 移除 | 使用 ureq + scraper 获取静态内容 |

### 保留的功能

| 功能 | 状态 | 说明 |
|------|------|------|
| 网页搜索 | ✅ 保留 | 使用 Bing + DuckDuckGo |
| 图片搜索 | ✅ 保留 | 使用 Bing Images + Pixabay |
| 新闻搜索 | ✅ 保留 | 使用 Bing News |
| 文件下载 | ✅ 保留 | 使用 ureq HTTP 客户端 |
| 普通网页内容获取 | ✅ 保留 | 使用 ureq + scraper |

---

## 编译和测试结果

### 编译状态
```
✅ cargo build --release 成功
⚠️  警告：120 个（从 204 个减少，-41%）
```

### 测试结果
```
✅ cargo test --release
test result: ok. 178 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### 代码大小变化
- 删除代码：~434 行（browser.rs）
- 修改代码：~50 行（分布在多个文件）
- 依赖减少：1 个（headless_chrome）

---

## 文档更新

### 保留的 Google 引用（仅作为公司信息）

以下引用**未移除**，因为它们只是提到 Google 作为公司名称，不是技术依赖：

- `README.md:357` - "gemma3 系列 - Google Gemma 3"（模型提供商信息）
- `USER_GUIDE.md:44` - "Google Gemini | https://makersuite.google.com"（API 提供商列表）
- `USER_GUIDE.md:309` - "Google Gemini | Gemini 2.5 Pro, Gemini 2.5 Flash"（模型列表）

这些引用是信息性的，不影响项目运行。

---

## 轻量化收益

### 依赖树简化
- 移除 `headless_chrome` 及其传递依赖
- 减少编译时间
- 减少二进制文件大小

### 部署简化
- 无需安装 Chrome/Chromium 浏览器
- 无需配置 CHROME_PATH 环境变量
- 减少系统要求

### 启动速度
- 移除浏览器初始化时间
- 减少内存占用

---

## 后续建议

### 可选增强（如果需要网页截图）

如果未来需要网页截图功能，可以考虑：

1. **使用轻量级替代方案**
   - `resvg` + `usvg` - SVG 渲染
   - `image` - 图片处理
   - 但不支持 JavaScript 渲染

2. **作为可选 feature**
   ```toml
   [features]
   browser = ["headless_chrome"]
   ```
   用户按需启用

3. **外部工具集成**
   - 调用系统命令 `webkit2png`
   - 调用在线截图 API

---

## 验证命令

```bash
# 编译验证
cargo build --release

# 测试验证
cargo test --release

# 检查依赖树
cargo tree | grep -i chrome  # 应该无输出

# 检查 Google 依赖
grep -r "google" --include="*.toml" --include="*.rs"  # 仅配置引用
```

---

## 总结

✅ **目标达成**: 成功移除所有 Google/Chrome 技术依赖

✅ **功能完整**: 核心搜索功能保留，仅移除非必要的截图功能

✅ **代码质量**: 编译通过，178 个测试全部通过

✅ **轻量化**: 减少 1 个重量级依赖，434 行代码

---

**报告生成时间**: 2026-03-14  
**生成者**: Tokitai AI Assistant
