use tokitai::tool;
use tracing::{info, warn};

/// Git 操作工具集
pub struct GitOperations;

#[tool]
impl GitOperations {
    /// 获取 git 状态
    ///
    /// # 参数
    /// - `repo_path`: 仓库路径（默认当前目录）
    ///
    /// # 返回
    /// 返回 git status 输出
    #[tool(default_repo_path = "null")]
    pub fn git_status(&self, repo_path: Option<String>) -> Result<String, String> {
        let path = repo_path.unwrap_or_else(|| ".".to_string());
        info!("执行 git status，路径：{}", path);

        let output = std::process::Command::new("git")
            .args(["-C", &path, "status"])
            .output()
            .map_err(|e| format!("执行 git status 失败：{}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("git status 失败：{}", stderr);
            return Err(format!("git status 失败：{}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    }

    /// 查看 git 日志
    ///
    /// # 参数
    /// - `repo_path`: 仓库路径（默认当前目录）
    /// - `limit`: 显示日志条数（默认 10）
    ///
    /// # 返回
    /// 返回 git log 输出
    #[tool(default_repo_path = "null", default_limit = "null")]
    pub fn git_log(&self, repo_path: Option<String>, limit: Option<usize>) -> Result<String, String> {
        let path = repo_path.unwrap_or_else(|| ".".to_string());
        let limit = limit.unwrap_or(10);
        info!("执行 git log，路径：{}，限制：{}", path, limit);

        let output = std::process::Command::new("git")
            .args(["-C", &path, "log", "-n", &limit.to_string(), "--pretty=format:%h - %an - %s - %cr"])
            .output()
            .map_err(|e| format!("执行 git log 失败：{}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("git log 失败：{}", stderr);
            return Err(format!("git log 失败：{}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    }

    /// 查看 git diff
    ///
    /// # 参数
    /// - `repo_path`: 仓库路径（默认当前目录）
    ///
    /// # 返回
    /// 返回 git diff 输出
    #[tool(default_repo_path = "null")]
    pub fn git_diff(&self, repo_path: Option<String>) -> Result<String, String> {
        let path = repo_path.unwrap_or_else(|| ".".to_string());
        info!("执行 git diff，路径：{}", path);

        let output = std::process::Command::new("git")
            .args(["-C", &path, "diff"])
            .output()
            .map_err(|e| format!("执行 git diff 失败：{}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("git diff 失败：{}", stderr);
            return Err(format!("git diff 失败：{}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.is_empty() {
            Ok("没有未暂存的更改".to_string())
        } else {
            Ok(stdout.to_string())
        }
    }

    /// 查看 git 分支
    ///
    /// # 参数
    /// - `repo_path`: 仓库路径（默认当前目录）
    ///
    /// # 返回
    /// 返回分支列表
    #[tool(default_repo_path = "null")]
    pub fn git_branch(&self, repo_path: Option<String>) -> Result<String, String> {
        let path = repo_path.unwrap_or_else(|| ".".to_string());
        info!("执行 git branch，路径：{}", path);

        let output = std::process::Command::new("git")
            .args(["-C", &path, "branch", "-a"])
            .output()
            .map_err(|e| format!("执行 git branch 失败：{}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("git branch 失败：{}", stderr);
            return Err(format!("git branch 失败：{}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    }

    /// 获取当前分支
    ///
    /// # 参数
    /// - `repo_path`: 仓库路径（默认当前目录）
    ///
    /// # 返回
    /// 返回当前分支名称
    #[tool(default_repo_path = "null")]
    pub fn git_current_branch(&self, repo_path: Option<String>) -> Result<String, String> {
        let path = repo_path.unwrap_or_else(|| ".".to_string());
        info!("获取当前分支，路径：{}", path);

        let output = std::process::Command::new("git")
            .args(["-C", &path, "branch", "--show-current"])
            .output()
            .map_err(|e| format!("执行 git branch --show-current 失败：{}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("获取当前分支失败：{}", stderr);
            return Err(format!("获取当前分支失败：{}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_status() {
        let git = GitOperations;
        let result = git.git_status(Some(".".to_string()));
        assert!(result.is_ok(), "git status 应该成功：{:?}", result);
        let output = result.unwrap();
        assert!(output.contains("位于分支") || output.contains("On branch"));
    }

    #[test]
    fn test_git_current_branch() {
        let git = GitOperations;
        let result = git.git_current_branch(Some(".".to_string()));
        assert!(result.is_ok(), "获取当前分支应该成功：{:?}", result);
        let branch = result.unwrap();
        assert!(!branch.is_empty(), "分支名称不应为空");
    }

    #[test]
    fn test_git_log() {
        let git = GitOperations;
        let result = git.git_log(Some(".".to_string()), Some(5));
        assert!(result.is_ok(), "git log 应该成功：{:?}", result);
    }

    #[test]
    fn test_git_branch() {
        let git = GitOperations;
        let result = git.git_branch(Some(".".to_string()));
        assert!(result.is_ok(), "git branch 应该成功：{:?}", result);
    }
}
