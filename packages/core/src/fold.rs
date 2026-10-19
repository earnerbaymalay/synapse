//! Reversible context folding for the gateway.
//!
//! Inspired by billion-context's "fold consumed conversation, restore on
//! demand", but deterministic and model-free: large tool results that have
//! scrolled out of the recent window are replaced by a short digest stub, and
//! the untouched original is kept in the spool under a content-addressed id.
//! The agent restores any stub with `synapse spool show <id>` (it already has
//! a shell), so no tool injection or response rewriting is needed.
//!
//! Folding is a pure function of message content, so a folded message looks
//! byte-identical on every later request and prefix caching keeps working.

use std::fs;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::compression::{compress_text, estimate_tokens, CompressionLevel, SpoolManager};

/// Marker that makes folding idempotent.
pub const FOLD_MARKER: &str = "[SYNAPSE folded";
/// Results smaller than this are never worth a stub.
pub const MIN_FOLD_TOKENS: usize = 400;
const DIGEST_MAX_CHARS: usize = 600;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FoldReport {
    pub folded: usize,
    pub tokens_before: usize,
    pub tokens_after: usize,
}

impl FoldReport {
    pub fn saved(&self) -> usize {
        self.tokens_before.saturating_sub(self.tokens_after)
    }
}

/// Folds tool results in `messages` except for the newest `keep_recent`
/// messages. Handles OpenAI (`role: "tool"`) and Anthropic
/// (`tool_result` content blocks) shapes. Originals go to `spool`; if the
/// spool cannot be written the result is left unfolded (never lossy).
pub fn fold_messages(
    messages: &mut [Value],
    keep_recent: usize,
    spool: &SpoolManager,
) -> FoldReport {
    let mut report = FoldReport::default();
    let cutoff = messages.len().saturating_sub(keep_recent);
    for msg in messages.iter_mut().take(cutoff) {
        if msg["role"] == "tool" {
            if let Some(text) = msg["content"].as_str().map(String::from) {
                if let Some(stub) = fold_text(&text, spool, &mut report) {
                    msg["content"] = json!(stub);
                }
            }
        }
        if let Some(blocks) = msg["content"].as_array_mut() {
            for block in blocks.iter_mut().filter(|b| b["type"] == "tool_result") {
                fold_block(block, spool, &mut report);
            }
        }
    }
    report
}

fn fold_block(block: &mut Value, spool: &SpoolManager, report: &mut FoldReport) {
    match &block["content"] {
        Value::String(s) => {
            if let Some(stub) = fold_text(&s.clone(), spool, report) {
                block["content"] = json!(stub);
            }
        }
        // Only fold the all-text form; anything with images stays intact.
        Value::Array(parts) if parts.iter().all(|p| p["type"] == "text") => {
            let joined: String = parts
                .iter()
                .filter_map(|p| p["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n");
            if let Some(stub) = fold_text(&joined, spool, report) {
                block["content"] = json!([{ "type": "text", "text": stub }]);
            }
        }
        _ => {}
    }
}

/// Returns the stub for `text`, or `None` if it should stay as-is.
fn fold_text(text: &str, spool: &SpoolManager, report: &mut FoldReport) -> Option<String> {
    if text.starts_with(FOLD_MARKER) {
        return None;
    }
    let tokens = estimate_tokens(text);
    if tokens < MIN_FOLD_TOKENS {
        return None;
    }
    let id = content_id(text);
    spool.ensure_dir().ok()?;
    let path = spool_path(spool, &id);
    if !path.exists() {
        fs::write(&path, text).ok()?;
    }
    let (digest, ..) = compress_text(text, CompressionLevel::Aggressive);
    let stub = format!(
        "{FOLD_MARKER} tool output · ~{tokens} tokens · restore: synapse spool show {id} (supports --grep, --tail)]\n{}",
        clip(&digest, DIGEST_MAX_CHARS)
    );
    let stub_tokens = estimate_tokens(&stub);
    if stub_tokens * 2 > tokens {
        return None;
    }
    report.folded += 1;
    report.tokens_before += tokens;
    report.tokens_after += stub_tokens;
    Some(stub)
}

fn spool_path(spool: &SpoolManager, id: &str) -> std::path::PathBuf {
    spool.dir().join(format!("{id}.log"))
}

/// 12 hex chars of SHA-256 — stable across requests and restarts.
fn content_id(text: &str) -> String {
    let digest = Sha256::digest(text.as_bytes());
    digest.iter().take(6).map(|b| format!("{b:02x}")).collect()
}

fn clip(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut t: String = s.chars().take(max).collect();
    t.push_str("\n[… digest truncated]");
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn big(tag: &str) -> String {
        (0..300)
            .map(|i| format!("{tag} noisy line number {i} with padding padding padding\n"))
            .collect::<String>()
            + "error: boom at the end"
    }

    fn spool() -> (tempfile::TempDir, SpoolManager) {
        let d = tempfile::tempdir().unwrap();
        let s = SpoolManager::new(d.path().to_path_buf());
        (d, s)
    }

    #[test]
    fn folds_old_openai_tool_results_and_restores_losslessly() {
        let (_d, sp) = spool();
        let raw = big("a");
        let mut msgs = vec![
            json!({"role":"tool","content": raw}),
            json!({"role":"user","content":"next"}),
            json!({"role":"assistant","content":"ok"}),
        ];
        let r = fold_messages(&mut msgs, 2, &sp);
        assert_eq!(r.folded, 1);
        assert!(r.saved() > 0);
        let stub = msgs[0]["content"].as_str().unwrap();
        assert!(stub.starts_with(FOLD_MARKER) && stub.contains("error: boom"));
        let id = stub
            .split("synapse spool show ")
            .nth(1)
            .unwrap()
            .split(' ')
            .next()
            .unwrap();
        assert_eq!(sp.read_log(id, None, None).unwrap(), raw);
    }

    #[test]
    fn recent_and_small_results_are_untouched() {
        let (_d, sp) = spool();
        let mut msgs = vec![
            json!({"role":"tool","content":"tiny"}),
            json!({"role":"tool","content": big("b")}),
        ];
        let r = fold_messages(&mut msgs, 1, &sp);
        assert_eq!(r, FoldReport::default());
        assert_eq!(msgs[0]["content"], "tiny");
    }

    #[test]
    fn folds_anthropic_tool_result_blocks_and_keeps_other_blocks() {
        let (_d, sp) = spool();
        let mut msgs = vec![
            json!({"role":"user","content":[
                {"type":"tool_result","tool_use_id":"t1","content": big("c")},
                {"type":"tool_result","tool_use_id":"t2","content":[{"type":"text","text": big("d")}]},
                {"type":"text","text":"hello"}
            ]}),
            json!({"role":"assistant","content":"done"}),
        ];
        let r = fold_messages(&mut msgs, 1, &sp);
        assert_eq!(r.folded, 2);
        let blocks = msgs[0]["content"].as_array().unwrap();
        assert!(blocks[0]["content"]
            .as_str()
            .unwrap()
            .starts_with(FOLD_MARKER));
        assert!(blocks[1]["content"][0]["text"]
            .as_str()
            .unwrap()
            .starts_with(FOLD_MARKER));
        assert_eq!(blocks[0]["tool_use_id"], "t1");
        assert_eq!(blocks[2]["text"], "hello");
    }

    #[test]
    fn folding_is_idempotent_and_deterministic() {
        let (_d, sp) = spool();
        let make = || {
            vec![
                json!({"role":"tool","content": big("e")}),
                json!({"role":"user","content":"x"}),
            ]
        };
        let mut a = make();
        let mut b = make();
        fold_messages(&mut a, 1, &sp);
        fold_messages(&mut b, 1, &sp);
        assert_eq!(
            a, b,
            "same input must fold to identical bytes (prefix-cache safe)"
        );
        let again = fold_messages(&mut a, 1, &sp);
        assert_eq!(again, FoldReport::default());
    }
}
