// OpenClaw+ 集成测试
// 测试完整的端到端流程：AI 推理 + 沙箱 + 安全拦截

#[cfg(test)]
mod integration_tests {
    use openclaw_security::{AgentProfile, AgentRole};
    use std::path::PathBuf;

    #[test]
    fn test_agent_profile_creation() {
        let profile = AgentProfile::new(
            "测试客服助手",
            AgentRole::CustomerSupport,
            "test-user",
            "system"
        );
        
        assert_eq!(profile.display_name, "测试客服助手");
        assert_eq!(profile.role, AgentRole::CustomerSupport);
        assert_eq!(profile.memory_limit_mb, 256);
        assert!(profile.intercept_shell);
        assert!(profile.confirm_file_delete);
        assert!(profile.capabilities.is_empty());
    }

    #[test]
    fn test_agent_profile_save_load() {
        let mut profile = AgentProfile::new(
            "测试助手",
            AgentRole::TicketAssistant,
            "test-user",
            "system"
        );
        
        profile.description = "集成测试用数字员工".to_string();
        
        // 保存到临时目录
        let result = profile.save();
        assert!(result.is_ok(), "Failed to save profile: {:?}", result.err());
        
        // 加载并验证
        let loaded = AgentProfile::load(profile.id.as_str());
        assert!(loaded.is_ok(), "Failed to load profile: {:?}", loaded.err());
        
        let loaded = loaded.unwrap();
        assert_eq!(loaded.display_name, "测试助手");
        assert_eq!(loaded.description, "集成测试用数字员工");
    }

    #[test]
    fn test_agent_security_config_conversion() {
        let profile = AgentProfile::new(
            "安全测试",
            AgentRole::SecurityAuditor,
            "test-user",
            "system"
        );
        
        let security_config = profile.to_security_config();
        
        assert_eq!(security_config.memory_limit_mb, profile.memory_limit_mb);
        assert_eq!(security_config.intercept_shell, profile.intercept_shell);
        assert_eq!(security_config.confirm_file_delete, profile.confirm_file_delete);
        assert_eq!(security_config.network_allowlist, profile.network_allowlist);
    }

    #[test]
    fn test_agent_capabilities_management() {
        use openclaw_security::AgentCapability;
        
        let mut profile = AgentProfile::new(
            "能力测试",
            AgentRole::DataAnalyst,
            "test-user",
            "system"
        );
        
        // 添加能力
        let cap = AgentCapability {
            id: "file_read".to_string(),
            name: "文件读取".to_string(),
            description: "读取文件内容".to_string(),
            risk_level: "low".to_string(),
            enabled: true,
        };
        
        profile.add_capability(cap.clone());
        assert_eq!(profile.capabilities.len(), 1);
        assert_eq!(profile.capabilities[0].id, "file_read");
        
        // 移除能力
        profile.remove_capability("file_read");
        assert_eq!(profile.capabilities.len(), 0);
    }

    #[test]
    fn test_agent_network_allowlist() {
        let mut profile = AgentProfile::new(
            "网络测试",
            AgentRole::CustomerSupport,
            "test-user",
            "system"
        );
        
        profile.allow_network("api.openai.com");
        profile.allow_network("api.anthropic.com");
        
        assert!(profile.is_network_allowed("api.openai.com"));
        assert!(profile.is_network_allowed("api.anthropic.com"));
        assert!(!profile.is_network_allowed("malicious.com"));
    }

    #[test]
    fn test_agent_stats_tracking() {
        let mut profile = AgentProfile::new(
            "统计测试",
            AgentRole::CodeReviewer,
            "test-user",
            "system"
        );
        
        assert_eq!(profile.stats.total_runs, 0);
        assert_eq!(profile.stats.successful_runs, 0);
        
        profile.record_run_success(120);
        assert_eq!(profile.stats.total_runs, 1);
        assert_eq!(profile.stats.successful_runs, 1);
        assert_eq!(profile.stats.total_runtime_secs, 120);
        
        profile.record_run_failure(60);
        assert_eq!(profile.stats.total_runs, 2);
        assert_eq!(profile.stats.failed_runs, 1);
        assert_eq!(profile.stats.total_runtime_secs, 180);
        
        let success_rate = profile.stats.success_rate();
        assert!((success_rate - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_agent_role_display() {
        let roles = vec![
            (AgentRole::TicketAssistant, "🎫", "工单助手"),
            (AgentRole::CodeReviewer, "🔍", "代码审查员"),
            (AgentRole::CustomerSupport, "💬", "客服助手"),
            (AgentRole::SecurityAuditor, "🛡", "安全审计员"),
        ];
        
        for (role, emoji, zh_name) in roles {
            assert_eq!(role.role_emoji(), emoji);
            assert_eq!(role.display_zh(), zh_name);
            assert!(!role.default_avatar_url().is_empty());
        }
    }
}
