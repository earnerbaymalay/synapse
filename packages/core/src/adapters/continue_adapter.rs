use super::{
    compute_sha256, parse_mdc_frontmatter, safe_join, serialize_mdc_frontmatter, split_frontmatter,
    strip_provenance, MdcFrontmatter,
};
use crate::adapter::{Adapter, AdapterError, ImportResult, ProjectResult};
use crate::model::{Agent, HealthStatus, McpServer, Skill};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub struct ContinueAdapter;

const RULE_ID_PREFIX: &str = "continue-rule-";

impl Adapter for ContinueAdapter {
    fn id(&self) -> &'static str {
        "continue"
    }

    fn detect(&self, root: &Path) -> bool {
        root.join(".continue/config.json").exists() || root.join(".continue/rules").is_dir()
    }

    fn import(&self, root: &Path) -> Result<ImportResult, AdapterError> {
        let mut skills = Vec::new();
        let mut mcp_servers = Vec::new();

        let config_path = root.join(".continue/config.json");
        if config_path.exists() {
            let raw_json = fs::read_to_string(&config_path).map_err(|e| {
                AdapterError::Io(format!("Failed to read .continue/config.json: {}", e))
            })?;
            let parsed: Value = serde_json::from_str(&raw_json).map_err(|e| {
                AdapterError::Malformed(format!("Invalid JSON in .continue/config.json: {}", e))
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
                        targets: vec!["continue".to_string()],
                        health: HealthStatus::Unknown,
                    });
                }
            }
        }

        let rules_dir = root.join(".continue/rules");
        if rules_dir.is_dir() {
            let mut entries: Vec<_> = fs::read_dir(&rules_dir)
                .map_err(|e| AdapterError::Io(format!("Failed to read .continue/rules: {}", e)))?
                .filter_map(|e| e.ok())
                .filter(|e| {
                    // See cursor.rs's identical guard: `file_type()` does
                    // not follow symlinks, unlike `path.is_dir()`/`is_file()`.
                    e.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                        && e.path().extension().and_then(|ext| ext.to_str()) == Some("md")
                })
                .collect();
            entries.sort_by_key(|e| e.file_name());

            for entry in entries {
                let path = entry.path();
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("rule")
                    .to_string();
                let raw = fs::read_to_string(&path).map_err(|e| {
                    AdapterError::Io(format!("Failed to read {}: {}", path.display(), e))
                })?;
                let content = strip_provenance(&raw);
                let (fm_opt, body) = split_frontmatter(&content);
                let fm = fm_opt
                    .map(|fm| parse_mdc_frontmatter(&fm))
                    .unwrap_or_default();

                let triggers = if !fm.globs.is_empty() {
                    fm.globs
                } else if fm.always_apply {
                    vec!["*".to_string()]
                } else {
                    Vec::new()
                };

                let body = body.trim().to_string();
                let sha256 = compute_sha256(&body);
                skills.push(Skill {
                    id: format!("{}{}", RULE_ID_PREFIX, stem),
                    version: "1.0.0".to_string(),
                    triggers,
                    targets: vec!["continue".to_string()],
                    source: body,
                    sha256,
                });
            }
        }

        Ok(ImportResult {
            skills,
            agents: Vec::new(),
            mcp_servers,
        })
    }

    fn project(
        &self,
        root: &Path,
        skills: &[Skill],
        _agents: &[Agent],
        mcp_servers: &[McpServer],
    ) -> Result<ProjectResult, AdapterError> {
        let mut written = Vec::new();

        let continue_servers: Vec<&McpServer> = mcp_servers
            .iter()
            .filter(|s| s.targets.contains(&"continue".to_string()))
            .collect();

        if !continue_servers.is_empty() {
            let config_dir = root.join(".continue");
            fs::create_dir_all(&config_dir).map_err(|e| {
                AdapterError::Io(format!("Failed to create .continue directory: {}", e))
            })?;
            let config_path = config_dir.join("config.json");

            let mut current_json = if config_path.exists() {
                let raw_json = fs::read_to_string(&config_path).map_err(|e| {
                    AdapterError::Io(format!("Failed to read .continue/config.json: {}", e))
                })?;
                serde_json::from_str(&raw_json).map_err(|e| {
                    AdapterError::Malformed(format!("Invalid JSON in .continue/config.json: {}", e))
                })?
            } else {
                json!({})
            };

            if !current_json.is_object() {
                return Err(AdapterError::Malformed(
                    ".continue/config.json root is not an object".to_string(),
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

            for server in continue_servers {
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
            fs::write(&config_path, pretty).map_err(|e| {
                AdapterError::Io(format!("Failed to write .continue/config.json: {}", e))
            })?;
            written.push(".continue/config.json".to_string());
        }

        let rule_skills: Vec<&Skill> = skills
            .iter()
            .filter(|s| {
                s.targets.contains(&"continue".to_string()) && s.id.starts_with(RULE_ID_PREFIX)
            })
            .collect();

        if !rule_skills.is_empty() {
            let rules_dir = root.join(".continue/rules");
            fs::create_dir_all(&rules_dir).map_err(|e| {
                AdapterError::Io(format!("Failed to create .continue/rules: {}", e))
            })?;

            for skill in rule_skills {
                let slug = &skill.id[RULE_ID_PREFIX.len()..];
                let fm = MdcFrontmatter {
                    globs: skill.triggers.clone(),
                    always_apply: skill.triggers == vec!["*".to_string()],
                };
                let output = format!(
                    "<!-- GENERATED BY LLM NEUROSURGEON — edit in the Brain -->\n{}\n\n{}",
                    serialize_mdc_frontmatter(&fm),
                    skill.source
                );
                let rel_path = format!(".continue/rules/{}.md", slug);
                let target_path = safe_join(root, &rel_path)?;
                fs::write(&target_path, output).map_err(|e| {
                    AdapterError::Io(format!("Failed to write {}: {}", rel_path, e))
                })?;
                written.push(rel_path);
            }
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
    fn test_continue_project_rejects_path_traversal_in_rule_slug() {
        let dir = tempdir().unwrap();
        let adapter = ContinueAdapter;

        let skill = Skill {
            id: format!("{}../../../evil", RULE_ID_PREFIX),
            version: "1.0.0".to_string(),
            triggers: vec!["*.rs".to_string()],
            targets: vec!["continue".to_string()],
            source: "malicious".to_string(),
            sha256: "hash".to_string(),
        };

        let result = adapter.project(dir.path(), &[skill], &[], &[]);
        assert!(matches!(result, Err(AdapterError::Malformed(_))));
    }

    #[test]
    fn test_continue_detect() {
        let dir = tempdir().unwrap();
        let adapter = ContinueAdapter;
        assert!(!adapter.detect(dir.path()));

        fs::create_dir_all(dir.path().join(".continue")).unwrap();
        fs::write(dir.path().join(".continue/config.json"), "{}").unwrap();
        assert!(adapter.detect(dir.path()));
    }

    #[test]
    fn test_continue_import_export_roundtrip() {
        let dir = tempdir().unwrap();
        let adapter = ContinueAdapter;

        fs::create_dir_all(dir.path().join(".continue")).unwrap();
        fs::write(
            dir.path().join(".continue/config.json"),
            r#"{"models": [], "mcpServers": {"search": {"command": "uvx", "args": ["search-mcp"]}}}"#,
        ).unwrap();

        fs::create_dir_all(dir.path().join(".continue/rules")).unwrap();
        fs::write(
            dir.path().join(".continue/rules/01-style.md"),
            "---\nglobs: [\"*.ts\"]\nalwaysApply: false\n---\nUse 2-space indentation.",
        )
        .unwrap();

        let imported = adapter.import(dir.path()).unwrap();
        assert_eq!(imported.mcp_servers.len(), 1);
        assert_eq!(imported.mcp_servers[0].id, "search");
        assert_eq!(imported.mcp_servers[0].command_or_url, "uvx search-mcp");

        assert_eq!(imported.skills.len(), 1);
        assert_eq!(imported.skills[0].id, "continue-rule-01-style");
        assert_eq!(imported.skills[0].triggers, vec!["*.ts".to_string()]);
        assert!(imported.skills[0].source.contains("2-space indentation"));

        let out_dir = tempdir().unwrap();
        fs::create_dir_all(out_dir.path().join(".continue")).unwrap();
        fs::write(
            out_dir.path().join(".continue/config.json"),
            r#"{"models": [{"title": "gpt"}]}"#,
        )
        .unwrap();

        let project_res = adapter
            .project(out_dir.path(), &imported.skills, &[], &imported.mcp_servers)
            .unwrap();
        assert_eq!(project_res.written.len(), 2);

        let projected_config =
            fs::read_to_string(out_dir.path().join(".continue/config.json")).unwrap();
        let parsed: Value = serde_json::from_str(&projected_config).unwrap();
        assert_eq!(parsed["models"][0]["title"], "gpt");
        assert_eq!(parsed["mcpServers"]["search"]["command"], "uvx");

        let projected_rule =
            fs::read_to_string(out_dir.path().join(".continue/rules/01-style.md")).unwrap();
        assert!(projected_rule.contains("2-space indentation"));
    }

    #[cfg(unix)]
    #[test]
    fn test_continue_import_does_not_follow_symlinked_rule_files() {
        let dir = tempdir().unwrap();
        let adapter = ContinueAdapter;

        let outside = tempdir().unwrap();
        let secret = outside.path().join("secret.md");
        fs::write(&secret, "top secret content").unwrap();

        fs::create_dir_all(dir.path().join(".continue/rules")).unwrap();
        std::os::unix::fs::symlink(&secret, dir.path().join(".continue/rules/planted.md")).unwrap();

        let imported = adapter.import(dir.path()).unwrap();
        assert!(imported.skills.is_empty());
    }

    #[test]
    fn test_continue_stress_malformed_config() {
        let dir = tempdir().unwrap();
        let adapter = ContinueAdapter;
        fs::create_dir_all(dir.path().join(".continue")).unwrap();
        fs::write(dir.path().join(".continue/config.json"), "{ not json").unwrap();
        let res = adapter.import(dir.path());
        assert!(matches!(res, Err(AdapterError::Malformed(_))));
    }
}
