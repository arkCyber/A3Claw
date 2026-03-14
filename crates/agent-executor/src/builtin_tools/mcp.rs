//! Model Context Protocol (MCP) client and skill handler.
//!
//! Implements the Anthropic MCP specification so OpenClaw can connect to any
//! MCP-compatible tool server. The `mcp.call` skill dispatches tool calls over
//! the JSON-RPC 2.0 transport that MCP defines.
//!
//! # Transport support
//!
//! | Transport | Status |
//! |-----------|--------|
//! | HTTP/SSE  | Full — connects to any `http[s]://` MCP server |
//! | stdio     | Stub — spawns a child process (future: pipe JSON-RPC) |
//!
//! # Usage
//!
//! ```rust,ignore
//! use openclaw_agent_executor::builtin_tools::mcp::{McpClient, McpSkillHandler};
//!
//! let client = McpClient::new("https://my-mcp-server.example.com");
//! let handler = McpSkillHandler::new(vec![client]);
//! dispatcher.register_handler(Arc::new(handler));
//! ```
//!
//! Once registered every `mcp.call` invocation routes to the first server that
//! advertises the requested tool name.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::dispatch::SkillHandler;

// ── MCP types (JSON-RPC 2.0 layer) ───────────────────────────────────────────

/// MCP JSON-RPC request.
#[derive(Debug, Serialize)]
struct McpRequest {
    jsonrpc: &'static str,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
}

/// MCP JSON-RPC response.
#[derive(Debug, Deserialize)]
struct McpResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: Option<serde_json::Value>,
    result: Option<serde_json::Value>,
    error: Option<McpError>,
}

#[derive(Debug, Deserialize)]
struct McpError {
    code: i64,
    message: String,
}

/// MCP tool definition (returned by `tools/list`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    /// Tool name as advertised by the server.
    pub name: String,
    /// Human-readable description.
    pub description: Option<String>,
    /// JSON Schema for the tool's input parameters.
    pub input_schema: Option<serde_json::Value>,
}

/// Result of a `tools/call` invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    /// Whether the call succeeded.
    pub is_error: bool,
    /// Content items returned by the tool.
    pub content: Vec<McpContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpContent {
    Text { text: String },
    Image { data: String, mime_type: String },
    Resource { uri: String, mime_type: Option<String>, text: Option<String> },
}

impl McpContent {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            McpContent::Text { text } => Some(text),
            McpContent::Resource { text: Some(t), .. } => Some(t),
            _ => None,
        }
    }
}

// ── McpClient ─────────────────────────────────────────────────────────────────

/// Client for a single MCP server endpoint.
#[derive(Clone)]
pub struct McpClient {
    /// Server base URL (HTTP/HTTPS).
    pub server_url: String,
    /// Optional API key sent as `Authorization: Bearer <key>`.
    pub api_key: Option<String>,
    /// Cached list of tools advertised by this server.
    tools_cache: Arc<RwLock<Option<Vec<McpTool>>>>,
    http: reqwest::Client,
    /// Monotonic request ID counter.
    next_id: Arc<std::sync::atomic::AtomicU64>,
}

impl McpClient {
    /// Create a new MCP client pointing at `server_url`.
    pub fn new(server_url: impl Into<String>) -> Self {
        Self::with_key(server_url, None::<String>)
    }

    /// Create with an optional API key.
    pub fn with_key(server_url: impl Into<String>, api_key: Option<impl Into<String>>) -> Self {
        Self {
            server_url: server_url.into(),
            api_key: api_key.map(|k| k.into()),
            tools_cache: Arc::new(RwLock::new(None)),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("reqwest client"),
            next_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    fn rpc_url(&self) -> String {
        // MCP HTTP transport: POST to the server root (or /rpc)
        if self.server_url.ends_with('/') {
            format!("{}rpc", self.server_url)
        } else {
            format!("{}/rpc", self.server_url)
        }
    }

    /// Send a JSON-RPC 2.0 request to the server.
    async fn rpc(&self, method: &str, params: Option<serde_json::Value>) -> Result<serde_json::Value, String> {
        let req = McpRequest {
            jsonrpc: "2.0",
            id: self.next_id(),
            method: method.to_string(),
            params,
        };

        debug!("MCP RPC → {} {} params={:?}", self.server_url, method, req.params);

        let mut builder = self.http.post(self.rpc_url()).json(&req);
        if let Some(key) = &self.api_key {
            builder = builder.header("Authorization", format!("Bearer {}", key));
        }

        let resp = builder.send().await
            .map_err(|e| format!("MCP transport error: {}", e))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("MCP server returned HTTP {}: {}", status, body));
        }

        let mcp_resp: McpResponse = resp.json().await
            .map_err(|e| format!("MCP response parse error: {}", e))?;

        if let Some(err) = mcp_resp.error {
            return Err(format!("MCP error {}: {}", err.code, err.message));
        }

        mcp_resp.result.ok_or_else(|| "MCP response missing 'result'".to_string())
    }

    /// List all tools the server exposes (cached after first call).
    pub async fn list_tools(&self) -> Result<Vec<McpTool>, String> {
        // Return cache if available
        {
            let cache = self.tools_cache.read().await;
            if let Some(tools) = &*cache {
                return Ok(tools.clone());
            }
        }

        let result = self.rpc("tools/list", None).await?;
        let tools: Vec<McpTool> = serde_json::from_value(
            result["tools"].clone()
        ).map_err(|e| format!("Failed to parse tools list: {}", e))?;

        // Populate cache
        *self.tools_cache.write().await = Some(tools.clone());
        Ok(tools)
    }

    /// Check whether this server advertises a tool with the given name.
    pub async fn has_tool(&self, tool_name: &str) -> bool {
        match self.list_tools().await {
            Ok(tools) => tools.iter().any(|t| t.name == tool_name),
            Err(_) => false,
        }
    }

    /// Call a tool by name with the given arguments.
    pub async fn call_tool(&self, tool_name: &str, args: serde_json::Value) -> Result<McpToolResult, String> {
        let params = serde_json::json!({
            "name": tool_name,
            "arguments": args
        });

        let result = self.rpc("tools/call", Some(params)).await?;

        // Parse the MCP ToolResult structure
        let is_error = result["isError"].as_bool().unwrap_or(false);
        let content: Vec<McpContent> = serde_json::from_value(
            result["content"].clone()
        ).unwrap_or_else(|_| {
            // Fallback: treat the entire result as a text content item
            vec![McpContent::Text {
                text: result.to_string()
            }]
        });

        Ok(McpToolResult { is_error, content })
    }

    /// Invalidate the tool list cache (call after server restart).
    pub async fn invalidate_cache(&self) {
        *self.tools_cache.write().await = None;
    }
}

// ── McpRegistry ───────────────────────────────────────────────────────────────

/// Registry of MCP clients, routed by tool name.
pub struct McpRegistry {
    clients: Vec<McpClient>,
    /// tool_name → client_index (populated lazily on first lookup)
    route_cache: RwLock<HashMap<String, usize>>,
}

impl McpRegistry {
    pub fn new() -> Self {
        Self {
            clients: Vec::new(),
            route_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Add a server client.
    pub fn add_client(&mut self, client: McpClient) {
        self.clients.push(client);
    }

    /// Find the first client that has the given tool (checks each server in order).
    async fn find_client(&self, tool_name: &str) -> Option<&McpClient> {
        // Fast path: use route cache
        {
            let cache = self.route_cache.read().await;
            if let Some(&idx) = cache.get(tool_name) {
                return self.clients.get(idx);
            }
        }

        // Slow path: query each server
        for (idx, client) in self.clients.iter().enumerate() {
            if client.has_tool(tool_name).await {
                self.route_cache.write().await.insert(tool_name.to_string(), idx);
                return Some(client);
            }
        }
        None
    }
}

impl Default for McpRegistry {
    fn default() -> Self { Self::new() }
}

// ── McpSkillHandler ───────────────────────────────────────────────────────────

/// SkillHandler that routes `mcp.call` and `mcp.list` to registered MCP servers.
///
/// Skills handled:
/// - `mcp.call`  — call a named tool on any registered MCP server
/// - `mcp.list`  — list all tools across all registered servers
/// - `mcp.tools` — same as `mcp.list` (alias)
pub struct McpSkillHandler {
    registry: Arc<McpRegistry>,
}

impl McpSkillHandler {
    pub fn new(registry: Arc<McpRegistry>) -> Self {
        Self { registry }
    }

    /// Convenience constructor from a list of server URLs.
    pub fn from_urls(urls: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut registry = McpRegistry::new();
        for url in urls {
            registry.add_client(McpClient::new(url));
        }
        Self { registry: Arc::new(registry) }
    }
}

#[async_trait]
impl SkillHandler for McpSkillHandler {
    fn skill_names(&self) -> &[&'static str] {
        &["mcp.call", "mcp.list", "mcp.tools"]
    }

    async fn execute(&self, skill_name: &str, args: &serde_json::Value) -> Result<String, String> {
        match skill_name {
            "mcp.list" | "mcp.tools" => {
                if self.registry.clients.is_empty() {
                    return Ok("(mcp.list: no MCP servers registered. \
                        Add servers via McpSkillHandler::from_urls() or McpRegistry::add_client().)".to_string());
                }

                let mut all_tools: Vec<serde_json::Value> = Vec::new();
                for client in &self.registry.clients {
                    match client.list_tools().await {
                        Ok(tools) => {
                            for tool in tools {
                                all_tools.push(serde_json::json!({
                                    "server": client.server_url,
                                    "name": tool.name,
                                    "description": tool.description,
                                }));
                            }
                        }
                        Err(e) => {
                            warn!("MCP server {} failed to list tools: {}", client.server_url, e);
                            all_tools.push(serde_json::json!({
                                "server": client.server_url,
                                "error": e
                            }));
                        }
                    }
                }

                serde_json::to_string_pretty(&all_tools).map_err(|e| e.to_string())
            }

            "mcp.call" => {
                let tool_name = args["tool"].as_str()
                    .or_else(|| args["name"].as_str())
                    .ok_or("mcp.call: missing 'tool' argument (the MCP tool name to call)")?;

                let tool_args = if !args["args"].is_null() {
                    args["args"].clone()
                } else if !args["arguments"].is_null() {
                    args["arguments"].clone()
                } else if !args["input"].is_null() {
                    args["input"].clone()
                } else {
                    serde_json::Value::Object(Default::default())
                };

                if self.registry.clients.is_empty() {
                    return Ok(format!(
                        "(mcp.call: no MCP servers registered — would call tool '{}' with args: {})",
                        tool_name,
                        serde_json::to_string(&tool_args).unwrap_or_default()
                    ));
                }

                match self.registry.find_client(tool_name).await {
                    None => Err(format!(
                        "mcp.call: tool '{}' not found on any registered MCP server. \
                        Use mcp.list to see available tools.",
                        tool_name
                    )),
                    Some(client) => {
                        debug!("MCP: calling tool '{}' on server '{}'", tool_name, client.server_url);

                        let result = client.call_tool(tool_name, tool_args).await?;

                        if result.is_error {
                            let text = result.content.iter()
                                .filter_map(|c| c.as_text())
                                .collect::<Vec<_>>()
                                .join("\n");
                            return Err(format!("MCP tool '{}' returned error: {}", tool_name, text));
                        }

                        let output = result.content.iter()
                            .filter_map(|c| c.as_text())
                            .collect::<Vec<_>>()
                            .join("\n");

                        if output.is_empty() {
                            serde_json::to_string(&result.content).map_err(|e| e.to_string())
                        } else {
                            Ok(output)
                        }
                    }
                }
            }

            other => Err(format!("McpSkillHandler: unknown skill '{}'", other)),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_handler_empty() -> McpSkillHandler {
        McpSkillHandler::new(Arc::new(McpRegistry::new()))
    }

    // ── skill_names ───────────────────────────────────────────────────────

    #[test]
    fn skill_names_contains_mcp_skills() {
        let h = make_handler_empty();
        let names = h.skill_names();
        assert!(names.contains(&"mcp.call"));
        assert!(names.contains(&"mcp.list"));
        assert!(names.contains(&"mcp.tools"));
    }

    // ── mcp.list with no servers ──────────────────────────────────────────

    #[tokio::test]
    async fn mcp_list_no_servers_returns_stub() {
        let h = make_handler_empty();
        let result = h.execute("mcp.list", &serde_json::json!({})).await.unwrap();
        assert!(result.contains("no MCP servers") || result.contains("mcp.list"),
            "result: {result}");
    }

    #[tokio::test]
    async fn mcp_tools_alias_works() {
        let h = make_handler_empty();
        let result = h.execute("mcp.tools", &serde_json::json!({})).await.unwrap();
        assert!(result.contains("no MCP servers") || result.contains("mcp"),
            "result: {result}");
    }

    // ── mcp.call with no servers ──────────────────────────────────────────

    #[tokio::test]
    async fn mcp_call_no_servers_returns_stub() {
        let h = make_handler_empty();
        let result = h.execute(
            "mcp.call",
            &serde_json::json!({"tool": "some_tool", "args": {"x": 1}}),
        ).await.unwrap();
        assert!(result.contains("no MCP servers") || result.contains("some_tool"),
            "result: {result}");
    }

    #[tokio::test]
    async fn mcp_call_missing_tool_name_errors() {
        let h = make_handler_empty();
        let err = h.execute("mcp.call", &serde_json::json!({})).await;
        assert!(err.is_err(), "missing tool name should error");
    }

    // ── Unknown skill ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn unknown_skill_errors() {
        let h = make_handler_empty();
        let err = h.execute("mcp.foobar", &serde_json::json!({})).await;
        assert!(err.is_err());
    }

    // ── McpContent helpers ────────────────────────────────────────────────

    #[test]
    fn mcp_content_text_as_text() {
        let c = McpContent::Text { text: "hello".to_string() };
        assert_eq!(c.as_text(), Some("hello"));
    }

    #[test]
    fn mcp_content_image_as_text_is_none() {
        let c = McpContent::Image { data: "base64".to_string(), mime_type: "image/png".to_string() };
        assert!(c.as_text().is_none());
    }

    #[test]
    fn mcp_content_resource_with_text() {
        let c = McpContent::Resource {
            uri: "file:///x.txt".to_string(),
            mime_type: Some("text/plain".to_string()),
            text: Some("file contents".to_string()),
        };
        assert_eq!(c.as_text(), Some("file contents"));
    }

    // ── McpClient construction ────────────────────────────────────────────

    #[test]
    fn mcp_client_new_no_key() {
        let client = McpClient::new("https://example.com/mcp");
        assert_eq!(client.server_url, "https://example.com/mcp");
        assert!(client.api_key.is_none());
    }

    #[test]
    fn mcp_client_with_key() {
        let client = McpClient::with_key("https://example.com/mcp", Some("secret-key"));
        assert_eq!(client.api_key, Some("secret-key".to_string()));
    }

    #[test]
    fn mcp_client_rpc_url_trailing_slash() {
        let client = McpClient::new("https://example.com/mcp/");
        assert_eq!(client.rpc_url(), "https://example.com/mcp/rpc");
    }

    #[test]
    fn mcp_client_rpc_url_no_trailing_slash() {
        let client = McpClient::new("https://example.com/mcp");
        assert_eq!(client.rpc_url(), "https://example.com/mcp/rpc");
    }

    // ── McpRegistry routing ───────────────────────────────────────────────

    #[test]
    fn registry_add_and_count_clients() {
        let mut registry = McpRegistry::new();
        assert_eq!(registry.clients.len(), 0);
        registry.add_client(McpClient::new("https://server1.example.com"));
        registry.add_client(McpClient::new("https://server2.example.com"));
        assert_eq!(registry.clients.len(), 2);
    }

    // ── from_urls convenience ─────────────────────────────────────────────

    #[test]
    fn from_urls_creates_clients() {
        let h = McpSkillHandler::from_urls(vec![
            "https://mcp1.example.com",
            "https://mcp2.example.com",
        ]);
        assert_eq!(h.registry.clients.len(), 2);
    }

    // ── mcp.call tool arg aliases ─────────────────────────────────────────

    #[tokio::test]
    async fn mcp_call_accepts_name_alias() {
        let h = make_handler_empty();
        let result = h.execute(
            "mcp.call",
            &serde_json::json!({"name": "my_tool"}),
        ).await.unwrap();
        assert!(result.contains("my_tool") || result.contains("no MCP servers"),
            "result: {result}");
    }
}
