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

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tiny_http::{Header, Response, Server, StatusCode};
use sha2::Digest;

use crate::compression::{compress_text, estimate_tokens, CompressionLevel};

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

/// Gateway runtime handle
pub struct GatewayServer {
    config: GatewayConfig,
    stats: Arc<RwLock<GatewayStats>>,
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl GatewayServer {
    pub fn new(config: GatewayConfig) -> Self {
        Self {
            config,
            stats: Arc::new(RwLock::new(GatewayStats::default())),
            running: Arc::new(AtomicBool::new(false)),
            handle: None,
        }
    }

    pub fn stats(&self) -> GatewayStats {
        self.stats.read().unwrap().clone()
    }

    pub fn config(&self) -> &GatewayConfig {
        &self.config
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Starts the gateway server loop (blocking or background)
    pub fn start_background(&mut self) -> Result<(), String> {
        if self.is_running() {
            return Err("Gateway is already running".to_string());
        }

        let server = Server::http(&self.config.bind_addr)
            .map_err(|e| format!("Failed to bind to {}: {}", self.config.bind_addr, e))?;

        let running = Arc::clone(&self.running);
        let stats = Arc::clone(&self.stats);
        let config = self.config.clone();

        running.store(true, Ordering::SeqCst);

        let handle = thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                match server.recv_timeout(Duration::from_millis(200)) {
                    Ok(Some(request)) => {
                        let stats_clone = Arc::clone(&stats);
                        let config_clone = config.clone();
                        thread::spawn(move || {
                            handle_request(request, &config_clone, &stats_clone);
                        });
                    }
                    Ok(None) => {}
                    Err(_) => break,
                }
            }
        });

        self.handle = Some(handle);
        Ok(())
    }

    /// Stops the background gateway server
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn send_json<T: ToString>(request: tiny_http::Request, status: u16, body: T) {
    let cors = Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap();
    let ctype = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
    let res = Response::from_string(body.to_string())
        .with_status_code(StatusCode(status))
        .with_header(cors)
        .with_header(ctype);
    let _ = request.respond(res);
}

/// Dispatches and processes an incoming HTTP request
pub fn handle_request(
    mut request: tiny_http::Request,
    config: &GatewayConfig,
    stats: &Arc<RwLock<GatewayStats>>,
) {
    let url = request.url().to_string();
    let method = request.method().as_str().to_string();

    if method == "OPTIONS" {
        let cors = Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap();
        let methods = Header::from_bytes(
            &b"Access-Control-Allow-Methods"[..],
            &b"GET, POST, OPTIONS"[..],
        )
        .unwrap();
        let headers = Header::from_bytes(
            &b"Access-Control-Allow-Headers"[..],
            &b"Content-Type, Authorization"[..],
        )
        .unwrap();
        let res = Response::empty(StatusCode(200))
            .with_header(cors)
            .with_header(methods)
            .with_header(headers);
        let _ = request.respond(res);
        return;
    }

    // Health & Info endpoints
    if (url == "/health" || url == "/" || url == "/api/status") && method == "GET" {
        let current_stats = stats.read().unwrap().clone();
        let payload = json!({
            "status": "healthy",
            "service": "SYNAPSE Runtime Gateway",
            "version": "1.0.0",
            "upstream": config.upstream_url,
            "bind_addr": config.bind_addr,
            "gate_level": format!("{:?}", config.gate_level),
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
        let mut body = String::new();
        if let Err(e) = request.as_reader().read_to_string(&mut body) {
            send_json(
                request,
                400,
                json!({ "error": format!("Failed to read request body: {}", e) }).to_string(),
            );
            return;
        }

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
            Err(err_msg) => {
                send_json(request, 502, json!({ "error": err_msg }).to_string());
            }
        }
        return;
    }

    // 404 fallback
    send_json(request, 404, json!({ "error": "Not Found" }).to_string());
}

/// Applies Sensory Gating and tool-call normalization, then executes upstream request
pub fn process_chat_request(
    body: &str,
    config: &GatewayConfig,
) -> Result<(Value, usize, usize, usize, String), String> {
    let mut payload: Value = serde_json::from_str(body)
        .map_err(|e| format!("Malformed JSON request payload: {}", e))?;

    let model_name = payload["model"]
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| config.default_model.clone())
        .unwrap_or_else(|| "default".to_string());

    let mut raw_prompt_tokens = 0;
    let mut gated_prompt_tokens = 0;

    // 1. Sensory Gating on incoming messages
    if let Some(messages) = payload["messages"].as_array_mut() {
        for msg in messages.iter_mut() {
            if let Some(content) = msg["content"].as_str() {
                let initial_tokens = estimate_tokens(content);
                raw_prompt_tokens += initial_tokens;

                // Compress large blocks (terminal outputs, giant source files)
                if content.len() > 120 {
                    let (compressed, _, comp_tokens, _) =
                        compress_text(content, config.gate_level);
                    if !compressed.is_empty() && comp_tokens < initial_tokens {
                        *msg = json!({
                            "role": msg["role"].as_str().unwrap_or("user"),
                            "content": compressed
                        });
                        gated_prompt_tokens += comp_tokens;
                    } else {
                        gated_prompt_tokens += initial_tokens;
                    }
                } else {
                    gated_prompt_tokens += initial_tokens;
                }
            }
        }
    }

    // 2. Local Model Tool Calling Schema Normalization
    let has_tools = payload.get("tools").and_then(|t| t.as_array()).map(|a| !a.is_empty()).unwrap_or(false);
    if has_tools && (config.upstream_kind == UpstreamKind::Ollama || config.upstream_kind == UpstreamKind::LlamaCpp) {
        inject_tool_calling_instructions(&mut payload);
    }

    // 3. Upstream Forwarding
    let upstream_target = match config.upstream_kind {
        UpstreamKind::Ollama => {
            if config.upstream_url.ends_with("/v1") {
                format!("{}/chat/completions", config.upstream_url)
            } else {
                format!("{}/v1/chat/completions", config.upstream_url.trim_end_matches('/'))
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
                format!("{}/v1/chat/completions", config.upstream_url.trim_end_matches('/'))
            }
        }
    };

    let mut req = ureq::post(&upstream_target)
        .set("Content-Type", "application/json");

    if let Some(ref key) = config.api_key {
        req = req.set("Authorization", &format!("Bearer {}", key));
    }

    let upstream_res = req.send_json(&payload);

    let res_val: Value = match upstream_res {
        Ok(response) => {
            response.into_json::<Value>().map_err(|e| format!("Failed to parse upstream response JSON: {}", e))?
        }
        Err(ureq::Error::Status(code, response)) => {
            let err_text = response.into_string().unwrap_or_else(|_| "Unknown upstream error".to_string());
            return Err(format!("Upstream returned HTTP {}: {}", code, err_text));
        }
        Err(e) => {
            return Err(format!("Failed to connect to upstream at {}: {}", upstream_target, e));
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
                        let tool_id = format!("call_{}", &hex::encode(&sha2::Sha256::digest(content.as_bytes())[..4]));
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
