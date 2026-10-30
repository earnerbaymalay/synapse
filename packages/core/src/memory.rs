//! Synaptic Memory — persistent cross-session memory for AI coding agents.
//!
//! Inspired by claude-mem (thedotmack/claude-mem), re-built to match Synapse's
//! local-first, zero-telemetry stance:
//!
//! * **Capture** — tool calls, prompts and session ends become *observations*
//!   (via `synapse mem hook <event>` wired to agent lifecycle hooks).
//! * **Compression** — no remote LLM. Tool output is distilled with the
//!   Sensory Gating engine ([`crate::compression`]) and truncated.
//! * **Privacy** — `<private>…</private>` spans are dropped before storage and
//!   obvious credentials are redacted.
//! * **Storage** — an append-only JSONL file; no database, no network.
//! * **Progressive disclosure** — `search` returns a cheap index, `timeline`
//!   adds surrounding context, `get` fetches full detail for chosen ids.
//! * **Injection** — [`MemoryStore::context`] renders a compact index of recent
//!   work for SessionStart.

use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::compression::{compress_text, estimate_tokens, CompressionLevel};

const NARRATIVE_MAX_CHARS: usize = 700;
const TITLE_MAX_CHARS: usize = 90;

/// One remembered event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub id: u64,
    pub ts: u64,
    pub session: String,
    pub project: String,
    /// prompt | command | change | discovery | research | tool | summary | note
    pub kind: String,
    pub title: String,
    pub narrative: String,
    #[serde(default)]
    pub files: Vec<String>,
    /// Subject area for organisation (e.g. "networking", "memory-system").
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub topic: String,
    /// Freeform keyword tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// AI model / agent that recorded this (e.g. "claude-opus-4-8", "qwen2.5-coder").
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model: String,
}

impl Observation {
    /// Approximate token cost of the full record (what `get` will spend).
    pub fn full_tokens(&self) -> usize {
        estimate_tokens(&format!(
            "{} {} {} {} {}",
            self.title,
            self.narrative,
            self.files.join(" "),
            self.topic,
            self.tags.join(" ")
        ))
    }

    /// One-line index entry (what `search`/`context` spend). The optional
    /// `[topic]` tag keeps the index organised without much extra token cost.
    pub fn index_line(&self) -> String {
        let topic = if self.topic.is_empty() {
            String::new()
        } else {
            format!(" [{}]", self.topic)
        };
        format!(
            "#{:<5} {} {:<9} {}{}  (~{}t)",
            self.id,
            format_ts(self.ts),
            self.kind,
            self.title,
            topic,
            self.full_tokens()
        )
    }

    /// Text handed to the embedding model: topic + tags + title + narrative,
    /// so semantically similar fragments cluster by subject, not just wording.
    pub fn embed_input(&self) -> String {
        let mut s = String::new();
        if !self.topic.is_empty() {
            s.push_str(&self.topic);
            s.push_str(". ");
        }
        if !self.tags.is_empty() {
            s.push_str(&self.tags.join(", "));
            s.push_str(". ");
        }
        s.push_str(&self.title);
        if !self.narrative.is_empty() {
            s.push_str(". ");
            s.push_str(&self.narrative);
        }
        s
    }
}

/// One stored embedding vector, keyed to an [`Observation`] id. Kept in a
/// sidecar `embeddings.jsonl` so `observations.jsonl` stays human-readable.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EmbeddingRecord {
    id: u64,
    v: Vec<f32>,
}

/// Input for [`MemoryStore::add`]; id and timestamp are assigned by the store.
#[derive(Debug, Clone, Default)]
pub struct NewObservation {
    pub session: String,
    pub project: String,
    pub kind: String,
    pub title: String,
    pub narrative: String,
    pub files: Vec<String>,
    pub topic: String,
    pub tags: Vec<String>,
    pub model: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryStats {
    pub observations: usize,
    pub sessions: usize,
    pub projects: usize,
    pub stored_tokens: usize,
    pub bytes: u64,
}

/// One collapsed cluster of near-duplicate observations.
#[derive(Debug, Clone)]
pub struct DistillCluster {
    /// Id of the member kept (the most informative one).
    pub keep: u64,
    pub keep_title: String,
    /// Ids of the redundant members removed (or that would be, in a dry run).
    pub drop: Vec<u64>,
    pub tokens_saved: usize,
}

/// Outcome of a [`MemoryStore::distill`] pass.
#[derive(Debug, Clone, Default)]
pub struct DistillReport {
    pub applied: bool,
    pub tokens_before: usize,
    pub tokens_after: usize,
    pub removed: usize,
    pub clusters: Vec<DistillCluster>,
}

pub struct MemoryStore {
    dir: PathBuf,
}

impl MemoryStore {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// `$SYNAPSE_MEMORY_DIR`, else `~/.synapse/memory`.
    pub fn default_dir() -> PathBuf {
        if let Ok(p) = std::env::var("SYNAPSE_MEMORY_DIR") {
            if !p.is_empty() {
                return PathBuf::from(p);
            }
        }
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join(".synapse").join("memory")
    }

    fn log_path(&self) -> PathBuf {
        self.dir.join("observations.jsonl")
    }

    fn embeddings_path(&self) -> PathBuf {
        self.dir.join("embeddings.jsonl")
    }

    /// Sanitises (privacy + redaction), assigns an id, and appends.
    /// Returns `None` when nothing is left to store after sanitising.
    pub fn add(&self, new: NewObservation) -> std::io::Result<Option<Observation>> {
        let title = clip(&sanitize(&new.title), TITLE_MAX_CHARS);
        let narrative = clip(&sanitize(&new.narrative), NARRATIVE_MAX_CHARS);
        if title.trim().is_empty() && narrative.trim().is_empty() {
            return Ok(None);
        }
        fs::create_dir_all(&self.dir)?;
        let existing = self.all()?;
        // Collapse immediate duplicates (e.g. repeated identical reads).
        if let Some(last) = existing.iter().rev().find(|o| o.session == new.session) {
            if last.kind == new.kind && last.title == title && last.narrative == narrative {
                return Ok(None);
            }
        }
        let id = existing.iter().map(|o| o.id).max().unwrap_or(0) + 1;
        let obs = Observation {
            id,
            ts: now_secs(),
            session: new.session,
            project: new.project,
            kind: if new.kind.is_empty() {
                "note".into()
            } else {
                new.kind
            },
            title,
            narrative,
            files: new.files,
            topic: sanitize(&new.topic).trim().to_string(),
            tags: new
                .tags
                .iter()
                .map(|t| t.trim().to_lowercase())
                .filter(|t| !t.is_empty())
                .collect(),
            model: new.model.trim().to_string(),
        };
        let mut line = serde_json::to_string(&obs).map_err(std::io::Error::other)?;
        line.push('\n');
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.log_path())?;
        f.write_all(line.as_bytes())?;
        // Best-effort local embedding — never fails the write if Ollama is down.
        let _ = self.store_embedding(obs.id, &obs.embed_input());
        Ok(Some(obs))
    }

    // ── semantic layer (local embeddings via Ollama) ───────────────────────

    /// Append one embedding to the sidecar. Returns `true` only when a vector
    /// was computed and written; a silent `false` when embeddings are disabled
    /// or the model is unreachable (so the write path never fails).
    fn store_embedding(&self, id: u64, text: &str) -> bool {
        if !crate::embedding::enabled() {
            return false;
        }
        let Some(v) = crate::embedding::embed_document(text) else {
            return false;
        };
        let line = match serde_json::to_string(&EmbeddingRecord { id, v }) {
            Ok(mut l) => {
                l.push('\n');
                l
            }
            Err(_) => return false,
        };
        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.embeddings_path())
        {
            Ok(mut f) => f.write_all(line.as_bytes()).is_ok(),
            Err(_) => false,
        }
    }

    /// Load all stored embeddings keyed by observation id (last write wins).
    pub fn load_embeddings(&self) -> HashMap<u64, Vec<f32>> {
        let mut map = HashMap::new();
        if let Ok(raw) = fs::read_to_string(self.embeddings_path()) {
            for line in raw.lines() {
                if let Ok(rec) = serde_json::from_str::<EmbeddingRecord>(line) {
                    map.insert(rec.id, rec.v);
                }
            }
        }
        map
    }

    /// Rewrite the embedding sidecar to exactly `kept` (compacts duplicates and
    /// drops entries for forgotten observations).
    fn rewrite_embeddings(&self, kept: &HashMap<u64, Vec<f32>>) -> std::io::Result<()> {
        if kept.is_empty() {
            let _ = fs::remove_file(self.embeddings_path());
            return Ok(());
        }
        let mut ids: Vec<&u64> = kept.keys().collect();
        ids.sort();
        let mut out = String::new();
        for id in ids {
            out.push_str(
                &serde_json::to_string(&EmbeddingRecord {
                    id: *id,
                    v: kept[id].clone(),
                })
                .map_err(std::io::Error::other)?,
            );
            out.push('\n');
        }
        fs::write(self.embeddings_path(), out)
    }

    /// Compute embeddings for every observation that is missing one. Returns the
    /// number newly embedded. Used by `mem reindex` and after a bulk import.
    pub fn reembed_missing(&self) -> std::io::Result<usize> {
        if !crate::embedding::enabled() {
            return Ok(0);
        }
        let all = self.all()?;
        let have = self.load_embeddings();
        let mut added = 0usize;
        for o in &all {
            if have.contains_key(&o.id) {
                continue;
            }
            if self.store_embedding(o.id, &o.embed_input()) {
                added += 1;
            }
        }
        Ok(added)
    }

    /// Distillation — find semantically near-duplicate observations (cosine ≥
    /// `threshold`) within a project and collapse each cluster to its single
    /// most-informative member, cutting the memory footprint while keeping the
    /// genuinely distinct knowledge. `summary` observations are never touched.
    /// With `apply=false` this only reports; with `apply=true` it forgets the
    /// redundant ids. Requires embeddings (returns an empty report otherwise).
    pub fn distill(
        &self,
        project: Option<&str>,
        threshold: f32,
        apply: bool,
    ) -> std::io::Result<DistillReport> {
        let all = self.all()?;
        let embeddings = self.load_embeddings();
        let pool: Vec<&Observation> = all
            .iter()
            .filter(|o| project.is_none_or(|p| o.project == p))
            .filter(|o| o.kind != "summary")
            .filter(|o| embeddings.contains_key(&o.id))
            .collect();

        let tokens_before: usize = all.iter().map(Observation::full_tokens).sum();
        let mut assigned: HashSet<u64> = HashSet::new();
        let mut clusters: Vec<DistillCluster> = Vec::new();

        for (i, a) in pool.iter().enumerate() {
            if assigned.contains(&a.id) {
                continue;
            }
            let av = &embeddings[&a.id];
            let mut members: Vec<&Observation> = vec![*a];
            assigned.insert(a.id);
            for b in pool.iter().skip(i + 1) {
                if assigned.contains(&b.id) {
                    continue;
                }
                if crate::embedding::cosine(av, &embeddings[&b.id]) >= threshold {
                    members.push(*b);
                    assigned.insert(b.id);
                }
            }
            if members.len() < 2 {
                continue;
            }
            // Keep the most informative (token-richest); drop the rest.
            members.sort_by_key(|o| std::cmp::Reverse(o.full_tokens()));
            let keep = members[0].id;
            let drop: Vec<u64> = members[1..].iter().map(|o| o.id).collect();
            let saved: usize = members[1..].iter().map(|o| o.full_tokens()).sum();
            clusters.push(DistillCluster {
                keep,
                keep_title: members[0].title.clone(),
                drop,
                tokens_saved: saved,
            });
        }

        let removed_ids: Vec<u64> = clusters.iter().flat_map(|c| c.drop.clone()).collect();
        let tokens_saved: usize = clusters.iter().map(|c| c.tokens_saved).sum();
        if apply && !removed_ids.is_empty() {
            self.forget(&removed_ids)?;
        }
        Ok(DistillReport {
            applied: apply,
            tokens_before,
            tokens_after: tokens_before.saturating_sub(tokens_saved),
            removed: removed_ids.len(),
            clusters,
        })
    }

    /// All observations in id order. Corrupt lines are skipped, not fatal.
    pub fn all(&self) -> std::io::Result<Vec<Observation>> {
        let raw = match fs::read_to_string(self.log_path()) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(e),
        };
        Ok(raw
            .lines()
            .filter_map(|l| serde_json::from_str::<Observation>(l).ok())
            .collect())
    }

    /// Layer 1: ranked search. Keyword relevance (title ×3, files ×2, narrative
    /// ×1, tags ×2, topic ×2, rarity-weighted) is blended with local semantic
    /// similarity from `nomic-embed-text`, so paraphrases and related subjects
    /// surface even when they share no literal terms. Falls back cleanly to
    /// keyword-only when embeddings are disabled or Ollama is unreachable.
    pub fn search(
        &self,
        query: &str,
        project: Option<&str>,
        limit: usize,
    ) -> std::io::Result<Vec<Observation>> {
        self.search_filtered(query, project, None, None, limit)
    }

    /// Like [`search`], with optional `topic` and `model` equality filters.
    pub fn search_filtered(
        &self,
        query: &str,
        project: Option<&str>,
        topic: Option<&str>,
        model: Option<&str>,
        limit: usize,
    ) -> std::io::Result<Vec<Observation>> {
        let all = self.all()?;
        let pool: Vec<&Observation> = all
            .iter()
            .filter(|o| project.is_none_or(|p| o.project == p))
            .filter(|o| topic.is_none_or(|t| o.topic.eq_ignore_ascii_case(t)))
            .filter(|o| model.is_none_or(|m| o.model.eq_ignore_ascii_case(m)))
            .collect();
        let terms = tokenize(query);
        if terms.is_empty() {
            return Ok(pool.into_iter().rev().take(limit).cloned().collect());
        }

        // Keyword tf-idf over title/files/narrative/tags/topic.
        let n = pool.len().max(1) as f64;
        let docs: Vec<(HashMap<String, f64>, &Observation)> = pool
            .iter()
            .map(|o| {
                let mut tf: HashMap<String, f64> = HashMap::new();
                for t in tokenize(&o.title) {
                    *tf.entry(t).or_default() += 3.0;
                }
                for t in o.files.iter().flat_map(|f| tokenize(f)) {
                    *tf.entry(t).or_default() += 2.0;
                }
                for t in o.tags.iter().flat_map(|t| tokenize(t)) {
                    *tf.entry(t).or_default() += 2.0;
                }
                for t in tokenize(&o.topic) {
                    *tf.entry(t).or_default() += 2.0;
                }
                for t in tokenize(&o.narrative) {
                    *tf.entry(t).or_default() += 1.0;
                }
                (tf, *o)
            })
            .collect();
        let kw: HashMap<u64, f64> = docs
            .iter()
            .filter_map(|(tf, o)| {
                let mut score = 0.0;
                for t in &terms {
                    if let Some(w) = tf.get(t) {
                        let df = docs.iter().filter(|(d, _)| d.contains_key(t)).count() as f64;
                        score += w * (1.0 + (n / df).ln());
                    }
                }
                (score > 0.0).then_some((o.id, score))
            })
            .collect();
        let kw_max = kw.values().cloned().fold(0.0f64, f64::max);

        // Semantic similarity (best-effort; empty map ⇒ keyword-only behaviour).
        let embeddings = self.load_embeddings();
        let qvec = if embeddings.is_empty() {
            None
        } else {
            crate::embedding::embed_query(query)
        };
        // Raw cosine per doc (so the semantic-only floor is interpretable).
        let sem_cos: HashMap<u64, f64> = match &qvec {
            Some(qv) => pool
                .iter()
                .filter_map(|o| {
                    embeddings
                        .get(&o.id)
                        .map(|dv| (o.id, crate::embedding::cosine(qv, dv) as f64))
                })
                .collect(),
            None => HashMap::new(),
        };

        // Admit a semantic-only match (no keyword hit) at cosine ≥ this. nomic's
        // related query/document pairs land around 0.5–0.75 with task prefixes;
        // 0.5 recalls paraphrases without dragging in unrelated notes. Tunable.
        let sem_floor: f64 = std::env::var("SYNAPSE_SEM_FLOOR")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.5);
        // Blend: 0.6 keyword + 0.4 semantic (cosine mapped to [0,1]). Without
        // embeddings this collapses to the original keyword ranking.
        let mut scored: Vec<(f64, &Observation)> = pool
            .iter()
            .filter_map(|o| {
                let kw_norm = if kw_max > 0.0 {
                    kw.get(&o.id).copied().unwrap_or(0.0) / kw_max
                } else {
                    0.0
                };
                let cos = sem_cos.get(&o.id).copied().unwrap_or(0.0);
                let combined = if sem_cos.is_empty() {
                    kw.get(&o.id).copied().unwrap_or(0.0)
                } else {
                    0.6 * kw_norm + 0.4 * ((cos + 1.0) / 2.0)
                };
                let relevant =
                    kw.contains_key(&o.id) || (!sem_cos.is_empty() && cos >= sem_floor);
                relevant.then_some((combined + o.id as f64 * 1e-9, *o))
            })
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored
            .into_iter()
            .take(limit)
            .map(|(_, o)| o.clone())
            .collect())
    }

    /// Layer 2: chronological neighbours of `id` within the same project.
    pub fn timeline(
        &self,
        id: u64,
        before: usize,
        after: usize,
    ) -> std::io::Result<Vec<Observation>> {
        let all = self.all()?;
        let Some(pos) = all.iter().position(|o| o.id == id) else {
            return Ok(vec![]);
        };
        let project = all[pos].project.clone();
        let same: Vec<&Observation> = all.iter().filter(|o| o.project == project).collect();
        let p = same.iter().position(|o| o.id == id).unwrap_or(0);
        let lo = p.saturating_sub(before);
        let hi = (p + after + 1).min(same.len());
        Ok(same[lo..hi].iter().map(|o| (*o).clone()).collect())
    }

    /// Layer 3: full records for specific ids (in the order requested).
    pub fn get(&self, ids: &[u64]) -> std::io::Result<Vec<Observation>> {
        let all = self.all()?;
        Ok(ids
            .iter()
            .filter_map(|id| all.iter().find(|o| o.id == *id).cloned())
            .collect())
    }

    /// Delete observations by id. Returns how many were removed. Keeps the
    /// embedding sidecar in sync so vectors never outlive their observation.
    pub fn forget(&self, ids: &[u64]) -> std::io::Result<usize> {
        let all = self.all()?;
        let keep: Vec<&Observation> = all.iter().filter(|o| !ids.contains(&o.id)).collect();
        let removed = all.len() - keep.len();
        if removed > 0 {
            let mut out = String::new();
            for o in &keep {
                out.push_str(&serde_json::to_string(o).map_err(std::io::Error::other)?);
                out.push('\n');
            }
            fs::write(self.log_path(), out)?;
            let mut embeddings = self.load_embeddings();
            if !embeddings.is_empty() {
                embeddings.retain(|id, _| !ids.contains(id));
                self.rewrite_embeddings(&embeddings)?;
            }
        }
        Ok(removed)
    }

    pub fn stats(&self) -> std::io::Result<MemoryStats> {
        let all = self.all()?;
        Ok(MemoryStats {
            observations: all.len(),
            sessions: all
                .iter()
                .map(|o| o.session.as_str())
                .collect::<HashSet<_>>()
                .len(),
            projects: all
                .iter()
                .map(|o| o.project.as_str())
                .collect::<HashSet<_>>()
                .len(),
            stored_tokens: all.iter().map(Observation::full_tokens).sum(),
            bytes: fs::metadata(self.log_path()).map(|m| m.len()).unwrap_or(0),
        })
    }

    /// SessionStart payload: session summaries first, then the most recent
    /// observations, as a compact index the agent can drill into.
    pub fn context(&self, project: &str, limit: usize) -> std::io::Result<String> {
        let all = self.all()?;
        let mine: Vec<&Observation> = all.iter().filter(|o| o.project == project).collect();
        if mine.is_empty() {
            return Ok(String::new());
        }
        let summaries: Vec<&&Observation> = mine
            .iter()
            .rev()
            .filter(|o| o.kind == "summary")
            .take(3)
            .collect();
        let recent: Vec<&&Observation> = mine
            .iter()
            .rev()
            .filter(|o| o.kind != "summary" && o.kind != "prompt")
            .take(limit)
            .collect();
        let index_tokens: usize = summaries
            .iter()
            .chain(recent.iter())
            .map(|o| estimate_tokens(&o.index_line()))
            .sum();
        let hidden: usize = summaries
            .iter()
            .chain(recent.iter())
            .map(|o| o.full_tokens())
            .sum();

        let mut out = format!(
            "# Synapse memory — {}\n_{} remembered events; index below costs ~{} tokens, full detail ~{}._\n",
            project,
            mine.len(),
            index_tokens,
            hidden
        );
        if !summaries.is_empty() {
            out.push_str("\n## Previous sessions\n");
            for o in &summaries {
                out.push_str(&format!(
                    "- #{} {} — {}\n",
                    o.id,
                    format_ts(o.ts),
                    o.narrative
                ));
            }
        }
        if !recent.is_empty() {
            out.push_str("\n## Recent activity (newest first)\n");
            for o in &recent {
                out.push_str(&format!("{}\n", o.index_line()));
            }
        }
        out.push_str(
            "\nDrill down on demand: `synapse mem search <query>` → `synapse mem timeline <id>` → `synapse mem get <id>…`. \
             Wrap anything sensitive in <private>…</private> to keep it out of memory.\n",
        );
        Ok(out)
    }
}

/// Distils a Claude-Code-style tool call into an observation. Returns `None`
/// for tools that carry no durable signal.
pub fn observation_from_tool(
    session: &str,
    project: &str,
    tool: &str,
    input: &Value,
    response: &Value,
) -> Option<NewObservation> {
    if tool.is_empty()
        || matches!(
            tool,
            "TodoWrite" | "TaskCreate" | "TaskUpdate" | "TaskList" | "ToolSearch"
        )
    {
        return None;
    }
    let s = |k: &str| input.get(k).and_then(Value::as_str).unwrap_or("");
    let file = ["file_path", "path", "notebook_path"]
        .iter()
        .map(|k| s(k))
        .find(|v| !v.is_empty());
    let files: Vec<String> = file.map(|f| vec![f.to_string()]).unwrap_or_default();
    let out_text = response_text(response);

    let (kind, title, narrative) = match tool {
        "Bash" => {
            let cmd = s("command");
            // Never memorise our own memory queries (feedback loop).
            if cmd.contains("synapse mem") || cmd.contains("synapse mem") {
                return None;
            }
            let (digest, ..) = compress_text(&out_text, CompressionLevel::Aggressive);
            (
                "command",
                format!("Ran: {}", first_line(cmd)),
                format!("$ {}\n{}", cmd, digest),
            )
        }
        "Edit" | "MultiEdit" | "Write" | "NotebookEdit" => {
            let verb = if tool == "Write" { "Wrote" } else { "Edited" };
            let f = file.unwrap_or("(unknown file)");
            (
                "change",
                format!("{} {}", verb, f),
                format!("{} {}", verb.to_lowercase(), f),
            )
        }
        "Read" | "Grep" | "Glob" => {
            let target = file.unwrap_or_else(|| {
                [s("pattern"), s("query")]
                    .into_iter()
                    .find(|v| !v.is_empty())
                    .unwrap_or("")
            });
            (
                "discovery",
                format!("{} {}", tool, target),
                format!("{} {}", tool.to_lowercase(), target),
            )
        }
        "WebFetch" | "WebSearch" => {
            let target = [s("url"), s("query")]
                .into_iter()
                .find(|v| !v.is_empty())
                .unwrap_or("");
            (
                "research",
                format!("{} {}", tool, target),
                clip(&out_text, 400),
            )
        }
        _ => (
            "tool",
            format!("{}: {}", tool, first_line(&out_text)),
            clip(&out_text, 300),
        ),
    };
    Some(NewObservation {
        session: session.into(),
        project: project.into(),
        kind: kind.into(),
        title,
        narrative,
        files,
        ..Default::default()
    })
}

/// Builds the end-of-session rollup from what was recorded.
pub fn summarize_session(
    store: &MemoryStore,
    session: &str,
    project: &str,
) -> std::io::Result<Option<NewObservation>> {
    let obs: Vec<Observation> = store
        .all()?
        .into_iter()
        .filter(|o| o.session == session && o.kind != "summary")
        .collect();
    if obs.is_empty() {
        return Ok(None);
    }
    let prompts: Vec<&str> = obs
        .iter()
        .filter(|o| o.kind == "prompt")
        .map(|o| o.title.as_str())
        .collect();
    let mut files: Vec<String> = obs
        .iter()
        .filter(|o| o.kind == "change")
        .flat_map(|o| o.files.clone())
        .collect();
    files.sort();
    files.dedup();
    let cmds = obs.iter().filter(|o| o.kind == "command").count();
    let goal = prompts.first().copied().unwrap_or("(no prompt captured)");
    let mut narrative = format!("Goal: {}.", goal);
    if prompts.len() > 1 {
        narrative.push_str(&format!(" Latest ask: {}.", prompts.last().unwrap()));
    }
    narrative.push_str(&format!(
        " {} commands run, {} files changed",
        cmds,
        files.len()
    ));
    if !files.is_empty() {
        narrative.push_str(&format!(
            ": {}",
            files.iter().take(6).cloned().collect::<Vec<_>>().join(", ")
        ));
    }
    narrative.push('.');
    Ok(Some(NewObservation {
        session: session.into(),
        project: project.into(),
        kind: "summary".into(),
        title: format!("Session: {}", clip(goal, 60)),
        narrative,
        files,
        ..Default::default()
    }))
}

/// Drops `<private>…</private>` spans (an unclosed tag drops to end of text)
/// and redacts credential-looking tokens.
pub fn sanitize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        let lower = rest.to_ascii_lowercase();
        match lower.find("<private>") {
            None => {
                out.push_str(rest);
                break;
            }
            Some(start) => {
                out.push_str(&rest[..start]);
                match lower[start..].find("</private>") {
                    Some(end) => rest = &rest[start + end + "</private>".len()..],
                    None => break,
                }
            }
        }
    }
    redact_secrets(&out)
}

fn redact_secrets(text: &str) -> String {
    const PREFIXES: [&str; 8] = [
        "sk-",
        "ghp_",
        "gho_",
        "github_pat_",
        "AKIA",
        "xoxb-",
        "xoxp-",
        "AIza",
    ];
    text.split_inclusive(char::is_whitespace)
        .map(|chunk| {
            let word = chunk.trim_end();
            let tail = &chunk[word.len()..];
            let core = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_');
            if core.len() >= 20 && PREFIXES.iter().any(|p| core.starts_with(p)) {
                format!("[REDACTED]{}", tail)
            } else {
                chunk.to_string()
            }
        })
        .collect()
}

fn response_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        Value::Object(m) => ["stdout", "output", "content", "text", "stderr"]
            .iter()
            .filter_map(|k| m.get(*k))
            .map(response_text)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Array(a) => a.iter().map(response_text).collect::<Vec<_>>().join("\n"),
        other => other.to_string(),
    }
}

fn tokenize(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 2)
        .map(String::from)
        .collect()
}

fn first_line(s: &str) -> String {
    clip(s.lines().next().unwrap_or("").trim(), TITLE_MAX_CHARS)
}

fn clip(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut t: String = s.chars().take(max.saturating_sub(1)).collect();
    t.push('…');
    t
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// `YYYY-MM-DD HH:MM` in UTC (civil-from-days; avoids a chrono dependency).
pub fn format_ts(ts: u64) -> String {
    let days = (ts / 86_400) as i64;
    let secs = ts % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        y,
        m,
        d,
        secs / 3600,
        (secs % 3600) / 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn store() -> (tempfile::TempDir, MemoryStore) {
        let d = tempfile::tempdir().unwrap();
        let s = MemoryStore::new(d.path().to_path_buf());
        (d, s)
    }

    fn note(
        session: &str,
        project: &str,
        kind: &str,
        title: &str,
        narrative: &str,
    ) -> NewObservation {
        NewObservation {
            session: session.into(),
            project: project.into(),
            kind: kind.into(),
            title: title.into(),
            narrative: narrative.into(),
            files: vec![],
            ..Default::default()
        }
    }

    #[test]
    fn private_spans_and_secrets_never_reach_disk() {
        let (d, s) = store();
        s.add(note(
            "s",
            "p",
            "note",
            "deploy",
            "key sk-abcdefghijklmnopqrstuvwx <private>hunter2</private> ok",
        ))
        .unwrap();
        let raw = fs::read_to_string(d.path().join("observations.jsonl")).unwrap();
        assert!(!raw.contains("hunter2") && !raw.contains("sk-abcdef"));
        assert!(raw.contains("[REDACTED]"));
    }

    #[test]
    fn unclosed_private_tag_drops_the_rest() {
        assert_eq!(sanitize("keep <PRIVATE>secret forever"), "keep ");
    }

    #[test]
    fn fully_private_observation_is_not_stored() {
        let (_d, s) = store();
        assert!(s
            .add(note(
                "s",
                "p",
                "note",
                "<private>x</private>",
                "<private>y</private>"
            ))
            .unwrap()
            .is_none());
        assert_eq!(s.stats().unwrap().observations, 0);
    }

    #[test]
    fn search_ranks_title_hits_and_scopes_by_project() {
        let (_d, s) = store();
        s.add(note(
            "s",
            "p",
            "note",
            "Fixed auth token refresh",
            "oauth flow",
        ))
        .unwrap();
        s.add(note(
            "s",
            "p",
            "note",
            "Tweaked css",
            "mentions auth once in passing",
        ))
        .unwrap();
        s.add(note("s", "other", "note", "auth in other project", ""))
            .unwrap();
        let hits = s.search("auth", Some("p"), 10).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].title, "Fixed auth token refresh");
    }

    #[test]
    fn timeline_and_get_progressive_disclosure() {
        let (_d, s) = store();
        for i in 1..=5 {
            s.add(note("s", "p", "note", &format!("step {i}"), ""))
                .unwrap();
        }
        let t = s.timeline(3, 1, 1).unwrap();
        assert_eq!(t.iter().map(|o| o.id).collect::<Vec<_>>(), vec![2, 3, 4]);
        assert_eq!(
            s.get(&[5, 1])
                .unwrap()
                .iter()
                .map(|o| o.id)
                .collect::<Vec<_>>(),
            vec![5, 1]
        );
    }

    #[test]
    fn consecutive_duplicates_collapse() {
        let (_d, s) = store();
        assert!(s
            .add(note("s", "p", "discovery", "Read a.rs", "read a.rs"))
            .unwrap()
            .is_some());
        assert!(s
            .add(note("s", "p", "discovery", "Read a.rs", "read a.rs"))
            .unwrap()
            .is_none());
    }

    #[test]
    fn forget_removes_only_requested_ids() {
        let (_d, s) = store();
        s.add(note("s", "p", "note", "one", "")).unwrap();
        s.add(note("s", "p", "note", "two", "")).unwrap();
        assert_eq!(s.forget(&[1]).unwrap(), 1);
        assert_eq!(s.all().unwrap()[0].title, "two");
    }

    #[test]
    fn tool_calls_become_observations_and_memory_queries_are_ignored() {
        let o = observation_from_tool(
            "s",
            "p",
            "Edit",
            &json!({"file_path": "src/a.rs"}),
            &Value::Null,
        )
        .unwrap();
        assert_eq!(o.kind, "change");
        assert_eq!(o.files, vec!["src/a.rs"]);
        assert!(observation_from_tool(
            "s",
            "p",
            "Bash",
            &json!({"command": "synapse mem search x"}),
            &Value::Null
        )
        .is_none());
        assert!(observation_from_tool("s", "p", "", &Value::Null, &Value::Null).is_none());
        assert!(observation_from_tool("s", "p", "TodoWrite", &json!({}), &Value::Null).is_none());
    }

    #[test]
    fn session_summary_rolls_up_prompts_and_changed_files() {
        let (_d, s) = store();
        s.add(note("s", "p", "prompt", "add dark mode", ""))
            .unwrap();
        let mut c = note("s", "p", "change", "Edited a.rs", "edited a.rs");
        c.files = vec!["a.rs".into()];
        s.add(c).unwrap();
        let sum = summarize_session(&s, "s", "p").unwrap().unwrap();
        assert!(sum.narrative.contains("add dark mode") && sum.narrative.contains("a.rs"));
    }

    #[test]
    fn context_lists_summaries_then_recent_and_is_empty_for_unknown_project() {
        let (_d, s) = store();
        s.add(note("s", "p", "summary", "Session: x", "did x"))
            .unwrap();
        s.add(note("s", "p", "change", "Edited a.rs", "")).unwrap();
        let ctx = s.context("p", 10).unwrap();
        assert!(ctx.contains("Previous sessions") && ctx.contains("Edited a.rs"));
        assert_eq!(s.context("nope", 10).unwrap(), "");
    }

    #[test]
    fn format_ts_epoch_and_known_date() {
        assert_eq!(format_ts(0), "1970-01-01 00:00");
        assert_eq!(format_ts(1_760_227_200), "2025-10-12 00:00");
    }

    #[test]
    fn new_fields_are_stored_and_old_lines_still_parse() {
        let (d, s) = store();
        let mut n = note("s", "p", "note", "stealth egress policy", "firewalld drops mdns");
        n.topic = "Networking".into();
        n.tags = vec!["Firewall".into(), " ".into(), "mDNS".into()];
        n.model = "claude-opus-4-8".into();
        let o = s.add(n).unwrap().unwrap();
        // topic preserved, tags trimmed + lowercased + blanks dropped.
        assert_eq!(o.topic, "Networking");
        assert_eq!(o.tags, vec!["firewall", "mdns"]);
        assert_eq!(o.model, "claude-opus-4-8");
        // A legacy line without the new fields deserialises with defaults.
        fs::write(
            d.path().join("observations.jsonl"),
            "{\"id\":9,\"ts\":1,\"session\":\"x\",\"project\":\"p\",\"kind\":\"note\",\"title\":\"old\",\"narrative\":\"\"}\n",
        )
        .unwrap();
        let back = &s.all().unwrap()[0];
        assert_eq!(back.id, 9);
        assert_eq!(back.topic, "");
        assert!(back.tags.is_empty());
        assert_eq!(back.model, "");
    }

    #[test]
    fn search_filters_by_topic_and_model() {
        let (_d, s) = store();
        let mk = |title: &str, topic: &str, model: &str| {
            let mut n = note("s", "p", "note", title, "shared body about tokens");
            n.topic = topic.into();
            n.model = model.into();
            n
        };
        s.add(mk("alpha tokens", "memory", "claude")).unwrap();
        s.add(mk("beta tokens", "networking", "qwen")).unwrap();
        // No embeddings sidecar ⇒ pure keyword, but topic/model still filter.
        let only_mem = s
            .search_filtered("tokens", Some("p"), Some("memory"), None, 10)
            .unwrap();
        assert_eq!(only_mem.len(), 1);
        assert_eq!(only_mem[0].title, "alpha tokens");
        let only_qwen = s
            .search_filtered("tokens", Some("p"), None, Some("qwen"), 10)
            .unwrap();
        assert_eq!(only_qwen.len(), 1);
        assert_eq!(only_qwen[0].title, "beta tokens");
    }

    #[test]
    fn distill_collapses_near_duplicates_to_the_richest() {
        let (d, s) = store();
        // Three notes; #1 and #2 will be "duplicates" via hand-written vectors.
        s.add(note("s", "p", "note", "short dup", "brief")).unwrap();
        s.add(note("s", "p", "note", "long dup", "a much longer, more informative narrative body"))
            .unwrap();
        s.add(note("s", "p", "note", "distinct", "unrelated")).unwrap();
        // Inject embeddings directly (no Ollama in tests): 1 & 2 ≈ parallel, 3 ⊥.
        fs::write(
            d.path().join("embeddings.jsonl"),
            "{\"id\":1,\"v\":[1.0,0.0,0.0]}\n\
             {\"id\":2,\"v\":[0.99,0.1,0.0]}\n\
             {\"id\":3,\"v\":[0.0,0.0,1.0]}\n",
        )
        .unwrap();
        // Dry run: reports but keeps everything.
        let report = s.distill(Some("p"), 0.9, false).unwrap();
        assert_eq!(report.clusters.len(), 1);
        assert_eq!(report.removed, 1);
        assert_eq!(s.all().unwrap().len(), 3);
        // The richer of the pair is kept (#2 has the longer narrative).
        assert_eq!(report.clusters[0].keep, 2);
        assert_eq!(report.clusters[0].drop, vec![1]);
        // Apply: the redundant one is forgotten, along with its embedding.
        let applied = s.distill(Some("p"), 0.9, true).unwrap();
        assert!(applied.applied);
        let ids: Vec<u64> = s.all().unwrap().iter().map(|o| o.id).collect();
        assert_eq!(ids, vec![2, 3]);
        assert!(!s.load_embeddings().contains_key(&1));
    }
}
