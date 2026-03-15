//! GitWorkflow 工具包装器
//!
//! 将 GitWorkflow 包装为 tokitai ToolProvider，以便集成到工具矩阵中
//!
//! ## 设计理念
//! - 利用 tokitai 的 `#[tool]` 宏自动生成工具定义
//! - 保持与现有 GitWorkflow 的兼容性
//! - 支持通过工具矩阵统一调度

use tokitai::tool;
use super::git_workflow::GitWorkflow;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;

/// Git 自主工作流工具集
pub struct GitWorkflowTools {
    workflow: Arc<RwLock<GitWorkflow>>,
}

impl GitWorkflowTools {
    /// 创建新的 Git 工作流工具
    pub fn new(repo_dir: PathBuf, storage_dir: PathBuf) -> Result<Self, String> {
        let workflow = GitWorkflow::new(repo_dir, storage_dir)
            .map_err(|e| format!("创建 Git 工作流失败：{}", e))?;
        Ok(Self {
            workflow: Arc::new(RwLock::new(workflow)),
        })
    }

    /// 从现有 GitWorkflow 创建
    pub fn from_workflow(workflow: GitWorkflow) -> Self {
        Self {
            workflow: Arc::new(RwLock::new(workflow)),
        }
    }
}

#[tool]
impl GitWorkflowTools {
    /// 检查 Git 状态
    pub fn git_status(&self) -> Result<String, String> {
        let workflow = self.workflow.read();
        let status = workflow.git_status()
            .map_err(|e| format!("获取 Git 状态失败：{}", e))?;
        
        let mut output = format!("分支：{}\n", status.branch);
        
        if !status.modified.is_empty() {
            output.push_str(&format!("修改的文件：{}\n", status.modified.join(", ")));
        }
        if !status.untracked.is_empty() {
            output.push_str(&format!("未跟踪的文件：{}\n", status.untracked.join(", ")));
        }
        if !status.deleted.is_empty() {
            output.push_str(&format!("删除的文件：{}\n", status.deleted.join(", ")));
        }
        
        if status.modified.is_empty() && status.untracked.is_empty() && status.deleted.is_empty() {
            output.push_str("工作区干净，无修改");
        }
        
        Ok(output)
    }

    /// 获取 Git diff 摘要
    pub fn get_diff_summary(&self) -> Result<String, String> {
        let workflow = self.workflow.read();
        workflow.get_diff_summary()
            .map_err(|e| format!("获取 diff 失败：{}", e))
    }

    /// 生成提交消息
    pub fn generate_commit_message(&self, changes_summary: String) -> Result<String, String> {
        let workflow = self.workflow.read();
        let (commit_type, message) = workflow.generate_commit_message(&changes_summary)
            .map_err(|e| format!("生成提交消息失败：{}", e))?;
        Ok(format!("{}: {}", commit_type, message))
    }

    /// 提交变更
    pub fn commit(&self, message: String, run_pre_commit: bool) -> Result<String, String> {
        let mut workflow = self.workflow.write();
        let record = workflow.commit(&message, run_pre_commit)
            .map_err(|e| format!("提交失败：{}", e))?;
        Ok(format!("提交成功：{} - {}", &record.hash[..8], record.message))
    }

    /// 推送到远程
    pub fn push(&self) -> Result<String, String> {
        let workflow = self.workflow.read();
        workflow.push()
            .map_err(|e| format!("推送失败：{}", e))
            .map(|_| "推送成功".to_string())
    }

    /// 执行回滚
    pub fn rollback(&self) -> Result<String, String> {
        let workflow = self.workflow.read();
        workflow.rollback()
            .map_err(|e| format!("回滚失败：{}", e))
            .map(|_| "回滚成功".to_string())
    }

    /// 获取提交历史
    pub fn get_commit_history(&self) -> Result<String, String> {
        let workflow = self.workflow.read();
        let history = workflow.commits();
        
        let mut output = String::new();
        for commit in history {
            output.push_str(&format!(
                "{} - {} ({})\n",
                &commit.hash[..8],
                commit.message,
                commit.commit_type
            ));
        }
        
        Ok(output)
    }

    /// 执行预提交检查
    pub fn pre_commit_check(&self) -> Result<String, String> {
        let workflow = self.workflow.read();
        match workflow.pre_commit_check() {
            Ok(_) => Ok("预提交检查通过".to_string()),
            Err(e) => Err(format!("预提交检查失败：{}", e)),
        }
    }

    /// 设置回滚检查点
    pub fn set_rollback_checkpoint(&self) -> Result<String, String> {
        let mut workflow = self.workflow.write();
        workflow.set_rollback_checkpoint()
            .map_err(|e| format!("设置检查点失败：{}", e))
            .map(|_| "回滚检查点已设置".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokitai::ToolProvider;
    use tempfile::TempDir;

    #[test]
    fn test_git_workflow_tools_creation() {
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().to_path_buf();
        let storage_dir = temp_dir.path().join(".tokitai").join("autonomy");
        
        // 初始化 Git 仓库
        std::process::Command::new("git")
            .arg("init")
            .current_dir(&repo_dir)
            .output()
            .expect("Failed to init git repo");
        
        let tools = GitWorkflowTools::new(repo_dir, storage_dir);
        assert!(tools.is_ok());
    }

    #[test]
    fn test_tool_definitions() {
        let defs = GitWorkflowTools::tool_definitions();
        assert!(!defs.is_empty());
        assert!(defs.iter().any(|d| d.name == "git_status"));
        assert!(defs.iter().any(|d| d.name == "commit"));
        assert!(defs.iter().any(|d| d.name == "push"));
    }
}
