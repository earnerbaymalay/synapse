//! Context Firewall: Outbound egress pseudonymization and inbound rehydration.
//!
//! Protects against leakage of physical hardware MAC addresses, RFC 1918 private IPs,
//! and secret credentials/API keys when sending prompts upstream to local or remote LLMs.
//! Rehydrates synthetic tokens back to real values when responses/tool calls return.

use regex::Regex;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaskCategory {
    Mac,
    Ip,
    Secret,
    Path,
}

#[derive(Debug, Clone, Default)]
pub struct FirewallSession {
    /// Mapping: real sensitive string -> synthetic token (e.g. "b4:69:21:fa:71:d4" -> "[SYNAPSE_MAC_1]")
    outbound: BTreeMap<String, String>,
    /// Mapping: synthetic token -> real sensitive string (e.g. "[SYNAPSE_MAC_1]" -> "b4:69:21:fa:71:d4")
    inbound: BTreeMap<String, String>,
    mac_counter: usize,
    ip_counter: usize,
    secret_counter: usize,
    path_counter: usize,
}

impl FirewallSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.outbound.is_empty()
    }

    pub fn len(&self) -> usize {
        self.outbound.len()
    }

    pub fn outbound_mappings(&self) -> &BTreeMap<String, String> {
        &self.outbound
    }

    pub fn inbound_mappings(&self) -> &BTreeMap<String, String> {
        &self.inbound
    }

    /// Retrieves an existing synthetic token for a real value or registers a new one.
    pub fn mask_token(&mut self, real: &str, category: MaskCategory) -> String {
        if let Some(existing) = self.outbound.get(real) {
            return existing.clone();
        }

        let placeholder = match category {
            MaskCategory::Mac => {
                self.mac_counter += 1;
                format!("[SYNAPSE_MAC_{}]", self.mac_counter)
            }
            MaskCategory::Ip => {
                self.ip_counter += 1;
                format!("[SYNAPSE_IP_{}]", self.ip_counter)
            }
            MaskCategory::Secret => {
                self.secret_counter += 1;
                format!("[SYNAPSE_SECRET_{}]", self.secret_counter)
            }
            MaskCategory::Path => {
                self.path_counter += 1;
                format!("[SYNAPSE_PATH_{}]", self.path_counter)
            }
        };

        self.outbound.insert(real.to_string(), placeholder.clone());
        self.inbound.insert(placeholder.clone(), real.to_string());
        placeholder
    }

    /// Replaces all synthetic tokens in `text` with their original real values.
    pub fn unmask_text(&self, text: &str) -> String {
        if self.inbound.is_empty() || !text.contains("[SYNAPSE_") {
            return text.to_string();
        }

        let mut result = text.to_string();
        for (placeholder, real) in &self.inbound {
            result = result.replace(placeholder, real);
        }
        result
    }
}

// Global compiled regexes for sensitive entity detection
static RE_MAC: OnceLock<Regex> = OnceLock::new();
static RE_IPV4_PRIVATE: OnceLock<Regex> = OnceLock::new();
static RE_IPV6_PRIVATE: OnceLock<Regex> = OnceLock::new();
static RE_SECRETS_PREFIXED: OnceLock<Regex> = OnceLock::new();
static RE_BEARER: OnceLock<Regex> = OnceLock::new();

fn get_re_mac() -> &'static Regex {
    RE_MAC.get_or_init(|| {
        Regex::new(r"(?i)\b(?:[0-9a-f]{2}[:-]){5}[0-9a-f]{2}\b").expect("Valid MAC regex")
    })
}

fn get_re_ipv4_private() -> &'static Regex {
    RE_IPV4_PRIVATE.get_or_init(|| {
        Regex::new(r"\b(?:10\.\d{1,3}\.\d{1,3}\.\d{1,3}|192\.168\.\d{1,3}\.\d{1,3}|172\.(?:1[6-9]|2\d|3[01])\.\d{1,3}\.\d{1,3})\b")
            .expect("Valid IPv4 regex")
    })
}

fn get_re_ipv6_private() -> &'static Regex {
    RE_IPV6_PRIVATE.get_or_init(|| {
        Regex::new(r"(?i)\b(?:fe80:[0-9a-f:]+|fd[0-9a-f]{2}:[0-9a-f:]+)\b")
            .expect("Valid IPv6 private regex")
    })
}

fn get_re_secrets_prefixed() -> &'static Regex {
    RE_SECRETS_PREFIXED.get_or_init(|| {
        Regex::new(r"\b(?:sk-[a-zA-Z0-9_\-]{20,}|ghp_[a-zA-Z0-9]{36}|gho_[a-zA-Z0-9]{36}|github_pat_[a-zA-Z0-9_]{22,}|AKIA[0-9A-Z]{16}|xoxb-[a-zA-Z0-9_\-]{10,}|xoxp-[a-zA-Z0-9_\-]{10,}|AIza[0-9A-Za-z\-_]{35})\b")
            .expect("Valid secrets regex")
    })
}

fn get_re_bearer() -> &'static Regex {
    RE_BEARER.get_or_init(|| {
        Regex::new(r"(?i)\bBearer\s+([a-zA-Z0-9_\-\.]{24,})\b").expect("Valid Bearer regex")
    })
}

/// Masks sensitive tokens in a raw text string, tracking mappings in `session`.
pub fn mask_text(text: &str, session: &mut FirewallSession) -> String {
    if text.is_empty() {
        return String::new();
    }

    // Collect sensitive spans: (start, end, real_val, category)
    let mut spans: Vec<(usize, usize, String, MaskCategory)> = Vec::new();

    // 1. MAC addresses
    for m in get_re_mac().find_iter(text) {
        let val = m.as_str();
        // Ignore dummy/all-zeros/broadcast
        if val != "00:00:00:00:00:00"
            && val != "00-00-00-00-00-00"
            && val != "ff:ff:ff:ff:ff:ff"
            && val != "FF:FF:FF:FF:FF:FF"
        {
            spans.push((m.start(), m.end(), val.to_string(), MaskCategory::Mac));
        }
    }

    // 2. Private IPv4
    for m in get_re_ipv4_private().find_iter(text) {
        let val = m.as_str();
        // Validate octets are <= 255
        let valid_octets = val
            .split('.')
            .filter_map(|part| part.parse::<u16>().ok())
            .filter(|&num| num <= 255)
            .count();
        if valid_octets == 4 {
            spans.push((m.start(), m.end(), val.to_string(), MaskCategory::Ip));
        }
    }

    // 3. Private IPv6 (link-local and ULA)
    for m in get_re_ipv6_private().find_iter(text) {
        spans.push((
            m.start(),
            m.end(),
            m.as_str().to_string(),
            MaskCategory::Ip,
        ));
    }

    // 4. API keys and credentials
    for m in get_re_secrets_prefixed().find_iter(text) {
        spans.push((
            m.start(),
            m.end(),
            m.as_str().to_string(),
            MaskCategory::Secret,
        ));
    }

    // 5. Bearer tokens (mask the captured token portion)
    for cap in get_re_bearer().captures_iter(text) {
        if let Some(m) = cap.get(1) {
            spans.push((
                m.start(),
                m.end(),
                m.as_str().to_string(),
                MaskCategory::Secret,
            ));
        }
    }

    if spans.is_empty() {
        return text.to_string();
    }

    // Sort spans by start index, discard overlaps
    spans.sort_by_key(|s| s.0);
    let mut non_overlapping: Vec<(usize, usize, String, MaskCategory)> = Vec::new();
    let mut last_end = 0;
    for span in spans {
        if span.0 >= last_end {
            last_end = span.1;
            non_overlapping.push(span);
        }
    }

    // Build replacement string
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for (start, end, real, category) in non_overlapping {
        out.push_str(&text[cursor..start]);
        let placeholder = session.mask_token(&real, category);
        out.push_str(&placeholder);
        cursor = end;
    }
    if cursor < text.len() {
        out.push_str(&text[cursor..]);
    }

    out
}

/// Recursively traverses a JSON value to mask sensitive tokens in all strings.
fn walk_and_mask(val: &mut Value, session: &mut FirewallSession) {
    match val {
        Value::String(s) => {
            *s = mask_text(s, session);
        }
        Value::Array(arr) => {
            for item in arr {
                walk_and_mask(item, session);
            }
        }
        Value::Object(map) => {
            for (_, item) in map.iter_mut() {
                walk_and_mask(item, session);
            }
        }
        _ => {}
    }
}

/// Recursively traverses a JSON value to unmask placeholders back to real strings.
fn walk_and_unmask(val: &mut Value, session: &FirewallSession) {
    match val {
        Value::String(s) => {
            *s = session.unmask_text(s);
        }
        Value::Array(arr) => {
            for item in arr {
                walk_and_unmask(item, session);
            }
        }
        Value::Object(map) => {
            for (_, item) in map.iter_mut() {
                walk_and_unmask(item, session);
            }
        }
        _ => {}
    }
}

/// Masks sensitive outbound payload contents (chat messages, prompts, tool inputs) before upstream dispatch.
pub fn mask_chat_payload(payload: &mut Value, session: &mut FirewallSession) {
    walk_and_mask(payload, session);
}

/// Rehydrates synthetic tokens in upstream model completions (assistant text, tool calls) back to real values.
pub fn unmask_chat_response(res: &mut Value, session: &FirewallSession) {
    if !session.is_empty() {
        walk_and_unmask(res, session);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_firewall_mask_and_unmask_mac_and_ip() {
        let mut session = FirewallSession::new();
        let prompt = "Investigate device b4:69:21:fa:71:d4 at internal address 192.168.1.150 and 10.0.0.5.";
        let masked = mask_text(prompt, &mut session);

        assert!(!masked.contains("b4:69:21:fa:71:d4"));
        assert!(!masked.contains("192.168.1.150"));
        assert!(!masked.contains("10.0.0.5"));
        assert!(masked.contains("[SYNAPSE_MAC_1]"));
        assert!(masked.contains("[SYNAPSE_IP_1]"));
        assert!(masked.contains("[SYNAPSE_IP_2]"));

        let unmasked = session.unmask_text(&masked);
        assert_eq!(unmasked, prompt);
    }

    #[test]
    fn test_firewall_mask_secrets_and_bearer() {
        let mut session = FirewallSession::new();
        let prompt = "Use key sk-ant-api03-abcdef123456789012345678 and Authorization: Bearer ghp_123456789012345678901234567890123456";
        let masked = mask_text(prompt, &mut session);

        assert!(!masked.contains("sk-ant-api03-abcdef123456789012345678"));
        assert!(!masked.contains("ghp_123456789012345678901234567890123456"));
        assert!(masked.contains("[SYNAPSE_SECRET_1]"));
        assert!(masked.contains("[SYNAPSE_SECRET_2]"));

        let unmasked = session.unmask_text(&masked);
        assert_eq!(unmasked, prompt);
    }

    #[test]
    fn test_firewall_leaves_loopback_unmasked() {
        let mut session = FirewallSession::new();
        let prompt = "Connect to http://127.0.0.1:8088 and localhost:11434.";
        let masked = mask_text(prompt, &mut session);

        assert_eq!(masked, prompt);
        assert!(session.is_empty());
    }

    #[test]
    fn test_round_trip_chat_payload_and_tool_calls() {
        let mut session = FirewallSession::new();
        let mut req = json!({
            "model": "qwen2.5-coder",
            "messages": [
                {
                    "role": "user",
                    "content": "Ping host 192.168.0.42 with MAC b4:69:21:fa:71:d4 using secret sk-112233445566778899aabbccdd"
                }
            ]
        });

        mask_chat_payload(&mut req, &mut session);
        let user_content = req["messages"][0]["content"].as_str().unwrap();
        assert!(user_content.contains("[SYNAPSE_IP_1]"));
        assert!(user_content.contains("[SYNAPSE_MAC_1]"));
        assert!(user_content.contains("[SYNAPSE_SECRET_1]"));

        // Simulate upstream model output calling a tool with the masked tokens
        let mut model_response = json!({
            "choices": [
                {
                    "message": {
                        "role": "assistant",
                        "content": "Executing ping for [SYNAPSE_IP_1]",
                        "tool_calls": [
                            {
                                "id": "call_1",
                                "type": "function",
                                "function": {
                                    "name": "ping",
                                    "arguments": "{\"target\": \"[SYNAPSE_IP_1]\", \"mac\": \"[SYNAPSE_MAC_1]\"}"
                                }
                            }
                        ]
                    }
                }
            ]
        });

        unmask_chat_response(&mut model_response, &session);
        let assistant_content = model_response["choices"][0]["message"]["content"]
            .as_str()
            .unwrap();
        let tool_args = model_response["choices"][0]["message"]["tool_calls"][0]["function"]["arguments"]
            .as_str()
            .unwrap();

        assert_eq!(assistant_content, "Executing ping for 192.168.0.42");
        assert!(tool_args.contains("192.168.0.42"));
        assert!(tool_args.contains("b4:69:21:fa:71:d4"));
        assert!(!tool_args.contains("[SYNAPSE_"));
    }
}
