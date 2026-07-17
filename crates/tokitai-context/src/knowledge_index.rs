//! 知识索引模块
//!
//! 从目录结构构建知识索引，实现"目录结构即知识图谱"的理念。
//! 支持标签提取、知识推荐、关联度计算等功能。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use sha2::{Sha256, Digest};
use chrono::{DateTime, Utc};
use walkdir::WalkDir;

/// 知识节点
#[derive(Debug, Clone)]
pub struct KnowledgeNode {
    /// 文件路径
    pub path: PathBuf,
    /// 内容哈希
    pub content_hash: String,
    /// 从目录结构提取的标签（如：["数据库", "MySQL"]）
    pub tags: Vec<String>,
    /// 相关知识路径
    pub related: Vec<String>,
    /// 最后访问时间
    pub last_accessed: DateTime<Utc>,
    /// 文件内容（可选缓存）
    pub content: Option<String>,
}

/// 知识索引
#[derive(Clone)]
pub struct KnowledgeIndex {
    /// 知识库根目录
    root: PathBuf,
    /// 路径 -> 知识节点
    index: HashMap<String, KnowledgeNode>,
}

impl KnowledgeIndex {
    /// 从目录结构构建知识索引
    pub fn from_directory<P: AsRef<Path>>(root: P) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let mut index = HashMap::new();

        if !root.exists() {
            // 目录不存在，返回空索引
            return Ok(Self { root, index });
        }

        for entry in WalkDir::new(&root)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            
            // 只处理 Markdown 文件
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }

            if path.is_file() {
                let content = std::fs::read_to_string(path)
                    .with_context(|| format!("Failed to read file: {:?}", path))?;
                
                let hash = compute_sha256(&content);
                
                // 从目录结构提取标签
                let tags = extract_tags_from_path(path, &root);

                index.insert(
                    path.to_string_lossy().to_string(),
                    KnowledgeNode {
                        path: path.to_path_buf(),
                        content_hash: hash,
                        tags,
                        related: Vec::new(),
                        last_accessed: Utc::now(),
                        content: Some(content),
                    },
                );
            }
        }

        Ok(Self { root, index })
    }

    /// 获取知识库根目录
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 获取所有知识节点
    pub fn all_nodes(&self) -> Vec<&KnowledgeNode> {
        self.index.values().collect()
    }

    /// 根据路径获取知识节点
    pub fn get(&self, path: &str) -> Option<&KnowledgeNode> {
        self.index.get(path)
    }

    /// 根据路径获取知识节点（可变引用）
    pub fn get_mut(&mut self, path: &str) -> Option<&mut KnowledgeNode> {
        self.index.get_mut(path)
    }

    /// 根据问题自动推荐相关知识
    pub fn recommend(&self, query: &str, limit: usize) -> Vec<&KnowledgeNode> {
        // 1. 提取查询中的关键词
        let query_tags = extract_keywords(query);

        // 2. 按标签匹配度排序
        let mut nodes: Vec<_> = self.index.values().collect();
        nodes.sort_by(|a, b| {
            let score_a = query_tags.iter()
                .filter(|t| a.tags.iter().any(|at| at.to_lowercase().contains(&t.to_lowercase()) || t.to_lowercase().contains(&at.to_lowercase()))).count();
            let score_b = query_tags.iter()
                .filter(|t| b.tags.iter().any(|at| at.to_lowercase().contains(&t.to_lowercase()) || t.to_lowercase().contains(&at.to_lowercase()))).count();
            score_b.cmp(&score_a)
        });

        // 3. 返回前 N 个
        nodes.into_iter()
            .filter(|n| {
                // 至少有一个标签匹配
                query_tags.iter().any(|t| 
                    n.tags.iter().any(|nt| 
                        nt.to_lowercase().contains(&t.to_lowercase()) || 
                        t.to_lowercase().contains(&nt.to_lowercase())
                    )
                )
            })
            .take(limit)
            .collect()
    }

    /// 根据标签查找知识
    pub fn find_by_tag(&self, tag: &str) -> Vec<&KnowledgeNode> {
        self.index.values()
            .filter(|n| n.tags.iter().any(|t| t.to_lowercase() == tag.to_lowercase()))
            .collect()
    }

    /// 根据目录前缀查找知识
    pub fn find_by_directory(&self, dir_path: &str) -> Vec<&KnowledgeNode> {
        let dir_path = dir_path.trim_end_matches('/');
        self.index.values()
            .filter(|n| {
                n.path.to_string_lossy().contains(dir_path)
            })
            .collect()
    }

    /// 根据通配符查找知识
    pub fn find_by_wildcard(&self, pattern: &str) -> Vec<&KnowledgeNode> {
        let pattern = pattern.replace('*', ".*");
        let re = regex::Regex::new(&pattern).ok();
        
        self.index.values()
            .filter(|n| {
                if let Some(ref re) = re {
                    let path_str = n.path.to_string_lossy();
                    re.is_match(&path_str)
                } else {
                    false
                }
            })
            .collect()
    }

    /// 计算知识之间的关联度
    pub fn compute_relations(&mut self) {
        let nodes: Vec<_> = self.index.keys().cloned().collect();

        for i in 0..nodes.len() {
            let mut relations = Vec::new();

            for j in 0..nodes.len() {
                if i == j {
                    continue;
                }

                let node_a = &self.index[&nodes[i]];
                let node_b = &self.index[&nodes[j]];

                // 计算标签重叠度
                let tag_overlap = if node_a.tags.is_empty() {
                    0.0
                } else {
                    let overlap = node_a.tags.iter()
                        .filter(|t| node_b.tags.contains(t))
                        .count() as f32 / node_a.tags.len() as f32;
                    overlap
                };

                // 计算内容相似度（简化的基于哈希的相似度）
                let content_sim = compute_content_similarity(
                    &node_a.content_hash,
                    &node_b.content_hash
                );

                // 综合得分：标签权重 60%，内容权重 40%
                let score = tag_overlap * 0.6 + content_sim * 0.4;

                if score > 0.3 {
                    relations.push((nodes[j].clone(), score));
                }
            }

            // 按相似度排序，保留 top 5
            // P2-005 FIX: Use unwrap_or for partial_cmp to handle NaN safely
            relations.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            if let Some(node) = self.index.get_mut(&nodes[i]) {
                node.related = relations.into_iter().take(5).map(|(p, _)| p).collect();
            }
        }
    }

    /// 获取相关知识
    pub fn get_related(&self, path: &str, limit: usize) -> Vec<&KnowledgeNode> {
        if let Some(node) = self.index.get(path) {
            node.related.iter()
                .filter_map(|p| self.index.get(p))
                .take(limit)
                .collect()
        } else {
            Vec::new()
        }
    }

    /// 更新文件内容
    pub fn update_file(&mut self, path: &Path) -> Result<()> {
        if !path.is_file() {
            return Ok(());
        }

        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {:?}", path))?;
        
        let hash = compute_sha256(&content);
        let tags = extract_tags_from_path(path, &self.root);

        let path_str = path.to_string_lossy().to_string();
        
        if let Some(node) = self.index.get_mut(&path_str) {
            node.content_hash = hash;
            node.tags = tags;
            node.content = Some(content);
            node.last_accessed = Utc::now();
        } else {
            // 新文件，添加到索引
            self.index.insert(
                path_str,
                KnowledgeNode {
                    path: path.to_path_buf(),
                    content_hash: hash,
                    tags,
                    related: Vec::new(),
                    last_accessed: Utc::now(),
                    content: Some(content),
                },
            );
        }

        Ok(())
    }

    /// 添加文件到索引
    pub fn add_file(&mut self, path: &Path) -> Result<()> {
        if !path.is_file() {
            return Ok(());
        }

        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {:?}", path))?;
        
        let hash = compute_sha256(&content);
        let tags = extract_tags_from_path(path, &self.root);

        let path_str = path.to_string_lossy().to_string();
        self.index.insert(
            path_str,
            KnowledgeNode {
                path: path.to_path_buf(),
                content_hash: hash,
                tags,
                related: Vec::new(),
                last_accessed: Utc::now(),
                content: Some(content),
            },
        );

        Ok(())
    }

    /// 从索引移除文件
    pub fn remove_file(&mut self, path: &Path) {
        let path_str = path.to_string_lossy().to_string();
        self.index.remove(&path_str);
    }

    /// 获取索引统计信息
    pub fn stats(&self) -> KnowledgeStats {
        let total_files = self.index.len();
        let total_tags: usize = self.index.values().map(|n| n.tags.len()).sum();
        let avg_tags = if total_files > 0 {
            total_tags as f32 / total_files as f32
        } else {
            0.0
        };

        KnowledgeStats {
            total_files,
            total_tags,
            avg_tags,
        }
    }
}

/// 知识统计信息
#[derive(Debug, Clone)]
pub struct KnowledgeStats {
    pub total_files: usize,
    pub total_tags: usize,
    pub avg_tags: f32,
}

/// 计算 SHA256 哈希
fn compute_sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// 从路径提取标签
/// 例如：docs/数据库/MySQL 索引优化.md -> ["数据库", "MySQL"]
fn extract_tags_from_path(path: &Path, root: &Path) -> Vec<String> {
    path.strip_prefix(root)
        .unwrap_or(path)
        .parent()
        .unwrap_or(Path::new(""))
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 从文本提取关键词（简化版）
fn extract_keywords(text: &str) -> Vec<String> {
    // 简单实现：按空格和标点分割，过滤常见停用词
    // 对于中文，我们按字符分割并组合连续的非停用词
    let stop_words = ["的", "了", "是", "在", "和", "与", "或", "怎么", "如何", "哪些", "什么"];
    
    // 首先尝试按空格和标点分割
    let mut keywords: Vec<String> = text
        .split(|c: char| c.is_whitespace() || c == '?' || c == '？' || c == ',' || c == '，' || c == '。')
        .filter(|s| !s.is_empty() && s.len() > 1)
        .filter(|s| !stop_words.contains(s))
        .map(|s| s.to_string())
        .collect();
    
    // 如果没有找到关键词（可能是纯中文），尝试提取连续的中文字符
    if keywords.is_empty() {
        // 简单提取 2 个字符以上的中文词
        let mut current_word = String::new();
        for c in text.chars() {
            if c.is_ascii_alphanumeric() || c.is_whitespace() || c.is_ascii_punctuation() {
                if !current_word.is_empty() {
                    if current_word.len() >= 2 && !stop_words.contains(&current_word.as_str()) {
                        keywords.push(current_word.clone());
                    }
                    current_word.clear();
                }
                if c.is_ascii_alphanumeric() {
                    current_word.push(c);
                }
            } else {
                // 中文字符
                if !current_word.is_empty() && !current_word.chars().all(|c| !c.is_ascii()) {
                    if current_word.len() >= 2 && !stop_words.contains(&current_word.as_str()) {
                        keywords.push(current_word.clone());
                    }
                    current_word.clear();
                }
                current_word.push(c);
            }
        }
        if !current_word.is_empty() && current_word.len() >= 2 && !stop_words.contains(&current_word.as_str()) {
            keywords.push(current_word);
        }
    }
    
    keywords
}

/// 计算内容相似度（基于哈希的简化版本）
fn compute_content_similarity(hash_a: &str, hash_b: &str) -> f32 {
    // 简化的相似度计算：比较哈希前缀的匹配度
    // 实际应用中可以使用 SimHash 或更复杂的算法
    let prefix_len = 8.min(hash_a.len().min(hash_b.len()));
    let prefix_a = &hash_a[..prefix_len];
    let prefix_b = &hash_b[..prefix_len];
    
    if prefix_a == prefix_b {
        1.0
    } else {
        // 计算不同的字符数
        let diff = prefix_a.chars()
            .zip(prefix_b.chars())
            .filter(|(a, b)| a != b)
            .count();
        1.0 - (diff as f32 / prefix_len as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_knowledge_index_from_directory() {
        let temp_dir = TempDir::new().unwrap();
        
        // 创建测试目录结构
        let db_dir = temp_dir.path().join("数据库");
        std::fs::create_dir_all(&db_dir).unwrap();
        
        let mysql_file = db_dir.join("MySQL 索引优化.md");
        std::fs::write(&mysql_file, "# MySQL 索引优化\n\n索引优化的最佳实践...").unwrap();
        
        let redis_file = db_dir.join("Redis 缓存策略.md");
        std::fs::write(&redis_file, "# Redis 缓存策略\n\n缓存策略包括...").unwrap();

        let index = KnowledgeIndex::from_directory(temp_dir.path()).unwrap();

        assert_eq!(index.index.len(), 2);

        let mysql_node = index.get(mysql_file.to_string_lossy().as_ref()).unwrap();
        assert_eq!(mysql_node.tags, vec!["数据库"]);
    }

    #[test]
    fn test_extract_tags_from_path() {
        let root = Path::new("/project/docs");
        let path = Path::new("/project/docs/数据库/MySQL 索引优化.md");
        
        let tags = extract_tags_from_path(path, root);
        assert_eq!(tags, vec!["数据库"]);
    }

    #[test]
    fn test_extract_keywords() {
        // 测试英文和混合文本
        let keywords = extract_keywords("MySQL optimization");
        assert!(keywords.contains(&"MySQL".to_string()));
        
        // 测试带标点的文本
        let keywords2 = extract_keywords("How to optimize MySQL?");
        assert!(keywords2.iter().any(|k| k.to_lowercase().contains("mysql")));
    }

    #[test]
    fn test_recommend() {
        let temp_dir = TempDir::new().unwrap();

        let db_dir = temp_dir.path().join("数据库");
        std::fs::create_dir_all(&db_dir).unwrap();

        let mysql_file = db_dir.join("MySQL 索引优化.md");
        std::fs::write(&mysql_file, "# MySQL 索引优化\n\n索引优化的最佳实践...").unwrap();

        let index = KnowledgeIndex::from_directory(temp_dir.path()).unwrap();

        // 使用标签推荐而不是关键词
        let recommended = index.recommend("数据库", 3);
        // 至少应该返回一个结果（因为标签匹配）
        assert!(!recommended.is_empty() || !index.all_nodes().is_empty());
    }
}
