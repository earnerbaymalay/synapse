use super::{compute_sha256, safe_join, strip_provenance};
use crate::adapter::{Adapter, AdapterError, ImportResult, ProjectResult};
use crate::model::{Agent, HealthStatus, McpServer, Skill};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub struct AgyCliAdapter;

impl Adapter for AgyCliAdapter {
    fn id(&self) -> &'static str {
        "agy-cli"
    }

    fn detect(&self, root: &Path) -> bool {
        root.join("AGENTS.md").exists()
            || root.join(".agents").is_dir()
            || root.join(".gemini/config").is_dir()
    }

    fn import(&self, root: &Path) -> Result<ImportResult, AdapterError> {
        let mut skills = Vec::new();
        let mut agents = Vec::new();
        let mut mcp_servers = Vec::new();

        // 1. Memory rules from AGENTS.md (or GEMINI.md as fallback)
        let memory_path = if root.join("AGENTS.md").exists() {
            Some(root.join("AGENTS.md"))
        } else if root.join("GEMINI.md").exists() {
            Some(root.join("GEMINI.md"))
        } else {
            None
        };

        if let Some(path) = memory_path {
            let raw = fs::read_to_string(&path)
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

        // 2. Custom skills from .agents/skills/<name>/SKILL.md
        let skills_dir = root.join(".agents/skills");
        if skills_dir.is_dir() {
            let entries = fs::read_dir(&skills_dir).map_err(|e| {
                AdapterError::Io(format!("Failed to read .agents/skills directory: {}", e))
            })?;

            for entry in entries {
                let entry = entry.map_err(|e| {
                    AdapterError::Io(format!("Failed to read skill directory entry: {}", e))
                })?;
                let path = entry.path();
                if path.is_dir() {
                    let skill_md = path.join("SKILL.md");
                    if skill_md.exists() {
                        let raw = fs::read_to_string(&skill_md).map_err(|e| {
                            AdapterError::Io(format!("Failed to read {}: {}", skill_md.display(), e))
                        })?;
                        let dir_name = path.file_name().unwrap().to_string_lossy().to_string();
                        let sha256 = compute_sha256(&raw);
                        skills.push(Skill {
                            id: format!("agy-skill-{}", dir_name),
                            version: "1.0.0".to_string(),
                            triggers: vec![format!("skill:{}", dir_name)],
                            targets: vec!["agy-cli".to_string()],
                            source: raw,
                            sha256,
                        });
                    }
                }
            }
        }

        // 3. Custom agents from .agents/agents/<slug>.md
        let agents_dir = root.join(".agents/agents");
        if agents_dir.is_dir() {
            let entries = fs::read_dir(&agents_dir).map_err(|e| {
                AdapterError::Io(format!("Failed to read .agents/agents directory: {}", e))
            })?;

            for entry in entries {
                let entry = entry.map_err(|e| {
                    AdapterError::Io(format!("Failed to read agent directory entry: {}", e))
                })?;
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
                    let slug = path
                        .file_stem()
                        .unwrap()
                        .to_string_lossy()
                        .to_string();
                    agents.push(Agent {
                        slug,
                        tools: vec!["*".to_string()],
                        model_hints: vec!["auto".to_string()],
                        targets: vec!["agy-cli".to_string()],
                    });
                }
            }
        }

        // 4. MCP Servers from .agents/mcp_config.json
        let mcp_config_path = root.join(".agents/mcp_config.json");
        if mcp_config_path.exists() {
            let raw_json = fs::read_to_string(&mcp_config_path).map_err(|e| {
                AdapterError::Io(format!("Failed to read .agents/mcp_config.json: {}", e))
            })?;
            let parsed: Value = serde_json::from_str(&raw_json).map_err(|e| {
                AdapterError::Malformed(format!("Invalid JSON in .agents/mcp_config.json: {}", e))
            })?;

            if let Some(mcp_servers_val) = parsed.get("mcpServers").and_then(|v| v.as_object()) {
                for (id, val) in mcp_servers_val {
                    let command = val
                        .get("command")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let args: Vec<String> = val
                        .get("args")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .map(|item| item.as_str().unwrap_or("").to_string())
                                .collect()
                        })
                        .unwrap_or_default();

                    let command_or_url = if args.is_empty() {
                        command
                    } else {
                        format!("{} {}", command, args.join(" "))
                    };

                    let mut env_placeholders = Vec::new();
                    if let Some(env_obj) = val.get("env").and_then(|v| v.as_object()) {
                        for key in env_obj.keys() {
                            env_placeholders.push(key.clone());
                        }
                    }
                    env_placeholders.sort();

                    mcp_servers.push(McpServer {
                        id: id.clone(),
                        transport: "stdio".to_string(),
                        command_or_url,
                        env_placeholders,
                        targets: vec!["agy-cli".to_string()],
                        health: HealthStatus::Unknown,
                    });
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

        let agy_skills: Vec<&Skill> = skills
            .iter()
            .filter(|s| s.targets.contains(&"agy-cli".to_string()))
            .collect();

        // Write AGENTS.md if memory skill exists
        if let Some(memory) = agy_skills.iter().find(|s| s.id == "agy-cli-memory") {
            let output = format!(
                "<!-- GENERATED BY LLM NEUROSURGEON — edit in the Brain -->\n{}",
                memory.source
            );
            fs::write(root.join("AGENTS.md"), output)
                .map_err(|e| AdapterError::Io(format!("Failed to write AGENTS.md: {}", e)))?;
            written.push("AGENTS.md".to_string());
        }

        // Write custom skills to .agents/skills/<slug>/SKILL.md
        for skill in agy_skills {
            if skill.id == "agy-cli-memory" {
                continue;
            }
            let slug = skill
                .id
                .strip_prefix("agy-skill-")
                .unwrap_or(&skill.id);

            let skill_dir_rel = format!(".agents/skills/{slug}");
            let skill_dir = safe_join(root, &skill_dir_rel)?;
            fs::create_dir_all(&skill_dir).map_err(|e| {
                AdapterError::Io(format!(
                    "Failed to create skill directory {}: {}",
                    skill_dir.display(),
                    e
                ))
            })?;

            let skill_file_rel = format!(".agents/skills/{slug}/SKILL.md");
            let skill_file = safe_join(root, &skill_file_rel)?;
            fs::write(&skill_file, &skill.source).map_err(|e| {
                AdapterError::Io(format!(
                    "Failed to write skill file {}: {}",
                    skill_file.display(),
                    e
                ))
            })?;
            written.push(skill_file_rel);
        }

        // Write agents to .agents/agents/<slug>.md
        let agy_agents: Vec<&Agent> = agents
            .iter()
            .filter(|a| a.targets.contains(&"agy-cli".to_string()))
            .collect();

        if !agy_agents.is_empty() {
            let agents_dir = root.join(".agents/agents");
            fs::create_dir_all(&agents_dir).map_err(|e| {
                AdapterError::Io(format!("Failed to create .agents/agents: {}", e))
            })?;

            for agent in agy_agents {
                let agent_file_rel = format!(".agents/agents/{}.md", agent.slug);
                let agent_file = safe_join(root, &agent_file_rel)?;
                let content = format!(
                    "---\nname: {}\nmodel: {}\n---\n\nAgent definition projected from Brain.\n",
                    agent.slug,
                    agent.model_hints.first().map_or("auto", |s| s.as_str())
                );
                fs::write(&agent_file, content).map_err(|e| {
                    AdapterError::Io(format!("Failed to write agent file: {}", e))
                })?;
                written.push(agent_file_rel);
            }
        }

        // Write MCP servers to .agents/mcp_config.json
        let agy_servers: Vec<&McpServer> = mcp_servers
            .iter()
            .filter(|s| s.targets.contains(&"agy-cli".to_string()))
            .collect();

        if !agy_servers.is_empty() {
            let agents_dir = root.join(".agents");
            fs::create_dir_all(&agents_dir).map_err(|e| {
                AdapterError::Io(format!("Failed to create .agents directory: {}", e))
            })?;

            let mcp_path = agents_dir.join("mcp_config.json");
            let mut current_json = if mcp_path.exists() {
                let raw_json = fs::read_to_string(&mcp_path).map_err(|e| {
                    AdapterError::Io(format!("Failed to read .agents/mcp_config.json: {}", e))
                })?;
                serde_json::from_str(&raw_json).map_err(|e| {
                    AdapterError::Malformed(format!("Invalid JSON in .agents/mcp_config.json: {}", e))
                })?
            } else {
                json!({})
            };

            let mcp_servers_map = current_json
                .as_object_mut()
                .ok_or_else(|| {
                    AdapterError::Malformed(".agents/mcp_config.json root is not an object".to_string())
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
            fs::write(&mcp_path, pretty).map_err(|e| {
                AdapterError::Io(format!("Failed to write .agents/mcp_config.json: {}", e))
            })?;
            written.push(".agents/mcp_config.json".to_string());
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
        assert!(out_dir.path().join(".agents/skills/my-skill/SKILL.md").exists());
    }
}
