//! Outcomes and actions of sync passes (import + project).

use crate::adapters::{all_adapters, safe_join, strip_provenance, write_if_changed};
use crate::conflict_queue::{ConflictQueue, QueuedConflict};
use crate::mappings::{Mapping, MappingsFile};
use crate::model::{Agent, McpServer, Skill};
use crate::snapshot;
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
            Ok(mut file) => {
                use std::io::Write;
                let _ = writeln!(file, "{}", std::process::id());
                let _ = file.sync_all();
                Ok(SyncLock {
                    path: lock_path,
                    acquired: true,
                })
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Err(format!(
                "lock file already exists at {}: another sync process may be running",
                lock_path.display()
            )),
            Err(e) => Err(format!("failed to acquire lock: {e}")),
        }
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

pub const SYNAPSE_MEMORY_SKILL_ID: &str = "synapse-memory";
pub const SYNAPSE_MEMORY_SKILL_CONTENT: &str = r#"---
name: synapse-memory
description: Built-in persistent cross-session memory for AI coding sessions — load context at session start, record durable decisions and milestones, recall by meaning, and distil. Replaces vault/worklog note-taking across all tools.
version: 1.0.0
author: SYNAPSE AI
license: MIT
metadata:
  synapse:
    tags: [Memory, Synapse, Semantic, Distillation, All-Tools]
---

# Synapse Memory Skill

The single source of durable memory for every AI coding session (Claude Code,
Cursor, Gemini CLI, Antigravity, Zed, Cline, Roo, Aider, Codex, OpenCode,
Continue, Copilot). It is local-first and zero-telemetry: nothing leaves the
machine. There is no Obsidian vault, Trilium, or external note store — memory
lives in `~/.synapse/memory/` and is reached only through `synapse mem`.

## When to use it

- **At session start** — load prior context for the current project:
  `synapse mem context` (the SessionStart hook does this automatically).
- **When something durable happens** — a decision, a fix, a non-obvious
  discovery, an architectural trade-off — record it:
  `synapse mem observe "<title>" -n "<detail>" -k <kind> --topic <area> --tag <kw> [--tag <kw>] --model <agent>`
- **Before relying on memory** — recall by meaning, not just keywords:
  `synapse mem search <query>` → `synapse mem timeline <id>` → `synapse mem get <id>…`

## How fragments are organised

Every observation carries, and can be filtered by:

- **project** — the working directory it belongs to (automatic; `--all` searches every project).
- **date/time** — assigned automatically; drives the timeline view.
- **topic** — a subject area you set (e.g. `networking`, `audio`, `synapse`, `pentest`). Filter with `--topic <area>`.
- **tags** — freeform keywords (`--tag`, repeatable).
- **model** — which agent recorded it (`--model`, or `$SYNAPSE_MODEL`). Filter with `--model <agent>`.
- **kind** — `decision | solution | insight | runbook | research | note | summary`.

## Semantic recall (nomic-embed-text)

Each fragment is embedded locally with `nomic-embed-text` via Ollama, so
`synapse mem search` blends keyword relevance with meaning — paraphrases and
related subjects surface even with no shared words. If Ollama is unavailable it
degrades silently to keyword search. `synapse mem reindex` embeds anything that
is missing a vector. Set `$SYNAPSE_EMBED_MODEL=off` to disable embeddings.

## Distillation (keep only genuinely valuable knowledge)

Memory must stay high-signal. Record distilled facts, not transcripts:

- Prefer one dense observation over several near-identical ones.
- Wrap anything sensitive in `<private>…</private>` — it never reaches disk.
- Periodically run `synapse mem distill` (dry-run) to find semantically
  near-duplicate fragments; `synapse mem distill --apply` collapses each cluster
  to its most informative member, shrinking the footprint without losing meaning.

## Auto-sync points

1. **Session initialization** — `synapse mem context` loads the project's memory index.
2. **Milestones & git operations** — record decisions, resolved anomalies, commit-worthy outcomes as observations with a topic and tags.
3. **Housekeeping** — distil before the store grows noisy; forget superseded facts with `synapse mem forget <id>`.
"#;

/// Ensures built-in canonical skills (e.g. `synapse-memory`) exist in `brain_root/skills/`.
pub fn ensure_default_skills(brain_root: &Path) -> Result<Vec<String>, String> {
    let mut provisioned = Vec::new();
    let skill_dir = brain_root.join("skills").join(SYNAPSE_MEMORY_SKILL_ID);
    let skill_md = skill_dir.join("SKILL.md");
    let skill_yaml = skill_dir.join("skill.yaml");

    if !skill_dir.exists() || !skill_md.exists() {
        fs::create_dir_all(&skill_dir)
            .map_err(|e| format!("Failed to create skill directory: {e}"))?;

        write_if_changed(&skill_md, SYNAPSE_MEMORY_SKILL_CONTENT)
            .map_err(|e| format!("Failed to write SKILL.md: {e}"))?;

        let sha256 = crate::adapters::compute_sha256(SYNAPSE_MEMORY_SKILL_CONTENT);
        let skill = Skill {
            id: SYNAPSE_MEMORY_SKILL_ID.to_string(),
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
            source: SYNAPSE_MEMORY_SKILL_CONTENT.to_string(),
            sha256,
        };

        let yaml_content = serde_json::to_string_pretty(&skill)
            .map_err(|e| format!("Failed to serialize skill.yaml: {e}"))?;
        write_if_changed(&skill_yaml, yaml_content)
            .map_err(|e| format!("Failed to write skill.yaml: {e}"))?;

        provisioned.push(format!("skills/{}/SKILL.md", SYNAPSE_MEMORY_SKILL_ID));
    }
    Ok(provisioned)
}

/// Imports detected tool configs under `root` into `brain_root` and commits a snapshot.
pub fn perform_import(root: &Path, brain_root: &Path) -> Result<Vec<String>, String> {
    snapshot::ensure_repo(brain_root).map_err(|e| format!("Failed to ensure brain repo: {e}"))?;

    let _ = ensure_default_skills(brain_root);

    let mut imported_paths = Vec::new();
    let mappings_path = brain_root.join(".brain/mappings.json");
    let mut mappings_file = MappingsFile::load(&mappings_path).unwrap_or_default();

    for adapter in all_adapters() {
        if !adapter.detect(root) {
            continue;
        }
        let import_res = adapter
            .import(root)
            .map_err(|e| format!("Adapter {} import failed: {e}", adapter.id()))?;

        for mut skill in import_res.skills {
            if skill.targets.is_empty() {
                skill.targets = all_adapters().iter().map(|a| a.id().to_string()).collect();
            }
            let rel_skill_dir = format!("skills/{}", skill.id);
            let skill_dir = safe_join(brain_root, &rel_skill_dir)
                .map_err(|e| format!("Invalid skill id '{}': {e}", skill.id))?;
            fs::create_dir_all(&skill_dir)
                .map_err(|e| format!("Failed to create skill dir: {e}"))?;

            let skill_file = skill_dir.join("SKILL.md");
            write_if_changed(&skill_file, &skill.source)
                .map_err(|e| format!("Failed to write skill file: {e}"))?;

            let skill_yaml = skill_dir.join("skill.yaml");
            let yaml_content = serde_json::to_string_pretty(&skill)
                .map_err(|e| format!("Failed to serialize skill.yaml: {e}"))?;
            write_if_changed(&skill_yaml, yaml_content)
                .map_err(|e| format!("Failed to write skill.yaml: {e}"))?;

            let rel_path = format!("skills/{}/SKILL.md", skill.id);
            imported_paths.push(rel_path.clone());

            let projection_path = match adapter.id() {
                "agy-cli" => {
                    if skill.id == "agy-cli-memory" {
                        "AGENTS.md".to_string()
                    } else {
                        let slug = skill.id.trim_start_matches("agy-skill-");
                        let slug = if slug.is_empty() { "skill" } else { slug };
                        format!(".agents/skills/{}/SKILL.md", slug)
                    }
                }
                "claude-code" => {
                    if skill.id == "claude-rules" {
                        "CLAUDE.md".to_string()
                    } else {
                        let slug = skill.id.trim_start_matches("claude-skill-");
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
                "continue" => format!(".continue/rules/{}.md", skill.id),
                _ => format!(".agents/skills/{}/SKILL.md", skill.id),
            };

            let mapping = Mapping {
                tool_id: adapter.id().to_string(),
                canonical_path: rel_path,
                projection_path,
                policy: crate::projector::ProjectionPolicy::Generate,
                content_sha256: skill.sha256,
            };
            if let Some(existing) = mappings_file.mappings.iter_mut().find(|m| {
                m.canonical_path == mapping.canonical_path && m.tool_id == mapping.tool_id
            }) {
                *existing = mapping;
            } else {
                mappings_file.mappings.push(mapping);
            }
        }

        for agent in import_res.agents {
            let agents_dir = brain_root.join("agents");
            fs::create_dir_all(&agents_dir)
                .map_err(|e| format!("Failed to create agents dir: {e}"))?;
            let rel_path = format!("agents/{}.md", agent.slug);
            let agent_file = safe_join(brain_root, &rel_path)
                .map_err(|e| format!("Invalid agent slug '{}': {e}", agent.slug))?;
            let content = format!(
                "---\nname: {}\nmodel: {}\n---\n\nImported agent definition.\n",
                agent.slug,
                agent.model_hints.first().map_or("auto", |s| s.as_str())
            );
            write_if_changed(&agent_file, content)
                .map_err(|e| format!("Failed to write agent file: {e}"))?;

            imported_paths.push(rel_path);
        }

        for server in import_res.mcp_servers {
            let mcp_dir = brain_root.join("mcp_servers");
            fs::create_dir_all(&mcp_dir)
                .map_err(|e| format!("Failed to create mcp_servers dir: {e}"))?;
            let rel_path = format!("mcp_servers/{}.json", server.id);
            let server_file = safe_join(brain_root, &rel_path)
                .map_err(|e| format!("Invalid mcp server id '{}': {e}", server.id))?;
            let json_str = serde_json::to_string_pretty(&server)
                .map_err(|e| format!("Failed to serialize mcp server: {e}"))?;
            write_if_changed(&server_file, json_str)
                .map_err(|e| format!("Failed to write mcp server file: {e}"))?;

            imported_paths.push(rel_path);
        }
    }

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

    let _ = mappings_file.save(&mappings_path);

    if !imported_paths.is_empty() {
        let _ = snapshot::snapshot(brain_root, "Import tool configs into Brain");
    }

    Ok(imported_paths)
}

/// Projects Brain content from `brain_root` out to `tool_root`.
pub fn perform_project(brain_root: &Path, tool_root: &Path) -> Result<Vec<String>, String> {
    let _ = ensure_default_skills(brain_root);

    let mut projected_paths = Vec::new();

    let mut skills = Vec::new();
    let skills_dir = brain_root.join("skills");
    if skills_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&skills_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let skill_yaml = path.join("skill.yaml");
                    let skill_md = path.join("SKILL.md");

                    if skill_yaml.exists() {
                        if let Ok(raw) = fs::read_to_string(&skill_yaml) {
                            if let Ok(mut skill) = serde_json::from_str::<Skill>(&raw) {
                                if skill.targets.is_empty() {
                                    skill.targets =
                                        all_adapters().iter().map(|a| a.id().to_string()).collect();
                                }
                                skills.push(skill);
                                continue;
                            }
                        }
                    }

                    if skill_md.exists() {
                        if let Ok(raw) = fs::read_to_string(&skill_md) {
                            let id = path.file_name().unwrap().to_string_lossy().to_string();
                            let sha256 = crate::adapters::compute_sha256(&raw);
                            skills.push(Skill {
                                id,
                                version: "1.0.0".to_string(),
                                triggers: vec!["*".to_string()],
                                targets: all_adapters()
                                    .iter()
                                    .map(|a| a.id().to_string())
                                    .collect(),
                                source: raw,
                                sha256,
                            });
                        }
                    }
                }
            }
        }
    }

    let mut agents = Vec::new();
    let agents_dir = brain_root.join("agents");
    if agents_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&agents_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                    let slug = path.file_stem().unwrap().to_string_lossy().to_string();
                    agents.push(Agent {
                        slug,
                        tools: vec!["*".to_string()],
                        model_hints: vec!["auto".to_string()],
                        targets: all_adapters().iter().map(|a| a.id().to_string()).collect(),
                    });
                }
            }
        }
    }

    let mut mcp_servers = Vec::new();
    let mcp_dir = brain_root.join("mcp_servers");
    if mcp_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&mcp_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                    if let Ok(raw) = fs::read_to_string(&path) {
                        if let Ok(mut server) = serde_json::from_str::<McpServer>(&raw) {
                            if server.targets.is_empty() {
                                server.targets =
                                    all_adapters().iter().map(|a| a.id().to_string()).collect();
                            }
                            mcp_servers.push(server);
                        }
                    }
                }
            }
        }
    }

    let mut had_error = false;
    let mut last_err = String::new();

    for adapter in all_adapters() {
        match adapter.project(tool_root, &skills, &agents, &mcp_servers) {
            Ok(res) => {
                projected_paths.extend(res.written);
            }
            Err(e) => {
                had_error = true;
                last_err = format!("Adapter {} project failed: {}", adapter.id(), e);
            }
        }
    }

    if had_error {
        return Err(last_err);
    }

    Ok(projected_paths)
}

/// Runs a full sync pass (import + 3-way merge conflict detection + project + snapshot).
pub fn perform_sync(brain_root: &Path, tool_root: &Path) -> Result<SyncOutcome, String> {
    snapshot::ensure_repo(brain_root).map_err(|e| format!("Failed to ensure brain repo: {e}"))?;

    // 1. Collect imported workspace skills
    let mut ws_skills = Vec::new();
    for adapter in all_adapters() {
        if adapter.detect(tool_root) {
            if let Ok(import_res) = adapter.import(tool_root) {
                for skill in import_res.skills {
                    ws_skills.push((adapter.id(), skill));
                }
            }
        }
    }

    // 2. Collect current brain skills
    let mut brain_skills = Vec::new();
    let skills_dir = brain_root.join("skills");
    if skills_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&skills_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let skill_yaml = path.join("skill.yaml");
                    let skill_md = path.join("SKILL.md");

                    if skill_yaml.exists() {
                        if let Ok(raw) = fs::read_to_string(&skill_yaml) {
                            if let Ok(mut skill) = serde_json::from_str::<Skill>(&raw) {
                                if skill.targets.is_empty() {
                                    skill.targets =
                                        all_adapters().iter().map(|a| a.id().to_string()).collect();
                                }
                                brain_skills.push(skill);
                                continue;
                            }
                        }
                    }

                    if skill_md.exists() {
                        if let Ok(raw) = fs::read_to_string(&skill_md) {
                            let id = path.file_name().unwrap().to_string_lossy().to_string();
                            let sha256 = crate::adapters::compute_sha256(&raw);
                            brain_skills.push(Skill {
                                id,
                                version: "1.0.0".to_string(),
                                triggers: vec!["*".to_string()],
                                targets: all_adapters()
                                    .iter()
                                    .map(|a| a.id().to_string())
                                    .collect(),
                                source: raw,
                                sha256,
                            });
                        }
                    }
                }
            }
        }
    }

    // 3. Detect 3-way merge conflicts between workspace and Brain
    let mut conflicts = Vec::new();
    for (_adapter_id, ws_skill) in &ws_skills {
        if let Some(brain_skill) = brain_skills.iter().find(|s| s.id == ws_skill.id) {
            let ws_source = strip_provenance(&ws_skill.source);
            let brain_source = strip_provenance(&brain_skill.source);

            if ws_source != brain_source {
                let base_content = get_git_file_content(
                    brain_root,
                    &format!("skills/{}/SKILL.md", brain_skill.id),
                )
                .or_else(|| {
                    get_git_file_content(
                        brain_root,
                        &format!("skills/{}/skill.yaml", brain_skill.id),
                    )
                    .and_then(|raw| serde_json::from_str::<Skill>(&raw).ok().map(|s| s.source))
                })
                .unwrap_or_default();

                let ws_sha = crate::adapters::compute_sha256(&ws_source);
                let brain_sha = crate::adapters::compute_sha256(&brain_source);
                let base_sha = crate::adapters::compute_sha256(&base_content);

                let both_changed = if !base_content.is_empty() {
                    ws_sha != base_sha && brain_sha != base_sha
                } else {
                    brain_skill.sha256 != brain_sha && brain_skill.sha256 != ws_sha
                };

                if both_changed {
                    match crate::merge::three_way_merge(&base_content, &brain_source, &ws_source) {
                        crate::merge::MergeOutcome::Clean(_) => {}
                        crate::merge::MergeOutcome::Conflict {
                            merged_with_markers,
                        } => {
                            conflicts.push(QueuedConflict {
                                id: brain_skill.id.clone(),
                                canonical_path: format!("skills/{}", brain_skill.id),
                                base: base_content,
                                local: brain_source,
                                remote: ws_source,
                                merged_with_markers,
                            });
                        }
                    }
                }
            }
        }
    }

    if !conflicts.is_empty() {
        let conflicts_path = brain_root.join("conflicts.json");
        let json = serde_json::to_string_pretty(&conflicts)
            .map_err(|e| format!("Failed to serialize conflicts: {e}"))?;
        write_if_changed(&conflicts_path, json)
            .map_err(|e| format!("Failed to write conflicts.json: {e}"))?;

        let queue = ConflictQueue {
            conflicts: conflicts.clone(),
        };
        let _ = queue.save(&brain_root.join(".brain/conflicts.json"));

        return Ok(SyncOutcome::ConflictQueued {
            conflict_ids: conflicts.into_iter().map(|c| c.id).collect(),
        });
    }

    // Clean up resolved conflicts file if no conflicts remain
    let conflicts_path = brain_root.join("conflicts.json");
    if conflicts_path.exists() {
        let _ = fs::remove_file(&conflicts_path);
    }
    let dot_brain_conflicts = brain_root.join(".brain/conflicts.json");
    if dot_brain_conflicts.exists() {
        let _ = fs::remove_file(&dot_brain_conflicts);
    }

    let imported = perform_import(tool_root, brain_root)?;
    let projected = perform_project(brain_root, tool_root)?;

    let mut changed_paths = imported;
    changed_paths.extend(projected);

    if changed_paths.is_empty() {
        Ok(SyncOutcome::NoChanges)
    } else {
        Ok(SyncOutcome::Applied { changed_paths })
    }
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
    fn test_default_synapse_memory_skill_auto_provisioned_and_projected() {
        let home = tempfile::tempdir().unwrap();
        let _home = crate::test_home::HomeGuard::set(home.path());
        let brain = tempfile::tempdir().unwrap();
        let tool_root = tempfile::tempdir().unwrap();

        // Projecting from an empty brain should auto-provision the synapse-memory skill
        let projected = perform_project(brain.path(), tool_root.path()).unwrap();
        assert!(!projected.is_empty());

        // Check brain has the skill
        let brain_skill = brain.path().join("skills/synapse-memory/SKILL.md");
        assert!(
            brain_skill.exists(),
            "Brain must contain synapse-memory SKILL.md"
        );
        let content = fs::read_to_string(&brain_skill).unwrap();
        assert!(content.contains("Synapse Memory Skill"));

        // Check projected to tool targets:
        // Claude Code
        let claude_skill = tool_root
            .path()
            .join(".claude/skills/synapse-memory/SKILL.md");
        assert!(
            claude_skill.exists(),
            "Claude must have synapse-memory skill projected"
        );

        // Cursor
        let cursor_rule = tool_root.path().join(".cursor/rules/synapse-memory.mdc");
        assert!(
            cursor_rule.exists(),
            "Cursor must have synapse-memory rule projected"
        );

        // AGY / Agents
        let agy_skill = tool_root
            .path()
            .join(".agents/skills/synapse-memory/SKILL.md");
        assert!(
            agy_skill.exists(),
            "AGY must have synapse-memory skill projected"
        );

        // Continue
        let continue_rule = tool_root.path().join(".continue/rules/synapse-memory.md");
        assert!(
            continue_rule.exists(),
            "Continue must have synapse-memory rule projected"
        );
    }
}
