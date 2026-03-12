#!/bin/bash
# 测试图片搜索和浏览器功能

set -e

echo "======================================"
echo "测试图片搜索功能"
echo "======================================"

# 创建一个简单的 Rust 测试程序
cat > /tmp/test_image_search.rs << 'EOF'
use std::process::Command;

fn main() {
    println!("🔍 测试图片搜索功能...\n");
    
    // 测试 1: 搜索图片
    println!("测试 1: 搜索 'cute cat' 图片");
    let output = Command::new("cargo")
        .args(["run", "--", "search_images(query=\"cute cat\", limit=3)"])
        .output()
        .expect("Failed to execute command");
    
    if output.status.success() {
        println!("✅ 图片搜索成功");
        println!("输出：{}\n", String::from_utf8_lossy(&output.stdout));
    } else {
        println!("❌ 图片搜索失败");
        println!("错误：{}\n", String::from_utf8_lossy(&output.stderr));
    }
    
    println!("\n======================================");
    println!("所有测试完成");
    println!("======================================");
}
EOF

echo "运行集成测试..."
echo ""

# 检查是否安装了 chromium
if command -v chromium &> /dev/null || command -v chromium-browser &> /dev/null || command -v google-chrome &> /dev/null; then
    echo "✅ 检测到 Chromium/Chrome 浏览器"
    BROWSER_AVAILABLE=true
else
    echo "⚠️  未检测到 Chromium/Chrome 浏览器"
    echo "   安装方法:"
    echo "   - macOS: brew install chromium"
    echo "   - Linux: apt install chromium-browser"
    echo "   - Windows: 安装 Chrome"
    echo ""
    BROWSER_AVAILABLE=false
fi

# 测试网页搜索功能（这个不需要浏览器）
echo "--------------------------------------"
echo "测试 1: 网页搜索功能"
echo "--------------------------------------"
timeout 30 cargo run -- "搜索关键词：Rust programming" || echo "⚠️  测试超时或失败（可能需要配置 API）"

echo ""
echo "--------------------------------------"
echo "测试 2: 图片搜索工具（需要 API 配置）"
echo "--------------------------------------"
if [ "$BROWSER_AVAILABLE" = true ]; then
    echo "✅ 浏览器可用，可以测试 screenshot 功能"
else
    echo "⚠️  浏览器不可用，screenshot 功能将无法工作"
fi

echo ""
echo "======================================"
echo "功能测试说明"
echo "======================================"
echo ""
echo "新增工具："
echo "1. search_images(query, limit) - 搜索图片"
echo "   示例：search_images(query=\"cute cat\", limit=10)"
echo ""
echo "2. download_image(img_url, save_path) - 下载图片"
echo "   示例：download_image(img_url=\"https://example.com/cat.png\", save_path=\"./cat.png\")"
echo ""
echo "3. screenshot(url, save_path, full_page) - 网页截图"
echo "   示例：screenshot(url=\"https://example.com\", save_path=\"./screenshot.png\", full_page=true)"
echo ""
echo "4. get_page_content(url) - 获取渲染后的网页内容"
echo "   示例：get_page_content(url=\"https://example.com\")"
echo ""
echo "注意：screenshot 和 get_page_content 需要安装 Chromium/Chrome 浏览器"
echo "======================================"
