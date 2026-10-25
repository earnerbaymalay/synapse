use serde_json::json;
use synapse_core::compression::CompressionLevel;
use synapse_core::gateway::{
    sanitize_tool_calling_response, GatewayConfig, GatewayServer, UpstreamKind,
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

#[test]
fn test_gateway_server_lifecycle_and_endpoints() {
    // Pick an ephemeral port for testing
    let config = GatewayConfig {
        bind_addr: "127.0.0.1:18088".to_string(),
        upstream_url: "http://127.0.0.1:11434".to_string(),
        upstream_kind: UpstreamKind::Ollama,
        default_model: Some("qwen2.5-coder:7b".to_string()),
        gate_level: CompressionLevel::Balanced,
        api_key: None,
        fold_keep_recent: None,
    };

    let mut server = GatewayServer::new(config);
    assert!(!server.is_running());

    server.start_background().expect("Failed to start gateway");
    assert!(server.is_running());

    // Give the server a moment to bind
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Test GET /health
    let res = ureq::get("http://127.0.0.1:18088/health")
        .call()
        .expect("Health request failed");
    assert_eq!(res.status(), 200);
    let health_body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(health_body["status"], "healthy");
    assert_eq!(health_body["service"], "SYNAPSE Runtime Gateway");

    // Test GET /v1/models
    let models_res = ureq::get("http://127.0.0.1:18088/v1/models")
        .call()
        .expect("Models request failed");
    assert_eq!(models_res.status(), 200);
    let models_body: serde_json::Value = models_res.into_json().unwrap();
    assert_eq!(models_body["data"][0]["id"], "qwen2.5-coder:7b");

    // Test Host header rejection (DNS rebinding protection)
    let bad_host_res = ureq::get("http://127.0.0.1:18088/health")
        .set("Host", "attacker.com")
        .call();
    match bad_host_res {
        Ok(r) => assert_eq!(r.status(), 403),
        Err(ureq::Error::Status(code, _)) => assert_eq!(code, 403),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }

    // Test CORS loopback vs foreign origin
    let loopback_cors_res = ureq::get("http://127.0.0.1:18088/health")
        .set("Origin", "http://localhost:3000")
        .call()
        .expect("loopback origin request failed");
    assert_eq!(
        loopback_cors_res.header("Access-Control-Allow-Origin"),
        Some("http://localhost:3000")
    );

    let foreign_cors_res = ureq::get("http://127.0.0.1:18088/health")
        .set("Origin", "https://evil-site.com")
        .call()
        .expect("foreign origin request failed");
    assert_eq!(foreign_cors_res.header("Access-Control-Allow-Origin"), None);

    // Stop server
    server.stop();
    assert!(!server.is_running());
}

#[test]
fn test_gateway_bearer_auth() {
    let config = GatewayConfig {
        bind_addr: "127.0.0.1:18089".to_string(),
        upstream_url: "http://127.0.0.1:11434".to_string(),
        upstream_kind: UpstreamKind::Ollama,
        default_model: Some("qwen2.5-coder:7b".to_string()),
        gate_level: CompressionLevel::Balanced,
        api_key: Some("secret-token-123".to_string()),
        fold_keep_recent: None,
    };

    let mut server = GatewayServer::new(config);
    server.start_background().expect("Failed to start gateway");
    std::thread::sleep(std::time::Duration::from_millis(100));

    // /health is public
    let health_res = ureq::get("http://127.0.0.1:18089/health")
        .call()
        .expect("health check failed");
    assert_eq!(health_res.status(), 200);

    // /v1/models without token -> 401
    let unauth_res = ureq::get("http://127.0.0.1:18089/v1/models").call();
    match unauth_res {
        Ok(r) => assert_eq!(r.status(), 401),
        Err(ureq::Error::Status(code, _)) => assert_eq!(code, 401),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }

    // /v1/models with correct token -> 200
    let auth_res = ureq::get("http://127.0.0.1:18089/v1/models")
        .set("Authorization", "Bearer secret-token-123")
        .call()
        .expect("authorized request failed");
    assert_eq!(auth_res.status(), 200);

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
