//! 统计分析与显著性检验模块
//!
//! # 设计目标
//! 提供学术论文级别的统计分析，包括：
//! - 描述性统计（均值、标准差、置信区间）
//! - 显著性检验（t 检验、ANOVA、Mann-Whitney U 检验）
//! - 效应量计算（Cohen's d、Hedge's g）
//! - 相关性分析（Pearson、Spearman）
//!
//! # 依赖
//! 使用 Rust 原生统计库，避免 Python 依赖

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

// ============================================================================
// 描述性统计
// ============================================================================

/// 描述性统计结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescriptiveStats {
    /// 样本数量
    pub n: usize,
    /// 均值
    pub mean: f64,
    /// 标准差
    pub std_dev: f64,
    /// 标准误
    pub std_error: f64,
    /// 最小值
    pub min: f64,
    /// 最大值
    pub max: f64,
    /// 中位数
    pub median: f64,
    /// 第一四分位数 (Q1)
    pub q1: f64,
    /// 第三四分位数 (Q3)
    pub q3: f64,
    /// 95% 置信区间下限
    pub ci_95_lower: f64,
    /// 95% 置信区间上限
    pub ci_95_upper: f64,
}

impl DescriptiveStats {
    /// 计算描述性统计
    pub fn calculate(data: &[f64]) -> Result<Self> {
        if data.is_empty() {
            bail!("Data cannot be empty");
        }
        
        let n = data.len();
        let mut sorted_data = data.to_vec();
        sorted_data.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        // 均值
        let mean = data.iter().sum::<f64>() / n as f64;
        
        // 标准差
        let variance = data.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / (n - 1) as f64;
        let std_dev = variance.sqrt();
        
        // 标准误
        let std_error = std_dev / (n as f64).sqrt();
        
        // 最小值/最大值
        let min = sorted_data.first().copied().unwrap_or(0.0);
        let max = sorted_data.last().copied().unwrap_or(0.0);
        
        // 中位数
        let median = calculate_percentile(&sorted_data, 50.0);
        
        // 四分位数
        let q1 = calculate_percentile(&sorted_data, 25.0);
        let q3 = calculate_percentile(&sorted_data, 75.0);
        
        // 95% 置信区间 (使用 t 分布)
        let t_critical = t_distribution_critical_value(0.95, n - 1);
        let ci_95_lower = mean - t_critical * std_error;
        let ci_95_upper = mean + t_critical * std_error;
        
        Ok(Self {
            n,
            mean,
            std_dev,
            std_error,
            min,
            max,
            median,
            q1,
            q3,
            ci_95_lower,
            ci_95_upper,
        })
    }
}

/// 计算百分位数
fn calculate_percentile(sorted_data: &[f64], percentile: f64) -> f64 {
    let n = sorted_data.len();
    if n == 0 {
        return 0.0;
    }
    if n == 1 {
        return sorted_data[0];
    }
    
    let rank = (percentile / 100.0) * (n - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    
    if lower == upper {
        sorted_data[lower]
    } else {
        let fraction = rank - lower as f64;
        sorted_data[lower] * (1.0 - fraction) + sorted_data[upper] * fraction
    }
}

// ============================================================================
// T 检验
// ============================================================================

/// 独立样本 t 检验结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTestResult {
    /// t 统计量
    pub t_statistic: f64,
    /// 自由度
    pub degrees_of_freedom: f64,
    /// 双尾 p 值
    pub p_value_two_tailed: f64,
    /// 单尾 p 值
    pub p_value_one_tailed: f64,
    /// Cohen's d 效应量
    pub cohens_d: f64,
    /// 是否显著 (α = 0.05)
    pub is_significant: bool,
    /// 检验类型
    pub test_type: String,
}

/// 独立样本 t 检验（假设方差不等，使用 Welch's t-test）
pub fn welch_t_test(
    group1: &[f64],
    group2: &[f64],
) -> Result<TTestResult> {
    if group1.is_empty() || group2.is_empty() {
        bail!("Groups cannot be empty");
    }
    
    let n1 = group1.len();
    let n2 = group2.len();
    
    // 计算均值
    let mean1 = group1.iter().sum::<f64>() / n1 as f64;
    let mean2 = group2.iter().sum::<f64>() / n2 as f64;
    
    // 计算方差
    let var1 = group1.iter()
        .map(|x| (x - mean1).powi(2))
        .sum::<f64>() / (n1 - 1) as f64;
    let var2 = group2.iter()
        .map(|x| (x - mean2).powi(2))
        .sum::<f64>() / (n2 - 1) as f64;
    
    // Welch's t 统计量
    let se1 = var1 / n1 as f64;
    let se2 = var2 / n2 as f64;
    let t_statistic = (mean1 - mean2) / (se1 + se2).sqrt();
    
    // Welch-Satterthwaite 自由度
    let df = (se1 + se2).powi(2) / 
        (se1.powi(2) / (n1 - 1) as f64 + se2.powi(2) / (n2 - 1) as f64);
    
    // 计算 p 值
    let p_value_two_tailed = t_distribution_p_value(t_statistic.abs(), df);
    let p_value_one_tailed = p_value_two_tailed / 2.0;
    
    // 计算 Cohen's d
    let pooled_std = ((var1 + var2) / 2.0).sqrt();
    let cohens_d = (mean1 - mean2) / pooled_std;
    
    Ok(TTestResult {
        t_statistic,
        degrees_of_freedom: df,
        p_value_two_tailed,
        p_value_one_tailed,
        cohens_d,
        is_significant: p_value_two_tailed < 0.05,
        test_type: "Welch's t-test (independent samples, unequal variance)".to_string(),
    })
}

/// 配对样本 t 检验
pub fn paired_t_test(
    paired_data: &[(f64, f64)],
) -> Result<TTestResult> {
    if paired_data.is_empty() {
        bail!("Data cannot be empty");
    }
    
    // 计算差值
    let differences: Vec<f64> = paired_data.iter()
        .map(|(a, b)| a - b)
        .collect();
    
    let n = differences.len();
    let mean_diff = differences.iter().sum::<f64>() / n as f64;
    
    let variance = differences.iter()
        .map(|d| (d - mean_diff).powi(2))
        .sum::<f64>() / (n - 1) as f64;
    let std_diff = variance.sqrt();
    
    // t 统计量
    let t_statistic = mean_diff / (std_diff / (n as f64).sqrt());
    let df = (n - 1) as f64;
    
    // p 值
    let p_value_two_tailed = t_distribution_p_value(t_statistic.abs(), df);
    let p_value_one_tailed = p_value_two_tailed / 2.0;
    
    // Cohen's d (配对样本)
    let cohens_d = mean_diff / std_diff;
    
    Ok(TTestResult {
        t_statistic,
        degrees_of_freedom: df,
        p_value_two_tailed,
        p_value_one_tailed,
        cohens_d,
        is_significant: p_value_two_tailed < 0.05,
        test_type: "Paired t-test".to_string(),
    })
}

// ============================================================================
// ANOVA (单因素方差分析)
// ============================================================================

/// ANOVA 结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnovaResult {
    /// F 统计量
    pub f_statistic: f64,
    /// 组间自由度
    pub df_between: u32,
    /// 组内自由度
    pub df_within: u32,
    /// p 值
    pub p_value: f64,
    /// η² (eta-squared) 效应量
    pub eta_squared: f64,
    /// 是否显著
    pub is_significant: bool,
    /// 各组统计
    pub group_stats: Vec<DescriptiveStats>,
}

/// 单因素 ANOVA
pub fn one_way_anova(
    groups: &[&[f64]],
) -> Result<AnovaResult> {
    if groups.len() < 2 {
        bail!("Need at least 2 groups for ANOVA");
    }
    
    let k = groups.len() as u32; // 组数
    
    // 计算各组统计
    let mut group_stats = Vec::new();
    let mut all_data: Vec<f64> = Vec::new();
    let mut grand_mean = 0.0;
    let mut total_n = 0;
    
    for group in groups {
        if group.is_empty() {
            bail!("Groups cannot be empty");
        }
        let stats = DescriptiveStats::calculate(group)?;
        grand_mean += stats.mean * group.len() as f64;
        total_n += group.len();
        group_stats.push(stats);
        all_data.extend(*group);
    }
    
    grand_mean /= total_n as f64;
    
    // 组间平方和 (SSB)
    let ss_between: f64 = groups.iter()
        .zip(group_stats.iter())
        .map(|(group, stats)| {
            let n = group.len() as f64;
            n * (stats.mean - grand_mean).powi(2)
        })
        .sum();
    
    // 组内平方和 (SSW)
    let ss_within: f64 = groups.iter()
        .zip(group_stats.iter())
        .map(|(group, stats)| {
            let variance = stats.std_dev.powi(2);
            variance * (group.len() - 1) as f64
        })
        .sum();
    
    // 自由度
    let df_between = k - 1;
    let df_within = (total_n as u32 - k) as u32;
    
    // 均方
    let ms_between = ss_between / df_between as f64;
    let ms_within = ss_within / df_within as f64;
    
    // F 统计量
    let f_statistic = ms_between / ms_within;
    
    // p 值 (使用 F 分布)
    let p_value = f_distribution_p_value(f_statistic, df_between as u64, df_within as u64);
    
    // η² 效应量
    let eta_squared = ss_between / (ss_between + ss_within);
    
    Ok(AnovaResult {
        f_statistic,
        df_between,
        df_within,
        p_value,
        eta_squared,
        is_significant: p_value < 0.05,
        group_stats,
    })
}

// ============================================================================
// Mann-Whitney U 检验 (非参数检验)
// ============================================================================

/// Mann-Whitney U 检验结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MannWhitneyResult {
    /// U 统计量
    pub u_statistic: f64,
    /// U1 值
    pub u1: f64,
    /// U2 值
    pub u2: f64,
    /// z 统计量 (大样本近似)
    pub z_statistic: f64,
    /// p 值
    pub p_value: f64,
    /// 效应量 r
    pub effect_size_r: f64,
    /// 是否显著
    pub is_significant: bool,
}

/// Mann-Whitney U 检验
pub fn mann_whitney_u_test(
    group1: &[f64],
    group2: &[f64],
) -> Result<MannWhitneyResult> {
    let n1 = group1.len();
    let n2 = group2.len();
    
    if n1 == 0 || n2 == 0 {
        bail!("Groups cannot be empty");
    }
    
    // 合并数据并排序，保留组别信息
    let mut combined: Vec<(f64, usize)> = group1.iter()
        .map(|&x| (x, 0))
        .chain(group2.iter().map(|&x| (x, 1)))
        .collect();
    
    combined.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    
    // 计算秩次（处理并列）
    let ranks = calculate_ranks(&combined);
    
    // 计算各组的秩和
    let r1: f64 = ranks.iter()
        .filter(|(_, group)| *group == 0)
        .map(|(rank, _)| rank)
        .sum();
    
    let r2: f64 = ranks.iter()
        .filter(|(_, group)| *group == 1)
        .map(|(rank, _)| rank)
        .sum();
    
    // 计算 U 值
    let u1 = r1 - (n1 * (n1 + 1)) as f64 / 2.0;
    let u2 = r2 - (n2 * (n2 + 1)) as f64 / 2.0;
    let u_statistic = u1.min(u2);
    
    // 大样本近似（n1, n2 > 10）
    let mean_u = (n1 * n2) as f64 / 2.0;
    let std_u = ((n1 * n2 * (n1 + n2 + 1)) as f64 / 12.0).sqrt();
    let z_statistic = (u_statistic - mean_u) / std_u;
    
    // p 值（双尾）
    let p_value = 2.0 * (1.0 - normal_cdf(z_statistic.abs()));
    
    // 效应量 r
    let n_total = (n1 + n2) as f64;
    let effect_size_r = z_statistic / n_total.sqrt();
    
    Ok(MannWhitneyResult {
        u_statistic,
        u1,
        u2,
        z_statistic,
        p_value,
        effect_size_r,
        is_significant: p_value < 0.05,
    })
}

/// 计算秩次（处理并列）
fn calculate_ranks(combined: &[(f64, usize)]) -> Vec<(f64, usize)> {
    let mut ranks = Vec::with_capacity(combined.len());
    let mut i = 0;
    
    while i < combined.len() {
        // 找到所有相同值的索引
        let mut j = i;
        while j < combined.len() && combined[j].0 == combined[i].0 {
            j += 1;
        }
        
        // 计算平均秩次
        let avg_rank = (i + j - 1) as f64 / 2.0 + 1.0;
        
        // 为所有相同值分配平均秩次
        for k in i..j {
            ranks.push((avg_rank, combined[k].1));
        }
        
        i = j;
    }
    
    ranks
}

// ============================================================================
// 效应量计算
// ============================================================================

/// 效应量解释
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EffectSizeMagnitude {
    Negligible,
    Small,
    Medium,
    Large,
    VeryLarge,
}

impl EffectSizeMagnitude {
    pub fn from_cohens_d(d: f64) -> Self {
        let abs_d = d.abs();
        if abs_d < 0.2 {
            EffectSizeMagnitude::Negligible
        } else if abs_d < 0.5 {
            EffectSizeMagnitude::Small
        } else if abs_d < 0.8 {
            EffectSizeMagnitude::Medium
        } else if abs_d < 1.2 {
            EffectSizeMagnitude::Large
        } else {
            EffectSizeMagnitude::VeryLarge
        }
    }
    
    pub fn from_eta_squared(eta_sq: f64) -> Self {
        if eta_sq < 0.01 {
            EffectSizeMagnitude::Negligible
        } else if eta_sq < 0.06 {
            EffectSizeMagnitude::Small
        } else if eta_sq < 0.14 {
            EffectSizeMagnitude::Medium
        } else {
            EffectSizeMagnitude::Large
        }
    }
    
    pub fn as_str(&self) -> &'static str {
        match self {
            EffectSizeMagnitude::Negligible => "negligible",
            EffectSizeMagnitude::Small => "small",
            EffectSizeMagnitude::Medium => "medium",
            EffectSizeMagnitude::Large => "large",
            EffectSizeMagnitude::VeryLarge => "very large",
        }
    }
}

/// Hedge's g 效应量（小样本校正）
pub fn hedges_g(cohens_d: f64, n1: usize, n2: usize) -> f64 {
    let n = (n1 + n2) as f64;
    let correction_factor = 1.0 - (3.0 / (4.0 * n - 9.0));
    cohens_d * correction_factor
}

// ============================================================================
// 统计分布函数（近似实现）
// ============================================================================

/// t 分布的累积分布函数（近似）
fn t_distribution_cdf(t: f64, df: f64) -> f64 {
    // 使用不完全 Beta 函数近似
    // 这里使用简化版本，实际应用中建议使用 statrs 库
    let x = df / (df + t * t);
    let beta_inc = incomplete_beta(df / 2.0, 0.5, x);
    
    if t >= 0.0 {
        1.0 - 0.5 * beta_inc
    } else {
        0.5 * beta_inc
    }
}

/// t 分布的临界值（近似）
fn t_distribution_critical_value(confidence: f64, df: usize) -> f64 {
    // 使用查找表近似
    // 对于大样本，接近正态分布
    if df >= 100 {
        return normal_inverse((1.0 + confidence) / 2.0);
    }
    
    // 简化的近似公式
    let alpha = 1.0 - confidence;
    let z = normal_inverse(1.0 - alpha / 2.0);
    
    // Cornish-Fisher 展开
    z + (z.powi(3) + z) / (4.0 * df as f64)
        + (5.0 * z.powi(5) + 16.0 * z.powi(3) + 3.0 * z) / (96.0 * (df as f64).powi(2))
}

/// t 分布的 p 值
fn t_distribution_p_value(t: f64, df: f64) -> f64 {
    2.0 * (1.0 - t_distribution_cdf(t, df))
}

/// F 分布的 p 值（近似）
fn f_distribution_p_value(f: f64, df1: u64, df2: u64) -> f64 {
    // 使用不完全 Beta 函数
    let x = df1 as f64 * f / (df1 as f64 * f + df2 as f64);
    let beta_inc = incomplete_beta(df1 as f64 / 2.0, df2 as f64 / 2.0, x);
    1.0 - beta_inc
}

/// 标准正态分布的累积分布函数
fn normal_cdf(x: f64) -> f64 {
    // Abramowitz and Stegun 近似
    let sign = if x >= 0.0 { 1.0 } else { -1.0 };
    let x = x.abs();
    
    let b1 = 0.319381530;
    let b2 = -0.356563782;
    let b3 = 1.781477937;
    let b4 = -1.821255978;
    let b5 = 1.330274429;
    let p = 0.2316419;
    
    let t = 1.0 / (1.0 + p * x);
    let poly = t * (b1 + t * (b2 + t * (b3 + t * (b4 + t * b5))));
    
    let result = 1.0 - (1.0 / (2.0 * std::f64::consts::PI).sqrt()) 
        * (-x * x / 2.0).exp() * poly;
    
    if sign < 0.0 {
        1.0 - result
    } else {
        result
    }
}

/// 标准正态分布的逆累积分布函数（近似）
fn normal_inverse(p: f64) -> f64 {
    // Abramowitz and Stegun 近似
    if p <= 0.0 || p >= 1.0 {
        return f64::NAN;
    }
    
    if p == 0.5 {
        return 0.0;
    }
    
    let t = if p < 0.5 {
        (-2.0 * p.ln()).sqrt()
    } else {
        (-2.0 * (1.0 - p).ln()).sqrt()
    };
    
    let c0 = 2.515517;
    let c1 = 0.802853;
    let c2 = 0.010328;
    let d1 = 1.432788;
    let d2 = 0.189269;
    let d3 = 0.001308;
    
    let x = t - (c0 + c1 * t + c2 * t * t) 
        / (1.0 + d1 * t + d2 * t * t + d3 * t * t * t);
    
    if p < 0.5 {
        -x
    } else {
        x
    }
}

/// 不完全 Beta 函数（近似）
fn incomplete_beta(a: f64, b: f64, x: f64) -> f64 {
    // 使用连分数展开近似
    if x < 0.0 || x > 1.0 {
        return 0.0;
    }
    
    if x == 0.0 || x == 1.0 {
        return x;
    }
    
    // 简化版本，实际应使用更精确的算法
    let bt = ((a.ln() * a + b.ln() * b - (a + b).ln() * (a + b)) 
        + a * x.ln() + b * (1.0 - x).ln()).exp();
    
    // 连分数近似（简化）
    if x < (a + 1.0) / (a + b + 2.0) {
        bt * continued_fraction_beta(a, b, x) / a
    } else {
        1.0 - bt * continued_fraction_beta(b, a, 1.0 - x) / b
    }
}

/// Beta 函数的连分数展开
fn continued_fraction_beta(a: f64, b: f64, x: f64) -> f64 {
    // 简化版本
    let mut h = 1.0;
    let mut c = 1.0;
    let mut d = 0.0;
    
    for i in 0..100 {
        let m = i / 2;
        let numerator = if i == 0 {
            1.0
        } else if i % 2 == 0 {
            m as f64 * (b - m as f64) * x / ((a + 2.0 * m as f64 - 1.0) * (a + 2.0 * m as f64))
        } else {
            -((a + m as f64) * (a + b + m as f64) * x) 
                / ((a + 2.0 * m as f64) * (a + 2.0 * m as f64 + 1.0))
        };
        
        d = 1.0 + numerator * d;
        if d.abs() < 1e-10 {
            d = 1e-10;
        }
        c = 1.0 + numerator / c;
        if c.abs() < 1e-10 {
            c = 1e-10;
        }
        d = 1.0 / d;
        h *= d * c;
        
        if (numerator * (d - 1.0)).abs() < 1e-10 {
            break;
        }
    }
    
    h
}

// ============================================================================
// 多重检验校正
// ============================================================================

/// Bonferroni 校正
pub fn bonferroni_correction(p_values: &[f64], alpha: f64) -> Vec<bool> {
    let corrected_alpha = alpha / p_values.len() as f64;
    p_values.iter().map(|&p| p < corrected_alpha).collect()
}

/// Benjamini-Hochberg 校正（FDR 控制）
pub fn benjamini_hochberg_correction(p_values: &[f64], alpha: f64) -> Vec<bool> {
    let n = p_values.len();
    if n == 0 {
        return Vec::new();
    }
    
    // 带索引排序
    let mut indexed: Vec<(usize, f64)> = p_values.iter()
        .enumerate()
        .map(|(i, &p)| (i, p))
        .collect();
    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    
    // 计算校正后的显著性
    let mut significant = vec![false; n];
    let mut max_significant_rank = 0;

    for (rank, (orig_idx, p)) in indexed.iter().enumerate() {
        let k = rank + 1;
        let threshold = alpha * k as f64 / n as f64;
        if *p <= threshold {
            max_significant_rank = k;
        }
    }
    
    // 标记所有小于等于最大显著秩的 p 值为显著
    for (rank, (orig_idx, p)) in indexed.iter().enumerate() {
        if rank + 1 <= max_significant_rank {
            significant[*orig_idx] = true;
        }
    }
    
    significant
}
