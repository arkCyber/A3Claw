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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoltisRisk {
    Safe,
    Confirm,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        // 实现：从 WASM 文件中提取 manifest
        // 这里需要集成 WasmEdge 或其他 WASM 运行时
        todo!("实现 manifest 加载")
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
        
        // 3. 安全检查
        self.security_scan(plugin_path)?;
        
        Ok(())
    }

    fn security_scan(&self, plugin_path: &str) -> Result<(), CompatError> {
        // 实现安全扫描逻辑
        // 1. 检查是否包含危险系统调用
        // 2. 验证数字签名
        // 3. 检查权限声明
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
    ) -> Result<WasmResult, CompatError> {
        // 实现 WASM 技能执行
        // 1. 加载 WASM 模块
        // 2. 调用技能函数
        // 3. 处理返回结果
        todo!("实现 WASM 执行")
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
}
