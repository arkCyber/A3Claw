use crate::app::AppMessage;
use crate::theme::Language;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

/// CLI Terminal command history entry
#[derive(Debug, Clone)]
pub struct CliHistoryEntry {
    pub command: String,
    pub output: Vec<(String, bool)>, // (line, is_error)
    pub timestamp: u64,
}

/// CLI Terminal state - Aerospace-grade implementation
pub struct CliTerminalState {
    /// Current command input buffer
    pub command_input: String,
    /// Command execution history (bounded to prevent memory overflow)
    pub history: Vec<CliHistoryEntry>,
    /// Current position in history navigation (None = not navigating)
    pub history_index: Option<usize>,
    /// Command execution state flag
    pub is_executing: bool,
    /// Temporary buffer to preserve current input when navigating history
    pub input_buffer: String,
    /// List of all available commands for autocomplete
    pub available_commands: Vec<String>,
    /// Current autocomplete suggestion
    pub autocomplete_suggestion: Option<String>,
    /// Maximum history entries (prevent unbounded growth)
    pub max_history_size: usize,
    /// Command execution timeout in seconds
    pub execution_timeout_secs: u64,
    /// Last command execution timestamp
    pub last_execution_time: Option<std::time::Instant>,
    /// Scrollable ID for auto-scroll functionality
    pub scroll_id: widget::Id,
    /// Input field ID for auto-focus
    pub input_id: widget::Id,
}

impl Default for CliTerminalState {
    fn default() -> Self {
        let mut state = Self {
            command_input: String::new(),
            history: Vec::new(),
            history_index: None,
            is_executing: false,
            input_buffer: String::new(),
            available_commands: vec![
                "help".to_string(),
                "version".to_string(),
                "status".to_string(),
                "clear".to_string(),
                "agent".to_string(),
                "agent list".to_string(),
                "agent info".to_string(),
                "gateway".to_string(),
                "gateway status".to_string(),
                "gateway url".to_string(),
                "ai".to_string(),
                "ai model".to_string(),
                "ai status".to_string(),
                "weather".to_string(),
                "news".to_string(),
                "sysinfo".to_string(),
                "uptime".to_string(),
                "whoami".to_string(),
                "pwd".to_string(),
                "env".to_string(),
            ],
            autocomplete_suggestion: None,
            max_history_size: 1000, // Aerospace-grade: bounded history
            execution_timeout_secs: 30, // 30 second timeout
            last_execution_time: None,
            scroll_id: widget::Id::unique(), // Unique ID for scrollable widget
            input_id: widget::Id::unique(), // Unique ID for input field (auto-focus)
        };
        
        // Add welcome message to history (aerospace-grade: informative startup)
        state.add_welcome_message();
        state
    }
}

impl CliTerminalState {
    /// Add welcome message on first launch (aerospace-grade initialization)
    fn add_welcome_message(&mut self) {
        let welcome_entry = CliHistoryEntry {
            command: String::new(),
            output: vec![
                ("╔══════════════════════════════════════════════════════════════════╗".to_string(), false),
                ("║          OpenClaw+ CLI Terminal - Aerospace Grade v1.0          ║".to_string(), false),
                ("╚══════════════════════════════════════════════════════════════════╝".to_string(), false),
                ("".to_string(), false),
                ("Welcome to OpenClaw+ Command Line Interface".to_string(), false),
                ("".to_string(), false),
                ("System Information:".to_string(), false),
                (format!("  • Platform:       {}", std::env::consts::OS), false),
                (format!("  • Architecture:   {}", std::env::consts::ARCH), false),
                (format!("  • Version:        v0.1.0"), false),
                (format!("  • Build:          Debug (Aerospace-grade)"), false),
                ("".to_string(), false),
                ("Quick Start:".to_string(), false),
                ("  • Type 'help' to see all available commands".to_string(), false),
                ("  • Use ↑↓ to navigate command history".to_string(), false),
                ("  • Press Tab for command auto-completion".to_string(), false),
                ("  • Type 'clear' to clear the terminal".to_string(), false),
                ("".to_string(), false),
                ("Ready for commands...".to_string(), false),
                ("".to_string(), false),
            ],
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };
        self.history.push(welcome_entry);
    }

    /// Navigate to previous command in history (↑ key)
    pub fn history_previous(&mut self) {
        if self.history.is_empty() {
            return;
        }

        // Save current input if starting navigation
        if self.history_index.is_none() {
            self.input_buffer = self.command_input.clone();
        }

        let new_index = match self.history_index {
            None => Some(self.history.len() - 1),
            Some(idx) if idx > 0 => Some(idx - 1),
            Some(idx) => Some(idx), // Already at oldest
        };

        if let Some(idx) = new_index {
            self.history_index = new_index;
            self.command_input = self.history[idx].command.clone();
        }
    }

    /// Navigate to next command in history (↓ key)
    pub fn history_next(&mut self) {
        if self.history.is_empty() {
            return;
        }

        match self.history_index {
            None => {}, // Not navigating
            Some(idx) if idx < self.history.len() - 1 => {
                let new_idx = idx + 1;
                self.history_index = Some(new_idx);
                self.command_input = self.history[new_idx].command.clone();
            },
            Some(_) => {
                // At newest, restore original input
                self.history_index = None;
                self.command_input = self.input_buffer.clone();
            }
        }
    }

    /// Add command to history with bounds checking
    pub fn add_to_history(&mut self, entry: CliHistoryEntry) {
        // Aerospace-grade: enforce maximum history size
        if self.history.len() >= self.max_history_size {
            self.history.remove(0); // Remove oldest entry
        }
        self.history.push(entry);
        self.history_index = None; // Reset navigation
        self.input_buffer.clear();
    }

    /// Find autocomplete suggestion for current input
    pub fn update_autocomplete(&mut self) {
        let input = self.command_input.trim();
        if input.is_empty() {
            self.autocomplete_suggestion = None;
            return;
        }

        // Find first matching command
        self.autocomplete_suggestion = self.available_commands
            .iter()
            .find(|cmd| cmd.starts_with(input) && cmd.as_str() != input)
            .cloned();
    }

    /// Apply autocomplete suggestion
    pub fn apply_autocomplete(&mut self) {
        if let Some(suggestion) = &self.autocomplete_suggestion {
            self.command_input = suggestion.clone();
            self.autocomplete_suggestion = None;
        }
    }

    /// Validate command input (aerospace-grade input validation)
    pub fn validate_input(&self) -> Result<(), String> {
        let input = self.command_input.trim();
        
        // Check for empty input
        if input.is_empty() {
            return Err("Command cannot be empty".to_string());
        }

        // Check for maximum command length (prevent buffer overflow)
        if input.len() > 1024 {
            return Err("Command too long (max 1024 characters)".to_string());
        }

        // Check for invalid characters (aerospace-grade security)
        if input.contains('\0') {
            return Err("Command contains null character".to_string());
        }

        // Check for command injection attempts
        if input.contains("&&") || input.contains("||") || input.contains(";") {
            return Err("Command chaining not allowed".to_string());
        }

        Ok(())
    }

    /// Clear all history (with confirmation in production)
    pub fn clear_history(&mut self) {
        self.history.clear();
        self.history_index = None;
        self.input_buffer.clear();
    }

    /// Check if command execution has timed out
    pub fn is_execution_timeout(&self) -> bool {
        if let Some(start_time) = self.last_execution_time {
            start_time.elapsed().as_secs() > self.execution_timeout_secs
        } else {
            false
        }
    }
}

pub struct CliTerminalPage;

impl CliTerminalPage {
    pub fn view<'a>(
        lang: Language,
        state: &'a CliTerminalState,
    ) -> Element<'a, AppMessage> {
        // Terminal color scheme - professional and readable
        let color_prompt = cosmic::iced::Color::from_rgb(0.28, 0.92, 0.78);    // Cyan for $
        let color_command = cosmic::iced::Color::from_rgb(0.98, 0.98, 0.98);   // White for commands
        let color_error = cosmic::iced::Color::from_rgb(0.98, 0.38, 0.38);     // Red for errors
        let color_output = cosmic::iced::Color::from_rgb(0.78, 0.88, 0.98);    // Light blue for output
        let color_muted = cosmic::iced::Color::from_rgb(0.52, 0.50, 0.48);     // Gray for hints

        // Header
        let header = widget::row::with_children(vec![
            widget::text("⚡").size(20).into(),
            widget::Space::new(10.0, 0.0).into(),
            widget::text(crate::theme::t(lang, "CLI Terminal", "CLI 终端"))
                .size(22)
                .font(cosmic::font::bold())
                .into(),
            widget::Space::new(Length::Fill, 0.0).into(),
            widget::text(crate::theme::t(
                lang,
                "Type 'help' for available commands",
                "输入 'help' 查看可用命令",
            ))
            .size(11)
            .class(cosmic::theme::Text::Color(color_muted))
            .into(),
        ])
        .spacing(6)
        .align_y(Alignment::Center);

        // Build terminal content - history + current input (inline)
        let mut terminal_widgets: Vec<Element<AppMessage>> = Vec::new();

        // Add command history
        for entry in &state.history {
            // Command line with prompt
            terminal_widgets.push(
                widget::row::with_children(vec![
                    widget::text("$ ")
                        .size(14)
                        .font(cosmic::font::mono())
                        .class(cosmic::theme::Text::Color(color_prompt))
                        .into(),
                    widget::text(&entry.command)
                        .size(14)
                        .font(cosmic::font::mono())
                        .class(cosmic::theme::Text::Color(color_command))
                        .into(),
                ])
                .spacing(0)
                .into(),
            );

            // Output lines
            for (line, is_error) in &entry.output {
                let color = if *is_error { color_error } else { color_output };
                terminal_widgets.push(
                    widget::text(line.as_str())
                        .size(13)
                        .font(cosmic::font::mono())
                        .class(cosmic::theme::Text::Color(color))
                        .into(),
                );
            }

            // Small spacing between entries
            terminal_widgets.push(widget::Space::new(0.0, 4.0).into());
        }

        // Current input line (inline)
        terminal_widgets.push(
            widget::row::with_children(vec![
                widget::text("$ ")
                    .size(14)
                    .font(cosmic::font::mono())
                    .class(cosmic::theme::Text::Color(color_prompt))
                    .into(),
                {
                    let input = widget::text_input("", &state.command_input)
                        .id(state.input_id.clone())
                        .on_input(AppMessage::CliInputChanged)
                        .on_submit(|_| AppMessage::CliExecuteCommand)
                        .font(cosmic::font::mono())
                        .size(14)
                        .width(Length::Fill)
                        .padding(0);
                    input.into()
                },
            ])
            .spacing(0)
            .into(),
        );

        // Autocomplete hint
        if let Some(suggestion) = &state.autocomplete_suggestion {
            terminal_widgets.push(
                widget::text(format!("  → Tab: {}", suggestion))
                    .size(11)
                    .font(cosmic::font::mono())
                    .class(cosmic::theme::Text::Color(color_muted))
                    .into(),
            );
        }

        // Scrollable terminal area (all content inline)
        let terminal_scroll = widget::scrollable(
            widget::column::with_children(terminal_widgets)
                .spacing(2)
                .padding([16, 20]),
        )
        .id(state.scroll_id.clone())
        .height(Length::Fill);

        // Status bar with helpful info
        let status_content = if state.is_executing {
            widget::row::with_children(vec![
                widget::text("⟳")
                    .size(13)
                    .class(cosmic::theme::Text::Color(color_prompt))
                    .into(),
                widget::Space::new(8.0, 0.0).into(),
                widget::text(crate::theme::t(lang, "Executing...", "执行中..."))
                    .size(12)
                    .class(cosmic::theme::Text::Color(color_prompt))
                    .into(),
            ])
            .spacing(0)
        } else {
            widget::row::with_children(vec![
                widget::text("●")
                    .size(13)
                    .class(cosmic::theme::Text::Color(color_prompt))
                    .into(),
                widget::Space::new(8.0, 0.0).into(),
                widget::text(crate::theme::t(lang, "Ready", "就绪"))
                    .size(12)
                    .into(),
                widget::Space::new(Length::Fill, 0.0).into(),
                widget::text(crate::theme::t(
                    lang,
                    "↑↓ History | Tab Complete | Enter Execute | Ctrl+L Clear",
                    "↑↓ 历史 | Tab 补全 | Enter 执行 | Ctrl+L 清空",
                ))
                .size(10)
                .class(cosmic::theme::Text::Color(color_muted))
                .into(),
            ])
            .spacing(0)
        };

        let status_bar = widget::container(status_content)
            .padding([6, 20])
            .class(cosmic::theme::Container::Card);

        // Main layout - header + terminal + status
        widget::column::with_children(vec![
            header.into(),
            widget::Space::new(0.0, 8.0).into(),
            widget::container(terminal_scroll)
                .class(cosmic::theme::Container::Card)
                .height(Length::Fill)
                .into(),
            widget::Space::new(0.0, 8.0).into(),
            status_bar.into(),
        ])
        .spacing(0)
        .padding([16, 20])
        .height(Length::Fill)
        .into()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Unit Tests - Aerospace-grade testing
// ═══════════════════════════════════════════════════════════════════════════

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
    fn test_validate_command_injection() {
        let mut state = CliTerminalState::default();
        state.command_input = "cmd1 && cmd2".to_string();
        assert!(state.validate_input().is_err());
        
        state.command_input = "cmd1 || cmd2".to_string();
        assert!(state.validate_input().is_err());
        
        state.command_input = "cmd1; cmd2".to_string();
        assert!(state.validate_input().is_err());
    }

    #[test]
    fn test_validate_valid_input() {
        let mut state = CliTerminalState::default();
        state.command_input = "help".to_string();
        assert!(state.validate_input().is_ok());
        
        state.command_input = "weather beijing".to_string();
        assert!(state.validate_input().is_ok());
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 历史导航测试 (History Navigation Tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_history_navigation_empty() {
        let mut state = CliTerminalState::default();
        state.history.clear();
        state.history_previous();
        assert_eq!(state.command_input, "");
    }

    #[test]
    fn test_history_navigation_single() {
        let mut state = CliTerminalState::default();
        state.history.clear();
        
        state.add_to_history(CliHistoryEntry {
            command: "test".to_string(),
            output: vec![],
            timestamp: 0,
        });
        
        state.history_previous();
        assert_eq!(state.command_input, "test");
        
        state.history_next();
        assert_eq!(state.command_input, "");
    }

    #[test]
    fn test_history_bounds_checking() {
        let mut state = CliTerminalState::default();
        state.history.clear();
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
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 自动补全测试 (Autocomplete Tests)
    // ═══════════════════════════════════════════════════════════════════════

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
    fn test_autocomplete_apply() {
        let mut state = CliTerminalState::default();
        state.command_input = "hel".to_string();
        state.update_autocomplete();
        state.apply_autocomplete();
        assert_eq!(state.command_input, "help");
    }
}
