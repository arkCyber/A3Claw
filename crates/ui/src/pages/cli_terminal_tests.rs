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
}
