use super::{compute_sha256, safe_join, split_frontmatter, strip_provenance};
use crate::adapter::{Adapter, AdapterError, ImportResult, ProjectResult};
use crate::model::{Agent, HealthStatus, McpServer, Skill};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub struct ClaudeCodeAdapter;

const MEMORY_SKILL_ID: &str = "claude-code-memory";
const SKILL_ID_PREFIX: &str = "claude-skill-";
const AGENT_SKILL_ID_PREFIX: &str = "claude-agent-";

/// Parses the `tools:` list out of an agent frontmatter block. Claude Code's
/// real agent frontmatter has more fields (description, model, ...) but only
/// `tools` maps onto the canonical `Agent` model today.
fn parse_agent_tools(fm: &str) -> Vec<String> {
    let mut tools = Vec::new();
    let mut in_tools = false;
    for line in fm.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("tools:") {
            let rest = rest.trim();
            if rest.starts_with('[') && rest.ends_with(']') {
                tools = rest[1..rest.len() - 1]
                    .split(',')
                    .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                in_tools = false;
            } else if !rest.is_empty() {
                tools = rest
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                in_tools = false;
            } else {
                in_tools = true;
            }
        } else if in_tools {
            if let Some(item) = trimmed.strip_prefix('-') {
                tools.push(item.trim().to_string());
            } else if !trimmed.is_empty() {
                in_tools = false;
            }
        }
    }
    tools
}

fn serialize_agent_frontmatter(tools: &[String]) -> String {
    let mut fm = String::from("---\ntools:\n");
    for t in tools {
        fm.push_str(&format!("  - {}\n", t));
    }
    fm.push_str("---");
    fm
}

impl Adapter for ClaudeCodeAdapter {
    fn id(&self) -> &'static str {
        "claude-code"
    }

    fn detect(&self, root: &Path) -> bool {
        root.join("CLAUDE.md").exists()
            || root.join(".claude").is_dir()
            || root.join(".mcp.json").exists()
    }

    fn import(&self, root: &Path) -> Result<ImportResult, AdapterError> {
        let mut skills = Vec::new();
        let mut agents = Vec::new();
        let mut mcp_servers = Vec::new();

        let memory_path = root.join("CLAUDE.md");
        if memory_path.exists() {
            let raw = fs::read_to_string(&memory_path)
                .map_err(|e| AdapterError::Io(format!("Failed to read CLAUDE.md: {}", e)))?;
            let content = strip_provenance(&raw);
            let sha256 = compute_sha256(&content);
            skills.push(Skill {
                id: MEMORY_SKILL_ID.to_string(),
                version: "1.0.0".to_string(),
                triggers: vec!["*".to_string()],
                targets: vec!["claude-code".to_string()],
                source: content,
                sha256,
            });
        }

        let skills_dir = root.join(".claude/skills");
        if skills_dir.is_dir() {
            let mut entries: Vec<_> = fs::read_dir(&skills_dir)
                .map_err(|e| AdapterError::Io(format!("Failed to read .claude/skills: {}", e)))?
                .filter_map(|e| e.ok())
                // `entry.file_type()` reports the entry itself rather than
                // following it, unlike `path.is_dir()`; a symlinked
                // directory planted in a cloned repo must never be
                // descended into — per MASTER_PROMPT.md's "import never
                // follows symlinks outside scanned roots".
                .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
                .collect();
            entries.sort_by_key(|e| e.file_name());

            for entry in entries {
                let slug = entry.file_name().to_string_lossy().into_owned();
                let skill_md = entry.path().join("SKILL.md");
                if !skill_md.exists() {
                    continue;
                }
                let raw = fs::read_to_string(&skill_md).map_err(|e| {
                    AdapterError::Io(format!("Failed to read {}: {}", skill_md.display(), e))
                })?;
                let content = strip_provenance(&raw).trim().to_string();
                let sha256 = compute_sha256(&content);
                skills.push(Skill {
                    id: format!("{}{}", SKILL_ID_PREFIX, slug),
                    version: "1.0.0".to_string(),
                    triggers: vec!["*".to_string()],
                    targets: vec!["claude-code".to_string()],
                    source: content,
                    sha256,
                });
            }
        }

        let agents_dir = root.join(".claude/agents");
        if agents_dir.is_dir() {
            let mut entries: Vec<_> = fs::read_dir(&agents_dir)
                .map_err(|e| AdapterError::Io(format!("Failed to read .claude/agents: {}", e)))?
                .filter_map(|e| e.ok())
                .filter(|e| {
                    // See the .claude/skills guard above.
                    e.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                        && e.path().extension().and_then(|ext| ext.to_str()) == Some("md")
                })
                .collect();
            entries.sort_by_key(|e| e.file_name());

            for entry in entries {
                let path = entry.path();
                let slug = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("agent")
                    .to_string();
                let raw = fs::read_to_string(&path).map_err(|e| {
                    AdapterError::Io(format!("Failed to read {}: {}", path.display(), e))
                })?;
                let content = strip_provenance(&raw);
                let (fm_opt, body) = split_frontmatter(&content);
                let tools = fm_opt.as_deref().map(parse_agent_tools).unwrap_or_default();

                agents.push(Agent {
                    slug: slug.clone(),
                    tools,
                    model_hints: Vec::new(),
                    targets: vec!["claude-code".to_string()],
                });

                let body = body.trim().to_string();
                let sha256 = compute_sha256(&body);
                skills.push(Skill {
                    id: format!("{}{}", AGENT_SKILL_ID_PREFIX, slug),
                    version: "1.0.0".to_string(),
                    triggers: vec!["*".to_string()],
                    targets: vec!["claude-code".to_string()],
                    source: body,
                    sha256,
                });
            }
        }

        let mcp_path = root.join(".mcp.json");
        if mcp_path.exists() {
            let raw_json = fs::read_to_string(&mcp_path)
                .map_err(|e| AdapterError::Io(format!("Failed to read .mcp.json: {}", e)))?;
            let parsed: Value = serde_json::from_str(&raw_json).map_err(|e| {
                AdapterError::Malformed(format!("Invalid JSON in .mcp.json: {}", e))
            })?;

            if let Some(servers) = parsed.get("mcpServers").and_then(|v| v.as_object()) {
                for (id, val) in servers {
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
                        targets: vec!["claude-code".to_string()],
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

        let claude_skills: Vec<&Skill> = skills
            .iter()
            .filter(|s| s.targets.contains(&"claude-code".to_string()))
            .collect();

        if let Some(memory) = claude_skills.iter().find(|s| s.id == MEMORY_SKILL_ID) {
            let output = format!(
                "<!-- GENERATED BY LLM NEUROSURGEON — edit in the Brain -->\n{}",
                memory.source
            );
            fs::write(root.join("CLAUDE.md"), output)
                .map_err(|e| AdapterError::Io(format!("Failed to write CLAUDE.md: {}", e)))?;
            written.push("CLAUDE.md".to_string());
        }

        for skill in claude_skills
            .iter()
            .filter(|s| s.id.starts_with(SKILL_ID_PREFIX))
        {
            let slug = &skill.id[SKILL_ID_PREFIX.len()..];
            let rel_path = format!(".claude/skills/{}/SKILL.md", slug);
            let target_path = safe_join(root, &rel_path)?;
            if let Some(dir) = target_path.parent() {
                fs::create_dir_all(dir).map_err(|e| {
                    AdapterError::Io(format!("Failed to create {}: {}", dir.display(), e))
                })?;
            }
            let output = format!(
                "<!-- GENERATED BY LLM NEUROSURGEON — edit in the Brain -->\n{}",
                skill.source
            );
            fs::write(&target_path, output)
                .map_err(|e| AdapterError::Io(format!("Failed to write {}: {}", rel_path, e)))?;
            written.push(rel_path);
        }

        let claude_agents: Vec<&Agent> = agents
            .iter()
            .filter(|a| a.targets.contains(&"claude-code".to_string()))
            .collect();

        if !claude_agents.is_empty() {
            let agents_dir = root.join(".claude/agents");
            fs::create_dir_all(&agents_dir)
                .map_err(|e| AdapterError::Io(format!("Failed to create .claude/agents: {}", e)))?;

            for agent in claude_agents {
                let companion_id = format!("{}{}", AGENT_SKILL_ID_PREFIX, agent.slug);
                let body = claude_skills
                    .iter()
                    .find(|s| s.id == companion_id)
                    .map(|s| s.source.clone())
                    .unwrap_or_default();

                let output = format!(
                    "<!-- GENERATED BY LLM NEUROSURGEON — edit in the Brain -->\n{}\n\n{}",
                    serialize_agent_frontmatter(&agent.tools),
                    body
                );
                let rel_path = format!(".claude/agents/{}.md", agent.slug);
                let target_path = safe_join(root, &rel_path)?;
                fs::write(&target_path, output).map_err(|e| {
                    AdapterError::Io(format!("Failed to write {}: {}", rel_path, e))
                })?;
                written.push(rel_path);
            }
        }

        let claude_servers: Vec<&McpServer> = mcp_servers
            .iter()
            .filter(|s| s.targets.contains(&"claude-code".to_string()))
            .collect();

        if !claude_servers.is_empty() {
            let mcp_path = root.join(".mcp.json");
            let mut current_json = if mcp_path.exists() {
                let raw_json = fs::read_to_string(&mcp_path)
                    .map_err(|e| AdapterError::Io(format!("Failed to read .mcp.json: {}", e)))?;
                serde_json::from_str(&raw_json).map_err(|e| {
                    AdapterError::Malformed(format!("Invalid JSON in .mcp.json: {}", e))
                })?
            } else {
                json!({})
            };

            if !current_json.is_object() {
                return Err(AdapterError::Malformed(
                    ".mcp.json root is not an object".to_string(),
                ));
            }

            let servers_map = current_json
                .as_object_mut()
                .unwrap()
                .entry("mcpServers")
                .or_insert_with(|| json!({}))
                .as_object_mut()
                .ok_or_else(|| {
                    AdapterError::Malformed("mcpServers is not an object".to_string())
                })?;

            for server in claude_servers {
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
                if let Some(existing_server) = servers_map.get(&server.id) {
                    if let Some(existing_env) =
                        existing_server.get("env").and_then(|v| v.as_object())
                    {
                        env_obj = existing_env.clone();
                    }
                }
                for key in &server.env_placeholders {
                    if !env_obj.contains_key(key) {
                        env_obj.insert(key.clone(), Value::String("".to_string()));
                    }
                }

                servers_map.insert(
                    server.id.clone(),
                    json!({
                        "command": command,
                        "args": args,
                        "env": env_obj
                    }),
                );
            }

            let pretty = serde_json::to_string_pretty(&current_json)
                .map_err(|e| AdapterError::Malformed(format!("Failed to serialize JSON: {}", e)))?;
            fs::write(&mcp_path, pretty)
                .map_err(|e| AdapterError::Io(format!("Failed to write .mcp.json: {}", e)))?;
            written.push(".mcp.json".to_string());
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
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_claude_code_project_rejects_path_traversal_in_skill_slug() {
        let dir = tempdir().unwrap();
        let adapter = ClaudeCodeAdapter;

        let skill = Skill {
            id: format!("{}../../../evil", SKILL_ID_PREFIX),
            version: "1.0.0".to_string(),
            triggers: vec!["*".to_string()],
            targets: vec!["claude-code".to_string()],
            source: "malicious".to_string(),
            sha256: "hash".to_string(),
        };

        let result = adapter.project(dir.path(), &[skill], &[], &[]);
        assert!(matches!(result, Err(AdapterError::Malformed(_))));
    }

    #[test]
    fn test_claude_code_project_rejects_path_traversal_in_agent_slug() {
        let dir = tempdir().unwrap();
        let adapter = ClaudeCodeAdapter;

        let agent = Agent {
            slug: "../../../evil".to_string(),
            tools: vec![],
            model_hints: vec![],
            targets: vec!["claude-code".to_string()],
        };

        let result = adapter.project(dir.path(), &[], &[agent], &[]);
        assert!(matches!(result, Err(AdapterError::Malformed(_))));
    }

    #[test]
    fn test_claude_code_detect() {
        let dir = tempdir().unwrap();
        let adapter = ClaudeCodeAdapter;
        assert!(!adapter.detect(dir.path()));

        fs::write(dir.path().join("CLAUDE.md"), "test").unwrap();
        assert!(adapter.detect(dir.path()));
    }

    #[test]
    fn test_claude_code_import_export_roundtrip() {
        let dir = tempdir().unwrap();
        let adapter = ClaudeCodeAdapter;

        fs::write(dir.path().join("CLAUDE.md"), "Project instructions").unwrap();

        fs::create_dir_all(dir.path().join(".claude/skills/pdf-extractor")).unwrap();
        fs::write(
            dir.path().join(".claude/skills/pdf-extractor/SKILL.md"),
            "Extract text from PDFs.",
        )
        .unwrap();

        fs::create_dir_all(dir.path().join(".claude/agents")).unwrap();
        fs::write(
            dir.path().join(".claude/agents/reviewer.md"),
            "---\ntools:\n  - Read\n  - Grep\n---\nYou review code for bugs.",
        )
        .unwrap();

        fs::write(
            dir.path().join(".mcp.json"),
            r#"{"mcpServers": {"github": {"command": "npx", "args": ["-y", "@mcp/github"], "env": {"GITHUB_TOKEN": "x"}}}}"#,
        ).unwrap();

        let imported = adapter.import(dir.path()).unwrap();
        assert_eq!(imported.skills.len(), 3);
        assert!(imported
            .skills
            .iter()
            .any(|s| s.id == MEMORY_SKILL_ID && s.source == "Project instructions"));
        assert!(imported
            .skills
            .iter()
            .any(|s| s.id == "claude-skill-pdf-extractor"));
        assert!(imported
            .skills
            .iter()
            .any(|s| s.id == "claude-agent-reviewer" && s.source.contains("review code")));

        assert_eq!(imported.agents.len(), 1);
        assert_eq!(imported.agents[0].slug, "reviewer");
        assert_eq!(
            imported.agents[0].tools,
            vec!["Read".to_string(), "Grep".to_string()]
        );

        assert_eq!(imported.mcp_servers.len(), 1);
        assert_eq!(imported.mcp_servers[0].command_or_url, "npx -y @mcp/github");

        let out_dir = tempdir().unwrap();
        let project_res = adapter
            .project(
                out_dir.path(),
                &imported.skills,
                &imported.agents,
                &imported.mcp_servers,
            )
            .unwrap();
        assert_eq!(project_res.written.len(), 4);

        assert!(fs::read_to_string(out_dir.path().join("CLAUDE.md"))
            .unwrap()
            .contains("Project instructions"));
        assert!(
            fs::read_to_string(out_dir.path().join(".claude/skills/pdf-extractor/SKILL.md"))
                .unwrap()
                .contains("Extract text")
        );
        let projected_agent =
            fs::read_to_string(out_dir.path().join(".claude/agents/reviewer.md")).unwrap();
        assert!(projected_agent.contains("- Read"));
        assert!(projected_agent.contains("review code for bugs"));
        let projected_mcp = fs::read_to_string(out_dir.path().join(".mcp.json")).unwrap();
        let parsed: Value = serde_json::from_str(&projected_mcp).unwrap();
        assert_eq!(parsed["mcpServers"]["github"]["command"], "npx");
    }

    #[test]
    fn test_claude_code_stress_malformed_mcp() {
        let dir = tempdir().unwrap();
        let adapter = ClaudeCodeAdapter;
        fs::write(dir.path().join(".mcp.json"), "{ not json").unwrap();
        let res = adapter.import(dir.path());
        assert!(matches!(res, Err(AdapterError::Malformed(_))));
    }

    #[test]
    fn test_claude_code_stress_missing_skill_md_skipped() {
        let dir = tempdir().unwrap();
        let adapter = ClaudeCodeAdapter;
        fs::create_dir_all(dir.path().join(".claude/skills/empty-dir")).unwrap();
        let imported = adapter.import(dir.path()).unwrap();
        assert!(imported.skills.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn test_claude_code_import_does_not_follow_symlinked_skill_dir() {
        let dir = tempdir().unwrap();
        let adapter = ClaudeCodeAdapter;

        let outside = tempdir().unwrap();
        fs::write(outside.path().join("SKILL.md"), "top secret skill").unwrap();

        fs::create_dir_all(dir.path().join(".claude/skills")).unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join(".claude/skills/planted"))
            .unwrap();

        let imported = adapter.import(dir.path()).unwrap();
        assert!(imported.skills.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn test_claude_code_import_does_not_follow_symlinked_agent_files() {
        let dir = tempdir().unwrap();
        let adapter = ClaudeCodeAdapter;

        let outside = tempdir().unwrap();
        let secret = outside.path().join("secret.md");
        fs::write(&secret, "top secret agent").unwrap();

        fs::create_dir_all(dir.path().join(".claude/agents")).unwrap();
        std::os::unix::fs::symlink(&secret, dir.path().join(".claude/agents/planted.md")).unwrap();

        let imported = adapter.import(dir.path()).unwrap();
        assert!(imported.agents.is_empty());
    }
}
