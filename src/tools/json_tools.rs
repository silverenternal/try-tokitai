use tokitai::tool;
use serde_json::{json, Value};

/// JSON 处理工具集
/// 提供 JSON 解析、格式化、查询等功能
pub struct JsonTools;

// 递归深度限制，防止栈溢出
const MAX_JSON_DEPTH: usize = 100;

#[tool]
impl JsonTools {
    /// 格式化 JSON 字符串
    /// 将压缩的 JSON 格式化为易读的格式
    pub fn format_json(&self, json_string: String) -> Result<String, String> {
        validate_json_length(&json_string)?;
        
        let parsed: Value = serde_json::from_str(&json_string)
            .map_err(|e| format!("JSON 解析失败：{}", e))?;
        
        check_json_depth(&parsed, 0, MAX_JSON_DEPTH)?;

        let formatted = serde_json::to_string_pretty(&parsed)
            .map_err(|e| format!("JSON 格式化失败：{}", e))?;

        Ok(formatted)
    }

    /// 压缩 JSON 字符串
    /// 移除 JSON 中的所有空白字符
    pub fn minify_json(&self, json_string: String) -> Result<String, String> {
        validate_json_length(&json_string)?;
        
        let parsed: Value = serde_json::from_str(&json_string)
            .map_err(|e| format!("JSON 解析失败：{}", e))?;

        let minified = serde_json::to_string(&parsed)
            .map_err(|e| format!("JSON 压缩失败：{}", e))?;

        Ok(minified)
    }

    /// 查询 JSON 数据（支持 JSONPath 风格的路径）
    /// 使用点号分隔的路径查询，如 "user.name" 或 "data.0.id"
    pub fn query_json(&self, json_string: String, path: String) -> Result<Value, String> {
        validate_json_length(&json_string)?;
        validate_path_length(&path)?;
        
        let parsed: Value = serde_json::from_str(&json_string)
            .map_err(|e| format!("JSON 解析失败：{}", e))?;

        let result = navigate_json(&parsed, &path, 0, MAX_JSON_DEPTH)
            .ok_or_else(|| format!("路径 '{}' 不存在", path))?;

        Ok(result.clone())
    }

    /// 从 JSON 中提取所有键
    /// 递归提取 JSON 中的所有键名
    pub fn extract_keys(&self, json_string: String) -> Result<Value, String> {
        validate_json_length(&json_string)?;
        
        let parsed: Value = serde_json::from_str(&json_string)
            .map_err(|e| format!("JSON 解析失败：{}", e))?;

        let mut keys = Vec::new();
        collect_keys(&parsed, &mut keys, 0, MAX_JSON_DEPTH);

        Ok(json!({
            "keys": keys,
            "total": keys.len()
        }))
    }

    /// 验证 JSON 格式
    /// 检查字符串是否是有效的 JSON 格式
    pub fn validate_json(&self, json_string: String) -> Result<Value, String> {
        validate_json_length(&json_string)?;
        
        match serde_json::from_str::<Value>(&json_string) {
            Ok(_) => Ok(json!({
                "valid": true,
                "message": "JSON 格式有效"
            })),
            Err(e) => Ok(json!({
                "valid": false,
                "error": e.to_string()
            })),
        }
    }

    /// 合并多个 JSON 对象
    /// 将多个 JSON 对象合并为一个
    pub fn merge_json(&self, json_objects: Vec<String>) -> Result<Value, String> {
        const MAX_MERGE_COUNT: usize = 100;
        
        if json_objects.len() > MAX_MERGE_COUNT {
            return Err(format!(
                "JSON 对象数量过多 ({} > {})",
                json_objects.len(),
                MAX_MERGE_COUNT
            ));
        }
        
        let mut merged = serde_json::Map::new();

        for (index, json_string) in json_objects.iter().enumerate() {
            validate_json_length(json_string)?;
            
            let parsed: Value = serde_json::from_str(json_string)
                .map_err(|e| format!("第 {} 个 JSON 解析失败：{}", index + 1, e))?;

            if let Some(obj) = parsed.as_object() {
                for (key, value) in obj {
                    merged.insert(key.clone(), value.clone());
                }
            } else {
                return Err(format!("第 {} 个输入不是 JSON 对象", index + 1));
            }
        }

        Ok(Value::Object(merged))
    }

    /// 将 JSON 转换为 CSV 格式
    /// 适用于 JSON 数组转换为表格数据
    pub fn json_to_csv(&self, json_string: String) -> Result<String, String> {
        validate_json_length(&json_string)?;
        
        let parsed: Value = serde_json::from_str(&json_string)
            .map_err(|e| format!("JSON 解析失败：{}", e))?;

        let array = parsed.as_array()
            .ok_or_else(|| "输入必须是 JSON 数组".to_string())?;

        if array.is_empty() {
            return Ok("".to_string());
        }

        // 收集所有键
        let mut headers = Vec::new();
        for item in array {
            if let Some(obj) = item.as_object() {
                for key in obj.keys() {
                    if !headers.contains(key) {
                        headers.push(key.clone());
                    }
                }
            }
        }

        // 构建 CSV
        let mut csv = String::new();

        // 添加表头（使用 CSV 转义）
        csv.push_str(&headers.iter()
            .map(|h| escape_csv_field(h))
            .collect::<Vec<_>>()
            .join(","));
        csv.push('\n');

        // 添加数据行（使用 CSV 转义）
        for item in array {
            if let Some(obj) = item.as_object() {
                let row: Vec<String> = headers.iter()
                    .map(|key| {
                        obj.get(key)
                            .map(|v| match v {
                                Value::String(s) => escape_csv_field(s),
                                Value::Number(n) => n.to_string(),
                                Value::Bool(b) => b.to_string(),
                                Value::Null => "".to_string(),
                                _ => escape_csv_field(&v.to_string()),
                            })
                            .unwrap_or_else(|| "".to_string())
                    })
                    .collect();
                csv.push_str(&row.join(","));
                csv.push('\n');
            }
        }

        Ok(csv)
    }
}

/// 验证 JSON 字符串长度
fn validate_json_length(json_string: &str) -> Result<(), String> {
    const MAX_JSON_LENGTH: usize = 10 * 1024 * 1024; // 10MB
    
    if json_string.len() > MAX_JSON_LENGTH {
        return Err(format!(
            "JSON 字符串过长 ({} > {} bytes)",
            json_string.len(),
            MAX_JSON_LENGTH
        ));
    }
    Ok(())
}

/// 验证路径长度
fn validate_path_length(path: &str) -> Result<(), String> {
    const MAX_PATH_LENGTH: usize = 4096;
    
    if path.len() > MAX_PATH_LENGTH {
        return Err(format!(
            "路径过长 ({} > {} 字符)",
            path.len(),
            MAX_PATH_LENGTH
        ));
    }
    Ok(())
}

/// 检查 JSON 深度
fn check_json_depth(value: &Value, current_depth: usize, max_depth: usize) -> Result<(), String> {
    if current_depth > max_depth {
        return Err(format!("JSON 嵌套过深 ({} > {})", current_depth, max_depth));
    }
    
    match value {
        Value::Object(obj) => {
            for val in obj.values() {
                check_json_depth(val, current_depth + 1, max_depth)?;
            }
        }
        Value::Array(arr) => {
            for item in arr {
                check_json_depth(item, current_depth + 1, max_depth)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// 在 JSON 中导航（带深度限制）
fn navigate_json<'a>(value: &'a Value, path: &str, current_depth: usize, max_depth: usize) -> Option<&'a Value> {
    if current_depth > max_depth {
        return None;
    }
    
    let mut current = value;

    for part in path.split('.') {
        current = if let Some(index) = part.parse::<usize>().ok() {
            current.as_array().and_then(|arr| arr.get(index))?
        } else {
            current.as_object().and_then(|obj| obj.get(part))?
        };
    }

    Some(current)
}

/// 递归收集所有键（带深度限制）
fn collect_keys(value: &Value, keys: &mut Vec<String>, current_depth: usize, max_depth: usize) {
    if current_depth > max_depth {
        return;
    }
    
    match value {
        Value::Object(obj) => {
            for (key, val) in obj {
                keys.push(key.clone());
                collect_keys(val, keys, current_depth + 1, max_depth);
            }
        }
        Value::Array(arr) => {
            for item in arr {
                collect_keys(item, keys, current_depth + 1, max_depth);
            }
        }
        _ => {}
    }
}

/// CSV 字段转义（RFC 4180 兼容）
fn escape_csv_field(s: &str) -> String {
    // 如果字段包含逗号、双引号、换行符或回车符，需要用双引号包裹
    if s.contains(|c| c == ',' || c == '"' || c == '\n' || c == '\r') {
        // 双引号需要转义为两个双引号
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_csv_field() {
        // 普通字段不需要转义
        assert_eq!(escape_csv_field("hello"), "hello");
        assert_eq!(escape_csv_field("123"), "123");
        
        // 含逗号需要转义
        assert_eq!(escape_csv_field("hello,world"), "\"hello,world\"");
        
        // 含双引号需要转义
        assert_eq!(escape_csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
        
        // 含换行需要转义
        assert_eq!(escape_csv_field("line1\nline2"), "\"line1\nline2\"");
        
        // 含回车需要转义
        assert_eq!(escape_csv_field("a\rb"), "\"a\rb\"");
    }

    #[test]
    fn test_json_depth_limit() {
        // 创建一个有效但深度嵌套的 JSON
        // 使用数组嵌套而不是对象嵌套，这样可以成功解析
        let mut deep_json = String::from("1");
        for _ in 0..110 {
            deep_json = format!("[{}]", deep_json);
        }

        let tools = JsonTools;
        // 应该返回深度超限错误
        let result = tools.format_json(deep_json);
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("嵌套过深") || err_msg.contains("解析失败"));
    }

    #[test]
    fn test_json_to_csv_with_special_chars() {
        let tools = JsonTools;
        let json = r#"[{"name": "John, Jr.", "quote": "He said \"Hello\""}, {"name": "Jane\nDoe", "quote": "Hi"}]"#;
        
        let result = tools.json_to_csv(json.to_string()).unwrap();
        
        // 验证 CSV 正确转义
        assert!(result.contains("\"John, Jr.\""));
        assert!(result.contains("\"He said \"\"Hello\"\"\""));
        assert!(result.contains("\"Jane\nDoe\""));
    }

    #[test]
    fn test_validate_json_length() {
        let long_json = "{\"data\": \"".to_string() + &"a".repeat(11 * 1024 * 1024) + "\"}";
        assert!(validate_json_length(&long_json).is_err());
        
        let short_json = "{\"key\": \"value\"}";
        assert!(validate_json_length(short_json).is_ok());
    }

    #[test]
    fn test_query_json() {
        let tools = JsonTools;
        let json = r#"{"user": {"name": "Alice", "age": 30, "address": {"city": "Beijing"}}}"#;
        
        let result = tools.query_json(json.to_string(), "user.name".to_string()).unwrap();
        assert_eq!(result, "Alice");
        
        let result = tools.query_json(json.to_string(), "user.address.city".to_string()).unwrap();
        assert_eq!(result, "Beijing");
        
        let result = tools.query_json(json.to_string(), "user.nonexistent".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_merge_json() {
        let tools = JsonTools;
        let json1 = r#"{"a": 1, "b": 2}"#;
        let json2 = r#"{"c": 3, "d": 4}"#;
        
        let result = tools.merge_json(vec![json1.to_string(), json2.to_string()]).unwrap();
        
        assert_eq!(result.get("a").unwrap(), 1);
        assert_eq!(result.get("b").unwrap(), 2);
        assert_eq!(result.get("c").unwrap(), 3);
        assert_eq!(result.get("d").unwrap(), 4);
    }
}
