//! 平行上下文 CLI 命令
//!
//! 提供 Git 风格的平行上下文管理命令：
//! - `tokitai ctx branch [name]` - 创建或列出分支
//! - `tokitai ctx checkout <branch>` - 切换分支
//! - `tokitai ctx merge <source> [target]` - 合并分支
//! - `tokitai ctx abort <branch>` - 废弃分支
//! - `tokitai ctx diff <branch1> [branch2]` - 查看差异
//! - `tokitai ctx log [branch]` - 查看历史
//! - `tokitai ctx time-travel <branch> <hash>` - 时间旅行

use anyhow::{Context, Result};
use std::path::PathBuf;

use tokitai_context::{MergeStrategy, ParallelContextManager, ParallelContextManagerConfig};

/// 处理平行上下文命令
pub fn handle_context_command(args: &[String]) -> Result<()> {
    if args.is_empty() {
        print_help();
        return Ok(());
    }

    let command = &args[0];

    // 查找 --context-root 参数
    let context_root = find_context_root(args)?;

    match command.as_str() {
        "branch" => handle_branch_command(&args[1..], &context_root),
        "checkout" => handle_checkout_command(&args[1..], &context_root),
        "merge" => handle_merge_command(&args[1..], &context_root),
        "abort" => handle_abort_command(&args[1..], &context_root),
        "diff" => handle_diff_command(&args[1..], &context_root),
        "log" => handle_log_command(&args[1..], &context_root),
        "time-travel" => handle_time_travel_command(&args[1..], &context_root),
        "status" => handle_status_command(&context_root),
        "init" => handle_init_command(&context_root),
        _ => {
            eprintln!("❌ 未知命令：{}", command);
            eprintln!();
            print_help();
            Ok(())
        }
    }
}

/// 打印帮助信息
fn print_help() {
    println!("🌿 Tokitai 平行上下文管理");
    println!();
    println!("用法：cargo run -- context <command> [arguments]");
    println!();
    println!("命令:");
    println!("  init                            初始化平行上下文系统");
    println!("  branch [name]                   创建新分支或列出所有分支");
    println!("  checkout <branch>               切换到指定分支");
    println!("  merge <source> [target]         合并分支（默认合并到 main）");
    println!("  abort <branch>                  废弃分支");
    println!("  diff <branch1> [branch2]        比较两个分支的差异");
    println!("  log [branch]                    查看分支历史");
    println!("  time-travel <branch> <hash>     回溯到历史状态");
    println!("  status                          查看当前状态");
    println!();
    println!("全局选项:");
    println!("  --context-root <path>           指定上下文根目录（默认：.context）");
    println!();
    println!("示例:");
    println!("  cargo run -- context init");
    println!("  cargo run -- context branch feature-auth");
    println!("  cargo run -- context branch");
    println!("  cargo run -- context checkout feature-auth");
    println!("  cargo run -- context merge feature-auth main");
    println!("  cargo run -- context abort feature-auth");
    println!("  cargo run -- context diff main feature-auth");
    println!("  cargo run -- context log main");
}

/// 查找 context-root 参数
fn find_context_root(args: &[String]) -> Result<PathBuf> {
    for i in 0..args.len() {
        if args[i] == "--context-root" && i + 1 < args.len() {
            return Ok(PathBuf::from(&args[i + 1]));
        }
    }
    // 默认使用当前目录的 .context
    Ok(PathBuf::from(".context"))
}

/// 创建平行上下文管理器
fn create_manager(context_root: &PathBuf) -> Result<ParallelContextManager> {
    let config = ParallelContextManagerConfig {
        context_root: context_root.clone(),
        default_merge_strategy: MergeStrategy::SelectiveMerge,
        auto_cleanup_abandoned: false,
        branch_ttl_hours: None,
    };

    ParallelContextManager::new(config)
        .with_context(|| format!("Failed to create context manager at {:?}", context_root))
}

/// 处理 init 命令
fn handle_init_command(context_root: &PathBuf) -> Result<()> {
    println!("🚀 初始化平行上下文系统...");

    let manager = create_manager(context_root)?;

    println!("✅ 平行上下文系统已初始化");
    println!("   根目录：{:?}", context_root);
    println!("   当前分支：main");

    // 显示统计信息
    let stats = manager.stats();
    println!();
    println!("📊 统计信息:");
    println!("   总分支数：{}", stats.total_branches);
    println!("   活跃分支：{}", stats.active_branches);

    Ok(())
}

/// 处理 branch 命令
fn handle_branch_command(args: &[String], context_root: &PathBuf) -> Result<()> {
    let mut manager = create_manager(context_root)?;

    if args.is_empty() {
        // 列出所有分支
        let branches = manager.list_branches();
        let current = manager.get_current_branch();

        println!("📋 分支列表:");
        println!();

        if branches.is_empty() {
            println!("   (无分支)");
        } else {
            for branch in branches {
                let is_current = current
                    .as_ref()
                    .map(|c| c.branch_id == branch.branch_id)
                    .unwrap_or(false);

                let marker = if is_current { "*" } else { " " };
                let state = format!("{:?}", branch.state);

                println!(
                    "{} {:<30} [{}]",
                    marker,
                    branch.branch_name,
                    state.to_lowercase()
                );
            }
        }

        println!();
        println!("提示：使用 'cargo run -- context branch <name>' 创建新分支");
    } else {
        // 创建新分支
        let branch_name = &args[0];
        let from_branch = args.get(1).map(|s| s.as_str()).unwrap_or("main");

        println!("🌿 创建分支 '{}' 从 '{}'...", branch_name, from_branch);

        let branch = manager.create_branch(branch_name, from_branch)?;

        println!("✅ 分支创建成功");
        println!("   分支 ID: {}", branch.branch_id);
        println!("   名称：{}", branch.branch_name);
        println!("   父分支：{}", branch.parent_branch);
        println!("   状态：{:?}", branch.state);
    }

    Ok(())
}

/// 处理 checkout 命令
fn handle_checkout_command(args: &[String], context_root: &PathBuf) -> Result<()> {
    if args.is_empty() {
        eprintln!("❌ 错误：缺少分支名称");
        eprintln!("用法：context checkout <branch>");
        return Ok(());
    }

    let branch_name = &args[0];
    let mut manager = create_manager(context_root)?;

    // 先查找分支
    let branch = manager
        .get_branch(branch_name)
        .or_else(|| {
            // 尝试通过分支名称查找（而非 branch_id）
            manager
                .list_branches()
                .iter()
                .find(|b| b.branch_name == *branch_name)
                .cloned()
        })
        .with_context(|| format!("分支不存在：{}", branch_name))?;

    let branch_id = branch.branch_id.clone();

    println!("🔄 切换到分支 '{}'...", branch_name);

    manager.checkout(&branch_id)?;

    println!("✅ 已切换到分支 '{}'", branch_name);

    Ok(())
}

/// 处理 merge 命令
fn handle_merge_command(args: &[String], context_root: &PathBuf) -> Result<()> {
    if args.is_empty() {
        eprintln!("❌ 错误：缺少源分支名称");
        eprintln!("用法：context merge <source> [target]");
        return Ok(());
    }

    let source_branch = &args[0];
    let target_branch = args.get(1).map(|s| s.as_str()).unwrap_or("main");

    let mut manager = create_manager(context_root)?;

    println!("🔀 合并 '{}' 到 '{}'...", source_branch, target_branch);

    // 查找源分支
    let source = manager
        .get_branch(source_branch)
        .or_else(|| {
            manager
                .list_branches()
                .iter()
                .find(|b| b.branch_name == *source_branch)
                .cloned()
        })
        .with_context(|| format!("源分支不存在：{}", source_branch))?;

    let source_id = source.branch_id.clone();

    // 执行合并
    let result = manager.merge(&source_id, target_branch, None)?;

    if result.success {
        println!("✅ 合并成功");
        println!("   合并的项目：{}", result.merged_count);
        println!("   冲突数量：{}", result.conflict_count);
        println!("   解决的冲突：{}", result.resolved_count);
    } else {
        println!("⚠️  合并失败");
        if let Some(error) = &result.error {
            println!("   错误：{}", error);
        }
    }

    Ok(())
}

/// 处理 abort 命令
fn handle_abort_command(args: &[String], context_root: &PathBuf) -> Result<()> {
    if args.is_empty() {
        eprintln!("❌ 错误：缺少分支名称");
        eprintln!("用法：context abort <branch>");
        return Ok(());
    }

    let branch_name = &args[0];
    let mut manager = create_manager(context_root)?;

    // 查找分支
    let branch = manager
        .get_branch(branch_name)
        .or_else(|| {
            manager
                .list_branches()
                .iter()
                .find(|b| b.branch_name == *branch_name)
                .cloned()
        })
        .with_context(|| format!("分支不存在：{}", branch_name))?;

    if branch.branch_name == "main" {
        eprintln!("❌ 错误：不能废弃 main 分支");
        return Ok(());
    }

    let branch_id = branch.branch_id.clone();

    println!("🗑️  废弃分支 '{}'...", branch_name);

    manager.abort_branch(&branch_id)?;

    println!("✅ 分支 '{}' 已废弃", branch_name);

    Ok(())
}

/// 处理 diff 命令
fn handle_diff_command(args: &[String], context_root: &PathBuf) -> Result<()> {
    if args.is_empty() {
        eprintln!("❌ 错误：缺少分支名称");
        eprintln!("用法：context diff <branch1> [branch2]");
        return Ok(());
    }

    let branch1 = &args[0];
    let branch2 = args.get(1).map(|s| s.as_str()).unwrap_or("main");

    let manager = create_manager(context_root)?;

    // 查找分支
    let b1 = manager
        .get_branch(branch1)
        .or_else(|| {
            manager
                .list_branches()
                .iter()
                .find(|b| b.branch_name == *branch1)
                .cloned()
        })
        .with_context(|| format!("分支不存在：{}", branch1))?;

    let b2 = manager
        .get_branch(branch2)
        .or_else(|| {
            manager
                .list_branches()
                .iter()
                .find(|b| b.branch_name == *branch2)
                .cloned()
        })
        .with_context(|| format!("分支不存在：{}", branch2))?;

    let diff = manager.diff(&b1.branch_id, &b2.branch_id)?;

    println!("📊 分支差异：{} vs {}", branch1, branch2);
    println!();

    if !diff.added_items.is_empty() {
        println!("➕ 新增项目 ({}):", diff.added_items.len());
        for item in &diff.added_items {
            println!("   • {} ({})", item.id, item.layer);
        }
        println!();
    }

    if !diff.removed_items.is_empty() {
        println!("➖ 删除项目 ({}):", diff.removed_items.len());
        for item in &diff.removed_items {
            println!("   • {} ({})", item.id, item.layer);
        }
        println!();
    }

    if !diff.modified_items.is_empty() {
        println!("✏️  修改项目 ({}):", diff.modified_items.len());
        for item in &diff.modified_items {
            println!("   • {} ({})", item.id, item.layer);
        }
        println!();
    }

    if !diff.conflicts.is_empty() {
        println!("⚠️  潜在冲突 ({}):", diff.conflicts.len());
        for conflict in &diff.conflicts {
            println!("   • {} ({:?})", conflict.item_id, conflict.conflict_type);
        }
        println!();
    }

    let total_changes =
        diff.added_items.len() + diff.removed_items.len() + diff.modified_items.len();
    if total_changes == 0 && diff.conflicts.is_empty() {
        println!("✅ 两个分支没有差异");
    }

    Ok(())
}

/// 处理 log 命令
fn handle_log_command(args: &[String], context_root: &PathBuf) -> Result<()> {
    let branch_name = args.first().map(|s| s.as_str()).unwrap_or("main");

    let manager = create_manager(context_root)?;

    // 查找分支
    let branch = manager
        .get_branch(branch_name)
        .or_else(|| {
            manager
                .list_branches()
                .iter()
                .find(|b| b.branch_name == *branch_name)
                .cloned()
        })
        .with_context(|| format!("分支不存在：{}", branch_name))?;

    println!("📜 分支历史：{}", branch_name);
    println!();

    let nodes = manager.log(&branch.branch_id, 20)?;

    if nodes.is_empty() {
        println!("   (无历史记录)");
    } else {
        for (i, node) in nodes.iter().enumerate().rev() {
            let marker = if i == nodes.len() - 1 { "🌱" } else { "  " };
            println!("{} {} - {}", marker, &node.hash[..10], node.hash);
        }
    }

    Ok(())
}

/// 处理 time-travel 命令
fn handle_time_travel_command(args: &[String], context_root: &PathBuf) -> Result<()> {
    if args.len() < 2 {
        eprintln!("❌ 错误：缺少参数");
        eprintln!("用法：context time-travel <branch> <hash>");
        return Ok(());
    }

    let branch_name = &args[0];
    let target_hash = &args[1];

    let mut manager = create_manager(context_root)?;

    println!("⏳ 时间旅行到 {} 的 {}...", branch_name, &target_hash[..10]);

    let temp_branch_id = manager.time_travel(branch_name, target_hash)?;

    println!("✅ 已创建临时分支 '{}'", temp_branch_id);
    println!("   提示：使用 'cargo run -- context checkout main' 返回主分支");

    Ok(())
}

/// 处理 status 命令
fn handle_status_command(context_root: &PathBuf) -> Result<()> {
    let manager = create_manager(context_root)?;

    let current = manager.get_current_branch();
    let stats = manager.stats();
    let cow_stats = manager.cow_stats();

    println!("📊 平行上下文状态");
    println!("{}", "=".repeat(50));
    println!();

    if let Some(branch) = current {
        println!("当前分支：{} ({})", branch.branch_name, branch.branch_id);
        println!("分支状态：{:?}", branch.state);
        println!(
            "创建时间：{}",
            branch.fork_point.format("%Y-%m-%d %H:%M:%S")
        );
        println!("父分支：{}", branch.parent_branch);
        println!();
    }

    println!("统计信息:");
    println!("  总分支数：{}", stats.total_branches);
    println!("  活跃分支：{}", stats.active_branches);
    println!("  已合并：{}", stats.merged_branches);
    println!("  已废弃：{}", stats.abandoned_branches);
    println!("  总合并次数：{}", stats.total_merges);
    println!("  成功合并：{}", stats.successful_merges);
    println!();

    println!("COW 统计:");
    println!("  符号链接数：{}", cow_stats.total_symlinks);
    println!("  已写入链接：{}", cow_stats.written_symlinks);
    if cow_stats.total_symlinks > 0 {
        let ratio = cow_stats.written_symlinks as f64 / cow_stats.total_symlinks as f64 * 100.0;
        println!("  COW 触发率：{:.2}%", ratio);
    }

    Ok(())
}
