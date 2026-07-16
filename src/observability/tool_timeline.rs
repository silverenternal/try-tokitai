//! 工具调用链可视化
//!
//! TUI 中添加工具调用时间线面板，显示 AI 的工具选择决策过程
//!
//! ## UI 组件
//! - Timeline View: 按时间顺序显示工具调用
//! - Dependency Graph: DAG 展示工具调用关系
//! - Decision Explanation: AI 生成选择原因

#![allow(dead_code)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// 工具调用事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallEvent {
    /// 事件 ID
    pub id: String,
    /// 工具名称
    pub tool_name: String,
    /// 工具参数（JSON）
    pub parameters: Option<String>,
    /// 调用开始时间
    pub start_time: u64,
    /// 调用结束时间
    pub end_time: Option<u64>,
    /// 执行状态
    pub status: ToolCallStatus,
    /// 执行结果
    pub result: Option<String>,
    /// 错误信息
    pub error: Option<String>,
    /// 决策原因
    pub decision_reason: Option<String>,
    /// 依赖的前置工具调用 ID
    pub dependencies: Vec<String>,
    /// 关联的请求 ID
    pub request_id: Option<String>,
}

/// 工具调用状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolCallStatus {
    /// 等待中
    Pending,
    /// 执行中
    Running,
    /// 成功
    Success,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
}

/// 工具调用时间线
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolTimeline {
    /// 时间线 ID
    pub id: String,
    /// 关联的请求 ID
    pub request_id: String,
    /// 工具调用事件
    pub events: Vec<ToolCallEvent>,
    /// 创建时间
    pub created_at: u64,
    /// 更新时间
    pub updated_at: u64,
}

/// 工具依赖图节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyNode {
    /// 节点 ID（工具调用 ID）
    pub node_id: String,
    /// 工具名称
    pub tool_name: String,
    /// 执行时间（ms）
    pub execution_time_ms: Option<u64>,
    /// 状态
    pub status: ToolCallStatus,
    /// 依赖的节点 ID
    pub dependency_ids: Vec<String>,
}

/// 工具依赖图
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    /// 节点列表
    pub nodes: Vec<DependencyNode>,
    /// 边列表（源->目标）
    pub edges: Vec<(String, String)>,
}

/// 工具调用链可视化器
pub struct ToolCallVisualizer {
    /// 数据目录
    data_dir: PathBuf,
    /// 时间线列表
    timelines: Vec<ToolTimeline>,
    /// 配置
    config: VisualizerConfig,
}

/// 可视化器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizerConfig {
    /// 最大时间线数量
    pub max_timelines: usize,
    /// 是否自动保存
    pub auto_save_enabled: bool,
    /// 是否记录决策原因
    pub record_decision_reasons: bool,
}

impl Default for VisualizerConfig {
    fn default() -> Self {
        Self {
            max_timelines: 100,
            auto_save_enabled: true,
            record_decision_reasons: true,
        }
    }
}

impl ToolCallVisualizer {
    /// 创建新的可视化器
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&data_dir)?;

        let mut visualizer = Self {
            data_dir,
            timelines: Vec::new(),
            config: VisualizerConfig::default(),
        };

        visualizer.load_timelines().ok();

        Ok(visualizer)
    }

    /// 创建新的时间线
    pub fn create_timeline(&mut self, request_id: &str) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let id = format!("timeline_{}", now);

        let timeline = ToolTimeline {
            id: id.clone(),
            request_id: request_id.to_string(),
            events: Vec::new(),
            created_at: now,
            updated_at: now,
        };

        self.timelines.push(timeline);

        // 清理旧时间线
        while self.timelines.len() > self.config.max_timelines {
            self.timelines.remove(0);
        }

        id
    }

    /// 添加工具调用事件
    pub fn add_tool_call(
        &mut self,
        timeline_id: &str,
        tool_name: &str,
        parameters: Option<serde_json::Value>,
        decision_reason: Option<&str>,
        dependencies: Vec<String>,
    ) -> Result<String> {
        let timeline = self
            .timelines
            .iter_mut()
            .find(|t| t.id == timeline_id)
            .ok_or_else(|| anyhow::anyhow!("Timeline not found: {}", timeline_id))?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let id = format!("call_{}_{}", tool_name, now);

        let event = ToolCallEvent {
            id: id.clone(),
            tool_name: tool_name.to_string(),
            parameters: parameters.map(|v| serde_json::to_string(&v).unwrap()),
            start_time: now,
            end_time: None,
            status: ToolCallStatus::Pending,
            result: None,
            error: None,
            decision_reason: decision_reason.map(String::from),
            dependencies,
            request_id: Some(timeline.request_id.clone()),
        };

        timeline.events.push(event);
        timeline.updated_at = now;

        Ok(id)
    }

    /// 更新工具调用状态
    pub fn update_tool_call(
        &mut self,
        timeline_id: &str,
        event_id: &str,
        status: ToolCallStatus,
        result: Option<String>,
        error: Option<String>,
    ) -> Result<()> {
        let timeline = self
            .timelines
            .iter_mut()
            .find(|t| t.id == timeline_id)
            .ok_or_else(|| anyhow::anyhow!("Timeline not found: {}", timeline_id))?;

        let event = timeline
            .events
            .iter_mut()
            .find(|e| e.id == event_id)
            .ok_or_else(|| anyhow::anyhow!("Event not found: {}", event_id))?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        event.status = status;
        event.result = result;
        event.error = error;
        event.end_time = Some(now);
        timeline.updated_at = now;

        Ok(())
    }

    /// 获取时间线
    pub fn get_timeline(&self, timeline_id: &str) -> Option<&ToolTimeline> {
        self.timelines.iter().find(|t| t.id == timeline_id)
    }

    /// 获取依赖图
    pub fn get_dependency_graph(&self, timeline_id: &str) -> Option<DependencyGraph> {
        let timeline = self.get_timeline(timeline_id)?;

        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        for event in &timeline.events {
            let execution_time = event.end_time.map(|end| (end - event.start_time) * 1000);

            nodes.push(DependencyNode {
                node_id: event.id.clone(),
                tool_name: event.tool_name.clone(),
                execution_time_ms: execution_time,
                status: event.status.clone(),
                dependency_ids: event.dependencies.clone(),
            });

            for dep in &event.dependencies {
                edges.push((dep.clone(), event.id.clone()));
            }
        }

        Some(DependencyGraph { nodes, edges })
    }

    /// 获取所有时间线索引
    pub fn list_timelines(&self) -> Vec<&ToolTimeline> {
        self.timelines.iter().collect()
    }

    /// 保存时间线
    pub fn save_timelines(&self) -> Result<()> {
        let file_path = self.data_dir.join("tool_timelines.json");
        let json = serde_json::to_string_pretty(&self.timelines)?;
        std::fs::write(file_path, json)?;
        Ok(())
    }

    /// 加载时间线
    fn load_timelines(&mut self) -> Result<()> {
        let file_path = self.data_dir.join("tool_timelines.json");
        if file_path.exists() {
            let json = std::fs::read_to_string(file_path)?;
            self.timelines = serde_json::from_str(&json)?;
        }
        Ok(())
    }

    /// 清空所有时间线
    pub fn clear(&mut self) {
        self.timelines.clear();
    }
}

/// 时间线索引项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineSummary {
    pub id: String,
    pub request_id: String,
    pub event_count: usize,
    pub created_at: u64,
    pub updated_at: u64,
}

impl ToolTimeline {
    /// 获取摘要
    pub fn summary(&self) -> TimelineSummary {
        TimelineSummary {
            id: self.id.clone(),
            request_id: self.request_id.clone(),
            event_count: self.events.len(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_visualizer_creation() {
        let temp_dir = TempDir::new().unwrap();
        let visualizer = ToolCallVisualizer::new(temp_dir.path()).unwrap();
        assert!(visualizer.list_timelines().is_empty());
    }

    #[test]
    fn test_timeline_creation() {
        let temp_dir = TempDir::new().unwrap();
        let mut visualizer = ToolCallVisualizer::new(temp_dir.path()).unwrap();

        let timeline_id = visualizer.create_timeline("req_1");
        assert!(timeline_id.starts_with("timeline_"));

        let timeline = visualizer.get_timeline(&timeline_id);
        assert!(timeline.is_some());
        assert_eq!(timeline.unwrap().request_id, "req_1");
    }

    #[test]
    fn test_add_tool_call() {
        let temp_dir = TempDir::new().unwrap();
        let mut visualizer = ToolCallVisualizer::new(temp_dir.path()).unwrap();

        let timeline_id = visualizer.create_timeline("req_1");

        let event_id = visualizer
            .add_tool_call(
                &timeline_id,
                "read_file",
                Some(serde_json::json!({"path": "test.txt"})),
                Some("Need to read the file"),
                vec![],
            )
            .unwrap();

        let timeline = visualizer.get_timeline(&timeline_id);
        assert_eq!(timeline.unwrap().events.len(), 1);
        assert_eq!(timeline.unwrap().events[0].tool_name, "read_file");
    }

    #[test]
    fn test_dependency_graph() {
        let temp_dir = TempDir::new().unwrap();
        let mut visualizer = ToolCallVisualizer::new(temp_dir.path()).unwrap();

        let timeline_id = visualizer.create_timeline("req_1");

        // 添加第一个工具调用
        let event1_id = visualizer
            .add_tool_call(&timeline_id, "read_file", None, None, vec![])
            .unwrap();

        // 添加依赖第一个工具调用的第二个工具
        let _event2_id = visualizer
            .add_tool_call(
                &timeline_id,
                "analyze_file",
                None,
                None,
                vec![event1_id.clone()],
            )
            .unwrap();

        let graph = visualizer.get_dependency_graph(&timeline_id);
        assert!(graph.is_some());
        let graph = graph.unwrap();
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
    }
}
