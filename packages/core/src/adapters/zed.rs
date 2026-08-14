use super::{compute_sha256, has_path_traversal, strip_provenance, write_if_changed};
use crate::adapter::{Adapter, AdapterError, ImportResult, ProjectResult};
use crate::model::{Agent, HealthStatus, McpServer, Skill};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

pub struct ZedAdapter;

impl Adapter for ZedAdapter {
    fn id(&self) -> &'static str {
        "zed"
    }

    fn detect(&self, root: &Path) -> bool {
        let in_root = root.join(".rules").exists()
            || root.join(".zed/settings.json").exists()
            || root.join(".config/zed/AGENTS.md").exists()
            || root.join(".config/zed/settings.json").exists();
        let in_home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .is_some_and(|h| {
                h.join(".config/zed/AGENTS.md").exists()
                    || h.join(".config/zed/settings.json").exists()
            });
        in_root || in_home
    }

    fn import(&self, root: &Path) -> Result<ImportResult, AdapterError> {
        let mut skills = Vec::new();
        let mut mcp_servers = Vec::new();

        let rules_path = root.join(".rules");
        let zed_agents_path = root.join(".config/zed/AGENTS.md");
        let home_agents_path = std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|h| h.join(".config/zed/AGENTS.md"));

        let target_rules_path = if rules_path.exists() {
            Some(rules_path)
        } else if zed_agents_path.exists() {
            Some(zed_agents_path)
        } else {
            home_agents_path.filter(|hp| hp.exists())
        };

        if let Some(path) = target_rules_path {
            let raw = fs::read_to_string(&path).map_err(|e| {
                AdapterError::Io(format!("Failed to read {}: {}", path.display(), e))
            })?;
            let content = strip_provenance(&raw);
            let sha256 = compute_sha256(&content);
            skills.push(Skill {
                id: "zed-rules".to_string(),
                version: "1.0.0".to_string(),
                triggers: vec!["*".to_string()],
                targets: vec!["zed".to_string()],
                source: content,
                sha256,
            });
        }

        let mut settings_candidates = Vec::new();
        if root.join(".zed/settings.json").exists() {
            settings_candidates.push(root.join(".zed/settings.json"));
        }
        if root.join(".config/zed/settings.json").exists() {
            settings_candidates.push(root.join(".config/zed/settings.json"));
        }
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            let home_settings = home.join(".config/zed/settings.json");
            if home_settings.exists() && !settings_candidates.contains(&home_settings) {
                settings_candidates.push(home_settings);
            }
        }

        for path in settings_candidates {
            let raw_json = fs::read_to_string(&path).map_err(|e| {
                AdapterError::Io(format!("Failed to read {}: {}", path.display(), e))
            })?;
            let parsed: Value = serde_json::from_str(&raw_json).map_err(|e| {
                AdapterError::Malformed(format!("Invalid JSON in {}: {}", path.display(), e))
            })?;

            if let Some(servers) = parsed
                .get("context_servers")
                .or_else(|| parsed.get("contextServers"))
                .or_else(|| parsed.get("mcp_servers"))
                .or_else(|| parsed.get("mcpServers"))
                .and_then(|v| v.as_object())
            {
                for (id, val) in servers {
                    let Some(obj) = val.as_object() else {
                        return Err(AdapterError::Malformed(format!(
                            "Context server '{id}' is not an object"
                        )));
                    };

                    let (command, args) =
                        if let Some(cmd_str) = obj.get("command").and_then(|v| v.as_str()) {
                            let args: Vec<String> = obj
                                .get("args")
                                .and_then(|v| v.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|i| i.as_str().map(|s| s.to_string()))
                                        .collect()
                                })
                                .unwrap_or_default();
                            (cmd_str.to_string(), args)
                        } else if let Some(cmd_obj) = obj.get("command").and_then(|v| v.as_object())
                        {
                            let path_cmd = cmd_obj
                                .get("path")
                                .and_then(|v| v.as_str())
                                .ok_or_else(|| {
                                    AdapterError::Malformed(format!(
                                        "Context server '{id}' command missing `path`"
                                    ))
                                })?;
                            let args: Vec<String> = cmd_obj
                                .get("args")
                                .and_then(|v| v.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|i| i.as_str().map(|s| s.to_string()))
                                        .collect()
                                })
                                .unwrap_or_default();
                            (path_cmd.to_string(), args)
                        } else if let Some(url) = obj.get("url").and_then(|v| v.as_str()) {
                            if url.trim().is_empty() {
                                return Err(AdapterError::Malformed(format!(
                                    "Context server '{id}' empty url"
                                )));
                            }
                            if has_path_traversal(url) {
                                return Err(AdapterError::Malformed(format!(
                                    "Path traversal in context server '{id}' url: {url}"
                                )));
                            }
                            let mut env_placeholders = Vec::new();
                            if let Some(env_obj) = obj.get("env").and_then(|v| v.as_object()) {
                                for key in env_obj.keys() {
                                    env_placeholders.push(key.clone());
                                }
                            }
                            env_placeholders.sort();
                            mcp_servers.push(McpServer {
                                id: id.clone(),
                                transport: "http".to_string(),
                                command_or_url: url.to_string(),
                                env_placeholders,
                                targets: vec!["zed".to_string()],
                                health: HealthStatus::Unknown,
                            });
                            continue;
                        } else {
                            return Err(AdapterError::Malformed(format!(
                                "Context server '{id}' missing command or url"
                            )));
                        };

                    if command.trim().is_empty() {
                        return Err(AdapterError::Malformed(format!(
                            "Context server '{id}' has empty command"
                        )));
                    }
                    if has_path_traversal(&command) {
                        return Err(AdapterError::Malformed(format!(
                            "Path traversal in context server '{id}' command: {command}"
                        )));
                    }
                    for arg in &args {
                        if has_path_traversal(arg) {
                            return Err(AdapterError::Malformed(format!(
                                "Path traversal in context server '{id}' arg: {arg}"
                            )));
                        }
                    }

                    let command_or_url = if args.is_empty() {
                        command
                    } else {
                        format!("{} {}", command, args.join(" "))
                    };

                    let mut env_placeholders = Vec::new();
                    if let Some(env_obj) = obj.get("env").and_then(|v| v.as_object()) {
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
                        targets: vec!["zed".to_string()],
                        health: HealthStatus::Unknown,
                    });
                }
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

        let zed_skills: Vec<&Skill> = skills
            .iter()
            .filter(|s| s.targets.contains(&"zed".to_string()))
            .collect();

        if !zed_skills.is_empty() {
            let concatenated: String = zed_skills
                .iter()
                .map(|s| s.source.as_str())
                .collect::<Vec<&str>>()
                .join("\n\n---\n\n");
            let output = format!(
                "<!-- GENERATED BY LLM NEUROSURGEON — edit in the Brain -->\n{}",
                concatenated
            );
            let rules_path = root.join(".rules");
            write_if_changed(&rules_path, &output)
                .map_err(|e| AdapterError::Io(format!("Failed to write .rules: {}", e)))?;
            written.push(".rules".to_string());

            let config_zed = root.join(".config/zed");
            let target_config_zed = if config_zed.exists() || root.join(".config").exists() {
                Some(config_zed)
            } else {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .map(|home| home.join(".config/zed"))
            };

            if let Some(dir) = target_config_zed {
                fs::create_dir_all(&dir).map_err(|e| {
                    AdapterError::Io(format!("Failed to create .config/zed directory: {}", e))
                })?;
                let agents_path = dir.join("AGENTS.md");
                write_if_changed(&agents_path, &output).map_err(|e| {
                    AdapterError::Io(format!("Failed to write {}: {}", agents_path.display(), e))
                })?;
                written.push(".config/zed/AGENTS.md".to_string());
            }
        }

        let zed_servers: Vec<&McpServer> = mcp_servers
            .iter()
            .filter(|s| s.targets.contains(&"zed".to_string()))
            .collect();

        if !zed_servers.is_empty() {
            let settings_dir = root.join(".zed");
            fs::create_dir_all(&settings_dir)
                .map_err(|e| AdapterError::Io(format!("Failed to create .zed directory: {}", e)))?;
            let settings_path = settings_dir.join("settings.json");

            let mut current_json = if settings_path.exists() {
                let raw_json = fs::read_to_string(&settings_path).map_err(|e| {
                    AdapterError::Io(format!("Failed to read .zed/settings.json: {}", e))
                })?;
                serde_json::from_str(&raw_json).map_err(|e| {
                    AdapterError::Malformed(format!("Invalid JSON in .zed/settings.json: {}", e))
                })?
            } else {
                json!({})
            };

            if !current_json.is_object() {
                return Err(AdapterError::Malformed(
                    ".zed/settings.json root is not an object".to_string(),
                ));
            }

            let servers_map = current_json
                .as_object_mut()
                .unwrap()
                .entry("context_servers")
                .or_insert_with(|| json!({}))
                .as_object_mut()
                .ok_or_else(|| {
                    AdapterError::Malformed("context_servers is not an object".to_string())
                })?;

            for server in zed_servers {
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

                let new_server = json!({
                    "command": command,
                    "args": args,
                    "env": env_obj
                });

                servers_map.insert(server.id.clone(), new_server);
            }

            let pretty = serde_json::to_string_pretty(&current_json)
                .map_err(|e| AdapterError::Malformed(format!("Failed to serialize JSON: {}", e)))?;
            write_if_changed(&settings_path, pretty).map_err(|e| {
                AdapterError::Io(format!("Failed to write .zed/settings.json: {}", e))
            })?;
            written.push(".zed/settings.json".to_string());
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
    fn test_zed_detect() {
        let dir = tempdir().unwrap();
        let adapter = ZedAdapter;
        assert!(!adapter.detect(dir.path()));

        fs::write(dir.path().join(".rules"), "test").unwrap();
        assert!(adapter.detect(dir.path()));
    }

    #[test]
    fn test_zed_import_export_roundtrip() {
        let dir = tempdir().unwrap();
        let adapter = ZedAdapter;

        fs::write(dir.path().join(".rules"), "Always write tests first").unwrap();
        fs::create_dir_all(dir.path().join(".zed")).unwrap();
        fs::write(
            dir.path().join(".zed/settings.json"),
            r#"{"vim_mode": true, "context_servers": {"docs": {"command": "uvx", "args": ["docs-server"]}}}"#,
        ).unwrap();

        let imported = adapter.import(dir.path()).unwrap();
        assert_eq!(imported.skills.len(), 1);
        assert_eq!(imported.skills[0].id, "zed-rules");
        assert_eq!(imported.skills[0].source, "Always write tests first");

        assert_eq!(imported.mcp_servers.len(), 1);
        assert_eq!(imported.mcp_servers[0].id, "docs");
        assert_eq!(imported.mcp_servers[0].command_or_url, "uvx docs-server");

        let out_dir = tempdir().unwrap();
        fs::create_dir_all(out_dir.path().join(".zed")).unwrap();
        fs::write(
            out_dir.path().join(".zed/settings.json"),
            r#"{"vim_mode": true}"#,
        )
        .unwrap();

        let project_res = adapter
            .project(out_dir.path(), &imported.skills, &[], &imported.mcp_servers)
            .unwrap();

        assert!(project_res.written.contains(&".rules".to_string()));
        assert!(project_res
            .written
            .iter()
            .any(|w| w.contains("settings.json")));
        let projected_rules = fs::read_to_string(out_dir.path().join(".rules")).unwrap();
        assert!(projected_rules.contains("Always write tests first"));

        let projected_settings =
            fs::read_to_string(out_dir.path().join(".zed/settings.json")).unwrap();
        let parsed: Value = serde_json::from_str(&projected_settings).unwrap();
        assert_eq!(parsed["vim_mode"], true);
        assert_eq!(parsed["context_servers"]["docs"]["command"], "uvx");
    }

    #[test]
    fn test_zed_stress_malformed_settings() {
        let dir = tempdir().unwrap();
        let adapter = ZedAdapter;
        fs::create_dir_all(dir.path().join(".zed")).unwrap();
        fs::write(dir.path().join(".zed/settings.json"), "{ broken").unwrap();
        let res = adapter.import(dir.path());
        assert!(matches!(res, Err(AdapterError::Malformed(_))));
    }
}
