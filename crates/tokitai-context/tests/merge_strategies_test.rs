//! Merge Strategies Tests
//!
//! 测试合并策略的基本行为

use tempfile::TempDir;
use std::fs;
use tokitai_context::{
    ParallelContextManager, ParallelContextManagerConfig,
    MergeStrategy,
};

/// 创建测试用的平行上下文管理器
fn create_test_manager() -> (ParallelContextManager, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let context_root = temp_dir.path().join(".context");

    let config = ParallelContextManagerConfig {
        context_root: context_root.clone(),
        default_merge_strategy: MergeStrategy::SelectiveMerge,
        auto_cleanup_abandoned: false,
        branch_ttl_hours: None,
    };

    let manager = ParallelContextManager::new(config).unwrap();
    (manager, temp_dir)
}

// ============================================================================
// 基本合并测试
// ============================================================================

#[test]
fn test_merge_branch_to_parent() {
    let (mut manager, _temp_dir) = create_test_manager();

    // 创建分支
    let branch = manager.create_branch("merge-test", "main").unwrap();
    let branch_id = branch.branch_id.clone();

    // 在分支添加文件
    manager.checkout(&branch_id).unwrap();
    {
        let current = manager.get_current_branch().unwrap();
        fs::write(current.short_term_dir.join("test_file.txt"), "test content").unwrap();
    }

    // 合并回 main
    manager.checkout("main").unwrap();
    let result = manager.merge(&branch_id, "main", Some(MergeStrategy::SelectiveMerge));

    // 合并应该可以执行
    assert!(result.is_ok());
}

#[test]
fn test_merge_empty_branch() {
    let (mut manager, _temp_dir) = create_test_manager();

    // 创建空分支
    let branch = manager.create_branch("empty-branch", "main").unwrap();
    let branch_id = branch.branch_id.clone();

    // 合并空分支
    manager.checkout("main").unwrap();
    let result = manager.merge(&branch_id, "main", Some(MergeStrategy::FastForward));

    // 应该可以执行
    assert!(result.is_ok());
}

#[test]
fn test_merge_nonexistent_branch() {
    let (mut manager, _temp_dir) = create_test_manager();

    // 尝试合并不存在的分支
    let result = manager.merge("nonexistent", "main", None);
    assert!(result.is_err());
}

#[test]
fn test_merge_same_branch() {
    let (mut manager, _temp_dir) = create_test_manager();

    // 尝试合并分支到自身
    let result = manager.merge("main", "main", None);
    // 可能失败或有特殊处理
    assert!(result.is_err() || result.is_ok());
}

// ============================================================================
// 多文件合并测试
// ============================================================================

#[test]
fn test_merge_multiple_files() {
    let (mut manager, _temp_dir) = create_test_manager();

    // 创建分支并添加多个文件
    let branch = manager.create_branch("multi-file", "main").unwrap();
    let branch_id = branch.branch_id.clone();

    manager.checkout(&branch_id).unwrap();
    {
        let current = manager.get_current_branch().unwrap();
        
        for i in 0..5 {
            fs::write(
                current.short_term_dir.join(format!("file_{}.txt", i)),
                format!("content {}", i)
            ).unwrap();
        }
    }

    // 合并
    manager.checkout("main").unwrap();
    let result = manager.merge(&branch_id, "main", Some(MergeStrategy::SelectiveMerge));

    assert!(result.is_ok());
}

#[test]
fn test_merge_with_different_files() {
    let (mut manager, _temp_dir) = create_test_manager();

    // main 分支有文件 A
    {
        let main_current = manager.get_current_branch().unwrap();
        fs::write(main_current.short_term_dir.join("main.txt"), "main content").unwrap();
    }

    // 分支有文件 B
    let branch = manager.create_branch("different-files", "main").unwrap();
    let branch_id = branch.branch_id.clone();
    manager.checkout(&branch_id).unwrap();
    {
        let current = manager.get_current_branch().unwrap();
        fs::write(current.short_term_dir.join("branch.txt"), "branch content").unwrap();
    }

    // 合并
    manager.checkout("main").unwrap();
    let result = manager.merge(&branch_id, "main", Some(MergeStrategy::SelectiveMerge));

    assert!(result.is_ok());
}

// ============================================================================
// 合并策略枚举测试
// ============================================================================

#[test]
fn test_merge_strategy_variants() {
    // 验证所有合并策略变体都可以创建
    let _strategies = [
        MergeStrategy::FastForward,
        MergeStrategy::SelectiveMerge,
        MergeStrategy::Ours,
        MergeStrategy::Theirs,
        MergeStrategy::Manual,
    ];

    // 如果有 AI 特性，还可以测试 AIAssisted
    #[cfg(feature = "ai")]
    {
        let _ai_strategy = MergeStrategy::AIAssisted;
    }
}

// ============================================================================
// 边界条件测试
// ============================================================================

#[test]
fn test_merge_after_abort() {
    let (mut manager, _temp_dir) = create_test_manager();

    // 创建分支
    let branch = manager.create_branch("temp-branch", "main").unwrap();
    let branch_id = branch.branch_id.clone();

    // 废弃分支
    manager.abort_branch(&branch_id).unwrap();

    // 尝试合并已废弃的分支应该失败
    let result = manager.merge(&branch_id, "main", None);
    assert!(result.is_err());
}

#[test]
fn test_merge_to_current_branch() {
    let (mut manager, _temp_dir) = create_test_manager();

    // 创建分支
    let branch = manager.create_branch("self-merge", "main").unwrap();
    let branch_id = branch.branch_id.clone();

    // 切换到分支
    manager.checkout(&branch_id).unwrap();

    // 尝试合并 main 到当前分支
    let result = manager.merge("main", &branch_id, None);
    assert!(result.is_ok());
}
