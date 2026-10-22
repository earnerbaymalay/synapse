//! Outcomes and actions of sync passes (import + project).

use crate::adapters::{all_adapters, safe_join, strip_provenance, write_if_changed};
use crate::conflict_queue::{ConflictQueue, QueuedConflict};
use crate::mappings::{Mapping, MappingsFile};
use crate::model::{Agent, McpServer, Skill};
use crate::snapshot;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncOutcome {
    /// Nothing changed on either side of the Brain.
    NoChanges,
    /// Import and/or projection applied cleanly.
    Applied { changed_paths: Vec<String> },
    /// Both sides changed the same region since the last snapshot; queued
    /// for human review rather than auto-resolved.
    ConflictQueued { conflict_ids: Vec<String> },
}

impl SyncOutcome {
    pub fn is_clean(&self) -> bool {
        matches!(self, SyncOutcome::NoChanges | SyncOutcome::Applied { .. })
    }
}

/// A crash-safe file lock on `brain_root/.lock` to prevent concurrent sync operations.
pub struct SyncLock {
    path: PathBuf,
    acquired: bool,
}

impl SyncLock {
    pub fn acquire(brain_root: &Path) -> Result<Self, String> {
        let lock_path = brain_root.join(".lock");
        if let Err(e) = fs::create_dir_all(brain_root) {
            return Err(format!("failed to create brain directory: {e}"));
        }

        if let Ok(meta) = fs::symlink_metadata(&lock_path) {
            if meta.file_type().is_symlink() {
                return Err(format!(
                    "refusing to acquire symlinked lock file at {}",
                    lock_path.display()
                ));
            }
        }

        let mut opts = fs::OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }

        match opts.open(&lock_path) {
            Ok(file) => Ok(Self::hold(file, lock_path)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // A lock left behind by a crashed sync would otherwise block
                // every future run. Reclaim it only when it names a PID that
                // is provably gone; an empty file means a live holder is
                // between create and write, so it is never stolen.
                let holder = fs::read_to_string(&lock_path)
                    .ok()
                    .and_then(|s| s.trim().parse::<u32>().ok());
                if let Some(pid) = holder {
                    if !pid_alive(pid) && fs::remove_file(&lock_path).is_ok() {
                        if let Ok(file) = opts.open(&lock_path) {
                            return Ok(Self::hold(file, lock_path));
                        }
                    }
                }
                Err(format!(
                    "lock file already exists at {}: another sync process may be running",
                    lock_path.display()
                ))
            }
            Err(e) => Err(format!("failed to acquire lock: {e}")),
        }
    }

    fn hold(mut file: fs::File, path: PathBuf) -> Self {
        use std::io::Write;
        let _ = writeln!(file, "{}", std::process::id());
        let _ = file.sync_all();
        SyncLock {
            path,
            acquired: true,
        }
    }
}

/// Whether `pid` names a running process. Errs on the side of "alive" where
/// liveness can't be checked, so a lock is never reclaimed by mistake.
fn pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(true)
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        true
    }
}

impl Drop for SyncLock {
    fn drop(&mut self) {
        if self.acquired {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn get_git_file_content(brain_root: &Path, rel_path: &str) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["show", &format!("HEAD:{rel_path}")])
        .current_dir(brain_root)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}

pub const OBSIDIAN_WORKLOG_SKILL_ID: &str = "obsidian-session-worklog";
pub const OBSIDIAN_WORKLOG_SKILL_CONTENT: &str = r#"---
name: obsidian-session-worklog
description: Inbuilt Obsidian integration skill for AI coding sessions — manages vault notes, session worklogs, and project checklists automatically across all tools.
version: 1.0.0
author: SYNAPSE AI
license: MIT
metadata:
  synapse:
    tags: [Obsidian, Vault, Worklog, Synapse, All-Tools]
---

# Obsidian Session Worklog Skill

This skill provides automatic and on-demand integration between AI coding sessions (Claude Code, Cursor, Gemini CLI, Windsurf, Zed, Antigravity, Cline, Roo, Aider, Codex, OpenCode, Continue, Copilot) and your local Obsidian vault.

## Vault Location & Discovery

1. Check the `OBSIDIAN_VAULT_PATH` environment variable.
2. Fallback location: `~/Documents/My-Vault` (or `/home/$USER/Documents/My-Vault`).
3. If neither path exists, prompt the user for their active Obsidian vault directory.

## Standard Structure

- **AI Session Worklogs**: `${VAULT}/Worklog/AGY Sessions/YYYY-MM-DD - AI Session - ${CONVERSATION_ID}.md`
- **Project Notes**: `${VAULT}/Projects/${PROJECT_NAME}/Repository To-Do.md`

## Note Template (AI Session Worklog)

```markdown
---
type: ai-session-worklog
date: YYYY-MM-DD
session_id: "CONVERSATION_ID"
status: active | completed | partial | blocked
tags:
  - ai/session
  - worklog
---

# AI Session Worklog — ${TITLE}

## Objective
High-level summary of the session goal.

## Work Completed
- [x] Concrete task or milestone achieved.

## Verification
- Verified build and test results.

## Decisions & Trade-offs
- Architectural or implementation choices made during session.

## Follow-ups & Next Steps
- [ ] Task for future work.
```

## Workflows & Auto-Sync Points

1. **Session Initialization**: Automatically discover and load active Obsidian vault notes and project backlog checklists at the start of each session.
2. **Milestones & Git Operations**: Record major milestones, commit hashes, test pass metrics, and architectural decisions into the active session worklog.
3. **Project Checklist Sync**: Reconcile `${VAULT}/Projects/${PROJECT_NAME}/Repository To-Do.md` with git commit state and completed backlog items.
"#;

/// Ensures built-in canonical skills (e.g. `obsidian-session-worklog`) exist in `brain_root/skills/`.
pub fn ensure_default_skills(brain_root: &Path) -> Result<Vec<String>, String> {
    let mut provisioned = Vec::new();
    let skill_dir = brain_root.join("skills").join(OBSIDIAN_WORKLOG_SKILL_ID);
    let skill_md = skill_dir.join("SKILL.md");
    let skill_yaml = skill_dir.join("skill.yaml");

    if !skill_dir.exists() || !skill_md.exists() {
        fs::create_dir_all(&skill_dir)
            .map_err(|e| format!("Failed to create skill directory: {e}"))?;

        write_if_changed(&skill_md, OBSIDIAN_WORKLOG_SKILL_CONTENT)
            .map_err(|e| format!("Failed to write SKILL.md: {e}"))?;

        let sha256 = crate::adapters::compute_sha256(OBSIDIAN_WORKLOG_SKILL_CONTENT);
        let skill = Skill {
            id: OBSIDIAN_WORKLOG_SKILL_ID.to_string(),
            version: "1.0.0".to_string(),
            triggers: vec![
                "*".to_string(),
                "session-init".to_string(),
                "milestone".to_string(),
            ],
            targets: vec![
                "claude-code".to_string(),
                "cursor".to_string(),
                "agy-cli".to_string(),
                "continue".to_string(),
            ],
            source: OBSIDIAN_WORKLOG_SKILL_CONTENT.to_string(),
            sha256,
        };

        let yaml_content = serde_json::to_string_pretty(&skill)
            .map_err(|e| format!("Failed to serialize skill.yaml: {e}"))?;
        write_if_changed(&skill_yaml, yaml_content)
            .map_err(|e| format!("Failed to write skill.yaml: {e}"))?;

        provisioned.push(format!("skills/{}/SKILL.md", OBSIDIAN_WORKLOG_SKILL_ID));
    }
    Ok(provisioned)
}

const PROVENANCE_HEADER: &str = "<!-- GENERATED BY Synapse — edit in the Brain -->";

fn all_targets() -> Vec<String> {
    all_adapters().iter().map(|a| a.id().to_string()).collect()
}

/// Whether `source` was written by a Synapse projection rather than by hand.
fn is_projection(source: &str) -> bool {
    crate::security::strip_invisible_unicode(source)
        .trim_start()
        .starts_with(PROVENANCE_HEADER)
}

/// Loads the Brain's skills. `SKILL.md` is the source of truth (it is the
/// file users are told to edit); `skill.yaml` only contributes metadata
/// (version, triggers, targets).
pub fn load_brain_skills(brain_root: &Path) -> Vec<Skill> {
    let mut skills = Vec::new();
    let Ok(entries) = fs::read_dir(brain_root.join("skills")) else {
        return skills;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().to_string();
        let meta = fs::read_to_string(path.join("skill.yaml"))
            .ok()
            .and_then(|raw| serde_json::from_str::<Skill>(&raw).ok());
        let source = match fs::read_to_string(path.join("SKILL.md")) {
            Ok(raw) => raw,
            Err(_) => match &meta {
                Some(m) => m.source.clone(),
                None => continue,
            },
        };
        let mut skill = meta.unwrap_or_else(|| Skill {
            id: id.clone(),
            version: "1.0.0".to_string(),
            triggers: vec!["*".to_string()],
            targets: Vec::new(),
            source: String::new(),
            sha256: String::new(),
        });
        skill.id = id;
        skill.sha256 = crate::adapters::compute_sha256(&source);
        skill.source = source;
        if skill.targets.is_empty() {
            skill.targets = all_targets();
        }
        skills.push(skill);
    }
    skills.sort_by(|a, b| a.id.cmp(&b.id));
    skills
}

/// Serializes an agent into its Brain file. List fields are stored as JSON
/// arrays so every field survives a round trip.
fn agent_file_content(agent: &Agent) -> String {
    let list = |v: &[String]| serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string());
    format!(
        "---\nname: {}\ntools: {}\nmodel_hints: {}\ntargets: {}\n---\n",
        agent.slug,
        list(&agent.tools),
        list(&agent.model_hints),
        list(&agent.targets)
    )
}

/// Parses a Brain agent file. Files written before tools were persisted
/// carry only `model:`; they load with no explicit tools (the tool's own
/// default) rather than being widened to `*`.
fn parse_agent_file(slug: &str, raw: &str) -> Agent {
    let mut agent = Agent {
        slug: slug.to_string(),
        tools: Vec::new(),
        model_hints: Vec::new(),
        targets: Vec::new(),
    };
    let list = |v: &str| serde_json::from_str::<Vec<String>>(v.trim()).unwrap_or_default();
    for line in raw.lines() {
        if let Some(v) = line.strip_prefix("tools:") {
            agent.tools = list(v);
        } else if let Some(v) = line.strip_prefix("model_hints:") {
            agent.model_hints = list(v);
        } else if let Some(v) = line.strip_prefix("targets:") {
            agent.targets = list(v);
        } else if let Some(v) = line.strip_prefix("model:") {
            let v = v.trim();
            if !v.is_empty() && v != "auto" && agent.model_hints.is_empty() {
                agent.model_hints = vec![v.to_string()];
            }
        }
    }
    if agent.targets.is_empty() {
        agent.targets = all_targets();
    }
    agent
}

pub fn load_brain_agents(brain_root: &Path) -> Vec<Agent> {
    let mut agents = Vec::new();
    let Ok(entries) = fs::read_dir(brain_root.join("agents")) else {
        return agents;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
            let slug = path.file_stem().unwrap().to_string_lossy().to_string();
            if let Ok(raw) = fs::read_to_string(&path) {
                agents.push(parse_agent_file(&slug, &raw));
            }
        }
    }
    agents.sort_by(|a, b| a.slug.cmp(&b.slug));
    agents
}

pub fn load_brain_mcp_servers(brain_root: &Path) -> Vec<McpServer> {
    let mut servers = Vec::new();
    let Ok(entries) = fs::read_dir(brain_root.join("mcp_servers")) else {
        return servers;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
            if let Some(mut server) = fs::read_to_string(&path)
                .ok()
                .and_then(|raw| serde_json::from_str::<McpServer>(&raw).ok())
            {
                if server.targets.is_empty() {
                    server.targets = all_targets();
                }
                servers.push(server);
            }
        }
    }
    servers.sort_by(|a, b| a.id.cmp(&b.id));
    servers
}

/// Where a skill imported by `adapter_id` is projected back to, relative to the tool root.
fn projection_path_for(adapter_id: &str, skill_id: &str, root: &Path) -> String {
    match adapter_id {
        "agy-cli" => {
            if skill_id == "agy-cli-memory" {
                "AGENTS.md".to_string()
            } else {
                let slug = skill_id.trim_start_matches("agy-skill-");
                let slug = if slug.is_empty() { "skill" } else { slug };
                format!(".agents/skills/{}/SKILL.md", slug)
            }
        }
        "claude-code" => {
            if let Some(agent) = skill_id.strip_prefix("claude-agent-") {
                format!(".claude/agents/{agent}.md")
            } else if skill_id == "claude-rules" {
                "CLAUDE.md".to_string()
            } else {
                let slug = skill_id.trim_start_matches("claude-skill-");
                let slug = if slug.is_empty() { "skill" } else { slug };
                format!(".claude/skills/{}/SKILL.md", slug)
            }
        }
        "gemini-cli" => "GEMINI.md".to_string(),
        "zed" => {
            if root.join(".config/zed/AGENTS.md").exists() {
                ".config/zed/AGENTS.md".to_string()
            } else {
                ".rules".to_string()
            }
        }
        "opencode" => {
            if root.join("AGENTS.md").exists() {
                "AGENTS.md".to_string()
            } else {
                ".roomodes".to_string()
            }
        }
        "roo-code" => ".roomodes".to_string(),
        "openai-codex" => "AGENTS.md".to_string(),
        "continue" => format!(".continue/rules/{}.md", skill_id),
        _ => format!(".agents/skills/{}/SKILL.md", skill_id),
    }
}

/// Everything detected under a tool root, tagged with the adapter that read it.
#[derive(Default)]
struct Detected {
    skills: Vec<(&'static str, Skill)>,
    agents: Vec<(&'static str, Agent)>,
    mcp_servers: Vec<(&'static str, McpServer)>,
}

fn detect_all(root: &Path) -> Result<Detected, String> {
    let mut found = Detected::default();
    for adapter in all_adapters() {
        if !adapter.detect(root) {
            continue;
        }
        let res = adapter
            .import(root)
            .map_err(|e| format!("Adapter {} import failed: {e}", adapter.id()))?;
        let id = adapter.id();
        found.skills.extend(res.skills.into_iter().map(|s| (id, s)));
        found.agents.extend(res.agents.into_iter().map(|a| (id, a)));
        found
            .mcp_servers
            .extend(res.mcp_servers.into_iter().map(|m| (id, m)));
    }
    Ok(found)
}

/// Writes the given items into the Brain. Returns the Brain-relative paths
/// whose content actually changed.
fn write_into_brain(
    root: &Path,
    brain_root: &Path,
    items: &Detected,
    mappings_file: &mut MappingsFile,
) -> Result<Vec<String>, String> {
    let mut changed = Vec::new();
    let all_ids = all_targets();

    for (adapter_id, skill) in &items.skills {
        let mut skill = skill.clone();
        skill.source = strip_provenance(&skill.source);
        skill.sha256 = crate::adapters::compute_sha256(&skill.source);
        if skill.targets.is_empty() {
            skill.targets = all_ids.clone();
        }
        let rel_skill_dir = format!("skills/{}", skill.id);
        let skill_dir = safe_join(brain_root, &rel_skill_dir)
            .map_err(|e| format!("Invalid skill id '{}': {e}", skill.id))?;
        fs::create_dir_all(&skill_dir).map_err(|e| format!("Failed to create skill dir: {e}"))?;

        let md_changed = write_if_changed(&skill_dir.join("SKILL.md"), &skill.source)
            .map_err(|e| format!("Failed to write skill file: {e}"))?;
        let yaml_content = serde_json::to_string_pretty(&skill)
            .map_err(|e| format!("Failed to serialize skill.yaml: {e}"))?;
        let yaml_changed = write_if_changed(&skill_dir.join("skill.yaml"), yaml_content)
            .map_err(|e| format!("Failed to write skill.yaml: {e}"))?;

        let rel_path = format!("skills/{}/SKILL.md", skill.id);
        if md_changed || yaml_changed {
            changed.push(rel_path.clone());
        }

        let mapping = Mapping {
            tool_id: adapter_id.to_string(),
            canonical_path: rel_path,
            projection_path: projection_path_for(adapter_id, &skill.id, root),
            policy: crate::projector::ProjectionPolicy::Generate,
            content_sha256: skill.sha256,
        };
        if let Some(existing) = mappings_file
            .mappings
            .iter_mut()
            .find(|m| m.canonical_path == mapping.canonical_path && m.tool_id == mapping.tool_id)
        {
            *existing = mapping;
        } else {
            mappings_file.mappings.push(mapping);
        }
    }

    for (_, agent) in &items.agents {
        let mut agent = agent.clone();
        if agent.targets.is_empty() {
            agent.targets = all_ids.clone();
        }
        fs::create_dir_all(brain_root.join("agents"))
            .map_err(|e| format!("Failed to create agents dir: {e}"))?;
        let rel_path = format!("agents/{}.md", agent.slug);
        let agent_file = safe_join(brain_root, &rel_path)
            .map_err(|e| format!("Invalid agent slug '{}': {e}", agent.slug))?;
        if write_if_changed(&agent_file, agent_file_content(&agent))
            .map_err(|e| format!("Failed to write agent file: {e}"))?
        {
            changed.push(rel_path);
        }
    }

    for (_, server) in &items.mcp_servers {
        fs::create_dir_all(brain_root.join("mcp_servers"))
            .map_err(|e| format!("Failed to create mcp_servers dir: {e}"))?;
        let rel_path = format!("mcp_servers/{}.json", server.id);
        let server_file = safe_join(brain_root, &rel_path)
            .map_err(|e| format!("Invalid mcp server id '{}': {e}", server.id))?;
        let json_str = serde_json::to_string_pretty(server)
            .map_err(|e| format!("Failed to serialize mcp server: {e}"))?;
        if write_if_changed(&server_file, json_str)
            .map_err(|e| format!("Failed to write mcp server file: {e}"))?
        {
            changed.push(rel_path);
        }
    }

    Ok(changed)
}

fn save_mappings(mappings_file: &mut MappingsFile, path: &Path) -> Result<(), String> {
    mappings_file.mappings.sort_by(|a, b| {
        (&a.tool_id, &a.canonical_path, &a.projection_path).cmp(&(
            &b.tool_id,
            &b.canonical_path,
            &b.projection_path,
        ))
    });
    mappings_file
        .mappings
        .dedup_by(|a, b| a.tool_id == b.tool_id && a.canonical_path == b.canonical_path);
    mappings_file
        .save(path)
        .map_err(|e| format!("Failed to save mappings: {e}"))
}

/// Imports detected tool configs under `root` into `brain_root` and commits a snapshot.
///
/// Files that carry the Synapse provenance header are projections of the
/// Brain; they only update a skill the Brain already has and never mint a
/// new one (otherwise every sync re-imports its own output under new ids).
pub fn perform_import(root: &Path, brain_root: &Path) -> Result<Vec<String>, String> {
    snapshot::ensure_repo(brain_root).map_err(|e| format!("Failed to ensure brain repo: {e}"))?;
    ensure_default_skills(brain_root)?;

    let brain_ids: HashSet<String> = load_brain_skills(brain_root)
        .into_iter()
        .map(|s| s.id)
        .collect();
    let mut detected = detect_all(root)?;
    detected
        .skills
        .retain(|(_, s)| !is_projection(&s.source) || brain_ids.contains(&s.id));

    let mappings_path = brain_root.join(".brain/mappings.json");
    let mut mappings_file = MappingsFile::load(&mappings_path).unwrap_or_default();
    let changed = write_into_brain(root, brain_root, &detected, &mut mappings_file)?;
    save_mappings(&mut mappings_file, &mappings_path)?;

    if !changed.is_empty() {
        snapshot::snapshot(brain_root, "Import tool configs into Brain")
            .map_err(|e| format!("Failed to snapshot Brain: {e}"))?;
    }
    Ok(changed)
}

fn project_items(
    tool_root: &Path,
    skills: &[Skill],
    agents: &[Agent],
    mcp_servers: &[McpServer],
) -> Result<Vec<String>, String> {
    let mut projected_paths = Vec::new();
    let mut last_err = None;
    for adapter in all_adapters() {
        match adapter.project(tool_root, skills, agents, mcp_servers) {
            Ok(res) => projected_paths.extend(res.written),
            Err(e) => last_err = Some(format!("Adapter {} project failed: {}", adapter.id(), e)),
        }
    }
    match last_err {
        Some(e) => Err(e),
        None => Ok(projected_paths),
    }
}

/// Projects Brain content from `brain_root` out to `tool_root`.
pub fn perform_project(brain_root: &Path, tool_root: &Path) -> Result<Vec<String>, String> {
    ensure_default_skills(brain_root)?;
    let written = project_items(
        tool_root,
        &load_brain_skills(brain_root),
        &load_brain_agents(brain_root),
        &load_brain_mcp_servers(brain_root),
    )?;
    record_projected_hashes(brain_root, tool_root)?;
    Ok(written)
}

/// Drift detection compares a projection file's hash with its mapping, so
/// the mapping must record what was actually written (provenance header,
/// tool-specific wrapping, shared-file aggregation) rather than the hash of
/// the Brain source — otherwise `doctor` flags every fresh projection as
/// hand-edited.
fn record_projected_hashes(brain_root: &Path, tool_root: &Path) -> Result<(), String> {
    let mappings_path = brain_root.join(".brain/mappings.json");
    if !mappings_path.exists() {
        return Ok(());
    }
    let mut mappings_file = MappingsFile::load(&mappings_path).unwrap_or_default();
    let mut dirty = false;
    for mapping in &mut mappings_file.mappings {
        if mapping.policy != crate::projector::ProjectionPolicy::Generate {
            continue;
        }
        let Ok(path) = safe_join(tool_root, &mapping.projection_path) else {
            continue;
        };
        if let Ok(content) = fs::read_to_string(path) {
            let sha = crate::adapters::compute_sha256(&content);
            if mapping.content_sha256 != sha {
                mapping.content_sha256 = sha;
                dirty = true;
            }
        }
    }
    if dirty {
        save_mappings(&mut mappings_file, &mappings_path)?;
    }
    Ok(())
}

/// A scratch directory removed on drop.
struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new() -> Result<Self, String> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("synapse-echo-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create scratch dir: {e}"))?;
        Ok(ScratchDir(dir))
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// What a pure projection of the Brain looks like once read back through
/// the adapters. Anything in a real tool root that matches this exactly was
/// produced by Synapse, not edited by the user — which is how sync tells
/// derived files (shared `AGENTS.md`, `.roomodes`, ...) apart from edits
/// without needing a header in every file format.
struct Echo {
    skills: HashMap<String, String>,
    agents: Vec<Agent>,
    mcp_servers: Vec<McpServer>,
    paths: Vec<String>,
}

fn echo_projection(
    skills: &[Skill],
    agents: &[Agent],
    mcp_servers: &[McpServer],
) -> Result<Echo, String> {
    let scratch = ScratchDir::new()?;
    let paths = project_items(&scratch.0, skills, agents, mcp_servers)?;
    let read_back = detect_all(&scratch.0)?;
    Ok(Echo {
        skills: read_back
            .skills
            .into_iter()
            .map(|(_, s)| (s.id, strip_provenance(&s.source)))
            .collect(),
        agents: read_back.agents.into_iter().map(|(_, a)| a).collect(),
        mcp_servers: read_back.mcp_servers.into_iter().map(|(_, m)| m).collect(),
        paths,
    })
}

fn same_agent(a: &Agent, b: &Agent) -> bool {
    a.slug == b.slug && a.tools == b.tools
}

fn same_server(a: &McpServer, b: &McpServer) -> bool {
    let mut a = a.clone();
    let mut b = b.clone();
    a.targets.clear();
    b.targets.clear();
    a == b
}

/// Runs a full sync pass: decides per item which side changed since the
/// last snapshot, imports tool-side edits, keeps Brain-side edits, merges
/// edits made on both sides, then projects the Brain back out and commits.
///
/// | tool side vs. last snapshot | Brain vs. last snapshot | result                      |
/// |-----------------------------|-------------------------|-----------------------------|
/// | unchanged                   | changed                 | Brain kept, projected out   |
/// | changed                     | unchanged               | tool edit imported          |
/// | changed                     | changed                 | 3-way merge; conflicts queue |
///
/// Conflicting items are left untouched on both sides and projection is
/// held back until they are resolved, so nothing is overwritten.
pub fn perform_sync(brain_root: &Path, tool_root: &Path) -> Result<SyncOutcome, String> {
    snapshot::ensure_repo(brain_root).map_err(|e| format!("Failed to ensure brain repo: {e}"))?;
    ensure_default_skills(brain_root)?;

    let brain_skills = load_brain_skills(brain_root);
    let brain_agents = load_brain_agents(brain_root);
    let brain_servers = load_brain_mcp_servers(brain_root);
    let echo = echo_projection(&brain_skills, &brain_agents, &brain_servers)?;
    let detected = detect_all(tool_root)?;

    let mut to_import = Detected::default();
    let mut merged: Vec<(String, String)> = Vec::new();
    let mut conflicts = Vec::new();
    let mut seen = HashSet::new();

    for (adapter_id, ws_skill) in detected.skills {
        if !seen.insert(ws_skill.id.clone()) {
            continue;
        }
        let ws_source = strip_provenance(&ws_skill.source);
        if echo.skills.get(&ws_skill.id) == Some(&ws_source) {
            continue; // our own projection, untouched
        }
        let Some(brain_skill) = brain_skills.iter().find(|s| s.id == ws_skill.id) else {
            if !is_projection(&ws_skill.source) {
                to_import.skills.push((adapter_id, ws_skill));
            }
            continue;
        };
        let brain_source = strip_provenance(&brain_skill.source);
        if ws_source == brain_source {
            continue;
        }
        let base = get_git_file_content(brain_root, &format!("skills/{}/SKILL.md", ws_skill.id))
            .map(|b| strip_provenance(&b))
            .unwrap_or_default();

        if brain_source == base {
            to_import.skills.push((adapter_id, ws_skill));
        } else if ws_source == base {
            // Brain-only edit: keep it; projection below carries it out.
        } else {
            match crate::merge::three_way_merge(&base, &brain_source, &ws_source) {
                crate::merge::MergeOutcome::Clean(text) => merged.push((ws_skill.id.clone(), text)),
                crate::merge::MergeOutcome::Conflict {
                    merged_with_markers,
                } => conflicts.push(QueuedConflict {
                    id: ws_skill.id.clone(),
                    canonical_path: format!("skills/{}", ws_skill.id),
                    base,
                    local: brain_source,
                    remote: ws_source,
                    merged_with_markers,
                }),
            }
        }
    }

    for (adapter_id, agent) in detected.agents {
        if echo.agents.iter().any(|e| same_agent(e, &agent)) {
            continue;
        }
        if brain_agents.iter().any(|b| same_agent(b, &agent)) {
            continue;
        }
        to_import.agents.push((adapter_id, agent));
    }

    for (adapter_id, server) in detected.mcp_servers {
        if echo.mcp_servers.iter().any(|e| same_server(e, &server))
            || brain_servers.iter().any(|b| same_server(b, &server))
        {
            continue;
        }
        to_import.mcp_servers.push((adapter_id, server));
    }

    let mappings_path = brain_root.join(".brain/mappings.json");
    let mut mappings_file = MappingsFile::load(&mappings_path).unwrap_or_default();
    let mut changed = write_into_brain(tool_root, brain_root, &to_import, &mut mappings_file)?;
    save_mappings(&mut mappings_file, &mappings_path)?;

    for (id, text) in &merged {
        let skill_dir = safe_join(brain_root, &format!("skills/{id}"))
            .map_err(|e| format!("Invalid skill id '{id}': {e}"))?;
        if write_if_changed(&skill_dir.join("SKILL.md"), text)
            .map_err(|e| format!("Failed to write merged skill: {e}"))?
        {
            changed.push(format!("skills/{id}/SKILL.md"));
        }
    }

    let conflicts_path = brain_root.join(".brain/conflicts.json");
    if !conflicts.is_empty() {
        ConflictQueue {
            conflicts: conflicts.clone(),
        }
        .save(&conflicts_path)
        .map_err(|e| format!("Failed to write conflicts: {e}"))?;
        // Human-readable copy at the Brain root (gitignored).
        let json = serde_json::to_string_pretty(&conflicts)
            .map_err(|e| format!("Failed to serialize conflicts: {e}"))?;
        write_if_changed(&brain_root.join("conflicts.json"), json)
            .map_err(|e| format!("Failed to write conflicts.json: {e}"))?;
        if !changed.is_empty() {
            snapshot::snapshot(brain_root, "Sync: import non-conflicting changes")
                .map_err(|e| format!("Failed to snapshot Brain: {e}"))?;
        }
        return Ok(SyncOutcome::ConflictQueued {
            conflict_ids: conflicts.into_iter().map(|c| c.id).collect(),
        });
    }
    let _ = fs::remove_file(&conflicts_path);
    let _ = fs::remove_file(brain_root.join("conflicts.json"));

    // Project the (possibly updated) Brain, reporting only files whose
    // content actually changed so a quiescent sync reports NoChanges.
    let before: HashMap<String, Option<Vec<u8>>> = echo
        .paths
        .iter()
        .map(|p| (p.clone(), fs::read(tool_root.join(p)).ok()))
        .collect();
    let projected = perform_project(brain_root, tool_root)?;
    for path in projected {
        let now = fs::read(tool_root.join(&path)).ok();
        let was = before.get(&path).cloned().flatten();
        if now != was && !changed.contains(&path) {
            changed.push(path);
        }
    }

    if changed.is_empty() {
        return Ok(SyncOutcome::NoChanges);
    }
    snapshot::snapshot(brain_root, "Sync").map_err(|e| format!("Failed to snapshot Brain: {e}"))?;
    Ok(SyncOutcome::Applied {
        changed_paths: changed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_changes_is_clean() {
        assert!(SyncOutcome::NoChanges.is_clean());
    }

    #[test]
    fn conflict_queued_is_not_clean() {
        let outcome = SyncOutcome::ConflictQueued {
            conflict_ids: vec!["skills/repo-conventions".into()],
        };
        assert!(!outcome.is_clean());
    }

    #[test]
    fn perform_import_and_project_roundtrip() {
        let home = tempfile::tempdir().unwrap();
        let _home = crate::test_home::HomeGuard::set(home.path());
        let root = tempfile::tempdir().unwrap();
        let brain = tempfile::tempdir().unwrap();
        let tool_root = tempfile::tempdir().unwrap();

        fs::write(root.path().join("AGENTS.md"), "Project agent memory").unwrap();

        let imported = perform_import(root.path(), brain.path()).unwrap();
        assert!(!imported.is_empty());
        assert!(brain.path().join("skills/agy-cli-memory/SKILL.md").exists());
        assert!(brain
            .path()
            .join("skills/agy-cli-memory/skill.yaml")
            .exists());

        let projected = perform_project(brain.path(), tool_root.path()).unwrap();
        assert!(!projected.is_empty());
        assert!(tool_root.path().join("AGENTS.md").exists());
    }

    #[test]
    fn test_default_obsidian_skill_auto_provisioned_and_projected() {
        let home = tempfile::tempdir().unwrap();
        let _home = crate::test_home::HomeGuard::set(home.path());
        let brain = tempfile::tempdir().unwrap();
        let tool_root = tempfile::tempdir().unwrap();

        // Projecting from an empty brain should auto-provision the obsidian-session-worklog skill
        let projected = perform_project(brain.path(), tool_root.path()).unwrap();
        assert!(!projected.is_empty());

        // Check brain has the skill
        let brain_skill = brain
            .path()
            .join("skills/obsidian-session-worklog/SKILL.md");
        assert!(
            brain_skill.exists(),
            "Brain must contain obsidian-session-worklog SKILL.md"
        );
        let content = fs::read_to_string(&brain_skill).unwrap();
        assert!(content.contains("Obsidian Session Worklog"));

        // Check projected to tool targets:
        // Claude Code
        let claude_skill = tool_root
            .path()
            .join(".claude/skills/obsidian-session-worklog/SKILL.md");
        assert!(
            claude_skill.exists(),
            "Claude must have obsidian skill projected"
        );

        // Cursor
        let cursor_rule = tool_root
            .path()
            .join(".cursor/rules/obsidian-session-worklog.mdc");
        assert!(
            cursor_rule.exists(),
            "Cursor must have obsidian rule projected"
        );

        // AGY / Agents
        let agy_skill = tool_root
            .path()
            .join(".agents/skills/obsidian-session-worklog/SKILL.md");
        assert!(agy_skill.exists(), "AGY must have obsidian skill projected");

        // Continue
        let continue_rule = tool_root
            .path()
            .join(".continue/rules/obsidian-session-worklog.md");
        assert!(
            continue_rule.exists(),
            "Continue must have obsidian rule projected"
        );
    }
    struct Env {
        _home: crate::test_home::HomeGuard,
        _dirs: Vec<tempfile::TempDir>,
        brain: PathBuf,
        ws: PathBuf,
    }

    fn env() -> Env {
        let home = tempfile::tempdir().unwrap();
        let guard = crate::test_home::HomeGuard::set(home.path());
        let brain = tempfile::tempdir().unwrap();
        let ws = tempfile::tempdir().unwrap();
        Env {
            _home: guard,
            brain: brain.path().to_path_buf(),
            ws: ws.path().to_path_buf(),
            _dirs: vec![home, brain, ws],
        }
    }

    fn brain_rules(e: &Env) -> String {
        fs::read_to_string(e.brain.join("skills/claude-rules/SKILL.md")).unwrap()
    }

    fn skill_ids(e: &Env) -> Vec<String> {
        load_brain_skills(&e.brain)
            .into_iter()
            .map(|s| s.id)
            .collect()
    }

    #[test]
    fn brain_only_edit_survives_sync_and_is_projected() {
        let e = env();
        fs::write(e.ws.join("CLAUDE.md"), "line1\nline2\nline3\n").unwrap();
        perform_sync(&e.brain, &e.ws).unwrap();

        fs::write(e.brain.join("skills/claude-rules/SKILL.md"), "BRAIN EDIT\n").unwrap();
        perform_sync(&e.brain, &e.ws).unwrap();

        assert_eq!(brain_rules(&e), "BRAIN EDIT\n");
        let projected = fs::read_to_string(e.ws.join("CLAUDE.md")).unwrap();
        assert_eq!(strip_provenance(&projected), "BRAIN EDIT\n");
    }

    #[test]
    fn tool_only_edit_is_imported() {
        let e = env();
        fs::write(e.ws.join("CLAUDE.md"), "line1\nline2\nline3\n").unwrap();
        perform_sync(&e.brain, &e.ws).unwrap();

        fs::write(e.ws.join("CLAUDE.md"), "line1\nTOOL EDIT\nline3\n").unwrap();
        perform_sync(&e.brain, &e.ws).unwrap();
        assert_eq!(brain_rules(&e), "line1\nTOOL EDIT\nline3\n");
    }

    #[test]
    fn clean_three_way_merge_keeps_both_sides() {
        let e = env();
        fs::write(e.ws.join("CLAUDE.md"), "a\nb\nc\nd\ne\n").unwrap();
        perform_sync(&e.brain, &e.ws).unwrap();

        fs::write(
            e.brain.join("skills/claude-rules/SKILL.md"),
            "a\nB\nc\nd\ne\n",
        )
        .unwrap();
        fs::write(e.ws.join("CLAUDE.md"), "a\nb\nc\nD\ne\n").unwrap();
        let outcome = perform_sync(&e.brain, &e.ws).unwrap();

        assert!(outcome.is_clean(), "{outcome:?}");
        assert_eq!(brain_rules(&e), "a\nB\nc\nD\ne\n");
        let projected = fs::read_to_string(e.ws.join("CLAUDE.md")).unwrap();
        assert_eq!(strip_provenance(&projected), "a\nB\nc\nD\ne\n");
    }

    #[test]
    fn conflicting_edits_are_queued_and_neither_side_is_overwritten() {
        let e = env();
        fs::write(e.ws.join("CLAUDE.md"), "a\nb\nc\n").unwrap();
        perform_sync(&e.brain, &e.ws).unwrap();

        fs::write(
            e.brain.join("skills/claude-rules/SKILL.md"),
            "a\nBRAIN\nc\n",
        )
        .unwrap();
        fs::write(e.ws.join("CLAUDE.md"), "a\nTOOL\nc\n").unwrap();
        let outcome = perform_sync(&e.brain, &e.ws).unwrap();

        assert_eq!(
            outcome,
            SyncOutcome::ConflictQueued {
                conflict_ids: vec!["claude-rules".into()]
            }
        );
        assert_eq!(brain_rules(&e), "a\nBRAIN\nc\n");
        assert_eq!(
            fs::read_to_string(e.ws.join("CLAUDE.md")).unwrap(),
            "a\nTOOL\nc\n"
        );
        assert!(e.brain.join(".brain/conflicts.json").exists());
    }

    #[test]
    fn repeated_sync_reaches_a_fixed_point() {
        let e = env();
        fs::create_dir_all(e.ws.join(".claude/agents")).unwrap();
        fs::create_dir_all(e.ws.join(".claude/skills/pdf")).unwrap();
        fs::write(e.ws.join("CLAUDE.md"), "line1\nline2\n").unwrap();
        fs::write(
            e.ws.join(".claude/agents/rev.md"),
            "---\ntools: Read, Grep\n---\nYou review.\n",
        )
        .unwrap();
        fs::write(
            e.ws.join(".claude/skills/pdf/SKILL.md"),
            "---\nname: pdf\n---\nDo pdf.\n",
        )
        .unwrap();

        perform_sync(&e.brain, &e.ws).unwrap();
        let first = skill_ids(&e);
        perform_sync(&e.brain, &e.ws).unwrap();
        let third = perform_sync(&e.brain, &e.ws).unwrap();

        assert_eq!(
            skill_ids(&e),
            first,
            "sync must not mint new skills from its own output"
        );
        assert_eq!(third, SyncOutcome::NoChanges);
    }

    #[test]
    fn agent_tools_survive_a_round_trip() {
        let e = env();
        fs::create_dir_all(e.ws.join(".claude/agents")).unwrap();
        fs::write(
            e.ws.join(".claude/agents/rev.md"),
            "---\ntools: Read, Grep\n---\nYou review.\n",
        )
        .unwrap();

        perform_sync(&e.brain, &e.ws).unwrap();
        perform_sync(&e.brain, &e.ws).unwrap();

        let agents = load_brain_agents(&e.brain);
        let rev = agents.iter().find(|a| a.slug == "rev").unwrap();
        assert_eq!(rev.tools, vec!["Read".to_string(), "Grep".to_string()]);
        let projected = fs::read_to_string(e.ws.join(".claude/agents/rev.md")).unwrap();
        assert!(
            !projected.contains("- *"),
            "tools must not widen to *: {projected}"
        );
        assert!(projected.contains("Read") && projected.contains("Grep"));
    }

    #[test]
    fn legacy_agent_file_does_not_widen_tools() {
        let agent = parse_agent_file(
            "old",
            "---\nname: old\nmodel: opus\n---\n\nImported agent definition.\n",
        );
        assert!(agent.tools.is_empty());
        assert_eq!(agent.model_hints, vec!["opus".to_string()]);
        let back = parse_agent_file("old", &agent_file_content(&agent));
        assert_eq!(back, agent);
    }

    #[test]
    fn skill_md_edit_wins_over_stale_skill_yaml() {
        let e = env();
        fs::write(e.ws.join("CLAUDE.md"), "original\n").unwrap();
        perform_sync(&e.brain, &e.ws).unwrap();
        fs::write(e.brain.join("skills/claude-rules/SKILL.md"), "edited\n").unwrap();

        let skill = load_brain_skills(&e.brain)
            .into_iter()
            .find(|s| s.id == "claude-rules")
            .unwrap();
        assert_eq!(skill.source, "edited\n");
    }

    #[test]
    fn sync_lock_is_never_committed() {
        let e = env();
        fs::write(e.ws.join("CLAUDE.md"), "x\n").unwrap();
        let _lock = SyncLock::acquire(&e.brain).unwrap();
        perform_sync(&e.brain, &e.ws).unwrap();
        let tracked = std::process::Command::new("git")
            .args(["ls-files"])
            .current_dir(&e.brain)
            .output()
            .unwrap();
        let tracked = String::from_utf8_lossy(&tracked.stdout);
        assert!(!tracked.lines().any(|l| l == ".lock"), "{tracked}");
    }

    #[test]
    fn stale_lock_from_dead_process_is_reclaimed() {
        let brain = tempfile::tempdir().unwrap();
        // PIDs this high are not handed out on Linux/macOS defaults.
        fs::write(brain.path().join(".lock"), "4194303\n").unwrap();
        assert!(SyncLock::acquire(brain.path()).is_ok());
    }

    #[test]
    fn live_lock_is_respected() {
        let brain = tempfile::tempdir().unwrap();
        let _held = SyncLock::acquire(brain.path()).unwrap();
        assert!(SyncLock::acquire(brain.path()).is_err());
    }
}
