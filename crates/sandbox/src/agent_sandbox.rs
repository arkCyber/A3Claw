//! # AgentSandboxFactory — per-Agent WasmEdge 沙盒实例化
//!
//! 从 [`AgentProfile`] 生成独立的 WasmEdge 沙盒配置，
//! 每个数字员工拥有完全隔离的沙盒实例：
//! - 独立的工作目录（`~/.openclaw-plus/agents/{id}/workspace/`）
//! - 独立的审计日志（`~/.openclaw-plus/agents/{id}/audit.log`）
//! - 独立的内存限制、文件系统挂载、网络白名单
//! - 独立的 SecurityConfig → Interceptor → SandboxRunner 实例

use anyhow::{Context, Result};
use openclaw_security::{AgentProfile, Interceptor, SecurityConfig};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

use crate::runner::SandboxRunner;
use crate::wasi_builder::WasiBuilder;

// ── AgentSandboxSpec ──────────────────────────────────────────────────────────

/// 从 AgentProfile 派生的沙盒实例化规格。
///
/// 包含启动一个 per-Agent WasmEdge 沙盒所需的所有参数。
pub struct AgentSandboxSpec {
    /// 数字员工 ID。
    pub agent_id: String,
    /// 数字员工显示名称（用于日志）。
    pub display_name: String,
    /// 从 AgentProfile 生成的 SecurityConfig。
    pub security_config: SecurityConfig,
    /// 沙盒工作目录（per-Agent 隔离）。
    pub workspace_dir: PathBuf,
    /// 审计日志路径（per-Agent 隔离）。
    pub audit_log_path: PathBuf,
    /// 内存限制（MB）。
    pub memory_limit_mb: u32,
    /// WASI 参数（由 WasiBuilder 生成）。
    pub wasi_args: crate::wasi_builder::WasiArgs,
}

impl AgentSandboxSpec {
    /// 从 AgentProfile 构建沙盒规格。
    pub fn from_profile(profile: &AgentProfile) -> Self {
        let security_config = profile.to_security_config();
        let workspace_dir = security_config.workspace_dir.clone();
        let audit_log_path = security_config.audit_log_path.clone();
        let memory_limit_mb = security_config.memory_limit_mb;

        let wasi_args = WasiBuilder::new(&security_config).build_wasi_args();

        Self {
            agent_id: profile.id.as_str().to_string(),
            display_name: profile.display_name.clone(),
            security_config,
            workspace_dir,
            audit_log_path,
            memory_limit_mb,
            wasi_args,
        }
    }

    /// 确保沙盒所需的目录结构存在。
    pub fn ensure_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(&self.workspace_dir)
            .with_context(|| format!("create workspace: {}", self.workspace_dir.display()))?;
        if let Some(parent) = self.audit_log_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create audit log dir: {}", parent.display()))?;
        }
        info!(
            agent_id = %self.agent_id,
            workspace = %self.workspace_dir.display(),
            "Sandbox directories ensured"
        );
        Ok(())
    }

    /// 返回沙盒规格的摘要（用于日志和审计）。
    pub fn summary(&self) -> SandboxSpecSummary {
        SandboxSpecSummary {
            agent_id: self.agent_id.clone(),
            display_name: self.display_name.clone(),
            memory_limit_mb: self.memory_limit_mb,
            fs_mount_count: self.security_config.fs_mounts.len(),
            network_allowlist_count: self.security_config.network_allowlist.len(),
            intercept_shell: self.security_config.intercept_shell,
            confirm_shell_exec: self.security_config.confirm_shell_exec,
            confirm_file_delete: self.security_config.confirm_file_delete,
            confirm_network: self.security_config.confirm_network,
            workspace_dir: self.workspace_dir.to_string_lossy().to_string(),
        }
    }
}

/// 沙盒规格摘要（可序列化，用于审计）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SandboxSpecSummary {
    pub agent_id: String,
    pub display_name: String,
    pub memory_limit_mb: u32,
    pub fs_mount_count: usize,
    pub network_allowlist_count: usize,
    pub intercept_shell: bool,
    pub confirm_shell_exec: bool,
    pub confirm_file_delete: bool,
    pub confirm_network: bool,
    pub workspace_dir: String,
}

// ── AgentSandboxFactory ───────────────────────────────────────────────────────

/// per-Agent WasmEdge 沙盒工厂。
///
/// 负责从 `AgentProfile` 实例化独立的 `SandboxRunner`，
/// 每个数字员工拥有完全隔离的沙盒环境。
pub struct AgentSandboxFactory;

impl AgentSandboxFactory {
    /// 从 AgentProfile 构建 `SandboxRunner`。
    ///
    /// # 步骤
    /// 1. 从 AgentProfile 生成 `SecurityConfig`
    /// 2. 构建 `AgentSandboxSpec`（含 WasiArgs）
    /// 3. 确保沙盒目录存在
    /// 4. 构建 `Interceptor`（含 PolicyEngine）
    /// 5. 返回 `SandboxRunner` 实例
    pub fn build(
        profile: &AgentProfile,
        event_tx: flume::Sender<openclaw_security::SandboxEvent>,
    ) -> Result<(SandboxRunner, AgentSandboxSpec)> {
        let spec = AgentSandboxSpec::from_profile(profile);
        spec.ensure_dirs()?;

        let summary = spec.summary();
        info!(
            agent_id = %summary.agent_id,
            memory_mb = summary.memory_limit_mb,
            mounts = summary.fs_mount_count,
            network = summary.network_allowlist_count,
            intercept_shell = summary.intercept_shell,
            "Building per-Agent sandbox"
        );

        let policy_engine = openclaw_security::PolicyEngine::new(spec.security_config.clone());
        let audit_log = openclaw_security::AuditLog::new(spec.security_config.audit_log_path.clone());
        let (_cmd_tx, cmd_rx) = flume::unbounded::<openclaw_security::ControlCommand>();
        let interceptor = Arc::new(Interceptor::new(
            policy_engine,
            audit_log,
            event_tx,
            cmd_rx,
        ));

        let runner = SandboxRunner::new(spec.security_config.clone(), interceptor);
        Ok((runner, spec))
    }

    /// 验证 AgentProfile 的沙盒配置是否合法（不启动沙盒）。
    ///
    /// 检查项：
    /// - 内存限制在合理范围内（32 MB ~ 16 GB）
    /// - 文件系统挂载路径格式合法
    /// - 网络白名单格式合法
    pub fn validate(profile: &AgentProfile) -> Result<Vec<ValidationWarning>> {
        let mut warnings = Vec::new();

        // 内存限制检查
        if profile.memory_limit_mb < 32 {
            anyhow::bail!(
                "Memory limit too low: {} MB (minimum 32 MB)",
                profile.memory_limit_mb
            );
        }
        if profile.memory_limit_mb > 16 * 1024 {
            warnings.push(ValidationWarning {
                field: "memory_limit_mb".to_string(),
                message: format!(
                    "Memory limit {} MB is very high (>16 GB). Consider reducing.",
                    profile.memory_limit_mb
                ),
            });
        }

        // 文件系统挂载检查
        for mount in &profile.fs_mounts {
            if mount.host_path.to_string_lossy().is_empty() {
                anyhow::bail!("FsMount has empty host_path");
            }
            if mount.guest_path.is_empty() {
                anyhow::bail!("FsMount has empty guest_path");
            }
            if !mount.host_path.is_absolute() {
                warnings.push(ValidationWarning {
                    field: "fs_mounts".to_string(),
                    message: format!(
                        "Mount host_path '{}' is not absolute",
                        mount.host_path.display()
                    ),
                });
            }
        }

        // 网络白名单检查
        for host in &profile.network_allowlist {
            if host.is_empty() {
                warnings.push(ValidationWarning {
                    field: "network_allowlist".to_string(),
                    message: "Empty entry in network_allowlist".to_string(),
                });
            }
            // 检查是否包含协议前缀（应该只是域名/IP）
            if host.starts_with("http://") || host.starts_with("https://") {
                warnings.push(ValidationWarning {
                    field: "network_allowlist".to_string(),
                    message: format!(
                        "Entry '{}' should be a hostname/domain, not a URL",
                        host
                    ),
                });
            }
        }

        // 能力集合风险检查
        let high_risk_caps: Vec<&str> = profile.capabilities.iter()
            .filter(|c| c.risk_level >= 3)
            .map(|c| c.id.as_str())
            .collect();
        if !high_risk_caps.is_empty() {
            warnings.push(ValidationWarning {
                field: "capabilities".to_string(),
                message: format!(
                    "High-risk capabilities enabled: {}. Ensure human-in-the-loop is configured.",
                    high_risk_caps.join(", ")
                ),
            });
        }

        // 安全配置一致性检查
        if !profile.intercept_shell && profile.confirm_shell_exec {
            warnings.push(ValidationWarning {
                field: "intercept_shell/confirm_shell_exec".to_string(),
                message: "confirm_shell_exec=true has no effect when intercept_shell=false".to_string(),
            });
        }

        Ok(warnings)
    }
}

/// 沙盒配置验证警告。
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    pub field: String,
    pub message: String,
}

impl std::fmt::Display for ValidationWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.field, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openclaw_security::{AgentProfile, AgentRole, AgentCapability, FsMount};
    use std::path::PathBuf;

    fn make_profile(name: &str) -> AgentProfile {
        AgentProfile::new(name, AgentRole::TicketAssistant, "acme", "admin")
    }

    #[test]
    fn test_spec_from_profile() {
        let p = make_profile("工单助手");
        let spec = AgentSandboxSpec::from_profile(&p);
        assert_eq!(spec.agent_id, p.id.as_str());
        assert_eq!(spec.display_name, "工单助手");
        assert_eq!(spec.memory_limit_mb, 256);
        assert!(spec.workspace_dir.to_string_lossy().contains(p.id.as_str()));
        assert!(spec.audit_log_path.to_string_lossy().ends_with("audit.log"));
    }

    #[test]
    fn test_spec_summary() {
        let p = make_profile("测试助手");
        let spec = AgentSandboxSpec::from_profile(&p);
        let summary = spec.summary();
        assert_eq!(summary.agent_id, p.id.as_str());
        assert_eq!(summary.memory_limit_mb, 256);
        assert!(summary.intercept_shell);
        assert!(summary.confirm_shell_exec);
    }

    #[test]
    fn test_validate_ok() {
        let p = make_profile("合规助手");
        let warnings = AgentSandboxFactory::validate(&p).unwrap();
        assert!(warnings.is_empty(), "Default profile should have no warnings");
    }

    #[test]
    fn test_validate_memory_too_low() {
        let mut p = make_profile("低内存助手");
        p.memory_limit_mb = 16;
        let result = AgentSandboxFactory::validate(&p);
        assert!(result.is_err(), "Should fail with memory < 32 MB");
    }

    #[test]
    fn test_validate_high_memory_warning() {
        let mut p = make_profile("高内存助手");
        p.memory_limit_mb = 20 * 1024;
        let warnings = AgentSandboxFactory::validate(&p).unwrap();
        assert!(warnings.iter().any(|w| w.field == "memory_limit_mb"));
    }

    #[test]
    fn test_validate_url_in_allowlist_warning() {
        let mut p = make_profile("网络助手");
        p.network_allowlist.push("https://api.example.com".to_string());
        let warnings = AgentSandboxFactory::validate(&p).unwrap();
        assert!(warnings.iter().any(|w| w.field == "network_allowlist"));
    }

    #[test]
    fn test_validate_high_risk_capability_warning() {
        let mut p = make_profile("高风险助手");
        p.add_capability(AgentCapability::new("shell.exec", "Shell Exec", 3));
        let warnings = AgentSandboxFactory::validate(&p).unwrap();
        assert!(warnings.iter().any(|w| w.field == "capabilities"));
    }

    #[test]
    fn test_validate_intercept_consistency_warning() {
        let mut p = make_profile("配置不一致助手");
        p.intercept_shell = false;
        p.confirm_shell_exec = true;
        let warnings = AgentSandboxFactory::validate(&p).unwrap();
        assert!(warnings.iter().any(|w| w.field.contains("intercept_shell")));
    }

    #[test]
    fn test_validate_relative_mount_warning() {
        let mut p = make_profile("相对路径助手");
        p.fs_mounts.push(FsMount {
            host_path: PathBuf::from("relative/path"),
            guest_path: "/data".to_string(),
            readonly: true,
        });
        let warnings = AgentSandboxFactory::validate(&p).unwrap();
        assert!(warnings.iter().any(|w| w.field == "fs_mounts"));
    }

    #[test]
    fn test_security_config_isolation() {
        let p1 = make_profile("助手-1");
        let p2 = make_profile("助手-2");
        let spec1 = AgentSandboxSpec::from_profile(&p1);
        let spec2 = AgentSandboxSpec::from_profile(&p2);
        // 每个 Agent 的工作目录必须完全隔离
        assert_ne!(spec1.workspace_dir, spec2.workspace_dir);
        assert_ne!(spec1.audit_log_path, spec2.audit_log_path);
        assert_ne!(spec1.agent_id, spec2.agent_id);
    }

    #[test]
    fn test_wasi_args_generated() {
        let p = make_profile("WASI测试助手");
        let spec = AgentSandboxSpec::from_profile(&p);
        // WASI args 应该至少包含 wasmedge_quickjs.wasm
        assert!(!spec.wasi_args.args.is_empty());
        assert!(spec.wasi_args.args[0].contains("wasmedge_quickjs"));
        // 环境变量应该包含 OPENCLAW_SANDBOX
        assert!(spec.wasi_args.envs.iter().any(|e| e.starts_with("OPENCLAW_SANDBOX")));
        assert!(spec.wasi_args.envs.iter().any(|e| e.starts_with("OPENCLAW_MEMORY_LIMIT")));
    }
}
