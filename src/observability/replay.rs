//! 迭代回放系统
//!
//! 支持回放自主迭代的完整过程用于调试和学习
//!
//! ## 录制格式
//! - Header: 迭代元数据（目标、开始时间、结束时间）
//! - Events: 按时间排序的事件列表
//! - Snapshots: 关键状态快照
//!
//! ## 回放控制
//! - play - 播放
//! - pause - 暂停
//! - step_forward - 单步前进
//! - step_back - 单步后退
//! - jump_to - 跳转到指定事件

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::Result;
use std::sync::Arc;

/// 事件类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventType {
    /// 迭代开始
    IterationStart,
    /// 任务分解
    TaskDecompose,
    /// 规划生成
    PlanGenerated,
    /// 工具调用开始
    ToolCallStart,
    /// 工具调用结束
    ToolCallEnd,
    /// 任务完成
    TaskComplete,
    /// 审查开始
    ReviewStart,
    /// 审查完成
    ReviewComplete,
    /// 错误发生
    ErrorOccurred,
    /// 用户干预
    UserIntervention,
    /// 迭代结束
    IterationEnd,
}

/// 回放事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayEvent {
    /// 事件 ID
    pub id: String,
    /// 事件类型
    pub event_type: EventType,
    /// 时间戳
    pub timestamp: u64,
    /// 事件数据（JSON）
    pub data: Option<String>,
    /// 关联的任务 ID
    pub task_id: Option<String>,
    /// 关联的工具名称
    pub tool_name: Option<String>,
}

/// 状态快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// 快照 ID
    pub id: String,
    /// 时间戳
    pub timestamp: u64,
    /// 快照数据（JSON）
    pub data: String,
    /// 快照描述
    pub description: String,
}

/// 迭代头信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayHeader {
    /// 迭代 ID
    pub iteration_id: String,
    /// 迭代目标
    pub goal: String,
    /// 开始时间
    pub start_time: u64,
    /// 结束时间（0 表示进行中）
    pub end_time: u64,
    /// 总事件数
    pub total_events: usize,
    /// 总快照数
    pub total_snapshots: usize,
    /// 迭代状态
    pub status: IterationStatus,
}

/// 迭代状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IterationStatus {
    /// 进行中
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已终止
    Aborted,
}

/// 迭代回放记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayRecord {
    /// 头信息
    pub header: ReplayHeader,
    /// 事件列表
    pub events: Vec<ReplayEvent>,
    /// 快照列表
    pub snapshots: Vec<StateSnapshot>,
}

/// 回放位置
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ReplayPosition {
    /// 当前事件索引
    pub event_index: usize,
    /// 当前快照索引
    pub snapshot_index: usize,
}

/// 回放播放器
pub struct ReplayPlayer {
    /// 当前回放记录
    record: Option<Arc<ReplayRecord>>,
    /// 当前位置
    position: ReplayPosition,
    /// 是否暂停
    is_paused: bool,
    /// 播放速度（1.0 为正常）
    playback_speed: f32,
}

/// 迭代回放系统
pub struct ReplaySystem {
    /// 数据目录
    data_dir: PathBuf,
    /// 当前回放记录
    current_recording: Option<ReplayRecord>,
    /// 已保存的回放列表
    saved_replays: Vec<PathBuf>,
    /// 播放器
    player: ReplayPlayer,
    /// 配置
    config: ReplayConfig,
}

/// 回放配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayConfig {
    /// 是否自动录制
    pub auto_record_enabled: bool,
    /// 是否压缩存储
    pub compress_storage: bool,
    /// 最大回放数量
    pub max_replays: usize,
    /// 事件数据大小限制（字节）
    pub max_event_data_size: usize,
}

impl Default for ReplayConfig {
    fn default() -> Self {
        Self {
            auto_record_enabled: true,
            compress_storage: false,
            max_replays: 50,
            max_event_data_size: 10240, // 10KB
        }
    }
}

impl ReplayPlayer {
    /// 创建新的播放器
    pub fn new() -> Self {
        Self {
            record: None,
            position: ReplayPosition {
                event_index: 0,
                snapshot_index: 0,
            },
            is_paused: true,
            playback_speed: 1.0,
        }
    }

    /// 加载回放记录
    pub fn load(&mut self, record: ReplayRecord) {
        self.record = Some(Arc::new(record));
        self.position.event_index = 0;
        self.position.snapshot_index = 0;
        self.is_paused = true;
    }

    /// 播放
    pub fn play(&mut self) {
        self.is_paused = false;
    }

    /// 暂停
    pub fn pause(&mut self) {
        self.is_paused = true;
    }

    /// 单步前进
    pub fn step_forward(&mut self) -> Option<&ReplayEvent> {
        if let Some(ref record) = self.record {
            if self.position.event_index < record.events.len() {
                let event = &record.events[self.position.event_index];
                self.position.event_index += 1;
                return Some(event);
            }
        }
        None
    }

    /// 单步后退
    pub fn step_backward(&mut self) -> Option<&ReplayEvent> {
        if self.position.event_index > 0 {
            self.position.event_index -= 1;
            if let Some(ref record) = self.record {
                return Some(&record.events[self.position.event_index]);
            }
        }
        None
    }

    /// 跳转到指定事件
    pub fn jump_to_event(&mut self, event_id: &str) -> Option<&ReplayEvent> {
        if let Some(ref record) = self.record {
            if let Some(index) = record.events.iter().position(|e| e.id == event_id) {
                self.position.event_index = index;
                return Some(&record.events[index]);
            }
        }
        None
    }

    /// 获取当前事件
    pub fn current_event(&self) -> Option<&ReplayEvent> {
        if let Some(ref record) = self.record {
            if self.position.event_index < record.events.len() {
                return Some(&record.events[self.position.event_index]);
            }
        }
        None
    }

    /// 获取播放状态
    pub fn get_status(&self) -> ReplayStatus {
        let total_events = self.record.as_ref().map(|r| r.events.len()).unwrap_or(0);
        
        ReplayStatus {
            is_playing: !self.is_paused,
            current_event_index: self.position.event_index,
            total_events,
            playback_speed: self.playback_speed,
            progress: if total_events > 0 {
                self.position.event_index as f32 / total_events as f32
            } else {
                0.0
            },
        }
    }

    /// 设置播放速度
    pub fn set_speed(&mut self, speed: f32) {
        self.playback_speed = speed.max(0.25).min(4.0);
    }
}

impl Default for ReplayPlayer {
    fn default() -> Self {
        Self::new()
    }
}

/// 回放状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayStatus {
    /// 是否正在播放
    pub is_playing: bool,
    /// 当前事件索引
    pub current_event_index: usize,
    /// 总事件数
    pub total_events: usize,
    /// 播放速度
    pub playback_speed: f32,
    /// 进度 (0.0-1.0)
    pub progress: f32,
}

impl ReplaySystem {
    /// 创建新的回放系统
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        let replays_dir = data_dir.join("replays");
        std::fs::create_dir_all(&replays_dir)?;
        
        let mut system = Self {
            data_dir,
            current_recording: None,
            saved_replays: Vec::new(),
            player: ReplayPlayer::new(),
            config: ReplayConfig::default(),
        };
        
        // 加载已有回放
        system.load_saved_replays()?;
        
        Ok(system)
    }

    /// 开始录制
    pub fn start_recording(&mut self, iteration_id: &str, goal: &str) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let header = ReplayHeader {
            iteration_id: iteration_id.to_string(),
            goal: goal.to_string(),
            start_time: now,
            end_time: 0,
            total_events: 0,
            total_snapshots: 0,
            status: IterationStatus::Running,
        };
        
        self.current_recording = Some(ReplayRecord {
            header,
            events: Vec::new(),
            snapshots: Vec::new(),
        });
        
        // 记录开始事件
        self.record_event(EventType::IterationStart, Some(serde_json::json!({
            "iteration_id": iteration_id,
            "goal": goal,
        })), None, None)?;
        
        Ok(())
    }

    /// 记录事件
    pub fn record_event(
        &mut self,
        event_type: EventType,
        data: Option<serde_json::Value>,
        task_id: Option<&str>,
        tool_name: Option<&str>,
    ) -> Result<String> {
        if self.current_recording.is_none() {
            anyhow::bail!("No active recording");
        }
        
        let recording = self.current_recording.as_mut().unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let id = format!("evt_{}_{}", event_type.as_str(), now);
        
        let data_str = data.map(|v| {
            let json = serde_json::to_string(&v).unwrap();
            if json.len() > self.config.max_event_data_size {
                json[..self.config.max_event_data_size].to_string()
            } else {
                json
            }
        });
        
        let event = ReplayEvent {
            id: id.clone(),
            event_type,
            timestamp: now,
            data: data_str,
            task_id: task_id.map(String::from),
            tool_name: tool_name.map(String::from),
        };
        
        recording.events.push(event);
        recording.header.total_events = recording.events.len();
        
        Ok(id)
    }

    /// 添加状态快照
    pub fn add_snapshot(&mut self, data: serde_json::Value, description: &str) -> Result<String> {
        if self.current_recording.is_none() {
            anyhow::bail!("No active recording");
        }
        
        let recording = self.current_recording.as_mut().unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let id = format!("snap_{}", now);
        
        let snapshot = StateSnapshot {
            id: id.clone(),
            timestamp: now,
            data: serde_json::to_string(&data)?,
            description: description.to_string(),
        };
        
        recording.snapshots.push(snapshot);
        recording.header.total_snapshots = recording.snapshots.len();
        
        Ok(id)
    }

    /// 停止录制
    pub fn stop_recording(&mut self, status: IterationStatus) -> Result<Option<PathBuf>> {
        if let Some(mut recording) = self.current_recording.take() {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            recording.header.end_time = now;
            recording.header.status = status.clone();

            // 记录结束事件
            let event_data = serde_json::to_string(&serde_json::json!({
                "status": status.as_str()
            })).unwrap();
            
            recording.events.push(ReplayEvent {
                id: format!("evt_IterationEnd_{}", now),
                event_type: EventType::IterationEnd,
                timestamp: now,
                data: Some(event_data),
                task_id: None,
                tool_name: None,
            });

            recording.header.total_events = recording.events.len();
            
            // 保存回放
            let file_path = self.save_replay(&recording)?;
            
            return Ok(Some(file_path));
        }
        
        Ok(None)
    }

    /// 保存回放
    fn save_replay(&mut self, replay: &ReplayRecord) -> Result<PathBuf> {
        let replays_dir = self.data_dir.join("replays");
        std::fs::create_dir_all(&replays_dir)?;
        
        let file_name = format!("replay_{}.json", replay.header.iteration_id);
        let file_path = replays_dir.join(&file_name);
        
        let json = serde_json::to_string_pretty(replay)?;
        std::fs::write(&file_path, json)?;
        
        self.saved_replays.push(file_path.clone());

        // 清理旧回放
        while self.saved_replays.len() > self.config.max_replays {
            let old_path = self.saved_replays.remove(0);
            std::fs::remove_file(old_path).ok();
        }

        Ok(file_path)
    }

    /// 加载已保存的回放
    fn load_saved_replays(&mut self) -> Result<()> {
        let replays_dir = self.data_dir.join("replays");
        
        if replays_dir.exists() {
            for entry in std::fs::read_dir(&replays_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    self.saved_replays.push(path);
                }
            }
        }
        
        Ok(())
    }

    /// 加载回放用于播放
    pub fn load_replay(&mut self, iteration_id: &str) -> Result<()> {
        let replays_dir = self.data_dir.join("replays");
        let file_name = format!("replay_{}.json", iteration_id);
        let file_path = replays_dir.join(&file_name);
        
        if !file_path.exists() {
            anyhow::bail!("Replay not found: {}", iteration_id);
        }
        
        let json = std::fs::read_to_string(&file_path)?;
        let replay: ReplayRecord = serde_json::from_str(&json)?;
        
        self.player.load(replay);
        
        Ok(())
    }

    /// 获取播放器
    pub fn player(&mut self) -> &mut ReplayPlayer {
        &mut self.player
    }

    /// 获取所有回放列表
    pub fn get_replay_list(&self) -> Vec<&Path> {
        self.saved_replays.iter().map(|p| p.as_path()).collect()
    }

    /// 获取当前录制状态
    pub fn get_recording_status(&self) -> Option<&ReplayHeader> {
        self.current_recording.as_ref().map(|r| &r.header)
    }
}

impl EventType {
    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::IterationStart => "IterationStart",
            EventType::TaskDecompose => "TaskDecompose",
            EventType::PlanGenerated => "PlanGenerated",
            EventType::ToolCallStart => "ToolCallStart",
            EventType::ToolCallEnd => "ToolCallEnd",
            EventType::TaskComplete => "TaskComplete",
            EventType::ReviewStart => "ReviewStart",
            EventType::ReviewComplete => "ReviewComplete",
            EventType::ErrorOccurred => "ErrorOccurred",
            EventType::UserIntervention => "UserIntervention",
            EventType::IterationEnd => "IterationEnd",
        }
    }
}

impl IterationStatus {
    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            IterationStatus::Running => "Running",
            IterationStatus::Completed => "Completed",
            IterationStatus::Failed => "Failed",
            IterationStatus::Aborted => "Aborted",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_replay_system_creation() {
        let temp_dir = TempDir::new().unwrap();
        let system = ReplaySystem::new(temp_dir.path()).unwrap();
        assert!(system.saved_replays.is_empty());
    }

    #[test]
    fn test_recording() {
        let temp_dir = TempDir::new().unwrap();
        let mut system = ReplaySystem::new(temp_dir.path()).unwrap();
        
        system.start_recording("iter_1", "Test goal").unwrap();
        
        system.record_event(
            EventType::TaskDecompose,
            Some(serde_json::json!({"tasks": ["task1", "task2"]})),
            None,
            None,
        ).unwrap();
        
        system.add_snapshot(
            serde_json::json!({"state": "test"}),
            "Test snapshot",
        ).unwrap();
        
        let file_path = system.stop_recording(IterationStatus::Completed).unwrap();
        assert!(file_path.is_some());
        assert!(file_path.unwrap().exists());
    }

    #[test]
    fn test_player_controls() {
        let mut player = ReplayPlayer::new();
        
        let record = ReplayRecord {
            header: ReplayHeader {
                iteration_id: "test".to_string(),
                goal: "Test".to_string(),
                start_time: 0,
                end_time: 0,
                total_events: 2,
                total_snapshots: 0,
                status: IterationStatus::Completed,
            },
            events: vec![
                ReplayEvent {
                    id: "evt1".to_string(),
                    event_type: EventType::IterationStart,
                    timestamp: 0,
                    data: None,
                    task_id: None,
                    tool_name: None,
                },
                ReplayEvent {
                    id: "evt2".to_string(),
                    event_type: EventType::IterationEnd,
                    timestamp: 100,
                    data: None,
                    task_id: None,
                    tool_name: None,
                },
            ],
            snapshots: Vec::new(),
        };
        
        player.load(record);

        // 测试单步前进
        let event1 = player.step_forward();
        assert!(event1.is_some());
        assert_eq!(event1.unwrap().id, "evt1");

        let event2 = player.step_forward();
        assert_eq!(event2.unwrap().id, "evt2");

        // 测试单步后退（后退一步回到 evt2）
        let event2_again = player.step_backward();
        assert_eq!(event2_again.unwrap().id, "evt2");
        
        // 再后退一步回到 evt1
        let event1_again = player.step_backward();
        assert_eq!(event1_again.unwrap().id, "evt1");
    }
}
