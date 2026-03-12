fn main() {
    println!("测试图片搜索功能...\n");
    
    // 测试 WebSearchTools 的图片搜索
    match test_image_search() {
        Ok(_) => println!("✅ 图片搜索测试成功"),
        Err(e) => println!("❌ 图片搜索测试失败：{}", e),
    }
    
    println!("\n测试浏览器截图功能...\n");
    
    // 测试 BrowserTools 的截图
    match test_screenshot() {
        Ok(_) => println!("✅ 浏览器截图测试成功"),
        Err(e) => println!("❌ 浏览器截图测试失败：{}", e),
    }
}

fn test_image_search() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "search_images(query=\"cute cat\", limit=2)"])
        .output()?;
    
    if output.status.success() {
        println!("搜索成功!");
        println!("{}", String::from_utf8_lossy(&output.stdout));
        Ok(())
    } else {
        Err(format!("搜索失败：{}", String::from_utf8_lossy(&output.stderr)).into())
    }
}

fn test_screenshot() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "screenshot(url=\"https://example.com\", save_path=\"./test_screenshot.png\")"])
        .output()?;
    
    if output.status.success() {
        println!("截图成功!");
        println!("{}", String::from_utf8_lossy(&output.stdout));
        Ok(())
    } else {
        Err(format!("截图失败：{}", String::from_utf8_lossy(&output.stderr)).into())
    }
}
