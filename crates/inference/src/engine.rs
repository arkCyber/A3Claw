//! InferenceEngine — orchestrates backend selection, circuit breakers, audit, and fallback.
//!
//! # Safety guarantees
//! - Every request is validated before dispatch (empty messages, context window).
//! - Every request is wrapped in a hard timeout.
//! - Every backend call is guarded by a per-backend circuit breaker.
//! - All state transitions are recorded in the audit log.
//! - Fallback chain prevents total failure when one backend is unavailable.

use crate::audit::{AuditEventKind, InferenceAuditLog};
use crate::backend::{HttpBackend, WasiNnBackend};
use crate::circuit_breaker::CircuitBreaker;
use crate::error::InferenceError;
use crate::health::{BackendHealth, HealthStatus};
use crate::types::{BackendKind, ConversationTurn, InferenceConfig, InferenceRequest, InferenceResponse, StreamToken};
use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

pub struct InferenceEngine {
    config: InferenceConfig,
    http_backend: Option<HttpBackend>,
    wasi_backend: Option<WasiNnBackend>,
    circuit_breakers: Arc<RwLock<Vec<CircuitBreaker>>>,
    health_monitors: Arc<RwLock<Vec<BackendHealth>>>,
    audit_log: Arc<InferenceAuditLog>,
    request_counter: AtomicU64,
}

impl InferenceEngine {
    pub fn new(config: InferenceConfig) -> Result<Self, InferenceError> {
        let http_backend = match config.backend {
            BackendKind::WasiNn => None,
            _ => Some(HttpBackend::new(config.clone())?),
        };

        let wasi_backend = match config.backend {
            BackendKind::WasiNn => Some(WasiNnBackend::new(config.clone())),
            _ => None,
        };

        let mut breakers = Vec::new();
        let mut monitors = Vec::new();

        for kind in &[BackendKind::WasiNn, BackendKind::LlamaCppHttp, BackendKind::Ollama, BackendKind::OpenAiCompat] {
            breakers.push(CircuitBreaker::new(
                kind.to_string(),
                config.circuit_breaker_threshold,
                config.circuit_breaker_reset,
            ));
            monitors.push(BackendHealth::new(kind.clone()));
        }

        Ok(Self {
            config,
            http_backend,
            wasi_backend,
            circuit_breakers: Arc::new(RwLock::new(breakers)),
            health_monitors: Arc::new(RwLock::new(monitors)),
            audit_log: Arc::new(InferenceAuditLog::new()),
            request_counter: AtomicU64::new(1),
        })
    }

    /// Validate a request before dispatching it to a backend.
    ///
    /// Returns `Err(ValidationFailed)` if:
    /// - The message list is empty.
    /// - The total estimated token count exceeds `context_window`.
    fn validate_request(&self, request: &InferenceRequest) -> Result<(), InferenceError> {
        if request.messages.is_empty() {
            let reason = "messages list is empty".to_string();
            self.audit_log.record(
                request.request_id,
                AuditEventKind::ValidationFailed,
                &self.config.backend.to_string(),
                &reason,
                None,
            );
            return Err(InferenceError::ValidationFailed { reason });
        }

        // Rough token estimate: 1 token ≈ 4 chars (conservative).
        let estimated_tokens: usize = request.messages
            .iter()
            .map(|m| m.content.len() / 4 + 1)
            .sum();

        let limit = self.config.context_window as usize;
        if estimated_tokens > limit {
            let reason = format!(
                "estimated {estimated_tokens} tokens exceeds context window {limit}"
            );
            self.audit_log.record(
                request.request_id,
                AuditEventKind::ValidationFailed,
                &self.config.backend.to_string(),
                &reason,
                None,
            );
            return Err(InferenceError::ContextWindowExceeded {
                tokens: estimated_tokens,
                limit: self.config.context_window,
            });
        }

        debug!(
            request_id = request.request_id,
            messages = request.messages.len(),
            estimated_tokens,
            "request validation passed"
        );
        Ok(())
    }

    /// Run a non-streaming inference request.
    ///
    /// Assigns a request ID if none is provided (0), validates the request,
    /// wraps execution in a hard timeout, and falls back to other backends
    /// on failure.
    pub async fn infer(&self, mut request: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
        if request.request_id == 0 {
            request.request_id = self.request_counter.fetch_add(1, Ordering::Relaxed);
        }

        let max_tokens = request.max_tokens_override.unwrap_or(self.config.max_tokens);
        let temperature = request.temperature_override.unwrap_or(self.config.temperature);
        let request_id = request.request_id;

        self.audit_log.record(
            request_id,
            AuditEventKind::RequestReceived,
            &self.config.backend.to_string(),
            &format!(
                "messages={} max_tokens={} temperature={:.2} stream={}",
                request.messages.len(), max_tokens, temperature, request.stream
            ),
            None,
        );

        if request.stream {
            return Err(InferenceError::ValidationFailed {
                reason: "use infer_stream() for streaming requests".into(),
            });
        }

        // Validate before touching any backend.
        self.validate_request(&request)?;

        let timeout_duration = self.config.inference_timeout;
        let fut = self.infer_internal(request_id, &request.messages, max_tokens, temperature);

        match timeout(timeout_duration, fut).await {
            Ok(result) => result,
            Err(_elapsed) => {
                let timeout_ms = timeout_duration.as_millis() as u64;
                self.audit_log.record(
                    request_id,
                    AuditEventKind::TimeoutTriggered,
                    &self.config.backend.to_string(),
                    &format!("hard timeout after {timeout_ms}ms"),
                    Some(timeout_ms),
                );
                error!(
                    request_id,
                    timeout_ms,
                    backend = %self.config.backend,
                    "inference hard timeout"
                );
                Err(InferenceError::Timeout { request_id, timeout_ms })
            }
        }
    }

    async fn infer_internal(
        &self,
        request_id: u64,
        messages: &[ConversationTurn],
        max_tokens: u32,
        temperature: f32,
    ) -> Result<InferenceResponse, InferenceError> {
        let backend_idx = self.select_backend_index();
        let backend_name = self.get_backend_name(backend_idx);

        if !self.check_circuit_breaker(backend_idx) {
            warn!(
                request_id,
                backend = %backend_name,
                "primary backend circuit open — routing to fallback"
            );
            self.audit_log.record(
                request_id,
                AuditEventKind::CircuitBreakerOpened,
                &backend_name,
                "circuit open, routing to fallback",
                None,
            );
            return self.try_fallback(request_id, messages, max_tokens, temperature, backend_idx).await;
        }

        self.audit_log.record(
            request_id,
            AuditEventKind::BackendSelected,
            &backend_name,
            &format!("primary backend selected (max_tokens={max_tokens} temperature={temperature:.2})"),
            None,
        );
        self.audit_log.record(
            request_id,
            AuditEventKind::InferenceStarted,
            &backend_name,
            "dispatching to backend",
            None,
        );

        match self.execute_backend(backend_idx, request_id, messages, max_tokens, temperature).await {
            Ok(response) => {
                self.record_success(backend_idx);
                self.audit_log.record(
                    request_id,
                    AuditEventKind::InferenceCompleted,
                    &response.served_by.to_string(),
                    &format!(
                        "prompt_tokens={:?} completion_tokens={:?} latency_ms={}",
                        response.prompt_tokens, response.completion_tokens, response.latency_ms
                    ),
                    Some(response.latency_ms),
                );
                info!(
                    request_id,
                    backend = %response.served_by,
                    latency_ms = response.latency_ms,
                    "inference completed"
                );
                Ok(response)
            }
            Err(e) => {
                self.record_failure(backend_idx);
                self.audit_log.record(
                    request_id,
                    AuditEventKind::InferenceFailed,
                    &backend_name,
                    &format!("code={} error={e}", e.error_code()),
                    None,
                );
                error!(
                    request_id,
                    backend = %backend_name,
                    error_code = e.error_code(),
                    retryable = e.is_retryable(),
                    severity = %e.severity(),
                    error = %e,
                    "primary backend failed — attempting fallback"
                );
                self.try_fallback(request_id, messages, max_tokens, temperature, backend_idx).await
            }
        }
    }

    async fn execute_backend(
        &self,
        backend_idx: usize,
        request_id: u64,
        messages: &[ConversationTurn],
        max_tokens: u32,
        temperature: f32,
    ) -> Result<InferenceResponse, InferenceError> {
        match backend_idx {
            0 => {
                if let Some(ref backend) = self.wasi_backend {
                    backend.infer(request_id, messages, max_tokens, temperature).await
                } else {
                    Err(InferenceError::NoBackendAvailable)
                }
            }
            _ => {
                if let Some(ref backend) = self.http_backend {
                    backend.infer(request_id, messages, max_tokens, temperature).await
                } else {
                    Err(InferenceError::NoBackendAvailable)
                }
            }
        }
    }

    async fn try_fallback(
        &self,
        request_id: u64,
        messages: &[ConversationTurn],
        max_tokens: u32,
        temperature: f32,
        failed_idx: usize,
    ) -> Result<InferenceResponse, InferenceError> {
        let mut attempted = 0usize;

        for idx in 0..4 {
            if idx == failed_idx {
                continue;
            }
            if !self.check_circuit_breaker(idx) {
                debug!(
                    request_id,
                    backend = %self.get_backend_name(idx),
                    "fallback backend circuit open — skipping"
                );
                continue;
            }

            let fb_name = self.get_backend_name(idx);
            attempted += 1;
            warn!(
                request_id,
                fallback_backend = %fb_name,
                attempt = attempted,
                "trying fallback backend"
            );
            self.audit_log.record(
                request_id,
                AuditEventKind::FallbackAttempted,
                &fb_name,
                &format!("fallback attempt {attempted}"),
                None,
            );

            match self.execute_backend(idx, request_id, messages, max_tokens, temperature).await {
                Ok(response) => {
                    self.record_success(idx);
                    self.audit_log.record(
                        request_id,
                        AuditEventKind::InferenceCompleted,
                        &response.served_by.to_string(),
                        &format!("fallback succeeded after {attempted} attempt(s)"),
                        Some(response.latency_ms),
                    );
                    info!(
                        request_id,
                        backend = %response.served_by,
                        latency_ms = response.latency_ms,
                        fallback_attempts = attempted,
                        "fallback inference succeeded"
                    );
                    return Ok(response);
                }
                Err(e) => {
                    self.record_failure(idx);
                    self.audit_log.record(
                        request_id,
                        AuditEventKind::InferenceFailed,
                        &fb_name,
                        &format!("fallback failed: code={} error={e}", e.error_code()),
                        None,
                    );
                    error!(
                        request_id,
                        backend = %fb_name,
                        error_code = e.error_code(),
                        retryable = e.is_retryable(),
                        error = %e,
                        "fallback backend failed"
                    );
                }
            }
        }

        error!(
            request_id,
            failed_primary = %self.get_backend_name(failed_idx),
            fallback_attempts = attempted,
            "all backends exhausted — NoBackendAvailable"
        );
        self.audit_log.record(
            request_id,
            AuditEventKind::InferenceFailed,
            "all",
            &format!("all backends failed after {attempted} fallback attempt(s)"),
            None,
        );
        Err(InferenceError::NoBackendAvailable)
    }

    pub async fn infer_stream(
        &self,
        mut request: InferenceRequest,
    ) -> Result<mpsc::Receiver<StreamToken>, InferenceError> {
        if request.request_id == 0 {
            request.request_id = self.request_counter.fetch_add(1, Ordering::Relaxed);
        }

        let max_tokens = request.max_tokens_override.unwrap_or(self.config.max_tokens);
        let temperature = request.temperature_override.unwrap_or(self.config.temperature);

        self.audit_log.record(
            request.request_id,
            AuditEventKind::StreamStarted,
            &self.config.backend.to_string(),
            &format!("messages={}", request.messages.len()),
            None,
        );

        let (tx, rx) = mpsc::channel(128);

        if let Some(ref backend) = self.http_backend {
            let backend = backend.clone();
            let request_id = request.request_id;
            let messages = request.messages.clone();
            let audit = self.audit_log.clone();

            tokio::spawn(async move {
                match backend.infer_stream(request_id, &messages, max_tokens, temperature, tx).await {
                    Ok(_) => {
                        audit.record(
                            request_id,
                            AuditEventKind::StreamCompleted,
                            "http",
                            "stream finished",
                            None,
                        );
                    }
                    Err(e) => {
                        error!(request_id, error = %e, "stream failed");
                    }
                }
            });

            Ok(rx)
        } else {
            Err(InferenceError::Internal(anyhow::anyhow!(
                "Streaming only supported for HTTP backends"
            )))
        }
    }

    fn select_backend_index(&self) -> usize {
        match self.config.backend {
            BackendKind::WasiNn        => 0,
            BackendKind::LlamaCppHttp  => 1,
            BackendKind::Ollama        => 2,
            BackendKind::OpenAiCompat  => 3,
        }
    }

    fn get_backend_name(&self, idx: usize) -> String {
        match idx {
            0 => "WasiNn".to_string(),
            1 => "LlamaCppHttp".to_string(),
            2 => "Ollama".to_string(),
            3 => "OpenAiCompat".to_string(),
            _ => "Unknown".to_string(),
        }
    }

    fn check_circuit_breaker(&self, idx: usize) -> bool {
        let mut breakers = self.circuit_breakers.write();
        if let Some(breaker) = breakers.get_mut(idx) {
            breaker.allow()
        } else {
            false
        }
    }

    fn record_success(&self, idx: usize) {
        let mut breakers = self.circuit_breakers.write();
        if let Some(breaker) = breakers.get_mut(idx) {
            breaker.record_success();
        }

        let mut monitors = self.health_monitors.write();
        if let Some(monitor) = monitors.get_mut(idx) {
            monitor.record_probe_ok(0);
        }
    }

    fn record_failure(&self, idx: usize) {
        let mut breakers = self.circuit_breakers.write();
        if let Some(breaker) = breakers.get_mut(idx) {
            breaker.record_failure();
        }

        let mut monitors = self.health_monitors.write();
        if let Some(monitor) = monitors.get_mut(idx) {
            monitor.record_probe_fail("inference failed");
        }
    }

    /// Returns the current health status of all monitored backends.
    pub fn health_status(&self) -> Vec<(String, HealthStatus)> {
        let monitors = self.health_monitors.read();
        monitors
            .iter()
            .map(|m| (m.backend.to_string(), m.status.clone()))
            .collect()
    }

    /// Returns engine-level statistics for observability.
    pub fn stats(&self) -> EngineStats {
        let breakers = self.circuit_breakers.read();
        let monitors = self.health_monitors.read();
        EngineStats {
            total_requests: self.request_counter.load(Ordering::Relaxed).saturating_sub(1),
            open_breakers: breakers.iter().filter(|b| b.is_open()).count(),
            usable_backends: monitors.iter().filter(|m| m.is_usable()).count(),
            audit_events: self.audit_log.event_count(),
        }
    }
}

/// Snapshot of engine-level statistics.
#[derive(Debug, Clone)]
pub struct EngineStats {
    /// Total requests dispatched since engine creation.
    pub total_requests: u64,
    /// Number of backends with an open circuit breaker.
    pub open_breakers: usize,
    /// Number of backends currently considered usable.
    pub usable_backends: usize,
    /// Total audit events emitted.
    pub audit_events: u64,
}

