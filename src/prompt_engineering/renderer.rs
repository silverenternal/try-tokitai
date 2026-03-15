//! 提示词渲染引擎
//!
//! 实现 Mustache 风格的变量替换和条件渲染

use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;

/// 提示词渲染器
pub struct PromptRenderer {
    /// 内置函数缓存
    functions: HashMap<String, Box<dyn Fn(&str) -> String + Send + Sync>>,
}

impl Default for PromptRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptRenderer {
    /// 创建新的渲染器
    pub fn new() -> Self {
        let mut functions: HashMap<String, Box<dyn Fn(&str) -> String + Send + Sync>> =
            HashMap::new();

        // 注册内置函数
        functions.insert(
            "uppercase".to_string(),
            Box::new(|s: &str| s.to_uppercase()),
        );
        functions.insert(
            "lowercase".to_string(),
            Box::new(|s: &str| s.to_lowercase()),
        );
        functions.insert(
            "trim".to_string(),
            Box::new(|s: &str| s.trim().to_string()),
        );

        Self { functions }
    }

    /// 渲染提示词模板
    ///
    /// 支持以下语法：
    /// - {{variable}} - 变量替换
    /// - {{#if condition}}...{{/if}} - 条件渲染
    /// - {{#each items}}...{{/each}} - 循环渲染
    /// - {{function variable}} - 内置函数
    pub fn render(&self, template: &str, variables: &Value) -> Result<String> {
        let mut result = template.to_string();

        // 1. 处理条件渲染 {{#if condition}}...{{/if}}
        result = self.render_conditionals(&result, variables)?;

        // 2. 处理循环渲染 {{#each items}}...{{/each}}
        result = self.render_loops(&result, variables)?;

        // 3. 处理函数调用 {{function variable}}
        result = self.render_functions(&result, variables)?;

        // 4. 处理普通变量替换 {{variable}}
        result = self.render_variables(&result, variables)?;

        Ok(result)
    }

    /// 渲染条件语句
    fn render_conditionals(&self, template: &str, variables: &Value) -> Result<String> {
        let mut result = template.to_string();

        // 正则匹配 {{#if condition}}...{{/if}}
        let if_pattern = regex::Regex::new(r"\{\{#if\s+(\w+)\}\}(.*?)\{\{/if\}\}")?;

        while let Some(caps) = if_pattern.captures(&result) {
            let full_match = caps.get(0).unwrap().as_str();
            let condition = caps.get(1).unwrap().as_str();
            let content = caps.get(2).unwrap().as_str();

            // 检查条件是否为真
            let is_true = self.evaluate_condition(variables, condition);

            // 替换为内容或空字符串
            let replacement = if is_true { content } else { "" };
            result = result.replacen(full_match, replacement, 1);
        }

        Ok(result)
    }

    /// 渲染循环语句
    fn render_loops(&self, template: &str, variables: &Value) -> Result<String> {
        let mut result = template.to_string();

        // 正则匹配 {{#each items}}...{{/each}}，使用 (?s) 启用 dotall 模式
        let each_pattern = regex::Regex::new(r"(?s)\{\{#each\s+(\w+)\}\}(.*?)\{\{/each\}\}")?;

        while let Some(caps) = each_pattern.captures(&result) {
            let full_match = caps.get(0).unwrap().as_str();
            let array_name = caps.get(1).unwrap().as_str();
            let template_content = caps.get(2).unwrap().as_str();

            // 获取数组
            let empty_array: Vec<Value> = vec![];
            let array = variables
                .get(array_name)
                .and_then(|v| v.as_array())
                .unwrap_or(&empty_array);

            // 渲染每个元素
            let mut rendered_items = Vec::new();
            for item in array {
                let mut item_template = template_content.to_string();

                // 替换 {{this}} 或 {{.}} 为当前元素
                if let Some(s) = item.as_str() {
                    item_template = item_template.replace("{{this}}", s);
                    item_template = item_template.replace("{{.}}", s);
                }

                // 替换对象字段 {{name}}, {{value}} 等
                if let Some(obj) = item.as_object() {
                    for (key, val) in obj {
                        let placeholder = format!("{{{{{}}}}}", key);
                        let value_str = match val {
                            Value::String(s) => s.clone(),
                            Value::Number(n) => n.to_string(),
                            Value::Bool(b) => b.to_string(),
                            _ => val.to_string(),
                        };
                        item_template = item_template.replace(&placeholder, &value_str);
                    }
                }

                rendered_items.push(item_template);
            }

            result = result.replacen(full_match, &rendered_items.join("\n"), 1);
        }

        Ok(result)
    }

    /// 渲染函数调用
    fn render_functions(&self, template: &str, variables: &Value) -> Result<String> {
        let mut result = template.to_string();

        // 正则匹配 {{function variable}}
        let func_pattern = regex::Regex::new(r"\{\{(\w+)\s+(\w+)\}\}")?;

        while let Some(caps) = func_pattern.captures(&result) {
            let full_match = caps.get(0).unwrap().as_str();
            let func_name = caps.get(1).unwrap().as_str();
            let var_name = caps.get(2).unwrap().as_str();

            // 获取变量值
            let value = variables
                .get(var_name)
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // 调用函数
            let rendered = if let Some(func) = self.functions.get(func_name) {
                func(value)
            } else {
                value.to_string()
            };

            result = result.replacen(full_match, &rendered, 1);
        }

        Ok(result)
    }

    /// 渲染普通变量
    fn render_variables(&self, template: &str, variables: &Value) -> Result<String> {
        let mut result = template.to_string();

        // 正则匹配 {{variable}}
        let var_pattern = regex::Regex::new(r"\{\{(\w+)\}\}")?;

        // 收集所有匹配，避免借用冲突
        let matches: Vec<_> = var_pattern
            .captures_iter(&result)
            .filter_map(|caps| {
                let full_match = caps.get(0).unwrap().as_str().to_string();
                let var_name = caps.get(1).unwrap().as_str().to_string();
                Some((full_match, var_name))
            })
            .collect();

        for (full_match, var_name) in matches {
            // 跳过已处理的（可能是条件或循环的一部分）
            if var_name.starts_with('#') || var_name.starts_with('/') {
                continue;
            }

            let value = variables
                .get(&var_name)
                .map(|v| match v {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Array(arr) => arr
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                    _ => v.to_string(),
                })
                .unwrap_or_default();

            result = result.replace(&full_match, &value);
        }

        Ok(result)
    }

    /// 评估条件
    fn evaluate_condition(&self, variables: &Value, condition: &str) -> bool {
        // 简单的条件评估：检查变量是否存在且为真
        variables
            .get(condition)
            .map(|v| v.as_bool().unwrap_or(true))
            .unwrap_or(false)
    }

    /// 注册自定义函数
    pub fn register_function<F>(&mut self, name: &str, func: F)
    where
        F: Fn(&str) -> String + Send + Sync + 'static,
    {
        self.functions
            .insert(name.to_string(), Box::new(func));
    }

    /// 截断文本到指定长度
    pub fn truncate(text: &str, max_len: usize) -> String {
        if text.len() <= max_len {
            text.to_string()
        } else {
            format!("{}...", &text[..max_len])
        }
    }
}

/// 便捷渲染函数
pub fn render_prompt(template: &str, variables: &Value) -> Result<String> {
    let renderer = PromptRenderer::new();
    renderer.render(template, variables)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_variable_replacement() {
        let template = "Hello, {{name}}!";
        let variables = json!({"name": "World"});

        let result = render_prompt(template, &variables).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_conditional_rendering() {
        let template = "{{#if show}}Visible{{/if}}";
        let variables = json!({"show": true});

        let result = render_prompt(template, &variables).unwrap();
        assert_eq!(result, "Visible");

        let variables_false = json!({"show": false});
        let result_false = render_prompt(template, &variables_false).unwrap();
        assert_eq!(result_false, "");
    }

    #[test]
    fn test_loop_rendering() {
        let template = "{{#each items}}- {{this}}\n{{/each}}";
        let variables = json!({"items": ["apple", "banana", "cherry"]});

        let result = render_prompt(template, &variables).unwrap();
        assert!(result.contains("- apple"));
        assert!(result.contains("- banana"));
        assert!(result.contains("- cherry"));
    }

    #[test]
    fn test_function_rendering() {
        let template = "{{uppercase name}}";
        let variables = json!({"name": "hello"});

        let result = render_prompt(template, &variables).unwrap();
        assert_eq!(result, "HELLO");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(PromptRenderer::truncate("hello", 10), "hello");
        assert_eq!(PromptRenderer::truncate("hello world", 5), "hello...");
    }
}
