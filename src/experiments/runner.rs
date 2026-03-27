//! Experiment runner for executing benchmark tasks across different experiment groups

use anyhow::{Context, Result};
use serde_json;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::experiments::{
    ExperimentConfig, ExperimentGroup, TaskExecutionRecord, ToolCallRecord,
    EvolutionCycleRecord, ReflectionRecord, GapRecord, EvolutionMetrics,
    GroupSummary,
};
use crate::autonomy::hybrid_gap_detector::HybridGapDetector;

/// Experiment runner for executing benchmark tasks
pub struct ExperimentRunner {
    /// Experiment configuration
    config: ExperimentConfig,
    /// Current experiment group
    group: ExperimentGroup,
    /// Task execution records
    records: Arc<Mutex<Vec<TaskExecutionRecord>>>,
    /// Evolution cycle records
    evolution_records: Arc<Mutex<Vec<EvolutionCycleRecord>>>,
    /// Gap detector (for evolution groups)
    gap_detector: Option<Arc<Mutex<HybridGapDetector>>>,
    /// Project path
    project_path: PathBuf,
}

impl ExperimentRunner {
    /// Create a new experiment runner
    pub fn new(config: ExperimentConfig, group: ExperimentGroup) -> Result<Self> {
        let project_path = config.project_path.clone();
        
        // Initialize gap detector for evolution groups
        let gap_detector = if group.has_evolution() {
            let data_dir = project_path.join(".tokitai").join("evolution");
            std::fs::create_dir_all(&data_dir)
                .with_context(|| "Failed to create evolution data directory")?;
            
            // Use statistical-only detector for experiments (no LLM calls)
            let detector = HybridGapDetector::new_statistical_only(data_dir)?;
            
            Some(Arc::new(Mutex::new(detector)))
        } else {
            None
        };

        Ok(Self {
            config,
            group,
            records: Arc::new(Mutex::new(Vec::new())),
            evolution_records: Arc::new(Mutex::new(Vec::new())),
            gap_detector,
            project_path,
        })
    }

    /// Get the log directory for this experiment
    pub fn log_dir(&self) -> PathBuf {
        let experiments_dir = self.project_path.join("experiments");
        experiments_dir
            .join("logs")
            .join(self.group.log_dir_name())
    }

    /// Ensure log directory exists
    pub fn ensure_log_dir(&self) -> Result<PathBuf> {
        let log_dir = self.log_dir();
        std::fs::create_dir_all(&log_dir)
            .with_context(|| format!("Failed to create log directory: {:?}", log_dir))?;
        Ok(log_dir)
    }

    /// Run a single benchmark task
    pub async fn run_task(
        &self,
        task_id: &str,
        category: &str,
        difficulty: &str,
        description: &str,
    ) -> Result<TaskExecutionRecord> {
        use std::time::Instant;
        
        let start_time = Instant::now();
        
        // Create task record
        let mut record = TaskExecutionRecord::new(
            task_id.to_string(),
            category.to_string(),
            difficulty.to_string(),
            description.to_string(),
            self.group.description().to_string(),
        );

        info!("Running task {}: {}", task_id, description);

        // Execute task based on group configuration
        let result = self.execute_task(description).await;

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        match result {
            Ok((tool_calls, satisfaction)) => {
                record.complete(true, tool_calls, execution_time_ms, satisfaction);
                info!("Task {} completed successfully", task_id);
            }
            Err(e) => {
                record.fail(e.to_string());
                warn!("Task {} failed: {}", task_id, e);
            }
        }

        // Update gap detector if evolution is enabled
        if let Some(ref detector) = self.gap_detector {
            if self.group.has_evolution() {
                let mut detector = detector.lock().await;
                // Record task execution for gap analysis
                detector.record_task_execution(
                    &record.task_id,
                    record.success,
                    record.total_tool_calls,
                ).await;
                
                // Update gap statistics in record
                let stats = detector.get_current_stats();
                record.gaps_detected = stats.gaps_detected;
                record.tools_created = stats.tools_created;
                record.tools_optimized = stats.tools_optimized;
            }
        }

        Ok(record)
    }

    /// Execute a task (placeholder - needs integration with actual task execution)
    async fn execute_task(&self, description: &str) -> Result<(Vec<ToolCallRecord>, u8)> {
        // TODO: Integrate with actual task execution system
        // For now, return a mock result
        
        // In real implementation:
        // 1. Parse task description
        // 2. Plan tool usage (with/without CoT based on group)
        // 3. Execute tools (with/without multi-agent negotiation)
        // 4. Self-correct if needed (with/without self-fix loop)
        // 5. Record all tool calls
        
        let tool_calls = vec![
            ToolCallRecord {
                tool: "mock_tool".to_string(),
                args: serde_json::json!({"description": description}),
                result: "success".to_string(),
                execution_time_ms: Some(100),
            }
        ];

        Ok((tool_calls, 4))
    }

    /// Run an evolution cycle (for evolution groups)
    pub async fn run_evolution_cycle(&self, cycle_num: u32) -> Result<Option<EvolutionCycleRecord>> {
        if !self.group.has_evolution() {
            return Ok(None);
        }

        let gap_detector = self.gap_detector.as_ref()
            .context("Gap detector not initialized")?;

        let mut detector = gap_detector.lock().await;
        
        // Detect gaps
        let gaps = detector.detect_gaps().await;
        
        // Collect metrics
        let metrics = detector.get_metrics();
        let stats = detector.get_current_stats();

        let record = EvolutionCycleRecord {
            cycle_id: format!("cycle_{:03}", cycle_num),
            group: self.group.description().to_string(),
            timestamp: chrono::Utc::now(),
            reflection: ReflectionRecord {
                coverage_score: 0.75,
                systemic_issues: vec!["Sample issue".to_string()],
                strategic_recommendations: vec!["Sample recommendation".to_string()],
            },
            gaps_detected: gaps.into_iter().take(5).map(|g| GapRecord {
                gap_type: format!("{:?}", g.gap_type),
                description: g.description,
                suggested_name: g.suggested_tool_name,
                priority: g.priority,
            }).collect(),
            actions_taken: vec![],
            metrics: EvolutionMetrics {
                api_calls: metrics.api_calls,
                api_cost_usd: metrics.api_cost_usd,
                cycle_duration_ms: 0,
            },
        };

        Ok(Some(record))
    }

    /// Record a task execution
    pub async fn record_task(&self, record: TaskExecutionRecord) {
        let mut records = self.records.lock().await;
        records.push(record);
    }

    /// Record an evolution cycle
    pub async fn record_evolution(&self, record: EvolutionCycleRecord) {
        let mut records = self.evolution_records.lock().await;
        records.push(record);
    }

    /// Run all benchmark tasks
    pub async fn run_benchmark(&self, tasks: &[crate::experiments::benchmark_tasks::BenchmarkTask]) -> Result<GroupSummary> {
        info!("Starting benchmark for group: {:?}", self.group);
        info!("Loading {} tasks", tasks.len());

        let mut task_num = 0;
        for task in tasks {
            task_num += 1;
            info!("[{}/{}] Running task: {}", task_num, tasks.len(), task.id);
            
            let record = self.run_task(
                &task.id,
                &task.category,
                &task.difficulty,
                &task.description,
            ).await?;
            
            self.record_task(record).await;
            
            // Run evolution cycle every 5 tasks
            if task_num % 5 == 0 && self.group.has_evolution() {
                let cycle_num = task_num / 5;
                if let Some(evo_record) = self.run_evolution_cycle(cycle_num).await? {
                    self.record_evolution(evo_record).await;
                }
            }
        }

        // Save logs
        self.save_logs().await?;

        // Generate summary
        let records = self.records.lock().await;
        let summary = GroupSummary::from_records(
            self.group.description(),
            &records,
        );

        info!("Benchmark completed for group: {:?}", self.group);
        info!("  Tasks completed: {}", summary.total_tasks);
        info!("  Success rate: {:.1}%", summary.success_rate * 100.0);
        info!("  Avg tool calls: {:.1}", summary.avg_tool_calls);

        Ok(summary)
    }

    /// Save logs to disk
    pub async fn save_logs(&self) -> Result<()> {
        let log_dir = self.ensure_log_dir()?;
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");

        // Save task execution logs
        let records = self.records.lock().await;
        let task_log_file = log_dir.join(format!("task_logs_{}.jsonl", timestamp));
        
        let mut file = tokio::fs::File::create(&task_log_file).await
            .with_context(|| format!("Failed to create task log file: {:?}", task_log_file))?;

        for record in records.iter() {
            let line = serde_json::to_string(record)
                .with_context(|| "Failed to serialize task record")?;
            tokio::io::AsyncWriteExt::write_all(&mut file, line.as_bytes()).await?;
            tokio::io::AsyncWriteExt::write_all(&mut file, b"\n").await?;
        }

        info!("Task logs saved to: {:?}", task_log_file);

        // Save evolution logs
        if !self.evolution_records.lock().await.is_empty() {
            let evo_records = self.evolution_records.lock().await;
            let evo_log_file = log_dir.join(format!("evolution_logs_{}.jsonl", timestamp));

            let mut file = tokio::fs::File::create(&evo_log_file).await
                .with_context(|| format!("Failed to create evolution log file: {:?}", evo_log_file))?;

            for record in evo_records.iter() {
                let line = serde_json::to_string(record)
                    .with_context(|| "Failed to serialize evolution record")?;
                tokio::io::AsyncWriteExt::write_all(&mut file, line.as_bytes()).await?;
                tokio::io::AsyncWriteExt::write_all(&mut file, b"\n").await?;
            }

            info!("Evolution logs saved to: {:?}", evo_log_file);
        }

        Ok(())
    }

    /// Get current summary
    pub async fn get_summary(&self) -> GroupSummary {
        let records = self.records.lock().await;
        GroupSummary::from_records(self.group.description(), &records)
    }
}
