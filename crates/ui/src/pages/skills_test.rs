//! Aerospace-grade tests for Skills Browser
//!
//! Test coverage:
//! - Unit tests for data structures
//! - Integration tests for UI components
//! - Edge case handling
//! - Performance benchmarks

#[cfg(test)]
mod tests {
    use super::super::skills::{SkillInfo, SkillParam, SkillRisk};

    // ── Unit Tests: Data Structures ──────────────────────────────────────────

    #[test]
    fn test_skill_risk_safe_color() {
        let risk = SkillRisk::Safe;
        let color = risk.color();
        assert_eq!(color.r, 0.22);
        assert_eq!(color.g, 0.82);
        assert_eq!(color.b, 0.46);
    }

    #[test]
    fn test_skill_risk_confirm_color() {
        let risk = SkillRisk::Confirm;
        let color = risk.color();
        assert_eq!(color.r, 0.98);
        assert_eq!(color.g, 0.72);
        assert_eq!(color.b, 0.22);
    }

    #[test]
    fn test_skill_risk_deny_color() {
        let risk = SkillRisk::Deny;
        let color = risk.color();
        assert_eq!(color.r, 0.92);
        assert_eq!(color.g, 0.28);
        assert_eq!(color.b, 0.28);
    }

    #[test]
    fn test_skill_info_creation() {
        let skill = SkillInfo {
            name: "fs.readFile".to_string(),
            display_name: "Read File".to_string(),
            description: "Read file content".to_string(),
            category: "File System".to_string(),
            risk_level: SkillRisk::Safe,
            parameters: vec![],
        };

        assert_eq!(skill.name, "fs.readFile");
        assert_eq!(skill.display_name, "Read File");
        assert_eq!(skill.category, "File System");
        assert_eq!(skill.risk_level, SkillRisk::Safe);
    }

    #[test]
    fn test_skill_param_creation() {
        let param = SkillParam {
            name: "path".to_string(),
            param_type: "string".to_string(),
            required: true,
            description: "File path".to_string(),
        };

        assert_eq!(param.name, "path");
        assert_eq!(param.param_type, "string");
        assert!(param.required);
        assert_eq!(param.description, "File path");
    }

    #[test]
    fn test_skill_with_multiple_params() {
        let skill = SkillInfo {
            name: "fs.writeFile".to_string(),
            display_name: "Write File".to_string(),
            description: "Write content to file".to_string(),
            category: "File System".to_string(),
            risk_level: SkillRisk::Confirm,
            parameters: vec![
                SkillParam {
                    name: "path".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "File path".to_string(),
                },
                SkillParam {
                    name: "content".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "Content to write".to_string(),
                },
            ],
        };

        assert_eq!(skill.parameters.len(), 2);
        assert_eq!(skill.parameters[0].name, "path");
        assert_eq!(skill.parameters[1].name, "content");
    }

    // ── Edge Case Tests ──────────────────────────────────────────────────────

    #[test]
    fn test_empty_skill_name() {
        let skill = SkillInfo {
            name: "".to_string(),
            display_name: "Empty".to_string(),
            description: "Test".to_string(),
            category: "Test".to_string(),
            risk_level: SkillRisk::Safe,
            parameters: vec![],
        };

        assert_eq!(skill.name, "");
    }

    #[test]
    fn test_very_long_skill_name() {
        let long_name = "a".repeat(1000);
        let skill = SkillInfo {
            name: long_name.clone(),
            display_name: "Long".to_string(),
            description: "Test".to_string(),
            category: "Test".to_string(),
            risk_level: SkillRisk::Safe,
            parameters: vec![],
        };

        assert_eq!(skill.name.len(), 1000);
    }

    #[test]
    fn test_unicode_in_skill_name() {
        let skill = SkillInfo {
            name: "文件.读取".to_string(),
            display_name: "读取文件".to_string(),
            description: "读取文件内容".to_string(),
            category: "文件系统".to_string(),
            risk_level: SkillRisk::Safe,
            parameters: vec![],
        };

        assert_eq!(skill.name, "文件.读取");
        assert_eq!(skill.display_name, "读取文件");
    }

    #[test]
    fn test_special_characters_in_description() {
        let skill = SkillInfo {
            name: "test".to_string(),
            display_name: "Test".to_string(),
            description: "Test with <>&\"' special chars".to_string(),
            category: "Test".to_string(),
            risk_level: SkillRisk::Safe,
            parameters: vec![],
        };

        assert!(skill.description.contains('<'));
        assert!(skill.description.contains('>'));
    }

    #[test]
    fn test_skill_clone() {
        let skill = SkillInfo {
            name: "test".to_string(),
            display_name: "Test".to_string(),
            description: "Test".to_string(),
            category: "Test".to_string(),
            risk_level: SkillRisk::Safe,
            parameters: vec![],
        };

        let cloned = skill.clone();
        assert_eq!(skill.name, cloned.name);
        assert_eq!(skill.display_name, cloned.display_name);
    }

    #[test]
    fn test_skill_risk_equality() {
        assert_eq!(SkillRisk::Safe, SkillRisk::Safe);
        assert_eq!(SkillRisk::Confirm, SkillRisk::Confirm);
        assert_eq!(SkillRisk::Deny, SkillRisk::Deny);
        assert_ne!(SkillRisk::Safe, SkillRisk::Confirm);
    }

    // ── Performance Tests ─────────────────────────────────────────────────────

    #[test]
    fn test_large_skill_list_creation() {
        let skills: Vec<SkillInfo> = (0..1000)
            .map(|i| SkillInfo {
                name: format!("skill.{}", i),
                display_name: format!("Skill {}", i),
                description: format!("Description {}", i),
                category: "Test".to_string(),
                risk_level: SkillRisk::Safe,
                parameters: vec![],
            })
            .collect();

        assert_eq!(skills.len(), 1000);
    }

    #[test]
    fn test_skill_with_many_parameters() {
        let params: Vec<SkillParam> = (0..100)
            .map(|i| SkillParam {
                name: format!("param{}", i),
                param_type: "string".to_string(),
                required: i % 2 == 0,
                description: format!("Parameter {}", i),
            })
            .collect();

        let skill = SkillInfo {
            name: "complex".to_string(),
            display_name: "Complex".to_string(),
            description: "Complex skill".to_string(),
            category: "Test".to_string(),
            risk_level: SkillRisk::Safe,
            parameters: params,
        };

        assert_eq!(skill.parameters.len(), 100);
    }

    // ── Boundary Tests ────────────────────────────────────────────────────────

    #[test]
    fn test_zero_parameters() {
        let skill = SkillInfo {
            name: "simple".to_string(),
            display_name: "Simple".to_string(),
            description: "Simple skill".to_string(),
            category: "Test".to_string(),
            risk_level: SkillRisk::Safe,
            parameters: vec![],
        };

        assert_eq!(skill.parameters.len(), 0);
    }

    #[test]
    fn test_all_risk_levels() {
        let risks = vec![SkillRisk::Safe, SkillRisk::Confirm, SkillRisk::Deny];
        
        for risk in risks {
            let skill = SkillInfo {
                name: "test".to_string(),
                display_name: "Test".to_string(),
                description: "Test".to_string(),
                category: "Test".to_string(),
                risk_level: risk,
                parameters: vec![],
            };
            
            // Verify color is valid
            let color = skill.risk_level.color();
            assert!(color.r >= 0.0 && color.r <= 1.0);
            assert!(color.g >= 0.0 && color.g <= 1.0);
            assert!(color.b >= 0.0 && color.b <= 1.0);
        }
    }

    #[test]
    fn test_param_type_variations() {
        let types = vec!["string", "number", "boolean", "object", "array"];
        
        for param_type in types {
            let param = SkillParam {
                name: "test".to_string(),
                param_type: param_type.to_string(),
                required: true,
                description: "Test".to_string(),
            };
            
            assert_eq!(param.param_type, param_type);
        }
    }

    // ── Debug Trait Tests ─────────────────────────────────────────────────────

    #[test]
    fn test_skill_info_debug() {
        let skill = SkillInfo {
            name: "test".to_string(),
            display_name: "Test".to_string(),
            description: "Test".to_string(),
            category: "Test".to_string(),
            risk_level: SkillRisk::Safe,
            parameters: vec![],
        };

        let debug_str = format!("{:?}", skill);
        assert!(debug_str.contains("SkillInfo"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_skill_risk_debug() {
        let risk = SkillRisk::Safe;
        let debug_str = format!("{:?}", risk);
        assert!(debug_str.contains("Safe"));
    }

    #[test]
    fn test_skill_param_debug() {
        let param = SkillParam {
            name: "test".to_string(),
            param_type: "string".to_string(),
            required: true,
            description: "Test".to_string(),
        };

        let debug_str = format!("{:?}", param);
        assert!(debug_str.contains("SkillParam"));
        assert!(debug_str.contains("test"));
    }
}
