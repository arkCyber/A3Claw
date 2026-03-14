#[cfg(test)]
mod tests {
    use crate::engine::{EngineStats, InferenceEngine};
    use crate::error::InferenceError;
    use crate::health::HealthStatus;
    use crate::types::{BackendKind, ConversationTurn, InferenceConfig, InferenceRequest};
    use std::time::Duration;

    // ── Helpers ────────────────────────────────────────────────────────────────

    fn base_config() -> InferenceConfig {
        InferenceConfig {
            backend: BackendKind::LlamaCppHttp,
            model_path: None,
            model_sha256: None,
            endpoint: "http://127.0.0.1:19999".into(), // nothing listening here
            model_name: "test-model".into(),
            api_key: None,
            max_tokens: 128,
            temperature: 0.7,
            top_p: 0.95,
            inference_timeout: Duration::from_secs(30),
            circuit_breaker_threshold: 3,
            circuit_breaker_reset: Duration::from_secs(60),
            context_window: 512,
        }
    }

    fn user_msg(content: &str) -> ConversationTurn {
        ConversationTurn { role: "user".into(), content: content.into() }
    }

    fn request_with(messages: Vec<ConversationTurn>) -> InferenceRequest {
        InferenceRequest {
            request_id: 0,
            messages,
            max_tokens_override: None,
            temperature_override: None,
            stream: false,
        }
    }

    // ── InferenceEngine::new() ──────────────────────────────────────────────

    #[test]
    fn new_with_http_backend_succeeds() {
        let engine = InferenceEngine::new(base_config());
        assert!(engine.is_ok(), "engine creation should succeed for HTTP backend");
    }

    #[test]
    fn new_with_ollama_backend_succeeds() {
        let mut cfg = base_config();
        cfg.backend = BackendKind::Ollama;
        cfg.endpoint = "http://127.0.0.1:11434".into();
        let engine = InferenceEngine::new(cfg);
        assert!(engine.is_ok());
    }

    #[test]
    fn new_with_wasi_nn_backend_succeeds() {
        let mut cfg = base_config();
        cfg.backend = BackendKind::WasiNn;
        let engine = InferenceEngine::new(cfg);
        assert!(engine.is_ok());
    }

    // ── health_status() ─────────────────────────────────────────────────────

    #[test]
    fn health_status_returns_four_backends() {
        let engine = InferenceEngine::new(base_config()).unwrap();
        let status = engine.health_status();
        assert_eq!(status.len(), 4, "should have one entry per backend kind (WasiNn, LlamaCppHttp, Ollama, OpenAiCompat)");
    }

    #[test]
    fn health_status_all_start_unknown() {
        let engine = InferenceEngine::new(base_config()).unwrap();
        for (_name, status) in engine.health_status() {
            assert_eq!(status, HealthStatus::Unknown, "all backends should start Unknown");
        }
    }

    #[test]
    fn health_status_names_are_non_empty() {
        let engine = InferenceEngine::new(base_config()).unwrap();
        for (name, _) in engine.health_status() {
            assert!(!name.is_empty());
        }
    }

    // ── stats() ────────────────────────────────────────────────────────────────

    #[test]
    fn stats_initial_state() {
        let engine = InferenceEngine::new(base_config()).unwrap();
        let s = engine.stats();
        assert_eq!(s.total_requests, 0);
        assert_eq!(s.open_breakers, 0);
        assert_eq!(s.usable_backends, 4);
        assert_eq!(s.audit_events, 0);
    }

    #[test]
    fn stats_debug_impl() {
        let engine = InferenceEngine::new(base_config()).unwrap();
        let s = engine.stats();
        let dbg = format!("{:?}", s);
        assert!(dbg.contains("EngineStats"));
    }

    // ── validate_request (via infer) ───────────────────────────────────────

    #[tokio::test]
    async fn infer_rejects_empty_messages() {
        let engine = InferenceEngine::new(base_config()).unwrap();
        let req = request_with(vec![]);
        let result = engine.infer(req).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            InferenceError::ValidationFailed { reason } => {
                assert!(reason.contains("empty"), "reason should mention empty: {reason}");
            }
            other => panic!("expected ValidationFailed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn infer_rejects_stream_flag() {
        let engine = InferenceEngine::new(base_config()).unwrap();
        let mut req = request_with(vec![user_msg("hi")]);
        req.stream = true;
        let result = engine.infer(req).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            InferenceError::ValidationFailed { .. } => {}
            other => panic!("expected ValidationFailed for stream=true, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn infer_rejects_oversized_context() {
        let mut cfg = base_config();
        cfg.context_window = 1; // extremely small
        let engine = InferenceEngine::new(cfg).unwrap();
        // A message with 100 chars ≈ 25 tokens > limit of 1
        let req = request_with(vec![user_msg(&"x".repeat(100))]);
        let result = engine.infer(req).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            InferenceError::ContextWindowExceeded { tokens, limit } => {
                assert!(tokens > limit as usize);
            }
            other => panic!("expected ContextWindowExceeded, got {other:?}"),
        }
    }

    // ── infer() — timeout ───────────────────────────────────────────────────

    #[tokio::test]
    async fn infer_returns_error_when_backend_unreachable() {
        // 1ms timeout guarantees failure against a non-listening port.
        let mut cfg = base_config();
        cfg.inference_timeout = Duration::from_millis(1);
        let engine = InferenceEngine::new(cfg).unwrap();
        let req = request_with(vec![user_msg("hello")]);
        let result = engine.infer(req).await;
        assert!(result.is_err(), "should fail when backend is unreachable");
    }

    #[tokio::test]
    async fn infer_assigns_request_id_when_zero() {
        // We can't observe the assigned ID directly from the return value on error,
        // but we can verify the audit log increments (via stats).
        let mut cfg = base_config();
        cfg.inference_timeout = Duration::from_millis(1);
        let engine = InferenceEngine::new(cfg).unwrap();
        let _ = engine.infer(request_with(vec![user_msg("a")])).await;
        let _ = engine.infer(request_with(vec![user_msg("b")])).await;
        // request_counter should have advanced
        let s = engine.stats();
        assert!(s.total_requests >= 2, "total_requests should be >= 2, got {}", s.total_requests);
    }

    // ── infer_stream() ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn infer_stream_returns_receiver_for_http_backend() {
        let engine = InferenceEngine::new(base_config()).unwrap();
        let req = InferenceRequest {
            request_id: 0,
            messages: vec![user_msg("hello")],
            max_tokens_override: None,
            temperature_override: None,
            stream: true,
        };
        // Should return Ok(receiver) even if the backend is not running;
        // the actual stream will fail asynchronously.
        let result = engine.infer_stream(req).await;
        assert!(result.is_ok(), "infer_stream should return a receiver channel");
    }

    #[tokio::test]
    async fn infer_stream_fails_for_wasi_nn_backend() {
        let mut cfg = base_config();
        cfg.backend = BackendKind::WasiNn;
        let engine = InferenceEngine::new(cfg).unwrap();
        let req = InferenceRequest {
            request_id: 0,
            messages: vec![user_msg("hello")],
            max_tokens_override: None,
            temperature_override: None,
            stream: true,
        };
        let result = engine.infer_stream(req).await;
        assert!(result.is_err(), "streaming should fail for WASI-NN backend");
    }

    // ── ConversationTurn ─────────────────────────────────────────────────────

    #[test]
    fn conversation_turn_clone() {
        let t = user_msg("hello");
        let t2 = t.clone();
        assert_eq!(t.role, t2.role);
        assert_eq!(t.content, t2.content);
    }

    // ── InferenceConfig defaults ──────────────────────────────────────────────

    #[test]
    fn inference_config_default_is_valid() {
        let cfg = InferenceConfig::default();
        assert_eq!(cfg.backend, BackendKind::LlamaCppHttp);
        assert!(cfg.max_tokens > 0);
        assert!(cfg.context_window > 0);
        assert!(cfg.circuit_breaker_threshold > 0);
        assert!(!cfg.endpoint.is_empty());
        assert!(!cfg.model_name.is_empty());
    }

    #[test]
    fn backend_kind_display() {
        assert_eq!(BackendKind::WasiNn.to_string(),       "WasmEdge WASI-NN");
        assert_eq!(BackendKind::LlamaCppHttp.to_string(), "llama.cpp HTTP");
        assert_eq!(BackendKind::Ollama.to_string(),       "Ollama");
        assert_eq!(BackendKind::OpenAiCompat.to_string(), "OpenAI-compat");
    }
}
