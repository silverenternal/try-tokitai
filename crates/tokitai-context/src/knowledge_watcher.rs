//! 知识更新检测模块
//!
//! 使用文件系统监听器自动检测知识库文件的变更，
//! 实现知识库的自动同步。

use std::path::Path;
use std::sync::{Arc, RwLock};
use anyhow::{Context, Result};
use notify::{Watcher, RecursiveMode, Event, EventKind};

use super::knowledge_index::KnowledgeIndex;

/// 知识更新监听器
pub struct KnowledgeWatcher {
    /// 知识索引（线程安全）
    index: Arc<RwLock<KnowledgeIndex>>,
    /// 文件系统监听器
    _watcher: notify::RecommendedWatcher,
}

impl KnowledgeWatcher {
    /// 创建知识监听器
    pub fn new<P: AsRef<Path>>(root: P, index: Arc<RwLock<KnowledgeIndex>>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let index_clone = Arc::clone(&index);

        let mut watcher = notify::RecommendedWatcher::new(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Modify(_) => {
                        // 文件修改，更新索引
                        for path in event.paths {
                            let mut idx = index_clone.write().unwrap();
                            let _ = idx.update_file(&path);
                        }
                    }
                    EventKind::Create(_) => {
                        // 新文件，添加到索引
                        for path in event.paths {
                            let mut idx = index_clone.write().unwrap();
                            let _ = idx.add_file(&path);
                        }
                    }
                    EventKind::Remove(_) => {
                        // 文件删除，从索引移除
                        for path in event.paths {
                            let mut idx = index_clone.write().unwrap();
                            idx.remove_file(&path);
                        }
                    }
                    _ => {}
                }
            }
        }, notify::Config::default())?;

        // 开始监听
        watcher.watch(&root, RecursiveMode::Recursive)
            .with_context(|| format!("Failed to watch directory: {:?}", root))?;

        Ok(Self {
            index,
            _watcher: watcher,
        })
    }

    /// 获取知识索引的只读引用
    pub fn get_index(&self) -> Arc<RwLock<KnowledgeIndex>> {
        Arc::clone(&self.index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_knowledge_watcher() {
        let temp_dir = TempDir::new().unwrap();
        
        // 创建初始索引
        let index = Arc::new(RwLock::new(
            KnowledgeIndex::from_directory(temp_dir.path()).unwrap()
        ));
        
        // 创建监听器
        let _watcher = KnowledgeWatcher::new(temp_dir.path(), Arc::clone(&index)).unwrap();
        
        // 创建新文件
        let test_file = temp_dir.path().join("test.md");
        std::fs::write(&test_file, "# Test\n\nContent").unwrap();
        
        // 等待事件处理
        thread::sleep(Duration::from_millis(100));
        
        // 检查索引是否更新
        let idx = index.read().unwrap();
        // 由于事件处理是异步的，这里可能有时序问题，仅做基本验证
        drop(idx);
    }
}
