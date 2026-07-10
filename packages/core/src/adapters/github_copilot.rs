use std::path::{Path, PathBuf};
use std::fs;
use serde_json::{json, Value};
use crate::adapter::{Adapter, AdapterError, ImportResult, ProjectResult};
use crate::model::{Agent, McpServer, Skill, HealthStatus};
use super::{compute_sha256, strip_provenance, clean_jsonc};

pub struct GitHubCopilotAdapter;

fn find_instruction_files(dir: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();
    let mut queue = vec![dir.to_path_buf()];
    while let Some(current) = queue.pop() {
        if let Ok(entries) = fs::read_dir(current) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name == ".git" || name == "node_modules" || name == "target" {
                        continue;
                    }
                    queue.push(path);
                } else if path.is_file() {
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        if filename.ends_with(".instructions.md") {
                            results.push(path);
                        }
                    }
                }
            }
        }
    }
    results
}

fn get_scoped_trigger(relative_path: &Path) -> Vec<String> {
    if let Some(parent) = relative_path.parent() {
        let parent_str = parent.to_string_lossy().replace('\\', "/");
        if !parent_str.is_empty() && parent_str != "." {
            let prefix = if parent_str.ends_with('/') {
                parent_str
            } else {
                format!("{}/", parent_str)
            };
            return vec![format!("{}**/*", prefix)];
        }
    }
    vec!["*".to_string()]
}

impl Adapter for GitHubCopilotAdapter {
    fn id(&self) -> &'static str {
        "github-copilot"
    }

    fn detect(&self, root: &Path) -> bool {
        root.join(".github/copilot-instructions.md").exists()
            || root.join("copilot-instructions.md").exists()
            || root.join(".vscode/mcp.json").exists()
    }

    fn import(&self, root: &Path) -> Result<ImportResult, AdapterError> {
        let mut skills = Vec::new();
        let mut mcp_servers = Vec::new();

        // 1. Global instructions
        let dot_github_path = root.join(".github/copilot-instructions.md");
        if dot_github_path.exists() {
            let raw = fs::read_to_string(&dot_github_path)
                .map_err(|e| AdapterError::Io(format!("Failed to read .github/copilot-instructions.md: {}", e)))?;
            let content = strip_provenance(&raw);
            let sha256 = compute_sha256(&content);
            skills.push(Skill {
                id: "github-copilot-instructions".to_string(),
                version: "1.0.0".to_string(),
                triggers: vec!["*".to_string()],
                targets: vec!["github-copilot".to_string()],
                source: content,
                sha256,
            });
        }

        let root_instructions_path = root.join("copilot-instructions.md");
        if root_instructions_path.exists() {
            let raw = fs::read_to_string(&root_instructions_path)
                .map_err(|e| AdapterError::Io(format!("Failed to read copilot-instructions.md: {}", e)))?;
            let content = strip_provenance(&raw);
            let sha256 = compute_sha256(&content);
            skills.push(Skill {
                id: "copilot-instructions-root".to_string(),
                version: "1.0.0".to_string(),
                triggers: vec!["*".to_string()],
                targets: vec!["github-copilot".to_string()],
                source: content,
                sha256,
            });
        }

        // 2. Scoped instructions
        let found_files = find_instruction_files(root);
        for file in found_files {
            let rel_path = file.strip_prefix(root).unwrap_or(&file);
            let rel_str = rel_path.to_string_lossy().replace('\\', "/");
            if rel_str == ".github/copilot-instructions.md" || rel_str == "copilot-instructions.md" {
                continue;
            }

            let raw = fs::read_to_string(&file)
                .map_err(|e| AdapterError::Io(format!("Failed to read {}: {}", rel_str, e)))?;
            let content = strip_provenance(&raw);
            let sha256 = compute_sha256(&content);
            let triggers = get_scoped_trigger(rel_path);
            let id = rel_str.replace(".instructions.md", "").replace('/', "-");

            skills.push(Skill {
                id,
                version: "1.0.0".to_string(),
                triggers,
                targets: vec!["github-copilot".to_string()],
                source: content,
                sha256,
            });
        }

        // 3. MCP Servers
        let mcp_path = root.join(".vscode/mcp.json");
        if mcp_path.exists() {
            let raw_json = fs::read_to_string(&mcp_path)
                .map_err(|e| AdapterError::Io(format!("Failed to read .vscode/mcp.json: {}", e)))?;
            let cleaned = clean_jsonc(&raw_json);
            let parsed: Value = serde_json::from_str(&cleaned)
                .map_err(|e| AdapterError::Malformed(format!("Invalid JSON in .vscode/mcp.json: {}", e)))?;

            if let Some(mcp_servers_val) = parsed.get("mcpServers").and_then(|v| v.as_object()) {
                for (id, val) in mcp_servers_val {
                    let command = val.get("command")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let args: Vec<String> = val.get("args")
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
                        targets: vec!["github-copilot".to_string()],
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

        let copilot_skills: Vec<&Skill> = skills
            .iter()
            .filter(|s| s.targets.contains(&"github-copilot".to_string()))
            .collect();

        // Separate global vs scoped
        let mut global_skills = Vec::new();
        let mut scoped_skills = Vec::new();

        for skill in copilot_skills {
            let is_scoped = skill.triggers.iter().any(|t| t.contains("/**/*"));
            if is_scoped {
                scoped_skills.push(skill);
            } else {
                global_skills.push(skill);
            }
        }

        // 1. Write Global Skills
        if !global_skills.is_empty() {
            let concatenated: String = global_skills
                .iter()
                .map(|s| s.source.as_str())
                .collect::<Vec<&str>>()
                .join("\n\n---\n\n");
            let output = format!(
                "<!-- GENERATED BY LLM NEUROSURGEON — edit in the Brain -->\n{}",
                concatenated
            );
            
            let dot_github_dir = root.join(".github");
            fs::create_dir_all(&dot_github_dir)
                .map_err(|e| AdapterError::Io(format!("Failed to create .github folder: {}", e)))?;

            let target_path = dot_github_dir.join("copilot-instructions.md");
            fs::write(&target_path, output)
                .map_err(|e| AdapterError::Io(format!("Failed to write .github/copilot-instructions.md: {}", e)))?;
            
            written.push(".github/copilot-instructions.md".to_string());
        }

        // 2. Write Scoped Skills
        for skill in scoped_skills {
            if let Some(trigger) = skill.triggers.first() {
                let dir_str = trigger.replace("/**/*", "");
                let clean_dir = dir_str.trim_end_matches('/');
                
                let prefix = format!("{}-", clean_dir.replace('/', "-"));
                let name = if skill.id.starts_with(&prefix) {
                    &skill.id[prefix.len()..]
                } else {
                    &skill.id
                };

                let relative_target = if clean_dir.is_empty() || clean_dir == "." {
                    format!("{}.instructions.md", name)
                } else {
                    format!("{}/{}.instructions.md", clean_dir, name)
                };

                let target_path = root.join(&relative_target);
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| AdapterError::Io(format!("Failed to create parent directory for scoped skill: {}", e)))?;
                }

                let output = format!(
                    "<!-- GENERATED BY LLM NEUROSURGEON — edit in the Brain -->\n{}",
                    skill.source
                );

                fs::write(&target_path, output)
                    .map_err(|e| AdapterError::Io(format!("Failed to write scoped skill to {}: {}", relative_target, e)))?;

                written.push(relative_target);
            }
        }

        // 3. Write MCP Servers
        let copilot_servers: Vec<&McpServer> = mcp_servers
            .iter()
            .filter(|s| s.targets.contains(&"github-copilot".to_string()))
            .collect();

        if !copilot_servers.is_empty() {
            let mcp_dir = root.join(".vscode");
            fs::create_dir_all(&mcp_dir)
                .map_err(|e| AdapterError::Io(format!("Failed to create .vscode directory: {}", e)))?;

            let mcp_path = mcp_dir.join("mcp.json");
            let mut current_json = if mcp_path.exists() {
                let raw_json = fs::read_to_string(&mcp_path)
                    .map_err(|e| AdapterError::Io(format!("Failed to read .vscode/mcp.json: {}", e)))?;
                let cleaned = clean_jsonc(&raw_json);
                serde_json::from_str(&cleaned).unwrap_or_else(|_| json!({ "mcpServers": {} }))
            } else {
                json!({ "mcpServers": {} })
            };

            if !current_json.is_object() {
                current_json = json!({ "mcpServers": {} });
            }

            let mcp_servers_map = current_json
                .as_object_mut()
                .unwrap()
                .entry("mcpServers")
                .or_insert_with(|| json!({}))
                .as_object_mut()
                .ok_or_else(|| AdapterError::Malformed("mcpServers is not an object".to_string()))?;

            for server in copilot_servers {
                let parts: Vec<&str> = server.command_or_url.split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }
                let command = parts[0].to_string();
                let args: Vec<Value> = parts[1..].iter().map(|s| Value::String(s.to_string())).collect();

                let mut env_obj = serde_json::Map::new();
                if let Some(existing_server) = mcp_servers_map.get(&server.id) {
                    if let Some(existing_env) = existing_server.get("env").and_then(|v| v.as_object()) {
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

                mcp_servers_map.insert(server.id.clone(), new_server);
            }

            let pretty = serde_json::to_string_pretty(&current_json)
                .map_err(|e| AdapterError::Malformed(format!("Failed to serialize JSON: {}", e)))?;
            fs::write(&mcp_path, pretty)
                .map_err(|e| AdapterError::Io(format!("Failed to write .vscode/mcp.json: {}", e)))?;
            written.push(".vscode/mcp.json".to_string());
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
    fn test_github_copilot_detect() {
        let dir = tempdir().unwrap();
        let adapter = GitHubCopilotAdapter;
        assert!(!adapter.detect(dir.path()));

        let dot_github = dir.path().join(".github");
        fs::create_dir_all(&dot_github).unwrap();
        fs::write(dot_github.join("copilot-instructions.md"), "test").unwrap();
        assert!(adapter.detect(dir.path()));
    }

    #[test]
    fn test_github_copilot_import_export_roundtrip() {
        let dir = tempdir().unwrap();
        let adapter = GitHubCopilotAdapter;

        // 1. Create global rules
        let dot_github = dir.path().join(".github");
        fs::create_dir_all(&dot_github).unwrap();
        fs::write(dot_github.join("copilot-instructions.md"), "global config").unwrap();

        // 2. Create scoped rules
        let src_dir = dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("rust.instructions.md"), "rust specific config").unwrap();

        // 3. Create mcp settings
        let vscode_dir = dir.path().join(".vscode");
        fs::create_dir_all(&vscode_dir).unwrap();
        fs::write(vscode_dir.join("mcp.json"), r#"{"mcpServers": {"test-mcp": {"command": "npm", "args": ["run", "mcp"]}}}"#).unwrap();

        let imported = adapter.import(dir.path()).unwrap();
        assert_eq!(imported.skills.len(), 2);
        
        let global_skill = imported.skills.iter().find(|s| s.id == "github-copilot-instructions").unwrap();
        assert_eq!(global_skill.source, "global config");
        assert_eq!(global_skill.triggers, vec!["*".to_string()]);

        let scoped_skill = imported.skills.iter().find(|s| s.id == "src-rust").unwrap();
        assert_eq!(scoped_skill.source, "rust specific config");
        assert_eq!(scoped_skill.triggers, vec!["src/**/*".to_string()]);

        assert_eq!(imported.mcp_servers.len(), 1);
        assert_eq!(imported.mcp_servers[0].id, "test-mcp");
        assert_eq!(imported.mcp_servers[0].command_or_url, "npm run mcp");

        // Now project back
        let out_dir = tempdir().unwrap();
        let project_res = adapter.project(
            out_dir.path(),
            &imported.skills,
            &[],
            &imported.mcp_servers,
        ).unwrap();

        assert_eq!(project_res.written.len(), 3);
        assert!(project_res.written.contains(&".github/copilot-instructions.md".to_string()));
        assert!(project_res.written.contains(&"src/rust.instructions.md".to_string()));
        assert!(project_res.written.contains(&".vscode/mcp.json".to_string()));

        let projected_global = fs::read_to_string(out_dir.path().join(".github/copilot-instructions.md")).unwrap();
        assert!(projected_global.contains("global config"));

        let projected_scoped = fs::read_to_string(out_dir.path().join("src/rust.instructions.md")).unwrap();
        assert!(projected_scoped.contains("rust specific config"));
    }
}
