use super::{
    compute_sha256, parse_and_validate_mcp_server, safe_join, strip_provenance, write_if_changed,
};
use crate::adapter::{Adapter, AdapterError, ImportResult, ProjectResult};
use crate::model::{Agent, McpServer, Skill};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

pub struct AgyCliAdapter;

struct AgyPaths {
    is_global: bool,
    memory_file: Option<PathBuf>,
    skills_dir: PathBuf,
    agents_dir: PathBuf,
    mcp_config: PathBuf,
}

impl AgyCliAdapter {
    fn resolve_paths(&self, root: &Path) -> AgyPaths {
        if root.ends_with(".gemini/config") {
            AgyPaths {
                is_global: true,
                memory_file: None,
                skills_dir: root.join("skills"),
                agents_dir: root.join("sidecars"),
                mcp_config: root.join("mcp_config.json"),
            }
        } else {
            let memory = if root.join("AGENTS.md").exists() {
                Some(root.join("AGENTS.md"))
            } else if root.join("GEMINI.md").exists() {
                Some(root.join("GEMINI.md"))
            } else {
                Some(root.join("AGENTS.md"))
            };
            AgyPaths {
                is_global: false,
                memory_file: memory,
                skills_dir: root.join(".agents/skills"),
                agents_dir: root.join(".agents/agents"),
                mcp_config: root.join(".agents/mcp_config.json"),
            }
        }
    }
}

impl Adapter for AgyCliAdapter {
    fn id(&self) -> &'static str {
        "agy-cli"
    }

    fn detect(&self, root: &Path) -> bool {
        root.join("AGENTS.md").exists()
            || root.join(".agents").is_dir()
            || root.ends_with(".gemini/config")
            || root.join(".gemini/config").is_dir()
    }

    fn import(&self, root: &Path) -> Result<ImportResult, AdapterError> {
        let mut skills = Vec::new();
        let mut agents = Vec::new();
        let mut mcp_servers = Vec::new();

        let paths = self.resolve_paths(root);

        // 1. Memory rules (if applicable)
        if let Some(path) = paths.memory_file.as_ref().filter(|p| {
            fs::symlink_metadata(p)
                .map(|m| m.file_type().is_file())
                .unwrap_or(false)
        }) {
            let raw = fs::read_to_string(path)
                .map_err(|e| AdapterError::Io(format!("Failed to read memory file: {}", e)))?;
            let content = strip_provenance(&raw);
            let sha256 = compute_sha256(&content);
            skills.push(Skill {
                id: "agy-cli-memory".to_string(),
                version: "1.0.0".to_string(),
                triggers: vec!["*".to_string()],
                targets: vec!["agy-cli".to_string()],
                source: content,
                sha256,
            });
        }

        // 2. Custom skills (do not follow symlinked directories or symlinked SKILL.md)
        if paths.skills_dir.is_dir() {
            let entries = fs::read_dir(&paths.skills_dir)
                .map_err(|e| AdapterError::Io(format!("Failed to read skills directory: {}", e)))?;

            for entry in entries {
                let entry = entry.map_err(|e| {
                    AdapterError::Io(format!("Failed to read skill directory entry: {}", e))
                })?;
                if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    continue;
                }
                let path = entry.path();
                let dir_name = path.file_name().unwrap().to_string_lossy().to_string();
                if dir_name == "agy-cli-memory"
                    || dir_name.ends_with("-rules")
                    || dir_name.starts_with("claude-")
                    || dir_name.starts_with("roo-mode-")
                {
                    continue;
                }
                let skill_md = path.join("SKILL.md");
                if fs::symlink_metadata(&skill_md)
                    .map(|m| m.file_type().is_file())
                    .unwrap_or(false)
                {
                    let raw = fs::read_to_string(&skill_md).map_err(|e| {
                        AdapterError::Io(format!("Failed to read {}: {}", skill_md.display(), e))
                    })?;
                    let content = strip_provenance(&raw);
                    let clean_slug = dir_name.trim_start_matches("agy-skill-");
                    let clean_slug = if clean_slug.is_empty() { "skill" } else { clean_slug };
                    let sha256 = compute_sha256(&content);
                    let id = if clean_slug == "obsidian-session-worklog" {
                        "obsidian-session-worklog".to_string()
                    } else {
                        format!("agy-skill-{}", clean_slug)
                    };
                    skills.push(Skill {
                        id,
                        version: "1.0.0".to_string(),
                        triggers: vec![format!("skill:{}", clean_slug)],
                        targets: vec!["agy-cli".to_string()],
                        source: content,
                        sha256,
                    });
                }
            }
        }

        // 3. Custom agents (do not follow symlinked agent files)
        if paths.agents_dir.is_dir() {
            let entries = fs::read_dir(&paths.agents_dir)
                .map_err(|e| AdapterError::Io(format!("Failed to read agents directory: {}", e)))?;

            for entry in entries {
                let entry = entry.map_err(|e| {
                    AdapterError::Io(format!("Failed to read agent directory entry: {}", e))
                })?;
                if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                    continue;
                }
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "md") {
                    let slug = path.file_stem().unwrap().to_string_lossy().to_string();
                    agents.push(Agent {
                        slug,
                        tools: vec!["*".to_string()],
                        model_hints: vec!["auto".to_string()],
                        targets: vec!["agy-cli".to_string()],
                    });
                }
            }
        }

        // 4. MCP Servers
        if paths.mcp_config.exists() {
            let raw_json = fs::read_to_string(&paths.mcp_config)
                .map_err(|e| AdapterError::Io(format!("Failed to read mcp_config.json: {}", e)))?;
            let parsed: Value = serde_json::from_str(&raw_json).map_err(|e| {
                AdapterError::Malformed(format!("Invalid JSON in mcp_config.json: {}", e))
            })?;

            if let Some(mcp_servers_val) = parsed.get("mcpServers").and_then(|v| v.as_object()) {
                for (id, val) in mcp_servers_val {
                    parse_and_validate_mcp_server(id, val, "agy-cli", &mut mcp_servers)?;
                }
            }
        }

        Ok(ImportResult {
            skills,
            agents,
            mcp_servers,
        })
    }

    fn project(
        &self,
        root: &Path,
        skills: &[Skill],
        agents: &[Agent],
        mcp_servers: &[McpServer],
    ) -> Result<ProjectResult, AdapterError> {
        let mut written = Vec::new();
        let paths = self.resolve_paths(root);

        let agy_skills: Vec<&Skill> = skills
            .iter()
            .filter(|s| s.targets.contains(&"agy-cli".to_string()))
            .collect();

        // Write memory if not global
        if !paths.is_global {
            if let Some(memory) = agy_skills.iter().find(|s| s.id == "agy-cli-memory") {
                if let Some(mem_path) = &paths.memory_file {
                    let output = format!(
                        "<!-- GENERATED BY Synapse — edit in the Brain -->\n{}",
                        memory.source
                    );
                    write_if_changed(mem_path, output).map_err(|e| {
                        AdapterError::Io(format!("Failed to write memory file: {}", e))
                    })?;

                    let rel_path = mem_path
                        .strip_prefix(root)
                        .unwrap_or(mem_path)
                        .to_string_lossy()
                        .to_string();
                    written.push(rel_path);
                }
            }
        }

        // Write custom skills
        for skill in agy_skills {
            if skill.id == "agy-cli-memory"
                || skill.id.ends_with("-rules")
                || skill.id.starts_with("claude-")
                || skill.id.starts_with("roo-mode-")
            {
                continue;
            }
            let slug = skill.id.trim_start_matches("agy-skill-");
            let slug = if slug.is_empty() { "skill" } else { slug };

            let skill_dir = safe_join(&paths.skills_dir, slug)?;
            fs::create_dir_all(&skill_dir).map_err(|e| {
                AdapterError::Io(format!(
                    "Failed to create skill directory {}: {}",
                    skill_dir.display(),
                    e
                ))
            })?;

            let skill_file = safe_join(&skill_dir, "SKILL.md")?;
            write_if_changed(&skill_file, &skill.source).map_err(|e| {
                AdapterError::Io(format!(
                    "Failed to write skill file {}: {}",
                    skill_file.display(),
                    e
                ))
            })?;

            let rel_path = skill_file
                .strip_prefix(root)
                .unwrap_or(&skill_file)
                .to_string_lossy()
                .to_string();
            written.push(rel_path);
        }

        // Write agents
        let agy_agents: Vec<&Agent> = agents
            .iter()
            .filter(|a| a.targets.contains(&"agy-cli".to_string()))
            .collect();

        if !agy_agents.is_empty() {
            fs::create_dir_all(&paths.agents_dir).map_err(|e| {
                AdapterError::Io(format!("Failed to create agents directory: {}", e))
            })?;

            for agent in agy_agents {
                let agent_filename = format!("{}.md", agent.slug);
                let agent_file = safe_join(&paths.agents_dir, &agent_filename)?;
                let content = format!(
                    "---\nname: {}\nmodel: {}\n---\n\nAgent definition projected from Brain.\n",
                    agent.slug,
                    agent.model_hints.first().map_or("auto", |s| s.as_str())
                );
                write_if_changed(&agent_file, content)
                    .map_err(|e| AdapterError::Io(format!("Failed to write agent file: {}", e)))?;

                let rel_path = agent_file
                    .strip_prefix(root)
                    .unwrap_or(&agent_file)
                    .to_string_lossy()
                    .to_string();
                written.push(rel_path);
            }
        }

        // Write MCP servers
        let agy_servers: Vec<&McpServer> = mcp_servers
            .iter()
            .filter(|s| s.targets.contains(&"agy-cli".to_string()))
            .collect();

        if !agy_servers.is_empty() {
            if let Some(parent) = paths.mcp_config.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    AdapterError::Io(format!("Failed to create mcp config directory: {}", e))
                })?;
            }

            let mut current_json = if paths.mcp_config.exists() {
                let raw_json = fs::read_to_string(&paths.mcp_config).map_err(|e| {
                    AdapterError::Io(format!("Failed to read mcp_config.json: {}", e))
                })?;
                serde_json::from_str(&raw_json).map_err(|e| {
                    AdapterError::Malformed(format!("Invalid JSON in mcp_config.json: {}", e))
                })?
            } else {
                json!({})
            };

            let mcp_servers_map = current_json
                .as_object_mut()
                .ok_or_else(|| {
                    AdapterError::Malformed("mcp_config.json root is not an object".to_string())
                })?
                .entry("mcpServers")
                .or_insert_with(|| json!({}))
                .as_object_mut()
                .ok_or_else(|| {
                    AdapterError::Malformed("mcpServers is not an object".to_string())
                })?;

            for server in agy_servers {
                let parts: Vec<&str> = server.command_or_url.split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }
                let command = parts[0].to_string();
                let args: Vec<Value> = parts[1..]
                    .iter()
                    .map(|s| Value::String(s.to_string()))
                    .collect();

                let mut env_obj = serde_json::Map::new();
                for key in &server.env_placeholders {
                    env_obj.insert(key.clone(), Value::String("".to_string()));
                }

                let new_server = json!({
                    "command": command,
                    "args": args,
                    "env": env_obj
                });

                mcp_servers_map.insert(server.id.clone(), new_server);
            }

            let pretty = serde_json::to_string_pretty(&current_json)
                .map_err(|e| AdapterError::Malformed(format!("Failed to serialize JSON: {}", e)))?;
            write_if_changed(&paths.mcp_config, pretty)
                .map_err(|e| AdapterError::Io(format!("Failed to write mcp_config.json: {}", e)))?;

            let rel_path = paths
                .mcp_config
                .strip_prefix(root)
                .unwrap_or(&paths.mcp_config)
                .to_string_lossy()
                .to_string();
            written.push(rel_path);
        }

        Ok(ProjectResult {
            written,
            symlinked: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_agy_cli_detect() {
        let dir = tempdir().unwrap();
        let adapter = AgyCliAdapter;
        assert!(!adapter.detect(dir.path()));

        fs::write(dir.path().join("AGENTS.md"), "# Agents").unwrap();
        assert!(adapter.detect(dir.path()));

        let dir2 = tempdir().unwrap();
        fs::create_dir_all(dir2.path().join(".agents")).unwrap();
        assert!(adapter.detect(dir2.path()));

        let dir3 = tempdir().unwrap();
        let global_path = dir3.path().join(".gemini/config");
        fs::create_dir_all(&global_path).unwrap();
        assert!(adapter.detect(&global_path));
    }

    #[test]
    fn test_agy_cli_import_project_roundtrip() {
        let dir = tempdir().unwrap();
        let adapter = AgyCliAdapter;

        fs::write(dir.path().join("AGENTS.md"), "Project agent memory").unwrap();
        let skill_dir = dir.path().join(".agents/skills/my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# My Skill Content").unwrap();

        let imported = adapter.import(dir.path()).unwrap();
        assert_eq!(imported.skills.len(), 2);
        assert!(imported.skills.iter().any(|s| s.id == "agy-cli-memory"));
        assert!(imported.skills.iter().any(|s| s.id == "agy-skill-my-skill"));

        let out_dir = tempdir().unwrap();
        let project_res = adapter
            .project(out_dir.path(), &imported.skills, &[], &[])
            .unwrap();

        assert_eq!(project_res.written.len(), 2);
        assert!(out_dir.path().join("AGENTS.md").exists());
        assert!(out_dir
            .path()
            .join(".agents/skills/my-skill/SKILL.md")
            .exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_agy_cli_import_blocks_symlinked_skills_and_agents() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        fs::write(outside.path().join("SKILL.md"), "outside secret skill").unwrap();
        fs::write(outside.path().join("evil-agent.md"), "outside secret agent").unwrap();

        let skills_dir = dir.path().join(".agents/skills");
        let agents_dir = dir.path().join(".agents/agents");
        fs::create_dir_all(&skills_dir).unwrap();
        fs::create_dir_all(&agents_dir).unwrap();

        // Plant symlinked skill dir and symlinked SKILL.md inside a real dir
        std::os::unix::fs::symlink(outside.path(), skills_dir.join("linked-dir")).unwrap();
        let real_skill_dir = skills_dir.join("real-dir");
        fs::create_dir_all(&real_skill_dir).unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("SKILL.md"),
            real_skill_dir.join("SKILL.md"),
        )
        .unwrap();

        // Plant symlinked agent file
        std::os::unix::fs::symlink(
            outside.path().join("evil-agent.md"),
            agents_dir.join("planted.md"),
        )
        .unwrap();

        let adapter = AgyCliAdapter;
        let imported = adapter.import(dir.path()).unwrap();
        assert!(imported.skills.is_empty());
        assert!(imported.agents.is_empty());
    }

    #[test]
    fn test_agy_cli_rejects_mcp_path_traversal() {
        let dir = tempdir().unwrap();
        let agents_dir = dir.path().join(".agents");
        fs::create_dir_all(&agents_dir).unwrap();
        fs::write(
            agents_dir.join("mcp_config.json"),
            r#"{"mcpServers": {"evil": {"command": "../../../bin/sh"}}}"#,
        )
        .unwrap();

        let adapter = AgyCliAdapter;
        assert!(matches!(
            adapter.import(dir.path()),
            Err(AdapterError::Malformed(_))
        ));
    }
}
