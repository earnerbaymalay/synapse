//! Local semantic embeddings for Synaptic Memory.
//!
//! Memory stays local-first and zero-telemetry: embeddings are produced by a
//! model running on the user's own Ollama instance (default `nomic-embed-text`
//! at `127.0.0.1:11434`). Every entry point degrades to `None` on any failure —
//! Ollama down, model missing, malformed response — so memory never blocks the
//! agent and keyword search keeps working without a vector backend.
//!
//! * Endpoint — `$SYNAPSE_OLLAMA_URL` (default `http://127.0.0.1:11434`).
//! * Model    — `$SYNAPSE_EMBED_MODEL` (default `nomic-embed-text`).
//! * Off switch — set `$SYNAPSE_EMBED_MODEL=off` (or empty) to disable entirely.

use std::time::Duration;

use serde_json::json;

/// Default embedding model; `nomic-embed-text` is a 768-dim, 274 MB local model.
pub const DEFAULT_EMBED_MODEL: &str = "nomic-embed-text";
const DEFAULT_OLLAMA_URL: &str = "http://127.0.0.1:11434";

/// Resolved embedding model name, or `None` when embeddings are disabled.
pub fn model() -> Option<String> {
    match std::env::var("SYNAPSE_EMBED_MODEL") {
        Ok(m) if m.trim().is_empty() || m.trim().eq_ignore_ascii_case("off") => None,
        Ok(m) => Some(m),
        Err(_) => Some(DEFAULT_EMBED_MODEL.to_string()),
    }
}

/// True when a local embedding model is configured (not switched off).
pub fn enabled() -> bool {
    model().is_some()
}

fn ollama_url() -> String {
    std::env::var("SYNAPSE_OLLAMA_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_OLLAMA_URL.to_string())
}

/// Embed a stored memory fragment. nomic-embed-text is trained with task
/// prefixes; documents use `search_document:` so that queries embedded with
/// `search_query:` land near the fragments they are asking about.
pub fn embed_document(text: &str) -> Option<Vec<f32>> {
    embed_prefixed("search_document: ", text)
}

/// Embed a search query (see [`embed_document`]).
pub fn embed_query(text: &str) -> Option<Vec<f32>> {
    embed_prefixed("search_query: ", text)
}

fn embed_prefixed(prefix: &str, text: &str) -> Option<Vec<f32>> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    embed(&format!("{prefix}{text}"))
}

/// Embed a single piece of raw text (no task prefix). Prefer
/// [`embed_document`] / [`embed_query`] for memory. `None` on any error (never
/// panics, never blocks for long: a short connect/read timeout keeps hooks snappy).
pub fn embed(text: &str) -> Option<Vec<f32>> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let model = model()?;
    let url = format!("{}/api/embeddings", ollama_url().trim_end_matches('/'));
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_millis(600))
        .timeout_read(Duration::from_secs(20))
        .build();
    let resp = agent
        .post(&url)
        .set("Content-Type", "application/json")
        .send_json(json!({ "model": model, "prompt": text }))
        .ok()?;
    let body: serde_json::Value = resp.into_json().ok()?;
    // `/api/embeddings` → {"embedding":[..]}; tolerate `/api/embed` shape too.
    let arr = body
        .get("embedding")
        .or_else(|| body.get("embeddings").and_then(|e| e.get(0)))?
        .as_array()?;
    let v: Vec<f32> = arr
        .iter()
        .filter_map(|x| x.as_f64().map(|f| f as f32))
        .collect();
    (!v.is_empty()).then_some(v)
}

/// Cosine similarity in `[-1, 1]`; `0.0` for empty or mismatched vectors.
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

/// Cosine mapped to `[0, 1]` for blending with keyword scores.
pub fn similarity(a: &[f32], b: &[f32]) -> f32 {
    ((cosine(a, b) + 1.0) / 2.0).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_identity_and_orthogonal() {
        assert!((cosine(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
        assert!(cosine(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-6);
        assert!((cosine(&[1.0, 0.0], &[-1.0, 0.0]) + 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_handles_degenerate_inputs() {
        assert_eq!(cosine(&[], &[]), 0.0);
        assert_eq!(cosine(&[1.0, 2.0], &[1.0]), 0.0);
        assert_eq!(cosine(&[0.0, 0.0], &[1.0, 1.0]), 0.0);
    }

    #[test]
    fn similarity_is_bounded() {
        assert!((similarity(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
        assert!((similarity(&[1.0, 0.0], &[-1.0, 0.0]) - 0.0).abs() < 1e-6);
        assert!((similarity(&[1.0, 0.0], &[0.0, 1.0]) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn disabled_when_model_off() {
        // Not a strict test of env (parallel tests share env), just the parser.
        std::env::set_var("SYNAPSE_EMBED_MODEL", "off");
        assert!(model().is_none());
        std::env::remove_var("SYNAPSE_EMBED_MODEL");
    }
}
