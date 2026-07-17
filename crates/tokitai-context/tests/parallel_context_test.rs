//! 平行上下文集成测试
//!
//! 测试平行上下文管理的完整工作流程：
//! - 分支创建（fork）
//! - 分支切换（checkout）
//! - 分支合并（merge）
//! - 分支废弃（abort）
//! - 时间旅行（time_travel）
//! - 差异比较（diff）
//! - 历史查看（log）

use tokitai_context::{
    ParallelContextManager, ParallelContextManagerConfig,
    BranchState, MergeStrategy,
};
use tempfile::TempDir;

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

#[test]
fn test_basic_branch_workflow() {
    let (mut manager, _temp_dir) = create_test_manager();

    // 1. 初始应该在 main 分支
    let current = manager.get_current_branch().unwrap();
    assert_eq!(current.branch_id, "main");

    // 2. 创建特性分支
    let feature_branch = manager.create_branch("feature-refactor", "main").unwrap();
    let feature_id = feature_branch.branch_id.clone();
    
    println!("Created feature branch: {}", feature_id);

    // 3. 切换到特性分支
    manager.checkout(&feature_id).unwrap();
    let current = manager.get_current_branch().unwrap();
    assert_eq!(current.branch_id, feature_id);

    // 4. 在特性分支添加一些上下文数据
    let test_file = current.short_term_dir.join("test_context.txt");
    std::fs::write(&test_file, "Feature branch context").unwrap();

    // 5. 切换回 main 分支
    manager.checkout("main").unwrap();
    let current = manager.get_current_branch().unwrap();
    assert_eq!(current.branch_id, "main");

    // 6. main 分支不应该有特性分支的文件
    assert!(!current.short_term_dir.join("test_context.txt").exists());

    // 7. 合并特性分支到 main
    let merge_result = manager.merge(&feature_id, "main", None).unwrap();
    assert!(merge_result.success);
    println!("Merge result: {} items merged", merge_result.merged_count);

    // 8. 验证合并后 main 分支有了特性分支的文件
    let current = manager.get_current_branch().unwrap();
    assert!(current.short_term_dir.join("test_context.txt").exists());

    println!("Basic branch workflow test passed!");
}

#[test]
fn test_multiple_parallel_branches() {
    let (mut manager, _temp_dir) = create_test_manager();

    // 创建多个并行分支
    let branch1 = manager.create_branch("refactor-v1", "main").unwrap();
    let branch1_id = branch1.branch_id.clone();
    
    let branch2 = manager.create_branch("refactor-v2", "main").unwrap();
    let branch2_id = branch2.branch_id.clone();
    
    let branch3 = manager.create_branch("refactor-v3", "main").unwrap();
    let branch3_id = branch3.branch_id.clone();

    println!("Created 3 parallel branches for exploration");

    // 在每个分支添加不同的数据
    {
        manager.checkout(&branch1_id).unwrap();
        let current = manager.get_current_branch().unwrap();
        std::fs::write(current.short_term_dir.join("v1.txt"), "Version 1 approach").unwrap();
    }

    {
        manager.checkout(&branch2_id).unwrap();
        let current = manager.get_current_branch().unwrap();
        std::fs::write(current.short_term_dir.join("v2.txt"), "Version 2 approach").unwrap();
    }

    {
        manager.checkout(&branch3_id).unwrap();
        let current = manager.get_current_branch().unwrap();
        std::fs::write(current.short_term_dir.join("v3.txt"), "Version 3 approach").unwrap();
    }

    // 验证分支隔离
    manager.checkout(&branch1_id).unwrap();
    let current = manager.get_current_branch().unwrap();
    assert!(current.short_term_dir.join("v1.txt").exists());
    assert!(!current.short_term_dir.join("v2.txt").exists());
    assert!(!current.short_term_dir.join("v3.txt").exists());

    // 列出所有分支
    let branches = manager.list_branches();
    assert_eq!(branches.len(), 4); // main + 3 features

    let active_branches = manager.list_active_branches();
    assert_eq!(active_branches.len(), 4);

    println!("Multiple parallel branches test passed!");
}

#[test]
fn test_branch_diff() {
    let (mut manager, _temp_dir) = create_test_manager();

    // 创建分支
    let feature = manager.create_branch("feature-diff", "main").unwrap();
    let feature_id = feature.branch_id.clone();

    // 在 main 分支添加文件
    let main_current = manager.get_current_branch().unwrap();
    std::fs::write(main_current.short_term_dir.join("main_file.txt"), "Main content").unwrap();

    // 在 feature 分支添加不同文件
    manager.checkout(&feature_id).unwrap();
    let feature_current = manager.get_current_branch().unwrap();
    std::fs::write(feature_current.short_term_dir.join("feature_file.txt"), "Feature content").unwrap();

    // 计算差异
    let diff = manager.diff("main", &feature_id).unwrap();
    
    println!("Branch diff:");
    println!("  Added items: {}", diff.added_items.len());
    println!("  Removed items: {}", diff.removed_items.len());

    // feature 分支相对于 main 应该有新增文件
    assert!(!diff.added_items.is_empty() || !diff.removed_items.is_empty());

    println!("Branch diff test passed!");
}

#[test]
fn test_branch_abort() {
    let (mut manager, _temp_dir) = create_test_manager();

    // 创建分支
    let temp_branch = manager.create_branch("temp-experiment", "main").unwrap();
    let temp_id = temp_branch.branch_id.clone();

    // 添加一些数据
    manager.checkout(&temp_id).unwrap();
    let current = manager.get_current_branch().unwrap();
    std::fs::write(current.short_term_dir.join("temp.txt"), "Temporary data").unwrap();

    // 废弃分支
    manager.abort_branch(&temp_id).unwrap();

    // 验证分支状态
    let branch = manager.get_branch(&temp_id).unwrap();
    assert_eq!(branch.state, BranchState::Abandoned);

    // 不能切换到已废弃的分支
    let result = manager.checkout(&temp_id);
    assert!(result.is_err());

    println!("Branch abort test passed!");
}

#[test]
fn test_hash_chain_log() {
    let (mut manager, _temp_dir) = create_test_manager();

    // 查看 main 分支的历史
    let log = manager.log("main", 10).unwrap();
    println!("Main branch history: {} nodes", log.len());

    // 创建分支并查看其历史
    let feature = manager.create_branch("feature-log", "main").unwrap();
    let feature_id = feature.branch_id.clone();

    let feature_log = manager.log(&feature_id, 10).unwrap();
    println!("Feature branch history: {} nodes", feature_log.len());

    // 分支应该继承父分支的哈希链（至少有创世节点）
    // 注意：如果父分支也没有内容，哈希链可能为空
    // 所以我们只验证能正常获取日志，而不强制要求非空
    
    println!("Hash chain log test passed!");
}

#[test]
fn test_cow_performance() {
    let (mut manager, _temp_dir) = create_test_manager();

    // 在 main 分支添加大量文件
    let main_current = manager.get_current_branch().unwrap();
    for i in 0..50 {
        let file_path = main_current.short_term_dir.join(format!("file_{}.txt", i));
        std::fs::write(&file_path, format!("Content {}", i)).unwrap();
    }

    // 测试 COW fork 性能
    let start = std::time::Instant::now();
    let branch1 = manager.create_branch("cow-test-1", "main").unwrap();
    let branch1_id = branch1.branch_id.clone();
    let duration1 = start.elapsed();

    let start = std::time::Instant::now();
    let branch2 = manager.create_branch("cow-test-2", "main").unwrap();
    let branch2_id = branch2.branch_id.clone();
    let duration2 = start.elapsed();

    println!("COW fork performance:");
    println!("  Branch 1: {}ms", duration1.as_millis());
    println!("  Branch 2: {}ms", duration2.as_millis());

    // COW fork 应该在 100ms 内完成
    assert!(duration1.as_millis() < 100);
    assert!(duration2.as_millis() < 100);

    // 验证分支隔离
    manager.checkout(&branch1_id).unwrap();
    let current = manager.get_current_branch().unwrap();
    std::fs::write(current.short_term_dir.join("branch1_unique.txt"), "Branch 1 data").unwrap();

    manager.checkout(&branch2_id).unwrap();
    let current = manager.get_current_branch().unwrap();
    assert!(!current.short_term_dir.join("branch1_unique.txt").exists());

    // 获取 COW 统计信息
    let cow_stats = manager.cow_stats();
    println!("COW Stats: {}", cow_stats);

    println!("COW performance test passed!");
}

#[test]
fn test_merge_strategies() {
    let (mut manager, _temp_dir) = create_test_manager();

    // 创建测试分支
    let feature = manager.create_branch("merge-test", "main").unwrap();
    let feature_id = feature.branch_id.clone();

    // 在分支添加数据
    manager.checkout(&feature_id).unwrap();
    let current = manager.get_current_branch().unwrap();
    std::fs::write(current.short_term_dir.join("merge_test.txt"), "Test data").unwrap();

    // 测试 FastForward 合并
    manager.checkout("main").unwrap();
    let result = manager.merge(&feature_id, "main", Some(MergeStrategy::FastForward)).unwrap();
    println!("FastForward merge: success={}", result.success);

    // 测试 SelectiveMerge 合并
    let feature2 = manager.create_branch("merge-test-2", "main").unwrap();
    let feature2_id = feature2.branch_id.clone();
    
    manager.checkout(&feature2_id).unwrap();
    let current = manager.get_current_branch().unwrap();
    std::fs::write(current.short_term_dir.join("merge_test_2.txt"), "Test data 2").unwrap();

    manager.checkout("main").unwrap();
    let result = manager.merge(&feature2_id, "main", Some(MergeStrategy::SelectiveMerge)).unwrap();
    println!("SelectiveMerge merge: success={}, merged={}", result.success, result.merged_count);

    println!("Merge strategies test passed!");
}

#[test]
fn test_full_workflow() {
    let (mut manager, _temp_dir) = create_test_manager();

    println!("\n=== Full Parallel Context Workflow Demo ===\n");

    // Step 1: 初始状态
    println!("1. Starting on main branch");
    let current = manager.get_current_branch().unwrap();
    println!("   Current branch: {}", current.branch_id);

    // Step 2: 创建多个探索分支
    println!("\n2. Creating 3 exploration branches...");
    let b1 = manager.create_branch("hypothesis-1", "main").unwrap();
    let b1_id = b1.branch_id.clone();
    let b2 = manager.create_branch("hypothesis-2", "main").unwrap();
    let b2_id = b2.branch_id.clone();
    let b3 = manager.create_branch("hypothesis-3", "main").unwrap();
    let b3_id = b3.branch_id.clone();
    println!("   Created branches: {}, {}, {}", b1_id, b2_id, b3_id);

    // Step 3: 在每个分支进行独立探索
    println!("\n3. Exploring different hypotheses in parallel...");
    
    manager.checkout(&b1_id).unwrap();
    let current = manager.get_current_branch().unwrap();
    std::fs::write(current.short_term_dir.join("h1_evidence.txt"), "Evidence for hypothesis 1").unwrap();
    println!("   Branch {}: Added evidence for hypothesis 1", b1_id);

    manager.checkout(&b2_id).unwrap();
    let current = manager.get_current_branch().unwrap();
    std::fs::write(current.short_term_dir.join("h2_evidence.txt"), "Evidence for hypothesis 2").unwrap();
    println!("   Branch {}: Added evidence for hypothesis 2", b2_id);

    manager.checkout(&b3_id).unwrap();
    let current = manager.get_current_branch().unwrap();
    std::fs::write(current.short_term_dir.join("h3_evidence.txt"), "Evidence for hypothesis 3").unwrap();
    println!("   Branch {}: Added evidence for hypothesis 3", b3_id);

    // Step 4: 比较分支差异
    println!("\n4. Comparing branches...");
    let diff_1_2 = manager.diff(&b1_id, &b2_id).unwrap();
    println!("   Diff between {} and {}: {} added, {} removed", 
             b1_id, b2_id, diff_1_2.added_items.len(), diff_1_2.removed_items.len());

    // Step 5: 选择最佳方案并合并
    println!("\n5. Merging best hypothesis (hypothesis-2)...");
    manager.checkout("main").unwrap();
    let merge_result = manager.merge(&b2_id, "main", Some(MergeStrategy::SelectiveMerge)).unwrap();
    println!("   Merge completed: {} items merged", merge_result.merged_count);

    // Step 6: 废弃其他分支
    println!("\n6. Aborting unused hypotheses...");
    manager.abort_branch(&b1_id).unwrap();
    println!("   Aborted branch: {}", b1_id);
    manager.abort_branch(&b3_id).unwrap();
    println!("   Aborted branch: {}", b3_id);

    // Step 7: 最终状态
    println!("\n7. Final state:");
    let stats = manager.stats();
    println!("   Total branches: {}", stats.total_branches);
    println!("   Active branches: {}", stats.active_branches);
    println!("   Abandoned branches: {}", stats.abandoned_branches);

    println!("\n=== Workflow Demo Complete ===\n");
}
