# Tokitai Tool Templates

This directory contains 10 ready-to-use tool templates for the Tokitai ecosystem.

## Available Templates

| # | Template | Category | Description |
|---|----------|----------|-------------|
| 01 | `basic-tool.toml` | Utility | Simple starter template for basic operations |
| 02 | `network-tool.toml` | Network | HTTP/API integration with rate limiting |
| 03 | `file-tool.toml` | Filesystem | File read/write operations |
| 04 | `ai-tool.toml` | AI | LLM-powered tool with model selection |
| 05 | `code-analysis-tool.toml` | Code | Static analysis and linting |
| 06 | `git-tool.toml` | Version Control | Git operations and commands |
| 07 | `database-tool.toml` | Database | SQL query execution |
| 08 | `search-tool.toml` | Search | File and content search |
| 09 | `webhook-tool.toml` | Integration | Event-driven webhook calls |
| 10 | `automation-tool.toml` | Automation | Workflow orchestration |

## How to Use

1. **Copy a template**: `cp templates/01-basic-tool.toml tools/my-tool/tool.toml`
2. **Customize**: Edit the TOML file with your tool's metadata and parameters
3. **Implement**: Create the Rust implementation in `tools/my-tool/src/lib.rs`
4. **Test**: Run `cargo test` to verify your tool works
5. **Publish**: Use `tokitai publish my-tool` to share with the community

## Template Structure

Each template includes:

```toml
[tool]
# Tool metadata (name, version, description, author, category, tags)

[[parameters]]
# Parameter definitions (name, type, required, default, description)

[permissions]
# Permission declarations (network, file read/write, command execution)

[rate_limit]
# Optional rate limiting (requests per minute)
```

## Supported Parameter Types

- `string` - Text input
- `integer` - Whole numbers
- `number` - Floating-point numbers
- `boolean` - True/false values
- `array` - List of values
- `object` - Key-value pairs

## Best Practices

1. **Descriptive names**: Use clear, action-oriented tool names
2. **Detailed descriptions**: Help users understand what your tool does
3. **Type safety**: Mark required parameters and provide sensible defaults
4. **Permission minimalism**: Request only the permissions you need
5. **Rate limiting**: Set appropriate limits for external API calls

## Contributing

Submit your custom templates to the Tokitai community registry!
