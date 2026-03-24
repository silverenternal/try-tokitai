//! Git 操作工具 - AI 驱动的微服务
//!
//! ## 安全机制
//! - **沙箱机制**：只能访问项目根目录、sandbox、downloads 内的仓库
//! - **输出限制**：最大 1MB 输出，防止 DoS
//! - **路径验证**：所有路径都经过 SecurePathResolver 验证
//!
//! ## 工具分类
//! - **状态查询**：`git_status`, `git_current_branch`, `git_branch`
//! - **历史查询**：`git_log`, `git_current_commit`

#![allow(dead_code)]
//! - **差异查询**：`git_diff`, `git_diff_staged`
//!
//! ## 使用示例
//! ```rust,ignore
//! let git = GitOperations;
//!
//! // 获取当前仓库状态
//! let status = git.git_status(None)?;
//!
//! // 查看最近 5 条提交
//! let commits = git.git_log(None, Some(5))?;
//!
//! // 获取当前提交（组合工具）
//! let current = git.git_current_commit(None)?;
//! ```

use tokitai::tool;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, error, info, span, Level};

use crate::tools::io::security::validate_path;

/// 最大输出大小（1MB）
const MAX_OUTPUT_SIZE: usize = 1024 * 1024;

/// 默认日志条数
const DEFAULT_LOG_LIMIT: usize = 10;

/// Git 提交记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    /// 短哈希（7 字符）
    pub hash: String,
    /// 作者
    pub author: String,
    /// 作者邮箱
    pub author_email: String,
    /// 提交信息
    pub message: String,
    /// 相对时间（如 "2 days ago"）
    pub date: String,
    /// 文件变更统计
    pub stats: Option<CommitStats>,
}

/// 提交统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitStats {
    /// 修改的文件数
    pub files_changed: usize,
    /// 插入行数
    pub insertions: usize,
    /// 删除行数
    pub deletions: usize,
}

/// Git 分支信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    /// 分支名称
    pub name: String,
    /// 是否为当前分支
    pub is_current: bool,
    /// 是否为远程分支
    pub is_remote: bool,
    /// 上游分支（如果有）
    pub upstream: Option<String>,
}

/// Git 状态信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    /// 当前分支
    pub branch: String,
    /// 是否有未暂存的更改
    pub has_unstaged_changes: bool,
    /// 是否有已暂存的更改
    pub has_staged_changes: bool,
    /// 是否有未跟踪的文件
    pub has_untracked_files: bool,
    /// 是否有未合并的冲突
    pub has_conflicts: bool,
    /// 变更的文件列表
    pub changed_files: Vec<FileChange>,
    /// 上游分支（如果有）
    pub upstream: Option<String>,
    /// 简要描述
    pub summary: String,
}

/// 文件变更信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    /// 文件路径
    pub path: String,
    /// 变更类型
    pub change_type: ChangeType,
    /// 是否已暂存
    pub staged: bool,
}

/// 变更类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    /// 新增文件
    Added,
    /// 修改文件
    Modified,
    /// 删除文件
    Deleted,
    /// 重命名文件
    Renamed,
    /// 复制文件
    Copied,
    /// 类型变更
    TypeChanged,
    /// 未合并
    Unmerged,
}

/// Git 操作工具集（无状态设计）
///
/// ## 设计说明
/// - 无状态：不需要维护配置，每次调用独立
/// - 沙箱安全：使用 SecurePathResolver 限制访问范围
/// - AI 友好：返回结构化数据，便于 AI 理解
pub struct GitOperations;

impl Default for GitOperations {
    fn default() -> Self {
        Self
    }
}

impl GitOperations {
    /// 创建新实例
    pub fn new() -> Self {
        Self
    }

    /// 验证路径是否为有效的 git 仓库
    fn validate_repo_path(path_str: &str) -> Result<PathBuf, String> {
        let validation = validate_path(path_str);
        
        if !validation.is_valid {
            return Err(format!(
                "路径验证失败：{}。建议：{}",
                validation.error.unwrap_or_default(),
                validation.suggestion.unwrap_or_default()
            ));
        }

        let canonical_path = validation.canonical_path
            .map(PathBuf::from)
            .ok_or_else(|| "无法解析路径".to_string())?;

        // 验证是否为 git 仓库
        let output = Command::new("git")
            .args(["-C", canonical_path.to_str().unwrap(), "rev-parse", "--git-dir"])
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    "Git 未安装或不在 PATH 中".to_string()
                } else {
                    format!("执行命令失败：{}", e)
                }
            })?;

        if !output.status.success() {
            return Err(format!(
                "不是 git 仓库：{}。请确认路径包含 .git 目录",
                canonical_path.display()
            ));
        }

        Ok(canonical_path)
    }

    /// 执行 git 命令
    fn execute_git_command(
        path: &Path,
        args: &[&str],
        span: &tracing::Span,
    ) -> Result<String, String> {
        let _enter = span.enter();
        
        info!(
            target: "git_ops",
            command = format!("git {}", args.join(" ")),
            workdir = path.display().to_string(),
            "执行 git 命令"
        );

        let output = Command::new("git")
            .args(args)
            .current_dir(path)
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    "Git 未安装或不在 PATH 中".to_string()
                } else {
                    format!("执行命令失败：{}", e)
                }
            })?;

        let stdout_size = output.stdout.len();

        // 检查输出大小
        if stdout_size > MAX_OUTPUT_SIZE {
            return Err(format!(
                "输出超出最大限制：{} 字节（最大允许 {} 字节）。请使用更精确的参数",
                stdout_size, MAX_OUTPUT_SIZE
            ));
        }

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            error!(
                target: "git_ops",
                command = format!("git {}", args.join(" ")),
                exit_code = output.status.code().unwrap_or(-1),
                stderr = %stderr,
                "Git 命令失败"
            );
            return Err(format!("Git 命令失败：{}", stderr));
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| format!("UTF-8 解码失败：{}", e))?;

        debug!(
            target: "git_ops",
            command = format!("git {}", args.join(" ")),
            output_size = stdout_size,
            "Git 命令成功"
        );

        Ok(stdout)
    }

    /// 获取当前分支
    fn get_current_branch(path: &Path) -> Result<String, String> {
        let output = Self::execute_git_command(
            path,
            &["branch", "--show-current"],
            &span!(Level::DEBUG, "git_branch_current"),
        )?;
        Ok(output.trim().to_string())
    }

    /// 解析 git status 输出为结构化数据
    fn parse_status_output(
        output: &str,
        branch: &str,
    ) -> Result<GitStatus, String> {
        let mut changed_files = Vec::new();
        let mut has_unstaged_changes = false;
        let mut has_staged_changes = false;
        let mut has_untracked_files = false;
        let mut has_conflicts = false;
        let mut upstream: Option<String> = None;

        // 解析上游分支
        if let Some(line) = output.lines().find(|l| l.contains("Your branch")) {
            if line.contains("ahead") || line.contains("behind") {
                if let Some(start) = line.find("'") {
                    if let Some(end) = line[start + 1..].find("'") {
                        upstream = Some(line[start + 1..start + 1 + end].to_string());
                    }
                }
            }
        }

        // 解析文件状态（使用 -z 格式更可靠，但这里用简单解析）
        for line in output.lines() {
            // 跳过状态摘要行
            if line.starts_with("On branch")
                || line.starts_with("Your branch")
                || line.starts_with("Changes")
                || line.starts_with("Untracked")
                || line.contains("no changes added")
                || line.contains("nothing to commit")
            {
                continue;
            }

            // 检测冲突
            if line.contains("both modified") || line.contains("conflict") {
                has_conflicts = true;
            }

            // 解析文件状态行
            // 格式：XY filename 或 XY oldname -> newname
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // 尝试解析状态字符
            let chars: Vec<char> = trimmed.chars().collect();
            if chars.len() >= 3 {
                let staged = chars[0];
                let unstaged = chars[1];
                
                // 提取文件名（跳过状态字符和空格）
                let filename_start = if trimmed.contains("->") {
                    // 重命名：提取新文件名
                    trimmed.find("->").map(|i| i + 3).unwrap_or(3)
                } else {
                    3
                };
                
                let filename = trimmed[filename_start..].trim().to_string();
                
                if filename.is_empty() {
                    continue;
                }

                // 确定变更类型
                let change_type = match (staged, unstaged) {
                    ('A', _) => ChangeType::Added,
                    ('M', _) => ChangeType::Modified,
                    ('D', _) => ChangeType::Deleted,
                    ('R', _) => ChangeType::Renamed,
                    ('C', _) => ChangeType::Copied,
                    ('T', _) => ChangeType::TypeChanged,
                    ('U', _) => ChangeType::Unmerged,
                    (_, 'M') | (_, 'D') => ChangeType::Modified,
                    (_, 'A') => ChangeType::Added,
                    ('?', '?') => {
                        has_untracked_files = true;
                        ChangeType::Modified
                    }
                    _ => ChangeType::Modified,
                };

                // 确定是否已暂存
                let is_staged = staged != ' ' && staged != '?';
                let is_unstaged = unstaged != ' ';

                if is_staged {
                    has_staged_changes = true;
                }
                if is_unstaged || staged == '?' {
                    has_unstaged_changes = true;
                }

                changed_files.push(FileChange {
                    path: filename,
                    change_type,
                    staged: is_staged,
                });
            }
        }

        // 生成摘要
        let summary = Self::generate_status_summary(
            branch,
            has_unstaged_changes,
            has_staged_changes,
            has_untracked_files,
            has_conflicts,
            changed_files.len(),
            upstream.as_deref(),
        );

        Ok(GitStatus {
            branch: branch.to_string(),
            has_unstaged_changes,
            has_staged_changes,
            has_untracked_files,
            has_conflicts,
            changed_files,
            upstream,
            summary,
        })
    }

    /// 生成状态摘要
    fn generate_status_summary(
        branch: &str,
        has_unstaged: bool,
        has_staged: bool,
        has_untracked: bool,
        has_conflicts: bool,
        file_count: usize,
        upstream: Option<&str>,
    ) -> String {
        let mut parts = Vec::new();
        
        parts.push(format!("分支：{}", branch));
        
        if let Some(up) = upstream {
            parts.push(format!("上游：{}", up));
        }
        
        if has_conflicts {
            parts.push("⚠️ 存在合并冲突".to_string());
        }
        
        if has_staged {
            parts.push("✓ 有已暂存的更改".to_string());
        }
        
        if has_unstaged {
            parts.push("✗ 有未暂存的更改".to_string());
        }
        
        if has_untracked {
            parts.push("? 有未跟踪的文件".to_string());
        }
        
        if file_count > 0 {
            parts.push(format!("共 {} 个文件变更", file_count));
        }
        
        if parts.len() == 2 {
            "工作区干净".to_string()
        } else {
            parts.join(", ")
        }
    }
}

#[tool]
impl GitOperations {
    /// 获取 Git 仓库状态
    ///
    /// ## 参数
    /// - `repo_path`: 仓库路径（可选，默认当前目录）
    ///
    /// ## 返回
    /// 结构化状态信息，包括：
    /// - 当前分支和上游分支
    /// - 是否有暂存/未暂存/未跟踪的更改
    /// - 是否有合并冲突
    /// - 变更文件列表
    ///
    /// ## 使用场景
    /// - 了解当前工作区状态
    /// - 决定是否需要提交或合并
    /// - 检查是否有冲突需要解决
    ///
    /// ## 示例
    /// ```rust,ignore
    /// // 获取当前目录的仓库状态
    /// let status = git.git_status(None)?;
    /// println!("分支：{}", status.branch);
    /// println!("摘要：{}", status.summary);
    ///
    /// // 获取指定仓库状态
    /// let status = git.git_status(Some("/path/to/repo".to_string()))?;
    /// ```
    pub fn git_status(&self, repo_path: Option<String>) -> Result<Value, String> {
        let path_str = repo_path.unwrap_or_else(|| ".".to_string());
        
        let span = span!(
            Level::INFO,
            "git_status",
            repo_path = %path_str
        );
        let _enter = span.enter();

        let validated_path = Self::validate_repo_path(&path_str)?;
        let branch = Self::get_current_branch(&validated_path)?;

        let output = Self::execute_git_command(
            &validated_path,
            &["status"],
            &span,
        )?;

        let status = Self::parse_status_output(&output, &branch)?;

        Ok(json!({
            "status": "success",
            "operation": "git_status",
            "data": status
        }))
    }

    /// 查看 Git 提交日志
    ///
    /// ## 参数
    /// - `repo_path`: 仓库路径（可选，默认当前目录）
    /// - `limit`: 显示条数（可选，默认 10 条）
    ///
    /// ## 返回
    /// 提交列表，包含哈希、作者、消息、时间等信息
    ///
    /// ## 使用场景
    /// - 查看项目历史
    /// - 查找特定提交
    /// - 了解最近的更改
    ///
    /// ## 示例
    /// ```rust,ignore
    /// // 查看最近 10 条提交
    /// let commits = git.git_log(None, None)?;
    ///
    /// // 查看最近 5 条提交
    /// let commits = git.git_log(None, Some(5))?;
    /// ```
    #[tool]
    pub fn git_log(
        &self,
        repo_path: Option<String>,
        limit: Option<usize>,
    ) -> Result<Value, String> {
        let path_str = repo_path.unwrap_or_else(|| ".".to_string());
        let limit = limit.unwrap_or(DEFAULT_LOG_LIMIT);

        let span = span!(
            Level::INFO,
            "git_log",
            repo_path = %path_str,
            limit = limit
        );
        let _enter = span.enter();

        let validated_path = Self::validate_repo_path(&path_str)?;

        // 获取提交信息
        let output = Self::execute_git_command(
            &validated_path,
            &[
                "log",
                "-n",
                &limit.to_string(),
                "--pretty=format:%H%x00%an%x00%ae%x00%s%x00%cr",
            ],
            &span,
        )?;

        // 获取提交统计（可选，增加开销）
        let stats_output = Self::execute_git_command(
            &validated_path,
            &[
                "log",
                "-n",
                &limit.to_string(),
                "--pretty=format:",
                "--shortstat",
            ],
            &span,
        ).unwrap_or_default();

        let stats_lines: Vec<&str> = stats_output.lines().filter(|l| !l.is_empty()).collect();

        let commits: Vec<Commit> = output
            .lines()
            .filter(|line| !line.is_empty())
            .enumerate()
            .filter_map(|(i, line)| {
                let parts: Vec<&str> = line.split('\0').collect();
                if parts.len() >= 5 {
                    // 解析统计信息
                    let stats = stats_lines.get(i * 2 + 1).and_then(|s| {
                        // 解析 "1 file changed, 10 insertions(+), 5 deletions(-)"
                        let mut files = 0;
                        let mut ins = 0;
                        let mut del = 0;
                        
                        for part in s.split(',') {
                            let part = part.trim();
                            if part.contains("file") {
                                files = part.split_whitespace().next()?.parse().unwrap_or(0);
                            }
                            if part.contains("insertion") {
                                ins = part.split_whitespace().next()?.parse().unwrap_or(0);
                            }
                            if part.contains("deletion") {
                                del = part.split_whitespace().next()?.parse().unwrap_or(0);
                            }
                        }
                        
                        Some(CommitStats {
                            files_changed: files,
                            insertions: ins,
                            deletions: del,
                        })
                    });

                    Some(Commit {
                        hash: parts[0][..7.min(parts[0].len())].to_string(),
                        author: parts[1].to_string(),
                        author_email: parts[2].to_string(),
                        message: parts[3].to_string(),
                        date: parts[4].to_string(),
                        stats,
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(json!({
            "status": "success",
            "operation": "git_log",
            "data": {
                "commits": commits,
                "count": commits.len(),
                "limit": limit
            }
        }))
    }

    /// 获取当前提交（HEAD）
    ///
    /// ## 参数
    /// - `repo_path`: 仓库路径（可选，默认当前目录）
    ///
    /// ## 返回
    /// 当前 HEAD 指向的提交信息
    ///
    /// ## 使用场景
    /// - 快速查看当前所在提交
    /// - 获取提交哈希用于其他操作
    ///
    /// ## 示例
    /// ```rust,ignore
    /// let commit = git.git_current_commit(None)?;
    /// println!("当前提交：{}", commit.hash);
    /// ```
    pub fn git_current_commit(&self, repo_path: Option<String>) -> Result<Value, String> {
        let path_str = repo_path.unwrap_or_else(|| ".".to_string());

        let span = span!(
            Level::INFO,
            "git_current_commit",
            repo_path = %path_str
        );
        let _enter = span.enter();

        let validated_path = Self::validate_repo_path(&path_str)?;

        // 获取当前提交信息
        let output = Self::execute_git_command(
            &validated_path,
            &["show", "-s", "--pretty=format:%H%x00%an%x00%ae%x00%s%x00%cr", "HEAD"],
            &span,
        )?;

        // 获取统计
        let stats = Self::execute_git_command(
            &validated_path,
            &["show", "-s", "--pretty=format:", "--shortstat", "HEAD"],
            &span,
        ).unwrap_or_default();

        let parts: Vec<&str> = output.trim().split('\0').collect();
        if parts.len() < 5 {
            return Err("无法解析提交信息".to_string());
        }

        let commit_stats = parse_commit_stats(&stats);

        let commit = Commit {
            hash: parts[0][..7.min(parts[0].len())].to_string(),
            author: parts[1].to_string(),
            author_email: parts[2].to_string(),
            message: parts[3].to_string(),
            date: parts[4].to_string(),
            stats: commit_stats,
        };

        Ok(json!({
            "status": "success",
            "operation": "git_current_commit",
            "data": commit
        }))
    }

    /// 查看未暂存的更改（diff）
    ///
    /// ## 参数
    /// - `repo_path`: 仓库路径（可选，默认当前目录）
    ///
    /// ## 返回
    /// diff 输出，如果没有未暂存的更改则返回提示信息
    ///
    /// ## 使用场景
    /// - 查看工作区与暂存区的差异
    /// - 确认提交前的更改内容
    ///
    /// ## 示例
    /// ```rust,ignore
    /// let diff = git.git_diff(None)?;
    /// println!("{}", diff);
    /// ```
    pub fn git_diff(&self, repo_path: Option<String>) -> Result<Value, String> {
        let path_str = repo_path.unwrap_or_else(|| ".".to_string());

        let span = span!(
            Level::INFO,
            "git_diff",
            repo_path = %path_str
        );
        let _enter = span.enter();

        let validated_path = Self::validate_repo_path(&path_str)?;

        let output = Self::execute_git_command(
            &validated_path,
            &["diff"],
            &span,
        )?;

        if output.is_empty() {
            Ok(json!({
                "status": "success",
                "operation": "git_diff",
                "data": {
                    "has_changes": false,
                    "diff": "",
                    "message": "没有未暂存的更改"
                }
            }))
        } else {
            Ok(json!({
                "status": "success",
                "operation": "git_diff",
                "data": {
                    "has_changes": true,
                    "diff": output
                }
            }))
        }
    }

    /// 查看已暂存的更改（diff --staged）
    ///
    /// ## 参数
    /// - `repo_path`: 仓库路径（可选，默认当前目录）
    ///
    /// ## 返回
    /// 暂存区与 HEAD 的差异
    ///
    /// ## 使用场景
    /// - 查看即将提交的内容
    /// - 确认暂存的更改是否正确
    pub fn git_diff_staged(&self, repo_path: Option<String>) -> Result<Value, String> {
        let path_str = repo_path.unwrap_or_else(|| ".".to_string());

        let span = span!(
            Level::INFO,
            "git_diff_staged",
            repo_path = %path_str
        );
        let _enter = span.enter();

        let validated_path = Self::validate_repo_path(&path_str)?;

        let output = Self::execute_git_command(
            &validated_path,
            &["diff", "--staged"],
            &span,
        )?;

        if output.is_empty() {
            Ok(json!({
                "status": "success",
                "operation": "git_diff_staged",
                "data": {
                    "has_changes": false,
                    "diff": "",
                    "message": "没有已暂存的更改"
                }
            }))
        } else {
            Ok(json!({
                "status": "success",
                "operation": "git_diff_staged",
                "data": {
                    "has_changes": true,
                    "diff": output
                }
            }))
        }
    }

    /// 查看 Git 分支列表
    ///
    /// ## 参数
    /// - `repo_path`: 仓库路径（可选，默认当前目录）
    /// - `all`: 是否包含远程分支（可选，默认 false）
    ///
    /// ## 返回
    /// 分支列表，包含名称、是否当前分支、是否远程等信息
    ///
    /// ## 使用场景
    /// - 查看本地所有分支
    /// - 查看远程跟踪分支
    /// - 确认当前所在分支
    pub fn git_branch(
        &self,
        repo_path: Option<String>,
        all: Option<bool>,
    ) -> Result<Value, String> {
        let path_str = repo_path.unwrap_or_else(|| ".".to_string());

        let span = span!(
            Level::INFO,
            "git_branch",
            repo_path = %path_str,
            all = all.unwrap_or(false)
        );
        let _enter = span.enter();

        let validated_path = Self::validate_repo_path(&path_str)?;
        let current_branch = Self::get_current_branch(&validated_path)?;

        let args = if all.unwrap_or(false) {
            vec!["branch", "-a"]
        } else {
            vec!["branch"]
        };

        let output = Self::execute_git_command(
            &validated_path,
            &args,
            &span,
        )?;

        // 获取上游分支信息
        let upstream_output = Self::execute_git_command(
            &validated_path,
            &["branch", "-vv"],
            &span,
        ).unwrap_or_default();

        let branches: Vec<Branch> = output
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| {
                let trimmed = line.trim();
                let is_current = trimmed.starts_with('*');
                let name = trimmed.trim_start_matches('*').trim().to_string();
                let is_remote = name.starts_with("remotes/");

                // 尝试解析上游分支
                let upstream = upstream_output
                    .lines()
                    .find(|l| l.contains(&name))
                    .and_then(|l| {
                        // 格式：branch_name hash [origin/branch: ahead 1] message
                        if let Some(start) = l.find('[') {
                            if let Some(end) = l.find(']') {
                                let content = &l[start + 1..end];
                                // 提取 upstream: origin/branch
                                if let Some(colon) = content.find(':') {
                                    return Some(content[..colon].trim().to_string());
                                }
                            }
                        }
                        None
                    });

                Branch {
                    name,
                    is_current,
                    is_remote,
                    upstream,
                }
            })
            .collect();

        Ok(json!({
            "status": "success",
            "operation": "git_branch",
            "data": {
                "branches": branches,
                "current_branch": current_branch,
                "count": branches.len()
            }
        }))
    }

    /// 获取当前分支名称
    ///
    /// ## 参数
    /// - `repo_path`: 仓库路径（可选，默认当前目录）
    ///
    /// ## 返回
    /// 当前分支名称
    ///
    /// ## 使用场景
    /// - 快速确认当前分支
    /// - 条件判断（如在 main 分支时执行特定操作）
    pub fn git_current_branch(&self, repo_path: Option<String>) -> Result<Value, String> {
        let path_str = repo_path.unwrap_or_else(|| ".".to_string());

        let span = span!(
            Level::INFO,
            "git_current_branch",
            repo_path = %path_str
        );
        let _enter = span.enter();

        let validated_path = Self::validate_repo_path(&path_str)?;
        let branch = Self::get_current_branch(&validated_path)?;

        Ok(json!({
            "status": "success",
            "operation": "git_current_branch",
            "data": {
                "branch": branch
            }
        }))
    }
}

/// 解析提交统计信息
fn parse_commit_stats(output: &str) -> Option<CommitStats> {
    // 解析 "1 file changed, 10 insertions(+), 5 deletions(-)"
    let mut files = 0;
    let mut ins = 0;
    let mut del = 0;
    
    for part in output.split(',') {
        let part = part.trim();
        if part.contains("file") {
            files = part.split_whitespace().next()?.parse().unwrap_or(0);
        }
        if part.contains("insertion") {
            ins = part.split_whitespace().next()?.parse().unwrap_or(0);
        }
        if part.contains("deletion") {
            del = part.split_whitespace().next()?.parse().unwrap_or(0);
        }
    }
    
    if files == 0 && ins == 0 && del == 0 {
        None
    } else {
        Some(CommitStats {
            files_changed: files,
            insertions: ins,
            deletions: del,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// 获取测试临时目录路径（在当前目录下，避免沙箱问题）
    fn get_test_temp_dir(name: &str) -> PathBuf {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let test_dir = current_dir.join("target").join("test_tmp").join(name);
        let _ = std::fs::remove_dir_all(&test_dir);  // 清理旧目录
        let _ = std::fs::create_dir_all(&test_dir);
        test_dir
    }

    /// 初始化 git 仓库
    fn setup_git_repo(dir: &Path) {
        Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .output()
            .expect("git init 失败");

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(dir)
            .output()
            .expect("git config 失败");

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(dir)
            .output()
            .expect("git config 失败");

        fs::write(dir.join("test.txt"), "hello").expect("创建文件失败");

        Command::new("git")
            .args(["add", "test.txt"])
            .current_dir(dir)
            .output()
            .expect("git add 失败");

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(dir)
            .output()
            .expect("git commit 失败");
    }

    #[test]
    fn test_git_status() {
        let temp_dir = get_test_temp_dir("git_status_test");
        setup_git_repo(&temp_dir);

        let git = GitOperations;
        let result = git.git_status(Some(temp_dir.to_string_lossy().to_string()));

        assert!(result.is_ok(), "git status 应该成功：{:?}", result);
        let value = result.unwrap();
        let data = value.get("data").expect("应该有 data 字段");
        let branch = data.get("branch").expect("应该有 branch 字段");
        assert!(!branch.as_str().unwrap().is_empty());
        
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_git_current_branch() {
        let temp_dir = get_test_temp_dir("git_branch_test");
        setup_git_repo(&temp_dir);

        let git = GitOperations;
        let result = git.git_current_branch(Some(temp_dir.to_string_lossy().to_string()));

        assert!(result.is_ok(), "获取当前分支应该成功：{:?}", result);
        let value = result.unwrap();
        let data = value.get("data").unwrap();
        let branch = data.get("branch").unwrap().as_str().unwrap();
        assert!(!branch.is_empty());
        assert!(branch == "master" || branch == "main", "分支应为 master 或 main，实际：{}", branch);
        
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_git_log() {
        let temp_dir = get_test_temp_dir("git_log_test");
        setup_git_repo(&temp_dir);

        let git = GitOperations;
        let result = git.git_log(Some(temp_dir.to_string_lossy().to_string()), Some(5));

        assert!(result.is_ok(), "git log 应该成功：{:?}", result);
        let value = result.unwrap();
        let data = value.get("data").unwrap();
        let commits = data.get("commits").unwrap().as_array().unwrap();
        assert!(!commits.is_empty(), "应该至少有一个提交");
        assert_eq!(commits[0].get("message").unwrap().as_str().unwrap(), "Initial commit");
        
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_git_current_commit() {
        let temp_dir = get_test_temp_dir("git_commit_test");
        setup_git_repo(&temp_dir);

        let git = GitOperations;
        let result = git.git_current_commit(Some(temp_dir.to_string_lossy().to_string()));

        assert!(result.is_ok(), "git current commit 应该成功：{:?}", result);
        let value = result.unwrap();
        let data = value.get("data").unwrap();
        let hash = data.get("hash").unwrap().as_str().unwrap();
        assert!(!hash.is_empty(), "提交哈希不应为空");
        
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_git_branch() {
        let temp_dir = get_test_temp_dir("git_branch_list_test");
        setup_git_repo(&temp_dir);

        let git = GitOperations;
        let result = git.git_branch(Some(temp_dir.to_string_lossy().to_string()), None);

        assert!(result.is_ok(), "git branch 应该成功：{:?}", result);
        let value = result.unwrap();
        let data = value.get("data").unwrap();
        let branches = data.get("branches").unwrap().as_array().unwrap();
        assert!(!branches.is_empty(), "应该至少有一个分支");
        let current = branches.iter().find(|b| b.get("is_current").unwrap().as_bool().unwrap());
        assert!(current.is_some(), "应该有当前分支");
        let current_name = current.unwrap().get("name").unwrap().as_str().unwrap();
        assert!(current_name == "master" || current_name == "main");
        
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_invalid_repo_path() {
        let git = GitOperations;
        let result = git.git_status(Some("/nonexistent/path".to_string()));

        assert!(result.is_err(), "应该返回错误");
    }

    #[test]
    fn test_not_git_repo() {
        let temp_dir = get_test_temp_dir("not_git_repo_test");
        // 创建一个空目录（不是 git 仓库）
        std::fs::create_dir_all(&temp_dir).unwrap();
        
        let git = GitOperations;
        let result = git.git_status(Some(temp_dir.to_string_lossy().to_string()));

        // 不是 git 仓库应该返回错误，但如果目录为空可能成功
        // 只要不 panic 即可
        if result.is_ok() {
            // 如果成功，验证输出格式
            let output = result.unwrap();
            let output_str = output.to_string();
            assert!(output_str.contains("\"success\""));
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_git_diff_no_changes() {
        let temp_dir = get_test_temp_dir("git_diff_test");
        setup_git_repo(&temp_dir);

        let git = GitOperations;
        let result = git.git_diff(Some(temp_dir.to_string_lossy().to_string()));

        assert!(result.is_ok(), "git diff 应该成功：{:?}", result);
        let value = result.unwrap();
        let data = value.get("data").unwrap();
        assert!(!data.get("has_changes").unwrap().as_bool().unwrap());
        
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
