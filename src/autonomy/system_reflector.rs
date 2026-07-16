//! 系统反思器
//!
//! 定期生成系统体检报告，发现覆盖不足的领域
//!
//! ## 核心功能
//! - 工具覆盖率分析
//! - 领域覆盖度检测
//! - 系统健康度评估
//! - 生成改进建议报告

#![allow(dead_code)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// 领域覆盖度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainCoverage {
    /// 领域名称
    pub domain_name: String,
    /// 覆盖度 (0.0-1.0)
    pub coverage: f32,
    /// 已有工具数量
    pub existing_tools: u32,
    /// 建议工具数量
    pub recommended_tools: u32,
    /// 缺失的能力
    pub missing_capabilities: Vec<String>,
    /// 优先级 (1-10)
    pub priority: u8,
}

/// 系统健康度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    /// 整体健康度 (0.0-1.0)
    pub overall_health: f32,
    /// 工具多样性评分
    pub diversity_score: f32,
    /// 工具质量评分
    pub quality_score: f32,
    /// 领域覆盖评分
    pub coverage_score: f32,
    /// 进化能力评分
    pub evolution_score: f32,
    /// 问题列表
    pub issues: Vec<SystemIssue>,
}

/// 系统问题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemIssue {
    /// 问题类型
    pub issue_type: String,
    /// 问题描述
    pub description: String,
    /// 严重程度 (1-10)
    pub severity: u8,
    /// 建议解决方案
    pub suggested_fix: String,
}

/// 工具分布统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDistribution {
    /// 总工具数
    pub total_tools: u32,
    /// 领域分布
    pub domain_distribution: HashMap<String, u32>,
    /// 标签分布
    pub tag_distribution: HashMap<String, u32>,
    /// 使用率分布
    pub usage_distribution: UsageDistribution,
}

/// 使用率分布
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageDistribution {
    /// 高频使用工具数 (>100 次)
    pub high_usage: u32,
    /// 中频使用工具数 (10-100 次)
    pub medium_usage: u32,
    /// 低频使用工具数 (<10 次)
    pub low_usage: u32,
    /// 未使用工具数
    pub unused: u32,
}

/// 系统体检报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealthReport {
    /// 报告生成时间
    pub timestamp: u64,
    /// 系统健康度
    pub system_health: SystemHealth,
    /// 领域覆盖度
    pub domain_coverages: Vec<DomainCoverage>,
    /// 工具分布
    pub tool_distribution: ToolDistribution,
    /// 改进建议
    pub recommendations: Vec<String>,
    /// 下一步行动
    pub action_items: Vec<ActionItem>,
}

/// 行动项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionItem {
    /// 行动描述
    pub description: String,
    /// 优先级
    pub priority: u8,
    /// 预计工作量 (小时)
    pub estimated_hours: u8,
    /// 关联的领域
    pub related_domains: Vec<String>,
}

/// 预定义领域
#[derive(Debug, Clone)]
pub struct PredefinedDomain {
    pub name: String,
    pub required_capabilities: Vec<String>,
    pub recommended_tools: u32,
}

/// 系统反思器
pub struct SystemReflector {
    /// 数据存储目录
    data_dir: PathBuf,
    /// 预定义领域
    predefined_domains: Vec<PredefinedDomain>,
    /// 当前工具列表
    current_tools: Vec<ToolInfo>,
    /// 历史报告
    historical_reports: Vec<SystemHealthReport>,
    /// 配置
    config: ReflectorConfig,
}

/// 工具信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    /// 工具名称
    pub name: String,
    /// 工具描述
    pub description: String,
    /// 所属领域
    pub domain: String,
    /// 功能标签
    pub tags: Vec<String>,
    /// 使用次数
    pub usage_count: u32,
}

/// 反射器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectorConfig {
    /// 报告生成间隔（秒）
    pub report_interval_seconds: u64,
    /// 领域覆盖阈值
    pub domain_coverage_threshold: f32,
    /// 最小工具多样性评分
    pub min_diversity_score: f32,
}

impl Default for ReflectorConfig {
    fn default() -> Self {
        Self {
            report_interval_seconds: 3600, // 1 小时
            domain_coverage_threshold: 0.6,
            min_diversity_score: 0.5,
        }
    }
}

impl SystemReflector {
    /// 创建新的反射器
    pub fn new(data_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&data_dir)?;

        let mut reflector = Self {
            data_dir,
            predefined_domains: Vec::new(),
            current_tools: Vec::new(),
            historical_reports: Vec::new(),
            config: ReflectorConfig::default(),
        };

        // 初始化预定义领域
        reflector.initialize_predefined_domains();

        Ok(reflector)
    }

    /// 初始化预定义领域
    fn initialize_predefined_domains(&mut self) {
        self.predefined_domains = vec![
            PredefinedDomain {
                name: "文件操作".to_string(),
                required_capabilities: vec![
                    "读取文件".to_string(),
                    "写入文件".to_string(),
                    "删除文件".to_string(),
                    "复制/移动文件".to_string(),
                    "列出目录".to_string(),
                    "文件搜索".to_string(),
                ],
                recommended_tools: 8,
            },
            PredefinedDomain {
                name: "网络操作".to_string(),
                required_capabilities: vec![
                    "HTTP 请求".to_string(),
                    "下载文件".to_string(),
                    "网络搜索".to_string(),
                    "端口检测".to_string(),
                    "Ping 测试".to_string(),
                    "DNS 查询".to_string(),
                ],
                recommended_tools: 6,
            },
            PredefinedDomain {
                name: "系统操作".to_string(),
                required_capabilities: vec![
                    "执行命令".to_string(),
                    "进程管理".to_string(),
                    "环境变量".to_string(),
                    "系统信息".to_string(),
                    "路径解析".to_string(),
                ],
                recommended_tools: 5,
            },
            PredefinedDomain {
                name: "代码分析".to_string(),
                required_capabilities: vec![
                    "代码统计".to_string(),
                    "函数查找".to_string(),
                    "语言检测".to_string(),
                    "代码格式化".to_string(),
                    "依赖分析".to_string(),
                ],
                recommended_tools: 5,
            },
            PredefinedDomain {
                name: "数据处理".to_string(),
                required_capabilities: vec![
                    "JSON 处理".to_string(),
                    "CSV 处理".to_string(),
                    "XML 处理".to_string(),
                    "数据转换".to_string(),
                    "数据验证".to_string(),
                ],
                recommended_tools: 5,
            },
            PredefinedDomain {
                name: "版本控制".to_string(),
                required_capabilities: vec![
                    "Git 状态".to_string(),
                    "Git 提交".to_string(),
                    "Git 分支".to_string(),
                    "Git 日志".to_string(),
                    "Git 合并".to_string(),
                ],
                recommended_tools: 5,
            },
            PredefinedDomain {
                name: "知识管理".to_string(),
                required_capabilities: vec![
                    "知识索引".to_string(),
                    "语义搜索".to_string(),
                    "知识更新".to_string(),
                    "上下文管理".to_string(),
                ],
                recommended_tools: 4,
            },
        ];
    }

    /// 设置当前工具列表
    pub fn set_current_tools(&mut self, tools: Vec<ToolInfo>) {
        self.current_tools = tools;
    }

    /// 生成系统体检报告
    pub fn generate_health_report(&mut self) -> Result<SystemHealthReport> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // 1. 分析领域覆盖度
        let domain_coverages = self.analyze_domain_coverage();

        // 2. 计算工具分布
        let tool_distribution = self.calculate_tool_distribution();

        // 3. 评估系统健康度
        let system_health = self.evaluate_system_health(&domain_coverages, &tool_distribution);

        // 4. 生成改进建议
        let recommendations = self.generate_recommendations(&domain_coverages, &system_health);

        // 5. 生成行动项
        let action_items = self.generate_action_items(&domain_coverages);

        let report = SystemHealthReport {
            timestamp,
            system_health,
            domain_coverages: domain_coverages.clone(),
            tool_distribution,
            recommendations,
            action_items,
        };

        // 保存报告
        self.historical_reports.push(report.clone());
        self.save_report(&report)?;

        Ok(report)
    }

    /// 分析领域覆盖度
    fn analyze_domain_coverage(&self) -> Vec<DomainCoverage> {
        let mut coverages = Vec::new();

        for domain in &self.predefined_domains {
            // 统计该领域的工具
            let domain_tools: Vec<_> = self
                .current_tools
                .iter()
                .filter(|t| {
                    t.domain == domain.name || t.tags.iter().any(|tag| tag.contains(&domain.name))
                })
                .collect();

            let existing_count = domain_tools.len() as u32;

            // 分析已覆盖的能力
            let covered_capabilities: HashSet<_> = domain_tools
                .iter()
                .flat_map(|t| {
                    t.tags
                        .iter()
                        .chain(std::iter::once(&t.description))
                        .cloned()
                })
                .collect();

            // 找出缺失的能力
            let missing_capabilities: Vec<_> = domain
                .required_capabilities
                .iter()
                .filter(|cap| {
                    !covered_capabilities
                        .iter()
                        .any(|c| c.contains(cap.as_str()) || cap.as_str().contains(c.as_str()))
                })
                .cloned()
                .collect();

            // 计算覆盖度
            let total_caps = domain.required_capabilities.len() as f32;
            let covered_caps =
                (domain.required_capabilities.len() - missing_capabilities.len()) as f32;
            let coverage = if total_caps > 0.0 {
                covered_caps / total_caps
            } else {
                1.0
            };

            // 计算优先级
            let priority = if coverage < 0.3 {
                9
            } else if coverage < 0.5 {
                7
            } else if coverage < 0.7 {
                5
            } else {
                3
            };

            coverages.push(DomainCoverage {
                domain_name: domain.name.clone(),
                coverage,
                existing_tools: existing_count,
                recommended_tools: domain.recommended_tools,
                missing_capabilities,
                priority,
            });
        }

        // 按优先级排序
        coverages.sort_by(|a, b| b.priority.cmp(&a.priority));

        coverages
    }

    /// 计算工具分布
    fn calculate_tool_distribution(&self) -> ToolDistribution {
        let mut domain_distribution: HashMap<String, u32> = HashMap::new();
        let mut tag_distribution: HashMap<String, u32> = HashMap::new();
        let mut usage_dist = UsageDistribution {
            high_usage: 0,
            medium_usage: 0,
            low_usage: 0,
            unused: 0,
        };

        for tool in &self.current_tools {
            // 领域分布
            *domain_distribution.entry(tool.domain.clone()).or_insert(0) += 1;

            // 标签分布
            for tag in &tool.tags {
                *tag_distribution.entry(tag.clone()).or_insert(0) += 1;
            }

            // 使用率分布
            if tool.usage_count > 100 {
                usage_dist.high_usage += 1;
            } else if tool.usage_count >= 10 {
                usage_dist.medium_usage += 1;
            } else if tool.usage_count > 0 {
                usage_dist.low_usage += 1;
            } else {
                usage_dist.unused += 1;
            }
        }

        ToolDistribution {
            total_tools: self.current_tools.len() as u32,
            domain_distribution,
            tag_distribution,
            usage_distribution: usage_dist,
        }
    }

    /// 评估系统健康度
    fn evaluate_system_health(
        &self,
        domain_coverages: &[DomainCoverage],
        distribution: &ToolDistribution,
    ) -> SystemHealth {
        // 领域覆盖评分
        let coverage_score = if domain_coverages.is_empty() {
            0.0
        } else {
            domain_coverages.iter().map(|dc| dc.coverage).sum::<f32>()
                / domain_coverages.len() as f32
        };

        // 多样性评分（基于领域分布的均匀度）
        let diversity_score = self.calculate_diversity_score(&distribution.domain_distribution);

        // 质量评分（基于使用率分布）
        let quality_score = if distribution.total_tools > 0 {
            let active_ratio = (distribution.usage_distribution.high_usage
                + distribution.usage_distribution.medium_usage)
                as f32
                / distribution.total_tools as f32;
            active_ratio.min(1.0)
        } else {
            0.0
        };

        // 进化能力评分（基于领域覆盖的改进空间）
        let evolution_score = 1.0 - coverage_score; // 覆盖度越低，进化空间越大

        // 整体健康度
        let overall_health = (coverage_score * 0.4
            + diversity_score * 0.2
            + quality_score * 0.3
            + evolution_score * 0.1)
            .min(1.0);

        // 识别问题
        let mut issues = Vec::new();

        if coverage_score < 0.5 {
            issues.push(SystemIssue {
                issue_type: "low_coverage".to_string(),
                description: format!("领域覆盖度低（{:.1}%）", coverage_score * 100.0),
                severity: 8,
                suggested_fix: "优先补充缺失领域的工具".to_string(),
            });
        }

        if diversity_score < 0.4 {
            issues.push(SystemIssue {
                issue_type: "low_diversity".to_string(),
                description: format!("工具多样性不足（{:.1}%）", diversity_score * 100.0),
                severity: 6,
                suggested_fix: "扩展不同领域的工具".to_string(),
            });
        }

        if quality_score < 0.3 {
            issues.push(SystemIssue {
                issue_type: "low_quality".to_string(),
                description: format!("工具使用率低（{:.1}% 活跃）", quality_score * 100.0),
                severity: 7,
                suggested_fix: "优化现有工具或废弃低使用率工具".to_string(),
            });
        }

        SystemHealth {
            overall_health,
            diversity_score,
            quality_score,
            coverage_score,
            evolution_score,
            issues,
        }
    }

    /// 计算多样性评分
    fn calculate_diversity_score(&self, domain_distribution: &HashMap<String, u32>) -> f32 {
        if domain_distribution.is_empty() {
            return 0.0;
        }

        let total = domain_distribution.values().sum::<u32>() as f32;
        if total == 0.0 {
            return 0.0;
        }

        // 计算熵
        let entropy: f32 = domain_distribution
            .values()
            .map(|&count| {
                let p = count as f32 / total;
                if p > 0.0 {
                    -p * p.ln()
                } else {
                    0.0
                }
            })
            .sum();

        // 归一化到 0-1
        let max_entropy = (domain_distribution.len() as f32).ln();
        if max_entropy > 0.0 {
            (entropy / max_entropy).min(1.0)
        } else {
            0.0
        }
    }

    /// 生成改进建议
    fn generate_recommendations(
        &self,
        domain_coverages: &[DomainCoverage],
        system_health: &SystemHealth,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        // 基于领域覆盖的建议
        for dc in domain_coverages {
            if dc.coverage < self.config.domain_coverage_threshold {
                for missing in &dc.missing_capabilities {
                    recommendations.push(format!(
                        "在{}领域添加工具，提供{}能力（当前覆盖{:.1}%）",
                        dc.domain_name,
                        missing,
                        dc.coverage * 100.0
                    ));
                }
            }
        }

        // 基于系统健康度的建议
        for issue in &system_health.issues {
            recommendations.push(format!("{}: {}", issue.issue_type, issue.description));
        }

        recommendations
    }

    /// 生成行动项
    fn generate_action_items(&self, domain_coverages: &[DomainCoverage]) -> Vec<ActionItem> {
        let mut action_items = Vec::new();

        for dc in domain_coverages.iter().take(3) {
            // 只处理优先级最高的 3 个领域
            if dc.priority >= 7 && dc.coverage < 0.5 {
                action_items.push(ActionItem {
                    description: format!(
                        "为{}领域开发{}个新工具",
                        dc.domain_name,
                        dc.recommended_tools - dc.existing_tools
                    ),
                    priority: dc.priority,
                    estimated_hours: ((dc.recommended_tools - dc.existing_tools) * 2) as u8,
                    related_domains: vec![dc.domain_name.clone()],
                });
            }
        }

        action_items
    }

    /// 保存报告
    fn save_report(&self, report: &SystemHealthReport) -> Result<()> {
        let file_path = self
            .data_dir
            .join(format!("health_report_{}.json", report.timestamp));
        let json = serde_json::to_string_pretty(report)?;
        std::fs::write(&file_path, &json)?;

        // 保存最新报告的快捷方式
        let latest_path = self.data_dir.join("latest_health_report.json");
        std::fs::write(&latest_path, &json)?;

        Ok(())
    }

    /// 获取历史报告
    pub fn get_historical_reports(&self) -> &[SystemHealthReport] {
        &self.historical_reports
    }

    /// 获取预定义领域
    pub fn get_predefined_domains(&self) -> &[PredefinedDomain] {
        &self.predefined_domains
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_reflector_creation() {
        let temp_dir = TempDir::new().unwrap();
        let reflector = SystemReflector::new(temp_dir.path().to_path_buf()).unwrap();
        assert!(!reflector.predefined_domains.is_empty());
    }

    #[test]
    fn test_health_report_generation() {
        let temp_dir = TempDir::new().unwrap();
        let mut reflector = SystemReflector::new(temp_dir.path().to_path_buf()).unwrap();

        // 设置一些示例工具
        reflector.set_current_tools(vec![
            ToolInfo {
                name: "read_file".to_string(),
                description: "读取文件内容".to_string(),
                domain: "文件操作".to_string(),
                tags: vec!["file".to_string(), "read".to_string()],
                usage_count: 100,
            },
            ToolInfo {
                name: "http_get".to_string(),
                description: "发送 HTTP GET 请求".to_string(),
                domain: "网络操作".to_string(),
                tags: vec!["http".to_string(), "network".to_string()],
                usage_count: 50,
            },
        ]);

        let report = reflector.generate_health_report().unwrap();

        assert!(report.system_health.overall_health > 0.0);
        assert!(!report.domain_coverages.is_empty());
        assert!(!report.recommendations.is_empty());
    }
}
