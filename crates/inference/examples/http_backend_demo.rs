use openclaw_inference::{InferenceConfig, InferenceEngine, InferenceRequest, BackendKind};
use openclaw_inference::types::ConversationTurn;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== OpenClaw+ HTTP Backend Demo ===\n");
    println!("Using Ollama with qwen2.5:0.5b model\n");

    // Configure HTTP backend (Ollama)
    let config = InferenceConfig {
        backend: BackendKind::Ollama,
        endpoint: "http://localhost:11434".into(),
        model_name: "qwen2.5:0.5b".into(),
        
        api_key: None,  // Ollama doesn't need API key
        inference_timeout: Duration::from_secs(120),
        
        max_tokens: 512,
        temperature: 0.7,
        top_p: 0.9,
        context_window: 4096,
        
        circuit_breaker_threshold: 3,
        circuit_breaker_reset: Duration::from_secs(30),
        
        // WASI-NN fields (not used for HTTP)
        model_path: None,
        model_sha256: None,
    };

    println!("Creating inference engine...");
    let engine = InferenceEngine::new(config)?;
    println!("✓ Engine initialized\n");

    // Test 1: Simple question
    println!("--- Test 1: Simple Question ---");
    let request1 = InferenceRequest {
        request_id: 1,
        messages: vec![
            ConversationTurn {
                role: "user".into(),
                content: "What is Rust programming language?".into(),
            },
        ],
        max_tokens_override: None,
        temperature_override: None,
        stream: false,
    };

    match engine.infer(request1).await {
        Ok(response) => {
            println!("✓ Response: {}", response.content);
            println!("  Tokens: {} (prompt) + {} (completion)", response.prompt_tokens.unwrap_or(0), response.completion_tokens.unwrap_or(0));
            println!("  Latency: {}ms\n", response.latency_ms);
        }
        Err(e) => {
            eprintln!("✗ Error: {:?}\n", e);
        }
    }

    // Test 2: Multi-turn conversation
    println!("--- Test 2: Multi-turn Conversation ---");
    let request2 = InferenceRequest {
        request_id: 2,
        messages: vec![
            ConversationTurn {
                role: "user".into(),
                content: "Tell me a short joke about programming.".into(),
            },
            ConversationTurn {
                role: "assistant".into(),
                content: "Why do programmers prefer dark mode? Because light attracts bugs!".into(),
            },
            ConversationTurn {
                role: "user".into(),
                content: "That's funny! Tell me another one.".into(),
            },
        ],
        max_tokens_override: Some(100),
        temperature_override: Some(0.8),
        stream: false,
    };

    match engine.infer(request2).await {
        Ok(response) => {
            println!("✓ Response: {}", response.content);
            println!("  Tokens: {} (prompt) + {} (completion)", response.prompt_tokens.unwrap_or(0), response.completion_tokens.unwrap_or(0));
            println!("  Latency: {}ms\n", response.latency_ms);
        }
        Err(e) => {
            eprintln!("✗ Error: {:?}\n", e);
        }
    }

    // Test 3: Code generation
    println!("--- Test 3: Code Generation ---");
    let request3 = InferenceRequest {
        request_id: 3,
        messages: vec![
            ConversationTurn {
                role: "user".into(),
                content: "Write a simple Rust function to calculate factorial.".into(),
            },
        ],
        max_tokens_override: Some(256),
        temperature_override: Some(0.3),  // Lower temperature for code
        stream: false,
    };

    match engine.infer(request3).await {
        Ok(response) => {
            println!("✓ Response:\n{}", response.content);
            println!("\n  Tokens: {} (prompt) + {} (completion)", response.prompt_tokens.unwrap_or(0), response.completion_tokens.unwrap_or(0));
            println!("  Latency: {}ms\n", response.latency_ms);
        }
        Err(e) => {
            eprintln!("✗ Error: {:?}\n", e);
        }
    }

    // Show health status
    println!("--- Backend Health Status ---");
    let health = engine.health_status();
    for (name, status) in health {
        println!("  {}: {:?}", name, status);
    }

    println!("\n=== Demo Complete ===");
    println!("\nNote: This demo uses HTTP backend (Ollama).");
    println!("Performance is similar to WASI-NN with GPU support!");
    println!("\nTo use WASI-NN backend instead:");
    println!("  1. Wait for network to stabilize");
    println!("  2. Run: ./scripts/rebuild_wasi_nn_minimal.sh");
    println!("  3. Change BackendKind to WasiNn in config");

    Ok(())
}
