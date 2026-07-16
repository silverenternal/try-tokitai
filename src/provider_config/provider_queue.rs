//! 供应商循环队列
//!
//! 实现循环队列结构用于供应商切换

#![allow(dead_code)]

use super::ProviderConfig;
use std::collections::VecDeque;

/// 供应商循环队列
pub struct ProviderQueue {
    queue: VecDeque<ProviderConfig>,
    current_index: usize,
}

impl ProviderQueue {
    /// 创建新的循环队列
    pub fn new(providers: Vec<ProviderConfig>) -> Self {
        let queue: VecDeque<ProviderConfig> = providers.into();
        Self {
            queue,
            current_index: 0,
        }
    }

    /// 获取当前供应商
    pub fn current(&self) -> Option<&ProviderConfig> {
        self.queue.get(self.current_index)
    }

    /// 切换到下一个供应商（循环）
    pub fn next(&mut self) -> Option<&ProviderConfig> {
        if self.queue.is_empty() {
            return None;
        }

        self.current_index = (self.current_index + 1) % self.queue.len();
        self.current()
    }

    /// 切换到上一个供应商（循环）
    pub fn previous(&mut self) -> Option<&ProviderConfig> {
        if self.queue.is_empty() {
            return None;
        }

        if self.current_index == 0 {
            self.current_index = self.queue.len() - 1;
        } else {
            self.current_index -= 1;
        }
        self.current()
    }

    /// 获取供应商数量
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// 获取所有供应商
    pub fn all_providers(&self) -> Vec<&ProviderConfig> {
        self.queue.iter().collect()
    }

    /// 切换到指定索引的供应商
    pub fn switch_to(&mut self, index: usize) -> Option<&ProviderConfig> {
        if index >= self.queue.len() {
            return None;
        }

        self.current_index = index;
        self.current()
    }

    /// 获取当前索引
    pub fn current_index(&self) -> usize {
        self.current_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_providers() -> Vec<ProviderConfig> {
        vec![
            ProviderConfig {
                name: "ollama".to_string(),
                api_url: "https://ollama.com".to_string(),
                api_key: Some("key1".to_string()),
                model: "qwen".to_string(),
            },
            ProviderConfig {
                name: "openai".to_string(),
                api_url: "https://openai.com".to_string(),
                api_key: Some("key2".to_string()),
                model: "gpt-4".to_string(),
            },
            ProviderConfig {
                name: "anthropic".to_string(),
                api_url: "https://anthropic.com".to_string(),
                api_key: Some("key3".to_string()),
                model: "claude".to_string(),
            },
        ]
    }

    #[test]
    fn test_next_circular() {
        let mut queue = ProviderQueue::new(create_test_providers());

        assert_eq!(queue.current().unwrap().name, "ollama");

        let next = queue.next().unwrap();
        assert_eq!(next.name, "openai");

        let next = queue.next().unwrap();
        assert_eq!(next.name, "anthropic");

        // 循环回到第一个
        let next = queue.next().unwrap();
        assert_eq!(next.name, "ollama");
    }

    #[test]
    fn test_previous_circular() {
        let mut queue = ProviderQueue::new(create_test_providers());

        assert_eq!(queue.current().unwrap().name, "ollama");

        let prev = queue.previous().unwrap();
        assert_eq!(prev.name, "anthropic");

        let prev = queue.previous().unwrap();
        assert_eq!(prev.name, "openai");
    }

    #[test]
    fn test_switch_to_index() {
        let mut queue = ProviderQueue::new(create_test_providers());

        queue.switch_to(2);
        assert_eq!(queue.current().unwrap().name, "anthropic");

        queue.switch_to(1);
        assert_eq!(queue.current().unwrap().name, "openai");
    }
}
