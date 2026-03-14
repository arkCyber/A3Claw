// WASI-NN inference demo
// Run with: cargo run --release --features wasi-nn --example wasi_nn_inference_demo

use openclaw_inference::{InferenceEngine, InferenceConfig, BackendKind, InferenceRequest};
use openclaw_inference::types::ConversationTurn;
use std::path::PathBuf;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== WASI-NN Inference Demo ===\n");

    // Configure WASI-NN backend
    let config = InferenceConfig {
        backend: BackendKind::WasiNn,
        model_path: Some(PathBuf::from("models/gguf/qwen2.5-0.5b-instruct-q4_k_m.gguf")),
        model_sha256: None, // Optional: Some("74a4da8c9fdbcd15bd1f6d01d621410d31c6fc00986f5eb687824e7b93d7a9db".to_string())
        endpoint: "http://localhost:8080".into(),
        model_name: "qwen2.5-0.5b-instruct".into(),
        api_key: None,
        max_tokens: 256,
        temperature: 0.7,
        top_p: 0.9,
        inference_timeout: Duration::from_secs(120),
        circuit_breaker_threshold: 3,
        circuit_breaker_reset: Duration::from_secs(30),
        context_window: 8192,
    };

    println!("Initializing inference engine...");
    let engine = InferenceEngine::new(config)?;
    println!("✓ Engine initialized\n");

    // Test 1: Simple question
    println!("Test 1: Simple question");
    println!("Question: What is Rust programming language?");
    
    let request1 = InferenceRequest {
        request_id: 1,
        messages: vec![
            ConversationTurn {
                role: "system".to_string(),
                content: "You are a helpful programming assistant. Answer concisely.".to_string(),
            },
            ConversationTurn {
                role: "user".to_string(),
                content: "What is Rust programming language? Answer in 2-3 sentences.".to_string(),
            },
        ],
        max_tokens_override: Some(128),
        temperature_override: Some(0.7),
        stream: false,
    };

    println!("Running inference...");
    let start = std::time::Instant::now();
    let response1 = engine.infer(request1).await?;
    let elapsed = start.elapsed();
    
    println!("Response: {}", response1.content);
    println!("Time: {:.2}s", elapsed.as_secs_f64());
    println!("Tokens: {} input + {} output", response1.prompt_tokens.unwrap_or(0), response1.completion_tokens.unwrap_or(0));
    println!();

    // Test 2: Code generation
    println!("Test 2: Code generation");
    println!("Question: Write a Fibonacci function in Rust");
    
    let request2 = InferenceRequest {
        request_id: 2,
        messages: vec![
            ConversationTurn {
                role: "system".to_string(),
                content: "You are a Rust expert. Write clean, idiomatic code.".to_string(),
            },
            ConversationTurn {
                role: "user".to_string(),
                content: "Write a simple Fibonacci function in Rust. Just show the code.".to_string(),
            },
        ],
        max_tokens_override: Some(256),
        temperature_override: Some(0.3),
        stream: false,
    };

    println!("Running inference...");
    let start = std::time::Instant::now();
    let response2 = engine.infer(request2).await?;
    let elapsed = start.elapsed();
    
    println!("Response:\n{}", response2.content);
    println!("Time: {:.2}s", elapsed.as_secs_f64());
    println!("Tokens: {} input + {} output", response2.prompt_tokens.unwrap_or(0), response2.completion_tokens.unwrap_or(0));
    println!();

    // Test 3: Multi-turn conversation
    println!("Test 3: Multi-turn conversation");
    
    let request3 = InferenceRequest {
        request_id: 3,
        messages: vec![
            ConversationTurn {
                role: "system".to_string(),
                content: "You are a helpful assistant.".to_string(),
            },
            ConversationTurn {
                role: "user".to_string(),
                content: "What is 2+2?".to_string(),
            },
            ConversationTurn {
                role: "assistant".to_string(),
                content: "2+2 equals 4.".to_string(),
            },
            ConversationTurn {
                role: "user".to_string(),
                content: "What about 3+3?".to_string(),
            },
        ],
        max_tokens_override: Some(64),
        temperature_override: Some(0.5),
        stream: false,
    };

    println!("Running inference...");
    let start = std::time::Instant::now();
    let response3 = engine.infer(request3).await?;
    let elapsed = start.elapsed();
    
    println!("Response: {}", response3.content);
    println!("Time: {:.2}s", elapsed.as_secs_f64());
    println!("Tokens: {} input + {} output", response3.prompt_tokens.unwrap_or(0), response3.completion_tokens.unwrap_or(0));
    println!();

    println!("=== All tests completed successfully! ===");
    
    Ok(())
}
