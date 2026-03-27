//! Experiment CLI for running benchmark experiments

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::info;

use crate::experiments::{
    ExperimentGroup, ExperimentConfig,
    runner::ExperimentRunner,
    collector::DataCollector,
    benchmark_tasks::BenchmarkTask,
};

/// Run experiment CLI command
pub async fn run_experiment_command(args: &[String]) -> Result<()> {
    println!("🔬 Tokitai Experiment Framework v1.0");
    println!("====================================\n");

    if args.is_empty() {
        print_help();
        return Ok(());
    }

    match args[0].as_str() {
        "run" => run_benchmark(args).await,
        "analyze" => analyze_results(args),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        _ => {
            eprintln!("❌ Unknown command: {}", args[0]);
            print_help();
            std::process::exit(1);
        }
    }
}

/// Print help information
fn print_help() {
    println!("Tokitai Experiment CLI - Benchmark Testing Tool");
    println!();
    println!("Usage:");
    println!("  cargo run --release -- experiment <command> [options]");
    println!();
    println!("Commands:");
    println!("  run      Run benchmark experiments");
    println!("  analyze  Analyze existing experiment results");
    println!("  help     Show this help message");
    println!();
    println!("Examples:");
    println!("  # Run single group benchmark");
    println!("  cargo run --release -- experiment run --group Ours-Full --days 1");
    println!();
    println!("  # Run all comparison groups");
    println!("  cargo run --release -- experiment run --all-groups");
    println!();
    println!("  # Run ablation study");
    println!("  cargo run --release -- experiment run --ablation");
    println!();
    println!("  # Analyze results");
    println!("  cargo run --release -- experiment analyze");
}

/// Parse command line arguments
fn parse_args(args: &[String]) -> Result<ExperimentArgs> {
    let mut parsed = ExperimentArgs::default();
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--group" | "-g" => {
                i += 1;
                if i >= args.len() {
                    anyhow::bail!("Missing group name after --group");
                }
                let group_str = &args[i];
                parsed.group = ExperimentGroup::from_str(group_str)
                    .with_context(|| format!("Invalid group: {}. Valid options: control, ours-full, ours-single, ours-nocot, ours-nofix", group_str))?;
                parsed.single_group = true;
            }
            "--days" | "-d" => {
                i += 1;
                if i >= args.len() {
                    anyhow::bail!("Missing days value after --days");
                }
                parsed.days = args[i].parse()
                    .with_context(|| "Invalid days value")?;
            }
            "--all-groups" | "-a" => {
                parsed.all_groups = true;
            }
            "--ablation" => {
                parsed.ablation = true;
            }
            "--project-path" | "-p" => {
                i += 1;
                if i >= args.len() {
                    anyhow::bail!("Missing path after --project-path");
                }
                parsed.project_path = PathBuf::from(&args[i]);
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => {
                anyhow::bail!("Unknown option: {}", args[i]);
            }
        }
        i += 1;
    }

    Ok(parsed)
}

/// Experiment arguments
#[derive(Debug, Clone)]
struct ExperimentArgs {
    group: ExperimentGroup,
    days: u32,
    all_groups: bool,
    ablation: bool,
    single_group: bool,
    project_path: PathBuf,
}

impl Default for ExperimentArgs {
    fn default() -> Self {
        Self {
            group: ExperimentGroup::OursFull,
            days: 1,
            all_groups: false,
            ablation: false,
            single_group: false,
            project_path: std::env::current_dir().unwrap_or_default(),
        }
    }
}

/// Run benchmark experiments
async fn run_benchmark(args: &[String]) -> Result<()> {
    let parsed = parse_args(args)?;
    
    // Load benchmark tasks
    let tasks = load_benchmark_tasks(&parsed.project_path)?;
    println!("📊 Loaded {} benchmark tasks\n", tasks.len());

    // Determine which groups to run
    let groups_to_run = determine_groups(&parsed);
    println!("🔬 Running experiments for {} groups\n", groups_to_run.len());

    let mut all_summaries = Vec::new();

    // Run each group
    for group in &groups_to_run {
        println!("════════════════════════════════════════");
        println!("Running: {} ({})", group.description(), group.log_dir_name());
        println!("════════════════════════════════════════\n");

        let config = ExperimentConfig {
            days: parsed.days,
            project_path: parsed.project_path.clone(),
            log_dir: parsed.project_path.join("experiments/logs"),
            verbose: true,
        };

        let runner = ExperimentRunner::new(config, group.clone())?;
        let summary = runner.run_benchmark(&tasks).await?;
        
        println!("\n✅ {} completed", group.description());
        println!("   Tasks: {}", summary.total_tasks);
        println!("   Success rate: {:.1}%", summary.success_rate * 100.0);
        println!("   Avg tool calls: {:.1}", summary.avg_tool_calls);
        println!("   Avg satisfaction: {:.1}/5", summary.avg_satisfaction);
        println!();

        all_summaries.push(summary);
    }

    // Generate combined summary
    if all_summaries.len() > 1 {
        save_combined_summary(&all_summaries, &parsed.project_path)?;
    }

    println!("════════════════════════════════════════");
    println!("✅ All experiments completed!");
    println!("════════════════════════════════════════");
    println!();
    println!("📊 Results saved to: experiments/logs/");
    println!();
    println!("Next steps:");
    println!("  1. Review logs in experiments/logs/<group>/");
    println!("  2. Run analysis: cargo run --release -- experiment analyze");
    println!("  3. Generate charts: python experiments/scripts/analyze_results.py");
    println!();

    Ok(())
}

/// Analyze existing experiment results
fn analyze_results(args: &[String]) -> Result<()> {
    println!("📊 Analyzing experiment results...\n");

    let project_path = if args.len() > 1 && !args[1].starts_with('-') {
        PathBuf::from(&args[1])
    } else {
        std::env::current_dir().unwrap_or_default()
    };

    let collector = DataCollector::new(project_path.join("experiments"), "all".to_string());

    // Load and analyze results for all groups
    let groups = [
        ExperimentGroup::Control,
        ExperimentGroup::OursFull,
        ExperimentGroup::OursSingle,
        ExperimentGroup::OursNoCoT,
        ExperimentGroup::OursNoFix,
    ];

    println!("{:<20} {:>10} {:>12} {:>12} {:>12} {:>10}", 
             "Group", "Tasks", "Success", "Avg Tools", "Avg Time", "Satisfaction");
    println!("{:-<80}", "");

    for group in &groups {
        let group_collector = DataCollector::new(
            project_path.join("experiments"),
            group.log_dir_name().to_string(),
        );

        match group_collector.load_task_records() {
            Ok(records) if !records.is_empty() => {
                let metrics = crate::experiments::collector::ExperimentMetrics::from_records(
                    group.description(),
                    &records,
                    &[],
                );

                let success_pct = metrics.success_rate * 100.0;
                println!("{:<20} {:>10} {:>11.1}% {:>12.1} {:>11.0}ms {:>10.1}/5",
                         group.description(),
                         metrics.total_tasks,
                         success_pct,
                         metrics.avg_tool_calls,
                         metrics.avg_execution_time_ms,
                         metrics.avg_satisfaction);
            }
            _ => {
                println!("{:<20} {:>10}", group.description(), "No data");
            }
        }
    }

    println!();
    println!("💡 Tip: Run Python analysis script for detailed reports:");
    println!("   python experiments/scripts/analyze_results.py");
    println!();

    Ok(())
}

/// Load benchmark tasks from JSON file
fn load_benchmark_tasks(project_path: &Path) -> Result<Vec<BenchmarkTask>> {
    use crate::experiments::benchmark_tasks::load_benchmark_tasks_from_file;

    let tasks_file = project_path.join("experiments/tasks/benchmark_tasks.json");
    
    let tasks = load_benchmark_tasks_from_file(&tasks_file)
        .with_context(|| format!("Failed to load benchmark tasks from {:?}", tasks_file))?;

    Ok(tasks)
}

/// Determine which groups to run based on args
fn determine_groups(args: &ExperimentArgs) -> Vec<ExperimentGroup> {
    if args.all_groups {
        vec![
            ExperimentGroup::Control,
            ExperimentGroup::OursFull,
            ExperimentGroup::OursSingle,
            ExperimentGroup::OursNoCoT,
            ExperimentGroup::OursNoFix,
        ]
    } else if args.ablation {
        vec![
            ExperimentGroup::OursFull,
            ExperimentGroup::OursSingle,
            ExperimentGroup::OursNoCoT,
            ExperimentGroup::OursNoFix,
        ]
    } else if args.single_group {
        vec![args.group.clone()]
    } else {
        vec![ExperimentGroup::OursFull]
    }
}

/// Save combined summary for all groups
fn save_combined_summary(
    summaries: &[crate::experiments::GroupSummary],
    project_path: &Path,
) -> Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;

    let analysis_dir = project_path.join("experiments/analysis");
    std::fs::create_dir_all(&analysis_dir)?;

    let summary_file = analysis_dir.join("all_groups_summary.json");
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&summary_file)?;

    let mut groups_data = serde_json::Map::new();
    for summary in summaries {
        let json = serde_json::to_value(summary)?;
        groups_data.insert(summary.group.clone(), json);
    }

    let output = serde_json::json!({
        "generated_at": chrono::Utc::now().to_rfc3339(),
        "groups": groups_data
    });

    writeln!(file, "{}", serde_json::to_string_pretty(&output)?)?;

    info!("Combined summary saved to: {:?}", summary_file);
    Ok(())
}
