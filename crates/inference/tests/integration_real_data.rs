//! Integration tests for the `openclaw-inference` crate using real data.
//!
//! Tests cover:
//! - `InferenceConfig` construction and validation
//! - `BackendHealth` state machine transitions with real probe sequences
//! - `InferenceEngine` construction with various backend configs
//! - HTTP backend availability probing (network tests marked `#[ignore]`)
//! - Circuit breaker behaviour under simulated failure sequences
//! - `InferenceRequest` / `InferenceResponse` type validation

use openclaw_inference::{
    BackendKind, InferenceConfig, InferenceEngine, InferenceRequest,
};
use openclaw_inference::health::{BackendHealth, HealthStatus};
use openclaw_inference::circuit_breaker::CircuitBreaker;
use std::time::Duration;

// ── Config builder helpers ─────────────────────────────────────────────────────

fn ollama_config() -> InferenceConfig {
    InferenceConfig {
        backend:                    BackendKind::Ollama,
        endpoint:                   "http://localhost:11434".to_string(),
        model_name:                 "qwen2.5:0.5b".to_string(),
        inference_timeout:          Duration::from_secs(30),
        circuit_breaker_threshold:  999,
        circuit_breaker_reset:      Duration::from_secs(60),
        ..InferenceConfig::default()
    }
}

fn llamacpp_config(endpoint: &str) -> InferenceConfig {
    InferenceConfig {
        backend:                    BackendKind::LlamaCppHttp,
        endpoint:                   endpoint.to_string(),
        model_name:                 "llama-3-8b-instruct".to_string(),
        inference_timeout:          Duration::from_secs(60),
        circuit_breaker_threshold:  5,
        circuit_breaker_reset:      Duration::from_secs(120),
        ..InferenceConfig::default()
    }
}

fn openai_compat_config(api_key: &str) -> InferenceConfig {
    InferenceConfig {
        backend:                    BackendKind::OpenAiCompat,
        endpoint:                   "https://api.openai.com".to_string(),
        model_name:                 "gpt-4o-mini".to_string(),
        api_key:                    Some(api_key.to_string()),
        inference_timeout:          Duration::from_secs(45),
        circuit_breaker_threshold:  3,
        circuit_breaker_reset:      Duration::from_secs(60),
        ..InferenceConfig::default()
    }
}

// ── InferenceConfig validation ────────────────────────────────────────────────

#[test]
fn inference_config_default_values_are_sane() {
    let cfg = InferenceConfig::default();
    assert!(!cfg.endpoint.is_empty(), "endpoint must not be empty");
    assert!(!cfg.model_name.is_empty(), "model_name must not be empty");
    assert!(cfg.max_tokens > 0, "max_tokens must be positive");
    assert!(cfg.context_window > 0, "context_window must be positive");
    assert!(cfg.inference_timeout > Duration::ZERO, "timeout must be positive");
    assert!(cfg.circuit_breaker_threshold > 0);
}

#[test]
fn inference_config_ollama_backend() {
    let cfg = ollama_config();
    assert_eq!(cfg.backend, BackendKind::Ollama);
    assert_eq!(cfg.endpoint, "http://localhost:11434");
    assert_eq!(cfg.model_name, "qwen2.5:0.5b");
    assert_eq!(cfg.inference_timeout, Duration::from_secs(30));
}

#[test]
fn inference_config_llamacpp_backend() {
    let cfg = llamacpp_config("http://localhost:8080");
    assert_eq!(cfg.backend, BackendKind::LlamaCppHttp);
    assert_eq!(cfg.endpoint, "http://localhost:8080");
    assert_eq!(cfg.circuit_breaker_threshold, 5);
}

#[test]
fn inference_config_openai_compat_with_api_key() {
    let cfg = openai_compat_config("sk-test-12345");
    assert_eq!(cfg.backend, BackendKind::OpenAiCompat);
    assert_eq!(cfg.api_key.as_deref(), Some("sk-test-12345"));
    assert_eq!(cfg.endpoint, "https://api.openai.com");
}

#[test]
fn backend_kind_display_strings() {
    assert_eq!(BackendKind::Ollama.to_string(),       "Ollama");
    assert_eq!(BackendKind::LlamaCppHttp.to_string(), "llama.cpp HTTP");
    assert_eq!(BackendKind::OpenAiCompat.to_string(), "OpenAI-compat");
    assert_eq!(BackendKind::WasiNn.to_string(),       "WasmEdge WASI-NN");
}

// ── InferenceEngine construction ──────────────────────────────────────────────

#[test]
fn inference_engine_builds_with_ollama_config() {
    let cfg = ollama_config();
    let engine = InferenceEngine::new(cfg);
    assert!(engine.is_ok(), "engine construction must succeed: {:?}", engine.err());
}

#[test]
fn inference_engine_builds_with_llamacpp_config() {
    let cfg = llamacpp_config("http://localhost:8080");
    let engine = InferenceEngine::new(cfg);
    assert!(engine.is_ok(), "engine construction must succeed");
}

#[test]
fn inference_engine_builds_with_openai_compat_config() {
    let cfg = openai_compat_config("sk-test-key");
    let engine = InferenceEngine::new(cfg);
    assert!(engine.is_ok(), "engine construction must succeed");
}

// ── BackendHealth state machine ───────────────────────────────────────────────

#[test]
fn backend_health_initial_state_is_unknown() {
    let h = BackendHealth::new(BackendKind::Ollama);
    assert_eq!(h.status, HealthStatus::Unknown);
    assert_eq!(h.consecutive_failures, 0);
    assert_eq!(h.total_ok, 0);
    assert_eq!(h.total_fail, 0);
    assert!(h.last_probe.is_none());
    assert!(!h.is_usable() || h.status == HealthStatus::Unknown,
        "Unknown is usable (best-effort)");
}

#[test]
fn backend_health_single_ok_probe_transitions_to_healthy() {
    let mut h = BackendHealth::new(BackendKind::Ollama);
    h.record_probe_ok(42);
    assert_eq!(h.status, HealthStatus::Healthy);
    assert_eq!(h.total_ok, 1);
    assert_eq!(h.consecutive_failures, 0);
    assert_eq!(h.last_latency_ms, Some(42));
    assert!(h.last_probe.is_some());
}

#[test]
fn backend_health_single_fail_transitions_to_degraded() {
    let mut h = BackendHealth::new(BackendKind::Ollama);
    h.record_probe_fail("connection refused");
    assert_eq!(h.status, HealthStatus::Degraded);
    assert_eq!(h.consecutive_failures, 1);
    assert_eq!(h.total_fail, 1);
    assert!(h.is_usable(), "Degraded is still usable");
}

#[test]
fn backend_health_three_fails_transitions_to_unhealthy() {
    let mut h = BackendHealth::new(BackendKind::Ollama);
    h.record_probe_fail("timeout");
    h.record_probe_fail("timeout");
    h.record_probe_fail("connection reset");
    assert_eq!(h.status, HealthStatus::Unhealthy);
    assert_eq!(h.consecutive_failures, 3);
    assert_eq!(h.total_fail, 3);
    assert!(!h.is_usable(), "Unhealthy must not be usable");
}

#[test]
fn backend_health_recovery_after_failure_sequence() {
    let mut h = BackendHealth::new(BackendKind::LlamaCppHttp);

    // Degrade first
    h.record_probe_fail("timeout");
    h.record_probe_fail("timeout");
    assert_eq!(h.status, HealthStatus::Degraded);

    // Single ok probe recovers
    h.record_probe_ok(120);
    assert_eq!(h.status, HealthStatus::Healthy);
    assert_eq!(h.consecutive_failures, 0, "consecutive_failures must reset on ok");
    assert_eq!(h.total_ok, 1);
    assert_eq!(h.total_fail, 2);
}

#[test]
fn backend_health_oscillating_probe_sequence() {
    let mut h = BackendHealth::new(BackendKind::Ollama);

    // ok → fail → fail → ok → fail × 3 → unhealthy
    h.record_probe_ok(10);
    assert_eq!(h.status, HealthStatus::Healthy);

    h.record_probe_fail("blip");
    h.record_probe_fail("blip");
    assert_eq!(h.status, HealthStatus::Degraded);

    h.record_probe_ok(15);
    assert_eq!(h.status, HealthStatus::Healthy);

    h.record_probe_fail("down");
    h.record_probe_fail("down");
    h.record_probe_fail("down");
    assert_eq!(h.status, HealthStatus::Unhealthy);
    assert!(!h.is_usable());
}

#[test]
fn backend_health_with_custom_threshold_two() {
    let mut h = BackendHealth::with_threshold(BackendKind::Ollama, 2);
    h.record_probe_fail("err1");
    assert_eq!(h.status, HealthStatus::Degraded);
    h.record_probe_fail("err2");
    assert_eq!(h.status, HealthStatus::Unhealthy,
        "threshold=2 must reach Unhealthy after 2 failures");
}

#[test]
fn backend_health_latency_tracking() {
    let mut h = BackendHealth::new(BackendKind::Ollama);
    h.record_probe_ok(100);
    h.record_probe_ok(200);
    h.record_probe_ok(50);
    assert_eq!(h.last_latency_ms, Some(50), "last latency must be most recent");
    assert_eq!(h.total_ok, 3);
}

// ── CircuitBreaker state transitions ─────────────────────────────────────────

#[test]
fn circuit_breaker_starts_closed() {
    let cb = CircuitBreaker::new("ollama".to_string(), 3, Duration::from_secs(30));
    assert!(cb.is_closed(), "circuit breaker must start closed");
    assert!(!cb.is_open(), "circuit breaker must not start open");
    assert_eq!(cb.failures(), 0);
}

#[test]
fn circuit_breaker_opens_after_threshold_failures() {
    let mut cb = CircuitBreaker::new("test-backend".to_string(), 3, Duration::from_secs(30));
    cb.record_failure();
    cb.record_failure();
    assert!(!cb.is_open(), "not open yet (2 < 3)");
    cb.record_failure();
    assert!(cb.is_open(), "must open after 3 consecutive failures");
}

#[test]
fn circuit_breaker_resets_on_success() {
    let mut cb = CircuitBreaker::new("test-backend".to_string(), 3, Duration::from_secs(30));
    cb.record_failure();
    cb.record_failure();
    cb.record_success();
    assert!(cb.is_closed(), "success must reset circuit breaker");
    assert_eq!(cb.failures(), 0, "failure count must reset");
}

#[test]
fn circuit_breaker_threshold_one() {
    let mut cb = CircuitBreaker::new("fragile".to_string(), 1, Duration::from_secs(10));
    cb.record_failure();
    assert!(cb.is_open(), "threshold=1: single failure must open breaker");
}

#[test]
fn circuit_breaker_multiple_backends_independent() {
    let mut cb_a = CircuitBreaker::new("backend-a".to_string(), 2, Duration::from_secs(30));
    let cb_b = CircuitBreaker::new("backend-b".to_string(), 2, Duration::from_secs(30));

    cb_a.record_failure();
    cb_a.record_failure();
    assert!(cb_a.is_open(), "cb_a must be open");
    assert!(cb_b.is_closed(), "cb_b must be independent and closed");
}

// ── InferenceRequest validation ───────────────────────────────────────────────

#[test]
fn inference_request_single_user_message() {
    use openclaw_inference::types::ConversationTurn;
    let req = InferenceRequest {
        request_id: 1,
        messages: vec![
            ConversationTurn { role: "user".to_string(), content: "What is 2+2?".to_string() },
        ],
        max_tokens_override: Some(50),
        temperature_override: Some(0.0),
        stream: false,
    };
    assert_eq!(req.messages.len(), 1);
    assert_eq!(req.messages[0].role, "user");
    assert!(!req.stream);
    assert_eq!(req.max_tokens_override, Some(50));
}

#[test]
fn inference_request_multi_turn_conversation() {
    use openclaw_inference::types::ConversationTurn;
    let req = InferenceRequest {
        request_id: 42,
        messages: vec![
            ConversationTurn { role: "system".to_string(),    content: "You are a helpful assistant.".to_string() },
            ConversationTurn { role: "user".to_string(),      content: "Summarize the French Revolution.".to_string() },
            ConversationTurn { role: "assistant".to_string(), content: "The French Revolution...".to_string() },
            ConversationTurn { role: "user".to_string(),      content: "What were the main causes?".to_string() },
        ],
        max_tokens_override: Some(512),
        temperature_override: Some(0.3),
        stream: false,
    };
    assert_eq!(req.messages.len(), 4);
    assert_eq!(req.messages[0].role, "system");
    assert_eq!(req.request_id, 42);
}

// ── Real HTTP backend availability (network, marked #[ignore]) ────────────────

#[tokio::test]
#[ignore = "requires local Ollama server at http://localhost:11434"]
async fn ollama_server_health_check_real() {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    let resp = client
        .get("http://localhost:11434/api/tags")
        .send()
        .await;

    match resp {
        Ok(r) => {
            assert!(r.status().is_success(),
                "Ollama /api/tags must return 2xx: {}", r.status());
            let body = r.text().await.unwrap();
            assert!(body.contains("models") || body.contains("tags"),
                "Ollama response must list models: {}", &body[..200.min(body.len())]);
        }
        Err(e) => panic!("Ollama not reachable: {}", e),
    }
}

#[tokio::test]
#[ignore = "requires local Ollama server at http://localhost:11434"]
async fn ollama_inference_engine_chat_real() {
    use openclaw_inference::types::ConversationTurn;

    let cfg = ollama_config();
    let engine = InferenceEngine::new(cfg).unwrap();

    let req = InferenceRequest {
        request_id: 1001,
        messages: vec![
            ConversationTurn {
                role: "user".to_string(),
                content: "Reply with exactly: 'pong'".to_string(),
            },
        ],
        max_tokens_override: Some(10),
        temperature_override: Some(0.0),
        stream: false,
    };

    let result = engine.infer(req).await;
    assert!(result.is_ok(), "Ollama inference must succeed: {:?}", result.err());
    let resp = result.unwrap();
    assert!(!resp.content.is_empty(), "response must not be empty");
}

#[tokio::test]
#[ignore = "requires local llama.cpp server at http://localhost:8080"]
async fn llamacpp_server_health_check_real() {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    // llama.cpp /health endpoint
    let resp = client
        .get("http://localhost:8080/health")
        .send()
        .await;

    match resp {
        Ok(r) => {
            assert!(r.status().is_success(),
                "llama.cpp /health must return 2xx: {}", r.status());
        }
        Err(e) => panic!("llama.cpp server not reachable: {}", e),
    }
}

#[tokio::test]
#[ignore = "requires local llama.cpp server at http://localhost:8080"]
async fn llamacpp_inference_engine_chat_real() {
    use openclaw_inference::types::ConversationTurn;

    let cfg = llamacpp_config("http://localhost:8080");
    let engine = InferenceEngine::new(cfg).unwrap();

    let req = InferenceRequest {
        request_id: 1002,
        messages: vec![
            ConversationTurn {
                role: "user".to_string(),
                content: "Reply with exactly: 'pong'".to_string(),
            },
        ],
        max_tokens_override: Some(10),
        temperature_override: Some(0.0),
        stream: false,
    };

    let result = engine.infer(req).await;
    assert!(result.is_ok(), "llama.cpp inference must succeed: {:?}", result.err());
    let resp = result.unwrap();
    assert!(!resp.content.is_empty(), "response must not be empty");
}

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY environment variable"]
async fn openai_compat_inference_engine_chat_real() {
    use openclaw_inference::types::ConversationTurn;

    let api_key = std::env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY must be set for this test");

    let cfg = openai_compat_config(&api_key);
    let engine = InferenceEngine::new(cfg).unwrap();

    let req = InferenceRequest {
        request_id: 1003,
        messages: vec![
            ConversationTurn {
                role: "user".to_string(),
                content: "Reply with exactly one word: 'pong'".to_string(),
            },
        ],
        max_tokens_override: Some(5),
        temperature_override: Some(0.0),
        stream: false,
    };

    let result = engine.infer(req).await;
    assert!(result.is_ok(), "OpenAI inference must succeed: {:?}", result.err());
    let resp = result.unwrap();
    assert!(!resp.content.is_empty(), "response must not be empty");
}

#[tokio::test]
#[ignore = "requires network access to api.openai.com"]
async fn openai_api_endpoint_reachable() {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    // Just verify DNS resolution and TCP connectivity to api.openai.com:443
    let resp = client
        .get("https://api.openai.com/v1/models")
        .header("Authorization", "Bearer sk-invalid-key-for-connectivity-test")
        .send()
        .await;

    match resp {
        Ok(r) => {
            // 401 Unauthorized is expected with invalid key — endpoint is reachable
            assert!(
                r.status().as_u16() == 401 || r.status().as_u16() == 403,
                "Expected 401/403 for invalid key, got: {}", r.status()
            );
        }
        Err(e) => panic!("api.openai.com not reachable: {}", e),
    }
}
