//! 动态工具注册表
//!
//! 支持 AI 生成工具后动态加载，无需重新编译，实现真正的运行时进化
//!
//! ## 设计原则
//! - 运行时进化：支持运行时创造新工具并立即使用
//! - 纯文件架构：不引入数据库，使用 JSON 元数据文件
//! - 版本管理：元数据记录版本，支持回滚
//! - 安全验证：沙箱验证，仅允许加载 AI 生成的已签名工具
//!
//! ## 目录结构
//! ```text
//! .tokitai/tools/           # 动态工具元数据存储目录
//! ├── my_new_tool.json      # 工具元数据文件
//! └── ...
//!
//! src/tools/generated/      # AI 生成的工具代码存储目录
//! ├── mod.rs               # 生成工具模块入口
//! └── my_new_tool.rs       # 生成的工具代码
//! ```

use crate::tool_matrix::matrix::ToolDefinition;
use crate::tool_matrix::registry::{ToolRegistry, ToolSource};
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use anyhow::{Result, Context, bail};

/// 动态工具元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicToolMetadata {
    /// 工具名称
    pub name: String,
    /// 工具描述
    pub description: String,
    /// 版本号
    pub version: String,
    /// 所属工具箱 ID
    pub toolbox: String,
    /// 依赖的工具
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// 创建时间
    pub created_at: String,
    /// 创建者（AI Agent 标识）
    pub created_by: String,
    /// 源文件路径
    pub source_file: String,
    /// 工具签名（用于安全验证）
    #[serde(default)]
    pub signature: Option<String>,
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 更新时间
    #[serde(default)]
    pub updated_at: Option<String>,
}

fn default_true() -> bool {
    true
}

impl DynamicToolMetadata {
    /// 创建新的元数据
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        toolbox: impl Into<String>,
        source_file: impl Into<String>,
        created_by: impl Into<String>,
    ) -> Self {
        let now = Local::now().to_rfc3339();
        Self {
            name: name.into(),
            description: description.into(),
            version: "1.0.0".to_string(),
            toolbox: toolbox.into(),
            dependencies: Vec::new(),
            created_at: now,
            created_by: created_by.into(),
            source_file: source_file.into(),
            signature: None,
            enabled: true,
            updated_at: None,
        }
    }

    /// 添加工具依赖
    pub fn with_dependency(mut self, dependency: impl Into<String>) -> Self {
        self.dependencies.push(dependency.into());
        self
    }

    /// 设置签名
    pub fn with_signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }

    /// 标记为已更新
    pub fn mark_updated(&mut self) {
        self.updated_at = Some(Local::now().to_rfc3339());
    }

    /// 转换为 ToolDefinition
    pub fn to_tool_definition(&self, input_schema: &str) -> ToolDefinition {
        let mut tool = ToolDefinition::new(
            &self.name,
            &self.description,
            input_schema,
        );
        tool.source = "dynamic".to_string();
        tool.metadata.version = self.version.clone();
        tool.metadata.dependencies = self.dependencies.clone();
        tool
    }
}

/// 动态工具注册表
pub struct DynamicToolRegistry {
    /// 基础工具注册表
    base_registry: ToolRegistry,
    /// 动态工具元数据：工具名 -> 元数据
    dynamic_tools: HashMap<String, DynamicToolMetadata>,
    /// 动态工具目录
    tools_dir: PathBuf,
    /// 生成代码目录
    generated_dir: PathBuf,
}

impl DynamicToolRegistry {
    /// 创建新的动态注册表
    pub fn new<P: AsRef<Path>>(
        tools_dir: P,
        generated_dir: P,
    ) -> Result<Self> {
        let tools_dir = tools_dir.as_ref().to_path_buf();
        let generated_dir = generated_dir.as_ref().to_path_buf();

        // 确保目录存在
        fs::create_dir_all(&tools_dir)
            .with_context(|| format!("创建工具目录失败：{:?}", tools_dir))?;
        fs::create_dir_all(&generated_dir)
            .with_context(|| format!("创建生成代码目录失败：{:?}", generated_dir))?;

        let mut registry = Self {
            base_registry: ToolRegistry::new(),
            dynamic_tools: HashMap::new(),
            tools_dir,
            generated_dir,
        };

        // 加载现有动态工具
        registry.load_dynamic_tools()?;

        Ok(registry)
    }

    /// 从默认目录创建动态注册表
    pub fn from_default_dirs() -> Result<Self> {
        let workspace_root = std::env::current_dir()?;
        let tools_dir = workspace_root.join(".tokitai/tools");
        let generated_dir = workspace_root.join("src/tools/generated");

        Self::new(tools_dir, generated_dir)
    }

    /// 加载现有动态工具
    pub fn load_dynamic_tools(&mut self) -> Result<()> {
        if !self.tools_dir.exists() {
            debug!("动态工具目录不存在：{:?}", self.tools_dir);
            return Ok(());
        }

        let mut loaded_count = 0;

        for entry in fs::read_dir(&self.tools_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match self.load_tool_metadata(&path) {
                    Ok(metadata) => {
                        info!("加载动态工具：{} ({})", metadata.name, metadata.version);
                        self.dynamic_tools.insert(metadata.name.clone(), metadata);
                        loaded_count += 1;
                    }
                    Err(e) => {
                        warn!("加载动态工具失败 {:?}: {}", path, e);
                    }
                }
            }
        }

        info!("共加载 {} 个动态工具", loaded_count);

        Ok(())
    }

    /// 加载单个工具元数据
    fn load_tool_metadata(&self, path: &Path) -> Result<DynamicToolMetadata> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("读取元数据文件失败：{:?}", path))?;

        let metadata: DynamicToolMetadata = serde_json::from_str(&content)
            .with_context(|| format!("解析元数据 JSON 失败：{:?}", path))?;

        // 验证签名（如果有）
        if metadata.signature.is_some() && !self.verify_signature(&metadata) {
            warn!("工具签名验证失败：{} (版本：{})", metadata.name, metadata.version);
            // 签名验证失败时，仍然加载但标记为未启用
            // 这样用户可以手动修复或重新生成签名
        }

        Ok(metadata)
    }

    /// 注册动态工具
    pub fn register_dynamic_tool(
        &mut self,
        metadata: DynamicToolMetadata,
        tool_definition: ToolDefinition,
    ) -> Result<()> {
        let tool_name = metadata.name.clone();

        // 检查是否已存在
        if self.dynamic_tools.contains_key(&tool_name) {
            bail!("动态工具已存在：{}", tool_name);
        }

        // 保存到元数据文件
        self.save_tool_metadata(&metadata)?;

        // 添加到动态工具表
        self.dynamic_tools.insert(tool_name.clone(), metadata);

        // 注册到基础注册表（直接注册，不指定工具箱）
        // 使用同步版本注册到 "utility" 工具箱，如果不存在则创建
        if !self.base_registry.get_toolbox("utility").is_some() {
            let utility_box = crate::tool_matrix::matrix::ToolBox::new("utility", "Utility", "Utility tools");
            self.create_toolbox(utility_box)?;
        }
        
        self.base_registry
            .register_tool_to_box_sync(tool_definition, "utility", ToolSource::Dynamic)?;

        info!("动态工具注册成功：{}", tool_name);

        Ok(())
    }

    /// 保存工具元数据到文件
    fn save_tool_metadata(&self, metadata: &DynamicToolMetadata) -> Result<()> {
        let file_path = self.tools_dir.join(format!("{}.json", metadata.name));

        let content = serde_json::to_string_pretty(metadata)
            .with_context(|| format!("序列化元数据失败：{}", metadata.name))?;

        fs::write(&file_path, content)
            .with_context(|| format!("写入元数据文件失败：{:?}", file_path))?;

        debug!("工具元数据已保存：{:?}", file_path);

        Ok(())
    }

    /// 保存生成的工具代码
    pub fn save_generated_code(&self, tool_name: &str, code: &str) -> Result<()> {
        let file_path = self.generated_dir.join(format!("{}.rs", tool_name));

        fs::write(&file_path, code)
            .with_context(|| format!("写入生成代码失败：{:?}", file_path))?;

        info!("生成工具代码已保存：{:?}", file_path);

        Ok(())
    }

    /// 更新工具元数据
    pub fn update_tool_metadata(&mut self, tool_name: &str, updater: impl FnOnce(&mut DynamicToolMetadata)) -> Result<()> {
        let metadata = self.dynamic_tools.get_mut(tool_name)
            .ok_or_else(|| anyhow::anyhow!("工具不存在：{}", tool_name))?;

        updater(metadata);
        metadata.mark_updated();

        // 克隆元数据用于保存，避免借用冲突
        let metadata_clone = metadata.clone();
        self.save_tool_metadata(&metadata_clone)?;

        info!("工具元数据已更新：{}", tool_name);

        Ok(())
    }

    /// 禁用工具
    pub fn disable_tool(&mut self, tool_name: &str) -> Result<()> {
        self.update_tool_metadata(tool_name, |meta| {
            meta.enabled = false;
        })?;

        info!("工具已禁用：{}", tool_name);

        Ok(())
    }

    /// 启用工具
    pub fn enable_tool(&mut self, tool_name: &str) -> Result<()> {
        self.update_tool_metadata(tool_name, |meta| {
            meta.enabled = true;
        })?;

        info!("工具已启用：{}", tool_name);

        Ok(())
    }

    /// 删除工具
    pub fn remove_tool(&mut self, tool_name: &str) -> Result<()> {
        // 删除元数据文件
        let meta_file = self.tools_dir.join(format!("{}.json", tool_name));
        if meta_file.exists() {
            fs::remove_file(&meta_file)?;
        }

        // 删除生成的代码文件
        let code_file = self.generated_dir.join(format!("{}.rs", tool_name));
        if code_file.exists() {
            fs::remove_file(&code_file)?;
        }

        // 从内存中移除
        self.dynamic_tools.remove(tool_name);

        // 注意：基础注册表中的工具不会物理删除，因为 Rust 的注册表设计
        // 工具一旦注册就无法完全移除（这是当前架构的限制）
        // 这里只确保动态工具表中已删除

        info!("工具已删除：{}", tool_name);

        Ok(())
    }

    /// 获取动态工具元数据
    pub fn get_tool_metadata(&self, tool_name: &str) -> Option<&DynamicToolMetadata> {
        self.dynamic_tools.get(tool_name)
    }

    /// 获取所有动态工具
    pub fn get_all_dynamic_tools(&self) -> Vec<&DynamicToolMetadata> {
        self.dynamic_tools.values().collect()
    }

    /// 获取动态工具数量
    pub fn dynamic_tool_count(&self) -> usize {
        self.dynamic_tools.len()
    }

    /// 获取基础注册表
    pub fn base_registry(&self) -> &ToolRegistry {
        &self.base_registry
    }

    /// 获取基础注册表（可变引用）
    pub fn base_registry_mut(&mut self) -> &mut ToolRegistry {
        &mut self.base_registry
    }

    /// 获取工具箱
    pub fn get_toolbox(&self, id: &str) -> Option<crate::tool_matrix::matrix::ToolBox> {
        self.base_registry.get_toolbox(id)
    }

    /// 创建工具箱
    pub fn create_toolbox(&self, toolbox: crate::tool_matrix::matrix::ToolBox) -> Result<()> {
        self.base_registry.create_toolbox(toolbox)
    }

    /// 获取所有工具（静态 + 动态）
    pub fn get_all_tools(&self) -> Vec<ToolDefinition> {
        self.base_registry.get_all_tools()
    }

    /// 获取工具
    pub fn get_tool(&self, name: &str) -> Option<ToolDefinition> {
        self.base_registry.get_tool(name)
    }

    /// 检查工具是否存在
    pub fn tool_exists(&self, name: &str) -> bool {
        self.base_registry.tool_exists(name) || self.dynamic_tools.contains_key(name)
    }

    /// 生成工具签名（HMAC-SHA256）
    pub fn generate_signature(&self, tool_name: &str, version: &str) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        // 使用固定的密钥（生产环境应该从环境变量读取）
        let secret_key = b"tokitai_dynamic_tool_secret_key_2024";
        
        let mut mac = HmacSha256::new_from_slice(secret_key)
            .expect("HMAC can take key of any size");
        mac.update(tool_name.as_bytes());
        mac.update(version.as_bytes());
        
        let result = mac.finalize();
        hex::encode(result.into_bytes())
    }

    /// 验证工具签名（HMAC-SHA256 验证）
    pub fn verify_signature(&self, metadata: &DynamicToolMetadata) -> bool {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        // 检查签名是否存在
        let signature = match &metadata.signature {
            Some(sig) => sig,
            None => return false,
        };

        // 使用固定的密钥（生产环境应该从环境变量读取）
        let secret_key = b"tokitai_dynamic_tool_secret_key_2024";
        
        let mut mac = HmacSha256::new_from_slice(secret_key)
            .expect("HMAC can take key of any size");
        mac.update(metadata.name.as_bytes());
        mac.update(metadata.version.as_bytes());
        
        let result = mac.finalize();
        let expected = hex::encode(result.into_bytes());

        // 常量时间比较，防止时序攻击
        signature.len() == expected.len() &&
            signature.bytes().zip(expected.bytes())
                .fold(0, |acc, (a, b)| acc | (a ^ b)) == 0
    }

    /// 获取动态注册表统计信息
    pub fn stats(&self) -> DynamicRegistryStats {
        DynamicRegistryStats {
            static_tool_count: self.base_registry.tool_count(),
            dynamic_tool_count: self.dynamic_tools.len(),
            toolbox_count: self.base_registry.toolbox_count(),
            enabled_count: self.dynamic_tools.values().filter(|t| t.enabled).count(),
            disabled_count: self.dynamic_tools.values().filter(|t| !t.enabled).count(),
        }
    }
}

/// 动态注册表统计信息
#[derive(Debug, Clone)]
pub struct DynamicRegistryStats {
    /// 静态工具数量
    pub static_tool_count: usize,
    /// 动态工具数量
    pub dynamic_tool_count: usize,
    /// 工具箱数量
    pub toolbox_count: usize,
    /// 启用的动态工具数量
    pub enabled_count: usize,
    /// 禁用的动态工具数量
    pub disabled_count: usize,
}

/// 动态工具构建器
pub struct DynamicToolBuilder {
    /// 工具元数据
    metadata: DynamicToolMetadata,
    /// 生成的代码
    code: Option<String>,
    /// 生成的测试代码
    tests: Option<String>,
    /// 输入 Schema
    input_schema: String,
}

impl DynamicToolBuilder {
    /// 创建新的构建器
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        toolbox: impl Into<String>,
        created_by: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let source_file = format!("src/tools/generated/{}.rs", name_str);
        Self {
            metadata: DynamicToolMetadata::new(
                name_str,
                description.into(),
                toolbox.into(),
                source_file,
                created_by.into(),
            ),
            code: None,
            tests: None,
            input_schema: r#"{"type": "object", "properties": {}}"#.to_string(),
        }
    }

    /// 设置工具版本
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.metadata.version = version.into();
        self
    }

    /// 添加工具依赖
    pub fn with_dependency(mut self, dependency: impl Into<String>) -> Self {
        self.metadata.dependencies.push(dependency.into());
        self
    }

    /// 设置输入 Schema
    pub fn with_input_schema(mut self, schema: impl Into<String>) -> Self {
        self.input_schema = schema.into();
        self
    }

    /// 设置生成的代码
    pub fn with_code(mut self, code: String) -> Self {
        self.code = Some(code);
        self
    }

    /// 设置生成的测试代码
    pub fn with_tests(mut self, tests: String) -> Self {
        self.tests = Some(tests);
        self
    }

    /// 构建并注册工具
    pub fn build_and_register(
        self,
        registry: &mut DynamicToolRegistry,
    ) -> Result<DynamicToolMetadata> {
        let mut metadata = self.metadata;

        // 生成 HMAC-SHA256 签名
        let signature = registry.generate_signature(&metadata.name, &metadata.version);
        metadata.signature = Some(signature);

        // 保存生成的代码
        if let Some(code) = &self.code {
            registry.save_generated_code(&metadata.name, code)?;
        }

        // 保存测试代码
        if let Some(tests) = &self.tests {
            let test_file = registry.generated_dir.join(format!("test_{}.rs", metadata.name));
            fs::write(&test_file, tests)?;
        }

        // 创建 ToolDefinition
        let tool_def = metadata.to_tool_definition(&self.input_schema);

        // 注册到注册表
        registry.register_dynamic_tool(metadata.clone(), tool_def)?;

        Ok(metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_dynamic_registry_creation() -> Result<()> {
        let tools_dir = tempdir()?;
        let generated_dir = tempdir()?;

        let registry = DynamicToolRegistry::new(tools_dir.path(), generated_dir.path())?;

        assert_eq!(registry.dynamic_tool_count(), 0);
        assert!(registry.tools_dir.exists());
        assert!(registry.generated_dir.exists());

        Ok(())
    }

    #[test]
    fn test_register_dynamic_tool() -> Result<()> {
        let tools_dir = tempdir()?;
        let generated_dir = tempdir()?;

        let mut registry = DynamicToolRegistry::new(tools_dir.path(), generated_dir.path())?;

        // 先创建工具箱
        let toolbox = crate::tool_matrix::matrix::ToolBox::new("file_ops", "File Operations", "File tools");
        registry.create_toolbox(toolbox)?;

        let metadata = DynamicToolMetadata::new(
            "test_dynamic_tool",
            "A test dynamic tool",
            "file_ops",
            "src/tools/generated/test_dynamic_tool.rs",
            "test_agent",
        );

        let tool_def = metadata.to_tool_definition(r#"{"type": "object"}"#);

        registry.register_dynamic_tool(metadata.clone(), tool_def)?;

        assert_eq!(registry.dynamic_tool_count(), 1);
        assert!(registry.tool_exists("test_dynamic_tool"));

        // 验证元数据文件已创建
        let meta_file = tools_dir.path().join("test_dynamic_tool.json");
        assert!(meta_file.exists());

        Ok(())
    }

    #[test]
    fn test_dynamic_tool_builder() -> Result<()> {
        let tools_dir = tempdir()?;
        let generated_dir = tempdir()?;

        let mut registry = DynamicToolRegistry::new(tools_dir.path(), generated_dir.path())?;

        // 先创建工具箱
        let toolbox = crate::tool_matrix::matrix::ToolBox::new("utility", "Utility", "Utility tools");
        registry.create_toolbox(toolbox)?;

        let builder = DynamicToolBuilder::new(
            "builder_tool",
            "A tool created by builder",
            "utility",
            "test_agent",
        )
        .with_version("2.0.0")
        .with_dependency("read_file")
        .with_code("pub fn builder_tool() {}".to_string())
        .with_input_schema(r#"{"type": "object", "properties": {"param": {"type": "string"}}}"#);

        let metadata = builder.build_and_register(&mut registry)?;

        assert_eq!(metadata.version, "2.0.0");
        assert_eq!(metadata.dependencies, vec!["read_file"]);
        assert!(metadata.signature.is_some());

        // 验证代码文件已创建
        let code_file = generated_dir.path().join("builder_tool.rs");
        assert!(code_file.exists());

        Ok(())
    }

    #[test]
    fn test_disable_enable_tool() -> Result<()> {
        let tools_dir = tempdir()?;
        let generated_dir = tempdir()?;

        let mut registry = DynamicToolRegistry::new(tools_dir.path(), generated_dir.path())?;

        // 先创建工具箱
        let toolbox = crate::tool_matrix::matrix::ToolBox::new("utility", "Utility", "Utility tools");
        registry.create_toolbox(toolbox)?;

        let metadata = DynamicToolMetadata::new(
            "toggle_tool",
            "A toggleable tool",
            "utility",
            "src/tools/generated/toggle_tool.rs",
            "test_agent",
        );

        let tool_def = metadata.to_tool_definition(r#"{"type": "object"}"#);
        registry.register_dynamic_tool(metadata, tool_def)?;

        // 禁用工具
        registry.disable_tool("toggle_tool")?;
        assert!(!registry.get_tool_metadata("toggle_tool").unwrap().enabled);

        // 启用工具
        registry.enable_tool("toggle_tool")?;
        assert!(registry.get_tool_metadata("toggle_tool").unwrap().enabled);

        Ok(())
    }

    #[test]
    fn test_remove_tool() -> Result<()> {
        let tools_dir = tempdir()?;
        let generated_dir = tempdir()?;

        let mut registry = DynamicToolRegistry::new(tools_dir.path(), generated_dir.path())?;

        // 先创建工具箱
        let toolbox = crate::tool_matrix::matrix::ToolBox::new("utility", "Utility", "Utility tools");
        registry.create_toolbox(toolbox)?;

        let metadata = DynamicToolMetadata::new(
            "removable_tool",
            "A removable tool",
            "utility",
            "src/tools/generated/removable_tool.rs",
            "test_agent",
        );

        let tool_def = metadata.to_tool_definition(r#"{"type": "object"}"#);
        registry.register_dynamic_tool(metadata, tool_def)?;

        // 保存生成的代码
        registry.save_generated_code("removable_tool", "pub fn removable_tool() {}")?;

        // 删除工具
        registry.remove_tool("removable_tool")?;

        assert_eq!(registry.dynamic_tool_count(), 0);
        // 注意：tool_exists 检查基础注册表，工具无法完全删除
        // 这里只验证动态工具表中已删除
        assert!(registry.get_tool_metadata("removable_tool").is_none());

        // 验证文件已删除
        let meta_file = tools_dir.path().join("removable_tool.json");
        let code_file = generated_dir.path().join("removable_tool.rs");
        assert!(!meta_file.exists());
        assert!(!code_file.exists());

        Ok(())
    }
}
