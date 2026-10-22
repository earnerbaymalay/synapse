//! SYNAPSE Runtime Gateway & Tool-Calling Stabilizer
//!
//! Provides a local-first, zero-telemetry HTTP reverse proxy on localhost
//! that intercepts OpenAI-compatible (`/v1/chat/completions`) and Anthropic-compatible
//! (`/v1/messages`) requests from coding agents (Claude Code, Antigravity CLI, Cursor, Aider).
//!
//! Features:
//! - Sensory Gating: automatically compresses code payloads and terminal output inside messages
//! - Tool-Calling Normalization: validates and repairs local model tool calls (GBNF / JSON schema)
//! - Upstream Routing: connects to local Ollama (11434), llama.cpp (8080), or BYOK endpoints
//! - Zero-telemetry audit logging of token savings
//!
//! Security model: the gateway attaches the *user's* upstream API key, so
//! it must only serve the user's own agents.
//! - No CORS headers: browsers cannot read responses or send preflighted
//!   JSON/auth requests cross-origin.
//! - Host allowlist when bound to loopback: defeats DNS rebinding.
//! - Per-launch bearer token on `/v1/*` (`Authorization: Bearer <t>` or
//!   `x-api-key: <t>`), generated at start. `SYNAPSE_GATEWAY_TOKEN` pins
//!   it; `SYNAPSE_GATEWAY_NO_TOKEN=1` disables it.
//! - The client's credentials are never forwarded; only `config.api_key` is.

use std::io::Read;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Digest;
use tiny_http::{Header, Response, Server, StatusCode};

use crate::compression::{compress_text, estimate_tokens, CompressionLevel, SpoolManager};
use crate::security::constant_time_eq;

/// Largest request body accepted (413 above this).
pub const MAX_BODY_BYTES: usize = 8 * 1024 * 1024;
/// Fixed handler pool size: bounds concurrent upstream calls and threads.
const WORKER_THREADS: usize = 16;
const ANTHROPIC_VERSION: &str = "2023-06-01";
/// Env var that pins the gateway token instead of generating one.
pub const TOKEN_ENV: &str = "SYNAPSE_GATEWAY_TOKEN";
/// Env var that, set to `1`/`true`, disables the gateway token.
pub const NO_TOKEN_ENV: &str = "SYNAPSE_GATEWAY_NO_TOKEN";

/// Upstream provider backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpstreamKind {
    Ollama,
    LlamaCpp,
    OpenAi,
    Anthropic,
    Generic,
}

impl UpstreamKind {
    pub fn from_url_or_name(s: &str) -> Self {
        let lower = s.to_lowercase();
        if lower.contains("11434") || lower.contains("ollama") {
            Self::Ollama
        } else if lower.contains("8080") || lower.contains("llama") {
            Self::LlamaCpp
        } else if lower.contains("anthropic") {
            Self::Anthropic
        } else if lower.contains("openai") || lower.contains("openrouter") {
            Self::OpenAi
        } else {
            Self::Generic
        }
    }
}

/// Configuration for the SYNAPSE Gateway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub bind_addr: String,
    pub upstream_url: String,
    pub upstream_kind: UpstreamKind,
    pub default_model: Option<String>,
    pub gate_level: CompressionLevel,
    pub api_key: Option<String>,
    /// When `Some(n)`, fold large tool results older than the newest `n`
    /// messages into restorable stubs (see [`crate::fold`]). `None` = off.
    #[serde(default)]
    pub fold_keep_recent: Option<usize>,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:8088".to_string(),
            upstream_url: "http://127.0.0.1:11434".to_string(),
            upstream_kind: UpstreamKind::Ollama,
            default_model: Some("qwen2.5-coder:7b".to_string()),
            gate_level: CompressionLevel::Balanced,
            api_key: None,
            fold_keep_recent: None,
        }
    }
}

/// Real-time Gateway metrics and telemetry (zero-telemetry, stored locally only)
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GatewayStats {
    pub total_requests: u64,
    pub prompt_tokens_raw: usize,
    pub prompt_tokens_gated: usize,
    pub completion_tokens: usize,
    pub tokens_saved: usize,
    pub avg_reduction_percent: u32,
    pub last_model: String,
    pub last_latency_ms: u64,
}

/// How the `/v1/*` bearer token is chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenPolicy {
    /// No token required (explicit opt-out).
    Disabled,
    /// Use this exact token.
    Fixed(String),
    /// Generate a fresh random token at start (default).
    Generate,
}

impl TokenPolicy {
    /// Resolves the policy from the values of [`NO_TOKEN_ENV`] and
    /// [`TOKEN_ENV`] (opt-out wins).
    pub fn from_values(no_token: Option<&str>, fixed: Option<&str>) -> Self {
        if no_token.is_some_and(|v| matches!(v.trim(), "1" | "true" | "yes")) {
            Self::Disabled
        } else if let Some(t) = fixed.map(str::trim).filter(|t| !t.is_empty()) {
            Self::Fixed(t.to_string())
        } else {
            Self::Generate
        }
    }

    pub fn from_env() -> Self {
        Self::from_values(
            std::env::var(NO_TOKEN_ENV).ok().as_deref(),
            std::env::var(TOKEN_ENV).ok().as_deref(),
        )
    }
}

/// 32 random bytes from the OS CSPRNG, hex-encoded.
pub fn generate_token() -> Result<String, String> {
    let mut buf = [0u8; 32];
    getrandom::getrandom(&mut buf)
        .map_err(|e| format!("cannot generate gateway token from the OS RNG: {e}"))?;
    Ok(hex::encode(buf))
}

/// Per-server state shared by the worker pool.
struct RequestContext {
    config: GatewayConfig,
    token: Option<String>,
    /// `Host` values accepted; `None` when bound to a non-loopback address.
    allowed_hosts: Option<Vec<String>>,
}

/// Gateway runtime handle
pub struct GatewayServer {
    config: GatewayConfig,
    stats: Arc<RwLock<GatewayStats>>,
    running: Arc<AtomicBool>,
    token_policy: TokenPolicy,
    token: Option<String>,
    local_addr: Option<SocketAddr>,
    handles: Vec<JoinHandle<()>>,
}

impl GatewayServer {
    /// Token policy comes from the environment ([`TokenPolicy::from_env`]).
    pub fn new(config: GatewayConfig) -> Self {
        Self {
            config,
            stats: Arc::new(RwLock::new(GatewayStats::default())),
            running: Arc::new(AtomicBool::new(false)),
            token_policy: TokenPolicy::from_env(),
            token: None,
            local_addr: None,
            handles: Vec::new(),
        }
    }

    /// Overrides the environment-derived token policy (call before start).
    pub fn with_token_policy(mut self, policy: TokenPolicy) -> Self {
        self.token_policy = policy;
        self
    }

    pub fn stats(&self) -> GatewayStats {
        self.stats.read().unwrap().clone()
    }

    pub fn config(&self) -> &GatewayConfig {
        &self.config
    }

    /// The bearer token clients must send on `/v1/*`; `None` before start
    /// or when the token is disabled.
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    /// The address actually bound (useful with port 0).
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.local_addr
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Starts the gateway on a fixed pool of worker threads.
    pub fn start_background(&mut self) -> Result<(), String> {
        if self.is_running() {
            return Err("Gateway is already running".to_string());
        }

        let token = match &self.token_policy {
            TokenPolicy::Disabled => None,
            TokenPolicy::Fixed(t) => Some(t.clone()),
            TokenPolicy::Generate => Some(generate_token()?),
        };

        let server = Server::http(&self.config.bind_addr)
            .map_err(|e| format!("Failed to bind to {}: {}", self.config.bind_addr, e))?;
        let local_addr = server.server_addr().to_ip();
        let allowed_hosts = local_addr.and_then(loopback_hosts);

        let ctx = Arc::new(RequestContext {
            config: self.config.clone(),
            token: token.clone(),
            allowed_hosts,
        });
        let server = Arc::new(server);
        self.running.store(true, Ordering::SeqCst);

        for _ in 0..WORKER_THREADS {
            let server = Arc::clone(&server);
            let running = Arc::clone(&self.running);
            let stats = Arc::clone(&self.stats);
            let ctx = Arc::clone(&ctx);
            self.handles.push(thread::spawn(move || {
                while running.load(Ordering::SeqCst) {
                    match server.recv_timeout(Duration::from_millis(200)) {
                        Ok(Some(request)) => handle_request(request, &ctx, &stats),
                        Ok(None) => {}
                        Err(_) => break,
                    }
                }
            }));
        }

        self.token = token;
        self.local_addr = local_addr;
        Ok(())
    }

    /// Stops the background gateway server
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        for handle in self.handles.drain(..) {
            let _ = handle.join();
        }
    }
}

/// `Host` header values accepted for a loopback bind; `None` (no check)
/// for any other bind address, where the token is the only gate.
fn loopback_hosts(addr: SocketAddr) -> Option<Vec<String>> {
    if !addr.ip().is_loopback() {
        return None;
    }
    let port = addr.port();
    let mut hosts = vec![
        format!("127.0.0.1:{port}"),
        format!("localhost:{port}"),
        format!("[::1]:{port}"),
    ];
    if port == 80 {
        hosts.extend(["127.0.0.1", "localhost", "[::1]"].map(String::from));
    }
    Some(hosts)
}

/// Shared upstream client: connection reuse plus bounded connect/read waits.
fn upstream_agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(10))
            .timeout_read(Duration::from_secs(300))
            .build()
    })
}

fn header<'a>(request: &'a tiny_http::Request, name: &'static str) -> Option<&'a str> {
    request
        .headers()
        .iter()
        .find(|h| h.field.equiv(name))
        .map(|h| h.value.as_str())
}

/// True if the request carries `token` as `Authorization: Bearer` or
/// `x-api-key` (constant-time comparison).
fn has_valid_token(request: &tiny_http::Request, token: &str) -> bool {
    let bearer = header(request, "Authorization").and_then(|v| {
        let (scheme, rest) = v.trim().split_once(' ')?;
        scheme.eq_ignore_ascii_case("bearer").then_some(rest.trim())
    });
    [bearer, header(request, "x-api-key").map(str::trim)]
        .into_iter()
        .flatten()
        .any(|presented| constant_time_eq(presented.as_bytes(), token.as_bytes()))
}

fn send_json<T: ToString>(request: tiny_http::Request, status: u16, body: T) {
    let ctype = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
    let res = Response::from_string(body.to_string())
        .with_status_code(StatusCode(status))
        .with_header(ctype);
    let _ = request.respond(res);
}

/// Dispatches and processes an incoming HTTP request
fn handle_request(
    mut request: tiny_http::Request,
    ctx: &RequestContext,
    stats: &Arc<RwLock<GatewayStats>>,
) {
    let config = &ctx.config;
    let url = request.url().to_string();
    let method = request.method().as_str().to_string();

    // DNS rebinding: a page on evil.example resolving to 127.0.0.1 still
    // sends `Host: evil.example`.
    if let Some(allowed) = &ctx.allowed_hosts {
        let host_ok = header(&request, "Host")
            .is_some_and(|h| allowed.iter().any(|a| a.eq_ignore_ascii_case(h.trim())));
        if !host_ok {
            send_json(request, 403, json!({ "error": "Forbidden Host header" }));
            return;
        }
    }

    // Health & Info endpoints (no secrets: the upstream URL is not disclosed)
    if (url == "/health" || url == "/" || url == "/api/status") && method == "GET" {
        let current_stats = stats.read().unwrap().clone();
        let payload = json!({
            "status": "healthy",
            "service": "SYNAPSE Runtime Gateway",
            "version": "1.0.0",
            "bind_addr": config.bind_addr,
            "gate_level": format!("{:?}", config.gate_level),
            "auth": if ctx.token.is_some() { "token" } else { "none" },
            "stats": current_stats
        });
        send_json(request, 200, payload.to_string());
        return;
    }

    if url == "/api/stats" && method == "GET" {
        let current_stats = stats.read().unwrap().clone();
        send_json(request, 200, serde_json::to_string(&current_stats).unwrap());
        return;
    }

    if url.starts_with("/v1/") {
        if let Some(token) = &ctx.token {
            if !has_valid_token(&request, token) {
                send_json(
                    request,
                    401,
                    json!({ "error": "Missing or invalid gateway token (send `Authorization: Bearer <token>` or `x-api-key: <token>`; printed by `synapse serve`)" }),
                );
                return;
            }
        }
    }

    // Model list endpoint
    if url == "/v1/models" && method == "GET" {
        let models_payload = json!({
            "object": "list",
            "data": [
                {
                    "id": config.default_model.clone().unwrap_or_else(|| "default".to_string()),
                    "object": "model",
                    "created": 1700000000,
                    "owned_by": "synapse-local"
                }
            ]
        });
        send_json(request, 200, models_payload.to_string());
        return;
    }

    // Chat completion endpoint (OpenAI / Anthropic standard)
    if (url.starts_with("/v1/chat/completions") || url.starts_with("/v1/messages"))
        && method == "POST"
    {
        let too_large =
            json!({ "error": format!("Request body exceeds {} bytes", MAX_BODY_BYTES) });
        if request.body_length().is_some_and(|n| n > MAX_BODY_BYTES) {
            send_json(request, 413, too_large);
            return;
        }
        let mut raw = Vec::new();
        if let Err(e) = request
            .as_reader()
            .take(MAX_BODY_BYTES as u64 + 1)
            .read_to_end(&mut raw)
        {
            send_json(
                request,
                400,
                json!({ "error": format!("Failed to read request body: {}", e) }).to_string(),
            );
            return;
        }
        if raw.len() > MAX_BODY_BYTES {
            send_json(request, 413, too_large);
            return;
        }
        let Ok(body) = String::from_utf8(raw) else {
            send_json(
                request,
                400,
                json!({ "error": "Request body is not UTF-8" }),
            );
            return;
        };

        let start_time = std::time::Instant::now();
        match process_chat_request(&body, config) {
            Ok((response_json, raw_tokens, gated_tokens, completion_tokens, model_name)) => {
                let latency_ms = start_time.elapsed().as_millis() as u64;

                // Update real-time statistics
                {
                    let mut s = stats.write().unwrap();
                    s.total_requests += 1;
                    s.prompt_tokens_raw += raw_tokens;
                    s.prompt_tokens_gated += gated_tokens;
                    s.completion_tokens += completion_tokens;
                    if raw_tokens > gated_tokens {
                        s.tokens_saved += raw_tokens - gated_tokens;
                    }
                    if s.prompt_tokens_raw > 0 {
                        s.avg_reduction_percent =
                            ((s.tokens_saved as f64 / s.prompt_tokens_raw as f64) * 100.0) as u32;
                    }
                    s.last_model = model_name;
                    s.last_latency_ms = latency_ms;
                }

                send_json(request, 200, response_json.to_string());
            }
            Err(err) => send_json(request, err.status, err.body.to_string()),
        }
        return;
    }

    // 404 fallback
    send_json(request, 404, json!({ "error": "Not Found" }).to_string());
}

/// A failed chat request: the HTTP status to return and its JSON body.
/// Upstream HTTP errors (429, 401, 400…) keep their status and body.
#[derive(Debug, Clone, PartialEq)]
pub struct GatewayError {
    pub status: u16,
    pub body: Value,
}

impl GatewayError {
    fn new(status: u16, message: impl Into<String>) -> Self {
        Self {
            status,
            body: json!({ "error": message.into() }),
        }
    }
}

impl std::fmt::Display for GatewayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HTTP {}: {}", self.status, self.body)
    }
}

/// Applies Sensory Gating to each message's string `content` in place,
/// leaving every other field untouched. Returns `(raw_tokens, gated_tokens)`.
pub fn gate_messages(messages: &mut [Value], level: CompressionLevel) -> (usize, usize) {
    let mut raw_tokens = 0;
    let mut gated_tokens = 0;
    for msg in messages.iter_mut() {
        if let Some(content) = msg["content"].as_str() {
            let initial_tokens = estimate_tokens(content);
            raw_tokens += initial_tokens;

            // Compress large blocks (terminal outputs, giant source files)
            if content.len() > 120 {
                let (compressed, _, comp_tokens, _) = compress_text(content, level);
                if !compressed.is_empty() && comp_tokens < initial_tokens {
                    // Replace only the content: tool_call_id, tool_calls and
                    // name must survive or OpenAI-style upstreams reject the turn.
                    msg["content"] = Value::String(compressed);
                    gated_tokens += comp_tokens;
                } else {
                    gated_tokens += initial_tokens;
                }
            } else {
                gated_tokens += initial_tokens;
            }
        }
    }
    (raw_tokens, gated_tokens)
}

/// Applies Sensory Gating and tool-call normalization, then executes the
/// upstream request.
pub fn process_chat_request(
    body: &str,
    config: &GatewayConfig,
) -> Result<(Value, usize, usize, usize, String), GatewayError> {
    let mut payload: Value = serde_json::from_str(body)
        .map_err(|e| GatewayError::new(400, format!("Malformed JSON request payload: {}", e)))?;

    // Responses are rewritten as whole JSON documents (tool-call repair,
    // usage accounting), which an SSE stream cannot be. Refuse clearly
    // rather than buffer a stream and hand back the wrong shape.
    if payload.get("stream").and_then(Value::as_bool) == Some(true) {
        return Err(GatewayError::new(
            400,
            "streaming not supported by synapse gateway yet; send \"stream\": false",
        ));
    }

    let model_name = payload["model"]
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| config.default_model.clone())
        .unwrap_or_else(|| "default".to_string());

    let mut raw_prompt_tokens = 0;
    let mut gated_prompt_tokens = 0;

    // 0. Reversible folding of stale tool results (opt-in)
    if let (Some(keep), Some(messages)) =
        (config.fold_keep_recent, payload["messages"].as_array_mut())
    {
        let spool = SpoolManager::new(SpoolManager::default_dir());
        raw_prompt_tokens += crate::fold::fold_messages(messages, keep, &spool).saved();
    }

    // 1. Sensory Gating on incoming messages
    if let Some(messages) = payload["messages"].as_array_mut() {
        let (raw, gated) = gate_messages(messages, config.gate_level);
        raw_prompt_tokens += raw;
        gated_prompt_tokens += gated;
    }

    // 2. Local Model Tool Calling Schema Normalization
    let has_tools = payload
        .get("tools")
        .and_then(|t| t.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false);
    if has_tools
        && (config.upstream_kind == UpstreamKind::Ollama
            || config.upstream_kind == UpstreamKind::LlamaCpp)
    {
        inject_tool_calling_instructions(&mut payload);
    }

    // 3. Upstream Forwarding
    let upstream_target = match config.upstream_kind {
        UpstreamKind::Ollama => {
            if config.upstream_url.ends_with("/v1") {
                format!("{}/chat/completions", config.upstream_url)
            } else {
                format!(
                    "{}/v1/chat/completions",
                    config.upstream_url.trim_end_matches('/')
                )
            }
        }
        UpstreamKind::Anthropic => {
            if config.upstream_url.ends_with("/v1") {
                format!("{}/messages", config.upstream_url)
            } else {
                format!("{}/v1/messages", config.upstream_url.trim_end_matches('/'))
            }
        }
        _ => {
            if config.upstream_url.ends_with("/chat/completions") {
                config.upstream_url.clone()
            } else {
                format!(
                    "{}/v1/chat/completions",
                    config.upstream_url.trim_end_matches('/')
                )
            }
        }
    };

    // A fresh request: none of the client's headers (notably its gateway
    // token) are forwarded. Only the configured upstream key is attached.
    let mut req = upstream_agent()
        .post(&upstream_target)
        .set("Content-Type", "application/json");

    if config.upstream_kind == UpstreamKind::Anthropic {
        req = req.set("anthropic-version", ANTHROPIC_VERSION);
        if let Some(ref key) = config.api_key {
            req = req.set("x-api-key", key);
        }
    } else if let Some(ref key) = config.api_key {
        req = req.set("Authorization", &format!("Bearer {}", key));
    }

    let upstream_res = req.send_json(&payload);

    let res_val: Value = match upstream_res {
        Ok(response) => response.into_json::<Value>().map_err(|e| {
            GatewayError::new(
                502,
                format!("Failed to parse upstream response JSON: {}", e),
            )
        })?,
        Err(ureq::Error::Status(code, response)) => {
            let err_text = response
                .into_string()
                .unwrap_or_else(|_| "Unknown upstream error".to_string());
            let body = serde_json::from_str::<Value>(&err_text)
                .unwrap_or_else(|_| json!({ "error": err_text }));
            return Err(GatewayError { status: code, body });
        }
        Err(e) => {
            return Err(GatewayError::new(
                502,
                format!(
                    "Failed to connect to upstream at {}: {}",
                    upstream_target, e
                ),
            ));
        }
    };

    // 4. Sanitize and Extract Tool Calls if emitted as raw text
    let sanitized_res = sanitize_tool_calling_response(res_val);

    let completion_tokens = sanitized_res["usage"]["completion_tokens"]
        .as_u64()
        .unwrap_or(0) as usize;

    Ok((
        sanitized_res,
        raw_prompt_tokens,
        gated_prompt_tokens,
        completion_tokens,
        model_name,
    ))
}

/// Injects explicit JSON formatting instructions when running against local LLMs
fn inject_tool_calling_instructions(payload: &mut Value) {
    if let Some(tools) = payload.get("tools").cloned() {
        let tool_instruction = format!(
            "\n[SYNAPSE TOOL PROTOCOL]\nYou have access to the following tools: {}\nIf invoking a tool, respond ONLY with a JSON object in this exact format:\n{{\"name\": \"<function_name>\", \"arguments\": {{...}}}}",
            tools
        );

        if let Some(messages) = payload["messages"].as_array_mut() {
            if let Some(first_msg) = messages.first_mut() {
                if first_msg["role"] == "system" {
                    if let Some(content) = first_msg["content"].as_str() {
                        first_msg["content"] = json!(format!("{}\n{}", content, tool_instruction));
                        return;
                    }
                }
            }
            // Insert system message if not present
            messages.insert(
                0,
                json!({
                    "role": "system",
                    "content": tool_instruction
                }),
            );
        }
    }
}

/// Normalizes raw model output strings into valid OpenAI `tool_calls`
pub fn sanitize_tool_calling_response(mut res: Value) -> Value {
    if let Some(choices) = res["choices"].as_array_mut() {
        for choice in choices.iter_mut() {
            if let Some(message) = choice.get_mut("message") {
                // If message already has tool_calls, ensure proper structure
                if message.get("tool_calls").is_some() {
                    continue;
                }

                // Check if message content contains an embedded JSON tool call
                if let Some(content) = message["content"].as_str() {
                    if let Some(parsed_tool) = extract_json_tool_call(content) {
                        let tool_id = format!(
                            "call_{}",
                            hex::encode(&sha2::Sha256::digest(content.as_bytes())[..4])
                        );
                        message["tool_calls"] = json!([
                            {
                                "id": tool_id,
                                "type": "function",
                                "function": {
                                    "name": parsed_tool["name"].as_str().unwrap_or(""),
                                    "arguments": parsed_tool["arguments"].to_string()
                                }
                            }
                        ]);
                    }
                }
            }
        }
    }
    res
}

/// Extracts a JSON tool call object from text output
fn extract_json_tool_call(text: &str) -> Option<Value> {
    let trimmed = text.trim();
    // Direct JSON object
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        if let Ok(val) = serde_json::from_str::<Value>(trimmed) {
            if val.get("name").is_some() && val.get("arguments").is_some() {
                return Some(val);
            }
        }
    }

    // Markdown fenced JSON
    if let Some(start) = text.find("```json") {
        let rest = &text[start + 7..];
        if let Some(end) = rest.find("```") {
            let json_str = rest[..end].trim();
            if let Ok(val) = serde_json::from_str::<Value>(json_str) {
                if val.get("name").is_some() && val.get("arguments").is_some() {
                    return Some(val);
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_policy_from_env_values() {
        assert_eq!(TokenPolicy::from_values(None, None), TokenPolicy::Generate);
        assert_eq!(
            TokenPolicy::from_values(None, Some(" fixed ")),
            TokenPolicy::Fixed("fixed".into())
        );
        assert_eq!(
            TokenPolicy::from_values(None, Some("")),
            TokenPolicy::Generate
        );
        assert_eq!(
            TokenPolicy::from_values(Some("1"), Some("fixed")),
            TokenPolicy::Disabled
        );
        assert_eq!(
            TokenPolicy::from_values(Some("0"), None),
            TokenPolicy::Generate
        );
    }

    #[test]
    fn loopback_binds_get_a_host_allowlist() {
        let hosts = loopback_hosts("127.0.0.1:9000".parse().unwrap()).unwrap();
        assert!(hosts.contains(&"localhost:9000".to_string()));
        assert!(hosts.contains(&"[::1]:9000".to_string()));
        assert!(loopback_hosts("0.0.0.0:9000".parse().unwrap()).is_none());
    }
}
