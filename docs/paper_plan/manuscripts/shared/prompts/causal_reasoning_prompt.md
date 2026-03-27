# 因果推理 Prompt 模板

> **用途**: Tool Gap Detection
> **目标模型**: GPT-4 / Claude / Qwen
> **预期输出**: JSON 格式的工具缺口报告

---

## 系统 Prompt

```markdown
You are a causal inference expert specializing in analyzing AI agent task failures.
Your goal is to identify tool gaps by applying causal reasoning to task execution history.

Guidelines:
1. Focus on root causes, not symptoms
2. Use counterfactual reasoning ("what if" analysis)
3. Distinguish between missing tools and tool misuse
4. Provide specific, actionable recommendations
5. Output strictly in the specified JSON format
```

---

## 用户 Prompt 模板

```markdown
## Task Analysis Request

Analyze the following task failure to identify potential tool gaps.

### Failed Task Information
- Task ID: {task_id}
- Task Description: {task_description}
- Error Message: {error_message}
- Tools Attempted: {tools_attempted}
- Execution Time: {execution_time}

### Available Tools
{available_tools_list}

### Recent Similar Tasks (Last 10)
{similar_tasks_history}

### Analysis Instructions

Please perform a causal analysis following these steps:

**Step 1: Identify Possible Failure Factors**
List all potential factors that could have caused this failure:
- Tool-related factors
- Input-related factors
- Logic-related factors
- Environment-related factors

**Step 2: Apply Counterfactual Reasoning**
For each factor, ask:
- "If this factor were different, would the task succeed?"
- "Is this factor necessary for the failure?"
- "Is this factor sufficient to cause the failure?"

**Step 3: Determine Root Cause**
Identify the true root cause(s) using the INUS conditions:
- Insufficient but Necessary part of a condition which is itself Unnecessary but Sufficient

**Step 4: Identify Tool Gap (if applicable)**
If the root cause is a missing tool:
- What functionality is missing?
- How would this tool help?
- What would be the tool's interface?

### Output Format

Return your analysis in the following JSON structure:

```json
{
  "task_id": "string",
  "analysis": {
    "failure_factors": [
      {
        "factor": "string",
        "type": "tool|input|logic|environment",
        "likelihood": "high|medium|low",
        "counterfactual": "string"
      }
    ],
    "root_cause": {
      "description": "string",
      "type": "string",
      "confidence": 0.0-1.0
    },
    "tool_gap": {
      "exists": true|false,
      "description": "string",
      "suggested_tool": {
        "name": "string",
        "functionality": "string",
        "input_schema": {},
        "output_schema": {},
        "priority": "high|medium|low"
      }
    }
  },
  "reasoning_chain": "string"
}
```

Ensure your JSON is valid and complete.
```

---

## Few-Shot 示例

### 示例 1: 成功识别工具缺口

**输入**:
```markdown
Task: "Download all images from a webpage and resize them to 100x100"
Error: "Tool 'download_image' only handles single URLs, called 47 times"
Tools Available: [read_file, write_file, download_image, resize_image]
```

**输出**:
```json
{
  "task_id": "task_001",
  "analysis": {
    "failure_factors": [
      {
        "factor": "No batch download capability",
        "type": "tool",
        "likelihood": "high",
        "counterfactual": "If batch_download existed, task would complete in 1 call instead of 47"
      },
      {
        "factor": "No HTML parsing tool",
        "type": "tool",
        "likelihood": "medium",
        "counterfactual": "If parse_html existed, could extract image URLs automatically"
      }
    ],
    "root_cause": {
      "description": "Missing batch download tool for handling multiple URLs",
      "type": "tool_gap",
      "confidence": 0.92
    },
    "tool_gap": {
      "exists": true,
      "description": "Need a tool to download multiple files from URLs in batch",
      "suggested_tool": {
        "name": "batch_download",
        "functionality": "Download multiple files from URLs in parallel",
        "input_schema": {
          "urls": "array of strings",
          "output_dir": "string",
          "max_concurrent": "integer"
        },
        "output_schema": {
          "downloaded_files": "array of strings",
          "failed_urls": "array of strings"
        },
        "priority": "high"
      }
    }
  },
  "reasoning_chain": "The task requires downloading multiple images, but only single-image download is available. The high number of tool calls (47) indicates inefficiency. Counterfactual analysis shows batch_download would reduce calls by 97%."
}
```

### 示例 2: 非工具缺口 (输入问题)

**输入**:
```markdown
Task: "Read file 'config.yaml'"
Error: "File not found: config.yaml"
Tools Available: [read_file, list_directory, search_files]
```

**输出**:
```json
{
  "task_id": "task_002",
  "analysis": {
    "failure_factors": [
      {
        "factor": "File does not exist",
        "type": "input",
        "likelihood": "high",
        "counterfactual": "If file existed, read would succeed"
      }
    ],
    "root_cause": {
      "description": "Incorrect file path provided by user",
      "type": "input_error",
      "confidence": 0.95
    },
    "tool_gap": {
      "exists": false,
      "description": "No tool gap - file simply doesn't exist",
      "suggested_tool": null
    }
  },
  "reasoning_chain": "The error clearly indicates file not found. This is an input error, not a tool gap. The read_file tool works correctly for existing files."
}
```

---

## 使用说明

### 变量替换
- `{task_id}`: 任务唯一标识
- `{task_description}`: 任务描述
- `{error_message}`: 错误信息
- `{tools_attempted}`: 尝试的工具列表
- `{execution_time}`: 执行时间
- `{available_tools_list}`: 可用工具列表
- `{similar_tasks_history}`: 相似任务历史

### 调用参数
```python
{
    "model": "gpt-4",
    "temperature": 0.2,  # 低温度确保一致性
    "max_tokens": 2000,
    "response_format": { "type": "json_object" }
}
```

### 后处理
1. 验证 JSON 格式
2. 检查置信度阈值 (>0.7)
3. 去重相似缺口
4. 优先级排序

---

**最后更新**: 2026-03-27
**版本**: 1.0
