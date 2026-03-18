//! 数据工具统一配置
//!
//! 使用 builder pattern 支持灵活配置

use serde::{Serialize, Deserialize};

/// 数据工具统一配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataToolConfig {
    /// 最大 JSON 长度 (bytes)
    pub max_length: usize,
    /// 最大 JSON 深度
    pub max_depth: usize,
    /// 最大路径长度
    pub max_path_length: usize,
    /// 最大合并数量
    pub max_merge_count: usize,
    /// 最大数组项数（转换时）
    pub max_items: usize,
    /// 格式化缩进空格数
    pub indent: usize,
}

impl Default for DataToolConfig {
    fn default() -> Self {
        Self {
            max_length: 10 * 1024 * 1024, // 10MB
            max_depth: 100,
            max_path_length: 4096,
            max_merge_count: 100,
            max_items: 10000,
            indent: 2,
        }
    }
}

impl DataToolConfig {
    /// 创建配置构建器
    pub fn builder() -> DataToolConfigBuilder {
        DataToolConfigBuilder::new()
    }

    /// 创建默认配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建精简配置（适用于资源受限场景）
    pub fn minimal() -> Self {
        Self {
            max_length: 1024 * 1024, // 1MB
            max_depth: 50,
            max_path_length: 1024,
            max_merge_count: 20,
            max_items: 1000,
            indent: 2,
        }
    }

    /// 创建宽松配置（适用于批处理场景）
    pub fn permissive() -> Self {
        Self {
            max_length: 100 * 1024 * 1024, // 100MB
            max_depth: 200,
            max_path_length: 8192,
            max_merge_count: 500,
            max_items: 100000,
            indent: 4,
        }
    }
}

/// 配置构建器
#[derive(Debug, Clone, Default)]
pub struct DataToolConfigBuilder {
    config: DataToolConfig,
}

impl DataToolConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: DataToolConfig::default(),
        }
    }

    pub fn max_length(mut self, value: usize) -> Self {
        self.config.max_length = value;
        self
    }

    pub fn max_depth(mut self, value: usize) -> Self {
        self.config.max_depth = value;
        self
    }

    pub fn max_path_length(mut self, value: usize) -> Self {
        self.config.max_path_length = value;
        self
    }

    pub fn max_merge_count(mut self, value: usize) -> Self {
        self.config.max_merge_count = value;
        self
    }

    pub fn max_items(mut self, value: usize) -> Self {
        self.config.max_items = value;
        self
    }

    pub fn indent(mut self, value: usize) -> Self {
        self.config.indent = value;
        self
    }

    pub fn build(self) -> DataToolConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DataToolConfig::default();
        assert_eq!(config.max_length, 10 * 1024 * 1024);
        assert_eq!(config.max_depth, 100);
        assert_eq!(config.indent, 2);
    }

    #[test]
    fn test_minimal_config() {
        let config = DataToolConfig::minimal();
        assert_eq!(config.max_length, 1024 * 1024);
        assert_eq!(config.max_depth, 50);
        assert!(config.max_merge_count < 50);
    }

    #[test]
    fn test_permissive_config() {
        let config = DataToolConfig::permissive();
        assert_eq!(config.max_length, 100 * 1024 * 1024);
        assert_eq!(config.max_depth, 200);
        assert!(config.max_merge_count > 100);
    }

    #[test]
    fn test_builder() {
        let config = DataToolConfig::builder()
            .max_length(5 * 1024 * 1024)
            .max_depth(80)
            .indent(4)
            .build();

        assert_eq!(config.max_length, 5 * 1024 * 1024);
        assert_eq!(config.max_depth, 80);
        assert_eq!(config.indent, 4);
    }
}
