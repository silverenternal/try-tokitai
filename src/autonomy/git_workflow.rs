//! 自主 Git 工作流
//!
//! 实现 AI 自主的 Git 操作能力，支持自动提交和推送
//!
//! # 工作流步骤
//! 1. git status 检查变更
//! 2. git diff 生成变更摘要
//! 3. AI 生成提交消息
//! 4. 预提交检查（可选）
//! 5. git add + commit

#![allow(dead_code)]
//! 6. git push（可选）
//! 7. 失败回滚机制

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

/// Git 工作流错误类型
#[derive(Error, Debug)]
pub enum GitWorkflowError {
    #[error("Git 命令执行失败：{0}")]
    GitCommandFailed(String),
    #[error("预提交检查失败：{0}")]
    PreCommitCheckFailed(String),
    #[error("回滚失败：{0}")]
    RollbackFailed(String),
    #[error("文件操作失败：{0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON 处理失败：{0}")]
    JsonError(#[from] serde_json::Error),
}

/// Git 状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    pub branch: String,
    pub modified: Vec<String>,
    pub staged: Vec<String>,
    pub untracked: Vec<String>,
    pub deleted: Vec<String>,
}

/// 提交记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitRecord {
    /// 提交哈希
    pub hash: String,
    /// 提交消息
    pub message: String,
    /// 提交类型
    pub commit_type: String,
    /// 时间戳
    pub timestamp: i64,
    /// 变更文件列表
    pub changed_files: Vec<String>,
    /// 预提交检查结果
    pub pre_commit_passed: bool,
}

/// Git 工作流
pub struct GitWorkflow {
    /// 仓库根目录
    repo_dir: PathBuf,
    /// 存储目录
    storage_dir: PathBuf,
    /// 提交历史
    commits: Vec<CommitRecord>,
    /// 回滚检查点
    rollback_checkpoint: Option<String>,
}

impl GitWorkflow {
    /// 创建新的 Git 工作流
    pub fn new(repo_dir: PathBuf, storage_dir: PathBuf) -> Result<Self, GitWorkflowError> {
        fs::create_dir_all(&storage_dir)?;

        let mut workflow = Self {
            repo_dir,
            storage_dir,
            commits: vec![],
            rollback_checkpoint: None,
        };

        workflow.load_commits()?;

        Ok(workflow)
    }

    /// 检查 Git 状态
    pub fn git_status(&self) -> Result<GitStatus, GitWorkflowError> {
        // 获取分支
        let branch_output = self.run_git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
        let branch = branch_output.trim().to_string();

        // 获取状态
        let status_output = self.run_git(&["status", "--porcelain"])?;
        
        let mut modified = vec![];
        let mut staged = vec![];
        let mut untracked = vec![];
        let mut deleted = vec![];

        for line in status_output.lines() {
            if line.len() < 3 {
                continue;
            }
            let status = &line[0..2];
            let file = line[3..].trim().to_string();

            match status {
                "M " | " M" => modified.push(file),
                "A " => staged.push(file),
                "?? " => untracked.push(file),
                "D " => deleted.push(file),
                _ => {}
            }
        }

        Ok(GitStatus {
            branch,
            modified,
            staged,
            untracked,
            deleted,
        })
    }

    /// 获取变更摘要
    pub fn get_diff_summary(&self) -> Result<String, GitWorkflowError> {
        self.run_git(&["diff", "--stat"])
    }

    /// 生成提交消息（AI 辅助）
    pub fn generate_commit_message(&self, changes: &str) -> Result<(String, String), GitWorkflowError> {
        // 简单实现：根据变更内容生成提交类型和消息
        // 实际应该调用 AI 生成
        
        let commit_type = if changes.contains("feat") || changes.contains("add") {
            "feat"
        } else if changes.contains("fix") || changes.contains("修复") {
            "fix"
        } else if changes.contains("doc") || changes.contains("README") {
            "docs"
        } else if changes.contains("refactor") || changes.contains("重构") {
            "refactor"
        } else if changes.contains("test") || changes.contains("测试") {
            "test"
        } else {
            "chore"
        };

        let description = format!("自动提交：{}", changes.lines().next().unwrap_or("代码变更"));
        
        Ok((commit_type.to_string(), description))
    }

    /// 预提交检查
    pub fn pre_commit_check(&self) -> Result<(), GitWorkflowError> {
        // 运行 cargo fmt 检查
        let fmt_check = Command::new("cargo")
            .arg("fmt")
            .arg("--check")
            .current_dir(&self.repo_dir)
            .output();

        if let Ok(output) = fmt_check {
            if !output.status.success() {
                return Err(GitWorkflowError::PreCommitCheckFailed(
                    "cargo fmt 检查失败".to_string()
                ));
            }
        }

        // 运行 cargo clippy 检查
        let clippy_check = Command::new("cargo")
            .arg("clippy")
            .arg("--quiet")
            .current_dir(&self.repo_dir)
            .output();

        if let Ok(output) = clippy_check {
            if !output.status.success() {
                return Err(GitWorkflowError::PreCommitCheckFailed(
                    "cargo clippy 检查失败".to_string()
                ));
            }
        }

        Ok(())
    }

    /// 设置回滚检查点
    pub fn set_rollback_checkpoint(&mut self) -> Result<(), GitWorkflowError> {
        // 获取当前 HEAD
        let head = self.run_git(&["rev-parse", "HEAD"])?;
        self.rollback_checkpoint = Some(head.trim().to_string());
        Ok(())
    }

    /// 执行回滚
    pub fn rollback(&self) -> Result<(), GitWorkflowError> {
        if let Some(checkpoint) = &self.rollback_checkpoint {
            self.run_git(&["reset", "--hard", checkpoint])?;
            Ok(())
        } else {
            Err(GitWorkflowError::RollbackFailed("没有设置回滚检查点".to_string()))
        }
    }

    /// 提交变更
    pub fn commit(&mut self, message: &str, run_pre_commit: bool) -> Result<CommitRecord, GitWorkflowError> {
        // 设置回滚点
        self.set_rollback_checkpoint()?;

        // 预提交检查
        let pre_commit_passed = if run_pre_commit {
            self.pre_commit_check().is_ok()
        } else {
            true
        };

        // git add
        self.run_git(&["add", "-A"])?;

        // git commit
        self.run_git(&["commit", "-m", message])?;

        // 获取提交哈希
        let hash = self.run_git(&["rev-parse", "HEAD"])?;

        // 获取变更文件
        let changed_files_output = self.run_git(&["diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD"])?;
        let changed_files: Vec<String> = changed_files_output.lines().map(|s| s.to_string()).collect();

        // 获取提交类型
        let commit_type = message.split(':').next().unwrap_or("chore").to_string();

        let record = CommitRecord {
            hash: hash.trim().to_string(),
            message: message.to_string(),
            commit_type,
            timestamp: chrono::Utc::now().timestamp(),
            changed_files,
            pre_commit_passed,
        };

        self.commits.push(record.clone());
        self.save_commits()?;

        Ok(record)
    }

    /// 推送变更
    pub fn push(&self) -> Result<(), GitWorkflowError> {
        self.run_git(&["push"])?;
        Ok(())
    }

    /// 运行 Git 命令
    fn run_git(&self, args: &[&str]) -> Result<String, GitWorkflowError> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.repo_dir)
            .output()
            .map_err(|e| GitWorkflowError::GitCommandFailed(e.to_string()))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(GitWorkflowError::GitCommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string()
            ))
        }
    }

    /// 保存提交历史
    fn save_commits(&self) -> Result<(), GitWorkflowError> {
        let commits_path = self.storage_dir.join("commits.json");
        let content = serde_json::to_string_pretty(&self.commits)?;
        fs::write(&commits_path, content)?;
        Ok(())
    }

    /// 加载提交历史
    fn load_commits(&mut self) -> Result<(), GitWorkflowError> {
        let commits_path = self.storage_dir.join("commits.json");
        if commits_path.exists() {
            let content = fs::read_to_string(&commits_path)?;
            self.commits = serde_json::from_str(&content)?;
        }
        Ok(())
    }

    /// 获取提交历史
    pub fn commits(&self) -> &[CommitRecord] {
        &self.commits
    }

    /// 获取仓库目录
    pub fn repo_dir(&self) -> &Path {
        &self.repo_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_git_workflow_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = TempDir::new().unwrap();

        // 初始化 Git 仓库
        Command::new("git")
            .arg("init")
            .current_dir(&temp_dir)
            .output()
            .unwrap();

        let workflow = GitWorkflow::new(temp_dir.path().to_path_buf(), storage_dir.path().to_path_buf()).unwrap();
        
        assert!(workflow.repo_dir().exists());
    }
}
