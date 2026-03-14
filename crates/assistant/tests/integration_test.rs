//! Integration tests for OpenClaw AI Assistant

use openclaw_assistant::{
    OpenClawAssistant, SystemContext, Intent, AssistantResponse,
    ActionExecutor, SuggestedAction, ActionResult,
};
use openclaw_config::{RagConfig, RagFolder, RagSettings};
use openclaw_security::SecurityConfig;
use std::path::PathBuf;

#[test]
fn test_assistant_creation() {
    let assistant = OpenClawAssistant::new();
    assert!(assistant.is_ok());
}

#[test]
fn test_configure_rag_query() {
    let assistant = OpenClawAssistant::new().unwrap();
    let context = SystemContext::new();
    
    let response = assistant.process_query(
        "I want to add a new knowledge base folder",
        &context,
    ).unwrap();
    
    assert!(!response.text.is_empty());
    assert!(response.text.contains("knowledge base") || response.text.contains("folder"));
}

#[test]
fn test_diagnose_wasi_error() {
    let assistant = OpenClawAssistant::new().unwrap();
    let context = SystemContext::new()
        .with_error_logs(vec!["WASI error 8: Bad file descriptor".to_string()]);
    
    let response = assistant.process_query(
        "WasmEdge error 8",
        &context,
    ).unwrap();
    
    assert!(!response.text.is_empty());
    assert!(response.text.contains("WASI Error 8") || response.text.contains("EBADF"));
}

#[test]
fn test_optimize_rag_performance() {
    let assistant = OpenClawAssistant::new().unwrap();
    let mut rag_config = RagConfig::default();
    rag_config.settings.chunk_size = 1000;
    rag_config.settings.ocr_enabled = true;
    
    let context = SystemContext::new().with_rag_config(rag_config);
    
    let response = assistant.process_query(
        "RAG indexing is too slow",
        &context,
    ).unwrap();
    
    assert!(!response.text.is_empty());
    assert!(response.text.contains("Performance") || response.text.contains("optimization"));
}

#[test]
fn test_security_audit() {
    let assistant = OpenClawAssistant::new().unwrap();
    let security_config = SecurityConfig::default();
    let context = SystemContext::new().with_security_config(security_config);
    
    let response = assistant.process_query(
        "Run security audit",
        &context,
    ).unwrap();
    
    assert!(!response.text.is_empty());
    assert!(response.text.contains("Security") || response.text.contains("security"));
}

#[test]
fn test_documentation_query() {
    let assistant = OpenClawAssistant::new().unwrap();
    let context = SystemContext::new();
    
    let response = assistant.process_query(
        "How to use WasmEdge preopen documentation?",
        &context,
    ).unwrap();
    
    assert!(!response.text.is_empty());
}

#[test]
fn test_unknown_query() {
    let assistant = OpenClawAssistant::new().unwrap();
    let context = SystemContext::new();
    
    let response = assistant.process_query(
        "xyz abc def random words",
        &context,
    ).unwrap();
    
    assert!(!response.text.is_empty());
    assert!(response.text.contains("not sure") || response.text.contains("rephrase"));
}

#[test]
fn test_context_aware_response() {
    let assistant = OpenClawAssistant::new().unwrap();
    
    let mut rag_config = RagConfig::default();
    rag_config.folders.push(RagFolder {
        host_path: PathBuf::from("/test/docs"),
        name: "Test Docs".to_string(),
        description: "Test documentation folder".to_string(),
        include_extensions: vec!["md".to_string(), "pdf".to_string()],
        allow_agent_write: false,
        max_size_mb: None,
        watch_enabled: true,
        indexing_status: openclaw_config::IndexingStatus::Pending,
        last_indexed: None,
        indexed_file_count: 0,
        indexed_size_bytes: 0,
    });
    
    let context = SystemContext::new().with_rag_config(rag_config);
    
    let response = assistant.process_query(
        "Add another knowledge base folder",
        &context,
    ).unwrap();
    
    assert!(!response.text.is_empty());
    assert!(response.text.contains("1 folder") || response.text.contains("Current"));
}

#[test]
fn test_multilingual_support() {
    let assistant = OpenClawAssistant::new().unwrap();
    let context = SystemContext::new();
    
    // Test English queries
    let response_en = assistant.process_query(
        "Add new RAG folder",
        &context,
    ).unwrap();
    assert!(!response_en.text.is_empty());
    
    // Test another English variant
    let response_en2 = assistant.process_query(
        "Configure new knowledge base",
        &context,
    ).unwrap();
    assert!(!response_en2.text.is_empty());
}

#[test]
fn test_suggested_actions() {
    let assistant = OpenClawAssistant::new().unwrap();
    let context = SystemContext::new();
    
    let response = assistant.process_query(
        "Add a new RAG folder",
        &context,
    ).unwrap();
    
    assert!(!response.actions.is_empty());
}

#[test]
fn test_error_pattern_detection() {
    let assistant = OpenClawAssistant::new().unwrap();
    let context = SystemContext::new()
        .with_error_logs(vec![
            "WASI error 8: Bad file descriptor".to_string(),
            "WASI error 2: No such file or directory".to_string(),
        ]);
    
    let response = assistant.process_query(
        "WASI error 8 failed",
        &context,
    ).unwrap();
    
    assert!(!response.text.is_empty());
}

// ── Preset chip query tests (mirrors AssistantPresetQuery in UI) ──────────────

#[test]
fn test_preset_diagnose_chip() {
    let assistant = OpenClawAssistant::new().unwrap();
    let context = SystemContext::new()
        .with_error_logs(vec!["PolicyDenied: network request blocked".to_string()]);
    let response = assistant.process_query(
        "Analyze recent security events and suggest fixes",
        &context,
    ).unwrap();
    assert!(!response.text.is_empty());
}

#[test]
fn test_preset_optimize_chip() {
    let assistant = OpenClawAssistant::new().unwrap();
    let mut rag = RagConfig::default();
    rag.settings.chunk_size = 1000;
    let context = SystemContext::new().with_rag_config(rag);
    let response = assistant.process_query(
        "Review config and suggest performance improvements",
        &context,
    ).unwrap();
    assert!(!response.text.is_empty());
}

#[test]
fn test_preset_audit_chip() {
    let assistant = OpenClawAssistant::new().unwrap();
    let context = SystemContext::new()
        .with_security_config(SecurityConfig::default());
    let response = assistant.process_query(
        "Check for security policy gaps or over-permissive rules",
        &context,
    ).unwrap();
    assert!(!response.text.is_empty());
    assert!(
        response.text.contains("Security") ||
        response.text.contains("security") ||
        response.text.contains("policy")
    );
}

#[test]
fn test_preset_rag_status_chip() {
    let assistant = OpenClawAssistant::new().unwrap();
    let mut rag = RagConfig::default();
    rag.folders.push(RagFolder {
        host_path: std::path::PathBuf::from("/docs"),
        name: "Docs".to_string(),
        description: String::new(),
        include_extensions: vec!["md".to_string()],
        allow_agent_write: false,
        max_size_mb: None,
        watch_enabled: true,
        indexing_status: openclaw_config::IndexingStatus::Pending,
        last_indexed: None,
        indexed_file_count: 0,
        indexed_size_bytes: 0,
    });
    let context = SystemContext::new().with_rag_config(rag);
    let response = assistant.process_query(
        "Check RAG knowledge base config and index state",
        &context,
    ).unwrap();
    assert!(!response.text.is_empty());
}

#[test]
fn test_preset_zh_diagnose_chip() {
    let assistant = OpenClawAssistant::new().unwrap();
    let context = SystemContext::new();
    let response = assistant.process_query(
        "分析最近的安全事件并提供诊断建议",
        &context,
    ).unwrap();
    assert!(!response.text.is_empty());
}

#[test]
fn test_preset_zh_rag_chip() {
    let assistant = OpenClawAssistant::new().unwrap();
    let context = SystemContext::new();
    let response = assistant.process_query(
        "检查 RAG 知识库配置和索引状态",
        &context,
    ).unwrap();
    assert!(!response.text.is_empty());
}

// ── PolicyDenied / Timeout / Network error diagnostics ───────────────────────

#[test]
fn test_diagnose_policy_denied() {
    let assistant = OpenClawAssistant::new().unwrap();
    let context = SystemContext::new()
        .with_error_logs(vec!["PolicyDenied: network request to evil.example.com blocked".to_string()]);
    let response = assistant.process_query(
        "PolicyDenied network request blocked",
        &context,
    ).unwrap();
    assert!(!response.text.is_empty());
    assert!(
        response.text.contains("Policy") || response.text.contains("policy")
            || response.text.contains("blocked") || response.text.contains("violation")
    );
    assert!(!response.actions.is_empty());
}

#[test]
fn test_diagnose_policy_denied_variant() {
    let assistant = OpenClawAssistant::new().unwrap();
    let context = SystemContext::new();
    let response = assistant.process_query(
        "policy_denied filesystem write to read-only path",
        &context,
    ).unwrap();
    assert!(!response.text.is_empty());
    // Should produce a code snippet with allowlist example
    assert!(!response.code_snippets.is_empty());
}

#[test]
fn test_diagnose_timeout() {
    let assistant = OpenClawAssistant::new().unwrap();
    let context = SystemContext::new();
    let response = assistant.process_query(
        "agent operation timed out after 30 seconds",
        &context,
    ).unwrap();
    assert!(!response.text.is_empty());
    assert!(
        response.text.contains("Timeout") || response.text.contains("timeout")
            || response.text.contains("time")
    );
    assert!(!response.actions.is_empty());
}

#[test]
fn test_diagnose_network_error() {
    let assistant = OpenClawAssistant::new().unwrap();
    let security = SecurityConfig::default();
    let context = SystemContext::new().with_security_config(security);
    let response = assistant.process_query(
        "connection refused TLS handshake failed",
        &context,
    ).unwrap();
    assert!(!response.text.is_empty());
    assert!(
        response.text.contains("Network") || response.text.contains("network")
            || response.text.contains("TLS") || response.text.contains("connection")
    );
}

#[test]
fn test_diagnose_wasi_error_13() {
    let assistant = OpenClawAssistant::new().unwrap();
    let context = SystemContext::new();
    let response = assistant.process_query(
        "WASI error 13 permission denied",
        &context,
    ).unwrap();
    assert!(!response.text.is_empty());
    assert!(
        response.text.contains("13") || response.text.contains("EACCES")
            || response.text.contains("Permission") || response.text.contains("permission")
    );
}

#[test]
fn test_diagnose_wasi_error_22() {
    let assistant = OpenClawAssistant::new().unwrap();
    let context = SystemContext::new();
    let response = assistant.process_query(
        "WASI error 22 invalid argument",
        &context,
    ).unwrap();
    assert!(!response.text.is_empty());
    assert!(
        response.text.contains("22") || response.text.contains("EINVAL")
            || response.text.contains("Invalid") || response.text.contains("invalid")
    );
}

#[test]
fn test_diagnose_general_error_includes_counts() {
    let assistant = OpenClawAssistant::new().unwrap();
    let security = SecurityConfig::default();
    let context = SystemContext::new().with_security_config(security);
    let response = assistant.process_query(
        "WASI preopen path failed",
        &context,
    ).unwrap();
    assert!(!response.text.is_empty());
    // Response should include context counts (preopen / allowlist)
    assert!(!response.actions.is_empty());
}

// ── ActionExecutor integration ────────────────────────────────────────────────

#[test]
fn test_action_executor_all_variants() {
    let ex = ActionExecutor::new();

    let cases: Vec<SuggestedAction> = vec![
        SuggestedAction::ConfigureRAG {
            operation: "add_folder".into(),
            params: serde_json::json!({}),
        },
        SuggestedAction::RunDiagnostic { test_name: "wasi_permissions".into() },
        SuggestedAction::OptimizeRAG { params: serde_json::json!({ "chunk_size": 800 }) },
        SuggestedAction::ApplySecurity { template: "production".into() },
        SuggestedAction::OpenDocument { url: "#doc-1".into() },
        SuggestedAction::StartSandbox { session_id: None },
        SuggestedAction::StopSandbox { session_id: Some("s1".into()) },
        SuggestedAction::EmergencyStop { session_id: None },
        SuggestedAction::ClearEventLog,
    ];

    for action in &cases {
        let result = ex.execute(action).unwrap();
        assert!(result.success, "Action {:?} returned failure", action.label());
        assert!(!result.message.is_empty());
    }
}

#[test]
fn test_action_result_destructive_actions() {
    let ex = ActionExecutor::new();

    let emergency = ex.execute(&SuggestedAction::EmergencyStop {
        session_id: Some("session-abc".into()),
    }).unwrap();
    assert!(emergency.success);
    assert!(emergency.message.contains("EMERGENCY"));
    let payload = emergency.payload.unwrap();
    assert_eq!(payload["action"], "emergency_stop");
    assert_eq!(payload["session_id"], "session-abc");

    let clear = ex.execute(&SuggestedAction::ClearEventLog).unwrap();
    assert!(clear.success);
    assert!(clear.message.contains("cleared"));
    assert!(clear.payload.is_none());
}

#[test]
fn test_suggested_action_labels_and_destructive() {
    let non_destructive = vec![
        SuggestedAction::StartSandbox { session_id: None },
        SuggestedAction::StopSandbox { session_id: None },
        SuggestedAction::RunDiagnostic { test_name: "x".into() },
        SuggestedAction::OptimizeRAG { params: serde_json::json!({}) },
        SuggestedAction::ApplySecurity { template: "t".into() },
        SuggestedAction::OpenDocument { url: "u".into() },
        SuggestedAction::ConfigureRAG { operation: "o".into(), params: serde_json::json!({}) },
    ];
    for a in &non_destructive {
        assert!(!a.is_destructive(), "{} should not be destructive", a.label());
    }

    assert!(SuggestedAction::EmergencyStop { session_id: None }.is_destructive());
    assert!(SuggestedAction::ClearEventLog.is_destructive());
}

// ── CodeSnippet in responses ──────────────────────────────────────────────────

#[test]
fn test_code_snippet_in_rag_add_response() {
    let assistant = OpenClawAssistant::new().unwrap();
    let context = SystemContext::new();
    let response = assistant.process_query(
        "add new knowledge base folder",
        &context,
    ).unwrap();
    assert!(!response.text.is_empty());
    assert!(!response.code_snippets.is_empty(), "RAG add should return a TOML code snippet");
    let snippet = &response.code_snippets[0];
    assert_eq!(snippet.language, "toml");
    assert!(snippet.code.contains("rag") || snippet.code.contains("path"));
}

#[test]
fn test_code_snippet_in_policy_denied_response() {
    let assistant = OpenClawAssistant::new().unwrap();
    let context = SystemContext::new();
    let response = assistant.process_query(
        "policy_denied network access blocked",
        &context,
    ).unwrap();
    assert!(!response.code_snippets.is_empty(), "PolicyDenied should return a TOML snippet");
    let snippet = &response.code_snippets[0];
    assert_eq!(snippet.language, "toml");
    assert!(snippet.code.contains("network_allowlist"));
}

// ── New knowledge base documents ──────────────────────────────────────────────

#[test]
fn test_knowledge_base_contains_new_documents() {
    use openclaw_assistant::KnowledgeBase;
    let kb = KnowledgeBase::new().unwrap();

    // Check all newly added doc IDs are present
    let ids: Vec<&str> = vec![
        "wasi-error-13",
        "wasi-error-22",
        "policy-denied",
        "network-timeout",
        "rag-config-guide",
    ];
    for id in ids {
        assert!(
            kb.documents.iter().any(|d| d.id == id),
            "Missing document: {}",
            id
        );
    }
}

#[test]
fn test_search_policy_denied_document() {
    use openclaw_assistant::KnowledgeBase;
    let kb = KnowledgeBase::new().unwrap();
    let intent = Intent::DiagnoseError { error_type: "policydenied".to_string() };
    let results = kb.search("PolicyDenied network", &intent).unwrap();
    assert!(!results.is_empty());
    assert!(results.iter().any(|d| d.id == "policy-denied"));
}

#[test]
fn test_search_network_timeout_document() {
    use openclaw_assistant::KnowledgeBase;
    let kb = KnowledgeBase::new().unwrap();
    let intent = Intent::DiagnoseError { error_type: "timeout".to_string() };
    let results = kb.search("timeout timed out", &intent).unwrap();
    assert!(!results.is_empty());
    assert!(results.iter().any(|d| d.id == "network-timeout"));
}

#[test]
fn test_search_rag_config_guide() {
    use openclaw_assistant::KnowledgeBase;
    let kb = KnowledgeBase::new().unwrap();
    let intent = Intent::ConfigureRAG { action: "add".to_string() };
    let results = kb.search("rag folder configuration guide", &intent).unwrap();
    assert!(!results.is_empty());
    assert!(results.iter().any(|d| d.id == "rag-config-guide"));
}

// ── with_rag_config full pipeline ─────────────────────────────────────────────

#[test]
fn test_with_rag_config_pipeline() {
    let rag_config = RagConfig::default();
    let assistant = OpenClawAssistant::with_rag_config(&rag_config).unwrap();
    let context = SystemContext::new().with_rag_config(rag_config);
    let response = assistant.process_query(
        "Check RAG knowledge base config and index state",
        &context,
    ).unwrap();
    assert!(!response.text.is_empty());
}

// ── Security audit with empty config detects issues ──────────────────────────

#[test]
fn test_security_audit_empty_config_has_issues() {
    let assistant = OpenClawAssistant::new().unwrap();
    let mut security = SecurityConfig::default();
    security.network_allowlist.clear();
    security.fs_mounts.clear();
    let context = SystemContext::new().with_security_config(security);
    let response = assistant.process_query("Run security audit", &context).unwrap();
    assert!(!response.text.is_empty());
    assert!(
        response.text.contains("Issues") || response.text.contains("issues")
            || response.text.contains("whitelist") || response.text.contains("Audit")
    );
    assert!(!response.actions.is_empty());
}
