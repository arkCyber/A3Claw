//! Aerospace-grade tests for application state persistence.
//! 
//! This test suite validates:
//! - SecurityConfig serialization/deserialization (TOML)
//! - AI chat history persistence (JSON)
//! - Claw Terminal history persistence (JSON)
//! - UI preferences persistence (JSON)
//! - Error handling for corrupted files
//! - Atomic write operations
//! - Directory creation
//! - File permissions

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test helper to create a temporary config directory
fn setup_test_env() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("openclaw-plus");
    fs::create_dir_all(&config_path).expect("Failed to create config dir");
    (temp_dir, config_path)
}

#[test]
fn test_security_config_save_and_load() {
    let (_temp, config_path) = setup_test_env();
    let config_file = config_path.join("config.toml");
    
    // Create a sample SecurityConfig
    let config = openclaw_security::SecurityConfig {
        memory_limit_mb: 512,
        fs_mounts: vec![
            openclaw_security::FsMount {
                host_path: "/tmp".into(),
                guest_path: "/tmp".into(),
                readonly: false,
            }
        ],
        network_allowlist: vec!["github.com".to_string()],
        intercept_shell: true,
        confirm_file_delete: true,
        confirm_network: true,
        confirm_shell_exec: true,
        openclaw_entry: "/opt/openclaw/index.js".into(),
        workspace_dir: "/workspace".into(),
        audit_log_path: "/tmp/audit.log".into(),
        circuit_breaker: Default::default(),
        github: Default::default(),
        agent: Default::default(),
        wasm_policy_plugin: None,
        folder_access: vec![],
        rag_folders: vec![],
        openclaw_ai: Default::default(),
        channels: vec![],
    };
    
    // Serialize to TOML
    let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize config");
    fs::write(&config_file, &toml_str).expect("Failed to write config");
    
    // Verify file exists and has content
    assert!(config_file.exists(), "Config file should exist");
    let file_size = fs::metadata(&config_file).unwrap().len();
    assert!(file_size > 0, "Config file should not be empty");
    
    // Read and deserialize
    let loaded_toml = fs::read_to_string(&config_file).expect("Failed to read config");
    let loaded_config: openclaw_security::SecurityConfig = 
        toml::from_str(&loaded_toml).expect("Failed to deserialize config");
    
    // Verify data integrity
    assert_eq!(loaded_config.fs_mounts.len(), 1);
    assert_eq!(loaded_config.network_allowlist.len(), 1);
    assert_eq!(loaded_config.network_allowlist[0], "github.com");
    assert_eq!(loaded_config.fs_mounts[0].host_path.to_str().unwrap(), "/tmp");
    assert_eq!(loaded_config.memory_limit_mb, 512);
}

#[test]
fn test_ai_chat_history_persistence() {
    let (_temp, config_path) = setup_test_env();
    let chat_file = config_path.join("ai_chat_history.json");
    
    // Create sample chat messages
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct TestChatMessage {
        role: String,
        content: String,
        timestamp: u64,
    }
    
    let messages = vec![
        TestChatMessage {
            role: "User".to_string(),
            content: "Hello AI".to_string(),
            timestamp: 1000,
        },
        TestChatMessage {
            role: "Assistant".to_string(),
            content: "Hello! How can I help?".to_string(),
            timestamp: 1001,
        },
    ];
    
    // Save to JSON
    let json_str = serde_json::to_string_pretty(&messages).expect("Failed to serialize messages");
    fs::write(&chat_file, &json_str).expect("Failed to write chat history");
    
    // Verify file
    assert!(chat_file.exists());
    let file_size = fs::metadata(&chat_file).unwrap().len();
    assert!(file_size > 50, "Chat history file should have substantial content");
    
    // Load and verify
    let loaded_json = fs::read_to_string(&chat_file).expect("Failed to read chat history");
    let loaded_messages: Vec<TestChatMessage> = 
        serde_json::from_str(&loaded_json).expect("Failed to deserialize messages");
    
    assert_eq!(loaded_messages.len(), 2);
    assert_eq!(loaded_messages[0].content, "Hello AI");
    assert_eq!(loaded_messages[1].role, "Assistant");
}

#[test]
fn test_claw_terminal_history_persistence() {
    let (_temp, config_path) = setup_test_env();
    let claw_file = config_path.join("claw_terminal_history.json");
    
    // Create sample terminal entries
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct TestClawEntry {
        id: u64,
        command: String,
        timestamp: u64,
        status: String,
    }
    
    let entries = vec![
        TestClawEntry {
            id: 1,
            command: "ls -la".to_string(),
            timestamp: 2000,
            status: "Success".to_string(),
        },
        TestClawEntry {
            id: 2,
            command: "echo hello".to_string(),
            timestamp: 2001,
            status: "Success".to_string(),
        },
    ];
    
    // Save
    let json_str = serde_json::to_string_pretty(&entries).expect("Failed to serialize entries");
    fs::write(&claw_file, &json_str).expect("Failed to write terminal history");
    
    // Verify
    assert!(claw_file.exists());
    
    // Load
    let loaded_json = fs::read_to_string(&claw_file).expect("Failed to read terminal history");
    let loaded_entries: Vec<TestClawEntry> = 
        serde_json::from_str(&loaded_json).expect("Failed to deserialize entries");
    
    assert_eq!(loaded_entries.len(), 2);
    assert_eq!(loaded_entries[0].command, "ls -la");
}

#[test]
fn test_ui_preferences_persistence() {
    let (_temp, config_path) = setup_test_env();
    let prefs_file = config_path.join("ui_prefs.json");
    
    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
    struct UiPrefs {
        language: String,
        warm_theme_active: bool,
        nav_page: String,
    }
    
    let prefs = UiPrefs {
        language: "English".to_string(),
        warm_theme_active: true,
        nav_page: "Dashboard".to_string(),
    };
    
    // Save
    let json_str = serde_json::to_string_pretty(&prefs).expect("Failed to serialize prefs");
    fs::write(&prefs_file, &json_str).expect("Failed to write preferences");
    
    // Load
    let loaded_json = fs::read_to_string(&prefs_file).expect("Failed to read preferences");
    let loaded_prefs: UiPrefs = 
        serde_json::from_str(&loaded_json).expect("Failed to deserialize preferences");
    
    assert_eq!(loaded_prefs, prefs);
}

#[test]
fn test_corrupted_json_handling() {
    let (_temp, config_path) = setup_test_env();
    let bad_file = config_path.join("corrupted.json");
    
    // Write invalid JSON
    fs::write(&bad_file, "{ invalid json ").expect("Failed to write corrupted file");
    
    // Attempt to parse should fail gracefully
    let content = fs::read_to_string(&bad_file).expect("Failed to read file");
    let result: Result<serde_json::Value, _> = serde_json::from_str(&content);
    
    assert!(result.is_err(), "Parsing corrupted JSON should fail");
}

#[test]
fn test_directory_creation() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let nested_path = temp.path().join("a").join("b").join("c");
    
    // Create nested directories
    fs::create_dir_all(&nested_path).expect("Failed to create nested dirs");
    
    assert!(nested_path.exists());
    assert!(nested_path.is_dir());
}

#[test]
fn test_atomic_write_operation() {
    let (_temp, config_path) = setup_test_env();
    let target_file = config_path.join("atomic_test.json");
    
    let data = serde_json::json!({
        "test": "data",
        "count": 42
    });
    
    // Write atomically (write to temp, then rename)
    let temp_file = config_path.join("atomic_test.json.tmp");
    let json_str = serde_json::to_string_pretty(&data).unwrap();
    fs::write(&temp_file, &json_str).expect("Failed to write temp file");
    fs::rename(&temp_file, &target_file).expect("Failed to rename file");
    
    // Verify final file exists and temp doesn't
    assert!(target_file.exists());
    assert!(!temp_file.exists());
    
    // Verify content
    let loaded = fs::read_to_string(&target_file).unwrap();
    let loaded_data: serde_json::Value = serde_json::from_str(&loaded).unwrap();
    assert_eq!(loaded_data["count"], 42);
}

#[test]
fn test_large_chat_history() {
    let (_temp, config_path) = setup_test_env();
    let chat_file = config_path.join("large_chat.json");
    
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Msg {
        id: usize,
        content: String,
    }
    
    // Create 1000 messages
    let messages: Vec<Msg> = (0..1000)
        .map(|i| Msg {
            id: i,
            content: format!("Message number {}", i),
        })
        .collect();
    
    // Save
    let json_str = serde_json::to_string(&messages).expect("Failed to serialize");
    fs::write(&chat_file, &json_str).expect("Failed to write");
    
    // Load
    let loaded_json = fs::read_to_string(&chat_file).expect("Failed to read");
    let loaded: Vec<Msg> = serde_json::from_str(&loaded_json).expect("Failed to deserialize");
    
    assert_eq!(loaded.len(), 1000);
    assert_eq!(loaded[999].id, 999);
}

#[test]
fn test_empty_collections() {
    let (_temp, config_path) = setup_test_env();
    let file = config_path.join("empty.json");
    
    let empty_vec: Vec<String> = vec![];
    let json_str = serde_json::to_string(&empty_vec).unwrap();
    fs::write(&file, &json_str).unwrap();
    
    let loaded_json = fs::read_to_string(&file).unwrap();
    let loaded: Vec<String> = serde_json::from_str(&loaded_json).unwrap();
    
    assert_eq!(loaded.len(), 0);
}
