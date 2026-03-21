#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════
    // 输入验证测试 (Input Validation Tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_validate_empty_input() {
        let state = CliTerminalState::default();
        let result = state.validate_input();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Command cannot be empty");
    }

    #[test]
    fn test_validate_too_long_input() {
        let mut state = CliTerminalState::default();
        state.command_input = "a".repeat(1025);
        let result = state.validate_input();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Command too long (max 1024 characters)");
    }

    #[test]
    fn test_validate_null_character() {
        let mut state = CliTerminalState::default();
        state.command_input = "test\0command".to_string();
        let result = state.validate_input();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Command contains null character");
    }

    #[test]
    fn test_validate_command_injection_and() {
        let mut state = CliTerminalState::default();
        state.command_input = "cmd1 && cmd2".to_string();
        let result = state.validate_input();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Command chaining not allowed");
    }

    #[test]
    fn test_validate_command_injection_or() {
        let mut state = CliTerminalState::default();
        state.command_input = "cmd1 || cmd2".to_string();
        let result = state.validate_input();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Command chaining not allowed");
    }

    #[test]
    fn test_validate_command_injection_semicolon() {
        let mut state = CliTerminalState::default();
        state.command_input = "cmd1; cmd2".to_string();
        let result = state.validate_input();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Command chaining not allowed");
    }

    #[test]
    fn test_validate_valid_input() {
        let mut state = CliTerminalState::default();
        state.command_input = "help".to_string();
        let result = state.validate_input();
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_valid_input_with_args() {
        let mut state = CliTerminalState::default();
        state.command_input = "weather beijing".to_string();
        let result = state.validate_input();
        assert!(result.is_ok());
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 历史导航测试 (History Navigation Tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_history_navigation_empty() {
        let mut state = CliTerminalState::default();
        state.history.clear(); // Remove welcome message
        state.history_previous();
        assert_eq!(state.command_input, "");
        assert_eq!(state.history_index, None);
    }

    #[test]
    fn test_history_navigation_single_entry() {
        let mut state = CliTerminalState::default();
        state.history.clear(); // Remove welcome message
        
        state.add_to_history(CliHistoryEntry {
            command: "test".to_string(),
            output: vec![],
            timestamp: 0,
        });
        
        state.history_previous();
        assert_eq!(state.command_input, "test");
        assert_eq!(state.history_index, Some(0));
        
        state.history_next();
        assert_eq!(state.command_input, "");
        assert_eq!(state.history_index, None);
    }

    #[test]
    fn test_history_navigation_multiple_entries() {
        let mut state = CliTerminalState::default();
        state.history.clear(); // Remove welcome message
        
        state.add_to_history(CliHistoryEntry {
            command: "cmd1".to_string(),
            output: vec![],
            timestamp: 0,
        });
        state.add_to_history(CliHistoryEntry {
            command: "cmd2".to_string(),
            output: vec![],
            timestamp: 1,
        });
        state.add_to_history(CliHistoryEntry {
            command: "cmd3".to_string(),
            output: vec![],
            timestamp: 2,
        });
        
        // Navigate backwards
        state.history_previous();
        assert_eq!(state.command_input, "cmd3");
        
        state.history_previous();
        assert_eq!(state.command_input, "cmd2");
        
        state.history_previous();
        assert_eq!(state.command_input, "cmd1");
        
        // Try to go beyond oldest
        state.history_previous();
        assert_eq!(state.command_input, "cmd1");
        
        // Navigate forwards
        state.history_next();
        assert_eq!(state.command_input, "cmd2");
        
        state.history_next();
        assert_eq!(state.command_input, "cmd3");
        
        // Go back to current input
        state.history_next();
        assert_eq!(state.command_input, "");
    }

    #[test]
    fn test_history_navigation_preserves_input() {
        let mut state = CliTerminalState::default();
        state.history.clear(); // Remove welcome message
        
        state.add_to_history(CliHistoryEntry {
            command: "old_cmd".to_string(),
            output: vec![],
            timestamp: 0,
        });
        
        state.command_input = "new_input".to_string();
        
        // Navigate to history
        state.history_previous();
        assert_eq!(state.command_input, "old_cmd");
        
        // Return to current input
        state.history_next();
        assert_eq!(state.command_input, "new_input");
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 边界检查测试 (Bounds Checking Tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_history_bounds_checking() {
        let mut state = CliTerminalState::default();
        state.history.clear(); // Remove welcome message
        state.max_history_size = 3;
        
        for i in 0..5 {
            state.add_to_history(CliHistoryEntry {
                command: format!("cmd{}", i),
                output: vec![],
                timestamp: i as u64,
            });
        }
        
        assert_eq!(state.history.len(), 3);
        assert_eq!(state.history[0].command, "cmd2");
        assert_eq!(state.history[1].command, "cmd3");
        assert_eq!(state.history[2].command, "cmd4");
    }

    #[test]
    fn test_history_max_size_default() {
        let state = CliTerminalState::default();
        assert_eq!(state.max_history_size, 1000);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 自动补全测试 (Autocomplete Tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_autocomplete_exact_match() {
        let mut state = CliTerminalState::default();
        state.command_input = "help".to_string();
        state.update_autocomplete();
        assert_eq!(state.autocomplete_suggestion, None);
    }

    #[test]
    fn test_autocomplete_partial_match() {
        let mut state = CliTerminalState::default();
        state.command_input = "hel".to_string();
        state.update_autocomplete();
        assert_eq!(state.autocomplete_suggestion, Some("help".to_string()));
    }

    #[test]
    fn test_autocomplete_no_match() {
        let mut state = CliTerminalState::default();
        state.command_input = "xyz".to_string();
        state.update_autocomplete();
        assert_eq!(state.autocomplete_suggestion, None);
    }

    #[test]
    fn test_autocomplete_empty_input() {
        let mut state = CliTerminalState::default();
        state.command_input = "".to_string();
        state.update_autocomplete();
        assert_eq!(state.autocomplete_suggestion, None);
    }

    #[test]
    fn test_autocomplete_apply() {
        let mut state = CliTerminalState::default();
        state.command_input = "hel".to_string();
        state.update_autocomplete();
        
        assert_eq!(state.autocomplete_suggestion, Some("help".to_string()));
        
        state.apply_autocomplete();
        assert_eq!(state.command_input, "help");
        assert_eq!(state.autocomplete_suggestion, None);
    }

    #[test]
    fn test_autocomplete_multi_word() {
        let mut state = CliTerminalState::default();
        state.command_input = "agent l".to_string();
        state.update_autocomplete();
        assert_eq!(state.autocomplete_suggestion, Some("agent list".to_string()));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 超时检查测试 (Timeout Tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_timeout_not_started() {
        let state = CliTerminalState::default();
        assert_eq!(state.is_execution_timeout(), false);
    }

    #[test]
    fn test_timeout_default_value() {
        let state = CliTerminalState::default();
        assert_eq!(state.execution_timeout_secs, 30);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 清空历史测试 (Clear History Tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_clear_history() {
        let mut state = CliTerminalState::default();
        
        state.add_to_history(CliHistoryEntry {
            command: "cmd1".to_string(),
            output: vec![],
            timestamp: 0,
        });
        state.add_to_history(CliHistoryEntry {
            command: "cmd2".to_string(),
            output: vec![],
            timestamp: 1,
        });
        
        assert!(state.history.len() > 0);
        
        state.clear_history();
        
        assert_eq!(state.history.len(), 0);
        assert_eq!(state.history_index, None);
        assert_eq!(state.input_buffer, "");
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 状态一致性测试 (State Consistency Tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_add_to_history_resets_navigation() {
        let mut state = CliTerminalState::default();
        state.history.clear(); // Remove welcome message
        
        state.add_to_history(CliHistoryEntry {
            command: "cmd1".to_string(),
            output: vec![],
            timestamp: 0,
        });
        
        state.history_previous();
        assert_eq!(state.history_index, Some(0));
        
        state.add_to_history(CliHistoryEntry {
            command: "cmd2".to_string(),
            output: vec![],
            timestamp: 1,
        });
        
        assert_eq!(state.history_index, None);
        assert_eq!(state.input_buffer, "");
    }

    #[test]
    fn test_default_state() {
        let state = CliTerminalState::default();
        
        assert_eq!(state.command_input, "");
        assert!(state.history.len() > 0); // Has welcome message
        assert_eq!(state.history_index, None);
        assert_eq!(state.is_executing, false);
        assert_eq!(state.input_buffer, "");
        assert!(state.available_commands.len() > 0);
        assert_eq!(state.autocomplete_suggestion, None);
        assert_eq!(state.max_history_size, 1000);
        assert_eq!(state.execution_timeout_secs, 30);
        assert_eq!(state.last_execution_time, None);
    }

    #[test]
    fn test_available_commands_not_empty() {
        let state = CliTerminalState::default();
        assert!(state.available_commands.contains(&"help".to_string()));
        assert!(state.available_commands.contains(&"version".to_string()));
        assert!(state.available_commands.contains(&"status".to_string()));
        assert!(state.available_commands.contains(&"clear".to_string()));
        assert!(state.available_commands.contains(&"sysinfo".to_string()));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 命令执行集成测试 (Command Execution Integration Tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_command_list_completeness() {
        let state = CliTerminalState::default();
        let expected_commands = vec![
            "help", "version", "status", "clear",
            "agent", "agent list", "agent info",
            "gateway", "gateway status", "gateway url",
            "ai", "ai model", "ai status",
            "weather", "news", "sysinfo", "uptime", "whoami", "pwd", "env",
        ];
        
        for cmd in expected_commands {
            assert!(
                state.available_commands.contains(&cmd.to_string()),
                "Command '{}' should be in available_commands",
                cmd
            );
        }
    }

    #[test]
    fn test_autocomplete_all_commands() {
        let test_cases = vec![
            ("hel", Some("help")),
            ("ver", Some("version")),
            ("sta", Some("status")),
            ("cle", Some("clear")),
            ("age", Some("agent")),
            ("agent l", Some("agent list")),
            ("agent i", Some("agent info")),
            ("gate", Some("gateway")),
            ("gateway s", Some("gateway status")),
            ("gateway u", Some("gateway url")),
            ("ai m", Some("ai model")),
            ("ai s", Some("ai status")),
            ("wea", Some("weather")),
            ("new", Some("news")),
            ("sys", Some("sysinfo")),
            ("upt", Some("uptime")),
            ("who", Some("whoami")),
            ("pw", Some("pwd")),
            ("en", Some("env")),
            ("xyz", None), // No match
        ];

        for (input, expected) in test_cases {
            let mut state = CliTerminalState::default();
            state.command_input = input.to_string();
            state.update_autocomplete();
            
            assert_eq!(
                state.autocomplete_suggestion.as_deref(),
                expected,
                "Autocomplete for '{}' should be {:?}",
                input,
                expected
            );
        }
    }

    #[test]
    fn test_subcommand_validation() {
        let test_cases = vec![
            ("agent", false),      // Missing subcommand
            ("agent list", true),  // Valid
            ("agent info", true),  // Valid
            ("agent foo", true),   // Invalid subcommand but valid format
            ("gateway", false),    // Missing subcommand
            ("gateway status", true),
            ("gateway url", true),
            ("ai", false),         // Missing subcommand
            ("ai model", true),
            ("ai status", true),
        ];

        for (cmd, should_pass_validation) in test_cases {
            let mut state = CliTerminalState::default();
            state.command_input = cmd.to_string();
            let result = state.validate_input();
            
            if should_pass_validation {
                assert!(
                    result.is_ok(),
                    "Command '{}' should pass validation",
                    cmd
                );
            }
        }
    }

    #[test]
    fn test_command_parsing() {
        let test_cases = vec![
            ("help", vec!["help"]),
            ("agent list", vec!["agent", "list"]),
            ("gateway status", vec!["gateway", "status"]),
            ("weather beijing", vec!["weather", "beijing"]),
            ("weather new york", vec!["weather", "new", "york"]),
            ("echo hello world", vec!["echo", "hello", "world"]),
        ];

        for (cmd, expected_parts) in test_cases {
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            assert_eq!(
                parts, expected_parts,
                "Command '{}' should parse to {:?}",
                cmd, expected_parts
            );
        }
    }

    #[test]
    fn test_history_entry_structure() {
        let entry = CliHistoryEntry {
            command: "test command".to_string(),
            output: vec![
                ("line 1".to_string(), false),
                ("error line".to_string(), true),
            ],
            timestamp: 1234567890,
        };

        assert_eq!(entry.command, "test command");
        assert_eq!(entry.output.len(), 2);
        assert_eq!(entry.output[0].0, "line 1");
        assert_eq!(entry.output[0].1, false);
        assert_eq!(entry.output[1].0, "error line");
        assert_eq!(entry.output[1].1, true);
        assert_eq!(entry.timestamp, 1234567890);
    }

    #[test]
    fn test_error_output_format() {
        let mut state = CliTerminalState::default();
        state.history.clear();
        
        let error_entry = CliHistoryEntry {
            command: "invalid_command".to_string(),
            output: vec![
                ("Error: Command not found".to_string(), true),
                ("Hint: Type 'help' for available commands".to_string(), false),
            ],
            timestamp: 0,
        };
        
        state.add_to_history(error_entry);
        
        assert_eq!(state.history.len(), 1);
        assert_eq!(state.history[0].output.len(), 2);
        assert!(state.history[0].output[0].1); // First line is error
        assert!(!state.history[0].output[1].1); // Second line is not error
    }

    #[test]
    fn test_multiline_output_handling() {
        let mut state = CliTerminalState::default();
        state.history.clear();
        
        let multiline_entry = CliHistoryEntry {
            command: "help".to_string(),
            output: vec![
                ("Line 1".to_string(), false),
                ("Line 2".to_string(), false),
                ("Line 3".to_string(), false),
                ("Line 4".to_string(), false),
                ("Line 5".to_string(), false),
            ],
            timestamp: 0,
        };
        
        state.add_to_history(multiline_entry);
        
        assert_eq!(state.history.len(), 1);
        assert_eq!(state.history[0].output.len(), 5);
    }

    #[test]
    fn test_command_case_sensitivity() {
        let mut state = CliTerminalState::default();
        
        // Commands should be case-sensitive (lowercase expected)
        state.command_input = "HELP".to_string();
        state.update_autocomplete();
        assert_eq!(state.autocomplete_suggestion, None);
        
        state.command_input = "help".to_string();
        state.update_autocomplete();
        assert_eq!(state.autocomplete_suggestion, None); // Exact match, no suggestion
    }

    #[test]
    fn test_empty_output_handling() {
        let mut state = CliTerminalState::default();
        state.history.clear();
        
        let empty_output_entry = CliHistoryEntry {
            command: "clear".to_string(),
            output: vec![],
            timestamp: 0,
        };
        
        state.add_to_history(empty_output_entry);
        
        assert_eq!(state.history.len(), 1);
        assert_eq!(state.history[0].output.len(), 0);
    }

    #[test]
    fn test_special_characters_in_command() {
        let mut state = CliTerminalState::default();
        
        // Test with special characters that should be allowed
        state.command_input = "echo \"hello world\"".to_string();
        let result = state.validate_input();
        assert!(result.is_ok());
        
        // Test with path separators
        state.command_input = "cat /path/to/file.txt".to_string();
        let result = state.validate_input();
        assert!(result.is_ok());
    }

    #[test]
    fn test_whitespace_handling() {
        let mut state = CliTerminalState::default();
        
        // Leading/trailing whitespace should be handled by trim
        state.command_input = "  help  ".to_string();
        let result = state.validate_input();
        assert!(result.is_ok());
        
        // Multiple spaces between words
        state.command_input = "agent    list".to_string();
        let result = state.validate_input();
        assert!(result.is_ok());
    }

    #[test]
    fn test_command_with_arguments() {
        let mut state = CliTerminalState::default();
        
        // Weather command with city argument
        state.command_input = "weather beijing".to_string();
        let result = state.validate_input();
        assert!(result.is_ok());
        
        // Weather command with multi-word city
        state.command_input = "weather new york".to_string();
        let result = state.validate_input();
        assert!(result.is_ok());
        
        // Echo command with multiple arguments
        state.command_input = "echo hello world test".to_string();
        let result = state.validate_input();
        assert!(result.is_ok());
    }

    #[test]
    fn test_state_consistency_after_operations() {
        let mut state = CliTerminalState::default();
        let initial_history_len = state.history.len();
        
        // Add entry
        state.add_to_history(CliHistoryEntry {
            command: "test".to_string(),
            output: vec![],
            timestamp: 0,
        });
        assert_eq!(state.history.len(), initial_history_len + 1);
        
        // Clear history
        state.clear_history();
        assert_eq!(state.history.len(), 0);
        assert_eq!(state.history_index, None);
        assert_eq!(state.input_buffer, "");
        
        // State should be consistent
        assert_eq!(state.command_input, "");
        assert_eq!(state.is_executing, false);
    }
}
