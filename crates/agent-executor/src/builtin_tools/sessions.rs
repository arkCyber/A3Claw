//! `sessions_*` — agent session management tools.
//!
//! Mirrors the official OpenClaw session tools:
//! - `sessions_list`   — list active/recent sessions
//! - `sessions_history` — fetch message history for a session
//! - `sessions_send`   — send a message into a session
//! - `sessions_spawn`  — spawn a new agent session
//! - `session_status`  — get status of a specific session
//!
//! Delegates to the Gateway `/sessions/*` endpoints.
//! Provides descriptive stubs when the gateway is unreachable.

pub async fn sessions_list(
    client: &reqwest::Client,
    gateway_url: &str,
    args: &serde_json::Value,
) -> String {
    let limit = args["limit"].as_u64().unwrap_or(20);
    let url = format!("{}/sessions?limit={}", gateway_url, limit);
    match client.get(&url).send().await {
        Ok(r) if r.status().is_success() => r.text().await.unwrap_or_default(),
        Ok(r) => format!("(sessions_list: gateway returned HTTP {})", r.status()),
        Err(e) => format!("(sessions_list: gateway unreachable — {})", e),
    }
}

pub async fn sessions_history(
    client: &reqwest::Client,
    gateway_url: &str,
    args: &serde_json::Value,
) -> String {
    let session_id = match args["sessionId"].as_str().or_else(|| args["session_id"].as_str()) {
        Some(id) => id,
        None => return "(sessions_history: missing 'sessionId')".to_string(),
    };
    let limit = args["limit"].as_u64().unwrap_or(50);
    let url = format!("{}/sessions/{}/history?limit={}", gateway_url, session_id, limit);
    match client.get(&url).send().await {
        Ok(r) if r.status().is_success() => r.text().await.unwrap_or_default(),
        Ok(r) => format!("(sessions_history: HTTP {})", r.status()),
        Err(e) => format!("(sessions_history: gateway unreachable — {})", e),
    }
}

pub async fn sessions_send(
    client: &reqwest::Client,
    gateway_url: &str,
    args: &serde_json::Value,
) -> String {
    let session_id = match args["sessionId"].as_str().or_else(|| args["session_id"].as_str()) {
        Some(id) => id,
        None => return "(sessions_send: missing 'sessionId')".to_string(),
    };
    let message = match args["message"].as_str() {
        Some(m) => m,
        None => return "(sessions_send: missing 'message')".to_string(),
    };
    let url = format!("{}/sessions/{}/send", gateway_url, session_id);
    let payload = serde_json::json!({ "message": message });
    match client.post(&url).json(&payload).send().await {
        Ok(r) if r.status().is_success() => r.text().await.unwrap_or_default(),
        Ok(r) => format!("(sessions_send: HTTP {})", r.status()),
        Err(e) => format!("(sessions_send: gateway unreachable — {})", e),
    }
}

pub async fn sessions_spawn(
    client: &reqwest::Client,
    gateway_url: &str,
    args: &serde_json::Value,
) -> String {
    let agent_id = match args["agentId"].as_str().or_else(|| args["agent_id"].as_str()) {
        Some(id) => id,
        None => return "(sessions_spawn: missing 'agentId')".to_string(),
    };
    let goal = args["goal"].as_str().unwrap_or("");
    let url = format!("{}/sessions/spawn", gateway_url);
    let payload = serde_json::json!({ "agentId": agent_id, "goal": goal });
    match client.post(&url).json(&payload).send().await {
        Ok(r) if r.status().is_success() => r.text().await.unwrap_or_default(),
        Ok(r) => format!(
            "(sessions_spawn: HTTP {} — ensure agent '{}' exists in OpenClaw+)",
            r.status(), agent_id
        ),
        Err(e) => format!("(sessions_spawn: gateway unreachable — {})", e),
    }
}

pub async fn session_status(
    client: &reqwest::Client,
    gateway_url: &str,
    args: &serde_json::Value,
) -> String {
    let session_id = match args["sessionId"].as_str().or_else(|| args["session_id"].as_str()) {
        Some(id) => id,
        None => return "(session_status: missing 'sessionId')".to_string(),
    };
    let url = format!("{}/sessions/{}/status", gateway_url, session_id);
    match client.get(&url).send().await {
        Ok(r) if r.status().is_success() => r.text().await.unwrap_or_default(),
        Ok(r) => format!("(session_status: HTTP {})", r.status()),
        Err(e) => format!("(session_status: gateway unreachable — {})", e),
    }
}

pub async fn agents_list(
    client: &reqwest::Client,
    gateway_url: &str,
    _args: &serde_json::Value,
) -> String {
    let url = format!("{}/agents", gateway_url);
    match client.get(&url).send().await {
        Ok(r) if r.status().is_success() => r.text().await.unwrap_or_default(),
        Ok(r) => format!("(agents_list: HTTP {})", r.status()),
        Err(e) => format!("(agents_list: gateway unreachable — {})", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_client() -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(100))
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn sessions_list_unreachable_graceful() {
        let client = make_client();
        let out = sessions_list(&client, "http://127.0.0.1:1", &serde_json::json!({})).await;
        assert!(out.contains("gateway unreachable") || out.contains("sessions_list"));
    }

    #[tokio::test]
    async fn sessions_history_missing_id() {
        let client = make_client();
        let out = sessions_history(&client, "http://127.0.0.1:1", &serde_json::json!({})).await;
        assert!(out.contains("missing 'sessionId'"));
    }

    #[tokio::test]
    async fn sessions_send_missing_session() {
        let client = make_client();
        let out = sessions_send(&client, "http://127.0.0.1:1", &serde_json::json!({"message": "hi"})).await;
        assert!(out.contains("missing 'sessionId'"));
    }

    #[tokio::test]
    async fn sessions_send_missing_message() {
        let client = make_client();
        let out = sessions_send(
            &client,
            "http://127.0.0.1:1",
            &serde_json::json!({"sessionId": "abc"}),
        ).await;
        assert!(out.contains("missing 'message'"));
    }

    #[tokio::test]
    async fn sessions_spawn_missing_agent() {
        let client = make_client();
        let out = sessions_spawn(&client, "http://127.0.0.1:1", &serde_json::json!({})).await;
        assert!(out.contains("missing 'agentId'"));
    }

    #[tokio::test]
    async fn session_status_missing_id() {
        let client = make_client();
        let out = session_status(&client, "http://127.0.0.1:1", &serde_json::json!({})).await;
        assert!(out.contains("missing 'sessionId'"));
    }
}
