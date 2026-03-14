//! WASM 运行时集成层
//!
//! 提供在 Moltis 中运行 OpenClaw WASM 技能的能力

use crate::{CompatError, WasmResult};
use serde_json::Value;
use std::collections::HashMap;

#[cfg(feature = "wasm-runtime")]
use wasmedge_sdk::{
    config::{Config, CommonConfigOptions},
    params, VmBuilder, WasmVal,
};

/// WASM 运行时管理器
pub struct WasmRuntime {
    #[allow(dead_code)]
    vm: Option<VmWrapper>,
    loaded_plugins: HashMap<String, LoadedPlugin>,
}

struct VmWrapper {
    #[cfg(feature = "wasm-runtime")]
    vm: wasmedge_sdk::Vm,
}

struct LoadedPlugin {
    manifest: crate::OpenClawSkillManifest,
    module_path: String,
    cached_functions: HashMap<String, CachedFunction>,
}

struct CachedFunction {
    name: String,
    // 预编译的函数句柄或其他优化
}

impl WasmRuntime {
    /// 创建新的 WASM 运行时
    pub fn new() -> Result<Self, CompatError> {
        #[cfg(feature = "wasm-runtime")]
        {
            let config = Config::builder()
                .with_common_config(CommonConfigOptions::default())
                .build()?;
            
            let vm = VmBuilder::new().with_config(config).build()?;
            
            Ok(Self {
                vm: Some(VmWrapper { vm }),
                loaded_plugins: HashMap::new(),
            })
        }
        
        #[cfg(not(feature = "wasm-runtime"))]
        {
            // WASM runtime support not compiled
            Ok(Self {
                vm: None,
                loaded_plugins: HashMap::new(),
            })
        }
    }

    /// 加载 OpenClaw WASM 插件
    pub async fn load_plugin(&mut self, _plugin_path: &str) -> Result<(), CompatError> {
        // 1. 读取 manifest
        let manifest_path = format!("{}.manifest.json", _plugin_path);
        let manifest_content = std::fs::read_to_string(&manifest_path)
            .map_err(|e| CompatError::IoError(e))?;
        
        let manifest: crate::OpenClawSkillManifest = serde_json::from_str(&manifest_content)?;
        
        // 2. 加载 WASM 模块
        #[cfg(feature = "wasm-runtime")]
        {
            if let Some(ref mut vm_wrapper) = self.vm {
                vm_wrapper.vm.load_wasm_from_file(_plugin_path)?;
            }
        }
        
        // 3. 缓存插件信息
        let plugin = LoadedPlugin {
            manifest,
            module_path: _plugin_path.to_string(),
            cached_functions: HashMap::new(),
        };
        
        self.loaded_plugins.insert(_plugin_path.to_string(), plugin);
        
        Ok(())
    }

    /// 执行 WASM 技能
    pub async fn execute_skill(
        &mut self,
        _skill_name: &str,
        _args: &Value,
    ) -> Result<WasmResult, CompatError> {
        #[cfg(feature = "wasm-runtime")]
        {
            if let Some(vm_wrapper) = &mut self.vm {
                // 1. 找到包含该技能的插件
                let plugin = self.find_plugin_for_skill(_skill_name)?;
                
                // 2. 准备执行参数
                let args_json = serde_json::to_string(_args)
                    .map_err(|e| CompatError::SerializationError(e))?;
                
                // 3. 调用 WASM 函数
                let result = vm_wrapper.vm.run_func(
                    Some("plugin"),
                    "skill_execute",
                    params!(args_json),
                );

                // 4. 处理执行结果
                match result {
                    Ok(vals) => {
                        if let Some(val) = vals.get(0) {
                            let output = self.wasm_val_to_string(val)?;
                            let parsed: Value = serde_json::from_str(&output)
                                .unwrap_or_else(|_| Value::String(output));
                            
                            return Ok(WasmResult {
                                ok: true,
                                output: parsed.to_string(),
                                error: None,
                            });
                        }
                    }
                    Err(_e) => {
                        // WASM 执行失败
                        return Ok(WasmResult {
                            ok: false,
                            output: String::new(),
                            error: Some(format!("WASM 执行错误: {:?}", e)),
                        });
                    }
                }
            }
        }
        
        Err(CompatError::WasmExecutionError("WASM runtime not available".to_string()))
    }

    /// 获取所有已加载的技能
    pub fn get_loaded_skills(&self) -> Vec<String> {
        let mut skills = Vec::new();
        for plugin in self.loaded_plugins.values() {
            for skill in &plugin.manifest.skills {
                skills.push(skill.name.clone());
            }
        }
        skills
    }

    /// 检查技能是否存在
    pub fn has_skill(&self, skill_name: &str) -> bool {
        self.loaded_plugins.values().any(|plugin| {
            plugin.manifest.skills.iter().any(|skill| skill.name == skill_name)
        })
    }

    // ── 私有方法 ────────────────────────────────────────────────────────────────

    #[cfg(feature = "wasm-runtime")]
    fn extract_manifest(&self, vm: &wasmedge_sdk::Vm) -> Result<crate::OpenClawSkillManifest, CompatError> {
        // 调用 WASM 模块的 skill_manifest 函数
        let result = vm.run_func(Some("plugin"), "skill_manifest", params!());
        
        match result {
            Ok(vals) => {
                if let Some(val) = vals.get(0) {
                    let manifest_json = self.wasm_val_to_string(val)?;
                    let manifest: crate::OpenClawSkillManifest = serde_json::from_str(&manifest_json)
                        .map_err(|e| CompatError::SerializationError(e))?;
                    return Ok(manifest);
                }
            }
            Err(e) => {
                return Err(CompatError::WasmExecutionError(
                    format!("无法提取 manifest: {:?}", e)
                ));
            }
        }
        
        Err(CompatError::InvalidManifest("无法获取 manifest".to_string()))
    }

    fn find_plugin_for_skill(&self, skill_name: &str) -> Result<&LoadedPlugin, CompatError> {
        for plugin in self.loaded_plugins.values() {
            if plugin.manifest.skills.iter().any(|skill| skill.name == skill_name) {
                return Ok(plugin);
            }
        }
        Err(CompatError::SkillNotFound(skill_name.to_string()))
    }

    #[cfg(feature = "wasm-runtime")]
    fn wasm_val_to_string(&self, val: &WasmVal) -> Result<String, CompatError> {
        match val {
            WasmVal::I32(i) => Ok(i.to_string()),
            WasmVal::I64(i) => Ok(i.to_string()),
            WasmVal::F32(f) => Ok(f.to_string()),
            WasmVal::F64(f) => Ok(f.to_string()),
            WasmVal::V128(i) => Ok(i.to_string()),
            WasmVal::FuncRef(_) => Ok("[function]".to_string()),
            WasmVal::ExternRef(_) => Ok("[extern]".to_string()),
        }
    }
}

impl Default for WasmRuntime {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_e| {
            // Failed to create WASM runtime
            Self {
                vm: None,
                loaded_plugins: HashMap::new(),
            }
        })
    }
}

// ── 测试 ───────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_runtime_creation() {
        let runtime = WasmRuntime::new();
        assert!(runtime.is_ok());
    }

    #[test]
    fn test_skill_lookup() {
        let runtime = WasmRuntime::new().unwrap();
        assert!(!runtime.has_skill("nonexistent.skill"));
    }

    #[cfg(feature = "wasm-runtime")]
    #[test]
    fn test_plugin_loading() {
        // 这个测试需要一个真实的 WASM 文件
        // 在实际测试中，应该创建一个测试用的 WASM 插件
        let mut runtime = WasmRuntime::new().unwrap();
        
        // 尝试加载不存在的文件应该失败
        let result = runtime.load_plugin("/nonexistent/plugin.wasm");
        assert!(result.is_err());
    }
}
