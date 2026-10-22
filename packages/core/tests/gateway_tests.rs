use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::json;
use synapse_core::compression::CompressionLevel;
use synapse_core::gateway::{
    sanitize_tool_calling_response, GatewayConfig, GatewayServer, TokenPolicy, UpstreamKind,
    MAX_BODY_BYTES,
};
use synapse_core::hud::{format_hud, HudSnapshot};

#[test]
fn test_upstream_kind_detection() {
    assert_eq!(
        UpstreamKind::from_url_or_name("http://127.0.0.1:11434"),
        UpstreamKind::Ollama
    );
    assert_eq!(
        UpstreamKind::from_url_or_name("http://localhost:8080"),
        UpstreamKind::LlamaCpp
    );
    assert_eq!(
        UpstreamKind::from_url_or_name("https://api.anthropic.com"),
        UpstreamKind::Anthropic
    );
    assert_eq!(
        UpstreamKind::from_url_or_name("https://openrouter.ai/api/v1"),
        UpstreamKind::OpenAi
    );
}

#[test]
fn test_tool_call_sanitizer_from_markdown_fenced_json() {
    let raw_response = json!({
        "id": "chatcmpl-test",
        "choices": [
            {
                "message": {
                    "role": "assistant",
                    "content": "I will execute the tool now:\n```json\n{\n  \"name\": \"read_file\",\n  \"arguments\": {\"path\": \"src/main.rs\"}\n}\n```"
                }
            }
        ]
    });

    let sanitized = sanitize_tool_calling_response(raw_response);
    let message = &sanitized["choices"][0]["message"];

    assert!(message.get("tool_calls").is_some());
    let tool_calls = message["tool_calls"].as_array().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0]["function"]["name"], "read_file");
    assert!(tool_calls[0]["function"]["arguments"]
        .as_str()
        .unwrap()
        .contains("src/main.rs"));
}

#[test]
fn test_tool_call_sanitizer_from_raw_json() {
    let raw_response = json!({
        "id": "chatcmpl-test",
        "choices": [
            {
                "message": {
                    "role": "assistant",
                    "content": "{\"name\": \"run_command\", \"arguments\": {\"command\": \"cargo test\"}}"
                }
            }
        ]
    });

    let sanitized = sanitize_tool_calling_response(raw_response);
    let message = &sanitized["choices"][0]["message"];

    assert!(message.get("tool_calls").is_some());
    let tool_calls = message["tool_calls"].as_array().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0]["function"]["name"], "run_command");
}

/// Starts a gateway on an ephemeral loopback port with the given token policy.
fn start_gateway(config: GatewayConfig, policy: TokenPolicy) -> (GatewayServer, String) {
    let mut server = GatewayServer::new(config).with_token_policy(policy);
    server.start_background().expect("Failed to start gateway");
    let base = format!("http://{}", server.local_addr().unwrap());
    (server, base)
}

fn local_config(upstream_url: &str, kind: UpstreamKind) -> GatewayConfig {
    GatewayConfig {
        bind_addr: "127.0.0.1:0".to_string(),
        upstream_url: upstream_url.to_string(),
        upstream_kind: kind,
        default_model: Some("qwen2.5-coder:7b".to_string()),
        gate_level: CompressionLevel::Balanced,
        api_key: None,
        fold_keep_recent: None,
    }
}

/// Sends a raw HTTP/1.1 request (full control over Host and
/// Content-Length) and returns `(status, raw response)`.
fn raw_http(addr: std::net::SocketAddr, request: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut out = Vec::new();
    let _ = stream.read_to_end(&mut out);
    let out = String::from_utf8_lossy(&out).to_string();
    let status = out
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    (status, out)
}

type Seen = Arc<Mutex<Vec<(Vec<(String, String)>, String)>>>;

/// A mock upstream: records each request's headers and body and answers
/// with `status` + `body`.
fn mock_upstream(status: u16, body: &'static str) -> (String, Seen) {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr().to_ip().unwrap());
    let seen: Seen = Arc::new(Mutex::new(Vec::new()));
    let seen_bg = Arc::clone(&seen);
    std::thread::spawn(move || {
        for mut req in server.incoming_requests() {
            let headers = req
                .headers()
                .iter()
                .map(|h| (h.field.to_string().to_lowercase(), h.value.to_string()))
                .collect();
            let mut b = String::new();
            let _ = req.as_reader().read_to_string(&mut b);
            seen_bg.lock().unwrap().push((headers, b));
            let ctype =
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                    .unwrap();
            let _ = req.respond(
                tiny_http::Response::from_string(body)
                    .with_status_code(status)
                    .with_header(ctype),
            );
        }
    });
    (url, seen)
}

const OK_COMPLETION: &str = r#"{"id":"x","choices":[{"index":0,"message":{"role":"assistant","content":"ok"}}],"usage":{"completion_tokens":1}}"#;

#[test]
fn test_gateway_server_lifecycle_and_endpoints() {
    let (mut server, base) = start_gateway(
        local_config("http://127.0.0.1:11434", UpstreamKind::Ollama),
        TokenPolicy::Fixed("t0ken".into()),
    );
    assert!(server.is_running());
    assert_eq!(server.token(), Some("t0ken"));

    // /health is open, has no CORS header, and does not disclose the upstream.
    let res = ureq::get(&format!("{base}/health")).call().unwrap();
    assert_eq!(res.status(), 200);
    assert!(res.header("Access-Control-Allow-Origin").is_none());
    let health_body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(health_body["status"], "healthy");
    assert_eq!(health_body["service"], "SYNAPSE Runtime Gateway");
    assert!(health_body.get("upstream").is_none());
    assert!(!health_body.to_string().contains("11434"));

    // /v1/* requires the token, via Bearer or x-api-key.
    match ureq::get(&format!("{base}/v1/models")).call() {
        Err(ureq::Error::Status(401, _)) => {}
        other => panic!("expected 401 without token, got {other:?}"),
    }
    match ureq::get(&format!("{base}/v1/models"))
        .set("Authorization", "Bearer wrong")
        .call()
    {
        Err(ureq::Error::Status(401, _)) => {}
        other => panic!("expected 401 with a wrong token, got {other:?}"),
    }
    let models_res = ureq::get(&format!("{base}/v1/models"))
        .set("Authorization", "Bearer t0ken")
        .call()
        .unwrap();
    assert_eq!(models_res.status(), 200);
    let models_body: serde_json::Value = models_res.into_json().unwrap();
    assert_eq!(models_body["data"][0]["id"], "qwen2.5-coder:7b");
    let via_key = ureq::get(&format!("{base}/v1/models"))
        .set("x-api-key", "t0ken")
        .call()
        .unwrap();
    assert_eq!(via_key.status(), 200);

    // No CORS preflight support.
    let addr = server.local_addr().unwrap();
    let (_, preflight) = raw_http(
        addr,
        &format!(
            "OPTIONS /v1/chat/completions HTTP/1.1\r\nHost: {addr}\r\nOrigin: https://evil.example\r\nConnection: close\r\n\r\n"
        ),
    );
    assert!(!preflight
        .to_ascii_lowercase()
        .contains("access-control-allow-origin"));

    server.stop();
    assert!(!server.is_running());
}

#[test]
fn test_gateway_generates_a_random_token_by_default() {
    let (mut a, _) = start_gateway(
        local_config("http://127.0.0.1:1", UpstreamKind::Generic),
        TokenPolicy::Generate,
    );
    let (mut b, _) = start_gateway(
        local_config("http://127.0.0.1:1", UpstreamKind::Generic),
        TokenPolicy::Generate,
    );
    let ta = a.token().unwrap().to_string();
    let tb = b.token().unwrap().to_string();
    assert_eq!(ta.len(), 64);
    assert!(ta.chars().all(|c| c.is_ascii_hexdigit()));
    assert_ne!(ta, tb);
    a.stop();
    b.stop();
}

#[test]
fn test_gateway_token_can_be_disabled() {
    let (mut server, base) = start_gateway(
        local_config("http://127.0.0.1:1", UpstreamKind::Generic),
        TokenPolicy::Disabled,
    );
    assert_eq!(server.token(), None);
    let res = ureq::get(&format!("{base}/v1/models")).call().unwrap();
    assert_eq!(res.status(), 200);
    server.stop();
}

#[test]
fn test_gateway_rejects_foreign_host_headers() {
    let (mut server, _) = start_gateway(
        local_config("http://127.0.0.1:1", UpstreamKind::Generic),
        TokenPolicy::Disabled,
    );
    let addr = server.local_addr().unwrap();
    let get = |host: &str| {
        raw_http(
            addr,
            &format!("GET /health HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"),
        )
        .0
    };
    assert_eq!(get(&format!("evil.example:{}", addr.port())), 403);
    assert_eq!(get("127.0.0.1"), 403, "port must match");
    assert_eq!(get(&format!("localhost:{}", addr.port())), 200);
    assert_eq!(get(&format!("127.0.0.1:{}", addr.port())), 200);
    server.stop();
}

#[test]
fn test_gateway_rejects_oversized_bodies() {
    let (mut server, _) = start_gateway(
        local_config("http://127.0.0.1:1", UpstreamKind::Generic),
        TokenPolicy::Disabled,
    );
    let addr = server.local_addr().unwrap();
    let (status, _) = raw_http(
        addr,
        &format!(
            "POST /v1/chat/completions HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{{}}",
            MAX_BODY_BYTES + 1
        ),
    );
    assert_eq!(status, 413);
    server.stop();
}

#[test]
fn test_gateway_preserves_upstream_status_codes() {
    let (up, _) = mock_upstream(429, r#"{"error":{"type":"rate_limit"}}"#);
    let (mut server, base) = start_gateway(
        local_config(&up, UpstreamKind::Generic),
        TokenPolicy::Disabled,
    );
    match ureq::post(&format!("{base}/v1/chat/completions"))
        .send_json(json!({"model": "m", "messages": [{"role": "user", "content": "hi"}]}))
    {
        Err(ureq::Error::Status(429, res)) => {
            let body: serde_json::Value = res.into_json().unwrap();
            assert_eq!(body["error"]["type"], "rate_limit");
        }
        other => panic!("expected upstream 429 to pass through, got {other:?}"),
    }
    server.stop();
}

#[test]
fn test_gateway_refuses_streaming_requests_clearly() {
    let (up, seen) = mock_upstream(200, OK_COMPLETION);
    let (mut server, base) = start_gateway(
        local_config(&up, UpstreamKind::Generic),
        TokenPolicy::Disabled,
    );
    match ureq::post(&format!("{base}/v1/chat/completions")).send_json(
        json!({"model": "m", "stream": true, "messages": [{"role": "user", "content": "hi"}]}),
    ) {
        Err(ureq::Error::Status(400, res)) => {
            assert!(res
                .into_string()
                .unwrap()
                .contains("streaming not supported"));
        }
        other => panic!("expected 400 for stream:true, got {other:?}"),
    }
    assert!(
        seen.lock().unwrap().is_empty(),
        "nothing may reach upstream"
    );
    server.stop();
}

#[test]
fn test_gateway_sends_anthropic_auth_and_never_forwards_the_client_token() {
    let (up, seen) = mock_upstream(200, OK_COMPLETION);
    let mut config = local_config(&format!("{up}/anthropic"), UpstreamKind::Anthropic);
    config.api_key = Some("upstream-secret".into());
    let (mut server, base) = start_gateway(config, TokenPolicy::Fixed("client-token".into()));
    let res = ureq::post(&format!("{base}/v1/messages"))
        .set("x-api-key", "client-token")
        .send_json(json!({"model": "m", "messages": [{"role": "user", "content": "hi"}]}))
        .unwrap();
    assert_eq!(res.status(), 200);

    let seen = seen.lock().unwrap();
    let headers = &seen[0].0;
    let get = |k: &str| {
        headers
            .iter()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.as_str())
    };
    assert_eq!(get("x-api-key"), Some("upstream-secret"));
    assert_eq!(get("anthropic-version"), Some("2023-06-01"));
    assert_eq!(get("authorization"), None);
    assert!(!headers.iter().any(|(_, v)| v.contains("client-token")));
    server.stop();
}

#[test]
fn test_gateway_serves_concurrent_requests_on_its_pool() {
    let (mut server, base) = start_gateway(
        local_config("http://127.0.0.1:1", UpstreamKind::Generic),
        TokenPolicy::Disabled,
    );
    let handles: Vec<_> = (0..40)
        .map(|_| {
            let url = format!("{base}/health");
            std::thread::spawn(move || ureq::get(&url).call().map(|r| r.status()).unwrap_or(0))
        })
        .collect();
    for h in handles {
        assert_eq!(h.join().unwrap(), 200);
    }
    server.stop();
}

#[test]
fn test_hud_formatting() {
    let snap = HudSnapshot {
        gateway_active: true,
        gateway_bind: "127.0.0.1:8088".to_string(),
        upstream_url: "http://127.0.0.1:11434 (Ollama)".to_string(),
        stats: synapse_core::gateway::GatewayStats {
            total_requests: 42,
            prompt_tokens_raw: 100000,
            prompt_tokens_gated: 5000,
            completion_tokens: 1200,
            tokens_saved: 95000,
            avg_reduction_percent: 95,
            last_model: "qwen2.5-coder:7b".to_string(),
            last_latency_ms: 120,
        },
        brain_path: "~/Desktop/AI BRAIN".to_string(),
        total_adapters: 15,
        doctor_healthy: true,
        doctor_criticals: 0,
        doctor_warnings: 0,
    };

    let formatted = format_hud(&snap, false);
    assert!(formatted.contains("SYNAPSE VITAL SIGNS HUD"));
    assert!(formatted.contains("ONLINE"));
    assert!(formatted.contains("95% reduction"));
    assert!(formatted.contains("15 verified tools"));
    assert!(formatted.contains("LOCKSTEP SYNCED"));
}

#[test]
fn test_gating_preserves_tool_message_fields() {
    let noisy: String = (0..400)
        .map(|i| format!("   Compiling crate{i} v0.1.0\n"))
        .collect();
    let body = json!({
        "model": "m",
        "messages": [
            { "role": "assistant", "content": null,
              "tool_calls": [{ "id": "c1", "type": "function",
                               "function": { "name": "sh", "arguments": "{}" } }] },
            { "role": "tool", "tool_call_id": "c1", "name": "sh", "content": noisy }
        ]
    });
    let mut payload = body.clone();
    let messages = payload["messages"].as_array_mut().unwrap();
    synapse_core::gateway::gate_messages(messages, CompressionLevel::Balanced);
    assert_eq!(messages[1]["tool_call_id"], "c1");
    assert_eq!(messages[1]["name"], "sh");
    assert_eq!(messages[1]["role"], "tool");
    assert!(messages[1]["content"].as_str().unwrap().len() < noisy.len());
    assert_eq!(messages[0]["tool_calls"], body["messages"][0]["tool_calls"]);
}

#[test]
fn test_gate_level_off_is_passthrough() {
    assert_eq!(CompressionLevel::parse("off"), Some(CompressionLevel::Off));
    let noisy: String = (0..400)
        .map(|i| format!("   Compiling crate{i} v0.1.0\n"))
        .collect();
    let (out, raw, comp, pct) =
        synapse_core::compression::compress_text(&noisy, CompressionLevel::Off);
    assert_eq!(out, noisy);
    assert_eq!(raw, comp);
    assert_eq!(pct, 0);
}
