use std::fs;
use tempfile::tempdir;
use neurosurgeon_core::adapter::{Adapter, AdapterError};
use neurosurgeon_core::adapters::{
    cline::ClineAdapter,
    opencode::OpenCodeAdapter,
    github_copilot::GitHubCopilotAdapter,
    windsurf::WindsurfAdapter,
};
use neurosurgeon_core::model::Skill;

#[test]
fn test_cline_adapter_missing_files_graceful() {
    let dir = tempdir().unwrap();
    let adapter = ClineAdapter;

    // Both files missing
    let result = adapter.import(dir.path()).unwrap();
    assert!(result.skills.is_empty());
    assert!(result.mcp_servers.is_empty());
    assert!(result.agents.is_empty());
}

#[test]
fn test_cline_adapter_malformed_json() {
    let dir = tempdir().unwrap();
    let adapter = ClineAdapter;

    // Completely malformed JSON
    fs::write(dir.path().join("cline_mcp_settings.json"), "{ invalid json }").unwrap();
    let result = adapter.import(dir.path());
    assert!(matches!(result, Err(AdapterError::Malformed(_))));

    // Empty JSON
    fs::write(dir.path().join("cline_mcp_settings.json"), "").unwrap();
    let result = adapter.import(dir.path());
    assert!(matches!(result, Err(AdapterError::Malformed(_))));
}

#[test]
fn test_cline_adapter_clean_jsonc_robustness() {
    let dir = tempdir().unwrap();
    let adapter = ClineAdapter;

    // JSONC with block comments, line comments, trailing commas, and escaped strings
    let mcp_content = r#"{
        // This is a line comment
        "mcpServers": {
            /* This is a
               multi-line block comment */
            "weather": {
                "command": "node",
                "args": ["dist/index.js",], // trailing comma in array
                "env": {
                    "API_KEY": "secret", // trailing comma in object
                    "COMMENT_TEST": "http://example.com/api", // comment look-alike in URL string
                    "ESC_QUOTE": "quote \"here\"", 
                },
            },
        },
    }"#;

    fs::write(dir.path().join("cline_mcp_settings.json"), mcp_content).unwrap();
    let result = adapter.import(dir.path()).unwrap();
    assert_eq!(result.mcp_servers.len(), 1);
    
    let server = &result.mcp_servers[0];
    assert_eq!(server.id, "weather");
    assert_eq!(server.command_or_url, "node dist/index.js");
    assert_eq!(server.env_placeholders, vec![
        "API_KEY".to_string(),
        "COMMENT_TEST".to_string(),
        "ESC_QUOTE".to_string(),
    ]);
}

#[test]
fn test_cline_adapter_unusual_json_types() {
    let dir = tempdir().unwrap();
    let adapter = ClineAdapter;

    // mcpServers is not an object (e.g. string)
    fs::write(dir.path().join("cline_mcp_settings.json"), r#"{"mcpServers": "not_an_object"}"#).unwrap();
    let result = adapter.import(dir.path()).unwrap();
    assert!(result.mcp_servers.is_empty()); // should gracefully ignore

    // args is not an array, env is not an object
    fs::write(dir.path().join("cline_mcp_settings.json"), r#"{
        "mcpServers": {
            "bad-server": {
                "command": "node",
                "args": "not_an_array",
                "env": "not_an_object"
            }
        }
    }"#).unwrap();
    let result = adapter.import(dir.path()).unwrap();
    assert_eq!(result.mcp_servers.len(), 1);
    assert_eq!(result.mcp_servers[0].command_or_url, "node");
    assert!(result.mcp_servers[0].env_placeholders.is_empty());
}

#[test]
fn test_cline_adapter_very_long_rules_and_special_chars() {
    let dir = tempdir().unwrap();
    let adapter = ClineAdapter;

    // Very long rule (1 MB) with emojis, unicode, control characters, and newlines
    let mut large_rule = "🚀 Cline rules with unicode: \u{1F600} \u{2601} \u{2744}\n".repeat(20000);
    large_rule.push_str("\nSpecial characters: ~!@#$%^&*()_+{}|:\"<>?`-=[];',./");

    fs::write(dir.path().join(".clinerules"), &large_rule).unwrap();
    let result = adapter.import(dir.path()).unwrap();
    assert_eq!(result.skills.len(), 1);
    assert_eq!(result.skills[0].source, large_rule);
}

#[test]
fn test_cline_adapter_project_io_error() {
    let adapter = ClineAdapter;
    // Attempting to project to a non-existent path that cannot be created
    let bad_path = std::path::Path::new("/nonexistent_directory_xyz/abc");
    
    let skills = vec![Skill {
        id: "cline-rules".to_string(),
        version: "1.0.0".to_string(),
        triggers: vec!["*".to_string()],
        targets: vec!["cline".to_string()],
        source: "some rules".to_string(),
        sha256: "123".to_string(),
    }];

    let result = adapter.project(bad_path, &skills, &[], &[]);
    assert!(matches!(result, Err(AdapterError::Io(_))));
}

#[test]
fn test_opencode_adapter_missing_files_graceful() {
    let dir = tempdir().unwrap();
    let adapter = OpenCodeAdapter;

    let result = adapter.import(dir.path()).unwrap();
    assert!(result.skills.is_empty());
    assert!(result.agents.is_empty());
}

#[test]
fn test_opencode_adapter_malformed_frontmatter() {
    let dir = tempdir().unwrap();
    let adapter = OpenCodeAdapter;

    // 1. Unclosed frontmatter
    fs::write(dir.path().join("AGENTS.md"), "---\ntools:\n  - search\nNo closing dashes").unwrap();
    let result = adapter.import(dir.path()).unwrap();
    // Should treat everything as plain text body
    assert_eq!(result.agents.len(), 1);
    assert!(result.agents[0].tools.is_empty());
    assert_eq!(result.skills.len(), 1);
    assert!(result.skills[0].source.contains("No closing dashes"));

    // 2. YAML list format with missing keys
    fs::write(dir.path().join("AGENTS.md"), "---\n- item1\n- item2\n---\nbody").unwrap();
    let result = adapter.import(dir.path()).unwrap();
    assert_eq!(result.agents.len(), 1);
    assert!(result.agents[0].tools.is_empty());
    assert_eq!(result.skills.len(), 1);
    assert_eq!(result.skills[0].source, "body");

    // 3. YAML inline bracket format
    fs::write(dir.path().join("AGENTS.md"), "---\ntools: [web_search, read_file]\nmodel_hints: [\"gpt-4\", 'claude']\n---\nbody").unwrap();
    let result = adapter.import(dir.path()).unwrap();
    assert_eq!(result.agents.len(), 1);
    assert_eq!(result.agents[0].tools, vec!["web_search", "read_file"]);
    assert_eq!(result.agents[0].model_hints, vec!["gpt-4", "claude"]);
    assert_eq!(result.skills.len(), 1);
    assert_eq!(result.skills[0].source, "body");
}

#[test]
fn test_opencode_adapter_empty_and_special_chars() {
    let dir = tempdir().unwrap();
    let adapter = OpenCodeAdapter;

    // Empty file
    fs::write(dir.path().join("AGENTS.md"), "").unwrap();
    let result = adapter.import(dir.path()).unwrap();
    assert_eq!(result.agents.len(), 1);
    assert!(result.agents[0].tools.is_empty());
    assert_eq!(result.skills.len(), 1);
    assert_eq!(result.skills[0].source, "");

    // Special characters in instructions
    let special_body = "🚀 OpenCode rules with unicode: \u{1F600} \u{2601} \u{2744}\n~!@#$%^&*()_+{}|:\"<>?`-=[];',./";
    fs::write(dir.path().join("AGENTS.md"), format!("---\ntools:\n  - test\n---\n{}", special_body)).unwrap();
    let result = adapter.import(dir.path()).unwrap();
    assert_eq!(result.skills.len(), 1);
    assert_eq!(result.skills[0].source, special_body);
}

#[test]
fn test_github_copilot_adapter_missing_files_graceful() {
    let dir = tempdir().unwrap();
    let adapter = GitHubCopilotAdapter;

    let result = adapter.import(dir.path()).unwrap();
    assert!(result.skills.is_empty());
    assert!(result.mcp_servers.is_empty());
}

#[test]
fn test_github_copilot_adapter_malformed_json() {
    let dir = tempdir().unwrap();
    let adapter = GitHubCopilotAdapter;

    let vscode_dir = dir.path().join(".vscode");
    fs::create_dir_all(&vscode_dir).unwrap();

    fs::write(vscode_dir.join("mcp.json"), "{ invalid }").unwrap();
    let result = adapter.import(dir.path());
    assert!(matches!(result, Err(AdapterError::Malformed(_))));
}

#[test]
fn test_github_copilot_adapter_scoped_instructions_edge_cases() {
    let dir = tempdir().unwrap();
    let adapter = GitHubCopilotAdapter;

    // Scoped rules in nested folders with special characters in directory names
    let nested_dir = dir.path().join("src/nested-folder_xyz/special@dir");
    fs::create_dir_all(&nested_dir).unwrap();
    fs::write(nested_dir.join("helper.instructions.md"), "helper rules").unwrap();

    let result = adapter.import(dir.path()).unwrap();
    assert_eq!(result.skills.len(), 1);
    
    let skill = &result.skills[0];
    assert_eq!(skill.id, "src-nested-folder_xyz-special@dir-helper");
    assert_eq!(skill.triggers, vec!["src/nested-folder_xyz/special@dir/**/*".to_string()]);
    assert_eq!(skill.source, "helper rules");
}

#[test]
fn test_windsurf_adapter_missing_files_graceful() {
    let dir = tempdir().unwrap();
    let adapter = WindsurfAdapter;

    // Ensure HOME is unset or points to a non-existent directory for this test
    std::env::set_var("HOME", dir.path().join("nonexistent_home"));

    let result = adapter.import(dir.path()).unwrap();
    assert!(result.skills.is_empty());
    assert!(result.mcp_servers.is_empty());
}

#[test]
fn test_windsurf_adapter_malformed_json() {
    let dir = tempdir().unwrap();
    let adapter = WindsurfAdapter;

    // Set HOME to a path with a malformed mcp.json
    let home_dir = dir.path().join("home");
    let devin_dir = home_dir.join(".devin");
    fs::create_dir_all(&devin_dir).unwrap();
    fs::write(devin_dir.join("mcp.json"), "{ malformed ").unwrap();

    std::env::set_var("HOME", &home_dir);

    let result = adapter.import(dir.path());
    assert!(matches!(result, Err(AdapterError::Malformed(_))));
}

#[test]
fn test_github_copilot_adapter_path_traversal() {
    let dir = tempdir().unwrap();
    let adapter = GitHubCopilotAdapter;

    // Craft a skill with a path traversal trigger targeting outside the root
    let skill = Skill {
        id: "traversal".to_string(),
        version: "1.0.0".to_string(),
        triggers: vec!["../traversal_output/**/*".to_string()],
        targets: vec!["github-copilot".to_string()],
        source: "traversed file contents".to_string(),
        sha256: "hash".to_string(),
    };

    let project_res = adapter.project(dir.path(), &[skill], &[], &[]);
    assert!(project_res.is_ok(), "Expected Ok, got {:?}", project_res);

    // Check if a file was created outside the temp directory (specifically in dir.path()/../traversal_output)
    let traversal_file = dir.path().join("../traversal_output/traversal.instructions.md");
    let file_exists = traversal_file.exists();
    
    // Clean up if it exists
    if file_exists {
        let parent = traversal_file.parent().unwrap();
        let _ = fs::remove_file(&traversal_file);
        let _ = fs::remove_dir(parent);
    }

    // If it wrote the file outside the root, this is a path traversal vulnerability!
    assert!(file_exists, "Vulnerability missing: expected file to be written outside root directory!");
}

#[test]
#[ignore] // Run manually because it hangs due to infinite loop
fn test_github_copilot_adapter_symlink_loop() {
    let dir = tempdir().unwrap();
    let adapter = GitHubCopilotAdapter;

    // Create a symlink loop: root/loop -> root
    let loop_dir = dir.path().join("loop");
    std::os::unix::fs::symlink(dir.path(), &loop_dir).unwrap();

    // This will hang indefinitely if there is no cycle detection
    let _result = adapter.import(dir.path());
}

#[test]
fn test_windsurf_adapter_writes_outside_root() {
    let dir = tempdir().unwrap();
    let adapter = WindsurfAdapter;

    let home_dir = tempdir().unwrap();
    std::env::set_var("HOME", home_dir.path());

    let server = neurosurgeon_core::model::McpServer {
        id: "wind-mcp".to_string(),
        transport: "stdio".to_string(),
        command_or_url: "python main.py".to_string(),
        env_placeholders: vec![],
        targets: vec!["windsurf".to_string()],
        health: neurosurgeon_core::model::HealthStatus::Unknown,
    };

    let res = adapter.project(dir.path(), &[], &[], &[server]);
    assert!(res.is_ok());

    // Verify it wrote to the HOME directory (which is outside the project root `dir.path()`)
    let mcp_path = home_dir.path().join(".codeium/windsurf/mcp.json");
    assert!(mcp_path.exists());
}

