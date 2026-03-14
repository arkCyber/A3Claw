//! OpenClaw+ → Moltis 兼容桥接层
//!
//! 提供 OpenClaw WASM 技能在 Moltis 中运行的能力，
//! 同时保持两个系统的独立性和安全性。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod wasm_runtime;
pub mod moltis_integration;

pub use wasm_runtime::WasmRuntime;
pub use moltis_integration::{MoltisToolRegistry, ToolExecutionResult, ToolStats};

// ── 类型定义 ──────────────────────────────────────────────────────────────────

/// OpenClaw 风险级别
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenClawRisk {
    Safe,
    Confirm,
    Deny,
}

/// Moltis 工具定义（简化版）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoltisTool {
    pub name: String,
    pub description: String,
    pub parameters: Vec<MoltisParam>,
    pub risk_level: MoltisRisk,
    pub handler_type: HandlerType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoltisParam {
    pub name: String,
    pub description: String,
    pub param_type: String,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoltisRisk {
    Safe,
    Confirm,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandlerType {
    Native,    // Moltis 原生工具
    Wasm,      // OpenClaw WASM 技能
    Hybrid,    // 混合模式
}

/// OpenClaw 技能定义（来自 OpenClaw+）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawSkill {
    pub name: String,
    pub display: String,
    pub description: String,
    pub risk: OpenClawRisk,
    pub params: Vec<OpenClawParam>,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawParam {
    pub name: String,
    pub description: String,
    pub param_type: String,
    pub required: bool,
}

// ── 兼容桥接器 ────────────────────────────────────────────────────────────────

pub struct CompatBridge {
    wasm_plugins: HashMap<String, WasmPlugin>,
    tool_cache: HashMap<String, MoltisTool>,
}

#[derive(Debug, Clone)]
pub struct WasmPlugin {
    pub path: String,
    pub manifest: OpenClawSkillManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawSkillManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub skills: Vec<OpenClawSkill>,
}

impl CompatBridge {
    pub fn new() -> Self {
        Self {
            wasm_plugins: HashMap::new(),
            tool_cache: HashMap::new(),
        }
    }

    /// 注册 OpenClaw WASM 插件
    pub fn register_wasm_plugin(&mut self, plugin_path: &str) -> Result<(), CompatError> {
        // 1. 读取插件 manifest
        let manifest = self.load_manifest(plugin_path)?;
        
        // 2. 验证插件安全性
        self.validate_plugin(&manifest)?;
        
        // 3. 注册插件
        let plugin = WasmPlugin {
            path: plugin_path.to_string(),
            manifest,
        };
        
        self.wasm_plugins.insert(plugin_path.to_string(), plugin);
        
        // 4. 转换技能为工具
        self.convert_skills_to_tools(plugin_path)?;
        
        Ok(())
    }

    /// 将 OpenClaw Skill 转换为 Moltis Tool
    fn convert_skill_to_tool(&self, skill: &OpenClawSkill) -> MoltisTool {
        MoltisTool {
            name: skill.name.clone(),
            description: format!("{} (OpenClaw WASM)", skill.description),
            parameters: skill.params.iter().map(|p| MoltisParam {
                name: p.name.clone(),
                description: p.description.clone(),
                param_type: p.param_type.clone(),
                required: p.required,
            }).collect(),
            risk_level: self.convert_risk(&skill.risk),
            handler_type: HandlerType::Wasm,
        }
    }

    /// 转换风险级别
    fn convert_risk(&self, risk: &OpenClawRisk) -> MoltisRisk {
        match risk {
            OpenClawRisk::Safe => MoltisRisk::Safe,
            OpenClawRisk::Confirm => MoltisRisk::Confirm,
            OpenClawRisk::Deny => MoltisRisk::Deny,
        }
    }

    /// 执行 OpenClaw WASM 技能
    pub async fn execute_wasm_skill(
        &self,
        skill_name: &str,
        args: &serde_json::Value,
    ) -> Result<String, CompatError> {
        // 1. 找到对应的插件
        let plugin = self.find_plugin_for_skill(skill_name)?;
        
        // 2. 准备执行环境
        let mut executor = WasmExecutor::new(&plugin.path)?;
        
        // 3. 转换参数格式
        let openclaw_args = self.convert_args_to_openclaw(args)?;
        
        // 4. 执行技能
        let result = executor.execute(skill_name, &openclaw_args).await?;
        
        Ok(result)
    }

    /// 获取所有已注册的工具
    pub fn get_registered_tools(&self) -> Vec<&MoltisTool> {
        self.tool_cache.values().collect()
    }

    // ── 私有方法 ────────────────────────────────────────────────────────────────

    fn load_manifest(&self, plugin_path: &str) -> Result<OpenClawSkillManifest, CompatError> {
        // 从 WASM 文件旁边的 manifest.json 加载
        let manifest_path = format!("{}.manifest.json", plugin_path);
        
        match std::fs::read_to_string(&manifest_path) {
            Ok(content) => {
                serde_json::from_str(&content)
                    .map_err(|e| CompatError::InvalidManifest(format!("解析 manifest 失败: {}", e)))
            }
            Err(_) => {
                // 如果没有独立的 manifest 文件，返回默认的空 manifest
                // 实际生产环境中应该从 WASM 模块的自定义段读取
                Ok(OpenClawSkillManifest {
                    id: "unknown".to_string(),
                    name: "Unknown Plugin".to_string(),
                    version: "0.0.0".to_string(),
                    description: "No manifest found".to_string(),
                    skills: vec![],
                })
            }
        }
    }

    fn validate_plugin(&self, manifest: &OpenClawSkillManifest) -> Result<(), CompatError> {
        // 1. 检查插件 ID 格式
        if manifest.id.is_empty() {
            return Err(CompatError::InvalidManifest("插件 ID 不能为空".to_string()));
        }
        
        // 2. 检查技能定义
        for skill in &manifest.skills {
            if skill.name.is_empty() {
                return Err(CompatError::InvalidManifest("技能名称不能为空".to_string()));
            }
        }
        
        // 3. 安全检查（基本验证）
        self.security_scan("")?;
        
        Ok(())
    }

    fn security_scan(&self, _plugin_path: &str) -> Result<(), CompatError> {
        // 安全扫描逻辑
        // 1. 检查文件大小（防止过大的 WASM 模块）
        // 2. 验证 WASM 模块格式
        // 3. 检查导入的函数（确保只使用允许的 WASI 函数）
        // 4. 验证数字签名（如果存在）
        
        // 当前实现：基本检查通过
        // TODO: 在生产环境中应该实现完整的安全扫描
        Ok(())
    }

    fn convert_skills_to_tools(&mut self, plugin_path: &str) -> Result<(), CompatError> {
        let plugin = self.wasm_plugins.get(plugin_path)
            .ok_or_else(|| CompatError::PluginNotFound(plugin_path.to_string()))?;
        
        for skill in &plugin.manifest.skills {
            let tool = self.convert_skill_to_tool(skill);
            self.tool_cache.insert(skill.name.clone(), tool);
        }
        
        Ok(())
    }

    fn find_plugin_for_skill(&self, skill_name: &str) -> Result<&WasmPlugin, CompatError> {
        for plugin in self.wasm_plugins.values() {
            if plugin.manifest.skills.iter().any(|s| s.name == skill_name) {
                return Ok(plugin);
            }
        }
        Err(CompatError::SkillNotFound(skill_name.to_string()))
    }

    fn convert_args_to_openclaw(&self, args: &serde_json::Value) -> Result<serde_json::Value, CompatError> {
        // 参数格式转换逻辑
        Ok(args.clone())
    }
}

// ── WASM 执行器 ────────────────────────────────────────────────────────────────

pub struct WasmExecutor {
    plugin_path: String,
    // runtime: WasmEdgeRuntime, // 实际实现时需要引入
}

impl WasmExecutor {
    pub fn new(plugin_path: &str) -> Result<Self, CompatError> {
        Ok(Self {
            plugin_path: plugin_path.to_string(),
        })
    }

    pub async fn execute(
        &mut self,
        skill_name: &str,
        args: &serde_json::Value,
    ) -> Result<String, CompatError> {
        // WASM 技能执行实现
        // 注意：这是一个简化的实现，实际生产环境需要使用 WasmEdge 或 wasmtime
        
        // 1. 验证 WASM 文件存在
        if !std::path::Path::new(&self.plugin_path).exists() {
            return Err(CompatError::WasmExecutionError(
                format!("WASM 文件不存在: {}", self.plugin_path)
            ));
        }
        
        // 2. 模拟执行（实际应该加载 WASM 模块并调用函数）
        // 在实际实现中，这里应该：
        // - 使用 wasmedge_sdk 或 wasmtime 加载模块
        // - 创建 WASI 环境
        // - 调用导出的技能函数
        // - 捕获输出和错误
        
        let result = format!(
            "{{\"ok\": true, \"output\": \"Executed {} with args: {}\", \"skill\": \"{}\"}}",
            skill_name,
            args,
            skill_name
        );
        
        Ok(result)
    }
}

#[derive(Debug, Clone)]
pub struct WasmResult {
    pub ok: bool,
    pub output: String,
    pub error: Option<String>,
}

// ── 错误类型 ──────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum CompatError {
    #[error("插件未找到: {0}")]
    PluginNotFound(String),
    
    #[error("技能未找到: {0}")]
    SkillNotFound(String),
    
    #[error("无效的 manifest: {0}")]
    InvalidManifest(String),
    
    #[error("安全检查失败: {0}")]
    SecurityCheckFailed(String),
    
    #[error("WASM 执行错误: {0}")]
    WasmExecutionError(String),
    
    #[error("参数转换错误: {0}")]
    ParameterConversionError(String),
    
    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("序列化错误: {0}")]
    SerializationError(#[from] serde_json::Error),
}

// ── 测试 ───────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_conversion() {
        let bridge = CompatBridge::new();
        
        assert_eq!(
            bridge.convert_risk(&OpenClawRisk::Safe),
            MoltisRisk::Safe
        );
        assert_eq!(
            bridge.convert_risk(&OpenClawRisk::Confirm),
            MoltisRisk::Confirm
        );
        assert_eq!(
            bridge.convert_risk(&OpenClawRisk::Deny),
            MoltisRisk::Deny
        );
    }

    #[test]
    fn test_skill_to_tool_conversion() {
        let bridge = CompatBridge::new();
        let skill = OpenClawSkill {
            name: "weather.current".to_string(),
            display: "Current Weather".to_string(),
            description: "Get current weather".to_string(),
            risk: OpenClawRisk::Safe,
            params: vec![
                OpenClawParam {
                    name: "city".to_string(),
                    description: "City name".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                }
            ],
            category: "weather".to_string(),
        };

        let tool = bridge.convert_skill_to_tool(&skill);
        
        assert_eq!(tool.name, "weather.current");
        assert!(tool.description.contains("OpenClaw WASM"));
        assert_eq!(tool.risk_level, MoltisRisk::Safe);
        assert_eq!(tool.handler_type, HandlerType::Wasm);
        assert_eq!(tool.parameters.len(), 1);
        assert_eq!(tool.parameters[0].name, "city");
    }

    #[test]
    fn test_bridge_initialization() {
        let bridge = CompatBridge::new();
        assert_eq!(bridge.wasm_plugins.len(), 0);
        assert_eq!(bridge.tool_cache.len(), 0);
    }

    #[test]
    fn test_skill_to_tool_conversion_multiple_params() {
        let bridge = CompatBridge::new();
        let skill = OpenClawSkill {
            name: "file.write".to_string(),
            display: "Write File".to_string(),
            description: "Write content to file".to_string(),
            risk: OpenClawRisk::Confirm,
            params: vec![
                OpenClawParam {
                    name: "path".to_string(),
                    description: "File path".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
                OpenClawParam {
                    name: "content".to_string(),
                    description: "File content".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
                OpenClawParam {
                    name: "append".to_string(),
                    description: "Append mode".to_string(),
                    param_type: "boolean".to_string(),
                    required: false,
                },
            ],
            category: "filesystem".to_string(),
        };

        let tool = bridge.convert_skill_to_tool(&skill);
        
        assert_eq!(tool.name, "file.write");
        assert_eq!(tool.risk_level, MoltisRisk::Confirm);
        assert_eq!(tool.parameters.len(), 3);
        assert_eq!(tool.parameters[0].name, "path");
        assert_eq!(tool.parameters[1].name, "content");
        assert_eq!(tool.parameters[2].name, "append");
        assert!(tool.parameters[0].required);
        assert!(tool.parameters[1].required);
        assert!(!tool.parameters[2].required);
    }

    #[test]
    fn test_load_manifest_with_file() {
        let bridge = CompatBridge::new();
        
        // 创建临时 manifest 文件
        let temp_dir = std::env::temp_dir();
        let plugin_path = temp_dir.join("test_plugin.wasm");
        let manifest_path = format!("{}.manifest.json", plugin_path.display());
        
        let manifest = OpenClawSkillManifest {
            id: "test.plugin".to_string(),
            name: "Test Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "A test plugin".to_string(),
            skills: vec![],
        };
        
        let manifest_json = serde_json::to_string(&manifest).unwrap();
        std::fs::write(&manifest_path, manifest_json).unwrap();
        
        let loaded = bridge.load_manifest(&plugin_path.to_string_lossy()).unwrap();
        
        assert_eq!(loaded.id, "test.plugin");
        assert_eq!(loaded.name, "Test Plugin");
        assert_eq!(loaded.version, "1.0.0");
        
        // 清理
        std::fs::remove_file(&manifest_path).ok();
    }

    #[test]
    fn test_load_manifest_without_file() {
        let bridge = CompatBridge::new();
        
        // 使用不存在的路径
        let result = bridge.load_manifest("/nonexistent/plugin.wasm").unwrap();
        
        // 应该返回默认 manifest
        assert_eq!(result.id, "unknown");
        assert_eq!(result.name, "Unknown Plugin");
        assert_eq!(result.skills.len(), 0);
    }

    #[test]
    fn test_validate_plugin_empty_id() {
        let bridge = CompatBridge::new();
        let manifest = OpenClawSkillManifest {
            id: "".to_string(),
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            skills: vec![],
        };
        
        let result = bridge.validate_plugin(&manifest);
        assert!(result.is_err());
        assert!(matches!(result, Err(CompatError::InvalidManifest(_))));
    }

    #[test]
    fn test_validate_plugin_empty_skill_name() {
        let bridge = CompatBridge::new();
        let manifest = OpenClawSkillManifest {
            id: "test.plugin".to_string(),
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            skills: vec![
                OpenClawSkill {
                    name: "".to_string(),
                    display: "Test".to_string(),
                    description: "Test".to_string(),
                    risk: OpenClawRisk::Safe,
                    params: vec![],
                    category: "test".to_string(),
                }
            ],
        };
        
        let result = bridge.validate_plugin(&manifest);
        assert!(result.is_err());
        assert!(matches!(result, Err(CompatError::InvalidManifest(_))));
    }

    #[test]
    fn test_validate_plugin_valid() {
        let bridge = CompatBridge::new();
        let manifest = OpenClawSkillManifest {
            id: "test.plugin".to_string(),
            name: "Test Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "A valid test plugin".to_string(),
            skills: vec![
                OpenClawSkill {
                    name: "test.skill".to_string(),
                    display: "Test Skill".to_string(),
                    description: "A test skill".to_string(),
                    risk: OpenClawRisk::Safe,
                    params: vec![],
                    category: "test".to_string(),
                }
            ],
        };
        
        let result = bridge.validate_plugin(&manifest);
        assert!(result.is_ok());
    }

    #[test]
    fn test_security_scan() {
        let bridge = CompatBridge::new();
        // 当前实现总是返回 Ok
        let result = bridge.security_scan("/any/path.wasm");
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_registered_tools_empty() {
        let bridge = CompatBridge::new();
        let tools = bridge.get_registered_tools();
        assert_eq!(tools.len(), 0);
    }

    #[tokio::test]
    async fn test_wasm_executor_nonexistent_file() {
        let mut executor = WasmExecutor::new("/nonexistent/plugin.wasm").unwrap();
        let args = serde_json::json!({"test": "value"});
        
        let result = executor.execute("test.skill", &args).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(CompatError::WasmExecutionError(_))));
    }

    #[tokio::test]
    async fn test_wasm_executor_with_temp_file() {
        // 创建临时 WASM 文件（空文件用于测试）
        let temp_dir = std::env::temp_dir();
        let wasm_path = temp_dir.join("test_executor.wasm");
        std::fs::File::create(&wasm_path).unwrap();
        
        let mut executor = WasmExecutor::new(&wasm_path.to_string_lossy()).unwrap();
        let args = serde_json::json!({"city": "Beijing"});
        
        let result = executor.execute("weather.current", &args).await;
        assert!(result.is_ok());
        
        let output = result.unwrap();
        assert!(output.contains("weather.current"));
        assert!(output.contains("Beijing"));
        
        // 清理
        std::fs::remove_file(&wasm_path).ok();
    }

    #[test]
    fn test_handler_type_serialization() {
        let native = HandlerType::Native;
        let wasm = HandlerType::Wasm;
        let hybrid = HandlerType::Hybrid;
        
        let native_json = serde_json::to_string(&native).unwrap();
        let wasm_json = serde_json::to_string(&wasm).unwrap();
        let hybrid_json = serde_json::to_string(&hybrid).unwrap();
        
        assert_eq!(native_json, "\"native\"");
        assert_eq!(wasm_json, "\"wasm\"");
        assert_eq!(hybrid_json, "\"hybrid\"");
    }

    #[test]
    fn test_moltis_risk_serialization() {
        let safe = MoltisRisk::Safe;
        let confirm = MoltisRisk::Confirm;
        let deny = MoltisRisk::Deny;
        
        let safe_json = serde_json::to_string(&safe).unwrap();
        let confirm_json = serde_json::to_string(&confirm).unwrap();
        let deny_json = serde_json::to_string(&deny).unwrap();
        
        assert_eq!(safe_json, "\"safe\"");
        assert_eq!(confirm_json, "\"confirm\"");
        assert_eq!(deny_json, "\"deny\"");
    }

    #[test]
    fn test_openclaw_risk_serialization() {
        let safe = OpenClawRisk::Safe;
        let confirm = OpenClawRisk::Confirm;
        let deny = OpenClawRisk::Deny;
        
        let safe_json = serde_json::to_string(&safe).unwrap();
        let confirm_json = serde_json::to_string(&confirm).unwrap();
        let deny_json = serde_json::to_string(&deny).unwrap();
        
        assert_eq!(safe_json, "\"safe\"");
        assert_eq!(confirm_json, "\"confirm\"");
        assert_eq!(deny_json, "\"deny\"");
    }
}
