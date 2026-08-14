use super::{compute_sha256, has_path_traversal, strip_provenance};
use crate::adapter::{Adapter, AdapterError, ImportResult, ProjectResult};
use crate::model::{Agent, McpServer, Skill};
use std::fs;
use std::path::Path;

pub struct AiderAdapter;

/// `.aider.conf.yml` is a flat top-level YAML map in practice (no nested
/// structures for the keys this adapter cares about), so a line-based parser
/// is enough — pulling in a YAML crate for one file isn't worth it.
fn parse_flat_yaml(content: &str) -> Result<Vec<(String, String)>, AdapterError> {
    let mut pairs = Vec::new();
    for (lineno, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim_end();
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let colon_idx = trimmed.find(':').ok_or_else(|| {
            AdapterError::Malformed(format!(
                "malformed config: line {}: expected `key: value`, got `{}`",
                lineno + 1,
                trimmed
            ))
        })?;
        let key = trimmed[..colon_idx].trim().to_string();
        let val_part = trimmed[colon_idx + 1..].trim();
        if val_part.starts_with('[') && !val_part.ends_with(']') {
            return Err(AdapterError::Malformed(format!(
                "malformed config: line {} has unclosed bracket in YAML",
                lineno + 1
            )));
        }
        let value = val_part.trim_matches('"').trim_matches('\'').to_string();
        pairs.push((key, value));
    }
    Ok(pairs)
}

fn serialize_flat_yaml(pairs: &[(String, String)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{}: {}", k, v))
        .collect::<Vec<String>>()
        .join("\n")
}

fn set_key(pairs: &mut Vec<(String, String)>, key: &str, value: &str) {
    if let Some(entry) = pairs.iter_mut().find(|(k, _)| k == key) {
        entry.1 = value.to_string();
    } else {
        pairs.push((key.to_string(), value.to_string()));
    }
}

impl Adapter for AiderAdapter {
    fn id(&self) -> &'static str {
        "aider"
    }

    fn detect(&self, root: &Path) -> bool {
        root.join("CONVENTIONS.md").exists() || root.join(".aider.conf.yml").exists()
    }

    fn import(&self, root: &Path) -> Result<ImportResult, AdapterError> {
        let mut skills = Vec::new();

        let conf_path = root.join(".aider.conf.yml");
        if conf_path.exists() {
            let raw = fs::read_to_string(&conf_path)
                .map_err(|e| AdapterError::Io(format!("Failed to read .aider.conf.yml: {}", e)))?;
            let pairs = parse_flat_yaml(&raw)?;
            for (key, val) in pairs {
                if key == "read" {
                    let cleaned_val = val.trim_matches('[').trim_matches(']').trim();
                    for item in cleaned_val.split(',') {
                        let path_str = item
                            .trim()
                            .trim_matches('"')
                            .trim_matches('\'')
                            .replace('\\', "/");
                        if path_str.is_empty() {
                            continue;
                        }
                        if has_path_traversal(&path_str) {
                            return Err(AdapterError::Malformed(format!(
                                "malformed config: path traversal detected in read path: {}",
                                path_str
                            )));
                        }
                        let target_file = root.join(&path_str);
                        if target_file.exists() {
                            if let Ok(raw_content) = fs::read_to_string(&target_file) {
                                let content = strip_provenance(&raw_content);
                                let sha256 = compute_sha256(&content);
                                let stem = Path::new(&path_str)
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("rules");
                                skills.push(Skill {
                                    id: stem.to_string(),
                                    version: "1.0.0".to_string(),
                                    triggers: vec![path_str],
                                    targets: vec!["aider".to_string()],
                                    source: content,
                                    sha256,
                                });
                            }
                        }
                    }
                }
            }
        }

        let conventions_path = root.join("CONVENTIONS.md");
        if conventions_path.exists() {
            let raw = fs::read_to_string(&conventions_path)
                .map_err(|e| AdapterError::Io(format!("Failed to read CONVENTIONS.md: {}", e)))?;
            let content = strip_provenance(&raw);
            let sha256 = compute_sha256(&content);
            skills.push(Skill {
                id: "conventions".to_string(),
                version: "1.0.0".to_string(),
                triggers: vec!["*".to_string()],
                targets: vec!["aider".to_string()],
                source: content,
                sha256,
            });
        }

        Ok(ImportResult {
            skills,
            agents: Vec::new(),
            mcp_servers: Vec::new(),
        })
    }

    fn project(
        &self,
        root: &Path,
        skills: &[Skill],
        _agents: &[Agent],
        _mcp_servers: &[McpServer],
    ) -> Result<ProjectResult, AdapterError> {
        let mut written = Vec::new();

        let aider_skills: Vec<&Skill> = skills
            .iter()
            .filter(|s| s.targets.contains(&"aider".to_string()))
            .collect();

        if !aider_skills.is_empty() {
            let concatenated: String = aider_skills
                .iter()
                .map(|s| s.source.as_str())
                .collect::<Vec<&str>>()
                .join("\n\n---\n\n");
            let output = format!(
                "<!-- GENERATED BY LLM NEUROSURGEON — edit in the Brain -->\n{}",
                concatenated
            );
            fs::write(root.join("CONVENTIONS.md"), output)
                .map_err(|e| AdapterError::Io(format!("Failed to write CONVENTIONS.md: {}", e)))?;
            written.push("CONVENTIONS.md".to_string());

            let conf_path = root.join(".aider.conf.yml");
            let mut pairs = if conf_path.exists() {
                let raw = fs::read_to_string(&conf_path).map_err(|e| {
                    AdapterError::Io(format!("Failed to read .aider.conf.yml: {}", e))
                })?;
                parse_flat_yaml(&raw)?
            } else {
                Vec::new()
            };
            set_key(&mut pairs, "read", "CONVENTIONS.md");
            fs::write(&conf_path, serialize_flat_yaml(&pairs))
                .map_err(|e| AdapterError::Io(format!("Failed to write .aider.conf.yml: {}", e)))?;
            written.push(".aider.conf.yml".to_string());
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
    fn test_aider_detect() {
        let dir = tempdir().unwrap();
        let adapter = AiderAdapter;
        assert!(!adapter.detect(dir.path()));

        fs::write(dir.path().join("CONVENTIONS.md"), "test").unwrap();
        assert!(adapter.detect(dir.path()));

        let dir2 = tempdir().unwrap();
        fs::write(dir2.path().join(".aider.conf.yml"), "read: CONVENTIONS.md").unwrap();
        assert!(adapter.detect(dir2.path()));
    }

    #[test]
    fn test_aider_import_export_roundtrip() {
        let dir = tempdir().unwrap();
        let adapter = AiderAdapter;

        fs::write(
            dir.path().join("CONVENTIONS.md"),
            "Use snake_case for Python.",
        )
        .unwrap();
        fs::write(
            dir.path().join(".aider.conf.yml"),
            "model: gpt-4o\nread: CONVENTIONS.md\n# a comment\nauto-commits: false",
        )
        .unwrap();

        let imported = adapter.import(dir.path()).unwrap();
        assert!(imported.skills.len() >= 1);
        assert_eq!(imported.skills[0].id.to_lowercase(), "conventions");
        assert_eq!(imported.skills[0].source, "Use snake_case for Python.");

        let out_dir = tempdir().unwrap();
        // Pre-existing unrelated key must survive the merge.
        fs::write(out_dir.path().join(".aider.conf.yml"), "model: gpt-4o").unwrap();

        let project_res = adapter
            .project(out_dir.path(), &imported.skills, &[], &[])
            .unwrap();
        assert_eq!(project_res.written.len(), 2);

        let projected_conventions =
            fs::read_to_string(out_dir.path().join("CONVENTIONS.md")).unwrap();
        assert!(projected_conventions.contains("Use snake_case for Python."));

        let projected_conf = fs::read_to_string(out_dir.path().join(".aider.conf.yml")).unwrap();
        assert!(projected_conf.contains("model: gpt-4o"));
        assert!(projected_conf.contains("read: CONVENTIONS.md"));
    }

    #[test]
    fn test_aider_stress_malformed_conf() {
        let dir = tempdir().unwrap();
        let adapter = AiderAdapter;
        fs::write(
            dir.path().join(".aider.conf.yml"),
            "this is not a key value line",
        )
        .unwrap();
        let res = adapter.import(dir.path());
        assert!(matches!(res, Err(AdapterError::Malformed(_))));
    }

    #[test]
    fn test_aider_stress_empty_conventions() {
        let dir = tempdir().unwrap();
        let adapter = AiderAdapter;
        fs::write(dir.path().join("CONVENTIONS.md"), "").unwrap();
        let imported = adapter.import(dir.path()).unwrap();
        assert_eq!(imported.skills.len(), 1);
        assert_eq!(imported.skills[0].source, "");
    }
}
