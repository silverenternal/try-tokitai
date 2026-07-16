//! 实验报告生成器
//!
//! # 设计目标
//! 生成符合学术论文标准的实验报告，包括：
//! - LaTeX 表格（可直接用于论文）
//! - Markdown 报告（用于快速预览）
//! - JSON 数据（用于进一步分析）
//! - 图表数据（用于可视化）

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Write, BufWriter};
use std::path::{Path, PathBuf};
use anyhow::{Result};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use super::framework::{
    ExperimentConfig, ExperimentGroupType, GroupExperimentResult,
    ComparativeExperimentResult, AblationExperimentResult, CoreMetrics,
};
use super::metrics::{MetricsReport, ComparisonResult};
use super::statistical_analysis::{
    DescriptiveStats, TTestResult, AnovaResult, MannWhitneyResult,
    EffectSizeMagnitude,
};

// ============================================================================
// 实验报告结构
// ============================================================================

/// 完整实验报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentReport {
    /// 报告 ID
    pub report_id: String,
    /// 实验配置
    pub config: ExperimentConfig,
    /// 生成时间
    pub generated_at: DateTime<Utc>,
    /// 对比实验结果
    pub comparative_results: Option<ComparativeExperimentResult>,
    /// 消融实验结果
    pub ablation_results: Option<AblationExperimentResult>,
    /// 统计检验结果
    pub statistical_tests: Vec<StatisticalTestResult>,
    /// 文本摘要
    pub executive_summary: String,
}

/// 统计检验结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalTestResult {
    /// 检验名称
    pub test_name: String,
    /// 比较的组别
    pub groups: Vec<String>,
    /// t 检验结果（如果适用）
    pub t_test: Option<TTestResult>,
    /// ANOVA 结果（如果适用）
    pub anova: Option<AnovaResult>,
    /// Mann-Whitney U 检验结果（如果适用）
    pub mann_whitney: Option<MannWhitneyResult>,
    /// 效应量解释
    pub effect_size_interpretation: String,
}

// ============================================================================
// 报告生成器
// ============================================================================

/// 实验报告生成器
pub struct ReportGenerator {
    /// 输出目录
    output_dir: PathBuf,
    /// 报告格式
    formats: Vec<ReportFormat>,
}

/// 报告格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    /// JSON（原始数据）
    Json,
    /// Markdown（人类可读）
    Markdown,
    /// LaTeX（学术论文表格）
    Latex,
    /// CSV（用于 Excel/R/Python 分析）
    Csv,
}

impl ReportGenerator {
    /// 创建新的报告生成器
    pub fn new(output_dir: &Path) -> Result<Self> {
        fs::create_dir_all(output_dir)?;
        
        Ok(Self {
            output_dir: output_dir.to_path_buf(),
            formats: vec![
                ReportFormat::Json,
                ReportFormat::Markdown,
                ReportFormat::Latex,
                ReportFormat::Csv,
            ],
        })
    }
    
    /// 设置报告格式
    pub fn with_formats(mut self, formats: Vec<ReportFormat>) -> Self {
        self.formats = formats;
        self
    }
    
    /// 生成完整实验报告
    pub fn generate_full_report(
        &self,
        config: &ExperimentConfig,
        comparative: Option<&ComparativeExperimentResult>,
        ablation: Option<&AblationExperimentResult>,
        statistical_tests: Vec<StatisticalTestResult>,
    ) -> Result<ExperimentReport> {
        let report = ExperimentReport {
            report_id: uuid::Uuid::new_v4().to_string(),
            config: config.clone(),
            generated_at: Utc::now(),
            comparative_results: comparative.cloned(),
            ablation_results: ablation.cloned(),
            statistical_tests: statistical_tests.clone(),
            executive_summary: self.generate_executive_summary(
                comparative,
                ablation,
                &statistical_tests,
            ),
        };
        
        // 保存报告
        self.save_report(&report)?;
        
        Ok(report)
    }
    
    /// 生成执行摘要
    fn generate_executive_summary(
        &self,
        comparative: Option<&ComparativeExperimentResult>,
        ablation: Option<&AblationExperimentResult>,
        statistical_tests: &[StatisticalTestResult],
    ) -> String {
        let mut summary = String::new();
        
        summary.push_str("# Experiment Executive Summary\n\n");
        summary.push_str(&format!("**Generated**: {}\n\n", Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
        
        // 对比实验摘要
        if let Some(comp) = comparative {
            summary.push_str("## Comparative Experiment Results\n\n");
            
            if let (Some(control), Some(ours)) = (
                comp.groups.get(&ExperimentGroupType::Control),
                comp.groups.get(&ExperimentGroupType::OursFull),
            ) {
                let m_control = &control.metrics;
                let m_ours = &ours.metrics;
                
                summary.push_str("### Primary Metrics Comparison\n\n");
                summary.push_str(&format!(
                    "- **Task Completion Rate**: Control={:.1}%, Ours-Full={:.1}% ({:+.1}%)\n",
                    m_control.task_completion_rate * 100.0,
                    m_ours.task_completion_rate * 100.0,
                    (m_ours.task_completion_rate - m_control.task_completion_rate) * 100.0
                ));
                summary.push_str(&format!(
                    "- **Avg Tool Calls**: Control={:.2}, Ours-Full={:.2} ({:+.1}%)\n",
                    m_control.avg_tool_calls,
                    m_ours.avg_tool_calls,
                    if m_control.avg_tool_calls > 0.0 {
                        ((m_ours.avg_tool_calls - m_control.avg_tool_calls) / m_control.avg_tool_calls) * 100.0
                    } else {
                        0.0
                    }
                ));
                summary.push_str(&format!(
                    "- **Tool Failure Rate**: Control={:.1}%, Ours-Full={:.1}% ({:+.1}%)\n",
                    m_control.tool_failure_rate * 100.0,
                    m_ours.tool_failure_rate * 100.0,
                    (m_ours.tool_failure_rate - m_control.tool_failure_rate) * 100.0
                ));
                summary.push_str(&format!(
                    "- **User Satisfaction**: Control={:.2}/5.0, Ours-Full={:.2}/5.0 ({:+.1}%)\n",
                    m_control.user_satisfaction,
                    m_ours.user_satisfaction,
                    if m_control.user_satisfaction > 0.0 {
                        ((m_ours.user_satisfaction - m_control.user_satisfaction) / m_control.user_satisfaction) * 100.0
                    } else {
                        0.0
                    }
                ));
            }
        }
        
        // 消融实验摘要
        if let Some(abl) = ablation {
            summary.push_str("\n## Ablation Study Results\n\n");
            summary.push_str("The following components were evaluated:\n\n");
            
            for (group_type, result) in &abl.groups {
                summary.push_str(&format!(
                    "- **{}**: Completion Rate={:.1}%, Gaps Detected={}\n",
                    group_type.name(),
                    result.metrics.task_completion_rate * 100.0,
                    result.metrics.gaps_detected
                ));
            }
        }
        
        // 统计检验摘要
        if !statistical_tests.is_empty() {
            summary.push_str("\n## Statistical Significance\n\n");
            
            for test in statistical_tests {
                summary.push_str(&format!("### {}\n", test.test_name));
                summary.push_str(&format!("**Groups**: {}\n", test.groups.join(" vs ")));
                
                if let Some(t_test) = &test.t_test {
                    summary.push_str(&format!(
                        "- t({:.1}) = {:.3}, p = {:.4}, Cohen's d = {:.3} ({})\n",
                        t_test.degrees_of_freedom,
                        t_test.t_statistic,
                        t_test.p_value_two_tailed,
                        t_test.cohens_d,
                        EffectSizeMagnitude::from_cohens_d(t_test.cohens_d).as_str()
                    ));
                    
                    if t_test.is_significant {
                        summary.push_str("**Result**: Statistically significant (p < 0.05)\n");
                    } else {
                        summary.push_str("**Result**: Not statistically significant (p >= 0.05)\n");
                    }
                }
                
                if let Some(anova) = &test.anova {
                    summary.push_str(&format!(
                        "- F({},{}) = {:.3}, p = {:.4}, η² = {:.3}\n",
                        anova.df_between,
                        anova.df_within,
                        anova.f_statistic,
                        anova.p_value,
                        anova.eta_squared
                    ));
                    
                    if anova.is_significant {
                        summary.push_str("**Result**: Statistically significant (p < 0.05)\n");
                    }
                }
                
                summary.push_str("\n");
            }
        }
        
        summary
    }
    
    /// 保存报告
    fn save_report(&self, report: &ExperimentReport) -> Result<()> {
        for format in &self.formats {
            match format {
                ReportFormat::Json => self.save_json_report(report)?,
                ReportFormat::Markdown => self.save_markdown_report(report)?,
                ReportFormat::Latex => self.save_latex_tables(report)?,
                ReportFormat::Csv => self.save_csv_data(report)?,
            }
        }
        
        Ok(())
    }
    
    /// 保存 JSON 报告
    fn save_json_report(&self, report: &ExperimentReport) -> Result<()> {
        let path = self.output_dir.join("experiment_report.json");
        let file = File::create(&path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, report)?;
        println!("Saved JSON report to {:?}", path);
        Ok(())
    }
    
    /// 保存 Markdown 报告
    fn save_markdown_report(&self, report: &ExperimentReport) -> Result<()> {
        let path = self.output_dir.join("experiment_report.md");
        let mut content = String::new();
        
        // 标题
        content.push_str(&format!("# Experiment Report: {}\n\n", report.config.name));
        content.push_str(&format!("**Generated**: {}\n\n", report.generated_at.format("%Y-%m-%d %H:%M:%S UTC")));
        content.push_str(&format!("**Git Commit**: `{}`\n\n", report.config.git_commit));
        
        // 执行摘要
        content.push_str(&report.executive_summary);
        
        // 对比实验结果
        if let Some(comparative) = &report.comparative_results {
            content.push_str("\n---\n\n");
            content.push_str("## Comparative Experiment Details\n\n");
            
            content.push_str("### Group Metrics\n\n");
            content.push_str("| Metric | Control | Ours-Full | Improvement |\n");
            content.push_str("|--------|---------|-----------|-------------|\n");
            
            if let (Some(control), Some(ours)) = (
                comparative.groups.get(&ExperimentGroupType::Control),
                comparative.groups.get(&ExperimentGroupType::OursFull),
            ) {
                let m_c = &control.metrics;
                let m_o = &ours.metrics;
                
                content.push_str(&format!(
                    "| Task Completion Rate | {:.2}% | {:.2}% | {:+.1}% |\n",
                    m_c.task_completion_rate * 100.0,
                    m_o.task_completion_rate * 100.0,
                    (m_o.task_completion_rate - m_c.task_completion_rate) * 100.0
                ));
                content.push_str(&format!(
                    "| Avg Tool Calls | {:.2} | {:.2} | {:+.1}% |\n",
                    m_c.avg_tool_calls,
                    m_o.avg_tool_calls,
                    if m_c.avg_tool_calls > 0.0 {
                        ((m_o.avg_tool_calls - m_c.avg_tool_calls) / m_c.avg_tool_calls) * 100.0
                    } else {
                        0.0
                    }
                ));
                content.push_str(&format!(
                    "| Tool Failure Rate | {:.2}% | {:.2}% | {:+.1}% |\n",
                    m_c.tool_failure_rate * 100.0,
                    m_o.tool_failure_rate * 100.0,
                    (m_o.tool_failure_rate - m_c.tool_failure_rate) * 100.0
                ));
                content.push_str(&format!(
                    "| User Satisfaction | {:.2} | {:.2} | {:+.1}% |\n",
                    m_c.user_satisfaction,
                    m_o.user_satisfaction,
                    if m_c.user_satisfaction > 0.0 {
                        ((m_o.user_satisfaction - m_c.user_satisfaction) / m_c.user_satisfaction) * 100.0
                    } else {
                        0.0
                    }
                ));
                content.push_str(&format!(
                    "| Gaps Detected | {} | {} | - |\n",
                    m_c.gaps_detected,
                    m_o.gaps_detected
                ));
                content.push_str(&format!(
                    "| API Cost (USD) | ${:.4} | ${:.4} | - |\n",
                    m_c.total_api_cost_usd,
                    m_o.total_api_cost_usd
                ));
            }
        }
        
        // 消融实验结果
        if let Some(ablation) = &report.ablation_results {
            content.push_str("\n---\n\n");
            content.push_str("## Ablation Study Details\n\n");
            
            content.push_str("### Component Contributions\n\n");
            content.push_str("| Configuration | Completion Rate | Gaps Detected | API Cost |\n");
            content.push_str("|---------------|-----------------|---------------|----------|\n");
            
            for (group_type, result) in &ablation.groups {
                content.push_str(&format!(
                    "| {} | {:.2}% | {} | ${:.4} |\n",
                    group_type.name(),
                    result.metrics.task_completion_rate * 100.0,
                    result.metrics.gaps_detected,
                    result.metrics.total_api_cost_usd
                ));
            }
        }
        
        // 统计检验结果
        if !report.statistical_tests.is_empty() {
            content.push_str("\n---\n\n");
            content.push_str("## Statistical Test Results\n\n");
            
            for test in &report.statistical_tests {
                content.push_str(&format!("### {}\n\n", test.test_name));
                content.push_str(&format!("**Groups**: {}\n\n", test.groups.join(" vs ")));
                
                if let Some(t_test) = &test.t_test {
                    content.push_str("#### T-Test Results\n\n");
                    content.push_str(&format!(
                        "- **t-statistic**: {:.4}\n",
                        t_test.t_statistic
                    ));
                    content.push_str(&format!(
                        "- **Degrees of Freedom**: {:.1}\n",
                        t_test.degrees_of_freedom
                    ));
                    content.push_str(&format!(
                        "- **p-value (two-tailed)**: {:.6}\n",
                        t_test.p_value_two_tailed
                    ));
                    content.push_str(&format!(
                        "- **Cohen's d**: {:.4} ({})\n",
                        t_test.cohens_d,
                        EffectSizeMagnitude::from_cohens_d(t_test.cohens_d).as_str()
                    ));
                    content.push_str(&format!(
                        "- **Significant**: {}\n",
                        if t_test.is_significant { "Yes (p < 0.05)" } else { "No" }
                    ));
                }
                
                content.push_str("\n");
            }
        }
        
        fs::write(&path, content)?;
        println!("Saved Markdown report to {:?}", path);
        Ok(())
    }
    
    /// 保存 LaTeX 表格
    fn save_latex_tables(&self, report: &ExperimentReport) -> Result<()> {
        let path = self.output_dir.join("experiment_tables.tex");
        let mut content = String::new();
        
        content.push_str("% Experiment Report LaTeX Tables\n");
        content.push_str("% Generated automatically by ReportGenerator\n\n");
        
        // 对比实验表格
        if let Some(comparative) = &report.comparative_results {
            content.push_str("\\begin{table}[t]\n");
            content.push_str("\\centering\n");
            content.push_str("\\caption{Comparative Experiment Results: Control vs Ours-Full}\n");
            content.push_str("\\label{tab:comparative}\n");
            content.push_str("\\begin{tabular}{lcc}\n");
            content.push_str("\\toprule\n");
            content.push_str("\\textbf{Metric} & \\textbf{Control} & \\textbf{Ours-Full} \\\\\n");
            content.push_str("\\midrule\n");
            
            if let (Some(control), Some(ours)) = (
                comparative.groups.get(&ExperimentGroupType::Control),
                comparative.groups.get(&ExperimentGroupType::OursFull),
            ) {
                let m_c = &control.metrics;
                let m_o = &ours.metrics;
                
                content.push_str(&format!(
                    "Task Completion Rate (\\%) & {:.2} & {:.2} \\\\\n",
                    m_c.task_completion_rate * 100.0,
                    m_o.task_completion_rate * 100.0
                ));
                content.push_str(&format!(
                    "Avg Tool Calls & {:.2} & {:.2} \\\\\n",
                    m_c.avg_tool_calls,
                    m_o.avg_tool_calls
                ));
                content.push_str(&format!(
                    "Tool Failure Rate (\\%) & {:.2} & {:.2} \\\\\n",
                    m_c.tool_failure_rate * 100.0,
                    m_o.tool_failure_rate * 100.0
                ));
                content.push_str(&format!(
                    "User Satisfaction (1-5) & {:.2} & {:.2} \\\\\n",
                    m_c.user_satisfaction,
                    m_o.user_satisfaction
                ));
                content.push_str(&format!(
                    "Gaps Detected & {} & {} \\\\\n",
                    m_c.gaps_detected,
                    m_o.gaps_detected
                ));
                content.push_str(&format!(
                    "API Cost (USD) & \\${:.4} & \\${:.4} \\\\\n",
                    m_c.total_api_cost_usd,
                    m_o.total_api_cost_usd
                ));
            }
            
            content.push_str("\\bottomrule\n");
            content.push_str("\\end{tabular}\n");
            content.push_str("\\end{table}\n\n");
        }
        
        // 消融实验表格
        if let Some(ablation) = &report.ablation_results {
            content.push_str("\\begin{table}[t]\n");
            content.push_str("\\centering\n");
            content.push_str("\\caption{Ablation Study Results: Component Contributions}\n");
            content.push_str("\\label{tab:ablation}\n");
            content.push_str("\\begin{tabular}{lccc}\n");
            content.push_str("\\toprule\n");
            content.push_str("\\textbf{Configuration} & \\textbf{Completion (\\%)} & \\textbf{Gaps} & \\textbf{Cost (USD)} \\\\\n");
            content.push_str("\\midrule\n");
            
            for (group_type, result) in &ablation.groups {
                content.push_str(&format!(
                    "{} & {:.2} & {} & \\${:.4} \\\\\n",
                    group_type.name(),
                    result.metrics.task_completion_rate * 100.0,
                    result.metrics.gaps_detected,
                    result.metrics.total_api_cost_usd
                ));
            }
            
            content.push_str("\\bottomrule\n");
            content.push_str("\\end{tabular}\n");
            content.push_str("\\end{table}\n\n");
        }
        
        // 统计检验表格
        if !report.statistical_tests.is_empty() {
            content.push_str("\\begin{table}[t]\n");
            content.push_str("\\centering\n");
            content.push_str("\\caption{Statistical Test Results}\n");
            content.push_str("\\label{tab:statistical}\n");
            content.push_str("\\begin{tabular}{lcccc}\n");
            content.push_str("\\toprule\n");
            content.push_str("\\textbf{Test} & \\textbf{Statistic} & \\textbf{df} & \\textbf{p-value} & \\textbf{Effect Size} \\\\\n");
            content.push_str("\\midrule\n");
            
            for test in &report.statistical_tests {
                if let Some(t_test) = &test.t_test {
                    content.push_str(&format!(
                        "{} ({} vs {}) & t={:.3} & {:.1} & {:.4} & d={:.3} ({}) \\\\\n",
                        test.test_name,
                        test.groups.get(0).unwrap_or(&"N/A".to_string()),
                        test.groups.get(1).unwrap_or(&"N/A".to_string()),
                        t_test.t_statistic,
                        t_test.degrees_of_freedom,
                        t_test.p_value_two_tailed,
                        t_test.cohens_d,
                        EffectSizeMagnitude::from_cohens_d(t_test.cohens_d).as_str()
                    ));
                }
            }
            
            content.push_str("\\bottomrule\n");
            content.push_str("\\end{tabular}\n");
            content.push_str("\\end{table}\n");
        }
        
        fs::write(&path, content)?;
        println!("Saved LaTeX tables to {:?}", path);
        Ok(())
    }
    
    /// 保存 CSV 数据
    fn save_csv_data(&self, report: &ExperimentReport) -> Result<()> {
        // 保存对比实验 CSV
        if let Some(comparative) = &report.comparative_results {
            let path = self.output_dir.join("comparative_results.csv");
            let mut content = String::from("group,task_completion_rate,avg_tool_calls,tool_failure_rate,user_satisfaction,gaps_detected,total_api_cost_usd\n");
            
            for (group_type, result) in &comparative.groups {
                let m = &result.metrics;
                content.push_str(&format!(
                    "{},{:.4},{:.4},{:.4},{:.4},{},{:.6}\n",
                    group_type.name(),
                    m.task_completion_rate,
                    m.avg_tool_calls,
                    m.tool_failure_rate,
                    m.user_satisfaction,
                    m.gaps_detected,
                    m.total_api_cost_usd
                ));
            }
            
            fs::write(&path, content)?;
            println!("Saved comparative CSV to {:?}", path);
        }
        
        // 保存消融实验 CSV
        if let Some(ablation) = &report.ablation_results {
            let path = self.output_dir.join("ablation_results.csv");
            let mut content = String::from("group,task_completion_rate,avg_tool_calls,tool_failure_rate,user_satisfaction,gaps_detected,total_api_cost_usd\n");
            
            for (group_type, result) in &ablation.groups {
                let m = &result.metrics;
                content.push_str(&format!(
                    "{},{:.4},{:.4},{:.4},{:.4},{},{:.6}\n",
                    group_type.name(),
                    m.task_completion_rate,
                    m.avg_tool_calls,
                    m.tool_failure_rate,
                    m.user_satisfaction,
                    m.gaps_detected,
                    m.total_api_cost_usd
                ));
            }
            
            fs::write(&path, content)?;
            println!("Saved ablation CSV to {:?}", path);
        }
        
        Ok(())
    }
}
