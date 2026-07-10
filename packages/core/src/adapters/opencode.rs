use super::{compute_sha256, strip_provenance};
use crate::adapter::{Adapter, AdapterError, ImportResult, ProjectResult};
use crate::model::{Agent, McpServer, Skill};
use std::fs;
use std::path::Path;

pub struct OpenCodeAdapter;

fn parse_frontmatter(content: &str) -> (Option<String>, String) {
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() >= 2 && lines[0].trim() == "---" {
        let mut fm_end = None;
        for i in 1..lines.len() {
            if lines[i].trim() == "---" {
                fm_end = Some(i);
                break;
            }
        }
        if let Some(end_idx) = fm_end {
            let fm = lines[1..end_idx].join("\n");
            let body = lines[end_idx + 1..].join("\n");
            return (Some(fm), body);
        }
    }
    (None, content.to_string())
}

fn parse_yaml_frontmatter(fm: &str) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut tools = Vec::new();
    let mut model_hints = Vec::new();
    let mut targets = Vec::new();
    let mut current_key: Option<String> = None;

    for line in fm.lines() {
        let line_trimmed = line.trim();
        if line_trimmed.is_empty() {
            continue;
        }

        if line_trimmed.starts_with('-') {
            let val = line_trimmed[1..]
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            if let Some(ref key) = current_key {
                match key.as_str() {
                    "tools" => tools.push(val),
                    "model_hints" | "model" | "models" => model_hints.push(val),
                    "targets" => targets.push(val),
                    _ => {}
                }
            }
        } else if let Some(colon_idx) = line_trimmed.find(':') {
            let key = line_trimmed[..colon_idx].trim().to_lowercase();
            let val = line_trimmed[colon_idx + 1..].trim();
            current_key = Some(key.clone());

            if !val.is_empty() {
                if val.starts_with('[') && val.ends_with(']') {
                    let items: Vec<String> = val[1..val.len() - 1]
                        .split(',')
                        .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    match key.as_str() {
                        "tools" => tools.extend(items),
                        "model_hints" | "model" | "models" => model_hints.extend(items),
                        "targets" => targets.extend(items),
                        _ => {}
                    }
                } else {
                    let val_clean = val.trim_matches('"').trim_matches('\'').to_string();
                    match key.as_str() {
                        "tools" => tools.push(val_clean),
                        "model_hints" | "model" | "models" => model_hints.push(val_clean),
                        "targets" => targets.push(val_clean),
                        _ => {}
                    }
                }
            }
        }
    }

    (tools, model_hints, targets)
}

fn serialize_yaml_frontmatter(
    tools: &[String],
    model_hints: &[String],
    targets: &[String],
) -> String {
    let mut fm = String::new();
    fm.push_str("---\n");

    fm.push_str("tools:\n");
    for t in tools {
        fm.push_str(&format!("  - {}\n", t));
    }

    fm.push_str("model_hints:\n");
    for m in model_hints {
        fm.push_str(&format!("  - {}\n", m));
    }

    fm.push_str("targets:\n");
    for t in targets {
        fm.push_str(&format!("  - {}\n", t));
    }

    fm.push_str("---");
    fm
}

impl Adapter for OpenCodeAdapter {
    fn id(&self) -> &'static str {
        "opencode"
    }

    fn detect(&self, root: &Path) -> bool {
        root.join("AGENTS.md").exists()
    }

    fn import(&self, root: &Path) -> Result<ImportResult, AdapterError> {
        let mut agents = Vec::new();
        let mut skills = Vec::new();

        let agents_path = root.join("AGENTS.md");
        if agents_path.exists() {
            let raw_content = fs::read_to_string(&agents_path)
                .map_err(|e| AdapterError::Io(format!("Failed to read AGENTS.md: {}", e)))?;
            let content = strip_provenance(&raw_content);
            let (fm_opt, body_raw) = parse_frontmatter(&content);
            let body = body_raw.trim().to_string();

            let (tools, model_hints, mut targets) = if let Some(fm) = fm_opt {
                parse_yaml_frontmatter(&fm)
            } else {
                (Vec::new(), Vec::new(), Vec::new())
            };

            if !targets.contains(&"opencode".to_string()) {
                targets.push("opencode".to_string());
            }

            agents.push(Agent {
                slug: "opencode-agent".to_string(),
                tools,
                model_hints,
                targets: targets.clone(),
            });

            let sha256 = compute_sha256(&body);
            skills.push(Skill {
                id: "opencode-agent-instructions".to_string(),
                version: "1.0.0".to_string(),
                triggers: vec!["*".to_string()],
                targets: vec!["opencode".to_string()],
                source: body,
                sha256,
            });
        }

        Ok(ImportResult {
            skills,
            agents,
            mcp_servers: Vec::new(),
        })
    }

    fn project(
        &self,
        root: &Path,
        skills: &[Skill],
        agents: &[Agent],
        _mcp_servers: &[McpServer],
    ) -> Result<ProjectResult, AdapterError> {
        let mut written = Vec::new();

        let opencode_agent = agents
            .iter()
            .find(|a| a.targets.contains(&"opencode".to_string()));

        let opencode_skills: Vec<&Skill> = skills
            .iter()
            .filter(|s| s.targets.contains(&"opencode".to_string()))
            .collect();

        if opencode_agent.is_some() || !opencode_skills.is_empty() {
            let (tools, model_hints, targets) = if let Some(agent) = opencode_agent {
                (
                    agent.tools.clone(),
                    agent.model_hints.clone(),
                    agent.targets.clone(),
                )
            } else {
                (Vec::new(), Vec::new(), vec!["opencode".to_string()])
            };

            let companion_skill = opencode_skills
                .iter()
                .find(|s| s.id == "opencode-agent-instructions")
                .cloned()
                .or_else(|| opencode_skills.first().cloned());

            let body = companion_skill
                .map(|s| s.source.clone())
                .unwrap_or_else(|| {
                    opencode_skills
                        .iter()
                        .map(|s| s.source.as_str())
                        .collect::<Vec<&str>>()
                        .join("\n\n---\n\n")
                });

            let fm_serialized = serialize_yaml_frontmatter(&tools, &model_hints, &targets);

            let output = format!(
                "<!-- GENERATED BY LLM NEUROSURGEON — edit in the Brain -->\n{}\n{}",
                fm_serialized, body
            );

            fs::write(root.join("AGENTS.md"), output)
                .map_err(|e| AdapterError::Io(format!("Failed to write AGENTS.md: {}", e)))?;
            written.push("AGENTS.md".to_string());
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
    fn test_opencode_detect() {
        let dir = tempdir().unwrap();
        let adapter = OpenCodeAdapter;
        assert!(!adapter.detect(dir.path()));

        fs::write(dir.path().join("AGENTS.md"), "test").unwrap();
        assert!(adapter.detect(dir.path()));
    }

    #[test]
    fn test_opencode_import_export_roundtrip() {
        let dir = tempdir().unwrap();
        let adapter = OpenCodeAdapter;

        let input_content = r#"---
tools:
  - web_search
  - read_file
model_hints:
  - claude-3-5-sonnet
targets:
  - opencode
---
Some system instructions
over multiple lines."#;

        fs::write(dir.path().join("AGENTS.md"), input_content).unwrap();

        let imported = adapter.import(dir.path()).unwrap();
        assert_eq!(imported.agents.len(), 1);
        assert_eq!(imported.agents[0].slug, "opencode-agent");
        assert_eq!(imported.agents[0].tools, vec!["web_search", "read_file"]);
        assert_eq!(imported.agents[0].model_hints, vec!["claude-3-5-sonnet"]);

        assert_eq!(imported.skills.len(), 1);
        assert_eq!(imported.skills[0].id, "opencode-agent-instructions");
        assert_eq!(
            imported.skills[0].source,
            "Some system instructions\nover multiple lines."
        );

        // Project back
        let out_dir = tempdir().unwrap();
        let project_res = adapter
            .project(out_dir.path(), &imported.skills, &imported.agents, &[])
            .unwrap();

        assert_eq!(project_res.written, vec!["AGENTS.md".to_string()]);

        let projected_content = fs::read_to_string(out_dir.path().join("AGENTS.md")).unwrap();
        assert!(projected_content
            .contains("<!-- GENERATED BY LLM NEUROSURGEON — edit in the Brain -->"));
        assert!(projected_content.contains("web_search"));
        assert!(projected_content.contains("claude-3-5-sonnet"));
        assert!(projected_content.contains("Some system instructions"));
    }
}
