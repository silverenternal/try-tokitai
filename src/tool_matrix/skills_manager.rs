//! Skills 文件管理器
//!
//! 管理工具箱的使用说明书（Skills 文件）
//! - 加载/保存 Skills 文件
//! - 生成 AI 可读的 Skills 提示词
//! - 支持运行时更新 Skills 文件

#![allow(dead_code)]

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;

use crate::tool_matrix::matrix::{
    SkillsFile, ToolGuide, UseCase, SkillExample,
};

/// Skills 文件管理器
pub struct SkillsManager {
    /// Skills 文件存储目录
    skills_dir: PathBuf,
    /// Skills 文件缓存
    cache: Arc<RwLock<Vec<SkillsFile>>>,
}

impl Default for SkillsManager {
    fn default() -> Self {
        Self::new(".context/tool_matrix/skills".to_string())
    }
}

impl SkillsManager {
    /// 创建新的 Skills 管理器
    pub fn new(skills_dir: String) -> Self {
        Self {
            skills_dir: PathBuf::from(skills_dir),
            cache: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 创建带自定义目录的 Skills 管理器
    pub fn with_path<P: AsRef<Path>>(skills_dir: P) -> Result<Self> {
        let path = skills_dir.as_ref().to_path_buf();

        // 确保目录存在
        if !path.exists() {
            fs::create_dir_all(&path)
                .with_context(|| format!("Failed to create skills directory: {:?}", path))?;
        }

        Ok(Self {
            skills_dir: path,
            cache: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// 加载所有 Skills 文件
    pub fn load_all(&self) -> Result<Vec<String>> {
        let mut loaded = Vec::new();

        if !self.skills_dir.exists() {
            return Ok(loaded);
        }

        for entry in fs::read_dir(&self.skills_dir)
            .with_context(|| format!("Failed to read skills directory: {:?}", self.skills_dir))?
        {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match self.load_skills_file(&path) {
                    Ok(skills) => {
                        loaded.push(skills.name.clone());
                        self.cache.write().push(skills);
                    }
                    Err(e) => {
                        tracing::warn!("加载 Skills 文件 {:?} 失败：{}", path, e);
                    }
                }
            }
        }

        Ok(loaded)
    }

    /// 加载单个 Skills 文件
    pub fn load_skills_file(&self, path: &Path) -> Result<SkillsFile> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read skills file: {:?}", path))?;

        let skills: SkillsFile = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse skills file: {:?}", path))?;

        Ok(skills)
    }

    /// 保存 Skills 文件
    pub fn save_skills_file(&self, skills: &SkillsFile) -> Result<PathBuf> {
        let file_path = self
            .skills_dir
            .join(format!("{}.json", skills.toolbox_id));

        // 确保目录存在
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }

        let content = serde_json::to_string_pretty(skills)
            .with_context(|| "Failed to serialize skills file")?;

        fs::write(&file_path, content)
            .with_context(|| format!("Failed to write skills file: {:?}", file_path))?;

        // 更新缓存
        let mut cache = self.cache.write();
        if let Some(pos) = cache.iter().position(|s| s.toolbox_id == skills.toolbox_id) {
            cache[pos] = skills.clone();
        } else {
            cache.push(skills.clone());
        }

        Ok(file_path)
    }

    /// 创建新的 Skills 文件
    pub fn create_skills_file(
        &self,
        toolbox_id: &str,
        name: &str,
        introduction: &str,
    ) -> SkillsFile {
        SkillsFile::new(toolbox_id, name, introduction, "1.0.0")
    }

    /// 添加工具指南到 Skills 文件
    pub fn add_tool_guide(
        &self,
        toolbox_id: &str,
        guide: ToolGuide,
    ) -> Result<()> {
        // 查找现有的 Skills 文件
        let mut cache = self.cache.write();
        
        if let Some(pos) = cache.iter().position(|s| s.toolbox_id == toolbox_id) {
            cache[pos].add_tool_guide(guide);
            let skills = cache[pos].clone();
            drop(cache);
            self.save_skills_file(&skills)?;
            Ok(())
        } else {
            anyhow::bail!("Skills 文件 {} 不存在", toolbox_id);
        }
    }

    /// 添加使用场景到 Skills 文件
    pub fn add_use_case(&self, toolbox_id: &str, use_case: UseCase) -> Result<()> {
        let mut cache = self.cache.write();
        
        if let Some(pos) = cache.iter().position(|s| s.toolbox_id == toolbox_id) {
            cache[pos].add_use_case(use_case);
            let skills = cache[pos].clone();
            drop(cache);
            self.save_skills_file(&skills)?;
            Ok(())
        } else {
            anyhow::bail!("Skills 文件 {} 不存在", toolbox_id);
        }
    }

    /// 添加示例到 Skills 文件
    pub fn add_example(&self, toolbox_id: &str, example: SkillExample) -> Result<()> {
        let mut cache = self.cache.write();
        
        if let Some(pos) = cache.iter().position(|s| s.toolbox_id == toolbox_id) {
            cache[pos].add_example(example);
            let skills = cache[pos].clone();
            drop(cache);
            self.save_skills_file(&skills)?;
            Ok(())
        } else {
            anyhow::bail!("Skills 文件 {} 不存在", toolbox_id);
        }
    }

    /// 获取 Skills 文件
    pub fn get_skills(&self, toolbox_id: &str) -> Option<SkillsFile> {
        self.cache
            .read()
            .iter()
            .find(|s| s.toolbox_id == toolbox_id)
            .cloned()
    }

    /// 获取所有 Skills 文件
    pub fn get_all_skills(&self) -> Vec<SkillsFile> {
        self.cache.read().clone()
    }

    /// 生成 AI 可读的 Skills 提示词（合并所有 Skills）
    pub fn generate_skills_prompt(&self) -> Result<String> {
        let cache = self.cache.read();
        
        if cache.is_empty() {
            return Ok("".to_string());
        }

        let mut prompt = String::new();
        prompt.push_str("# 工具箱使用指南 (Skills)\n\n");
        prompt.push_str("以下是你可以使用的工具箱及其使用说明：\n\n");

        for skills in cache.iter() {
            prompt.push_str(&skills.to_prompt());
            prompt.push_str("\n---\n\n");
        }

        Ok(prompt)
    }

    /// 生成单个工具箱的 Skills 提示词
    pub fn generate_toolbox_skills(&self, toolbox_id: &str) -> Option<String> {
        self.get_skills(toolbox_id).map(|s| s.to_prompt())
    }

    /// 检查 Skills 文件是否存在
    pub fn skills_exists(&self, toolbox_id: &str) -> bool {
        self.cache.read().iter().any(|s| s.toolbox_id == toolbox_id)
    }

    /// 清除缓存
    pub fn clear_cache(&self) {
        self.cache.write().clear();
    }

    /// 重新加载 Skills 文件
    pub fn reload(&self) -> Result<Vec<String>> {
        self.clear_cache();
        self.load_all()
    }
}

/// 便捷函数：创建 Skills 文件管理器并加载所有文件
pub fn load_skills_manager() -> Result<SkillsManager> {
    let manager = SkillsManager::default();
    manager.load_all()?;
    Ok(manager)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_skills_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SkillsManager::with_path(temp_dir.path()).unwrap();

        assert!(manager.skills_dir.exists());
    }

    #[test]
    fn test_save_and_load_skills() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SkillsManager::with_path(temp_dir.path()).unwrap();

        // 创建 Skills 文件
        let mut skills = SkillsFile::new(
            "test_box",
            "Test Skills",
            "Introduction to test skills",
            "1.0.0",
        );

        skills.add_tool_guide(ToolGuide {
            tool_name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            examples: vec!["Example 1".to_string()],
            parameters: vec![],
            returns: None,
            notes: vec!["Be careful".to_string()],
        });

        // 保存
        let path = manager.save_skills_file(&skills).unwrap();
        assert!(path.exists());

        // 重新加载
        let loaded = manager.load_skills_file(&path).unwrap();
        assert_eq!(loaded.toolbox_id, skills.toolbox_id);
        assert_eq!(loaded.tool_guides.len(), 1);
    }

    #[test]
    fn test_generate_skills_prompt() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SkillsManager::with_path(temp_dir.path()).unwrap();

        let skills = SkillsFile::new(
            "test_box",
            "Test Skills",
            "Introduction",
            "1.0.0",
        );

        manager.save_skills_file(&skills).unwrap();
        manager.load_all().unwrap();

        let prompt = manager.generate_skills_prompt().unwrap();
        assert!(prompt.contains("Test Skills"));
        assert!(prompt.contains("Introduction"));
    }
}
