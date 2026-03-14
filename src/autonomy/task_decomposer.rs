//! 任务分解引擎
//!
//! 将复杂目标分解为可执行的子任务，分析依赖关系并排序执行顺序
//!
//! # 设计原则
//! - 基于 LLM 的任务分解 + 依赖分析
//! - DAG 结构表示任务依赖关系
//! - 纯文件存储，无需数据库
//! - 支持拓扑排序确定执行顺序

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

/// 任务分解错误类型
#[derive(Error, Debug)]
pub enum TaskDecomposerError {
    #[error("任务分解失败：{0}")]
    DecompositionFailed(String),
    #[error("依赖解析失败：{0}")]
    DependencyError(String),
    #[error("循环依赖检测：{0:?}")]
    CircularDependency(Vec<String>),
    #[error("文件操作失败：{0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON 处理失败：{0}")]
    JsonError(#[from] serde_json::Error),
}

/// 任务状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// 等待执行
    Pending,
    /// 正在执行
    InProgress,
    /// 已完成
    Completed,
    /// 执行失败
    Failed,
    /// 被依赖任务阻塞
    Blocked,
}

/// 任务结构定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// 任务唯一标识
    pub id: String,
    /// 任务描述
    pub description: String,
    /// 依赖的任务 ID 列表
    pub dependencies: Vec<String>,
    /// 任务状态
    pub status: TaskStatus,
    /// 预估步骤数
    pub estimated_steps: usize,
    /// 实际步骤数
    pub actual_steps: usize,
    /// 执行结果（可选）
    pub result: Option<String>,
    /// 错误信息（如果失败）
    pub error: Option<String>,
    /// 创建时间戳
    pub created_at: i64,
    /// 更新时间戳
    pub updated_at: i64,
}

impl Task {
    /// 创建新任务
    pub fn new(description: String, dependencies: Vec<String>) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string()[..8].to_string(),
            description,
            dependencies,
            status: TaskStatus::Pending,
            estimated_steps: 1,
            actual_steps: 0,
            result: None,
            error: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// 创建根任务（无依赖）
    pub fn root(description: String) -> Self {
        Self::new(description, vec![])
    }

    /// 标记任务为进行中
    pub fn start(&mut self) {
        self.status = TaskStatus::InProgress;
        self.updated_at = chrono::Utc::now().timestamp();
    }

    /// 标记任务为完成
    pub fn complete(&mut self, result: String) {
        self.status = TaskStatus::Completed;
        self.result = Some(result);
        self.actual_steps += 1;
        self.updated_at = chrono::Utc::now().timestamp();
    }

    /// 标记任务为失败
    pub fn fail(&mut self, error: String) {
        self.status = TaskStatus::Failed;
        self.error = Some(error);
        self.updated_at = chrono::Utc::now().timestamp();
    }

    /// 检查任务是否可执行（所有依赖已完成）
    pub fn is_ready(&self, completed_tasks: &HashSet<String>) -> bool {
        self.dependencies.is_empty() || self.dependencies.iter().all(|dep| completed_tasks.contains(dep))
    }
}

/// 任务图（DAG 结构）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskGraph {
    /// 所有任务
    pub tasks: HashMap<String, Task>,
    /// 任务 ID 列表（保持顺序）
    pub task_order: Vec<String>,
    /// 根任务 ID 列表（无依赖的任务）
    pub root_tasks: Vec<String>,
}

impl TaskGraph {
    /// 创建空任务图
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加任务
    pub fn add_task(&mut self, task: Task) {
        let id = task.id.clone();
        if task.dependencies.is_empty() {
            self.root_tasks.push(id.clone());
        }
        self.tasks.insert(id.clone(), task);
        self.task_order.push(id);
    }

    /// 获取可执行的任务（所有依赖已完成）
    pub fn get_ready_tasks(&self, completed_tasks: &HashSet<String>) -> Vec<&Task> {
        self.tasks
            .values()
            .filter(|task| {
                task.status == TaskStatus::Pending && task.is_ready(completed_tasks)
            })
            .collect()
    }

    /// 拓扑排序，返回执行顺序
    pub fn topological_sort(&self) -> Result<Vec<String>, TaskDecomposerError> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adj_list: HashMap<String, Vec<String>> = HashMap::new();

        // 初始化
        for task_id in self.tasks.keys() {
            in_degree.insert(task_id.clone(), 0);
            adj_list.insert(task_id.clone(), vec![]);
        }

        // 构建图
        for (task_id, task) in &self.tasks {
            for dep in &task.dependencies {
                if let Some(deps) = adj_list.get_mut(dep) {
                    deps.push(task_id.clone());
                }
                *in_degree.get_mut(task_id).unwrap() += 1;
            }
        }

        // Kahn 算法
        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let mut result = vec![];

        while let Some(task_id) = queue.pop_front() {
            result.push(task_id.clone());

            if let Some(neighbors) = adj_list.get(&task_id) {
                for neighbor in neighbors {
                    let degree = in_degree.get_mut(neighbor).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }

        if result.len() != self.tasks.len() {
            // 检测循环依赖
            let cycle = self.detect_cycle()?;
            return Err(TaskDecomposerError::CircularDependency(cycle));
        }

        Ok(result)
    }

    /// 检测循环依赖
    fn detect_cycle(&self) -> Result<Vec<String>, TaskDecomposerError> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = vec![];

        for task_id in self.tasks.keys() {
            if !visited.contains(task_id) {
                if let Some(cycle) = self.dfs_cycle(task_id, &mut visited, &mut rec_stack, &mut path) {
                    return Ok(cycle);
                }
            }
        }

        Err(TaskDecomposerError::DependencyError("未检测到循环依赖".to_string()))
    }

    fn dfs_cycle(
        &self,
        task_id: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(task_id.to_string());
        rec_stack.insert(task_id.to_string());
        path.push(task_id.to_string());

        if let Some(task) = self.tasks.get(task_id) {
            for dep in &task.dependencies {
                if !visited.contains(dep) {
                    if let Some(cycle) = self.dfs_cycle(dep, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(dep) {
                    // 找到循环
                    let cycle_start = path.iter().position(|x| x == dep).unwrap_or(0);
                    return Some(path[cycle_start..].to_vec());
                }
            }
        }

        path.pop();
        rec_stack.remove(task_id);
        None
    }

    /// 获取任务进度统计
    pub fn get_progress(&self) -> TaskProgress {
        let total = self.tasks.len();
        let completed = self.tasks.values().filter(|t| t.status == TaskStatus::Completed).count();
        let failed = self.tasks.values().filter(|t| t.status == TaskStatus::Failed).count();
        let in_progress = self.tasks.values().filter(|t| t.status == TaskStatus::InProgress).count();
        let pending = self.tasks.values().filter(|t| t.status == TaskStatus::Pending).count();

        TaskProgress {
            total,
            completed,
            failed,
            in_progress,
            pending,
            percentage: if total > 0 { (completed as f64 / total as f64) * 100.0 } else { 0.0 },
        }
    }

    /// 获取可变任务引用
    pub fn get_task_mut(&mut self, task_id: &str) -> Option<&mut Task> {
        self.tasks.get_mut(task_id)
    }

    /// 获取任务引用
    pub fn get_task(&self, task_id: &str) -> Option<&Task> {
        self.tasks.get(task_id)
    }
}

/// 任务进度统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgress {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub in_progress: usize,
    pub pending: usize,
    pub percentage: f64,
}

/// 分解后的任务结构（用于 LLM 输出解析）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecomposedTask {
    pub description: String,
    pub dependencies: Vec<String>,
}

/// 任务分解器
pub struct TaskDecomposer {
    /// 存储目录
    storage_dir: PathBuf,
    /// 当前任务图
    task_graph: TaskGraph,
}

impl TaskDecomposer {
    /// 创建新的任务分解器
    pub fn new(storage_dir: PathBuf) -> Result<Self, TaskDecomposerError> {
        fs::create_dir_all(&storage_dir)?;
        Ok(Self {
            storage_dir,
            task_graph: TaskGraph::new(),
        })
    }

    /// 从文件加载任务图
    pub fn load(storage_dir: PathBuf) -> Result<Self, TaskDecomposerError> {
        let graph_path = storage_dir.join("task_graph.json");
        let task_graph = if graph_path.exists() {
            let content = fs::read_to_string(&graph_path)?;
            serde_json::from_str(&content)?
        } else {
            TaskGraph::new()
        };

        Ok(Self {
            storage_dir,
            task_graph,
        })
    }

    /// 手动添加任务
    pub fn add_task(&mut self, description: String, dependencies: Vec<String>) -> Result<&Task, TaskDecomposerError> {
        let task = Task::new(description, dependencies);
        let task_id = task.id.clone();
        self.task_graph.add_task(task);
        self.save()?;
        Ok(self.task_graph.tasks.get(&task_id).unwrap())
    }

    /// 从 LLM 分解结果添加任务
    pub fn add_decomposed_tasks(&mut self, tasks: Vec<DecomposedTask>) -> Result<(), TaskDecomposerError> {
        self.task_graph = TaskGraph::new();
        for task_desc in tasks {
            let task = Task::new(task_desc.description, task_desc.dependencies);
            self.task_graph.add_task(task);
        }
        // 验证依赖关系
        self.task_graph.topological_sort()?;
        self.save()?;
        Ok(())
    }

    /// 标记任务为进行中
    pub fn start_task(&mut self, task_id: &str) -> Result<(), TaskDecomposerError> {
        if let Some(task) = self.task_graph.tasks.get_mut(task_id) {
            task.start();
            self.save()?;
        }
        Ok(())
    }

    /// 标记任务为完成
    pub fn complete_task(&mut self, task_id: &str, result: String) -> Result<(), TaskDecomposerError> {
        if let Some(task) = self.task_graph.tasks.get_mut(task_id) {
            task.complete(result);
            self.save()?;
        }
        Ok(())
    }

    /// 标记任务为失败
    pub fn fail_task(&mut self, task_id: &str, error: String) -> Result<(), TaskDecomposerError> {
        if let Some(task) = self.task_graph.tasks.get_mut(task_id) {
            task.fail(error);
            self.save()?;
        }
        Ok(())
    }

    /// 获取下一个可执行的任务
    pub fn get_next_task(&self) -> Option<&Task> {
        let completed: HashSet<String> = self.task_graph
            .tasks
            .iter()
            .filter(|(_, t)| t.status == TaskStatus::Completed)
            .map(|(id, _)| id.clone())
            .collect();

        self.task_graph
            .get_ready_tasks(&completed)
            .first()
            .copied()
    }

    /// 获取任务图
    pub fn task_graph(&self) -> &TaskGraph {
        &self.task_graph
    }

    /// 获取进度
    pub fn progress(&self) -> TaskProgress {
        self.task_graph.get_progress()
    }

    /// 保存到文件
    fn save(&self) -> Result<(), TaskDecomposerError> {
        let graph_path = self.storage_dir.join("task_graph.json");
        let content = serde_json::to_string_pretty(&self.task_graph)?;
        fs::write(&graph_path, content)?;
        Ok(())
    }

    /// 重置任务图
    pub fn reset(&mut self) -> Result<(), TaskDecomposerError> {
        self.task_graph = TaskGraph::new();
        self.save()
    }

    /// 获取存储目录
    pub fn storage_dir(&self) -> &Path {
        &self.storage_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_decomposer() -> (TaskDecomposer, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let decomposer = TaskDecomposer::new(temp_dir.path().to_path_buf()).unwrap();
        (decomposer, temp_dir)
    }

    #[test]
    fn test_task_creation() {
        let task = Task::root("测试任务".to_string());
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.dependencies.is_empty());
    }

    #[test]
    fn test_task_graph_topological_sort() {
        let mut graph = TaskGraph::new();
        
        let mut t1 = Task::root("任务 1".to_string());
        t1.id = "t1".to_string();
        
        let mut t2 = Task::new("任务 2".to_string(), vec!["t1".to_string()]);
        t2.id = "t2".to_string();
        
        let mut t3 = Task::new("任务 3".to_string(), vec!["t1".to_string(), "t2".to_string()]);
        t3.id = "t3".to_string();

        graph.add_task(t1);
        graph.add_task(t2);
        graph.add_task(t3);

        let order = graph.topological_sort().unwrap();
        assert_eq!(order.len(), 3);
        assert_eq!(order[0], "t1"); // t1 必须在最前面
    }

    #[test]
    fn test_task_progress() {
        let mut graph = TaskGraph::new();
        
        let mut t1 = Task::root("任务 1".to_string());
        t1.id = "t1".to_string();
        t1.status = TaskStatus::Completed;
        
        let mut t2 = Task::root("任务 2".to_string());
        t2.id = "t2".to_string();
        t2.status = TaskStatus::Pending;

        graph.add_task(t1);
        graph.add_task(t2);

        let progress = graph.get_progress();
        assert_eq!(progress.total, 2);
        assert_eq!(progress.completed, 1);
        assert_eq!(progress.percentage, 50.0);
    }

    #[test]
    fn test_decomposer_persistence() {
        let (mut decomposer, _temp_dir) = create_test_decomposer();
        
        // 添加任务
        decomposer.add_task("任务 1".to_string(), vec![]).unwrap();
        decomposer.add_task("任务 2".to_string(), vec![]).unwrap();
        
        // 重新加载
        let loaded = TaskDecomposer::load(decomposer.storage_dir().to_path_buf()).unwrap();
        assert_eq!(loaded.task_graph().tasks.len(), 2);
    }
}
