//! 增量式上下文哈希链（Incremental Context Hash Chain, ICHC）
//!
//! 核心思想：
//! - 链式哈希：后一条上下文的哈希 = 前一条哈希 + 当前内容哈希
//! - 形成不可篡改的哈希链，支持快照回溯
//! - 按会话/任务拆分，无需存储完整历史

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use chrono::{DateTime, Utc};

/// 哈希链节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainNode {
    /// 当前节点哈希（parent_hash + content_hash 的组合哈希）
    pub hash: String,
    /// 父节点哈希（创世节点为 "0x0000..."）
    pub parent_hash: String,
    /// 内容哈希（原始内容的 SHA256）
    pub content_hash: String,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 可选的元数据标签
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

/// 哈希链数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashChain {
    /// 会话 ID
    pub session_id: String,
    /// 当前链尾哈希
    pub current_chain_hash: String,
    /// 链节点列表
    pub chain: Vec<ChainNode>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后更新时间
    pub updated_at: DateTime<Utc>,
}

impl HashChain {
    /// 创建新的哈希链（创世节点）
    pub fn new(session_id: &str) -> Self {
        let now = Utc::now();
        let genesis_hash = "0x0000000000000000000000000000000000000000000000000000000000000000";
        
        Self {
            session_id: session_id.to_string(),
            current_chain_hash: genesis_hash.to_string(),
            chain: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// 添加新节点到链上
    pub fn append(&mut self, content_hash: &str, tag: Option<String>) -> String {
        let parent_hash = self.current_chain_hash.clone();
        
        // 计算链式哈希：SHA256(parent_hash + content_hash)
        let mut hasher = Sha256::new();
        hasher.update(parent_hash.as_bytes());
        hasher.update(content_hash.as_bytes());
        let result = hasher.finalize();
        let chain_hash = format!("0x{}", hex::encode(result));

        let node = ChainNode {
            hash: chain_hash.clone(),
            parent_hash,
            content_hash: content_hash.to_string(),
            timestamp: Utc::now(),
            tag,
        };

        self.chain.push(node);
        self.current_chain_hash = chain_hash.clone();
        self.updated_at = Utc::now();

        chain_hash
    }

    /// 获取最新 N 个节点
    pub fn get_latest(&self, n: usize) -> &[ChainNode] {
        if n >= self.chain.len() {
            &self.chain
        } else {
            &self.chain[self.chain.len() - n..]
        }
    }

    /// 获取节点数量
    pub fn len(&self) -> usize {
        self.chain.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.chain.is_empty()
    }

    /// 验证哈希链完整性
    pub fn verify(&self) -> bool {
        if self.chain.is_empty() {
            return self.current_chain_hash == "0x0000000000000000000000000000000000000000000000000000000000000000";
        }

        // 验证第一个节点的父哈希
        if self.chain[0].parent_hash != "0x0000000000000000000000000000000000000000000000000000000000000000" {
            return false;
        }

        // 验证每个节点的链式哈希
        for i in 1..self.chain.len() {
            let prev_hash = &self.chain[i - 1].hash;
            let current = &self.chain[i];

            if current.parent_hash != *prev_hash {
                return false;
            }

            // 验证当前节点的哈希计算是否正确
            let mut hasher = Sha256::new();
            hasher.update(current.parent_hash.as_bytes());
            hasher.update(current.content_hash.as_bytes());
            let result = hasher.finalize();
            let expected_hash = format!("0x{}", hex::encode(result));

            if current.hash != expected_hash {
                return false;
            }
        }

        // 验证链尾哈希
        if let Some(last) = self.chain.last() {
            if self.current_chain_hash != last.hash {
                return false;
            }
        }

        true
    }

    /// 创建快照（用于回溯）
    pub fn create_snapshot(&self) -> HashChainSnapshot {
        HashChainSnapshot {
            session_id: self.session_id.clone(),
            snapshot_hash: self.current_chain_hash.clone(),
            chain_length: self.chain.len(),
            nodes: self.chain.clone(),
            snapshot_at: Utc::now(),
        }
    }
}

/// 哈希链快照（用于回溯）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashChainSnapshot {
    pub session_id: String,
    pub snapshot_hash: String,
    pub chain_length: usize,
    pub nodes: Vec<ChainNode>,
    pub snapshot_at: DateTime<Utc>,
}

/// 哈希链管理器
pub struct HashChainManager {
    session_dir: PathBuf,
    chains: HashMap<String, HashChain>,
}

impl HashChainManager {
    /// 创建哈希链管理器
    pub fn new<P: AsRef<Path>>(session_dir: P) -> Result<Self> {
        let session_dir = session_dir.as_ref().to_path_buf();

        // 确保会话目录存在
        std::fs::create_dir_all(&session_dir)
            .with_context(|| format!("Failed to create session directory: {:?}", session_dir))?;

        Ok(Self {
            session_dir,
            chains: HashMap::new(),
        })
    }

    /// 获取或创建哈希链（使用 entry API 避免 borrow checker 问题）
    pub fn get_or_create_chain(&mut self, session_id: &str) -> Result<&mut HashChain> {
        use std::collections::hash_map::Entry;

        // 先检查是否已存在（避免不必要的文件操作）
        if self.chains.contains_key(session_id) {
            return Ok(self.chains.get_mut(session_id).unwrap());
        }

        // 构建链文件路径（在 entry 借用之前）
        let chain_file = self.get_chain_file_path(session_id);

        // 使用 entry API 原子性地检查并插入
        let entry = match self.chains.entry(session_id.to_string()) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let chain = if chain_file.exists() {
                    // 从文件加载
                    let content = std::fs::read_to_string(&chain_file)
                        .with_context(|| format!("Failed to read chain file: {:?}", chain_file))?;
                    serde_json::from_str(&content)
                        .with_context(|| format!("Failed to parse chain file: {:?}", chain_file))?
                } else {
                    // 创建新链
                    HashChain::new(session_id)
                };

                entry.insert(chain)
            }
        };

        Ok(entry)
    }

    /// 添加内容到哈希链
    pub fn append(&mut self, session_id: &str, content_hash: &str, tag: Option<String>) -> Result<String> {
        let chain = self.get_or_create_chain(session_id)?;
        let chain_hash = chain.append(content_hash, tag);
        
        // 持久化到文件
        self.save_chain(session_id)?;

        Ok(chain_hash)
    }

    /// 保存哈希链到文件
    fn save_chain(&self, session_id: &str) -> Result<()> {
        if let Some(chain) = self.chains.get(session_id) {
            let chain_file = self.get_chain_file_path(session_id);
            let content = serde_json::to_string_pretty(chain)
                .with_context(|| format!("Failed to serialize chain: {:?}", session_id))?;
            
            std::fs::write(&chain_file, content)
                .with_context(|| format!("Failed to write chain file: {:?}", chain_file))?;
        }
        Ok(())
    }

    /// 获取哈希链文件路径
    fn get_chain_file_path(&self, session_id: &str) -> PathBuf {
        self.session_dir.join(format!("hash_chain_{}.json", session_id))
    }

    /// 获取最新 N 个节点
    pub fn get_latest_nodes(&mut self, session_id: &str, n: usize) -> Result<Vec<ChainNode>> {
        let chain = self.get_or_create_chain(session_id)?;
        Ok(chain.get_latest(n).to_vec())
    }

    /// 验证哈希链完整性
    pub fn verify_chain(&mut self, session_id: &str) -> Result<bool> {
        let chain = self.get_or_create_chain(session_id)?;
        Ok(chain.verify())
    }

    /// 初始化哈希链到指定文件路径（用于新分支创建）
    pub fn initialize_chain_to_path(&mut self, session_id: &str, chain_file_path: &Path) -> Result<String> {
        let chain = self.get_or_create_chain(session_id)?;
        let genesis_hash = chain.current_chain_hash.clone();
        
        // 保存到指定路径
        let content = serde_json::to_string_pretty(chain)
            .with_context(|| format!("Failed to serialize chain: {:?}", session_id))?;
        std::fs::write(chain_file_path, content)
            .with_context(|| format!("Failed to write chain file: {:?}", chain_file_path))?;
        
        Ok(genesis_hash)
    }

    /// 创建快照
    pub fn create_snapshot(&mut self, session_id: &str) -> Result<HashChainSnapshot> {
        let chain = self.get_or_create_chain(session_id)?;
        let snapshot = chain.create_snapshot();

        // 保存快照到文件
        let snapshot_file = self.session_dir.join(format!(
            "snapshot_{}_{}.json",
            session_id,
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        ));

        let content = serde_json::to_string_pretty(&snapshot)
            .with_context(|| format!("Failed to serialize snapshot: {:?}", session_id))?;
        
        std::fs::write(&snapshot_file, content)
            .with_context(|| format!("Failed to write snapshot file: {:?}", snapshot_file))?;

        Ok(snapshot)
    }

    /// 获取所有快照
    pub fn list_snapshots(&self, session_id: &str) -> Result<Vec<PathBuf>> {
        let pattern = format!("snapshot_{}_", session_id);
        let mut snapshots = Vec::new();

        if self.session_dir.exists() {
            for entry in std::fs::read_dir(&self.session_dir)
                .with_context(|| format!("Failed to read session directory: {:?}", self.session_dir))?
            {
                let entry = entry?;
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy();
                
                if file_name_str.starts_with(&pattern) && file_name_str.ends_with(".json") {
                    snapshots.push(entry.path());
                }
            }
        }

        snapshots.sort_by(|a, b| b.cmp(a)); // 按时间倒序
        Ok(snapshots)
    }

    /// 加载快照（回溯到某个状态）
    pub fn load_snapshot(&mut self, session_id: &str, snapshot_path: &Path) -> Result<HashChainSnapshot> {
        let content = std::fs::read_to_string(snapshot_path)
            .with_context(|| format!("Failed to read snapshot file: {:?}", snapshot_path))?;
        
        let snapshot: HashChainSnapshot = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse snapshot: {:?}", snapshot_path))?;

        // 恢复哈希链状态
        let chain = HashChain {
            session_id: snapshot.session_id.clone(),
            current_chain_hash: snapshot.snapshot_hash.clone(),
            chain: snapshot.nodes.clone(),
            created_at: snapshot.snapshot_at,
            updated_at: snapshot.snapshot_at,
        };

        self.chains.insert(session_id.to_string(), chain);

        Ok(snapshot)
    }

    /// 获取当前链哈希（用于云端传输）
    pub fn get_current_hash(&mut self, session_id: &str) -> Result<Option<String>> {
        let chain = self.get_or_create_chain(session_id)?;
        if chain.is_empty() {
            Ok(None)
        } else {
            Ok(Some(chain.current_chain_hash.clone()))
        }
    }

    /// 获取云端传输数据（当前链哈希 + 最新 N 个节点）
    pub fn get_cloud_payload(&mut self, session_id: &str, n: usize) -> Result<CloudChainPayload> {
        let chain = self.get_or_create_chain(session_id)?;
        
        Ok(CloudChainPayload {
            session_id: chain.session_id.clone(),
            current_chain_hash: chain.current_chain_hash.clone(),
            latest_nodes: chain.get_latest(n).to_vec(),
            chain_length: chain.len(),
        })
    }
}

/// 云端传输载荷（最小化数据传输）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudChainPayload {
    pub session_id: String,
    pub current_chain_hash: String,
    pub latest_nodes: Vec<ChainNode>,
    pub chain_length: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_hash_chain_creation() {
        let chain = HashChain::new("test_session");
        assert_eq!(chain.current_chain_hash, "0x0000000000000000000000000000000000000000000000000000000000000000");
        assert!(chain.is_empty());
    }

    #[test]
    fn test_hash_chain_append() {
        let mut chain = HashChain::new("test_session");
        
        let hash1 = chain.append("content_hash_1", Some("tag1".to_string()));
        assert_ne!(hash1, "0x0000000000000000000000000000000000000000000000000000000000000000");
        assert_eq!(chain.len(), 1);

        let hash2 = chain.append("content_hash_2", None);
        assert_ne!(hash1, hash2);
        assert_eq!(chain.len(), 2);
    }

    #[test]
    fn test_hash_chain_verify() {
        let mut chain = HashChain::new("test_session");
        chain.append("content_hash_1", None);
        chain.append("content_hash_2", None);
        
        assert!(chain.verify());
    }

    #[test]
    fn test_hash_chain_manager() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = HashChainManager::new(temp_dir.path()).unwrap();

        // 添加内容
        let chain_hash = manager.append("sess1", "content_hash_1", Some("test".to_string())).unwrap();
        assert!(!chain_hash.is_empty());

        // 获取最新节点
        let nodes = manager.get_latest_nodes("sess1", 10).unwrap();
        assert_eq!(nodes.len(), 1);

        // 验证链
        let is_valid = manager.verify_chain("sess1").unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = HashChainManager::new(temp_dir.path()).unwrap();

        // 添加多个节点
        manager.append("sess1", "hash1", None).unwrap();
        manager.append("sess1", "hash2", None).unwrap();
        manager.append("sess1", "hash3", None).unwrap();

        // 创建快照
        let snapshot = manager.create_snapshot("sess1").unwrap();
        assert_eq!(snapshot.chain_length, 3);

        // 添加更多内容
        manager.append("sess1", "hash4", None).unwrap();

        // 加载快照（回溯）
        let snapshots = manager.list_snapshots("sess1").unwrap();
        assert!(!snapshots.is_empty());

        let loaded = manager.load_snapshot("sess1", &snapshots[0]).unwrap();
        assert_eq!(loaded.chain_length, 3);

        // 验证回溯后的状态
        let nodes = manager.get_latest_nodes("sess1", 10).unwrap();
        assert_eq!(nodes.len(), 3);
    }

    #[test]
    fn test_cloud_payload() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = HashChainManager::new(temp_dir.path()).unwrap();

        // 添加 5 个节点
        for i in 0..5 {
            manager.append("sess1", &format!("hash{}", i), None).unwrap();
        }

        // 获取云端载荷（只传最新 2 个）
        let payload = manager.get_cloud_payload("sess1", 2).unwrap();
        assert_eq!(payload.latest_nodes.len(), 2);
        assert_eq!(payload.chain_length, 5);
        assert!(!payload.current_chain_hash.is_empty());
    }

    #[test]
    fn test_chain_persistence() {
        let temp_dir = TempDir::new().unwrap();

        // 创建管理器并添加数据
        {
            let mut manager = HashChainManager::new(temp_dir.path()).unwrap();
            manager.append("sess1", "hash1", None).unwrap();
            manager.append("sess1", "hash2", None).unwrap();
        }

        // 重新创建管理器（从文件加载）
        {
            let mut manager = HashChainManager::new(temp_dir.path()).unwrap();
            let nodes = manager.get_latest_nodes("sess1", 10).unwrap();
            assert_eq!(nodes.len(), 2);

            let is_valid = manager.verify_chain("sess1").unwrap();
            assert!(is_valid);
        }
    }

    #[test]
    fn test_hash_chain_large_scale() {
        // 测试大规模节点（1000 个）
        let temp_dir = TempDir::new().unwrap();
        let mut manager = HashChainManager::new(temp_dir.path()).unwrap();

        const NODE_COUNT: usize = 1000;
        for i in 0..NODE_COUNT {
            manager.append("sess1", &format!("hash{}", i), None).unwrap();
        }

        // 验证节点数量
        let nodes = manager.get_latest_nodes("sess1", NODE_COUNT).unwrap();
        assert_eq!(nodes.len(), NODE_COUNT);

        // 验证链完整性
        let is_valid = manager.verify_chain("sess1").unwrap();
        assert!(is_valid);

        // 验证链尾哈希不为空
        let current_hash = manager.get_current_hash("sess1").unwrap();
        assert!(current_hash.is_some());
    }

    #[test]
    fn test_hash_chain_concurrent_append() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        // 测试多线程并发添加
        let temp_dir = TempDir::new().unwrap();
        let manager = Arc::new(Mutex::new(HashChainManager::new(temp_dir.path()).unwrap()));

        let mut handles = vec![];
        const THREAD_COUNT: usize = 4;
        const APPEND_PER_THREAD: usize = 25;

        for t in 0..THREAD_COUNT {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                for i in 0..APPEND_PER_THREAD {
                    let mut mgr = manager_clone.lock().unwrap();
                    let session_id = format!("thread_{}", t);
                    mgr.append(&session_id, &format!("hash_{}_{}", t, i), None).unwrap();
                }
            });
            handles.push(handle);
        }

        // 等待所有线程完成
        for handle in handles {
            handle.join().unwrap();
        }

        // 验证每个会话的链（只验证管理器没有崩溃）
        let _mgr = manager.lock().unwrap();
    }

    #[test]
    fn test_hash_chain_empty_state() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = HashChainManager::new(temp_dir.path()).unwrap();

        // 测试空链状态
        let current_hash = manager.get_current_hash("empty_sess").unwrap();
        assert!(current_hash.is_none());

        let nodes = manager.get_latest_nodes("empty_sess", 10).unwrap();
        assert!(nodes.is_empty());

        let is_valid = manager.verify_chain("empty_sess").unwrap();
        assert!(is_valid);
    }
}
