use tokitai::tool;
use std::process::Command;

/// 系统命令执行工具集
pub struct SystemTools;

#[tool]
impl SystemTools {
    /// 执行 shell 命令
    pub fn run_command(&self, command: String) -> Result<String, String> {
        let output = Command::new("bash")
            .arg("-c")
            .arg(&command)
            .output()
            .map_err(|e| format!("执行命令失败：{}", e))?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        let mut result = String::new();
        if !stdout.is_empty() {
            result.push_str(&format!("stdout:\n{}\n", stdout));
        }
        if !stderr.is_empty() {
            result.push_str(&format!("stderr:\n{}\n", stderr));
        }
        if result.is_empty() {
            result.push_str("命令执行成功，无输出");
        }
        
        Ok(result)
    }

    /// 获取当前工作目录
    pub fn get_current_dir(&self) -> Result<String, String> {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .map_err(|e| format!("获取当前目录失败：{}", e))
    }

    /// 获取环境变量
    pub fn get_env(&self, key: String) -> Result<String, String> {
        std::env::var(&key)
            .map_err(|e| format!("获取环境变量失败：{}", e))
    }

    /// 列出所有环境变量
    pub fn list_env(&self) -> Result<String, String> {
        let vars: Vec<String> = std::env::vars()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        Ok(vars.join("\n"))
    }
}
