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
//! * **Storage** — an append-only JSONL file (owner-only permissions on
//!   unix); no database, no network. Writers serialise on an advisory lock.
//! * **Progressive disclosure** — `search` returns a cheap index, `timeline`
//!   adds surrounding context, `get` fetches full detail for chosen ids.
//! * **Injection** — [`MemoryStore::context`] renders a compact index of recent
//!   work for SessionStart.

use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::compression::{compress_text, estimate_tokens, CompressionLevel};

const NARRATIVE_MAX_CHARS: usize = 700;
const TITLE_MAX_CHARS: usize = 90;
/// How much of the log `add` reads to find the next id.
const TAIL_BYTES: u64 = 64 * 1024;

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
}

impl Observation {
    /// Approximate token cost of the full record (what `get` will spend).
    pub fn full_tokens(&self) -> usize {
        estimate_tokens(&format!(
            "{} {} {}",
            self.title,
            self.narrative,
            self.files.join(" ")
        ))
    }

    /// One-line index entry (what `search`/`context` spend).
    pub fn index_line(&self) -> String {
        format!(
            "#{:<5} {} {:<9} {}  (~{}t)",
            self.id,
            format_ts(self.ts),
            self.kind,
            self.title,
            self.full_tokens()
        )
    }
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
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryStats {
    pub observations: usize,
    pub sessions: usize,
    pub projects: usize,
    pub stored_tokens: usize,
    pub bytes: u64,
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

    /// Takes the store-wide advisory lock (released when the guard drops).
    /// A separate lock file is used because `forget` replaces the log by
    /// rename, which would orphan a lock held on the log itself.
    fn lock(&self) -> std::io::Result<File> {
        crate::security::create_private_dir_all(&self.dir)?;
        let mut opts = OpenOptions::new();
        opts.create(true).truncate(false).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let f = opts.open(self.dir.join("observations.lock"))?;
        f.lock()?;
        Ok(f)
    }

    /// Sanitises (privacy + redaction), assigns an id, and appends.
    /// Returns `None` when nothing is left to store after sanitising.
    pub fn add(&self, new: NewObservation) -> std::io::Result<Option<Observation>> {
        let title = clip(&sanitize(&new.title), TITLE_MAX_CHARS);
        let narrative = clip(&sanitize(&new.narrative), NARRATIVE_MAX_CHARS);
        if title.trim().is_empty() && narrative.trim().is_empty() {
            return Ok(None);
        }
        let _guard = self.lock()?;
        // Only the tail is needed: ids are assigned under the lock and
        // appended in order, so the newest record holds the max id, and
        // duplicate-collapse only cares about the session's latest event.
        let recent = self.tail()?;
        // Collapse immediate duplicates (e.g. repeated identical reads).
        if let Some(last) = recent.iter().rev().find(|o| o.session == new.session) {
            if last.kind == new.kind && last.title == title && last.narrative == narrative {
                return Ok(None);
            }
        }
        let id = recent.iter().map(|o| o.id).max().unwrap_or(0) + 1;
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
        };
        let mut line = serde_json::to_string(&obs).map_err(std::io::Error::other)?;
        line.push('\n');
        let mut opts = OpenOptions::new();
        opts.create(true).append(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
            opts.mode(0o600).custom_flags(libc::O_NOFOLLOW);
            let f = opts.open(self.log_path())?;
            // Tighten logs created by older versions with the default umask.
            f.set_permissions(fs::Permissions::from_mode(0o600))?;
            (&f).write_all(line.as_bytes())?;
        }
        #[cfg(not(unix))]
        opts.open(self.log_path())?.write_all(line.as_bytes())?;
        Ok(Some(obs))
    }

    /// All observations in id order. Corrupt lines are skipped, not fatal.
    /// Each session's summary is rolling: the stop hook appends a fresh one
    /// rather than rewriting the log, and only the newest is visible here.
    pub fn all(&self) -> std::io::Result<Vec<Observation>> {
        let raw = match fs::read_to_string(self.log_path()) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(e),
        };
        let parsed: Vec<Observation> = raw
            .lines()
            .filter_map(|l| serde_json::from_str::<Observation>(l).ok())
            .collect();
        let mut latest_summary: HashMap<&str, u64> = HashMap::new();
        for o in parsed.iter().filter(|o| o.kind == "summary") {
            latest_summary.insert(o.session.as_str(), o.id);
        }
        let superseded: HashSet<u64> = parsed
            .iter()
            .filter(|o| {
                o.kind == "summary" && latest_summary.get(o.session.as_str()) != Some(&o.id)
            })
            .map(|o| o.id)
            .collect();
        Ok(parsed
            .into_iter()
            .filter(|o| !superseded.contains(&o.id))
            .collect())
    }

    /// The records in the last [`TAIL_BYTES`] of the log (all of them for a
    /// small log). Falls back to a full parse if the tail holds no
    /// parseable record, so the max id is never under-counted.
    fn tail(&self) -> std::io::Result<Vec<Observation>> {
        use std::io::{Read, Seek, SeekFrom};
        let mut f = match File::open(self.log_path()) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(e),
        };
        let len = f.metadata()?.len();
        let start = len.saturating_sub(TAIL_BYTES);
        f.seek(SeekFrom::Start(start))?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        let text = String::from_utf8_lossy(&buf);
        // Skip the (probably partial) first line when we seeked mid-file.
        let body = if start > 0 {
            text.split_once('\n').map_or("", |(_, rest)| rest)
        } else {
            &text
        };
        let parsed: Vec<Observation> = body
            .lines()
            .filter_map(|l| serde_json::from_str::<Observation>(l).ok())
            .collect();
        if parsed.is_empty() && start > 0 {
            return self.all();
        }
        Ok(parsed)
    }

    /// Layer 1: ranked keyword search. Terms are scored title ×3, files ×2,
    /// narrative ×1, weighted by rarity, with a small recency tiebreak.
    pub fn search(
        &self,
        query: &str,
        project: Option<&str>,
        limit: usize,
    ) -> std::io::Result<Vec<Observation>> {
        let all = self.all()?;
        let pool: Vec<&Observation> = all
            .iter()
            .filter(|o| project.is_none_or(|p| o.project == p))
            .collect();
        let terms = tokenize(query);
        if terms.is_empty() {
            return Ok(pool.into_iter().rev().take(limit).cloned().collect());
        }
        let n = pool.len().max(1) as f64;
        // Term weights are only tracked for query terms, and document
        // frequency is counted once per term: O(N) per query, not O(N^2).
        let wanted: HashSet<&str> = terms.iter().map(String::as_str).collect();
        let docs: Vec<(HashMap<String, f64>, &Observation)> = pool
            .iter()
            .map(|o| {
                let mut tf: HashMap<String, f64> = HashMap::new();
                let mut bump = |t: String, w: f64| {
                    if wanted.contains(t.as_str()) {
                        *tf.entry(t).or_default() += w;
                    }
                };
                for t in tokenize(&o.title) {
                    bump(t, 3.0);
                }
                for t in o.files.iter().flat_map(|f| tokenize(f)) {
                    bump(t, 2.0);
                }
                for t in tokenize(&o.narrative) {
                    bump(t, 1.0);
                }
                (tf, *o)
            })
            .collect();
        let mut df: HashMap<&str, f64> = HashMap::new();
        for (tf, _) in &docs {
            for t in tf.keys() {
                *df.entry(t.as_str()).or_default() += 1.0;
            }
        }
        let mut scored: Vec<(f64, &Observation)> = docs
            .iter()
            .filter_map(|(tf, o)| {
                let mut score = 0.0;
                for t in &terms {
                    if let Some(w) = tf.get(t) {
                        score += w * (1.0 + (n / df[t.as_str()]).ln());
                    }
                }
                (score > 0.0).then_some((score + o.id as f64 * 1e-6, *o))
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

    /// Delete observations by id. Returns how many were removed. Lines that
    /// do not parse are kept verbatim (they may be from a newer version),
    /// and the log is replaced atomically so a crash never truncates it.
    pub fn forget(&self, ids: &[u64]) -> std::io::Result<usize> {
        let _guard = self.lock()?;
        let raw = match fs::read_to_string(self.log_path()) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(e) => return Err(e),
        };
        let mut removed = 0;
        let mut out = String::with_capacity(raw.len());
        for line in raw.lines() {
            match serde_json::from_str::<Observation>(line) {
                Ok(o) if ids.contains(&o.id) => removed += 1,
                _ => {
                    out.push_str(line);
                    out.push('\n');
                }
            }
        }
        if removed > 0 {
            crate::security::write_private_atomic(&self.log_path(), out)?;
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
    crate::security::redact_secrets(&out)
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
    fn sanitize_redacts_assignment_and_header_shaped_secrets() {
        let out = sanitize(
            "export GITHUB_TOKEN=ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123 then curl -H 'Authorization: Bearer abc.def.ghi' postgres://u:pw99@h",
        );
        for secret in ["ghp_ABCDEF", "abc.def.ghi", "pw99"] {
            assert!(!out.contains(secret), "{out}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn memory_log_and_dir_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let d = tempfile::tempdir().unwrap();
        let s = MemoryStore::new(d.path().join("mem"));
        s.add(note("s", "p", "note", "hello", "")).unwrap();
        let mode = |p: PathBuf| fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(d.path().join("mem")), 0o700);
        assert_eq!(mode(d.path().join("mem/observations.jsonl")), 0o600);
    }

    #[test]
    fn forget_keeps_unparsable_lines_verbatim() {
        let (d, s) = store();
        s.add(note("s", "p", "note", "one", "")).unwrap();
        let log = d.path().join("observations.jsonl");
        let mut raw = fs::read_to_string(&log).unwrap();
        raw.push_str("{\"future\":\"format\"}\n");
        fs::write(&log, raw).unwrap();
        s.add(note("s", "p", "note", "two", "")).unwrap();
        assert_eq!(s.forget(&[1]).unwrap(), 1);
        let after = fs::read_to_string(&log).unwrap();
        assert!(after.contains("{\"future\":\"format\"}"));
        assert!(after.contains("\"two\"") && !after.contains("\"one\""));
    }

    #[test]
    fn concurrent_adds_never_reuse_an_id() {
        let (d, _s) = store();
        let dir = d.path().to_path_buf();
        let handles: Vec<_> = (0..8)
            .map(|t| {
                let dir = dir.clone();
                std::thread::spawn(move || {
                    let s = MemoryStore::new(dir);
                    for i in 0..10 {
                        s.add(note(&format!("s{t}"), "p", "note", &format!("t{t} n{i}"), ""))
                            .unwrap();
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        let all = MemoryStore::new(dir).all().unwrap();
        let ids: HashSet<u64> = all.iter().map(|o| o.id).collect();
        assert_eq!(all.len(), 80);
        assert_eq!(ids.len(), 80);
    }

    #[test]
    fn search_scales_linearly_with_history() {
        let (d, s) = store();
        let mut raw = String::new();
        for i in 1..=5000u64 {
            let o = Observation {
                id: i,
                ts: i,
                session: format!("s{}", i % 50),
                project: "p".into(),
                kind: "command".into(),
                title: format!("Ran: cargo test module{} common words here", i % 97),
                narrative: format!("compiling crate{} and running shared common checks", i % 13),
                files: vec![format!("src/file{}.rs", i % 31)],
            };
            raw.push_str(&serde_json::to_string(&o).unwrap());
            raw.push('\n');
        }
        fs::write(d.path().join("observations.jsonl"), raw).unwrap();
        let t = std::time::Instant::now();
        let hits = s.search("common cargo module5 crate3", None, 10).unwrap();
        assert!(t.elapsed() < std::time::Duration::from_secs(2), "{:?}", t.elapsed());
        assert_eq!(hits.len(), 10);
        assert!(hits[0].title.contains("module5 "));
        // Appending is a tail read, not a full parse: the next id follows on.
        let added = s.add(note("s", "p", "note", "tail id", "")).unwrap().unwrap();
        assert_eq!(added.id, 5001);
    }

    #[test]
    fn only_the_latest_summary_per_session_is_visible() {
        let (_d, s) = store();
        s.add(note("s", "p", "summary", "Session: a", "stalewombat")).unwrap();
        s.add(note("s", "p", "change", "Edited a.rs", "")).unwrap();
        s.add(note("s", "p", "summary", "Session: a", "second")).unwrap();
        s.add(note("t", "p", "summary", "Session: b", "other")).unwrap();
        let summaries: Vec<String> = s
            .all()
            .unwrap()
            .into_iter()
            .filter(|o| o.kind == "summary")
            .map(|o| o.narrative)
            .collect();
        assert_eq!(summaries, vec!["second", "other"]);
        assert!(!s.context("p", 10).unwrap().contains("stalewombat"));
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
}
