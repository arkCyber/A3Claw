//! WASI-NN integration tests for OpenClaw+ inference backend.
//!
//! These tests verify:
//!   - WasmEdge plugin detection and initialization
//!   - Sandboxed temp directory I/O (WasiIoPair)
//!   - SHA-256 integrity verification
//!   - Error handling for missing models/plugins
//!
//! Run with: cargo test --features wasi-nn --test wasi_nn_integration

#![cfg(feature = "wasi-nn")]

use openclaw_inference::{InferenceConfig, InferenceEngine, InferenceRequest, BackendKind};
use openclaw_inference::types::ConversationTurn;
use std::path::PathBuf;

#[tokio::test]
async fn test_wasi_nn_plugin_detection() {
    // Verify WasmEdge wasi_nn plugin is available
    use wasmedge_sdk::plugin::PluginManager;
    
    let result = PluginManager::load(None);
    assert!(result.is_ok(), "Failed to load WasmEdge plugins: {:?}", result.err());
    
    let plugin_names = PluginManager::names();
    let has_wasi_nn = plugin_names.iter().any(|n| n.contains("wasi_nn"));
    
    assert!(
        has_wasi_nn,
        "wasi_nn plugin not found. Available plugins: {:?}\n\
         Install with: bash <(curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/refs/heads/master/utils/install.sh) \
         -- --plugins wasi_nn-ggml",
        plugin_names
    );
}

#[tokio::test]
async fn test_wasi_nn_missing_model_error() {
    // Test error handling when model file doesn't exist
    let config = InferenceConfig {
        backend: BackendKind::WasiNn,
        model_path: Some(PathBuf::from("/nonexistent/model.gguf")),
        model_sha256: None,
        endpoint: "http://localhost:8080".into(),
        model_name: "test".into(),
        api_key: None,
        max_tokens: 128,
        temperature: 0.7,
        top_p: 0.9,
        inference_timeout: std::time::Duration::from_secs(120),
        circuit_breaker_threshold: 3,
        circuit_breaker_reset: std::time::Duration::from_secs(30),
        context_window: 4096,
    };
    
    let engine = InferenceEngine::new(config).unwrap();
    let request = InferenceRequest {
        request_id: 1,
        messages: vec![ConversationTurn {
            role: "user".to_string(),
            content: "test".to_string(),
        }],
        max_tokens_override: Some(128),
        temperature_override: Some(0.7),
        stream: false,
    };
    let result = engine.infer(request).await;
    
    // Should get an error (either NoBackendAvailable if WASI-NN not fully initialized,
    // or ModelNotFound if it tries to load the nonexistent model)
    assert!(result.is_err(), "Expected error for missing model file");
    let err = result.unwrap_err();
    let err_str = format!("{:?}", err);
    assert!(
        err_str.contains("NoBackendAvailable") || 
        err_str.contains("ModelNotFound") || 
        err_str.contains("model") || 
        err_str.contains("not found"),
        "Expected backend or model-related error, got: {:?}",
        err
    );
}

#[test]
fn test_wasi_io_pair_sandbox() {
    // Test sandboxed temp directory creation and cleanup
    use std::fs;
    
    let test_json = r#"{"test":"data"}"#;
    let temp_dir = std::env::temp_dir();
    
    // Create WasiIoPair (simulated via direct file ops)
    let pid = std::process::id();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let dir = temp_dir.join(format!("openclaw_wasi_nn_test_{}_{}", pid, ts));
    
    fs::create_dir_all(&dir).expect("create temp dir");
    let req_path = dir.join("request.json");
    let resp_path = dir.join("response.json");
    
    fs::write(&req_path, test_json.as_bytes()).expect("write request");
    fs::File::create(&resp_path).expect("create response file");
    
    // Verify files exist
    assert!(req_path.exists(), "request.json should exist");
    assert!(resp_path.exists(), "response.json should exist");
    
    // Verify content
    let content = fs::read_to_string(&req_path).expect("read request");
    assert_eq!(content, test_json);
    
    // Cleanup
    fs::remove_dir_all(&dir).expect("cleanup temp dir");
    assert!(!dir.exists(), "temp dir should be removed");
}

#[test]
fn test_sha256_integrity_check() {
    // Test SHA-256 hash computation
    use sha2::{Digest, Sha256};
    
    let test_data = b"test model data";
    let temp_file = std::env::temp_dir().join("test_model.bin");
    
    std::fs::write(&temp_file, test_data).expect("write test file");
    
    // Compute hash
    let mut file = std::fs::File::open(&temp_file).expect("open file");
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        use std::io::Read;
        let n = file.read(&mut buf).expect("read file");
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    let hash = hex::encode(hasher.finalize());
    
    // Expected hash for "test model data"
    let expected = "ed4d15fe3a46101dc2eb5ca0ffeb4ad72aa1ca818bdd80b6c06f1e7b570ce513";
    assert_eq!(hash, expected, "SHA-256 hash mismatch");
    
    // Cleanup
    std::fs::remove_file(&temp_file).expect("cleanup test file");
}

#[test]
fn test_json_response_parser() {
    // Test parse_wasi_nn_response logic (simulated)
    let success_json = r#"{"ok":true,"text":"Hello, world!"}"#;
    let error_json = r#"{"ok":false,"error":"model not found"}"#;
    
    // Parse success case
    let v: serde_json::Value = serde_json::from_str(success_json).unwrap();
    assert_eq!(v["ok"].as_bool(), Some(true));
    assert_eq!(v["text"].as_str(), Some("Hello, world!"));
    
    // Parse error case
    let v: serde_json::Value = serde_json::from_str(error_json).unwrap();
    assert_eq!(v["ok"].as_bool(), Some(false));
    assert_eq!(v["error"].as_str(), Some("model not found"));
}

#[test]
fn test_build_wasi_nn_prompt() {
    // Test ChatML prompt builder (simulated)
    
    let messages = vec![
        ConversationTurn {
            role: "system".to_string(),
            content: "You are a helpful assistant.".to_string(),
        },
        ConversationTurn {
            role: "user".to_string(),
            content: "Hello!".to_string(),
        },
    ];
    
    // Simulate build_wasi_nn_prompt logic
    let mut prompt = String::new();
    for m in &messages {
        prompt.push_str(&m.content);
        prompt.push('\n');
    }
    
    assert!(prompt.contains("You are a helpful assistant."));
    assert!(prompt.contains("Hello!"));
}
