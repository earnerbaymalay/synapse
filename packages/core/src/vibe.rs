//! Vibe-coded vs hand-coded line counter over a rolling window (default 24h).
//!
//! Ported natively into Synapse Core from the lab lines subsystem for sub-10ms CLI
//! and status-bar genmon reporting.
//!
//! Vibe (AI) lines come directly from agent transcripts:
//!   - Claude Code: `~/.claude/projects/**/*.jsonl` (`structuredPatch` / `bashEditDiff` "+" lines,
//!     full line count for created files)
//!   - Antigravity / agy: `~/.gemini/antigravity*/conversations/*.db` (protobuf tool calls:
//!     `write_to_file`, `write_file`, `replace_file_content`, `multi_replace_file_content`, `edit_file`)
//!
//! Hand lines are inferred from git: lines added per file in uncommitted/untracked changes
//! inside the window minus the AI lines landing in that same file (floored at 0).

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

const SKIP_SUFFIXES: &[&str] = &[
    ".lock", ".min.js", ".min.css", ".map", ".svg", ".csv", ".log", ".pcap",
];

const SKIP_NAMES: &[&str] = &[
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "uv.lock",
    "Cargo.lock",
];

#[derive(Debug, Default, Clone)]
pub struct Tally {
    pub by_source: HashMap<String, usize>,
    pub by_file: HashMap<PathBuf, usize>,
}

impl Tally {
    pub fn add(&mut self, source: &str, path: &str, n: usize) {
        if n == 0 || path.is_empty() || !is_counted(path) {
            return;
        }
        *self.by_source.entry(source.to_string()).or_insert(0) += n;
        let canon = fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path));
        *self.by_file.entry(canon).or_insert(0) += n;
    }

    pub fn total(&self) -> usize {
        self.by_source.values().sum()
    }
}

pub fn is_counted(path: &str) -> bool {
    let p = Path::new(path);
    let name = match p.file_name().and_then(|s| s.to_str()) {
        Some(n) => n,
        None => return false,
    };
    if SKIP_NAMES.contains(&name) {
        return false;
    }
    for suff in SKIP_SUFFIXES {
        if name.ends_with(suff) {
            return false;
        }
    }
    true
}

pub fn added_lines(old: &str, new: &str) -> usize {
    let patch = diffy::create_patch(old, new);
    patch
        .hunks()
        .iter()
        .flat_map(|h| h.lines())
        .filter(|l| matches!(l, diffy::Line::Insert(_)))
        .count()
}

fn count_plus_hunks(hunks: &[serde_json::Value]) -> usize {
    let mut count = 0;
    for h in hunks {
        if let Some(lines) = h.get("lines").and_then(|l| l.as_array()) {
            for ln in lines {
                if let Some(s) = ln.as_str() {
                    if s.starts_with('+') {
                        count += 1;
                    }
                }
            }
        }
    }
    count
}

fn parse_iso_ts(ts: &str) -> Option<f64> {
    // Basic ISO 8601 parsing fallback using date string heuristics if chrono is not desired
    // Format: "2026-10-25T06:12:34.567Z"
    let clean = ts.trim_end_matches('Z');
    let parts: Vec<&str> = clean.split('T').collect();
    if parts.len() != 2 {
        return None;
    }
    let date_parts: Vec<i64> = parts[0].split('-').filter_map(|s| s.parse().ok()).collect();
    let time_parts: Vec<&str> = parts[1].split(':').collect();
    if date_parts.len() != 3 || time_parts.len() < 3 {
        return None;
    }
    let (year, month, day) = (date_parts[0], date_parts[1], date_parts[2]);
    let hour: i64 = time_parts[0].parse().ok()?;
    let minute: i64 = time_parts[1].parse().ok()?;
    let sec_parts: Vec<&str> = time_parts[2].split('.').collect();
    let sec: i64 = sec_parts[0].parse().ok()?;

    // Compute unix epoch days
    let days = days_from_civil(year, month, day);
    let total_secs = days * 86400 + hour * 3600 + minute * 60 + sec;
    Some(total_secs as f64)
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let mut y = y;
    if m <= 2 {
        y -= 1;
    }
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe as i64 * 365 + yoe as i64 / 4 - yoe as i64 / 100 + doy;
    era * 146097 + doe - 719468
}

fn claude_entry_lines(entry: &serde_json::Value) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    let res = match entry.get("toolUseResult") {
        Some(r) => r,
        None => return out,
    };
    if let Some(file_path) = res.get("filePath").and_then(|p| p.as_str()) {
        if res.get("type").and_then(|t| t.as_str()) == Some("create") {
            let content = res.get("content").and_then(|c| c.as_str()).unwrap_or("");
            out.push((file_path.to_string(), content.lines().count()));
        } else if let Some(sp) = res.get("structuredPatch").and_then(|s| s.as_array()) {
            out.push((file_path.to_string(), count_plus_hunks(sp)));
        }
    }
    if let Some(files) = res
        .get("bashEditDiff")
        .and_then(|b| b.get("files"))
        .and_then(|f| f.as_array())
    {
        for f in files {
            if let Some(fp) = f.get("filePath").and_then(|p| p.as_str()) {
                if let Some(hunks) = f.get("hunks").and_then(|h| h.as_array()) {
                    out.push((fp.to_string(), count_plus_hunks(hunks)));
                }
            }
        }
    }
    out
}

pub fn scan_claude(since_secs: f64, tally: &mut Tally) {
    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return,
    };
    let claude_dir = home.join(".claude").join("projects");
    if !claude_dir.is_dir() {
        return;
    }

    let mut stack = vec![claude_dir];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                if let Ok(meta) = p.metadata() {
                    if let Ok(mtime) = meta.modified() {
                        if let Ok(dur) = mtime.duration_since(UNIX_EPOCH) {
                            if (dur.as_secs_f64()) < since_secs {
                                continue;
                            }
                        }
                    }
                }
                let file = match File::open(&p) {
                    Ok(f) => f,
                    Err(_) => continue,
                };
                let reader = BufReader::new(file);
                for line in reader.lines().map_while(Result::ok) {
                    if !line.contains("\"toolUseResult\"") {
                        continue;
                    }
                    let parsed: serde_json::Value = match serde_json::from_str(&line) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    if let Some(ts_str) = parsed.get("timestamp").and_then(|t| t.as_str()) {
                        if let Some(ts) = parse_iso_ts(ts_str) {
                            if ts < since_secs {
                                continue;
                            }
                        }
                    }
                    for (fp, n) in claude_entry_lines(&parsed) {
                        tally.add("claude", &fp, n);
                    }
                }
            }
        }
    }
}

// Minimal protobuf wire-format parser
fn read_varint(buf: &[u8], mut idx: usize) -> Option<(u64, usize)> {
    let mut v: u64 = 0;
    let mut s: u32 = 0;
    while idx < buf.len() {
        let c = buf[idx];
        idx += 1;
        v |= ((c & 0x7F) as u64) << s;
        s += 7;
        if c < 0x80 {
            return Some((v, idx));
        }
    }
    None
}

fn pb_get_submessage(data: &[u8], field_num: u32) -> Option<&[u8]> {
    let mut i = 0;
    while i < data.len() {
        let (key, next_i) = read_varint(data, i)?;
        i = next_i;
        let num = (key >> 3) as u32;
        let wire = (key & 7) as u8;
        match wire {
            0 => {
                let (_, next_i) = read_varint(data, i)?;
                i = next_i;
            }
            1 => {
                if i + 8 > data.len() {
                    return None;
                }
                i += 8;
            }
            2 => {
                let (len, next_i) = read_varint(data, i)?;
                i = next_i;
                let len = len as usize;
                if i + len > data.len() {
                    return None;
                }
                if num == field_num {
                    return Some(&data[i..i + len]);
                }
                i += len;
            }
            5 => {
                if i + 4 > data.len() {
                    return None;
                }
                i += 4;
            }
            _ => return None,
        }
    }
    None
}

fn pb_get_varint(data: &[u8], field_num: u32) -> Option<u64> {
    let mut i = 0;
    while i < data.len() {
        let (key, next_i) = read_varint(data, i)?;
        i = next_i;
        let num = (key >> 3) as u32;
        let wire = (key & 7) as u8;
        match wire {
            0 => {
                let (v, next_i) = read_varint(data, i)?;
                if num == field_num {
                    return Some(v);
                }
                i = next_i;
            }
            1 => {
                if i + 8 > data.len() {
                    return None;
                }
                i += 8;
            }
            2 => {
                let (len, next_i) = read_varint(data, i)?;
                i = next_i + len as usize;
            }
            5 => {
                if i + 4 > data.len() {
                    return None;
                }
                i += 4;
            }
            _ => return None,
        }
    }
    None
}

fn agy_call_lines(name: &str, a: &serde_json::Value) -> Option<(String, usize)> {
    match name {
        "write_to_file" => {
            let path = a.get("TargetFile")?.as_str()?.to_string();
            let content = a.get("CodeContent")?.as_str().unwrap_or("");
            Some((path, content.lines().count()))
        }
        "write_file" => {
            let path = a.get("Path")?.as_str()?.to_string();
            let content = a.get("Content")?.as_str().unwrap_or("");
            Some((path, content.lines().count()))
        }
        "replace_file_content" => {
            let path = a.get("TargetFile")?.as_str()?.to_string();
            let old = a.get("TargetContent")?.as_str().unwrap_or("");
            let new = a.get("ReplacementContent")?.as_str().unwrap_or("");
            Some((path, added_lines(old, new)))
        }
        "multi_replace_file_content" => {
            let path = a.get("TargetFile")?.as_str()?.to_string();
            let chunks_val = a.get("ReplacementChunks")?;
            let chunks: Vec<serde_json::Value> = if let Some(s) = chunks_val.as_str() {
                serde_json::from_str(s).unwrap_or_default()
            } else if let Some(arr) = chunks_val.as_array() {
                arr.clone()
            } else {
                Vec::new()
            };
            let mut total = 0;
            for c in chunks {
                let old = c
                    .get("TargetContent")
                    .and_then(|s| s.as_str())
                    .unwrap_or("");
                let new = c
                    .get("ReplacementContent")
                    .and_then(|s| s.as_str())
                    .unwrap_or("");
                total += added_lines(old, new);
            }
            Some((path, total))
        }
        "edit_file" => {
            let path = a.get("Path")?.as_str()?.to_string();
            let old = a.get("Target")?.as_str().unwrap_or("");
            let new = a.get("Replacement")?.as_str().unwrap_or("");
            Some((path, added_lines(old, new)))
        }
        _ => None,
    }
}

pub fn scan_agy(since_secs: f64, tally: &mut Tally) {
    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return,
    };
    let gemini = home.join(".gemini");
    if !gemini.is_dir() {
        return;
    }

    let entries = match fs::read_dir(&gemini) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.starts_with("antigravity") {
            continue;
        }
        let conv_dir = entry.path().join("conversations");
        if !conv_dir.is_dir() {
            continue;
        }
        let db_entries = match fs::read_dir(&conv_dir) {
            Ok(d) => d,
            Err(_) => continue,
        };
        for d in db_entries.flatten() {
            let p = d.path();
            if p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("db") {
                if let Ok(meta) = p.metadata() {
                    if let Ok(mtime) = meta.modified() {
                        if let Ok(dur) = mtime.duration_since(UNIX_EPOCH) {
                            if (dur.as_secs_f64()) < since_secs {
                                continue;
                            }
                        }
                    }
                }
                // Shell out to sqlite3 in read-only batch mode to retrieve hex encoded payloads
                let output = Command::new("sqlite3")
                    .args([
                        "-batch",
                        "-noheader",
                        p.to_str().unwrap_or(""),
                        "SELECT hex(step_payload) FROM steps WHERE status = 3 AND step_payload IS NOT NULL;",
                    ])
                    .output();
                if let Ok(out) = output {
                    if out.status.success() {
                        let stdout = String::from_utf8_lossy(&out.stdout);
                        for hex_line in stdout.lines() {
                            let hex_line = hex_line.trim();
                            if hex_line.is_empty() {
                                continue;
                            }
                            if let Ok(bytes) = hex::decode(hex_line) {
                                if let Some(step_5) = pb_get_submessage(&bytes, 5) {
                                    if let Some(step_1) = pb_get_submessage(step_5, 1) {
                                        if let Some(ts) = pb_get_varint(step_1, 1) {
                                            if (ts as f64) < since_secs {
                                                continue;
                                            }
                                            if let Some(step_4) = pb_get_submessage(step_5, 4) {
                                                if let Some(name_bytes) =
                                                    pb_get_submessage(step_4, 2)
                                                {
                                                    let tool_name =
                                                        String::from_utf8_lossy(name_bytes);
                                                    let args_bytes = pb_get_submessage(step_4, 3)
                                                        .unwrap_or(b"{}");
                                                    let args_val: serde_json::Value =
                                                        serde_json::from_slice(args_bytes)
                                                            .unwrap_or(serde_json::json!({}));
                                                    if let Some((fp, n)) =
                                                        agy_call_lines(&tool_name, &args_val)
                                                    {
                                                        tally.add("antigravity", &fp, n);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn find_repos(roots: &[PathBuf], max_depth: usize) -> Vec<PathBuf> {
    let mut repos = Vec::new();
    for root in roots {
        if !root.is_dir() {
            continue;
        }
        let mut stack = vec![(root.clone(), 0)];
        while let Some((dir, depth)) = stack.pop() {
            if depth >= max_depth {
                continue;
            }
            let entries = match fs::read_dir(&dir) {
                Ok(e) => e,
                Err(_) => continue,
            };
            let mut is_repo = false;
            let mut subdirs = Vec::new();
            for entry in entries.flatten() {
                let p = entry.path();
                let file_name = entry.file_name();
                let name = file_name.to_string_lossy();
                if name == ".git" {
                    is_repo = true;
                } else if p.is_dir()
                    && !name.starts_with('.')
                    && name != "node_modules"
                    && name != ".venv"
                    && name != "venv"
                    && name != "target"
                {
                    subdirs.push(p);
                }
            }
            if is_repo {
                repos.push(dir);
            } else {
                for s in subdirs {
                    stack.push((s, depth + 1));
                }
            }
        }
    }
    repos
}

fn parse_numstat(text: &str, repo: &Path) -> HashMap<PathBuf, usize> {
    let mut out = HashMap::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() != 3 {
            continue;
        }
        let added: usize = match parts[0].parse() {
            Ok(n) => n,
            Err(_) => continue, // binary files or '-'
        };
        let mut path = parts[2].to_string();
        if path.contains(" => ") {
            if let Some(dest) = path.split(" => ").last() {
                path = dest.replace(['}', '{'], "");
            }
        }
        if is_counted(&path) {
            let full = repo.join(&path);
            let canon = fs::canonicalize(&full).unwrap_or(full);
            *out.entry(canon).or_insert(0) += added;
        }
    }
    out
}

pub fn scan_git(
    since_secs: f64,
    repos: &[PathBuf],
) -> (HashMap<PathBuf, usize>, HashMap<PathBuf, usize>) {
    let mut working = HashMap::new();
    let mut committed = HashMap::new();

    for repo in repos {
        // Committed lines in window
        let since_arg = format!("--since=@{}", since_secs as u64);
        if let Ok(out) = Command::new("git")
            .args([
                "-C",
                repo.to_str().unwrap_or(""),
                "log",
                &since_arg,
                "--format=",
                "--numstat",
                "--no-merges",
            ])
            .output()
        {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                for (k, v) in parse_numstat(&text, repo) {
                    *committed.entry(k).or_insert(0) += v;
                }
            }
        }

        // Uncommitted working tree diff
        if let Ok(out) = Command::new("git")
            .args([
                "-C",
                repo.to_str().unwrap_or(""),
                "diff",
                "HEAD",
                "--numstat",
            ])
            .output()
        {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                for (k, v) in parse_numstat(&text, repo) {
                    *working.entry(k).or_insert(0) += v;
                }
            }
        }

        // Untracked files
        if let Ok(out) = Command::new("git")
            .args([
                "-C",
                repo.to_str().unwrap_or(""),
                "ls-files",
                "--others",
                "--exclude-standard",
            ])
            .output()
        {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                for f in text.lines() {
                    let p = repo.join(f);
                    if is_counted(f) {
                        if let Ok(meta) = p.metadata() {
                            if let Ok(mtime) = meta.modified() {
                                if let Ok(dur) = mtime.duration_since(UNIX_EPOCH) {
                                    if (dur.as_secs_f64()) >= since_secs && meta.len() < 2_000_000 {
                                        if let Ok(content) = fs::read_to_string(&p) {
                                            let canon = fs::canonicalize(&p).unwrap_or(p);
                                            *working.entry(canon).or_insert(0) +=
                                                content.lines().count();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    (working, committed)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VibeReport {
    pub hours: f64,
    pub vibe: usize,
    pub hand: usize,
    pub committed: usize,
    pub vibe_pct: usize,
    pub by_source: HashMap<String, usize>,
    pub top_vibe_files: Vec<(String, usize)>,
}

pub fn compute_vibe(hours: f64, roots: Option<Vec<PathBuf>>) -> VibeReport {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs_f64();
    let since = now - hours * 3600.0;

    let mut ai = Tally::default();
    scan_claude(since, &mut ai);
    scan_agy(since, &mut ai);

    let default_roots = match std::env::var("HOME") {
        Ok(h) => {
            let p = PathBuf::from(h);
            vec![
                p.join("Desktop"),
                p.join("Projects"),
                p.join("jailbox"),
                p.join("Downloads"),
                p.join("work"),
            ]
        }
        Err(_) => vec![],
    };

    let search_roots = roots.unwrap_or_else(|| {
        if let Ok(env_roots) = std::env::var("LAB_CODE_ROOTS") {
            env_roots.split(':').map(PathBuf::from).collect()
        } else {
            default_roots
        }
    });

    let repos = find_repos(&search_roots, 4);
    let (working, committed) = scan_git(since, &repos);

    let mut hand = 0;
    for (f, n) in &working {
        let ai_count = ai.by_file.get(f).copied().unwrap_or(0);
        if *n > ai_count {
            hand += *n - ai_count;
        }
    }

    let total = ai.total() + hand;
    let vibe_pct = if total > 0 {
        ((100.0 * ai.total() as f64) / total as f64).round() as usize
    } else {
        0
    };

    let total_vibe = ai.total();
    let mut sorted_files: Vec<(PathBuf, usize)> = ai.by_file.into_iter().collect();
    sorted_files.sort_by_key(|b| std::cmp::Reverse(b.1));

    let top_vibe_files = sorted_files
        .into_iter()
        .take(5)
        .map(|(p, n)| (p.to_string_lossy().to_string(), n))
        .collect();

    let committed_total: usize = committed.values().sum();

    VibeReport {
        hours,
        vibe: total_vibe,
        hand,
        committed: committed_total,
        vibe_pct,
        by_source: ai.by_source,
        top_vibe_files,
    }
}

pub fn render_text(r: &VibeReport) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut sources: Vec<String> = r
        .by_source
        .iter()
        .map(|(k, v)| format!("{k} {v}"))
        .collect();
    sources.sort();
    let src = if sources.is_empty() {
        "none".to_string()
    } else {
        sources.join(", ")
    };

    let hours_display = if (r.hours.fract()).abs() < 1e-6 {
        format!("{:.0}", r.hours)
    } else {
        format!("{:.1}", r.hours)
    };
    let mut lines = vec![
        format!("Last {hours_display}h"),
        format!("  󰚩 vibe-coded  {:>7}   ({src})", r.vibe),
        format!(
            "  ✍ hand-coded  {:>7}   (uncommitted + untracked, minus AI)",
            r.hand
        ),
        format!("  vibe share    {:>6}%", r.vibe_pct),
        format!(
            "   committed    {:>7}   (in-window commits; mixed authorship, uncounted)",
            r.committed
        ),
    ];

    if !r.top_vibe_files.is_empty() {
        lines.push("  top AI-edited files:".to_string());
        for (f, n) in &r.top_vibe_files {
            let display = if !home.is_empty() && f.starts_with(&home) {
                f.replacen(&home, "~", 1)
            } else {
                f.clone()
            };
            lines.push(format!("    {n:>6}  {display}"));
        }
    }

    lines.join("\n")
}

pub fn render_genmon(r: &VibeReport) -> String {
    let raw = render_text(r);
    let tip = raw
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    format!(
        "<txt><span foreground='#cba6f7'>󰚩 {}</span><span foreground='#6c7086'> · </span><span foreground='#89b4fa'>✍ {}</span></txt>\n<tool>{}</tool>\n<txtclick>xfce4-terminal --hold -x synapse vibe</txtclick>",
        r.vibe, r.hand, tip
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_counted() {
        assert!(is_counted("src/main.rs"));
        assert!(is_counted("/path/to/project/lib.py"));
        assert!(!is_counted("Cargo.lock"));
        assert!(!is_counted("package-lock.json"));
        assert!(!is_counted("app.min.js"));
        assert!(!is_counted("test.pcap"));
        assert!(!is_counted("output.log"));
    }

    #[test]
    fn test_added_lines() {
        let old = "line 1\nline 2\n";
        let new = "line 1\nline 1.5\nline 2\nline 3\n";
        assert_eq!(added_lines(old, new), 2);

        let replaced = "line 1\nline CHANGED\n";
        assert_eq!(added_lines(old, replaced), 1);
    }

    #[test]
    fn test_parse_iso_ts() {
        let ts = parse_iso_ts("2026-10-25T12:00:00Z");
        assert!(ts.is_some());
        let ts_sec = ts.unwrap();
        assert!(ts_sec > 1_700_000_000.0);
    }

    #[test]
    fn test_render_formats() {
        let mut by_source = HashMap::new();
        by_source.insert("antigravity".to_string(), 100);
        by_source.insert("claude".to_string(), 200);

        let report = VibeReport {
            hours: 24.0,
            vibe: 300,
            hand: 50,
            committed: 150,
            vibe_pct: 86,
            by_source,
            top_vibe_files: vec![("src/main.rs".to_string(), 120)],
        };

        let text = render_text(&report);
        assert!(text.contains("Last 24h"));
        assert!(text.contains("300"));
        assert!(text.contains("86%"));

        let genmon = render_genmon(&report);
        assert!(genmon.contains("<txt><span foreground='#cba6f7'>󰚩 300</span>"));
        assert!(genmon.contains("<tool>"));
    }
}
