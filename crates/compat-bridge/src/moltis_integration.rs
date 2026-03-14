//! Moltis 集成接口
//!
//! 将 OpenClaw 技能集成到 Moltis 的工具系统中

use crate::{CompatBridge, CompatError, MoltisTool, HandlerType};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Moltis 工具注册器适配器
pub struct MoltisToolRegistry {
    /// OpenClaw 兼容桥
    compat_bridge: Arc<RwLock<CompatBridge>>,
    /// 已注册的 Moltis 工具
    registered_tools: Arc<RwLock<Vec<MoltisTool>>>,
}

impl MoltisToolRegistry {
    /// 创建新的工具注册器
    pub fn new() -> Self {
        Self {
            compat_bridge: Arc::new(RwLock::new(CompatBridge::new())),
            registered_tools: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 注册 OpenClaw WASM 插件
    pub async fn register_openclaw_plugin(&self, plugin_path: &str) -> Result<(), CompatError> {
        let mut bridge = self.compat_bridge.write().await;
        
        // 1. 注册插件到兼容桥
        bridge.register_wasm_plugin(plugin_path)?;
        
        // 2. 转换技能为 Moltis 工具
        let tools = bridge.get_registered_tools();
        
        // 3. 注册到 Moltis 工具列表
        let mut registered = self.registered_tools.write().await;
        for tool in tools {
            if !registered.iter().any(|t| t.name == tool.name) {
                registered.push(tool.clone());
            }
        }

        Ok(())
    }

    /// 执行工具（兼容 Moltis 接口）
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        args: &Value,
    ) -> Result<ToolExecutionResult, CompatError> {
        let registered = self.registered_tools.read().await;
        
        // 1. 找到工具定义
        let tool = registered.iter()
            .find(|t| t.name == tool_name)
            .ok_or_else(|| CompatError::SkillNotFound(tool_name.to_string()))?;

        // 2. 根据工具类型执行
        match tool.handler_type {
            HandlerType::Wasm => {
                let bridge = self.compat_bridge.read().await;
                let output = bridge.execute_wasm_skill(tool_name, args).await?;
                Ok(ToolExecutionResult {
                    success: true,
                    output,
                    error: None,
                })
            }
            HandlerType::Native => {
                // 原生 Moltis 工具执行逻辑
                Ok(ToolExecutionResult {
                    success: false,
                    output: String::new(),
                    error: Some("原生工具执行尚未实现".to_string()),
                })
            }
            HandlerType::Hybrid => {
                // 混合模式执行逻辑
                Ok(ToolExecutionResult {
                    success: false,
                    output: String::new(),
                    error: Some("混合模式执行尚未实现".to_string()),
                })
            }
        }
    }

    /// 获取所有可用工具
    pub async fn get_available_tools(&self) -> Vec<MoltisTool> {
        let registered = self.registered_tools.read().await;
        registered.clone()
    }

    /// 搜索工具
    pub async fn search_tools(&self, query: &str) -> Vec<MoltisTool> {
        let registered = self.registered_tools.read().await;
        registered
            .iter()
            .filter(|tool| {
                tool.name.contains(query) || 
                tool.description.contains(query)
            })
            .cloned()
            .collect()
    }

    /// 按类别获取工具
    pub async fn get_tools_by_category(&self, category: &str) -> Vec<MoltisTool> {
        let registered = self.registered_tools.read().await;
        registered
            .iter()
            .filter(|tool| {
                // 从工具名称推断类别（例如 "weather.current" -> "weather"）
                tool.name.split('.').next() == Some(category)
            })
            .cloned()
            .collect()
    }

    /// 获取工具统计信息
    pub async fn get_tool_stats(&self) -> ToolStats {
        let registered = self.registered_tools.read().await;
        let mut stats = ToolStats::default();
        
        for tool in registered.iter() {
            stats.total_tools += 1;
            
            match tool.handler_type {
                HandlerType::Wasm => stats.wasm_tools += 1,
                HandlerType::Native => stats.native_tools += 1,
                HandlerType::Hybrid => stats.hybrid_tools += 1,
            }
            
            match tool.risk_level {
                crate::MoltisRisk::Safe => stats.safe_tools += 1,
                crate::MoltisRisk::Confirm => stats.confirm_tools += 1,
                crate::MoltisRisk::Deny => stats.deny_tools += 1,
            }
        }
        
        stats
    }
}

/// 工具执行结果
#[derive(Debug, Clone)]
pub struct ToolExecutionResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

/// 工具统计信息
#[derive(Debug, Clone, Default)]
pub struct ToolStats {
    pub total_tools: usize,
    pub wasm_tools: usize,
    pub native_tools: usize,
    pub hybrid_tools: usize,
    pub safe_tools: usize,
    pub confirm_tools: usize,
    pub deny_tools: usize,
}

/// Moltis 风险检查器
pub struct MoltisRiskChecker {
    registry: Arc<MoltisToolRegistry>,
}

impl MoltisRiskChecker {
    pub fn new(registry: Arc<MoltisToolRegistry>) -> Self {
        Self { registry }
    }

    /// 检查工具执行权限
    pub async fn check_execution_permission(
        &self,
        tool_name: &str,
        user_context: &UserContext,
    ) -> Result<RiskDecision, CompatError> {
        let tools = self.registry.get_available_tools().await;
        
        let tool = tools.iter()
            .find(|t| t.name == tool_name)
            .ok_or_else(|| CompatError::SkillNotFound(tool_name.to_string()))?;

        match tool.risk_level {
            crate::MoltisRisk::Safe => Ok(RiskDecision::Allow),
            crate::MoltisRisk::Confirm => {
                if user_context.auto_confirm {
                    Ok(RiskDecision::Allow)
                } else {
                    Ok(RiskDecision::RequireConfirmation(format!(
                        "即将执行工具 '{}': {}",
                        tool.name, tool.description
                    )))
                }
            }
            crate::MoltisRisk::Deny => Ok(RiskDecision::Deny(format!(
                "工具 '{}' 被安全策略禁止",
                tool.name
            ))),
        }
    }
}

/// 用户上下文
#[derive(Debug, Clone)]
pub struct UserContext {
    pub user_id: String,
    pub permissions: Vec<String>,
    pub auto_confirm: bool,
    pub risk_tolerance: RiskTolerance,
}

#[derive(Debug, Clone)]
pub enum RiskTolerance {
    Conservative,
    Moderate,
    Permissive,
}

/// 风险决策
#[derive(Debug, Clone)]
pub enum RiskDecision {
    Allow,
    RequireConfirmation(String),
    Deny(String),
}

// ── 测试 ───────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_registry_creation() {
        let registry = MoltisToolRegistry::new();
        let tools = registry.get_available_tools().await;
        assert_eq!(tools.len(), 0);
    }

    #[tokio::test]
    async fn test_tool_search() {
        let registry = MoltisToolRegistry::new();
        let results = registry.search_tools("weather").await;
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_tool_stats() {
        let registry = MoltisToolRegistry::new();
        let stats = registry.get_tool_stats().await;
        assert_eq!(stats.total_tools, 0);
        assert_eq!(stats.wasm_tools, 0);
        assert_eq!(stats.native_tools, 0);
    }

    #[tokio::test]
    async fn test_risk_checker() {
        let registry = Arc::new(MoltisToolRegistry::new());
        let checker = MoltisRiskChecker::new(registry);
        
        let user_context = UserContext {
            user_id: "test_user".to_string(),
            permissions: vec![],
            auto_confirm: false,
            risk_tolerance: RiskTolerance::Moderate,
        };

        let result = checker.check_execution_permission("nonexistent.tool", &user_context).await;
        assert!(result.is_err());
    }
}
