//! Multi-backend fallback demonstration
//!
//! This example shows how to configure multiple inference backends with
//! automatic failover for production redundancy.
//!
//! Backends tested (in order):
//! 1. Ollama (primary, local GPU)
//! 2. llama.cpp server (backup, local)
//! 3. OpenAI API (cloud fallback)
//! 4. DeepSeek API (cloud fallback, cost-effective)

use openclaw_inference::{
    BackendKind, InferenceConfig, InferenceEngine, InferenceError,
    InferenceRequest,
};
use openclaw_inference::types::ConversationTurn;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Multi-Backend Fallback Demo ===\n");

    // Test message
    let test_message = vec![ConversationTurn {
        role: "user".to_string(),
        content: "Say 'Hello from' followed by your service name in 5 words or less.".to_string(),
    }];

    let request = InferenceRequest {
        request_id: 1,
        messages: test_message.clone(),
        max_tokens_override: Some(50),
        temperature_override: Some(0.7),
        stream: false,
    };

    // ========================================================================
    // Backend 1: Ollama (Primary)
    // ========================================================================
    println!("🔵 Testing Backend 1: Ollama (Primary)");
    println!("   Endpoint: http://localhost:11434");
    println!("   Model: qwen2.5:0.5b");
    
    let ollama_config = InferenceConfig {
        backend: BackendKind::Ollama,
        endpoint: "http://localhost:11434".into(),
        model_name: "qwen2.5:0.5b".into(),
        max_tokens: 256,
        temperature: 0.7,
        top_p: 0.95,
        inference_timeout: Duration::from_secs(30),
        circuit_breaker_threshold: 3,
        circuit_breaker_reset: Duration::from_secs(60),
        context_window: 8192,
        ..Default::default()
    };

    match test_backend("Ollama", ollama_config, request.clone()).await {
        Ok(_) => {
            println!("✅ Primary service healthy, using Ollama\n");
            return Ok(());
        }
        Err(e) => {
            println!("❌ Primary service failed: {}", e);
            println!("   Trying backup services...\n");
        }
    }

    // ========================================================================
    // Backend 2: llama.cpp server (Local Backup)
    // ========================================================================
    println!("🟡 Testing Backend 2: llama.cpp server (Local Backup)");
    println!("   Endpoint: http://localhost:8080");
    println!("   Model: qwen2.5-0.5b-instruct-q4_k_m");
    
    let llama_config = InferenceConfig {
        backend: BackendKind::LlamaCppHttp,
        endpoint: "http://localhost:8080".into(),
        model_name: "qwen2.5-0.5b-instruct-q4_k_m".into(),
        max_tokens: 256,
        temperature: 0.7,
        top_p: 0.95,
        inference_timeout: Duration::from_secs(30),
        circuit_breaker_threshold: 3,
        circuit_breaker_reset: Duration::from_secs(60),
        context_window: 8192,
        ..Default::default()
    };

    match test_backend("llama.cpp", llama_config, request.clone()).await {
        Ok(_) => {
            println!("✅ Local backup healthy, using llama.cpp\n");
            return Ok(());
        }
        Err(e) => {
            println!("❌ Local backup failed: {}", e);
            println!("   Trying cloud services...\n");
        }
    }

    // ========================================================================
    // Backend 3: OpenAI API (Cloud Fallback)
    // ========================================================================
    println!("🟢 Testing Backend 3: OpenAI API (Cloud Fallback)");
    println!("   Endpoint: https://api.openai.com/v1");
    println!("   Model: gpt-3.5-turbo");
    
    // Read API key from environment
    let openai_key = std::env::var("OPENAI_API_KEY").ok();
    
    if openai_key.is_none() {
        println!("⚠️  OPENAI_API_KEY not set, skipping OpenAI backend");
        println!("   Set it with: export OPENAI_API_KEY='sk-...'\n");
    } else {
        let openai_config = InferenceConfig {
            backend: BackendKind::OpenAiCompat,
            endpoint: "https://api.openai.com/v1".into(),
            model_name: "gpt-3.5-turbo".into(),
            api_key: openai_key,
            max_tokens: 256,
            temperature: 0.7,
            top_p: 0.95,
            inference_timeout: Duration::from_secs(60),
            circuit_breaker_threshold: 5,
            circuit_breaker_reset: Duration::from_secs(120),
            context_window: 16384,
            ..Default::default()
        };

        match test_backend("OpenAI", openai_config, request.clone()).await {
            Ok(_) => {
                println!("✅ OpenAI API healthy, using cloud service\n");
                return Ok(());
            }
            Err(e) => {
                println!("❌ OpenAI API failed: {}", e);
                println!("   Trying DeepSeek...\n");
            }
        }
    }

    // ========================================================================
    // Backend 4: DeepSeek API (Cost-Effective Cloud)
    // ========================================================================
    println!("🟣 Testing Backend 4: DeepSeek API (Cost-Effective Cloud)");
    println!("   Endpoint: https://api.deepseek.com/v1");
    println!("   Model: deepseek-chat");
    
    let deepseek_key = std::env::var("DEEPSEEK_API_KEY").ok();
    
    if deepseek_key.is_none() {
        println!("⚠️  DEEPSEEK_API_KEY not set, skipping DeepSeek backend");
        println!("   Set it with: export DEEPSEEK_API_KEY='sk-...'\n");
    } else {
        let deepseek_config = InferenceConfig {
            backend: BackendKind::OpenAiCompat,
            endpoint: "https://api.deepseek.com/v1".into(),
            model_name: "deepseek-chat".into(),
            api_key: deepseek_key,
            max_tokens: 256,
            temperature: 0.7,
            top_p: 0.95,
            inference_timeout: Duration::from_secs(60),
            circuit_breaker_threshold: 5,
            circuit_breaker_reset: Duration::from_secs(120),
            context_window: 32768,
            ..Default::default()
        };

        match test_backend("DeepSeek", deepseek_config, request.clone()).await {
            Ok(_) => {
                println!("✅ DeepSeek API healthy, using cloud service\n");
                return Ok(());
            }
            Err(e) => {
                println!("❌ DeepSeek API failed: {}", e);
            }
        }
    }

    // ========================================================================
    // All backends failed
    // ========================================================================
    println!("\n❌ All backends failed!");
    println!("\nTroubleshooting:");
    println!("  1. Start Ollama:     ollama serve");
    println!("  2. Start llama.cpp:  ./scripts/start_llama_cpp_server.sh models/gguf/qwen2.5-0.5b-instruct-q4_k_m.gguf 8080");
    println!("  3. Set API keys:     export OPENAI_API_KEY='sk-...'");
    println!("                       export DEEPSEEK_API_KEY='sk-...'");
    
    Err("No backends available".into())
}

/// Test a single backend configuration
async fn test_backend(
    _name: &str,
    config: InferenceConfig,
    request: InferenceRequest,
) -> Result<(), InferenceError> {
    let engine = InferenceEngine::new(config)?;
    
    let start = std::time::Instant::now();
    let response = engine.infer(request).await?;
    let elapsed = start.elapsed();
    
    println!("   ✓ Response: {}", response.content.trim());
    println!("   ✓ Latency: {}ms", elapsed.as_millis());
    if let Some(prompt_tokens) = response.prompt_tokens {
        println!("   ✓ Prompt tokens: {}", prompt_tokens);
    }
    if let Some(completion_tokens) = response.completion_tokens {
        println!("   ✓ Completion tokens: {}", completion_tokens);
    }
    println!();
    
    Ok(())
}
