//! Skill Manager - 技能加载、注册和调用系统
//!
//! 负责从 ~/.openclaw/skills/ 目录加载 WASM 技能模块，
//! 并提供统一的技能调用接口。

use anyhow::{anyhow, Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// 技能清单信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub skills: Vec<SkillInfo>,
}

/// 单个技能信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub name: String,
    pub display: String,
    pub description: String,
    pub risk: String,
    pub params: Vec<SkillParam>,
}

/// 技能参数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillParam {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: String,
    pub description: String,
    pub required: bool,
}

/// 技能执行请求
#[derive(Debug, Serialize, Deserialize)]
pub struct SkillRequest {
    pub skill: String,
    pub args: serde_json::Value,
    pub request_id: String,
}

/// 技能执行响应
#[derive(Debug, Serialize, Deserialize)]
pub struct SkillResponse {
    pub request_id: String,
    pub ok: bool,
    pub output: String,
    pub error: String,
}

/// 技能管理器
pub struct SkillManager {
    /// 已加载的技能清单
    manifests: Arc<RwLock<HashMap<String, SkillManifest>>>,
    /// 技能名称到插件ID的映射
    skill_to_plugin: Arc<RwLock<HashMap<String, String>>>,
    /// 技能目录路径
    skills_dir: PathBuf,
}

impl SkillManager {
    /// 创建新的技能管理器
    pub fn new() -> Result<Self> {
        let skills_dir = dirs::home_dir()
            .ok_or_else(|| anyhow!("Cannot determine home directory"))?
            .join(".openclaw")
            .join("skills");

        // 确保技能目录存在
        if !skills_dir.exists() {
            std::fs::create_dir_all(&skills_dir)
                .context("Failed to create skills directory")?;
        }

        Ok(Self {
            manifests: Arc::new(RwLock::new(HashMap::new())),
            skill_to_plugin: Arc::new(RwLock::new(HashMap::new())),
            skills_dir,
        })
    }

    /// 扫描并加载所有技能
    pub fn load_all_skills(&self) -> Result<usize> {
        let mut count = 0;

        if !self.skills_dir.exists() {
            tracing::warn!("Skills directory does not exist: {:?}", self.skills_dir);
            return Ok(0);
        }

        for entry in std::fs::read_dir(&self.skills_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // 查找 manifest.json
                let manifest_path = path.join("manifest.json");
                if manifest_path.exists() {
                    match self.load_skill_manifest(&manifest_path) {
                        Ok(_) => count += 1,
                        Err(e) => {
                            tracing::warn!("Failed to load skill from {:?}: {}", path, e);
                        }
                    }
                }
            }
        }

        tracing::info!("Loaded {} skill plugins", count);
        Ok(count)
    }

    /// 加载单个技能清单
    fn load_skill_manifest(&self, manifest_path: &PathBuf) -> Result<()> {
        let content = std::fs::read_to_string(manifest_path)
            .context("Failed to read manifest file")?;

        let manifest: SkillManifest = serde_json::from_str(&content)
            .context("Failed to parse manifest JSON")?;

        let plugin_id = manifest.id.clone();

        // 注册所有技能
        {
            let mut skill_map = self.skill_to_plugin.write();
            for skill in &manifest.skills {
                skill_map.insert(skill.name.clone(), plugin_id.clone());
            }
        }

        // 保存清单
        {
            let mut manifests = self.manifests.write();
            manifests.insert(plugin_id.clone(), manifest);
        }

        tracing::debug!("Loaded skill plugin: {}", plugin_id);
        Ok(())
    }

    /// 获取所有已加载的技能列表
    pub fn list_skills(&self) -> Vec<SkillInfo> {
        let manifests = self.manifests.read();
        let mut skills = Vec::new();

        for manifest in manifests.values() {
            skills.extend(manifest.skills.clone());
        }

        skills
    }

    /// 获取技能信息
    pub fn get_skill_info(&self, skill_name: &str) -> Option<SkillInfo> {
        let skill_map = self.skill_to_plugin.read();
        let plugin_id = skill_map.get(skill_name)?;

        let manifests = self.manifests.read();
        let manifest = manifests.get(plugin_id)?;

        manifest.skills.iter()
            .find(|s| s.name == skill_name)
            .cloned()
    }

    /// 检查技能是否存在
    pub fn has_skill(&self, skill_name: &str) -> bool {
        self.skill_to_plugin.read().contains_key(skill_name)
    }

    /// 获取技能所属的插件ID
    pub fn get_plugin_id(&self, skill_name: &str) -> Option<String> {
        self.skill_to_plugin.read().get(skill_name).cloned()
    }

    /// 获取已加载的插件数量
    pub fn plugin_count(&self) -> usize {
        self.manifests.read().len()
    }

    /// 获取已加载的技能数量
    pub fn skill_count(&self) -> usize {
        self.skill_to_plugin.read().len()
    }

    /// 获取技能目录路径
    pub fn skills_dir(&self) -> &PathBuf {
        &self.skills_dir
    }
}

impl Default for SkillManager {
    fn default() -> Self {
        Self::new().expect("Failed to create SkillManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_manager_creation() {
        let manager = SkillManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    fn test_skill_manager_default() {
        let manager = SkillManager::default();
        assert_eq!(manager.plugin_count(), 0);
        assert_eq!(manager.skill_count(), 0);
    }

    #[test]
    fn test_skills_directory() {
        let manager = SkillManager::new().unwrap();
        let skills_dir = manager.skills_dir();
        assert!(skills_dir.to_str().unwrap().contains(".openclaw"));
        assert!(skills_dir.to_str().unwrap().contains("skills"));
    }

    #[test]
    fn test_load_all_skills() {
        let manager = SkillManager::new().unwrap();
        // 即使目录为空也应该成功
        let result = manager.load_all_skills();
        assert!(result.is_ok());
    }
}
